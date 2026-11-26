use super::{
    resolve::{migrated_path, resolve, within},
    *,
};
use crate::arxml::{Document, path_of};
use crate::project_model::ValueKind;

#[derive(Clone, Debug)]
pub struct BaseUpgradePreview {
    pub revision: String,
    pub conflicts: Vec<UpgradeConflict>,
    pub agreements: Vec<LayerTarget>,
}

#[derive(Clone, Debug)]
pub enum ConflictResolution {
    KeepProject,
    UseBase,
    Custom(LayerAction),
}

fn base_semantics(
    base: &ProjectBase,
    sources: &[InputSource],
    catalog: &crate::definitions::DefinitionCatalog,
) -> Result<BTreeMap<LayerTarget, SemanticEntry>, Vec<PlanDiagnostic>> {
    let base_only = ProjectLayers {
        base_reference: base.reference.clone(),
        base: base.clone(),
        project: ProjectLayer::default(),
        groups: BTreeMap::new(),
        selection: BTreeMap::new(),
        conflicts: Vec::new(),
        applied_migrations: Vec::new(),
    };
    resolve(&base_only, sources, catalog).map(|resolution| resolution.semantics)
}

fn descendants<'a>(
    entries: &'a BTreeMap<LayerTarget, SemanticEntry>,
    target: &'a LayerTarget,
) -> impl Iterator<Item = (&'a LayerTarget, &'a SemanticEntry)> {
    entries.iter().filter(move |(candidate, _)| {
        if target.field.is_some() || !target.object.instance_context.is_empty() {
            *candidate == target
        } else {
            within(&candidate.object.path, &target.object.path)
        }
    })
}

fn incoming_refs(
    sources: &[InputSource],
    base: &ProjectBase,
    target: &LayerTarget,
) -> BTreeSet<(String, String)> {
    let mut inbound = BTreeSet::new();
    for source in sources.iter().filter(|source| {
        base.members
            .iter()
            .any(|member| member == source.logical_path())
    }) {
        let Some(document) = std::str::from_utf8(source.bytes())
            .ok()
            .and_then(|text| Document::parse(text).ok())
        else {
            continue;
        };
        for node in document
            .descendants()
            .filter(|node| super::resolve::reference_element(*node))
        {
            if let Some(value) = crate::arxml::xml::literal_text(node)
                && within(value.trim(), &target.object.path)
            {
                let owner = path_of(node);
                if !within(&owner, &target.object.path) {
                    inbound.insert((owner, node.tag_name().name().to_owned()));
                }
            }
        }
    }
    inbound
}

fn candidate_reference_is_opaque(
    sources: &[InputSource],
    path: &str,
    history: &[Vec<PathMigration>],
) -> Result<bool, LocalizedText> {
    for source in sources {
        let text = std::str::from_utf8(source.bytes()).map_err(|error| error.to_string())?;
        let document = Document::parse(text).map_err(|error| error.to_string())?;
        for node in document.descendants().filter(|node| node.is_element()) {
            let vendor = node.ancestors().any(|ancestor| {
                ancestor.is_element()
                    && (ancestor.tag_name().namespace() != Some(crate::arxml::NS)
                        || ancestor.has_tag_name((crate::arxml::NS, "SDG")))
            });
            let points_to_old = |value: &str| {
                value.contains(path)
                    || within(
                        &super::resolve::replay_migrations(value.trim(), history),
                        path,
                    )
            };
            if node.attributes().any(|attribute| {
                (vendor
                    || attribute.namespace().is_some_and(|namespace| {
                        namespace != "http://www.w3.org/2001/XMLSchema-instance"
                    }))
                    && points_to_old(attribute.value())
            }) || (vendor
                || super::resolve::reference_element(node) && node.attribute("DEST").is_none())
                && crate::arxml::xml::literal_text(node).is_some_and(|value| points_to_old(&value))
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

impl Workspace {
    fn base_upgrade_candidate(
        &self,
        base: ProjectBase,
    ) -> Result<(Self, BaseUpgradePreview), LocalizedText> {
        self.verify_saved_sources()?;
        let (project, layers) = self.project_parts()?;
        if !layers.conflicts.is_empty() {
            return Err("Resolve the current upgrade before changing the base again.".into());
        }
        if base.reference.base_id != layers.base.reference.base_id
            || base.reference.revision == layers.base.reference.revision
                && base.reference != layers.base.reference
        {
            return Err(
                "The new base must have the same base ID and a new revision for changed bytes."
                    .into(),
            );
        }
        let root = project.path.parent().ok_or("Project root missing.")?;
        let old_sources = self.original_layer_sources()?;
        let old = base_semantics(&layers.base, &old_sources, &self.catalog)
            .map_err(crate::arxml::integration_errors)?;
        let mut membership = project.clone();
        membership
            .manifest
            .inputs
            .retain(|input| !layers.base.members.contains(&input.path));
        membership
            .manifest
            .inputs
            .extend(base.members.iter().map(|path| crate::arxml::ProjectInput {
                path: path.clone(),
                role_hint: "base".into(),
            }));
        let next = membership
            .manifest
            .layers
            .as_mut()
            .ok_or("Layered project required.")?;
        next.base_reference = base.reference.clone();
        next.base = base.clone();
        crate::arxml::project::validate_manifest(&membership.manifest)?;
        let paths = membership
            .manifest
            .inputs
            .iter()
            .map(|input| root.join(&input.path))
            .collect();
        let mut candidate = crate::arxml::load_sources_with(paths, |workspace| {
            workspace.catalog = self.catalog.clone();
            workspace.integration_input_root = Some(root.to_path_buf());
            workspace.project = Some(membership);
        })?;
        candidate.verify_project_base()?;
        let new_sources = candidate.original_layer_sources()?;
        let new = base_semantics(&base, &new_sources, &self.catalog)
            .map_err(crate::arxml::integration_errors)?;
        let project = candidate
            .project
            .as_mut()
            .ok_or("Project manifest required.")?;
        let next = project
            .manifest
            .layers
            .as_mut()
            .ok_or("Layered project required.")?;
        let mut migration_sources = BTreeSet::new();
        let mut migration_targets = BTreeSet::new();
        for migration in &base.migrations {
            if candidate_reference_is_opaque(
                &new_sources,
                &migration.previous.path,
                &next.applied_migrations,
            )? {
                return Err(crate::product_message!(
                    "backend.arxml.changes.opaque_extension_reference_impact_unknown"
                ));
            }
            let previous = LayerTarget {
                object: migration.previous.clone(),
                field: None,
            };
            let current = LayerTarget {
                object: migration.current.clone(),
                field: None,
            };
            if migration.previous.definition_type != migration.current.definition_type
                || !old.contains_key(&previous)
                || !new.contains_key(&current)
                || !migration_sources.insert(&migration.previous)
                || !migration_targets.insert(&migration.current)
            {
                return Err(format!(
                    "Invalid migration from {} to {}.",
                    migration.previous.path, migration.current.path
                )
                .into());
            }
        }
        if !base.migrations.is_empty() {
            next.applied_migrations.push(base.migrations.clone());
        }
        let layers_base = layers.base.clone();
        let mut conflicts = Vec::new();
        let mut agreements = Vec::new();
        let layers = std::iter::once((vec!["project".into()], &mut next.project)).chain(
            next.groups.iter_mut().flat_map(|(group, variants)| {
                variants.options.iter_mut().map(move |(option, layer)| {
                    (vec!["variant".into(), group.clone(), option.clone()], layer)
                })
            }),
        );
        for (origin, layer) in layers {
            let occupied: BTreeSet<_> = layer
                .operations
                .iter()
                .map(|operation| operation.target.clone())
                .collect();
            let mut redundant = BTreeSet::new();
            for operation in &mut layer.operations {
                let target = operation.target.clone();
                let previous = old.get(&target);
                let mut updated = target.clone();
                if let Some(path) = migrated_path(&target.object.path, &base.migrations) {
                    updated.object.path = path;
                }
                if let LayerAction::Set { values, .. } = &mut operation.action {
                    for value in values
                        .iter_mut()
                        .filter(|value| value.kind == ValueKind::Reference)
                    {
                        if let Some(path) = migrated_path(value.lexeme.trim(), &base.migrations) {
                            value.lexeme = path;
                        }
                    }
                }
                let incoming = new.get(&updated);
                let removes =
                    matches!(operation.action, LayerAction::Delete | LayerAction::Exclude);
                let new_inbound = removes
                    && target.field.is_none()
                    && !incoming_refs(&new_sources, &base, &updated).is_subset(&incoming_refs(
                        &old_sources,
                        &layers_base,
                        &target,
                    ));
                if removes && previous.is_some() && incoming.is_none() && !new_inbound {
                    redundant.insert(target);
                    agreements.push(updated);
                    continue;
                }
                let reason = if new_inbound {
                    Some(UpgradeConflictKind::DeleteReferenced)
                } else if updated != target && occupied.contains(&updated)
                    || previous.is_some()
                        && incoming.is_none()
                        && !new.contains_key(&LayerTarget {
                            object: updated.object.clone(),
                            field: None,
                        })
                {
                    Some(UpgradeConflictKind::PathMigration)
                } else if previous.map(|row| &row.values) == incoming.map(|row| &row.values)
                    && (target.field.is_some()
                        || descendants(&old, &target)
                            .map(|(_, row)| &row.values)
                            .eq(descendants(&new, &updated).map(|(_, row)| &row.values)))
                {
                    None
                } else {
                    match &operation.action {
                        LayerAction::Set { values, .. } => {
                            let new_values = values
                                .iter()
                                .map(|value| {
                                    let destination = if value.kind == ValueKind::Reference {
                                        incoming
                                            .into_iter()
                                            .chain(previous)
                                            .flat_map(|entry| &entry.values)
                                            .filter_map(|entry| entry.split_once(' '))
                                            .find(|(_, path)| *path == value.lexeme.trim())
                                            .map(|(destination, _)| destination)
                                    } else {
                                        None
                                    };
                                    super::resolve::semantic_value(
                                        Some(value.kind),
                                        &value.lexeme,
                                        destination,
                                    )
                                })
                                .collect::<Vec<_>>();
                            if incoming.is_some_and(|row| row.values == new_values) {
                                agreements.push(updated.clone());
                                None
                            } else {
                                Some(UpgradeConflictKind::FieldChanged)
                            }
                        }
                        LayerAction::Delete | LayerAction::Exclude if incoming.is_none() => {
                            agreements.push(updated.clone());
                            None
                        }
                        LayerAction::Delete | LayerAction::Exclude => {
                            Some(UpgradeConflictKind::DeleteModified)
                        }
                    }
                };
                if reason != Some(UpgradeConflictKind::PathMigration) {
                    operation.target = updated;
                }
                if let Some(reason) = reason {
                    conflicts.push(UpgradeConflict {
                        layer: origin.clone(),
                        operation: operation.clone(),
                        reason,
                        previous: previous.cloned(),
                        incoming: incoming.cloned(),
                    });
                }
            }
            layer
                .operations
                .retain(|operation| !redundant.contains(&operation.target));
        }
        next.conflicts = conflicts.clone();
        project.current = crate::arxml::project::render_manifest(&project.manifest)?;
        candidate.revision = self.revision + 1;
        let mut confirmation = Sha256::new();
        frame(&mut confirmation, self.layered_plan_identity()?.as_bytes());
        frame(
            &mut confirmation,
            candidate.layered_plan_identity()?.as_bytes(),
        );
        let revision = hex::encode(confirmation.finalize());
        Ok((
            candidate,
            BaseUpgradePreview {
                revision,
                conflicts,
                agreements,
            },
        ))
    }

    /// The incoming revision's immutable members are staged under new project paths.
    pub fn preview_base_upgrade(
        &self,
        base: ProjectBase,
    ) -> Result<BaseUpgradePreview, LocalizedText> {
        self.base_upgrade_candidate(base)
            .map(|(_, preview)| preview)
    }

    pub fn apply_base_upgrade(
        &mut self,
        base: ProjectBase,
        revision: &str,
    ) -> Result<(), LocalizedText> {
        let (mut candidate, preview) = self.base_upgrade_candidate(base)?;
        // This token is portable across the two independently read candidates.
        if preview.revision != revision {
            return Err("The base upgrade preview is stale.".into());
        }
        if preview.conflicts.is_empty() {
            candidate
                .integration_plan()
                .map_err(crate::arxml::integration_errors)?;
        }
        candidate.save()?;
        *self = candidate;
        Ok(())
    }

    pub fn resolve_base_conflict(
        &mut self,
        index: usize,
        resolution: ConflictResolution,
    ) -> Result<(), LocalizedText> {
        let mut candidate = self.clone();
        candidate.edit_project_layers(|layers| {
            let conflict = layers
                .conflicts
                .get(index)
                .cloned()
                .ok_or("Unknown base upgrade conflict.")?;
            let operations = match conflict.layer.as_slice() {
                [project] if project == "project" => &mut layers.project.operations,
                [variant, group, option] if variant == "variant" => {
                    &mut layers
                        .groups
                        .get_mut(group)
                        .and_then(|group| group.options.get_mut(option))
                        .ok_or("Unknown variant layer.")?
                        .operations
                }
                _ => return Err("Unknown conflict layer.".into()),
            };
            let index_of_operation = operations
                .iter()
                .position(|operation| operation == &conflict.operation)
                .ok_or("The conflicting operation changed.")?;
            match resolution {
                ConflictResolution::KeepProject => {}
                ConflictResolution::UseBase => {
                    operations.remove(index_of_operation);
                }
                ConflictResolution::Custom(action) => {
                    operations[index_of_operation].action = action
                }
            }
            layers.conflicts.remove(index);
            Ok(())
        })?;
        if candidate.project_parts()?.1.conflicts.is_empty() {
            candidate
                .integration_plan()
                .map_err(crate::arxml::integration_errors)?;
        }
        *self = candidate;
        Ok(())
    }
}
