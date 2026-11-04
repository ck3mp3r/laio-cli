use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct SessionListOutput {
    #[serde(default)]
    pub(crate) sessions: Vec<SessionListEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct SessionListEntry {
    pub(crate) session_id: String,
    pub(crate) label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct CreateOutput {
    pub(crate) session_id: String,
    #[serde(default)]
    pub(crate) initial_windows: Vec<CreateWindowOut>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct CreateWindowOut {
    pub(crate) window_id: String,
    #[serde(default)]
    pub(crate) block_ids: Vec<String>,
    pub(crate) revision: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct SessionView {
    pub(crate) revision: u32,
    pub(crate) session_id: Option<String>,
    pub(crate) label: Option<String>,
    pub(crate) active_window_id: Option<String>,
    pub(crate) windows: Option<Vec<WindowView>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct WindowView {
    pub(crate) window_id: Option<String>,
    pub(crate) label: Option<String>,
    pub(crate) focused_block_id: Option<String>,
    pub(crate) zoomed_block_id: Option<String>,
    pub(crate) layers: Option<Vec<LayerView>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct LayerView {
    pub(crate) layer_id: Option<String>,
    pub(crate) kind: String,
    pub(crate) bounds: Vec<f64>,
    pub(crate) layout: Option<ViewNode>,
    pub(crate) blocks: Option<Vec<BlockRef>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct ViewNode {
    pub(crate) rect: Vec<f64>,
    pub(crate) block_id: Option<String>,
    pub(crate) split: Option<Box<ViewSplit>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct ViewSplit {
    pub(crate) split_id: Option<String>,
    pub(crate) direction: String,
    pub(crate) ratio: Option<f64>,
    pub(crate) before: Option<ViewNode>,
    pub(crate) after: Option<ViewNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct BlockRef {
    pub(crate) block_id: String,
    pub(crate) creator_name: Option<String>,
    pub(crate) flavor: Option<String>,
    pub(crate) label: Option<String>,
    pub(crate) owner: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct PwdOutput {
    pub(crate) pwd: String,
    pub(crate) owner: Option<PwdOwner>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct PwdOwner {
    pub(crate) pid: u32,
    pub(crate) name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct ClientListOutput {
    pub(crate) clients: Option<Vec<ClientEntry>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct ClientEntry {
    pub(crate) client_id: String,
    pub(crate) connection_state: Option<String>,
    pub(crate) info: Option<ClientInfo>,
    pub(crate) session_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct ClientInfo {
    pub(crate) kind: Option<String>,
    pub(crate) platform: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct FocusOutput {
    pub(crate) revision: u32,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub(crate) struct CreateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) initial_windows: Option<Vec<CreateWindow>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct CreateWindow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) window_label: Option<String>,
    pub(crate) layout: LayoutNode,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct LayoutNode {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) block: Option<BlockNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) split: Option<SplitNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct BlockNode {
    pub(crate) flavor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) options: Option<BlockOptions>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct SplitNode {
    pub(crate) direction: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ratio: Option<f64>,
    pub(crate) before: Box<LayoutNode>,
    pub(crate) after: Box<LayoutNode>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub(crate) struct BlockOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) shell: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) login: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) initial_input: Option<String>,
}
