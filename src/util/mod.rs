use serde_valid::validation::Error::{self};
use serde_valid::validation::Errors::{self};
use std::{env, path::PathBuf};

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

pub(crate) fn to_absolute_path(input_path: &str) -> Result<String> {
    let mut path = PathBuf::from(input_path);

    // Check if the path starts with '~'
    if input_path.starts_with("~") {
        let home_dir =
            env::var("HOME").map_err(|_| anyhow!("Unable to determine home directory"))?;
        let relative_part = path
            .strip_prefix("~")
            .map_err(|_| anyhow!("Invalid path"))?;
        path = PathBuf::from(home_dir).join(relative_part);
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

pub(crate) fn stringify_validation_errors(errors: &Errors) -> String {
    process_errors(errors, "").join("\n")
}

fn process_errors(errors: &Errors, prefix: &str) -> Vec<String> {
    match errors {
        Errors::Array(array_errors) => {
            log::trace!("array_errors: {}", array_errors);
            array_errors
                .items
                .iter()
                .enumerate()
                .flat_map(|(field, err)| {
                    process_errors(err.1, &format!("{}.{}", prefix, field))
                })
                .collect()
        }

        Errors::Object(object_errors) => {
            log::trace!("object_errors: {}", object_errors);
            object_errors
                .properties
                .iter()
                .flat_map(|(field, err)| process_errors(err, &format!("{}.{}", prefix, field)))
                .collect()
        }

        Errors::NewType(new_type_error) => {
            log::trace!("new_type_error: {:?}", new_type_error);
            process_error(new_type_error, prefix)
        },
    }
}

fn process_error(errors: &[Error], prefix: &str) -> Vec<String> {
    errors
        .iter()
        .flat_map(|err| vec![format!("{}: {}", prefix, err.to_string())])
        .collect()
}
