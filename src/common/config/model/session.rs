use super::{
    command::Command, common::default_path, pane::count_matching_panes, pane::Pane, script::Script,
    window::Window,
};
use crate::common::config::{template, validation::generate_report, variables::parse_variables};
use crate::common::path::to_absolute_path;
use miette::{bail, IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};
use serde_valid::{yaml::FromYamlStr, Error::DeserializeError, Error::ValidationError, Validate};
use std::{collections::HashMap, fs::read_to_string, path::Path};

#[derive(Debug, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct Session {
    #[validate(
        min_length = 3,
        message = "The session name should have at least 3 characters."
    )]
    pub(crate) name: String,
    #[serde(default = "default_path")]
    pub(crate) path: String,
    #[serde(default, alias = "commands", skip_serializing_if = "Vec::is_empty")]
    pub(crate) startup: Vec<Command>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) startup_script: Option<Script>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) shutdown: Vec<Command>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) shutdown_script: Option<Script>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) env: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) shell: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pane_cmd_delay: Option<u64>,
    #[validate]
    #[validate(min_items = 1, message = "At least one window is required.")]
    pub(crate) windows: Vec<Window>,
}

impl Session {
    pub(crate) fn from_config(config: &Path, variables: Option<&[String]>) -> Result<Session> {
        let session_config = read_to_string(config).into_diagnostic()?;

        // Parse variables and render template
        let var_map = parse_variables(variables.unwrap_or(&[]))?;
        let rendered_config = template::render(&session_config, &var_map)?;

        let mut session: Session =
            Session::from_yaml_str(&rendered_config).map_err(|e| -> miette::Report {
                match e {
                    DeserializeError(_) => miette::Report::msg(format!(
                        "Failed to parse config: {:?}\n\n{}",
                        &config, e
                    )),
                    ValidationError(_) => {
                        let validation_errors = e.as_validation_errors();
                        let error_tree = generate_report(validation_errors); // Converts the error tree into a Report
                        miette::Report::msg(format!(
                            "Failed to parse config: {:?}\n\n{}",
                            &config, error_tree
                        ))
                    }
                }
            })?;

        session.validate_exclusive_pane_property(|p| p.zoom, "zoom enabled")?;
        session.validate_exclusive_pane_property(|p| p.focus, "focus")?;

        let session_path = if session.path.starts_with('.') {
            let parent = config
                .parent()
                .ok_or_else(|| miette::miette!("Config path has no parent directory: {:?}", config))?
                .to_str()
                .ok_or_else(|| miette::miette!("Parent directory path is not valid UTF-8"))?;

            to_absolute_path(parent)?
        } else {
            to_absolute_path(&session.path)?
        };

        session.path = session_path.to_string_lossy().to_string();

        log::debug!("Final session path: {}", session.path);
        Ok(session)
    }

    fn validate_exclusive_pane_property(
        &self,
        predicate: impl Fn(&Pane) -> bool,
        property_name: &str,
    ) -> Result<()> {
        for window in &self.windows {
            if count_matching_panes(&window.panes, &predicate) > 1 {
                bail!(
                    "Window '{}' has more than one pane with {}",
                    window.name,
                    property_name
                );
            }
        }
        Ok(())
    }
}
