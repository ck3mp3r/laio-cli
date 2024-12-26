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
        layout: &str,
        skip_attach: bool,
    ) -> Result<()> {
        let cmd = if skip_attach {
            &cmd_forget!(
                "nohup zellij --session {} --new-session-with-layout {} > /dev/null 2>&1 </dev/null & disown",
                name,
                layout
            )
        } else {
            &cmd_forget!(
                "zellij --session {} --new-session-with-layout {}",
                name,
                layout
            )
        };
        let _res: () = self.cmd_runner.run(cmd)?;
        Ok(())
    }

    pub(crate) fn attach(&self, name: &str) -> Result<()> {
        let _res: () = self
            .cmd_runner
            .run(&cmd_forget!("zellij attach {} ", name))?;
        Ok(())
    }

    pub(crate) fn session_exists(&self, name: &str) -> bool {
        self.cmd_runner
            .run(&cmd_basic!("zellij list-sessions | grep \"{}\"", name))
            .unwrap_or(false)
    }
}
