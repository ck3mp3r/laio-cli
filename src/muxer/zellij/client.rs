use std::{
    env::temp_dir,
    fs::{File, remove_file},
    rc::Rc,
    thread,
    time::Duration,
};

use crate::common::{muxer::client::Client, path::sanitize_filename};
use crate::{
    cmd_basic, cmd_forget,
    common::cmd::{Runner, Type},
};
use kdl::{KdlDocument, KdlNode};
use miette::{IntoDiagnostic, Result, miette};

pub(crate) const DEFAULT_POLL_ATTEMPTS: usize = 40;
pub(crate) const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub(crate) struct ZellijClient<R: Runner> {
    pub cmd_runner: Rc<R>,
    poll_attempts: usize,
    poll_interval: Duration,
}

impl<R: Runner> Client<R> for ZellijClient<R> {
    fn get_runner(&self) -> &R {
        &self.cmd_runner
    }
}

impl<R: Runner> ZellijClient<R> {
    pub(crate) fn new(cmd_runner: Rc<R>) -> Self {
        Self {
            cmd_runner,
            poll_attempts: DEFAULT_POLL_ATTEMPTS,
            poll_interval: DEFAULT_POLL_INTERVAL,
        }
    }

    pub(crate) fn with_poll(mut self, attempts: usize, interval: Duration) -> Self {
        self.poll_attempts = attempts;
        self.poll_interval = interval;
        self
    }

    pub(crate) fn create_session_with_layout(
        &self,
        name: &str,
        env_vars: &[(&str, &str)],
        layout: &str,
    ) -> Result<()> {
        let error_path = zellij_error_path(name);
        let mut cmd = cmd_forget!(
            "sh",
            args = [
                "-c",
                format!(
                    "nohup zellij --session {} --new-session-with-layout {} > /dev/null 2>{} </dev/null & disown ",
                    name, layout, error_path
                )
            ]
        );

        // Apply environment variables to the command
        if let crate::common::cmd::Type::Forget(ref mut command) = cmd {
            for (key, value) in env_vars {
                command.env(key, value);
            }
        }

        self.cmd_runner.run(&cmd)
    }

    pub(crate) fn verify_session(&self, name: &str) -> Result<()> {
        let error_path = zellij_error_path(name);

        for _ in 0..self.poll_attempts {
            if self.session_exists(name) {
                let _ = remove_file(&error_path);
                return Ok(());
            }

            thread::sleep(self.poll_interval);
        }

        let captured: Result<String> = self
            .cmd_runner
            .run(&cmd_basic!("cat", args = [&error_path]));
        let _ = remove_file(&error_path);

        let message = match captured {
            Ok(output) => format!("zellij failed to create session '{name}': {output}"),
            Err(_) => format!(
                "zellij failed to create session '{name}'; could not read zellij stderr from {error_path}"
            ),
        };

        Err(miette!("{message}"))
    }

    pub(crate) fn stop_session(&self, name: &str) -> Result<()> {
        if self.session_exists(name) {
            self.cmd_runner.run(&cmd_basic!(
                "zellij",
                args = ["delete-session", name, "--force"]
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn attach(&self, name: &str) -> Result<()> {
        self.cmd_runner
            .run(&cmd_forget!("zellij", args = ["attach", name]))
    }

    pub(crate) fn session_exists(&self, name: &str) -> bool {
        self.cmd_runner
            .run(&cmd_basic!(
                "sh",
                args = [
                    "-c",
                    format!("zellij list-sessions --short | grep \"{}\"", name)
                ]
            ))
            .unwrap_or(false)
    }

    pub(crate) fn is_inside_session(&self) -> bool {
        self.cmd_runner
            .run(&cmd_basic!("printenv", args = ["ZELLIJ"]))
            .is_ok_and(|s: String| !s.is_empty())
    }

    pub(crate) fn current_session_name(&self) -> Result<String> {
        self.cmd_runner.run(&cmd_basic!(
            "sh",
            args = ["-c", "printenv ZELLIJ_SESSION_NAME || true"]
        ))
    }

    pub(crate) fn getenv(&self, name: &str, key: &str) -> Result<String> {
        if self.is_inside_session() {
            self.cmd_runner.run(&cmd_basic!(
                "sh",
                args = ["-c", format!("printenv {} || true", key)]
            ))
        } else {
            // workaround as zellij does not really support scripting with output to stdout
            let mut temp_path = temp_dir();
            temp_path.push(format!("{}.tmp", sanitize_filename(name)));
            let temp_path_str = temp_path.to_str().unwrap().to_string();
            let _temp_file = File::create(&temp_path).into_diagnostic()?;

            let _res: () = self.cmd_runner.run(&cmd_basic!(
                "zellij",
                args = [
                    "run",
                    "-c",
                    "--name",
                    name,
                    "--",
                    "sh",
                    "-c",
                    format!("\"printenv {} > {}\"", key, &temp_path_str)
                ]
            ))?;
            let result = self
                .cmd_runner
                .run(&cmd_basic!("cat", args = [&temp_path_str]))?;
            remove_file(&temp_path_str).into_diagnostic()?;

            Ok(result)
        }
    }

    pub(crate) fn list_sessions(&self) -> Result<Vec<(String, bool)>> {
        self.cmd_runner
            .run(&cmd_basic!("zellij", args = ["list-sessions", "--short"]))
            .map(|res: String| res.lines().map(|name| (name.to_string(), false)).collect())
            .or_else(|_| Ok(vec![]))
    }

    pub(crate) fn get_layout(&self) -> Result<KdlNode> {
        let res: String = self
            .cmd_runner
            .run(&cmd_basic!("zellij", args = ["action", "dump-layout"]))?;
        let kdl_doc = KdlDocument::parse_v1(res.as_str())?;
        let layout_node = kdl_doc.get("layout").expect("Missing layout node.").clone();
        Ok(layout_node)
    }
}

fn zellij_error_path(name: &str) -> String {
    let mut path = temp_dir();
    path.push(format!("{}.zellij.err", sanitize_filename(name)));
    path.to_string_lossy().to_string()
}
