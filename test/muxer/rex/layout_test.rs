use std::collections::HashMap;

use crate::common::config::{Command, FlexDirection, Pane, Script, Session, Window};
use crate::muxer::rex::model::{BlockNode, CreateRequest, LayoutNode, SplitNode};
use crate::muxer::rex::{LeafPlan, build_create_request};

type TestResult<T = ()> = core::result::Result<T, Box<dyn std::error::Error>>;

fn test_session(windows: Vec<Window>) -> Session {
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

fn test_window(name: &str, flex_direction: FlexDirection, panes: Vec<Pane>) -> Window {
    Window {
        name: name.to_string(),
        path: None,
        flex_direction,
        panes,
        focus: false,
    }
}

fn test_pane(flex: usize, name: Option<&str>, path: &str) -> Pane {
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

fn split_of<'a>(node: &'a LayoutNode, context: &str) -> TestResult<&'a SplitNode> {
    node.split
        .as_ref()
        .ok_or_else(|| format!("{context} should be a split, got {node:?}").into())
}

fn block_of<'a>(node: &'a LayoutNode, context: &str) -> TestResult<&'a BlockNode> {
    node.block
        .as_ref()
        .ok_or_else(|| format!("{context} should be a block, got {node:?}").into())
}

fn collect_blocks<'a>(node: &'a LayoutNode, blocks: &mut Vec<&'a BlockNode>) {
    if let Some(block) = &node.block {
        blocks.push(block);
    }

    if let Some(split) = &node.split {
        collect_blocks(&split.before, blocks);
        collect_blocks(&split.after, blocks);
    }
}

fn first_window(request: &CreateRequest) -> TestResult<&LayoutNode> {
    request
        .initial_windows
        .as_ref()
        .and_then(|windows| windows.first())
        .map(|window| &window.layout)
        .ok_or_else(|| "request should carry one initial window".into())
}

fn positions(leaves: &[LeafPlan]) -> Vec<(usize, usize)> {
    leaves
        .iter()
        .map(|leaf| (leaf.window_index, leaf.block_index))
        .collect()
}

#[test]
fn build_create_request_splits_flex_two_one_one_into_nested_halves() -> TestResult {
    let session = test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![
            test_pane(2, None, "."),
            test_pane(1, None, "."),
            test_pane(1, None, "."),
        ],
    )]);

    let (request, leaves) = build_create_request(&session)?;

    assert_eq!(request.label, None);
    let window = request
        .initial_windows
        .as_ref()
        .and_then(|windows| windows.first())
        .ok_or("request should carry one initial window")?;
    assert_eq!(window.window_label.as_deref(), Some("main"));

    let root = split_of(&window.layout, "window root")?;
    assert_eq!(root.direction, "horizontal");
    assert_eq!(root.ratio, Some(0.5));

    let after = split_of(&root.after, "inner split")?;
    assert_eq!(after.direction, "horizontal");
    assert_eq!(after.ratio, Some(0.5));

    assert_eq!(leaves.len(), 3);
    Ok(())
}

#[test]
fn build_create_request_numbers_leaves_per_window() -> TestResult {
    let session = test_session(vec![
        test_window(
            "first",
            FlexDirection::Row,
            vec![
                test_pane(1, None, "."),
                test_pane(1, None, "."),
                test_pane(1, None, "."),
            ],
        ),
        test_window(
            "second",
            FlexDirection::Row,
            vec![test_pane(1, None, "."), test_pane(1, None, ".")],
        ),
    ]);

    let (_, leaves) = build_create_request(&session)?;

    assert_eq!(positions(&leaves), [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1)]);
    Ok(())
}

#[test]
fn build_create_request_maps_column_windows_to_vertical_splits() -> TestResult {
    let session = test_session(vec![test_window(
        "stack",
        FlexDirection::Column,
        vec![test_pane(1, None, "."), test_pane(1, None, ".")],
    )]);

    let (request, _) = build_create_request(&session)?;

    let root = split_of(first_window(&request)?, "window root")?;
    assert_eq!(root.direction, "vertical");
    Ok(())
}

#[test]
fn build_create_request_emits_one_block_for_a_window_without_panes() -> TestResult {
    let mut window = test_window("empty", FlexDirection::Row, Vec::new());
    window.path = Some("sub".to_string());

    let (request, leaves) = build_create_request(&test_session(vec![window]))?;

    let block = block_of(first_window(&request)?, "window root")?;
    assert_eq!(block.flavor, "com.superlogical.terminal.shell");
    assert_eq!(
        block
            .options
            .as_ref()
            .and_then(|options| options.cwd.as_deref()),
        Some("/tmp/project/sub")
    );

    assert_eq!(leaves.len(), 1);
    assert_eq!(leaves[0].block_index, 0);
    assert_eq!(leaves[0].cwd, "/tmp/project/sub");
    Ok(())
}

#[test]
fn build_create_request_clamps_ratios_outside_the_valid_range() -> TestResult {
    let low = test_session(vec![test_window(
        "low",
        FlexDirection::Row,
        vec![test_pane(1, None, "."), test_pane(10, None, ".")],
    )]);
    let high = test_session(vec![test_window(
        "high",
        FlexDirection::Row,
        vec![test_pane(10, None, "."), test_pane(1, None, ".")],
    )]);

    let (low_request, _) = build_create_request(&low)?;
    let (high_request, _) = build_create_request(&high)?;

    let low_root = split_of(first_window(&low_request)?, "low window root")?;
    assert_eq!(low_root.ratio, Some(0.1));
    block_of(&low_root.before, "low first child")?;
    block_of(&low_root.after, "low second child")?;

    let high_root = split_of(first_window(&high_request)?, "high window root")?;
    assert_eq!(high_root.ratio, Some(0.9));
    block_of(&high_root.before, "high first child")?;
    block_of(&high_root.after, "high second child")?;
    Ok(())
}

#[test]
fn build_create_request_sets_shell_on_every_block() -> TestResult {
    let mut session = test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![
            test_pane(2, None, "."),
            test_pane(1, None, "."),
            test_pane(1, None, "."),
        ],
    )]);
    session.shell = Some("/bin/zsh".to_string());

    let (request, _) = build_create_request(&session)?;

    let mut blocks = Vec::new();
    collect_blocks(first_window(&request)?, &mut blocks);
    assert_eq!(blocks.len(), 3);

    for block in blocks {
        assert_eq!(
            block
                .options
                .as_ref()
                .and_then(|options| options.shell.as_deref()),
            Some("/bin/zsh"),
            "{block:?}"
        );
    }
    Ok(())
}

#[test]
fn build_create_request_maps_pane_name_to_label_and_leaf_name() -> TestResult {
    let session = test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![test_pane(1, Some("build"), ".")],
    )]);

    let (request, leaves) = build_create_request(&session)?;

    let block = block_of(first_window(&request)?, "window root")?;
    assert_eq!(block.label.as_deref(), Some("build"));
    assert_eq!(leaves[0].name.as_deref(), Some("build"));
    Ok(())
}

#[test]
fn build_create_request_orders_leaf_commands_before_the_script() -> TestResult {
    let script: Script = noyalib::compat::serde_yaml::from_str("echo hi")?;
    let mut pane = test_pane(1, None, ".");
    pane.commands = vec![
        Command::from_string("echo one"),
        Command::from_string("echo two"),
    ];
    pane.script = Some(script);

    let session = test_session(vec![test_window("main", FlexDirection::Row, vec![pane])]);
    let (_, leaves) = build_create_request(&session)?;

    let commands = &leaves[0].commands;
    assert_eq!(commands.len(), 3, "{commands:?}");
    assert_eq!(commands[0], "echo one");
    assert_eq!(commands[1], "echo two");
    assert!(
        commands[2].contains("laio-"),
        "script command should point at the materialized script: {commands:?}"
    );
    Ok(())
}

#[test]
fn build_create_request_switches_direction_for_a_nested_pane_group() -> TestResult {
    let mut group = test_pane(1, None, ".");
    group.flex_direction = FlexDirection::Column;
    group.panes = vec![test_pane(1, None, "a"), test_pane(1, None, "b")];

    let session = test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![group, test_pane(1, None, "c")],
    )]);

    let (request, leaves) = build_create_request(&session)?;

    let root = split_of(first_window(&request)?, "window root")?;
    assert_eq!(root.direction, "horizontal");

    let group_split = split_of(&root.before, "nested pane group")?;
    assert_eq!(group_split.direction, "vertical");

    let block_indexes: Vec<usize> = leaves.iter().map(|leaf| leaf.block_index).collect();
    assert_eq!(block_indexes, [0, 1, 2]);
    let cwds: Vec<&str> = leaves.iter().map(|leaf| leaf.cwd.as_str()).collect();
    assert_eq!(cwds, ["/tmp/project/a", "/tmp/project/b", "/tmp/project/c"]);
    Ok(())
}

#[test]
fn build_create_request_carries_zoom_and_focus() -> TestResult {
    let mut zoomed = test_pane(1, None, ".");
    zoomed.zoom = true;
    let mut focused = test_pane(1, None, ".");
    focused.focus = true;

    let session = test_session(vec![test_window(
        "main",
        FlexDirection::Row,
        vec![zoomed, focused],
    )]);

    let (_, leaves) = build_create_request(&session)?;

    assert!(leaves[0].zoom);
    assert!(!leaves[0].focus);
    assert!(!leaves[1].zoom);
    assert!(leaves[1].focus);
    Ok(())
}

#[test]
fn build_create_request_rejects_a_session_without_windows() -> TestResult {
    let error = build_create_request(&test_session(Vec::new()))
        .err()
        .ok_or("a session without windows should fail")?;
    let message = error.to_string();

    assert!(message.contains("test"), "{message}");
    assert!(message.contains("no windows"), "{message}");
    Ok(())
}

#[test]
fn build_create_request_rejects_a_window_above_the_block_limit() -> TestResult {
    let panes = (0..25).map(|_| test_pane(1, None, ".")).collect();
    let error = build_create_request(&test_session(vec![test_window(
        "wide",
        FlexDirection::Row,
        panes,
    )]))
    .err()
    .ok_or("25 blocks should fail")?;
    let message = error.to_string();

    assert!(message.contains("wide"), "{message}");
    assert!(message.contains("25"), "{message}");
    assert!(message.contains("24"), "{message}");
    Ok(())
}

#[test]
fn build_create_request_rejects_a_window_above_the_depth_limit() -> TestResult {
    let mut nested = test_pane(1, None, "leaf");
    for _ in 0..15 {
        let mut group = test_pane(1, None, ".");
        group.panes = vec![test_pane(1, None, "leaf"), nested];
        nested = group;
    }

    let error = build_create_request(&test_session(vec![test_window(
        "deep",
        FlexDirection::Row,
        vec![test_pane(1, None, "leaf"), nested],
    )]))
    .err()
    .ok_or("depth 17 should fail")?;
    let message = error.to_string();

    assert!(message.contains("deep"), "{message}");
    assert!(message.contains("depth"), "{message}");
    assert!(message.contains("16"), "{message}");
    Ok(())
}
