use std::fmt;
use tabled::Tabled;

#[derive(Debug, Clone, Tabled)]
pub struct SessionInfo {
    #[tabled(rename = "name")]
    pub name: String,

    #[tabled(rename = "status")]
    pub status: String,
}

impl SessionInfo {
    pub fn new(name: String, is_active: bool) -> Self {
        Self {
            name: name.clone(),
            status: if is_active {
                "●".to_string()
            } else {
                "○".to_string()
            },
        }
    }

    pub fn display_name(&self) -> String {
        format!("{} {}", self.status, self.name)
    }
}

impl fmt::Display for SessionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.status, self.name)
    }
}
