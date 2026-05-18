use std::{fs::create_dir_all, process::exit, rc::Rc};

use clap::{Parser, Subcommand};
use miette::{Context, Error, IntoDiagnostic, Result};
use tabled::{builder::Builder, settings::Style};

use crate::{
    app::{ConfigManager, SessionManager},
    common::{cmd::ShellRunner, path::to_absolute_path, session_info::SessionInfo},
    muxer::{Muxer, create_muxer},
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

        /// Template variables in key=value format (can be specified multiple times)
        /// Example: --var name=myproject --var path=/home/user/dev
        #[clap(long = "var")]
        variables: Vec<String>,
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

    /// tmux socket path; takes priority over LAIO_TMUX_SOCKET env var.
    #[arg(long, global = true)]
    pub tmux_socket: Option<String>,

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
                variables,
            } => self
                .session(muxer)?
                .stop(name, variables, *skip_shutdown_cmds, *stop_all, *stop_other)
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
            Commands::Config(cli) => cli.run(&self.config_dir, self.resolved_socket()),
            Commands::Session(cli) => cli.run(&self.config_dir, self.resolved_socket()),
            Commands::Completion(cli) => cli.run(),
        };

        if let Err(e) = res {
            self.handle_error(&e);
            exit(1);
        }

        res
    }

    fn session(&self, muxer: &Option<Muxer>) -> Result<SessionManager> {
        self.session_with_socket(muxer, self.resolved_socket())
    }

    fn session_with_socket(
        &self,
        muxer: &Option<Muxer>,
        socket: Option<String>,
    ) -> Result<SessionManager> {
        let muxer = create_muxer(muxer, socket).wrap_err("Could not create desired multiplexer")?;
        Ok(SessionManager::new(&self.config_dir, muxer))
    }

    fn resolved_socket(&self) -> Option<String> {
        self.tmux_socket
            .clone()
            .or_else(|| std::env::var("LAIO_TMUX_SOCKET").ok())
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
                let _ = self
                    .session(muxer)
                    .unwrap()
                    .stop(name, &[], true, false, false);
            } else {
                log::warn!("No tmux session to shut down!");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::sync::Mutex;

    // Serialize env-var tests to avoid races when Rust's test runner uses multiple threads.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn parse(args: &[&str]) -> Cli {
        Cli::parse_from(std::iter::once("laio").chain(args.iter().copied()))
    }

    // `--tmux-socket` flag is parsed and stored on the Start subcommand.
    #[test]
    fn start_socket_flag_is_parsed() {
        let cli = parse(&["start", "--tmux-socket", "/tmp/test.sock", "--skip-attach"]);
        assert_eq!(cli.tmux_socket.as_deref(), Some("/tmp/test.sock"));
    }

    // absent `--tmux-socket` flag leaves the field None.
    #[test]
    fn start_socket_flag_absent_is_none() {
        let cli = parse(&["start", "--skip-attach"]);
        assert!(cli.tmux_socket.is_none());
    }

    // LAIO_TMUX_SOCKET env var is honoured when --tmux-socket is absent.
    #[test]
    fn start_socket_resolved_from_env_var() {
        let _guard = ENV_LOCK.lock().unwrap();
        // SAFETY: test is serialized via ENV_LOCK; no concurrent env access.
        unsafe { std::env::set_var("LAIO_TMUX_SOCKET", "/tmp/env-test.sock") };
        let cli = parse(&["start", "--skip-attach"]);
        assert!(cli.tmux_socket.is_none());
        let resolved = cli.resolved_socket();
        // SAFETY: restoring env state.
        unsafe { std::env::remove_var("LAIO_TMUX_SOCKET") };
        assert_eq!(resolved.as_deref(), Some("/tmp/env-test.sock"));
    }

    // explicit --tmux-socket takes priority over LAIO_TMUX_SOCKET env var.
    #[test]
    fn start_socket_flag_overrides_env_var() {
        let _guard = ENV_LOCK.lock().unwrap();
        // SAFETY: test is serialized via ENV_LOCK; no concurrent env access.
        unsafe { std::env::set_var("LAIO_TMUX_SOCKET", "/tmp/env-value.sock") };
        let cli = parse(&["start", "--tmux-socket", "/tmp/flag-value.sock", "--skip-attach"]);
        let resolved = cli.resolved_socket();
        // SAFETY: restoring env state.
        unsafe { std::env::remove_var("LAIO_TMUX_SOCKET") };
        assert_eq!(resolved.as_deref(), Some("/tmp/flag-value.sock"));
    }

    #[test]
    fn socket_flag_is_global() {
        let cli = parse(&["session", "list", "--tmux-socket", "/tmp/test.sock"]);
        assert_eq!(cli.tmux_socket.as_deref(), Some("/tmp/test.sock"));
    }
}
