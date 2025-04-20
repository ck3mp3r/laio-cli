use crate::common::config::FlexDirection;
use crate::common::config::Script;
use miette::bail;
use miette::Result;
use serde::{Deserialize, Serialize};
use serde_valid::Validate;

use super::command::Command;
use super::common::default_path;

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
pub(crate) fn validate_pane_property<F>(
    panes: &[Pane],
    property_checker: &F,
    error_message: &str,
) -> Result<u32>
where
    F: Fn(&Pane) -> bool,
{
    let mut property_count = 0;
    for pane in panes {
        if property_checker(pane) {
            property_count += 1;
        }
        property_count +=
            validate_pane_property(&pane.panes.clone(), property_checker, error_message)?;
        log::trace!("property_count {}", property_count);

        if property_count > 1 {
            bail!(error_message.to_owned());
        }
    }
    Ok(property_count)
}
