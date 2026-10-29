use std::collections::HashMap;

use miette::{IntoDiagnostic, Result, miette};
use serde_json::json;

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::config::FlexDirection;
use crate::common::muxer::Multiplexer;

use super::super::client::HerdrClient;
use super::super::model::{
    LayoutDescription, LayoutNode, LayoutPane, PaneInfo, PaneLayoutSnapshot, SessionSnapshot,
    SplitDirection, TabInfo,
};
use super::super::mux::Herdr;

use super::*;

#[test]
fn get_session_config_path_joins_chunked_tokens() -> Result<()> {
    let session = TestSession::new()?;
    let previous_socket = set_herdr_socket_path(&session.socket_path()?.to_string_lossy());
    let listener = session.bind()?;
    let handle = spawn_canned_server(
        listener,
        vec![
            workspace_list_response(json!([workspace_entry(
                session.name(),
                json!({"LAIO_CONFIG": "/tmp/laio.yaml"})
            )])),
            workspace_info_response(json!({
                "LAIO_CONFIG_1": "/tmp/la",
                "LAIO_CONFIG_2": "io.yaml"
            })),
        ],
    );

    let herdr =
        Herdr::new_with_runner(empty_runner(), HerdrClient::new_with_runner(empty_runner()));
    let config = herdr.get_session_config_path(session.name())?;

    let _ = join_thread(handle)?;
    restore_env("HERDR_SOCKET_PATH", previous_socket);
    assert_eq!(config.as_deref(), Some("/tmp/laio.yaml"));
    Ok(())
}

#[test]
fn get_session_config_path_returns_none_when_not_running() -> Result<()> {
    let session = TestSession::new()?;
    let herdr = Herdr::new_with_runner(
        runner_with_session_list(empty_session_list()),
        HerdrClient::new_with_runner(empty_runner()),
    );

    assert_eq!(herdr.get_session_config_path(session.name())?, None);
    Ok(())
}

#[test]
fn get_session_variables_joins_and_decodes_chunked_tokens() -> Result<()> {
    let session = TestSession::new()?;
    let previous_socket = set_herdr_socket_path(&session.socket_path()?.to_string_lossy());
    let listener = session.bind()?;
    let handle = spawn_canned_server(
        listener,
        vec![
            workspace_list_response(json!([workspace_entry(
                session.name(),
                json!({"LAIO_CONFIG": "/tmp/laio.yaml", "LAIO_VARS": "a=1&b=2"})
            )])),
            workspace_info_response(json!({
                "LAIO_VARS_1": "a=1&b",
                "LAIO_VARS_2": "=2"
            })),
        ],
    );

    let herdr =
        Herdr::new_with_runner(empty_runner(), HerdrClient::new_with_runner(empty_runner()));
    let variables = herdr.get_session_variables(session.name())?;

    let _ = join_thread(handle)?;
    restore_env("HERDR_SOCKET_PATH", previous_socket);
    assert_eq!(variables, Some(vec!["a=1".to_string(), "b=2".to_string()]));
    Ok(())
}

#[test]
fn get_session_variables_returns_none_when_absent() -> Result<()> {
    let session = TestSession::new()?;
    let previous_socket = set_herdr_socket_path(&session.socket_path()?.to_string_lossy());
    let listener = session.bind()?;
    let handle = spawn_canned_server(
        listener,
        vec![
            workspace_list_response(json!([workspace_entry(
                session.name(),
                json!({"LAIO_CONFIG": "/tmp/laio.yaml"})
            )])),
            workspace_info_response(json!({})),
        ],
    );

    let herdr =
        Herdr::new_with_runner(empty_runner(), HerdrClient::new_with_runner(empty_runner()));
    let variables = herdr.get_session_variables(session.name())?;

    let _ = join_thread(handle)?;
    restore_env("HERDR_SOCKET_PATH", previous_socket);
    assert_eq!(variables, None);
    Ok(())
}

pub(super) fn session_snapshot() -> SessionSnapshot {
    SessionSnapshot {
        focused_workspace_id: Some("w1".to_string()),
        focused_tab_id: Some("t1".to_string()),
        workspaces: vec![workspace_info()],
        tabs: vec![TabInfo {
            tab_id: "t1".to_string(),
            workspace_id: "w1".to_string(),
            label: "main".to_string(),
            focused: true,
        }],
        panes: vec![pane_info()],
        layouts: vec![PaneLayoutSnapshot {
            tab_id: "t1".to_string(),
            zoomed: false,
            focused_pane_id: "p1".to_string(),
            panes: Vec::new(),
            splits: Vec::new(),
        }],
    }
}

pub(super) fn export_layout(
    tab_id: &str,
    zoomed: bool,
    focused_pane_id: &str,
    root: LayoutNode,
) -> LayoutDescription {
    LayoutDescription {
        workspace_id: "w1".to_string(),
        tab_id: tab_id.to_string(),
        zoomed,
        focused_pane_id: focused_pane_id.to_string(),
        root,
    }
}

pub(super) fn export_pane(
    pane_id: &str,
    label: Option<&str>,
    cwd: &str,
    command: Option<Vec<&str>>,
) -> LayoutNode {
    LayoutNode::Pane {
        pane: LayoutPane {
            pane_id: Some(pane_id.to_string()),
            label: label.map(str::to_string),
            cwd: Some(cwd.to_string()),
            command: command.map(|argv| argv.into_iter().map(str::to_string).collect()),
            env: HashMap::new(),
        },
    }
}

fn export_pane_info(workspace_id: &str, pane_id: &str, tab_id: &str, cwd: &str) -> PaneInfo {
    PaneInfo {
        pane_id: pane_id.to_string(),
        workspace_id: workspace_id.to_string(),
        tab_id: tab_id.to_string(),
        focused: false,
        cwd: Some(cwd.to_string()),
        label: None,
    }
}

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
#[test]
fn session_snapshot_deserializes_the_wire_shape() -> Result<()> {
    let snapshot: SessionSnapshot = serde_json::from_value(json!({
        "version": "0.9.5",
        "protocol": 22,
        "focused_workspace_id": "w1",
        "focused_tab_id": "t1",
        "workspaces": [{"workspace_id": "w1", "label": "main", "focused": true}],
        "tabs": [{"tab_id": "t1", "workspace_id": "w1", "label": "editor", "focused": true}],
        "panes": [{
            "pane_id": "p1",
            "workspace_id": "w1",
            "tab_id": "t1",
            "focused": true,
            "cwd": "/tmp/project"
        }],
        "layouts": [{
            "workspace_id": "w1",
            "tab_id": "t1",
            "zoomed": true,
            "area": {"x": 0, "y": 0, "width": 80, "height": 24},
            "focused_pane_id": "p1",
            "panes": [{
                "pane_id": "p1",
                "focused": true,
                "rect": {"x": 0, "y": 0, "width": 40, "height": 24}
            }],
            "splits": [{
                "id": "s1",
                "direction": "right",
                "ratio": 0.5,
                "rect": {"x": 0, "y": 0, "width": 80, "height": 24}
            }]
        }]
    }))
    .into_diagnostic()?;

    assert_eq!(snapshot.focused_workspace_id.as_deref(), Some("w1"));
    assert_eq!(snapshot.focused_tab_id.as_deref(), Some("t1"));
    assert_eq!(snapshot.workspaces.len(), 1);
    assert_eq!(snapshot.tabs.len(), 1);
    assert_eq!(snapshot.panes.len(), 1);

    let layout = snapshot
        .layouts
        .first()
        .ok_or_else(|| miette!("snapshot should carry a layout"))?;
    assert_eq!(layout.tab_id, "t1");
    assert!(layout.zoomed);
    assert_eq!(layout.focused_pane_id, "p1");

    let pane = layout
        .panes
        .first()
        .ok_or_else(|| miette!("layout should carry a pane"))?;
    assert_eq!(pane.pane_id, "p1");
    assert!(pane.focused);
    assert_eq!(pane.rect.x, 0);
    assert_eq!(pane.rect.y, 0);
    assert_eq!(pane.rect.width, 40);
    assert_eq!(pane.rect.height, 24);

    let split = layout
        .splits
        .first()
        .ok_or_else(|| miette!("layout should carry a split"))?;
    assert_eq!(split.id, "s1");
    assert_eq!(split.direction, SplitDirection::Right);
    assert_eq!(split.ratio, 0.5);
    assert_eq!(split.rect.width, 80);
    Ok(())
}

#[test]
fn read_backs_resolve_a_workspace_hosted_in_a_foreign_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous_socket = clear_herdr_socket_path();

    let sessions = two_session_list();
    let tokens = HashMap::from([
        ("LAIO_CONFIG".to_string(), "/tmp/x.yaml".to_string()),
        ("LAIO_VARS".to_string(), "a=1&b=2".to_string()),
    ]);

    let mut api = MockHerdrApiMock::new();
    let mut cmd_string = MockCmdStringMock::new();

    cmd_string
        .expect_run()
        .times(2)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));
    api.expect_session_exists()
        .withf(|session| session == "laio")
        .times(2)
        .returning(|_| true);
    api.expect_workspace_list()
        .withf(|session| session == "laio")
        .times(4)
        .returning(|_| Ok(Vec::new()));
    api.expect_workspace_list()
        .withf(|session| session == "work")
        .times(2)
        .returning({
            let tokens = tokens.clone();
            move |_| Ok(vec![workspace_info_with("wx", "x", tokens.clone())])
        });
    api.expect_workspace_get()
        .times(2)
        .withf(|session, workspace_id| session == "work" && workspace_id == "wx")
        .returning(move |_, _| Ok(workspace_info_with("wx", "x", tokens.clone())));

    let runner = RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string,
        cmd_bool: MockCmdBoolMock::new(),
    };
    let herdr = Herdr::new_with_runner(runner, api);

    let config = herdr.get_session_config_path("x");
    let variables = herdr.get_session_variables("x");
    restore_env("HERDR_SOCKET_PATH", previous_socket);

    assert_eq!(config?, Some("/tmp/x.yaml".to_string()));
    assert_eq!(variables?, Some(vec!["a=1".to_string(), "b=2".to_string()]));
    Ok(())
}
