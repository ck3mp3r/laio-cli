use std::env::{self, current_dir};

use miette::Result;
use miette::{miette, IntoDiagnostic};

use crate::common::cmd::Runner;
use crate::common::cmd::Type;
use crate::common::config::Command as ConfigCommand;
use crate::common::path::to_absolute_path;

pub(crate) trait Client<R: Runner> {
    fn get_runner(&self) -> &R;

    fn run_commands(&self, commands: &[ConfigCommand], cwd: &str) -> Result<()> {
        if commands.is_empty() {
            log::info!("No commands to run...");
            return Ok(());
        }

        log::info!("Running commands...");

        let current_dir = current_dir().into_diagnostic()?;

        log::trace!("Current directory: {:?}", current_dir);
        log::trace!("Changing to: {:?}", cwd);

        env::set_current_dir(to_absolute_path(cwd)?)
            .map_err(|_| miette!("Unable to change to directory: {:?}", &cwd))?;

        for cmd in commands {
            let _res: String = self
                .get_runner()
                .run(&Type::Verbose(cmd.to_process_command()))
                .map_err(|_| miette!("Failed to run command: {}", cmd.to_string()))?;
        }

        env::set_current_dir(&current_dir)
            .map_err(|_| miette!("Failed to restore original directory {:?}", current_dir))?;

        log::info!("Completed commands.");

        Ok(())
    }
}
