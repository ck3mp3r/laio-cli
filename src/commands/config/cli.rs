use std::rc::Rc;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::app::{cmd::SystemCmdRunner, manager::config::ConfigManager};

#[derive(Clone, Subcommand, Debug)]
pub enum Commands {
    /// Create new laio configuration.
    Create {
        /// Name of the new configuration.
        name: String,

        /// Existing configuration to copy from.
        #[clap(short, long)]
        copy: Option<String>,

        /// Copy to PWD
        #[clap(short, long)]
        pwd: bool,
    },

    /// Edit laio configuration.
    Edit {
        /// Name of the configuration to edit.
        name: String,
    },

    /// Delete laio configuration.
    #[clap(alias = "rm")]
    Delete {
        /// Name of the configuration to delete.
        name: String,

        /// Force delete, no prompt.
        #[clap(short, long)]
        force: bool,
    },

    /// List all laio configurations.
    #[clap(alias = "ls")]
    List,
}

/// Manage Configurations
#[derive(Args, Debug)]
#[command()]
pub struct Cli {
    #[clap(subcommand)]
    commands: Commands,
}

impl Cli {
    pub fn run(&self, config_path: &String) -> Result<()> {
        let cfg = ConfigManager::new(config_path, Rc::clone(&Rc::new(SystemCmdRunner::new())));
        match &self.commands {
            Commands::Create { name, copy, pwd } => cfg.create(&name, &copy, pwd.to_owned()),
            Commands::Edit { name } => cfg.edit(&name),
            Commands::Delete { name, force } => cfg.delete(&name, &force),
            Commands::List => cfg.list(),
        }
    }
}
