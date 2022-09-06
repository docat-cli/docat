use dirs::home_dir;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

pub struct CommandWrapper {
    pub command: Command,
    pub ignore_output: bool,
    pub ignore_error: bool,
}

pub fn new(program: &str, dir: &PathBuf) -> CommandWrapper {
    let mut cmd = Command::new(program);
    cmd.current_dir(dir);

    let cmd = CommandWrapper {
        command: cmd,
        ignore_output: false,
        ignore_error: false,
    };

    cmd
}

pub fn run(cmd_wrapper: CommandWrapper) -> std::io::Result<Output> {
    let mut cmd = cmd_wrapper.command;
    if !cmd_wrapper.ignore_output {
        cmd.stdout(Stdio::inherit());
    }

    if !cmd_wrapper.ignore_error {
        cmd.stderr(Stdio::inherit());
    }

    cmd.output()
}

pub fn run_from_list(cmds: &Vec<String>, dir: &PathBuf, message: &str) -> Vec<Output> {
    cmds.iter()
        .map(parse)
        .flat_map(|cmd_result| {
            cmd_result
                .map(|mut cmd| {
                    cmd.current_dir(dir);
                    cmd
                })
                .map(|cmd| CommandWrapper {
                    command: cmd,
                    ignore_output: false,
                    ignore_error: false,
                })
        })
        .map(run)
        .map(|result| result.expect(message))
        .collect()
}

fn parse(cmd_string: &String) -> Option<Command> {
    shellwords::split(cmd_string)
        .ok()
        .and_then(|cmds| match cmds.as_slice() {
            [] => None,
            [first, args @ ..] => Some((first.clone(), args.to_vec())),
        })
        .map(|(first, args)| {
            // look for $HOME var and replace with home directory
            let args = args
                .iter()
                .map(|arg| str::replace(arg, "$HOME", home_dir().unwrap().to_str().unwrap()))
                .collect::<Vec<String>>();
            let mut cmd = Command::new(first);
            cmd.args(args);
            cmd
        })
}
