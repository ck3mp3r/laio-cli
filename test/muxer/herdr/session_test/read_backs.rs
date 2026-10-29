use std::collections::HashMap;

use miette::Result;
use serde_json::json;

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};
use crate::common::muxer::Multiplexer;

use super::client::HerdrClient;
use super::mux::Herdr;

use super::support_api_mock::*;
use super::support_env::*;
use super::support_models::*;
use super::support_responses::*;
use super::support_socket::*;

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
