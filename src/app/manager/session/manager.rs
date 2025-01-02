use crate::common::muxer::Multiplexer;
use inquire::Select;
use miette::{bail, IntoDiagnostic, Result};
use std::{env, fs, path::PathBuf};

use crate::{
    common::config::Session,
    common::path::{find_config, resolve_symlink, to_absolute_path},
};

pub(crate) const LAIO_CONFIG: &str = "LAIO_CONFIG";
pub(crate) const LOCAL_CONFIG: &str = ".laio.yaml";

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

    pub(crate) fn start(
        &self,
        name: &Option<String>,
        file: &Option<String>,
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

        let config = match name {
            Some(name) => {
                to_absolute_path(&format!("{}/{}.yaml", &self.config_path, name).to_string())?
            }
            None => match file {
                Some(file) => to_absolute_path(file)?,
                None => match self.select_config(show_picker)? {
                    Some(config) => config,
                    None => bail!("No configuration selected!"),
                },
            },
        };

        let session = Session::from_config(&resolve_symlink(&config)?)?;

        self.multiplexer
            .start(&session, config.to_str().unwrap(), skip_attach, skip_cmds)
    }

    pub(crate) fn stop(
        &self,
        name: &Option<String>,
        skip_cmds: bool,
        stop_all: bool,
    ) -> Result<()> {
        self.multiplexer.stop(name, skip_cmds, stop_all)
    }

    pub(crate) fn list(&self) -> Result<Vec<String>> {
        self.multiplexer.list_sessions()
    }

    pub(crate) fn to_yaml(&self) -> Result<String> {
        let session = self.multiplexer.get_session()?;
        let yaml = serde_yaml::to_string(&session).into_diagnostic()?;

        Ok(yaml)
    }

    pub(crate) fn select_config(&self, show_picker: bool) -> Result<Option<PathBuf>> {
        fn picker(config_path: &str, sessions: &[String]) -> Result<Option<PathBuf>> {
            let configs = fs::read_dir(config_path)
                .into_diagnostic()?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("yaml"))
                .filter_map(|path| {
                    path.file_stem()
                        .and_then(|name| name.to_str())
                        .map(String::from)
                })
                .collect::<Vec<String>>();

            let mut merged: Vec<String> = sessions
                .iter()
                .map(|s| {
                    if configs.contains(s) {
                        format!("{} *", s)
                    } else {
                        s.to_string()
                    }
                })
                .collect();

            merged.extend(
                configs
                    .iter()
                    .filter(|s| !sessions.contains(s))
                    .map(|s| s.to_string()),
            );

            merged.sort();
            merged.dedup();

            let selected = Select::new("Select configuration:", merged)
                .with_page_size(12)
                .prompt();

            match selected {
                Ok(config) => Ok(Some(PathBuf::from(format!(
                    "{}/{}.yaml",
                    &config_path,
                    config.trim_end_matches(" *")
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
                    log::debug!("{}", err);
                    picker(&self.config_path, &self.list()?)
                }
            }
        }
    }
}
