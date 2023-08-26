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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) path: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) commands: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) panes: Option<Vec<Pane>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Window {
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) flex_direction: Option<FlexDirection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) path: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) commands: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) panes: Vec<Pane>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Session {
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) path: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) commands: Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub(crate) env: HashMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) windows: Vec<Window>,
}

pub(crate) fn session_from_tokens(tokens: &Vec<Token>) -> Session {
    let mut windows = vec![];
    for token in tokens {
        log::trace!("{:?}", token);
        let window = window_from_token(&token);
        windows.push(window);
    }
    let name = "foo".to_string();
    let commands = vec![];
    let env = HashMap::new();
    let path = None;

    let session = Session {
        name,
        commands,
        env,
        path,
        windows,
    };

    session
}

impl FlexDirection {
    fn from_split_type(split_type: &SplitType) -> Self {
        match split_type {
            SplitType::Horizontal => Self::Column,
            SplitType::Vertical => Self::Row,
        }
    }
}

fn window_from_token(token: &Token) -> Window {
    let name = match &token.name {
        Some(name) => name.clone(),
        None => "foo".to_string(),
    };

    let flex_direction = match &token.split_type {
        Some(split_type) => Some(FlexDirection::from_split_type(split_type)),
        None => None,
    };

    let panes = pane_from_tokens(&token.children, flex_direction.clone()).unwrap_or(vec![]);
    let commands = vec![];
    let path = None;

    Window {
        name,
        flex_direction,
        commands,
        path,
        panes,
    }
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

fn pane_from_tokens(
    children: &Vec<Token>,
    flex_direction: Option<FlexDirection>,
) -> Option<Vec<Pane>> {
    let mut panes = vec![];

    let gcd = match flex_direction {
        Some(FlexDirection::Row) => children
            .iter()
            .map(|c| c.dimensions.height)
            .fold(children[0].dimensions.height as usize, |acc, x| {
                gcd(acc, x as usize)
            }),
        Some(FlexDirection::Column) => children
            .iter()
            .map(|c| c.dimensions.width)
            .fold(children[0].dimensions.width as usize, |acc, x| {
                gcd(acc, x as usize)
            }),
        None => 0,
    };

    log::trace!("total_size: {:?}", gcd);

    for token in children {
        let pane_flex_direction = match &token.split_type {
            Some(split_type) => Some(FlexDirection::from_split_type(split_type)),
            None => None,
        };

        let flex = match flex_direction {
            Some(FlexDirection::Row) => {
                Some(token.dimensions.height as usize / gcd as usize)
            }
            Some(FlexDirection::Column) => {
                Some(token.dimensions.width as usize / gcd as usize)
            }
            None => None,
        };

        let pane = Pane {
            flex_direction: pane_flex_direction.clone(),
            flex,
            path: None,
            commands: vec![],
            panes: match token.children.is_empty() {
                false => pane_from_tokens(&token.children, pane_flex_direction),
                true => None,
            },
        };
        log::trace!("pane: {:?}", token);
        panes.push(pane);
    }

    match panes.len() {
        0 => None,
        _ => Some(panes),
    }
}
