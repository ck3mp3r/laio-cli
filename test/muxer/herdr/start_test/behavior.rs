use std::time::{Duration, Instant};

use miette::Result;

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::config::{Command, FlexDirection};
use crate::common::muxer::Multiplexer;

use super::mux::Herdr;

use super::start_test_support::*;
use super::support::*;

#[test]
fn start_skips_startup_hooks_when_requested() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let mut session = start_session(Vec::new());
    session.startup = vec![Command::from_string("date")];

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| false);
    api.expect_ensure_server().times(1).returning(|_| Ok(()));
    api.expect_workspace_create()
        .times(1)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_report_metadata()
        .times(1)
        .returning(|_, _| Ok(()));

    let mut cmd_unit = MockCmdUnitMock::new();
    cmd_unit
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Forget(_) if cmd.to_string() == "herdr session attach laio"),
        )
        .returning(|_| Ok(()));

    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(2)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(|_| Ok(empty_session_list()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.start(&session, &start_env_vars(), false, true);
    restore_env("HERDR_ENV", previous);

    result
}

#[test]
fn start_skips_attach_when_herdr_env_is_set() -> Result<()> {
    let _guard = env_lock()?;
    let previous = set_herdr_env("1");

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| false);
    api.expect_ensure_server().times(1).returning(|_| Ok(()));
    api.expect_workspace_create()
        .times(1)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_report_metadata()
        .times(1)
        .returning(|_, _| Ok(()));

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let result = herdr.start(&start_session(Vec::new()), &start_env_vars(), false, true);
    restore_env("HERDR_ENV", previous);

    result
}

#[test]
fn start_skips_attach_when_requested() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| false);
    api.expect_ensure_server().times(1).returning(|_| Ok(()));
    api.expect_workspace_create()
        .times(1)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_report_metadata()
        .times(1)
        .returning(|_, _| Ok(()));

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let result = herdr.start(&start_session(Vec::new()), &start_env_vars(), true, true);
    restore_env("HERDR_ENV", previous);

    result
}

#[test]
fn start_sleeps_pane_cmd_delay_before_first_send_input() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let mut pane = test_pane(1, None, ".");
    pane.commands = vec![Command::from_string("echo hi")];

    let mut session = start_session(vec![test_window("main", FlexDirection::Row, vec![pane])]);
    session.pane_cmd_delay = Some(50);

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| false);
    api.expect_ensure_server().times(1).returning(|_| Ok(()));
    api.expect_workspace_create()
        .times(1)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_layout_apply()
        .times(1)
        .returning(|_, _| Ok(layout_with_pane("t1", "p1")));
    api.expect_pane_send_input()
        .times(1)
        .returning(|_, _| Ok(()));
    api.expect_workspace_report_metadata()
        .times(1)
        .returning(|_, _| Ok(()));

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let started = Instant::now();
    let result = herdr.start(&session, &start_env_vars(), true, true);
    let elapsed = started.elapsed();
    restore_env("HERDR_ENV", previous);

    result?;
    assert!(
        elapsed >= Duration::from_millis(50),
        "start should wait for pane_cmd_delay, elapsed {elapsed:?}"
    );
    Ok(())
}
