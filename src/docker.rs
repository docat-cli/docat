use crate::{cmd, file};
use std::path::PathBuf;
use std::process::{Command, Output};

pub enum NetworkCmd {
    Create(String),
}

pub enum VolumeCmd {
    Create(String),
}

pub enum ComposeCmd {
    Up(Vec<String>, Vec<String>),
    Config(Vec<String>),
    List(Vec<String>),
    Down,
    Exec(String, Vec<String>, Vec<String>),
}

pub fn network(subcommand: NetworkCmd) -> Output {
    let mut cmd_wrapper = cmd::new("docker", &file::cwd());
    cmd_wrapper.ignore_error = true;
    let cmd = &mut cmd_wrapper.command;
    cmd.arg("network");

    match subcommand {
        NetworkCmd::Create(name) => cmd.arg("create").arg(name),
    };

    cmd::run(cmd_wrapper).expect("Could not create docker network")
}

pub fn volume(subcommand: VolumeCmd) -> Output {
    let mut cmd_wrapper = cmd::new("docker", &file::cwd());
    cmd_wrapper.ignore_output = true;
    let cmd = &mut cmd_wrapper.command;
    cmd.arg("volume");

    match subcommand {
        VolumeCmd::Create(volume) => cmd.args(["create", &volume[..]]),
    };

    cmd::run(cmd_wrapper).expect("Could not create docker volume")
}

pub fn compose(subcommand: ComposeCmd, dir: &PathBuf) -> Output {
    let mut cmd_wrapper = cmd::new("docker", dir);
    let cmd = &mut cmd_wrapper.command;
    cmd.arg("compose");

    match subcommand {
        ComposeCmd::Up(services, files) => {
            add_files(cmd, files);
            cmd.args(["up", "-d"]).args(services)
        }
        ComposeCmd::Down => cmd.arg("down"),
        ComposeCmd::Config(files) => {
            cmd_wrapper.ignore_output = true;
            add_files(cmd, files);
            cmd.args(["config", "--services"])
        }
        ComposeCmd::List(files) => {
            cmd_wrapper.ignore_output = true;
            add_files(cmd, files);
            cmd.args(["ps", "--format", "json"])
        }
        ComposeCmd::Exec(service, files, command_string) => {
            add_files(cmd, files);
            cmd.args(["exec", "-T"]).arg(service).args(command_string)
        }
    };

    cmd::run(cmd_wrapper).expect("Could not start project")
}

pub fn add_files(cmd: &mut Command, files: Vec<String>) {
    files.iter().for_each(|file| {
        cmd.arg("-f").arg(file);
    });
}
