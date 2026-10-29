use miette::{IntoDiagnostic, Result, miette};

use crate::common::config::{Command, FlexDirection, Script};

use super::convert::{PaneOp, TabState, bind_pane_ops, build_plans, window_from_layout};
use super::model::{LayoutDescription, LayoutNode, LayoutPane, SplitDirection};

use super::session_test_support::{export_layout, export_pane};
use super::support_models::*;

#[test]
fn build_plans_uses_sequential_ratios() -> Result<()> {
    let session = test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![
            test_pane(1, None, "."),
            test_pane(1, None, "."),
            test_pane(1, None, "."),
        ],
    )]);

    let plans = build_plans(&session)?;
    let root = &plans[0].root;

    let LayoutNode::Split { ratio, second, .. } = root else {
        return Err(miette!("root should be a split, got {root:?}"));
    };
    assert!(
        (*ratio - 1.0 / 3.0).abs() < 1e-6,
        "first ratio should be 1/3, got {ratio}"
    );

    let LayoutNode::Split { ratio, .. } = second.as_ref() else {
        return Err(miette!("second node should be a split, got {second:?}"));
    };
    assert!(
        (*ratio - 0.5).abs() < 1e-6,
        "second ratio should be 1/2, got {ratio}"
    );
    Ok(())
}

#[test]
fn build_plans_clamps_ratios_outside_the_valid_range() -> Result<()> {
    let session = test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![test_pane(1, None, "."), test_pane(10, None, ".")],
    )]);

    let plans = build_plans(&session)?;
    let root = &plans[0].root;

    let LayoutNode::Split { ratio, .. } = root else {
        return Err(miette!("root should be a split, got {root:?}"));
    };
    assert_eq!(*ratio, 0.1);
    Ok(())
}

#[test]
fn build_plans_rejects_more_than_twenty_four_panes() -> Result<()> {
    let panes = (0..25).map(|_| test_pane(1, None, ".")).collect();
    let error = build_plans(&test_session(vec![test_window(
        "wide",
        FlexDirection::Row,
        panes,
    )]))
    .err()
    .ok_or_else(|| miette!("25 panes should return Err"))?;

    let report = error.to_string();
    assert!(
        report.contains("wide"),
        "report should name the window: {report}"
    );
    assert!(
        report.contains("25"),
        "report should name the pane count: {report}"
    );
    assert!(
        report.contains("24"),
        "report should name the pane limit: {report}"
    );
    Ok(())
}

#[test]
fn build_plans_rejects_split_depth_above_sixteen() -> Result<()> {
    let panes = (0..17).map(|_| test_pane(1, None, ".")).collect();
    let error = build_plans(&test_session(vec![test_window(
        "narrow",
        FlexDirection::Row,
        panes,
    )]))
    .err()
    .ok_or_else(|| miette!("17 levels should return Err"))?;

    let report = error.to_string();
    assert!(
        report.contains("narrow"),
        "report should name the window: {report}"
    );
    assert!(
        report.contains("depth"),
        "report should name the depth limit: {report}"
    );
    assert!(
        report.contains("16"),
        "report should name the depth limit value: {report}"
    );
    Ok(())
}

#[test]
fn build_plans_maps_flex_direction_to_split_direction() -> Result<()> {
    let row = build_plans(&test_session(vec![test_window(
        "row",
        FlexDirection::Row,
        vec![test_pane(1, None, "."), test_pane(1, None, ".")],
    )]))?;
    let column = build_plans(&test_session(vec![test_window(
        "column",
        FlexDirection::Column,
        vec![test_pane(1, None, "."), test_pane(1, None, ".")],
    )]))?;

    let LayoutNode::Split { direction, .. } = &row[0].root else {
        return Err(miette!("row root should be a split"));
    };
    assert_eq!(*direction, SplitDirection::Right);

    let LayoutNode::Split { direction, .. } = &column[0].root else {
        return Err(miette!("column root should be a split"));
    };
    assert_eq!(*direction, SplitDirection::Down);
    Ok(())
}

#[test]
fn build_plans_resolves_pane_cwd_and_label() -> Result<()> {
    let mut window = test_window(
        "main",
        FlexDirection::Row,
        vec![test_pane(1, Some("editor"), "."), test_pane(1, None, "src")],
    );
    window.path = Some("sub".to_string());

    let plans = build_plans(&test_session(vec![window]))?;
    let root = &plans[0].root;

    let LayoutNode::Split { first, second, .. } = root else {
        return Err(miette!("root should be a split, got {root:?}"));
    };

    let LayoutNode::Pane { pane } = first.as_ref() else {
        return Err(miette!("first node should be a pane"));
    };
    assert_eq!(pane.cwd.as_deref(), Some("/tmp/project/sub"));
    assert_eq!(pane.label.as_deref(), Some("editor"));

    let LayoutNode::Pane { pane } = second.as_ref() else {
        return Err(miette!("second node should be a pane"));
    };
    assert_eq!(pane.cwd.as_deref(), Some("/tmp/project/sub/src"));
    assert_eq!(pane.label, None);

    assert_eq!(plans[0].leaves[0].name.as_deref(), Some("editor"));
    assert_eq!(plans[0].leaves[1].name, None);
    Ok(())
}

#[test]
fn build_plans_omits_the_command_field() -> Result<()> {
    let mut pane = test_pane(1, None, ".");
    pane.commands = vec![Command::from_string("echo hi")];

    let plans = build_plans(&test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![pane],
    )]))?;

    let json = serde_json::to_value(&plans[0].root).into_diagnostic()?;
    assert!(
        !contains_key(&json, "command"),
        "pane nodes must not carry a command field: {json}"
    );
    assert_eq!(plans[0].leaves[0].commands, vec!["echo hi".to_string()]);
    Ok(())
}

#[test]
fn build_plans_includes_script_command() -> Result<()> {
    let script: Script = noyalib::compat::serde_yaml::from_str("echo hi").into_diagnostic()?;
    let mut pane = test_pane(1, None, ".");
    pane.script = Some(script);

    let plans = build_plans(&test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![pane],
    )]))?;

    let commands = &plans[0].leaves[0].commands;
    assert_eq!(commands.len(), 1);
    assert!(
        commands[0].contains("laio-"),
        "script command should point at the materialized script: {commands:?}"
    );
    Ok(())
}

#[test]
fn build_plans_rejects_multiple_focused_windows() -> Result<()> {
    let mut first = test_window("first", FlexDirection::Row, vec![test_pane(1, None, ".")]);
    first.focus = true;
    let mut second = test_window("second", FlexDirection::Row, vec![test_pane(1, None, ".")]);
    second.focus = true;

    let error = build_plans(&test_session(vec![first, second]))
        .err()
        .ok_or_else(|| miette!("two focused windows should return Err"))?;

    assert!(
        error.to_string().contains("focus"),
        "report should name the focus conflict: {error}"
    );
    Ok(())
}

#[test]
fn build_plans_sets_focus_tab_for_the_focused_window() -> Result<()> {
    let mut focused = test_window("focused", FlexDirection::Row, vec![test_pane(1, None, ".")]);
    focused.focus = true;
    let other = test_window("other", FlexDirection::Row, vec![test_pane(1, None, ".")]);

    let plans = build_plans(&test_session(vec![focused, other]))?;

    assert_eq!(plans.len(), 2);
    assert_eq!(plans[0].tab_label, "focused");
    assert!(plans[0].focus_tab);
    assert_eq!(plans[1].tab_label, "other");
    assert!(!plans[1].focus_tab);
    Ok(())
}

#[test]
fn build_plans_handles_window_without_panes() -> Result<()> {
    let plans = build_plans(&test_session(vec![test_window(
        "empty",
        FlexDirection::Row,
        Vec::new(),
    )]))?;

    assert_eq!(plans[0].leaves.len(), 1);
    let LayoutNode::Pane { pane } = &plans[0].root else {
        return Err(miette!("empty window root should be a pane"));
    };
    assert_eq!(pane.cwd.as_deref(), Some("/tmp/project"));
    Ok(())
}

#[test]
fn bind_pane_ops_emits_ops_in_leaf_order() -> Result<()> {
    let mut first = test_pane(1, None, ".");
    first.zoom = true;
    first.focus = true;
    first.commands = vec![
        Command::from_string("echo one"),
        Command::from_string("echo two"),
    ];

    let mut third = test_pane(1, None, ".");
    third.commands = vec![Command::from_string("echo three")];

    let plans = build_plans(&test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![first, test_pane(1, None, "."), third],
    )]))?;

    let layout = LayoutDescription {
        workspace_id: "w1".to_string(),
        tab_id: "t1".to_string(),
        zoomed: false,
        focused_pane_id: "p1".to_string(),
        root: LayoutNode::Split {
            direction: SplitDirection::Right,
            ratio: 1.0 / 3.0,
            first: Box::new(pane_node("p1")),
            second: Box::new(LayoutNode::Split {
                direction: SplitDirection::Right,
                ratio: 0.5,
                first: Box::new(pane_node("p2")),
                second: Box::new(pane_node("p3")),
            }),
        },
    };

    let ops = bind_pane_ops(&plans[0], &layout)?;
    let enter = vec!["Enter".to_string()];

    assert_eq!(
        ops,
        vec![
            PaneOp::Zoom("p1".to_string()),
            PaneOp::Focus("p1".to_string()),
            PaneOp::SendInput {
                pane_id: "p1".to_string(),
                text: "echo one".to_string(),
                keys: enter.clone(),
            },
            PaneOp::SendInput {
                pane_id: "p1".to_string(),
                text: "echo two".to_string(),
                keys: enter.clone(),
            },
            PaneOp::SendInput {
                pane_id: "p3".to_string(),
                text: "echo three".to_string(),
                keys: enter,
            },
        ]
    );
    Ok(())
}

#[test]
fn bind_pane_ops_rejects_mismatched_pane_count() -> Result<()> {
    let plans = build_plans(&test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![test_pane(1, None, "."), test_pane(1, None, ".")],
    )]))?;

    let layout = LayoutDescription {
        workspace_id: "w1".to_string(),
        tab_id: "t1".to_string(),
        zoomed: false,
        focused_pane_id: "p1".to_string(),
        root: pane_node("p1"),
    };

    let error = bind_pane_ops(&plans[0], &layout)
        .err()
        .ok_or_else(|| miette!("mismatched pane count should return Err"))?;

    assert!(
        error.to_string().contains("main"),
        "report should name the tab: {error}"
    );
    Ok(())
}

#[test]
fn bind_pane_ops_rejects_layout_pane_without_id() -> Result<()> {
    let plans = build_plans(&test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![test_pane(1, None, ".")],
    )]))?;

    let layout = LayoutDescription {
        workspace_id: "w1".to_string(),
        tab_id: "t1".to_string(),
        zoomed: false,
        focused_pane_id: "p1".to_string(),
        root: LayoutNode::Pane {
            pane: LayoutPane::default(),
        },
    };

    let error = bind_pane_ops(&plans[0], &layout)
        .err()
        .ok_or_else(|| miette!("missing pane_id should return Err"))?;

    assert!(
        error.to_string().contains("pane_id"),
        "report should name the missing pane_id: {error}"
    );
    Ok(())
}

#[test]
fn window_from_layout_converts_split_ratios_to_flex_weights() -> Result<()> {
    let state = TabState {
        zoomed: false,
        focused_pane_id: "p1".to_string(),
        focused: false,
    };

    let half = window_from_layout(
        &export_layout(
            "t1",
            false,
            "p1",
            LayoutNode::Split {
                direction: SplitDirection::Right,
                ratio: 0.5,
                first: Box::new(export_pane("p1", None, "/tmp/project/a", None)),
                second: Box::new(export_pane("p2", None, "/tmp/project/b", None)),
            },
        ),
        "main",
        "/tmp/project",
        &state,
    );
    assert_eq!(half.panes[0].flex, 1);
    assert_eq!(half.panes[1].flex, 1);

    let quarter = window_from_layout(
        &export_layout(
            "t1",
            false,
            "p1",
            LayoutNode::Split {
                direction: SplitDirection::Down,
                ratio: 0.25,
                first: Box::new(export_pane("p1", None, "/tmp/project/a", None)),
                second: Box::new(export_pane("p2", None, "/tmp/project/b", None)),
            },
        ),
        "main",
        "/tmp/project",
        &state,
    );
    assert_eq!(quarter.flex_direction, FlexDirection::Column);
    assert_eq!(quarter.panes[0].flex, 1);
    assert_eq!(quarter.panes[1].flex, 3);
    Ok(())
}

#[test]
fn window_from_layout_falls_back_to_dot_for_the_session_path() -> Result<()> {
    let state = TabState {
        zoomed: false,
        focused_pane_id: "p1".to_string(),
        focused: false,
    };

    let window = window_from_layout(
        &export_layout(
            "t1",
            false,
            "p1",
            export_pane("p1", None, "/tmp/project", None),
        ),
        "main",
        "/tmp/project",
        &state,
    );

    assert_eq!(window.panes.len(), 1);
    assert_eq!(window.panes[0].path, ".");
    Ok(())
}
