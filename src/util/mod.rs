use std::env;

use anyhow::{anyhow, Result};

pub(crate) fn current_working_path() -> Result<String> {
    let current_dir = env::current_dir()?;
    let home_dir = env::var("HOME").map_err(|_| anyhow!("Failed to get home directory"))?;

    let current_dir_str = current_dir
        .to_str()
        .ok_or_else(|| anyhow!("Failed to convert current directory to string"))?;
    if current_dir_str.starts_with(&home_dir) {
        Ok(current_dir_str.replacen(&home_dir, "~", 1))
    } else {
        Ok(String::from(current_dir_str))
    }
}
