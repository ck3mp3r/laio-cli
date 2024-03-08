use std::rc::Rc;

use crate::app::{cmd::SystemCmdRunner, manager::session::SessionManager};

use anyhow::{Result, Ok};
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
        let session = SessionManager::new(config_path, Rc::new(SystemCmdRunner::new()));

        match &self.commands {
            Commands::List => {
                let list = session.list()?;
                    println!("{}", list);
                    Ok({})
            },
            Commands::Yaml => {
               let yaml =  session.to_yaml()?;
                   println!("{}", yaml);
                    Ok({})
            },
        }
    }
}
