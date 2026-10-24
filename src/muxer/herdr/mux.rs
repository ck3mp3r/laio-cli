use std::collections::HashMap;
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
use crate::{cmd_basic, cmd_interactive};

use super::client::{
    DEFAULT_SESSION_NAME, HerdrApi, HerdrClient, SESSIONS_DIR_NAME, SOCKET_FILE_NAME,
    parse_session_list,
};
use super::model::{
    LayoutApplyParams, PaneInfo, PaneSendInputParams, PaneZoomMode, SessionSnapshot,
    WorkspaceCreateParams, WorkspaceReportMetadataParams,
};
use super::{PaneOp, TabState, bind_pane_ops, build_plans, window_from_layout};

const MAX_SESSION_NAME_LEN: usize = 64;

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

    pub(crate) fn is_laio_session(&self, name: &str) -> Result<bool> {
        validate_session_name(name)?;

        if !self.api.session_exists(name) {
            return Ok(false);
        }

        self.is_laio_owned(name)
    }

    fn is_laio_owned(&self, name: &str) -> Result<bool> {
        let workspaces = self.api.workspace_list(name)?;

        Ok(workspaces
            .first()
            .is_some_and(|workspace| workspace.tokens.contains_key(LAIO_CONFIG)))
    }

    fn attach(&self, name: &str) -> Result<()> {
        self.runner.run(&cmd_interactive!(
            "herdr",
            args = ["session", "attach", name]
        ))
    }

    fn run_startup_hooks(&self, session: &Session) -> Result<()> {
        let mut commands = session.startup.clone();
        if let Some(script) = &session.startup_script {
            commands.push(script.to_cmd()?);
        }

        self.run_commands(&commands, &session.path)
            .wrap_err_with(|| format!("Failed to run startup commands for '{}'", session.name))
    }

    fn create_workspace(&self, session: &Session) -> Result<String> {
        let created = self
            .api
            .workspace_create(
                &session.name,
                &WorkspaceCreateParams {
                    cwd: Some(session.path.clone()),
                    focus: true,
                    label: Some("laio".to_string()),
                    env: session.env.clone(),
                },
            )
            .wrap_err_with(|| format!("Failed to create a workspace for '{}'", session.name))?;

        Ok(created.workspace.workspace_id)
    }

    fn apply_windows(&self, session: &Session, workspace_id: &str) -> Result<()> {
        let plans = build_plans(session)?;
        let mut delayed = false;

        for plan in &plans {
            let layout = self
                .api
                .layout_apply(
                    &session.name,
                    &LayoutApplyParams {
                        workspace_id: Some(workspace_id.to_string()),
                        tab_label: Some(plan.tab_label.clone()),
                        focus: plan.focus_tab,
                        root: plan.root.clone(),
                    },
                )
                .wrap_err_with(|| {
                    format!("Failed to apply the layout for window '{}'", plan.tab_label)
                })?;

            let ops = bind_pane_ops(plan, &layout)?;
            self.run_pane_ops(session, &ops, &mut delayed)?;
        }

        Ok(())
    }

    fn run_pane_ops(&self, session: &Session, ops: &[PaneOp], delayed: &mut bool) -> Result<()> {
        for op in ops {
            if matches!(op, PaneOp::SendInput { .. }) && !*delayed {
                if let Some(delay_ms) = session.pane_cmd_delay {
                    thread::sleep(Duration::from_millis(delay_ms));
                }
                *delayed = true;
            }

            match op {
                PaneOp::Zoom(pane_id) => {
                    self.api
                        .pane_zoom(&session.name, pane_id, PaneZoomMode::On)?;
                }
                PaneOp::Focus(pane_id) => {
                    self.api.pane_focus(&session.name, pane_id)?;
                }
                PaneOp::SendInput {
                    pane_id,
                    text,
                    keys,
                } => {
                    self.api.pane_send_input(
                        &session.name,
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
                &session.name,
                &WorkspaceReportMetadataParams {
                    workspace_id: workspace_id.to_string(),
                    source: "laio".to_string(),
                    tokens,
                    seq: None,
                },
            )
            .wrap_err_with(|| format!("Failed to write laio metadata for '{}'", session.name))
    }

    fn attach_if_possible(&self, session: &Session, skip_attach: bool) -> Result<()> {
        if skip_attach {
            return Ok(());
        }

        if env::var("HERDR_ENV").is_ok() {
            log::info!(
                "Skipping attach: nested herdr clients are blocked. Run `herdr session attach {}` from outside herdr.",
                session.name
            );
            return Ok(());
        }

        self.attach(&session.name)
    }

    fn current_session_name(&self) -> String {
        env::var("HERDR_SOCKET_PATH")
            .ok()
            .and_then(|path| session_name_from_socket_path(&path))
            .unwrap_or_else(|| DEFAULT_SESSION_NAME.to_string())
    }

    fn is_inside_session(&self) -> bool {
        self.current_session_name() != DEFAULT_SESSION_NAME
    }

    fn stop_session(&self, name: &str) -> Result<()> {
        self.runner
            .run(&cmd_basic!("herdr", args = ["session", "stop", name]))
    }

    fn run_shutdown_hooks(
        &self,
        name: &str,
        session: &Option<Session>,
        skip_cmds: bool,
        stop_other: bool,
    ) -> Result<()> {
        if skip_cmds || stop_other {
            log::trace!("Skipping shutdown commands for session: {name:?}");
            return Ok(());
        }

        if let Some(session) = session {
            return self.run_shutdown_commands(session);
        }

        match self.get_session_config_path(name)? {
            Some(config) => {
                log::trace!("Config: {config:?}");

                let variables = self.get_session_variables(name)?;
                let session = Session::from_config(
                    &resolve_symlink(&to_absolute_path(&config)?)?,
                    variables.as_deref(),
                )?;

                self.run_shutdown_commands(&session)
            }
            None => {
                log::warn!("LAIO_CONFIG metadata not found for session '{name}'");
                Ok(())
            }
        }
    }

    fn run_shutdown_commands(&self, session: &Session) -> Result<()> {
        let mut commands = session.shutdown.clone();
        if let Some(script) = &session.shutdown_script {
            commands.push(script.to_cmd()?);
        }

        self.run_commands(&commands, &session.path)
            .wrap_err_with(|| format!("Failed to run shutdown commands for '{}'", session.name))
    }

    fn laio_tokens(&self, name: &str) -> Result<HashMap<String, String>> {
        let workspaces = self.api.workspace_list(name)?;
        let Some(workspace) = workspaces.first() else {
            return Ok(HashMap::new());
        };

        let info = self.api.workspace_get(name, &workspace.workspace_id)?;

        Ok(info.tokens)
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

        if self.switch(&session.name, skip_attach)? {
            return Ok(());
        }

        self.api
            .ensure_server(&session.name)
            .wrap_err_with(|| format!("Failed to start the herdr server for '{}'", session.name))?;

        if !skip_cmds {
            self.run_startup_hooks(session)?;
        }

        let workspace_id = self.create_workspace(session)?;
        self.apply_windows(session, &workspace_id)?;
        self.write_metadata(session, &workspace_id, env_vars)?;
        self.attach_if_possible(session, skip_attach)?;

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
        let current_session_name = self.current_session_name();
        log::trace!("Current session name: {current_session_name}");

        if !stop_all && !stop_other && name.is_none() && !self.is_inside_session() {
            bail!("Specify laio session you want to stop.");
        }

        if (stop_all || stop_other) && name.is_some() {
            bail!("Stopping all/other and specifying a session name are mutually exclusive.");
        }

        if stop_all || (stop_other && self.is_inside_session()) {
            log::trace!("Closing all/other laio sessions.");

            self.list_sessions()?
                .into_iter()
                .filter(|info| info.name != current_session_name)
                .try_for_each(|info| -> Result<()> {
                    if self.is_laio_session(&info.name)? {
                        log::trace!("Closing session: {:?}", info.name);
                        self.stop(&Some(info.name.to_string()), &None, skip_cmds, false, false)?;
                    }
                    Ok(())
                })?;

            if !self.is_inside_session() {
                log::debug!("Not inside a session");
                return Ok(());
            }
        }

        let name = name.clone().unwrap_or(current_session_name);
        validate_session_name(&name)?;

        if !self.api.session_exists(&name) {
            bail!("Session {name} does not exist!");
        }

        if !self.is_laio_owned(&name)? {
            log::debug!("Not a laio session: {name}");
            return Ok(());
        }

        let result = self.run_shutdown_hooks(&name, session, skip_cmds, stop_other);

        let stop_result = if stop_other {
            Ok(())
        } else {
            self.stop_session(&name)
        };

        result.and(stop_result)
    }

    fn get_session_config_path(&self, name: &str) -> Result<Option<String>> {
        validate_session_name(name)?;

        if !self.api.session_exists(name) {
            return Ok(None);
        }

        Ok(self.laio_tokens(name)?.remove(LAIO_CONFIG))
    }

    fn get_session_variables(&self, name: &str) -> Result<Option<Vec<String>>> {
        validate_session_name(name)?;

        if !self.api.session_exists(name) {
            return Ok(None);
        }

        let Some(encoded) = self.laio_tokens(name)?.remove(LAIO_VARS) else {
            return Ok(None);
        };

        if encoded.is_empty() {
            return Ok(Some(Vec::new()));
        }

        match decode_variables(&encoded) {
            Ok(variables) => Ok(Some(variables)),
            Err(error) => {
                log::warn!("Failed to decode LAIO_VARS for session '{name}': {error:?}");
                Ok(None)
            }
        }
    }

    fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let output: String = self
            .runner
            .run(&cmd_basic!("herdr", args = ["session", "list", "--json"]))?;

        Ok(parse_session_list(&output)?
            .into_iter()
            .filter(|(_, running)| *running)
            .map(|(name, _)| SessionInfo::active(name, false))
            .collect())
    }

    fn switch(&self, name: &str, skip_attach: bool) -> Result<bool> {
        validate_session_name(name)?;

        if !self.api.session_exists(name) {
            return Ok(false);
        }

        if !self.is_laio_owned(name)? {
            bail!("Session '{name}' is not a laio session");
        }

        if !skip_attach {
            self.attach(name)?;
        }

        Ok(true)
    }

    fn get_session(&self) -> Result<Session> {
        if env::var("HERDR_ENV").is_err() {
            bail!("You do not seem to be inside a herdr session.");
        }

        let name = self.current_session_name();
        let snapshot = self.api.session_snapshot(&name)?;
        let workspace_id = laio_workspace_id(&snapshot)?;
        let session_path = common_pane_path(&snapshot.panes)?;

        let mut windows = Vec::new();
        for tab in snapshot
            .tabs
            .iter()
            .filter(|tab| tab.workspace_id == workspace_id)
        {
            let layout = self.api.layout_export(&name, &tab.tab_id)?;
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
            name,
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

fn laio_workspace_id(snapshot: &SessionSnapshot) -> Result<String> {
    snapshot
        .focused_workspace_id
        .clone()
        .or_else(|| {
            snapshot
                .workspaces
                .first()
                .map(|workspace| workspace.workspace_id.clone())
        })
        .ok_or_else(|| miette!("herdr session has no workspace"))
}

fn common_pane_path(panes: &[PaneInfo]) -> Result<String> {
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
