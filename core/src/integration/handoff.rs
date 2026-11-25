use super::ValidatedIntegrationPlan;
use crate::execution::{ProcessOwner, ProcessSpec};
use crate::generator;
use crate::target::{BuildTarget, ExecutionSettings};
use crate::{BuildReport, RunReport};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::Path;
use std::time::Duration;

/// The single reference echo application shape checked by the independent
/// verifier: one application component with one periodic runnable, exactly one
/// 32-bit COM Rx/Tx pair on its ports, and an application DID.
pub(super) struct ReferenceShape<'a> {
    pub period_ms: u32,
    pub periodic_symbol: &'a str,
    pub rx: &'a super::SignalChannel,
    pub tx: &'a super::SignalChannel,
    pub diagnostic: &'a super::DiagnosticContract,
    pub read: &'a super::component::DataPort,
    pub write: &'a super::component::DataPort,
}

pub(super) fn reference_shape(plan: &ValidatedIntegrationPlan) -> Option<ReferenceShape<'_>> {
    let d = plan.description();
    let slots = plan.application_slot_descriptors().ok()?;
    let [slot] = slots.as_slice() else {
        return None;
    };
    let component = d
        .composition
        .as_ref()?
        .components
        .iter()
        .find(|component| component.component == slot.component_path)?;
    let (receive, transmit): (Vec<_>, Vec<_>) = d.signals.iter().partition(|signal| signal.receive);
    let (Some(diagnostic), [rx], [tx]) = (&d.diagnostic, receive.as_slice(), transmit.as_slice())
    else {
        return None;
    };
    let periodic: Vec<_> = component
        .runnables
        .iter()
        .filter_map(|runnable| Some((runnable.period_ms?, runnable.symbol.as_str())))
        .collect();
    let [(period_ms, periodic_symbol)] = periodic.as_slice() else {
        return None;
    };
    let read = component
        .data_ports
        .iter()
        .find(|port| port.read && port.path == rx.port)?;
    let write = component
        .data_ports
        .iter()
        .find(|port| !port.read && port.path == tx.port)?;
    if [rx, tx]
        .iter()
        .any(|signal| signal.dlc != 4 || signal.bit_length != 32)
    {
        return None;
    }
    Some(ReferenceShape {
        period_ms: *period_ms,
        periodic_symbol,
        rx,
        tx,
        diagnostic,
        read,
        write,
    })
}

/// Endpoints for the independent reference-behaviour verifier, emitted only
/// for the reference shape.
pub(super) fn verification_files(
    plan: &ValidatedIntegrationPlan,
) -> Result<BTreeMap<String, Vec<u8>>, crate::message::LocalizedText> {
    let Some(shape) = reference_shape(plan) else {
        return Ok(BTreeMap::new());
    };
    let description = plan.description();
    let (Some(communication), Some(transport)) = (
        &description.communication_runtime,
        &description.diagnostic_transport,
    ) else {
        return Ok(BTreeMap::new());
    };
    let endpoint = |direction, handle, id, accepts_fd, length| {
        json!({
            "module": "CanIf", "direction": direction, "handle": handle, "channel": 0,
            "format": if id & super::can_id::EXTENDED != 0 { "EXTENDED" } else { "STANDARD" },
            "fdPolicy": if accepts_fd { "CLASSICAL_OR_FD" } else { "CLASSICAL" },
            "id": id & super::can_id::EXTENDED_MAX, "logicalLength": length, "wireLength": length,
        })
    };
    let inputs = json!({
        "format": "autosar-ecu-test-inputs-v2", "periodMs": shape.period_ms,
        "channels": {"0": communication.can.physical_channel},
        "endpoints": {
            "receive": endpoint("RX", shape.rx.can_if_handle, shape.rx.encoded_id(), shape.rx.accepts_fd, 4),
            "transmit": endpoint("TX", shape.tx.can_if_handle, shape.tx.encoded_id(), false, 4),
            "request": endpoint("RX", transport.request_can_if_handle, transport.encoded_request_id(), transport.request_accepts_fd, 8),
            "response": endpoint("TX", transport.response_can_if_handle, transport.encoded_response_id(), false, 8),
        },
        "did": shape.diagnostic.did, "initialReceiveValue": shape.read.initial_value,
        "receiveTimeoutMs": shape.diagnostic.n_cr_ms,
        "scope": "Actual configured endpoints; independent fixed echo bytes and protocol assertions are in tools/ecu_tools/verify.py. This is test input, not a verification result."
    });
    Ok(BTreeMap::from([(
        super::outputs::VERIFICATION_INPUTS_PATH.into(),
        serde_json::to_vec_pretty(&inputs).map_err(|error| error.to_string())?,
    )]))
}

fn compare_files(
    project: &Path,
    names: &[String],
    expected: &[(String, Vec<u8>)],
) -> Result<(), crate::message::LocalizedText> {
    let expected_names: BTreeSet<_> = expected
        .iter()
        .map(|(path, _)| path.as_str())
        .filter(|path| {
            ![generator::output::FILE_LIST, generator::output::FILE_HASHES].contains(path)
        })
        .collect();
    if names.iter().map(String::as_str).collect::<BTreeSet<_>>() != expected_names {
        return Err(crate::product_message!(
            "backend.integration.handoff.rebuilt_ecu_product_closure_mismatch"
        ));
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

/// The caller supplies the expected closure from the same preparation entry
/// point that generated the project (plan-only or workspace-aware), so the
/// built bytes are always the bytes the workbench would deliver now.
fn checked_project(
    expected: &[(String, Vec<u8>)],
    project: &Path,
) -> Result<BuildTarget, crate::message::LocalizedText> {
    let names = generator::output::verify_build_input(project)?;
    let integration = |bytes: &[u8]| -> Result<Value, crate::message::LocalizedText> {
        Ok(serde_json::from_slice(bytes).map_err(|error| error.to_string())?)
    };
    let data = integration(
        &fs::read(project.join(super::outputs::INTEGRATION_PATH))
            .map_err(|error| error.to_string())?,
    )?;
    let selected = expected
        .iter()
        .find(|(path, _)| path == super::outputs::INTEGRATION_PATH)
        .map(|(_, bytes)| integration(bytes))
        .transpose()?;
    if data["format"] != "autosar-ecu-integration-v3"
        || selected.is_none_or(|selected| data["plan"]["profile"] != selected["plan"]["profile"])
    {
        return Err(crate::product_message!(
            "backend.integration.handoff.source_directory_integration_profile_mismatch"
        ));
    }
    let target: BuildTarget =
        serde_json::from_value(data["target"].clone()).map_err(|error| error.to_string())?;
    compare_files(project, &names, expected)?;
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
    expected: &[(String, Vec<u8>)],
    project: &Path,
    output: &Path,
    settings: &ExecutionSettings,
    owner: &ProcessOwner,
) -> Result<BuildReport, crate::message::LocalizedText> {
    let target = checked_project(expected, project)?;
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
    checked_project(expected, project)?;
    let binary = output.join(target.spec().binary_name);
    if !binary.is_file() {
        return Err(crate::product_message!(
            "backend.integration.handoff.declared_native_binary_missing"
        ));
    }
    fs::remove_dir_all(capture).map_err(|error| error.to_string())?;
    Ok(BuildReport {
        binary_path: binary.display().to_string(),
        log,
    })
}

pub fn verify_ecu_project(
    expected: &[(String, Vec<u8>)],
    project: &Path,
    settings: &ExecutionSettings,
    owner: &ProcessOwner,
) -> Result<RunReport, crate::message::LocalizedText> {
    checked_project(expected, project)?;
    if !expected
        .iter()
        .any(|(path, _)| path == super::outputs::VERIFICATION_INPUTS_PATH)
    {
        return Err(crate::product_message!(
            "backend.integration.handoff.reference_verification_inputs_missing"
        ));
    }
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
    checked_project(expected, project)?;
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
        events: vec![crate::product_message!(
            "backend.integration.handoff.host_behavior_checks_passed"
        )],
    })
}
