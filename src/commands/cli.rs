use std::{process::exit, rc::Rc};

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::app::{cmd::SystemCmdRunner, manager::session::SessionManager};

#[derive(Subcommand, Debug)]
enum Commands {
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

    Config(super::config::cli::Cli),
    Session(super::session::cli::Cli),
    Complete(super::complete::Cli),
}

#[derive(Debug, Parser)]
#[command(name = "rmx")]
#[command(author = "Christian Kemper <christian.kemper@me.com")]
#[command(version = concat!("v", env!("CARGO_PKG_VERSION")))]
#[command(about = "A simple flexbox-like layout manager for tmux.")]
pub(crate) struct Cli {
    #[command(subcommand)]
    commands: Commands,

    #[arg[long, default_value = "~/.config/rmx", global=true]]
    pub config_dir: String,

    #[clap(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        let res = match &self.commands {
            Commands::Start { name, file, attach } => self.session().start(&name, &file, &attach),
            Commands::Stop { name } => self.session().stop(&name),
            Commands::Config(cli) => cli.run(&self.config_dir),
            Commands::Session(cli) => cli.run(&self.config_dir),
            Commands::Complete(cli) => cli.run(),
        };

        if let Err(e) = res {
            log::error!("{}", e);
            match &self.commands {
                Commands::Start { name, .. } => match &name {
                    Some(n) => {
                        log::error!("Shutting down session: {}", n);
                        let _ = self.session().stop(&name);
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

    fn session(&self) -> SessionManager<SystemCmdRunner> {
        let session = SessionManager::new(
            &self.config_dir,
            Rc::clone(&Rc::new(SystemCmdRunner::new())),
        );
        session
    }
}
