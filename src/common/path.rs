use std::{
    env,
    ffi::OsString,
    fs::{read_link, symlink_metadata},
    path::{Path, PathBuf},
};

use miette::{bail, miette, Error, IntoDiagnostic, Result};

pub(crate) fn home_dir() -> Result<String> {
    env::var("HOME").map_err(|_| miette!("Failed to get home directory"))
}

pub(crate) fn to_absolute_path(input_path: &str) -> Result<PathBuf> {
    log::debug!("Input path: {input_path}");

    let path = match input_path {
        "." | "./" | "" => env::current_dir().into_diagnostic()?,
        _ if input_path.starts_with('~') => {
            let without_tilde = input_path.strip_prefix('~').unwrap();
            let suffix = Path::new(without_tilde)
                .strip_prefix("/")
                .unwrap_or(Path::new(without_tilde));
            PathBuf::from(home_dir()?).join(suffix)
        }
        _ => {
            let path = PathBuf::from(input_path);
            if path.is_absolute() {
                path
            } else if path.starts_with("./") {
                env::current_dir()
                    .into_diagnostic()?
                    .join(path.strip_prefix("./").unwrap())
            } else {
                env::current_dir().into_diagnostic()?.join(path)
            }
        }
    };

    log::debug!("Output path: {path:?}");
    Ok(path)
}

pub(crate) fn resolve_symlink(path: &PathBuf) -> Result<PathBuf> {
    let new_path = if symlink_metadata(path)
        .into_diagnostic()?
        .file_type()
        .is_symlink()
    {
        let symlink = read_link(path).into_diagnostic()?;
        log::debug!("Found symlink: {path:?} -> {symlink:?}");
        symlink
    } else {
        path.to_path_buf()
    };
    Ok(new_path)
}

pub(crate) fn sanitize_path(path: &String, parent_path: &String) -> String {
    log::debug!("Original path: {path}");
    let path = match path {
        path if path.starts_with('/') || path.starts_with('~') => path.clone(),
        path if path == "." => parent_path.clone(),
        path => format!(
            "{}/{}",
            parent_path,
            path.strip_prefix("./").unwrap_or(path)
        ),
    };
    log::debug!("Sanitized path: {path}");
    path
}

pub(crate) fn find_config(config_path: &Path) -> Result<PathBuf> {
    fn recursive_find_config(config_path: &Path, home_dir: &PathBuf) -> Result<PathBuf> {
        let filename = config_path
            .file_name()
            .ok_or_else(|| Error::msg("Failed to extract filename"))?
            .to_os_string();

        fn search_upwards(
            mut current_path: PathBuf,
            filename: &OsString,
            home: &PathBuf,
        ) -> Result<PathBuf> {
            let file_path = current_path.join(filename);

            if file_path.exists() {
                log::info!("Found config: {file_path:?}");
                return Ok(file_path);
            }

            log::warn!("Failed to locate {file_path:?}, searching up...");

            if &current_path == home || current_path.parent().is_none() {
                bail!(
                    "Failed to find the config file {:?} in parent directories.",
                    filename
                )
            }

            current_path.pop();

            search_upwards(current_path, filename, home)
        }

        let start_path = config_path
            .parent()
            .ok_or_else(|| Error::msg("Failed to extract directory from path"))?
            .to_path_buf();
        search_upwards(start_path, &filename, home_dir)
    }

    recursive_find_config(config_path, &PathBuf::from(home_dir()?))
}

pub(crate) fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

pub(crate) fn relative_path(absolute_path: &str, base_path: &str) -> Option<String> {
    let home_dir: String = match home_dir() {
        Ok(home) => home,
        Err(_) => "/".to_string(),
    };
    let full_path = PathBuf::from(absolute_path.replace(&home_dir, "~"));

    let rel_path = full_path
        .strip_prefix(base_path.replace(&home_dir, "~"))
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| full_path.to_string_lossy().into_owned());

    match rel_path.is_empty() {
        true => None,
        false => Some(rel_path),
    }
}

#[cfg(test)]
pub(crate) fn current_working_path() -> Result<PathBuf> {
    let current_dir = env::current_dir().into_diagnostic()?;
    let home_dir = home_dir()?;

    let current_dir_str = current_dir
        .to_str()
        .ok_or_else(|| miette!("Failed to convert current directory to string"))?;
    if current_dir_str.starts_with(&home_dir) {
        Ok(current_dir_str.replacen(&home_dir, "~", 1).into())
    } else {
        Ok(current_dir_str.into())
    }
}
