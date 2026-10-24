use std::{
    collections::HashMap,
    env,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

use miette::{IntoDiagnostic, Result, bail, miette};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::cmd_forget;
use crate::common::cmd::{Runner, ShellRunner, Type};
use crate::common::muxer::Client;

use super::model::{
    HerdrPong, HerdrRequest, HerdrResponse, LayoutApplyParams, LayoutDescription, LayoutResponse,
    PaneInfo, PaneResponse, PaneSendInputParams, PaneZoomMode, PaneZoomResponse, PaneZoomResult,
    SessionSnapshot, SessionSnapshotResponse, WorkspaceCreateParams, WorkspaceCreated,
    WorkspaceInfo, WorkspaceInfoResponse, WorkspaceListResponse, WorkspaceReportMetadataParams,
};

const APP_DIR_NAME: &str = "herdr";
pub(crate) const SESSIONS_DIR_NAME: &str = "sessions";
pub(crate) const SOCKET_FILE_NAME: &str = "herdr.sock";
pub(crate) const DEFAULT_SESSION_NAME: &str = "default";
const MAX_TOKEN_VALUE_CHARS: usize = 80;
const MAX_TOKEN_KEYS_PER_REQUEST: usize = 16;
const MAX_TOKEN_KEYS_PER_RESOURCE: usize = 32;
const SERVER_START_TIMEOUT: Duration = Duration::from_secs(10);
const SERVER_POLL_INTERVAL: Duration = Duration::from_millis(50);

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub(crate) struct HerdrConnection {
    socket_path: PathBuf,
    stream: BufReader<UnixStream>,
}

impl HerdrConnection {
    pub(crate) fn connect(name: &str) -> Result<Self> {
        Self::connect_to(&resolve_socket_path(name)?)
    }

    pub(crate) fn connect_to(socket_path: &Path) -> Result<Self> {
        let stream = UnixStream::connect(socket_path).map_err(|error| {
            miette!(
                "Failed to connect to herdr socket {}: {error}",
                socket_path.display()
            )
        })?;

        Ok(Self {
            socket_path: socket_path.to_path_buf(),
            stream: BufReader::new(stream),
        })
    }

    pub(crate) fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let request = HerdrRequest::new(next_request_id(), method, params);
        self.send_request(&request)?;
        into_result(self.read_response()?)
    }

    fn send_request(&mut self, request: &HerdrRequest) -> Result<()> {
        let mut line = serde_json::to_string(request).into_diagnostic()?;
        line.push('\n');

        let stream = self.stream.get_mut();
        stream.write_all(line.as_bytes()).into_diagnostic()?;
        stream.flush().into_diagnostic()
    }

    fn read_response(&mut self) -> Result<HerdrResponse> {
        let mut line = String::new();
        let read = self.stream.read_line(&mut line).into_diagnostic()?;
        if read == 0 {
            bail!(
                "herdr closed the connection at {} without a response",
                self.socket_path.display()
            );
        }

        serde_json::from_str(&line).into_diagnostic()
    }
}

pub(crate) trait HerdrApi {
    fn ping(&self, session: &str) -> Result<(String, u32)>;
    fn workspace_create(
        &self,
        session: &str,
        params: &WorkspaceCreateParams,
    ) -> Result<WorkspaceCreated>;
    fn workspace_get(&self, session: &str, workspace_id: &str) -> Result<WorkspaceInfo>;
    fn workspace_list(&self, session: &str) -> Result<Vec<WorkspaceInfo>>;
    fn session_snapshot(&self, session: &str) -> Result<SessionSnapshot>;
    fn workspace_report_metadata(
        &self,
        session: &str,
        params: &WorkspaceReportMetadataParams,
    ) -> Result<()>;
    fn layout_apply(&self, session: &str, params: &LayoutApplyParams) -> Result<LayoutDescription>;
    fn layout_export(&self, session: &str, tab_id: &str) -> Result<LayoutDescription>;
    fn pane_focus(&self, session: &str, pane_id: &str) -> Result<PaneInfo>;
    fn pane_zoom(&self, session: &str, pane_id: &str, mode: PaneZoomMode)
    -> Result<PaneZoomResult>;
    fn pane_send_input(&self, session: &str, params: &PaneSendInputParams) -> Result<()>;
    fn session_exists(&self, session: &str) -> bool;
    fn ensure_server(&self, session: &str) -> Result<()>;
}

pub(crate) struct HerdrClient<R: Runner = ShellRunner> {
    cmd_runner: Rc<R>,
}

impl HerdrClient {
    pub(crate) fn new() -> Self {
        Self::new_with_runner(ShellRunner::new())
    }
}

impl<R: Runner> HerdrClient<R> {
    pub(crate) fn new_with_runner(runner: R) -> Self {
        Self {
            cmd_runner: Rc::new(runner),
        }
    }

    pub(crate) fn spawn_server(&self, session: &str) -> Result<()> {
        self.cmd_runner.run(&cmd_forget!(
            "herdr",
            args = ["server"],
            env = ["HERDR_SESSION" => session]
        ))
    }

    pub(crate) fn ensure_server_with_timeout(
        &self,
        session: &str,
        timeout: Duration,
    ) -> Result<()> {
        if self.ping(session).is_ok() {
            return Ok(());
        }

        self.spawn_server(session)?;

        let socket_path = resolve_socket_path(session)?;
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.ping(session).is_ok() {
                return Ok(());
            }
            thread::sleep(SERVER_POLL_INTERVAL);
        }

        Err(miette!(
            "herdr server for session '{session}' did not start within {} ms (socket: {})",
            timeout.as_millis(),
            socket_path.display()
        ))
    }

    fn connection(&self, session: &str) -> Result<HerdrConnection> {
        HerdrConnection::connect(session)
    }

    fn request(&self, session: &str, method: &str, params: Value) -> Result<Value> {
        self.connection(session)?.request(method, params)
    }

    fn typed_result(
        &self,
        session: &str,
        method: &str,
        params: Value,
        expected_type: &str,
    ) -> Result<Value> {
        let result = self.request(session, method, params)?;
        let kind = result
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| miette!("herdr {method} result is missing the type field"))?;

        if kind != expected_type {
            bail!("herdr {method} returned result type '{kind}', expected '{expected_type}'");
        }

        Ok(result)
    }
}

impl<R: Runner> Client<R> for HerdrClient<R> {
    fn get_runner(&self) -> &R {
        &self.cmd_runner
    }
}

impl<R: Runner> HerdrApi for HerdrClient<R> {
    fn ping(&self, session: &str) -> Result<(String, u32)> {
        let result = self.typed_result(session, "ping", json!({}), "pong")?;
        let pong: HerdrPong = serde_json::from_value(result).into_diagnostic()?;
        Ok((pong.version, pong.protocol))
    }

    fn workspace_create(
        &self,
        session: &str,
        params: &WorkspaceCreateParams,
    ) -> Result<WorkspaceCreated> {
        let result = self.typed_result(
            session,
            "workspace.create",
            to_params(params)?,
            "workspace_created",
        )?;
        serde_json::from_value(result).into_diagnostic()
    }

    fn workspace_get(&self, session: &str, workspace_id: &str) -> Result<WorkspaceInfo> {
        let result = self.typed_result(
            session,
            "workspace.get",
            json!({"workspace_id": workspace_id}),
            "workspace_info",
        )?;
        let response: WorkspaceInfoResponse = serde_json::from_value(result).into_diagnostic()?;
        let mut workspace = response.workspace;
        workspace.tokens = join_tokens(workspace.tokens);
        Ok(workspace)
    }

    fn workspace_list(&self, session: &str) -> Result<Vec<WorkspaceInfo>> {
        let result = self.typed_result(session, "workspace.list", json!({}), "workspace_list")?;
        let response: WorkspaceListResponse = serde_json::from_value(result).into_diagnostic()?;
        Ok(response.workspaces)
    }

    fn session_snapshot(&self, session: &str) -> Result<SessionSnapshot> {
        let result =
            self.typed_result(session, "session.snapshot", json!({}), "session_snapshot")?;
        let response: SessionSnapshotResponse = serde_json::from_value(result).into_diagnostic()?;
        Ok(response.snapshot)
    }

    fn workspace_report_metadata(
        &self,
        session: &str,
        params: &WorkspaceReportMetadataParams,
    ) -> Result<()> {
        let pairs = chunk_tokens(&params.tokens);
        if pairs.len() > MAX_TOKEN_KEYS_PER_RESOURCE {
            bail!(
                "metadata for workspace '{}' needs {} token keys, exceeding the {MAX_TOKEN_KEYS_PER_RESOURCE}-key limit",
                params.workspace_id,
                pairs.len()
            );
        }

        for chunk in pairs.chunks(MAX_TOKEN_KEYS_PER_REQUEST) {
            let request = WorkspaceReportMetadataParams {
                workspace_id: params.workspace_id.clone(),
                source: params.source.clone(),
                tokens: chunk.iter().cloned().collect(),
                seq: params.seq,
            };
            self.typed_result(
                session,
                "workspace.report_metadata",
                to_params(&request)?,
                "ok",
            )?;
        }

        Ok(())
    }

    fn layout_apply(&self, session: &str, params: &LayoutApplyParams) -> Result<LayoutDescription> {
        let result =
            self.typed_result(session, "layout.apply", to_params(params)?, "layout_apply")?;
        let response: LayoutResponse = serde_json::from_value(result).into_diagnostic()?;
        Ok(response.layout)
    }

    fn layout_export(&self, session: &str, tab_id: &str) -> Result<LayoutDescription> {
        let result = self.typed_result(
            session,
            "layout.export",
            json!({"tab_id": tab_id}),
            "layout_export",
        )?;
        let response: LayoutResponse = serde_json::from_value(result).into_diagnostic()?;
        Ok(response.layout)
    }

    fn pane_focus(&self, session: &str, pane_id: &str) -> Result<PaneInfo> {
        let result = self.typed_result(
            session,
            "pane.focus",
            json!({"pane_id": pane_id}),
            "pane_info",
        )?;
        let response: PaneResponse = serde_json::from_value(result).into_diagnostic()?;
        Ok(response.pane)
    }

    fn pane_zoom(
        &self,
        session: &str,
        pane_id: &str,
        mode: PaneZoomMode,
    ) -> Result<PaneZoomResult> {
        let result = self.typed_result(
            session,
            "pane.zoom",
            json!({"pane_id": pane_id, "mode": mode}),
            "pane_zoom",
        )?;
        let response: PaneZoomResponse = serde_json::from_value(result).into_diagnostic()?;
        Ok(response.zoom)
    }

    fn pane_send_input(&self, session: &str, params: &PaneSendInputParams) -> Result<()> {
        self.typed_result(session, "pane.send_input", to_params(params)?, "ok")?;
        Ok(())
    }

    fn session_exists(&self, session: &str) -> bool {
        HerdrConnection::connect(session).is_ok()
    }

    fn ensure_server(&self, session: &str) -> Result<()> {
        self.ensure_server_with_timeout(session, SERVER_START_TIMEOUT)
    }
}

pub(crate) fn resolve_socket_path(name: &str) -> Result<PathBuf> {
    Ok(config_dir()?
        .join(SESSIONS_DIR_NAME)
        .join(name)
        .join(SOCKET_FILE_NAME))
}

fn config_dir() -> Result<PathBuf> {
    config_dir_from(env::var("XDG_CONFIG_HOME").ok(), env::var("HOME").ok())
}

pub(crate) fn config_dir_from(
    xdg_config_home: Option<String>,
    home: Option<String>,
) -> Result<PathBuf> {
    if let Some(dir) = xdg_config_home {
        return Ok(PathBuf::from(dir).join(APP_DIR_NAME));
    }

    let home = home.ok_or_else(|| miette!("Failed to get home directory"))?;
    Ok(PathBuf::from(home).join(".config").join(APP_DIR_NAME))
}

fn into_result(response: HerdrResponse) -> Result<Value> {
    if let Some(error) = response.error {
        bail!("herdr error {}: {}", error.code, error.message);
    }

    response
        .result
        .ok_or_else(|| miette!("herdr response carried neither result nor error"))
}

fn next_request_id() -> String {
    let id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("laio-{id}")
}

fn to_params<T: Serialize>(params: &T) -> Result<Value> {
    serde_json::to_value(params).into_diagnostic()
}

#[derive(Debug, Deserialize)]
struct SessionListOutput {
    #[serde(default)]
    sessions: Vec<SessionListEntry>,
}

#[derive(Debug, Deserialize)]
struct SessionListEntry {
    name: String,
    #[serde(default)]
    running: bool,
}

pub(crate) fn parse_session_list(output: &str) -> Result<Vec<(String, bool)>> {
    let parsed: SessionListOutput = serde_json::from_str(output).into_diagnostic()?;

    Ok(parsed
        .sessions
        .into_iter()
        .filter(|entry| entry.name != DEFAULT_SESSION_NAME)
        .map(|entry| (entry.name, entry.running))
        .collect())
}

pub(crate) fn chunk_tokens(tokens: &HashMap<String, String>) -> Vec<(String, String)> {
    let mut keys: Vec<&String> = tokens.keys().collect();
    keys.sort();

    keys.into_iter()
        .flat_map(|key| chunk_token(key, &tokens[key]))
        .collect()
}

pub(crate) fn chunk_token(key: &str, value: &str) -> Vec<(String, String)> {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= MAX_TOKEN_VALUE_CHARS {
        return vec![(key.to_string(), value.to_string())];
    }

    chars
        .chunks(MAX_TOKEN_VALUE_CHARS)
        .enumerate()
        .map(|(index, chunk)| (format!("{key}_{}", index + 1), chunk.iter().collect()))
        .collect()
}

pub(crate) fn join_tokens(tokens: HashMap<String, String>) -> HashMap<String, String> {
    let mut joined: HashMap<String, String> = HashMap::new();
    let mut chunks: HashMap<String, Vec<(usize, String)>> = HashMap::new();

    for (key, value) in tokens {
        match split_chunk_key(&key) {
            Some((base, index)) => chunks
                .entry(base.to_string())
                .or_default()
                .push((index, value)),
            None => {
                joined.insert(key, value);
            }
        }
    }

    for (base, mut parts) in chunks {
        parts.sort_by_key(|(index, _)| *index);
        joined.insert(base, parts.into_iter().map(|(_, value)| value).collect());
    }

    joined
}

pub(crate) fn split_chunk_key(key: &str) -> Option<(&str, usize)> {
    let (base, suffix) = key.rsplit_once('_')?;
    if base.is_empty() || suffix.is_empty() || !suffix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let index: usize = suffix.parse().ok()?;
    (index > 0).then_some((base, index))
}
