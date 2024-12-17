use anyhow::Result;
use clap::Parser;
use env_logger::Builder;
use laio::app::cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();

    Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .format_timestamp(None)
        .init();

    cli.run()
}
