use crossterm::terminal::size;
use log::trace;
use miette::{bail, miette, IntoDiagnostic, Result};
use serde::Deserialize;
use serde_yaml::from_str;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fmt::Debug,
    path::{Path, PathBuf},
    process::{self, Command},
    sync::Arc,
};
use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::{
    cmd_basic,
    common::{
        cmd::{Runner, Type},
        config::Command as ConfigCommand,
        muxer::Client,
    },
};

use super::Target;

#[derive(Debug, Deserialize)]
pub(crate) struct Dimensions {
    pub width: usize,
    pub height: usize,
}

#[derive(Debug)]
pub(crate) struct TmuxClient<R: Runner> {
    pub cmd_runner: Arc<R>,
    pub cmds: RefCell<HashMap<Target, VecDeque<Type>>>,
}

impl<R: Runner> Client<R> for TmuxClient<R> {
    fn get_runner(&self) -> &R {
        &self.cmd_runner
    }
}

impl<R: Runner> TmuxClient<R> {
    pub(crate) fn new(cmd_runner: Arc<R>) -> Self {
        Self {
            cmd_runner,
            cmds: RefCell::new(HashMap::new()),
        }
    }

    pub(crate) fn create_session(
        &self,
        session_name: &str,
        session_path: &str,
        env: &HashMap<String, String>,
        shell: &Option<String>,
    ) -> Result<()> {
        let mut args = vec!["new-session", "-d", "-s", session_name, "-c", session_path];

        let env_args: Vec<String> = env
            .iter()
            .flat_map(|(key, value)| vec!["-e".to_string(), format!("{}={}", key, value)])
            .collect();

        args.extend(env_args.iter().map(|s| s.as_str()));

        let _: () = self.cmd_runner.run(&Type::Basic({
            let mut command = Command::new("tmux");
            command.args(args);
            command
        }))?;

        if let Some(shell_path) = shell {
            let _: () = self.cmd_runner.run(&cmd_basic!(
                "tmux",
                args = [
                    "set-option",
                    "-t",
                    session_name,
                    "default-shell",
                    &shell_path
                ]
            ))?;
        }
        Ok(())
    }

    pub(crate) fn session_exists(&self, name: &str) -> bool {
        self.cmd_runner
            .run(&cmd_basic!("tmux", args = ["has-session", "-t", name]))
            .unwrap_or(false)
    }

    pub(crate) fn switch_client(&self, name: &str) -> Result<()> {
        self.cmd_runner
            .run(&cmd_basic!("tmux", args = ["switch-client", "-t", name]))
    }

    pub(crate) fn attach_session(&self, name: &str) -> Result<()> {
        self.cmd_runner
            .run(&cmd_basic!("tmux", args = ["attach-session", "-t", name]))
    }

    pub(crate) fn is_inside_session(&self) -> bool {
        self.cmd_runner
            .run(&cmd_basic!("printenv", args = ["TMUX"]))
            .is_ok_and(|s: String| !s.is_empty())
    }

    pub(crate) fn current_session_name(&self) -> Result<String> {
        self.cmd_runner.run(&cmd_basic!(
            "sh",
            args = [
                "-c",
                "[ -n \"$TMUX\" ] && tmux display-message -p '#S' || true"
            ]
        ))
    }

    pub(crate) fn stop_session(&self, name: &str) -> Result<()> {
        if self.session_exists(name) {
            self.cmd_runner
                .run(&cmd_basic!("tmux", args = ["kill-session", "-t", name]))
        } else {
            Ok(())
        }
    }

    pub(crate) fn new_window(
        &self,
        session_name: &str,
        window_name: &str,
        path: &str,
    ) -> Result<String> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux",
            args = [
                "new-window",
                "-Pd",
                "-t",
                session_name,
                "-n",
                window_name,
                "-c",
                path,
                "-F",
                "#{window_id}"
            ]
        ))
    }

    pub(crate) fn get_current_window(&self, session_name: &str) -> Result<String> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux",
            args = ["display-message", "-t", session_name, "-p", "#I"]
        ))
    }

    pub(crate) fn split_window(&self, target: &Target, path: &str) -> Result<String> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux",
            args = [
                "split-window",
                "-t",
                target.to_string(),
                "-c",
                path,
                "-P",
                "-F",
                "#{pane_id}"
            ]
        ))
    }

    pub(crate) fn get_current_pane(&self, target: &Target) -> Result<String> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux",
            args = ["display-message", "-t", target.to_string(), "-p", "#P"]
        ))
    }

    pub(crate) fn setenv(&self, target: &Target, name: &str, value: &str) {
        self.cmds
            .borrow_mut()
            .entry(target.clone())
            .or_default()
            .push_back(cmd_basic!(
                "tmux",
                args = ["set-environment", "-t", target.to_string(), name, value]
            ))
    }

    pub(crate) fn getenv(&self, target: &Target, name: &str) -> Result<String> {
        let output: String = self.cmd_runner.run(&cmd_basic!(
            "tmux",
            args = ["show-environment", "-t", target.to_string(), name]
        ))?;
        output
            .trim()
            .split_once('=')
            .map(|(_, value)| value.to_string())
            .ok_or_else(|| miette!("Variable not found or malformed output"))
    }

    pub(crate) fn register_commands(&self, target: &Target, cmds: &[ConfigCommand]) {
        let cmd_strings: Vec<String> = cmds.iter().map(|cmd| cmd.to_string()).collect();
        self.register_command(target, cmd_strings);
    }

    pub(crate) fn register_command(&self, target: &Target, cmds: Vec<String>) {
        if cmds.is_empty() {
            return;
        }

        // Build command: tmux send-keys -t target cmd1 C-m cmd2 C-m ...
        let mut command = Command::new("tmux");
        command.arg("send-keys").arg("-t").arg(target.to_string());

        // Interleave commands with C-m using flat_map
        for cmd in cmds.iter().flat_map(|cmd| [cmd.as_str(), "C-m"]) {
            command.arg(cmd);
        }

        self.cmds
            .borrow_mut()
            .entry(target.clone())
            .or_default()
            .push_back(Type::Basic(command))
    }

    pub(crate) fn zoom_pane(&self, target: &Target) {
        self.cmds
            .borrow_mut()
            .entry(target.clone())
            .or_default()
            .push_back(cmd_basic!(
                "tmux",
                args = ["resize-pane", "-Z", "-t", target.to_string()]
            ))
    }

    pub(crate) fn focus_pane(&self, target: &Target) {
        self.cmds
            .borrow_mut()
            .entry(target.clone())
            .or_default()
            .push_back(cmd_basic!(
                "tmux",
                args = ["select-pane", "-Z", "-t", target.to_string()]
            ))
    }

    pub(crate) fn flush_commands(&self) {
        let pane_commands: HashMap<Target, VecDeque<Type>> =
            self.cmds.borrow_mut().drain().collect();

        if pane_commands.is_empty() {
            return;
        }

        log::debug!("Flushing commands for {} panes", pane_commands.len());

        for (target, commands) in pane_commands {
            if commands.is_empty() {
                continue;
            }

            log::debug!(
                "Executing {} batched commands for pane {}",
                commands.len(),
                target
            );

            // Execute all commands synchronously (they're already batched)
            for cmd in commands {
                let _: () = self.cmd_runner.run(&cmd).unwrap_or_else(|e| {
                    log::warn!("Command execution failed for pane {}: {:?}", target, e);
                });
            }
        }
    }

    pub(crate) fn select_layout(&self, target: &Target, layout: &str) -> Result<()> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux",
            args = ["select-layout", "-t", target.to_string(), layout]
        ))
    }

    pub(crate) fn select_custom_layout(&self, target: &Target, layout: &str) -> Result<()> {
        self.select_layout(
            target,
            &format!("{},{}", self.layout_checksum(layout), layout),
        )
    }

    pub(crate) fn layout_checksum(&self, layout: &str) -> String {
        let csum = layout.as_bytes().iter().fold(0u16, |csum, &c| {
            let rotated = (csum >> 1) | ((csum & 1) << 15);
            rotated.wrapping_add(c as u16)
        });
        format!("{csum:04x}")
    }

    pub(crate) fn get_dimensions(&self) -> Result<Dimensions> {
        let res: String = if self.is_inside_session() {
            log::debug!("Inside session, using tmux dimensions.");
            self.cmd_runner.run(&cmd_basic!(
                "tmux",
                args = [
                    "display-message",
                    "-p",
                    "width: #{window_width}\nheight: #{window_height}"
                ]
            ))?
        } else {
            log::debug!("Outside session, using terminal dimensions.");
            let (width, height) = size().into_diagnostic()?;
            format!("width: {width}\nheight: {height}")
        };

        log::trace!("{}", &res);
        from_str(&res).into_diagnostic()
    }

    pub(crate) fn list_sessions(&self) -> Result<Vec<(String, bool)>> {
        self.cmd_runner
            .run(&cmd_basic!(
                "tmux",
                args = ["ls", "-F", "#{session_name}|#{session_attached}"]
            ))
            .map(|res: String| {
                res.lines()
                    .filter_map(|line| {
                        let parts: Vec<&str> = line.split('|').collect();
                        if parts.len() == 2 {
                            let name = parts[0].to_string();
                            let is_attached = parts[1] != "0";
                            Some((name, is_attached))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .or_else(|_| Ok(vec![]))
    }

    pub(crate) fn get_base_idx(&self) -> Result<usize> {
        let res: String = self.cmd_runner.run(&cmd_basic!(
            "tmux",
            args = ["show-options", "-g", "base-index"]
        ))?;
        res.split_whitespace()
            .last()
            .unwrap_or("0")
            .parse()
            .into_diagnostic()
    }

    pub(crate) fn set_pane_style(&self, target: &Target, style: &str) -> Result<()> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux",
            args = ["select-pane", "-t", target.to_string(), "-P", style]
        ))
    }

    pub(crate) fn bind_key(&self, key: &str, cmd: &str) -> Result<()> {
        let mut parts = key.split_whitespace();

        let (table, key) = match (parts.next(), parts.next(), parts.next()) {
            (Some(table), Some(key), None) => (table, key),
            (Some(key), None, None) => ("prefix", key),
            _ => {
                return Err(miette!(
                    "Invalid key format: expected 'table key' or just 'key'"
                ))
            }
        };

        self.cmd_runner.run(&cmd_basic!(
            "tmux",
            args = ["bind-key", "-T", table, key, cmd]
        ))
    }

    pub(crate) fn session_name(&self) -> Result<String> {
        self.cmd_runner
            .run(&cmd_basic!("tmux", args = ["display-message", "-p", "#S"]))
    }

    pub(crate) fn session_layout(&self) -> Result<String> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux",
            args = ["list-windows", "-F", "\"#{window_name} #{window_layout}\""]
        ))
    }

    pub(crate) fn rename_window(&self, target: &Target, name: &str) -> Result<()> {
        self.cmd_runner.run(&cmd_basic!(
            "tmux",
            args = ["rename-window", "-t", target.to_string(), name]
        ))
    }

    pub(crate) fn session_start_path(&self) -> Result<String> {
        let pane_map: HashMap<String, String> = self.pane_paths()?;
        let pane_paths: Vec<PathBuf> = pane_map.values().map(PathBuf::from).collect();

        if pane_paths.is_empty() {
            bail!("No pane paths found")
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
        let mut common_prefix = pane_paths
            .iter()
            .skip(1)
            .fold(pane_paths[0].clone(), |acc, path| {
                longest_common_prefix(&acc, path)
            });

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
            bail!("No valid session path found")
        }

        Ok(common_prefix.to_string_lossy().into_owned())
    }

    pub(crate) fn pane_paths(&self) -> Result<HashMap<String, String>> {
        let output: String = self.cmd_runner.run(&cmd_basic!(
            "tmux",
            args = ["list-panes", "-s", "-F", "#{pane_id} #{pane_current_path}"]
        ))?;

        let pane_map = output
            .lines()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                match (parts.next(), parts.next()) {
                    (Some(pane_id), Some(pane_path)) => {
                        trace!("pane-path: {pane_path}");
                        Some((pane_id.replace('%', ""), pane_path.to_string()))
                    }
                    _ => None,
                }
            })
            .collect();

        Ok(pane_map)
    }

    pub(crate) fn pane_command(&self) -> Result<HashMap<String, String>> {
        let current_pid = process::id().to_string();

        let output: String = self.cmd_runner.run(&cmd_basic!(
            "tmux",
            args = ["list-panes", "-s", "-F", "#{pane_id} #{pane_pid}"]
        ))?;

        let mut pane_map: HashMap<String, String> = HashMap::new();

        for line in output.lines() {
            let mut parts = line.split_whitespace();
            let (Some(pane_id), Some(pane_pid_str)) = (parts.next(), parts.next()) else {
                continue;
            };
            let pane_pid: i32 = pane_pid_str.parse().into_diagnostic()?;

            // Try sysinfo first, fall back to pgrep for testing/compatibility
            let child_pids = Self::get_child_processes(pane_pid);

            if child_pids.is_empty() {
                // Fallback to pgrep for testing - this maintains test compatibility
                let child_pids_output = self
                    .cmd_runner
                    .run(&cmd_basic!("pgrep", args = ["-P", pane_pid.to_string()]))
                    .unwrap_or_else(|_| String::new());

                for child_pid in child_pids_output
                    .lines()
                    .map(str::trim)
                    .filter(|&pid| pid != current_pid)
                {
                    let cmd_output: String = self
                        .cmd_runner
                        .run(&cmd_basic!("ps", args = ["-p", child_pid, "-o", "args="]))?;
                    let command = cmd_output.trim();

                    if !command.is_empty() && !command.starts_with('-') {
                        pane_map.insert(pane_id.replace('%', ""), command.to_string());
                        break;
                    }
                }
            } else {
                // Use sysinfo results
                for child_pid in child_pids
                    .into_iter()
                    .filter(|&pid| pid != current_pid.parse::<i32>().unwrap_or(0))
                {
                    if let Some(command) = Self::get_process_command(child_pid) {
                        if !command.is_empty() && !command.starts_with('-') {
                            pane_map.insert(pane_id.replace('%', ""), command);
                            break;
                        }
                    }
                }
            }
        }

        log::trace!("pane-pid-map: {pane_map:?}");

        Ok(pane_map)
    }

    pub(crate) fn set_pane_title(&self, target: &Target, title: &str) {
        self.register_command(
            target,
            vec![format!("tmux select-pane -t {target} -T {title} ")],
        );
    }

    /// Get child process PIDs using sysinfo instead of pgrep
    fn get_child_processes(parent_pid: i32) -> Vec<i32> {
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::All, true);

        let parent_pid = Pid::from_u32(parent_pid as u32);

        system
            .processes()
            .iter()
            .filter_map(|(pid, process)| {
                if process.parent() == Some(parent_pid) {
                    Some(pid.as_u32() as i32)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the command line of a process using sysinfo
    fn get_process_command(pid: i32) -> Option<String> {
        let mut system = System::new();
        let pid = Pid::from_u32(pid as u32);
        system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);

        system.process(pid).and_then(|process| {
            let cmd_args = process.cmd();
            if cmd_args.is_empty() {
                None
            } else {
                Some(
                    cmd_args
                        .iter()
                        .map(|arg| arg.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join(" "),
                )
            }
        })
    }
}
