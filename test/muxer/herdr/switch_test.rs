use std::collections::HashMap;

use miette::{Result, miette};

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::muxer::Multiplexer;

use super::mux::{Herdr, LAIO_SESSION_NAME};

use super::support::*;

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
        .withf(
            |cmd| matches!(cmd, Type::Forget(_) if cmd.to_string() == "herdr session attach laio"),
        )
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
        .withf(
            |cmd| matches!(cmd, Type::Forget(_) if cmd.to_string() == "herdr session attach work"),
        )
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
