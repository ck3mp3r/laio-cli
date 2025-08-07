use crate::common::config::Command;
use miette::Result;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::fmt::{self, Display};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Script(pub String);

impl Script {
    /// Convert script to a command that executes it properly handling shebangs
    /// This eliminates the need for temporary files while preserving shebang functionality
    pub(crate) fn to_cmd(&self) -> Result<Command> {
        let script_content = &self.0;

        // Check if script has a shebang
        if let Some(shebang_line) = script_content.lines().next() {
            if shebang_line.starts_with("#!") {
                return self.handle_shebang_script(shebang_line, script_content);
            }
        }

        // Default to sh -c for scripts without shebangs
        Ok(Command {
            command: "sh".to_string(),
            args: vec![
                Value::String("-c".to_string()),
                Value::String(script_content.clone()),
            ],
        })
    }

    fn handle_shebang_script(&self, shebang: &str, script_content: &str) -> Result<Command> {
        // Remove the #! prefix and parse the interpreter
        let interpreter_path = shebang.trim_start_matches("#!").trim();

        // Extract the actual interpreter name and handle common patterns
        let (interpreter, args) = if interpreter_path.starts_with("/usr/bin/env ") {
            // Handle #!/usr/bin/env python, #!/usr/bin/env bash, etc.
            let interpreter = interpreter_path
                .strip_prefix("/usr/bin/env ")
                .unwrap()
                .trim();
            (
                interpreter.to_string(),
                self.get_interpreter_stdin_args(interpreter),
            )
        } else if interpreter_path.contains('/') {
            // Handle absolute paths like #!/bin/bash, #!/usr/bin/python3
            let interpreter = interpreter_path.split('/').last().unwrap_or("sh");
            (
                interpreter.to_string(),
                self.get_interpreter_stdin_args(interpreter),
            )
        } else {
            // Fallback for unusual cases
            (interpreter_path.to_string(), vec!["-c".to_string()])
        };

        // For shebang scripts, we need to execute the whole script including the shebang line
        // We'll use the interpreter with appropriate flags to read from stdin
        Ok(Command {
            command: interpreter,
            args: args
                .into_iter()
                .map(|arg| Value::String(arg))
                .chain(std::iter::once(Value::String(script_content.to_string())))
                .collect(),
        })
    }

    fn get_interpreter_stdin_args(&self, interpreter: &str) -> Vec<String> {
        match interpreter {
            "bash" | "sh" | "zsh" | "fish" => vec!["-c".to_string()],
            "python" | "python3" | "python2" => vec!["-c".to_string()],
            "ruby" => vec!["-e".to_string()],
            "perl" => vec!["-e".to_string()],
            "node" | "nodejs" => vec!["-e".to_string()],
            "php" => vec!["-r".to_string()],
            _ => vec!["-c".to_string()], // Default fallback
        }
    }
}

impl Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
