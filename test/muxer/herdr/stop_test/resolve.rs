use std::collections::HashMap;

use miette::Result;

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
