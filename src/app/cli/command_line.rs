use std::{fs::create_dir_all, process::exit, rc::Rc};

use clap::{Parser, Subcommand};
use miette::{Context, Error, IntoDiagnostic, Result};
use tabled::{builder::Builder, settings::Style};

use crate::{
    app::{ConfigManager, SessionManager},
    common::{cmd::ShellRunner, path::to_absolute_path, session_info::SessionInfo},
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

        /// Specify the multiplexer to use. Note: Zellij support is experimental!
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

        /// Template variables in key=value format (can be specified multiple times)
        /// Example: --var name=myproject --var path=/home/user/dev
        #[clap(long = "var")]
        variables: Vec<String>,
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

        /// Stop other laio managed sessions
        #[clap(short, long)]
        others: bool,
    },

    /// List active (*) and available sessions
    #[clap(alias = "ls")]
    List {
        /// Specify the multiplexer to use.
        #[clap(short, long)]
        muxer: Option<Muxer>,

        /// Output as JSON.
        #[clap(short, long)]
        json: bool,
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
                variables,
            } => self
                .session(muxer)?
                .start(
                    name,
                    file,
                    variables,
                    *show_picker,
                    *skip_cmds,
                    *skip_attach,
                )
                .wrap_err("Could not start session!".to_string()),
            Commands::Stop {
                name,
                muxer,
                skip_cmds: skip_shutdown_cmds,
                all: stop_all,
                others: stop_other,
            } => self
                .session(muxer)?
                .stop(name, *skip_shutdown_cmds, *stop_all, *stop_other)
                .wrap_err("Unable to stop session(s)!"),
            Commands::List { muxer, json } => {
                let session_info = self
                    .session(muxer)?
                    .list()
                    .wrap_err("Could not retrieve active sessions.".to_string())?;
                let config: Vec<String> = self
                    .config()
                    .list()
                    .wrap_err("Could not retrieve configurations.".to_string())?;

                let session_names: Vec<String> =
                    session_info.iter().map(|s| s.name.clone()).collect();

                let mut merged: Vec<SessionInfo> = session_info;
                merged.extend(
                    config
                        .iter()
                        .filter(|c| !session_names.contains(c))
                        .map(|s| SessionInfo::inactive(s.to_string())),
                );
                merged.sort_by(|a, b| a.name.cmp(&b.name));
                merged.dedup_by(|a, b| a.name == b.name);

                if *json {
                    let json_output = serde_json::to_string_pretty(&merged).into_diagnostic()?;
                    println!("{}", json_output);
                } else {
                    let records: Vec<_> = merged
                        .iter()
                        .map(|item| [item.status.icon(), item.name.as_str()])
                        .collect();
                    let builder = Builder::from_iter(records);
                    let mut table = builder.build();
                    table.with(Style::rounded().remove_horizontals());
                    println!("{}", table);
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
        println!("{error:?}");
        println!();
        if let Commands::Start { name, muxer, .. } = &self.commands {
            if let Some(n) = name {
                log::warn!("Shutting down session: {n}");
                let _ = self.session(muxer).unwrap().stop(name, true, false, false);
            } else {
                log::warn!("No tmux session to shut down!");
            }
        }
    }
}
