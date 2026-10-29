use miette::{IntoDiagnostic, Result, miette};
use mockall::mock;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    fs,
    io::{BufRead, BufReader, ErrorKind, Write},
    os::unix::net::UnixListener,
    path::{Path, PathBuf},
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::config::{FlexDirection, Pane, Session, Window};

use super::super::client::{HerdrApi, HerdrClient, resolve_socket_path};
use super::super::model::{
    LayoutApplyParams, LayoutDescription, LayoutNode, LayoutPane, PaneInfo, PaneSendInputParams,
    PaneZoomMode, PaneZoomResult, SessionSnapshot, TabInfo, WorkspaceCreateParams,
    WorkspaceCreated, WorkspaceInfo, WorkspaceReportMetadataParams,
};

use super::session_test::session_snapshot;

mock! {
    pub HerdrApiMock {}

    impl HerdrApi for HerdrApiMock {
        fn ping<'a>(&self, session: &'a str) -> Result<(String, u32)>;
        fn workspace_create<'a, 'b>(
            &self,
            session: &'a str,
            params: &'b WorkspaceCreateParams,
        ) -> Result<WorkspaceCreated>;
        fn workspace_get<'a, 'b>(
            &self,
            session: &'a str,
            workspace_id: &'b str,
        ) -> Result<WorkspaceInfo>;
        fn workspace_focus<'a, 'b>(
            &self,
            session: &'a str,
            workspace_id: &'b str,
        ) -> Result<WorkspaceInfo>;
        fn workspace_close<'a, 'b>(
            &self,
            session: &'a str,
            workspace_id: &'b str,
        ) -> Result<()>;
        fn workspace_list<'a>(&self, session: &'a str) -> Result<Vec<WorkspaceInfo>>;
        fn session_snapshot<'a>(&self, session: &'a str) -> Result<SessionSnapshot>;
        fn workspace_report_metadata<'a, 'b>(
            &self,
            session: &'a str,
            params: &'b WorkspaceReportMetadataParams,
        ) -> Result<()>;
        fn layout_apply<'a, 'b>(
            &self,
            session: &'a str,
            params: &'b LayoutApplyParams,
        ) -> Result<LayoutDescription>;
        fn layout_export<'a, 'b>(
            &self,
            session: &'a str,
            tab_id: &'b str,
        ) -> Result<LayoutDescription>;
        fn pane_focus<'a, 'b>(&self, session: &'a str, pane_id: &'b str) -> Result<PaneInfo>;
        fn pane_zoom<'a, 'b>(
            &self,
            session: &'a str,
            pane_id: &'b str,
            mode: PaneZoomMode,
        ) -> Result<PaneZoomResult>;
        fn pane_send_input<'a, 'b>(
            &self,
            session: &'a str,
            params: &'b PaneSendInputParams,
        ) -> Result<()>;
        fn session_exists<'a>(&self, session: &'a str) -> bool;
        fn ensure_server<'a>(&self, session: &'a str) -> Result<()>;
    }
}

pub(super) static ENV_LOCK: Mutex<()> = Mutex::new(());

pub(super) const GUARDED_ENV: [&str; 3] = ["HERDR_ENV", "HERDR_SOCKET_PATH", "HERDR_WORKSPACE_ID"];

pub(super) struct EnvGuard {
    _guard: MutexGuard<'static, ()>,
    saved: [(&'static str, Option<OsString>); 3],
}

impl EnvGuard {
    pub(super) fn new() -> Self {
        let guard = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let saved = GUARDED_ENV.map(|key| {
            let previous = env::var_os(key);

            unsafe {
                env::remove_var(key);
            }

            (key, previous)
        });

        Self {
            _guard: guard,
            saved,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, previous) in self.saved.iter_mut() {
            restore_env(key, previous.take());
        }
    }
}

pub(super) fn env_lock() -> Result<EnvGuard> {
    Ok(EnvGuard::new())
}

pub(super) fn temp_socket_path(label: &str) -> PathBuf {
    env::temp_dir().join(format!("laio-herdr-{label}-{}.sock", std::process::id()))
}

pub(super) fn bind_listener(path: &Path) -> Result<UnixListener> {
    let _ = fs::remove_file(path);
    UnixListener::bind(path).into_diagnostic()
}

pub(super) fn spawn_responder(
    listener: UnixListener,
    response: String,
) -> JoinHandle<Result<String>> {
    thread::spawn(move || -> Result<String> {
        let (mut stream, _) = listener.accept().into_diagnostic()?;
        let mut reader = BufReader::new(stream.try_clone().into_diagnostic()?);
        let mut request = String::new();
        reader.read_line(&mut request).into_diagnostic()?;
        stream.write_all(response.as_bytes()).into_diagnostic()?;
        stream.flush().into_diagnostic()?;
        Ok(request)
    })
}

pub(super) fn spawn_canned_server(
    listener: UnixListener,
    responses: Vec<String>,
) -> JoinHandle<Result<Vec<String>>> {
    thread::spawn(move || -> Result<Vec<String>> {
        listener.set_nonblocking(true).into_diagnostic()?;
        let mut requests = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(2);

        while requests.len() < responses.len() && Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false).into_diagnostic()?;
                    let mut reader = BufReader::new(stream.try_clone().into_diagnostic()?);
                    let mut request = String::new();
                    let read = reader.read_line(&mut request).into_diagnostic()?;
                    if read == 0 {
                        continue;
                    }
                    stream
                        .write_all(responses[requests.len()].as_bytes())
                        .into_diagnostic()?;
                    stream.flush().into_diagnostic()?;
                    requests.push(request);
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2));
                }
                Err(error) => return Err(error).into_diagnostic(),
            }
        }

        Ok(requests)
    })
}

pub(super) fn join_thread<T>(handle: JoinHandle<Result<T>>) -> Result<T> {
    handle
        .join()
        .map_err(|_| miette!("responder thread panicked"))?
}

pub(super) fn parse_request(line: &str) -> Result<Value> {
    serde_json::from_str(line.trim_end()).into_diagnostic()
}

pub(super) fn empty_runner() -> RunnerMock {
    RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    }
}

pub(super) fn pong_response() -> String {
    json!({
        "id": "1",
        "result": {"type": "pong", "version": "0.9.5", "protocol": 22, "capabilities": {}}
    })
    .to_string()
        + "\n"
}

pub(super) fn ok_response() -> String {
    json!({"id": "1", "result": {"type": "ok"}}).to_string() + "\n"
}

pub(super) fn workspace_created_response() -> String {
    json!({
        "id": "1",
        "result": {
            "type": "workspace_created",
            "workspace": {"workspace_id": "w1", "label": "main", "focused": true},
            "tab": {"tab_id": "t1", "workspace_id": "w1", "label": "main", "focused": true},
            "root_pane": {"pane_id": "p1", "workspace_id": "w1", "tab_id": "t1", "focused": true}
        }
    })
    .to_string()
        + "\n"
}

pub(super) fn workspace_info_response(tokens: Value) -> String {
    json!({
        "id": "1",
        "result": {
            "type": "workspace_info",
            "workspace": {
                "workspace_id": "w1",
                "label": "main",
                "focused": true,
                "tokens": tokens
            }
        }
    })
    .to_string()
        + "\n"
}

pub(super) fn layout_response(kind: &str) -> String {
    json!({
        "id": "1",
        "result": {
            "type": kind,
            "layout": {
                "workspace_id": "w1",
                "tab_id": "t1",
                "zoomed": false,
                "focused_pane_id": "p1",
                "root": {
                    "type": "split",
                    "direction": "right",
                    "ratio": 0.5,
                    "first": {"type": "pane", "pane_id": "p1", "cwd": "/tmp/a"},
                    "second": {"type": "pane", "pane_id": "p2", "label": "logs"}
                }
            }
        }
    })
    .to_string()
        + "\n"
}

pub(super) fn pane_info_response() -> String {
    json!({
        "id": "1",
        "result": {
            "type": "pane_info",
            "pane": {
                "pane_id": "p1",
                "workspace_id": "w1",
                "tab_id": "t1",
                "focused": true,
                "cwd": "/tmp/a",
                "label": "editor"
            }
        }
    })
    .to_string()
        + "\n"
}

pub(super) fn pane_zoom_response() -> String {
    json!({
        "id": "1",
        "result": {
            "type": "pane_zoom",
            "zoom": {"pane_id": "p1", "focused_pane_id": "p1", "zoomed": true}
        }
    })
    .to_string()
        + "\n"
}

pub(super) fn restore_env(key: &str, previous: Option<OsString>) {
    unsafe {
        match previous {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }
}

pub(super) static SESSION_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub(super) struct TestSession {
    _guard: EnvGuard,
    xdg_config_home: PathBuf,
    previous_xdg: Option<OsString>,
    name: String,
}

impl TestSession {
    pub(super) fn new() -> Result<Self> {
        let guard = env_lock()?;
        let id = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
        let xdg_config_home = env::temp_dir().join(format!("lh{}-{id}", std::process::id()));
        let previous_xdg = env::var_os("XDG_CONFIG_HOME");

        unsafe {
            env::set_var("XDG_CONFIG_HOME", &xdg_config_home);
        }

        Ok(Self {
            _guard: guard,
            xdg_config_home,
            previous_xdg,
            name: "w".to_string(),
        })
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn socket_path(&self) -> Result<PathBuf> {
        resolve_socket_path(&self.name)
    }

    pub(super) fn bind(&self) -> Result<UnixListener> {
        let path = self.socket_path()?;
        let parent = path
            .parent()
            .ok_or_else(|| miette!("socket path should have a parent directory"))?;
        fs::create_dir_all(parent).into_diagnostic()?;
        bind_listener(&path)
    }

    pub(super) fn client(&self) -> HerdrClient<RunnerMock> {
        HerdrClient::new_with_runner(empty_runner())
    }
}

impl Drop for TestSession {
    fn drop(&mut self) {
        restore_env("XDG_CONFIG_HOME", self.previous_xdg.take());
        let _ = fs::remove_dir_all(&self.xdg_config_home);
    }
}

#[test]
fn herdr_api_mock_covers_every_operation() -> Result<()> {
    let mut api = MockHerdrApiMock::new();

    api.expect_ping()
        .times(1)
        .returning(|_| Ok(("0.9.5".to_string(), 22)));
    api.expect_workspace_create()
        .times(1)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_get()
        .times(1)
        .returning(|_, _| Ok(workspace_info()));
    api.expect_workspace_focus()
        .times(1)
        .returning(|_, _| Ok(workspace_info()));
    api.expect_workspace_close()
        .times(1)
        .returning(|_, _| Ok(()));
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info()]));
    api.expect_session_snapshot()
        .times(1)
        .returning(|_| Ok(session_snapshot()));
    api.expect_workspace_report_metadata()
        .times(1)
        .returning(|_, _| Ok(()));
    api.expect_layout_apply()
        .times(1)
        .returning(|_, _| Ok(layout_description()));
    api.expect_layout_export()
        .times(1)
        .returning(|_, _| Ok(layout_description()));
    api.expect_pane_focus()
        .times(1)
        .returning(|_, _| Ok(pane_info()));
    api.expect_pane_zoom()
        .times(1)
        .returning(|_, _, _| Ok(pane_zoom_result()));
    api.expect_pane_send_input()
        .times(1)
        .returning(|_, _| Ok(()));
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_ensure_server().times(1).returning(|_| Ok(()));

    let api: Box<dyn HerdrApi> = Box::new(api);

    assert_eq!(api.ping("w")?.1, 22);
    api.workspace_create(
        "w",
        &WorkspaceCreateParams {
            cwd: None,
            focus: false,
            label: None,
            env: HashMap::new(),
        },
    )?;
    api.workspace_get("w", "w1")?;
    api.workspace_focus("w", "w1")?;
    api.workspace_close("w", "w1")?;
    api.workspace_list("w")?;
    api.session_snapshot("w")?;
    api.workspace_report_metadata(
        "w",
        &WorkspaceReportMetadataParams {
            workspace_id: "w1".to_string(),
            source: "laio".to_string(),
            tokens: HashMap::new(),
            seq: None,
        },
    )?;
    api.layout_apply(
        "w",
        &LayoutApplyParams {
            workspace_id: None,
            tab_id: None,
            tab_label: None,
            focus: false,
            root: LayoutNode::Pane {
                pane: LayoutPane::default(),
            },
        },
    )?;
    api.layout_export("w", "t1")?;
    api.pane_focus("w", "p1")?;
    api.pane_zoom("w", "p1", PaneZoomMode::On)?;
    api.pane_send_input(
        "w",
        &PaneSendInputParams {
            pane_id: "p1".to_string(),
            text: "ls".to_string(),
            keys: Vec::new(),
        },
    )?;
    assert!(api.session_exists("w"));
    api.ensure_server("w")?;
    Ok(())
}

pub(super) fn workspace_created() -> WorkspaceCreated {
    WorkspaceCreated {
        workspace: workspace_info(),
        tab: TabInfo {
            tab_id: "t1".to_string(),
            workspace_id: "w1".to_string(),
            label: "main".to_string(),
            focused: true,
        },
        root_pane: pane_info(),
    }
}

pub(super) fn workspace_info() -> WorkspaceInfo {
    WorkspaceInfo {
        workspace_id: "w1".to_string(),
        label: "main".to_string(),
        focused: true,
        tokens: HashMap::new(),
    }
}

pub(super) fn layout_description() -> LayoutDescription {
    LayoutDescription {
        workspace_id: "w1".to_string(),
        tab_id: "t1".to_string(),
        zoomed: false,
        focused_pane_id: "p1".to_string(),
        root: LayoutNode::Pane {
            pane: LayoutPane::default(),
        },
    }
}

pub(super) fn pane_info() -> PaneInfo {
    PaneInfo {
        pane_id: "p1".to_string(),
        workspace_id: "w1".to_string(),
        tab_id: "t1".to_string(),
        focused: true,
        cwd: None,
        label: None,
    }
}

pub(super) fn pane_zoom_result() -> PaneZoomResult {
    PaneZoomResult {
        pane_id: "p1".to_string(),
        focused_pane_id: "p1".to_string(),
        zoomed: true,
    }
}

pub(super) fn test_session(windows: Vec<Window>) -> Session {
    Session {
        name: "test".to_string(),
        path: "/tmp/project".to_string(),
        startup: Vec::new(),
        startup_script: None,
        shutdown: Vec::new(),
        shutdown_script: None,
        env: HashMap::new(),
        shell: None,
        pane_cmd_delay: None,
        windows,
    }
}

pub(super) fn test_window(name: &str, flex_direction: FlexDirection, panes: Vec<Pane>) -> Window {
    Window {
        name: name.to_string(),
        path: None,
        flex_direction,
        panes,
        focus: false,
    }
}

pub(super) fn test_pane(flex: usize, name: Option<&str>, path: &str) -> Pane {
    Pane {
        flex_direction: FlexDirection::Row,
        flex,
        name: name.map(str::to_string),
        path: path.to_string(),
        style: None,
        commands: Vec::new(),
        script: None,
        panes: Vec::new(),
        zoom: false,
        focus: false,
    }
}

pub(super) fn pane_node(pane_id: &str) -> LayoutNode {
    LayoutNode::Pane {
        pane: LayoutPane {
            pane_id: Some(pane_id.to_string()),
            ..Default::default()
        },
    }
}

pub(super) fn contains_key(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(map) => {
            map.contains_key(key) || map.values().any(|value| contains_key(value, key))
        }
        Value::Array(items) => items.iter().any(|value| contains_key(value, key)),
        _ => false,
    }
}

pub(super) fn workspace_list_response(workspaces: Value) -> String {
    json!({
        "id": "1",
        "result": {"type": "workspace_list", "workspaces": workspaces}
    })
    .to_string()
        + "\n"
}

pub(super) fn workspace_entry(label: &str, tokens: Value) -> Value {
    json!({
        "workspace_id": "w1",
        "label": label,
        "focused": true,
        "tokens": tokens
    })
}

pub(super) fn laio_tokens() -> HashMap<String, String> {
    HashMap::from([("LAIO_CONFIG".to_string(), "/tmp/laio.yaml".to_string())])
}

pub(super) fn config_tokens(path: &str) -> HashMap<String, String> {
    HashMap::from([("LAIO_CONFIG".to_string(), path.to_string())])
}

pub(super) fn workspace_info_with(
    workspace_id: &str,
    label: &str,
    tokens: HashMap<String, String>,
) -> WorkspaceInfo {
    WorkspaceInfo {
        workspace_id: workspace_id.to_string(),
        label: label.to_string(),
        focused: false,
        tokens,
    }
}

pub(super) fn clear_herdr_socket_path() -> Option<OsString> {
    let previous = env::var_os("HERDR_SOCKET_PATH");

    unsafe {
        env::remove_var("HERDR_SOCKET_PATH");
    }

    previous
}

pub(super) fn set_herdr_socket_path(value: &str) -> Option<OsString> {
    let previous = env::var_os("HERDR_SOCKET_PATH");

    unsafe {
        env::set_var("HERDR_SOCKET_PATH", value);
    }

    previous
}

pub(super) fn clear_herdr_workspace_id() -> Option<OsString> {
    let previous = env::var_os("HERDR_WORKSPACE_ID");

    unsafe {
        env::remove_var("HERDR_WORKSPACE_ID");
    }

    previous
}

pub(super) fn set_herdr_workspace_id(value: &str) -> Option<OsString> {
    let previous = env::var_os("HERDR_WORKSPACE_ID");

    unsafe {
        env::set_var("HERDR_WORKSPACE_ID", value);
    }

    previous
}

pub(super) fn clear_herdr_env() -> Option<OsString> {
    let previous = env::var_os("HERDR_ENV");

    unsafe {
        env::remove_var("HERDR_ENV");
    }

    previous
}

pub(super) fn set_herdr_env(value: &str) -> Option<OsString> {
    let previous = env::var_os("HERDR_ENV");

    unsafe {
        env::set_var("HERDR_ENV", value);
    }

    previous
}

pub(super) fn two_session_list() -> String {
    json!({
        "sessions": [
            {"name": "laio", "default": false, "running": true},
            {"name": "work", "default": false, "running": true}
        ]
    })
    .to_string()
}

pub(super) fn empty_session_list() -> String {
    json!({"sessions": []}).to_string()
}

pub(super) fn session_list_cmd_string(sessions: String) -> MockCmdStringMock {
    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1..)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));

    cmd_string
}

pub(super) fn runner_with_session_list(sessions: String) -> RunnerMock {
    RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string: session_list_cmd_string(sessions),
        cmd_bool: MockCmdBoolMock::new(),
    }
}
