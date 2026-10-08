use super::projection::{IndexedField, IndexedObject};
use super::*;
use crate::project_model::*;
use std::sync::Arc;

struct Candidate {
    workspace: Workspace,
    impacts: Vec<ChangeImpact>,
    created: Vec<CreatedObject>,
    fields: Vec<CreatedField>,
    selection: Option<String>,
}

fn change_id(change: &ConfigurationChange) -> &str {
    match change {
        ConfigurationChange::SetValue { change_id, .. }
        | ConfigurationChange::SetReference { change_id, .. }
        | ConfigurationChange::CreateInstance { change_id, .. }
        | ConfigurationChange::RenameInstance { change_id, .. }
        | ConfigurationChange::RemoveInstance { change_id, .. } => change_id,
    }
}

fn object_id(
    reference: &ObjectRef,
    created: &BTreeMap<String, String>,
) -> Result<String, crate::message::LocalizedText> {
    match reference {
        ObjectRef::Existing { object_id } => Ok(object_id.clone()),
        ObjectRef::Created { change_id } => created.get(change_id).cloned().ok_or_else(|| {
            crate::product_message!(
                "backend.arxml.changes.unknown_or_cyclic_create_dependency",
                "change_id" => change_id
            )
        }),
    }
}

fn object<'a>(
    workspace: &'a Workspace,
    id: &str,
) -> Result<&'a IndexedObject, crate::message::LocalizedText> {
    workspace
        .snapshot
        .object_by_id
        .get(id)
        .map(|index| &workspace.snapshot.objects[*index])
        .ok_or_else(|| {
            crate::product_message!(
                "backend.arxml.changes.object_not_in_workspace",
                "object_id" => id
            )
        })
}

fn escape(text: &str) -> Result<String, crate::message::LocalizedText> {
    if text.chars().any(|ch| !matches!(ch, '\u{9}' | '\u{a}' | '\u{d}' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')) {
        return Err(crate::product_message!(
            "backend.arxml.changes.invalid_xml_unicode"
        ));
    }
    Ok(text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;"))
}

pub(super) fn group_insert(
    text: &str,
    object_range: std::ops::Range<usize>,
    group: &str,
    xml: &str,
    patches: &mut Vec<Patch>,
) -> Result<(), crate::message::LocalizedText> {
    let document = Document::parse(text).map_err(|error| error.to_string())?;
    let node = document
        .descendants()
        .find(|node| node.is_element() && node.range() == object_range)
        .ok_or_else(|| crate::product_message!("backend.arxml.changes.insertion_owner_missing"))?;
    let container = node
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == group);
    let location = container.unwrap_or(node);
    let prefix = if location.lookup_namespace_uri(None) == Some(NS) {
        None
    } else {
        location.lookup_prefix(NS)
    };
    let xml = qualify_fragment(xml, prefix);
    let group_name = prefix
        .map(|prefix| format!("{prefix}:{group}"))
        .unwrap_or_else(|| group.into());
    if let Some(container) = container {
        let range = container.range();
        let original = &text[range.clone()];
        if original.trim_end().ends_with("/>") {
            let name = original[1..]
                .split(|ch: char| ch.is_whitespace() || ch == '/' || ch == '>')
                .next()
                .ok_or_else(|| {
                    crate::product_message!("backend.arxml.changes.group_tag_name_missing")
                })?;
            let close = original.rfind("/>").ok_or_else(|| {
                crate::product_message!("backend.arxml.changes.self_closing_group_end_missing")
            })?;
            patches.push(Patch {
                range,
                value: format!("{}>{xml}</{name}>", &original[..close]),
            });
        } else {
            let offset = range.start
                + original.rfind("</").ok_or_else(|| {
                    crate::product_message!("backend.arxml.changes.container_closing_tag_missing")
                })?;
            patches.push(Patch {
                range: offset..offset,
                value: xml.into_owned(),
            });
        }
    } else {
        // Headers precede value groups; module headers also include the edition
        // and implementation variant. Existing byte order is never rearranged.
        let rank = |name: &str| match node.tag_name().name() {
            "AR-PACKAGE" => match name {
                "ELEMENTS" => 1,
                "AR-PACKAGES" => 2,
                _ => 0,
            },
            "ECUC-MODULE-CONFIGURATION-VALUES" => u8::from(name == "CONTAINERS"),
            _ => match name {
                "PARAMETER-VALUES" => 1,
                "REFERENCE-VALUES" => 2,
                "SUB-CONTAINERS" => 3,
                _ => 0,
            },
        };
        let offset = node
            .children()
            .filter(|child| child.is_element())
            .find(|child| rank(child.tag_name().name()) > rank(group))
            .map(|child| child.range().start)
            .unwrap_or(
                object_range.start
                    + text[object_range.clone()].rfind("</").ok_or_else(|| {
                        crate::product_message!("backend.arxml.changes.object_closing_tag_missing")
                    })?,
            );
        patches.push(Patch {
            range: offset..offset,
            value: format!("<{group_name}>{xml}</{group_name}>"),
        });
    }
    Ok(())
}

fn qualify_fragment<'a>(xml: &'a str, prefix: Option<&str>) -> std::borrow::Cow<'a, str> {
    let Some(prefix) = prefix else {
        return std::borrow::Cow::Borrowed(xml);
    };
    let mut qualified =
        String::with_capacity(xml.len() + xml.matches('<').count() * (prefix.len() + 1));
    let mut characters = xml.chars().peekable();
    while let Some(character) = characters.next() {
        qualified.push(character);
        if character == '<' {
            if characters.peek() == Some(&'/') {
                qualified.push(characters.next().unwrap());
            }
            if characters
                .peek()
                .is_some_and(|character| character.is_ascii_alphabetic())
            {
                qualified.push_str(prefix);
                qualified.push(':');
            }
        }
    }
    std::borrow::Cow::Owned(qualified)
}

fn apply_local(
    workspace: &mut Workspace,
    source: usize,
    patches: &mut Vec<Patch>,
) -> Result<(), crate::message::LocalizedText> {
    // Retire removed entry IDs before rebuilding; repeated siblings must not inherit
    // the identity of a deleted earlier entry merely because their ordinal shifted.
    let snapshot = Arc::make_mut(&mut workspace.snapshot);
    snapshot.fields.retain(|field| {
        field.source != source
            || field.entry_range.as_ref().is_none_or(|range| {
                !patches.iter().any(|patch| {
                    patch.value.is_empty()
                        && patch.range.start <= range.start
                        && patch.range.end >= range.end
                })
            })
    });
    let mut ordinals = BTreeMap::new();
    for field in &mut snapshot.fields {
        if field.entry_range.is_some() {
            let key = (
                field.view.object_id.clone(),
                field.view.definition.definition_id.clone(),
                field.element.clone(),
            );
            let ordinal = ordinals.entry(key).or_insert(0);
            field.ordinal = *ordinal;
            *ordinal += 1;
        }
    }
    apply_patches(&mut workspace.files[source].text, patches)?;
    workspace.refresh()
}

fn impact(
    change: &str,
    workspace: &Workspace,
    owner: &IndexedObject,
    field: Option<&IndexedField>,
    before: String,
    after: String,
    incoming: bool,
) -> ChangeImpact {
    ChangeImpact {
        change_id: change.into(),
        source_id: workspace.snapshot.sources[owner.source].source_id.clone(),
        object_id: Some(owner.view.object_id.clone()),
        field_id: field.map(|field| field.view.field_id.clone()),
        path: owner.view.path.clone(),
        before,
        after,
        incoming,
    }
}

fn value_patch(
    workspace: &Workspace,
    field: &IndexedField,
    lexeme: &str,
) -> Result<Patch, crate::message::LocalizedText> {
    if let Some(range) = &field.value_range {
        return Ok(Patch {
            range: range.clone(),
            value: escape(lexeme)?,
        });
    }
    let document =
        Document::parse(&workspace.files[field.source].text).map_err(|error| error.to_string())?;
    let entry = document
        .descendants()
        .find(|node| node.is_element() && field.entry_range.as_ref() == Some(&node.range()))
        .ok_or_else(|| crate::product_message!("backend.arxml.changes.value_entry_missing"))?;
    let leaf = entry
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "VALUE")
        .ok_or_else(|| crate::product_message!("backend.arxml.changes.simple_value_missing"))?;
    let original = &workspace.files[field.source].text[leaf.range()];
    if !original.ends_with("/>") || leaf.children().any(|node| !node.is_text()) {
        return Err(crate::product_message!(
            "backend.arxml.changes.existing_value_content_uneditable"
        ));
    }
    let name = original[1..]
        .split(|ch: char| ch.is_whitespace() || ch == '/' || ch == '>')
        .next()
        .ok_or_else(|| crate::product_message!("backend.arxml.changes.value_tag_name_missing"))?;
    Ok(Patch {
        range: leaf.range(),
        value: format!(
            "{}>{}</{name}>",
            &original[..original.len() - 2],
            escape(lexeme)?
        ),
    })
}

impl Workspace {
    pub fn prepare_change(
        &self,
        changes: &ChangeSet,
    ) -> Result<ChangePreview, crate::message::LocalizedText> {
        let mut candidate = self.change_candidate(changes)?;
        let change_revision = self.change_revision(changes, &candidate)?;
        let created: BTreeMap<_, _> = candidate
            .created
            .iter()
            .map(|item| (item.object_id.as_str(), item.change_id.as_str()))
            .collect();
        let new_fields: BTreeSet<_> = candidate
            .workspace
            .snapshot
            .fields
            .iter()
            .filter(|field| !self.snapshot.field_by_id.contains_key(&field.view.field_id))
            .map(|field| field.view.field_id.as_str())
            .collect();
        for impact in &mut candidate.impacts {
            if impact
                .object_id
                .as_deref()
                .is_some_and(|id| created.contains_key(id))
            {
                impact.object_id = None;
            }
            if impact
                .field_id
                .as_deref()
                .is_some_and(|id| id.is_empty() || new_fields.contains(id))
            {
                impact.field_id = None;
            }
        }
        let mut diagnostics: Vec<_> = candidate
            .workspace
            .snapshot
            .validation
            .iter()
            .flat_map(|scope| scope.diagnostics.clone())
            .collect();
        for diagnostic in &mut diagnostics {
            if diagnostic
                .object_id
                .as_deref()
                .is_some_and(|id| created.contains_key(id))
            {
                diagnostic.object_id = None;
            }
            if diagnostic
                .field_id
                .as_deref()
                .is_some_and(|id| new_fields.contains(id))
            {
                diagnostic.field_id = None;
            }
            if let Some(witness) = &mut diagnostic.witness {
                witness.subjects = witness
                    .subjects
                    .iter()
                    .filter_map(|subject| {
                        if let Some(change) = created.get(subject.as_str()) {
                            Some(format!("created:{change}"))
                        } else if new_fields.contains(subject.as_str()) {
                            None
                        } else {
                            Some(subject.clone())
                        }
                    })
                    .collect();
            }
        }
        Ok(ChangePreview {
            change_revision,
            input_fingerprint: changes.input_fingerprint.clone(),
            definition_fingerprint: self.definition_fingerprint()?,
            impacts: candidate.impacts,
            diagnostics,
        })
    }

    pub fn apply_change(
        &mut self,
        changes: &ChangeSet,
        revision: &str,
    ) -> Result<ChangeOutcome, crate::message::LocalizedText> {
        let mut candidate = self.change_candidate(changes)?;
        if self.change_revision(changes, &candidate)? != revision {
            return Err(crate::product_message!(
                "backend.arxml.changes.stale_change_preview"
            ));
        }
        if self
            .files
            .iter()
            .zip(&candidate.workspace.files)
            .any(|(before, after)| before.text != after.text)
        {
            candidate.workspace.revision = self.revision.checked_add(1).ok_or_else(|| {
                crate::product_message!("backend.arxml.changes.workspace_revision_overflow")
            })?;
        }
        let fingerprint = candidate.workspace.input_fingerprint()?;
        let projection = candidate.workspace.project_projection(&fingerprint)?;
        let outcome = ChangeOutcome {
            projection,
            selection_id: candidate.selection,
            created_ids: candidate.created,
            created_fields: candidate.fields,
        };
        *self = candidate.workspace;
        Ok(outcome)
    }

    fn change_revision(
        &self,
        changes: &ChangeSet,
        candidate: &Candidate,
    ) -> Result<String, crate::message::LocalizedText> {
        let mut normalized = changes.clone();
        normalized
            .changes
            .sort_by(|left, right| change_id(left).cmp(change_id(right)));
        let mut digest = Sha256::new();
        for text in [
            self.input_fingerprint()?,
            serde_json::to_string(&normalized).map_err(|error| error.to_string())?,
            serde_json::to_string(&candidate.impacts).map_err(|error| error.to_string())?,
            candidate.workspace.save_revision(),
        ] {
            digest.update((text.len() as u64).to_le_bytes());
            digest.update(text.as_bytes());
        }
        Ok(format!("{:x}", digest.finalize()))
    }

    fn change_candidate(
        &self,
        changes: &ChangeSet,
    ) -> Result<Candidate, crate::message::LocalizedText> {
        self.ensure_sources_current()?;
        // AppState validates both Session-token echoes before entering this core
        // transaction. Confirmation independently binds actual bytes/revision/catalog.
        if changes.workspace_epoch != self.epoch
            || changes.input_fingerprint.is_empty()
            || changes.definition_fingerprint != self.definition_fingerprint()?
        {
            return Err(crate::product_message!(
                "backend.arxml.changes.stale_change_set_identity"
            ));
        }
        let mut ids = BTreeSet::new();
        let mut targets = BTreeSet::new();
        let mut entry_keys = BTreeSet::new();
        for change in &changes.changes {
            if change_id(change).is_empty() || !ids.insert(change_id(change)) {
                return Err(crate::product_message!(
                    "backend.arxml.changes.change_id_required_unique"
                ));
            }
            let target = match change {
                ConfigurationChange::SetValue { field, .. }
                | ConfigurationChange::SetReference { field, .. } => {
                    if let FieldRef::New { entry_key, .. } = field {
                        if entry_key.is_empty() || !entry_keys.insert(entry_key) {
                            return Err(crate::product_message!(
                                "backend.arxml.changes.new_entry_key_required_unique"
                            ));
                        }
                    }
                    format!(
                        "field:{}",
                        serde_json::to_string(field).map_err(|error| error.to_string())?
                    )
                }
                ConfigurationChange::RenameInstance { object, .. }
                | ConfigurationChange::RemoveInstance { object, .. } => format!(
                    "object:{}",
                    serde_json::to_string(object).map_err(|error| error.to_string())?
                ),
                ConfigurationChange::CreateInstance { .. } => continue,
            };
            if !targets.insert(target) {
                return Err(crate::product_message!(
                    "backend.arxml.changes.conflicting_target_operations"
                ));
            }
        }
        // Normalized traversal makes previews independent of caller array ordering.
        let mut ordered: Vec<_> = changes.changes.iter().collect();
        ordered.sort_by(|left, right| change_id(left).cmp(change_id(right)));
        let mut candidate = Candidate {
            workspace: self.clone(),
            impacts: Vec::new(),
            created: Vec::new(),
            fields: Vec::new(),
            selection: None,
        };
        let mut created = BTreeMap::new();
        let mut remaining: Vec<_> = ordered
            .iter()
            .copied()
            .filter(|change| matches!(change, ConfigurationChange::CreateInstance { .. }))
            .collect();
        while !remaining.is_empty() {
            let mut progressed = false;
            let mut deferred = Vec::new();
            for change in remaining {
                let ConfigurationChange::CreateInstance {
                    change_id,
                    parent,
                    source_id,
                    definition_id,
                    short_name,
                } = change
                else {
                    unreachable!()
                };
                if matches!(parent, ObjectRef::Created { change_id } if !created.contains_key(change_id))
                {
                    deferred.push(change);
                    continue;
                }
                candidate
                    .create_instance(
                        change_id,
                        &object_id(parent, &created)?,
                        source_id,
                        definition_id,
                        short_name,
                    )
                    .map_err(|message| {
                        crate::message::LocalizedText::messages([
                            crate::product_message!(
                                "backend.arxml.changes.change_context",
                                "change_id" => change_id
                            ),
                            message,
                        ])
                    })?;
                created.insert(
                    change_id.clone(),
                    candidate.created.last().unwrap().object_id.clone(),
                );
                progressed = true;
            }
            if !progressed {
                return Err(crate::product_message!(
                    "backend.arxml.changes.unknown_or_cyclic_create_parent"
                ));
            }
            remaining = deferred;
        }
        // Compare every expected value with the original snapshot, never an earlier operation's output.
        for change in &ordered {
            match change {
                ConfigurationChange::SetValue {
                    field: FieldRef::Existing { field_id },
                    expected,
                    ..
                } => {
                    let original = self
                        .snapshot
                        .field_by_id
                        .get(field_id)
                        .map(|index| &self.snapshot.fields[*index])
                        .ok_or_else(|| {
                            crate::product_message!("backend.arxml.changes.stale_field_id")
                        })?;
                    if original.view.current != *expected {
                        return Err(crate::product_message!(
                            "backend.arxml.changes.expected_original_value_changed",
                            "change_id" => change_id(change)
                        ));
                    }
                }
                ConfigurationChange::SetReference {
                    field: FieldRef::Existing { field_id },
                    expected,
                    ..
                } => {
                    let original = self
                        .snapshot
                        .field_by_id
                        .get(field_id)
                        .map(|index| &self.snapshot.fields[*index])
                        .ok_or_else(|| {
                            crate::product_message!("backend.arxml.changes.stale_field_id")
                        })?;
                    if original.view.reference.as_ref() != Some(expected) {
                        return Err(crate::product_message!(
                            "backend.arxml.changes.expected_original_reference_changed",
                            "change_id" => change_id(change)
                        ));
                    }
                }
                _ => {}
            }
        }
        for change in &ordered {
            if let ConfigurationChange::RenameInstance {
                change_id,
                object,
                expected_short_name,
                short_name,
            } = change
            {
                let id = object_id(object, &created)?;
                candidate
                    .rename_instance(change_id, &id, expected_short_name, short_name)
                    .map_err(|message| {
                        crate::message::LocalizedText::messages([
                            crate::product_message!(
                                "backend.arxml.changes.object_change_context",
                                "change_id" => change_id,
                                "object_id" => &id
                            ),
                            message,
                        ])
                    })?;
            }
        }
        for change in &ordered {
            match change {
                ConfigurationChange::SetValue {
                    change_id,
                    field,
                    expected,
                    value,
                } => candidate
                    .set_value(change_id, field, expected, value, &created)
                    .map_err(|message| {
                        crate::message::LocalizedText::messages([
                            crate::product_message!(
                                "backend.arxml.changes.field_change_context",
                                "change_id" => change_id,
                                "field" => serde_json::to_string(field).unwrap()
                            ),
                            message,
                        ])
                    })?,
                ConfigurationChange::SetReference {
                    change_id,
                    field,
                    expected,
                    value,
                } => candidate
                    .set_reference(change_id, field, expected, value, &created)
                    .map_err(|message| {
                        crate::message::LocalizedText::messages([
                            crate::product_message!(
                                "backend.arxml.changes.field_change_context",
                                "change_id" => change_id,
                                "field" => serde_json::to_string(field).unwrap()
                            ),
                            message,
                        ])
                    })?,
                _ => {}
            }
        }
        let removed_ids: BTreeSet<_> = ordered
            .iter()
            .filter_map(|change| {
                if let ConfigurationChange::RemoveInstance { object, .. } = change {
                    Some(object_id(object, &created))
                } else {
                    None
                }
            })
            .collect::<Result<_, _>>()?;
        let removed_owner = |id: &str| -> Result<bool, crate::message::LocalizedText> {
            let mut current = Some(id.to_owned());
            while let Some(id) = current {
                if removed_ids.contains(&id) {
                    return Ok(true);
                }
                current = object(&candidate.workspace, &id)?.view.parent_id.clone();
            }
            Ok(false)
        };
        for created in &candidate.created {
            if removed_owner(&created.object_id)? {
                return Err(crate::product_message!(
                    "backend.arxml.changes.create_instance_in_removal_closure"
                ));
            }
        }
        for created in &candidate.fields {
            let field = candidate
                .workspace
                .snapshot
                .field_by_id
                .get(&created.field_id)
                .map(|index| &candidate.workspace.snapshot.fields[*index])
                .ok_or_else(|| {
                    crate::product_message!(
                        "backend.arxml.changes.created_field_missing_from_graph"
                    )
                })?;
            if removed_owner(&field.view.object_id)? {
                return Err(crate::product_message!(
                    "backend.arxml.changes.create_field_in_removal_closure"
                ));
            }
        }
        // Remove child instances first. References from any instance in the batch removal closure do not survive.
        let mut removals: Vec<_> = ordered
            .iter()
            .filter_map(|change| {
                if let ConfigurationChange::RemoveInstance {
                    change_id,
                    object,
                    expected_short_name,
                } = change
                {
                    Some(object_id(object, &created).and_then(|id| {
                        Ok((
                            candidate
                                .workspace
                                .snapshot
                                .object_by_id
                                .get(&id)
                                .map(|index| {
                                    candidate.workspace.snapshot.objects[*index].view.path.len()
                                })
                                .ok_or_else(|| {
                                    crate::product_message!(
                                        "backend.arxml.changes.removed_object_missing"
                                    )
                                })?,
                            change_id,
                            id,
                            expected_short_name,
                        ))
                    }))
                } else {
                    None
                }
            })
            .collect::<Result<_, crate::message::LocalizedText>>()?;
        removals.sort_by_key(|item| std::cmp::Reverse(item.0));
        for (_, change, id, expected) in removals {
            candidate
                .remove_instance(change, &id, expected, &removed_ids)
                .map_err(|message| {
                    crate::message::LocalizedText::messages([
                        crate::product_message!(
                            "backend.arxml.changes.object_change_context",
                            "change_id" => change,
                            "object_id" => &id
                        ),
                        message,
                    ])
                })?;
        }
        candidate.workspace.check_configuration_transition(self)?;
        Ok(candidate)
    }

    pub(super) fn check_configuration_transition(
        &self,
        before: &Workspace,
    ) -> Result<(), crate::message::LocalizedText> {
        for scope in &self.snapshot.validation {
            if scope.scope == ValidationScope::Schema {
                if let Some(issue) = scope
                    .diagnostics
                    .iter()
                    .find(|issue| matches!(issue.severity, Severity::Error))
                {
                    return Err(configuration_error(issue, before, &self.snapshot));
                }
            }
            if scope.scope != ValidationScope::Definition {
                continue;
            }
            let baseline: Vec<_> = before
                .snapshot
                .validation
                .iter()
                .filter(|scope| scope.scope == ValidationScope::Definition)
                .flat_map(|scope| &scope.diagnostics)
                .filter_map(|issue| issue.witness.as_ref())
                .collect();
            for issue in scope
                .diagnostics
                .iter()
                .filter(|issue| matches!(issue.severity, Severity::Error))
            {
                if issue
                    .witness
                    .as_ref()
                    .is_none_or(|witness| !baseline.contains(&witness))
                {
                    return Err(configuration_error(issue, before, &self.snapshot));
                }
            }
        }
        if let Some(archive) = &self.schema_zip {
            let files: Vec<_> = self
                .files
                .iter()
                .map(|file| (file.path.as_path(), file.text.as_str()))
                .collect();
            if let Some(issue) = schema::validate_files(archive, &files)?.first() {
                return Err(crate::message::LocalizedText::messages([
                    crate::product_message!(
                        "backend.arxml.changes.schema_diagnostic_context",
                        "code" => &issue.code
                    ),
                    issue.message.clone(),
                ]));
            }
        }
        Ok(())
    }
}

fn configuration_error(
    issue: &ConfigurationDiagnostic,
    before: &Workspace,
    snapshot: &super::projection::SourceSnapshot,
) -> crate::message::LocalizedText {
    let mut diagnostic = issue.clone();
    if diagnostic
        .object_id
        .as_ref()
        .is_some_and(|id| !before.snapshot.object_by_id.contains_key(id))
    {
        diagnostic.object_id = None;
    }
    if diagnostic
        .field_id
        .as_ref()
        .is_some_and(|id| !before.snapshot.field_by_id.contains_key(id))
    {
        diagnostic.field_id = None;
    }
    if let Some(witness) = &mut diagnostic.witness {
        for subject in &mut witness.subjects {
            if before.snapshot.object_by_id.contains_key(subject)
                || before.snapshot.field_by_id.contains_key(subject)
            {
                continue;
            }
            if let Some(index) = snapshot.object_by_id.get(subject) {
                *subject = format!("created-path:{}", snapshot.objects[*index].view.path);
            } else if let Some(index) = snapshot.field_by_id.get(subject) {
                *subject = format!(
                    "new-definition:{}",
                    snapshot.fields[*index].view.definition.definition_id
                );
            }
        }
    }
    crate::message::LocalizedText::messages(
        [
            crate::product_message!(
                "backend.arxml.changes.configuration_diagnostic_context",
                "code" => &diagnostic.code,
                "rule_id" => &diagnostic.rule_id,
                "scope" => serde_json::to_string(&diagnostic.scope).expect("Diagnostic scope serializes"),
                "severity" => serde_json::to_string(&diagnostic.severity).expect("Diagnostic severity serializes"),
                "file" => diagnostic.file.as_deref().unwrap_or_default(),
                "source_id" => diagnostic.source_id.as_deref().unwrap_or_default(),
                "path" => diagnostic.path.as_deref().unwrap_or_default(),
                "object_id" => diagnostic.object_id.as_deref().unwrap_or_default(),
                "field_id" => diagnostic.field_id.as_deref().unwrap_or_default()
            ),
            diagnostic.message.clone(),
            diagnostic.remedy.clone(),
        ]
        .into_iter()
        .chain(diagnostic.witness.iter().flat_map(|witness| [
            crate::product_message!(
                "backend.arxml.changes.validation_witness_context",
                "rule_id" => &witness.rule_id,
                "subjects" => serde_json::to_string(&witness.subjects).expect("Witness subjects serialize")
            ),
            witness.constraint.clone(),
            witness.counterexample.clone(),
        ])),
    )
}

impl Candidate {
    fn create_instance(
        &mut self,
        change: &str,
        parent_id: &str,
        source_id: &str,
        definition_id: &str,
        short_name: &str,
    ) -> Result<(), crate::message::LocalizedText> {
        if !valid_name(short_name) {
            return Err(crate::product_message!(
                "backend.arxml.changes.invalid_instance_short_name",
                "change_id" => change
            ));
        }
        let parent = object(&self.workspace, parent_id)?.clone();
        if !parent.view.writable || parent.view.source_id != source_id {
            return Err(crate::product_message!(
                "backend.arxml.changes.parent_read_only_or_source_mismatch",
                "change_id" => change
            ));
        }
        let descriptor = self.workspace.catalog.get(definition_id).ok_or_else(|| {
            crate::product_message!("backend.arxml.changes.unknown_instance_definition")
        })?;
        if !descriptor.writable || descriptor.kind.is_some() {
            return Err(crate::product_message!(
                "backend.arxml.changes.instance_creation_not_permitted"
            ));
        }
        let module = descriptor.element_kind == "ECUC-MODULE-DEF";
        let allowed = if module {
            parent.view.kind == "AR-PACKAGE"
        } else {
            parent.view.definition_id.as_deref().is_some_and(|id| {
                let mut ancestor = definition_id.rsplit_once('/').map(|(parent, _)| parent);
                while ancestor != Some(id)
                    && ancestor
                        .and_then(|id| self.workspace.catalog.get(id))
                        .is_some_and(|item| item.element_kind == "ECUC-CHOICE-CONTAINER-DEF")
                {
                    ancestor =
                        ancestor.and_then(|id| id.rsplit_once('/').map(|(parent, _)| parent));
                }
                ancestor == Some(id)
            })
        };
        if !allowed {
            return Err(crate::product_message!(
                "backend.arxml.changes.instance_parent_relation_not_permitted"
            ));
        }
        let path = format!("{}/{short_name}", parent.view.path);
        if self.workspace.snapshot.paths.contains_key(&path) {
            return Err(crate::product_message!(
                "backend.arxml.changes.instance_path_or_sibling_exists"
            ));
        }
        let (kind, group, category) = if module {
            (
                "ECUC-MODULE-CONFIGURATION-VALUES",
                "ELEMENTS",
                "<IMPLEMENTATION-CONFIG-VARIANT>VARIANT-PRE-COMPILE</IMPLEMENTATION-CONFIG-VARIANT>",
            )
        } else if parent.view.kind == "ECUC-MODULE-CONFIGURATION-VALUES" {
            ("ECUC-CONTAINER-VALUE", "CONTAINERS", "")
        } else {
            ("ECUC-CONTAINER-VALUE", "SUB-CONTAINERS", "")
        };
        let xml = format!(
            "<{kind}><SHORT-NAME>{}</SHORT-NAME><DEFINITION-REF DEST=\"{}\">{}</DEFINITION-REF>{category}</{kind}>",
            escape(short_name)?,
            descriptor.element_kind,
            escape(definition_id)?
        );
        let mut patches = Vec::new();
        group_insert(
            &self.workspace.files[parent.source].text,
            parent.range.clone(),
            group,
            &xml,
            &mut patches,
        )?;
        self.impacts.push(impact(
            change,
            &self.workspace,
            &parent,
            None,
            String::new(),
            xml,
            false,
        ));
        apply_local(&mut self.workspace, parent.source, &mut patches)?;
        let index = self
            .workspace
            .snapshot
            .paths
            .get(&path)
            .and_then(|items| (items.len() == 1).then_some(items[0]))
            .ok_or_else(|| {
                crate::product_message!(
                    "backend.arxml.changes.created_instance_not_uniquely_indexed"
                )
            })?;
        let id = self.workspace.snapshot.objects[index]
            .view
            .object_id
            .clone();
        self.created.push(CreatedObject {
            change_id: change.into(),
            object_id: id.clone(),
        });
        self.selection = Some(id);
        Ok(())
    }

    fn rename_instance(
        &mut self,
        change: &str,
        id: &str,
        expected: &str,
        short_name: &str,
    ) -> Result<(), crate::message::LocalizedText> {
        let owner = object(&self.workspace, id)?.clone();
        if self
            .workspace
            .snapshot
            .paths
            .get(&owner.view.path)
            .is_none_or(|items| items.len() != 1)
        {
            return Err(crate::product_message!(
                "backend.arxml.changes.ambiguous_instance_path_cannot_rename"
            ));
        }
        self.ensure_known_reference_impact(&owner.view.path)?;
        if !owner.view.writable || owner.view.short_name != expected || !valid_name(short_name) {
            return Err(crate::product_message!(
                "backend.arxml.changes.rename_invalid_read_only_or_stale",
                "change_id" => change
            ));
        }
        if owner.view.short_name == short_name {
            self.selection = Some(id.into());
            return Ok(());
        }
        let prefix = owner
            .view
            .path
            .rsplit_once('/')
            .ok_or_else(|| {
                crate::product_message!("backend.arxml.changes.object_path_parent_missing")
            })?
            .0;
        let new_path = format!("{prefix}/{short_name}");
        if self.workspace.snapshot.paths.contains_key(&new_path) {
            return Err(crate::product_message!(
                "backend.arxml.changes.rename_conflicts_with_instance"
            ));
        }
        let mut patches: Vec<Vec<Patch>> =
            self.workspace.files.iter().map(|_| Vec::new()).collect();
        patches[owner.source].push(Patch {
            range: owner.name_range.clone(),
            value: escape(short_name)?,
        });
        for edge in &self.workspace.snapshot.references {
            if edge.raw_path != owner.view.path
                && !edge
                    .raw_path
                    .strip_prefix(&owner.view.path)
                    .is_some_and(|suffix| suffix.starts_with('/'))
            {
                continue;
            }
            if edge.target_id.is_none() {
                return Err(crate::product_message!(
                    "backend.arxml.changes.rename_incoming_reference_unresolved"
                ));
            }
            let field = &self.workspace.snapshot.fields[*self
                .workspace
                .snapshot
                .field_by_id
                .get(&edge.field_id)
                .unwrap()];
            let ref_owner = object(&self.workspace, &edge.object_id)?;
            if ref_owner.unsafe_semantics {
                return Err(crate::product_message!(
                    "backend.arxml.changes.rename_unsupported_reference_or_variant"
                ));
            }
            if field.element == "ECUC-REFERENCE-VALUE"
                && field.view.definition.kind != Some(ValueKind::Reference)
            {
                return Err(crate::product_message!(
                    "backend.arxml.changes.rename_reference_definition_unavailable"
                ));
            }
            let range = field.value_range.clone().ok_or_else(|| {
                crate::product_message!("backend.arxml.changes.rename_mixed_reference_content")
            })?;
            let next_path = format!("{new_path}{}", &edge.raw_path[owner.view.path.len()..]);
            patches[field.source].push(Patch {
                range,
                value: escape(&next_path)?,
            });
            self.impacts.push(impact(
                change,
                &self.workspace,
                ref_owner,
                Some(field),
                edge.raw_path.clone(),
                next_path,
                true,
            ));
        }
        self.impacts.push(impact(
            change,
            &self.workspace,
            &owner,
            None,
            owner.view.short_name.clone(),
            short_name.into(),
            false,
        ));
        // Seed only path rebinding in the private candidate index; object and field IDs remain stable.
        let snapshot = Arc::make_mut(&mut self.workspace.snapshot);
        for item in &mut snapshot.objects {
            if item.view.path == owner.view.path
                || item
                    .view
                    .path
                    .strip_prefix(&owner.view.path)
                    .is_some_and(|suffix| suffix.starts_with('/'))
            {
                item.view.path = format!("{new_path}{}", &item.view.path[owner.view.path.len()..]);
            }
        }
        for (file, edits) in self.workspace.files.iter_mut().zip(&mut patches) {
            apply_patches(&mut file.text, edits)?;
        }
        if owner.view.kind == "AR-PACKAGE" && owner.view.path == format!("/{}", self.workspace.name)
        {
            self.workspace.name = short_name.into();
        }
        self.workspace.refresh()?;
        self.selection = Some(id.into());
        Ok(())
    }

    fn field(
        &self,
        reference: &FieldRef,
        created: &BTreeMap<String, String>,
    ) -> Result<IndexedField, crate::message::LocalizedText> {
        match reference {
            FieldRef::Existing { field_id } => self
                .workspace
                .snapshot
                .field_by_id
                .get(field_id)
                .map(|index| self.workspace.snapshot.fields[*index].clone())
                .ok_or_else(|| crate::product_message!("backend.arxml.changes.stale_field_id")),
            FieldRef::New {
                object: reference,
                definition_id,
                ..
            } => {
                let id = object_id(reference, created)?;
                let owner = object(&self.workspace, &id)?;
                if !owner.view.writable {
                    return Err(crate::product_message!(
                        "backend.arxml.changes.new_field_owner_read_only"
                    ));
                }
                let descriptor = self
                    .workspace
                    .catalog
                    .get(definition_id)
                    .ok_or_else(|| {
                        crate::product_message!(
                            "backend.arxml.changes.unknown_new_field_definition"
                        )
                    })?
                    .clone();
                if !owner.view.definition_id.as_deref().is_some_and(|parent| {
                    self.workspace
                        .catalog
                        .children(parent)
                        .iter()
                        .any(|child| child.definition_id == *definition_id)
                }) {
                    return Err(crate::product_message!(
                        "backend.arxml.changes.new_field_definition_owner_mismatch"
                    ));
                }
                let kind = descriptor.kind.ok_or_else(|| {
                    crate::product_message!("backend.arxml.changes.new_field_kind_required")
                })?;
                let ordinal = self
                    .workspace
                    .snapshot
                    .fields
                    .iter()
                    .filter(|field| {
                        field.view.object_id == id
                            && field.view.definition.definition_id == *definition_id
                            && field.entry_range.is_some()
                    })
                    .count();
                Ok(IndexedField {
                    view: FieldDescriptor {
                        field_id: String::new(),
                        object_id: id,
                        definition: descriptor,
                        current: ValueState::Absent,
                        reference: (kind == ValueKind::Reference).then_some(ReferenceState::Absent),
                    },
                    source: owner.source,
                    entry_range: None,
                    value_range: None,
                    element: super::projection::field_element(kind).into(),
                    ordinal,
                })
            }
        }
    }

    fn set_value(
        &mut self,
        change: &str,
        reference: &FieldRef,
        expected: &ValueState,
        value: &ValueState,
        created: &BTreeMap<String, String>,
    ) -> Result<(), crate::message::LocalizedText> {
        let field = self.field(reference, created)?;
        if !field.view.definition.writable || field.view.reference.is_some() {
            return Err(crate::product_message!(
                "backend.arxml.changes.field_value_editing_unsupported",
                "change_id" => change
            ));
        }
        if matches!(reference, FieldRef::New { .. }) && *expected != ValueState::Absent {
            return Err(crate::product_message!(
                "backend.arxml.changes.new_field_expected_absent"
            ));
        }
        if let ValueState::Explicit { value } = value {
            self.workspace
                .catalog
                .validate_value(&field.view.definition.definition_id, value)?;
            escape(&value.lexeme)?;
        }
        if field.view.current == *value {
            self.selection = Some(field.view.object_id.clone());
            return Ok(());
        }
        let owner = object(&self.workspace, &field.view.object_id)?.clone();
        let mut patches = Vec::new();
        match value {
            ValueState::Absent => {
                if let Some(range) = &field.entry_range {
                    patches.push(Patch {
                        range: range.clone(),
                        value: String::new(),
                    });
                }
            }
            ValueState::Explicit { value } => {
                if field.entry_range.is_some() {
                    patches.push(value_patch(&self.workspace, &field, &value.lexeme)?);
                } else {
                    let xml = format!(
                        "<{}><DEFINITION-REF DEST=\"{}\">{}</DEFINITION-REF><VALUE>{}</VALUE></{}>",
                        field.element,
                        field.view.definition.element_kind,
                        escape(&field.view.definition.definition_id)?,
                        escape(&value.lexeme)?,
                        field.element
                    );
                    group_insert(
                        &self.workspace.files[owner.source].text,
                        owner.range.clone(),
                        "PARAMETER-VALUES",
                        &xml,
                        &mut patches,
                    )?;
                }
            }
        }
        self.impacts.push(impact(
            change,
            &self.workspace,
            &owner,
            Some(&field),
            serde_json::to_string(&field.view.current).unwrap(),
            serde_json::to_string(value).unwrap(),
            false,
        ));
        self.retire_new_placeholder(reference, &field);
        apply_local(&mut self.workspace, field.source, &mut patches)?;
        self.record_field(reference, &field)?;
        self.selection = Some(owner.view.object_id);
        Ok(())
    }

    fn set_reference(
        &mut self,
        change: &str,
        reference: &FieldRef,
        expected: &ReferenceState,
        value: &ReferenceState,
        created: &BTreeMap<String, String>,
    ) -> Result<(), crate::message::LocalizedText> {
        let field = self.field(reference, created)?;
        if !field.view.definition.writable
            || field.view.definition.kind != Some(ValueKind::Reference)
        {
            return Err(crate::product_message!(
                "backend.arxml.changes.field_reference_editing_unsupported",
                "change_id" => change
            ));
        }
        if matches!(reference, FieldRef::New { .. }) && *expected != ReferenceState::Absent {
            return Err(crate::product_message!(
                "backend.arxml.changes.new_reference_expected_absent"
            ));
        }
        let owner = object(&self.workspace, &field.view.object_id)?.clone();
        let mut patches = Vec::new();
        match value {
            ReferenceState::Absent => {
                if let Some(range) = &field.entry_range {
                    patches.push(Patch {
                        range: range.clone(),
                        value: String::new(),
                    });
                }
            }
            ReferenceState::Explicit {
                raw_path,
                dest,
                target,
            } => {
                let target_id = object_id(
                    target.as_ref().ok_or_else(|| {
                        crate::product_message!(
                            "backend.arxml.changes.new_reference_target_required"
                        )
                    })?,
                    created,
                )?;
                let target = object(&self.workspace, &target_id)?;
                if target.view.path != *raw_path
                    || target.view.kind != *dest
                    || !field.view.definition.reference_destinations.contains(dest)
                    || self
                        .workspace
                        .snapshot
                        .paths
                        .get(raw_path)
                        .is_none_or(|items| items.len() != 1)
                {
                    return Err(crate::product_message!(
                        "backend.arxml.changes.reference_graph_definition_mismatch"
                    ));
                }
                let xml = format!(
                    "<{}><DEFINITION-REF DEST=\"{}\">{}</DEFINITION-REF><VALUE-REF DEST=\"{}\">{}</VALUE-REF></{}>",
                    field.element,
                    field.view.definition.element_kind,
                    escape(&field.view.definition.definition_id)?,
                    escape(dest)?,
                    escape(raw_path)?,
                    field.element
                );
                if let Some(range) = &field.entry_range {
                    let document = Document::parse(&self.workspace.files[field.source].text)
                        .map_err(|error| error.to_string())?;
                    let node = document
                        .descendants()
                        .find(|node| node.is_element() && node.range() == *range)
                        .ok_or_else(|| {
                            crate::product_message!("backend.arxml.changes.reference_entry_missing")
                        })?;
                    let leaf = node
                        .children()
                        .find(|node| node.is_element() && node.tag_name().name() == "VALUE-REF")
                        .ok_or_else(|| {
                            crate::product_message!("backend.arxml.changes.reference_value_missing")
                        })?;
                    let value_range = field.value_range.clone().ok_or_else(|| {
                        crate::product_message!(
                            "backend.arxml.changes.mixed_reference_content_read_only"
                        )
                    })?;
                    patches.push(Patch {
                        range: value_range,
                        value: escape(raw_path)?,
                    });
                    if leaf.attribute("DEST") != Some(dest.as_str()) {
                        let attribute = leaf.attribute_node("DEST").ok_or_else(|| {
                            crate::product_message!("backend.arxml.changes.reference_dest_missing")
                        })?;
                        let range = attribute.range();
                        let original = &self.workspace.files[field.source].text[range.clone()];
                        let quote = original
                            .bytes()
                            .position(|byte| byte == b'\'' || byte == b'"')
                            .ok_or_else(|| {
                                crate::product_message!(
                                    "backend.arxml.changes.reference_dest_quote_missing"
                                )
                            })?;
                        let last = original
                            .rfind(original.as_bytes()[quote] as char)
                            .ok_or_else(|| {
                                crate::product_message!(
                                    "backend.arxml.changes.reference_dest_closing_quote_missing"
                                )
                            })?;
                        patches.push(Patch {
                            range: range.start + quote + 1..range.start + last,
                            value: escape(dest)?,
                        });
                    }
                } else {
                    group_insert(
                        &self.workspace.files[owner.source].text,
                        owner.range.clone(),
                        "REFERENCE-VALUES",
                        &xml,
                        &mut patches,
                    )?;
                }
            }
        }
        self.retire_new_placeholder(reference, &field);
        if field.view.reference.as_ref() == Some(value) {
            self.selection = Some(owner.view.object_id);
            return Ok(());
        }
        self.impacts.push(impact(
            change,
            &self.workspace,
            &owner,
            Some(&field),
            serde_json::to_string(&field.view.reference).unwrap(),
            serde_json::to_string(value).unwrap(),
            false,
        ));
        apply_local(&mut self.workspace, field.source, &mut patches)?;
        self.record_field(reference, &field)?;
        self.selection = Some(owner.view.object_id);
        Ok(())
    }

    fn record_field(
        &mut self,
        reference: &FieldRef,
        field: &IndexedField,
    ) -> Result<(), crate::message::LocalizedText> {
        if let FieldRef::New { entry_key, .. } = reference {
            let created = self
                .workspace
                .snapshot
                .fields
                .iter()
                .find(|item| {
                    item.view.object_id == field.view.object_id
                        && item.view.definition.definition_id == field.view.definition.definition_id
                        && item.ordinal == field.ordinal
                        && item.entry_range.is_some()
                })
                .ok_or_else(|| {
                    crate::product_message!(
                        "backend.arxml.changes.materialized_field_not_uniquely_indexed"
                    )
                })?;
            self.fields.push(CreatedField {
                entry_key: entry_key.clone(),
                field_id: created.view.field_id.clone(),
            });
        }
        Ok(())
    }

    fn retire_new_placeholder(&mut self, reference: &FieldRef, field: &IndexedField) {
        if matches!(reference, FieldRef::New { .. }) {
            Arc::make_mut(&mut self.workspace.snapshot)
                .fields
                .retain(|item| {
                    item.entry_range.is_some()
                        || item.view.object_id != field.view.object_id
                        || item.view.definition.definition_id != field.view.definition.definition_id
                });
        }
    }

    fn remove_instance(
        &mut self,
        change: &str,
        id: &str,
        expected: &str,
        removed: &BTreeSet<String>,
    ) -> Result<(), crate::message::LocalizedText> {
        let owner = object(&self.workspace, id)?.clone();
        if self
            .workspace
            .snapshot
            .paths
            .get(&owner.view.path)
            .is_none_or(|items| items.len() != 1)
        {
            return Err(crate::product_message!(
                "backend.arxml.changes.ambiguous_instance_path_cannot_remove"
            ));
        }
        self.ensure_known_reference_impact(&owner.view.path)?;
        if !owner.view.writable || owner.view.short_name != expected {
            return Err(crate::product_message!(
                "backend.arxml.changes.remove_read_only_or_stale",
                "change_id" => change
            ));
        }
        let inside = |path: &str| {
            path == owner.view.path
                || path
                    .strip_prefix(&owner.view.path)
                    .is_some_and(|suffix| suffix.starts_with('/'))
        };
        for edge in &self.workspace.snapshot.references {
            if !inside(&edge.raw_path) {
                continue;
            }
            let ref_owner = object(&self.workspace, &edge.object_id)?;
            let removed_owner = removed.iter().any(|id| {
                object(&self.workspace, id).is_ok_and(|item| {
                    ref_owner.view.path == item.view.path
                        || ref_owner
                            .view
                            .path
                            .strip_prefix(&item.view.path)
                            .is_some_and(|suffix| suffix.starts_with('/'))
                })
            });
            if !inside(&ref_owner.view.path) && !removed_owner {
                return Err(crate::product_message!(
                    "backend.arxml.changes.remove_leaves_reference"
                ));
            }
        }
        // Do not silently discard unknown source content inside a known instance.
        // Rule coverage includes same-namespace extensions and unsupported
        // attributes, not only opaque references or unavailable definitions.
        if self.workspace.snapshot.validation.iter().any(|scope| {
            scope.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "NATIVE_UNSUPPORTED"
                    && diagnostic.source_id.as_deref() == Some(owner.view.source_id.as_str())
                    && diagnostic.path.as_deref().is_some_and(inside)
            })
        }) {
            return Err(crate::product_message!(
                "backend.arxml.changes.unsupported_content_prevents_removal"
            ));
        }
        let document = Document::parse(&self.workspace.files[owner.source].text)
            .map_err(|error| error.to_string())?;
        let node = document
            .descendants()
            .find(|node| node.is_element() && node.range() == owner.range)
            .ok_or_else(|| {
                crate::product_message!("backend.arxml.changes.removed_instance_missing")
            })?;
        if node.descendants().any(|node| {
            node.is_element()
                && (node.tag_name().namespace() != Some(NS)
                    || matches!(
                        node.tag_name().name(),
                        "ECUC-INSTANCE-REFERENCE-VALUE" | "VARIATION-POINT"
                    )
                    || definition(node).is_some_and(|id| {
                        self.workspace
                            .catalog
                            .get(&id)
                            .is_none_or(|definition| !definition.writable)
                    }))
        }) {
            return Err(crate::product_message!(
                "backend.arxml.changes.unknown_reference_or_variant_prevents_removal"
            ));
        }
        self.impacts.push(impact(
            change,
            &self.workspace,
            &owner,
            None,
            self.workspace.files[owner.source].text[owner.range.clone()].into(),
            String::new(),
            false,
        ));
        apply_local(
            &mut self.workspace,
            owner.source,
            &mut vec![Patch {
                range: owner.range,
                value: String::new(),
            }],
        )?;
        self.selection = owner.view.parent_id;
        Ok(())
    }

    fn ensure_known_reference_impact(
        &self,
        path: &str,
    ) -> Result<(), crate::message::LocalizedText> {
        if self
            .workspace
            .snapshot
            .opaque_values
            .iter()
            .any(|value| value.contains(path))
        {
            return Err(crate::product_message!(
                "backend.arxml.changes.opaque_extension_reference_impact_unknown"
            ));
        }
        for field in &self.workspace.snapshot.fields {
            if field.view.definition.kind.is_some() || field.view.reference.is_some() {
                continue;
            }
            if let ValueState::Explicit { value } = &field.view.current {
                if value.lexeme.contains(path) {
                    return Err(crate::product_message!(
                        "backend.arxml.changes.opaque_field_reference_impact_unknown"
                    ));
                }
            }
        }
        Ok(())
    }
}
