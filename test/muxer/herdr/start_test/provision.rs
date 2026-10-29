use std::{collections::HashMap, env};

use miette::{IntoDiagnostic, Result, miette};
use mockall::Sequence;

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::config::{Command, FlexDirection};
use crate::common::muxer::Multiplexer;

use super::model::PaneZoomMode;
use super::mux::{Herdr, LAIO_SESSION_NAME};

use super::start_test_support::*;
use super::support::*;

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
