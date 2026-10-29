use std::collections::HashMap;

use miette::{Result, miette};
use mockall::Sequence;
use serde_json::json;

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::muxer::Multiplexer;

use super::mux::Herdr;

use super::start_test_support::*;
use super::support::*;

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
        .withf(
            |cmd| matches!(cmd, Type::Forget(_) if cmd.to_string() == "herdr session attach laio"),
        )
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string: session_list_cmd_string(empty_session_list()),
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

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

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
fn start_deletes_a_stopped_laio_session_before_provisioning() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let stopped = json!({"sessions": [{"name": "laio", "running": false}]}).to_string();

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
        .returning(move |_| Ok(stopped.clone()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session delete laio"),
        )
        .returning(|_| Ok(()));
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
    api.expect_workspace_create()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_report_metadata()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_, _| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.start(&start_session(Vec::new()), &start_env_vars(), true, true);
    restore_env("HERDR_ENV", previous);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn start_keeps_a_running_laio_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let running = json!({"sessions": [{"name": "laio", "running": true}]}).to_string();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(running.clone()));
    cmd_unit
        .expect_run()
        .times(0)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session delete laio"),
        )
        .returning(|_| Ok(()));
    api.expect_session_exists().times(1).returning(|_| false);
    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(|_| Ok(empty_session_list()));
    api.expect_ensure_server().times(1).returning(|_| Ok(()));
    api.expect_workspace_create()
        .times(1)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_report_metadata()
        .times(1)
        .returning(|_, _| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.start(&start_session(Vec::new()), &start_env_vars(), true, true);
    restore_env("HERDR_ENV", previous);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn start_skips_the_cleanup_when_no_laio_session_row_exists() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(|_| Ok(empty_session_list()));
    cmd_unit
        .expect_run()
        .times(0)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session delete laio"),
        )
        .returning(|_| Ok(()));
    api.expect_session_exists().times(1).returning(|_| false);
    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(|_| Ok(empty_session_list()));
    api.expect_ensure_server().times(1).returning(|_| Ok(()));
    api.expect_workspace_create()
        .times(1)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_report_metadata()
        .times(1)
        .returning(|_, _| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.start(&start_session(Vec::new()), &start_env_vars(), true, true);
    restore_env("HERDR_ENV", previous);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}

#[test]
fn start_continues_when_the_cleanup_delete_fails() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();
    let previous_socket = clear_herdr_socket_path();

    let stopped = json!({"sessions": [{"name": "laio", "running": false}]}).to_string();

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(stopped.clone()));
    cmd_unit
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session delete laio"),
        )
        .returning(|_| Err(miette!("session laio is running; stop it before deleting")));
    api.expect_session_exists().times(1).returning(|_| false);
    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(|_| Ok(empty_session_list()));
    api.expect_ensure_server().times(1).returning(|_| Ok(()));
    api.expect_workspace_create()
        .times(1)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_report_metadata()
        .times(1)
        .returning(|_, _| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.start(&start_session(Vec::new()), &start_env_vars(), true, true);
    restore_env("HERDR_ENV", previous);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    result
}
