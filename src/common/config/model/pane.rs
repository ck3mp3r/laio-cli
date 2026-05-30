use crate::common::config::FlexDirection;
use crate::common::config::Script;
use serde::{Deserialize, Serialize};
use serde_valid::Validate;

use super::command::Command;
use super::common::default_path;

#[derive(Debug, Deserialize, Serialize, Clone, Validate)]
#[serde(deny_unknown_fields)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) script: Option<Script>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) panes: Vec<Pane>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) zoom: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) focus: bool,
}

fn flex() -> usize {
    1
}

fn if_is_default_path(value: &str) -> bool {
    value == default_path()
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
pub(crate) fn count_matching_panes(panes: &[Pane], predicate: &impl Fn(&Pane) -> bool) -> usize {
    panes.iter().fold(0, |acc, pane| {
        acc + usize::from(predicate(pane)) + count_matching_panes(&pane.panes, predicate)
    })
}
