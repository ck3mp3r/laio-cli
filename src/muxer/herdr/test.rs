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

use miette::{IntoDiagnostic, Result, miette};
use mockall::{Sequence, mock};
use serde_json::{Value, json};

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::config::{Command, FlexDirection, Pane, Script, Session, Window};
use crate::common::muxer::Multiplexer;
use crate::common::session_info::{SessionInfo, SessionStatus};

use super::client::{
    HerdrApi, HerdrClient, HerdrConnection, chunk_token, chunk_tokens, config_dir_from,
    join_tokens, resolve_socket_path, split_chunk_key,
};
use super::convert::{PaneOp, TabState, bind_pane_ops, build_plans, window_from_layout};
use super::model::{
    LayoutApplyParams, LayoutDescription, LayoutNode, LayoutPane, PaneInfo, PaneLayoutSnapshot,
    PaneSendInputParams, PaneZoomMode, PaneZoomResult, SessionSnapshot, SplitDirection, TabInfo,
    WorkspaceCreateParams, WorkspaceCreated, WorkspaceInfo, WorkspaceReportMetadataParams,
};
use super::mux::{Herdr, LAIO_SESSION_NAME};

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

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_lock() -> Result<MutexGuard<'static, ()>> {
    ENV_LOCK.lock().map_err(|_| miette!("env lock poisoned"))
}

fn temp_socket_path(label: &str) -> PathBuf {
    env::temp_dir().join(format!("laio-herdr-{label}-{}.sock", std::process::id()))
}

fn bind_listener(path: &Path) -> Result<UnixListener> {
    let _ = fs::remove_file(path);
    UnixListener::bind(path).into_diagnostic()
}

fn spawn_responder(listener: UnixListener, response: String) -> JoinHandle<Result<String>> {
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

fn spawn_canned_server(
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

fn join_thread<T>(handle: JoinHandle<Result<T>>) -> Result<T> {
    handle
        .join()
        .map_err(|_| miette!("responder thread panicked"))?
}

fn parse_request(line: &str) -> Result<Value> {
    serde_json::from_str(line.trim_end()).into_diagnostic()
}

fn empty_runner() -> RunnerMock {
    RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    }
}

fn pong_response() -> String {
    json!({
        "id": "1",
        "result": {"type": "pong", "version": "0.9.5", "protocol": 22, "capabilities": {}}
    })
    .to_string()
        + "\n"
}

fn ok_response() -> String {
    json!({"id": "1", "result": {"type": "ok"}}).to_string() + "\n"
}

fn workspace_created_response() -> String {
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

fn workspace_info_response(tokens: Value) -> String {
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

fn layout_response(kind: &str) -> String {
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

fn pane_info_response() -> String {
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

fn pane_zoom_response() -> String {
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

fn restore_env(key: &str, previous: Option<OsString>) {
    unsafe {
        match previous {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }
}

static SESSION_COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TestSession {
    _guard: MutexGuard<'static, ()>,
    xdg_config_home: PathBuf,
    previous_xdg: Option<OsString>,
    name: String,
}

impl TestSession {
    fn new() -> Result<Self> {
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

    fn name(&self) -> &str {
        &self.name
    }

    fn socket_path(&self) -> Result<PathBuf> {
        resolve_socket_path(&self.name)
    }

    fn bind(&self) -> Result<UnixListener> {
        let path = self.socket_path()?;
        let parent = path
            .parent()
            .ok_or_else(|| miette!("socket path should have a parent directory"))?;
        fs::create_dir_all(parent).into_diagnostic()?;
        bind_listener(&path)
    }

    fn client(&self) -> HerdrClient<RunnerMock> {
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
fn request_is_one_json_line_with_id_method_and_params() -> Result<()> {
    let path = temp_socket_path("request");
    let listener = bind_listener(&path)?;
    let handle = spawn_responder(listener, "{\"result\":{\"type\":\"ok\"}}\n".to_string());

    let mut connection = HerdrConnection::connect_to(&path)?;
    connection.request("workspace.list", json!({"focus": false}))?;

    let request = join_thread(handle)?;
    let _ = fs::remove_file(&path);

    assert!(
        request.ends_with('\n'),
        "request should end with a newline: {request:?}"
    );
    assert_eq!(
        request.matches('\n').count(),
        1,
        "request should be a single line: {request:?}"
    );

    let parsed: Value = serde_json::from_str(request.trim_end()).into_diagnostic()?;
    let object = parsed
        .as_object()
        .ok_or_else(|| miette!("request should be a JSON object"))?;
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();

    assert_eq!(keys, vec!["id", "method", "params"]);
    assert!(object["id"].is_string(), "id should be a string");
    assert_eq!(object["method"], json!("workspace.list"));
    assert_eq!(object["params"], json!({"focus": false}));
    Ok(())
}

#[test]
fn result_response_returns_ok_value() -> Result<()> {
    let path = temp_socket_path("result");
    let listener = bind_listener(&path)?;
    let handle = spawn_responder(
        listener,
        "{\"id\":\"1\",\"result\":{\"type\":\"ok\",\"value\":42}}\n".to_string(),
    );

    let mut connection = HerdrConnection::connect_to(&path)?;
    let result = connection.request("pane.send_input", json!({"text": "ls"}))?;

    let _ = join_thread(handle)?;
    let _ = fs::remove_file(&path);

    assert_eq!(result, json!({"type": "ok", "value": 42}));
    Ok(())
}

#[test]
fn error_response_returns_err_with_code_and_message() -> Result<()> {
    let path = temp_socket_path("error");
    let listener = bind_listener(&path)?;
    let handle = spawn_responder(
        listener,
        "{\"id\":\"1\",\"error\":{\"code\":\"not_found\",\"message\":\"no such workspace\"}}\n"
            .to_string(),
    );

    let mut connection = HerdrConnection::connect_to(&path)?;
    let error = connection
        .request("workspace.get", json!({"workspace_id": "missing"}))
        .err()
        .ok_or_else(|| miette!("error response should return Err"))?;

    let _ = join_thread(handle)?;
    let _ = fs::remove_file(&path);

    let report = error.to_string();
    assert!(
        report.contains("not_found"),
        "report should contain the error code: {report}"
    );
    assert!(
        report.contains("no such workspace"),
        "report should contain the error message: {report}"
    );
    Ok(())
}

#[test]
fn connect_without_listener_returns_err_with_socket_path() -> Result<()> {
    let path = temp_socket_path("missing");
    let _ = fs::remove_file(&path);

    let error = HerdrConnection::connect_to(&path)
        .err()
        .ok_or_else(|| miette!("missing socket should return Err"))?;

    let report = error.to_string();
    assert!(
        report.contains(path.to_string_lossy().as_ref()),
        "report should contain the socket path: {report}"
    );
    Ok(())
}

#[test]
fn socket_path_uses_xdg_config_home_and_ignores_herdr_socket_path() -> Result<()> {
    let _guard = env_lock()?;
    let xdg_config_home = env::temp_dir().join("laio-herdr-xdg-config");
    let decoy = env::temp_dir().join("laio-herdr-decoy.sock");
    let previous_xdg = env::var_os("XDG_CONFIG_HOME");
    let previous_socket = env::var_os("HERDR_SOCKET_PATH");

    unsafe {
        env::set_var("XDG_CONFIG_HOME", &xdg_config_home);
        env::set_var("HERDR_SOCKET_PATH", &decoy);
    }
    let resolved = resolve_socket_path("work");
    restore_env("XDG_CONFIG_HOME", previous_xdg);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    let resolved = resolved?;
    let expected = xdg_config_home
        .join("herdr")
        .join("sessions")
        .join("work")
        .join("herdr.sock");

    assert_eq!(resolved, expected);
    assert_ne!(resolved, decoy);
    Ok(())
}

#[test]
fn config_dir_falls_back_to_home_config() -> Result<()> {
    let home = env::temp_dir().join("laio-herdr-home");
    let config_dir = config_dir_from(None, Some(home.to_string_lossy().to_string()))?;

    assert_eq!(config_dir, home.join(".config").join("herdr"));
    Ok(())
}

#[test]
fn config_dir_requires_home_when_xdg_config_home_is_unset() -> Result<()> {
    let error = config_dir_from(None, None)
        .err()
        .ok_or_else(|| miette!("missing HOME should return Err"))?;

    assert!(error.to_string().contains("home directory"));
    Ok(())
}

#[test]
fn connect_resolves_named_session_socket_and_pings() -> Result<()> {
    let _guard = env_lock()?;
    let xdg_config_home = env::temp_dir().join(format!("lh{}", std::process::id()));
    let previous_xdg = env::var_os("XDG_CONFIG_HOME");

    unsafe {
        env::set_var("XDG_CONFIG_HOME", &xdg_config_home);
    }
    let pong = (|| -> Result<Value> {
        let socket_path = resolve_socket_path("w")?;
        let parent = socket_path
            .parent()
            .ok_or_else(|| miette!("socket path should have a parent directory"))?;
        fs::create_dir_all(parent).into_diagnostic()?;

        let listener = bind_listener(&socket_path)?;
        let handle = spawn_responder(listener, pong_response());
        let mut connection = HerdrConnection::connect("w")?;
        let pong = connection.request("ping", json!({}))?;
        let _ = join_thread(handle)?;
        Ok(pong)
    })();
    restore_env("XDG_CONFIG_HOME", previous_xdg);

    let pong = pong?;
    let _ = fs::remove_dir_all(&xdg_config_home);

    assert_eq!(pong["version"], json!("0.9.5"));
    assert_eq!(pong["protocol"], json!(22));
    Ok(())
}

#[test]
fn request_returns_the_ping_result() -> Result<()> {
    let path = temp_socket_path("ping");
    let listener = bind_listener(&path)?;
    let handle = spawn_responder(listener, pong_response());

    let mut connection = HerdrConnection::connect_to(&path)?;
    let result = connection.request("ping", json!({}))?;

    let request = join_thread(handle)?;
    let _ = fs::remove_file(&path);

    assert_eq!(result["version"], json!("0.9.5"));
    assert_eq!(result["protocol"], json!(22));
    assert!(
        request.contains("\"method\":\"ping\""),
        "request should call ping: {request:?}"
    );
    Ok(())
}

#[test]
fn ping_rejects_unexpected_result_type() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![ok_response()]);

    let error = session
        .client()
        .ping(session.name())
        .err()
        .ok_or_else(|| miette!("unexpected result type should return Err"))?;

    let _ = join_thread(handle)?;

    assert!(
        error.to_string().contains("pong"),
        "report should name the expected result type: {error}"
    );
    Ok(())
}

#[test]
fn workspace_create_sends_typed_params_and_parses_result() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![workspace_created_response()]);

    let created = session.client().workspace_create(
        session.name(),
        &WorkspaceCreateParams {
            cwd: Some("/tmp/project".to_string()),
            focus: true,
            label: Some("main".to_string()),
            env: HashMap::from([("LAIO_CONFIG".to_string(), "/tmp/laio.yaml".to_string())]),
        },
    )?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(created.workspace.workspace_id, "w1");
    assert_eq!(created.tab.tab_id, "t1");
    assert_eq!(created.root_pane.pane_id, "p1");
    assert_eq!(request["method"], json!("workspace.create"));
    assert_eq!(request["params"]["cwd"], json!("/tmp/project"));
    assert_eq!(request["params"]["focus"], json!(true));
    assert_eq!(request["params"]["label"], json!("main"));
    assert_eq!(
        request["params"]["env"]["LAIO_CONFIG"],
        json!("/tmp/laio.yaml")
    );
    Ok(())
}

#[test]
fn workspace_get_joins_chunked_tokens_in_ascending_order() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(
        listener,
        vec![workspace_info_response(json!({
            "LAIO_CONFIG_2": "b",
            "LAIO_CONFIG_1": "a",
            "LAIO_CONFIG_10": "c",
            "LAIO_VARS": "plain"
        }))],
    );

    let workspace = session.client().workspace_get(session.name(), "w1")?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(
        workspace.tokens.get("LAIO_CONFIG"),
        Some(&"abc".to_string())
    );
    assert_eq!(
        workspace.tokens.get("LAIO_VARS"),
        Some(&"plain".to_string())
    );
    assert_eq!(request["method"], json!("workspace.get"));
    assert_eq!(request["params"]["workspace_id"], json!("w1"));
    Ok(())
}

#[test]
fn workspace_focus_sends_workspace_id_and_parses_info() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(
        listener,
        vec![workspace_info_response(
            json!({"LAIO_CONFIG": "/tmp/laio.yaml"}),
        )],
    );

    let workspace = session.client().workspace_focus(session.name(), "w1")?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(workspace.workspace_id, "w1");
    assert_eq!(
        workspace.tokens.get("LAIO_CONFIG"),
        Some(&"/tmp/laio.yaml".to_string())
    );
    assert_eq!(request["method"], json!("workspace.focus"));
    assert_eq!(request["params"]["workspace_id"], json!("w1"));
    Ok(())
}

#[test]
fn workspace_close_sends_workspace_id_and_returns_ok() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![ok_response()]);

    session.client().workspace_close(session.name(), "w1")?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(request["method"], json!("workspace.close"));
    assert_eq!(request["params"]["workspace_id"], json!("w1"));
    assert!(request["params"].get("close_group").is_none());
    Ok(())
}

#[test]
fn workspace_focus_returns_err_with_code_and_message() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(
        listener,
        vec![
            "{\"id\":\"1\",\"error\":{\"code\":\"not_found\",\"message\":\"no such workspace\"}}\n"
                .to_string(),
        ],
    );

    let error = session
        .client()
        .workspace_focus(session.name(), "missing")
        .err()
        .ok_or_else(|| miette!("error response should return Err"))?;

    let _ = join_thread(handle)?;

    let report = error.to_string();
    assert!(
        report.contains("not_found"),
        "report should contain the error code: {report}"
    );
    assert!(
        report.contains("no such workspace"),
        "report should contain the error message: {report}"
    );
    Ok(())
}

#[test]
fn workspace_close_returns_err_with_code_and_message() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(
        listener,
        vec![
            "{\"id\":\"1\",\"error\":{\"code\":\"not_found\",\"message\":\"no such workspace\"}}\n"
                .to_string(),
        ],
    );

    let error = session
        .client()
        .workspace_close(session.name(), "missing")
        .err()
        .ok_or_else(|| miette!("error response should return Err"))?;

    let _ = join_thread(handle)?;

    let report = error.to_string();
    assert!(
        report.contains("not_found"),
        "report should contain the error code: {report}"
    );
    assert!(
        report.contains("no such workspace"),
        "report should contain the error message: {report}"
    );
    Ok(())
}

#[test]
fn workspace_report_metadata_writes_short_value_under_plain_key() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![ok_response()]);

    session.client().workspace_report_metadata(
        session.name(),
        &WorkspaceReportMetadataParams {
            workspace_id: "w1".to_string(),
            source: "laio".to_string(),
            tokens: HashMap::from([("LAIO_CONFIG".to_string(), "/tmp/laio.yaml".to_string())]),
            seq: None,
        },
    )?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(request["method"], json!("workspace.report_metadata"));
    assert_eq!(request["params"]["workspace_id"], json!("w1"));
    assert_eq!(request["params"]["source"], json!("laio"));
    assert_eq!(
        request["params"]["tokens"],
        json!({"LAIO_CONFIG": "/tmp/laio.yaml"})
    );
    Ok(())
}

#[test]
fn workspace_report_metadata_chunks_long_value() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![ok_response()]);

    session.client().workspace_report_metadata(
        session.name(),
        &WorkspaceReportMetadataParams {
            workspace_id: "w1".to_string(),
            source: "laio".to_string(),
            tokens: HashMap::from([("LAIO_VARS".to_string(), "a".repeat(81))]),
            seq: None,
        },
    )?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;
    let tokens = request["params"]["tokens"]
        .as_object()
        .ok_or_else(|| miette!("tokens should be a JSON object"))?;

    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens["LAIO_VARS_1"].as_str().map(str::len), Some(80));
    assert_eq!(tokens["LAIO_VARS_2"].as_str().map(str::len), Some(1));
    Ok(())
}

#[test]
fn workspace_report_metadata_splits_requests_at_sixteen_keys() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![ok_response(), ok_response()]);

    session.client().workspace_report_metadata(
        session.name(),
        &WorkspaceReportMetadataParams {
            workspace_id: "w1".to_string(),
            source: "laio".to_string(),
            tokens: HashMap::from([("LAIO_VARS".to_string(), "a".repeat(80 * 16 + 1))]),
            seq: None,
        },
    )?;

    let requests = join_thread(handle)?;
    assert_eq!(requests.len(), 2);

    let first = parse_request(&requests[0])?;
    let second = parse_request(&requests[1])?;

    assert_eq!(
        first["params"]["tokens"].as_object().map(|t| t.len()),
        Some(16)
    );
    assert_eq!(
        second["params"]["tokens"].as_object().map(|t| t.len()),
        Some(1)
    );
    assert!(second["params"]["tokens"].get("LAIO_VARS_17").is_some());
    Ok(())
}

#[test]
fn workspace_report_metadata_rejects_more_than_thirty_two_keys() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, Vec::new());

    let error = session
        .client()
        .workspace_report_metadata(
            session.name(),
            &WorkspaceReportMetadataParams {
                workspace_id: "w1".to_string(),
                source: "laio".to_string(),
                tokens: HashMap::from([("LAIO_VARS".to_string(), "a".repeat(80 * 32 + 1))]),
                seq: None,
            },
        )
        .err()
        .ok_or_else(|| miette!("exceeding the key limit should return Err"))?;

    let requests = join_thread(handle)?;

    assert!(
        error.to_string().contains("32"),
        "report should name the key limit: {error}"
    );
    assert!(requests.is_empty(), "no request should be sent");
    Ok(())
}

#[test]
fn layout_apply_sends_root_tree_and_parses_layout() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![layout_response("layout_apply")]);

    let layout = session.client().layout_apply(
        session.name(),
        &LayoutApplyParams {
            workspace_id: Some("w1".to_string()),
            tab_id: None,
            tab_label: Some("main".to_string()),
            focus: true,
            root: LayoutNode::Split {
                direction: SplitDirection::Right,
                ratio: 0.5,
                first: Box::new(LayoutNode::Pane {
                    pane: LayoutPane {
                        cwd: Some("/tmp/a".to_string()),
                        ..Default::default()
                    },
                }),
                second: Box::new(LayoutNode::Pane {
                    pane: LayoutPane {
                        label: Some("logs".to_string()),
                        ..Default::default()
                    },
                }),
            },
        },
    )?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(layout.tab_id, "t1");
    assert_eq!(layout.focused_pane_id, "p1");
    match &layout.root {
        LayoutNode::Split {
            direction, ratio, ..
        } => {
            assert_eq!(*direction, SplitDirection::Right);
            assert_eq!(*ratio, 0.5);
        }
        other => return Err(miette!("root should be a split, got {other:?}")),
    }
    assert_eq!(request["method"], json!("layout.apply"));
    assert_eq!(request["params"]["workspace_id"], json!("w1"));
    assert_eq!(request["params"]["tab_label"], json!("main"));
    assert_eq!(request["params"]["focus"], json!(true));
    assert_eq!(request["params"]["root"]["type"], json!("split"));
    assert_eq!(request["params"]["root"]["direction"], json!("right"));
    assert_eq!(request["params"]["root"]["first"]["type"], json!("pane"));
    assert_eq!(request["params"]["root"]["first"]["cwd"], json!("/tmp/a"));
    assert_eq!(request["params"]["root"]["second"]["label"], json!("logs"));
    Ok(())
}

#[test]
fn layout_apply_sends_tab_id_and_omits_workspace_id() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![layout_response("layout_apply")]);

    session.client().layout_apply(
        session.name(),
        &LayoutApplyParams {
            workspace_id: None,
            tab_id: Some("w1:t1".to_string()),
            tab_label: Some("main".to_string()),
            focus: true,
            root: LayoutNode::Pane {
                pane: LayoutPane::default(),
            },
        },
    )?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(request["method"], json!("layout.apply"));
    assert_eq!(request["params"]["tab_id"], json!("w1:t1"));
    assert!(request["params"].get("workspace_id").is_none());
    Ok(())
}

#[test]
fn layout_apply_omits_both_ids_when_unset() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![layout_response("layout_apply")]);

    session.client().layout_apply(
        session.name(),
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

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(request["method"], json!("layout.apply"));
    assert!(request["params"].get("tab_id").is_none());
    assert!(request["params"].get("workspace_id").is_none());
    assert!(request["params"].get("tab_label").is_none());
    Ok(())
}

#[test]
fn layout_export_parses_layout() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![layout_response("layout_export")]);

    let layout = session.client().layout_export(session.name(), "t1")?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(layout.workspace_id, "w1");
    assert_eq!(layout.tab_id, "t1");
    assert_eq!(request["method"], json!("layout.export"));
    assert_eq!(request["params"]["tab_id"], json!("t1"));
    Ok(())
}

#[test]
fn pane_focus_parses_pane_info() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![pane_info_response()]);

    let pane = session.client().pane_focus(session.name(), "p1")?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(pane.pane_id, "p1");
    assert_eq!(pane.cwd.as_deref(), Some("/tmp/a"));
    assert_eq!(pane.label.as_deref(), Some("editor"));
    assert_eq!(request["method"], json!("pane.focus"));
    assert_eq!(request["params"]["pane_id"], json!("p1"));
    Ok(())
}

#[test]
fn pane_zoom_sends_mode_and_parses_result() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![pane_zoom_response()]);

    let zoom = session
        .client()
        .pane_zoom(session.name(), "p1", PaneZoomMode::On)?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(zoom.pane_id, "p1");
    assert!(zoom.zoomed);
    assert_eq!(request["method"], json!("pane.zoom"));
    assert_eq!(request["params"]["pane_id"], json!("p1"));
    assert_eq!(request["params"]["mode"], json!("on"));
    Ok(())
}

#[test]
fn pane_send_input_sends_text_and_keys() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![ok_response()]);

    session.client().pane_send_input(
        session.name(),
        &PaneSendInputParams {
            pane_id: "p1".to_string(),
            text: "cargo test".to_string(),
            keys: vec!["Enter".to_string()],
        },
    )?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(request["method"], json!("pane.send_input"));
    assert_eq!(request["params"]["pane_id"], json!("p1"));
    assert_eq!(request["params"]["text"], json!("cargo test"));
    assert_eq!(request["params"]["keys"], json!(["Enter"]));
    Ok(())
}

#[test]
fn chunk_token_splits_values_longer_than_eighty_chars() -> Result<()> {
    let short = chunk_token("LAIO_VARS", &"a".repeat(80));
    assert_eq!(short, vec![("LAIO_VARS".to_string(), "a".repeat(80))]);

    let long = chunk_token("LAIO_VARS", &"a".repeat(81));
    assert_eq!(long.len(), 2);
    assert_eq!(long[0].0, "LAIO_VARS_1");
    assert_eq!(long[0].1.len(), 80);
    assert_eq!(long[1].0, "LAIO_VARS_2");
    assert_eq!(long[1].1.len(), 1);
    Ok(())
}

#[test]
fn chunk_tokens_orders_keys_and_counts_chunks() -> Result<()> {
    let tokens = HashMap::from([
        ("LAIO_VARS".to_string(), "a".repeat(81)),
        ("LAIO_CONFIG".to_string(), "/tmp/laio.yaml".to_string()),
    ]);

    let pairs = chunk_tokens(&tokens);
    let keys: Vec<&str> = pairs.iter().map(|(key, _)| key.as_str()).collect();

    assert_eq!(keys, vec!["LAIO_CONFIG", "LAIO_VARS_1", "LAIO_VARS_2"]);
    Ok(())
}

#[test]
fn join_tokens_joins_chunks_in_numeric_order() -> Result<()> {
    let joined = join_tokens(HashMap::from([
        ("LAIO_VARS_10".to_string(), "c".to_string()),
        ("LAIO_VARS_2".to_string(), "b".to_string()),
        ("LAIO_VARS_1".to_string(), "a".to_string()),
        ("LAIO_CONFIG".to_string(), "/tmp/laio.yaml".to_string()),
    ]));

    assert_eq!(joined.get("LAIO_VARS"), Some(&"abc".to_string()));
    assert_eq!(
        joined.get("LAIO_CONFIG"),
        Some(&"/tmp/laio.yaml".to_string())
    );
    Ok(())
}

#[test]
fn split_chunk_key_recognises_only_numeric_suffixes() -> Result<()> {
    assert_eq!(split_chunk_key("LAIO_VARS_1"), Some(("LAIO_VARS", 1)));
    assert_eq!(split_chunk_key("LAIO_VARS_10"), Some(("LAIO_VARS", 10)));
    assert_eq!(split_chunk_key("LAIO_VARS"), None);
    assert_eq!(split_chunk_key("LAIO_VARS_0"), None);
    assert_eq!(split_chunk_key("LAIO_VARS_x"), None);
    assert_eq!(split_chunk_key("_1"), None);
    Ok(())
}

#[test]
fn chunk_and_join_round_trip() -> Result<()> {
    let value = "x".repeat(200);
    let pairs = chunk_token("LAIO_VARS", &value);
    let joined = join_tokens(pairs.into_iter().collect());

    assert_eq!(joined.get("LAIO_VARS"), Some(&value));
    Ok(())
}

#[test]
fn ensure_server_returns_ok_without_spawning_when_ping_succeeds() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![pong_response()]);

    session.client().ensure_server(session.name())?;

    let requests = join_thread(handle)?;
    assert_eq!(requests.len(), 1);
    Ok(())
}

#[test]
fn ensure_server_spawns_server_with_session_env_and_polls() -> Result<()> {
    let session = TestSession::new()?;
    let socket_path = session.socket_path()?;

    let mut cmd_unit = MockCmdUnitMock::new();
    let path = socket_path.clone();
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| {
            matches!(cmd, Type::Forget(_) if cmd.to_string() == "HERDR_SESSION=w herdr server")
        })
        .returning(move |_| {
            let parent = path
                .parent()
                .ok_or_else(|| miette!("socket path should have a parent directory"))?;
            fs::create_dir_all(parent).into_diagnostic()?;
            let listener = bind_listener(&path)?;
            let _ = spawn_canned_server(listener, vec![pong_response()]);
            Ok(())
        });

    let runner = RunnerMock {
        cmd_unit,
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    };
    let client = HerdrClient::new_with_runner(runner);

    client.ensure_server(session.name())?;
    Ok(())
}

#[test]
fn ensure_server_times_out_naming_the_socket_path() -> Result<()> {
    let session = TestSession::new()?;
    let socket_path = session.socket_path()?;

    let mut cmd_unit = MockCmdUnitMock::new();
    cmd_unit.expect_run().times(1).returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    };
    let client = HerdrClient::new_with_runner(runner);

    let error = client
        .ensure_server_with_timeout(session.name(), Duration::from_millis(100))
        .err()
        .ok_or_else(|| miette!("missing server should return Err"))?;

    assert!(
        error
            .to_string()
            .contains(socket_path.to_string_lossy().as_ref()),
        "report should name the socket path: {error}"
    );
    Ok(())
}

#[test]
fn session_exists_returns_true_when_socket_connects() -> Result<()> {
    let session = TestSession::new()?;
    let client = session.client();

    assert!(!client.session_exists(session.name()));

    let listener = session.bind()?;
    assert!(client.session_exists(session.name()));

    drop(listener);
    Ok(())
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

fn workspace_created() -> WorkspaceCreated {
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

fn workspace_info() -> WorkspaceInfo {
    WorkspaceInfo {
        workspace_id: "w1".to_string(),
        label: "main".to_string(),
        focused: true,
        tokens: HashMap::new(),
    }
}

fn layout_description() -> LayoutDescription {
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

fn pane_info() -> PaneInfo {
    PaneInfo {
        pane_id: "p1".to_string(),
        workspace_id: "w1".to_string(),
        tab_id: "t1".to_string(),
        focused: true,
        cwd: None,
        label: None,
    }
}

fn pane_zoom_result() -> PaneZoomResult {
    PaneZoomResult {
        pane_id: "p1".to_string(),
        focused_pane_id: "p1".to_string(),
        zoomed: true,
    }
}

fn test_session(windows: Vec<Window>) -> Session {
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

fn test_window(name: &str, flex_direction: FlexDirection, panes: Vec<Pane>) -> Window {
    Window {
        name: name.to_string(),
        path: None,
        flex_direction,
        panes,
        focus: false,
    }
}

fn test_pane(flex: usize, name: Option<&str>, path: &str) -> Pane {
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

fn pane_node(pane_id: &str) -> LayoutNode {
    LayoutNode::Pane {
        pane: LayoutPane {
            pane_id: Some(pane_id.to_string()),
            ..Default::default()
        },
    }
}

fn contains_key(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(map) => {
            map.contains_key(key) || map.values().any(|value| contains_key(value, key))
        }
        Value::Array(items) => items.iter().any(|value| contains_key(value, key)),
        _ => false,
    }
}

#[test]
fn build_plans_uses_sequential_ratios() -> Result<()> {
    let session = test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![
            test_pane(1, None, "."),
            test_pane(1, None, "."),
            test_pane(1, None, "."),
        ],
    )]);

    let plans = build_plans(&session)?;
    let root = &plans[0].root;

    let LayoutNode::Split { ratio, second, .. } = root else {
        return Err(miette!("root should be a split, got {root:?}"));
    };
    assert!(
        (*ratio - 1.0 / 3.0).abs() < 1e-6,
        "first ratio should be 1/3, got {ratio}"
    );

    let LayoutNode::Split { ratio, .. } = second.as_ref() else {
        return Err(miette!("second node should be a split, got {second:?}"));
    };
    assert!(
        (*ratio - 0.5).abs() < 1e-6,
        "second ratio should be 1/2, got {ratio}"
    );
    Ok(())
}

#[test]
fn build_plans_clamps_ratios_outside_the_valid_range() -> Result<()> {
    let session = test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![test_pane(1, None, "."), test_pane(10, None, ".")],
    )]);

    let plans = build_plans(&session)?;
    let root = &plans[0].root;

    let LayoutNode::Split { ratio, .. } = root else {
        return Err(miette!("root should be a split, got {root:?}"));
    };
    assert_eq!(*ratio, 0.1);
    Ok(())
}

#[test]
fn build_plans_rejects_more_than_twenty_four_panes() -> Result<()> {
    let panes = (0..25).map(|_| test_pane(1, None, ".")).collect();
    let error = build_plans(&test_session(vec![test_window(
        "wide",
        FlexDirection::Row,
        panes,
    )]))
    .err()
    .ok_or_else(|| miette!("25 panes should return Err"))?;

    let report = error.to_string();
    assert!(
        report.contains("wide"),
        "report should name the window: {report}"
    );
    assert!(
        report.contains("25"),
        "report should name the pane count: {report}"
    );
    assert!(
        report.contains("24"),
        "report should name the pane limit: {report}"
    );
    Ok(())
}

#[test]
fn build_plans_rejects_split_depth_above_sixteen() -> Result<()> {
    let panes = (0..17).map(|_| test_pane(1, None, ".")).collect();
    let error = build_plans(&test_session(vec![test_window(
        "narrow",
        FlexDirection::Row,
        panes,
    )]))
    .err()
    .ok_or_else(|| miette!("17 levels should return Err"))?;

    let report = error.to_string();
    assert!(
        report.contains("narrow"),
        "report should name the window: {report}"
    );
    assert!(
        report.contains("depth"),
        "report should name the depth limit: {report}"
    );
    assert!(
        report.contains("16"),
        "report should name the depth limit value: {report}"
    );
    Ok(())
}

#[test]
fn build_plans_maps_flex_direction_to_split_direction() -> Result<()> {
    let row = build_plans(&test_session(vec![test_window(
        "row",
        FlexDirection::Row,
        vec![test_pane(1, None, "."), test_pane(1, None, ".")],
    )]))?;
    let column = build_plans(&test_session(vec![test_window(
        "column",
        FlexDirection::Column,
        vec![test_pane(1, None, "."), test_pane(1, None, ".")],
    )]))?;

    let LayoutNode::Split { direction, .. } = &row[0].root else {
        return Err(miette!("row root should be a split"));
    };
    assert_eq!(*direction, SplitDirection::Right);

    let LayoutNode::Split { direction, .. } = &column[0].root else {
        return Err(miette!("column root should be a split"));
    };
    assert_eq!(*direction, SplitDirection::Down);
    Ok(())
}

#[test]
fn build_plans_resolves_pane_cwd_and_label() -> Result<()> {
    let mut window = test_window(
        "main",
        FlexDirection::Row,
        vec![test_pane(1, Some("editor"), "."), test_pane(1, None, "src")],
    );
    window.path = Some("sub".to_string());

    let plans = build_plans(&test_session(vec![window]))?;
    let root = &plans[0].root;

    let LayoutNode::Split { first, second, .. } = root else {
        return Err(miette!("root should be a split, got {root:?}"));
    };

    let LayoutNode::Pane { pane } = first.as_ref() else {
        return Err(miette!("first node should be a pane"));
    };
    assert_eq!(pane.cwd.as_deref(), Some("/tmp/project/sub"));
    assert_eq!(pane.label.as_deref(), Some("editor"));

    let LayoutNode::Pane { pane } = second.as_ref() else {
        return Err(miette!("second node should be a pane"));
    };
    assert_eq!(pane.cwd.as_deref(), Some("/tmp/project/sub/src"));
    assert_eq!(pane.label, None);

    assert_eq!(plans[0].leaves[0].name.as_deref(), Some("editor"));
    assert_eq!(plans[0].leaves[1].name, None);
    Ok(())
}

#[test]
fn build_plans_omits_the_command_field() -> Result<()> {
    let mut pane = test_pane(1, None, ".");
    pane.commands = vec![Command::from_string("echo hi")];

    let plans = build_plans(&test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![pane],
    )]))?;

    let json = serde_json::to_value(&plans[0].root).into_diagnostic()?;
    assert!(
        !contains_key(&json, "command"),
        "pane nodes must not carry a command field: {json}"
    );
    assert_eq!(plans[0].leaves[0].commands, vec!["echo hi".to_string()]);
    Ok(())
}

#[test]
fn build_plans_includes_script_command() -> Result<()> {
    let script: Script = noyalib::compat::serde_yaml::from_str("echo hi").into_diagnostic()?;
    let mut pane = test_pane(1, None, ".");
    pane.script = Some(script);

    let plans = build_plans(&test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![pane],
    )]))?;

    let commands = &plans[0].leaves[0].commands;
    assert_eq!(commands.len(), 1);
    assert!(
        commands[0].contains("laio-"),
        "script command should point at the materialized script: {commands:?}"
    );
    Ok(())
}

#[test]
fn build_plans_rejects_multiple_focused_windows() -> Result<()> {
    let mut first = test_window("first", FlexDirection::Row, vec![test_pane(1, None, ".")]);
    first.focus = true;
    let mut second = test_window("second", FlexDirection::Row, vec![test_pane(1, None, ".")]);
    second.focus = true;

    let error = build_plans(&test_session(vec![first, second]))
        .err()
        .ok_or_else(|| miette!("two focused windows should return Err"))?;

    assert!(
        error.to_string().contains("focus"),
        "report should name the focus conflict: {error}"
    );
    Ok(())
}

#[test]
fn build_plans_sets_focus_tab_for_the_focused_window() -> Result<()> {
    let mut focused = test_window("focused", FlexDirection::Row, vec![test_pane(1, None, ".")]);
    focused.focus = true;
    let other = test_window("other", FlexDirection::Row, vec![test_pane(1, None, ".")]);

    let plans = build_plans(&test_session(vec![focused, other]))?;

    assert_eq!(plans.len(), 2);
    assert_eq!(plans[0].tab_label, "focused");
    assert!(plans[0].focus_tab);
    assert_eq!(plans[1].tab_label, "other");
    assert!(!plans[1].focus_tab);
    Ok(())
}

#[test]
fn build_plans_handles_window_without_panes() -> Result<()> {
    let plans = build_plans(&test_session(vec![test_window(
        "empty",
        FlexDirection::Row,
        Vec::new(),
    )]))?;

    assert_eq!(plans[0].leaves.len(), 1);
    let LayoutNode::Pane { pane } = &plans[0].root else {
        return Err(miette!("empty window root should be a pane"));
    };
    assert_eq!(pane.cwd.as_deref(), Some("/tmp/project"));
    Ok(())
}

#[test]
fn bind_pane_ops_emits_ops_in_leaf_order() -> Result<()> {
    let mut first = test_pane(1, None, ".");
    first.zoom = true;
    first.focus = true;
    first.commands = vec![
        Command::from_string("echo one"),
        Command::from_string("echo two"),
    ];

    let mut third = test_pane(1, None, ".");
    third.commands = vec![Command::from_string("echo three")];

    let plans = build_plans(&test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![first, test_pane(1, None, "."), third],
    )]))?;

    let layout = LayoutDescription {
        workspace_id: "w1".to_string(),
        tab_id: "t1".to_string(),
        zoomed: false,
        focused_pane_id: "p1".to_string(),
        root: LayoutNode::Split {
            direction: SplitDirection::Right,
            ratio: 1.0 / 3.0,
            first: Box::new(pane_node("p1")),
            second: Box::new(LayoutNode::Split {
                direction: SplitDirection::Right,
                ratio: 0.5,
                first: Box::new(pane_node("p2")),
                second: Box::new(pane_node("p3")),
            }),
        },
    };

    let ops = bind_pane_ops(&plans[0], &layout)?;
    let enter = vec!["Enter".to_string()];

    assert_eq!(
        ops,
        vec![
            PaneOp::Zoom("p1".to_string()),
            PaneOp::Focus("p1".to_string()),
            PaneOp::SendInput {
                pane_id: "p1".to_string(),
                text: "echo one".to_string(),
                keys: enter.clone(),
            },
            PaneOp::SendInput {
                pane_id: "p1".to_string(),
                text: "echo two".to_string(),
                keys: enter.clone(),
            },
            PaneOp::SendInput {
                pane_id: "p3".to_string(),
                text: "echo three".to_string(),
                keys: enter,
            },
        ]
    );
    Ok(())
}

#[test]
fn bind_pane_ops_rejects_mismatched_pane_count() -> Result<()> {
    let plans = build_plans(&test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![test_pane(1, None, "."), test_pane(1, None, ".")],
    )]))?;

    let layout = LayoutDescription {
        workspace_id: "w1".to_string(),
        tab_id: "t1".to_string(),
        zoomed: false,
        focused_pane_id: "p1".to_string(),
        root: pane_node("p1"),
    };

    let error = bind_pane_ops(&plans[0], &layout)
        .err()
        .ok_or_else(|| miette!("mismatched pane count should return Err"))?;

    assert!(
        error.to_string().contains("main"),
        "report should name the tab: {error}"
    );
    Ok(())
}

#[test]
fn bind_pane_ops_rejects_layout_pane_without_id() -> Result<()> {
    let plans = build_plans(&test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![test_pane(1, None, ".")],
    )]))?;

    let layout = LayoutDescription {
        workspace_id: "w1".to_string(),
        tab_id: "t1".to_string(),
        zoomed: false,
        focused_pane_id: "p1".to_string(),
        root: LayoutNode::Pane {
            pane: LayoutPane::default(),
        },
    };

    let error = bind_pane_ops(&plans[0], &layout)
        .err()
        .ok_or_else(|| miette!("missing pane_id should return Err"))?;

    assert!(
        error.to_string().contains("pane_id"),
        "report should name the missing pane_id: {error}"
    );
    Ok(())
}

fn workspace_list_response(workspaces: Value) -> String {
    json!({
        "id": "1",
        "result": {"type": "workspace_list", "workspaces": workspaces}
    })
    .to_string()
        + "\n"
}

fn workspace_entry(label: &str, tokens: Value) -> Value {
    json!({
        "workspace_id": "w1",
        "label": label,
        "focused": true,
        "tokens": tokens
    })
}

fn laio_tokens() -> HashMap<String, String> {
    HashMap::from([("LAIO_CONFIG".to_string(), "/tmp/laio.yaml".to_string())])
}

fn config_tokens(path: &str) -> HashMap<String, String> {
    HashMap::from([("LAIO_CONFIG".to_string(), path.to_string())])
}

fn workspace_info_with(
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

fn clear_herdr_socket_path() -> Option<OsString> {
    let previous = env::var_os("HERDR_SOCKET_PATH");

    unsafe {
        env::remove_var("HERDR_SOCKET_PATH");
    }

    previous
}

fn set_herdr_socket_path(value: &str) -> Option<OsString> {
    let previous = env::var_os("HERDR_SOCKET_PATH");

    unsafe {
        env::set_var("HERDR_SOCKET_PATH", value);
    }

    previous
}

fn clear_herdr_workspace_id() -> Option<OsString> {
    let previous = env::var_os("HERDR_WORKSPACE_ID");

    unsafe {
        env::remove_var("HERDR_WORKSPACE_ID");
    }

    previous
}

fn set_herdr_workspace_id(value: &str) -> Option<OsString> {
    let previous = env::var_os("HERDR_WORKSPACE_ID");

    unsafe {
        env::set_var("HERDR_WORKSPACE_ID", value);
    }

    previous
}

#[test]
fn stop_requires_a_name_outside_a_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let result = herdr.stop(&None, &None, false, false, false);
    restore_env("HERDR_ENV", previous);

    let error = result
        .err()
        .ok_or_else(|| miette!("missing name outside a session should return Err"))?;

    assert_eq!(error.to_string(), "Specify laio session you want to stop.");
    Ok(())
}

#[test]
fn stop_other_requires_a_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let result = herdr.stop(&None, &None, false, false, true);
    restore_env("HERDR_ENV", previous);

    let error = result
        .err()
        .ok_or_else(|| miette!("stop_other outside a session should return Err"))?;

    assert_eq!(error.to_string(), "Not inside a laio session.");
    Ok(())
}

#[test]
fn stop_rejects_all_or_other_with_a_name() -> Result<()> {
    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());

    let all = herdr
        .stop(&Some("work".to_string()), &None, false, true, false)
        .err()
        .ok_or_else(|| miette!("stop_all with a name should return Err"))?;
    assert_eq!(
        all.to_string(),
        "Stopping all/other and specifying a session name are mutually exclusive."
    );

    let other = herdr
        .stop(&Some("work".to_string()), &None, false, false, true)
        .err()
        .ok_or_else(|| miette!("stop_other with a name should return Err"))?;
    assert_eq!(
        other.to_string(),
        "Stopping all/other and specifying a session name are mutually exclusive."
    );
    Ok(())
}

#[test]
fn stop_runs_shutdown_hooks_before_closing_the_workspace() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let mut session = start_session(Vec::new());
    session.shutdown = vec![
        Command::from_string("date"),
        Command::from_string("echo Bye"),
    ];
    session.shutdown_script =
        Some(noyalib::compat::serde_yaml::from_str("echo bye").into_diagnostic()?);

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", laio_tokens())]));
    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "date"))
        .returning(|_| Ok(String::new()));
    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "echo Bye"))
        .returning(|_| Ok(String::new()));
    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string().contains("laio-")))
        .returning(|_| Ok(String::new()));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == LAIO_SESSION_NAME && workspace_id == "w1")
        .returning(|_, _| Ok(()));
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(Vec::new()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session stop laio"))
        .returning(|_| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session delete laio"),
        )
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(
        &Some("work".to_string()),
        &Some(session),
        false,
        false,
        false,
    );
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_closes_the_workspace_when_shutdown_hooks_fail() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let mut session = start_session(Vec::new());
    session.shutdown = vec![Command::from_string("date")];

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", laio_tokens())]));
    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "date"))
        .returning(|_| Err(miette!("hook failed")));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == LAIO_SESSION_NAME && workspace_id == "w1")
        .returning(|_, _| Ok(()));
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(Vec::new()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session stop laio"))
        .returning(|_| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session delete laio"),
        )
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(
        &Some("work".to_string()),
        &Some(session),
        false,
        false,
        false,
    );
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    let error = result
        .err()
        .ok_or_else(|| miette!("a failing hook should surface an error"))?;

    assert!(
        format!("{error:?}").contains("date"),
        "report should carry the hook failure: {error:?}"
    );
    Ok(())
}

#[test]
fn stop_keeps_the_session_when_hooks_fail_and_an_owned_workspace_remains() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let mut session = start_session(Vec::new());
    session.shutdown = vec![Command::from_string("date")];

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();

    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", laio_tokens())]));
    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "date"))
        .returning(|_| Err(miette!("hook failed")));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == LAIO_SESSION_NAME && workspace_id == "w1")
        .returning(|_, _| Ok(()));
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(vec![workspace_info_with("w3", "late", laio_tokens())]));

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(
        &Some("work".to_string()),
        &Some(session),
        false,
        false,
        false,
    );
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    assert!(result.is_err(), "a failing hook should surface an error");
    Ok(())
}

#[test]
fn stop_skips_shutdown_hooks_and_tears_down_the_laio_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let mut session = start_session(Vec::new());
    session.shutdown = vec![Command::from_string("date")];

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", laio_tokens())]));
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == LAIO_SESSION_NAME && workspace_id == "w1")
        .returning(|_, _| Ok(()));
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(Vec::new()));

    let mut cmd_unit = MockCmdUnitMock::new();
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session stop laio"))
        .returning(|_| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session delete laio"),
        )
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(
        &Some("work".to_string()),
        &Some(session),
        true,
        false,
        false,
    );
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_all_closes_every_owned_workspace() -> Result<()> {
    let _guard = env_lock()?;
    let previous = set_herdr_env("1");

    let sessions = json!({
        "sessions": [
            {"name": "current", "running": true},
            {"name": "mine", "running": true},
            {"name": "other", "running": true}
        ]
    })
    .to_string();

    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list()
        .withf(|name| name == "current")
        .times(1)
        .returning(|_| {
            Ok(vec![workspace_info_with(
                "w-current",
                "current",
                laio_tokens(),
            )])
        });
    api.expect_workspace_list()
        .withf(|name| name == "mine")
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w-mine", "mine", laio_tokens())]));
    api.expect_workspace_list()
        .withf(|name| name == "other")
        .times(1)
        .returning(|_| {
            Ok(vec![workspace_info_with(
                "w-other",
                "other",
                HashMap::new(),
            )])
        });
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == "current" && workspace_id == "w-current")
        .returning(|_, _| Ok(()));
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == "mine" && workspace_id == "w-mine")
        .returning(|_, _| Ok(()));
    api.expect_session_exists()
        .withf(|name| name == "laio")
        .times(1)
        .returning(|_| false);

    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, true, true, false);
    restore_env("HERDR_ENV", previous);

    result
}

#[test]
fn stop_other_closes_other_workspaces_and_keeps_the_current_one() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w-current");
    let previous_socket = set_herdr_socket_path("/tmp/herdr/sessions/current/herdr.sock");

    let sessions = json!({
        "sessions": [
            {"name": "current", "running": true},
            {"name": "mine", "running": true}
        ]
    })
    .to_string();

    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list()
        .withf(|name| name == "mine")
        .times(1)
        .returning(|_| {
            Ok(vec![workspace_info_with(
                "w-mine",
                "mine",
                config_tokens("/tmp/mine.yaml"),
            )])
        });
    api.expect_workspace_list()
        .withf(|name| name == "current")
        .times(1)
        .returning(|_| {
            Ok(vec![workspace_info_with(
                "w-current",
                "current",
                config_tokens("/tmp/current.yaml"),
            )])
        });
    api.expect_workspace_get()
        .withf(|session, workspace_id| session == "current" && workspace_id == "w-current")
        .times(2)
        .returning(|_, _| {
            Ok(workspace_info_with(
                "w-current",
                "current",
                config_tokens("/tmp/current.yaml"),
            ))
        });
    api.expect_workspace_get()
        .withf(|session, workspace_id| session == "mine" && workspace_id == "w-mine")
        .times(1)
        .returning(|_, _| {
            Ok(workspace_info_with(
                "w-mine",
                "mine",
                config_tokens("/tmp/mine.yaml"),
            ))
        });
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == "mine" && workspace_id == "w-mine")
        .returning(|_, _| Ok(()));
    api.expect_session_exists()
        .withf(|name| name == "laio")
        .times(1)
        .returning(|_| false);

    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, true, false, true);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_other_skips_hooks_for_the_current_workspace() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w-current");
    let previous_socket = set_herdr_socket_path("/tmp/herdr/sessions/current/herdr.sock");

    let sessions = json!({"sessions": [{"name": "current", "running": true}]}).to_string();

    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list()
        .withf(|name| name == "current")
        .times(1)
        .returning(|_| {
            Ok(vec![workspace_info_with(
                "w-current",
                "current",
                config_tokens("/tmp/current.yaml"),
            )])
        });
    api.expect_workspace_get().times(2).returning(|_, _| {
        Ok(workspace_info_with(
            "w-current",
            "current",
            config_tokens("/tmp/current.yaml"),
        ))
    });
    api.expect_session_exists()
        .withf(|name| name == "laio")
        .times(1)
        .returning(|_| false);

    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, false, false, true);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_sweep_runs_token_loaded_shutdown_hooks_before_closing() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");

    let config_file = env::temp_dir().join(format!("laio-herdr-sweep-{}.yaml", std::process::id()));
    let config = "---\nname: sweep\npath: /tmp\nshutdown:\n  - command: echo\n    args:\n      - sweep-bye\nwindows:\n  - name: w\n    panes:\n      - flex: 1\n";
    fs::write(&config_file, config).into_diagnostic()?;
    let config_path = config_file.to_string_lossy().to_string();

    let sessions = json!({"sessions": [{"name": "laio", "running": true}]}).to_string();
    let tokens = config_tokens(&config_path);

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning({
            let tokens = tokens.clone();
            move |_| Ok(vec![workspace_info_with("w1", "sweep", tokens.clone())])
        });
    api.expect_workspace_get()
        .times(1)
        .in_sequence(&mut seq)
        .returning({
            let tokens = tokens.clone();
            move |_, _| Ok(workspace_info_with("w1", "sweep", tokens.clone()))
        });
    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "echo sweep-bye"))
        .returning(|_| Ok(String::new()));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == LAIO_SESSION_NAME && workspace_id == "w1")
        .returning(|_, _| Ok(()));
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(Vec::new()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session stop laio"))
        .returning(|_| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session delete laio"),
        )
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, false, true, false);
    restore_env("HERDR_ENV", previous_env);
    let _ = fs::remove_file(&config_file);

    result
}

#[test]
fn stop_sweep_warns_and_continues_without_laio_config() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");

    let sessions = json!({"sessions": [{"name": "laio", "running": true}]}).to_string();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "sweep", laio_tokens())]));
    api.expect_workspace_get()
        .times(1)
        .returning(|_, _| Ok(workspace_info_with("w1", "sweep", HashMap::new())));
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == LAIO_SESSION_NAME && workspace_id == "w1")
        .returning(|_, _| Ok(()));
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(Vec::new()));

    let mut cmd_unit = MockCmdUnitMock::new();
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session stop laio"))
        .returning(|_| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session delete laio"),
        )
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, false, true, false);
    restore_env("HERDR_ENV", previous_env);

    result
}

#[test]
fn stop_tears_down_the_laio_session_after_closing_the_last_workspace() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", laio_tokens())]));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == LAIO_SESSION_NAME && workspace_id == "w1")
        .returning(|_, _| Ok(()));
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(Vec::new()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session stop laio"))
        .returning(|_| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session delete laio"),
        )
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&Some("work".to_string()), &None, true, false, false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_tears_down_the_laio_session_when_a_tokenless_workspace_remains() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list().times(1).returning(|_| {
        Ok(vec![
            workspace_info_with("w1", "work", laio_tokens()),
            workspace_info_with("w2", "user", HashMap::new()),
        ])
    });
    api.expect_workspace_close()
        .times(1)
        .returning(|_, _| Ok(()));
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w2", "user", HashMap::new())]));

    let mut cmd_unit = MockCmdUnitMock::new();
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session stop laio"))
        .returning(|_| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session delete laio"),
        )
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&Some("work".to_string()), &None, true, false, false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_all_tears_down_only_the_laio_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");

    let sessions = json!({
        "sessions": [
            {"name": "laio", "running": true},
            {"name": "work", "running": true}
        ]
    })
    .to_string();

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|name| name == "laio")
        .returning(|_| Ok(vec![workspace_info_with("w-laio", "alpha", laio_tokens())]));
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|name| name == "work")
        .returning(|_| Ok(vec![workspace_info_with("w-work", "beta", laio_tokens())]));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w-laio")
        .returning(|_, _| Ok(()));
    api.expect_workspace_close()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, workspace_id| session == "work" && workspace_id == "w-work")
        .returning(|_, _| Ok(()));
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|name| name == "laio")
        .returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|name| name == "laio")
        .returning(|_| Ok(Vec::new()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session stop laio"))
        .returning(|_| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session delete laio"),
        )
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, true, true, false);
    restore_env("HERDR_ENV", previous_env);

    result
}

#[test]
fn stop_never_tears_down_a_foreign_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous_socket = set_herdr_socket_path("/tmp/herdr/sessions/work/herdr.sock");

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", laio_tokens())]));
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == "work" && workspace_id == "w1")
        .returning(|_, _| Ok(()));

    let herdr = Herdr::new_with_runner(empty_runner(), api);

    let result = herdr.stop(&Some("work".to_string()), &None, true, false, false);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_all_tears_down_an_empty_laio_session_with_no_owned_workspaces() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");

    let sessions = json!({"sessions": [{"name": "laio", "running": true}]}).to_string();

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(Vec::new()));
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(Vec::new()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session stop laio"))
        .returning(|_| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session delete laio"),
        )
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, true, true, false);
    restore_env("HERDR_ENV", previous_env);

    result
}

#[test]
fn stop_all_tears_down_the_laio_session_when_a_tokenless_workspace_remains() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");

    let sessions = json!({"sessions": [{"name": "laio", "running": true}]}).to_string();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_workspace_list().times(1).returning(|_| {
        Ok(vec![
            workspace_info_with("w1", "alpha", laio_tokens()),
            workspace_info_with("w2", "user", HashMap::new()),
        ])
    });
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == "laio" && workspace_id == "w1")
        .returning(|_, _| Ok(()));
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w2", "user", HashMap::new())]));
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session stop laio"))
        .returning(|_| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session delete laio"),
        )
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&None, &None, true, true, false);
    restore_env("HERDR_ENV", previous_env);

    result
}

#[test]
fn stop_keeps_the_laio_session_when_an_owned_workspace_remains() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", laio_tokens())]));
    api.expect_workspace_close()
        .times(1)
        .returning(|_, _| Ok(()));
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w3", "late", laio_tokens())]));

    let herdr = Herdr::new_with_runner(empty_runner(), api);

    let result = herdr.stop(&Some("work".to_string()), &None, true, false, false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_returns_ok_for_a_workspace_without_laio_config() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(2)
        .returning(|_| Ok(vec![workspace_info_with("w1", "other", HashMap::new())]));

    let herdr = Herdr::new_with_runner(empty_runner(), api);

    let result = herdr.stop(&Some("other".to_string()), &None, true, false, false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_bails_when_the_named_config_has_no_workspace() -> Result<()> {
    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(2)
        .returning(|_| Ok(vec![workspace_info_with("w1", "other", laio_tokens())]));

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let error = herdr
        .stop(&Some("work".to_string()), &None, true, false, false)
        .err()
        .ok_or_else(|| miette!("missing workspace should return Err"))?;

    assert_eq!(error.to_string(), "Session work does not exist!");
    Ok(())
}

#[test]
fn stop_bails_when_the_server_is_not_running() -> Result<()> {
    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| false);

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let error = herdr
        .stop(&Some("work".to_string()), &None, true, false, false)
        .err()
        .ok_or_else(|| miette!("missing server should return Err"))?;

    assert_eq!(error.to_string(), "Session work does not exist!");
    Ok(())
}

#[test]
fn stop_resolves_the_current_workspace_label_when_no_name_is_given() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w-current");
    let previous_socket = set_herdr_socket_path("/tmp/herdr/sessions/current/herdr.sock");

    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list().times(2).returning(|_| {
        Ok(vec![workspace_info_with(
            "w-current",
            "current",
            laio_tokens(),
        )])
    });
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == "current" && workspace_id == "w-current")
        .returning(|_, _| Ok(()));

    let herdr = Herdr::new_with_runner(empty_runner(), api);

    let result = herdr.stop(&None, &None, true, false, false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn stop_returns_ok_when_the_current_workspace_is_not_laio_owned() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w-other");
    let previous_socket = set_herdr_socket_path("/tmp/herdr/sessions/current/herdr.sock");

    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list().times(1).returning(|_| {
        Ok(vec![workspace_info_with(
            "w-other",
            "other",
            HashMap::new(),
        )])
    });

    let herdr = Herdr::new_with_runner(empty_runner(), api);

    let result = herdr.stop(&None, &None, true, false, false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn get_session_config_path_joins_chunked_tokens() -> Result<()> {
    let session = TestSession::new()?;
    let previous_socket = set_herdr_socket_path(&session.socket_path()?.to_string_lossy());
    let listener = session.bind()?;
    let handle = spawn_canned_server(
        listener,
        vec![
            workspace_list_response(json!([workspace_entry(
                session.name(),
                json!({"LAIO_CONFIG": "/tmp/laio.yaml"})
            )])),
            workspace_info_response(json!({
                "LAIO_CONFIG_1": "/tmp/la",
                "LAIO_CONFIG_2": "io.yaml"
            })),
        ],
    );

    let herdr =
        Herdr::new_with_runner(empty_runner(), HerdrClient::new_with_runner(empty_runner()));
    let config = herdr.get_session_config_path(session.name())?;

    let _ = join_thread(handle)?;
    restore_env("HERDR_SOCKET_PATH", previous_socket);
    assert_eq!(config.as_deref(), Some("/tmp/laio.yaml"));
    Ok(())
}

#[test]
fn get_session_config_path_returns_none_when_not_running() -> Result<()> {
    let session = TestSession::new()?;
    let herdr = Herdr::new_with_runner(
        runner_with_session_list(empty_session_list()),
        HerdrClient::new_with_runner(empty_runner()),
    );

    assert_eq!(herdr.get_session_config_path(session.name())?, None);
    Ok(())
}

#[test]
fn get_session_variables_joins_and_decodes_chunked_tokens() -> Result<()> {
    let session = TestSession::new()?;
    let previous_socket = set_herdr_socket_path(&session.socket_path()?.to_string_lossy());
    let listener = session.bind()?;
    let handle = spawn_canned_server(
        listener,
        vec![
            workspace_list_response(json!([workspace_entry(
                session.name(),
                json!({"LAIO_CONFIG": "/tmp/laio.yaml", "LAIO_VARS": "a=1&b=2"})
            )])),
            workspace_info_response(json!({
                "LAIO_VARS_1": "a=1&b",
                "LAIO_VARS_2": "=2"
            })),
        ],
    );

    let herdr =
        Herdr::new_with_runner(empty_runner(), HerdrClient::new_with_runner(empty_runner()));
    let variables = herdr.get_session_variables(session.name())?;

    let _ = join_thread(handle)?;
    restore_env("HERDR_SOCKET_PATH", previous_socket);
    assert_eq!(variables, Some(vec!["a=1".to_string(), "b=2".to_string()]));
    Ok(())
}

#[test]
fn get_session_variables_returns_none_when_absent() -> Result<()> {
    let session = TestSession::new()?;
    let previous_socket = set_herdr_socket_path(&session.socket_path()?.to_string_lossy());
    let listener = session.bind()?;
    let handle = spawn_canned_server(
        listener,
        vec![
            workspace_list_response(json!([workspace_entry(
                session.name(),
                json!({"LAIO_CONFIG": "/tmp/laio.yaml"})
            )])),
            workspace_info_response(json!({})),
        ],
    );

    let herdr =
        Herdr::new_with_runner(empty_runner(), HerdrClient::new_with_runner(empty_runner()));
    let variables = herdr.get_session_variables(session.name())?;

    let _ = join_thread(handle)?;
    restore_env("HERDR_SOCKET_PATH", previous_socket);
    assert_eq!(variables, None);
    Ok(())
}

fn session_snapshot() -> SessionSnapshot {
    SessionSnapshot {
        focused_workspace_id: Some("w1".to_string()),
        focused_tab_id: Some("t1".to_string()),
        workspaces: vec![workspace_info()],
        tabs: vec![TabInfo {
            tab_id: "t1".to_string(),
            workspace_id: "w1".to_string(),
            label: "main".to_string(),
            focused: true,
        }],
        panes: vec![pane_info()],
        layouts: vec![PaneLayoutSnapshot {
            tab_id: "t1".to_string(),
            zoomed: false,
            focused_pane_id: "p1".to_string(),
            panes: Vec::new(),
            splits: Vec::new(),
        }],
    }
}

fn export_layout(
    tab_id: &str,
    zoomed: bool,
    focused_pane_id: &str,
    root: LayoutNode,
) -> LayoutDescription {
    LayoutDescription {
        workspace_id: "w1".to_string(),
        tab_id: tab_id.to_string(),
        zoomed,
        focused_pane_id: focused_pane_id.to_string(),
        root,
    }
}

fn export_pane(
    pane_id: &str,
    label: Option<&str>,
    cwd: &str,
    command: Option<Vec<&str>>,
) -> LayoutNode {
    LayoutNode::Pane {
        pane: LayoutPane {
            pane_id: Some(pane_id.to_string()),
            label: label.map(str::to_string),
            cwd: Some(cwd.to_string()),
            command: command.map(|argv| argv.into_iter().map(str::to_string).collect()),
            env: HashMap::new(),
        },
    }
}

fn export_pane_info(workspace_id: &str, pane_id: &str, tab_id: &str, cwd: &str) -> PaneInfo {
    PaneInfo {
        pane_id: pane_id.to_string(),
        workspace_id: workspace_id.to_string(),
        tab_id: tab_id.to_string(),
        focused: false,
        cwd: Some(cwd.to_string()),
        label: None,
    }
}

#[test]
fn get_session_requires_herdr_env() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let result = herdr.get_session();
    restore_env("HERDR_ENV", previous);

    let error = result
        .err()
        .ok_or_else(|| miette!("missing HERDR_ENV should return Err"))?;

    assert_eq!(
        error.to_string(),
        "You do not seem to be inside a herdr session."
    );
    Ok(())
}

#[test]
fn get_session_requires_herdr_workspace_id() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = clear_herdr_workspace_id();

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let result = herdr.get_session();
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);

    let error = result
        .err()
        .ok_or_else(|| miette!("missing HERDR_WORKSPACE_ID should return Err"))?;

    assert!(
        error.to_string().contains("HERDR_WORKSPACE_ID"),
        "report should name the variable: {error}"
    );
    Ok(())
}

#[test]
fn get_session_bails_when_the_workspace_is_absent() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w-missing");

    let mut api = MockHerdrApiMock::new();
    api.expect_session_snapshot()
        .times(1)
        .returning(|_| Ok(session_snapshot()));

    let herdr = Herdr::new_with_runner(empty_runner(), api);
    let result = herdr.get_session();
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);

    let error = result
        .err()
        .ok_or_else(|| miette!("unknown workspace should return Err"))?;

    assert!(
        error.to_string().contains("w-missing"),
        "report should name the workspace: {error}"
    );
    Ok(())
}

#[test]
fn get_session_exports_tabs_panes_focus_and_zoom() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w1");
    let previous_socket = set_herdr_socket_path("/tmp/herdr/sessions/work/herdr.sock");

    let snapshot = SessionSnapshot {
        focused_workspace_id: Some("w1".to_string()),
        focused_tab_id: Some("t1".to_string()),
        workspaces: vec![
            workspace_info(),
            workspace_info_with("w2", "other", HashMap::new()),
        ],
        tabs: vec![
            TabInfo {
                tab_id: "t1".to_string(),
                workspace_id: "w1".to_string(),
                label: "editor".to_string(),
                focused: true,
            },
            TabInfo {
                tab_id: "t2".to_string(),
                workspace_id: "w1".to_string(),
                label: "logs".to_string(),
                focused: false,
            },
            TabInfo {
                tab_id: "t3".to_string(),
                workspace_id: "w2".to_string(),
                label: "foreign".to_string(),
                focused: false,
            },
        ],
        panes: vec![
            export_pane_info("w1", "p1", "t1", "/tmp/project/a"),
            export_pane_info("w1", "p2", "t1", "/tmp/project/b"),
            export_pane_info("w1", "p3", "t2", "/tmp/project/c"),
            export_pane_info("w2", "p4", "t3", "/tmp/elsewhere"),
        ],
        layouts: vec![
            PaneLayoutSnapshot {
                tab_id: "t1".to_string(),
                zoomed: true,
                focused_pane_id: "p1".to_string(),
                panes: Vec::new(),
                splits: Vec::new(),
            },
            PaneLayoutSnapshot {
                tab_id: "t2".to_string(),
                zoomed: false,
                focused_pane_id: "p3".to_string(),
                panes: Vec::new(),
                splits: Vec::new(),
            },
            PaneLayoutSnapshot {
                tab_id: "t3".to_string(),
                zoomed: false,
                focused_pane_id: "p4".to_string(),
                panes: Vec::new(),
                splits: Vec::new(),
            },
        ],
    };

    let editor = export_layout(
        "t1",
        false,
        "p1",
        LayoutNode::Split {
            direction: SplitDirection::Right,
            ratio: 0.5,
            first: Box::new(export_pane(
                "p1",
                Some("main"),
                "/tmp/project/a",
                Some(vec!["echo", "hi"]),
            )),
            second: Box::new(export_pane("p2", None, "/tmp/project/b", None)),
        },
    );
    let logs = export_layout(
        "t2",
        false,
        "p3",
        export_pane("p3", None, "/tmp/project/c", None),
    );

    let mut api = MockHerdrApiMock::new();
    api.expect_session_snapshot()
        .withf(|session| session == "work")
        .times(1)
        .returning(move |_| Ok(snapshot.clone()));
    api.expect_layout_export()
        .withf(|_, tab_id| tab_id == "t1")
        .times(1)
        .returning(move |_, _| Ok(editor.clone()));
    api.expect_layout_export()
        .withf(|_, tab_id| tab_id == "t2")
        .times(1)
        .returning(move |_, _| Ok(logs.clone()));

    let herdr = Herdr::new_with_runner(empty_runner(), api);
    let result = herdr.get_session();
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    let session = result?;

    assert_eq!(session.name, "main");
    assert_eq!(session.path, "/tmp/project");
    assert_eq!(session.windows.len(), 2);

    let editor = &session.windows[0];
    assert_eq!(editor.name, "editor");
    assert_eq!(editor.flex_direction, FlexDirection::Row);
    assert_eq!(editor.panes.len(), 2);
    assert_eq!(editor.panes[0].flex, 1);
    assert_eq!(editor.panes[0].name.as_deref(), Some("main"));
    assert_eq!(editor.panes[0].path, "a");
    assert_eq!(editor.panes[0].commands.len(), 1);
    assert_eq!(editor.panes[0].commands[0].to_string(), "echo hi");
    assert!(editor.panes[0].zoom);
    assert!(editor.panes[0].focus);
    assert_eq!(editor.panes[1].flex, 1);
    assert_eq!(editor.panes[1].path, "b");
    assert!(!editor.panes[1].zoom);
    assert!(!editor.panes[1].focus);

    let logs = &session.windows[1];
    assert_eq!(logs.name, "logs");
    assert_eq!(logs.panes.len(), 1);
    assert_eq!(logs.panes[0].path, "c");
    assert!(!logs.panes[0].zoom);
    assert!(!logs.panes[0].focus);
    Ok(())
}

#[test]
fn window_from_layout_converts_split_ratios_to_flex_weights() -> Result<()> {
    let state = TabState {
        zoomed: false,
        focused_pane_id: "p1".to_string(),
        focused: false,
    };

    let half = window_from_layout(
        &export_layout(
            "t1",
            false,
            "p1",
            LayoutNode::Split {
                direction: SplitDirection::Right,
                ratio: 0.5,
                first: Box::new(export_pane("p1", None, "/tmp/project/a", None)),
                second: Box::new(export_pane("p2", None, "/tmp/project/b", None)),
            },
        ),
        "main",
        "/tmp/project",
        &state,
    );
    assert_eq!(half.panes[0].flex, 1);
    assert_eq!(half.panes[1].flex, 1);

    let quarter = window_from_layout(
        &export_layout(
            "t1",
            false,
            "p1",
            LayoutNode::Split {
                direction: SplitDirection::Down,
                ratio: 0.25,
                first: Box::new(export_pane("p1", None, "/tmp/project/a", None)),
                second: Box::new(export_pane("p2", None, "/tmp/project/b", None)),
            },
        ),
        "main",
        "/tmp/project",
        &state,
    );
    assert_eq!(quarter.flex_direction, FlexDirection::Column);
    assert_eq!(quarter.panes[0].flex, 1);
    assert_eq!(quarter.panes[1].flex, 3);
    Ok(())
}

#[test]
fn window_from_layout_falls_back_to_dot_for_the_session_path() -> Result<()> {
    let state = TabState {
        zoomed: false,
        focused_pane_id: "p1".to_string(),
        focused: false,
    };

    let window = window_from_layout(
        &export_layout(
            "t1",
            false,
            "p1",
            export_pane("p1", None, "/tmp/project", None),
        ),
        "main",
        "/tmp/project",
        &state,
    );

    assert_eq!(window.panes.len(), 1);
    assert_eq!(window.panes[0].path, ".");
    Ok(())
}

#[test]
fn session_snapshot_deserializes_the_wire_shape() -> Result<()> {
    let snapshot: SessionSnapshot = serde_json::from_value(json!({
        "version": "0.9.5",
        "protocol": 22,
        "focused_workspace_id": "w1",
        "focused_tab_id": "t1",
        "workspaces": [{"workspace_id": "w1", "label": "main", "focused": true}],
        "tabs": [{"tab_id": "t1", "workspace_id": "w1", "label": "editor", "focused": true}],
        "panes": [{
            "pane_id": "p1",
            "workspace_id": "w1",
            "tab_id": "t1",
            "focused": true,
            "cwd": "/tmp/project"
        }],
        "layouts": [{
            "workspace_id": "w1",
            "tab_id": "t1",
            "zoomed": true,
            "area": {"x": 0, "y": 0, "width": 80, "height": 24},
            "focused_pane_id": "p1",
            "panes": [{
                "pane_id": "p1",
                "focused": true,
                "rect": {"x": 0, "y": 0, "width": 40, "height": 24}
            }],
            "splits": [{
                "id": "s1",
                "direction": "right",
                "ratio": 0.5,
                "rect": {"x": 0, "y": 0, "width": 80, "height": 24}
            }]
        }]
    }))
    .into_diagnostic()?;

    assert_eq!(snapshot.focused_workspace_id.as_deref(), Some("w1"));
    assert_eq!(snapshot.focused_tab_id.as_deref(), Some("t1"));
    assert_eq!(snapshot.workspaces.len(), 1);
    assert_eq!(snapshot.tabs.len(), 1);
    assert_eq!(snapshot.panes.len(), 1);

    let layout = snapshot
        .layouts
        .first()
        .ok_or_else(|| miette!("snapshot should carry a layout"))?;
    assert_eq!(layout.tab_id, "t1");
    assert!(layout.zoomed);
    assert_eq!(layout.focused_pane_id, "p1");

    let pane = layout
        .panes
        .first()
        .ok_or_else(|| miette!("layout should carry a pane"))?;
    assert_eq!(pane.pane_id, "p1");
    assert!(pane.focused);
    assert_eq!(pane.rect.x, 0);
    assert_eq!(pane.rect.y, 0);
    assert_eq!(pane.rect.width, 40);
    assert_eq!(pane.rect.height, 24);

    let split = layout
        .splits
        .first()
        .ok_or_else(|| miette!("layout should carry a split"))?;
    assert_eq!(split.id, "s1");
    assert_eq!(split.direction, SplitDirection::Right);
    assert_eq!(split.ratio, 0.5);
    assert_eq!(split.rect.width, 80);
    Ok(())
}

fn start_session(windows: Vec<Window>) -> Session {
    let mut session = test_session(windows);
    session.name = "work".to_string();
    session.path = env!("CARGO_MANIFEST_DIR").to_string();
    session
}

fn start_env_vars() -> Vec<(&'static str, &'static str)> {
    vec![("LAIO_CONFIG", "/tmp/laio.yaml"), ("LAIO_VARS", "a=b")]
}

fn layout_with_pane(tab_id: &str, pane_id: &str) -> LayoutDescription {
    LayoutDescription {
        workspace_id: "w1".to_string(),
        tab_id: tab_id.to_string(),
        zoomed: false,
        focused_pane_id: pane_id.to_string(),
        root: pane_node(pane_id),
    }
}

fn clear_herdr_env() -> Option<OsString> {
    let previous = env::var_os("HERDR_ENV");

    unsafe {
        env::remove_var("HERDR_ENV");
    }

    previous
}

fn set_herdr_env(value: &str) -> Option<OsString> {
    let previous = env::var_os("HERDR_ENV");

    unsafe {
        env::set_var("HERDR_ENV", value);
    }

    previous
}

#[test]
fn start_switches_to_running_laio_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", laio_tokens())]));
    api.expect_workspace_focus()
        .times(1)
        .returning(|_, _| Ok(workspace_info_with("w1", "work", laio_tokens())));

    let mut cmd_unit = MockCmdUnitMock::new();
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| {
            matches!(cmd, Type::Interactive(_) if cmd.to_string() == "herdr session attach laio")
        })
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.start(&start_session(Vec::new()), &start_env_vars(), false, false);
    restore_env("HERDR_ENV", previous);

    result
}

#[test]
fn start_rejects_running_session_that_is_not_laio_owned() -> Result<()> {
    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(2)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", HashMap::new())]));

    let herdr = Herdr::new_with_runner(empty_runner(), api);

    let error = herdr
        .start(&start_session(Vec::new()), &start_env_vars(), false, false)
        .err()
        .ok_or_else(|| miette!("non-laio session should return Err"))?;

    assert!(
        error.to_string().contains("work"),
        "report should name the session: {error}"
    );
    Ok(())
}

#[test]
fn start_creates_workspace_applies_windows_and_writes_metadata() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let mut first_pane = test_pane(1, Some("main"), ".");
    first_pane.zoom = true;
    first_pane.focus = true;
    first_pane.commands = vec![Command::from_string("echo hi")];

    let mut second_pane = test_pane(1, None, ".");
    second_pane.commands = vec![Command::from_string("tail -f log")];

    let mut session = start_session(vec![
        test_window("editor", FlexDirection::Row, vec![first_pane]),
        test_window("logs", FlexDirection::Row, vec![second_pane]),
    ]);
    session.windows[0].focus = true;
    session.startup = vec![Command::from_string("date")];
    session.startup_script =
        Some(noyalib::compat::serde_yaml::from_str("echo hi").into_diagnostic()?);
    session.env = HashMap::from([("FOO".to_string(), "bar".to_string())]);

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| false);
    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(|_| Ok(empty_session_list()));
    api.expect_ensure_server()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(()));
    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string() == "date"))
        .returning(|_| Ok(String::new()));
    cmd_string
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string().contains("laio-")))
        .returning(|_| Ok(String::new()));
    api.expect_workspace_create()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, params| {
            session == LAIO_SESSION_NAME
                && params.cwd.as_deref() == Some(env!("CARGO_MANIFEST_DIR"))
                && params.label.as_deref() == Some("work")
                && params.focus
                && params.env.get("FOO").map(String::as_str) == Some("bar")
        })
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_report_metadata()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| {
            params.workspace_id == "w1"
                && params.source == "laio"
                && params.tokens.get("LAIO_CONFIG").map(String::as_str) == Some("/tmp/laio.yaml")
                && params.tokens.get("LAIO_VARS").map(String::as_str) == Some("a=b")
        })
        .returning(|_, _| Ok(()));
    api.expect_layout_apply()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| {
            params.tab_label.as_deref() == Some("editor")
                && params.focus
                && params.tab_id.as_deref() == Some("t1")
                && params.workspace_id.is_none()
        })
        .returning(|_, _| Ok(layout_with_pane("t1", "p1")));
    api.expect_pane_zoom()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, pane_id, mode| pane_id == "p1" && *mode == PaneZoomMode::On)
        .returning(|_, _, _| Ok(pane_zoom_result()));
    api.expect_pane_focus()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, pane_id| pane_id == "p1")
        .returning(|_, _| Ok(pane_info()));
    api.expect_pane_send_input()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| {
            params.pane_id == "p1"
                && params.text == "echo hi"
                && params.keys == vec!["Enter".to_string()]
        })
        .returning(|_, _| Ok(()));
    api.expect_layout_apply()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| {
            params.tab_label.as_deref() == Some("logs")
                && !params.focus
                && params.workspace_id.as_deref() == Some("w1")
                && params.tab_id.is_none()
        })
        .returning(|_, _| Ok(layout_with_pane("t2", "p2")));
    api.expect_pane_send_input()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| params.pane_id == "p2" && params.text == "tail -f log")
        .returning(|_, _| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| {
            matches!(cmd, Type::Interactive(_) if cmd.to_string() == "herdr session attach laio")
        })
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.start(&session, &start_env_vars(), false, false);
    restore_env("HERDR_ENV", previous);

    result
}

#[test]
fn start_applies_first_window_as_replace_and_the_rest_as_append() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let mut session = start_session(vec![
        test_window("one", FlexDirection::Row, vec![test_pane(1, None, ".")]),
        test_window("two", FlexDirection::Row, vec![test_pane(1, None, ".")]),
        test_window("three", FlexDirection::Row, vec![test_pane(1, None, ".")]),
    ]);
    session.windows[1].focus = true;

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| false);
    api.expect_ensure_server().times(1).returning(|_| Ok(()));
    api.expect_workspace_create()
        .times(1)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_layout_apply()
        .times(1)
        .withf(|_, params| {
            params.tab_label.as_deref() == Some("one")
                && params.tab_id.as_deref() == Some("t1")
                && params.workspace_id.is_none()
                && !params.focus
        })
        .returning(|_, _| Ok(layout_with_pane("t1", "p1")));
    api.expect_layout_apply()
        .times(1)
        .withf(|_, params| {
            params.tab_label.as_deref() == Some("two")
                && params.workspace_id.as_deref() == Some("w1")
                && params.tab_id.is_none()
                && params.focus
        })
        .returning(|_, _| Ok(layout_with_pane("t2", "p2")));
    api.expect_layout_apply()
        .times(1)
        .withf(|_, params| {
            params.tab_label.as_deref() == Some("three")
                && params.workspace_id.as_deref() == Some("w1")
                && params.tab_id.is_none()
                && !params.focus
        })
        .returning(|_, _| Ok(layout_with_pane("t3", "p3")));
    api.expect_workspace_report_metadata()
        .times(1)
        .returning(|_, _| Ok(()));

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let result = herdr.start(&session, &start_env_vars(), true, true);
    restore_env("HERDR_ENV", previous);

    result
}

#[test]
fn start_writes_metadata_before_applying_windows() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let mut session = start_session(vec![
        test_window("one", FlexDirection::Row, vec![test_pane(1, None, ".")]),
        test_window("two", FlexDirection::Row, vec![test_pane(1, None, ".")]),
    ]);
    session.windows[0].focus = true;

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| false);
    api.expect_ensure_server()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(()));
    api.expect_workspace_create()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_report_metadata()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| {
            params.workspace_id == "w1"
                && params.tokens.get("LAIO_CONFIG").map(String::as_str) == Some("/tmp/laio.yaml")
                && params.tokens.get("LAIO_VARS").map(String::as_str) == Some("a=b")
        })
        .returning(|_, _| Ok(()));
    api.expect_layout_apply()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| params.tab_label.as_deref() == Some("one"))
        .returning(|_, _| Ok(layout_with_pane("t1", "p1")));
    api.expect_layout_apply()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| params.tab_label.as_deref() == Some("two"))
        .returning(|_, _| Ok(layout_with_pane("t2", "p2")));

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let result = herdr.start(&session, &start_env_vars(), true, true);
    restore_env("HERDR_ENV", previous);

    result
}

#[test]
fn start_writes_metadata_before_a_failing_layout_apply() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let mut session = start_session(vec![
        test_window("one", FlexDirection::Row, vec![test_pane(1, None, ".")]),
        test_window("two", FlexDirection::Row, vec![test_pane(1, None, ".")]),
    ]);
    session.windows[0].focus = true;

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| false);
    api.expect_ensure_server()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(()));
    api.expect_workspace_create()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_report_metadata()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_, _| Ok(()));
    api.expect_layout_apply()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| params.tab_label.as_deref() == Some("one"))
        .returning(|_, _| Ok(layout_with_pane("t1", "p1")));
    api.expect_layout_apply()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| params.tab_label.as_deref() == Some("two"))
        .returning(|_, _| Err(miette!("layout apply failed")));

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let result = herdr.start(&session, &start_env_vars(), true, true);
    restore_env("HERDR_ENV", previous);

    assert!(result.is_err(), "a failing layout apply should abort start");
    Ok(())
}

#[test]
fn start_skips_startup_hooks_when_requested() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let mut session = start_session(Vec::new());
    session.startup = vec![Command::from_string("date")];

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| false);
    api.expect_ensure_server().times(1).returning(|_| Ok(()));
    api.expect_workspace_create()
        .times(1)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_report_metadata()
        .times(1)
        .returning(|_, _| Ok(()));

    let mut cmd_unit = MockCmdUnitMock::new();
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| {
            matches!(cmd, Type::Interactive(_) if cmd.to_string() == "herdr session attach laio")
        })
        .returning(|_| Ok(()));

    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(|_| Ok(empty_session_list()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.start(&session, &start_env_vars(), false, true);
    restore_env("HERDR_ENV", previous);

    result
}

#[test]
fn start_skips_attach_when_herdr_env_is_set() -> Result<()> {
    let _guard = env_lock()?;
    let previous = set_herdr_env("1");

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| false);
    api.expect_ensure_server().times(1).returning(|_| Ok(()));
    api.expect_workspace_create()
        .times(1)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_report_metadata()
        .times(1)
        .returning(|_, _| Ok(()));

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let result = herdr.start(&start_session(Vec::new()), &start_env_vars(), false, true);
    restore_env("HERDR_ENV", previous);

    result
}

#[test]
fn start_skips_attach_when_requested() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| false);
    api.expect_ensure_server().times(1).returning(|_| Ok(()));
    api.expect_workspace_create()
        .times(1)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_report_metadata()
        .times(1)
        .returning(|_, _| Ok(()));

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let result = herdr.start(&start_session(Vec::new()), &start_env_vars(), true, true);
    restore_env("HERDR_ENV", previous);

    result
}

#[test]
fn start_sleeps_pane_cmd_delay_before_first_send_input() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let mut pane = test_pane(1, None, ".");
    pane.commands = vec![Command::from_string("echo hi")];

    let mut session = start_session(vec![test_window("main", FlexDirection::Row, vec![pane])]);
    session.pane_cmd_delay = Some(50);

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| false);
    api.expect_ensure_server().times(1).returning(|_| Ok(()));
    api.expect_workspace_create()
        .times(1)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_layout_apply()
        .times(1)
        .returning(|_, _| Ok(layout_with_pane("t1", "p1")));
    api.expect_pane_send_input()
        .times(1)
        .returning(|_, _| Ok(()));
    api.expect_workspace_report_metadata()
        .times(1)
        .returning(|_, _| Ok(()));

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let started = Instant::now();
    let result = herdr.start(&session, &start_env_vars(), true, true);
    let elapsed = started.elapsed();
    restore_env("HERDR_ENV", previous);

    result?;
    assert!(
        elapsed >= Duration::from_millis(50),
        "start should wait for pane_cmd_delay, elapsed {elapsed:?}"
    );
    Ok(())
}

#[test]
fn start_rejects_invalid_session_names() -> Result<()> {
    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let mut session = start_session(Vec::new());
    session.name = "../escape".to_string();

    let error = herdr
        .start(&session, &start_env_vars(), false, false)
        .err()
        .ok_or_else(|| miette!("invalid session name should return Err"))?;

    assert!(
        error.to_string().contains("Invalid"),
        "report should name the invalid session: {error}"
    );
    Ok(())
}

#[test]
fn switch_focuses_owned_workspace_and_attaches() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", laio_tokens())]));
    api.expect_workspace_focus()
        .times(1)
        .withf(|session, workspace_id| session == LAIO_SESSION_NAME && workspace_id == "w1")
        .returning(|_, _| Ok(workspace_info_with("w1", "work", laio_tokens())));

    let mut cmd_unit = MockCmdUnitMock::new();
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| {
            matches!(cmd, Type::Interactive(_) if cmd.to_string() == "herdr session attach laio")
        })
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.switch("work", false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    assert!(result?);
    Ok(())
}

#[test]
fn switch_skips_attach_when_requested() -> Result<()> {
    let _guard = env_lock()?;
    let previous_socket = clear_herdr_socket_path();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", laio_tokens())]));
    api.expect_workspace_focus()
        .times(1)
        .returning(|_, _| Ok(workspace_info_with("w1", "work", laio_tokens())));

    let herdr = Herdr::new_with_runner(empty_runner(), api);

    let result = herdr.switch("work", true);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    assert!(result?);
    Ok(())
}

#[test]
fn switch_skips_attach_when_herdr_env_is_set() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_socket = clear_herdr_socket_path();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", laio_tokens())]));
    api.expect_workspace_focus()
        .times(1)
        .returning(|_, _| Ok(workspace_info_with("w1", "work", laio_tokens())));

    let herdr = Herdr::new_with_runner(empty_runner(), api);

    let result = herdr.switch("work", false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    assert!(result?);
    Ok(())
}

#[test]
fn switch_rejects_workspace_label_without_laio_config() -> Result<()> {
    let _guard = env_lock()?;
    let previous_socket = clear_herdr_socket_path();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(2)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", HashMap::new())]));

    let herdr = Herdr::new_with_runner(empty_runner(), api);

    let result = herdr.switch("work", false);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    let error = result
        .err()
        .ok_or_else(|| miette!("non-laio workspace should return Err"))?;

    assert!(
        error.to_string().contains("work"),
        "report should name the workspace: {error}"
    );
    Ok(())
}

#[test]
fn switch_returns_false_when_no_workspace_matches() -> Result<()> {
    let _guard = env_lock()?;
    let previous_socket = clear_herdr_socket_path();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(2)
        .returning(|_| Ok(vec![workspace_info_with("w1", "other", laio_tokens())]));

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let result = herdr.switch("work", false);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    assert!(!result?);
    Ok(())
}

#[test]
fn switch_returns_false_when_the_server_is_not_running() -> Result<()> {
    let _guard = env_lock()?;
    let previous_socket = clear_herdr_socket_path();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| false);

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let result = herdr.switch("work", false);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    assert!(!result?);
    Ok(())
}

fn two_session_list() -> String {
    json!({
        "sessions": [
            {"name": "laio", "default": false, "running": true},
            {"name": "work", "default": false, "running": true}
        ]
    })
    .to_string()
}

fn empty_session_list() -> String {
    json!({"sessions": []}).to_string()
}

fn runner_with_session_list(sessions: String) -> RunnerMock {
    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));

    RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    }
}

#[test]
fn stop_closes_a_workspace_hosted_in_a_foreign_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let sessions = two_session_list();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_session_exists()
        .withf(|session| session == "laio")
        .times(1)
        .returning(|_| true);
    api.expect_workspace_list()
        .withf(|session| session == "laio")
        .times(3)
        .returning(|_| Ok(Vec::new()));
    api.expect_workspace_list()
        .withf(|session| session == "work")
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("wx", "x", laio_tokens())]));
    api.expect_workspace_close()
        .times(1)
        .withf(|session, workspace_id| session == "work" && workspace_id == "wx")
        .returning(|_, _| Ok(()));

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&Some("x".to_string()), &None, true, false, false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn switch_focuses_a_workspace_hosted_in_a_foreign_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let sessions = two_session_list();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_session_exists()
        .withf(|session| session == "laio")
        .times(1)
        .returning(|_| true);
    api.expect_workspace_list()
        .withf(|session| session == "laio")
        .times(3)
        .returning(|_| Ok(Vec::new()));
    api.expect_workspace_list()
        .withf(|session| session == "work")
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("wx", "x", laio_tokens())]));
    api.expect_workspace_focus()
        .times(1)
        .withf(|session, workspace_id| session == "work" && workspace_id == "wx")
        .returning(|_, _| Ok(workspace_info_with("wx", "x", laio_tokens())));
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| {
            matches!(cmd, Type::Interactive(_) if cmd.to_string() == "herdr session attach work")
        })
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.switch("x", false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    assert!(result?);
    Ok(())
}

#[test]
fn read_backs_resolve_a_workspace_hosted_in_a_foreign_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous_socket = clear_herdr_socket_path();

    let sessions = two_session_list();
    let tokens = HashMap::from([
        ("LAIO_CONFIG".to_string(), "/tmp/x.yaml".to_string()),
        ("LAIO_VARS".to_string(), "a=1&b=2".to_string()),
    ]);

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();

    cmd_string
        .expect_run()
        .times(2)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_session_exists()
        .withf(|session| session == "laio")
        .times(2)
        .returning(|_| true);
    api.expect_workspace_list()
        .withf(|session| session == "laio")
        .times(4)
        .returning(|_| Ok(Vec::new()));
    api.expect_workspace_list()
        .withf(|session| session == "work")
        .times(2)
        .returning({
            let tokens = tokens.clone();
            move |_| Ok(vec![workspace_info_with("wx", "x", tokens.clone())])
        });
    api.expect_workspace_get()
        .times(2)
        .withf(|session, workspace_id| session == "work" && workspace_id == "wx")
        .returning(move |_, _| Ok(workspace_info_with("wx", "x", tokens.clone())));

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let config = herdr.get_session_config_path("x");
    let variables = herdr.get_session_variables("x");
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    assert_eq!(config?, Some("/tmp/x.yaml".to_string()));
    assert_eq!(variables?, Some(vec!["a=1".to_string(), "b=2".to_string()]));
    Ok(())
}

#[test]
fn switch_rejects_invalid_session_names() -> Result<()> {
    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());

    let error = herdr
        .switch("bad/name", false)
        .err()
        .ok_or_else(|| miette!("invalid session name should return Err"))?;

    assert!(
        error.to_string().contains("Invalid"),
        "report should name the invalid session: {error}"
    );
    Ok(())
}

#[test]
fn list_sessions_maps_owned_workspaces_to_active() -> Result<()> {
    let _guard = env_lock()?;
    let previous_workspace = clear_herdr_workspace_id();

    let output = json!({
        "sessions": [
            {"name": "default", "default": true, "running": true},
            {"name": "laio", "default": false, "running": true},
            {"name": "stopped", "default": false, "running": false}
        ]
    })
    .to_string();

    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(output.clone()));

    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list()
        .withf(|session| session == "laio")
        .times(1)
        .returning(|_| {
            Ok(vec![
                workspace_info_with("w1", "alpha", laio_tokens()),
                workspace_info_with("w2", "beta", HashMap::new()),
                workspace_info_with("w3", "gamma", laio_tokens()),
            ])
        });

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let sessions = herdr.list_sessions();
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);

    let sessions = sessions?;

    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].name, "alpha");
    assert_eq!(sessions[0].status, SessionStatus::Active);
    assert_eq!(sessions[1].name, "gamma");
    assert_eq!(sessions[1].status, SessionStatus::Active);
    Ok(())
}

#[test]
fn list_sessions_skips_duplicate_workspaces() -> Result<()> {
    let _guard = env_lock()?;
    let previous_workspace = clear_herdr_workspace_id();

    let output = json!({
        "sessions": [
            {"name": "laio", "default": false, "running": true},
            {"name": "other", "default": false, "running": true}
        ]
    })
    .to_string();

    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(output.clone()));

    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list()
        .withf(|session| session == "laio")
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "alpha", laio_tokens())]));
    api.expect_workspace_list()
        .withf(|session| session == "other")
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "alpha", laio_tokens())]));

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let sessions = herdr.list_sessions();
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);

    let sessions = sessions?;

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "alpha");
    Ok(())
}

fn list_sessions_inventory() -> Vec<WorkspaceInfo> {
    vec![
        workspace_info_with("w1", "alpha", laio_tokens()),
        workspace_info_with("w2", "beta", laio_tokens()),
        workspace_info_with("w3", "gamma", HashMap::new()),
    ]
}

fn list_sessions_with_workspace_env(workspace_id: Option<&str>) -> Result<Vec<SessionInfo>> {
    let output =
        json!({"sessions": [{"name": "laio", "default": false, "running": true}]}).to_string();

    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(output.clone()));

    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list()
        .withf(|session| session == "laio")
        .times(1)
        .returning(|_| Ok(list_sessions_inventory()));

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let previous = match workspace_id {
        Some(workspace_id) => set_herdr_workspace_id(workspace_id),
        None => clear_herdr_workspace_id(),
    };
    let sessions = herdr.list_sessions();
    restore_env("HERDR_WORKSPACE_ID", previous);

    sessions
}

#[test]
fn list_sessions_marks_the_current_workspace_attached() -> Result<()> {
    let _guard = env_lock()?;

    let sessions = list_sessions_with_workspace_env(Some("w2"))?;

    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].name, "alpha");
    assert_eq!(sessions[0].status, SessionStatus::Active);
    assert_eq!(sessions[1].name, "beta");
    assert_eq!(sessions[1].status, SessionStatus::Attached);
    Ok(())
}

#[test]
fn list_sessions_marks_every_row_active_outside_herdr() -> Result<()> {
    let _guard = env_lock()?;

    let sessions = list_sessions_with_workspace_env(None)?;

    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].status, SessionStatus::Active);
    assert_eq!(sessions[1].status, SessionStatus::Active);
    Ok(())
}

#[test]
fn list_sessions_marks_every_row_active_when_the_workspace_is_absent() -> Result<()> {
    let _guard = env_lock()?;

    let sessions = list_sessions_with_workspace_env(Some("w9"))?;

    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].status, SessionStatus::Active);
    assert_eq!(sessions[1].status, SessionStatus::Active);
    Ok(())
}

#[test]
fn target_session_uses_the_socket_path_session_name() -> Result<()> {
    let _guard = env_lock()?;
    let previous = set_herdr_socket_path("/tmp/herdr/sessions/work/herdr.sock");

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let target = herdr.target_session();
    restore_env("HERDR_SOCKET_PATH", previous);

    assert_eq!(target, "work");
    Ok(())
}

#[test]
fn target_session_falls_back_to_the_laio_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_socket_path();

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let target = herdr.target_session();
    restore_env("HERDR_SOCKET_PATH", previous);

    assert_eq!(target, LAIO_SESSION_NAME);
    Ok(())
}

#[test]
fn target_session_falls_back_when_the_socket_path_does_not_match() -> Result<()> {
    let _guard = env_lock()?;
    let previous = set_herdr_socket_path("/tmp/herdr/herdr.sock");

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let target = herdr.target_session();
    restore_env("HERDR_SOCKET_PATH", previous);

    assert_eq!(target, LAIO_SESSION_NAME);
    Ok(())
}

#[test]
fn current_workspace_id_reads_the_env_var() -> Result<()> {
    let _guard = env_lock()?;
    let previous = set_herdr_workspace_id("w3");

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let workspace_id = herdr.current_workspace_id();
    restore_env("HERDR_WORKSPACE_ID", previous);

    assert_eq!(workspace_id.as_deref(), Some("w3"));
    Ok(())
}

#[test]
fn current_workspace_id_ignores_empty_and_unset_values() -> Result<()> {
    let _guard = env_lock()?;
    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());

    let previous = set_herdr_workspace_id("");
    let empty = herdr.current_workspace_id();
    restore_env("HERDR_WORKSPACE_ID", previous);

    let previous = clear_herdr_workspace_id();
    let unset = herdr.current_workspace_id();
    restore_env("HERDR_WORKSPACE_ID", previous);

    assert_eq!(empty, None);
    assert_eq!(unset, None);
    Ok(())
}

#[test]
fn owned_workspaces_keeps_only_token_carrying_workspaces() -> Result<()> {
    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list().times(1).returning(|_| {
        Ok(vec![
            workspace_info_with("w1", "alpha", laio_tokens()),
            workspace_info_with("w2", "beta", HashMap::new()),
            workspace_info_with("w3", "gamma", laio_tokens()),
        ])
    });

    let herdr = Herdr::new_with_runner(empty_runner(), api);
    let owned = herdr.owned_workspaces(LAIO_SESSION_NAME)?;

    assert_eq!(owned.len(), 2);
    assert_eq!(owned[0].label, "alpha");
    assert_eq!(owned[1].label, "gamma");
    Ok(())
}

#[test]
fn find_owned_workspace_matches_the_label() -> Result<()> {
    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list().times(1).returning(|_| {
        Ok(vec![
            workspace_info_with("w1", "alpha", laio_tokens()),
            workspace_info_with("w2", "beta", laio_tokens()),
        ])
    });

    let herdr = Herdr::new_with_runner(empty_runner(), api);
    let found = herdr.find_owned_workspace(LAIO_SESSION_NAME, "beta")?;

    let workspace = found.ok_or_else(|| miette!("beta should be found"))?;
    assert_eq!(workspace.workspace_id, "w2");
    Ok(())
}

#[test]
fn find_owned_workspace_ignores_workspaces_without_tokens() -> Result<()> {
    let mut api = MockHerdrApiMock::new();
    api.expect_workspace_list()
        .times(1)
        .returning(|_| Ok(vec![workspace_info_with("w1", "beta", HashMap::new())]));

    let herdr = Herdr::new_with_runner(empty_runner(), api);

    assert!(
        herdr
            .find_owned_workspace(LAIO_SESSION_NAME, "beta")?
            .is_none()
    );
    Ok(())
}

#[test]
fn unimplemented_multiplexer_methods_return_err() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());

    let result = herdr.get_session();
    restore_env("HERDR_ENV", previous);

    assert!(result.is_err());
    Ok(())
}

#[test]
fn herdr_new_rejects_invalid_session_names() -> Result<()> {
    let herdr = Herdr::new();

    let error = herdr
        .switch("", true)
        .err()
        .ok_or_else(|| miette!("empty session name should return Err"))?;

    assert!(
        error.to_string().contains("Invalid"),
        "report should name the invalid session: {error}"
    );
    Ok(())
}
