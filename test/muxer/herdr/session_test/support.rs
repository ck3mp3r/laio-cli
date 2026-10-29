use std::collections::HashMap;

use super::model::{
    LayoutDescription, LayoutNode, LayoutPane, PaneInfo, PaneLayoutSnapshot, SessionSnapshot,
    TabInfo,
};

use super::support_models::*;

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

pub(super) fn export_pane_info(
    workspace_id: &str,
    pane_id: &str,
    tab_id: &str,
    cwd: &str,
) -> PaneInfo {
    PaneInfo {
        pane_id: pane_id.to_string(),
        workspace_id: workspace_id.to_string(),
        tab_id: tab_id.to_string(),
        focused: false,
        cwd: Some(cwd.to_string()),
        label: None,
    }
}
