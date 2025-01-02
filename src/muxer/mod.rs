pub(crate) mod tmux;
pub(crate) mod zellij;

use std::env;

use miette::bail;
use miette::Result;
pub(crate) use tmux::Tmux;
pub(crate) use zellij::Zellij;

use crate::common::muxer::Multiplexer;

pub(crate) fn create_muxer() -> Result<Box<dyn Multiplexer>> {
    let muxer = env::var("LAIO_MUXER").unwrap_or_else(|_| "tmux".to_string());
    match muxer.as_str() {
        "tmux" => Ok(Box::new(Tmux::new())),
        "zellij" => Ok(Box::new(Zellij::new())),
        _ => bail!("Unsupported muxer specified in LAIO_MUXER"),
    }
}
