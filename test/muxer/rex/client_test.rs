use std::sync::Arc;

use miette::{Report, miette};
use serde_json::{Value, json};

use crate::common::cmd::Type;
use crate::common::cmd::test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock};
use crate::muxer::rex::client::RexClient;
use crate::muxer::rex::model::{
    BlockNode, BlockOptions, CreateRequest, CreateWindow, LayoutNode, SplitNode,
};

type TestResult<T = ()> = core::result::Result<T, Box<dyn std::error::Error>>;

fn argv(cmd: &Type) -> Option<(String, Vec<String>)> {
    match cmd {
        Type::Basic(command) => Some((
            command.get_program().to_string_lossy().into_owned(),
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
        )),
        Type::Verbose(_) | Type::Forget(_) => None,
    }
}

fn exact_argv(cmd: &Type, expected: &[String]) -> bool {
    argv(cmd).is_some_and(|(program, args)| program == "rex" && args == expected)
}

fn api_payload(cmd: &Type, method: &str) -> Option<Value> {
    let (program, args) = argv(cmd)?;
    if program != "rex"
        || args.len() != 4
        || args[0] != "api"
        || args[1] != "call"
        || args[2] != method
    {
        return None;
    }

    serde_json::from_str(&args[3]).ok()
}

fn expected(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| arg.to_string()).collect()
}

fn runner(cmd_unit: MockCmdUnitMock, cmd_string: MockCmdStringMock) -> Arc<RunnerMock> {
    Arc::new(RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    })
}

fn string_client_with<F>(matcher: F, output: &str) -> TestResult<RexClient<RunnerMock>>
where
    F: Fn(&Type) -> bool + Send + 'static,
{
    let output = output.to_string();
    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1)
        .withf(matcher)
        .returning(move |_| Ok(output.clone()));

    Ok(RexClient::new(runner(MockCmdUnitMock::new(), cmd_string)))
}

fn string_client(args: &[&str], output: &str) -> TestResult<RexClient<RunnerMock>> {
    let expected_args = expected(args);

    string_client_with(move |cmd| exact_argv(cmd, &expected_args), output)
}

fn api_client(
    method: &'static str,
    payload: Value,
    output: &str,
) -> TestResult<RexClient<RunnerMock>> {
    string_client_with(
        move |cmd| api_payload(cmd, method).is_some_and(|actual| actual == payload),
        output,
    )
}

fn failing_client(
    args: &[&str],
    stderr: &str,
    context: &str,
) -> Result<RexClient<RunnerMock>, Report> {
    let expected_args = expected(args);
    let stderr = stderr.to_string();
    let context = context.to_string();
    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1)
        .withf(move |cmd| exact_argv(cmd, &expected_args))
        .returning(move |_| Err(miette!(stderr.clone()).wrap_err(context.clone())));

    Ok(RexClient::new(runner(MockCmdUnitMock::new(), cmd_string)))
}

fn unit_client(args: &[&str]) -> TestResult<RexClient<RunnerMock>> {
    let expected_args = expected(args);
    let mut cmd_unit = MockCmdUnitMock::new();
    cmd_unit
        .expect_run()
        .times(1)
        .withf(move |cmd| exact_argv(cmd, &expected_args))
        .returning(|_| Ok(()));

    Ok(RexClient::new(runner(cmd_unit, MockCmdStringMock::new())))
}

fn terminal(label: Option<&str>, options: Option<BlockOptions>) -> LayoutNode {
    LayoutNode {
        block: Some(BlockNode {
            flavor: "terminal".to_string(),
            label: label.map(str::to_string),
            options,
        }),
        split: None,
    }
}

fn create_request() -> CreateRequest {
    let options = BlockOptions {
        cwd: Some("/tmp".to_string()),
        ..BlockOptions::default()
    };

    CreateRequest {
        label: Some("laio: probe".to_string()),
        initial_windows: Some(vec![
            CreateWindow {
                window_label: Some("w1".to_string()),
                layout: terminal(Some("p1"), Some(options)),
            },
            CreateWindow {
                window_label: None,
                layout: LayoutNode {
                    block: None,
                    split: Some(SplitNode {
                        direction: "horizontal".to_string(),
                        ratio: Some(0.5),
                        before: Box::new(terminal(None, None)),
                        after: Box::new(terminal(None, None)),
                    }),
                },
            },
        ]),
    }
}

fn create_payload() -> Value {
    json!({
        "label": "laio: probe",
        "initial_windows": [
            {
                "window_label": "w1",
                "layout": {
                    "block": {
                        "flavor": "terminal",
                        "label": "p1",
                        "options": { "cwd": "/tmp" }
                    }
                }
            },
            {
                "layout": {
                    "split": {
                        "direction": "horizontal",
                        "ratio": 0.5,
                        "before": { "block": { "flavor": "terminal" } },
                        "after": { "block": { "flavor": "terminal" } }
                    }
                }
            }
        ]
    })
}

#[test]
fn session_create_sends_request_json_as_one_argument() -> TestResult {
    let client = api_client(
        "session.create",
        create_payload(),
        r#"{"session_id":"session:1","initial_windows":[{"window_id":"window:1","block_ids":["block:1","block:2","block:3"],"revision":7}]}"#,
    )?;

    let output = client.session_create(&create_request())?;

    assert_eq!(output.session_id, "session:1");
    let window = output
        .initial_windows
        .first()
        .ok_or("should have one window")?;
    assert_eq!(window.window_id, "window:1");
    assert_eq!(window.block_ids, ["block:1", "block:2", "block:3"]);
    Ok(())
}

#[test]
fn session_view_parses_window_without_focus_zoom_or_block_owner() -> TestResult {
    let client = api_client(
        "session.view",
        json!({"session_id": "session:1"}),
        r#"{"revision":12,"session_id":"session:1","label":"laio: probe","active_window_id":"window:1","windows":[{"window_id":"window:1","label":"Window 01a1","layers":[{"layer_id":"layer:1","kind":"tiled","bounds":[0,0,1,1],"layout":{"rect":[0,0,1,1],"block_id":"block:1"},"blocks":[{"block_id":"block:1","creator_name":"rex","flavor":"terminal","label":"shell"}]}]}]}"#,
    )?;

    let view = client.session_view("session:1")?;

    assert_eq!(view.revision, 12);
    let window = view
        .windows
        .and_then(|windows| windows.into_iter().next())
        .ok_or("should have one window")?;
    assert!(window.focused_block_id.is_none());
    assert!(window.zoomed_block_id.is_none());
    let block = window
        .layers
        .and_then(|layers| layers.into_iter().next())
        .and_then(|layer| layer.blocks)
        .and_then(|blocks| blocks.into_iter().next())
        .ok_or("should have one block")?;
    assert_eq!(block.block_id, "block:1");
    assert!(block.owner.is_none());
    Ok(())
}

#[test]
fn session_focus_window_sends_session_and_window() -> TestResult {
    let client = api_client(
        "session.focus_window",
        json!({"session_id": "session:1", "window_id": "window:1"}),
        r#"{"revision":9}"#,
    )?;

    let output = client.session_focus_window("session:1", "window:1")?;

    assert_eq!(output.revision, 9);
    Ok(())
}

#[test]
fn session_focus_block_sends_session_and_block() -> TestResult {
    let client = api_client(
        "session.focus_block",
        json!({"session_id": "session:1", "block_id": "block:1"}),
        r#"{"revision":10}"#,
    )?;

    let output = client.session_focus_block("session:1", "block:1")?;

    assert_eq!(output.revision, 10);
    Ok(())
}

#[test]
fn session_list_reads_cli_json_output() -> TestResult {
    let client = string_client(
        &["session", "ls", "--json"],
        r#"{"sessions":[{"session_id":"session:1","label":"laio: probe"},{"session_id":"session:2","label":"other"}]}"#,
    )?;

    let output = client.session_list()?;

    let labels: Vec<&str> = output
        .sessions
        .iter()
        .map(|session| session.label.as_str())
        .collect();
    assert_eq!(labels, ["laio: probe", "other"]);
    Ok(())
}

#[test]
fn client_list_exposes_session_ids() -> TestResult {
    let client = string_client(
        &["client", "ls", "--json"],
        r#"{"clients":[{"client_id":"client:app","connection_state":"connected","info":{"kind":"app","platform":"macos","name":"Rex Beta","version":"0.1.0 (1758)","label":"Mac"},"session_ids":["session:1","session:2"]},{"client_id":"client:cli","connection_state":"connected","info":{"kind":"cli","platform":"macos","name":"rex","version":"0.0.1"}}]}"#,
    )?;

    let output = client.client_list()?;

    let clients = output.clients.ok_or("should have clients")?;
    let app = clients.first().ok_or("should have the app client")?;
    let session_ids = app
        .session_ids
        .clone()
        .ok_or("app client should carry session ids")?;
    assert_eq!(session_ids, ["session:1", "session:2"]);
    let cli = clients.get(1).ok_or("should have the cli client")?;
    assert!(cli.session_ids.is_none());
    Ok(())
}

#[test]
fn block_pwd_passes_session_scope() -> TestResult {
    let client = string_client(
        &["block", "call", "-s", "session:1", "-b", "block:1", "pwd"],
        r#"{"pwd":"file://host/tmp/project","owner":{"pid":42,"name":"zsh"}}"#,
    )?;

    let output = client.block_pwd("session:1", "block:1")?;

    assert_eq!(output.pwd, "file://host/tmp/project");
    let owner = output.owner.ok_or("should have an owner")?;
    assert_eq!(owner.pid, 42);
    Ok(())
}

#[test]
fn zoom_on_passes_session_scope() -> TestResult {
    let client = unit_client(&["zoom", "-s", "session:1", "block:1", "--on"])?;

    client.zoom("session:1", "block:1", true)?;
    Ok(())
}

#[test]
fn zoom_off_passes_session_scope() -> TestResult {
    let client = unit_client(&["zoom", "-s", "session:1", "block:1", "--off"])?;

    client.zoom("session:1", "block:1", false)?;
    Ok(())
}

#[test]
fn send_commands_passes_session_scope_and_verbatim_text() -> TestResult {
    let client = unit_client(&[
        "send",
        "-s",
        "session:1",
        "-b",
        "block:1",
        "--",
        "echo one\necho two\n",
    ])?;

    client.send_commands("session:1", "block:1", "echo one\necho two\n")?;
    Ok(())
}

#[test]
fn nonzero_exit_error_contains_stderr_detail() -> TestResult {
    let client = failing_client(
        &["session", "ls", "--json"],
        "Error: stream response \"control-1\" failed: block target block:9 not found (status=404 code=not_found)",
        "Basic: rex session ls --json",
    )?;

    let error = client.session_list().err().ok_or("should fail")?;

    assert!(
        error.to_string().contains("block target block:9 not found"),
        "{error}"
    );
    Ok(())
}

#[test]
fn nonzero_exit_error_contains_server_code() -> TestResult {
    let client = failing_client(
        &["session", "ls", "--json"],
        "Error: stream response \"control-2\" failed: session not found (status=404 code=not_found)",
        "Basic: rex session ls --json",
    )?;

    let error = client.session_list().err().ok_or("should fail")?;

    assert!(error.to_string().contains("not_found"), "{error}");
    Ok(())
}

#[test]
fn invalid_json_error_names_method_and_output() -> TestResult {
    let client = string_client(&["session", "ls", "--json"], "not json at all")?;

    let error = client.session_list().err().ok_or("should fail")?;
    let message = error.to_string();

    assert!(message.contains("session_list"), "{message}");
    assert!(message.contains("not json at all"), "{message}");
    Ok(())
}
