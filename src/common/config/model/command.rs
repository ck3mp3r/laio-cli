use core::fmt;
use serde::{Deserialize, Serialize};
use serde_valid::Validate;
use serde_yaml::Value;
use std::{fmt::Display, process::Command as ProcessCommand};

#[derive(Debug, Deserialize, Serialize, Clone, Validate, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Command {
    #[serde(default)]
    pub(crate) command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) args: Vec<Value>,
}

impl Command {
    pub fn from_string(input: &str) -> Self {
        let mut parts = input.split_whitespace();
        let command = parts.next().unwrap_or_default().to_string();
        let args = parts.map(|s| Value::String(s.to_string())).collect();
        Command { command, args }
    }

    pub fn to_process_command(&self) -> ProcessCommand {
        let mut process_command = ProcessCommand::new(&self.command);

        process_command.args(
            self.args
                .iter()
                .map(|v| serde_yaml::to_string(v).unwrap().trim().to_string())
                .collect::<Vec<String>>(),
        );
        process_command
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut cmd = self.command.clone();

        if !self.args.is_empty() {
            cmd.push(' ');

            let formatted_args: Vec<String> = self
                .args
                .iter()
                .map(|v| match v {
                    Value::String(s) => {
                        if s.contains(' ') || s.starts_with('"') || s.starts_with('\'') {
                            format!("\"{s}\"")
                        } else {
                            s.clone()
                        }
                    }
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => String::new(),
                })
                .collect();

            cmd.push_str(&formatted_args.join(" "));
        }

        write!(f, "{cmd}")
    }
}
