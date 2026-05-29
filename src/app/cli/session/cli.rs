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

    /// Export a config as a standalone bash script.
    Export {
        /// Name of the configuration.
        name: Option<String>,

        /// Specify the config file to use.
        #[clap(short, long)]
        file: Option<String>,

        /// Terminal size for layout computation (WIDTHxHEIGHT, default: 200x50).
        #[clap(short, long)]
        size: Option<String>,

        /// Template variables in key=value format.
        #[clap(long = "var")]
        variables: Vec<String>,
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
            Commands::Export {
                name,
                file,
                size,
                variables,
            } => {
                let session = SessionManager::new(
                    config_path,
                    create_muxer(&None).wrap_err("Could not create desired multiplexer.")?,
                );

                let script = session.export(name, file, size.as_deref(), variables)?;
                print!("{script}");
                Ok(())
            }
        }
    }
}
