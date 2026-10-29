use miette::{IntoDiagnostic, Result, miette};
use mockall::Sequence;

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::config::Command;
use crate::common::muxer::Multiplexer;

use super::mux::{Herdr, LAIO_SESSION_NAME};

use super::start_test_support::start_session;
use super::support::*;

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
