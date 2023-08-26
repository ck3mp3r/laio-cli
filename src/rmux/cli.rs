use clap::{Parser, Subcommand};

#[derive(Debug, Subcommand, Clone)]
pub(crate) enum CliCmd {
    /// Create new rmux configuration.
    New {
        /// Name of the new configuration.
        name: String,

        /// Existing configuration to copy from.
        #[clap(short, long)]
        copy: Option<String>,

        /// Copy to PWD
        #[clap(short, long)]
        pwd: bool,
    },

    /// Edit rmux configuration.
    Edit {
        /// Name of the configuration to edit.
        name: String,
    },

    /// Start new session from configuration.
    Start {
        /// Name of the configuration.
        name: Option<String>,

        /// Attach to session after creation.
        #[clap(short, long)]
        attach: bool,
    },

    /// Stop session.
    Stop {
        /// Name of the session to stop.
        name: Option<String>,
    },

    /// Delete rmux configuration.
    #[clap(alias = "rm")]
    Delete {
        /// Name of the configuration to delete.
        name: String,

        /// Force delete, no prompt.
        #[clap(short, long)]
        force: bool,
    },

    /// List all rmux configurations.
    #[clap(alias = "ls")]
    List,

    /// Save current tmux layout to configuration.
    #[clap(alias = "yaml")]
    Yaml,
}

#[derive(Debug, Parser)]
#[command(name = "rmux")]
#[command(author = "Christian Kemper <christian.kemper@me.com")]
#[command(version = concat!("v", env!("CARGO_PKG_VERSION")))]
#[command(about = "A simple tmux flexbox layout manager written in Rust.")]
pub(crate) struct Cli {
    #[clap(subcommand)]
    pub command: CliCmd,

    #[arg[long, default_value = "~/.config/rmux"]]
    pub config_dir: String,

    #[clap(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,
}
