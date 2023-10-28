use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Subcommand, Debug)]
enum Commands {
    Config(super::config::cli::Cli),
    Session(super::session::cli::Cli),
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
        match &self.commands {
            Commands::Config(cli) => cli.run(&self.config_dir),
            Commands::Session(cli) => cli.run(&self.config_dir),
        }
    }
}
