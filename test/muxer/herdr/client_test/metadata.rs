use std::collections::HashMap;

use miette::{Result, miette};
use serde_json::json;

use super::client::HerdrApi;
use super::model::WorkspaceReportMetadataParams;

use super::support::*;

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
