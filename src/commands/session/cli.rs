use std::rc::Rc;

use crate::app::{cmd::ShellRunner, manager::session::SessionManager, tmux::Client};

use anyhow::{Ok, Result};
use clap::{Args, Subcommand};

#[derive(Debug, Subcommand, Clone)]
pub(crate) enum Commands {
    /// List all active sessions.
    #[clap(alias = "ls")]
    List,

    /// Shows current session layout as yaml.
    #[clap()]
    Yaml,
}

/// Manage Sessions
#[derive(Args, Debug)]
pub struct Cli {
    #[clap(subcommand)]
    commands: Commands,
}

impl Cli {
    pub fn run(&self, config_path: &str) -> Result<()> {
        let session = SessionManager::new(config_path, Client::new(Rc::new(ShellRunner::new())));

        match &self.commands {
            Commands::List => {
                let list = session.list()?;
                println!("{}", list.join("\n"));
                Ok({})
            }
            Commands::Yaml => {
                let yaml = session.to_yaml()?;
                println!("{}", yaml);
                Ok({})
            }
        }
    }
}
