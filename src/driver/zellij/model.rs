use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use anyhow::bail;
use anyhow::Result;
use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};

use crate::common::config::util::gcd_vec;
use crate::common::config::{FlexDirection, Pane, Session, Window};

impl Display for FlexDirection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let output = match self {
            FlexDirection::Row => "vertical",
            FlexDirection::Column => "horizontal",
        };
        write!(f, "{}", output)
    }
}

impl FlexDirection {
    pub fn from(option: Option<&KdlValue>) -> Self {
        match option.and_then(|value| value.as_string()) {
            Some("horizontal") => FlexDirection::Column,
            _ => FlexDirection::Row,
        }
    }
}

impl Session {
    pub(crate) fn as_kdl(&self, cwd: &str) -> Result<KdlDocument> {
        let mut session_kdl = KdlDocument::new();
        let mut layout_node = KdlNode::new("layout");
        layout_node
            .entries_mut()
            .push(KdlEntry::new_prop("cwd", KdlValue::String(cwd.to_string())));

        let mut tabs_doc = KdlDocument::new();
        for window in &self.windows {
            tabs_doc.nodes_mut().push(window.as_kdl()?);
        }

        layout_node.set_children(tabs_doc);
        session_kdl.nodes_mut().push(layout_node);
        session_kdl.ensure_v1();
        Ok(session_kdl)
    }

    pub(crate) fn from_kdl(name: &str, layout_node: &kdl::KdlNode) -> Self {
        let path_node = extract_child_node(layout_node, "cwd");
        let path = match path_node {
            Some(node) => extract_entry(node).unwrap_or_else(|| ".".to_string()),
            None => ".".to_string(),
        };
        // let path = find_entry_value(layout_node, "cwd")
        //     .unwrap_or(".")
        //     .to_string();

        let window_nodes = extract_child_nodes(layout_node, "tab");

        Self {
            name: name.to_string(),
            path: path.to_string(),
            startup: vec![],
            shutdown: vec![],
            env: HashMap::new(),
            windows: Window::from_kdl(&window_nodes),
        }
    }
}

impl Window {
    pub fn as_kdl(&self) -> Result<KdlNode> {
        let mut tab_node = KdlNode::new("tab");
        tab_node.entries_mut().push(KdlEntry::new_prop(
            "name",
            KdlValue::String(self.name.to_string()),
        ));

        if !self.panes.is_empty() {
            let mut wrapper_pane = KdlNode::new("pane");
            wrapper_pane.entries_mut().push(KdlEntry::new_prop(
                "split_direction",
                KdlValue::from(self.flex_direction.to_string()),
            ));

            let mut panes_doc = KdlDocument::new();
            for pane in &self.panes {
                panes_doc.nodes_mut().push(pane.as_kdl(&self.panes)?);
            }
            wrapper_pane.set_children(panes_doc);

            let mut wrapper_doc = KdlDocument::new();
            wrapper_doc.nodes_mut().push(wrapper_pane);

            tab_node.set_children(wrapper_doc);
        }

        Ok(tab_node)
    }

    pub(crate) fn from_kdl(window_nodes: &[&KdlNode]) -> Vec<Window> {
        println!("number of windows: {}", window_nodes.len());
        window_nodes
            .iter()
            .map(|window_node| {
                let name = find_entry_value(window_node, "name")
                    .unwrap_or("nameless")
                    .to_string();
                let pane_nodes = extract_child_nodes(window_node, "pane");

                let panes = Pane::from_kdl(&pane_nodes);

                Window {
                    name,
                    flex_direction: FlexDirection::Row,
                    panes,
                }
            })
            .collect()
    }
}

impl Pane {
    pub fn as_kdl(&self, siblings: &[Pane]) -> Result<KdlNode> {
        let mut pane_node = KdlNode::new("pane");

        let percentage = self.calculate_percentage(siblings)?;
        pane_node
            .entries_mut()
            .push(KdlEntry::new_prop("size", KdlValue::String(percentage)));
        if !self.panes.is_empty() {
            let mut children_doc = KdlDocument::new();
            pane_node.entries_mut().push(KdlEntry::new_prop(
                "split_direction",
                KdlValue::from(self.flex_direction.to_string()),
            ));
            for child_pane in &self.panes {
                children_doc
                    .nodes_mut()
                    .push(child_pane.as_kdl(&self.panes)?);
            }
            pane_node.set_children(children_doc);
        } else {
            if self.name.is_some() {
                pane_node.entries_mut().push(KdlEntry::new_prop(
                    "name",
                    KdlValue::String(self.name.clone().unwrap()),
                ));
            };
            pane_node.entries_mut().push(KdlEntry::new_prop(
                "cwd",
                KdlValue::String(self.path.clone()),
            ));
            for command in &self.commands {
                let mut command_node = KdlNode::new("command");
                command_node
                    .entries_mut()
                    .push(KdlEntry::new(KdlValue::String(command.command.clone())));

                pane_node
                    .children_mut()
                    .get_or_insert_with(KdlDocument::new)
                    .nodes_mut()
                    .push(command_node);

                if !command.args.is_empty() {
                    let mut args_node = KdlNode::new("args");
                    for arg in command.args.iter() {
                        args_node
                            .entries_mut()
                            .push(KdlEntry::new(KdlValue::String(arg.to_string())));
                    }
                    pane_node
                        .children_mut()
                        .get_or_insert_with(KdlDocument::new)
                        .nodes_mut()
                        .push(args_node);
                }
            }
        }

        Ok(pane_node)
    }

    pub(crate) fn from_kdl(pane_nodes: &Vec<&KdlNode>) -> Vec<Pane> {
        // Parse sizes from the nodes, defaulting to equal distribution if missing
        let sizes: Vec<usize> = pane_nodes
            .iter()
            .map(|n| {
                n.get("size")
                    .and_then(|value| value.as_string())
                    .and_then(|s| s.strip_suffix('%'))
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(1)
            })
            .collect();

        let gcd = gcd_vec(&sizes);
        let reduced_sizes: Vec<usize> = sizes.iter().map(|&size| size / gcd).collect();

        pane_nodes
            .iter()
            .zip(reduced_sizes.iter())
            .map(|(n, &flex)| {
                let flex_direction = FlexDirection::from(n.get("split_direction"));
                let path: String = n
                    .get("cwd")
                    .and_then(|value| value.as_string().map(|s| s.to_string()))
                    .unwrap_or_else(|| ".".to_string());

                let pane_nodes = extract_child_nodes(n, "pane");
                let panes = Pane::from_kdl(&pane_nodes);

                Pane {
                    flex,
                    flex_direction,
                    name: None,
                    path,
                    style: None,
                    commands: vec![],
                    env: HashMap::new(),
                    panes,
                    zoom: false,
                }
            })
            .collect()
    }

    fn calculate_percentage(&self, siblings: &[Pane]) -> Result<String> {
        let total_flex: f64 = siblings.iter().map(|p| p.flex as f64).sum();
        if total_flex > 0.0 {
            let percentage = ((self.flex as f64 / total_flex) * 100.0).round();
            Ok(format!("{}%", percentage))
        } else {
            bail!("Total flex value is zero, cannot calculate percentage")
        }
    }
}

pub(crate) fn extract_child_nodes<'a>(node: &'a KdlNode, name: &str) -> Vec<&'a KdlNode> {
    node.iter_children()
        .filter(|child| child.name().value() == name)
        .collect()
}

pub(crate) fn extract_child_node<'a>(node: &'a KdlNode, name: &str) -> Option<&'a KdlNode> {
    extract_child_nodes(node, name).first().copied()
}

pub(crate) fn extract_entries(node: &KdlNode) -> Vec<String> {
    node.entries()
        .iter()
        .filter_map(|entry| {
            entry.value().as_string().and_then(|s| {
                if !s.is_empty() {
                    Some(s.to_string())
                } else {
                    None
                }
            })
        })
        .collect()
}

pub(crate) fn extract_entry(node: &KdlNode) -> Option<String> {
    extract_entries(node).first().cloned()
}

pub(crate) fn find_entry_value<'a>(node: &'a KdlNode, name: &str) -> Option<&'a str> {
    node.entries().iter().find_map(|entry| {
        if entry.name().map(|n| n.value()) == Some(name) {
            entry.value().as_string()
        } else {
            None
        }
    })
}
