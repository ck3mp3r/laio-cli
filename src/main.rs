use std::rc::Rc;

use clap::Parser;
use cmd::SystemCmdRunner;
use rmx::cli::Cli;
use rmx::cli::CliCmd;
use rmx::cli::ConfigSubCommand;
use rmx::Rmx;

mod cmd;
mod rmx;
mod tmux;

fn main() {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .init();

    let sys_cmd_runner = Rc::new(SystemCmdRunner::new());
    let rmx = Rmx::new(cli.config_dir, Rc::clone(&sys_cmd_runner));
    let res = match &cli.command {
        CliCmd::Start { name, file, attach } => rmx.start_session(&name, &file, &attach),
        CliCmd::Stop { name } => rmx.stop_session(&name),
        CliCmd::List => rmx.list_sessions(),
        CliCmd::Config { command } => match command {
            ConfigSubCommand::New { name, copy, pwd } => rmx.new_config(&name, &copy, &pwd),
            ConfigSubCommand::Edit { name } => rmx.edit_config(&name),
            ConfigSubCommand::Delete { name, force } => rmx.delete_config(&name, &force),
            ConfigSubCommand::List => rmx.list_config(),
            ConfigSubCommand::Yaml => rmx.session_to_yaml(),
        },
    };

    if let Err(e) = res {
        log::error!("{}", e);
        match &cli.command {
            CliCmd::Start { name, .. } => match &name {
                Some(n) => {
                    log::error!("Shutting down session: {}", n);
                    let _ = rmx.stop_session(&name);
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
