use serde::{Deserialize, Serialize};
use std::{collections::HashMap, u8};

use super::parser::{SplitType, Token};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub enum FlexDirection {
    #[serde(rename = "row")]
    #[default]
    Row,
    #[serde(rename = "column")]
    Column,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct Pane {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) flex_direction: Option<FlexDirection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) flex: Option<usize>,
    #[serde(default = "default_path")]
    pub(crate) path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) commands: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) panes: Option<Vec<Pane>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Window {
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) flex_direction: Option<FlexDirection>,
    #[serde(default = "default_path")]
    pub(crate) path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) commands: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) panes: Vec<Pane>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Session {
    pub(crate) name: String,
    #[serde(default = "default_path")]
    pub(crate) path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) commands: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) env: HashMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) windows: Vec<Window>,
}

fn default_path() -> Option<String> {
    log::trace!("default_path");
    Some(".".to_string())
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
        children: &Vec<Token>,
        flex_direction: Option<FlexDirection>,
    ) -> Option<Vec<Pane>> {
        let mut panes: Vec<Pane> = vec![];

        // Get dimensions into a Vec<usize> for computing GCD, rounded to nearest 5
        let dimensions: Vec<usize> = match flex_direction {
            Some(FlexDirection::Row) => children
                .iter()
                .map(|c| round(c.dimensions.height as usize))
                .collect(),
            Some(FlexDirection::Column) => children
                .iter()
                .map(|c| round(c.dimensions.width as usize))
                .collect(),
            None => vec![],
        };

        // Compute GCD of the dimensions using gcd_vec
        let gcd = gcd_vec(&dimensions);
        log::trace!("gcd of dimensions: {:?}", gcd);

        let mut flex_values = vec![];

        for token in children {
            let pane_flex_direction = match &token.split_type {
                Some(split_type) => Some(FlexDirection::from_split_type(split_type)),
                None => None,
            };

            let flex = match flex_direction {
                Some(FlexDirection::Row) => {
                    let flex_value = token.dimensions.height as usize / gcd as usize;
                    flex_values.push(flex_value);
                    Some(flex_value.max(1)) // Make sure it's at least 1
                }
                Some(FlexDirection::Column) => {
                    let flex_value = token.dimensions.width as usize / gcd as usize;
                    flex_values.push(flex_value);
                    Some(flex_value.max(1)) // Make sure it's at least 1
                }
                None => None,
            };

            let path = Some(".".to_string());
            let pane = Pane {
                flex_direction: pane_flex_direction.clone(),
                flex,
                path,
                commands: vec![],
                panes: match token.children.is_empty() {
                    false => Pane::from_tokens(&token.children, pane_flex_direction),
                    true => None,
                },
            };

            log::trace!("pane: {:?}", token);
            panes.push(pane);
        }

        // Compute GCD of the flex_values
        let flex_gcd = gcd_vec(&flex_values);
        log::trace!("gcd of flex_values: {:?}", flex_gcd);

        // Normalize flex values using the GCD
        for pane in panes.iter_mut() {
            if let Some(flex_value) = pane.flex {
                let new_flex_value = flex_value / flex_gcd;
                pane.flex = Some(if new_flex_value == 0 {
                    1
                } else {
                    new_flex_value
                });
            }
        }
        match panes.len() {
            0 => None,
            _ => Some(panes),
        }
    }
}

impl Window {
    fn from_token(token: &Token) -> Self {
        Self {
            name: token.name.clone().unwrap_or_else(|| "foo".to_string()),
            flex_direction: token
                .split_type
                .as_ref()
                .map(FlexDirection::from_split_type),
            commands: vec![],
            path: Some(".".to_string()),
            panes: Pane::from_tokens(
                &token.children,
                token
                    .split_type
                    .as_ref()
                    .map(FlexDirection::from_split_type),
            )
            .unwrap_or_else(Vec::new),
        }
    }
}

impl Session {
    pub(crate) fn from_tokens(name: &String, tokens: &Vec<Token>) -> Self {
        Self {
            name: name.clone(),
            commands: vec![],
            env: HashMap::new(),
            path: Some(".".to_string()),
            windows: tokens
                .iter()
                .map(|token| {
                    log::trace!("{:?}", token);
                    Window::from_token(token)
                })
                .collect(),
        }
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

// Function to round a number to the nearest multiple of 5
fn round(number: usize) -> usize {
    let base = 5;
    let remainder = number % base;
    if remainder >= base / 2 {
        number + base - remainder
    } else {
        number - remainder
    }
}
