use anyhow::{bail, Result};
use std::{
    fmt,
    io::{BufRead, BufReader, Write},
    process::{Command, ExitStatus, Stdio},
};

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Basic(String),
    Verbose(String),
    Forget(String),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Basic(cmd) | Type::Verbose(cmd) => write!(f, "{}", cmd),
            Type::Forget(cmd) => write!(f, "{}", cmd),
        }
    }
}

#[macro_export]
macro_rules! cmd_basic {
    ($($arg:tt)*) => {
        Type::Basic(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! cmd_verbose {
    ($($arg:tt)*) => {
        Type::Verbose(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! cmd_forget {
    ($($arg:tt)*) => {
        Type::Forget(format!($($arg)*))
    };
}

const PROMPT_CHAR: &str = "❯";

pub(crate) trait Cmd<T> {
    fn run(&self, cmd: &Type) -> Result<T>;
}

#[derive(Clone, Debug)]
pub(crate) struct ShellRunner;

impl Cmd<()> for ShellRunner {
    fn run(&self, cmd: &Type) -> Result<()> {
        let (_, status) = self.run(&cmd)?;

        if status.success() {
            Ok(())
        } else {
            bail!("Command failed: {}", cmd)
        }
    }
}

impl Cmd<String> for ShellRunner {
    fn run(&self, cmd: &Type) -> Result<String> {
        let (output, status) = self.run(&cmd)?;

        if status.success() {
            Ok(output)
        } else {
            bail!("Command failed: {}", cmd)
        }
    }
}

impl Cmd<bool> for ShellRunner {
    fn run(&self, cmd: &Type) -> Result<bool> {
        let (_, status) = self.run(&cmd)?;

        Ok(status.success())
    }
}

impl ShellRunner {
    pub(crate) fn new() -> Self {
        Self {}
    }

    fn run(&self, cmd: &Type) -> Result<(String, ExitStatus)> {
        let (command_string, is_verbose, should_wait) = match cmd {
            Type::Basic(c) => (c, false, true),
            Type::Verbose(c) => (c, true, true),
            Type::Forget(c) => (c, true, false),
        };

        log::trace!("{}", &command_string);

        if !should_wait {
            let status = Command::new("sh").arg("-c").arg(&command_string).status()?;
            return Ok((String::new(), status));
        }

        if is_verbose {
            println!("{} {}", &PROMPT_CHAR, &command_string);
        }

        let mut command = Command::new("sh")
            .arg("-c")
            .arg(&command_string)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut buffer = Vec::new();
        if let Some(o) = command.stdout.take() {
            let reader = BufReader::new(o);

            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if is_verbose {
                            println!("{}", line);
                        }
                        writeln!(buffer, "{}", line)?;
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
        }

        let status = command.wait()?;
        let output = String::from_utf8(buffer)?;
        Ok((output.trim().to_string(), status))
    }
}

pub(crate) trait Runner: Cmd<()> + Cmd<String> + Cmd<bool> + Clone {}

impl Runner for ShellRunner {}

#[cfg(test)]
pub mod test;
