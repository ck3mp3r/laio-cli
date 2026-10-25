use miette::{Result, bail, miette};

use crate::common::config::util::{gcd_vec, round};
use crate::common::config::{Command, FlexDirection, Pane, Session, Window};
use crate::common::path::{relative_path, sanitize_path};

use super::model::{LayoutDescription, LayoutNode, LayoutPane, SplitDirection};

const MAX_PANES_PER_TAB: usize = 24;
const MAX_SPLIT_DEPTH: usize = 16;
const MIN_SPLIT_RATIO: f32 = 0.1;
const MAX_SPLIT_RATIO: f32 = 0.9;
const SEND_INPUT_KEYS: [&str; 1] = ["Enter"];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WindowPlan {
    pub(crate) tab_label: String,
    pub(crate) focus_tab: bool,
    pub(crate) root: LayoutNode,
    pub(crate) leaves: Vec<LeafPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeafPlan {
    pub(crate) name: Option<String>,
    pub(crate) zoom: bool,
    pub(crate) focus: bool,
    pub(crate) commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PaneOp {
    Zoom(String),
    Focus(String),
    SendInput {
        pane_id: String,
        text: String,
        keys: Vec<String>,
    },
}

pub(crate) fn build_plans(session: &Session) -> Result<Vec<WindowPlan>> {
    let focused_windows = session.windows.iter().filter(|window| window.focus).count();
    if focused_windows > 1 {
        bail!(
            "Session '{}' has more than one window with focus enabled",
            session.name
        );
    }

    session
        .windows
        .iter()
        .map(|window| build_window_plan(session, window))
        .collect()
}

pub(crate) fn bind_pane_ops(plan: &WindowPlan, layout: &LayoutDescription) -> Result<Vec<PaneOp>> {
    let mut pane_ids = Vec::new();
    collect_pane_ids(&layout.root, &mut pane_ids)?;

    if pane_ids.len() != plan.leaves.len() {
        bail!(
            "Applied layout for tab '{}' has {} panes but the plan has {}",
            plan.tab_label,
            pane_ids.len(),
            plan.leaves.len()
        );
    }

    let mut ops = Vec::new();
    for (leaf, pane_id) in plan.leaves.iter().zip(pane_ids) {
        if leaf.zoom {
            ops.push(PaneOp::Zoom(pane_id.clone()));
        }
        if leaf.focus {
            ops.push(PaneOp::Focus(pane_id.clone()));
        }
        for command in &leaf.commands {
            ops.push(PaneOp::SendInput {
                pane_id: pane_id.clone(),
                text: command.clone(),
                keys: SEND_INPUT_KEYS.iter().map(|key| key.to_string()).collect(),
            });
        }
    }

    Ok(ops)
}

fn build_window_plan(session: &Session, window: &Window) -> Result<WindowPlan> {
    let window_path = window.effective_path(&session.path);
    let mut leaves = Vec::new();

    let root = if window.panes.is_empty() {
        leaves.push(LeafPlan {
            name: None,
            zoom: false,
            focus: false,
            commands: Vec::new(),
        });

        LayoutNode::Pane {
            pane: LayoutPane {
                cwd: Some(window_path.clone()),
                ..Default::default()
            },
        }
    } else {
        build_siblings(
            &window.panes,
            &window.flex_direction,
            &window_path,
            &window.name,
            &mut leaves,
        )?
    };

    if leaves.len() > MAX_PANES_PER_TAB {
        bail!(
            "Window '{}' has {} panes, exceeding the {} pane limit",
            window.name,
            leaves.len(),
            MAX_PANES_PER_TAB
        );
    }

    let depth = tree_depth(&root);
    if depth > MAX_SPLIT_DEPTH {
        bail!(
            "Window '{}' has split depth {depth}, exceeding the {MAX_SPLIT_DEPTH} depth limit",
            window.name
        );
    }

    Ok(WindowPlan {
        tab_label: window.name.clone(),
        focus_tab: window.focus,
        root,
        leaves,
    })
}

fn build_siblings(
    panes: &[Pane],
    direction: &FlexDirection,
    window_path: &String,
    window_name: &str,
    leaves: &mut Vec<LeafPlan>,
) -> Result<LayoutNode> {
    build_sibling(panes, 0, direction, window_path, window_name, leaves)
}

fn build_sibling(
    panes: &[Pane],
    index: usize,
    direction: &FlexDirection,
    window_path: &String,
    window_name: &str,
    leaves: &mut Vec<LeafPlan>,
) -> Result<LayoutNode> {
    let pane = panes
        .get(index)
        .ok_or_else(|| miette!("Window '{window_name}' has no pane at index {index}"))?;

    if index + 1 == panes.len() {
        return build_pane(pane, window_path, window_name, leaves);
    }

    let remaining: usize = panes[index..].iter().map(|pane| pane.flex).sum();
    let ratio = if remaining == 0 {
        0.0
    } else {
        pane.flex as f32 / remaining as f32
    };

    Ok(LayoutNode::Split {
        direction: split_direction(direction),
        ratio: clamp_ratio(ratio, window_name, index),
        first: Box::new(build_pane(pane, window_path, window_name, leaves)?),
        second: Box::new(build_sibling(
            panes,
            index + 1,
            direction,
            window_path,
            window_name,
            leaves,
        )?),
    })
}

fn build_pane(
    pane: &Pane,
    window_path: &String,
    window_name: &str,
    leaves: &mut Vec<LeafPlan>,
) -> Result<LayoutNode> {
    if pane.panes.is_empty() {
        leaves.push(LeafPlan {
            name: pane.name.clone(),
            zoom: pane.zoom,
            focus: pane.focus,
            commands: pane_commands(pane)?,
        });

        return Ok(LayoutNode::Pane {
            pane: LayoutPane {
                label: pane.name.clone(),
                cwd: Some(sanitize_path(&pane.path, window_path)),
                ..Default::default()
            },
        });
    }

    build_siblings(
        &pane.panes,
        &pane.flex_direction,
        window_path,
        window_name,
        leaves,
    )
}

fn pane_commands(pane: &Pane) -> Result<Vec<String>> {
    let mut commands: Vec<String> = pane
        .commands
        .iter()
        .map(|command| command.to_string())
        .collect();

    if let Some(script) = &pane.script {
        commands.push(script.to_cmd()?.to_string());
    }

    Ok(commands)
}

fn split_direction(direction: &FlexDirection) -> SplitDirection {
    match direction {
        FlexDirection::Row => SplitDirection::Right,
        FlexDirection::Column => SplitDirection::Down,
    }
}

fn clamp_ratio(ratio: f32, window_name: &str, index: usize) -> f32 {
    let clamped = ratio.clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO);

    if (clamped - ratio).abs() > f32::EPSILON {
        log::warn!("Window '{window_name}' pane index {index} ratio {ratio} clamped to {clamped}");
    }

    clamped
}

fn tree_depth(node: &LayoutNode) -> usize {
    match node {
        LayoutNode::Pane { .. } => 1,
        LayoutNode::Split { first, second, .. } => 1 + tree_depth(first).max(tree_depth(second)),
    }
}

fn collect_pane_ids(node: &LayoutNode, pane_ids: &mut Vec<String>) -> Result<()> {
    match node {
        LayoutNode::Pane { pane } => {
            let pane_id = pane
                .pane_id
                .clone()
                .ok_or_else(|| miette!("Applied layout pane is missing pane_id"))?;
            pane_ids.push(pane_id);
        }
        LayoutNode::Split { first, second, .. } => {
            collect_pane_ids(first, pane_ids)?;
            collect_pane_ids(second, pane_ids)?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TabState {
    pub(crate) zoomed: bool,
    pub(crate) focused_pane_id: String,
    pub(crate) focused: bool,
}

pub(crate) fn window_from_layout(
    layout: &LayoutDescription,
    tab_label: &str,
    session_path: &str,
    state: &TabState,
) -> Window {
    let (flex_direction, panes) = match &layout.root {
        LayoutNode::Pane { pane } => (
            FlexDirection::Row,
            vec![leaf_pane(pane, 1, session_path, state)],
        ),
        LayoutNode::Split {
            direction,
            ratio,
            first,
            second,
        } => {
            let (first_flex, second_flex) = flex_weights(*ratio);

            (
                flex_direction(*direction),
                vec![
                    pane_from_node(first, first_flex, session_path, state),
                    pane_from_node(second, second_flex, session_path, state),
                ],
            )
        }
    };

    Window {
        name: tab_label.to_string(),
        path: None,
        flex_direction,
        panes,
        focus: false,
    }
}

fn pane_from_node(node: &LayoutNode, flex: usize, session_path: &str, state: &TabState) -> Pane {
    match node {
        LayoutNode::Pane { pane } => leaf_pane(pane, flex, session_path, state),
        LayoutNode::Split {
            direction,
            ratio,
            first,
            second,
        } => {
            let (first_flex, second_flex) = flex_weights(*ratio);

            Pane {
                flex_direction: flex_direction(*direction),
                flex,
                name: None,
                path: ".".to_string(),
                style: None,
                commands: Vec::new(),
                script: None,
                panes: vec![
                    pane_from_node(first, first_flex, session_path, state),
                    pane_from_node(second, second_flex, session_path, state),
                ],
                zoom: false,
                focus: false,
            }
        }
    }
}

fn leaf_pane(pane: &LayoutPane, flex: usize, session_path: &str, state: &TabState) -> Pane {
    let is_focused_pane = pane.pane_id.as_deref() == Some(state.focused_pane_id.as_str());

    Pane {
        flex_direction: FlexDirection::Row,
        flex,
        name: pane.label.clone(),
        path: pane
            .cwd
            .as_deref()
            .and_then(|cwd| relative_path(cwd, session_path))
            .unwrap_or_else(|| ".".to_string()),
        style: None,
        commands: pane
            .command
            .as_ref()
            .map(|argv| vec![Command::from_string(&argv.join(" "))])
            .unwrap_or_default(),
        script: None,
        panes: Vec::new(),
        zoom: state.zoomed && is_focused_pane,
        focus: state.focused && is_focused_pane,
    }
}

fn flex_weights(ratio: f32) -> (usize, usize) {
    let first = round((ratio * 100.0) as usize);
    let second = round(((1.0 - ratio) * 100.0) as usize);
    let gcd = gcd_vec(&[first, second]);

    ((first / gcd).max(1), (second / gcd).max(1))
}

fn flex_direction(direction: SplitDirection) -> FlexDirection {
    match direction {
        SplitDirection::Right => FlexDirection::Row,
        SplitDirection::Down => FlexDirection::Column,
    }
}
