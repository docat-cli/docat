use crate::cmd;
use std::path::PathBuf;
use std::process::Output;

pub enum ConfigCmd {
    Get(String),
}

pub fn clone(repository: &String, directory: &PathBuf) -> Output {
    let mut cmd_wrapper = cmd::new("git", directory);
    let cmd = &mut cmd_wrapper.command;
    cmd.arg("clone").arg(repository);

    cmd::run(cmd_wrapper).expect("Failed to clone repository.")
}

pub fn config(subcommand: ConfigCmd, directory: &PathBuf) -> Output {
    let mut cmd_wrapper = cmd::new("git", directory);
    cmd_wrapper.ignore_output = true;
    cmd_wrapper.ignore_error = true;
    let cmd = &mut cmd_wrapper.command;
    cmd.arg("config");

    match subcommand {
        ConfigCmd::Get(value) => cmd.arg("--get").arg(value),
    };

    cmd::run(cmd_wrapper).expect("Could get get git config")
}
