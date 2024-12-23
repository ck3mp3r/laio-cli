use std::env::{self, current_dir};

use anyhow::anyhow;
use anyhow::Result;

use crate::cmd_verbose;
use crate::common::cmd::Runner;
use crate::common::cmd::Type;
use crate::common::config::Command;
use crate::common::path::to_absolute_path;

pub(crate) trait Client<R: Runner> {
    fn get_runner(&self) -> &R;

    fn run_commands(&self, commands: &[Command], cwd: &String) -> Result<()> {
        if commands.is_empty() {
            log::error!("No commands to run...");
            return Ok(());
        }

        log::info!("Running commands...");

        // Save the current directory to restore it later
        let current_dir = current_dir()?;

        log::trace!("Current directory: {:?}", current_dir);
        log::trace!("Changing to: {:?}", cwd);

        // Use to_absolute_path to handle the session path
        env::set_current_dir(to_absolute_path(cwd)?)
            .map_err(|_| anyhow!("Unable to change to directory: {:?}", &cwd))?;

        // Run each command
        for cmd in commands {
            let _res: String = self
                .get_runner()
                .run(&cmd_verbose!("{}", cmd.to_string()))
                .map_err(|_| anyhow!("Failed to run command: {}", cmd.to_string()))?;
        }

        // Restore the original directory
        env::set_current_dir(&current_dir)
            .map_err(|_| anyhow!("Failed to restore original directory {:?}", current_dir))?;

        log::info!("Completed commands.");

        Ok(())
    }
}
