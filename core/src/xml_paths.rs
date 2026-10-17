//! Document-local names used by the independent XML diagnostic locators.
use roxmltree::{Document, Node, NodeId};
use std::collections::HashMap;

pub(crate) struct NamedPaths {
    names: HashMap<NodeId, String>,
}

impl NamedPaths {
    pub(crate) fn new(
        document: &Document<'_>,
        short_name: impl Fn(Node<'_, '_>) -> Option<String>,
    ) -> Self {
        Self {
            names: document
                .descendants()
                .filter(|node| node.is_element())
                .filter_map(|node| short_name(node).map(|name| (node.id(), name)))
                .collect(),
        }
    }

    pub(crate) fn name(&self, node: Node<'_, '_>) -> Option<&str> {
        self.names.get(&node.id()).map(String::as_str)
    }

    pub(crate) fn path(&self, node: Node<'_, '_>) -> String {
        let mut names: Vec<_> = node
            .ancestors()
            .filter_map(|node| self.name(node))
            .collect();
        names.reverse();
        format!("/{}", names.join("/"))
    }
}
