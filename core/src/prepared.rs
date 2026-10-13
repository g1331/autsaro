use crate::Workspace;
use crate::generator;
use crate::generator::delivery::{
    ApplicationSlotDescriptor, HandoffMetadata, NativeGuard, NativeInputs,
};
use crate::integration::{DiagnosticCategory, PlanDiagnostic, ValidatedIntegrationPlan};
use crate::resources::{AssetEntry, AssetInventory};
use crate::target::BuildTarget;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::BTreeMap;
impl PlanDiagnostic {
    fn source_closure(message: impl Into<crate::LocalizedText>) -> Vec<Self> {
        vec![Self {
            category: DiagnosticCategory::Tool,
            code: "SOURCE_CLOSURE".into(),
            file: None,
            object: None,
            message: message.into(),
            remedy: crate::product_message!("backend.prepared.source_closure_remedy"),
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
    pub logs: Vec<crate::LocalizedText>,
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
    handoff: bool,
    files: Vec<PreparedFile<'a>>,
    fingerprint: String,
    preflight: PreflightReport,
    native_guard: Option<NativeGuard>,
    native_metadata: Option<HandoffMetadata>,
    definition_fingerprint: Option<String>,
    application_slots: Vec<ApplicationSlotDescriptor>,
}

impl<'a> PreparedProject<'a> {
    pub fn target(&self) -> BuildTarget {
        self.target
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

    pub fn rule_set_identity(&self) -> Option<&crate::project_model::RuleSetIdentity> {
        self.native_metadata
            .as_ref()
            .map(|metadata| &metadata.resource_identities.rule_set_identity)
    }

    pub fn definition_fingerprint(&self) -> Option<&str> {
        self.definition_fingerprint.as_deref()
    }

    pub fn application_slot(&self) -> Option<&ApplicationSlotDescriptor> {
        self.application_slots.first()
    }

    /// Caller-owned source producers from the actual validated component graph.
    pub fn application_slots(&self) -> &[ApplicationSlotDescriptor] {
        &self.application_slots
    }

    pub fn preview(
        mut self,
        output: &std::path::Path,
    ) -> Result<crate::GenerationPreview, crate::LocalizedText> {
        let guard = self.native_guard.take();
        if let Some(guard) = &guard {
            guard.verify(Some(output))?;
        }
        generator::output::preview_prepared_checked(&self.into_files(), output, guard.as_ref())
    }

    pub fn generate_previewed(
        self,
        output: &std::path::Path,
        revision: &str,
    ) -> Result<crate::GenerationReport, crate::LocalizedText> {
        self.stage(output, Some(revision))?.commit()
    }

    pub fn stage_previewed(
        self,
        output: &std::path::Path,
        revision: &str,
    ) -> Result<generator::StagedGeneration, crate::LocalizedText> {
        self.stage(output, Some(revision))
    }

    pub fn generate(
        self,
        output: &std::path::Path,
    ) -> Result<crate::GenerationReport, crate::LocalizedText> {
        self.stage(output, None)?.commit()
    }

    fn stage(
        mut self,
        output: &std::path::Path,
        revision: Option<&str>,
    ) -> Result<generator::StagedGeneration, crate::LocalizedText> {
        let guard = self.native_guard.take();
        if let Some(guard) = &guard {
            guard.verify(Some(output))?;
        }
        generator::output::StagedGeneration::new_guarded(self.into_files(), output, revision, guard)
    }

    pub fn native_preflight(
        &self,
        settings: &crate::target::ExecutionSettings,
        owner: &crate::execution::ProcessOwner,
    ) -> PreflightReport {
        let mut report = PreflightReport {
            status: PreflightStatus::NotRun,
            fingerprint: self.fingerprint.clone(),
            logs: Vec::new(),
        };
        if !self.target.is_native() {
            report.logs.push(crate::product_message!("backend.prepared.preflight_not_applicable", "target" => self.target.spec().id));
            return report;
        }
        if let Some(guard) = &self.native_guard {
            if let Err(error) = guard.verify(None) {
                report.status = PreflightStatus::Failed;
                report.logs.push(error);
                return report;
            }
        }
        let result = (|| -> Result<(), crate::LocalizedText> {
            use crate::execution::ProcessSpec;
            use std::ffi::OsString;
            use std::fmt::Write;
            use std::time::Duration;

            let stage = generator::output::reserve_directory(
                &std::env::temp_dir(),
                "ecu-preflight",
                std::ffi::OsStr::new("private"),
            )?;
            report.logs.push(crate::product_message!("backend.prepared.preflight_capture_directory", "path" => stage.display()));
            let source = stage.join("source");
            let logs = stage.join("logs");
            std::fs::create_dir(&source)
                .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
            std::fs::create_dir(&logs)
                .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o700))
                    .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
            }
            let mut list = String::new();
            let mut hashes = String::new();
            for file in &self.files {
                let path = source.join(&file.path);
                std::fs::create_dir_all(path.parent().unwrap())
                    .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
                std::fs::write(path, &file.bytes)
                    .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
                writeln!(list, "{}", file.path).unwrap();
                writeln!(hashes, "{:x}  {}", Sha256::digest(&file.bytes), file.path).unwrap();
            }
            writeln!(hashes, "{:x}  files.list", Sha256::digest(list.as_bytes())).unwrap();
            std::fs::write(source.join("files.list"), list)
                .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
            std::fs::write(source.join("files.sha256"), hashes)
                .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
            for mode in ["probe", "host-batch"] {
                let argv: Vec<OsString> = vec![
                    settings.python.as_os_str().into(),
                    "-I".into(),
                    "-S".into(),
                    source.join("tools/ecu-tool.py").into_os_string(),
                    "build".into(),
                    "--project".into(),
                    source.as_os_str().into(),
                    "--output".into(),
                    stage.join(mode).into_os_string(),
                    "--mode".into(),
                    mode.into(),
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
                let result = owner.run(spec)?;
                report.logs.push(
                    std::fs::read_to_string(result.stdout)
                        .map_err(|error| crate::LocalizedText::from(error.to_string()))?
                        .into(),
                );
                report.logs.push(
                    std::fs::read_to_string(result.stderr)
                        .map_err(|error| crate::LocalizedText::from(error.to_string()))?
                        .into(),
                );
            }
            std::fs::remove_dir_all(stage)
                .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
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
        generator::output::seal_files(
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
) -> Result<(), crate::LocalizedText> {
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
        return Err(
            crate::product_message!("backend.prepared.source_asset_collision", "path" => path),
        );
    }
    Ok(())
}

pub(crate) fn deliver_path(asset: &AssetEntry) -> Result<String, crate::LocalizedText> {
    let path = asset.relative_path;
    if matches!(path, "LICENSE" | "NOTICE") {
        return Ok(path.into());
    }
    if let Some(path) = path.strip_prefix("runtime/") {
        if path == "ecu-tool.py" {
            return Ok("tools/ecu-tool.py".into());
        }
        for family in ["bsw/", "ecu/"] {
            if let Some(relative) = path.strip_prefix(family) {
                if relative.starts_with("src/") || relative.starts_with("include/") {
                    return Ok(relative.into());
                }
            }
        }
        if path.starts_with("os/") {
            return Ok(path.into());
        }
        if path.starts_with("contracts/") {
            return Ok(format!("inventory/{path}"));
        }
        return Ok(format!("tools/{path}"));
    }
    if let Some(path) = path.strip_prefix("third_party/freertos/") {
        if path == "include/StackMacros.h" {
            return Ok("kernel-compat/include/StackMacros.h".into());
        }
        return Ok(format!("kernel/{path}"));
    }
    if let Some(path) = path.strip_prefix("tools/python/src/ecu_tools/") {
        return Ok(format!("tools/ecu_tools/{path}"));
    }
    Err(crate::product_message!("backend.prepared.asset_source_root_invalid", "path" => path))
}

struct NativePreparation {
    metadata: HandoffMetadata,
    definition_fingerprint: String,
    preparation_identity: String,
    guard: NativeGuard,
    application_slots: Vec<ApplicationSlotDescriptor>,
}

impl NativePreparation {
    fn new(
        files: &mut BTreeMap<String, PreparedFile<'_>>,
        mut inputs: NativeInputs,
        slots: Vec<ApplicationSlotDescriptor>,
        target: BuildTarget,
        handoff: bool,
    ) -> Result<Self, crate::LocalizedText> {
        let profile = crate::integration::PROFILE;
        if inputs.application.len() != slots.len() {
            return Err(crate::product_message!(
                "backend.integration.composition.consumer_unsupported"
            )
            .into());
        }
        inputs.manifest.profile_hint = profile.into();
        inputs.refresh_snapshot_identity()?;
        let snapshots = generator::delivery::populate_inputs(files, &mut inputs, &slots)?;
        let metadata = generator::delivery::metadata(profile, target, &inputs, snapshots);
        generator::delivery::ownership::validate_metadata(&metadata)?;
        if handoff {
            generator::delivery::insert_file(
                files,
                "handoff.json".into(),
                Cow::Owned(generator::delivery::json_bytes(&metadata)?),
            )?;
        }
        generator::delivery::append_native_readme(files, &metadata, &slots)?;
        Ok(Self {
            metadata,
            definition_fingerprint: inputs.definition_fingerprint,
            preparation_identity: inputs.preparation_identity,
            guard: inputs.guard,
            application_slots: slots,
        })
    }
}

fn finish<'a>(
    target: BuildTarget,
    handoff: bool,
    mut files: BTreeMap<String, PreparedFile<'a>>,
    native: NativePreparation,
) -> Result<PreparedProject<'a>, crate::LocalizedText> {
    let spec = target.spec();
    let toolchain_asset = AssetInventory::embedded()
        .get(spec.toolchain_asset)
        .ok_or_else(|| {
            crate::product_message!("backend.prepared.toolchain_asset_missing", "path" => spec.toolchain_asset)
        })?;
    let toolchain: serde_json::Value = serde_json::from_slice(toolchain_asset.bytes)
        .map_err(|error| crate::product_message!("backend.prepared.toolchain_invalid", "error" => error.to_string()))?;
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
    let mut include_paths = vec![
        ".",
        "include",
        "os",
        "os/include",
        "os/src",
        "kernel/include",
    ];
    let mut compiler_flags = vec!["-std=c99", "-O1", "-g", "-Wall", "-Wextra", "-Werror"];
    let mut kernel_sources = Vec::new();
    if linux {
        compiler_flags.extend(["-D_GNU_SOURCE", "-pthread"]);
        include_paths.extend(["os/src/host/linux", "kernel/portable/ThirdParty/GCC/Posix"]);
        kernel_sources.push("portable/ThirdParty/GCC/Posix/port.c");
    } else {
        include_paths.push("kernel/portable/MSVC-MingW");
        kernel_sources.push("portable/MSVC-MingW/port.c");
    }
    kernel_sources.extend(["tasks.c", "list.c", "queue.c"]);
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
        "sources": sources,
        "includePaths": include_paths,
        "compilerFlags": compiler_flags,
        "kernelSources": kernel_sources,
        "profile": "ecu",
        "handoff": handoff,
        "inputSha256": native.preparation_identity,
        "scope": "source-only preparation; native preflight not run",
        "nativeDelivery": native.metadata,
        "definitionFingerprint": native.definition_fingerprint,
    }))
    .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    metadata.push(b'\n');
    insert(&mut files, "target.json".into(), Cow::Owned(metadata), None)?;
    generator::delivery::tools::add(&mut files, &native.metadata)?;
    generator::delivery::ownership::add_ledger(&mut files, &native.metadata)?;
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
    digest.update(native.guard.revision_identity.as_bytes());
    let fingerprint = format!("{:x}", digest.finalize());
    Ok(PreparedProject {
        target,
        handoff,
        files: files.into_values().collect(),
        preflight: PreflightReport {
            status: PreflightStatus::NotRun,
            fingerprint: fingerprint.clone(),
            logs: Vec::new(),
        },
        fingerprint,
        native_guard: Some(native.guard),
        native_metadata: Some(native.metadata),
        definition_fingerprint: Some(native.definition_fingerprint),
        application_slots: native.application_slots,
    })
}

/// One caller-owned application producer, selected by its actual component
/// instance path. The physical source path is never a generated output path.
#[derive(Clone, Debug)]
pub struct ApplicationSource {
    pub component_instance: String,
    pub path: std::path::PathBuf,
}

/// Prepare a complete ECU with every caller application frozen
/// into the normal immutable source project. No reference algorithms are supplied.
pub fn prepare_ecu_project_with_applications(
    plan: &ValidatedIntegrationPlan,
    target: BuildTarget,
    sources: &[ApplicationSource],
) -> Result<PreparedProject<'static>, Vec<PlanDiagnostic>> {
    let slots = plan.application_slot_descriptors()?;
    let mut inputs = NativeInputs::from_plan(plan).map_err(PlanDiagnostic::source_closure)?;
    let mut members = Vec::new();
    let mut applications = Vec::new();
    let mut paths = std::collections::BTreeSet::new();
    let mut producers = std::collections::BTreeSet::new();
    if sources.len() != slots.len() {
        return Err(PlanDiagnostic::source_closure(crate::product_message!(
            "backend.delivery.application_membership_mismatch"
        )));
    }
    for source in sources {
        let producer = format!(
            "{}:{}",
            crate::integration::PROFILE,
            source.component_instance
        );
        let slot = slots
            .iter()
            .find(|slot| slot.producer_slot == producer)
            .ok_or_else(|| {
                PlanDiagnostic::source_closure(crate::product_message!(
                    "backend.delivery.application_membership_mismatch"
                ))
            })?;
        let bytes = generator::delivery::read_source(&source.path)
            .map_err(PlanDiagnostic::source_closure)?;
        std::str::from_utf8(&bytes).map_err(|_| {
            PlanDiagnostic::source_closure(crate::product_message!(
                "backend.delivery.application_source_not_utf8"
            ))
        })?;
        let path = generator::delivery::comparison_path(&source.path)
            .map_err(PlanDiagnostic::source_closure)?;
        if !paths.insert(path.clone()) || !producers.insert(producer.clone()) {
            return Err(PlanDiagnostic::source_closure(crate::product_message!(
                "backend.delivery.application_membership_mismatch"
            )));
        }
        let logical = slot.source_paths[0].clone();
        members.push(crate::arxml::ApplicationInput {
            path: logical.clone(),
            producer_slot: producer,
        });
        applications.push((logical, bytes.clone()));
        inputs.guard.sources.push(generator::delivery::SourceGuard {
            path: path.clone(),
            sha256: generator::delivery::digest(&bytes),
        });
        inputs.guard.roots.push(path);
    }
    members.sort_by(|left, right| left.path.cmp(&right.path));
    applications.sort_by(|left, right| left.0.cmp(&right.0));
    inputs.manifest.application_inputs = members;
    inputs.application = applications;
    inputs
        .refresh_snapshot_identity()
        .map_err(PlanDiagnostic::source_closure)?;
    prepare_ecu_sources(plan, target, false, inputs)
}

pub fn prepare_ecu_project<'a>(
    plan: &'a ValidatedIntegrationPlan,
    target: BuildTarget,
    handoff: bool,
) -> Result<PreparedProject<'a>, Vec<PlanDiagnostic>> {
    let native = NativeInputs::from_plan(plan).map_err(PlanDiagnostic::source_closure)?;
    prepare_ecu_sources(plan, target, handoff, native)
}

/// Workspace-aware native source preparation is the application/member entrypoint.
/// A plan-only native call remains valid for the product reference application.
pub fn prepare_ecu_project_for_workspace(
    workspace: &Workspace,
    plan: &ValidatedIntegrationPlan,
    target: BuildTarget,
    handoff: bool,
) -> Result<PreparedProject<'static>, Vec<PlanDiagnostic>> {
    let native = NativeInputs::from_workspace(
        workspace,
        plan.description().required_extension_definitions.clone(),
    )
    .map_err(PlanDiagnostic::source_closure)?;
    if native.configuration.len() != plan.sources().len()
        || !plan.sources().iter().all(|source| {
            native.configuration.iter().any(|(path, bytes)| {
                path == source.logical_path() && bytes.as_slice() == source.bytes()
            })
        })
        || plan.description().rule_set_identity.as_ref()
            != Some(&native.resources.rule_set_identity)
    {
        return Err(PlanDiagnostic::source_closure(crate::product_message!(
            "backend.prepared.plan_source_members_stale"
        )));
    }
    if plan
        .description()
        .validation_dependencies
        .get("definitions.sha256")
        != Some(&native.definition_fingerprint)
    {
        let scoped = crate::integration::build_plan_native(
            &native.sources().map_err(PlanDiagnostic::source_closure)?,
            &native.guard.catalog,
        )?;
        prepare_ecu_sources(&scoped, target, handoff, native)
    } else {
        prepare_ecu_sources(plan, target, handoff, native)
    }
}

fn prepare_ecu_sources(
    plan: &ValidatedIntegrationPlan,
    target: BuildTarget,
    handoff: bool,
    native: NativeInputs,
) -> Result<PreparedProject<'static>, Vec<PlanDiagnostic>> {
    let live_application = native
        .application
        .first()
        .map(|(_, bytes)| bytes.as_slice());
    let rendered = plan.render_ecu_sources(target, live_application)?;
    let source_owners =
        crate::integration::ecu::source_assets(target).map_err(PlanDiagnostic::source_closure)?;
    let mut files = BTreeMap::new();
    for (path, bytes) in rendered {
        if path.starts_with("inputs/") {
            continue;
        }
        let source = source_owners
            .get(&path)
            .copied()
            .filter(|asset| asset.bytes == bytes.as_ref());
        let bytes = match source {
            Some(asset) => Cow::Borrowed(asset.bytes),
            None => Cow::Owned(bytes.into_owned()),
        };
        insert(&mut files, path, bytes, source).map_err(PlanDiagnostic::source_closure)?;
    }
    let slots = plan.application_slot_descriptors()?;
    let native = NativePreparation::new(&mut files, native, slots, target, handoff)
        .map_err(PlanDiagnostic::source_closure)?;
    finish(target, handoff, files, native).map_err(PlanDiagnostic::source_closure)
}

#[cfg(test)]
mod localization_tests {
    use crate::LocalizedText;
    use crate::integration::PlanDiagnostic;

    #[test]
    fn source_closure_preserves_product_message_without_rendering() {
        let message = crate::product_message!(
            "backend.delivery.source_changed",
            "path" => "D:/用户/{{name}}.arxml",
        );
        let diagnostics = PlanDiagnostic::source_closure(message.clone());
        assert_eq!(diagnostics[0].code, "SOURCE_CLOSURE");
        assert_eq!(diagnostics[0].message, message);
        assert_eq!(
            serde_json::to_value(&diagnostics[0].remedy).unwrap(),
            serde_json::json!({"key": "backend.prepared.source_closure_remedy", "params": {}})
        );
    }

    #[test]
    fn source_closure_preserves_raw_system_error_evidence() {
        let evidence = "外部 {{path}}: permission denied\r\n";
        let diagnostics = PlanDiagnostic::source_closure(LocalizedText::from(evidence));
        assert_eq!(
            serde_json::to_value(&diagnostics[0].message).unwrap(),
            evidence
        );
    }
}

#[cfg(test)]
mod delivery_path_tests {
    // An unmapped runtime family would fall through to tools/ without an error.
    #[test]
    fn bsw_sources_deliver_to_the_flat_project_directories() {
        let bsw: Vec<_> = crate::resources::AssetInventory::embedded()
            .entries()
            .iter()
            .filter_map(|asset| Some((asset, asset.relative_path.strip_prefix("runtime/bsw/")?)))
            .collect();
        assert_eq!(bsw.len(), 69);
        for (asset, relative) in bsw {
            assert!(relative.starts_with("src/") || relative.starts_with("include/"));
            assert_eq!(super::deliver_path(asset).unwrap(), relative);
        }
    }
}
