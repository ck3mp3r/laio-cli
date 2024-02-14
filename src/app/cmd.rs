use anyhow::{bail, Result};
use std::{
    fmt,
    io::BufRead,
    io::BufReader,
    io::Write,
    process::{Command, ExitStatus, Stdio},
};

#[derive(Clone, Debug, PartialEq)]
pub enum CommandType {
    Basic(String),
    Verbose(String),
    Forget(String),
}

impl fmt::Display for CommandType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandType::Basic(cmd) | CommandType::Verbose(cmd) => write!(f, "{}", cmd),
            CommandType::Forget(cmd) => write!(f, "{}", cmd),
        }
    }
}

#[macro_export]
macro_rules! cmd_basic {
    ($($arg:tt)*) => {
        CommandType::Basic(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! cmd_verbose {
    ($($arg:tt)*) => {
        CommandType::Verbose(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! cmd_forget {
    ($($arg:tt)*) => {
        CommandType::Forget(format!($($arg)*))
    };
}

const PROMPT_CHAR: &str = "❯";

pub(crate) trait Cmd<T> {
    fn run(&self, cmd: &CommandType) -> Result<T>;
}

#[derive(Clone, Debug)]
pub(crate) struct SystemCmdRunner;

impl Cmd<()> for SystemCmdRunner {
    fn run(&self, cmd: &CommandType) -> Result<()> {
        let (_, status) = self.run(&cmd)?;

        if status.success() {
            Ok(())
        } else {
            bail!("Command failed: {}", cmd)
        }
    }
}

impl Cmd<String> for SystemCmdRunner {
    fn run(&self, cmd: &CommandType) -> Result<String> {
        let (output, status) = self.run(&cmd)?;

        if status.success() {
            Ok(output)
        } else {
            bail!("Command failed: {}", cmd)
        }
    }
}

impl Cmd<bool> for SystemCmdRunner {
    fn run(&self, cmd: &CommandType) -> Result<bool> {
        let (_, status) = self.run(&cmd)?;

        Ok(status.success())
    }
}

impl SystemCmdRunner {
    pub(crate) fn new() -> Self {
        Self {}
    }

    fn run(&self, cmd: &CommandType) -> Result<(String, ExitStatus)> {
        let (command_string, is_verbose, should_wait) = match cmd {
            CommandType::Basic(c) => (c, false, true),
            CommandType::Verbose(c) => (c, true, true),
            CommandType::Forget(c) => (c, true, false),
        };

        log::debug!("{}", &command_string);

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

pub(crate) trait CmdRunner: Cmd<()> + Cmd<String> + Cmd<bool> + Clone {}

impl CmdRunner for SystemCmdRunner {}
