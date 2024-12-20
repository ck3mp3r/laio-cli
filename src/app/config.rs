use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_valid::Validate;
use std::{collections::HashMap, fs::read_to_string, path::Path};

use crate::util::{path::to_absolute_path, validation::stringify_validation_errors};
use serde_valid::{
    yaml::FromYamlStr,
    Error::{DeserializeError, ValidationError},
};

use super::parser::{SplitType, Token};

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

#[derive(Debug, Deserialize, Serialize, Clone, Validate)]
pub(crate) struct Pane {
    #[serde(default, skip_serializing_if = "FlexDirection::is_default")]
    pub(crate) flex_direction: FlexDirection,
    #[validate(minimum = 1, message = "Flex has to be >= 0")]
    #[serde(default = "flex")]
    pub(crate) flex: usize,
    #[serde(default = "default_path", skip_serializing_if = "if_is_default_path")]
    pub(crate) path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) style: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) commands: Vec<String>,
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
    pub(crate) startup: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) shutdown: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) env: HashMap<String, String>,
    #[validate]
    #[validate(min_items = 1, message = "At least one window is required.")]
    pub(crate) windows: Vec<Window>,
}

impl FlexDirection {
    pub(crate) fn from_split_type(split_type: &SplitType) -> Self {
        match split_type {
            SplitType::Horizontal => Self::Column,
            SplitType::Vertical => Self::Row,
        }
    }
}

impl Pane {
    fn from_tokens(children: &[Token], flex_direction: FlexDirection) -> Vec<Pane> {
        if children.is_empty() {
            return vec![];
        }

        let dimension_selector = match flex_direction {
            FlexDirection::Row => |c: &Token| c.dimensions.width as usize,
            FlexDirection::Column => |c: &Token| c.dimensions.height as usize,
        };

        let dimensions: Vec<usize> = children.iter().map(dimension_selector).map(round).collect();

        let gcd = gcd_vec(&dimensions);
        log::trace!("gcd of dimensions: {:?}", gcd);

        // Calculate initial flex values
        let flex_values: Vec<usize> = children
            .iter()
            .map(|token| dimension_selector(token) / gcd)
            .collect();
        log::trace!("flex values: {:?}", flex_values);

        // Normalize flex values using the GCD
        let flex_gcd = gcd_vec(&flex_values);
        log::trace!("gcd of flex_values: {:?}", flex_gcd);

        // Creating panes with normalized flex values
        let panes: Vec<Pane> = children
            .iter()
            .zip(flex_values.iter())
            .map(|(token, &flex_value)| {
                let normalized_flex_value = (flex_value / flex_gcd).max(1);

                let pane_flex_direction = token
                    .split_type
                    .as_ref()
                    .map(FlexDirection::from_split_type)
                    .unwrap_or(FlexDirection::default());

                Pane {
                    flex_direction: pane_flex_direction.clone(),
                    flex: normalized_flex_value,
                    style: None,
                    path: match token.path {
                        Some(ref p) => p.clone(),
                        None => ".".to_string(),
                    },
                    commands: token.commands.clone(),
                    env: HashMap::new(),
                    panes: Pane::from_tokens(&token.children, pane_flex_direction),
                    zoom: false,
                }
            })
            .inspect(|pane| log::trace!("pane: {:?}", pane))
            .collect();

        panes
    }

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
    fn from_tokens(token: &Token) -> Self {
        let pane_flex_direction = token
            .split_type
            .as_ref()
            .map(FlexDirection::from_split_type);
        Self {
            name: token.name.clone().unwrap_or_else(|| "foo".to_string()),
            flex_direction: pane_flex_direction.clone().unwrap_or_default(),
            panes: Pane::from_tokens(&token.children, pane_flex_direction.unwrap_or_default()),
        }
    }

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
    pub(crate) fn from_tokens(name: &str, path: &str, tokens: &[Token]) -> Self {
        Self {
            name: name.to_string(),
            startup: vec![],
            shutdown: vec![],
            env: HashMap::new(),
            path: path.to_string(),
            windows: tokens
                .iter()
                .map(|token| {
                    log::trace!("{:?}", token);
                    Window::from_tokens(token)
                })
                .collect(),
        }
    }

    pub(crate) fn from_config(config: &Path) -> Result<Session> {
        let session_config = read_to_string(config)?;
        let mut session: Session =
            Session::from_yaml_str(&session_config).map_err(|e| -> Error {
                match e {
                    DeserializeError(_) => {
                        Error::msg(format!("Failed to parse config: {:?}\n\n{}", &config, e))
                    }
                    ValidationError(_) => {
                        let validation_errors: Vec<String> = e
                            .as_validation_errors()
                            .iter()
                            .map(|err| stringify_validation_errors(err))
                            .collect();

                        Error::msg(format!(
                            "Failed to parse config: {:?}\n\n{}",
                            &config,
                            &validation_errors.join("\n")
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

        log::trace!("Final session path: {}", session.path);
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
                anyhow::bail!(
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
                anyhow::bail!(
                    "Window '{}' has more than one pane with zoom enabled",
                    window.name
                );
            }
        }

        Ok(())
    }
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

fn gcd_vec(numbers: &[usize]) -> usize {
    if numbers.is_empty() || numbers.iter().all(|&x| x == 0) {
        return 1; // Return 1 if vector is empty or all zeros
    }
    numbers.iter().fold(0, |acc, &x| gcd(acc, x))
}

// Function to round a number to the nearest multiple of base
fn round(number: usize) -> usize {
    let base = 3;
    let remainder = number % base;
    if remainder >= base / 2 {
        number + base - remainder
    } else {
        number - remainder
    }
}
