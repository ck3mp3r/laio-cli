use miette::{Result, miette};

use crate::common::config::{FlexDirection, Pane, Session, Window};
use crate::common::path::sanitize_path;

use super::model::{BlockNode, BlockOptions, CreateRequest, CreateWindow, LayoutNode, SplitNode};

const MIN_SPLIT_RATIO: f64 = 0.1;
const MAX_SPLIT_RATIO: f64 = 0.9;
const SHELL_FLAVOR: &str = "com.superlogical.terminal.shell";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeafPlan {
    pub(crate) window_index: usize,
    pub(crate) block_index: usize,
    pub(crate) name: Option<String>,
    pub(crate) cwd: String,
    pub(crate) commands: Vec<String>,
    pub(crate) zoom: bool,
    pub(crate) focus: bool,
}

struct WindowContext<'a> {
    window_index: usize,
    window_path: &'a str,
    window_name: &'a str,
    shell: &'a Option<String>,
}

pub(crate) fn build_create_request(session: &Session) -> Result<(CreateRequest, Vec<LeafPlan>)> {
    let mut leaves = Vec::new();
    let mut windows = Vec::new();

    for (window_index, window) in session.windows.iter().enumerate() {
        let window_path = window.effective_path(&session.path);
        let mut window_leaves = Vec::new();
        let context = WindowContext {
            window_index,
            window_path: &window_path,
            window_name: &window.name,
            shell: &session.shell,
        };

        let layout = build_window(window, &context, &mut window_leaves)?;

        windows.push(CreateWindow {
            window_label: Some(window.name.clone()),
            layout,
        });
        leaves.extend(window_leaves);
    }

    Ok((
        CreateRequest {
            label: None,
            initial_windows: Some(windows),
        },
        leaves,
    ))
}

fn build_window(
    window: &Window,
    context: &WindowContext,
    leaves: &mut Vec<LeafPlan>,
) -> Result<LayoutNode> {
    if window.panes.is_empty() {
        leaves.push(LeafPlan {
            window_index: context.window_index,
            block_index: 0,
            name: None,
            cwd: context.window_path.to_string(),
            commands: Vec::new(),
            zoom: false,
            focus: false,
        });

        return Ok(block_node(
            None,
            context.window_path.to_string(),
            context.shell,
        ));
    }

    build_siblings(&window.panes, &window.flex_direction, context, leaves)
}

fn build_siblings(
    panes: &[Pane],
    direction: &FlexDirection,
    context: &WindowContext,
    leaves: &mut Vec<LeafPlan>,
) -> Result<LayoutNode> {
    build_sibling(panes, 0, direction, context, leaves)
}

fn build_sibling(
    panes: &[Pane],
    index: usize,
    direction: &FlexDirection,
    context: &WindowContext,
    leaves: &mut Vec<LeafPlan>,
) -> Result<LayoutNode> {
    let pane = panes.get(index).ok_or_else(|| {
        miette!(
            "Window '{}' has no pane at index {index}",
            context.window_name
        )
    })?;

    if index + 1 == panes.len() {
        return build_pane(pane, context, leaves);
    }

    let remaining: usize = panes[index..].iter().map(|pane| pane.flex).sum();
    let ratio = if remaining == 0 {
        0.0
    } else {
        pane.flex as f64 / remaining as f64
    };

    Ok(LayoutNode {
        block: None,
        split: Some(SplitNode {
            direction: split_direction(direction).to_string(),
            ratio: Some(clamp_ratio(ratio, context.window_name, index)),
            before: Box::new(build_pane(pane, context, leaves)?),
            after: Box::new(build_sibling(panes, index + 1, direction, context, leaves)?),
        }),
    })
}

fn build_pane(
    pane: &Pane,
    context: &WindowContext,
    leaves: &mut Vec<LeafPlan>,
) -> Result<LayoutNode> {
    if pane.panes.is_empty() {
        let cwd = sanitize_path(&pane.path, &context.window_path.to_string());

        leaves.push(LeafPlan {
            window_index: context.window_index,
            block_index: leaves.len(),
            name: pane.name.clone(),
            cwd: cwd.clone(),
            commands: pane_commands(pane)?,
            zoom: pane.zoom,
            focus: pane.focus,
        });

        return Ok(block_node(pane.name.clone(), cwd, context.shell));
    }

    build_siblings(&pane.panes, &pane.flex_direction, context, leaves)
}

fn block_node(label: Option<String>, cwd: String, shell: &Option<String>) -> LayoutNode {
    LayoutNode {
        block: Some(BlockNode {
            flavor: SHELL_FLAVOR.to_string(),
            label,
            options: Some(BlockOptions {
                shell: shell.clone(),
                cwd: Some(cwd),
                ..BlockOptions::default()
            }),
        }),
        split: None,
    }
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

fn split_direction(direction: &FlexDirection) -> &'static str {
    match direction {
        FlexDirection::Row => "horizontal",
        FlexDirection::Column => "vertical",
    }
}

fn clamp_ratio(ratio: f64, window_name: &str, index: usize) -> f64 {
    let clamped = ratio.clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO);

    if (clamped - ratio).abs() > f64::EPSILON {
        log::warn!("Window '{window_name}' pane index {index} ratio {ratio} clamped to {clamped}");
    }

    clamped
}
