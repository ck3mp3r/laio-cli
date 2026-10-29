use std::{
    env, env::temp_dir, fs::OpenOptions, fs::remove_file, io::Write, rc::Rc, time::Duration,
};

use miette::{IntoDiagnostic, Result, bail};

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

use super::client::{DEFAULT_POLL_ATTEMPTS, DEFAULT_POLL_INTERVAL, ZellijClient};

pub(crate) fn resolve_shell(
    explicit: Option<String>,
    env_shell: Option<String>,
    lookup: impl Fn() -> Option<String>,
) -> String {
    explicit
        .or_else(|| lookup().filter(|shell| !shell.trim().is_empty()))
        .or(env_shell)
        .unwrap_or_else(|| "sh".to_string())
}

fn current_user() -> Option<String> {
    let output = std::process::Command::new("id").arg("-un").output().ok()?;

    if !output.status.success() {
        return None;
    }

    let user = String::from_utf8(output.stdout).ok()?.trim().to_string();

    Some(user).filter(|user| !user.is_empty())
}

fn login_shell() -> Option<String> {
    let user = current_user()?;

    let output = if cfg!(target_os = "macos") {
        std::process::Command::new("dscl")
            .args([".", "-read", &format!("/Users/{user}"), "UserShell"])
            .output()
            .ok()?
    } else {
        std::process::Command::new("getent")
            .args(["passwd", &user])
            .output()
            .ok()?
    };

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;

    let shell = if cfg!(target_os = "macos") {
        stdout.trim().split_once("UserShell:")?.1
    } else {
        stdout.trim().split(':').next_back()?
    };

    Some(shell.trim().to_string())
}

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
        Self::new_with_runner_and_poll(runner, DEFAULT_POLL_ATTEMPTS, DEFAULT_POLL_INTERVAL)
    }

    pub fn new_with_runner_and_poll(runner: R, attempts: usize, interval: Duration) -> Self {
        Self {
            client: ZellijClient::new(Rc::new(runner)).with_poll(attempts, interval),
        }
    }

    fn session_to_layout(&self, cwd: &str, session: &Session, _skip_cmds: bool) -> Result<String> {
        let mut layout_location = temp_dir();
        layout_location.push(format!("{}.kdl", sanitize_filename(&session.name)));
        let layout_location = layout_location.to_str().unwrap().to_string();
        let shell = resolve_shell(session.shell.clone(), env::var("SHELL").ok(), login_shell);
        let session_kld = session.as_kdl(cwd, &shell)?.to_string();

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
        env_vars: &[(&str, &str)],
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
        let started = self
            .client
            .create_session_with_layout(&session.name, env_vars, layout.as_str())
            .and_then(|()| self.client.verify_session(&session.name));
        let _ = remove_file(&layout);
        started?;

        if !skip_attach {
            self.client.attach(&session.name)?;
        }

        Ok(())
    }

    fn stop(
        &self,
        name: &Option<String>,
        session: &Option<Session>,
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
            bail!("Stopping all/other and specifying a session name are mutually exclusive.");
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
                    self.stop(&Some(info.name.to_string()), &None, skip_cmds, false, false)?;
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
            log::debug!("Not a laio session: {}", name);
            return Ok(());
        }

        let result = (|| -> Result<()> {
            if !skip_cmds && !stop_other {
                // If session was provided (with variables), use it directly
                if let Some(sess) = session {
                    let commands = if sess.shutdown_script.is_some() {
                        let cmd = sess.shutdown_script.clone().unwrap().to_cmd()?;
                        &sess
                            .shutdown
                            .clone()
                            .into_iter()
                            .chain(std::iter::once(cmd))
                            .collect()
                    } else {
                        &sess.shutdown
                    };
                    return self.client.run_commands(commands, &sess.path);
                }

                // Otherwise, try to load from LAIO_CONFIG (backward compatibility)
                match self.client.getenv(&name, LAIO_CONFIG) {
                    Ok(config) => {
                        log::debug!("Config: {config:?}");

                        // Try to retrieve stored variables for templated configs
                        let variables = self.get_session_variables(&name)?;

                        let sess = Session::from_config(
                            &resolve_symlink(&to_absolute_path(&config)?)?,
                            variables.as_deref(),
                        )?;

                        let commands = if sess.shutdown_script.is_some() {
                            let cmd = sess.shutdown_script.clone().unwrap().to_cmd()?;
                            &sess
                                .shutdown
                                .clone()
                                .into_iter()
                                .chain(std::iter::once(cmd))
                                .collect()
                        } else {
                            &sess.shutdown
                        };

                        self.client.run_commands(commands, &sess.path)
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

    fn get_session_config_path(&self, name: &str) -> Result<Option<String>> {
        match self.client.getenv(name, LAIO_CONFIG) {
            Ok(config_path) => Ok(Some(config_path)),
            Err(_) => Ok(None),
        }
    }

    fn get_session_variables(&self, name: &str) -> Result<Option<Vec<String>>> {
        use crate::app::manager::session::manager::{LAIO_VARS, decode_variables};

        match self.client.getenv(name, LAIO_VARS) {
            Ok(encoded) => {
                if encoded.is_empty() {
                    Ok(Some(Vec::new()))
                } else {
                    match decode_variables(&encoded) {
                        Ok(vars) => Ok(Some(vars)),
                        Err(e) => {
                            log::warn!(
                                "Failed to decode LAIO_VARS for session '{}': {:?}",
                                name,
                                e
                            );
                            Ok(None)
                        }
                    }
                }
            }
            Err(_) => Ok(None),
        }
    }

    fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        self.client.list_sessions().map(|sessions| {
            sessions
                .into_iter()
                .map(|(name, is_attached)| SessionInfo::active(name, is_attached))
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
            bail!("You do not seem to be inside a Zellij session.");
        }
        let name = self.client.current_session_name()?;

        let layout_node = self.client.get_layout()?;
        let session = Session::from_kdl(name.as_str(), &layout_node);

        Ok(session)
    }
}
