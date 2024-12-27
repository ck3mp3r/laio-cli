use super::Cmd;
use super::Runner;
use super::Type;
use anyhow::{bail, Result};
use std::{
    fmt,
    io::{BufRead, BufReader, Write},
    process::{Command, ExitStatus, Stdio},
};

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Type::Basic(cmd) | Type::Verbose(cmd) | Type::Forget(cmd) => cmd,
            },
        )
    }
}
const PROMPT_CHAR: &str = "❯";

#[derive(Clone, Debug)]
pub(crate) struct ShellRunner;

impl Runner for ShellRunner {}

impl Cmd<()> for ShellRunner {
    fn run(&self, cmd: &Type) -> Result<()> {
        let (output, status) = self.run(cmd)?;

        if status.success() {
            log::trace!("Result:() {}", output);
            Ok(())
        } else {
            bail!("Command failed: {}", cmd)
        }
    }
}

impl Cmd<String> for ShellRunner {
    fn run(&self, cmd: &Type) -> Result<String> {
        let (output, status) = self.run(cmd)?;

        if status.success() {
            log::trace!("Result:<String> {}", output);
            Ok(output)
        } else {
            bail!("Command failed: {}", cmd)
        }
    }
}

impl Cmd<bool> for ShellRunner {
    fn run(&self, cmd: &Type) -> Result<bool> {
        let (output, status) = self.run(cmd)?;

        log::trace!("Result:<bool> {}", output);
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
            let status = Command::new("sh").arg("-c").arg(command_string).status()?;
            return Ok((String::new(), status));
        }

        if is_verbose {
            println!("{} {}", &PROMPT_CHAR, &command_string);
        }

        let mut command = Command::new("sh")
            .arg("-c")
            .arg(command_string)
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
        log::trace!("Result: {}", output);
        Ok((output.trim().to_string(), status))
    }
}
