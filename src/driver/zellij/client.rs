use std::rc::Rc;

use crate::common::mux::client::Client;
use crate::{
    cmd_basic, cmd_forget,
    common::cmd::{Runner, Type},
};
use anyhow::Result;

#[derive(Debug)]
pub(crate) struct ZellijClient<R: Runner> {
    pub cmd_runner: Rc<R>,
}

impl<R: Runner> Client<R> for ZellijClient<R> {
    fn get_runner(&self) -> &R {
        &self.cmd_runner
    }
}

impl<R: Runner> ZellijClient<R> {
    pub(crate) fn new(cmd_runner: Rc<R>) -> Self {
        Self { cmd_runner }
    }

    pub(crate) fn create_session_with_layout(
        &self,
        name: &str,
        config: &str,
        layout: &str,
        skip_attach: bool,
    ) -> Result<()> {
        let cmd = if skip_attach {
            &cmd_forget!(
                "nohup LAIO_CONFIG={} zellij --session {} --new-session-with-layout {} > /dev/null 2>&1 </dev/null & disown",
              config,
              name,
              layout
            )
        } else {
            &cmd_forget!(
                "LAIO_CONFIG={} zellij --session {} --new-session-with-layout {}",
                config,
                name,
                layout
            )
        };
        self.cmd_runner.run(cmd)
    }

    pub(crate) fn stop_session(&self, name: &str) -> Result<()> {
        self.session_exists(name)
            .then(|| {
                self.cmd_runner
                    .run(&cmd_basic!("zellij delete-session \"{}\" --force", name))
            })
            .unwrap_or(Ok(()))
    }

    pub(crate) fn attach(&self, name: &str) -> Result<()> {
        self.cmd_runner.run(&cmd_forget!("zellij attach {} ", name))
    }

    pub(crate) fn session_exists(&self, name: &str) -> bool {
        self.cmd_runner
            .run(&cmd_basic!("zellij list-sessions | grep \"{}\"", name))
            .unwrap_or(false)
    }

    pub(crate) fn is_inside_session(&self) -> bool {
        self.cmd_runner
            .run(&cmd_basic!("printenv ZELLIJ"))
            .is_ok_and(|s: String| !s.is_empty())
    }

    pub(crate) fn current_session_name(&self) -> Result<String> {
        self.cmd_runner
            .run(&cmd_basic!("printenv ZELLIJ_SESSION_NAME || true"))
    }

    pub(crate) fn getenv(&self, name: &str, key: &str) -> Result<String> {
        if self.is_inside_session() {
            self.cmd_runner.run(&cmd_basic!("printenv {} || true", key))
        } else {
            self.cmd_runner.run(&cmd_basic!(
                "zellij run -c --name {} -- sh -c \"printenv {} > /tmp/laio.env.tmp\" && cat /tmp/laio.env.tmp && rm /tmp/laio.env.tmp",
                name,
                key
            ))
        }
    }
}
