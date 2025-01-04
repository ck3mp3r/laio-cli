use miette::{bail, IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};
use serde_valid::Validate;
use std::{collections::HashMap, fmt::Display, fs::read_to_string, path::Path};

use crate::common::{config::validation::generate_report, path::to_absolute_path};
use serde_valid::{
    yaml::FromYamlStr,
    Error::{DeserializeError, ValidationError},
};

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub enum FlexDirection {
    #[serde(rename = "row")]
    #[default]
    Row,
    #[serde(rename = "column")]
    Column,
}

impl FlexDirection {
    fn is_default(&self) -> bool {
        *self == FlexDirection::Row
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Validate, PartialEq)]
pub(crate) struct Command {
    #[serde(default)]
    pub(crate) command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) args: Vec<String>,
}

impl Command {
    pub fn from_string(input: &str) -> Self {
        let mut parts = input.split_whitespace();
        let command = parts.next().unwrap_or_default().to_string();
        let args = parts.map(|s| s.to_string()).collect();
        Command { command, args }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut cmd = self.command.clone();
        if !self.args.is_empty() {
            cmd.push(' ');
            cmd.push_str(
                &self
                    .args
                    .iter()
                    .map(|arg| {
                        if arg.contains(' ') {
                            format!("\"{}\"", arg) // Quote arguments with spaces
                        } else {
                            arg.clone()
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(" "),
            );
        }
        write!(f, "{}", cmd)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Validate)]
pub(crate) struct Pane {
    #[serde(default, skip_serializing_if = "FlexDirection::is_default")]
    pub(crate) flex_direction: FlexDirection,
    #[validate(minimum = 1, message = "Flex has to be >= 0")]
    #[serde(default = "flex")]
    pub(crate) flex: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(default = "default_path", skip_serializing_if = "if_is_default_path")]
    pub(crate) path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) style: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) commands: Vec<Command>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) env: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) panes: Vec<Pane>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) zoom: bool,
}

fn flex() -> usize {
    1
}

fn default_path() -> String {
    ".".to_string()
}

fn if_is_default_path(value: &str) -> bool {
    value == default_path()
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub(crate) struct Window {
    #[validate(
        min_length = 3,
        message = "Window names should have at least 3 characters."
    )]
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "FlexDirection::is_default")]
    pub(crate) flex_direction: FlexDirection,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[validate]
    pub(crate) panes: Vec<Pane>,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub(crate) struct Session {
    #[validate(
        min_length = 3,
        message = "The session name should have at least 3 characters."
    )]
    pub(crate) name: String,
    #[serde(default = "default_path")]
    pub(crate) path: String,
    #[serde(default, alias = "commands", skip_serializing_if = "Vec::is_empty")]
    pub(crate) startup: Vec<Command>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) shutdown: Vec<Command>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) env: HashMap<String, String>,
    #[validate]
    #[validate(min_items = 1, message = "At least one window is required.")]
    pub(crate) windows: Vec<Window>,
}

impl Pane {
    pub(crate) fn first_leaf_path(&self) -> Option<&String> {
        if self.panes.is_empty() {
            return Some(&self.path);
        }
        for pane in &self.panes {
            if let Some(path) = pane.first_leaf_path() {
                return Some(path);
            }
        }
        None
    }
}

impl Window {
    pub(crate) fn first_leaf_path(&self) -> Option<&String> {
        for pane in &self.panes {
            if let Some(path) = pane.first_leaf_path() {
                return Some(path);
            }
        }
        None
    }
}

impl Session {
    pub(crate) fn from_config(config: &Path) -> Result<Session> {
        let session_config = read_to_string(config).into_diagnostic()?;
        let mut session: Session =
            Session::from_yaml_str(&session_config).map_err(|e| -> miette::Report {
                match e {
                    DeserializeError(_) => miette::Report::msg(format!(
                        "Failed to parse config: {:?}\n\n{}",
                        &config, e
                    )),
                    ValidationError(_) => {
                        let validation_errors = e.as_validation_errors();
                        let error_tree = generate_report(validation_errors); // Converts the error tree into a Report
                        miette::Report::msg(format!(
                            "Failed to parse config: {:?}\n\n{}",
                            &config, error_tree
                        ))
                    }
                }
            })?;

        session.validate_zoom()?;

        let session_path = if session.path.starts_with('.') {
            let parent = config
                .parent()
                .unwrap()
                .to_str()
                .expect("Failed to find parent directory!");

            to_absolute_path(parent)?
        } else {
            to_absolute_path(&session.path)?
        };

        session.path = session_path.to_string_lossy().to_string();

        log::debug!("Final session path: {}", session.path);
        Ok(session)
    }

    fn validate_pane_zoom(panes: &[Pane], window_name: &str) -> Result<u32> {
        let mut zoom_count = 0;
        for pane in panes {
            if pane.zoom {
                zoom_count += 1;
            }
            zoom_count += Session::validate_pane_zoom(&pane.panes.clone(), window_name)?;

            if zoom_count > 1 {
                bail!(
                    "Window '{}', has more than one pane with zoom enabled",
                    window_name
                );
            }
        }
        Ok(zoom_count)
    }

    fn validate_zoom(&self) -> Result<()> {
        for window in &self.windows {
            let zoom_count = Session::validate_pane_zoom(&window.panes, &window.name)?;
            if zoom_count > 1 {
                bail!(
                    "Window '{}' has more than one pane with zoom enabled",
                    window.name
                );
            }
        }

        Ok(())
    }
}
