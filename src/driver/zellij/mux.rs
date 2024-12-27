use std::{env::temp_dir, fs::OpenOptions, io::Write, rc::Rc};

use anyhow::{bail, Result};

use crate::{
    app::manager::session::manager::LAIO_CONFIG,
    common::{
        cmd::{Runner, ShellRunner},
        config::Session,
        mux::{Client, Multiplexer},
        path::{resolve_symlink, to_absolute_path},
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

    fn session_to_layout(&self, session: &Session, _skip_cmds: bool) -> Result<String> {
        let mut layout_location = temp_dir();
        layout_location.push(format!("{}.kdl", &session.name));
        let layout_location = layout_location.to_str().unwrap().to_string();
        let session_kld = session.as_kdl()?.to_string();

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&layout_location)?;
        let sanitized_kdl = session_kld.to_string().replace(" %", "");
        file.write_all(sanitized_kdl.as_bytes())?;

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
            self.client.run_commands(&session.startup, &session.path)?;
        }

        let layout: String = self.session_to_layout(session, skip_cmds)?;
        let _res: () = self.client.create_session_with_layout(
            &session.name,
            config,
            layout.as_str(),
            skip_attach,
        )?;

        Ok(())
    }

    fn stop(&self, name: &Option<String>, skip_cmds: bool, stop_all: bool) -> Result<()> {
        let current_session_name = self.client.current_session_name()?;
        log::trace!("Current session name: {}", current_session_name);

        if !stop_all && name.is_none() && !self.client.is_inside_session() {
            bail!("Specify laio session you want to stop.");
        }

        if stop_all && name.is_some() {
            bail!("Stopping all and specifying a session name are mutually exclusive.")
        };

        if stop_all {
            // stops all other laio sessions
            log::trace!("Closing all laio sessions.");
            for name in self.list_sessions()?.into_iter() {
                if name == current_session_name {
                    log::trace!("Skipping current session: {:?}", current_session_name);
                    continue;
                };

                if self.is_laio_session(&name)? {
                    log::trace!("Closing session: {:?}", name);
                    self.stop(&Some(name.to_string()), skip_cmds, false)?;
                }
            }
            if !self.client.is_inside_session() {
                log::debug!("Not inside a session");
                return Ok(());
            }
        };

        let name = name.clone().unwrap_or(current_session_name.to_string());
        let result = (|| -> Result<()> {
            if !skip_cmds {
                // checking if session is managed by laio
                match self.client.getenv(&name, LAIO_CONFIG) {
                    Ok(config) => {
                        log::trace!("Config: {:?}", config);

                        let session =
                            Session::from_config(&resolve_symlink(&to_absolute_path(&config)?)?)?;
                        self.client.run_commands(&session.shutdown, &session.path)
                    }
                    Err(e) => {
                        log::warn!("LAIO_CONFIG environment variable not found: {:?}", e);
                        Ok(())
                    }
                }
            } else {
                log::trace!("Skipping shutdown commands for session: {:?}", name);
                Ok(())
            }
        })();

        let stop_result = self.client.stop_session(name.as_str()).map_err(Into::into);

        result.and(stop_result)
    }

    fn list_sessions(&self) -> Result<Vec<String>> {
        todo!()
    }

    fn switch(&self, name: &str, skip_attach: bool) -> Result<bool> {
        if self.client.session_exists(name) {
            log::warn!("Session '{}' already exists", name);
            if !skip_attach {
                self.client.attach(name)?;
            }
            return Ok(true);
        }

        Ok(false)
    }

    fn get_session(&self) -> Result<Session> {
        todo!()
    }
}
