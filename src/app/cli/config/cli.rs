use std::rc::Rc;

use clap::{Args, Subcommand};
use miette::{Context, IntoDiagnostic, Result};
use tabled::{builder::Builder, settings::Style};

use crate::{
    app::{ConfigManager, SessionManager},
    common::{cmd::ShellRunner, session_info::SessionInfo},
    muxer::create_muxer,
};

#[derive(Clone, Subcommand, Debug)]
pub enum Commands {
    /// Create new laio configuration.
    Create {
        /// Name of the new configuration. Omit to create local .laio.yaml
        name: Option<String>,

        /// Existing configuration to copy from.
        #[clap(short, long)]
        copy: Option<String>,

        /// Template variable (repeatable, e.g., --var name=value)
        #[clap(long = "var")]
        variables: Vec<String>,
    },

    /// Edit laio configuration.
    Edit {
        /// Name of the configuration to edit.
        name: String,
    },

    /// Symlink local laio configuration to the laio config directory.
    Link {
        /// Name of the symlink.
        name: String,

        /// Specify the config file to use.
        #[clap(short, long, default_value = ".laio.yaml")]
        file: String,
    },

    /// Validate laio configuration
    Validate {
        /// Name of the configuration to validate, omit to validate local .laio.yaml.
        name: Option<String>,

        /// Specify the config file to use.
        #[clap(short, long, default_value = ".laio.yaml")]
        file: String,

        /// Template variable (repeatable, e.g., --var name=value)
        #[clap(long = "var")]
        variables: Vec<String>,
    },

    /// Delete laio configuration.
    #[clap(alias = "rm")]
    Delete {
        /// Name of the configuration to delete.
        name: String,

        /// Force delete, no prompt.
        #[clap(short, long)]
        force: bool,
    },

    /// List all laio configurations.
    #[clap(alias = "ls")]
    List {
        /// Specify the multiplexer to use.
        #[clap(short, long)]
        muxer: Option<crate::muxer::Muxer>,

        /// Output as JSON.
        #[clap(short, long)]
        json: bool,
    },
}

/// Manage Configurations
#[derive(Args, Debug)]
#[command()]
pub struct Cli {
    #[clap(subcommand)]
    commands: Commands,
}

impl Cli {
    pub fn run(&self, config_path: &str) -> Result<()> {
        let cfg = ConfigManager::new(config_path, Rc::new(ShellRunner::new()));

        match &self.commands {
            Commands::Create {
                name,
                copy,
                variables,
            } => cfg.create(name, copy, variables),
            Commands::Edit { name } => cfg.edit(name),
            Commands::Link { name, file } => cfg.link(name, file),
            Commands::Validate {
                name,
                file,
                variables,
            } => cfg.validate(name, Some(file), variables),
            Commands::Delete { name, force } => cfg.delete(name, *force),
            Commands::List { muxer, json } => {
                let muxer =
                    create_muxer(muxer).wrap_err("Could not create desired multiplexer.")?;
                let session_manager = SessionManager::new(config_path, muxer);

                let sessions = session_manager.list()?;
                let configs = cfg.list()?;

                let session_names: Vec<String> = sessions.iter().map(|s| s.name.clone()).collect();

                let mut merged: Vec<SessionInfo> = sessions;
                merged.extend(
                    configs
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
        }
    }
}
