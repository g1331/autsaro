use crate::Workspace;
use crate::generator;
use crate::integration::{DiagnosticCategory, PlanDiagnostic, ValidatedIntegrationPlan};
use crate::resources::{AssetEntry, AssetInventory};
use crate::target::BuildTarget;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::BTreeMap;
impl PlanDiagnostic {
    fn source_closure(message: impl Into<String>) -> Vec<Self> {
        vec![Self {
            category: DiagnosticCategory::Tool,
            code: "SOURCE_CLOSURE".into(),
            file: None,
            object: None,
            message: message.into(),
            remedy: "Restore the matching compiled resource inventory and validated input plan."
                .into(),
        }]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightStatus {
    NotRun,
    Passed,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightReport {
    pub status: PreflightStatus,
    pub fingerprint: String,
    pub logs: Vec<String>,
}

#[derive(Debug)]
pub struct PreparedFile<'a> {
    pub path: String,
    pub bytes: Cow<'a, [u8]>,
    pub source: Option<&'static AssetEntry>,
}

/// A source-only preview. No executable or sealed deliverable can be
/// installed from this stage's preparation result.
#[derive(Debug)]
pub struct PreparedProject<'a> {
    pub target: BuildTarget,
    pub profile: &'static str,
    pub handoff: bool,
    pub files: Vec<PreparedFile<'a>>,
    pub fingerprint: String,
    pub preflight: PreflightReport,
}

fn insert<'a>(
    files: &mut BTreeMap<String, PreparedFile<'a>>,
    path: String,
    bytes: Cow<'a, [u8]>,
    source: Option<&'static AssetEntry>,
) -> Result<(), String> {
    if files
        .insert(
            path.clone(),
            PreparedFile {
                path: path.clone(),
                bytes,
                source,
            },
        )
        .is_some()
    {
        return Err(format!(
            "Generated source path collides with a trusted asset: {path}"
        ));
    }
    Ok(())
}

fn deliver_path(asset: &AssetEntry, profile: &str) -> Result<String, String> {
    let path = asset.relative_path;
    if let Some(path) = path.strip_prefix("runtime/") {
        if profile == "ecu" && path == "include/Os.h" {
            return Ok("bsw-origin/include/Os.h".into());
        }
        if profile == "ecu" && path.starts_with("os/") {
            return Ok(path.into());
        }
        if path.starts_with("src/") || path.starts_with("include/") {
            return Ok(path.into());
        }
        if path.starts_with("contracts/") {
            return Ok(format!("inventory/{path}"));
        }
        return Ok(format!("tools/{path}"));
    }
    if let Some(path) = path.strip_prefix("third_party/freertos/") {
        if profile != "ecu" {
            return Err("A legacy host project cannot include a FreeRTOS kernel".into());
        }
        if path == "include/StackMacros.h" {
            return Ok("kernel-compat/include/StackMacros.h".into());
        }
        return Ok(format!("kernel/{path}"));
    }
    Err(format!(
        "Asset is outside the selected source roots: {path}"
    ))
}

fn start<'a>(
    target: BuildTarget,
    profile: &'static str,
) -> Result<BTreeMap<String, PreparedFile<'a>>, String> {
    let inventory = AssetInventory::embedded();
    let mut files = BTreeMap::new();
    for asset in inventory.selected(target, profile) {
        insert(
            &mut files,
            deliver_path(asset, profile)?,
            Cow::Borrowed(asset.bytes),
            Some(asset),
        )?;
    }
    Ok(files)
}

fn finish<'a>(
    target: BuildTarget,
    profile: &'static str,
    handoff: bool,
    mut files: BTreeMap<String, PreparedFile<'a>>,
    input_identity: Option<&str>,
) -> Result<PreparedProject<'a>, String> {
    let spec = target.spec();
    let toolchain: serde_json::Value = serde_json::from_str(spec.toolchain_lock)
        .map_err(|error| format!("Invalid pinned toolchain: {error}"))?;
    let patches: Vec<_> = spec
        .kernel_patches
        .iter()
        .map(|path| {
            let includes = spec
                .kernel_patch_includes
                .iter()
                .find(|(candidate, _)| candidate == path)
                .map(|(_, includes)| *includes)
                .unwrap_or(&[]);
            serde_json::json!({ "path": path, "includes": includes })
        })
        .collect();
    let mut metadata = serde_json::to_vec_pretty(&serde_json::json!({
        "format": "autosar-build-target-v1",
        "target": spec.id,
        "abi": spec.abi,
        "toolchain": toolchain,
        "nativePort": spec.native_port,
        "kernelPatches": patches,
        "linkLibraries": spec.link_libraries,
        "binaryName": spec.binary_name,
        "objectFormat": spec.object_format,
        "requiredSections": spec.required_sections,
        "logicalClock": spec.logical_clock,
        "profile": profile,
        "handoff": handoff,
        "inputSha256": input_identity,
        "scope": if target == BuildTarget::LinuxX64ControlledV1 && profile == "ecu" {
            "uninstalled source-only preview; Linux ECU native bridge and build absent"
        } else {
            "uninstalled source-only preview; native preflight not run"
        },
    }))
    .map_err(|error| error.to_string())?;
    metadata.push(b'\n');
    insert(&mut files, "target.json".into(), Cow::Owned(metadata), None)?;
    let mut digest = Sha256::new();
    for file in files.values() {
        digest.update((file.path.len() as u64).to_le_bytes());
        digest.update(file.path.as_bytes());
        digest.update((file.bytes.len() as u64).to_le_bytes());
        digest.update(file.bytes.as_ref());
        if let Some(asset) = file.source {
            digest.update(asset.relative_path.as_bytes());
            digest.update(asset.sha256.as_bytes());
        }
    }
    let fingerprint = format!("{:x}", digest.finalize());
    Ok(PreparedProject {
        target,
        profile,
        handoff,
        files: files.into_values().collect(),
        preflight: PreflightReport {
            status: PreflightStatus::NotRun,
            fingerprint: fingerprint.clone(),
            logs: Vec::new(),
        },
        fingerprint,
    })
}

pub fn prepare_ecu_project<'a>(
    plan: &'a ValidatedIntegrationPlan,
    target: BuildTarget,
    handoff: bool,
) -> Result<PreparedProject<'a>, Vec<PlanDiagnostic>> {
    let mut files = start(target, "ecu").map_err(PlanDiagnostic::source_closure)?;
    let contract = plan.component_contract_files()?;
    for (path, bytes) in contract.into_files() {
        if matches!(path.as_str(), "README.md" | "files.list" | "files.sha256") {
            continue;
        }
        if files.get(&path).is_some_and(|asset| asset.bytes == bytes) {
            continue;
        }
        insert(&mut files, path, Cow::Owned(bytes), None)
            .map_err(PlanDiagnostic::source_closure)?;
    }
    for source in plan.sources() {
        insert(
            &mut files,
            format!("inputs/{}", source.logical_path()),
            Cow::Borrowed(source.bytes()),
            None,
        )
        .map_err(PlanDiagnostic::source_closure)?;
    }
    let mut plan_bytes = serde_json::to_vec_pretty(plan.description())
        .map_err(|error| PlanDiagnostic::source_closure(error.to_string()))?;
    plan_bytes.push(b'\n');
    insert(
        &mut files,
        "integration.json".into(),
        Cow::Owned(plan_bytes),
        None,
    )
    .map_err(PlanDiagnostic::source_closure)?;
    finish(target, "ecu", handoff, files, None).map_err(PlanDiagnostic::source_closure)
}

pub fn prepare_host_project(
    workspace: &mut Workspace,
    target: BuildTarget,
    handoff: bool,
) -> Result<PreparedProject<'static>, String> {
    let saved = if handoff {
        Some(workspace.handoff_sources()?)
    } else {
        None
    };
    let generated = generator::render_host_profile(workspace, target)?;
    let input_identity = workspace.preparation_input_identity();
    let mut files = start(target, "host")?;
    for (path, bytes) in generated {
        insert(&mut files, path, Cow::Owned(bytes), None)?;
    }
    if let Some(sources) = saved {
        let mut identities = Vec::with_capacity(sources.len());
        for (index, source) in sources.into_iter().enumerate() {
            let path = format!("inputs/{index:03}.arxml");
            identities.push(serde_json::json!({
                "path": path,
                "originalName": source.original_name,
                "packageRoots": source.package_roots,
                "sha256": format!("{:x}", Sha256::digest(&source.contents)),
            }));
            insert(&mut files, path, Cow::Owned(source.contents), None)?;
        }
        let mut metadata = serde_json::to_vec_pretty(&serde_json::json!({
            "format": "autosar-host-source-inputs-v1",
            "sources": identities,
        }))
        .map_err(|error| error.to_string())?;
        metadata.push(b'\n');
        insert(
            &mut files,
            "handoff.json".into(),
            Cow::Owned(metadata),
            None,
        )?;
    }
    finish(target, "host", handoff, files, Some(&input_identity))
}
