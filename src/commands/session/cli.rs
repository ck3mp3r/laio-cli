use std::rc::Rc;

use crate::app::{cmd::SystemCmdRunner, manager::session::SessionManager};

use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Debug, Subcommand, Clone)]
pub(crate) enum Commands {
    /// List all active sessions.
    #[clap(alias = "ls")]
    List,

    /// Shows current session layout as yaml.
    #[clap()]
    Yaml,

    /// Shows current session layout as toml.
    #[clap()]
    Toml,
}

/// Manage Sessions
#[derive(Args, Debug)]
pub struct Cli {
    #[clap(subcommand)]
    commands: Commands,
}

impl Cli {
    pub fn run(&self, config_path: &str) -> Result<()> {
        let session = SessionManager::new(config_path, Rc::new(SystemCmdRunner::new()));

        match &self.commands {
            Commands::List => session.list(),
            Commands::Yaml => session.to_yaml(),
            Commands::Toml => session.to_toml(),
        }
    }
}
