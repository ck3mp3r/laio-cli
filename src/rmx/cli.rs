use clap::{Parser, Subcommand};

#[derive(Debug, Subcommand, Clone)]
pub(crate) enum CliCmd {
    /// Manage configurations.
    Config {
        #[clap(subcommand)]
        command: ConfigSubCommand,
    },

    /// List all active sessions.
    List,

    /// Start new session from configuration.
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

#[derive(Debug, Parser, Clone)]
pub(crate) enum ConfigSubCommand {
    /// Create new rmx configuration.
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

    /// Edit rmx configuration.
    Edit {
        /// Name of the configuration to edit.
        name: String,
    },

    /// Delete rmx configuration.
    #[clap(alias = "rm")]
    Delete {
        /// Name of the configuration to delete.
        name: String,

        /// Force delete, no prompt.
        #[clap(short, long)]
        force: bool,
    },

    /// List all rmx configurations.
    #[clap(alias = "ls")]
    List,

    /// Shows current session layout as yaml.
    #[clap()]
    Yaml,
}

#[derive(Debug, Parser)]
#[command(name = "rmx")]
#[command(author = "Christian Kemper <christian.kemper@me.com")]
#[command(version = concat!("v", env!("CARGO_PKG_VERSION")))]
#[command(about = "A simple flexbox-like layout manager for tmux.")]
pub(crate) struct Cli {
    #[clap(subcommand)]
    pub command: CliCmd,

    #[arg[long, default_value = "~/.config/rmx"]]
    pub config_dir: String,

    #[clap(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,
}
