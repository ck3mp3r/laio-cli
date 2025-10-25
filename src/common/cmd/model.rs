use std::{fmt, process::Command};

use miette::Result;

#[derive(Debug)]
pub(crate) enum Type {
    Basic(Command),
    Verbose(Command),
    Forget(Command),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Basic(cmd) | Type::Verbose(cmd) | Type::Forget(cmd) => {
                let envs: Vec<_> = cmd
                    .get_envs()
                    .filter_map(|(key, value)| {
                        value.map(|v| format!("{}={}", key.to_string_lossy(), v.to_string_lossy()))
                    })
                    .collect();
                let args: Vec<_> = cmd.get_args().map(|arg| arg.to_string_lossy()).collect();
                let cmd_str = if args.is_empty() {
                    cmd.get_program().to_string_lossy().to_string()
                } else {
                    format!("{} {}", cmd.get_program().to_string_lossy(), args.join(" "))
                };
                if envs.is_empty() {
                    write!(f, "{}", cmd_str)
                } else {
                    write!(f, "{} {}", envs.join(" "), cmd_str)
                }
            }
        }
    }
}

pub(crate) trait Runner: Cmd<()> + Cmd<String> + Cmd<bool> + Clone {}

pub(crate) trait Cmd<T> {
    fn run(&self, cmd: &Type) -> Result<T>;
}

#[macro_export]
macro_rules! cmd_forget {
    ($cmd:expr $(, args = [$($args:expr),*])? $(, env = [$($key:expr => $val:expr),*])?) => {
        Type::Forget({
            let mut command = std::process::Command::new($cmd);
            $( $(command.arg($args);)* )?
            $( $(command.env($key, $val);)* )?
            command
        })
    };
}

#[macro_export]
macro_rules! cmd_basic {
    ($cmd:expr $(, args = [$($args:expr),*])? $(, env = [$($key:expr => $val:expr),*])?) => {
        Type::Basic({
            let mut command = std::process::Command::new($cmd);
            $( $(command.arg($args);)* )?
            $( $(command.env($key, $val);)* )?
            command
        })
    };
}

#[macro_export]
macro_rules! cmd_verbose {
    ($cmd:expr $(, args = [$($args:expr),*])? $(, env = [$($key:expr => $val:expr),*])?) => {
        Type::Verbose({
            let mut command = std::process::Command::new($cmd);
            $( $(command.arg($args);)* )?
            $( $(command.env($key, $val);)* )?
            command
        })
    };
}


