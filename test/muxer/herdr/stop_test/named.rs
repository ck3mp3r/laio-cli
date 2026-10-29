use std::collections::HashMap;

use miette::{Result, miette};
use mockall::Sequence;

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::muxer::Multiplexer;

use super::mux::Herdr;

use super::support_api_mock::*;
use super::support_env::*;
use super::support_models::*;
use super::support_socket::*;

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
