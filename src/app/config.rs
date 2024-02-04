use anyhow::Error;
use serde::{Deserialize, Serialize};
use serde_valid::Validate;
use std::{collections::HashMap, u8};

use crate::util::validation::stringify_validation_errors;
use serde_valid::{
    yaml::FromYamlStr,
    Error::{DeserializeError, ValidationError},
};

use super::parser::{SplitType, Token};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub enum FlexDirection {
    #[serde(rename = "row")]
    #[default]
    Row,
    #[serde(rename = "column")]
    Column,
}

#[derive(Debug, Deserialize, Serialize, Clone, Validate)]
pub(crate) struct Pane {
    #[serde(default)]
    pub(crate) flex_direction: FlexDirection,
    #[validate(minimum = 1, message = "Flex has to be >= 0")]
    #[serde(default = "flex")]
    pub(crate) flex: usize,
    #[serde(default = "default_path")]
    pub(crate) path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) style: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) commands: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) env: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) panes: Option<Vec<Pane>>,
}

fn flex() -> usize {
    1
}

fn default_path() -> Option<String> {
    Some(".".to_string())
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub(crate) struct Window {
    #[validate(
        min_length = 3,
        message = "Window names should have at least 3 characters."
    )]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) flex_direction: FlexDirection,
    #[serde(default)]
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
    pub(crate) path: Option<String>,
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
    fn from_tokens(
        children: &[Token], // Use slice instead of Vec reference
        flex_direction: FlexDirection,
    ) -> Option<Vec<Pane>> {
        if children.is_empty() {
            return None;
        }

        let dimension_selector = match flex_direction {
            FlexDirection::Row => |c: &Token| c.dimensions.height as usize,
            FlexDirection::Column => |c: &Token| c.dimensions.width as usize,
        };

        let dimensions: Vec<usize> = children
            .iter()
            .map(|c| dimension_selector(c))
            .map(round)
            .collect();

        let gcd = gcd_vec(&dimensions);
        log::trace!("gcd of dimensions: {:?}", gcd);

        // Calculate initial flex values
        let flex_values: Vec<usize> = children
            .iter()
            .map(|token| dimension_selector(token) / gcd)
            .collect();

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
                    path: Some(".".to_string()),
                    commands: vec![],
                    env: HashMap::new(),
                    panes: Pane::from_tokens(&token.children, pane_flex_direction),
                }
            })
            .inspect(|pane| log::trace!("pane: {:?}", pane))
            .collect();

        Some(panes)
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
            flex_direction: pane_flex_direction
                .clone()
                .unwrap_or(FlexDirection::default()),
            panes: Pane::from_tokens(
                &token.children,
                pane_flex_direction.unwrap_or(FlexDirection::default()),
            )
            .unwrap_or_else(Vec::new),
        }
    }
}

impl Session {
    pub(crate) fn from_tokens(name: &String, tokens: &Vec<Token>) -> Self {
        Self {
            name: name.clone(),
            startup: vec![],
            shutdown: vec![],
            env: HashMap::new(),
            path: Some(".".to_string()),
            windows: tokens
                .iter()
                .map(|token| {
                    log::trace!("{:?}", token);
                    Window::from_tokens(token)
                })
                .collect(),
        }
    }

    pub(crate) fn from_config(config: &String) -> Result<Session, Error> {
        let config_contents = std::fs::read_to_string(config)
            .map_err(|_e| Error::msg(format!("Failed to read config: {}", &config)))?;
        let session: Session = Session::from_yaml_str(&config_contents).map_err(|e| -> Error {
            match e {
                DeserializeError(_) => Error::msg(format!(
                    "Failed to parse config: {}\n\n{}",
                    &config,
                    e.to_string()
                )),
                ValidationError(_) => {
                    let validation_errors: Vec<String> = e
                        .as_validation_errors()
                        .iter()
                        .map(|err| stringify_validation_errors(err))
                        .collect();

                    Error::msg(format!(
                        "Failed to parse config: {}\n\n{}",
                        &config,
                        &validation_errors.join("\n")
                    ))
                }
            }
        })?;
        Ok(session)
    }
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

fn gcd_vec(numbers: &Vec<usize>) -> usize {
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
