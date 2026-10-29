use std::collections::HashMap;

use miette::{IntoDiagnostic, Result, miette};
use mockall::Sequence;

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::config::Command;
use crate::common::muxer::Multiplexer;

use super::mux::{Herdr, LAIO_SESSION_NAME};

use super::start_test::start_session;
use super::support::*;

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

#[test]
fn stop_named_host_keeps_other_workspaces_and_the_session_running() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w-host");

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
        .returning(|_| {
            Ok(vec![
                workspace_info_with("w-host", "work", laio_tokens()),
                workspace_info_with("w-other", "other", laio_tokens()),
            ])
        });
    api.expect_workspace_list()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| {
            Ok(vec![
                workspace_info_with("w-host", "work", laio_tokens()),
                workspace_info_with("w-other", "other", laio_tokens()),
            ])
        });
    api.expect_workspace_close().times(0);
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|cmd| {
            matches!(cmd, Type::Forget(_) if cmd.to_string()
                == "HERDR_SESSION=laio sh -c nohup sh -c 'herdr workspace close w-host >/dev/null 2>&1' >/dev/null 2>&1 & disown")
        })
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&Some("work".to_string()), &None, true, false, false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);

    result
}

#[test]
fn stop_named_host_now_stops_the_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w1");

    let mut api = MockHerdrApiMock::new();
    let mut cmd_unit = MockCmdUnitMock::new();

    api.expect_session_exists().times(1).returning(|_| true);
    api.expect_workspace_list()
        .times(2)
        .returning(|_| Ok(vec![workspace_info_with("w1", "work", laio_tokens())]));
    api.expect_workspace_close().times(0);
    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| {
            matches!(cmd, Type::Forget(_) if cmd.to_string()
                == "HERDR_SESSION=laio sh -c nohup sh -c 'n=0; until herdr session stop laio >/dev/null 2>&1; do n=$((n+1)); [ $n -ge 15 ] && exit 1; sleep 1; done' >/dev/null 2>&1 & disown")
        })
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.stop(&Some("work".to_string()), &None, true, false, false);
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);

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
