use super::{
    DiagnosticCategory, InputSource, PlanDependencies, PlanDiagnostic, RuntimeCatalog,
    ValidatedIntegrationPlan, build_plan,
};
use crate::execution::{ProcessOwner, ProcessSpec};
use crate::generator;
use crate::target::{BuildTarget, ExecutionSettings};
use crate::{BuildReport, RunReport};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const FORMAT: &str = "autosar-ecu-handoff-v1";

fn issue(message: crate::message::LocalizedText) -> Vec<PlanDiagnostic> {
    vec![PlanDiagnostic {
        category: DiagnosticCategory::Input,
        code: "ECU_HANDOFF".into(),
        file: None,
        object: None,
        message: message.into(),
        remedy: crate::product_message!(
            "backend.integration.handoff.preserve_package_and_restore_original_identities"
        )
        .into(),
    }]
}

pub(crate) fn metadata(plan: &ValidatedIntegrationPlan, target: BuildTarget) -> Value {
    let d = plan.description();
    json!({
        "format": FORMAT, "release": "CP/FO R24-11", "toolVersion": env!("CARGO_PKG_VERSION"),
        "profile": d.profile, "target": target,
        "sources": d.sources, "runtimeSources": d.runtime_sources,
        "validationDependencies": d.validation_dependencies,
        "delivery": {
            "kernel": "kernel/source-manifest.json", "kernelLicense": "kernel/LICENSE.md",
            "compiler": "toolchain.json", "productRights": "Apache-2.0", "productLicense": "LICENSE", "productNotice": "NOTICE",
            "externalInputs": "Legally obtain the pinned R24-11 XSD and MOD archives; not redistributed."
        }
    })
}

pub(super) fn verification_files(
    plan: &ValidatedIntegrationPlan,
) -> Result<BTreeMap<String, Vec<u8>>, crate::message::LocalizedText> {
    let d = plan.description();
    let read =
        d.component
            .data_ports
            .iter()
            .find(|port| port.read)
            .ok_or(crate::product_message!(
                "backend.integration.handoff.selected_receive_port_missing"
            ))?;
    let write = d
        .component
        .data_ports
        .iter()
        .find(|port| !port.read)
        .ok_or(crate::product_message!(
            "backend.integration.handoff.selected_transmit_port_missing"
        ))?;
    let rx = d
        .signals
        .iter()
        .find(|signal| signal.port == read.path)
        .ok_or(crate::product_message!(
            "backend.integration.handoff.receive_channel_missing"
        ))?;
    let tx = d
        .signals
        .iter()
        .find(|signal| signal.port == write.path)
        .ok_or(crate::product_message!(
            "backend.integration.handoff.transmit_channel_missing"
        ))?;
    let inputs = json!({
        "format": "autosar-ecu-test-inputs-v1", "periodMs": d.component.period_ms,
        "receiveCanId": rx.can_id, "transmitCanId": tx.can_id,
        "requestCanId": d.diagnostic.request_can_id, "responseCanId": d.diagnostic.response_can_id,
        "did": d.diagnostic.did, "initialReceiveValue": read.initial_value,
        "receiveTimeoutMs": d.diagnostic.n_cr_ms,
        "scope": "Actual configured endpoints; independent fixed echo bytes and protocol assertions are in tools/ecu_tools/verify.py. This is test input, not a verification result."
    });
    Ok(BTreeMap::from([(
        "verification/inputs.json".into(),
        serde_json::to_vec_pretty(&inputs).map_err(|error| error.to_string())?,
    )]))
}

pub struct EcuHandoff {
    root: PathBuf,
    plan: ValidatedIntegrationPlan,
    target: BuildTarget,
}

impl EcuHandoff {
    pub fn plan(&self) -> &ValidatedIntegrationPlan {
        &self.plan
    }
    pub fn target(&self) -> BuildTarget {
        self.target
    }
    pub fn input_root(&self) -> PathBuf {
        self.root.join("inputs")
    }
    pub fn input_paths(&self) -> Vec<PathBuf> {
        self.plan
            .sources()
            .iter()
            .map(|source| self.input_root().join(source.logical_path()))
            .collect()
    }
}

pub fn open_ecu_handoff(
    output: &Path,
    dependencies: &PlanDependencies,
    runtime: &RuntimeCatalog,
) -> Result<EcuHandoff, Vec<PlanDiagnostic>> {
    let names = generator::output::verify_build_input(output).map_err(issue)?;
    let data: Value = serde_json::from_slice(
        &fs::read(output.join("handoff.json")).map_err(|error| issue(error.to_string().into()))?,
    )
    .map_err(|error| issue(error.to_string().into()))?;
    if data["format"] != FORMAT
        || data["release"] != "CP/FO R24-11"
        || data["toolVersion"] != env!("CARGO_PKG_VERSION")
    {
        return Err(issue(crate::product_message!(
            "backend.integration.handoff.ecu_handoff_version_mismatch"
        )));
    }
    let target: BuildTarget = serde_json::from_value(data["target"].clone())
        .map_err(|error| issue(crate::product_message!("backend.integration.handoff.handoff_target_unsupported", "error" => error)))?;
    let declarations = data["sources"]
        .as_array()
        .filter(|sources| !sources.is_empty())
        .ok_or_else(|| {
            issue(crate::product_message!(
                "backend.integration.handoff.ecu_handoff_input_identities_missing"
            ))
        })?;
    let mut sources = Vec::new();
    let mut paths = BTreeSet::new();
    for declaration in declarations {
        let logical = declaration["logicalPath"].as_str().ok_or_else(|| {
            issue(crate::product_message!(
                "backend.integration.handoff.logical_input_identity_invalid"
            ))
        })?;
        let input = InputSource::new(logical, Vec::new()).map_err(|diagnostic| vec![diagnostic])?;
        let path = format!("inputs/{}", input.logical_path());
        if !names.contains(&path) || !paths.insert(path.clone()) {
            return Err(issue(
                crate::product_message!("backend.integration.handoff.input_missing_or_repeated", "path" => path),
            ));
        }
        let bytes =
            fs::read(output.join(&path)).map_err(|error| issue(error.to_string().into()))?;
        if declaration["rawSha256"] != format!("{:x}", Sha256::digest(&bytes)) {
            return Err(issue(
                crate::product_message!("backend.integration.handoff.input_identity_mismatch", "path" => path),
            ));
        }
        sources.push(InputSource::new(logical, bytes).map_err(|diagnostic| vec![diagnostic])?);
    }
    if names
        .iter()
        .filter(|path| path.starts_with("inputs/"))
        .count()
        != paths.len()
    {
        return Err(issue(crate::product_message!(
            "backend.integration.handoff.input_closure_handoff_mismatch"
        )));
    }
    let plan = build_plan(&sources, dependencies, runtime)?;
    if metadata(&plan, target) != data {
        return Err(issue(crate::product_message!(
            "backend.integration.handoff.revalidated_identities_handoff_mismatch"
        )));
    }
    let expected = plan.ecu_handoff_files(target)?;
    compare_files(output, &names, expected.files()).map_err(issue)?;
    Ok(EcuHandoff {
        root: output
            .canonicalize()
            .map_err(|error| issue(error.to_string().into()))?,
        plan,
        target,
    })
}

fn compare_files(
    project: &Path,
    names: &[String],
    expected: &[(String, Vec<u8>)],
) -> Result<(), crate::message::LocalizedText> {
    let expected_names: BTreeSet<_> = expected
        .iter()
        .map(|(path, _)| path.as_str())
        .filter(|path| *path != "files.list" && *path != "files.sha256")
        .collect();
    if names.iter().map(String::as_str).collect::<BTreeSet<_>>() != expected_names {
        return Err(crate::product_message!(
            "backend.integration.handoff.rebuilt_ecu_product_closure_mismatch"
        )
        .into());
    }
    for (path, bytes) in expected {
        if fs::read(project.join(path)).map_err(|error| error.to_string())? != *bytes {
            return Err(
                crate::product_message!("backend.integration.handoff.rebuilt_ecu_product_bytes_mismatch", "path" => path),
            );
        }
    }
    Ok(())
}

fn checked_project(
    plan: &ValidatedIntegrationPlan,
    project: &Path,
) -> Result<BuildTarget, crate::message::LocalizedText> {
    let names = generator::output::verify_build_input(project)?;
    let data: Value = serde_json::from_slice(
        &fs::read(project.join("integration.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if data["format"] != "autosar-ecu-integration-v1" || data["plan"]["profile"] != super::PROFILE {
        return Err(crate::product_message!(
            "backend.integration.handoff.source_directory_integration_profile_mismatch"
        )
        .into());
    }
    let target: BuildTarget =
        serde_json::from_value(data["target"].clone()).map_err(|error| error.to_string())?;
    let expected = if project.join("handoff.json").exists() {
        plan.ecu_handoff_files(target)
    } else {
        plan.ecu_integration_files(target)
    }
    .map_err(|diagnostics| crate::message::LocalizedText::messages(diagnostics.into_iter().map(|diagnostic| {
        crate::message::LocalizedText::messages([
            crate::product_message!("backend.integration.handoff.diagnostic_location", "code" => diagnostic.code, "file" => diagnostic.file.as_deref().unwrap_or(""), "object" => diagnostic.object.as_deref().unwrap_or("")),
            diagnostic.message,
            diagnostic.remedy,
        ])
    })))?;
    compare_files(project, &names, expected.files())?;
    Ok(target)
}

pub(crate) fn run_tool(
    project: &Path,
    settings: &ExecutionSettings,
    arguments: Vec<OsString>,
    private: &Path,
    owner: &ProcessOwner,
) -> Result<String, crate::message::LocalizedText> {
    let logs = private.join("logs");
    fs::create_dir(&logs).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&logs, fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    let mut argv = vec![
        settings.python.as_os_str().into(),
        "-I".into(),
        "-S".into(),
        project.join("tools/ecu-tool.py").into_os_string(),
    ];
    argv.extend(arguments);
    let spec = ProcessSpec::for_duration(
        argv,
        project.to_path_buf(),
        vec![
            ("AUTOSAR_CC".into(), settings.compiler.as_os_str().into()),
            (
                "AUTOSAR_OBJDUMP".into(),
                settings.objdump.as_os_str().into(),
            ),
            ("AUTOSAR_GIT".into(), settings.git.as_os_str().into()),
        ],
        Duration::from_secs(300),
        logs,
    )?;
    let result = owner.run(spec)?;
    Ok(format!(
        "{}{}",
        fs::read_to_string(result.stdout).map_err(|error| error.to_string())?,
        fs::read_to_string(result.stderr).map_err(|error| error.to_string())?
    ))
}

pub fn build_ecu_project(
    plan: &ValidatedIntegrationPlan,
    project: &Path,
    output: &Path,
    settings: &ExecutionSettings,
    owner: &ProcessOwner,
) -> Result<BuildReport, crate::message::LocalizedText> {
    let target = checked_project(plan, project)?;
    let capture = generator::output::reserve_directory(
        &std::env::temp_dir(),
        "ecu-build",
        OsStr::new("private"),
    )?;
    let log = run_tool(
        project,
        settings,
        vec![
            "build".into(),
            "--project".into(),
            project.as_os_str().into(),
            "--output".into(),
            output.as_os_str().into(),
            "--mode".into(),
            "host-batch".into(),
        ],
        &capture,
        owner,
    )
    .map_err(|error| {
        crate::message::LocalizedText::messages([error, crate::product_message!("backend.integration.handoff.source_build_diagnostics_retained", "path" => capture.display())])
    })?;
    checked_project(plan, project)?;
    let binary = output.join(target.spec().binary_name);
    if !binary.is_file() {
        return Err(crate::product_message!(
            "backend.integration.handoff.declared_native_binary_missing"
        )
        .into());
    }
    fs::remove_dir_all(capture).map_err(|error| error.to_string())?;
    Ok(BuildReport {
        binary_path: binary.display().to_string(),
        log,
    })
}

pub fn verify_ecu_project(
    plan: &ValidatedIntegrationPlan,
    project: &Path,
    settings: &ExecutionSettings,
    owner: &ProcessOwner,
) -> Result<RunReport, crate::message::LocalizedText> {
    checked_project(plan, project)?;
    let scratch = generator::output::reserve_directory(
        &std::env::temp_dir(),
        "ecu-verify",
        OsStr::new("private"),
    )?;
    let output = scratch.join("build");
    let log = run_tool(
        project,
        settings,
        vec![
            "verify".into(),
            "--project".into(),
            project.as_os_str().into(),
            "--build-directory".into(),
            output.into_os_string(),
        ],
        &scratch,
        owner,
    )
    .map_err(|error| {
        crate::message::LocalizedText::messages([error, crate::product_message!("backend.integration.handoff.verification_diagnostics_retained", "path" => scratch.display())])
    })?;
    checked_project(plan, project)?;
    if !log
        .lines()
        .any(|line| line.starts_with("ECU_HANDOFF_VERIFY PASS:"))
    {
        return Err(
            crate::product_message!("backend.integration.handoff.independent_verifier_production_oracle_incomplete", "log" => log),
        );
    }
    fs::remove_dir_all(scratch).map_err(|error| error.to_string())?;
    Ok(RunReport {
        passed: true,
        log: log.into(),
        events: vec![
            crate::product_message!("backend.integration.handoff.host_behavior_checks_passed")
                .into(),
        ],
    })
}
