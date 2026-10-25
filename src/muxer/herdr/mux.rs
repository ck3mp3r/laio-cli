use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use miette::{Context, Result, bail, miette};

use crate::app::manager::session::manager::{LAIO_CONFIG, LAIO_VARS, decode_variables};
use crate::common::cmd::{Runner, ShellRunner, Type};
use crate::common::config::Session;
use crate::common::muxer::{Client, Multiplexer};
use crate::common::path::{resolve_symlink, to_absolute_path};
use crate::common::session_info::SessionInfo;
use crate::{cmd_basic, cmd_forget};

use super::client::{
    HerdrApi, HerdrClient, SESSIONS_DIR_NAME, SOCKET_FILE_NAME, parse_session_list,
};
use super::model::{
    LayoutApplyParams, PaneInfo, PaneSendInputParams, PaneZoomMode, WorkspaceCreateParams,
    WorkspaceCreated, WorkspaceInfo, WorkspaceReportMetadataParams,
};
use super::{PaneOp, TabState, bind_pane_ops, build_plans, window_from_layout};

const MAX_SESSION_NAME_LEN: usize = 64;
pub(crate) const LAIO_SESSION_NAME: &str = "laio";

struct Sweep<'a> {
    skip_cmds: bool,
    stop_other: bool,
    host_workspace: Option<&'a str>,
    current_config: &'a Option<String>,
}

fn retrying_stop_chain(target: &str) -> String {
    format!(
        "nohup sh -c 'n=0; until herdr session stop {target} >/dev/null 2>&1; do n=$((n+1)); [ $n -ge 15 ] && exit 1; sleep 1; done' >/dev/null 2>&1 & disown"
    )
}

fn detached_close_chain(workspace_id: &str) -> String {
    format!(
        "nohup sh -c 'herdr workspace close {workspace_id} >/dev/null 2>&1' >/dev/null 2>&1 & disown"
    )
}

pub(crate) struct Herdr<R: Runner = ShellRunner, A: HerdrApi = HerdrClient<R>> {
    runner: R,
    api: A,
}

impl Herdr {
    pub(crate) fn new() -> Self {
        Self::new_with_runner(ShellRunner::new(), HerdrClient::new())
    }
}

impl<R: Runner, A: HerdrApi> Herdr<R, A> {
    pub(crate) fn new_with_runner(runner: R, api: A) -> Self {
        Self { runner, api }
    }

    fn attach(&self, name: &str) -> Result<()> {
        self.runner
            .run(&cmd_forget!("herdr", args = ["session", "attach", name]))
    }

    pub(crate) fn target_session(&self) -> String {
        env::var("HERDR_SOCKET_PATH")
            .ok()
            .and_then(|path| session_name_from_socket_path(&path))
            .unwrap_or_else(|| LAIO_SESSION_NAME.to_string())
    }

    pub(crate) fn current_workspace_id(&self) -> Option<String> {
        env::var("HERDR_WORKSPACE_ID")
            .ok()
            .filter(|value| !value.is_empty())
    }

    pub(crate) fn owned_workspaces(&self, session: &str) -> Result<Vec<WorkspaceInfo>> {
        Ok(self
            .api
            .workspace_list(session)?
            .into_iter()
            .filter(|workspace| workspace.tokens.contains_key(LAIO_CONFIG))
            .collect())
    }

    fn clean_stopped_session(&self) -> Result<()> {
        let output: String = self
            .runner
            .run(&cmd_basic!("herdr", args = ["session", "list", "--json"]))?;

        let stopped = parse_session_list(&output)?
            .into_iter()
            .any(|(name, running)| name == LAIO_SESSION_NAME && !running);

        if !stopped {
            return Ok(());
        }

        log::trace!("Deleting the stopped laio session.");

        let _: () = self.runner.run(&cmd_basic!(
            "herdr",
            args = ["session", "delete", LAIO_SESSION_NAME]
        ))?;

        Ok(())
    }

    fn owned_workspaces_across_sessions(&self) -> Result<Vec<(String, WorkspaceInfo)>> {
        let output: String = self
            .runner
            .run(&cmd_basic!("herdr", args = ["session", "list", "--json"]))?;

        let mut seen: HashSet<String> = HashSet::new();
        let mut found = Vec::new();

        for (session, running) in parse_session_list(&output)? {
            if !running {
                continue;
            }

            for workspace in self.owned_workspaces(&session)? {
                if !seen.insert(workspace.workspace_id.clone()) {
                    continue;
                }

                found.push((session.clone(), workspace));
            }
        }

        Ok(found)
    }

    pub(crate) fn find_owned_workspace(
        &self,
        session: &str,
        name: &str,
    ) -> Result<Option<WorkspaceInfo>> {
        Ok(self
            .owned_workspaces(session)?
            .into_iter()
            .find(|workspace| workspace.label == name))
    }

    fn find_owned_workspace_across_sessions(
        &self,
        name: &str,
    ) -> Result<Option<(String, WorkspaceInfo)>> {
        Ok(self
            .owned_workspaces_across_sessions()?
            .into_iter()
            .find(|(_, workspace)| workspace.label == name))
    }

    fn resolve_owned_workspace(&self, name: &str) -> Result<Option<(String, WorkspaceInfo)>> {
        let target = self.target_session();

        if self.api.session_exists(&target)
            && let Some(workspace) = self.find_owned_workspace(&target, name)?
        {
            return Ok(Some((target, workspace)));
        }

        self.find_owned_workspace_across_sessions(name)
    }

    fn has_workspace_label(&self, session: &str, name: &str) -> Result<bool> {
        Ok(self
            .api
            .workspace_list(session)?
            .iter()
            .any(|workspace| workspace.label == name))
    }

    fn run_startup_hooks(&self, session: &Session) -> Result<()> {
        let mut commands = session.startup.clone();
        if let Some(script) = &session.startup_script {
            commands.push(script.to_cmd()?);
        }

        self.run_commands(&commands, &session.path)
            .wrap_err_with(|| format!("Failed to run startup commands for '{}'", session.name))
    }

    fn create_workspace(&self, target: &str, session: &Session) -> Result<WorkspaceCreated> {
        self.api
            .workspace_create(
                target,
                &WorkspaceCreateParams {
                    cwd: Some(session.path.clone()),
                    focus: true,
                    label: Some(session.name.clone()),
                    env: session.env.clone(),
                },
            )
            .wrap_err_with(|| format!("Failed to create a workspace for '{}'", session.name))
    }

    fn apply_windows(
        &self,
        target: &str,
        session: &Session,
        created: &WorkspaceCreated,
    ) -> Result<()> {
        let plans = build_plans(session)?;
        let mut delayed = false;

        for (index, plan) in plans.iter().enumerate() {
            let params = if index == 0 {
                LayoutApplyParams {
                    workspace_id: None,
                    tab_id: Some(created.tab.tab_id.clone()),
                    tab_label: Some(plan.tab_label.clone()),
                    focus: plan.focus_tab,
                    root: plan.root.clone(),
                }
            } else {
                LayoutApplyParams {
                    workspace_id: Some(created.workspace.workspace_id.clone()),
                    tab_id: None,
                    tab_label: Some(plan.tab_label.clone()),
                    focus: plan.focus_tab,
                    root: plan.root.clone(),
                }
            };

            let layout = self.api.layout_apply(target, &params).wrap_err_with(|| {
                format!("Failed to apply the layout for window '{}'", plan.tab_label)
            })?;

            let ops = bind_pane_ops(plan, &layout)?;
            self.run_pane_ops(target, session, &ops, &mut delayed)?;
        }

        Ok(())
    }

    fn run_pane_ops(
        &self,
        target: &str,
        session: &Session,
        ops: &[PaneOp],
        delayed: &mut bool,
    ) -> Result<()> {
        for op in ops {
            if matches!(op, PaneOp::SendInput { .. }) && !*delayed {
                if let Some(delay_ms) = session.pane_cmd_delay {
                    thread::sleep(Duration::from_millis(delay_ms));
                }
                *delayed = true;
            }

            match op {
                PaneOp::Zoom(pane_id) => {
                    self.api.pane_zoom(target, pane_id, PaneZoomMode::On)?;
                }
                PaneOp::Focus(pane_id) => {
                    self.api.pane_focus(target, pane_id)?;
                }
                PaneOp::SendInput {
                    pane_id,
                    text,
                    keys,
                } => {
                    self.api.pane_send_input(
                        target,
                        &PaneSendInputParams {
                            pane_id: pane_id.clone(),
                            text: text.clone(),
                            keys: keys.clone(),
                        },
                    )?;
                }
            }
        }

        Ok(())
    }

    fn write_metadata(
        &self,
        target: &str,
        session: &Session,
        workspace_id: &str,
        env_vars: &[(&str, &str)],
    ) -> Result<()> {
        let tokens: HashMap<String, String> = env_vars
            .iter()
            .filter(|(key, _)| *key == LAIO_CONFIG || *key == LAIO_VARS)
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();

        self.api
            .workspace_report_metadata(
                target,
                &WorkspaceReportMetadataParams {
                    workspace_id: workspace_id.to_string(),
                    source: "laio".to_string(),
                    tokens,
                    seq: None,
                },
            )
            .wrap_err_with(|| format!("Failed to write laio metadata for '{}'", session.name))
    }

    fn attach_if_possible(&self, name: &str, skip_attach: bool) -> Result<()> {
        if skip_attach {
            return Ok(());
        }

        if env::var("HERDR_ENV").is_ok() {
            log::info!(
                "Skipping attach: nested herdr clients are blocked. Run `herdr session attach {name}` from outside herdr."
            );
            return Ok(());
        }

        self.attach(name)
    }

    fn is_inside_session(&self) -> bool {
        env::var("HERDR_ENV").is_ok()
    }

    fn run_shutdown_hooks(
        &self,
        target: &str,
        name: &str,
        session: &Option<Session>,
        workspace_id: &str,
    ) -> Result<()> {
        if let Some(session) = session {
            return self.run_shutdown_commands(session);
        }

        let tokens = self.workspace_tokens(target, workspace_id)?;
        self.run_workspace_shutdown_hooks(name, &tokens)
    }

    fn run_workspace_shutdown_hooks(
        &self,
        name: &str,
        tokens: &HashMap<String, String>,
    ) -> Result<()> {
        let Some(config) = tokens.get(LAIO_CONFIG) else {
            log::warn!("LAIO_CONFIG metadata not found for session '{name}'");
            return Ok(());
        };

        log::trace!("Config: {config:?}");

        let variables = decode_workspace_variables(name, tokens.get(LAIO_VARS));
        let session = Session::from_config(
            &resolve_symlink(&to_absolute_path(config)?)?,
            variables.as_deref(),
        )?;

        self.run_shutdown_commands(&session)
    }

    fn run_shutdown_commands(&self, session: &Session) -> Result<()> {
        let mut commands = session.shutdown.clone();
        if let Some(script) = &session.shutdown_script {
            commands.push(script.to_cmd()?);
        }

        self.run_commands(&commands, &session.path)
            .wrap_err_with(|| format!("Failed to run shutdown commands for '{}'", session.name))
    }

    fn workspace_tokens(
        &self,
        target: &str,
        workspace_id: &str,
    ) -> Result<HashMap<String, String>> {
        Ok(self.api.workspace_get(target, workspace_id)?.tokens)
    }

    fn current_config_path(&self) -> Result<Option<String>> {
        let Some(workspace_id) = self.current_workspace_id() else {
            return Ok(None);
        };

        let target = self.target_session();

        Ok(self
            .workspace_tokens(&target, &workspace_id)?
            .remove(LAIO_CONFIG))
    }

    fn resolve_stop_name(&self, name: &Option<String>) -> Result<Option<String>> {
        if let Some(name) = name {
            return Ok(Some(name.clone()));
        }

        let Some(workspace_id) = self.current_workspace_id() else {
            log::debug!("Not inside a laio workspace");
            return Ok(None);
        };

        let target = self.target_session();
        let workspace = self
            .owned_workspaces(&target)?
            .into_iter()
            .find(|workspace| workspace.workspace_id == workspace_id);

        match workspace {
            Some(workspace) => Ok(Some(workspace.label)),
            None => {
                log::debug!("Current workspace '{workspace_id}' is not a laio workspace");
                Ok(None)
            }
        }
    }

    fn stop_owned_workspaces(&self, skip_cmds: bool, stop_other: bool) -> Result<()> {
        let current_config = if stop_other {
            self.current_config_path()?
        } else {
            None
        };
        let host_workspace = self.current_workspace_id();
        let mut hook_error: Option<miette::Report> = None;

        let sweep = Sweep {
            skip_cmds,
            stop_other,
            host_workspace: host_workspace.as_deref(),
            current_config: &current_config,
        };

        let mut owned = self.owned_workspaces_across_sessions()?;

        let host = if !stop_other && self.inside_laio_session() {
            let index = owned.iter().position(|(_, workspace)| {
                host_workspace.as_deref() == Some(workspace.workspace_id.as_str())
            });

            index.map(|index| owned.remove(index))
        } else {
            None
        };

        for (session, workspace) in owned {
            self.close_owned_workspace(&session, &workspace, &sweep, &mut hook_error)?;
        }

        if host.is_some() {
            log::trace!("Spawning the stop chain before closing laio's host workspace.");
            self.spawn_detached_chain(LAIO_SESSION_NAME)?;
        }

        if let Some((session, workspace)) = host {
            self.close_owned_workspace(&session, &workspace, &sweep, &mut hook_error)?;
        }

        let teardown = if stop_other {
            self.ensure_session_stopped_if_empty(LAIO_SESSION_NAME)
        } else {
            self.ensure_session_stopped(LAIO_SESSION_NAME)
        };

        match hook_error {
            Some(error) => Err(error),
            None => teardown,
        }
    }

    fn close_owned_workspace(
        &self,
        session: &str,
        workspace: &WorkspaceInfo,
        sweep: &Sweep<'_>,
        hook_error: &mut Option<miette::Report>,
    ) -> Result<()> {
        if sweep.stop_other && sweep.host_workspace == Some(workspace.workspace_id.as_str()) {
            log::trace!("Keeping the current workspace: {}", workspace.label);
            return Ok(());
        }

        let tokens = if sweep.stop_other || !sweep.skip_cmds {
            self.workspace_tokens(session, &workspace.workspace_id)?
        } else {
            HashMap::new()
        };

        if sweep.stop_other && tokens.get(LAIO_CONFIG) == sweep.current_config.as_ref() {
            log::trace!("Keeping the current workspace: {}", workspace.label);
            return Ok(());
        }

        if !sweep.skip_cmds
            && let Err(error) = self.run_workspace_shutdown_hooks(&workspace.label, &tokens)
        {
            log::warn!("Shutdown hooks failed for '{}': {error:?}", workspace.label);

            if hook_error.is_none() {
                *hook_error = Some(error);
            }
        }

        log::trace!("Closing workspace: {}", workspace.label);
        self.api.workspace_close(session, &workspace.workspace_id)
    }

    fn inside_laio_session(&self) -> bool {
        self.is_inside_session() && self.target_session() == LAIO_SESSION_NAME
    }

    fn ensure_session_stopped(&self, target: &str) -> Result<()> {
        if target != LAIO_SESSION_NAME {
            return Ok(());
        }

        if !self.api.session_exists(target) {
            return Ok(());
        }

        self.stop_session(target)
    }

    fn ensure_session_stopped_if_empty(&self, target: &str) -> Result<()> {
        if target != LAIO_SESSION_NAME {
            return Ok(());
        }

        if !self.api.session_exists(target) {
            return Ok(());
        }

        if !self.owned_workspaces(target)?.is_empty() {
            return Ok(());
        }

        log::trace!("Stopping the laio session: no laio-owned workspace remains.");

        self.stop_session(target)
    }

    fn stop_session(&self, target: &str) -> Result<()> {
        log::trace!("Stopping the laio session.");

        if self.inside_laio_session() {
            return self.spawn_detached_chain(target);
        }

        let _: () = self
            .runner
            .run(&cmd_basic!("herdr", args = ["session", "stop", target]))?;

        Ok(())
    }

    fn spawn_detached_chain(&self, target: &str) -> Result<()> {
        self.spawn_detached(target, &retrying_stop_chain(target))
    }

    fn spawn_detached(&self, target: &str, script: &str) -> Result<()> {
        self.runner.run(&cmd_forget!(
            "sh",
            args = ["-c", script],
            env = ["HERDR_SESSION" => target]
        ))
    }
}

impl<R: Runner, A: HerdrApi> Client<R> for Herdr<R, A> {
    fn get_runner(&self) -> &R {
        &self.runner
    }
}

impl<R: Runner, A: HerdrApi> Multiplexer for Herdr<R, A> {
    fn start(
        &self,
        session: &Session,
        env_vars: &[(&str, &str)],
        skip_attach: bool,
        skip_cmds: bool,
    ) -> Result<()> {
        validate_session_name(&session.name)?;

        if let Err(error) = self.clean_stopped_session() {
            log::warn!("Failed to clean the stopped laio session: {error:?}");
        }

        let target = self.target_session();

        if self.switch(&session.name, skip_attach)? {
            return Ok(());
        }

        self.api
            .ensure_server(&target)
            .wrap_err_with(|| format!("Failed to start the herdr server for '{target}'"))?;

        if !skip_cmds {
            self.run_startup_hooks(session)?;
        }

        let created = self.create_workspace(&target, session)?;
        self.write_metadata(&target, session, &created.workspace.workspace_id, env_vars)?;
        self.apply_windows(&target, session, &created)?;
        self.attach_if_possible(&target, skip_attach)?;

        Ok(())
    }

    fn stop(
        &self,
        name: &Option<String>,
        session: &Option<Session>,
        skip_cmds: bool,
        stop_all: bool,
        stop_other: bool,
    ) -> Result<()> {
        let inside = self.is_inside_session();

        if !stop_all && !stop_other && name.is_none() && !inside {
            bail!("Specify laio session you want to stop.");
        }

        if (stop_all || stop_other) && name.is_some() {
            bail!("Stopping all/other and specifying a session name are mutually exclusive.");
        }

        if stop_other && !inside {
            bail!("Not inside a laio session.");
        }

        if stop_all || (stop_other && inside) {
            log::trace!("Closing all/other laio workspaces.");
            return self.stop_owned_workspaces(skip_cmds, stop_other);
        }

        let Some(name) = self.resolve_stop_name(name)? else {
            return Ok(());
        };

        let target = self.target_session();
        let target_exists = self.api.session_exists(&target);

        let fast = if target_exists {
            self.find_owned_workspace(&target, &name)?
                .map(|workspace| (target.clone(), workspace))
        } else {
            None
        };

        let (host, workspace) = match fast {
            Some(found) => found,
            None => {
                if target_exists && self.has_workspace_label(&target, &name)? {
                    log::debug!("Not a laio workspace: {name}");
                    return Ok(());
                }

                let Some((session, workspace)) =
                    self.find_owned_workspace_across_sessions(&name)?
                else {
                    bail!("Session {name} does not exist!");
                };

                (session, workspace)
            }
        };

        let inside_host = host == LAIO_SESSION_NAME
            && self.is_inside_session()
            && self.current_workspace_id().as_deref() == Some(workspace.workspace_id.as_str());

        let hooks = if !skip_cmds && !stop_other {
            self.run_shutdown_hooks(&host, &name, session, &workspace.workspace_id)
        } else {
            Ok(())
        };

        if inside_host {
            let others_remain = self
                .owned_workspaces(&host)?
                .iter()
                .any(|owned| owned.workspace_id != workspace.workspace_id);

            let spawn = if others_remain {
                self.spawn_detached(&host, &detached_close_chain(&workspace.workspace_id))
            } else {
                self.spawn_detached_chain(&host)
            };

            return hooks.and(spawn);
        }

        let close = self.api.workspace_close(&host, &workspace.workspace_id);
        let teardown = self.ensure_session_stopped_if_empty(&host);

        hooks.and(close).and(teardown)
    }

    fn get_session_config_path(&self, name: &str) -> Result<Option<String>> {
        let Some((host, workspace)) = self.resolve_owned_workspace(name)? else {
            return Ok(None);
        };

        Ok(self
            .workspace_tokens(&host, &workspace.workspace_id)?
            .remove(LAIO_CONFIG))
    }

    fn get_session_variables(&self, name: &str) -> Result<Option<Vec<String>>> {
        let Some((host, workspace)) = self.resolve_owned_workspace(name)? else {
            return Ok(None);
        };

        let tokens = self.workspace_tokens(&host, &workspace.workspace_id)?;

        Ok(decode_workspace_variables(name, tokens.get(LAIO_VARS)))
    }

    fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let attached = self.current_workspace_id();

        Ok(self
            .owned_workspaces_across_sessions()?
            .into_iter()
            .map(|(_, workspace)| {
                SessionInfo::active(
                    workspace.label,
                    attached.as_deref() == Some(workspace.workspace_id.as_str()),
                )
            })
            .collect())
    }

    fn switch(&self, name: &str, skip_attach: bool) -> Result<bool> {
        validate_session_name(name)?;

        let session = self.target_session();

        if self.api.session_exists(&session) {
            if let Some(workspace) = self.find_owned_workspace(&session, name)? {
                self.api
                    .workspace_focus(&session, &workspace.workspace_id)?;
                self.attach_if_possible(&session, skip_attach)?;
                return Ok(true);
            }

            if self.has_workspace_label(&session, name)? {
                bail!("Session '{name}' is not a laio session");
            }
        }

        let Some((host, workspace)) = self.find_owned_workspace_across_sessions(name)? else {
            return Ok(false);
        };

        self.api.workspace_focus(&host, &workspace.workspace_id)?;
        self.attach_if_possible(&host, skip_attach)?;

        Ok(true)
    }

    fn get_session(&self) -> Result<Session> {
        if env::var("HERDR_ENV").is_err() {
            bail!("You do not seem to be inside a herdr session.");
        }

        let Some(workspace_id) = self.current_workspace_id() else {
            bail!("HERDR_WORKSPACE_ID is not set");
        };

        let target = self.target_session();
        let snapshot = self.api.session_snapshot(&target)?;

        let workspace = snapshot
            .workspaces
            .iter()
            .find(|workspace| workspace.workspace_id == workspace_id)
            .ok_or_else(|| {
                miette!("Workspace '{workspace_id}' is not present in the herdr session")
            })?;

        let panes: Vec<&PaneInfo> = snapshot
            .panes
            .iter()
            .filter(|pane| pane.workspace_id == workspace_id)
            .collect();

        let session_path = common_pane_path(&panes)?;

        let mut windows = Vec::new();
        for tab in snapshot
            .tabs
            .iter()
            .filter(|tab| tab.workspace_id == workspace_id)
        {
            let layout = self.api.layout_export(&target, &tab.tab_id)?;
            let tab_layout = snapshot
                .layouts
                .iter()
                .find(|layout| layout.tab_id == tab.tab_id);

            let state = TabState {
                zoomed: tab_layout
                    .map(|layout| layout.zoomed)
                    .unwrap_or(layout.zoomed),
                focused_pane_id: tab_layout
                    .map(|layout| layout.focused_pane_id.clone())
                    .unwrap_or_else(|| layout.focused_pane_id.clone()),
                focused: snapshot.focused_tab_id.as_deref() == Some(tab.tab_id.as_str()),
            };

            windows.push(window_from_layout(
                &layout,
                &tab.label,
                &session_path,
                &state,
            ));
        }

        Ok(Session {
            name: workspace.label.clone(),
            path: session_path,
            startup: Vec::new(),
            startup_script: None,
            shutdown: Vec::new(),
            shutdown_script: None,
            env: HashMap::new(),
            shell: None,
            pane_cmd_delay: None,
            windows,
        })
    }
}

fn decode_workspace_variables(name: &str, encoded: Option<&String>) -> Option<Vec<String>> {
    let encoded = encoded?;

    if encoded.is_empty() {
        return Some(Vec::new());
    }

    match decode_variables(encoded) {
        Ok(variables) => Some(variables),
        Err(error) => {
            log::warn!("Failed to decode LAIO_VARS for session '{name}': {error:?}");
            None
        }
    }
}

fn validate_session_name(name: &str) -> Result<()> {
    let valid = !name.is_empty()
        && name.len() <= MAX_SESSION_NAME_LEN
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));

    if !valid {
        bail!("Invalid herdr session name '{name}'");
    }

    Ok(())
}

fn session_name_from_socket_path(path: &str) -> Option<String> {
    let components: Vec<&str> = Path::new(path)
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect();

    components
        .windows(3)
        .find(|window| window[0] == SESSIONS_DIR_NAME && window[2] == SOCKET_FILE_NAME)
        .map(|window| window[1].to_string())
}

fn common_pane_path(panes: &[&PaneInfo]) -> Result<String> {
    let mut paths = panes.iter().filter_map(|pane| pane.cwd.as_deref());
    let first = paths
        .next()
        .ok_or_else(|| miette!("Cannot determine the session path: no pane working directory"))?;

    let mut prefix = PathBuf::from(first);
    for path in paths {
        while !Path::new(path).starts_with(&prefix) {
            if !prefix.pop() {
                bail!("Cannot determine the session path from the pane working directories");
            }
        }
    }

    let prefix = prefix.to_string_lossy().to_string();
    if prefix.is_empty() || prefix == "/" {
        bail!("Cannot determine the session path from the pane working directories");
    }

    Ok(prefix)
}
