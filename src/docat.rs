use crate::config::config::Config;
use crate::file::{cached_config_file, cached_config_path, CONFIG_FILENAME};
use crate::git::ConfigCmd;
use crate::service::{Service, Status};
use crate::{
    cmd, config, cwd, docker, git, ComposeCmd, NetworkCmd, Parameters, Project, ProjectDirName,
    VolumeCmd,
};
use anyhow::Result;
use dialoguer::Confirm;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::BufRead;
use std::path::Path;

pub fn init(app_name: String) -> Result<()> {
    let mut project_config_file = cwd();
    project_config_file.push(CONFIG_FILENAME);
    if Path::exists(&project_config_file) {
        println!("Config file found, skipping.");
        return Ok(());
    }

    let cached_config =
        &mut config::load_from(&cached_config_path()).or(Result::<Config>::Ok(Config::new()))?;

    // generate new project config
    let mut new_config = Config::new();
    let config_filename = cwd();
    let dir_name = config_filename.file_name().unwrap().to_str().unwrap();
    let new_app = new_config.add_app(&app_name);

    // if this is a new init
    if !cached_config.apps.contains_key(&app_name) {
        new_app.config.shared_network = app_name.clone();

        let project = new_app.add_project(dir_name);
        project.is_install = true;

        // add app config to cached config
        let cached_app = cached_config.add_app(&app_name);
        cached_app.config.init(&cwd());

        set_git_config(project, dir_name)
    } else {
        // add app to install config if it doesn't exist
        let cached_app = cached_config.get(&app_name);
        if !cached_app.projects.contains_key(dir_name) {
            cached_app
                .projects
                .iter()
                .find(|(_, project)| project.is_install)
                .and_then(|(_, project)| {
                    config::load_from(&project.dir)
                        .ok()
                        .and_then(|mut install_config| {
                            install_config
                                .apps
                                .get(&app_name)
                                .and_then(|project_app| {
                                    project_app
                                        .projects
                                        .get(dir_name)
                                        .cloned()
                                        .or(Some(project_app.clone().add_project(dir_name).clone()))
                                })
                                .map(|mut new_project| {
                                    let mut install_config_file = project.dir.clone();
                                    install_config_file.push(CONFIG_FILENAME);
                                    set_git_config(&mut new_project, dir_name);

                                    install_config
                                        .get(&app_name)
                                        .projects
                                        .insert(dir_name.parse().unwrap(), new_project);

                                    // write project config
                                    let config_yaml = serde_yaml::to_string(&install_config)
                                        .expect("Could not create yaml");
                                    fs::write(install_config_file, config_yaml)
                                        .expect("Could not write config");
                                })
                        })
                });
        }
    }

    let cached_config = cached_config.merge(&new_config);

    let config_yaml = serde_yaml::to_string(&new_config)?;
    let cached_config_yaml = serde_yaml::to_string(&cached_config)?;

    if Confirm::new()
        .with_prompt("Confirm generating docat.yml?")
        .interact()?
    {
        fs::write(project_config_file, config_yaml)?;
        fs::write(cached_config_file(), cached_config_yaml)?;
        println!("Config file generated");
    } else {
        println!("Aborted");
    };

    Ok(())
}

pub fn install(parameters: &Parameters) {
    parameters
        .projects
        .iter()
        .filter(|(_, project)| !project.dir.exists() && !project.git.is_empty())
        .for_each(|(dir_name, project)| {
            git::clone(&project.git, &parameters.app.config.shared_dir);

            let mut project_dir = project.dir.clone();
            project_dir.push(dir_name);

            // combine the config from the new directory
            let app = config::combine(&None).expect("Could not construct config");
            let project = app.projects.get(dir_name).unwrap();

            docker::network(NetworkCmd::Create(
                parameters.app.config.shared_network.clone(),
            ));
            project.networks.iter().for_each(|network| {
                docker::network(NetworkCmd::Create(network.clone()));
            });

            project.volumes.iter().for_each(|volume| {
                docker::volume(VolumeCmd::Create(volume.clone()));
            });

            cmd::run_from_list(
                &project.on_install,
                &project.dir,
                "Could not run install command",
            );
        });
}

pub fn run_install(parameters: &Parameters) {
    if parameters.projects.is_empty() {
        panic!("Cannot run install on all projects")
    }

    parameters.projects.iter().for_each(|(_, project)| {
        docker::network(NetworkCmd::Create(
            parameters.app.config.shared_network.clone(),
        ));
        project.networks.iter().for_each(|network| {
            docker::network(NetworkCmd::Create(network.clone()));
        });

        project.volumes.iter().for_each(|volume| {
            docker::volume(VolumeCmd::Create(volume.clone()));
        });

        cmd::run_from_list(
            &project.on_install,
            &project.dir,
            "Could not run install command",
        );
    });
}

pub fn up(parameters: &Parameters) {
    install(parameters);

    let down_projects = statuses(parameters)
        .iter()
        .filter(|(_, services)| {
            services
                .iter()
                .any(|service| service.status == Status::Down)
        })
        .collect::<BTreeMap<_, _>>()
        .keys()
        .cloned()
        .cloned()
        .collect::<HashSet<String>>();

    docker::network(NetworkCmd::Create(
        parameters.app.config.shared_network.clone(),
    ));

    parameters
        .app
        .projects
        .iter()
        .filter(|(dir_name, _)| down_projects.contains(dir_name.clone()))
        .for_each(|(_, project)| {
            project.networks.iter().for_each(|network| {
                docker::network(NetworkCmd::Create(network.clone()));
            });

            project.volumes.iter().for_each(|volume| {
                docker::volume(VolumeCmd::Create(volume.clone()));
            });

            cmd::run_from_list(&project.on_up, &project.dir, "Could not run build command");

            docker::compose(
                ComposeCmd::Up(Vec::new(), project.compose_files.clone()),
                &project.dir,
            );

            cmd::run_from_list(
                &project.after_up,
                &project.dir,
                "Could not run after up hooks",
            );
        });
}

pub fn down(parameters: &Parameters) {
    parameters.projects.iter().for_each(|(_, project)| {
        docker::compose(ComposeCmd::Down, &project.dir);
    });
}

pub fn restart(parameters: &Parameters) {
    down(parameters);
    up(parameters);
}

pub fn status(parameters: &Parameters) {
    statuses(parameters)
        .iter()
        .for_each(|(dir_name, services)| {
            println!();
            println!(
                "{}",
                parameters
                    .app
                    .projects
                    .get(dir_name)
                    .map(|project| project.name())
                    .unwrap_or(dir_name.clone())
            );
            services.iter().for_each(|service| {
                println!("  {}  {}", service.status, service.name);
            });
        });
}

pub fn run(service: &String, command: &Vec<String>, project: &Project) {
    docker::compose(
        ComposeCmd::Run(
            service.clone(),
            project.compose_files.clone(),
            command.clone(),
        ),
        &project.dir,
    );
}

pub fn exec(service: &String, command: &Vec<String>, project: &Project) {
    docker::compose(
        ComposeCmd::Exec(
            service.clone(),
            project.compose_files.clone(),
            command.clone(),
        ),
        &project.dir,
    );
}

fn statuses(parameters: &Parameters) -> BTreeMap<ProjectDirName, Vec<Service>> {
    parameters
        .projects
        .iter()
        .map(|(dir_name, project)| {
            let mut statuses: BTreeMap<String, Service> = String::from_utf8(
                docker::compose(
                    ComposeCmd::List(project.compose_files.clone()),
                    &project.dir,
                )
                .stdout,
            )
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .map(|services: Vec<Service>| {
                services
                    .into_iter()
                    .map(|service| (service.name.clone(), service))
                    .collect()
            })
            .unwrap_or_default();

            let services: Vec<_> = docker::compose(
                ComposeCmd::Config(project.compose_files.clone()),
                &project.dir,
            )
            .stdout
            .as_slice()
            .lines()
            .flat_map(|line| line)
            .map(|service_name| {
                statuses.remove(&*service_name).unwrap_or(Service {
                    name: service_name,
                    status: Status::Down,
                })
            })
            .collect();

            (dir_name.clone(), services)
        })
        .collect()
}

fn set_git_config(project: &mut Project, dir_name: &str) {
    let default_repo_string = format!("https://git@github.com:name/{}.git", dir_name.to_string());
    project.git = git::config(ConfigCmd::Get("remote.origin.url".to_string()), &cwd())
        .stdout
        .as_slice()
        .lines()
        .filter_map(|x| x.ok())
        .fold(default_repo_string, |acc: String, line| {
            match line.is_empty() {
                true => acc,
                false => line,
            }
        });
}
