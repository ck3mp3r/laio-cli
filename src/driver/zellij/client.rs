use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use anyhow::Result;

use crate::{
    cmd_basic, cmd_forget,
    common::{
        cmd::{Runner, Type},
        mux::client::Client,
    },
};

#[derive(Debug)]
pub(crate) struct ZellijClient<R: Runner> {
    pub cmd_runner: Rc<R>,
    pub _cmds: RefCell<VecDeque<Type>>,
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
            _cmds: RefCell::new(VecDeque::new()),
        }
    }
    pub(crate) fn create_session_with_layout(&self, name: &str, layout: &str) -> Result<()> {
        let _res: () = self.cmd_runner.run(&cmd_forget!(
            "zellij --session {} --new-session-with-layout {}",
            name,
            layout
        ))?;
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
