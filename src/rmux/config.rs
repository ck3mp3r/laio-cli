use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Default)]
pub enum SplitType {
    #[serde(rename = "vertical")]
    Vertical,
    #[serde(rename = "horizontal")]
    #[default]
    Horizontal,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Pane {
    #[serde(default, rename = "type")]
    pub(crate) split_type: Option<SplitType>,
    #[serde(default)]
    pub(crate) path: Option<String>,
    #[serde(default)]
    pub(crate) commands: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Window {
    pub(crate) name: String,
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
