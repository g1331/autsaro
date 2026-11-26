//! Project layers own operations and retain every original ARXML member.
use super::Workspace;
use crate::LocalizedText;
use crate::integration::{DiagnosticCategory, InputSource, PlanDiagnostic};
use crate::project_model::{ExtensionDefinitionIdentity, RuleSetIdentity, TypedValue};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

mod artifacts;
mod diff;
mod resolve;
mod split;
mod upgrade;

pub use artifacts::*;
pub use diff::*;
pub(crate) use resolve::resolve_manifest_sources;
pub use upgrade::{BaseUpgradePreview, ConflictResolution};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BaseReference {
    pub base_id: String,
    pub revision: String,
    pub source_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectBase {
    pub reference: BaseReference,
    pub release: String,
    pub rules: RuleSetIdentity,
    pub extensions: Vec<ExtensionDefinitionIdentity>,
    pub members: Vec<String>,
    #[serde(default)]
    pub migrations: Vec<PathMigration>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PathMigration {
    pub previous: LayerObject,
    pub current: LayerObject,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectLayers {
    pub base_reference: BaseReference,
    pub base: ProjectBase,
    pub project: ProjectLayer,
    pub groups: BTreeMap<String, VariantGroup>,
    pub selection: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    pub conflicts: Vec<UpgradeConflict>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectLayer {
    pub members: Vec<String>,
    pub operations: Vec<LayerOperation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VariantGroup {
    pub exactly_one: bool,
    pub options: BTreeMap<String, ProjectLayer>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayerObject {
    pub release: String,
    pub definition_type: String,
    pub path: String,
    #[serde(default)]
    pub instance_context: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayerTarget {
    pub object: LayerObject,
    pub field: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayerOperation {
    pub target: LayerTarget,
    pub action: LayerAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum LayerAction {
    Set {
        values: Vec<TypedValue>,
        #[serde(default)]
        default_origin: Option<String>,
    },
    Delete,
    Exclude,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpgradeConflict {
    pub layer: Vec<String>,
    pub operation: LayerOperation,
    pub reason: UpgradeConflictKind,
    pub previous: Option<SemanticEntry>,
    pub incoming: Option<SemanticEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpgradeConflictKind {
    FieldChanged,
    DeleteModified,
    DeleteReferenced,
    PathMigration,
    DefinitionChanged,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SemanticEntry {
    pub values: Vec<String>,
    pub sources: Vec<LayerOrigin>,
    pub operation: Option<LayerAction>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayerOrigin {
    pub layer: Vec<String>,
    pub file: String,
    pub sha256: String,
    pub byte_range: Option<(usize, usize)>,
    pub operation: Option<usize>,
}

fn diagnostic(code: &str, object: &str, message: impl Into<LocalizedText>) -> PlanDiagnostic {
    PlanDiagnostic {
        category: DiagnosticCategory::Input,
        code: code.into(),
        file: Some("workbench-project.json".into()),
        object: Some(object.into()),
        message: message.into(),
        remedy: "Check project layers, their members, and the selected variants.".into(),
    }
}

fn frame(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}

/// Identifies original base members by names relative to their base package.
pub fn base_source_digest(
    members: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<[u8]>)>,
) -> String {
    let sorted: BTreeMap<_, _> = members
        .into_iter()
        .map(|(path, bytes)| (path.as_ref().to_owned(), Sha256::digest(bytes)))
        .collect();
    let mut hash = Sha256::new();
    frame(&mut hash, b"autosar-project-base-v1");
    for (path, digest) in sorted {
        frame(&mut hash, path.as_bytes());
        frame(&mut hash, &digest);
    }
    hex::encode(hash.finalize())
}

impl ProjectLayers {
    pub(super) fn validate_members(
        &self,
        manifest: &super::ProjectManifest,
    ) -> Result<(), LocalizedText> {
        let declared: BTreeSet<_> = manifest.inputs.iter().map(|input| &input.path).collect();
        let mut assigned = BTreeSet::new();
        for member in self.base.members.iter().chain(&self.project.members).chain(
            self.groups
                .values()
                .flat_map(|group| group.options.values().flat_map(|layer| &layer.members)),
        ) {
            if !declared.contains(member) || !assigned.insert(member) {
                return Err(format!(
                    "Layer member must be declared and owned by one layer: {member}"
                )
                .into());
            }
        }
        if assigned != declared
            || self.base.members.is_empty()
            || self.base_reference != self.base.reference
            || self.base.release != manifest.declared_release
            || self.base.extensions != manifest.accepted_extension_definitions
            || self.base.reference.base_id.is_empty()
            || self.base.reference.revision.is_empty()
        {
            return Err(
                "The project base identity or layer membership does not match the manifest.".into(),
            );
        }
        Ok(())
    }

    fn active(&self) -> Result<Vec<(Vec<String>, &ProjectLayer)>, Vec<PlanDiagnostic>> {
        let mut result = vec![(vec!["project".into()], &self.project)];
        let mut diagnostics = Vec::new();
        for name in self
            .selection
            .keys()
            .filter(|name| !self.groups.contains_key(*name))
        {
            diagnostics.push(diagnostic(
                "VARIANT_GROUP_UNKNOWN",
                name,
                "Unknown variant group.",
            ));
        }
        for (name, group) in &self.groups {
            let selection = self.selection.get(name);
            if group.exactly_one && selection.map_or(0, BTreeSet::len) != 1 {
                diagnostics.push(diagnostic(
                    "VARIANT_SELECTION_REQUIRED",
                    name,
                    "Select exactly one option.",
                ));
            }
            for choice in selection.into_iter().flatten() {
                if let Some(layer) = group.options.get(choice) {
                    result.push((vec!["variant".into(), name.clone(), choice.clone()], layer));
                } else {
                    diagnostics.push(diagnostic(
                        "VARIANT_OPTION_UNKNOWN",
                        &format!("{name}/{choice}"),
                        "Unknown variant option.",
                    ));
                }
            }
        }
        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }
        Ok(result)
    }
}

impl Workspace {
    pub(super) fn verify_project_base(&self) -> Result<(), LocalizedText> {
        let Some((project, layers)) = self
            .project
            .as_ref()
            .and_then(|p| Some((p, p.manifest.layers.as_ref()?)))
        else {
            return Ok(());
        };
        let root = project.path.parent().ok_or("Project root missing.")?;
        let digest = base_source_digest(self.files.iter().filter_map(|file| {
            let relative = file
                .path
                .strip_prefix(root)
                .ok()?
                .to_str()?
                .replace('\\', "/");
            layers
                .base
                .members
                .contains(&relative)
                .then_some((relative, file.text.as_bytes()))
        }));
        if digest != layers.base.reference.source_sha256
            || layers.base.rules != crate::rules::rule_set_identity()?
            || layers.base.extensions != self.catalog.accepted_extensions()
            || self.catalog.has_missing_extensions()
        {
            return Err("The base source digest, release, or definition identity differs from the project reference.".into());
        }
        Ok(())
    }

    pub(crate) fn layered_plan_identity(&self) -> Result<String, LocalizedText> {
        self.verify_project_base()?;
        let project = self.project.as_ref().ok_or("Project manifest required.")?;
        let mut hash = Sha256::new();
        frame(&mut hash, b"autosar-layered-plan-v1");
        frame(&mut hash, project.current.as_bytes());
        frame(&mut hash, self.definition_fingerprint()?.as_bytes());
        let root = project.path.parent().ok_or("Project root missing.")?;
        for file in &self.files {
            let path = file
                .path
                .strip_prefix(root)
                .map_err(|error| error.to_string())?;
            frame(
                &mut hash,
                path.to_string_lossy().replace('\\', "/").as_bytes(),
            );
            frame(&mut hash, file.text.as_bytes());
        }
        Ok(hex::encode(hash.finalize()))
    }

    pub(crate) fn active_application_identity(
        &self,
        slots: &[crate::generator::delivery::ApplicationSlotDescriptor],
    ) -> Option<String> {
        let project = self.project.as_ref()?;
        project.manifest.layers.as_ref()?;
        let mut hash = Sha256::new();
        frame(&mut hash, b"autosar-layered-applications-v1");
        for (path, bytes) in &project.application_bytes {
            if slots.iter().any(|slot| slot.source_paths.contains(path)) {
                frame(&mut hash, path.as_bytes());
                frame(&mut hash, bytes);
            }
        }
        Some(hex::encode(hash.finalize()))
    }

    pub(crate) fn integration_plan_for_catalog(
        &self,
        catalog: &crate::definitions::DefinitionCatalog,
    ) -> Result<crate::integration::ValidatedIntegrationPlan, Vec<PlanDiagnostic>> {
        self.build_workspace_plan(&self.integration_sources()?, catalog)
    }

    pub(super) fn build_workspace_plan(
        &self,
        sources: &[InputSource],
        catalog: &crate::definitions::DefinitionCatalog,
    ) -> Result<crate::integration::ValidatedIntegrationPlan, Vec<PlanDiagnostic>> {
        let mut plan = crate::integration::build_plan_native(sources, catalog)?;
        if self
            .project_manifest()
            .is_some_and(|manifest| manifest.layers.is_some())
            && let Some(identity) =
                self.active_application_identity(&plan.application_slot_descriptors()?)
        {
            plan.bind_application_identity(identity);
        }
        Ok(plan)
    }

    pub(super) fn resolve_project_sources(
        &self,
        mut sources: Vec<InputSource>,
    ) -> Result<Vec<InputSource>, Vec<PlanDiagnostic>> {
        let Some(layers) = self
            .project_manifest()
            .and_then(|manifest| manifest.layers.as_ref())
        else {
            return Ok(sources);
        };
        let identity = self
            .layered_plan_identity()
            .map_err(|message| vec![diagnostic("BASE_IDENTITY", "baseReference", message)])?;
        if !layers.conflicts.is_empty() {
            return Err(layers
                .conflicts
                .iter()
                .map(|conflict| {
                    diagnostic(
                        "BASE_UPGRADE_CONFLICT",
                        &conflict.operation.target.object.path,
                        format!("Unresolved base upgrade conflict: {:?}.", conflict.reason),
                    )
                })
                .collect());
        }
        resolve_manifest_sources(
            self.project_manifest().unwrap(),
            &mut sources,
            &self.catalog,
            &identity,
        )?;
        Ok(sources)
    }

    pub fn select_project_variants(
        &mut self,
        selection: BTreeMap<String, BTreeSet<String>>,
    ) -> Result<(), LocalizedText> {
        self.edit_project_layers(|layers| {
            layers.selection = selection;
            Ok(())
        })
    }

    fn edit_project_layers(
        &mut self,
        edit: impl FnOnce(&mut ProjectLayers) -> Result<(), LocalizedText>,
    ) -> Result<(), LocalizedText> {
        self.ensure_sources_current()?;
        let project = self.project.as_mut().ok_or("Project manifest required.")?;
        let mut manifest = project.manifest.clone();
        let layers = manifest
            .layers
            .as_mut()
            .ok_or("Layered project required.")?;
        edit(layers)?;
        let current = super::project::render_manifest(&manifest)?;
        project.current = current;
        project.manifest = manifest;
        self.revision += 1;
        Ok(())
    }

    /// Replace a layer's operation set atomically. Omitting a target restores inheritance.
    pub fn replace_layer_operations(
        &mut self,
        layer: Option<(&str, &str)>,
        operations: Vec<LayerOperation>,
    ) -> Result<(), Vec<PlanDiagnostic>> {
        let mut candidate = self.clone();
        candidate
            .edit_project_layers(|layers| {
                let target = match layer {
                    Some((group, option)) => layers
                        .groups
                        .get_mut(group)
                        .and_then(|group| group.options.get_mut(option))
                        .ok_or("Unknown variant layer.")?,
                    None => &mut layers.project,
                };
                target.operations = operations;
                Ok(())
            })
            .map_err(|message| vec![diagnostic("LAYER_EDIT", "layers", message)])?;
        candidate.integration_plan()?;
        *self = candidate;
        Ok(())
    }
}

impl Workspace {
    fn project_parts(
        &self,
    ) -> Result<(&super::project::ProjectMembership, &ProjectLayers), LocalizedText> {
        let project = self.project.as_ref().ok_or("Project manifest required.")?;
        let layers = project
            .manifest
            .layers
            .as_ref()
            .ok_or("Layered project required.")?;
        Ok((project, layers))
    }

    fn original_layer_sources(&self) -> Result<Vec<InputSource>, LocalizedText> {
        let (project, _) = self.project_parts()?;
        let root = project.path.parent().ok_or("Project root missing.")?;
        self.files
            .iter()
            .map(|file| {
                let relative = file
                    .path
                    .strip_prefix(root)
                    .map_err(|error| error.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                InputSource::new(relative, file.text.as_bytes().to_vec())
                    .map_err(|diagnostic| diagnostic.message)
            })
            .collect()
    }
}
