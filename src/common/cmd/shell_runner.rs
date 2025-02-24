use super::Cmd;
use super::Runner;
use super::Type;
use miette::miette;
use miette::Context;
use miette::IntoDiagnostic;
use miette::Result;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, ExitStatus, Stdio},
};

const PROMPT_CHAR: &str = "❯";

#[derive(Clone, Debug)]
pub(crate) struct ShellRunner;

impl Runner for ShellRunner {}

impl Cmd<()> for ShellRunner {
    fn run(&self, cmd: &Type) -> Result<()> {
        let (output, status) = self
            .run(cmd)
            .wrap_err_with(|| format!("Failed to execute: {}", cmd))?;

        if !status.success() {
            return Err(miette!(output)).wrap_err_with(|| cmd.to_string());
        }

        log::trace!("Result:() {}", output);
        Ok(())
    }
}

impl Cmd<String> for ShellRunner {
    fn run(&self, cmd: &Type) -> Result<String> {
        let (output, status) = self
            .run(cmd)
            .wrap_err_with(|| format!("Failed to execute: {}", cmd))?;

        if !status.success() {
            return Err(miette!(output)).wrap_err_with(|| cmd.to_string());
        }

        log::trace!("Result:<String> {}", output);
        Ok(output)
    }
}

impl Cmd<bool> for ShellRunner {
    fn run(&self, cmd: &Type) -> Result<bool> {
        let (output, status) = self
            .run(cmd)
            .wrap_err_with(|| format!("Failed to execute: {}", cmd))?;

        if !status.success() {
            return Err(miette!(output)).wrap_err_with(|| cmd.to_string());
        }

        log::trace!("Result:<bool> {}", output);
        Ok(true)
    }
}

impl ShellRunner {
    pub(crate) fn new() -> Self {
        Self {}
    }

    fn run(&self, cmd: &Type) -> Result<(String, ExitStatus)> {
        let (oc, is_verbose, should_wait) = match cmd {
            Type::Basic(c) => (c, false, true),
            Type::Verbose(c) => (c, true, true),
            Type::Forget(c) => (c, true, false),
        };

        if is_verbose {
            println!("{} {:?}", PROMPT_CHAR, oc);
        }

        log::debug!("Running: {}", &cmd.to_string());

        let mut command = Command::new(oc.get_program());
        command.args(oc.get_args()).stderr(Stdio::piped());

        if should_wait {
            command.stdout(Stdio::piped());
        }

        let mut child = command.spawn().into_diagnostic()?;
        let mut stderr_buffer = Vec::new();

        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            reader.lines().try_for_each(|line| match line {
                Ok(line) => writeln!(stderr_buffer, "{}", line).into_diagnostic(),
                Err(e) => {
                    log::error!("Failed to read stderr: {}", e);
                    Ok(())
                }
            })?;
        }

        if !should_wait {
            return Ok((String::new(), ExitStatus::default()));
        }

        let mut stdout_buffer = Vec::new();
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            reader.lines().try_for_each(|line| match line {
                Ok(line) => {
                    if is_verbose {
                        println!("{}", line);
                    }
                    writeln!(stdout_buffer, "{}", line).into_diagnostic()
                }

                Err(e) => {
                    log::error!("Failed to read stderr: {}", e);
                    Ok(())
                }
            })?;
        }

        let status = child.wait().into_diagnostic()?;
        let stdout_output = String::from_utf8(stdout_buffer).into_diagnostic()?;
        let stderr_output = String::from_utf8(stderr_buffer).into_diagnostic()?;

        let final_output = if status.success() {
            stdout_output.trim().to_string()
        } else {
            stderr_output.trim().to_string()
        };

        log::trace!("Command result: {}", final_output);
        Ok((final_output, status))
    }
}
