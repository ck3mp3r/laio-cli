use std::fs;

use miette::{IntoDiagnostic, Result, miette};
use serde_json::{Value, json};

use super::client::{HerdrApi, HerdrConnection};

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
