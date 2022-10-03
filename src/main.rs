extern crate core;

mod cmd;
mod config;
mod docat;
mod docker;
mod file;
mod git;
mod service;

use crate::config::project::Project;
use crate::docker::{ComposeCmd, NetworkCmd, VolumeCmd};
use crate::file::cwd;
use anyhow::{bail, Result};
use clap::Parser;
use config::app::App;
use std::collections::BTreeMap;
use std::fs;

type ProjectDirName = String;

/// Run commands on multiple docker compose projects at the same time
#[derive(clap::Parser, Clone)]
pub struct Args {
    #[clap(subcommand)]
    command: Command,

    /// Optionally run commands on a specific app
    #[clap(global = true, long, short)]
    app: Option<String>,

    /// Run on all projects
    #[clap(global = true, long, default_missing_value = "true")]
    all: Option<bool>,
}

#[derive(clap::Subcommand, Clone)]
enum Command {
    /// Generate a new config file
    Init {
        /// Specify the app projects will be tied to
        app: String,
    },
    /// Fetch the projects if they don't exist
    Install {
        /// List of projects to install
        projects: Vec<String>,
    },
    /// Re-run commands on install
    RunInstall {
        /// List of projects to install
        projects: Vec<String>,
    },
    /// Bring up projects
    Up {
        /// List of projects to bring up
        projects: Vec<String>,
    },
    /// Bring down projects
    Down {
        /// List of projects to bring down
        projects: Vec<String>,
    },
    /// Restart projects
    Restart {
        /// List of projects to restart
        projects: Vec<String>,
    },
    /// Get status for projects
    Status {
        /// List of projects to get status for
        projects: Vec<String>,
    },
    /// Start a new container without dependencies and run command
    Run {
        /// Specify which project to execute command on
        #[clap(global = true, long, short)]
        project: Option<String>,
        /// The service to run the command on
        service: String,
        /// The command to run on the service
        command: Vec<String>,
    },
    /// Run a command on a running container
    Exec {
        /// Specify which project to execute command on
        #[clap(global = true, long, short)]
        project: Option<String>,
        /// The service to run the command on
        service: String,
        /// The command to run on the service
        command: Vec<String>,
    },
}

#[derive(Clone)]
pub struct Parameters {
    pub app: App,
    pub projects: BTreeMap<ProjectDirName, Project>,
}

fn get_app(args: &Args) -> Result<App> {
    config::combine(&args.app)
}

fn main() -> Result<()> {
    let mut args = Args::parse();

    fs::create_dir_all(file::cached_config_path())?;

    match args.clone().command {
        Command::Init { app } => docat::init(app)?,
        Command::Install { projects } => docat::install(&get_parameters(&args, &projects, false)?),
        Command::RunInstall { projects } => {
            docat::run_install(&get_parameters(&args, &projects, false)?)
        }
        Command::Up { projects } => {
            docat::up(&get_parameters(&args, &projects, projects.is_empty())?)
        }
        Command::Down { projects } => {
            docat::down(&get_parameters(&args, &projects, projects.is_empty())?)
        }
        Command::Restart { projects } => {
            docat::restart(&get_parameters(&args, &projects, projects.is_empty())?)
        }
        Command::Status { projects } => {
            if projects.is_empty() {
                args.all = Some(true);
            }
            docat::status(&get_parameters(&args, &projects, false)?)
        }
        Command::Run {
            service,
            command,
            project,
        } => {
            docat::run(&service, &command, &get_project(&args, project)?);
        }
        Command::Exec {
            service,
            command,
            project,
        } => {
            docat::exec(&service, &command, &get_project(&args, project)?);
        }
    };

    Ok(())
}

fn get_parameters(
    args: &Args,
    project_names: &Vec<String>,
    include_install: bool,
) -> Result<Parameters> {
    let app = get_app(args)?;
    let projects = get_projects(&app, project_names, args.all, include_install)?;

    Ok(Parameters { app, projects })
}

fn get_projects(
    app: &App,
    project_names: &Vec<String>,
    all: Option<bool>,
    mut include_install: bool,
) -> Result<BTreeMap<ProjectDirName, Project>> {
    if all.unwrap_or(false) {
        if !project_names.is_empty() {
            bail!("--all flag is not compatible with a project list")
        }

        return Ok(app.projects.clone());
    }

    if !project_names.is_empty() {
        return filter_projects(&app, project_names, include_install);
    }

    let cwd = cwd();
    let mut projects = app
        .projects
        .iter()
        .filter(|(_, project)| project.dir == cwd)
        .map(|(_, project)| project.name())
        .collect::<Vec<String>>();

    if projects.is_empty() {
        projects = app
            .projects
            .values()
            .map(|project| project.name().clone())
            .collect();
    } else if include_install {
        // don't include install project if running in project directory
        include_install = false;
    }

    filter_projects(&app, &projects, include_install)
}

fn filter_projects(
    app: &App,
    projects: &Vec<String>,
    include_install: bool,
) -> Result<BTreeMap<ProjectDirName, Project>> {
    let mut cloned_projects = app.projects.clone();
    let mut projects = projects
        .into_iter()
        .filter_map(|dir_or_project_name| {
            cloned_projects
                .remove(dir_or_project_name)
                .or_else(|| {
                    app.projects
                        .clone()
                        .into_iter()
                        .find(|(_, project)| project.name().eq(dir_or_project_name))
                        .and_then(|(dir_name, _)| cloned_projects.remove(&*dir_name))
                })
                .map(|project| (project.dir_name.clone(), project))
        })
        .collect::<BTreeMap<_, _>>();

    // if an install project still exists in the projects, add it
    if include_install {
        cloned_projects
            .into_iter()
            .find(|(_, project)| project.is_install)
            .map(|(dir_name, project)| projects.insert(dir_name, project));
    }

    Ok(projects)
}

fn get_project(args: &Args, project_name: Option<String>) -> Result<Project> {
    let app = get_app(args)?;
    let dir = cwd();

    app.projects
        .iter()
        .find(|(_, project)| project.dir == dir)
        .map(|tuple| tuple.1.clone())
        .or_else(|| {
            app.projects
                .iter()
                .find(|(_, project)| {
                    project_name.is_some()
                        && (project.name == project_name
                            || project_name == Some(project.dir_name.clone()))
                })
                .map(|tuple| tuple.1.clone())
        })
        .ok_or(anyhow::anyhow!(
            "Could not determine project, consider passing the --project flag"
        ))
}
