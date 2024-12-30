use std::{env::temp_dir, fs::OpenOptions, io::Write, rc::Rc};

use anyhow::{bail, Result};

use crate::{
    app::manager::session::manager::LAIO_CONFIG,
    common::{
        cmd::{Runner, ShellRunner},
        config::Session,
        mux::{Client, Multiplexer},
        path::{resolve_symlink, sanitize_filename, sanitize_path, to_absolute_path},
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
            .open(&layout_location)?;
        let sanitized_kdl = session_kld.to_string();
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

    fn stop(&self, name: &Option<String>, skip_cmds: bool, stop_all: bool) -> Result<()> {
        let current_session_name = self.client.current_session_name()?;
        log::debug!("Current session name: {}", current_session_name);

        if !stop_all && name.is_none() && !self.client.is_inside_session() {
            bail!("Specify laio session you want to stop.");
        }

        if stop_all && name.is_some() {
            bail!("Stopping all and specifying a session name are mutually exclusive.")
        };

        if stop_all {
            log::trace!("Closing all laio sessions.");
            for name in self.list_sessions()?.into_iter() {
                if name == current_session_name {
                    log::debug!("Skipping current session: {:?}", current_session_name);
                    continue;
                };

                if self.is_laio_session(&name)? {
                    log::debug!("Closing session: {:?}", name);
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
                        log::debug!("Config: {:?}", config);

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
                log::debug!("Skipping shutdown commands for session: {:?}", name);
                Ok(())
            }
        })();

        let stop_result = self.client.stop_session(name.as_str()).map_err(Into::into);

        result.and(stop_result)
    }

    fn list_sessions(&self) -> Result<Vec<String>> {
        self.client.list_sessions()
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
        let kdl_doc = self.client.get_layout()?;
        let layout_node = kdl_doc.nodes().first().expect("Missing layout node.");
        let session = Session::from_kdl("foo", layout_node);
        // let tab_nodes = extract_tabs(layout_node);

        // Debugging: Print the structure of the KdlDocument
        println!("{:#?}", session);

        todo!()
    }
}
