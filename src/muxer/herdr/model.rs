use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct HerdrRequest {
    pub(crate) id: String,
    pub(crate) method: String,
    pub(crate) params: Value,
}

impl HerdrRequest {
    pub(crate) fn new(id: String, method: &str, params: Value) -> Self {
        Self {
            id,
            method: method.to_string(),
            params,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct HerdrResponse {
    #[serde(default)]
    pub(crate) result: Option<Value>,
    #[serde(default)]
    pub(crate) error: Option<HerdrErrorBody>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct HerdrErrorBody {
    pub(crate) code: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct HerdrPong {
    pub(crate) version: String,
    pub(crate) protocol: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct WorkspaceCreateParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) cwd: Option<String>,
    pub(crate) focus: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) label: Option<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub(crate) env: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct WorkspaceReportMetadataParams {
    pub(crate) workspace_id: String,
    pub(crate) source: String,
    pub(crate) tokens: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) seq: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct LayoutApplyParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tab_label: Option<String>,
    pub(crate) focus: bool,
    pub(crate) root: LayoutNode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PaneSendInputParams {
    pub(crate) pane_id: String,
    pub(crate) text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct WorkspaceInfo {
    pub(crate) workspace_id: String,
    pub(crate) label: String,
    pub(crate) focused: bool,
    #[serde(default)]
    pub(crate) tokens: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct TabInfo {
    pub(crate) tab_id: String,
    pub(crate) workspace_id: String,
    pub(crate) label: String,
    pub(crate) focused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct PaneInfo {
    pub(crate) pane_id: String,
    pub(crate) workspace_id: String,
    pub(crate) tab_id: String,
    pub(crate) focused: bool,
    #[serde(default)]
    pub(crate) cwd: Option<String>,
    #[serde(default)]
    pub(crate) label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct WorkspaceCreated {
    pub(crate) workspace: WorkspaceInfo,
    pub(crate) tab: TabInfo,
    pub(crate) root_pane: PaneInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct WorkspaceInfoResponse {
    pub(crate) workspace: WorkspaceInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct WorkspaceListResponse {
    pub(crate) workspaces: Vec<WorkspaceInfo>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct SessionSnapshot {
    #[serde(default)]
    pub(crate) focused_workspace_id: Option<String>,
    #[serde(default)]
    pub(crate) focused_tab_id: Option<String>,
    #[serde(default)]
    pub(crate) workspaces: Vec<WorkspaceInfo>,
    #[serde(default)]
    pub(crate) tabs: Vec<TabInfo>,
    #[serde(default)]
    pub(crate) panes: Vec<PaneInfo>,
    #[serde(default)]
    pub(crate) layouts: Vec<PaneLayoutSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct SessionSnapshotResponse {
    pub(crate) snapshot: SessionSnapshot,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct PaneLayoutSnapshot {
    pub(crate) tab_id: String,
    pub(crate) zoomed: bool,
    pub(crate) focused_pane_id: String,
    #[serde(default)]
    pub(crate) panes: Vec<PaneLayoutPane>,
    #[serde(default)]
    pub(crate) splits: Vec<PaneLayoutSplit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct PaneLayoutPane {
    pub(crate) pane_id: String,
    pub(crate) focused: bool,
    pub(crate) rect: PaneLayoutRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct PaneLayoutRect {
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) width: u16,
    pub(crate) height: u16,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct PaneLayoutSplit {
    pub(crate) id: String,
    pub(crate) direction: SplitDirection,
    pub(crate) ratio: f32,
    pub(crate) rect: PaneLayoutRect,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct LayoutDescription {
    pub(crate) workspace_id: String,
    pub(crate) tab_id: String,
    pub(crate) zoomed: bool,
    pub(crate) focused_pane_id: String,
    pub(crate) root: LayoutNode,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct LayoutResponse {
    pub(crate) layout: LayoutDescription,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum LayoutNode {
    Pane {
        #[serde(flatten)]
        pane: LayoutPane,
    },
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct LayoutPane {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pane_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) command: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) env: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SplitDirection {
    Right,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PaneZoomMode {
    #[default]
    Toggle,
    On,
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct PaneZoomResult {
    pub(crate) pane_id: String,
    pub(crate) focused_pane_id: String,
    pub(crate) zoomed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct PaneZoomResponse {
    pub(crate) zoom: PaneZoomResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct PaneResponse {
    pub(crate) pane: PaneInfo,
}
