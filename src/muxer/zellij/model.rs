use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};
use miette::{bail, Result};
use serde_yaml::Value;

use crate::common::config::{Command, FlexDirection, Pane, Session, Window};
use crate::common::path::relative_path;

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
    pub fn from_kdl(option: Option<&KdlValue>) -> Self {
        match option.and_then(|value| value.as_string()) {
            Some("vertical") => FlexDirection::Row,
            _ => FlexDirection::Column,
        }
    }
}

impl Command {
    pub fn from_kdl(cmd: &KdlValue, args: &[&KdlNode]) -> Command {
        Self {
            command: cmd.to_string(),
            args: args
                .iter()
                .flat_map(|node| {
                    node.entries().iter().filter_map(|entry| {
                        entry
                            .value()
                            .as_string()
                            .map(|s| Value::String(s.to_string()))
                    })
                })
                .collect::<Vec<Value>>(),
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
        let path = layout_node
            .children()
            .and_then(|children| {
                children
                    .get("cwd")
                    .and_then(|cwd_node| cwd_node.entries().first())
                    .and_then(|e| e.value().as_string())
                    .map(|s| s.to_string())
            })
            .unwrap_or(".".to_string());

        let window_nodes = extract_child_nodes(layout_node, "tab");

        Self {
            name: name.to_string(),
            path: path.clone(),
            startup: vec![],
            shutdown: vec![],
            startup_script: None,
            shutdown_script: None,
            env: HashMap::new(),
            shell: None,
            windows: Window::from_kdl(&window_nodes, &path),
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
        tab_node.entries_mut().push(KdlEntry::new_prop(
            "split_direction",
            KdlValue::from(self.flex_direction.to_string()),
        ));

        if !self.panes.is_empty() {
            let mut panes_doc = KdlDocument::new();
            for pane in &self.panes {
                panes_doc.nodes_mut().push(pane.as_kdl(&self.panes)?);
            }

            tab_node.set_children(panes_doc);
        }

        Ok(tab_node)
    }

    pub(crate) fn from_kdl(window_nodes: &[&KdlNode], session_path: &String) -> Vec<Window> {
        window_nodes
            .iter()
            .map(|window_node| {
                let name = find_entry_value(window_node, "name")
                    .unwrap_or("nameless")
                    .to_string();

                let pane_nodes = extract_child_nodes(window_node, "pane");

                let panes = Pane::from_kdl(&pane_nodes, session_path);
                let flex_direction = if panes.is_empty() {
                    FlexDirection::Row
                } else {
                    FlexDirection::from_kdl(window_node.get("split_direction"))
                };

                Window {
                    name,
                    flex_direction,
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
            if self.path != "." {
                pane_node.entries_mut().push(KdlEntry::new_prop(
                    "cwd",
                    KdlValue::String(self.path.clone()),
                ));
            };

            if self.focus {
                pane_node
                    .entries_mut()
                    .push(KdlEntry::new_prop("focus", KdlValue::Bool(true)));
            }

            for command in &self.commands {
                pane_node.push(KdlEntry::new_prop("command", command.command.clone()));

                if !command.args.is_empty() {
                    let mut args_node = KdlNode::new("args");
                    command.args.iter().for_each(|arg| {
                        args_node.entries_mut().push(KdlEntry::new(KdlValue::String(
                            serde_yaml::to_string(arg)
                                .unwrap_or_default()
                                .trim_end()
                                .to_string(),
                        )));
                    });
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

    pub(crate) fn from_kdl(pane_nodes: &[&KdlNode], session_path: &String) -> Vec<Pane> {
        let size_strings: Vec<&str> = pane_nodes
            .iter()
            .map(|n| {
                n.get("size")
                    .and_then(|value| value.as_string())
                    .unwrap_or("100%")
            })
            .collect();

        let ratios = calculate_ratios(&size_strings);

        pane_nodes
            .iter()
            .zip(ratios.iter())
            .map(|(node, &flex)| {
                let full_path: String = node
                    .get("cwd")
                    .and_then(|value| value.as_string().map(|s| s.to_string()))
                    .unwrap_or_else(|| ".".to_string());

                let path = match relative_path(&full_path, session_path) {
                    Some(the_path) => the_path,
                    None => ".".to_string(),
                };

                let name: Option<String> = node
                    .get("name")
                    .and_then(|value| value.as_string().map(|s| s.to_string()));

                let commands: Vec<Command> = match node.get("command") {
                    Some(cmd) => {
                        let args = extract_child_nodes(node, "args");
                        vec![Command::from_kdl(cmd, &args)]
                    }
                    None => vec![],
                };

                let pane_nodes = extract_child_nodes(node, "pane");
                let panes = Pane::from_kdl(&pane_nodes, session_path);

                let flex_direction = if panes.is_empty() {
                    FlexDirection::Row
                } else {
                    FlexDirection::from_kdl(node.get("split_direction"))
                };

                Pane {
                    flex,
                    flex_direction,
                    name,
                    path,
                    style: None,
                    commands,
                    script: None,
                    panes,
                    zoom: false,
                    focus: false,
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

pub(crate) fn calculate_ratios(percentages: &[&str]) -> Vec<usize> {
    if percentages.is_empty() {
        return vec![];
    }

    let values: Vec<usize> = percentages
        .iter()
        .filter_map(|&p| p.strip_suffix('%').and_then(|s| s.parse::<usize>().ok()))
        .collect();

    let min_value = *values.iter().min().unwrap_or(&1);

    values.iter().map(|&value| value / min_value).collect()
}

pub(crate) fn extract_child_nodes<'a>(node: &'a KdlNode, name: &str) -> Vec<&'a KdlNode> {
    node.iter_children()
        .filter(|child| child.name().value() == name)
        .collect()
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
