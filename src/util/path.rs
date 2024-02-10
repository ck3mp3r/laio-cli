use std::{env, path::PathBuf};

use anyhow::{anyhow, Error, Result};

pub(crate) fn current_working_path() -> Result<String> {
    let current_dir = env::current_dir()?;
    let home_dir = home_dir()?;

    let current_dir_str = current_dir
        .to_str()
        .ok_or_else(|| anyhow!("Failed to convert current directory to string"))?;
    if current_dir_str.starts_with(&home_dir) {
        Ok(current_dir_str.replacen(&home_dir, "~", 1))
    } else {
        Ok(String::from(current_dir_str))
    }
}

pub(crate) fn home_dir() -> Result<String, Error> {
    env::var("HOME").map_err(|_| anyhow!("Failed to get home directory"))
}

pub(crate) fn to_absolute_path(input_path: &str) -> Result<String> {
    let mut path = PathBuf::from(input_path);

    // Check if the path starts with '~'
    if input_path.starts_with("~") {
        let relative_part = path
            .strip_prefix("~")
            .map_err(|_| anyhow!("Invalid path"))?;
        path = PathBuf::from(home_dir()?).join(relative_part);
    }

    // Convert to absolute path if it's not already
    if path.is_relative() {
        let current_dir =
            env::current_dir().map_err(|_| anyhow!("Unable to determine current directory"))?;
        path = current_dir.join(path);
    }

    path.to_str()
        .map(String::from)
        .ok_or_else(|| anyhow!("Failed to convert path to string"))
}

pub(crate) fn sanitize_path(path: &Option<String>, parent_path: &String) -> String {
    match path {
        Some(path) if path.starts_with("/") || path.starts_with("~") => path.clone(),
        Some(path) if path == "." => parent_path.clone(),
        Some(path) => format!(
            "{}/{}",
            parent_path,
            path.strip_prefix("./").unwrap_or(path)
        ),
        None => parent_path.clone(),
    }
}

pub(crate) fn find_config(config_path: &String) -> Result<PathBuf> {
    fn recursive_find_config(config_path: &PathBuf, home_dir: &PathBuf) -> Result<PathBuf> {
        let filename = config_path
            .file_name()
            .ok_or_else(|| Error::msg("Failed to extract filename"))?
            .to_os_string();

        fn search_upwards(
            mut current_path: PathBuf,
            filename: &std::ffi::OsString,
            home: &PathBuf,
        ) -> Result<PathBuf> {
            let file_path = current_path.join(filename);

            if file_path.exists() {
                log::info!("Found {:?}", file_path);
                return Ok(file_path);
            }

            log::warn!("Failed to locate {:?}, searching up...", file_path);

            if &current_path == home || current_path.parent().is_none() {
                return Err(Error::msg(format!(
                    "Failed to find the config file {:?} in parent directories.",
                    filename
                )));
            }

            current_path.pop();

            search_upwards(current_path, filename, home)
        }

        let start_path = config_path
            .parent()
            .ok_or_else(|| Error::msg("Failed to extract directory from path"))?
            .to_path_buf();
        search_upwards(start_path, &filename, &home_dir)
    }

    recursive_find_config(&PathBuf::from(config_path), &PathBuf::from(home_dir()?))
}
