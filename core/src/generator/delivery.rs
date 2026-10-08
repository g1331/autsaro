//! Source-only native delivery and strict v2 import. No official resource is read.
//! Configuration/member state remains owned by Workspace; this module only seals
//! immutable delivery snapshots and verifies producer-owned outputs.

pub(crate) mod ownership;
pub(crate) mod reopen;
pub(crate) mod tools;

pub use ownership::{FileOwner, OwnershipEntry, OwnershipLedger};
pub use reopen::{NativeHandoff, open_handoff};

use crate::Workspace;
use crate::arxml::{ProjectInput, ProjectManifest};
use crate::definitions::DefinitionCatalog;
use crate::integration::{InputSource, ValidatedIntegrationPlan};
use crate::prepared::PreparedFile;
use crate::project_model::{ExtensionDefinitionIdentity, RuleSetIdentity};
use crate::target::BuildTarget;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

pub const FORMAT: &str = "autosar-workbench-handoff-v2";
pub const HOST_PROFILE: &str = "host-can-v1";
pub const APPLICATION_SLOT: &str = "epic4-single-application-v1";
pub(crate) const PROJECT_PATH: &str = "inputs/workbench-project.json";
pub(crate) const OWNERSHIP_PATH: &str = "workbench-ownership.json";
pub(crate) const APPLICATION_OUTPUT: &str = "src/Application.c";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationSlotDescriptor {
    pub producer_slot: String,
    pub component_path: String,
    pub source_paths: Vec<String>,
    pub generated_headers: Vec<String>,
    pub entry_symbols: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceIdentities {
    pub rule_set_identity: RuleSetIdentity,
    pub required_extension_definitions: Vec<ExtensionDefinitionIdentity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputKind {
    Arxml,
    Application,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputSnapshot {
    pub logical_path: String,
    pub package_path: String,
    pub kind: InputKind,
    pub sha256: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer_slot: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandoffMetadata {
    pub format: String,
    pub producer_version: String,
    pub profile_id: String,
    pub target_id: BuildTarget,
    pub project_path: String,
    pub ownership_path: String,
    pub resource_identities: ResourceIdentities,
    pub input_snapshots: Vec<InputSnapshot>,
}

#[derive(Clone, Debug)]
pub(crate) struct SourceGuard {
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Clone, Debug)]
pub(crate) struct NativeGuard {
    pub resources: ResourceIdentities,
    pub sources: Vec<SourceGuard>,
    pub roots: Vec<PathBuf>,
    pub catalog: DefinitionCatalog,
    pub revision_identity: String,
}

impl NativeGuard {
    pub(crate) fn verify(&self, output: Option<&Path>) -> Result<(), crate::LocalizedText> {
        if crate::rules::rule_set_identity()? != self.resources.rule_set_identity {
            return Err(crate::product_message!(
                "backend.delivery.rule_identity_changed"
            ));
        }
        consumer_catalog(
            &self.catalog,
            &self.resources.required_extension_definitions,
        )?;
        for source in &self.sources {
            let bytes = read_source(&source.path)?;
            if digest(&bytes) != source.sha256 {
                return Err(
                    crate::product_message!("backend.delivery.source_changed", "path" => source.path.display()),
                );
            }
        }
        if let Some(output) = output {
            let output = comparison_path(&super::output::output_path(output)?)?;
            for root in &self.roots {
                let root = comparison_path(root)?;
                if output.starts_with(&root) || root.starts_with(&output) {
                    return Err(crate::product_message!(
                        "backend.delivery.output_source_overlap"
                    ));
                }
            }
        }
        Ok(())
    }
}

pub(crate) struct NativeInputs {
    pub manifest: ProjectManifest,
    pub configuration: Vec<(String, Vec<u8>)>,
    pub application: Vec<(String, Vec<u8>)>,
    pub resources: ResourceIdentities,
    pub definition_fingerprint: String,
    pub preparation_identity: String,
    pub guard: NativeGuard,
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>, crate::LocalizedText> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(crate) fn safe_relative(value: &str) -> Result<&Path, crate::LocalizedText> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains(['\\', ':'])
        || path.is_absolute()
        || value.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || part.ends_with(['.', ' '])
                || part
                    .chars()
                    .any(|ch| ch.is_control() || matches!(ch, '<' | '>' | '"' | '|' | '?' | '*'))
        })
        || !path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(
            crate::product_message!("backend.delivery.portable_path_unsafe", "path" => value),
        );
    }
    Ok(path)
}

pub(crate) fn refuse_links(path: &Path, allow_missing: bool) -> Result<(), crate::LocalizedText> {
    if path
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(crate::product_message!(
            "backend.delivery.parent_traversal_forbidden"
        ));
    }
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|error| crate::LocalizedText::from(error.to_string()))?
            .join(path)
    };
    for ancestor in absolute.ancestors() {
        let metadata = match fs::symlink_metadata(ancestor) {
            Ok(metadata) => metadata,
            Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("{}: {error}", ancestor.display()).into()),
        };
        #[cfg(windows)]
        let linked = {
            use std::os::windows::fs::MetadataExt;
            metadata.file_attributes() & 0x400 != 0
        };
        #[cfg(not(windows))]
        let linked = metadata.file_type().is_symlink();
        if linked {
            return Err(
                crate::product_message!("backend.delivery.linked_path_refused", "path" => ancestor.display()),
            );
        }
    }
    Ok(())
}

/// Canonicalize the existing prefix, preserving only validated future children.
/// This gives source roots and new destinations the same Windows verbatim/case
/// representation without creating anything or following links.
pub(crate) fn comparison_path(path: &Path) -> Result<PathBuf, crate::LocalizedText> {
    refuse_links(path, true)?;
    let mut existing = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|error| crate::LocalizedText::from(error.to_string()))?
            .join(path)
    };
    let mut children = Vec::new();
    loop {
        match fs::symlink_metadata(&existing) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let child = existing.file_name().ok_or_else(|| {
                    crate::product_message!("backend.delivery.existing_ancestor_required")
                })?;
                children.push(child.to_owned());
                if !existing.pop() {
                    return Err(crate::product_message!(
                        "backend.delivery.existing_ancestor_required"
                    ));
                }
            }
            Err(error) => return Err(format!("{}: {error}", existing.display()).into()),
        }
    }
    let mut canonical = fs::canonicalize(&existing)
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    for child in children.into_iter().rev() {
        canonical.push(child);
    }
    Ok(canonical)
}

pub(crate) fn read_source(path: &Path) -> Result<Vec<u8>, crate::LocalizedText> {
    refuse_links(path, false)?;
    let file = fs::File::open(path)
        .map_err(|error| crate::LocalizedText::from(format!("{}: {error}", path.display())))?;
    let metadata = file
        .metadata()
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    if !metadata.is_file() || metadata.len() > 50 * 1024 * 1024 {
        return Err(
            crate::product_message!("backend.delivery.source_boundary_invalid", "path" => path.display()),
        );
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(50 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    if bytes.len() > 50 * 1024 * 1024 {
        return Err(crate::product_message!(
            "backend.delivery.source_size_exceeded"
        ));
    }
    Ok(bytes)
}

pub(crate) fn consumer_catalog(
    catalog: &DefinitionCatalog,
    required: &[ExtensionDefinitionIdentity],
) -> Result<DefinitionCatalog, crate::LocalizedText> {
    let mut selected = catalog.clone();
    let accepted = selected.accepted_extensions();
    let mut previous = None;
    for identity in required {
        if previous.is_some_and(|id: &str| id >= identity.catalog_id.as_str())
            || identity.release != "R24-11"
            || !accepted.iter().any(|present| present == identity)
        {
            return Err(
                crate::product_message!("backend.delivery.required_extension_unaccepted", "catalog" => identity.catalog_id),
            );
        }
        previous = Some(identity.catalog_id.as_str());
    }
    for identity in accepted {
        if !required.iter().any(|needed| needed == &identity) {
            selected.remove_extension(&identity.catalog_id)?;
        }
    }
    Ok(selected)
}

fn snapshot_identity(
    manifest: &ProjectManifest,
    configuration: &[(String, Vec<u8>)],
    application: &[(String, Vec<u8>)],
    rule: &RuleSetIdentity,
    definitions: &str,
) -> Result<String, crate::LocalizedText> {
    let mut hash = Sha256::new();
    hash.update(b"autosar-native-consumer-snapshot-v1\0");
    hash.update(json_bytes(rule)?);
    hash.update(definitions.as_bytes());
    hash.update(json_bytes(manifest)?);
    let mut sources: Vec<_> = configuration.iter().chain(application).collect();
    sources.sort_by(|left, right| left.0.cmp(&right.0));
    for (path, bytes) in sources {
        hash.update((path.len() as u64).to_le_bytes());
        hash.update(path.as_bytes());
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    Ok(format!("{:x}", hash.finalize()))
}

impl NativeInputs {
    pub(crate) fn refresh_snapshot_identity(&mut self) -> Result<(), crate::LocalizedText> {
        self.preparation_identity = snapshot_identity(
            &self.manifest,
            &self.configuration,
            &self.application,
            &self.resources.rule_set_identity,
            &self.definition_fingerprint,
        )?;
        if self.guard.sources.is_empty() {
            self.guard.revision_identity = self.preparation_identity.clone();
        }
        Ok(())
    }

    pub(crate) fn from_workspace(
        workspace: &Workspace,
        required: Vec<ExtensionDefinitionIdentity>,
    ) -> Result<Self, crate::LocalizedText> {
        if workspace.uses_legacy_validation() {
            return Err(crate::product_message!(
                "backend.delivery.legacy_workspace_forbidden"
            ));
        }
        let snapshot = workspace.generation_snapshot()?;
        let rule_set_identity = crate::rules::rule_set_identity()?;
        let catalog = consumer_catalog(workspace.definition_catalog(), &required)?;
        let definition_fingerprint = catalog.fingerprint(&rule_set_identity);
        let mut guards = Vec::new();
        let mut identity = Sha256::new();
        identity.update(b"autosar-native-generation-inputs-v1\0");
        identity.update(json_bytes(&rule_set_identity)?);
        identity.update(workspace.definition_fingerprint()?.as_bytes());
        identity.update(&snapshot.manifest_bytes);
        if let Some(path) = snapshot.manifest_path {
            guards.push(SourceGuard {
                path,
                sha256: digest(&snapshot.manifest_bytes),
            });
        }
        let mut configuration = Vec::new();
        let mut application = Vec::new();
        let mut roots = BTreeSet::new();
        if let Some(root) = snapshot.project_root {
            roots.insert(root);
        }
        for (sources, target) in [
            (snapshot.inputs, &mut configuration),
            (snapshot.applications, &mut application),
        ] {
            for source in sources {
                safe_relative(&source.logical_path)?;
                identity.update((source.logical_path.len() as u64).to_le_bytes());
                identity.update(source.logical_path.as_bytes());
                identity.update((source.bytes.len() as u64).to_le_bytes());
                identity.update(&source.bytes);
                if let Some(parent) = source.disk_path.parent() {
                    roots.insert(parent.to_owned());
                }
                guards.push(SourceGuard {
                    path: source.disk_path,
                    sha256: digest(&source.bytes),
                });
                target.push((source.logical_path, source.bytes));
            }
        }
        let mut manifest = snapshot.manifest;
        manifest.accepted_extension_definitions = required.clone();
        let preparation_identity = snapshot_identity(
            &manifest,
            &configuration,
            &application,
            &rule_set_identity,
            &definition_fingerprint,
        )?;
        Ok(Self {
            manifest,
            configuration,
            application,
            resources: ResourceIdentities {
                rule_set_identity,
                required_extension_definitions: required,
            },
            definition_fingerprint,
            preparation_identity,
            guard: NativeGuard {
                resources: ResourceIdentities {
                    rule_set_identity: crate::rules::rule_set_identity()?,
                    required_extension_definitions: catalog.accepted_extensions(),
                },
                sources: guards,
                roots: roots.into_iter().collect(),
                catalog,
                revision_identity: format!("{:x}", identity.finalize()),
            },
        })
    }

    pub(crate) fn from_plan(plan: &ValidatedIntegrationPlan) -> Result<Self, crate::LocalizedText> {
        let description = plan.description();
        let identity = description
            .rule_set_identity
            .clone()
            .ok_or_else(|| crate::product_message!("backend.delivery.legacy_plan_forbidden"))?;
        if identity != crate::rules::rule_set_identity()? {
            return Err(crate::product_message!(
                "backend.delivery.plan_rule_identity_mismatch"
            ));
        }
        let catalog = DefinitionCatalog::builtin()?;
        let catalog = consumer_catalog(&catalog, &description.required_extension_definitions)?;
        let definition_fingerprint = catalog.fingerprint(&identity);
        let configuration: Vec<_> = plan
            .sources()
            .iter()
            .map(|source| (source.logical_path().to_owned(), source.bytes().to_vec()))
            .collect();
        let manifest = ProjectManifest {
            format_version: 1,
            declared_release: "R24-11".into(),
            profile_hint: description.profile.clone(),
            inputs: plan
                .sources()
                .iter()
                .map(|source| ProjectInput {
                    path: source.logical_path().into(),
                    role_hint: "source".into(),
                })
                .collect(),
            application_inputs: Vec::new(),
            accepted_extension_definitions: description.required_extension_definitions.clone(),
        };
        let resources = ResourceIdentities {
            rule_set_identity: identity,
            required_extension_definitions: description.required_extension_definitions.clone(),
        };
        let preparation_identity = snapshot_identity(
            &manifest,
            &configuration,
            &[],
            &resources.rule_set_identity,
            &definition_fingerprint,
        )?;
        Ok(Self {
            manifest,
            configuration,
            application: Vec::new(),
            resources: resources.clone(),
            definition_fingerprint,
            preparation_identity: preparation_identity.clone(),
            guard: NativeGuard {
                resources,
                sources: Vec::new(),
                roots: Vec::new(),
                catalog,
                revision_identity: preparation_identity,
            },
        })
    }

    pub(crate) fn sources(&self) -> Result<Vec<InputSource>, crate::LocalizedText> {
        self.configuration
            .iter()
            .map(|(path, bytes)| {
                InputSource::new(path, bytes.clone()).map_err(|issue| issue.message)
            })
            .collect()
    }
}

pub(crate) fn insert_file<'a>(
    files: &mut BTreeMap<String, PreparedFile<'a>>,
    path: String,
    bytes: Cow<'a, [u8]>,
) -> Result<(), crate::LocalizedText> {
    safe_relative(&path)?;
    if files
        .insert(
            path.clone(),
            PreparedFile {
                path: path.clone(),
                bytes,
                source: None,
            },
        )
        .is_some()
    {
        return Err(
            crate::product_message!("backend.delivery.producer_path_collision", "path" => path),
        );
    }
    Ok(())
}

pub(crate) fn populate_inputs<'a>(
    files: &mut BTreeMap<String, PreparedFile<'a>>,
    inputs: &mut NativeInputs,
    slot: Option<&ApplicationSlotDescriptor>,
) -> Result<Vec<InputSnapshot>, crate::LocalizedText> {
    let mut snapshots = Vec::new();
    let mut logical = BTreeSet::new();
    if inputs.configuration.len() != inputs.manifest.inputs.len()
        || inputs.application.len() != inputs.manifest.application_inputs.len()
    {
        return Err(crate::product_message!(
            "backend.delivery.workspace_snapshot_mismatch"
        ));
    }
    for member in &inputs.manifest.inputs {
        safe_relative(&member.path)?;
        if !logical.insert(member.path.to_ascii_lowercase()) {
            return Err(crate::product_message!(
                "backend.delivery.input_identity_duplicate"
            ));
        }
        let bytes = inputs
            .configuration
            .iter_mut()
            .find(|(path, _)| path == &member.path)
            .map(|(_, bytes)| bytes)
            .ok_or_else(|| crate::product_message!("backend.delivery.arxml_snapshot_missing"))?;
        let package_path = format!("inputs/{}", member.path);
        let sha256 = digest(bytes);
        // Standard rendering already delivers its real ARXML inputs. Equality is
        // mandatory; a second source cannot silently replace the plan's bytes.
        if let Some(existing) = files.get(&package_path) {
            if existing.bytes.as_ref() != bytes.as_slice() {
                return Err(
                    crate::product_message!("backend.delivery.plan_snapshot_mismatch", "path" => member.path),
                );
            }
        } else {
            insert_file(
                files,
                package_path.clone(),
                Cow::Owned(std::mem::take(bytes)),
            )?;
        }
        snapshots.push(InputSnapshot {
            logical_path: member.path.clone(),
            package_path,
            kind: InputKind::Arxml,
            sha256,
            role: member.role_hint.clone(),
            producer_slot: None,
        });
    }
    for member in &inputs.manifest.application_inputs {
        let slot = slot
            .ok_or_else(|| crate::product_message!("backend.delivery.application_slot_missing"))?;
        if member.producer_slot != slot.producer_slot
            || member.producer_slot != APPLICATION_SLOT
            || slot.source_paths != [member.path.clone()]
            || !logical.insert(member.path.to_ascii_lowercase())
        {
            return Err(crate::product_message!(
                "backend.delivery.application_membership_mismatch"
            ));
        }
        safe_relative(&member.path)?;
        let bytes = inputs
            .application
            .iter_mut()
            .find(|(path, _)| path == &member.path)
            .map(|(_, bytes)| bytes)
            .ok_or_else(|| {
                crate::product_message!("backend.delivery.application_source_missing")
            })?;
        std::str::from_utf8(bytes)
            .map_err(|_| crate::product_message!("backend.delivery.application_source_not_utf8"))?;
        let sha256 = digest(bytes);
        let contents = Cow::Owned(std::mem::take(bytes));
        if let Some(reference) = files.get_mut(APPLICATION_OUTPUT) {
            reference.bytes = contents;
            reference.source = None;
        } else {
            insert_file(files, APPLICATION_OUTPUT.into(), contents)?;
        }
        snapshots.push(InputSnapshot {
            logical_path: member.path.clone(),
            package_path: APPLICATION_OUTPUT.into(),
            kind: InputKind::Application,
            sha256,
            role: "user-application".into(),
            producer_slot: Some(member.producer_slot.clone()),
        });
    }
    insert_file(
        files,
        PROJECT_PATH.into(),
        Cow::Owned(json_bytes(&inputs.manifest)?),
    )?;
    Ok(snapshots)
}

pub(crate) fn metadata(
    profile: &str,
    target: BuildTarget,
    inputs: &NativeInputs,
    snapshots: Vec<InputSnapshot>,
) -> HandoffMetadata {
    HandoffMetadata {
        format: FORMAT.into(),
        producer_version: env!("CARGO_PKG_VERSION").into(),
        profile_id: profile.into(),
        target_id: target,
        project_path: PROJECT_PATH.into(),
        ownership_path: OWNERSHIP_PATH.into(),
        resource_identities: inputs.resources.clone(),
        input_snapshots: snapshots,
    }
}

pub(crate) fn append_native_readme(
    files: &mut BTreeMap<String, PreparedFile<'_>>,
) -> Result<(), crate::LocalizedText> {
    let readme = files
        .get_mut("README.md")
        .ok_or_else(|| crate::product_message!("backend.delivery.readme_missing"))?;
    let mut text = std::str::from_utf8(&readme.bytes)
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?
        .to_owned();
    text.push_str("\n## Native workbench source ownership\n\nThis package uses product-authored R24-11 rules, not official XSD/MOD certification. No official archive, compiler, checkout or network is needed to prepare these sources. `workbench-ownership.json` records every payload producer; the ledger, configuration/member snapshots and any user application snapshot are protected by `files.list` and `files.sha256`. These hashes detect accidental changes; they are not a publisher signature.\n\nThe live project remains separate. Explicit application initialization creates only a previously absent source for the recorded `epic4-single-application-v1` component contract. Subsequent generation reads its current bytes into the immutable compiled `src/Application.c` snapshot and never writes the live source. Do not edit a sealed snapshot or generated file, change its owner, or update hashes to hide an edit. Change the original live project and prepare a new preview instead.\n\nFor v2 handoffs, import with the matching installed product rule identity and explicitly accepted exact required extension identities into a new empty live project directory. The importer verifies the seal, rebuilds from real configuration/application snapshots and compares all regenerated bytes before publication. Legacy host/ECU v1 packages retain their original explicit official-resource compatibility checks; they are not silently upgraded.\n\nNative build and behavior verification remain separate, require the declared fixed tools, and do not inherit success from source generation. Run this package's `tools/ecu-tool.py` from its immutable tree; its v2 ownership checks precede the original sealed standalone build/verify implementation. Generated sources, live configuration/application inputs and build output must remain in separate directories.\n");
    readme.bytes = Cow::Owned(text.into_bytes());
    readme.source = None;
    Ok(())
}
