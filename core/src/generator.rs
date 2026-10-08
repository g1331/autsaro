pub mod delivery;
pub(crate) mod output;
pub(crate) mod render;

pub use output::{StagedBuild, StagedGeneration};

use crate::arxml::Workspace;
use crate::model::{BuildReport, GenerationPreview, GenerationReport};
use crate::target::BuildTarget;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub fn preview_generate(
    workspace: &mut Workspace,
    output: &Path,
    target: BuildTarget,
) -> Result<GenerationPreview, crate::LocalizedText> {
    crate::prepare_host_project(workspace, target, false)?.preview(output)
}

pub fn preview_handoff(
    workspace: &mut Workspace,
    output: &Path,
    target: BuildTarget,
) -> Result<GenerationPreview, crate::LocalizedText> {
    crate::prepare_host_project(workspace, target, true)?.preview(output)
}

pub fn generate_previewed(
    workspace: &mut Workspace,
    output: &Path,
    revision: &str,
    target: BuildTarget,
) -> Result<GenerationReport, crate::LocalizedText> {
    crate::prepare_host_project(workspace, target, false)?.generate_previewed(output, revision)
}

pub fn generate_handoff_previewed(
    workspace: &mut Workspace,
    output: &Path,
    revision: &str,
    target: BuildTarget,
) -> Result<GenerationReport, crate::LocalizedText> {
    crate::prepare_host_project(workspace, target, true)?.generate_previewed(output, revision)
}

pub fn generate_handoff(
    workspace: &mut Workspace,
    output: &Path,
    target: BuildTarget,
) -> Result<GenerationReport, crate::LocalizedText> {
    output::generate_prepared(
        crate::prepare_host_project(workspace, target, true)?.into_files(),
        output,
        None,
    )
}

pub fn open_handoff(
    output: &Path,
    schema_archive: PathBuf,
) -> Result<Workspace, crate::LocalizedText> {
    let names = output::verify_build_input(output).map_err(|error| {
        crate::LocalizedText::messages([
            crate::product_message!("backend.generation.legacy_handoff_integrity_failed"),
            error,
        ])
    })?;
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(output.join("handoff.json")).map_err(|e| crate::product_message!("backend.generation.legacy_handoff_mapping_missing", "error" => e.to_string()))?,
    )
    .map_err(|e| crate::product_message!("backend.generation.legacy_handoff_mapping_invalid", "error" => e.to_string()))?;
    if metadata["format"] != "autosar-host-handoff-v1"
        || metadata["release"] != "CP/FO R24-11"
        || metadata["toolVersion"] != env!("CARGO_PKG_VERSION")
    {
        return Err(crate::product_message!(
            "backend.generation.legacy_handoff_identity_mismatch"
        ));
    }
    let target: BuildTarget = serde_json::from_value(metadata["target"].clone())
        .map_err(|error| crate::product_message!("backend.generation.legacy_handoff_target_unsupported", "error" => error.to_string()))?;
    let sources = metadata["sources"]
        .as_array()
        .filter(|items| !items.is_empty())
        .ok_or_else(|| {
            crate::product_message!("backend.generation.legacy_handoff_inputs_missing")
        })?;
    let mut paths = Vec::with_capacity(sources.len());
    let mut original_names = std::collections::BTreeMap::new();
    for (index, source) in sources.iter().enumerate() {
        let expected = format!("inputs/{index:03}.arxml");
        if source["path"] != expected
            || !source["originalName"].as_str().is_some_and(|name| {
                !name.is_empty()
                    && name != "."
                    && name != ".."
                    && !name.contains('/')
                    && !name.contains('\\')
            })
            || source["packageRoots"].as_array().is_none()
        {
            return Err(
                crate::product_message!("backend.generation.legacy_handoff_input_mapping_invalid", "path" => expected),
            );
        }
        let path = output.join(&expected);
        if !names.contains(&expected) || source["sha256"] != output::file_digest(&path)? {
            return Err(
                crate::product_message!("backend.generation.legacy_handoff_input_digest_mismatch", "path" => expected),
            );
        }
        original_names.insert(
            path.canonicalize()
                .map_err(|error| crate::LocalizedText::from(error.to_string()))?,
            source["originalName"].as_str().unwrap().to_owned(),
        );
        paths.push(path);
    }
    if names
        .iter()
        .filter(|name| name.starts_with("inputs/"))
        .count()
        != paths.len()
    {
        return Err(crate::product_message!(
            "backend.generation.legacy_handoff_inputs_mismatch"
        ));
    }
    let mut workspace = Workspace::open_legacy(paths, schema_archive)?;
    workspace.restore_handoff_source_names(original_names)?;
    let expected = crate::prepare_host_project(&mut workspace, target, true)?.into_files();
    let expected_names: Vec<_> = expected
        .iter()
        .filter(|(name, _)| name != "files.list" && name != "files.sha256")
        .map(|(name, _)| name.clone())
        .collect();
    if names != expected_names {
        return Err(crate::product_message!(
            "backend.generation.legacy_handoff_closure_mismatch"
        ));
    }
    for (name, bytes) in expected {
        if fs::read(output.join(&name))
            .map_err(|error| crate::LocalizedText::from(error.to_string()))?
            != bytes
        {
            return Err(
                crate::product_message!("backend.generation.legacy_handoff_source_mismatch", "path" => name),
            );
        }
    }
    Ok(workspace)
}

pub fn generate(
    workspace: &mut Workspace,
    output: &Path,
    target: BuildTarget,
) -> Result<GenerationReport, crate::LocalizedText> {
    output::generate_prepared(
        crate::prepare_host_project(workspace, target, false)?.into_files(),
        output,
        None,
    )
}

pub fn build(
    project: &Path,
    output: &Path,
    settings: &crate::target::ExecutionSettings,
    owner: &crate::execution::ProcessOwner,
) -> Result<BuildReport, crate::LocalizedText> {
    output::verify_build_input(project)?;
    let project = project
        .canonicalize()
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(project.join("target.json"))
            .map_err(|error| crate::LocalizedText::from(error.to_string()))?,
    )
    .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    if metadata["format"] != "autosar-build-target-v1" || metadata["profile"] != "host" {
        return Err(crate::product_message!(
            "backend.generation.legacy_host_profile_mismatch"
        ));
    }
    let target: BuildTarget = serde_json::from_value(metadata["target"].clone())
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    let output = output::output_path(output)?;
    let capture =
        output::reserve_directory(&std::env::temp_dir(), "host-build", OsStr::new("private"))?;
    let log = crate::integration::handoff::run_tool(
        &project,
        settings,
        vec![
            "build".into(),
            "--project".into(),
            project.as_os_str().into(),
            "--output".into(),
            output.as_os_str().into(),
            "--mode".into(),
            "host".into(),
        ],
        &capture,
        owner,
    )
    .map_err(|error| crate::LocalizedText::messages([error, crate::product_message!("backend.generation.build_diagnostics_retained", "path" => capture.display())]))?;
    output::verify_build_input(&project)?;
    let binary = output.join(if target == BuildTarget::WindowsX64ControlledV1 {
        "ecu_host.exe"
    } else {
        "ecu_host"
    });
    if !binary.is_file() {
        return Err(crate::product_message!(
            "backend.generation.legacy_binary_missing"
        ));
    }
    fs::remove_dir_all(capture).map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    Ok(BuildReport {
        binary_path: binary.display().to_string(),
        log,
    })
}
