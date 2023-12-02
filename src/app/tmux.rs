use serde::Deserialize;

use std::{cell::RefCell, collections::VecDeque, fmt::Debug, rc::Rc};
use termion::terminal_size;

use super::cmd::CmdRunner;

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
    cmds: RefCell<VecDeque<String>>,
}

impl<R: CmdRunner> Tmux<R> {
    pub(crate) fn new(
        session_name: &Option<String>,
        session_path: &Option<String>,
        cmd_runner: Rc<R>,
    ) -> Self {
        Self {
            session_name: match session_name {
                Some(s) => s.clone(),
                None => cmd_runner
                    .run(&format!("tmux display-message -p \\#S"))
                    .unwrap_or_else(|_| "laio".to_string()),
            },
            session_path: match session_path {
                Some(s) => s.clone(),
                None => cmd_runner
                    .run(&format!(
                        "tmux display-message -p \"#{{session_base_path}}\""
                    ))
                    .unwrap_or_else(|_| ".".to_string()),
            },
            cmd_runner,
            cmds: RefCell::new(VecDeque::new()),
        }
    }

    pub(crate) fn create_session(&self) -> Result<(), anyhow::Error> {
        self.cmd_runner.run(&format!(
            "tmux new-session -d -s {} -c {}",
            self.session_name, self.session_path
        ))
    }

    pub(crate) fn session_exists(&self, name: &str) -> bool {
        self.cmd_runner
            .run(&format!("tmux has-session -t {}", name))
            .unwrap_or(false)
    }

    pub(crate) fn switch_client(&self) -> Result<(), anyhow::Error> {
        self.cmd_runner
            .run(&format!("tmux switch-client -t {}", self.session_name))
    }

    pub(crate) fn attach_session(&self) -> Result<(), anyhow::Error> {
        self.cmd_runner
            .run(&format!("tmux attach-session -t {}", self.session_name))
    }

    pub(crate) fn is_inside_session(&self) -> bool {
        self.cmd_runner
            .run(&format!("printenv TMUX"))
            .map_or(false, |s: String| !s.is_empty())
    }

    pub(crate) fn stop_session(&self, name: &str) -> Result<(), anyhow::Error> {
        self.session_exists(&name)
            .then(|| {
                self.cmd_runner
                    .run(&format!("tmux kill-session -t {}", name))
            })
            .unwrap_or(Ok(()))
    }

    pub(crate) fn new_window(
        &self,
        window_name: &String,
        path: &String,
    ) -> Result<String, anyhow::Error> {
        self.cmd_runner.run(&format!(
            "tmux new-window -Pd -t {} -n {} -c {} -F \"#{{window_id}}\"",
            &self.session_name, window_name, path
        ))
    }

    pub(crate) fn delete_window(&self, pos: usize) -> Result<(), anyhow::Error> {
        self.cmd_runner.run(&format!(
            "tmux kill-window -t {}:{}",
            &self.session_name, pos
        ))
    }

    pub(crate) fn move_windows(&self) -> Result<(), anyhow::Error> {
        self.cmd_runner.run(&format!(
            "tmux move-window -r -s {} -t {}",
            &self.session_name, &self.session_name
        ))
    }

    pub(crate) fn split_window(
        &self,
        target: &String,
        path: &String,
    ) -> Result<String, anyhow::Error> {
        self.cmd_runner.run(&format!(
            "tmux split-window -t {}:{} -c {} -P -F \"#{{pane_id}}\"",
            &self.session_name, target, path
        ))
    }

    pub(crate) fn get_current_pane(&self, target: &String) -> Result<String, anyhow::Error> {
        self.cmd_runner.run(&format!(
            "tmux display-message -t {}:{} -p \"#P\"",
            &self.session_name, target
        ))
    }

    pub(crate) fn register_commands(&self, target: &String, cmds: &Vec<String>) {
        for cmd in cmds {
            self.cmds.borrow_mut().push_back(format!(
                "tmux send-keys -t {}:{} '{}' C-m",
                self.session_name, target, cmd,
            ))
        }
    }

    pub(crate) fn flush_commands(&self) -> Result<(), anyhow::Error> {
        while let Some(cmd) = self.cmds.borrow_mut().pop_front() {
            self.cmd_runner.run(&cmd)?;
        }
        Ok(())
    }

    pub(crate) fn select_layout(
        &self,
        target: &String,
        layout: &String,
    ) -> Result<(), anyhow::Error> {
        self.cmd_runner.run(&format!(
            "tmux select-layout -t {}:{} \"{}\"",
            &self.session_name, &target, layout
        ))
    }

    pub(crate) fn layout_checksum(&self, layout: &String) -> String {
        let mut csum: u16 = 0;
        for &c in layout.as_bytes() {
            csum = (csum >> 1) | ((csum & 1) << 15);
            csum = csum.wrapping_add(c as u16);
        }
        format!("{:04x}", csum)
    }

    pub(crate) fn get_dimensions(&self) -> Result<Dimensions, anyhow::Error> {
        let res: String = if self.is_inside_session() {
            log::info!("Inside session, using tmux dimensions.");
            self.cmd_runner.run(&format!(
                "tmux display-message -p \"width: #{{window_width}}\nheight: #{{window_height}}\""
            ))?
        } else {
            log::info!("Outside session, using terminal dimensions.");
            let (width, height) = terminal_size()?;
            format!("width: {}\nheight: {}", width, height)
        };

        log::debug!("{}", &res);
        Ok(serde_yaml::from_str(&res)?)
    }

    pub(crate) fn list_sessions(&self) -> Result<Vec<String>, anyhow::Error> {
        let res: String = self
            .cmd_runner
            .run(&"tmux ls -F \"#{session_name}\"".to_string())?;
        Ok(res.lines().map(|line| line.to_string()).collect())
    }

    pub(crate) fn get_base_idx(&self) -> Result<usize, anyhow::Error> {
        let res: String = self
            .cmd_runner
            .run(&"tmux show-options -g base-index".to_string())?;
        Ok(res.split_whitespace().last().unwrap_or("0").parse()?)
    }
}

#[cfg(test)]
mod test {
    use std::rc::Rc;

    use crate::app::{cmd::test::MockCmdRunner, tmux::Tmux};

    #[test]
    fn new_session() -> Result<(), anyhow::Error> {
        let mock_cmd_runner = Rc::new(MockCmdRunner::new());
        let tmux = Tmux::new(
            &Some(String::from("test")),
            &Some(String::from("/tmp")),
            Rc::clone(&mock_cmd_runner),
        );

        tmux.create_session()?;
        tmux.new_window(&"test".to_string(), &"/tmp".to_string())?;
        tmux.select_layout(&"@1".to_string(), &"main-horizontal".to_string())?;

        let cmds = tmux.cmd_runner.get_cmds();
        assert_eq!(cmds[0], "tmux new-session -d -s test -c /tmp");
        assert_eq!(
            cmds[1],
            "tmux new-window -Pd -t test -n test -c /tmp -F \"#{window_id}\""
        );
        assert_eq!(cmds[2], "tmux select-layout -t test:@1 \"main-horizontal\"");
        Ok(())
    }
}
