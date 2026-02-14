use crate::{
    app::manager::config::manager::ConfigNameExt,
    common::{muxer::Multiplexer, session_info::SessionInfo},
};
use inquire::Select;
use miette::{bail, miette, Context, IntoDiagnostic, Result};
use std::{env, fs, io::Write, path::PathBuf};

use crate::{
    app::manager::config::manager::TEMPLATE,
    common::config::Session,
    common::path::{find_config, resolve_symlink, to_absolute_path},
};

pub(crate) const LAIO_CONFIG: &str = "LAIO_CONFIG";
pub(crate) const LOCAL_CONFIG: &str = ".laio.yaml";
const DEFAULT_CONFIG: &str = "_default.yaml";

pub(crate) struct SessionManager {
    pub(crate) config_path: String,
    pub(crate) multiplexer: Box<dyn Multiplexer>,
}

impl SessionManager {
    pub(crate) fn new(config_path: &str, multiplexer: Box<dyn Multiplexer>) -> Self {
        Self {
            config_path: config_path.replace('~', env::var("HOME").unwrap().as_str()),
            multiplexer,
        }
    }

    /// Generate _default.yaml if it doesn't exist
    fn ensure_default_config(&self) -> Result<PathBuf> {
        let default_path = PathBuf::from(&self.config_path).join(DEFAULT_CONFIG);

        if !default_path.exists() {
            log::info!("Generating default config at {}", default_path.display());

            // Write the raw template (NOT rendered) so it can be used with variables
            let mut file = fs::File::create(&default_path)
                .map_err(|e| miette!("Could not create '{}': {}", default_path.display(), e))?;
            file.write_all(TEMPLATE.as_bytes())
                .map_err(|e| miette!("Could not write to '{}': {}", default_path.display(), e))?;
        }

        Ok(default_path)
    }

    /// Resolve config file from name and prepare variables with default fallback support
    fn resolve_config_and_variables(
        &self,
        name: &str,
        variables: &[String],
    ) -> Result<(PathBuf, Vec<String>)> {
        let config_file = format!("{}/{}.yaml", &self.config_path, name.sanitize());
        let config_path = to_absolute_path(&config_file)
            .wrap_err(format!("Could not get absolute path for '{config_file}'"))?;

        // Try to resolve the symlink - this will fail if file doesn't exist
        let (config, session_name_for_default) = match resolve_symlink(&config_path) {
            Ok(resolved) => (resolved, None),
            Err(_) => {
                // Config doesn't exist, fallback to _default.yaml
                log::info!(
                    "Config '{}' not found, falling back to default template",
                    name
                );
                let default_config = self.ensure_default_config()?;
                let resolved_default = resolve_symlink(&default_config)?;
                (resolved_default, Some(name.to_string()))
            }
        };

        // Auto-inject session_name variable if using default fallback
        let mut effective_variables = variables.to_vec();
        if let Some(session_name) = session_name_for_default {
            effective_variables.push(format!("session_name={}", session_name));
        }

        Ok((config, effective_variables))
    }

    pub(crate) fn start(
        &self,
        name: &Option<String>,
        file: &Option<String>,
        variables: &[String],
        show_picker: bool,
        skip_cmds: bool,
        skip_attach: bool,
    ) -> Result<()> {
        if name.is_some()
            && self
                .multiplexer
                .switch(name.as_ref().unwrap(), skip_attach)?
        {
            return Ok(());
        }

        let (config, effective_variables) = match name {
            Some(name) => self.resolve_config_and_variables(name, variables)?,
            None => match file {
                Some(file) => {
                    let path = to_absolute_path(file)
                        .wrap_err(format!("Could not get absolute path for '{file}'"))?;
                    let resolved = resolve_symlink(&path)
                        .wrap_err(format!("Could not locate '{}'", path.to_string_lossy()))?;
                    (resolved, variables.to_vec())
                }
                None => match self.select_config(show_picker)? {
                    Some(config) => {
                        let resolved = resolve_symlink(&config)
                            .wrap_err(format!("Could not locate '{}'", config.to_string_lossy()))?;
                        (resolved, variables.to_vec())
                    }
                    None => bail!("No configuration selected!"),
                },
            },
        };

        let session = Session::from_config(&config, Some(&effective_variables)).wrap_err(
            format!("Could not load session from '{}'", config.to_string_lossy(),),
        )?;

        self.multiplexer
            .start(&session, config.to_str().unwrap(), skip_cmds, skip_attach)
    }

    pub(crate) fn stop(
        &self,
        name: &Option<String>,
        variables: &[String],
        skip_cmds: bool,
        stop_all: bool,
        stop_other: bool,
    ) -> Result<()> {
        // If we have a name and variables, load and render the config to get the Session
        let session = if let Some(session_name) = name {
            if !variables.is_empty() {
                // Load config and render with variables
                let (config, effective_variables) =
                    self.resolve_config_and_variables(session_name, variables)?;

                let session = Session::from_config(&config, Some(&effective_variables)).wrap_err(
                    format!("Could not load session from '{}'", config.to_string_lossy()),
                )?;

                Some(session)
            } else {
                None
            }
        } else {
            None
        };

        self.multiplexer
            .stop(name, &session, skip_cmds, stop_all, stop_other)
            .wrap_err("Multiplexer failed to stop session(s)".to_string())
    }

    pub(crate) fn list(&self) -> Result<Vec<SessionInfo>> {
        self.multiplexer
            .list_sessions()
            .wrap_err("Multiplexer failed to list sessions.".to_string())
    }

    pub(crate) fn to_yaml(&self) -> Result<String> {
        let session = self
            .multiplexer
            .get_session()
            .wrap_err("Unable to determine active session.")?;
        let yaml = serde_yaml::to_string(&session)
            .into_diagnostic()
            .wrap_err("Multiplexer unable to generate yaml representation of current session.")?;

        Ok(yaml)
    }

    pub(crate) fn select_config(&self, show_picker: bool) -> Result<Option<PathBuf>> {
        fn picker(config_path: &str, sessions: &[SessionInfo]) -> Result<Option<PathBuf>> {
            let configs = fs::read_dir(config_path)
                .into_diagnostic()
                .wrap_err(format!(
                    "Failed to list config entries in '{}'",
                    &config_path
                ))?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| {
                    // Filter out _default.yaml and only include .yaml files
                    path.extension().and_then(|ext| ext.to_str()) == Some("yaml")
                        && path.file_name().and_then(|n| n.to_str()) != Some("_default.yaml")
                })
                .map(|path| {
                    Session::from_config(&path, None)
                        .map(|session| session.name)
                        .wrap_err(format!("Warning: Failed to parse '{}'", path.display()))
                })
                .collect::<Result<Vec<String>, _>>()?;

            let session_names: Vec<String> = sessions.iter().map(|s| s.name.clone()).collect();

            let mut merged: Vec<SessionInfo> = sessions.to_vec();
            merged.extend(
                configs
                    .iter()
                    .filter(|s| !session_names.contains(s))
                    .map(|s| SessionInfo::inactive(s.to_string())),
            );

            merged.sort_by(|a, b| a.name.cmp(&b.name));
            merged.dedup_by(|a, b| a.name == b.name);

            let selected = Select::new("Select configuration:", merged)
                .with_page_size(12)
                .prompt();

            match selected {
                Ok(info) => Ok(Some(PathBuf::from(format!(
                    "{}/{}.yaml",
                    &config_path,
                    info.name.sanitize()
                )))),
                Err(_) => Ok(None),
            }
        }

        if show_picker {
            picker(&self.config_path, &self.list()?)
        } else {
            match find_config(&to_absolute_path(LOCAL_CONFIG)?) {
                Ok(config) => Ok(Some(config)),
                Err(err) => {
                    log::debug!("{err}");
                    picker(&self.config_path, &self.list()?)
                }
            }
        }
    }
}
