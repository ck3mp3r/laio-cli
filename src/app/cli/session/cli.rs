use crate::{
    app::SessionManager,
    muxer::{create_muxer, Muxer},
};

use clap::{Args, Subcommand};
use miette::{Context, IntoDiagnostic, Result};
use tabled::{builder::Builder, settings::Style};

#[derive(Debug, Subcommand, Clone)]
pub(crate) enum Commands {
    /// List all active sessions.
    #[clap(alias = "ls")]
    List {
        /// Specify the multiplexer to use.
        #[clap(short, long)]
        muxer: Option<Muxer>,

        /// Output as JSON.
        #[clap(short, long)]
        json: bool,
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
            Commands::List { muxer, json } => {
                let muxer =
                    create_muxer(muxer).wrap_err("Could not create desired multiplexer.")?;
                let session = SessionManager::new(config_path, muxer);

                let list = session.list()?;

                if *json {
                    let json_output = serde_json::to_string_pretty(&list).into_diagnostic()?;
                    println!("{}", json_output);
                } else {
                    let records: Vec<_> = list
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
            Commands::Yaml { muxer } => {
                let muxer =
                    create_muxer(muxer).wrap_err("Could not create desired multiplexer.")?;
                let session = SessionManager::new(config_path, muxer);

                let yaml = session.to_yaml()?;
                println!("{yaml}");
                Ok(())
            }
        }
    }
}
