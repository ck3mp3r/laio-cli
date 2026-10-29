use miette::{IntoDiagnostic, Result, miette};
use serde_json::json;

use super::model::{SessionSnapshot, SplitDirection};

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
