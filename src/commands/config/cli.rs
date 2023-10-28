use std::rc::Rc;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::cmd::SystemCmdRunner;
use crate::rmx::rmx::Rmx;

#[derive(Clone, Subcommand, Debug)]
pub enum Commands {
    /// Create new rmx configuration.
    New {
        /// Name of the new configuration.
        name: String,

        /// Existing configuration to copy from.
        #[clap(short, long)]
        copy: Option<String>,

        /// Copy to PWD
        #[clap(short, long)]
        pwd: bool,
    },

    /// Edit rmx configuration.
    Edit {
        /// Name of the configuration to edit.
        name: String,
    },

    /// Delete rmx configuration.
    #[clap(alias = "rm")]
    Delete {
        /// Name of the configuration to delete.
        name: String,

        /// Force delete, no prompt.
        #[clap(short, long)]
        force: bool,
    },

    /// List all rmx configurations.
    #[clap(alias = "ls")]
    List,

    /// Shows current session layout as yaml.
    #[clap()]
    Yaml,
}

#[derive(Args, Debug)]
#[command()]
pub struct Cli {
    #[clap(subcommand)]
    commands: Commands,
}

impl Cli {
    pub fn run(&self, config_path: &String) -> Result<()> {
        let rmx = Rmx::new(config_path, Rc::clone(&Rc::new(SystemCmdRunner::new())));
        match &self.commands {
            Commands::New { name, copy, pwd } => rmx.config_new(&name, &copy, &pwd),
            Commands::Edit { name } => rmx.config_edit(&name),
            Commands::Delete { name, force } => rmx.config_delete(&name, &force),
            Commands::List => rmx.config_list(),
            Commands::Yaml => rmx.session_to_yaml(),
        }
    }
}
