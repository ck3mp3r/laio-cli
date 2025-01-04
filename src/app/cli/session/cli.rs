use crate::{
    app::SessionManager,
    muxer::{create_muxer, Muxer},
};

use clap::{Args, Subcommand};
use miette::{Context, Result};

#[derive(Debug, Subcommand, Clone)]
pub(crate) enum Commands {
    /// List all active sessions.
    #[clap(alias = "ls")]
    List {
        /// Specify the multiplexer to use.
        #[clap(short, long)]
        muxer: Option<Muxer>,
    },

    /// Shows current session layout as yaml.
    #[clap()]
    Yaml {
        /// Specify the multiplexer to use.
        #[clap(short, long)]
        muxer: Option<Muxer>,
    },
}

/// Manage Sessions
#[derive(Args, Debug)]
pub struct Cli {
    #[clap(subcommand)]
    commands: Commands,
}

impl Cli {
    pub fn run(&self, config_path: &str) -> Result<()> {
        match &self.commands {
            Commands::List { muxer } => {
                let muxer =
                    create_muxer(muxer).wrap_err("Could not create desired multiplexer.")?;
                let session = SessionManager::new(config_path, muxer);

                let list = session.list()?;
                println!("{}", list.join("\n"));
                Ok(())
            }
            Commands::Yaml { muxer } => {
                let muxer =
                    create_muxer(muxer).wrap_err("Could not create desired multiplexer.")?;
                let session = SessionManager::new(config_path, muxer);

                let yaml = session.to_yaml()?;
                println!("{}", yaml);
                Ok(())
            }
        }
    }
}
