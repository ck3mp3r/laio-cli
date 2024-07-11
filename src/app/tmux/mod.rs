use anyhow::{anyhow, Error, Result};
use serde::Deserialize;
use std::{cell::RefCell, collections::VecDeque, fmt::Debug, rc::Rc};
use termion::terminal_size;

use crate::{
    app::cmd::{Runner, Type},
    cmd_basic, cmd_verbose,
};

#[derive(Debug, Deserialize)]
pub(crate) struct Dimensions {
    pub width: usize,
    pub height: usize,
}

#[derive(Debug)]
pub(crate) struct Client<R: Runner> {
    pub cmd_runner: Rc<R>,
    cmds: RefCell<VecDeque<Type>>,
}

impl<R: Runner> Client<R> {
    pub(crate) fn new(cmd_runner: Rc<R>) -> Self {
        Self {
            cmd_runner,
            cmds: RefCell::new(VecDeque::new()),
        }
    }

    pub(crate) fn create_session(
        &self,
        session_name: &str,
        session_path: &str,
        config: &str,
    ) -> Result<(), Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux new-session -d -s \"{}\" -c \"{}\"",
            session_name,
            session_path,
        ))?;

        self.setenv(session_name, "", "LAIO_CONFIG", config);
        Ok(())
    }

    pub(crate) fn session_exists(&self, name: &str) -> bool {
        self.cmd_runner
            .run(&cmd_basic!("tmux has-session -t \"{}\"", name))
            .unwrap_or(false)
    }

    pub(crate) fn switch_client(&self, name: &str) -> Result<(), Error> {
        self.cmd_runner
            .run(&cmd_basic!("tmux switch-client -t \"{}\"", name))
    }

    pub(crate) fn attach_session(&self, name: &str) -> Result<(), Error> {
        self.cmd_runner
            .run(&cmd_basic!("tmux attach-session -t \"{}\"", name))
    }

    pub(crate) fn is_inside_session(&self) -> bool {
        self.cmd_runner
            .run(&cmd_basic!("printenv TMUX"))
            .map_or(false, |s: String| !s.is_empty())
    }

    pub(crate) fn current_session_name(&self) -> Result<String, Error> {
        self.cmd_runner.run(&cmd_basic!(
            "[ -n \"$TMUX\" ] && tmux display-message -p '#S' || true"
        ))
    }

    pub(crate) fn stop_session(&self, name: &str) -> Result<(), Error> {
        self.session_exists(name)
            .then(|| {
                self.cmd_runner
                    .run(&cmd_basic!("tmux kill-session -t \"{}\"", name))
            })
            .unwrap_or(Ok(()))
    }

    pub(crate) fn new_window(
        &self,
        session_name: &str,
        window_name: &str,
        path: &str,
    ) -> Result<String, Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux new-window -Pd -t \"{}\" -n \"{}\" -c \"{}\" -F \"#{{window_id}}\"",
            session_name,
            window_name,
            path
        ))
    }

    pub(crate) fn get_current_window(&self, session_name: &str) -> Result<String, Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux display-message -t \"{}\" -p \"#I\"",
            session_name
        ))
    }

    //pub(crate) fn delete_window(&self, session_name: &str, pos: usize) -> Result<(), Error> {
    //    self.cmd_runner.run(&cmd_basic!(
    //        "tmux kill-window -t \"{}\":{}",
    //        session_name,
    //        pos
    //    ))
    //}
    //
    //pub(crate) fn move_windows(&self, session_name: &str) -> Result<(), Error> {
    //    self.cmd_runner.run(&cmd_basic!(
    //        "tmux move-window -r -s \"{}\" -t \"{}\"",
    //        session_name,
    //        session_name
    //    ))
    //}

    pub(crate) fn split_window(
        &self,
        session_name: &str,
        target: &str,
        path: &str,
    ) -> Result<String, Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux split-window -t \"{}\":{} -c \"{}\" -P -F \"#{{pane_id}}\"",
            session_name,
            target,
            path
        ))
    }

    pub(crate) fn get_current_pane(
        &self,
        session_name: &str,
        target: &str,
    ) -> Result<String, Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux display-message -t \"{}\":{} -p \"#P\"",
            session_name,
            target
        ))
    }

    pub(crate) fn setenv(&self, session_name: &str, target: &str, name: &str, value: &str) {
        self.cmds.borrow_mut().push_back(cmd_basic!(
            "tmux setenv -t \"{}\":{} {} \"{}\"",
            session_name,
            target,
            name,
            value
        ))
    }

    pub(crate) fn getenv(&self, session_name: &str, target: &str, name: &str) -> Result<String> {
        let output: String = self.cmd_runner.run(&cmd_basic!(
            "tmux show-environment -t \"{}\":{} {}",
            session_name,
            target,
            name
        ))?;

        output
            .split_once('=')
            .map(|(_, value)| value.to_string())
            .ok_or_else(|| anyhow!("Variable not found or malformed output"))
    }

    pub(crate) fn register_commands(&self, session_name: &str, target: &str, cmds: &Vec<String>) {
        for cmd in cmds {
            self.register_command(session_name, target, cmd)
        }
    }

    pub(crate) fn register_command(&self, session_name: &str, target: &str, cmd: &String) {
        self.cmds.borrow_mut().push_back(cmd_basic!(
            "tmux send-keys -t \"{}\":{} '{}' C-m",
            session_name,
            target,
            cmd,
        ))
    }

    pub(crate) fn zoom_pane(&self, session_name: &str, target: &str) {
        self.register_command(
            session_name,
            target,
            &format!("tmux resize-pane -Z -t \"{}\":{}", session_name, target),
        );
    }

    pub(crate) fn flush_commands(&self) -> Result<(), Error> {
        while let Some(cmd) = self.cmds.borrow_mut().pop_front() {
            self.cmd_runner.run(&cmd)?;
        }
        Ok(())
    }

    pub(crate) fn select_layout(
        &self,
        session_name: &str,
        target: &str,
        layout: &str,
    ) -> Result<(), Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux select-layout -t \"{}\":{} \"{}\"",
            session_name,
            &target,
            layout
        ))
    }

    pub(crate) fn select_custom_layout(
        &self,
        session_name: &str,
        target: &str,
        layout: &str,
    ) -> Result<(), Error> {
        self.select_layout(
            session_name,
            target,
            &format!("{},{}", self.layout_checksum(layout), layout),
        )
    }

    pub(crate) fn layout_checksum(&self, layout: &str) -> String {
        let mut csum: u16 = 0;
        for &c in layout.as_bytes() {
            csum = (csum >> 1) | ((csum & 1) << 15);
            csum = csum.wrapping_add(c as u16);
        }
        format!("{:04x}", csum)
    }

    pub(crate) fn get_dimensions(&self) -> Result<Dimensions, Error> {
        let res: String = if self.is_inside_session() {
            log::debug!("Inside session, using tmux dimensions.");
            self.cmd_runner.run(&cmd_basic!(
                "tmux display-message -p \"width: #{{window_width}}\nheight: #{{window_height}}\""
            ))?
        } else {
            log::debug!("Outside session, using terminal dimensions.");
            let (width, height) = terminal_size()?;
            format!("width: {}\nheight: {}", width, height)
        };

        log::trace!("{}", &res);
        Ok(serde_yaml::from_str(&res)?)
    }

    pub(crate) fn list_sessions(&self) -> Result<Vec<String>, Error> {
        self.cmd_runner
            .run(&cmd_basic!("tmux ls -F \"#{{session_name}}\""))
            .map(|res: String| res.lines().map(|line| line.to_string()).collect())
            .or(Ok(vec![]))
    }

    pub(crate) fn get_base_idx(&self) -> Result<usize, Error> {
        let res: String = self
            .cmd_runner
            .run(&cmd_basic!("tmux show-options -g base-index"))?;
        Ok(res.split_whitespace().last().unwrap_or("0").parse()?)
    }

    pub(crate) fn set_pane_style(
        &self,
        session_name: &str,
        target: &str,
        style: &str,
    ) -> Result<(), Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux select-pane -t \"{}\":{} -P '{}'",
            session_name,
            &target,
            style
        ))
    }

    pub(crate) fn bind_key(&self, key: &str, cmd: &str) -> Result<(), Error> {
        self.cmd_runner
            .run(&cmd_basic!("tmux bind-key -T {} {}", &key, &cmd))
    }

    pub(crate) fn run_session_command(&self, cmd: &str) -> Result<String, Error> {
        self.cmd_runner.run(&cmd_verbose!("{}", cmd))
    }

    pub(crate) fn session_name(&self) -> Result<String, Error> {
        self.cmd_runner
            .run(&cmd_basic!("tmux display-message -p \"#S\""))
    }

    pub(crate) fn session_layout(&self) -> Result<String, Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux list-windows -F \"#{{window_name}} #{{window_layout}}\""
        ))
    }

    pub(crate) fn rename_window(&self, session_name: &str, target: &str, name: &str) -> Result<()> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux rename-window -t \"{}\":{} \"{}\"",
            session_name,
            target,
            name,
        ))
    }
}

#[cfg(test)]
pub mod test;
