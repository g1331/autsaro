use super::{Issue, NS};
use roxmltree::Node;
use std::fmt::Write;

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

pub(super) fn param(node: Node<'_, '_>, name: &str) -> Option<String> {
    node.descendants()
        .filter(|n| {
            n.is_element()
                && matches!(
                    n.tag_name().name(),
                    "ECUC-NUMERICAL-PARAM-VALUE" | "ECUC-TEXTUAL-PARAM-VALUE"
                )
        })
        .find(|n| definition(*n).is_some_and(|p| p.ends_with(&format!("/{name}"))))
        .and_then(|n| child_text(n, "VALUE"))
}

pub(super) fn ref_value(node: Node<'_, '_>, suffix: &str) -> Option<String> {
    node.descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "ECUC-REFERENCE-VALUE")
        .find(|n| definition(*n).is_some_and(|p| p.ends_with(&format!("/{suffix}"))))
        .and_then(|n| child_text(n, "VALUE-REF"))
}
pub(super) fn ref_dest(node: Node<'_, '_>, suffix: &str) -> Option<String> {
    node.descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "ECUC-REFERENCE-VALUE")
        .find(|n| definition(*n).is_some_and(|path| path.ends_with(&format!("/{suffix}"))))
        .and_then(|reference| {
            reference
                .children()
                .find(|child| child.is_element() && child.tag_name().name() == "VALUE-REF")
        })
        .and_then(|value| value.attribute("DEST"))
        .map(str::to_owned)
}

pub(super) fn child_containers<'a, 'input>(node: Node<'a, 'input>) -> Vec<Node<'a, 'input>> {
    node.children()
        .filter(|child| {
            child.is_element() && matches!(child.tag_name().name(), "CONTAINERS" | "SUB-CONTAINERS")
        })
        .flat_map(|group| {
            group.children().filter(|child| {
                child.is_element() && child.tag_name().name() == "ECUC-CONTAINER-VALUE"
            })
        })
        .collect()
}
pub(super) fn parse_u32(value: Option<String>, field: &str, path: &str) -> Result<u32, Issue> {
    value
        .ok_or_else(|| {
            Issue::error(
                "MISSING_PARAMETER",
                crate::product_message!(
                    "backend.arxml.xml.missing_parameter",
                    "field" => field
                ),
                Some(path.into()),
            )
        })?
        .parse::<u32>()
        .map_err(|_| {
            Issue::error(
                "INVALID_PARAMETER",
                crate::product_message!(
                    "backend.arxml.xml.unsigned_integer_required",
                    "field" => field
                ),
                Some(path.into()),
            )
        })
}

pub(super) fn parse_milliseconds(
    value: Option<String>,
    field: &str,
    path: &str,
) -> Result<u32, Issue> {
    let text = value.ok_or_else(|| {
        Issue::error(
            "MISSING_PARAMETER",
            crate::product_message!(
                "backend.arxml.xml.missing_parameter",
                "field" => field
            ),
            Some(path.into()),
        )
    })?;
    let (seconds, fraction) = text.split_once('.').unwrap_or((&text, ""));
    if fraction.len() > 3 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Issue::error(
            "TIME_PRECISION",
            crate::product_message!(
                "backend.arxml.xml.whole_milliseconds_required",
                "field" => field
            ),
            Some(path.into()),
        ));
    }
    let seconds = seconds.parse::<u32>().map_err(|_| {
        Issue::error(
            "INVALID_PARAMETER",
            crate::product_message!(
                "backend.arxml.xml.invalid_time_value",
                "field" => field
            ),
            Some(path.into()),
        )
    })?;
    let fraction = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<u32>().map_err(|_| {
            Issue::error(
                "INVALID_PARAMETER",
                crate::product_message!(
                    "backend.arxml.xml.invalid_time_value",
                    "field" => field
                ),
                Some(path.into()),
            )
        })? * 10u32.pow((3 - fraction.len()) as u32)
    };
    seconds
        .checked_mul(1000)
        .and_then(|s| s.checked_add(fraction))
        .ok_or_else(|| {
            Issue::error(
                "TIME_RANGE",
                crate::product_message!(
                    "backend.arxml.xml.time_out_of_range",
                    "field" => field
                ),
                Some(path.into()),
            )
        })
}

pub(super) fn valid_name(name: &str) -> bool {
    name.len() <= 128
        && name.bytes().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

pub(super) fn seconds(milliseconds: u32) -> String {
    format!("{}.{:03}", milliseconds / 1000, milliseconds % 1000)
}

pub(super) fn signal_type(length: u8) -> &'static str {
    if length == 1 {
        "BOOLEAN"
    } else if length <= 8 {
        "UINT8"
    } else if length <= 16 {
        "UINT16"
    } else {
        "UINT32"
    }
}

pub(super) fn structural_node(node: Node<'_, '_>) -> String {
    fn append(node: Node<'_, '_>, out: &mut String) {
        if node.is_comment() {
            out.push_str("<!--");
            out.push_str(node.text().unwrap_or(""));
            out.push_str("-->");
        } else if node.is_element() {
            out.push('<');
            out.push_str(node.tag_name().name());
            let mut attributes: Vec<_> = node
                .attributes()
                .map(|a| (a.namespace().unwrap_or(""), a.name(), a.value()))
                .collect();
            attributes.sort_unstable();
            for attribute in attributes {
                write!(out, " {:?}", attribute).unwrap();
            }
            out.push('>');
            for child in node.children() {
                append(child, out);
            }
            out.push_str("</");
            out.push_str(node.tag_name().name());
            out.push('>');
        } else if node.is_text() {
            let text = node.text().unwrap_or("").trim();
            if !text.is_empty() {
                out.push_str(text);
            }
        }
    }
    let mut out = String::new();
    append(node, &mut out);
    out
}
