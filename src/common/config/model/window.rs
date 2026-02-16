use serde::{Deserialize, Serialize};
use serde_valid::Validate;

use super::{flex_direction::FlexDirection, pane::Pane};

#[derive(Debug, Deserialize, Serialize, Validate)]
pub(crate) struct Window {
    #[validate(
        min_length = 1,
        message = "Window names should have at least 1 character."
    )]
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "FlexDirection::is_default")]
    pub(crate) flex_direction: FlexDirection,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[validate]
    pub(crate) panes: Vec<Pane>,
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
