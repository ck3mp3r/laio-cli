use std::collections::HashMap;

use crate::common::config::{FlexDirection, Session};
use crate::muxer::rex::export::{local_path, session_from_view};
use crate::muxer::rex::model::{BlockRef, LayerView, SessionView, ViewNode, ViewSplit, WindowView};

type TestResult<T = ()> = core::result::Result<T, Box<dyn std::error::Error>>;

const TILED: &str = "tiled";
const FLOATING: &str = "floating";

fn leaf(block_id: &str) -> ViewNode {
    ViewNode {
        rect: Vec::new(),
        block_id: Some(block_id.to_string()),
        split: None,
    }
}

fn split(direction: &str, ratio: f64, before: ViewNode, after: ViewNode) -> ViewNode {
    ViewNode {
        rect: Vec::new(),
        block_id: None,
        split: Some(Box::new(ViewSplit {
            split_id: None,
            direction: direction.to_string(),
            ratio: Some(ratio),
            before: Some(before),
            after: Some(after),
        })),
    }
}

fn layer(kind: &str, layout: ViewNode) -> LayerView {
    LayerView {
        layer_id: None,
        kind: kind.to_string(),
        bounds: Vec::new(),
        layout: Some(layout),
        blocks: None,
    }
}

fn tiled(layout: ViewNode, labels: &[(&str, &str)]) -> LayerView {
    LayerView {
        layer_id: None,
        kind: TILED.to_string(),
        bounds: Vec::new(),
        layout: Some(layout),
        blocks: Some(
            labels
                .iter()
                .map(|(block_id, label)| BlockRef {
                    block_id: (*block_id).to_string(),
                    creator_name: None,
                    flavor: None,
                    label: Some((*label).to_string()),
                    owner: None,
                })
                .collect(),
        ),
    }
}

fn window(
    window_id: &str,
    label: &str,
    focused: Option<&str>,
    zoomed: Option<&str>,
    layers: Vec<LayerView>,
) -> WindowView {
    WindowView {
        window_id: Some(window_id.to_string()),
        label: Some(label.to_string()),
        focused_block_id: focused.map(str::to_string),
        zoomed_block_id: zoomed.map(str::to_string),
        layers: Some(layers),
    }
}

fn view(label: &str, active: &str, windows: Vec<WindowView>) -> SessionView {
    SessionView {
        revision: 1,
        session_id: Some("session:1".to_string()),
        label: Some(label.to_string()),
        active_window_id: Some(active.to_string()),
        windows: Some(windows),
    }
}

fn pwds(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(block_id, pwd)| ((*block_id).to_string(), (*pwd).to_string()))
        .collect()
}

fn flexes(session: &Session, window_index: usize) -> Vec<usize> {
    session.windows[window_index]
        .panes
        .iter()
        .map(|pane| pane.flex)
        .collect()
}

#[test]
fn horizontal_ratios_round_trip_to_the_exact_flex_weights() -> TestResult {
    let layout = split(
        "horizontal",
        0.5,
        leaf("b1"),
        split("horizontal", 0.5, leaf("b2"), leaf("b3")),
    );
    let view = view(
        "laio-other",
        "w1",
        vec![window(
            "w1",
            "main",
            None,
            None,
            vec![tiled(layout, &[("b1", "editor"), ("b3", "logs")])],
        )],
    );

    let session = session_from_view(
        &view,
        &pwds(&[
            ("b1", "/opt/project/a"),
            ("b2", "/opt/project/b"),
            ("b3", "/opt/project/c"),
        ]),
        Some("work"),
    )?;

    let window = &session.windows[0];
    assert_eq!(window.flex_direction, FlexDirection::Row);
    assert_eq!(flexes(&session, 0), [2, 1, 1]);
    assert_eq!(window.panes[0].name.as_deref(), Some("editor"));
    assert_eq!(window.panes[1].name, None);
    assert_eq!(window.panes[2].name.as_deref(), Some("logs"));
    assert_eq!(session.name, "work");
    Ok(())
}

#[test]
fn a_vertical_root_split_sets_the_window_direction_to_column() -> TestResult {
    let layout = split("vertical", 0.5, leaf("b1"), leaf("b2"));
    let view = view(
        "laio-work",
        "w1",
        vec![window("w1", "main", None, None, vec![tiled(layout, &[])])],
    );

    let session = session_from_view(
        &view,
        &pwds(&[("b1", "/opt/project/a"), ("b2", "/opt/project/b")]),
        Some("work"),
    )?;

    assert_eq!(session.windows[0].flex_direction, FlexDirection::Column);
    assert_eq!(flexes(&session, 0), [1, 1]);
    Ok(())
}

#[test]
fn a_vertical_split_inside_a_horizontal_window_becomes_a_column_container() -> TestResult {
    let layout = split(
        "horizontal",
        0.5,
        leaf("b1"),
        split("vertical", 0.5, leaf("b2"), leaf("b3")),
    );
    let view = view(
        "laio-work",
        "w1",
        vec![window("w1", "main", None, None, vec![tiled(layout, &[])])],
    );

    let session = session_from_view(
        &view,
        &pwds(&[
            ("b1", "/opt/project/a"),
            ("b2", "/opt/project/b"),
            ("b3", "/opt/project/c"),
        ]),
        Some("work"),
    )?;

    let window = &session.windows[0];
    assert_eq!(window.flex_direction, FlexDirection::Row);
    assert_eq!(window.panes.len(), 2);
    assert_eq!(window.panes[0].flex, 2);
    assert_eq!(window.panes[1].flex, 2);
    assert_eq!(window.panes[1].flex_direction, FlexDirection::Column);
    assert_eq!(window.panes[1].panes.len(), 2);
    assert_eq!(window.panes[1].panes[0].path, "b");
    Ok(())
}

#[test]
fn pane_paths_are_relative_to_the_longest_common_prefix() -> TestResult {
    assert_eq!(local_path("file://host/Users/x"), "/Users/x");
    assert_eq!(local_path("file:///Users/x"), "/Users/x");
    assert_eq!(local_path("/Users/x"), "/Users/x");

    let layout = split("horizontal", 0.5, leaf("b1"), leaf("b2"));
    let view = view(
        "laio-work",
        "w1",
        vec![window("w1", "main", None, None, vec![tiled(layout, &[])])],
    );
    let x = local_path("file://host/Users/x");
    let y = local_path("file://host/Users/y");

    let session = session_from_view(&view, &pwds(&[("b1", x), ("b2", y)]), Some("work"))?;

    assert_eq!(session.path, "/Users");
    assert_eq!(session.windows[0].panes[0].path, "x");
    assert_eq!(session.windows[0].panes[1].path, "y");
    Ok(())
}

#[test]
fn focused_and_zoomed_blocks_mark_their_panes() -> TestResult {
    let layout = split("horizontal", 0.5, leaf("b1"), leaf("b2"));
    let view = view(
        "laio-work",
        "w1",
        vec![window(
            "w1",
            "main",
            Some("b2"),
            Some("b1"),
            vec![tiled(layout, &[])],
        )],
    );

    let session = session_from_view(
        &view,
        &pwds(&[("b1", "/opt/project/a"), ("b2", "/opt/project/b")]),
        Some("work"),
    )?;

    let panes = &session.windows[0].panes;
    assert!(panes[0].zoom);
    assert!(!panes[0].focus);
    assert!(!panes[1].zoom);
    assert!(panes[1].focus);
    Ok(())
}

#[test]
fn only_the_active_window_is_focused() -> TestResult {
    let layout = || split("horizontal", 0.5, leaf("b1"), leaf("b2"));
    let view = view(
        "laio-work",
        "w2",
        vec![
            window("w1", "first", None, None, vec![tiled(layout(), &[])]),
            window("w2", "second", None, None, vec![tiled(layout(), &[])]),
        ],
    );

    let session = session_from_view(
        &view,
        &pwds(&[("b1", "/opt/project/a"), ("b2", "/opt/project/b")]),
        Some("work"),
    )?;

    assert_eq!(session.windows.len(), 2);
    assert!(!session.windows[0].focus);
    assert!(session.windows[1].focus);
    Ok(())
}

#[test]
fn floating_layers_are_ignored() -> TestResult {
    let tiled_layout = split("horizontal", 0.5, leaf("b1"), leaf("b2"));
    let view = view(
        "laio-work",
        "w1",
        vec![window(
            "w1",
            "main",
            None,
            None,
            vec![
                tiled(tiled_layout, &[]),
                layer(FLOATING, leaf("floating-block")),
            ],
        )],
    );

    let session = session_from_view(
        &view,
        &pwds(&[
            ("b1", "/opt/project/a"),
            ("b2", "/opt/project/b"),
            ("floating-block", "/elsewhere"),
        ]),
        Some("work"),
    )?;

    assert_eq!(session.windows[0].panes.len(), 2);
    assert_eq!(session.path, "/opt/project");
    Ok(())
}

#[test]
fn a_missing_record_falls_back_to_the_label_without_the_laio_prefix() -> TestResult {
    let layout = leaf("b1");
    let view = view(
        "laio-work",
        "w1",
        vec![window("w1", "main", None, None, vec![tiled(layout, &[])])],
    );

    let session = session_from_view(&view, &pwds(&[("b1", "/opt/project")]), None)?;

    assert_eq!(session.name, "work");
    assert_eq!(session.path, "/opt/project");
    assert_eq!(session.windows[0].panes[0].path, ".");
    Ok(())
}

#[test]
fn the_export_carries_no_unrecoverable_settings() -> TestResult {
    let layout = split("horizontal", 0.5, leaf("b1"), leaf("b2"));
    let view = view(
        "laio-work",
        "w1",
        vec![window("w1", "main", None, None, vec![tiled(layout, &[])])],
    );

    let session = session_from_view(
        &view,
        &pwds(&[("b1", "/opt/project/a"), ("b2", "/opt/project/b")]),
        Some("work"),
    )?;

    assert!(session.startup.is_empty());
    assert!(session.startup_script.is_none());
    assert!(session.shutdown.is_empty());
    assert!(session.shutdown_script.is_none());
    assert!(session.env.is_empty());
    assert!(session.shell.is_none());
    assert!(session.pane_cmd_delay.is_none());
    for pane in &session.windows[0].panes {
        assert!(pane.commands.is_empty());
        assert!(pane.style.is_none());
        assert!(pane.script.is_none());
    }
    Ok(())
}
