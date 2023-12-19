use std::{process::exit, rc::Rc};

use anyhow::{Error, Result};
use clap::{Parser, Subcommand};

use crate::app::{cmd::SystemCmdRunner, manager::session::SessionManager};

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start new session
    Start {
        /// Name of the configuration.
        name: Option<String>,

        /// Specify the config file to use.
        #[clap(short, long, default_value = ".laio.yaml")]
        file: String,
    },

    /// Stop session.
    Stop {
        /// Name of the session to stop.
        name: Option<String>,
    },

    Config(super::config::cli::Cli),
    Session(super::session::cli::Cli),
    Completion(super::completion::Cli),
}

#[derive(Debug, Parser)]
#[command(name = "laio")]
#[command(author = "Christian Kemper <christian.kemper@me.com")]
#[command(version = concat!("v", env!("CARGO_PKG_VERSION")))]
#[command(about = "A simple flexbox-like layout manager for tmux.")]
pub(crate) struct Cli {
    #[command(subcommand)]
    commands: Commands,

    #[arg[long, default_value = "~/.config/laio", global=true]]
    pub config_dir: String,

    #[clap(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        let res = match &self.commands {
            Commands::Start { name, file } => self.session().start(name, file),
            Commands::Stop { name } => self.session().stop(name),
            Commands::Config(cli) => cli.run(&self.config_dir),
            Commands::Session(cli) => cli.run(&self.config_dir),
            Commands::Completion(cli) => cli.run(),
        };

        if let Err(e) = res {
            self.handle_error(&e);
            exit(1);
        }

        res
    }

    fn session(&self) -> SessionManager<SystemCmdRunner> {
        SessionManager::new(&self.config_dir, Rc::new(SystemCmdRunner::new()))
    }

    fn handle_error(&self, error: &Error) {
        log::error!("{}", error);
        if let Commands::Start { name, .. } = &self.commands {
            if let Some(n) = name {
                log::error!("Shutting down session: {}", n);
                let _ = self.session().stop(name);
            } else {
                log::error!("Something went wrong, no tmux session to shut down!");
            }
        }
    }
}
