use anyhow::anyhow;
use anyhow::Result;
use std::env;

use crate::common::mux::Multiplexer;

use super::{Tmux, Zellij};

pub(crate) fn create_driver() -> Result<Box<dyn Multiplexer>> {
    let driver = env::var("LAIO_DRIVER").unwrap_or_else(|_| "tmux".to_string());
    match driver.as_str() {
        "tmux" => Ok(Box::new(Tmux::new())),
        "zellij" => Ok(Box::new(Zellij::new())),
        _ => Err(anyhow!("Unsupported driver specified in LAIO_DRIVER")),
    }
}
