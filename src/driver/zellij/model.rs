use std::fmt::{Display, Formatter};

use anyhow::Result;
use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};

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

impl Session {
    pub(crate) fn as_kdl(&self) -> Result<KdlDocument> {
        let mut session_kdl = KdlDocument::new();
        let mut layout_node = KdlNode::new("layout");

        let mut tabs_doc = KdlDocument::new();
        for window in &self.windows {
            tabs_doc.nodes_mut().push(window.as_kdl()?);
        }

        layout_node.set_children(tabs_doc);
        session_kdl.nodes_mut().push(layout_node);

        Ok(session_kdl)
    }
}

impl Window {
    pub fn as_kdl(&self) -> Result<KdlNode> {
        let mut tab_node = KdlNode::new("tab");
        tab_node.entries_mut().push(KdlEntry::new_prop(
            "name",
            KdlValue::String(format!("\"{}\"", self.name)),
        ));

        if !self.panes.is_empty() {
            let mut wrapper_pane = KdlNode::new("pane");
            wrapper_pane.entries_mut().push(KdlEntry::new_prop(
                "name",
                KdlValue::String(format!("\"{}\"", self.name)),
            ));

            wrapper_pane.entries_mut().push(KdlEntry::new_prop(
                "split_direction",
                KdlValue::from(format!("\"{}\"", self.flex_direction)),
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
}

impl Pane {
    pub fn as_kdl(&self, siblings: &[Pane]) -> Result<KdlNode> {
        let mut pane_node = KdlNode::new("pane");

        let percentage = self.calculate_percentage(siblings)?.round();
        pane_node.entries_mut().push(KdlEntry::new_prop(
            "size",
            KdlValue::String(format!("{}%", percentage)),
        ));

        if !self.panes.is_empty() {
            let mut children_doc = KdlDocument::new();
            pane_node.entries_mut().push(KdlEntry::new_prop(
                "split_direction",
                KdlValue::from(format!("\"{}\"", self.flex_direction)),
            ));
            for child_pane in &self.panes {
                children_doc
                    .nodes_mut()
                    .push(child_pane.as_kdl(&self.panes)?);
            }
            pane_node.set_children(children_doc);
        }

        Ok(pane_node)
    }

    fn calculate_percentage(&self, siblings: &[Pane]) -> Result<f64> {
        let total_flex: f64 = siblings.iter().map(|p| p.flex as f64).sum();
        if total_flex > 0.0 {
            Ok((self.flex as f64 / total_flex) * 100.0)
        } else {
            Err(anyhow::anyhow!(
                "Total flex value is zero, cannot calculate percentage"
            ))
        }
    }
}