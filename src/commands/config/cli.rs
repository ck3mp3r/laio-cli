use std::rc::Rc;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::app::{cmd::SystemCmdRunner, manager::config::ConfigManager};

#[derive(Clone, Subcommand, Debug)]
pub enum Commands {
    /// Create new laio configuration.
    Create {
        /// Name of the new configuration. Omit to create local .laio.yaml
        name: Option<String>,

        /// Existing configuration to copy from.
        #[clap(short, long)]
        copy: Option<String>,
    },

    /// Edit laio configuration.
    Edit {
        /// Name of the configuration to edit.
        name: String,
    },

    /// Symlink local laio configuration to the laio config directory.
    Link {
        /// Name of the symlink.
        name: String,

        /// Specify the config file to use.
        #[clap(short, long, default_value = ".laio.yaml")]
        file: String,
    },

    /// Validate laio configuration
    Validate {
        /// Name of the configuration to validate, omit to validate local .laio.yaml.
        name: Option<String>,

        /// Specify the config file to use.
        #[clap(short, long, default_value = ".laio.yaml")]
        file: String,
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
    pub fn run(&self, config_path: &str) -> Result<()> {
        let cfg = ConfigManager::new(config_path, Rc::new(SystemCmdRunner::new()));

        match &self.commands {
            Commands::Create { name, copy } => cfg.create(name, copy),
            Commands::Edit { name } => cfg.edit(name),
            Commands::Link { name, file } => cfg.link(name, file),
            Commands::Validate { name, file } => cfg.validate(name, file),
            Commands::Delete { name, force } => cfg.delete(name, *force),
            Commands::List => {
                let list = cfg.list()?;
                println!("{}", list);
                    Ok({})
            },
        }
    }
}
