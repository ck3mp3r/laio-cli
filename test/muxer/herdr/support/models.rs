use std::collections::HashMap;

use serde_json::{Value, json};

use crate::common::cmd::{
    Type,
    test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
};

use crate::common::config::{FlexDirection, Pane, Session, Window};

use super::model::{
    LayoutDescription, LayoutNode, LayoutPane, PaneInfo, PaneZoomResult, TabInfo, WorkspaceCreated,
    WorkspaceInfo,
};

pub(super) fn workspace_created() -> WorkspaceCreated {
    WorkspaceCreated {
        workspace: workspace_info(),
        tab: TabInfo {
            tab_id: "t1".to_string(),
            workspace_id: "w1".to_string(),
            label: "main".to_string(),
            focused: true,
        },
        root_pane: pane_info(),
    }
}

pub(super) fn workspace_info() -> WorkspaceInfo {
    WorkspaceInfo {
        workspace_id: "w1".to_string(),
        label: "main".to_string(),
        focused: true,
        tokens: HashMap::new(),
    }
}

pub(super) fn layout_description() -> LayoutDescription {
    LayoutDescription {
        workspace_id: "w1".to_string(),
        tab_id: "t1".to_string(),
        zoomed: false,
        focused_pane_id: "p1".to_string(),
        root: LayoutNode::Pane {
            pane: LayoutPane::default(),
        },
    }
}

pub(super) fn pane_info() -> PaneInfo {
    PaneInfo {
        pane_id: "p1".to_string(),
        workspace_id: "w1".to_string(),
        tab_id: "t1".to_string(),
        focused: true,
        cwd: None,
        label: None,
    }
}

pub(super) fn pane_zoom_result() -> PaneZoomResult {
    PaneZoomResult {
        pane_id: "p1".to_string(),
        focused_pane_id: "p1".to_string(),
        zoomed: true,
    }
}

pub(super) fn test_session(windows: Vec<Window>) -> Session {
    Session {
        name: "test".to_string(),
        path: "/tmp/project".to_string(),
        startup: Vec::new(),
        startup_script: None,
        shutdown: Vec::new(),
        shutdown_script: None,
        env: HashMap::new(),
        shell: None,
        pane_cmd_delay: None,
        windows,
    }
}

pub(super) fn test_window(name: &str, flex_direction: FlexDirection, panes: Vec<Pane>) -> Window {
    Window {
        name: name.to_string(),
        path: None,
        flex_direction,
        panes,
        focus: false,
    }
}

pub(super) fn test_pane(flex: usize, name: Option<&str>, path: &str) -> Pane {
    Pane {
        flex_direction: FlexDirection::Row,
        flex,
        name: name.map(str::to_string),
        path: path.to_string(),
        style: None,
        commands: Vec::new(),
        script: None,
        panes: Vec::new(),
        zoom: false,
        focus: false,
    }
}

pub(super) fn pane_node(pane_id: &str) -> LayoutNode {
    LayoutNode::Pane {
        pane: LayoutPane {
            pane_id: Some(pane_id.to_string()),
            ..Default::default()
        },
    }
}

pub(super) fn contains_key(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(map) => {
            map.contains_key(key) || map.values().any(|value| contains_key(value, key))
        }
        Value::Array(items) => items.iter().any(|value| contains_key(value, key)),
        _ => false,
    }
}

pub(super) fn laio_tokens() -> HashMap<String, String> {
    HashMap::from([("LAIO_CONFIG".to_string(), "/tmp/laio.yaml".to_string())])
}

pub(super) fn config_tokens(path: &str) -> HashMap<String, String> {
    HashMap::from([("LAIO_CONFIG".to_string(), path.to_string())])
}

pub(super) fn workspace_info_with(
    workspace_id: &str,
    label: &str,
    tokens: HashMap<String, String>,
) -> WorkspaceInfo {
    WorkspaceInfo {
        workspace_id: workspace_id.to_string(),
        label: label.to_string(),
        focused: false,
        tokens,
    }
}

pub(super) fn two_session_list() -> String {
    json!({
        "sessions": [
            {"name": "laio", "default": false, "running": true},
            {"name": "work", "default": false, "running": true}
        ]
    })
    .to_string()
}

pub(super) fn empty_session_list() -> String {
    json!({"sessions": []}).to_string()
}

pub(super) fn session_list_cmd_string(sessions: String) -> MockCmdStringMock {
    let mut cmd_string = MockCmdStringMock::new();
    cmd_string
        .expect_run()
        .times(1..)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "herdr session list --json"),
        )
        .returning(move |_| Ok(sessions.clone()));

    cmd_string
}

pub(super) fn runner_with_session_list(sessions: String) -> RunnerMock {
    RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string: session_list_cmd_string(sessions),
        cmd_bool: MockCmdBoolMock::new(),
    }
}
