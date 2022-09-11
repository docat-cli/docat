use crate::config::app::App;
use crate::config::config::Config;
use crate::file::{cached_config, cached_config_file, CONFIG_FILENAME};
use crate::git::ConfigCmd;
use crate::service::{Service, Status};
use crate::{cmd, config, cwd, docker, git, ComposeCmd, NetworkCmd, ProjectDirName, VolumeCmd};
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

    let cached_config = &mut config::load_from(&cached_config()).or_else(|_| {
        // create a new cached config if it doesn't already exist
        let mut cached_config = Config::new();
        let app = cached_config.add_app(&app_name);
        app.config.init(&cwd());

        Result::<Config>::Ok(cached_config)
    })?;

    if !cached_config.apps.contains_key(&app_name) {
        let app = cached_config.add_app(&app_name);
        app.config.init(&cwd());
    }

    // generate new project config
    let mut new_config = Config::new();
    let config_filename = cwd();
    let dir_name = config_filename.file_name().unwrap().to_str().unwrap();
    let app = new_config.add_app(&app_name);
    app.config.shared_network = app_name.clone();
    let project = app.add_project(dir_name);
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

pub fn install(app: &App, projects: Vec<ProjectDirName>) {
    app.filter_projects(projects, false)
        .into_iter()
        .filter(|(_, project)| !project.dir.exists() && !project.git.is_empty())
        .for_each(|(_, project)| {
            git::clone(&project.git, &app.config.shared_dir);

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

pub fn run_install(app: &App, projects: Vec<ProjectDirName>) {
    if projects.is_empty() {
        panic!("Must provide projects to run install steps on")
    }

    app.filter_projects(projects, false)
        .into_iter()
        .for_each(|(_, project)| {
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

pub fn up(app: &App, projects: Vec<ProjectDirName>) {
    install(app, projects.clone());

    let down_projects = statuses(app, projects)
        .into_iter()
        .filter(|(_, services)| {
            services
                .iter()
                .any(|service| service.status == Status::Down)
        })
        .collect::<BTreeMap<_, _>>()
        .into_keys()
        .collect::<HashSet<String>>();

    docker::network(NetworkCmd::Create(app.config.shared_network.clone()));

    app.projects
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
        });
}

pub fn down(app: &App, projects: Vec<String>) {
    let is_empty = projects.is_empty();
    app.filter_projects(projects, is_empty)
        .into_iter()
        .for_each(|(_, project)| {
            docker::compose(ComposeCmd::Down, &project.dir);
        });
}

pub fn restart(app: &App, projects: Vec<String>) {
    down(app, projects.clone());
    up(app, projects);
}

pub fn status(app: &App, projects: Vec<String>) {
    statuses(app, projects)
        .into_iter()
        .for_each(|(dir_name, services)| {
            println!();
            println!(
                "{}",
                app.projects
                    .get(&dir_name)
                    .map(|project| project.name.clone().unwrap_or(dir_name.clone()))
                    .unwrap_or(dir_name.clone())
            );
            services.iter().for_each(|service| {
                println!("  {}  {}", service.status, service.name);
            });
        });
}

fn statuses(app: &App, projects: Vec<ProjectDirName>) -> BTreeMap<ProjectDirName, Vec<Service>> {
    app.filter_projects(projects, true)
        .into_iter()
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
