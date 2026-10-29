use miette::{Result, miette};
use serde_json::json;

use super::client::HerdrApi;
use super::model::{
    LayoutApplyParams, LayoutNode, LayoutPane, PaneSendInputParams, PaneZoomMode, SplitDirection,
};

use super::support_env::*;
use super::support_responses::*;
use super::support_socket::*;

#[test]
fn layout_apply_sends_root_tree_and_parses_layout() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![layout_response("layout_apply")]);

    let layout = session.client().layout_apply(
        session.name(),
        &LayoutApplyParams {
            workspace_id: Some("w1".to_string()),
            tab_id: None,
            tab_label: Some("main".to_string()),
            focus: true,
            root: LayoutNode::Split {
                direction: SplitDirection::Right,
                ratio: 0.5,
                first: Box::new(LayoutNode::Pane {
                    pane: LayoutPane {
                        cwd: Some("/tmp/a".to_string()),
                        ..Default::default()
                    },
                }),
                second: Box::new(LayoutNode::Pane {
                    pane: LayoutPane {
                        label: Some("logs".to_string()),
                        ..Default::default()
                    },
                }),
            },
        },
    )?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(layout.tab_id, "t1");
    assert_eq!(layout.focused_pane_id, "p1");
    match &layout.root {
        LayoutNode::Split {
            direction, ratio, ..
        } => {
            assert_eq!(*direction, SplitDirection::Right);
            assert_eq!(*ratio, 0.5);
        }
        other => return Err(miette!("root should be a split, got {other:?}")),
    }
    assert_eq!(request["method"], json!("layout.apply"));
    assert_eq!(request["params"]["workspace_id"], json!("w1"));
    assert_eq!(request["params"]["tab_label"], json!("main"));
    assert_eq!(request["params"]["focus"], json!(true));
    assert_eq!(request["params"]["root"]["type"], json!("split"));
    assert_eq!(request["params"]["root"]["direction"], json!("right"));
    assert_eq!(request["params"]["root"]["first"]["type"], json!("pane"));
    assert_eq!(request["params"]["root"]["first"]["cwd"], json!("/tmp/a"));
    assert_eq!(request["params"]["root"]["second"]["label"], json!("logs"));
    Ok(())
}

#[test]
fn layout_apply_sends_tab_id_and_omits_workspace_id() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![layout_response("layout_apply")]);

    session.client().layout_apply(
        session.name(),
        &LayoutApplyParams {
            workspace_id: None,
            tab_id: Some("w1:t1".to_string()),
            tab_label: Some("main".to_string()),
            focus: true,
            root: LayoutNode::Pane {
                pane: LayoutPane::default(),
            },
        },
    )?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(request["method"], json!("layout.apply"));
    assert_eq!(request["params"]["tab_id"], json!("w1:t1"));
    assert!(request["params"].get("workspace_id").is_none());
    Ok(())
}

#[test]
fn layout_apply_omits_both_ids_when_unset() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![layout_response("layout_apply")]);

    session.client().layout_apply(
        session.name(),
        &LayoutApplyParams {
            workspace_id: None,
            tab_id: None,
            tab_label: None,
            focus: false,
            root: LayoutNode::Pane {
                pane: LayoutPane::default(),
            },
        },
    )?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(request["method"], json!("layout.apply"));
    assert!(request["params"].get("tab_id").is_none());
    assert!(request["params"].get("workspace_id").is_none());
    assert!(request["params"].get("tab_label").is_none());
    Ok(())
}

#[test]
fn layout_export_parses_layout() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![layout_response("layout_export")]);

    let layout = session.client().layout_export(session.name(), "t1")?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(layout.workspace_id, "w1");
    assert_eq!(layout.tab_id, "t1");
    assert_eq!(request["method"], json!("layout.export"));
    assert_eq!(request["params"]["tab_id"], json!("t1"));
    Ok(())
}

#[test]
fn pane_focus_parses_pane_info() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![pane_info_response()]);

    let pane = session.client().pane_focus(session.name(), "p1")?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(pane.pane_id, "p1");
    assert_eq!(pane.cwd.as_deref(), Some("/tmp/a"));
    assert_eq!(pane.label.as_deref(), Some("editor"));
    assert_eq!(request["method"], json!("pane.focus"));
    assert_eq!(request["params"]["pane_id"], json!("p1"));
    Ok(())
}

#[test]
fn pane_zoom_sends_mode_and_parses_result() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![pane_zoom_response()]);

    let zoom = session
        .client()
        .pane_zoom(session.name(), "p1", PaneZoomMode::On)?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(zoom.pane_id, "p1");
    assert!(zoom.zoomed);
    assert_eq!(request["method"], json!("pane.zoom"));
    assert_eq!(request["params"]["pane_id"], json!("p1"));
    assert_eq!(request["params"]["mode"], json!("on"));
    Ok(())
}

#[test]
fn pane_send_input_sends_text_and_keys() -> Result<()> {
    let session = TestSession::new()?;
    let listener = session.bind()?;
    let handle = spawn_canned_server(listener, vec![ok_response()]);

    session.client().pane_send_input(
        session.name(),
        &PaneSendInputParams {
            pane_id: "p1".to_string(),
            text: "cargo test".to_string(),
            keys: vec!["Enter".to_string()],
        },
    )?;

    let requests = join_thread(handle)?;
    let request = parse_request(&requests[0])?;

    assert_eq!(request["method"], json!("pane.send_input"));
    assert_eq!(request["params"]["pane_id"], json!("p1"));
    assert_eq!(request["params"]["text"], json!("cargo test"));
    assert_eq!(request["params"]["keys"], json!(["Enter"]));
    Ok(())
}
