extern crate core;

mod cmd;
mod config;
mod docat;
mod docker;
mod file;
mod git;
mod service;

use crate::docker::{ComposeCmd, NetworkCmd, VolumeCmd};
use crate::file::cwd;
use anyhow::Result;
use clap::Parser;
use config::app::App;
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
}

fn get_app(args: &Args) -> Result<App> {
    config::combine(&args.app)
}

fn main() -> Result<()> {
    let args = Args::parse();

    fs::create_dir_all(file::cached_config())?;

    match args.clone().command {
        Command::Init { app } => docat::init(app)?,
        Command::Install { projects } => docat::install(&get_app(&args)?, projects),
        Command::RunInstall { projects } => docat::run_install(&get_app(&args)?, projects),
        Command::Up { projects } => docat::up(&get_app(&args)?, projects),
        Command::Down { projects } => docat::down(&get_app(&args)?, projects),
        Command::Restart { projects } => docat::restart(&get_app(&args)?, projects),
        Command::Status { projects } => docat::status(&get_app(&args)?, projects),
    };

    Ok(())
}
