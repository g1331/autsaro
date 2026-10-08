use super::{
    HandoffMetadata, InputKind, PROJECT_PATH, consumer_catalog, digest, read_source, refuse_links,
};
use crate::Workspace;
use crate::arxml::ProjectManifest;
use crate::definitions::DefinitionCatalog;
use crate::integration::{InputSource, RuntimeCatalog};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A verified, independently reconstructed native project. The caller replaces
/// its current workspace only after this operation succeeds.
pub struct NativeHandoff {
    pub workspace: Workspace,
    pub metadata: HandoffMetadata,
}

struct PrivateDirectory(PathBuf);
impl Drop for PrivateDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct VerifiedPackage {
    directory: PrivateDirectory,
    metadata: HandoffMetadata,
    catalog: DefinitionCatalog,
    files: BTreeMap<String, Vec<u8>>,
}

fn diagnostics(errors: Vec<crate::integration::PlanDiagnostic>) -> crate::LocalizedText {
    crate::LocalizedText::messages(errors.into_iter().map(|error| {
        crate::LocalizedText::messages([
            crate::product_message!("backend.delivery.plan_diagnostic", "code" => error.code),
            error.message,
            error.remedy,
        ])
    }))
}

fn verify_members(
    manifest: &ProjectManifest,
    metadata: &HandoffMetadata,
) -> Result<(), crate::LocalizedText> {
    if manifest.declared_release != "R24-11"
        || manifest.format_version != 1
        || manifest.accepted_extension_definitions
            != metadata.resource_identities.required_extension_definitions
        || manifest.inputs.len() + manifest.application_inputs.len()
            != metadata.input_snapshots.len()
    {
        return Err(crate::product_message!(
            "backend.delivery.source_membership_mismatch"
        ));
    }
    for input in &metadata.input_snapshots {
        let matched = match input.kind {
            InputKind::Arxml => manifest
                .inputs
                .iter()
                .any(|member| member.path == input.logical_path && member.role_hint == input.role),
            InputKind::Application => manifest.application_inputs.iter().any(|member| {
                member.path == input.logical_path
                    && Some(member.producer_slot.as_str()) == input.producer_slot.as_deref()
            }),
        };
        if !matched {
            return Err(
                crate::product_message!("backend.delivery.manifest_snapshot_unowned", "path" => input.logical_path),
            );
        }
    }
    Ok(())
}

fn prepare_package(
    package: &Path,
    catalog: &DefinitionCatalog,
) -> Result<VerifiedPackage, crate::LocalizedText> {
    refuse_links(package, false)?;
    let names = super::super::output::verify_build_input(package)?;
    let target = read_source(&package.join("target.json"))?;
    let metadata = super::ownership::metadata_from_target(&target)?
        .ok_or_else(|| crate::product_message!("backend.delivery.native_v2_required"))?;
    let selected = consumer_catalog(
        catalog,
        &metadata.resource_identities.required_extension_definitions,
    )?;
    let mut payload = BTreeMap::new();
    let mut total = 0usize;
    for path in names
        .iter()
        .map(String::as_str)
        .chain(["files.list", "files.sha256"])
    {
        let bytes = read_source(&package.join(path))?;
        total = total
            .checked_add(bytes.len())
            .ok_or_else(|| crate::product_message!("backend.delivery.package_size_overflow"))?;
        if total > 512 * 1024 * 1024 {
            return Err(crate::product_message!(
                "backend.delivery.package_size_exceeded"
            ));
        }
        payload.insert(path.to_owned(), bytes);
    }
    let borrowed = payload
        .iter()
        .map(|(path, bytes)| (path.clone(), bytes.as_slice()))
        .collect();
    super::ownership::verify_ledger_bytes(&borrowed, &metadata)?;
    let manifest_bytes = payload
        .get(PROJECT_PATH)
        .ok_or_else(|| crate::product_message!("backend.delivery.source_manifest_missing"))?;
    let manifest: ProjectManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    verify_members(&manifest, &metadata)?;
    let directory = PrivateDirectory(super::super::output::reserve_directory(
        &std::env::temp_dir(),
        "native-import",
        std::ffi::OsStr::new("source"),
    )?);
    fs::write(directory.0.join("workbench-project.json"), manifest_bytes)
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    for input in &metadata.input_snapshots {
        let path = directory.0.join(super::safe_relative(&input.logical_path)?);
        fs::create_dir_all(
            path.parent()
                .ok_or_else(|| crate::product_message!("backend.delivery.input_parent_required"))?,
        )
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
        let bytes = payload
            .get(&input.package_path)
            .ok_or_else(|| crate::product_message!("backend.delivery.owned_snapshot_missing"))?;
        fs::write(path, bytes).map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    }
    let mut workspace = Workspace::open_project_with_catalog(
        &directory.0.join("workbench-project.json"),
        &selected,
    )?;
    let handoff = serde_json::from_slice::<serde_json::Value>(&target)
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?
        .get("handoff")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| crate::product_message!("backend.delivery.handoff_mode_required"))?;
    if handoff != payload.contains_key("handoff.json") {
        return Err(crate::product_message!(
            "backend.delivery.handoff_mode_mismatch"
        ));
    }
    let prepared = if metadata.profile_id == super::HOST_PROFILE {
        crate::prepared::prepare_host_project(&mut workspace, metadata.target_id, handoff)?
    } else {
        let sources = metadata
            .input_snapshots
            .iter()
            .filter(|input| input.kind == InputKind::Arxml)
            .map(|input| {
                InputSource::new(&input.logical_path, payload[&input.package_path].clone())
                    .map_err(|issue| issue.message)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let runtime = RuntimeCatalog::embedded().map_err(diagnostics)?;
        let plan = crate::integration::build_plan_native(&sources, &selected, &runtime)
            .map_err(diagnostics)?;
        if plan.description().required_extension_definitions
            != metadata.resource_identities.required_extension_definitions
        {
            return Err(crate::product_message!(
                "backend.delivery.extension_dependency_mismatch"
            ));
        }
        crate::prepared::prepare_ecu_project_for_workspace(
            &workspace,
            &plan,
            metadata.target_id,
            handoff,
        )
        .map_err(diagnostics)?
    };
    let regenerated: BTreeMap<_, _> = prepared.into_files().into_iter().collect();
    if regenerated.len() != payload.len() {
        return Err(crate::product_message!(
            "backend.delivery.reconstructed_closure_mismatch"
        ));
    }
    for (path, bytes) in &regenerated {
        if payload.get(path) != Some(bytes) {
            return Err(
                crate::product_message!("backend.delivery.trusted_payload_mismatch", "path" => path),
            );
        }
    }
    Ok(VerifiedPackage {
        directory,
        metadata,
        catalog: selected,
        files: payload,
    })
}

pub(crate) fn verify_package(
    package: &Path,
    catalog: &DefinitionCatalog,
) -> Result<(), crate::LocalizedText> {
    prepare_package(package, catalog).map(|_| ())
}

/// Import only v2, without official resources. Seal, identities, source members,
/// application and every generated byte are checked before publishing a new
/// independent source project. Existing nonempty destinations are never touched.
pub fn open_handoff(
    package: &Path,
    new_workspace_directory: &Path,
    catalog: &DefinitionCatalog,
) -> Result<NativeHandoff, crate::LocalizedText> {
    let verified = prepare_package(package, catalog)?;
    if !verified.files.contains_key("handoff.json") {
        return Err(crate::product_message!(
            "backend.delivery.explicit_handoff_required"
        ));
    }
    let output = super::super::output::output_path(new_workspace_directory)?;
    let output_identity = super::comparison_path(&output)?;
    let package = super::comparison_path(package)?;
    if output_identity.starts_with(&package) || package.starts_with(&output_identity) {
        return Err(crate::product_message!(
            "backend.delivery.import_output_overlap"
        ));
    }
    let existed = match fs::symlink_metadata(&output) {
        Ok(metadata) if metadata.is_dir() => {
            if fs::read_dir(&output)
                .map_err(|error| crate::LocalizedText::from(error.to_string()))?
                .next()
                .is_some()
            {
                return Err(crate::product_message!(
                    "backend.delivery.import_empty_directory_required"
                ));
            }
            true
        }
        Ok(_) => {
            return Err(crate::product_message!(
                "backend.delivery.import_plain_directory_required"
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error.to_string().into()),
    };
    let parent = output
        .parent()
        .ok_or_else(|| crate::product_message!("backend.delivery.import_parent_required"))?;
    fs::create_dir_all(parent).map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    let publish = PrivateDirectory(super::super::output::reserve_directory(
        parent,
        "native-source",
        output
            .file_name()
            .ok_or_else(|| crate::product_message!("backend.delivery.import_name_required"))?,
    )?);
    fs::write(
        publish.0.join("workbench-project.json"),
        &verified.files[PROJECT_PATH],
    )
    .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    for input in &verified.metadata.input_snapshots {
        let path = publish.0.join(super::safe_relative(&input.logical_path)?);
        fs::create_dir_all(
            path.parent()
                .ok_or_else(|| crate::product_message!("backend.delivery.input_parent_required"))?,
        )
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
        fs::write(path, &verified.files[&input.package_path])
            .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    }
    // The private source reconstruction remains owned until publication; verify
    // the original complete seal again so late edits cannot be confirmed.
    super::super::output::verify_build_input(&package)?;
    for (path, bytes) in &verified.files {
        if digest(&read_source(&package.join(path))?) != digest(bytes) {
            return Err(
                crate::product_message!("backend.delivery.import_confirmation_stale", "path" => path),
            );
        }
    }
    let check = Workspace::open_project_with_catalog(
        &publish.0.join("workbench-project.json"),
        &verified.catalog,
    )?;
    check.generation_snapshot()?;
    refuse_links(&output, true)?;
    if existed {
        fs::remove_dir(&output).map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    }
    if let Err(error) = fs::rename(&publish.0, &output) {
        if existed {
            fs::create_dir(&output).map_err(|restore| {
                crate::product_message!("backend.delivery.import_restore_failed", "error" => error.to_string(), "restore" => restore.to_string())
            })?;
        }
        return Err(
            crate::product_message!("backend.delivery.import_publication_failed", "error" => error.to_string()),
        );
    }
    let workspace = Workspace::open_project_with_catalog(
        &output.join("workbench-project.json"),
        &verified.catalog,
    )?;
    // Keep private reconstruction ownership explicit; Drop removes only our stage.
    drop(verified.directory);
    Ok(NativeHandoff {
        workspace,
        metadata: verified.metadata,
    })
}

#[cfg(test)]
mod localization_tests {
    use super::diagnostics;
    use crate::LocalizedText;
    use crate::integration::{DiagnosticCategory, PlanDiagnostic};

    #[test]
    fn plan_errors_preserve_code_message_remedy_and_external_evidence() {
        let evidence = "xml parser {{name}}\r\n原始证据";
        let error = diagnostics(vec![PlanDiagnostic {
            category: DiagnosticCategory::Tool,
            code: "SOURCE_CLOSURE".into(),
            file: None,
            object: None,
            message: LocalizedText::from(evidence),
            remedy: crate::product_message!("backend.prepared.source_closure_remedy",),
        }]);
        assert_eq!(
            serde_json::to_value(error).unwrap(),
            serde_json::json!([[
                {"key": "backend.delivery.plan_diagnostic", "params": {"code": "SOURCE_CLOSURE"}},
                evidence,
                {"key": "backend.prepared.source_closure_remedy", "params": {}},
            ]])
        );
    }
}
