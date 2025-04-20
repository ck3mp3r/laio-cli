use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub enum FlexDirection {
    #[serde(rename = "row")]
    #[default]
    Row,
    #[serde(rename = "column")]
    Column,
}

impl FlexDirection {
    pub(crate) fn is_default(&self) -> bool {
        *self == FlexDirection::Row
    }
}
