use anyhow::anyhow;
use anyhow::Error;
use serde::Deserialize;
use std::{cell::RefCell, collections::VecDeque, fmt::Debug, rc::Rc};
use termion::terminal_size;

use crate::cmd_basic;

use super::cmd::CmdRunner;
use super::cmd::CommandType;

#[derive(Debug, Deserialize)]
pub(crate) struct Dimensions {
    pub width: usize,
    pub height: usize,
}

#[derive(Debug)]
pub(crate) struct Tmux<R: CmdRunner> {
    pub session_name: String,
    pub session_path: String,
    pub cmd_runner: Rc<R>,
    cmds: RefCell<VecDeque<CommandType>>,
}

impl<R: CmdRunner> Tmux<R> {
    pub(crate) fn new(
        session_name: &Option<String>,
        session_path: &String,
        cmd_runner: Rc<R>,
    ) -> Self {
        Self {
            session_name: match session_name {
                Some(s) => s.clone(),
                None => cmd_runner
                    .run(&cmd_basic!("tmux display-message -p \\#S"))
                    .unwrap_or_else(|_| "laio".to_string()),
            },
            session_path: session_path.clone(),
            cmd_runner,
            cmds: RefCell::new(VecDeque::new()),
        }
    }

    pub(crate) fn create_session(&self, config: &String) -> Result<(), Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux new-session -d -s \"{}\" -c {}",
            self.session_name,
            self.session_path
        ))?;

        self.setenv(&"", "LAIO_CONFIG", config);
        Ok(())
    }

    pub(crate) fn session_exists(&self, name: &str) -> bool {
        self.cmd_runner
            .run(&cmd_basic!("tmux has-session -t \"{}\"", name))
            .unwrap_or(false)
    }

    pub(crate) fn switch_client(&self) -> Result<(), Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux switch-client -t \"{}\"",
            self.session_name
        ))
    }

    pub(crate) fn attach_session(&self) -> Result<(), Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux attach-session -t \"{}\"",
            self.session_name
        ))
    }

    pub(crate) fn is_inside_session(&self) -> bool {
        self.cmd_runner
            .run(&cmd_basic!("printenv TMUX"))
            .map_or(false, |s: String| !s.is_empty())
    }

    pub(crate) fn stop_session(&self, name: &str) -> Result<(), Error> {
        self.session_exists(&name)
            .then(|| {
                self.cmd_runner
                    .run(&cmd_basic!("tmux kill-session -t \"{}\"", name))
            })
            .unwrap_or(Ok(()))
    }

    pub(crate) fn new_window(&self, window_name: &str, path: &str) -> Result<String, Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux new-window -Pd -t \"{}\" -n \"{}\" -c {} -F \"#{{window_id}}\"",
            &self.session_name,
            window_name,
            path
        ))
    }

    pub(crate) fn delete_window(&self, pos: usize) -> Result<(), Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux kill-window -t \"{}\":{}",
            &self.session_name,
            pos
        ))
    }

    pub(crate) fn move_windows(&self) -> Result<(), Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux move-window -r -s \"{}\" -t \"{}\"",
            &self.session_name,
            &self.session_name
        ))
    }

    pub(crate) fn split_window(&self, target: &str, path: &str) -> Result<String, Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux split-window -t \"{}\":{} -c {} -P -F \"#{{pane_id}}\"",
            &self.session_name,
            target,
            path
        ))
    }

    pub(crate) fn get_current_pane(&self, target: &str) -> Result<String, Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux display-message -t \"{}\":{} -p \"#P\"",
            &self.session_name,
            target
        ))
    }

    pub(crate) fn setenv(&self, target: &str, name: &str, value: &str) {
        self.cmds.borrow_mut().push_back(cmd_basic!(
            "tmux setenv -t \"{}\":{} {} \"{}\"",
            self.session_name,
            target,
            name,
            value
        ))
    }

    pub(crate) fn getenv(&self, target: &str, name: &str) -> Result<String, Error> {
        let output: String = self.cmd_runner.run(&cmd_basic!(
            "tmux show-environment -t \"{}\":{} {}",
            &self.session_name,
            target,
            name
        ))?;

        output
            .split_once('=')
            .map(|(_, value)| value.to_string())
            .ok_or_else(|| anyhow!("Variable not found or malformed output"))
    }

    pub(crate) fn register_commands(&self, target: &str, cmds: &Vec<String>) {
        for cmd in cmds {
            self.cmds.borrow_mut().push_back(cmd_basic!(
                "tmux send-keys -t \"{}\":{} '{}' C-m",
                self.session_name,
                target,
                cmd,
            ))
        }
    }

    pub(crate) fn flush_commands(&self) -> Result<(), Error> {
        while let Some(cmd) = self.cmds.borrow_mut().pop_front() {
            self.cmd_runner.run(&cmd)?;
        }
        Ok(())
    }

    pub(crate) fn select_layout(&self, target: &str, layout: &str) -> Result<(), Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux select-layout -t \"{}\":{} \"{}\"",
            &self.session_name,
            &target,
            layout
        ))
    }

    pub(crate) fn select_custom_layout(&self, target: &str, layout: &str) -> Result<(), Error> {
        self.select_layout(
            target,
            &format!("{},{}", self.layout_checksum(&layout), layout),
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
        let res: Result<String, Error> = self
            .cmd_runner
            .run(&cmd_basic!("tmux ls -F \"#{{session_name}}\""));
        match res {
            Ok(res) => Ok(res.lines().map(|line| line.to_string()).collect()),
            Err(error) => {
                log::error!("{}", error);
                Ok(vec![])
            }
        }
    }

    pub(crate) fn get_base_idx(&self) -> Result<usize, Error> {
        let res: String = self
            .cmd_runner
            .run(&cmd_basic!("tmux show-options -g base-index"))?;
        Ok(res.split_whitespace().last().unwrap_or("0").parse()?)
    }

    pub(crate) fn set_pane_style(&self, target: &str, style: &str) -> Result<(), Error> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux select-pane -t \"{}\":{} -P '{}'",
            &self.session_name,
            &target,
            style
        ))
    }

    pub(crate) fn bind_key(&self, key: &str, cmd: &str) -> Result<(), Error> {
        self.cmd_runner
            .run(&cmd_basic!("tmux bind-key -T {} {}", &key, &cmd))
    }
}
