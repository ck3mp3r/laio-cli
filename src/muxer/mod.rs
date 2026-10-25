use crate::common::muxer::Multiplexer;
use clap::ValueEnum;
use miette::{Result, bail};
use std::env;
pub(crate) mod herdr;
pub(crate) mod tmux;
pub(crate) mod zellij;
pub(crate) use herdr::Herdr;
pub(crate) use tmux::Tmux;
pub(crate) use zellij::Zellij;

#[derive(Debug, Clone, ValueEnum)]
pub(crate) enum Muxer {
    Tmux,
    Zellij,
    Herdr,
}

pub(crate) fn create_muxer(muxer: &Option<Muxer>) -> Result<Box<dyn Multiplexer>> {
    let muxer = match muxer {
        Some(m) => m.clone(),
        None => match env::var("LAIO_MUXER") {
            Ok(env_value) => match env_value.to_lowercase().as_str() {
                "tmux" => Muxer::Tmux,
                "zellij" => Muxer::Zellij,
                "herdr" => Muxer::Herdr,
                _ => {
                    bail!(format!(
                        "Unsupported muxer specified in LAIO_MUXER: '{}'",
                        env_value
                    ));
                }
            },
            Err(_) => Muxer::Tmux,
        },
    };

    match muxer {
        Muxer::Tmux => Ok(Box::new(Tmux::new())),
        Muxer::Zellij => Ok(Box::new(Zellij::new())),
        Muxer::Herdr => Ok(Box::new(Herdr::new())),
    }
}

#[cfg(test)]
mod test;
