use anyhow::Result;
use clap::Parser;
use commands::cli::Cli;

mod app;
mod commands;

fn main() -> Result<()> {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .init();

    cli.run()
}
