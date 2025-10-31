use std::{env::temp_dir, fs::OpenOptions, io::Write, rc::Rc};

use miette::{bail, IntoDiagnostic, Result};

use crate::{
    app::manager::session::manager::LAIO_CONFIG,
    common::{
        cmd::{Runner, ShellRunner},
        config::Session,
        muxer::{Client, Multiplexer},
        path::{resolve_symlink, sanitize_filename, sanitize_path, to_absolute_path},
        session_info::SessionInfo,
    },
};

use super::client::ZellijClient;
pub(crate) struct Zellij<R: Runner = ShellRunner> {
    client: ZellijClient<R>,
}

impl Zellij {
    pub fn new() -> Self {
        Self::new_with_runner(ShellRunner::new())
    }
}

impl<R: Runner> Zellij<R> {
    pub fn new_with_runner(runner: R) -> Self {
        Self {
            client: ZellijClient::new(Rc::new(runner)),
        }
    }

    fn session_to_layout(&self, cwd: &str, session: &Session, _skip_cmds: bool) -> Result<String> {
        let mut layout_location = temp_dir();
        layout_location.push(format!("{}.kdl", sanitize_filename(&session.name)));
        let layout_location = layout_location.to_str().unwrap().to_string();
        let session_kld = session.as_kdl(cwd)?.to_string();

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&layout_location)
            .into_diagnostic()?;
        let sanitized_kdl = session_kld.to_string();
        file.write_all(sanitized_kdl.as_bytes()).into_diagnostic()?;

        Ok(layout_location)
    }

    fn is_laio_session(&self, name: &str) -> Result<bool> {
        Ok(self.client.getenv(name, LAIO_CONFIG).is_ok())
    }
}

impl<R: Runner> Multiplexer for Zellij<R> {
    fn start(
        &self,
        session: &Session,
        config: &str,
        skip_attach: bool,
        skip_cmds: bool,
    ) -> Result<()> {
        if self.switch(&session.name, skip_attach)? {
            return Ok(());
        }

        if !skip_cmds {
            let commands = if session.startup_script.is_some() {
                let cmd = session.startup_script.clone().unwrap().to_cmd()?;
                &session
                    .startup
                    .clone()
                    .into_iter()
                    .chain(std::iter::once(cmd))
                    .collect()
            } else {
                &session.startup
            };

            self.client.run_commands(commands, &session.path)?;
        }

        let cwd = session
            .windows
            .first()
            .and_then(|window| window.first_leaf_path())
            .map(|path| sanitize_path(path, &session.path))
            .unwrap_or(session.path.clone());

        let layout: String = self.session_to_layout(cwd.as_str(), session, skip_cmds)?;
        let _res: () = self.client.create_session_with_layout(
            &session.name,
            config,
            layout.as_str(),
            skip_attach,
        )?;

        Ok(())
    }

    fn stop(
        &self,
        name: &Option<String>,
        skip_cmds: bool,
        stop_all: bool,
        stop_other: bool,
    ) -> Result<()> {
        let current_session_name = self.client.current_session_name()?;
        log::debug!("Current session name: {current_session_name}");

        if !stop_all && name.is_none() && !self.client.is_inside_session() {
            bail!("Specify laio session you want to stop.");
        }

        if (stop_all || stop_other) && name.is_some() {
            bail!("Stopping all/other and specifying a session name are mutually exclusive.")
        };

        if stop_all || (stop_other && self.client.is_inside_session()) {
            log::trace!("Closing all/other laio sessions.");
            for info in self.list_sessions()?.into_iter() {
                if info.name == current_session_name {
                    log::debug!("Skipping current session: {current_session_name:?}");
                    continue;
                };

                if self.is_laio_session(&info.name)? {
                    log::debug!("Closing session: {:?}", info.name);
                    self.stop(&Some(info.name.to_string()), skip_cmds, false, false)?;
                }
            }
            if !self.client.is_inside_session() {
                log::debug!("Not inside a session");
                return Ok(());
            }
        };

        let name = name.clone().unwrap_or(current_session_name.to_string());
        if !self.client.session_exists(&name) {
            bail!("Session {} does not exist!", &name);
        }
        if !self.is_laio_session(&name)? {
            log::debug!("Not a laio session: {}", &name);
            return Ok(());
        }

        let result = (|| -> Result<()> {
            if !skip_cmds && !stop_other {
                match self.client.getenv(&name, LAIO_CONFIG) {
                    Ok(config) => {
                        log::debug!("Config: {config:?}");

                        let session =
                            Session::from_config(&resolve_symlink(&to_absolute_path(&config)?)?)?;

                        let commands = if session.shutdown_script.is_some() {
                            let cmd = session.shutdown_script.clone().unwrap().to_cmd()?;
                            &session
                                .shutdown
                                .clone()
                                .into_iter()
                                .chain(std::iter::once(cmd))
                                .collect()
                        } else {
                            &session.shutdown
                        };

                        self.client.run_commands(commands, &session.path)
                    }
                    Err(e) => {
                        log::warn!("LAIO_CONFIG environment variable not found: {e:?}");
                        Ok(())
                    }
                }
            } else {
                log::debug!("Skipping shutdown commands for session: {name:?}");
                Ok(())
            }
        })();

        let stop_result = if !stop_other {
            self.client.stop_session(name.as_str())
        } else {
            Ok(())
        };

        result.and(stop_result)
    }

    fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        self.client.list_sessions().map(|sessions| {
            sessions
                .into_iter()
                .map(|(name, is_attached)| SessionInfo::new(name, is_attached))
                .collect()
        })
    }

    fn switch(&self, name: &str, skip_attach: bool) -> Result<bool> {
        if self.client.session_exists(name) {
            log::warn!("Session '{name}' already exists");
            if !skip_attach {
                self.client.attach(name)?;
            }
            return Ok(true);
        }

        Ok(false)
    }

    fn get_session(&self) -> Result<Session> {
        if !self.client.is_inside_session() {
            bail!("You do not seem to be inside a Zellij session.")
        }
        let name = self.client.current_session_name()?;

        let layout_node = self.client.get_layout()?;
        let session = Session::from_kdl(name.as_str(), &layout_node);

        Ok(session)
    }
}
