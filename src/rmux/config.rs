use serde::{Deserialize, Serialize};
use std::{collections::HashMap, u8};

#[derive(Serialize, Deserialize, Debug, Default)]
pub enum FlexDirection {
    #[serde(rename = "row")]
    #[default]
    Row,
    #[serde(rename = "column")]
    Column,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Pane {
    #[serde(default)]
    pub(crate) flex_direction: Option<FlexDirection>,
    #[serde(default)]
    pub(crate) flex: Option<u8>,
    #[serde(default)]
    pub(crate) path: Option<String>,
    #[serde(default)]
    pub(crate) commands: Vec<String>,
    #[serde(default)]
    pub(crate) panes: Vec<Pane>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Window {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) flex_direction: Option<FlexDirection>,
    #[serde(default)]
    pub(crate) path: Option<String>,
    #[serde(default)]
    pub(crate) manual: bool,
    #[serde(default)]
    pub(crate) layout: String,
    #[serde(default)]
    pub(crate) commands: Vec<String>,
    #[serde(default)]
    pub(crate) panes: Vec<Pane>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Session {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) path: Option<String>,
    #[serde(default)]
    pub(crate) init: Vec<String>,
    #[serde(default)]
    pub(crate) env: HashMap<String, String>,
    #[serde(default)]
    pub(crate) stop: Vec<String>,
    #[serde(default)]
    pub(crate) windows: Vec<Window>,
}
