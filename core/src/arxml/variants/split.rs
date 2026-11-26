use super::{
    resolve::{element, object},
    *,
};
use crate::arxml::{Document, Node, Patch, apply_patches};

const MODULE: &str = "ECUC-MODULE-CONFIGURATION-VALUES";
const CONTAINER: &str = "ECUC-CONTAINER-VALUE";

type Fragment<'a, 'input> = (&'a str, Node<'a, 'input>);

fn xml<'a>(node: Node<'a, '_>) -> &'a str {
    &node.document().input_text()[node.range()]
}

/// GST named contributors merge into one container. Unnamed value
/// contributions are retained so the regular definition check sees every value.
fn combine(fragments: &[Fragment<'_, '_>]) -> Result<String, Vec<PlanDiagnostic>> {
    let first = fragments[0].1;
    let id = object(first);
    let tag = first.tag_name().name();
    let mut singles = BTreeMap::new();
    let mut lists: BTreeMap<&str, Vec<_>> = BTreeMap::new();
    for (file, node) in fragments {
        for child in node.children().filter(|child| child.is_element()) {
            let name = child.tag_name().name();
            if matches!(
                name,
                "CONTAINERS" | "SUB-CONTAINERS" | "PARAMETER-VALUES" | "REFERENCE-VALUES"
            ) {
                lists.entry(name).or_default().extend(
                    child
                        .children()
                        .filter(|entry| entry.is_element())
                        .map(|entry| (*file, entry)),
                );
            } else if let Some(previous) = singles.insert(name, child) {
                if xml(previous) != xml(child)
                    && (crate::arxml::xml::literal_text(previous)
                        != crate::arxml::xml::literal_text(child)
                        || previous
                            .attributes()
                            .map(|attribute| (attribute.name(), attribute.value()))
                            .ne(child
                                .attributes()
                                .map(|attribute| (attribute.name(), attribute.value()))))
                {
                    return Err(vec![diagnostic(
                        "ECUC_SPLIT_PROPERTY_CONFLICT",
                        &id.path,
                        format!("Split contributions disagree on {name} in {file}."),
                    )]);
                }
                singles.insert(name, previous);
            }
        }
    }
    let mut children: Vec<_> = singles
        .into_iter()
        .map(|(name, child)| (name, xml(child).to_owned()))
        .collect();
    for (name, entries) in lists {
        let mut body = String::new();
        if matches!(name, "CONTAINERS" | "SUB-CONTAINERS") {
            let mut groups: Vec<Vec<_>> = Vec::new();
            let mut seen = BTreeMap::new();
            for entry in entries {
                let key = object(entry.1);
                if let Some(index) = seen.get(&key).copied() {
                    let group: &mut Vec<_> = &mut groups[index];
                    group.push(entry);
                } else {
                    seen.insert(key, groups.len());
                    groups.push(vec![entry]);
                }
            }
            for group in groups {
                if group.len() == 1 {
                    body.push_str(xml(group[0].1));
                } else {
                    body.push_str(&combine(&group)?);
                }
            }
        } else {
            for (_, entry) in entries {
                body.push_str(xml(entry));
            }
        }
        if !body.is_empty() {
            children.push((name, format!("<{name}>{body}</{name}>")));
        }
    }
    children.sort_by_key(|(name, _)| crate::rules::child_rank(tag, name));
    let opening_end = first.range().start + xml(first).find('>').unwrap_or_default() + 1;
    let mut result = first.document().input_text()[first.range().start..opening_end].to_owned();
    for (_, child) in children {
        result.push_str(&child);
    }
    result.push_str(&format!("</{tag}>"));
    Ok(result)
}

pub(super) fn merge(text: &mut BTreeMap<String, String>) -> Result<(), Vec<PlanDiagnostic>> {
    let mut modules = BTreeMap::<LayerObject, Vec<_>>::new();
    let documents = text
        .iter()
        .map(|(file, text)| {
            let document = Document::parse(text)
                .map_err(|error| vec![diagnostic("LAYER_XML", file, error.to_string())])?;
            Ok((file, document))
        })
        .collect::<Result<Vec<_>, Vec<PlanDiagnostic>>>()?;
    for (file, document) in &documents {
        for node in document.descendants().filter(|node| element(*node, MODULE)) {
            modules
                .entry(object(node))
                .or_default()
                .push((file.as_str(), node));
        }
    }
    let mut patches = BTreeMap::<String, Vec<Patch>>::new();
    for fragments in modules.values().filter(|fragments| fragments.len() > 1) {
        let mut replacement = combine(fragments)?;
        for (file, node) in fragments {
            patches.entry((*file).into()).or_default().push(Patch {
                range: node.range(),
                value: std::mem::take(&mut replacement),
            });
        }
    }
    // Containers may also be split within a single module member.
    for (file, document) in &documents {
        for parent in document
            .descendants()
            .filter(|node| element(*node, "CONTAINERS") || element(*node, "SUB-CONTAINERS"))
        {
            let mut entries = BTreeMap::<LayerObject, Vec<_>>::new();
            for node in parent.children().filter(|node| element(*node, CONTAINER)) {
                entries
                    .entry(object(node))
                    .or_default()
                    .push((file.as_str(), node));
            }
            for fragments in entries.values().filter(|entries| entries.len() > 1) {
                let first = fragments[0].1.range();
                if patches.get(*file).is_some_and(|patches| {
                    patches
                        .iter()
                        .any(|patch| patch.range.contains(&first.start))
                }) {
                    continue;
                }
                let mut replacement = combine(fragments)?;
                for (_, node) in fragments {
                    patches.entry((*file).clone()).or_default().push(Patch {
                        range: node.range(),
                        value: std::mem::take(&mut replacement),
                    });
                }
            }
        }
    }
    drop(documents);
    for (file, mut changes) in patches {
        let input = text.get_mut(&file).expect("Split source is present");
        apply_patches(input, &mut changes)
            .map_err(|message| vec![diagnostic("ECUC_SPLIT", &file, message)])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contribution(value: &str, edition: &str) -> String {
        format!(
            r#"<AUTOSAR xmlns="http://autosar.org/schema/r4.0"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Configuration</SHORT-NAME><ELEMENTS>
<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>Os</SHORT-NAME>
<DEFINITION-REF>/AUTOSAR/EcucDefs/Os</DEFINITION-REF><ECUC-DEF-EDITION>{edition}</ECUC-DEF-EDITION>
<CONTAINERS><ECUC-CONTAINER-VALUE><SHORT-NAME>Task</SHORT-NAME><DEFINITION-REF>/AUTOSAR/EcucDefs/Os/OsTask</DEFINITION-REF>
<REFERENCE-VALUES><ECUC-REFERENCE-VALUE><DEFINITION-REF>/AUTOSAR/EcucDefs/Os/OsTask/OsTaskEventRef</DEFINITION-REF><VALUE-REF DEST="ECUC-CONTAINER-VALUE">/Configuration/Os/{value}</VALUE-REF></ECUC-REFERENCE-VALUE></REFERENCE-VALUES>
</ECUC-CONTAINER-VALUE></CONTAINERS></ECUC-MODULE-CONFIGURATION-VALUES></ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>"#
        )
    }

    #[test]
    fn split_containers_keep_all_list_values_and_require_equal_module_edition() {
        let left = contribution("Ev_First", "4.10.0");
        let right = contribution("Ev_Second", "4.10.0");
        let mut views = BTreeMap::from([
            ("base.arxml".into(), left.clone()),
            ("project.arxml".into(), right.clone()),
        ]);
        merge(&mut views).unwrap();
        assert_eq!(
            views["base.arxml"]
                .matches("<ECUC-CONTAINER-VALUE>")
                .count(),
            1
        );
        assert!(views["base.arxml"].contains("Ev_First"));
        assert!(views["base.arxml"].contains("Ev_Second"));
        assert!(!views["project.arxml"].contains("<ECUC-MODULE-CONFIGURATION-VALUES>"));

        let mut views = BTreeMap::from([
            ("base.arxml".into(), left),
            ("project.arxml".into(), contribution("Ev_Second", "4.11.0")),
        ]);
        let issues = merge(&mut views).unwrap_err();
        assert_eq!(issues[0].code, "ECUC_SPLIT_PROPERTY_CONFLICT");
        assert_eq!(issues[0].object.as_deref(), Some("/Configuration/Os"));
    }
}
