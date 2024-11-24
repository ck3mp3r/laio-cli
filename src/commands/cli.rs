use std::{process::exit, rc::Rc};

use anyhow::{Error, Ok, Result};
use clap::{Parser, Subcommand};

use crate::app::{
    cmd::ShellRunner,
    manager::{config::ConfigManager, session::SessionManager},
    tmux::Client,
};

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start new session
    Start {
        /// Name of the configuration.
        name: Option<String>,

        /// Specify the config file to use.
        #[clap(short, long)]
        file: Option<String>,

        /// Show config picker
        #[clap(short = 'p', long)]
        show_picker: bool,

        /// Skip the startup commands
        #[clap(long)]
        skip_cmds: bool,

        /// Skip attaching to session
        #[clap(long)]
        skip_attach: bool,
    },

    /// Stop session.
    Stop {
        /// Name of the session to stop.
        name: Option<String>,

        /// Skip the shutdown commands
        #[clap(long)]
        skip_cmds: bool,

        /// Stop all laio managed sessions
        #[clap(short, long)]
        all: bool,
    },

    /// List active (*) and available sessions
    #[clap(alias = "ls")]
    List,

    Config(super::config::cli::Cli),
    Session(super::session::cli::Cli),
    Completion(super::completion::Cli),
}

#[derive(Debug, Parser)]
#[command(name = "laio")]
#[command(author = "Christian Kemper <christian.kemper@me.com")]
#[command(version = concat!("v", env!("CARGO_PKG_VERSION")))]
#[command(about = "A simple flexbox-like layout manager for tmux.")]
pub struct Cli {
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
            Commands::Start {
                name,
                file,
                show_picker,
                skip_cmds,
                skip_attach,
            } => self
                .session()
                .start(name, file, *show_picker, *skip_cmds, *skip_attach),
            Commands::Stop {
                name,
                skip_cmds: skip_shutdown_cmds,
                all: stop_all,
            } => self.session().stop(name, *skip_shutdown_cmds, *stop_all),
            Commands::List => {
                let session: Vec<String> = self.session().list()?;
                let config: Vec<String> = self.config().list()?;

                // Merge and deduplicate
                let mut merged: Vec<String> = session.iter().map(|s| s.to_string()).collect();
                merged.extend(config.iter().map(|s| s.to_string()));
                merged.sort();
                merged.dedup();
                for item in &merged {
                    if session.contains(item) {
                        println!("{} *", item);
                    } else {
                        println!("{}", item);
                    }
                }
                Ok(())
            }
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

    fn session(&self) -> SessionManager<ShellRunner> {
        SessionManager::new(&self.config_dir, Client::new(Rc::new(ShellRunner::new())))
    }

    fn config(&self) -> ConfigManager<ShellRunner> {
        ConfigManager::new(&self.config_dir, Rc::new(ShellRunner::new()))
    }

    fn handle_error(&self, error: &Error) {
        println!();
        println!("⣶⣶⣦⠀⠀⠀⣰⣷⣦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣴⣾⣆⠀⠀⠀⣴⣶⣶");
        println!("⠻⣿⣿⡀⠀⢠⣿⣿⠏⠀⠀⢀⠀⢤⣴⣆⠀⠀⠀⠹⣿⣿⡄⠀⢀⣿⣿⠟");
        println!("⠀⢿⣿⣧⠀⢸⣿⡟⠀⠸⣿⡿⠄⠘⠋⠉⣠⣤⣄⠀⢻⣿⡇⠀⣼⣿⡿⠀");
        println!("⠀⠸⣿⣿⡀⢸⣿⣇⠀⠀⠁⠀⠀⠀⠀⣠⣿⣿⠇⠀⣸⣿⡇⢀⣿⣿⠇⠀");
        println!("⠀⠀⣿⣿⣇⣸⣿⣿⡀⠀⠀⠀⢀⣤⣾⣿⡿⠋⠀⢀⣿⣿⣇⣸⣿⣿⠀⠀");
        println!("⠀⠀⠸⣿⣿⣿⣿⣿⣷⡀⠀⠀⠘⡿⠟⠋⠀⠀⢀⣾⣿⣿⣿⣿⣿⠇⠀⠀");
        println!("⠀⠀⠀⠀⠀⠀⠀⠻⡿⠋⠀⠀⠀⠀⠀⠀⠀⠀⠙⢿⠟⠀⠀⠀⠀⠀⠀⠀");
        println!();
        println!("{}", error);
        println!();
        if let Commands::Start { name, .. } = &self.commands {
            if let Some(n) = name {
                log::warn!("Shutting down session: {}", n);
                let _ = self.session().stop(name, true, false);
            } else {
                log::warn!("No tmux session to shut down!");
            }
        }
    }
}
