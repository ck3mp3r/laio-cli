use std::rc::Rc;

use crate::common::{
    cmd::{Runner, ShellRunner},
    mux::Multiplexer,
};

use super::client::Client;

pub(crate) struct Zellij<R: Runner = ShellRunner> {
    client: Client<R>,
}

impl Zellij {
    pub fn new() -> Self {
        Self::new_with_runner(ShellRunner::new())
    }
}

impl<R: Runner> Zellij<R> {
    pub fn new_with_runner(runner: R) -> Self {
        Self {
            client: Client::new(Rc::new(runner)),
        }
    }
}

impl<R: Runner> Multiplexer for Zellij<R> {
    fn start(
        &self,
        session: &crate::common::config::Session,
        config: &str,
        skip_attach: bool,
        skip_cmds: bool,
    ) -> anyhow::Result<()> {
        todo!()
    }

    fn stop(&self, name: &Option<String>, skip_cmds: bool, stop_all: bool) -> anyhow::Result<()> {
        todo!()
    }

    fn list_sessions(&self) -> anyhow::Result<Vec<String>> {
        todo!()
    }

    fn switch(&self, name: &str, skip_attach: bool) -> anyhow::Result<bool> {
        todo!()
    }

    fn get_session(&self) -> anyhow::Result<crate::common::config::Session> {
        todo!()
    }
}
