use crate::common::config::{Session, Window};

use super::model::LayoutDescription;
use super::support_models::*;

pub(super) fn start_session(windows: Vec<Window>) -> Session {
    let mut session = test_session(windows);
    session.name = "work".to_string();
    session.path = env!("CARGO_MANIFEST_DIR").to_string();
    session
}

pub(super) fn start_env_vars() -> Vec<(&'static str, &'static str)> {
    vec![("LAIO_CONFIG", "/tmp/laio.yaml"), ("LAIO_VARS", "a=b")]
}

pub(super) fn layout_with_pane(tab_id: &str, pane_id: &str) -> LayoutDescription {
    LayoutDescription {
        workspace_id: "w1".to_string(),
        tab_id: tab_id.to_string(),
        zoomed: false,
        focused_pane_id: pane_id.to_string(),
        root: pane_node(pane_id),
    }
}
