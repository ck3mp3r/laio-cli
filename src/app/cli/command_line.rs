use std::{fs::create_dir_all, process::exit, rc::Rc};

use clap::{Parser, Subcommand};
use miette::{Context, Error, IntoDiagnostic, Result};

use crate::{
    app::{ConfigManager, SessionManager},
    common::{cmd::ShellRunner, path::to_absolute_path},
    muxer::{create_muxer, Muxer},
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

        /// Specify the multiplexer to use.
        #[clap(short, long)]
        muxer: Option<Muxer>,

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

        /// Specify the multiplexer to use.
        #[clap(short, long)]
        muxer: Option<Muxer>,

        /// Skip the shutdown commands
        #[clap(long)]
        skip_cmds: bool,

        /// Stop all laio managed sessions
        #[clap(short, long)]
        all: bool,
    },

    /// List active (*) and available sessions
    #[clap(alias = "ls")]
    List {
        /// Specify the multiplexer to use.
        #[clap(short, long)]
        muxer: Option<Muxer>,
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
        let config_path = to_absolute_path(&self.config_dir)?;
        if !config_path.exists() {
            create_dir_all(config_path)
                .into_diagnostic()
                .wrap_err(format!(
                    "Could not access config path '{}'",
                    &self.config_dir
                ))?;
        }
        let res = match &self.commands {
            Commands::Start {
                name,
                file,
                muxer,
                show_picker,
                skip_cmds,
                skip_attach,
            } => self
                .session(muxer)?
                .start(name, file, *show_picker, *skip_cmds, *skip_attach)
                .wrap_err("Could not start session!".to_string()),
            Commands::Stop {
                name,
                muxer,
                skip_cmds: skip_shutdown_cmds,
                all: stop_all,
            } => self
                .session(muxer)?
                .stop(name, *skip_shutdown_cmds, *stop_all)
                .wrap_err("Unable to stop session(s)!"),
            Commands::List { muxer } => {
                let session: Vec<String> = self
                    .session(muxer)?
                    .list()
                    .wrap_err("Could not retrieve active sessions.".to_string())?;
                let config: Vec<String> = self
                    .config()
                    .list()
                    .wrap_err("Could not retrieve configurations.".to_string())?;

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

    fn session(&self, muxer: &Option<Muxer>) -> Result<SessionManager> {
        // Create the muxer
        let muxer = create_muxer(muxer).wrap_err("Could not create desired multiplexer")?;
        Ok(SessionManager::new(&self.config_dir, muxer))
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
        println!("{:?}", error);
        println!();
        if let Commands::Start { name, muxer, .. } = &self.commands {
            if let Some(n) = name {
                log::warn!("Shutting down session: {}", n);
                let _ = self.session(muxer).unwrap().stop(name, true, false);
            } else {
                log::warn!("No tmux session to shut down!");
            }
        }
    }
}
