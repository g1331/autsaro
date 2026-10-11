//! Product-generated analysis inputs; no official archives or native compiler required.
use autosar_config_core::arxml_render::render_profile;
use autosar_config_core::integration::RuntimeCatalog;
use autosar_config_core::prepared::prepare_ecu_project_for_workspace;
use autosar_config_core::target::BuildTarget;
use autosar_config_core::{
    DiagnosticView, Direction, DtcView, FrameView, SignalView, Workspace, prepare_host_project,
};
use std::fs;
use std::path::Path;

#[path = "support/delivery.rs"]
#[allow(dead_code)]
mod delivery_support;
#[cfg(feature = "native-tests")]
#[path = "support/tooling.rs"]
#[allow(dead_code)]
mod tooling;
#[path = "support/workspace.rs"]
#[allow(dead_code)]
mod workspace;
#[cfg(feature = "native-tests")]
use workspace::Scratch;

fn host(root: &Path, case: &str, capacity: bool, diagnostic: bool) -> Workspace {
    let preview = Workspace::preview_project_creation(root, case, "can-signals-v1").unwrap();
    Workspace::create_project_previewed(&preview).unwrap();
    let mut frames = Vec::new();
    let mut signals = Vec::new();
    for index in 0..if capacity { 32 } else { 1 } {
        let path = format!("/{case}/Pdu_Frame{index:02}");
        frames.push(FrameView {
            path: path.clone(),
            name: format!("Frame{index:02}"),
            id: 0x100 + index,
            dlc: if capacity { 8 } else { 4 },
            direction: Direction::Tx,
            period_ms: Some(10),
            timeout_ms: None,
        });
        let layouts = if capacity {
            vec![(0, 1), (32, 32)]
        } else {
            vec![(0, 32)]
        };
        for (slot, (start_bit, length)) in layouts.into_iter().enumerate() {
            signals.push(SignalView {
                path: format!("/{case}/ComCfg/ComConfig/Value{index:02}_{slot}"),
                name: format!("Value{index:02}_{slot}"),
                frame_path: path.clone(),
                start_bit,
                length,
                initial_value: if length == 32 { u32::MAX } else { 1 },
            });
        }
    }
    if diagnostic {
        let path = format!("/{case}/Pdu_Monitor");
        frames.push(FrameView {
            path: path.clone(),
            name: "Monitor".into(),
            id: 0x456,
            dlc: 1,
            direction: Direction::Rx,
            period_ms: None,
            timeout_ms: Some(50),
        });
        signals.push(SignalView {
            path: format!("/{case}/ComCfg/ComConfig/Monitor"),
            name: "Monitor".into(),
            frame_path: path,
            start_bit: 0,
            length: 8,
            initial_value: 0,
        });
    }
    let diagnostics = diagnostic.then(|| DiagnosticView {
        path: format!("/{case}/DcmCfg/DcmConfigSet/DcmDsp/Did"),
        request_id: 0x700,
        response_id: 0x708,
        s3_ms: 5000,
        n_as_ms: 1000,
        n_bs_ms: 1000,
        n_cr_ms: 1000,
        did: 0xf190,
        signal_paths: vec![signals[0].path.clone()],
        write_enabled: true,
        reset_routine_id: None,
        security_enabled: false,
        dtc: Some(DtcView {
            path: format!("/{case}/DemCfg/DemConfigSet/Event"),
            code: 0x123456,
            monitor_frame_path: format!("/{case}/Pdu_Monitor"),
        }),
    });
    let xml = render_profile(case, &frames, &signals, diagnostics.as_ref());
    fs::write(root.join(format!("{case}.arxml")), xml).unwrap();
    Workspace::open_project_manifest(&root.join("workbench-project.json"), &root.join("cache"))
        .unwrap()
}

#[test]
fn generated_c_analysis_samples() {
    let scratch = workspace::Scratch::new();
    let output = std::env::var_os("AUTOSAR_C_ANALYSIS_SAMPLES")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| scratch.0.join("samples"));
    assert!(
        !output.exists(),
        "Sample output must be new; refusing to overwrite {}",
        output.display()
    );
    fs::create_dir_all(&output).unwrap();
    let target = if cfg!(windows) {
        BuildTarget::WindowsX64ControlledV1
    } else {
        BuildTarget::LinuxX64ControlledV1
    };
    for (case, capacity, diagnostic) in [
        ("signals", false, false),
        ("diagnostic", false, true),
        ("capacity", true, false),
    ] {
        let mut workspace = host(&scratch.0.join(case), case, capacity, diagnostic);
        let prepared = prepare_host_project(&mut workspace, target, true)
            .unwrap_or_else(|error| panic!("{case}: {error:?}; view={:?}", workspace.view()));
        prepared.generate(&output.join(case)).unwrap();
        // The shared real Det implementation needs its private adapter in the
        // delivered src directory; repository include paths must not rescue it.
        assert!(output.join(case).join("src/Det_Host.h").is_file());
    }
    for user in [false, true] {
        let case = if user {
            "user-application"
        } else {
            "standard-ecu"
        };
        let live = scratch.0.join(case);
        let preview =
            Workspace::preview_project_creation(&live, "Application", "standard-ecu-v1").unwrap();
        let mut workspace = Workspace::create_project_previewed(&preview).unwrap();
        let initialization = workspace.preview_application_initialization().unwrap();
        workspace
            .initialize_application_previewed(&initialization)
            .unwrap();
        if user {
            let application = workspace.generation_snapshot().unwrap().applications[0].clone();
            let original = String::from_utf8(application.bytes).unwrap();
            let changed = original.replace(
                "Data[i] = (uint8)(application.value >> ((3u - i) * 8u));",
                "Data[i] = (uint8)((application.value + UINT32_C(1)) >> ((3u - i) * 8u));",
            );
            assert_ne!(original, changed);
            fs::write(application.disk_path, changed).unwrap();
            workspace = Workspace::open_project_manifest(
                &live.join("workbench-project.json"),
                &scratch.0.join("cache"),
            )
            .unwrap();
        }
        let plan = workspace
            .saved_integration_plan(&RuntimeCatalog::embedded().unwrap())
            .unwrap();
        prepare_ecu_project_for_workspace(&workspace, &plan, target, true)
            .unwrap()
            .generate(&output.join(case))
            .unwrap();
    }
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi-component");
    let mut sources: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            autosar_config_core::integration::InputSource::new(
                entry.file_name().to_str().unwrap(),
                fs::read(entry.path()).unwrap(),
            )
            .unwrap()
        })
        .collect();
    sources.sort_by(|left, right| left.logical_path().cmp(right.logical_path()));
    let plan = autosar_config_core::integration::build_plan_native(
        &sources,
        &autosar_config_core::definitions::DefinitionCatalog::builtin().unwrap(),
        &RuntimeCatalog::embedded().unwrap(),
    )
    .unwrap();
    let applications: Vec<_> = plan
        .application_slot_descriptors()
        .unwrap()
        .iter()
        .map(|slot| autosar_config_core::ApplicationSource {
            component_instance: slot
                .producer_slot
                .strip_prefix("singlecore-multi-swc-v1:")
                .unwrap()
                .into(),
            path: Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/multi-application")
                .join(slot.source_paths[0].rsplit('/').next().unwrap()),
        })
        .collect();
    let live = scratch.0.join("multi-author");
    fs::create_dir(&live).unwrap();
    for source in &sources {
        fs::write(live.join(source.logical_path()), source.bytes()).unwrap();
    }
    let manifest = autosar_config_core::arxml::ProjectManifest {
        format_version: 1,
        declared_release: "R24-11".into(),
        profile_hint: "singlecore-multi-swc-v1".into(),
        inputs: sources
            .iter()
            .map(|source| autosar_config_core::arxml::ProjectInput {
                path: source.logical_path().into(),
                role_hint: "standard".into(),
            })
            .collect(),
        application_inputs: Vec::new(),
        accepted_extension_definitions: Vec::new(),
    };
    let manifest_path = live.join("workbench-project.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let mut workspace =
        Workspace::open_project_manifest(&manifest_path, &scratch.0.join("cache")).unwrap();
    let preview = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&preview)
        .unwrap();
    for (slot, application) in plan
        .application_slot_descriptors()
        .unwrap()
        .iter()
        .zip(&applications)
    {
        fs::copy(&application.path, live.join(&slot.source_paths[0])).unwrap();
    }
    let workspace =
        Workspace::open_project_manifest(&manifest_path, &scratch.0.join("cache")).unwrap();
    let plan = workspace
        .saved_integration_plan(&RuntimeCatalog::embedded().unwrap())
        .unwrap();
    let original = scratch.0.join("multi-author-package");
    prepare_ecu_project_for_workspace(&workspace, &plan, target, true)
        .unwrap()
        .generate(&original)
        .unwrap();
    let package = scratch.0.join("moved-multi-package");
    fs::rename(&original, &package).unwrap();
    fs::remove_dir_all(&live).unwrap();
    assert!(!original.exists() && !live.exists());
    let imported = autosar_config_core::generator::delivery::open_handoff(
        &package,
        &scratch.0.join("received-multi"),
        &autosar_config_core::definitions::DefinitionCatalog::builtin().unwrap(),
    )
    .unwrap();
    let plan = imported
        .workspace
        .saved_integration_plan(&RuntimeCatalog::embedded().unwrap())
        .unwrap();
    let regenerated = output.join("multi-component");
    prepare_ecu_project_for_workspace(&imported.workspace, &plan, target, true)
        .unwrap()
        .generate(&regenerated)
        .unwrap();
    assert_eq!(
        delivery_support::payload(&package),
        delivery_support::payload(&regenerated)
    );
    assert_eq!(fs::read_dir(&output).unwrap().count(), 6);
    #[cfg(feature = "native-tests")]
    for case in ["signals", "standard-ecu", "multi-component"] {
        generated_can_object_sections_and_clear(&output.join(case));
    }
}

#[test]
fn historical_host_diagnostic_generation_accepts_formatting_but_rejects_new_data() {
    use autosar_config_core::definitions::DefinitionCatalog;
    use autosar_config_core::project_model::ValidationStatus;
    let scratch = workspace::Scratch::new();
    let root = scratch.0.join("formatted");
    let workspace = host(&root, "formatted", false, true);
    let source = root.join("formatted.arxml");
    let original = fs::read_to_string(&source).unwrap();
    drop(workspace);
    let formatted = original
        .replace("><", ">\r\n  <")
        .replace("<AR-PACKAGES>", "<AR-PACKAGES><!-- formatting only -->");
    fs::write(&source, &formatted).unwrap();
    let mut workspace = Workspace::open_project_manifest(
        &root.join("workbench-project.json"),
        &scratch.0.join("cache"),
    )
    .unwrap();
    let target = if cfg!(windows) {
        BuildTarget::WindowsX64ControlledV1
    } else {
        BuildTarget::LinuxX64ControlledV1
    };
    prepare_host_project(&mut workspace, target, true)
        .unwrap()
        .generate(&scratch.0.join("delivery"))
        .unwrap();
    let catalog = DefinitionCatalog::builtin().unwrap();
    let validation = catalog
        .validate_documents(&[(source.as_path(), formatted.as_str())])
        .unwrap();
    assert!(validation.diagnostics.is_empty());
    assert_eq!(validation.status, ValidationStatus::Unsupported);
    assert!(validation.coverage.iter().any(|rule| rule.rule_id
        == "native.definition.legacy-dcm-mode-dependency"
        && !rule.supported));
    let changed = formatted.replacen("<ELEMENTS>", "<ELEMENTS><APPLICATION-SW-COMPONENT-TYPE><SHORT-NAME>NewComponent</SHORT-NAME></APPLICATION-SW-COMPONENT-TYPE>", 1);
    let validation = catalog
        .validate_documents(&[(source.as_path(), changed.as_str())])
        .unwrap();
    assert!(
        validation
            .diagnostics
            .iter()
            .any(|issue| issue.code == "MULTIPLICITY"
                && issue.witness.as_ref().is_some_and(|witness| format!(
                    "{:?}",
                    witness.constraint
                )
                .contains("DcmDslProtocolComMChannelRef")))
    );
}

#[cfg(feature = "native-tests")]
fn generated_can_object_sections_and_clear(project: &Path) {
    use std::process::Command;
    let target: serde_json::Value =
        serde_json::from_slice(&fs::read(project.join("target.json")).unwrap()).unwrap();
    let scratch = workspace::Scratch::new();
    let settings = tooling::execution_settings();
    let compile = |source: &Path, output: &Path, object: bool| {
        let mut command = Command::new(&settings.compiler);
        command.current_dir(project);
        for flag in target["compilerFlags"].as_array().unwrap() {
            command.arg(flag.as_str().unwrap());
        }
        for include in target["includePaths"].as_array().unwrap() {
            command.arg("-I").arg(include.as_str().unwrap());
        }
        if object {
            command.arg("-c");
        }
        let result = command.arg(source).arg("-o").arg(output).output().unwrap();
        assert!(
            result.status.success(),
            "{}: {}",
            project.display(),
            String::from_utf8_lossy(&result.stderr)
        );
    };
    let object = scratch.0.join("Can.o");
    // Compile the actual generated source with the delivered profile flags and headers.
    compile(&project.join("src/Can.c"), &object, true);
    let result = Command::new(&settings.objdump)
        .arg("-t")
        .arg(&object)
        .output()
        .unwrap();
    assert!(result.status.success());
    let symbols = String::from_utf8(result.stdout).unwrap();
    for (symbol, section) in [
        ("Can_Write", ".text"),
        ("Can_Init", ".text"),
        ("Can_MainFunction_Read", ".text"),
        ("can_controller_mode", ".bss"),
        ("can_initialized", ".bss"),
        ("tx_payload", ".bss"),
        ("rx_payload", ".bss"),
        ("driver_lifetime", ".bss"),
    ] {
        assert!(
            symbols
                .lines()
                .any(|line| line.contains(section) && line.ends_with(&format!(" {symbol}"))),
            "{symbol}/{section}: {symbols}"
        );
    }
    let source = scratch.0.join("can-clear.c");
    fs::write(&source, format!(r#"#include "{}"
#include <assert.h>
const EcuPolicyConfig Ecu_Policy = {{0}};
void Can_Lock(void) {{}}
void Can_Unlock(void) {{}}
int Can_TryLock(void) {{ return 1; }}
void SchM_Enter_CanIf_CANIF_STATE(void) {{}}
void SchM_Exit_CanIf_CANIF_STATE(void) {{}}
void CanIf_TxConfirmation(PduIdType id) {{ (void)id; }}
void CanIf_ControllerModeIndication(uint8_t id, Can_ControllerStateType mode) {{ (void)id; (void)mode; }}
void CanIf_ControllerBusOff(uint8_t id) {{ (void)id; }}
EcuStatus CanIf_HostRxIndication(uint32_t id, uint8_t length, const uint8_t data[8], uint64_t time) {{
    (void)id; (void)length; (void)data; (void)time; return ECU_OK;
}}
Std_ReturnType Det_ReportRuntimeError(uint16_t module, uint8_t instance, uint8_t api, uint8_t error) {{
    (void)module; (void)instance; (void)api; (void)error; return E_OK;
}}
EcuStatus Ecu_ExecutionTransmit(CanTxSink sink, PduIdType handle, uint32_t id, uint8_t length, const uint8_t data[8]) {{
    (void)sink; (void)handle; (void)id; (void)length; (void)data; return ECU_OK;
}}
int main(void) {{
    size_t i;
    assert(can_controller_mode == 0 && bus_off == 0u && can_initialized == 0u && tx_sink == NULL);
    assert(driver_lifetime == 0u && tx_pending == 0u && tx_in_flight == 0u && tx_cancelled == 0u);
    assert(tx_id == 0u && tx_length == 0u && tx_handle == 0u);
    assert(tx_confirmation_pending == 0u && tx_confirmation_handle == 0u && tx_confirming == 0u);
    assert(interrupt_disable_count == 0u && mode_notification_pending == 0u && mode_notification_processing == 0u);
    assert(pending_controller_mode == 0 && busoff_notification_pending == 0u);
    assert(rx_pending == 0u && rx_processing == 0u && rx_id == 0u && rx_length == 0u && rx_time_ms == 0u);
    assert(rx_overrun_pending == 0u);
#if CAN_HOST_QUEUED_COMPLETION == 1
    assert(host_tx_sequence == 0u && host_tx_cancelled_through == 0u && host_tx_completed_through == 0u);
#endif
    for (i = 0u; i < 8u; ++i) {{ assert(tx_payload[i] == 0u && rx_payload[i] == 0u); }}
    return 0;
}}
"#, project.join("src/Can.c").display().to_string().replace('\\', "/"))).unwrap();
    let executable = tooling::native_binary(&scratch.0, "can-clear");
    // Stubs only close linking: this oracle examines real generated static storage before Init.
    compile(&source, &executable, false);
    let result = Command::new(executable).output().unwrap();
    assert!(
        result.status.success(),
        "{}: {:?}",
        project.display(),
        result
    );
}
