//! Generation of the static-analysis samples (no compiler or official archives).
use super::*;

/// Writes the samples to `AUTOSAR_C_ANALYSIS_SAMPLES` when set, for CI and
/// `c-check`; otherwise to a scratch directory that is removed afterwards.
#[test]
fn generated_c_analysis_samples() {
    let scratch = Scratch::new();
    let output = std::env::var_os("AUTOSAR_C_ANALYSIS_SAMPLES")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| scratch.0.join("samples"));
    assert!(
        !output.exists(),
        "Sample output must be new; refusing to overwrite {}",
        output.display()
    );
    analysis_samples::generate(&output, &scratch);
    for (sample, id, length, mapping_count) in [
        ("standard-ecu", "UINT16", "UINT16", 15),
        ("user-application", "UINT16", "UINT16", 15),
        ("component-pipeline", "UINT8", "UINT32", 17),
        ("no-diagnostic", "UINT8", "UINT8", 16),
    ] {
        let mappings = std::fs::read_dir(output.join(sample).join("generated/include"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with("_MemMap.h")
            })
            .collect::<Vec<_>>();
        assert_eq!(mappings.len(), mapping_count, "{sample}: mapping headers");
        for path in mappings {
            let contents = std::fs::read_to_string(&path).unwrap();
            let file_comment = contents.split_once("*/").unwrap().0;
            assert!(
                file_comment.starts_with("/** @file")
                    && file_comment.contains("MR12 DIR 4.10 VIOLATION:"),
                "{sample}: {} requires its D4.10 file record",
                path.display()
            );
        }
        let metadata: serde_json::Value = serde_json::from_slice(
            &std::fs::read(output.join(sample).join("meta/integration.json")).unwrap(),
        )
        .unwrap();
        let target: serde_json::Value = serde_json::from_slice(
            &std::fs::read(output.join(sample).join("target.json")).unwrap(),
        )
        .unwrap();
        let flags = target["compilerFlags"].as_array().unwrap();
        assert!(
            flags.iter().any(|flag| flag == "-fexceptions"),
            "{sample}: native unwind contract"
        );
        assert!(
            !flags
                .iter()
                .any(|flag| flag.as_str().unwrap().starts_with("-D_GNU_SOURCE")),
            "{sample}: reserved feature selection"
        );
        let runtime_path = output
            .join(sample)
            .join("generated/src/Ecu_RuntimeConfig.c");
        let runtime = std::fs::read_to_string(runtime_path).unwrap();
        let callback = runtime
            .rsplit_once("Ecu_BswMAction(")
            .unwrap()
            .1
            .split_once("\n}")
            .unwrap()
            .0;
        assert!(
            callback.contains(
                "default:\n            result = E_NOT_OK;\n            break;\n    }\n    return result;"
            ),
            "{sample}: rejected mode must break and preserve E_NOT_OK"
        );
        let communication = &metadata["plan"]["communicationRuntime"];
        assert_eq!(communication["pduIdType"], id, "{sample}: PDU ID selection");
        assert_eq!(
            communication["pduLengthType"], length,
            "{sample}: PDU length selection"
        );
    }
}

/// The new mechanism has a sealed, real-application input beside the four regression profiles.
#[test]
fn typed_synchronous_application_analysis_sample() {
    use autosar_config_core::target::BuildTarget;
    let sources = mainline::typed_operation_inputs();
    let plan = mainline::build(&sources).unwrap();
    let scratch = Scratch::new();
    let output = std::env::var_os("AUTOSAR_TYPED_C_ANALYSIS_SAMPLE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| scratch.0.join("typed-services"));
    let target = if cfg!(windows) {
        BuildTarget::WindowsX64ControlledV1
    } else {
        BuildTarget::LinuxX64ControlledV1
    };
    mainline::prepared_typed_pipeline(&plan, target)
        .generate(&output)
        .unwrap();
    assert!(output.join("generated/src/Rte.c").is_file());
}

#[test]
fn mapped_bsw_service_analysis_sample() {
    let plan = mainline::build(&mainline::mapped_operation_inputs()).unwrap();
    let scratch = Scratch::new();
    let output = std::env::var_os("AUTOSAR_BSW_C_ANALYSIS_SAMPLE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| scratch.0.join("mapped-services"));
    assert!(!output.exists());
    let target = if cfg!(windows) {
        autosar_config_core::target::BuildTarget::WindowsX64ControlledV1
    } else {
        autosar_config_core::target::BuildTarget::LinuxX64ControlledV1
    };
    mainline::write_files(
        &output,
        &mainline::prepared_pipeline_files_for_target(&plan, target),
    );
    assert!(output.join("generated/src/Rte.c").is_file());
}

#[test]
fn synchronized_mode_analysis_sample() {
    let plan = mainline::build(&mainline::mode_binding_inputs()).unwrap();
    let scratch = Scratch::new();
    let output = std::env::var_os("AUTOSAR_MODE_C_ANALYSIS_SAMPLE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| scratch.0.join("synchronized-modes"));
    assert!(!output.exists());
    let target = if cfg!(windows) {
        autosar_config_core::target::BuildTarget::WindowsX64ControlledV1
    } else {
        autosar_config_core::target::BuildTarget::LinuxX64ControlledV1
    };
    mainline::write_files(
        &output,
        &mainline::prepared_pipeline_files_for_target(&plan, target),
    );
    assert!(output.join("generated/src/SchM_Main.c").is_file());
}

#[test]
fn dcm_services_analysis_sample() {
    let plan = mainline::build(&mainline::dcm_service_inputs()).unwrap();
    let scratch = Scratch::new();
    let output = std::env::var_os("AUTOSAR_DCM_C_ANALYSIS_SAMPLE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| scratch.0.join("dcm-services"));
    assert!(!output.exists());
    let target = if cfg!(windows) {
        autosar_config_core::target::BuildTarget::WindowsX64ControlledV1
    } else {
        autosar_config_core::target::BuildTarget::LinuxX64ControlledV1
    };
    let applications: Vec<_> = plan
        .application_slot_descriptors()
        .unwrap()
        .iter()
        .map(|slot| {
            let file = slot.source_paths[0].rsplit('/').next().unwrap();
            let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/component-pipeline/applications")
                .join(file);
            let mut text = std::fs::read_to_string(fixture).unwrap();
            if file == "Observe.c" {
                text = text.replace(
                    "    (void)Observe_CallContext(TRUE);",
                    r#"
    Dcm_SecLevelType security = 0u;
    Dcm_SesCtrlType session = 0u;
    Std_ReturnType status = Rte_Call_Diagnostics_GetSecurityLevel(&security);
    if (status == RTE_E_DCMServices_E_OK) {
        status = Rte_Call_Diagnostics_GetSesCtrlType(&session);
    }
    if (status == RTE_E_DCMServices_E_OK) {
        status = Rte_Call_Diagnostics_ResetToDefaultSession();
    }
    if (status == RTE_E_DCMServices_E_OK) {
        status = Rte_Call_Diagnostics_SetActiveDiagnostic(TRUE);
    }
    (void)Observe_Result(security, session, 0u, status);
    (void)Observe_CallContext(TRUE);"#,
                );
            }
            let path = scratch.0.join(file);
            std::fs::write(&path, text).unwrap();
            autosar_config_core::ApplicationSource {
                component_instance: slot
                    .producer_slot
                    .strip_prefix("singlecore-ecu-v1:")
                    .unwrap()
                    .into(),
                path,
            }
        })
        .collect();
    autosar_config_core::prepare_ecu_project_with_applications(&plan, target, &applications)
        .unwrap()
        .generate(&output)
        .unwrap();
}
