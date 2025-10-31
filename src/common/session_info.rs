use serde::{Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Attached,
    Active,
    Inactive,
}

impl Serialize for SessionStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let status_str = match self {
            SessionStatus::Attached => "attached",
            SessionStatus::Active => "active",
            SessionStatus::Inactive => "inactive",
        };
        serializer.serialize_str(status_str)
    }
}

impl SessionStatus {
    pub fn icon(&self) -> &str {
        match self {
            SessionStatus::Attached => "●",
            SessionStatus::Active => "○",
            SessionStatus::Inactive => "·",
        }
    }
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.icon())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub status: SessionStatus,
    pub name: String,
}

impl SessionInfo {
    pub fn active(name: String, is_attached: bool) -> Self {
        Self {
            status: if is_attached {
                SessionStatus::Attached
            } else {
                SessionStatus::Active
            },
            name,
        }
    }

    pub fn inactive(name: String) -> Self {
        Self {
            status: SessionStatus::Inactive,
            name,
        }
    }
}

impl fmt::Display for SessionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.status.icon(), self.name)
    }
}
