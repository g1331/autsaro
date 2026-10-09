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

#[path = "support/workspace.rs"]
#[allow(dead_code)]
mod workspace;

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
    autosar_config_core::prepare_ecu_project_with_applications(&plan, target, &applications)
        .unwrap()
        .generate(&output.join("multi-component"))
        .unwrap();
    assert_eq!(fs::read_dir(&output).unwrap().count(), 6);
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
