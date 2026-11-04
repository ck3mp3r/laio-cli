use std::collections::HashMap;
use std::path::{Path, PathBuf};

use miette::{Result, bail, miette};

use crate::common::config::util::gcd_vec;
use crate::common::config::{FlexDirection, Pane, Session, Window};
use crate::common::path::relative_path;

use super::model::{LayerView, SessionView, ViewNode, ViewSplit, WindowView};
use super::mux::LABEL_PREFIX;

const TILED_KIND: &str = "tiled";
const HORIZONTAL: &str = "horizontal";
const VERTICAL: &str = "vertical";
const DEFAULT_SPLIT_RATIO: f64 = 0.5;
const FLEX_SCALE: f64 = 100.0;
const FLEX_TOTAL: usize = 100;
const FILE_URL_PREFIX: &str = "file://";

pub(crate) fn local_path(pwd: &str) -> &str {
    let Some(rest) = pwd.strip_prefix(FILE_URL_PREFIX) else {
        return pwd;
    };

    rest.find('/').map_or(pwd, |index| &rest[index..])
}

pub(crate) fn tiled_block_ids(view: &SessionView) -> Result<Vec<&str>> {
    let mut block_ids = Vec::new();

    for window in view.windows.as_deref().unwrap_or_default() {
        let Some(layout) = tiled_layer(window).and_then(|layer| layer.layout.as_ref()) else {
            continue;
        };

        let mut leaves = Vec::new();
        collect_leaves(layout, 1.0, &mut leaves)?;
        block_ids.extend(leaves.into_iter().map(|(block_id, _)| block_id));
    }

    Ok(block_ids)
}

pub(crate) fn session_from_view(
    view: &SessionView,
    pwds: &HashMap<String, String>,
    fallback_name: Option<&str>,
) -> Result<Session> {
    let windows = view.windows.as_deref().unwrap_or_default();
    let leaves: Vec<Vec<(&str, f64)>> = windows.iter().map(window_leaves).collect::<Result<_>>()?;

    let session_path = session_path(&leaves, pwds)?;
    let active_window_id = view.active_window_id.as_deref();

    let mut exported = Vec::with_capacity(windows.len());
    for (window, leaves) in windows.iter().zip(&leaves) {
        exported.push(window_from_view(
            window,
            leaves,
            &session_path,
            pwds,
            active_window_id,
        )?);
    }

    Ok(Session {
        name: session_name(view, fallback_name)?,
        path: session_path,
        startup: Vec::new(),
        startup_script: None,
        shutdown: Vec::new(),
        shutdown_script: None,
        env: HashMap::new(),
        shell: None,
        pane_cmd_delay: None,
        windows: exported,
    })
}

fn window_leaves(window: &WindowView) -> Result<Vec<(&str, f64)>> {
    let Some(layout) = tiled_layer(window).and_then(|layer| layer.layout.as_ref()) else {
        return Ok(Vec::new());
    };

    let mut leaves = Vec::new();
    collect_leaves(layout, 1.0, &mut leaves)?;

    Ok(leaves)
}

fn tiled_layer(window: &WindowView) -> Option<&LayerView> {
    window
        .layers
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|layer| layer.kind == TILED_KIND)
}

fn collect_leaves<'a>(
    node: &'a ViewNode,
    fraction: f64,
    leaves: &mut Vec<(&'a str, f64)>,
) -> Result<()> {
    if let Some(split) = &node.split {
        let ratio = split.ratio.unwrap_or(DEFAULT_SPLIT_RATIO);

        collect_leaves(split_node(split, Side::Before)?, fraction * ratio, leaves)?;

        return collect_leaves(
            split_node(split, Side::After)?,
            fraction * (1.0 - ratio),
            leaves,
        );
    }

    leaves.push((node_block_id(node)?, fraction));

    Ok(())
}

fn node_block_id(node: &ViewNode) -> Result<&str> {
    node.block_id
        .as_deref()
        .ok_or_else(|| miette!("rex layout node has neither a block nor a split"))
}

#[derive(Clone, Copy)]
enum Side {
    Before,
    After,
}

fn split_node(split: &ViewSplit, side: Side) -> Result<&ViewNode> {
    let (node, name) = match side {
        Side::Before => (&split.before, "before"),
        Side::After => (&split.after, "after"),
    };

    node.as_ref()
        .ok_or_else(|| miette!("rex layout split has no {name} node"))
}

fn session_path(leaves: &[Vec<(&str, f64)>], pwds: &HashMap<String, String>) -> Result<String> {
    let paths: Vec<&str> = leaves
        .iter()
        .flatten()
        .filter_map(|(block_id, _)| pwds.get(*block_id).map(String::as_str))
        .collect();

    common_path(&paths)
}

fn common_path(paths: &[&str]) -> Result<String> {
    let mut paths = paths.iter();
    let Some(first) = paths.next() else {
        bail!("Cannot determine the session path from the rex block working directories");
    };

    let mut prefix = PathBuf::from(first);
    for path in paths {
        while !Path::new(path).starts_with(&prefix) {
            if !prefix.pop() {
                bail!("Cannot determine the session path from the rex block working directories");
            }
        }
    }

    let prefix = prefix.to_string_lossy().into_owned();
    if prefix.is_empty() || prefix == "/" {
        bail!("Cannot determine the session path from the rex block working directories");
    }

    Ok(prefix)
}

fn session_name(view: &SessionView, fallback: Option<&str>) -> Result<String> {
    if let Some(name) = fallback {
        return Ok(name.to_string());
    }

    let label = view
        .label
        .as_deref()
        .ok_or_else(|| miette!("Cannot determine the rex session name"))?;

    log::warn!("No laio record for rex session '{label}'; deriving the name from the label");

    Ok(label
        .strip_prefix(LABEL_PREFIX)
        .unwrap_or(label)
        .to_string())
}

fn window_from_view<'a>(
    window: &'a WindowView,
    leaves: &'a [(&'a str, f64)],
    session_path: &'a str,
    pwds: &'a HashMap<String, String>,
    active_window_id: Option<&'a str>,
) -> Result<Window> {
    let mut flex_direction = FlexDirection::Row;
    let mut panes = Vec::new();

    if let Some(layer) = tiled_layer(window)
        && let Some(layout) = &layer.layout
    {
        let direction = root_direction(layout)?;
        let context = ExportContext {
            session_path,
            pwds,
            labels: block_labels(layer),
            flexes: flexes(leaves),
            focused_block_id: window.focused_block_id.as_deref(),
            zoomed_block_id: window.zoomed_block_id.as_deref(),
        };
        let group = group_from_node(layout, &direction, &context)?;

        flex_direction = direction;
        panes = group.panes;
    }

    Ok(Window {
        name: window.label.clone().unwrap_or_default(),
        path: None,
        flex_direction,
        panes,
        focus: window
            .window_id
            .as_deref()
            .is_some_and(|window_id| Some(window_id) == active_window_id),
    })
}

fn block_labels(layer: &LayerView) -> HashMap<&str, &str> {
    layer
        .blocks
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|block| {
            block
                .label
                .as_deref()
                .map(|label| (block.block_id.as_str(), label))
        })
        .collect()
}

fn flexes<'a>(leaves: &[(&'a str, f64)]) -> HashMap<&'a str, usize> {
    let mut weights: Vec<usize> = leaves
        .iter()
        .map(|(_, fraction)| (fraction * FLEX_SCALE).round() as usize)
        .collect();

    let count = weights.len();
    if count > 1 {
        let others: usize = weights[..count - 1].iter().sum();
        weights[count - 1] = FLEX_TOTAL.saturating_sub(others);
    }

    let gcd = gcd_vec(&weights);

    leaves
        .iter()
        .zip(weights)
        .map(|((block_id, _), weight)| (*block_id, (weight / gcd).max(1)))
        .collect()
}

fn root_direction(node: &ViewNode) -> Result<FlexDirection> {
    match &node.split {
        Some(split) => split_direction(&split.direction),
        None => Ok(FlexDirection::Row),
    }
}

fn split_direction(direction: &str) -> Result<FlexDirection> {
    match direction {
        HORIZONTAL => Ok(FlexDirection::Row),
        VERTICAL => Ok(FlexDirection::Column),
        _ => Err(miette!("Unknown rex split direction '{direction}'")),
    }
}

struct PaneGroup {
    panes: Vec<Pane>,
    flex: usize,
}

struct ExportContext<'a> {
    session_path: &'a str,
    pwds: &'a HashMap<String, String>,
    labels: HashMap<&'a str, &'a str>,
    flexes: HashMap<&'a str, usize>,
    focused_block_id: Option<&'a str>,
    zoomed_block_id: Option<&'a str>,
}

impl ExportContext<'_> {
    fn flex(&self, block_id: &str) -> usize {
        self.flexes.get(block_id).copied().unwrap_or(1)
    }

    fn leaf(&self, block_id: &str) -> Pane {
        Pane {
            flex_direction: FlexDirection::Row,
            flex: self.flex(block_id),
            name: self.labels.get(block_id).map(|label| (*label).to_string()),
            path: self
                .pwds
                .get(block_id)
                .and_then(|pwd| relative_path(pwd, self.session_path))
                .unwrap_or_else(|| ".".to_string()),
            style: None,
            commands: Vec::new(),
            script: None,
            panes: Vec::new(),
            zoom: self.zoomed_block_id == Some(block_id),
            focus: self.focused_block_id == Some(block_id),
        }
    }
}

fn group_from_node<'a>(
    node: &'a ViewNode,
    direction: &FlexDirection,
    context: &ExportContext<'a>,
) -> Result<PaneGroup> {
    let Some(split) = &node.split else {
        let block_id = node_block_id(node)?;

        return Ok(PaneGroup {
            flex: context.flex(block_id),
            panes: vec![context.leaf(block_id)],
        });
    };

    let split_direction = split_direction(&split.direction)?;

    if &split_direction == direction {
        let before = group_from_node(split_node(split, Side::Before)?, direction, context)?;
        let after = group_from_node(split_node(split, Side::After)?, direction, context)?;
        let flex = before.flex + after.flex;
        let mut panes = before.panes;
        panes.extend(after.panes);

        return Ok(PaneGroup { panes, flex });
    }

    let nested = group_from_node(node, &split_direction, context)?;

    Ok(PaneGroup {
        flex: nested.flex,
        panes: vec![Pane {
            flex_direction: split_direction,
            flex: nested.flex,
            name: None,
            path: ".".to_string(),
            style: None,
            commands: Vec::new(),
            script: None,
            panes: nested.panes,
            zoom: false,
            focus: false,
        }],
    })
}
