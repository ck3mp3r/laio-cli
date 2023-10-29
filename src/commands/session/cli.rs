use std::{process::exit, rc::Rc};

use crate::app::{cmd::SystemCmdRunner, manager::session::SessionManager};

use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Debug, Subcommand, Clone)]
pub(crate) enum Commands {
    /// List all active sessions.
    #[clap(alias = "ls")]
    List,

    /// Start new session
    Start {
        /// Name of the configuration.
        name: Option<String>,

        /// Specify the config file to use.
        #[clap(short, long, default_value = ".rmx.yaml")]
        file: String,

        /// Attach to session after creation.
        #[clap(short, long)]
        attach: bool,
    },

    /// Stop session.
    Stop {
        /// Name of the session to stop.
        name: Option<String>,
    },

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
    pub fn run(&self, config_path: &String) -> Result<()> {
        let session = SessionManager::new(config_path, Rc::clone(&Rc::new(SystemCmdRunner::new())));
        let res = match &self.commands {
            Commands::Start { name, file, attach } => session.start(&name, &file, &attach),
            Commands::Stop { name } => session.stop(&name),
            Commands::List => session.list(),
            Commands::Yaml => session.to_yaml(),
        };

        if let Err(e) = res {
            log::error!("{}", e);
            match &self.commands {
                Commands::Start { name, .. } => match &name {
                    Some(n) => {
                        log::error!("Shutting down session: {}", n);
                        let _ = session.stop(&name);
                    }
                    None => {
                        log::error!("Something went wrong, no tmux session to shut down!");
                    }
                },
                _ => {}
            }
            exit(1);
        }
        res
    }
}
