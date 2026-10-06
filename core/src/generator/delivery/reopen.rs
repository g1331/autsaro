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

fn diagnostics(errors: Vec<crate::integration::PlanDiagnostic>) -> String {
    errors
        .into_iter()
        .map(|error| format!("{}: {}", error.code, error.message))
        .collect::<Vec<_>>()
        .join("\n")
}

fn verify_members(manifest: &ProjectManifest, metadata: &HandoffMetadata) -> Result<(), String> {
    if manifest.declared_release != "R24-11"
        || manifest.format_version != 1
        || manifest.accepted_extension_definitions
            != metadata.resource_identities.required_extension_definitions
        || manifest.inputs.len() + manifest.application_inputs.len()
            != metadata.input_snapshots.len()
    {
        return Err(
            "Native source membership/release/accepted identities differ from the sealed snapshot."
                .into(),
        );
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
            return Err(format!(
                "The source manifest does not own snapshot {}",
                input.logical_path
            ));
        }
    }
    Ok(())
}

fn prepare_package(package: &Path, catalog: &DefinitionCatalog) -> Result<VerifiedPackage, String> {
    refuse_links(package, false)?;
    let names = super::super::output::verify_build_input(package)?;
    let target = read_source(&package.join("target.json"))?;
    let metadata = super::ownership::metadata_from_target(&target)?
        .ok_or("Use the explicit legacy importer for v1; a native v2 identity is required.")?;
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
            .ok_or("Native package size overflow.")?;
        if total > 512 * 1024 * 1024 {
            return Err("Native package exceeds the 512 MiB source boundary.".into());
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
        .ok_or("The native source manifest is missing.")?;
    let manifest: ProjectManifest =
        serde_json::from_slice(manifest_bytes).map_err(|error| error.to_string())?;
    verify_members(&manifest, &metadata)?;
    let directory = PrivateDirectory(super::super::output::reserve_directory(
        &std::env::temp_dir(),
        "native-import",
        std::ffi::OsStr::new("source"),
    )?);
    fs::write(directory.0.join("workbench-project.json"), manifest_bytes)
        .map_err(|error| error.to_string())?;
    for input in &metadata.input_snapshots {
        let path = directory.0.join(super::safe_relative(&input.logical_path)?);
        fs::create_dir_all(
            path.parent()
                .ok_or("An input requires a parent directory.")?,
        )
        .map_err(|error| error.to_string())?;
        let bytes = payload
            .get(&input.package_path)
            .ok_or("The owned input snapshot is missing.")?;
        fs::write(path, bytes).map_err(|error| error.to_string())?;
    }
    let mut workspace = Workspace::open_project_with_catalog(
        &directory.0.join("workbench-project.json"),
        &selected,
    )?;
    let handoff = serde_json::from_slice::<serde_json::Value>(&target)
        .map_err(|error| error.to_string())?
        .get("handoff")
        .and_then(serde_json::Value::as_bool)
        .ok_or("Native target must declare handoff mode.")?;
    if handoff != payload.contains_key("handoff.json") {
        return Err("Native handoff mode and actual metadata file differ.".into());
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
            return Err(
                "Declared extension closure is not the real native consumer dependency set.".into(),
            );
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
        return Err(
            "Native source reconstruction changes the entire sealed ownership closure.".into(),
        );
    }
    for (path, bytes) in &regenerated {
        if payload.get(path) != Some(bytes) {
            return Err(format!(
                "Native payload differs from its actual trusted source producer: {path}"
            ));
        }
    }
    Ok(VerifiedPackage {
        directory,
        metadata,
        catalog: selected,
        files: payload,
    })
}

pub(crate) fn verify_package(package: &Path, catalog: &DefinitionCatalog) -> Result<(), String> {
    prepare_package(package, catalog).map(|_| ())
}

/// Import only v2, without official resources. Seal, identities, source members,
/// application and every generated byte are checked before publishing a new
/// independent source project. Existing nonempty destinations are never touched.
pub fn open_handoff(
    package: &Path,
    new_workspace_directory: &Path,
    catalog: &DefinitionCatalog,
) -> Result<NativeHandoff, String> {
    let verified = prepare_package(package, catalog)?;
    if !verified.files.contains_key("handoff.json") {
        return Err("This is source-only output, not an explicitly sealed v2 handoff.".into());
    }
    let output = super::super::output::output_path(new_workspace_directory)?;
    let output_identity = super::comparison_path(&output)?;
    let package = super::comparison_path(package)?;
    if output_identity.starts_with(&package) || package.starts_with(&output_identity) {
        return Err("Imported project and immutable handoff must remain independent.".into());
    }
    let existed = match fs::symlink_metadata(&output) {
        Ok(metadata) if metadata.is_dir() => {
            if fs::read_dir(&output)
                .map_err(|error| error.to_string())?
                .next()
                .is_some()
            {
                return Err("Native import requires a new or empty project directory.".into());
            }
            true
        }
        Ok(_) => return Err("Native import destination is not a plain directory.".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error.to_string()),
    };
    let parent = output
        .parent()
        .ok_or("Native import requires a parent directory.")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let publish = PrivateDirectory(super::super::output::reserve_directory(
        parent,
        "native-source",
        output
            .file_name()
            .ok_or("Native import requires a directory name.")?,
    )?);
    fs::write(
        publish.0.join("workbench-project.json"),
        &verified.files[PROJECT_PATH],
    )
    .map_err(|error| error.to_string())?;
    for input in &verified.metadata.input_snapshots {
        let path = publish.0.join(super::safe_relative(&input.logical_path)?);
        fs::create_dir_all(path.parent().ok_or("Input requires a parent directory.")?)
            .map_err(|error| error.to_string())?;
        fs::write(path, &verified.files[&input.package_path]).map_err(|error| error.to_string())?;
    }
    // The private source reconstruction remains owned until publication; verify
    // the original complete seal again so late edits cannot be confirmed.
    super::super::output::verify_build_input(&package)?;
    for (path, bytes) in &verified.files {
        if digest(&read_source(&package.join(path))?) != digest(bytes) {
            return Err(format!(
                "Native handoff changed during import confirmation: {path}"
            ));
        }
    }
    let check = Workspace::open_project_with_catalog(
        &publish.0.join("workbench-project.json"),
        &verified.catalog,
    )?;
    check.generation_snapshot()?;
    refuse_links(&output, true)?;
    if existed {
        fs::remove_dir(&output).map_err(|error| error.to_string())?;
    }
    if let Err(error) = fs::rename(&publish.0, &output) {
        if existed {
            fs::create_dir(&output).map_err(|restore| {
                format!("Import failed: {error}; empty directory restore failed: {restore}")
            })?;
        }
        return Err(format!("Native project publication failed: {error}"));
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
