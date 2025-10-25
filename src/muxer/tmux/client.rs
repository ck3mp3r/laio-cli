use crossterm::terminal::size;
use log::{debug, trace};
use miette::{bail, miette, IntoDiagnostic, Result};
use serde::Deserialize;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fmt::Debug,
    path::{Path, PathBuf},
    process,
    rc::Rc,
    str::{from_utf8, SplitWhitespace},
    thread,
    time::Duration,
};

use crate::{
    cmd_basic,
    common::{
        cmd::{Runner, Type},
        config::Command,
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
    pub cmd_runner: Rc<R>,
    pub cmds: RefCell<VecDeque<(Type, u64, String)>>, // (command, delay_ms, target)
}

impl<R: Runner> Client<R> for TmuxClient<R> {
    fn get_runner(&self) -> &R {
        &self.cmd_runner
    }
}

impl<R: Runner> TmuxClient<R> {
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
            let mut command = std::process::Command::new("tmux");
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
        self.cmds.borrow_mut().push_back((
            cmd_basic!(
                "tmux",
                args = ["set-environment", "-t", target.to_string(), name, value]
            ),
            0,
            target.to_string(),
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

    pub(crate) fn register_commands(&self, target: &Target, cmds: &Vec<Command>) {
        for cmd in cmds {
            self.register_command(target, &cmd.to_string(), None)
        }
    }

    pub(crate) fn register_command(&self, target: &Target, cmd: &String, delay_ms: Option<u64>) {
        let delay = delay_ms.unwrap_or(10); // Minimal delay for tmux processing
        self.cmds.borrow_mut().push_back((
            cmd_basic!(
                "tmux",
                args = ["send-keys", "-t", target.to_string(), cmd, "C-m"]
            ),
            delay,
            target.to_string(),
        ))
    }



    pub(crate) fn focus_pane(&self, target: &Target) {
        self.cmds.borrow_mut().push_back((
            cmd_basic!(
                "tmux",
                args = ["select-pane", "-Z", "-t", target.to_string()]
            ),
            0,
            target.to_string(),
        ))
    }

    pub(crate) fn flush_commands(&self) -> Result<()> {
        use std::collections::HashMap;
        use std::thread;

        // Group commands by target pane
        let mut pane_commands: HashMap<String, Vec<Type>> = HashMap::new();

        while let Some((cmd, _, target)) = self.cmds.borrow_mut().pop_front() {
            pane_commands.entry(target).or_default().push(cmd);
        }

        let mut handles = vec![];

        // Start event-driven execution for each pane
        for (target, commands) in pane_commands {
            if commands.len() > 1 {
                // Multi-command pane: spawn background thread for sequential execution
                let runner = (*self.cmd_runner).clone();
                let handle = thread::spawn(move || -> Result<()> {
                    execute_pane_commands_async(runner, target, commands)
                });
                handles.push(handle);
            } else if let Some(cmd) = commands.into_iter().next() {
                // Single command: execute immediately (no async needed)
                let _: () = self.cmd_runner.run(&cmd)?;
            }
        }

        // Don't wait for threads - let them run in background
        // This allows laio to attach to session immediately
        std::mem::drop(handles);

        Ok(())
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
        let mut csum: u16 = 0;
        for &c in layout.as_bytes() {
            csum = (csum >> 1) | ((csum & 1) << 15);
            csum = csum.wrapping_add(c as u16);
        }
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
        serde_yaml::from_str(&res).into_diagnostic()
    }

    pub(crate) fn list_sessions(&self) -> Result<Vec<String>> {
        self.cmd_runner
            .run(&cmd_basic!("tmux", args = ["ls", "-F", "#{session_name}"]))
            .map(|res: String| res.lines().map(String::from).collect())
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
        let key_parts: Vec<&str> = key.split_whitespace().collect();

        let (table, key) = match key_parts.as_slice() {
            [table, key] => (*table, *key),
            [key] => ("prefix", *key),
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
            bail!("No valid session path found")
        }

        Ok(common_prefix.to_string_lossy().into_owned())
    }

    pub(crate) fn pane_paths(&self) -> Result<HashMap<String, String>> {
        let output: String = self.cmd_runner.run(&cmd_basic!(
            "tmux",
            args = ["list-panes", "-s", "-F", "#{pane_id} #{pane_current_path}"]
        ))?;

        let mut pane_map: HashMap<String, String> = HashMap::new();

        for line in output.lines() {
            let mut parts = line.split_whitespace();
            if let (Some(pane_id), Some(pane_path)) = (parts.next(), parts.next()) {
                trace!("pane-path: {pane_path}");
                pane_map.insert(pane_id.to_string().replace('%', ""), pane_path.to_string());
            }
        }

        Ok(pane_map)
    }

    pub(crate) fn pane_command(&self) -> Result<HashMap<String, String>> {
        let current_pid: String = process::id().to_string();

        let output: String = self.cmd_runner.run(&cmd_basic!(
            "tmux",
            args = ["list-panes", "-s", "-F", "#{pane_id} #{pane_pid}"]
        ))?;

        let mut pane_map: HashMap<String, String> = HashMap::new();

        for line in output.lines() {
            let mut parts: SplitWhitespace = line.split_whitespace();
            let (Some(pane_id), Some(pane_pid_str)) = (parts.next(), parts.next()) else {
                continue;
            };
            let pane_pid: i32 = pane_pid_str.parse().into_diagnostic()?;

            let child_pids_output = match self
                .cmd_runner
                .run(&cmd_basic!("pgrep", args = ["-P", pane_pid.to_string()]))
            {
                Ok(output) => output,
                Err(e) => {
                    debug!("Error running command: {e}");
                    String::new()
                }
            };

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
                    .run(&cmd_basic!("ps", args = ["-p", child_pid, "-o", "args="]))?;
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

        log::trace!("pane-pid-map: {pane_map:?}");

        Ok(pane_map)
    }

    pub(crate) fn set_pane_title(&self, target: &Target, title: &str) {
        self.register_command(
            target,
            &format!("tmux select-pane -t {target} -T {title} "),
            None,
        );
    }

    pub(crate) fn wait_for_shell_ready(&self, target: &Target) -> Result<()> {
        // Wait for shell to be ready by checking if pane is responsive without sending keys
        let max_attempts = 20; // 2 seconds max wait

        for attempt in 1..=max_attempts {
            thread::sleep(Duration::from_millis(100));

            // Check if pane is responsive by getting its info (no visual impact)
            let pane_info: Result<String, _> = self.cmd_runner.run(&cmd_basic!(
                "tmux",
                args = [
                    "display-message",
                    "-t",
                    target.to_string(),
                    "-p",
                    "#{pane_active}:#{pane_id}"
                ]
            ));

            if pane_info.is_ok() {
                log::debug!("Shell ready for target: {} (attempt {})", target, attempt);
                return Ok(());
            }
        }

        log::warn!("Shell readiness timeout for target: {}", target);
        Ok(())
    }
}

// Standalone functions for pane idle detection (following SOLID - Single Responsibility)

pub(crate) fn get_pane_baseline_pid<R: Runner>(runner: &R, target: &str) -> Result<u32> {
    // Get the main process PID for the pane (baseline when ready/idle)
    let pid_str: String = runner.run(&cmd_basic!(
        "tmux",
        args = ["display-message", "-t", target, "-p", "#{pane_pid}"]
    ))?;

    let pid = pid_str
        .trim()
        .parse::<u32>()
        .map_err(|e| miette!("Failed to parse PID '{}': {}", pid_str.trim(), e))?;

    Ok(pid)
}

pub(crate) fn get_process_tree<R: Runner>(runner: &R, parent_pid: u32) -> Result<Vec<u32>> {
    // Get child process PIDs using command runner directly (no tmux pollution)
    let output: String = runner
        .run(&cmd_basic!("pgrep", args = ["-P", &parent_pid.to_string()]))
        .unwrap_or_else(|_| String::new()); // pgrep returns non-zero when no matches

    let child_pids: Vec<u32> = output
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .collect();

    Ok(child_pids)
}

pub(crate) fn wait_for_process_tree_empty<R: Runner>(
    runner: &R,
    target: &str,
    baseline_pid: u32,
) -> Result<()> {
    loop {
        // Get the full process tree under the baseline PID
        let child_pids = get_process_tree(runner, baseline_pid)?;

        // EVENT DETECTED: Process tree is empty
        // Command completed when only the baseline PID exists (no children)
        if child_pids.is_empty() {
            log::debug!(
                "EVENT: Pane {} process tree empty - command completed",
                target
            );

            return Ok(());
        }

        // Small polling interval for responsive event detection
        thread::sleep(Duration::from_millis(100));
    }
}

pub(crate) fn execute_pane_commands_event_driven<R: Runner>(
    runner: R,
    target: String,
    commands: Vec<Type>,
    _shell: String, // Not used anymore
) -> Result<()> {
    use std::collections::VecDeque;

    if commands.is_empty() {
        return Ok(());
    }

    let mut command_queue: VecDeque<Type> = commands.into();

    log::debug!(
        "Starting process tree event-driven execution for pane {} with {} commands",
        target,
        command_queue.len()
    );

    // DETECT BASELINE PID: Get the root process PID when pane is ready
    log::debug!("DETECT: Getting baseline PID for pane {}", target);
    let baseline_pid = get_pane_baseline_pid(&runner, &target)?;
    log::debug!("DETECTED: Pane {} baseline PID {}", target, baseline_pid);

    // Event loop: process one command at a time
    while let Some(cmd) = command_queue.front() {
        log::debug!("EVENT: Sending command to pane {}", target);

        // SEND-KEY EVENT: Send current command
        let _: () = runner.run(cmd)?;

        // POP: Remove sent command from queue
        command_queue.pop_front();
        log::debug!(
            "POP: Command sent, {} remaining in queue",
            command_queue.len()
        );

        // COMPLETION EVENT: Wait for command to complete (if more commands remain)
        if !command_queue.is_empty() {
            log::debug!(
                "POLL: Waiting for process tree to empty on pane {} (PID {})",
                target,
                baseline_pid
            );

            // Give the command a moment to start before checking completion
            thread::sleep(Duration::from_millis(200));

            wait_for_process_tree_empty(&runner, &target, baseline_pid)?;
            log::debug!(
                "COMPLETE: Command completed on pane {}, ready for next",
                target
            );
        }
    }

    log::debug!("Process tree event chain completed for pane {}", target);

    Ok(())
}

fn execute_pane_commands_async<R: Runner>(
    runner: R,
    target: String,
    commands: Vec<Type>,
) -> Result<()> {
    // Delegate to event-driven executor (shell detection happens inside now)
    execute_pane_commands_event_driven(runner, target, commands, String::new())
}
