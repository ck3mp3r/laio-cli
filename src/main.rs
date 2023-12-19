use anyhow::Result;
use clap::Parser;
use commands::cli::Cli;
use env_logger::Builder;

mod app;
mod commands;

fn main() -> Result<()> {
    let cli = Cli::parse();

    Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .format_timestamp(None)
        .init();

    cli.run()
}
