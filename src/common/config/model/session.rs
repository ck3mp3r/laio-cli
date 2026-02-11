use super::{
    command::Command, common::default_path, pane::validate_pane_property, script::Script,
    window::Window,
};
use crate::common::config::{template, validation::generate_report, variables::parse_variables};
use crate::common::path::to_absolute_path;
use miette::{IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};
use serde_valid::{yaml::FromYamlStr, Error::DeserializeError, Error::ValidationError, Validate};
use std::{collections::HashMap, fs::read_to_string, path::Path};

#[derive(Debug, Deserialize, Serialize, Validate)]
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

        session.validate_zoom()?;
        session.validate_focus()?;

        let session_path = if session.path.starts_with('.') {
            let parent = config
                .parent()
                .unwrap()
                .to_str()
                .expect("Failed to find parent directory!");

            to_absolute_path(parent)?
        } else {
            to_absolute_path(&session.path)?
        };

        session.path = session_path.to_string_lossy().to_string();

        log::debug!("Final session path: {}", session.path);
        Ok(session)
    }

    fn validate_zoom(&self) -> Result<()> {
        for window in &self.windows {
            let error_message = format!(
                "Window '{}', has more than one pane with zoom enabled",
                window.name
            );
            validate_pane_property(&window.panes, &|pane| pane.zoom, &error_message)?;
        }

        Ok(())
    }

    fn validate_focus(&self) -> Result<()> {
        for window in &self.windows {
            let error_message =
                format!("Window '{}', has more than one pane to focus", window.name);
            validate_pane_property(&window.panes, &|pane| pane.focus, &error_message)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_from_config_with_variables() {
        let config_path = PathBuf::from("src/common/config/test/templated.yaml");
        let variables = vec![
            "name=my-project".to_string(),
            "path=/home/user/projects".to_string(),
            "window_name=editor".to_string(),
        ];

        let session = Session::from_config(&config_path, Some(&variables)).unwrap();

        assert_eq!(session.name, "my-project");
        assert_eq!(session.path, "/home/user/projects");
        assert_eq!(session.windows.len(), 1);
        assert_eq!(session.windows[0].name, "editor");
    }

    #[test]
    fn test_from_config_with_defaults() {
        let config_path = PathBuf::from("src/common/config/test/templated.yaml");

        let session = Session::from_config(&config_path, None).unwrap();

        // Should use default values from template
        assert_eq!(session.name, "test-session");
        assert_eq!(session.path, "/tmp");
        assert_eq!(session.windows[0].name, "main");
    }

    #[test]
    fn test_from_config_partial_variables() {
        let config_path = PathBuf::from("src/common/config/test/templated.yaml");
        let variables = vec!["name=partial-test".to_string()];

        let session = Session::from_config(&config_path, Some(&variables)).unwrap();

        assert_eq!(session.name, "partial-test");
        assert_eq!(session.path, "/tmp"); // Uses default
        assert_eq!(session.windows[0].name, "main"); // Uses default
    }

    #[test]
    fn test_from_config_with_array_variables() {
        let config_path = PathBuf::from("src/common/config/test/multi-projects.yaml");
        let variables = vec![
            "name=multi-env".to_string(),
            "path=/home/dev".to_string(),
            "projects=web".to_string(),
            "projects=api".to_string(),
            "projects=cli".to_string(),
        ];

        let session = Session::from_config(&config_path, Some(&variables)).unwrap();

        assert_eq!(session.name, "multi-env");
        assert_eq!(session.path, "/home/dev");
        assert_eq!(session.windows.len(), 3);
        assert_eq!(session.windows[0].name, "web");
        assert_eq!(session.windows[1].name, "api");
        assert_eq!(session.windows[2].name, "cli");

        // Check paths are correct
        assert_eq!(session.windows[0].panes[0].path, "/home/dev/web");
        assert_eq!(session.windows[1].panes[0].path, "/home/dev/api");
        assert_eq!(session.windows[2].panes[0].path, "/home/dev/cli");
    }
}
