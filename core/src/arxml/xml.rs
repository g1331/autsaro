use super::NS;
use roxmltree::Node;

pub(super) fn child_text(node: Node<'_, '_>, name: &str) -> Option<String> {
    let leaf = node.children().find(|node| {
        node.is_element()
            && node.tag_name().namespace() == Some(NS)
            && node.tag_name().name() == name
    })?;
    let mut text = literal_text(leaf)?;
    if matches!(name, "SHORT-NAME" | "DEFINITION-REF") {
        text.truncate(text.trim_end().len());
        let leading = text.len() - text.trim_start().len();
        if leading != 0 {
            text.drain(..leading);
        }
    }
    Some(text)
}

pub(super) fn literal_text(node: Node<'_, '_>) -> Option<String> {
    if node.children().any(|child| child.is_element()) {
        return None;
    }
    Some(
        node.children()
            .filter(|child| child.is_text())
            .filter_map(|child| child.text())
            .collect(),
    )
}

pub(super) fn definition(node: Node<'_, '_>) -> Option<String> {
    child_text(node, "DEFINITION-REF")
}

pub(super) fn path_of(node: Node<'_, '_>) -> String {
    let mut names: Vec<String> = node
        .ancestors()
        .filter_map(|n| child_text(n, "SHORT-NAME"))
        .collect();
    names.reverse();
    format!("/{}", names.join("/"))
}

pub(super) fn valid_name(name: &str) -> bool {
    name.len() <= 128
        && name.bytes().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}
