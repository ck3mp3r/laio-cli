use serde::{Deserialize, Serialize};
use serde_valid::Validate;

use crate::common::path::sanitize_path;

use super::{flex_direction::FlexDirection, pane::Pane};

#[derive(Debug, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct Window {
    #[validate(
        min_length = 1,
        message = "Window names should have at least 1 character."
    )]
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) path: Option<String>,
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

    /// Effective working directory for this window's panes.
    /// `path` is resolved relative to `session_path`; absolute paths and `~`
    /// are kept as-is. When unset, the session path is used directly.
    pub(crate) fn effective_path(&self, session_path: &str) -> String {
        match &self.path {
            Some(p) => sanitize_path(p, &session_path.to_string()),
            None => session_path.to_string(),
        }
    }
}
