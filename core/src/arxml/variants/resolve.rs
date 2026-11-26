use super::*;
use crate::arxml::{Document, NS, Node, Patch, apply_patches, child_text, definition, path_of};
use crate::project_model::ValueKind;

#[derive(Clone)]
struct Field {
    node_range: std::ops::Range<usize>,
    value_range: Option<std::ops::Range<usize>>,
    value: String,
    dest: Option<String>,
}

pub(super) fn object(node: Node<'_, '_>) -> LayerObject {
    LayerObject {
        release: "R24-11".into(),
        definition_type: definition(node).unwrap_or_else(|| node.tag_name().name().to_owned()),
        path: path_of(node),
        instance_context: if is_instance_reference(node) {
            node.descendants()
                .filter(|child| {
                    child.is_element()
                        && child.tag_name().name().ends_with("-REF")
                        && !element(*child, "DEFINITION-REF")
                })
                .filter_map(crate::arxml::xml::literal_text)
                .map(|value| value.trim().to_owned())
                .collect()
        } else {
            Vec::new()
        },
    }
}

pub(super) fn element(node: Node<'_, '_>, tag: &str) -> bool {
    node.is_element() && node.tag_name().namespace() == Some(NS) && node.tag_name().name() == tag
}

fn is_named(node: Node<'_, '_>) -> bool {
    node.is_element()
        && node.tag_name().namespace() == Some(NS)
        && child_text(node, "SHORT-NAME").is_some()
}

fn is_instance_reference(node: Node<'_, '_>) -> bool {
    element(node, "COMPONENT-IREF") || element(node, "ECUC-INSTANCE-REFERENCE-VALUE")
}

fn field_nodes<'a, 'input>(
    node: Node<'a, 'input>,
) -> impl Iterator<Item = (String, Field)> + use<'a, 'input> {
    node.children()
        .filter(|child| element(*child, "PARAMETER-VALUES") || element(*child, "REFERENCE-VALUES"))
        .flat_map(|list| list.children())
        .filter_map(|field| {
            let id = definition(field)?;
            let leaf = field
                .children()
                .find(|child| element(*child, "VALUE") || element(*child, "VALUE-REF"))?;
            Some((
                id,
                Field {
                    node_range: field.range(),
                    value_range: {
                        let mut parts = leaf.children().filter(|child| child.is_text());
                        parts.next().map(|first| {
                            first.range().start
                                ..parts
                                    .next_back()
                                    .map_or(first.range().end, |last| last.range().end)
                        })
                    },
                    value: crate::arxml::xml::literal_text(leaf)?,
                    dest: leaf.attribute("DEST").map(str::to_owned),
                },
            ))
        })
}

fn standard_scalar(node: Node<'_, '_>, field: &str) -> Option<ValueKind> {
    match (node.tag_name().name(), field) {
        ("CAN-FRAME-TRIGGERING", "IDENTIFIER")
        | ("CAN-CLUSTER", "BAUDRATE")
        | ("CAN-FRAME", "FRAME-LENGTH")
        | ("I-SIGNAL-I-PDU", "LENGTH")
        | ("I-SIGNAL", "LENGTH")
        | ("I-SIGNAL-TO-I-PDU-MAPPING", "START-POSITION") => Some(ValueKind::Integer),
        _ => None,
    }
}

pub(super) fn normalized(kind: Option<ValueKind>, lexeme: &str) -> String {
    match kind {
        Some(ValueKind::Integer) => crate::definitions::integer(lexeme.trim())
            .map_or_else(|_| lexeme.to_owned(), |number| number.to_string()),
        Some(ValueKind::Boolean) => match lexeme.trim() {
            "true" | "1" => "true".into(),
            "false" | "0" => "false".into(),
            _ => lexeme.into(),
        },
        Some(ValueKind::Enumeration | ValueKind::Reference) => lexeme.trim().into(),
        _ => lexeme.into(),
    }
}

pub(super) fn semantic_value(kind: Option<ValueKind>, lexeme: &str, dest: Option<&str>) -> String {
    let value = normalized(kind, lexeme);
    match dest {
        Some(dest) => format!("{dest} {value}"),
        None => value,
    }
}

pub(super) fn within(path: &str, root: &str) -> bool {
    path == root
        || path
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

pub(super) fn migrated_path(path: &str, migrations: &[PathMigration]) -> Option<String> {
    let migration = migrations
        .iter()
        .filter(|migration| within(path, &migration.previous.path))
        .max_by_key(|migration| migration.previous.path.len())?;
    Some(format!(
        "{}{}",
        migration.current.path,
        &path[migration.previous.path.len()..]
    ))
}

pub(super) fn reference_element(node: Node<'_, '_>) -> bool {
    node.is_element()
        && (node.tag_name().name().ends_with("-REF") || node.tag_name().name().ends_with("-TREF"))
}

pub(super) fn replay_migrations(path: &str, history: &[Vec<PathMigration>]) -> String {
    history.iter().fold(path.to_owned(), |path, revision| {
        migrated_path(&path, revision).unwrap_or(path)
    })
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[derive(Default)]
pub(super) struct Resolution {
    text: BTreeMap<String, String>,
    pub(super) semantics: BTreeMap<LayerTarget, SemanticEntry>,
}

fn selected_operations(
    active: &[(Vec<String>, &ProjectLayer)],
) -> Result<BTreeMap<LayerTarget, LayerAction>, Vec<PlanDiagnostic>> {
    let mut operations = BTreeMap::new();
    for (layer, config) in active {
        let mut seen = BTreeSet::new();
        for operation in &config.operations {
            if !seen.insert(&operation.target) {
                return Err(vec![diagnostic(
                    "LAYER_OPERATION_DUPLICATE",
                    &operation.target.object.path,
                    format!("The {layer:?} layer operates on the same target more than once."),
                )]);
            }
            if layer[0] == "variant"
                && let Some((origin, previous)) = operations.get(&operation.target)
                && origin != "project"
                && previous != &operation.action
            {
                return Err(vec![diagnostic(
                    "VARIANT_OPERATION_CONFLICT",
                    &operation.target.object.path,
                    "Selected variants specify different operations for this target.",
                )]);
            }
            operations.insert(
                operation.target.clone(),
                (layer.join("/"), operation.action.clone()),
            );
        }
    }
    Ok(operations
        .into_iter()
        .map(|(target, (_, action))| (target, action))
        .collect())
}

pub(super) fn resolve(
    layers: &ProjectLayers,
    sources: &[InputSource],
    catalog: &crate::definitions::DefinitionCatalog,
) -> Result<Resolution, Vec<PlanDiagnostic>> {
    let active = layers.active()?;
    let operations = selected_operations(&active)?;
    let excluded_modules: BTreeSet<_> = operations
        .iter()
        .filter(|(_, action)| **action == LayerAction::Exclude)
        .map(|(target, _)| target.object.path.as_str())
        .collect();
    let mut origins = BTreeMap::new();
    for member in &layers.base.members {
        origins.insert(member.as_str(), vec!["base".to_owned()]);
    }
    for (layer, config) in &active {
        for member in &config.members {
            origins.insert(member.as_str(), layer.clone());
        }
    }
    let documents = sources
        .iter()
        .filter(|source| origins.contains_key(source.logical_path()))
        .map(|source| {
            let error = |message| vec![diagnostic("LAYER_XML", source.logical_path(), message)];
            let text = std::str::from_utf8(source.bytes())
                .map_err(|failure| error(failure.to_string()))?;
            let document = Document::parse(text).map_err(|failure| error(failure.to_string()))?;
            Ok((source, document))
        })
        .collect::<Result<Vec<_>, Vec<PlanDiagnostic>>>()?;
    let reference_types = documents
        .iter()
        .flat_map(|(_, document)| {
            document
                .descendants()
                .filter(|node| is_named(*node))
                .map(|node| (path_of(node), node.tag_name().name().to_owned()))
        })
        .collect();
    let mut result = Resolution::default();
    let mut known = BTreeSet::new();
    let mut applied_fields = BTreeSet::new();
    let mut inbound = Vec::new();
    let mut deleted = BTreeSet::new();
    let mut live = BTreeSet::new();
    let mut spellings = BTreeMap::new();
    let manifest_digest = serde_json::to_vec(layers)
        .map(|bytes| hex::encode(Sha256::digest(bytes)))
        .map_err(|error| {
            vec![diagnostic(
                "LAYER_SERIALIZATION",
                "layers",
                error.to_string(),
            )]
        })?;
    for (source, doc) in documents {
        let path = source.logical_path();
        let layer = &origins[path];
        let text = doc.input_text();
        let digest = source.sha256();
        let origin = |range: std::ops::Range<usize>| LayerOrigin {
            layer: layer.clone(),
            file: path.into(),
            sha256: digest.clone(),
            byte_range: Some((range.start, range.end)),
            operation: None,
        };
        let operation_origin = |target: &LayerTarget| {
            active.iter().rev().find_map(|(layer, config)| {
                let index = config
                    .operations
                    .iter()
                    .position(|operation| &operation.target == target)?;
                Some(LayerOrigin {
                    layer: layer.clone(),
                    file: "workbench-project.json".into(),
                    sha256: manifest_digest.clone(),
                    byte_range: None,
                    operation: Some(index),
                })
            })
        };
        let mut patches = Vec::new();
        let mut removals = Vec::new();
        for node in doc
            .descendants()
            .filter(|node| is_named(*node) || is_instance_reference(*node))
        {
            let id = object(node);
            let target = LayerTarget {
                object: id.clone(),
                field: None,
            };
            known.insert(target.clone());
            if removals
                .iter()
                .any(|range: &std::ops::Range<usize>| range.contains(&node.range().start))
            {
                if is_named(node) {
                    deleted.insert(id.path);
                }
                continue;
            }
            let action = operations.get(&target);
            if let Some(action) = action {
                if matches!(action, LayerAction::Set { .. })
                    || matches!(action, LayerAction::Exclude)
                        && !element(node, "ECUC-MODULE-CONFIGURATION-VALUES")
                {
                    return Err(vec![diagnostic(
                        "LAYER_OBJECT_OPERATION",
                        &id.path,
                        "This operation cannot be applied to the object.",
                    )]);
                }
                let range = node.range();
                removals.push(range.clone());
                patches.push(Patch {
                    range,
                    value: String::new(),
                });
                if is_named(node) {
                    deleted.insert(id.path);
                }
                result.semantics.insert(
                    target.clone(),
                    SemanticEntry {
                        values: Vec::new(),
                        sources: operation_origin(&target).into_iter().collect(),
                        operation: Some(action.clone()),
                    },
                );
                continue;
            }
            if is_named(node) {
                live.insert(id.path.clone());
            }
            if let Some(previous) = spellings.insert(id.path.to_ascii_lowercase(), id.path.clone())
                && previous != id.path
            {
                return Err(vec![diagnostic(
                    "LAYER_OBJECT_NAME",
                    &id.path,
                    format!("Object name collides with {previous}."),
                )]);
            }
            let mut attributes = Vec::new();
            for child in node
                .children()
                .filter(|child| child.is_element() && child.tag_name().namespace() == Some(NS))
            {
                let name = child.tag_name().name();
                if matches!(
                    name,
                    "SHORT-NAME" | "DEFINITION-REF" | "PARAMETER-VALUES" | "REFERENCE-VALUES"
                ) {
                    continue;
                }
                let Some(kind) = standard_scalar(node, name) else {
                    collect_attributes(child, &mut attributes);
                    continue;
                };
                let scalar = LayerTarget {
                    object: id.clone(),
                    field: Some(name.into()),
                };
                let action = operations.get(&scalar);
                let value = if let Some(LayerAction::Set {
                    values,
                    default_origin: None,
                }) = action
                    && let [value] = values.as_slice()
                    && value.kind == kind
                    && crate::definitions::integer(&value.lexeme).is_ok()
                {
                    crate::arxml::patch_text(child, xml_escape(&value.lexeme), &mut patches)
                        .map_err(|message| {
                            vec![diagnostic("LAYER_FIELD_OPERATION", &id.path, message)]
                        })?;
                    value.lexeme.clone()
                } else if action.is_some() {
                    return Err(vec![diagnostic(
                        "LAYER_FIELD_OPERATION",
                        &id.path,
                        format!("{name} requires a single integer."),
                    )]);
                } else {
                    crate::arxml::xml::literal_text(child).unwrap_or_default()
                };
                known.insert(scalar.clone());
                result.semantics.insert(
                    scalar.clone(),
                    SemanticEntry {
                        values: vec![normalized(Some(kind), &value)],
                        sources: operation_origin(&scalar)
                            .into_iter()
                            .chain(action.is_none().then(|| origin(child.range())))
                            .collect(),
                        operation: action.cloned(),
                    },
                );
            }
            let row = result
                .semantics
                .entry(target)
                .or_insert_with(|| SemanticEntry {
                    values: Vec::new(),
                    sources: Vec::new(),
                    operation: None,
                });
            row.values.extend(attributes);
            row.sources.push(origin(node.range()));
            let mut fields: BTreeMap<String, Vec<Field>> = BTreeMap::new();
            for (id, field) in field_nodes(node) {
                fields.entry(id).or_default().push(field);
            }
            for target in operations.keys().filter(|target| target.object == id) {
                if let Some(field) = &target.field
                    && standard_scalar(node, field).is_none()
                {
                    fields.entry(field.clone()).or_default();
                }
            }
            for (field, entries) in fields {
                let target = LayerTarget {
                    object: id.clone(),
                    field: Some(field.clone()),
                };
                let id = field;
                known.insert(target.clone());
                let kind = catalog.get(&id).and_then(|definition| definition.kind);
                let action = operations.get(&target);
                let row = result
                    .semantics
                    .entry(target.clone())
                    .or_insert_with(|| SemanticEntry {
                        values: Vec::new(),
                        sources: Vec::new(),
                        operation: action.cloned(),
                    });
                if let Some(action) = action {
                    let first = applied_fields.insert(target.clone());
                    let edit = if first { action } else { &LayerAction::Delete };
                    apply_field(
                        node,
                        &id,
                        (&entries, &reference_types),
                        edit,
                        catalog,
                        &mut patches,
                        first,
                    )
                    .map_err(|message| {
                        vec![diagnostic(
                            "LAYER_FIELD_OPERATION",
                            &format!("{}:{id}", target.object.path),
                            message,
                        )]
                    })?;
                    removals.extend(entries.iter().map(|entry| entry.node_range.clone()));
                    if first && let LayerAction::Set { values, .. } = action {
                        let descriptor = catalog.get(&id);
                        row.values.extend(values.iter().map(|value| {
                            let dest = descriptor.and_then(|definition| {
                                reference_destination(
                                    value,
                                    &entries,
                                    &reference_types,
                                    &definition.reference_destinations,
                                )
                            });
                            semantic_value(kind, &value.lexeme, dest)
                        }));
                        if kind == Some(ValueKind::Reference) {
                            inbound.extend(values.iter().map(|value| {
                                (
                                    path.to_owned(),
                                    target.object.path.clone(),
                                    id.clone(),
                                    value.lexeme.trim().to_owned(),
                                )
                            }));
                        }
                    }
                    if first {
                        row.sources.extend(operation_origin(&target));
                    }
                } else {
                    row.values.extend(
                        entries
                            .iter()
                            .map(|field| semantic_value(kind, &field.value, field.dest.as_deref())),
                    );
                    row.sources
                        .extend(entries.iter().map(|field| origin(field.node_range.clone())));
                    if matches!(kind, Some(ValueKind::Integer | ValueKind::Boolean)) {
                        for field in &entries {
                            let value = normalized(kind, &field.value);
                            if value != field.value
                                && let Some(range) = &field.value_range
                            {
                                patches.push(Patch {
                                    range: range.clone(),
                                    value,
                                });
                            }
                        }
                    }
                }
            }
        }
        for node in doc.descendants().filter(|node| reference_element(*node)) {
            if removals
                .iter()
                .any(|range| range.contains(&node.range().start))
            {
                continue;
            }
            if let Some(value) = crate::arxml::xml::literal_text(node) {
                let mut target = value.trim().to_owned();
                if element(node, "ECUC-MODULE-CONFIGURATION-VALUES-REF")
                    && excluded_modules.contains(target.as_str())
                    && let Some(entry) = node.parent_element().filter(|entry| {
                        element(*entry, "ECUC-MODULE-CONFIGURATION-VALUES-REF-CONDITIONAL")
                    })
                {
                    patches.push(Patch {
                        range: entry.range(),
                        value: String::new(),
                    });
                    continue;
                }
                let current = if layer[0] == "base" {
                    target.clone()
                } else {
                    replay_migrations(&target, &layers.applied_migrations)
                };
                if current != target {
                    if node.tag_name().namespace() != Some(NS) || node.attribute("DEST").is_none() {
                        return Err(vec![diagnostic(
                            "BASE_MIGRATION_REFERENCE_OPAQUE",
                            &path_of(node),
                            format!("Cannot qualify {} to {target}.", node.tag_name().name()),
                        )]);
                    }
                    crate::arxml::patch_text(node, xml_escape(&current), &mut patches).map_err(
                        |message| {
                            vec![diagnostic(
                                "BASE_MIGRATION_REFERENCE_OPAQUE",
                                &path_of(node),
                                message,
                            )]
                        },
                    )?;
                    target = current;
                }
                inbound.push((
                    path.to_owned(),
                    path_of(node),
                    node.tag_name().name().to_owned(),
                    target,
                ));
            }
        }
        let mut effective = text.to_owned();
        apply_patches(&mut effective, &mut patches)
            .map_err(|message| vec![diagnostic("LAYER_PATCH", path, message)])?;
        if !patches.is_empty() {
            remove_empty_ecuc_lists(&mut effective)
                .map_err(|message| vec![diagnostic("LAYER_PATCH", path, message)])?;
        }
        result.text.insert(path.to_owned(), effective);
    }
    let mut diagnostics = Vec::new();
    for target in operations.keys().filter(|target| !known.contains(*target)) {
        if !deleted.iter().any(|path| within(&target.object.path, path)) {
            diagnostics.push(diagnostic(
                "LAYER_TARGET_UNKNOWN",
                &target.object.path,
                "The operation target is absent from the selected inputs.",
            ));
        }
    }
    for (file, owner, field, target) in inbound {
        if !live.contains(&target) && deleted.iter().any(|path| within(&target, path)) {
            let mut issue = diagnostic(
                "LAYER_DELETE_REFERENCED",
                &owner,
                format!("{field} still references {target}."),
            );
            issue.file = Some(file);
            diagnostics.push(issue);
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    split::merge(&mut result.text)?;
    Ok(result)
}

fn remove_empty_ecuc_lists(text: &mut String) -> Result<(), LocalizedText> {
    let document = Document::parse(text).map_err(|error| error.to_string())?;
    let mut patches = document
        .descendants()
        .filter(|node| {
            matches!(
                node.tag_name().name(),
                "CONTAINERS" | "SUB-CONTAINERS" | "PARAMETER-VALUES" | "REFERENCE-VALUES"
            ) && node.tag_name().namespace() == Some(NS)
                && !node.children().any(|child| child.is_element())
        })
        .map(|node| Patch {
            range: node.range(),
            value: String::new(),
        })
        .collect();
    apply_patches(text, &mut patches)
}

fn collect_attributes(node: Node<'_, '_>, attributes: &mut Vec<String>) {
    if is_named(node) || is_instance_reference(node) || node.tag_name().namespace() != Some(NS) {
        return;
    }
    if let Some(value) = crate::arxml::xml::literal_text(node) {
        if !value.trim().is_empty() {
            let tag = node.tag_name().name();
            let dest = node.attribute("DEST").unwrap_or_default();
            attributes.push(format!("{tag} {dest} {}", value.trim()));
        }
    } else {
        for child in node.children().filter(|child| child.is_element()) {
            collect_attributes(child, attributes);
        }
    }
}

fn reference_destination<'a>(
    value: &TypedValue,
    entries: &'a [Field],
    types: &'a BTreeMap<String, String>,
    allowed: &'a [String],
) -> Option<&'a str> {
    if value.kind != ValueKind::Reference {
        return None;
    }
    let path = value.lexeme.trim();
    types
        .get(path)
        .map(String::as_str)
        .or_else(|| {
            entries
                .iter()
                .find(|entry| entry.value.trim() == path)
                .and_then(|entry| entry.dest.as_deref())
        })
        .or_else(|| (allowed.len() == 1).then(|| allowed[0].as_str()))
}

fn apply_field(
    node: Node<'_, '_>,
    id: &str,
    (entries, reference_types): (&[Field], &BTreeMap<String, String>),
    action: &LayerAction,
    catalog: &crate::definitions::DefinitionCatalog,
    patches: &mut Vec<Patch>,
    first: bool,
) -> Result<(), LocalizedText> {
    let descriptor = catalog
        .get(id)
        .ok_or_else(|| format!("Unknown field definition {id}."))?;
    let kind = descriptor
        .kind
        .ok_or("The target is not a field definition.")?;
    if !catalog
        .children(&definition(node).unwrap_or_default())
        .iter()
        .any(|child| child.definition_id == id)
    {
        return Err(format!("Field {id} does not belong to this object definition.").into());
    }
    if catalog.is_published_information(id) {
        return Err(
            format!("Field {id} is Published Information and cannot be overridden.").into(),
        );
    }
    if !descriptor.writable {
        return Err(format!("Field {id} is read-only.").into());
    }
    let field_tag = crate::arxml::projection::field_element(kind);
    let (list, leaf, def_dest) = if kind == ValueKind::Reference {
        (
            "REFERENCE-VALUES",
            "VALUE-REF",
            descriptor.element_kind.as_str(),
        )
    } else {
        (
            "PARAMETER-VALUES",
            "VALUE",
            descriptor.element_kind.as_str(),
        )
    };
    let replacement = match action {
        LayerAction::Delete if !first || descriptor.lower_multiplicity == 0 => String::new(),
        LayerAction::Delete => return Err("A required field cannot be deleted.".into()),
        LayerAction::Exclude => {
            return Err("Exclude applies only to a module configuration.".into());
        }
        LayerAction::Set {
            values,
            default_origin,
        } => {
            let count = values.len();
            if count == 0
                || count < descriptor.lower_multiplicity as usize
                || descriptor
                    .upper_multiplicity
                    .is_some_and(|upper| count > upper as usize)
            {
                return Err("The replacement list violates field multiplicity.".into());
            }
            if default_origin.is_some()
                && (values.len() != 1
                    || descriptor.default_value.as_ref() != values.first()
                    || descriptor.default_origin.as_ref() != default_origin.as_ref())
            {
                return Err("The recorded default does not match the definition default.".into());
            }
            let mut text = String::new();
            for value in values {
                catalog.validate_value(id, value)?;
                let attribute = if kind == ValueKind::Reference {
                    let dest = reference_destination(
                        value,
                        entries,
                        reference_types,
                        &descriptor.reference_destinations,
                    )
                    .ok_or("The reference destination type is ambiguous.")?;
                    format!(" DEST=\"{}\"", xml_escape(dest))
                } else {
                    String::new()
                };
                text.push_str(&format!("<{field_tag}><DEFINITION-REF DEST=\"{def_dest}\">{}</DEFINITION-REF><{leaf}{attribute}>{}</{leaf}></{field_tag}>", xml_escape(id), xml_escape(&normalized(Some(kind), &value.lexeme))));
            }
            text
        }
    };
    if let Some((first, rest)) = entries.split_first() {
        patches.push(Patch {
            range: first.node_range.clone(),
            value: replacement,
        });
        patches.extend(rest.iter().map(|entry| Patch {
            range: entry.node_range.clone(),
            value: String::new(),
        }));
    } else if !replacement.is_empty() {
        if let Some(list_node) = node.children().find(|child| element(*child, list)) {
            let range = list_node.range();
            let input = list_node.document().input_text();
            if input[range.clone()].ends_with("/>") {
                patches.push(Patch {
                    range,
                    value: format!("<{list}>{replacement}</{list}>"),
                });
            } else {
                let end = range.end - list.len() - 3;
                patches.push(Patch {
                    range: end..end,
                    value: replacement,
                });
            }
        } else {
            let boundary = match list {
                "PARAMETER-VALUES" => &["REFERENCE-VALUES", "SUB-CONTAINERS"][..],
                _ => &["SUB-CONTAINERS"][..],
            };
            let end = node
                .children()
                .find(|child| boundary.iter().any(|tag| element(*child, tag)))
                .map_or_else(
                    || node.range().end - node.tag_name().name().len() - 3,
                    |child| child.range().start,
                );
            patches.push(Patch {
                range: end..end,
                value: format!("<{list}>{replacement}</{list}>"),
            });
        }
    }
    Ok(())
}

pub(crate) fn resolve_manifest_sources(
    manifest: &crate::arxml::ProjectManifest,
    sources: &mut [InputSource],
    catalog: &crate::definitions::DefinitionCatalog,
    identity: &str,
) -> Result<(), Vec<PlanDiagnostic>> {
    if let Some(layers) = &manifest.layers {
        let resolution = resolve(layers, sources, catalog)?;
        for source in sources {
            let text = resolution.text.get(source.logical_path());
            let effective = text
                .filter(|text| text.as_bytes() != source.bytes())
                .cloned();
            source.bind_project(effective, text.is_none(), identity.to_owned());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migration(previous: &str, current: &str) -> PathMigration {
        let object = |path: &str| LayerObject {
            release: "R24-11".into(),
            definition_type: "ECUC-CONTAINER-VALUE".into(),
            path: path.into(),
            instance_context: Vec::new(),
        };
        PathMigration {
            previous: object(previous),
            current: object(current),
        }
    }

    #[test]
    fn replay_preserves_nested_moves_across_revisions() {
        let revisions = vec![vec![migration("/A", "/B")], vec![migration("/B/X", "/B/Y")]];
        assert_eq!(replay_migrations("/A/X/Child", &revisions), "/B/Y/Child");
        assert_eq!(replay_migrations("/A/Other", &revisions), "/B/Other");
    }

    #[test]
    fn replacement_looks_up_each_reference_target_independently() {
        let types = BTreeMap::from([
            ("/Application/Timing".into(), "TIMING-EVENT".into()),
            ("/Application/Init".into(), "INIT-EVENT".into()),
        ]);
        for (path, kind) in [
            (" /Application/Timing ", "TIMING-EVENT"),
            ("/Application/Init", "INIT-EVENT"),
        ] {
            let value = TypedValue {
                kind: ValueKind::Reference,
                lexeme: path.into(),
            };
            assert_eq!(reference_destination(&value, &[], &types, &[]), Some(kind));
            assert_eq!(
                semantic_value(Some(value.kind), &value.lexeme, Some(kind)),
                format!("{kind} {}", path.trim())
            );
        }
    }
}
