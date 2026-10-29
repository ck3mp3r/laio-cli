use std::{
    collections::HashMap,
    env,
    time::{Duration, Instant},
};

use miette::{IntoDiagnostic, Result, miette};
use mockall::Sequence;
use serde_json::json;

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::config::{Command, FlexDirection, Session, Window};
use crate::common::muxer::Multiplexer;

use super::model::{LayoutDescription, PaneZoomMode};
use super::mux::{Herdr, LAIO_SESSION_NAME};

use super::support::*;

pub(super) fn start_session(windows: Vec<Window>) -> Session {
    let mut session = test_session(windows);
    session.name = "work".to_string();
    session.path = env!("CARGO_MANIFEST_DIR").to_string();
    session
}

fn start_env_vars() -> Vec<(&'static str, &'static str)> {
    vec![("LAIO_CONFIG", "/tmp/laio.yaml"), ("LAIO_VARS", "a=b")]
}

fn layout_with_pane(tab_id: &str, pane_id: &str) -> LayoutDescription {
    LayoutDescription {
        workspace_id: "w1".to_string(),
        tab_id: tab_id.to_string(),
        zoomed: false,
        focused_pane_id: pane_id.to_string(),
        root: pane_node(pane_id),
    }
}
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
fn start_creates_workspace_applies_windows_and_writes_metadata() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let mut first_pane = test_pane(1, Some("main"), ".");
    first_pane.zoom = true;
    first_pane.focus = true;
    first_pane.commands = vec![Command::from_string("echo hi")];

    let mut second_pane = test_pane(1, None, ".");
    second_pane.commands = vec![Command::from_string("tail -f log")];

    let mut session = start_session(vec![
        test_window("editor", FlexDirection::Row, vec![first_pane]),
        test_window("logs", FlexDirection::Row, vec![second_pane]),
    ]);
    session.windows[0].focus = true;
    session.startup = vec![Command::from_string("date")];
    session.startup_script =
        Some(noyalib::compat::serde_yaml::from_str("echo hi").into_diagnostic()?);
    session.env = HashMap::from([("FOO".to_string(), "bar".to_string())]);

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
        .returning(|_| Ok(empty_session_list()));
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
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string().contains("laio-")))
        .returning(|_| Ok(String::new()));
    api.expect_workspace_create()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|session, params| {
            session == LAIO_SESSION_NAME
                && params.cwd.as_deref() == Some(env!("CARGO_MANIFEST_DIR"))
                && params.label.as_deref() == Some("work")
                && params.focus
                && params.env.get("FOO").map(String::as_str) == Some("bar")
        })
        .returning(|_, _| Ok(workspace_created()));
    api.expect_workspace_report_metadata()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| {
            params.workspace_id == "w1"
                && params.source == "laio"
                && params.tokens.get("LAIO_CONFIG").map(String::as_str) == Some("/tmp/laio.yaml")
                && params.tokens.get("LAIO_VARS").map(String::as_str) == Some("a=b")
        })
        .returning(|_, _| Ok(()));
    api.expect_layout_apply()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| {
            params.tab_label.as_deref() == Some("editor")
                && params.focus
                && params.tab_id.as_deref() == Some("t1")
                && params.workspace_id.is_none()
        })
        .returning(|_, _| Ok(layout_with_pane("t1", "p1")));
    api.expect_pane_zoom()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, pane_id, mode| pane_id == "p1" && *mode == PaneZoomMode::On)
        .returning(|_, _, _| Ok(pane_zoom_result()));
    api.expect_pane_focus()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, pane_id| pane_id == "p1")
        .returning(|_, _| Ok(pane_info()));
    api.expect_pane_send_input()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| {
            params.pane_id == "p1"
                && params.text == "echo hi"
                && params.keys == vec!["Enter".to_string()]
        })
        .returning(|_, _| Ok(()));
    api.expect_layout_apply()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| {
            params.tab_label.as_deref() == Some("logs")
                && !params.focus
                && params.workspace_id.as_deref() == Some("w1")
                && params.tab_id.is_none()
        })
        .returning(|_, _| Ok(layout_with_pane("t2", "p2")));
    api.expect_pane_send_input()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| params.pane_id == "p2" && params.text == "tail -f log")
        .returning(|_, _| Ok(()));
    cmd_unit
        .expect_run()
        .times(1)
        .in_sequence(&mut seq)
        .withf(
            |cmd| matches!(cmd, Type::Forget(_) if cmd.to_string() == "herdr session attach laio"),
        )
        .returning(|_| Ok(()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let result = herdr.start(&session, &start_env_vars(), false, false);
    restore_env("HERDR_ENV", previous);

    result
}

#[test]
fn start_applies_first_window_as_replace_and_the_rest_as_append() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let mut session = start_session(vec![
        test_window("one", FlexDirection::Row, vec![test_pane(1, None, ".")]),
        test_window("two", FlexDirection::Row, vec![test_pane(1, None, ".")]),
        test_window("three", FlexDirection::Row, vec![test_pane(1, None, ".")]),
    ]);
    session.windows[1].focus = true;

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists().times(1).returning(|_| false);
    api.expect_ensure_server().times(1).returning(|_| Ok(()));
    api.expect_workspace_create()
        .times(1)
        .returning(|_, _| Ok(workspace_created()));
    api.expect_layout_apply()
        .times(1)
        .withf(|_, params| {
            params.tab_label.as_deref() == Some("one")
                && params.tab_id.as_deref() == Some("t1")
                && params.workspace_id.is_none()
                && !params.focus
        })
        .returning(|_, _| Ok(layout_with_pane("t1", "p1")));
    api.expect_layout_apply()
        .times(1)
        .withf(|_, params| {
            params.tab_label.as_deref() == Some("two")
                && params.workspace_id.as_deref() == Some("w1")
                && params.tab_id.is_none()
                && params.focus
        })
        .returning(|_, _| Ok(layout_with_pane("t2", "p2")));
    api.expect_layout_apply()
        .times(1)
        .withf(|_, params| {
            params.tab_label.as_deref() == Some("three")
                && params.workspace_id.as_deref() == Some("w1")
                && params.tab_id.is_none()
                && !params.focus
        })
        .returning(|_, _| Ok(layout_with_pane("t3", "p3")));
    api.expect_workspace_report_metadata()
        .times(1)
        .returning(|_, _| Ok(()));

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let result = herdr.start(&session, &start_env_vars(), true, true);
    restore_env("HERDR_ENV", previous);

    result
}

#[test]
fn start_writes_metadata_before_applying_windows() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let mut session = start_session(vec![
        test_window("one", FlexDirection::Row, vec![test_pane(1, None, ".")]),
        test_window("two", FlexDirection::Row, vec![test_pane(1, None, ".")]),
    ]);
    session.windows[0].focus = true;

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| false);
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
        .withf(|_, params| {
            params.workspace_id == "w1"
                && params.tokens.get("LAIO_CONFIG").map(String::as_str) == Some("/tmp/laio.yaml")
                && params.tokens.get("LAIO_VARS").map(String::as_str) == Some("a=b")
        })
        .returning(|_, _| Ok(()));
    api.expect_layout_apply()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| params.tab_label.as_deref() == Some("one"))
        .returning(|_, _| Ok(layout_with_pane("t1", "p1")));
    api.expect_layout_apply()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| params.tab_label.as_deref() == Some("two"))
        .returning(|_, _| Ok(layout_with_pane("t2", "p2")));

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let result = herdr.start(&session, &start_env_vars(), true, true);
    restore_env("HERDR_ENV", previous);

    result
}

#[test]
fn start_writes_metadata_before_a_failing_layout_apply() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let mut session = start_session(vec![
        test_window("one", FlexDirection::Row, vec![test_pane(1, None, ".")]),
        test_window("two", FlexDirection::Row, vec![test_pane(1, None, ".")]),
    ]);
    session.windows[0].focus = true;

    let mut seq = Sequence::new();

    let mut api = MockHerdrApiMock::new();
    api.expect_session_exists()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| false);
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
    api.expect_layout_apply()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| params.tab_label.as_deref() == Some("one"))
        .returning(|_, _| Ok(layout_with_pane("t1", "p1")));
    api.expect_layout_apply()
        .times(1)
        .in_sequence(&mut seq)
        .withf(|_, params| params.tab_label.as_deref() == Some("two"))
        .returning(|_, _| Err(miette!("layout apply failed")));

    let herdr = Herdr::new_with_runner(runner_with_session_list(empty_session_list()), api);

    let result = herdr.start(&session, &start_env_vars(), true, true);
    restore_env("HERDR_ENV", previous);

    assert!(result.is_err(), "a failing layout apply should abort start");
    Ok(())
}

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
