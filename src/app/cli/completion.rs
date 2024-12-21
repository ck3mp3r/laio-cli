use anyhow::Result;
use clap::{Args, CommandFactory, ValueEnum};
use clap_complete::{generate, Shell};
use clap_complete_nushell::Nushell;
use std::io;

use crate::app::cli::Cli as RootCli;

#[derive(Args, Debug)]
#[command()]
pub struct Cli {
    #[arg(value_enum)]
    shell: ShellWrapper,
}

#[derive(Debug, Clone)]
enum ShellWrapper {
    Builtin(Shell),
    Nushell,
}

impl ValueEnum for ShellWrapper {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Builtin(Shell::Bash),
            Self::Builtin(Shell::Elvish),
            Self::Builtin(Shell::Fish),
            Self::Builtin(Shell::PowerShell),
            Self::Builtin(Shell::Zsh),
            Self::Nushell,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            Self::Builtin(shell) => shell.to_possible_value(),
            Self::Nushell => Some(clap::builder::PossibleValue::new("nushell")),
        }
    }
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        let mut cmd = RootCli::command();
        let bin_name = cmd.get_name().to_string();

        match &self.shell {
            ShellWrapper::Nushell => generate(Nushell, &mut cmd, bin_name, &mut io::stdout()),
            ShellWrapper::Builtin(shell) => generate(*shell, &mut cmd, bin_name, &mut io::stdout()),
        }
        Ok(())
    }
}
