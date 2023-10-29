use std::rc::Rc;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::{cmd::SystemCmdRunner, rmx::rmx::Rmx};

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
}

/// Manage Sessions
#[derive(Args, Debug)]
pub struct Cli {
    #[clap(subcommand)]
    commands: Commands,
}

impl Cli {
    pub fn run(&self, config_path: &String) -> Result<()> {
        let rmx = Rmx::new(config_path, Rc::clone(&Rc::new(SystemCmdRunner::new())));
        match &self.commands {
            Commands::Start { name, file, attach } => rmx.session_start(&name, &file, &attach),
            Commands::Stop { name } => rmx.session_stop(&name),
            Commands::List => rmx.session_list(),
        }
    }
}
