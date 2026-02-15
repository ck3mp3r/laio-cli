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
pub(crate) const LAIO_VARS: &str = "LAIO_VARS";
pub(crate) const LOCAL_CONFIG: &str = ".laio.yaml";
const DEFAULT_CONFIG: &str = "_default.yaml";

/// Encode variables to URL-encoded format: key1=value1&key2=value2
/// Values are percent-encoded for safety with special characters
pub(crate) fn encode_variables(variables: &[String]) -> Result<String> {
    if variables.is_empty() {
        return Ok(String::new());
    }

    let encoded_pairs: Vec<String> = variables
        .iter()
        .map(|var| {
            let parts: Vec<&str> = var.splitn(2, '=').collect();
            if parts.len() != 2 {
                bail!("Invalid variable format '{}', expected 'key=value'", var);
            }
            let key = parts[0];
            let value = parts[1];
            Ok(format!(
                "{}={}",
                urlencoding::encode(key),
                urlencoding::encode(value)
            ))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(encoded_pairs.join("&"))
}

/// Decode URL-encoded variables from format: key1=value1&key2=value2
/// Values are percent-decoded
pub(crate) fn decode_variables(encoded: &str) -> Result<Vec<String>> {
    if encoded.is_empty() {
        return Ok(Vec::new());
    }

    encoded
        .split('&')
        .map(|pair| {
            let parts: Vec<&str> = pair.splitn(2, '=').collect();
            if parts.len() != 2 {
                bail!(
                    "Invalid encoded variable pair '{}', expected 'key=value'",
                    pair
                );
            }
            let key = urlencoding::decode(parts[0])
                .into_diagnostic()
                .wrap_err(format!("Failed to decode key '{}'", parts[0]))?;
            let value = urlencoding::decode(parts[1])
                .into_diagnostic()
                .wrap_err(format!("Failed to decode value '{}'", parts[1]))?;
            Ok(format!("{}={}", key, value))
        })
        .collect()
}

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
        let config = match resolve_symlink(&config_path) {
            Ok(resolved) => resolved,
            Err(_) => {
                // Config doesn't exist, fallback to _default.yaml
                log::info!(
                    "Config '{}' not found, falling back to default template",
                    name
                );
                let default_config = self.ensure_default_config()?;
                resolve_symlink(&default_config)?
            }
        };

        // Filter out session_name from user variables (it's provided as first parameter)
        // and check if path is provided
        let mut effective_variables: Vec<String> = variables
            .iter()
            .filter(|v| !v.starts_with("session_name="))
            .cloned()
            .collect();

        // ALWAYS auto-inject session_name variable
        effective_variables.push(format!("session_name={}", name));

        // Auto-inject path variable if not provided, defaulting to cwd
        if !effective_variables.iter().any(|v| v.starts_with("path=")) {
            let cwd = env::current_dir()
                .into_diagnostic()
                .wrap_err("Failed to get current directory")?;
            effective_variables.push(format!("path={}", cwd.display()));
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
                    Some((config, active_session)) => {
                        // If session is already running, switch to it
                        if let Some(ref session_name) = active_session {
                            if self.multiplexer.switch(session_name, skip_attach)? {
                                return Ok(());
                            }
                        }

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

        // Prepare environment variables to pass to multiplexer
        let config_path = config.to_str().unwrap();
        let encoded_vars = encode_variables(&effective_variables)?;
        let env_vars: Vec<(&str, &str)> =
            vec![(LAIO_CONFIG, config_path), (LAIO_VARS, &encoded_vars)];

        self.multiplexer
            .start(&session, &env_vars, skip_cmds, skip_attach)
    }

    pub(crate) fn stop(
        &self,
        name: &Option<String>,
        variables: &[String],
        skip_cmds: bool,
        stop_all: bool,
        stop_other: bool,
    ) -> Result<()> {
        // If we have a name, get the config path from the session and load it
        let session = if let Some(session_name) = name {
            // Get the config path from the multiplexer (reads LAIO_CONFIG from session)
            if let Some(config_path) = self.multiplexer.get_session_config_path(session_name)? {
                // Filter out session_name from user variables (session_name is always auto-injected)
                let mut effective_variables: Vec<String> = variables
                    .iter()
                    .filter(|v| !v.starts_with("session_name="))
                    .cloned()
                    .collect();

                // ALWAYS auto-inject session_name variable
                effective_variables.push(format!("session_name={}", session_name));

                // Auto-inject path variable if not provided, defaulting to cwd
                if !effective_variables.iter().any(|v| v.starts_with("path=")) {
                    let cwd = env::current_dir()
                        .into_diagnostic()
                        .wrap_err("Failed to get current directory")?;
                    effective_variables.push(format!("path={}", cwd.display()));
                }

                // Load and render the config
                match Session::from_config(
                    &resolve_symlink(&to_absolute_path(&config_path)?)?,
                    Some(&effective_variables),
                ) {
                    Ok(sess) => Some(sess),
                    Err(e) => {
                        log::warn!(
                            "Failed to load config '{}' for session '{}': {:?}",
                            config_path,
                            session_name,
                            e
                        );
                        None
                    }
                }
            } else {
                // No LAIO_CONFIG found, session might not be a laio session
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

    pub(crate) fn select_config(
        &self,
        show_picker: bool,
    ) -> Result<Option<(PathBuf, Option<String>)>> {
        fn picker(
            config_path: &str,
            sessions: &[SessionInfo],
        ) -> Result<Option<(PathBuf, Option<String>)>> {
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
                Ok(info) => {
                    let path =
                        PathBuf::from(format!("{}/{}.yaml", &config_path, info.name.sanitize()));
                    // Return session name if it's active
                    let active_session = if info.is_active() {
                        Some(info.name.clone())
                    } else {
                        None
                    };
                    Ok(Some((path, active_session)))
                }
                Err(_) => Ok(None),
            }
        }

        if show_picker {
            picker(&self.config_path, &self.list()?)
        } else {
            match find_config(&to_absolute_path(LOCAL_CONFIG)?) {
                Ok(config) => Ok(Some((config, None))),
                Err(err) => {
                    log::debug!("{err}");
                    picker(&self.config_path, &self.list()?)
                }
            }
        }
    }
}
