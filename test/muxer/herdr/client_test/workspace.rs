use std::collections::HashMap;

use miette::{Result, miette};
use serde_json::json;

use super::client::HerdrApi;
use super::model::WorkspaceCreateParams;

use super::support::*;

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
