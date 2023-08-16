use std::rc::Rc;

use clap::Parser;
use cmd::SystemCmdRunner;
use rmux::cli::Cli;
use rmux::cli::CliCmd;
use rmux::Rmux;

mod cmd;
mod rmux;
mod tmux;

fn main() {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .init();

    let sys_cmd_runner = Rc::new(SystemCmdRunner::new());
    let rmux = Rmux::new(cli.config_dir, Rc::clone(&sys_cmd_runner));
    let res = match &cli.command {
        CliCmd::New { name, copy, pwd } => rmux.new_config(&name, &copy, &pwd),
        CliCmd::Edit { name } => rmux.edit_config(&name),
        CliCmd::Delete { name, force } => rmux.delete_config(&name, &force),
        CliCmd::Start { name, attach } => rmux.start_session(&name, &attach),
        CliCmd::Stop { name } => rmux.stop_session(&name),
        CliCmd::List => rmux.list_config(),
    };

    if let Err(e) = res {
        log::error!("{}", e);
        match &cli.command {
            CliCmd::Start { name, .. } => match &name {
                Some(n) => {
                    log::error!("Shutting down session: {}", n);
                    let _ = rmux.stop_session(&name);
                }
                None => {
                    log::error!("Something went wrong, no tmux session to shut down!");
                }
            },
            _ => {}
        }
        std::process::exit(1);
    }
}
