use serde_json::{Value, json};

pub(super) fn pong_response() -> String {
    json!({
        "id": "1",
        "result": {"type": "pong", "version": "0.9.5", "protocol": 22, "capabilities": {}}
    })
    .to_string()
        + "\n"
}

pub(super) fn ok_response() -> String {
    json!({"id": "1", "result": {"type": "ok"}}).to_string() + "\n"
}

pub(super) fn workspace_created_response() -> String {
    json!({
        "id": "1",
        "result": {
            "type": "workspace_created",
            "workspace": {"workspace_id": "w1", "label": "main", "focused": true},
            "tab": {"tab_id": "t1", "workspace_id": "w1", "label": "main", "focused": true},
            "root_pane": {"pane_id": "p1", "workspace_id": "w1", "tab_id": "t1", "focused": true}
        }
    })
    .to_string()
        + "\n"
}

pub(super) fn workspace_info_response(tokens: Value) -> String {
    json!({
        "id": "1",
        "result": {
            "type": "workspace_info",
            "workspace": {
                "workspace_id": "w1",
                "label": "main",
                "focused": true,
                "tokens": tokens
            }
        }
    })
    .to_string()
        + "\n"
}

pub(super) fn layout_response(kind: &str) -> String {
    json!({
        "id": "1",
        "result": {
            "type": kind,
            "layout": {
                "workspace_id": "w1",
                "tab_id": "t1",
                "zoomed": false,
                "focused_pane_id": "p1",
                "root": {
                    "type": "split",
                    "direction": "right",
                    "ratio": 0.5,
                    "first": {"type": "pane", "pane_id": "p1", "cwd": "/tmp/a"},
                    "second": {"type": "pane", "pane_id": "p2", "label": "logs"}
                }
            }
        }
    })
    .to_string()
        + "\n"
}

pub(super) fn pane_info_response() -> String {
    json!({
        "id": "1",
        "result": {
            "type": "pane_info",
            "pane": {
                "pane_id": "p1",
                "workspace_id": "w1",
                "tab_id": "t1",
                "focused": true,
                "cwd": "/tmp/a",
                "label": "editor"
            }
        }
    })
    .to_string()
        + "\n"
}

pub(super) fn pane_zoom_response() -> String {
    json!({
        "id": "1",
        "result": {
            "type": "pane_zoom",
            "zoom": {"pane_id": "p1", "focused_pane_id": "p1", "zoomed": true}
        }
    })
    .to_string()
        + "\n"
}

pub(super) fn workspace_list_response(workspaces: Value) -> String {
    json!({
        "id": "1",
        "result": {"type": "workspace_list", "workspaces": workspaces}
    })
    .to_string()
        + "\n"
}

pub(super) fn workspace_entry(label: &str, tokens: Value) -> Value {
    json!({
        "workspace_id": "w1",
        "label": label,
        "focused": true,
        "tokens": tokens
    })
}
