use crate::common::muxer::Multiplexer;
use clap::ValueEnum;
use miette::{Result, bail};
use std::env;
pub(crate) mod tmux;
pub(crate) mod zellij;
pub(crate) use tmux::Tmux;
pub(crate) use zellij::Zellij;

#[derive(Debug, Clone, ValueEnum)]
pub(crate) enum Muxer {
    Tmux,
    Zellij,
}

pub(crate) fn create_muxer(
    muxer: &Option<Muxer>,
    socket: Option<String>,
) -> Result<Box<dyn Multiplexer>> {
    let muxer = match muxer {
        Some(m) => m.clone(),
        None => match env::var("LAIO_MUXER") {
            Ok(env_value) => match env_value.to_lowercase().as_str() {
                "tmux" => Muxer::Tmux,
                "zellij" => Muxer::Zellij,
                _ => bail!(format!(
                    "Unsupported muxer specified in LAIO_MUXER: '{}'",
                    env_value
                )),
            },
            Err(_) => Muxer::Tmux,
        },
    };

    match muxer {
        Muxer::Tmux => Ok(Box::new(Tmux::new_with_socket(socket))),
        Muxer::Zellij => {
            if socket.is_some() {
                log::warn!("--tmux-socket is not supported by Zellij and will be ignored");
            }
            Ok(Box::new(Zellij::new()))
        }
    }
}
