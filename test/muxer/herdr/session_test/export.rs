use std::collections::HashMap;

use miette::{Result, miette};

use crate::common::config::FlexDirection;
use crate::common::muxer::Multiplexer;

use super::model::{LayoutNode, PaneLayoutSnapshot, SessionSnapshot, SplitDirection, TabInfo};
use super::mux::Herdr;

use super::session_test_support::*;
use super::support_api_mock::*;
use super::support_env::*;
use super::support_models::*;
use super::support_socket::*;

#[test]
fn get_session_requires_herdr_env() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let result = herdr.get_session();
    restore_env("HERDR_ENV", previous);

    let error = result
        .err()
        .ok_or_else(|| miette!("missing HERDR_ENV should return Err"))?;

    assert_eq!(
        error.to_string(),
        "You do not seem to be inside a herdr session."
    );
    Ok(())
}

#[test]
fn get_session_requires_herdr_workspace_id() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = clear_herdr_workspace_id();

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let result = herdr.get_session();
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);

    let error = result
        .err()
        .ok_or_else(|| miette!("missing HERDR_WORKSPACE_ID should return Err"))?;

    assert!(
        error.to_string().contains("HERDR_WORKSPACE_ID"),
        "report should name the variable: {error}"
    );
    Ok(())
}

#[test]
fn get_session_bails_when_the_workspace_is_absent() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w-missing");

    let mut api = MockHerdrApiMock::new();
    api.expect_session_snapshot()
        .times(1)
        .returning(|_| Ok(session_snapshot()));

    let herdr = Herdr::new_with_runner(empty_runner(), api);
    let result = herdr.get_session();
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);

    let error = result
        .err()
        .ok_or_else(|| miette!("unknown workspace should return Err"))?;

    assert!(
        error.to_string().contains("w-missing"),
        "report should name the workspace: {error}"
    );
    Ok(())
}

#[test]
fn get_session_exports_tabs_panes_focus_and_zoom() -> Result<()> {
    let _guard = env_lock()?;
    let previous_env = set_herdr_env("1");
    let previous_workspace = set_herdr_workspace_id("w1");
    let previous_socket = set_herdr_socket_path("/tmp/herdr/sessions/work/herdr.sock");

    let snapshot = SessionSnapshot {
        focused_workspace_id: Some("w1".to_string()),
        focused_tab_id: Some("t1".to_string()),
        workspaces: vec![
            workspace_info(),
            workspace_info_with("w2", "other", HashMap::new()),
        ],
        tabs: vec![
            TabInfo {
                tab_id: "t1".to_string(),
                workspace_id: "w1".to_string(),
                label: "editor".to_string(),
                focused: true,
            },
            TabInfo {
                tab_id: "t2".to_string(),
                workspace_id: "w1".to_string(),
                label: "logs".to_string(),
                focused: false,
            },
            TabInfo {
                tab_id: "t3".to_string(),
                workspace_id: "w2".to_string(),
                label: "foreign".to_string(),
                focused: false,
            },
        ],
        panes: vec![
            export_pane_info("w1", "p1", "t1", "/tmp/project/a"),
            export_pane_info("w1", "p2", "t1", "/tmp/project/b"),
            export_pane_info("w1", "p3", "t2", "/tmp/project/c"),
            export_pane_info("w2", "p4", "t3", "/tmp/elsewhere"),
        ],
        layouts: vec![
            PaneLayoutSnapshot {
                tab_id: "t1".to_string(),
                zoomed: true,
                focused_pane_id: "p1".to_string(),
                panes: Vec::new(),
                splits: Vec::new(),
            },
            PaneLayoutSnapshot {
                tab_id: "t2".to_string(),
                zoomed: false,
                focused_pane_id: "p3".to_string(),
                panes: Vec::new(),
                splits: Vec::new(),
            },
            PaneLayoutSnapshot {
                tab_id: "t3".to_string(),
                zoomed: false,
                focused_pane_id: "p4".to_string(),
                panes: Vec::new(),
                splits: Vec::new(),
            },
        ],
    };

    let editor = export_layout(
        "t1",
        false,
        "p1",
        LayoutNode::Split {
            direction: SplitDirection::Right,
            ratio: 0.5,
            first: Box::new(export_pane(
                "p1",
                Some("main"),
                "/tmp/project/a",
                Some(vec!["echo", "hi"]),
            )),
            second: Box::new(export_pane("p2", None, "/tmp/project/b", None)),
        },
    );
    let logs = export_layout(
        "t2",
        false,
        "p3",
        export_pane("p3", None, "/tmp/project/c", None),
    );

    let mut api = MockHerdrApiMock::new();
    api.expect_session_snapshot()
        .withf(|session| session == "work")
        .times(1)
        .returning(move |_| Ok(snapshot.clone()));
    api.expect_layout_export()
        .withf(|_, tab_id| tab_id == "t1")
        .times(1)
        .returning(move |_, _| Ok(editor.clone()));
    api.expect_layout_export()
        .withf(|_, tab_id| tab_id == "t2")
        .times(1)
        .returning(move |_, _| Ok(logs.clone()));

    let herdr = Herdr::new_with_runner(empty_runner(), api);
    let result = herdr.get_session();
    restore_env("HERDR_ENV", previous_env);
    restore_env("HERDR_WORKSPACE_ID", previous_workspace);
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    let session = result?;

    assert_eq!(session.name, "main");
    assert_eq!(session.path, "/tmp/project");
    assert_eq!(session.windows.len(), 2);

    let editor = &session.windows[0];
    assert_eq!(editor.name, "editor");
    assert_eq!(editor.flex_direction, FlexDirection::Row);
    assert_eq!(editor.panes.len(), 2);
    assert_eq!(editor.panes[0].flex, 1);
    assert_eq!(editor.panes[0].name.as_deref(), Some("main"));
    assert_eq!(editor.panes[0].path, "a");
    assert_eq!(editor.panes[0].commands.len(), 1);
    assert_eq!(editor.panes[0].commands[0].to_string(), "echo hi");
    assert!(editor.panes[0].zoom);
    assert!(editor.panes[0].focus);
    assert_eq!(editor.panes[1].flex, 1);
    assert_eq!(editor.panes[1].path, "b");
    assert!(!editor.panes[1].zoom);
    assert!(!editor.panes[1].focus);

    let logs = &session.windows[1];
    assert_eq!(logs.name, "logs");
    assert_eq!(logs.panes.len(), 1);
    assert_eq!(logs.panes[0].path, "c");
    assert!(!logs.panes[0].zoom);
    assert!(!logs.panes[0].focus);
    Ok(())
}
