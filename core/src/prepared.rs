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

/// Immutable source preparation. Native preflight is an explicit operation
/// whose report is bound to this project's generation fingerprint.
#[derive(Debug)]
pub struct PreparedProject<'a> {
    target: BuildTarget,
    profile: &'static str,
    handoff: bool,
    files: Vec<PreparedFile<'a>>,
    fingerprint: String,
    preflight: PreflightReport,
}

impl<'a> PreparedProject<'a> {
    pub fn target(&self) -> BuildTarget {
        self.target
    }
    pub fn profile(&self) -> &'static str {
        self.profile
    }
    pub fn handoff(&self) -> bool {
        self.handoff
    }
    pub fn files(&self) -> &[PreparedFile<'a>] {
        &self.files
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub fn preflight(&self) -> &PreflightReport {
        &self.preflight
    }

    pub fn preview(self, output: &std::path::Path) -> Result<crate::GenerationPreview, String> {
        generator::preview_prepared(&self.into_files(), output)
    }

    pub fn generate_previewed(
        self,
        output: &std::path::Path,
        revision: &str,
    ) -> Result<crate::GenerationReport, String> {
        generator::generate_prepared(self.into_files(), output, Some(revision))
    }

    pub fn native_preflight(&self, settings: &crate::target::ExecutionSettings) -> PreflightReport {
        let mut report = PreflightReport {
            status: PreflightStatus::NotRun,
            fingerprint: self.fingerprint.clone(),
            logs: Vec::new(),
        };
        if !self.target.is_native() {
            report.logs.push(format!(
                "Native preflight is not applicable to this host for {}",
                self.target.spec().id
            ));
            return report;
        }
        let result = (|| -> Result<(), String> {
            use crate::execution::{ProcessSpec, run_bounded};
            use std::ffi::OsString;
            use std::fmt::Write;
            use std::time::Duration;

            let stage = generator::reserve_directory(
                &std::env::temp_dir(),
                "ecu-preflight",
                std::ffi::OsStr::new("private"),
            )?;
            report.logs.push(format!(
                "Native preflight source/log directory: {}",
                stage.display()
            ));
            let source = stage.join("source");
            let logs = stage.join("logs");
            std::fs::create_dir(&source).map_err(|error| error.to_string())?;
            std::fs::create_dir(&logs).map_err(|error| error.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o700))
                    .map_err(|error| error.to_string())?;
            }
            let mut list = String::new();
            let mut hashes = String::new();
            for file in &self.files {
                let path = source.join(&file.path);
                std::fs::create_dir_all(path.parent().unwrap())
                    .map_err(|error| error.to_string())?;
                std::fs::write(path, &file.bytes).map_err(|error| error.to_string())?;
                writeln!(list, "{}", file.path).unwrap();
                writeln!(hashes, "{:x}  {}", Sha256::digest(&file.bytes), file.path).unwrap();
            }
            writeln!(hashes, "{:x}  files.list", Sha256::digest(list.as_bytes())).unwrap();
            std::fs::write(source.join("files.list"), list).map_err(|error| error.to_string())?;
            std::fs::write(source.join("files.sha256"), hashes)
                .map_err(|error| error.to_string())?;
            let modes: &[&str] = if self.profile == "ecu" {
                &["probe", "host-batch"]
            } else {
                &["host"]
            };
            for mode in modes {
                let argv: Vec<OsString> = vec![
                    settings.python.as_os_str().into(),
                    source.join("tools/ecu-tool.py").into_os_string(),
                    "build".into(),
                    "--project".into(),
                    source.as_os_str().into(),
                    "--output".into(),
                    stage.join(mode).into_os_string(),
                    "--mode".into(),
                    (*mode).into(),
                ];
                let spec = ProcessSpec::for_duration(
                    argv,
                    stage.clone(),
                    vec![
                        ("AUTOSAR_CC".into(), settings.compiler.as_os_str().into()),
                        (
                            "AUTOSAR_OBJDUMP".into(),
                            settings.objdump.as_os_str().into(),
                        ),
                        ("AUTOSAR_GIT".into(), settings.git.as_os_str().into()),
                    ],
                    Duration::from_secs(240),
                    logs.clone(),
                )?;
                let result = run_bounded(spec)?;
                report.logs.push(
                    std::fs::read_to_string(result.stdout).map_err(|error| error.to_string())?,
                );
                report.logs.push(
                    std::fs::read_to_string(result.stderr).map_err(|error| error.to_string())?,
                );
            }
            std::fs::remove_dir_all(stage).map_err(|error| error.to_string())?;
            Ok(())
        })();
        match result {
            Ok(()) => report.status = PreflightStatus::Passed,
            Err(error) => {
                report.status = PreflightStatus::Failed;
                report.logs.push(error);
            }
        }
        report
    }

    pub fn into_files(self) -> Vec<(String, Vec<u8>)> {
        generator::seal_files(
            self.files
                .into_iter()
                .map(|file| (file.path, file.bytes.into_owned()))
                .collect(),
        )
    }
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

pub(crate) fn deliver_path(asset: &AssetEntry, profile: &str) -> Result<String, String> {
    let path = asset.relative_path;
    if let Some(path) = path.strip_prefix("runtime/") {
        if path == "ecu-tool.py" {
            return Ok("tools/ecu-tool.py".into());
        }
        if let Some(relative) = path.strip_prefix("host/") {
            if relative.starts_with("src/") || relative.starts_with("include/") {
                return Ok(relative.into());
            }
        }
        if let Some(relative) = path.strip_prefix("ecu/") {
            if relative.starts_with("src/") || relative.starts_with("include/") {
                return Ok(relative.into());
            }
        }
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
    if let Some(path) = path.strip_prefix("scripts/ecu_tools/") {
        return Ok(format!("tools/ecu_tools/{path}"));
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
    let toolchain_asset = AssetInventory::embedded()
        .get(spec.toolchain_asset)
        .ok_or_else(|| {
            format!(
                "Pinned toolchain asset is missing: {}",
                spec.toolchain_asset
            )
        })?;
    let toolchain: serde_json::Value = serde_json::from_slice(toolchain_asset.bytes)
        .map_err(|error| format!("Invalid pinned toolchain: {error}"))?;
    let patches: Vec<_> = if profile == "ecu" {
        spec.kernel_patches
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
            .collect()
    } else {
        Vec::new()
    };
    let linux = target == BuildTarget::LinuxX64ControlledV1;
    let sources: Vec<_> = files
        .keys()
        .filter(|name| {
            name.ends_with(".c")
                && (name.starts_with("src/")
                    || name.starts_with("os/src/")
                    || name.as_str() == "Ecu_Config.c")
                && !matches!(name.as_str(), "src/ecu_probe.c" | "src/ecu_host_batch.c")
        })
        .collect();
    let mut include_paths = vec![".", "include"];
    let mut compiler_flags = vec!["-std=c99", "-O1", "-g", "-Wall", "-Wextra", "-Werror"];
    let mut kernel_sources = Vec::new();
    if profile == "ecu" {
        include_paths.extend(["os", "os/include", "os/src", "kernel/include"]);
        if linux {
            compiler_flags.extend(["-D_GNU_SOURCE", "-pthread"]);
            include_paths.extend(["os/src/host/linux", "kernel/portable/ThirdParty/GCC/Posix"]);
            kernel_sources.push("portable/ThirdParty/GCC/Posix/port.c");
        } else {
            include_paths.push("kernel/portable/MSVC-MingW");
            kernel_sources.push("portable/MSVC-MingW/port.c");
        }
        kernel_sources.extend(["tasks.c", "list.c", "queue.c"]);
    } else {
        compiler_flags.push("-pedantic");
    }
    let link_libraries: &[&str] = if profile == "ecu" {
        spec.link_libraries
    } else if linux {
        &[]
    } else {
        &["bcrypt"]
    };
    let binary_name = if profile == "ecu" {
        spec.binary_name
    } else if linux {
        "ecu_host"
    } else {
        "ecu_host.exe"
    };
    let required_sections: &[&str] = if profile == "ecu" {
        spec.required_sections
    } else {
        &[".text", ".data"]
    };
    let mut metadata = serde_json::to_vec_pretty(&serde_json::json!({
        "format": "autosar-build-target-v1",
        "target": spec.id,
        "abi": spec.abi,
        "toolchain": toolchain,
        "nativePort": if profile == "ecu" { spec.native_port } else { "C99 legacy host execution" },
        "kernelPatches": patches,
        "linkLibraries": link_libraries,
        "binaryName": binary_name,
        "objectFormat": spec.object_format,
        "requiredSections": required_sections,
        "logicalClock": if profile == "ecu" { spec.logical_clock } else { "explicit legacy host clock" },
        "sources": sources,
        "includePaths": include_paths,
        "compilerFlags": compiler_flags,
        "kernelSources": kernel_sources,
        "profile": profile,
        "handoff": handoff,
        "inputSha256": input_identity,
        "scope": "source-only preparation; native preflight not run",
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
    let rendered = plan.render_ecu_sources(target)?;
    let inventory = AssetInventory::embedded();
    let mut source_owners = BTreeMap::new();
    for asset in inventory.selected(target, "ecu") {
        let path = deliver_path(asset, "ecu").map_err(PlanDiagnostic::source_closure)?;
        source_owners.insert(path, asset);
    }
    if let Some(asset) = inventory.get("runtime/include/Com.h") {
        source_owners.insert("bsw-origin/include/Com.h".into(), asset);
    }
    let mut files = BTreeMap::new();
    for (path, bytes) in rendered {
        let source = source_owners
            .get(&path)
            .copied()
            .filter(|asset| asset.bytes == bytes.as_ref());
        insert(&mut files, path, bytes, source).map_err(PlanDiagnostic::source_closure)?;
    }
    if handoff {
        let mut metadata =
            serde_json::to_vec_pretty(&crate::integration::handoff::metadata(plan, target))
                .map_err(|error| PlanDiagnostic::source_closure(error.to_string()))?;
        metadata.push(b'\n');
        insert(
            &mut files,
            "handoff.json".into(),
            Cow::Owned(metadata),
            None,
        )
        .map_err(PlanDiagnostic::source_closure)?;
    }
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
    let readme = generator::handoff_readme(workspace.diagnostic_profile(), target, handoff)?;
    insert(
        &mut files,
        "README.md".into(),
        Cow::Owned(readme.into_bytes()),
        None,
    )?;
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
            "format": "autosar-host-handoff-v1",
            "release": "CP/FO R24-11",
            "toolVersion": env!("CARGO_PKG_VERSION"),
            "target": target,
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
