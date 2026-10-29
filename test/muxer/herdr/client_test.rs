use std::{collections::HashMap, env, fs, time::Duration};

use miette::{IntoDiagnostic, Result, miette};
use serde_json::{Value, json};

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use super::client::{
    HerdrApi, HerdrClient, HerdrConnection, chunk_token, chunk_tokens, config_dir_from,
    join_tokens, resolve_socket_path, split_chunk_key,
};
use super::model::{
    LayoutApplyParams, LayoutNode, LayoutPane, PaneSendInputParams, PaneZoomMode, SplitDirection,
    WorkspaceCreateParams, WorkspaceReportMetadataParams,
};

use super::support::*;

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
            matches!(cmd, Type::Forget(_) if cmd.to_string()
                == "HERDR_SESSION=w sh -c nohup herdr server >/dev/null 2>&1 </dev/null & disown")
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
