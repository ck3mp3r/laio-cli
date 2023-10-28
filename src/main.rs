use anyhow::Result;
use clap::Parser;
use commands::cli::Cli;

mod cmd;
mod commands;
mod rmx;
mod tmux;

fn main() -> Result<()> {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .init();

    cli.run()
    // let sys_cmd_runner = Rc::new(SystemCmdRunner::new());
    // let rmx = Rmx::new(cli.config_dir, Rc::clone(&sys_cmd_runner));
    // let res = match &cli.commands {
    //     CliCmd::Start { name, file, attach } => rmx.session_start(&name, &file, &attach),
    //     CliCmd::Stop { name } => rmx.session_stop(&name),
    //     CliCmd::List => rmx.session_list(),
    //     CliCmd::Config { command } => match command {
    //         ConfigSubCommand::New { name, copy, pwd } => rmx.config_new(&name, &copy, &pwd),
    //         ConfigSubCommand::Edit { name } => rmx.config_edit(&name),
    //         ConfigSubCommand::Delete { name, force } => rmx.config_delete(&name, &force),
    //         ConfigSubCommand::List => rmx.config_list(),
    //         ConfigSubCommand::Yaml => rmx.session_to_yaml(),
    //     },
    // };

    // if let Err(e) = res {
    //     log::error!("{}", e);
    //     match &cli.commands {
    //         CliCmd::Start { name, .. } => match &name {
    //             Some(n) => {
    //                 log::error!("Shutting down session: {}", n);
    //                 let _ = rmx.session_stop(&name);
    //             }
    //             None => {
    //                 log::error!("Something went wrong, no tmux session to shut down!");
    //             }
    //         },
    //         _ => {}
    //     }
    //     std::process::exit(1);
    // }
}
