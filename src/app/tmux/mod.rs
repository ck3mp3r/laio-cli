pub(crate) mod target;
use anyhow::{anyhow, Result};
use log::trace;
use serde::Deserialize;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fmt::Debug,
    path::{Path, PathBuf},
    process,
    rc::Rc,
    str::{from_utf8, SplitWhitespace},
};
use termion::terminal_size;

use crate::{
    app::cmd::{Runner, Type},
    cmd_basic, cmd_verbose,
};

use self::target::Target;

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

    pub(crate) fn create_session(&self, session_name: &str, session_path: &str) -> Result<()> {
        let _: () = self.cmd_runner.run(&cmd_basic!(
            "tmux new-session -d -s \"{}\" -c \"{}\"",
            session_name,
            session_path,
        ))?;

        Ok(())
    }

    pub(crate) fn session_exists(&self, name: &str) -> bool {
        self.cmd_runner
            .run(&cmd_basic!("tmux has-session -t \"{}\"", name))
            .unwrap_or(false)
    }

    pub(crate) fn switch_client(&self, name: &str) -> Result<()> {
        self.cmd_runner
            .run(&cmd_basic!("tmux switch-client -t \"{}\"", name))
    }

    pub(crate) fn attach_session(&self, name: &str) -> Result<()> {
        self.cmd_runner
            .run(&cmd_basic!("tmux attach-session -t \"{}\"", name))
    }

    pub(crate) fn is_inside_session(&self) -> bool {
        self.cmd_runner
            .run(&cmd_basic!("printenv TMUX"))
            .is_ok_and(|s: String| !s.is_empty())
    }

    pub(crate) fn current_session_name(&self) -> Result<String> {
        self.cmd_runner.run(&cmd_basic!(
            "[ -n \"$TMUX\" ] && tmux display-message -p '#S' || true"
        ))
    }

    pub(crate) fn stop_session(&self, name: &str) -> Result<()> {
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
    ) -> Result<String> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux new-window -Pd -t \"{}\" -n \"{}\" -c \"{}\" -F \"#{{window_id}}\"",
            session_name,
            window_name,
            path
        ))
    }

    pub(crate) fn get_current_window(&self, session_name: &str) -> Result<String> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux display-message -t \"{}\" -p \"#I\"",
            session_name
        ))
    }

    //pub(crate) fn delete_window(&self, session_name: &str, pos: usize) -> Result<()> {
    //    self.cmd_runner.run(&cmd_basic!(
    //        "tmux kill-window -t \"{}\":{}",
    //        session_name,
    //        pos
    //    ))
    //}
    //
    //pub(crate) fn move_windows(&self, session_name: &str) -> Result<()> {
    //    self.cmd_runner.run(&cmd_basic!(
    //        "tmux move-window -r -s \"{}\" -t \"{}\"",
    //        session_name,
    //        session_name
    //    ))
    //}

    pub(crate) fn split_window(&self, target: &Target, path: &str) -> Result<String> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux split-window -t {} -c \"{}\" -P -F \"#{{pane_id}}\"",
            target,
            path
        ))
    }

    pub(crate) fn get_current_pane(&self, target: &Target) -> Result<String> {
        self.cmd_runner
            .run(&cmd_basic!("tmux display-message -t {} -p \"#P\"", target))
    }

    pub(crate) fn setenv(&self, target: &Target, name: &str, value: &str) {
        self.cmds.borrow_mut().push_back(cmd_basic!(
            "tmux setenv -t {} {} \"{}\"",
            target,
            name,
            value
        ))
    }

    pub(crate) fn getenv(&self, target: &Target, name: &str) -> Result<String> {
        let output: String =
            self.cmd_runner
                .run(&cmd_basic!("tmux show-environment -t {} {}", target, name))?;

        output
            .split_once('=')
            .map(|(_, value)| value.to_string())
            .ok_or_else(|| anyhow!("Variable not found or malformed output"))
    }

    pub(crate) fn register_commands(&self, target: &Target, cmds: &Vec<String>) {
        for cmd in cmds {
            self.register_command(target, cmd)
        }
    }

    pub(crate) fn register_command(&self, target: &Target, cmd: &String) {
        self.cmds
            .borrow_mut()
            .push_back(cmd_basic!("tmux send-keys -t {} '{}' C-m", target, cmd,))
    }

    pub(crate) fn zoom_pane(&self, target: &Target) {
        self.register_command(target, &format!("tmux resize-pane -Z -t {}", target));
    }

    pub(crate) fn flush_commands(&self) -> Result<()> {
        while let Some(cmd) = self.cmds.borrow_mut().pop_front() {
            let _: () = self.cmd_runner.run(&cmd)?;
        }
        Ok(())
    }

    pub(crate) fn select_layout(&self, target: &Target, layout: &str) -> Result<()> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux select-layout -t {} \"{}\"",
            target,
            layout
        ))
    }

    pub(crate) fn select_custom_layout(&self, target: &Target, layout: &str) -> Result<()> {
        self.select_layout(
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

    pub(crate) fn get_dimensions(&self) -> Result<Dimensions> {
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

    pub(crate) fn list_sessions(&self) -> Result<Vec<String>> {
        self.cmd_runner
            .run(&cmd_basic!("tmux ls -F \"#{{session_name}}\""))
            .map(|res: String| res.lines().map(String::from).collect())
            .or_else(|_| Ok(vec![]))
    }

    pub(crate) fn get_base_idx(&self) -> Result<usize> {
        let res: String = self
            .cmd_runner
            .run(&cmd_basic!("tmux show-options -g base-index"))?;
        res.split_whitespace()
            .last()
            .unwrap_or("0")
            .parse()
            .map_err(Into::into)
    }

    pub(crate) fn set_pane_style(&self, target: &Target, style: &str) -> Result<()> {
        self.cmd_runner
            .run(&cmd_basic!("tmux select-pane -t {} -P '{}'", target, style))
    }

    pub(crate) fn bind_key(&self, key: &str, cmd: &str) -> Result<()> {
        self.cmd_runner
            .run(&cmd_basic!("tmux bind-key -T {} {}", &key, &cmd))
    }

    pub(crate) fn run_session_command(&self, cmd: &str) -> Result<String> {
        self.cmd_runner.run(&cmd_verbose!("{}", cmd))
    }

    pub(crate) fn session_name(&self) -> Result<String> {
        self.cmd_runner
            .run(&cmd_basic!("tmux display-message -p \"#S\""))
    }

    pub(crate) fn session_layout(&self) -> Result<String> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux list-windows -F \"#{{window_name}} #{{window_layout}}\""
        ))
    }

    pub(crate) fn rename_window(&self, target: &Target, name: &str) -> Result<()> {
        self.cmd_runner
            .run(&cmd_basic!("tmux rename-window -t {} \"{}\"", target, name,))
    }

    pub(crate) fn session_start_path(&self) -> Result<String> {
        let pane_map: HashMap<String, String> = self.pane_paths()?;
        let pane_paths: Vec<PathBuf> = pane_map.values().map(PathBuf::from).collect();

        if pane_paths.is_empty() {
            return Err(anyhow!("No pane paths found"));
        }

        // Closure to find the longest common path prefix between two paths
        let longest_common_prefix = |path1: &Path, path2: &Path| -> PathBuf {
            let mut prefix = PathBuf::new();
            let mut components1 = path1.components();
            let mut components2 = path2.components();

            while let (Some(c1), Some(c2)) = (components1.next(), components2.next()) {
                if c1 == c2 {
                    prefix.push(c1.as_os_str());
                } else {
                    break;
                }
            }

            prefix
        };

        // Determine the longest common prefix among all paths
        let mut common_prefix = pane_paths[0].clone();

        for path in pane_paths.iter().skip(1) {
            common_prefix = longest_common_prefix(&common_prefix, path);

            // If at any point the common prefix is "/", continue to find more specific common paths
            if common_prefix == Path::new("/") {
                continue;
            }
        }

        // If the longest common prefix is still "/", we should try to find the best common path
        if common_prefix == Path::new("/") {
            let mut best_prefix = PathBuf::new();
            let mut best_count = 0;

            // Compare all pairs to find the best common prefix
            for i in 0..pane_paths.len() {
                for j in i + 1..pane_paths.len() {
                    let prefix = longest_common_prefix(&pane_paths[i], &pane_paths[j]);
                    let count = pane_paths
                        .iter()
                        .filter(|path| path.starts_with(&prefix))
                        .count();

                    if count > best_count && prefix != Path::new("/") {
                        best_prefix = prefix;
                        best_count = count;
                    }
                }
            }

            common_prefix = best_prefix;
        }

        // Return the session root path as a string, ensuring it's never "/"
        if common_prefix.as_os_str().is_empty() || common_prefix == Path::new("/") {
            return Err(anyhow!("No valid session path found"));
        }

        Ok(common_prefix.to_string_lossy().into_owned())
    }

    pub(crate) fn pane_paths(&self) -> Result<HashMap<String, String>> {
        let output: String = self.cmd_runner.run(&cmd_basic!(
            "tmux list-panes -s -F \"#{{pane_id}} #{{pane_current_path}}\""
        ))?;

        let mut pane_map: HashMap<String, String> = HashMap::new();

        for line in output.lines() {
            let mut parts = line.split_whitespace();
            if let (Some(pane_id), Some(pane_path)) = (parts.next(), parts.next()) {
                trace!("pane-path: {}", pane_path.to_string());
                pane_map.insert(pane_id.to_string().replace('%', ""), pane_path.to_string());
            }
        }

        Ok(pane_map)
    }

    pub(crate) fn pane_command(&self) -> Result<HashMap<String, String>> {
        let current_pid: String = process::id().to_string();

        let output: String = self.cmd_runner.run(&cmd_basic!(
            "tmux list-panes -s -F \"#{{pane_id}} #{{pane_pid}}\""
        ))?;

        let mut pane_map: HashMap<String, String> = HashMap::new();

        for line in output.lines() {
            let mut parts: SplitWhitespace = line.split_whitespace();
            let (Some(pane_id), Some(pane_pid_str)) = (parts.next(), parts.next()) else {
                continue;
            };
            let pane_pid: i32 = pane_pid_str.parse()?;
            let child_pids_output: String =
                self.cmd_runner.run(&cmd_basic!("pgrep -P {}", pane_pid))?;

            for child_pid in from_utf8(child_pids_output.as_bytes())
                .unwrap_or("")
                .lines()
                .map(str::trim)
            {
                if child_pid == current_pid {
                    continue;
                }
                let cmd_output: String = self
                    .cmd_runner
                    .run(&cmd_basic!("ps -p {} -o args=", child_pid))?;
                let command: String = from_utf8(cmd_output.as_bytes())
                    .unwrap_or("")
                    .trim()
                    .to_string();

                if command.is_empty() || command.starts_with('-') {
                    continue;
                }
                pane_map.insert(pane_id.to_string().replace('%', ""), command);
            }
        }

        log::trace!("pane-pid-map: {:?}", pane_map);

        Ok(pane_map)
    }
}

#[cfg(test)]
pub mod test;
