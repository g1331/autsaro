//! A single-component mainline project built, run and verified.
use super::*;
use std::path::Path;

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn short_application_period_saves_builds_and_passes_reference_verifier() {
    // 1 ms and 3 ms let a CanTp frame take the single Tx HOH before the
    // periodic PDU in the same tick; the verifier does not model that (#82).
    use autosar_config_core::integration::{IntegrationEdit, verify_ecu_project};
    let scratch = Scratch::new();
    let mut workspace =
        live_pipeline_workspace(&scratch.0.join("live"), &single_component_sources());
    workspace
        .edit_integration(IntegrationEdit {
            application_period_ms: Some(2),
            ..IntegrationEdit::default()
        })
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    let save = workspace.preview_integration_save().unwrap();
    workspace
        .save_integration_previewed(&save.revision)
        .unwrap();
    let plan = workspace
        .saved_integration_plan()
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert!(
        plan.description()
            .schedule
            .entities
            .iter()
            .filter(|entity| entity.application)
            .all(|entity| entity.period_ms == 2)
    );
    let (project, files) =
        single_component_project_from(&scratch.0, tooling::native_target(), &plan);
    let report = verify_ecu_project(
        &files,
        &project,
        &tooling::execution_settings(),
        &tooling::execution_owner(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert!(report.passed, "{}", report.log);
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn single_application_component_builds_and_runs_production_owner() {
    let scratch = Scratch::new();
    let (project, _) = single_component_project(&scratch.0, tooling::native_target());
    verify_production_owner(&project, &scratch.0, None);
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn single_application_component_passes_independent_reference_verifier() {
    use autosar_config_core::integration::{build_ecu_project, verify_ecu_project};
    use autosar_config_core::{ApplicationSource, prepare_ecu_project_with_applications};
    // The workbench Build/Verify entry points compare the project with the
    // closure from the same preparation, then run the delivered ecu-tool.
    let scratch = Scratch::new();
    let (project, files) = single_component_project(&scratch.0, tooling::native_target());
    let settings = tooling::execution_settings();
    let owner = tooling::execution_owner();
    let output = scratch.0.join("build");
    std::fs::create_dir(&output).unwrap();
    let built = build_ecu_project(&files, &project, &output, &settings, &owner)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(Path::new(&built.binary_path).is_file(), "{}", built.log);
    let report = verify_ecu_project(&files, &project, &settings, &owner)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(report.passed, "{}", report.log);
    assert!(report.log.to_string().contains("ECU_HANDOFF_VERIFY PASS:"));

    // A project edited after generation is not the selected delivery.
    let readme = project.join("README.md");
    let original = std::fs::read(&readme).unwrap();
    std::fs::write(&readme, b"edited\n").unwrap();
    let stale = verify_ecu_project(&files, &project, &settings, &owner).unwrap_err();
    assert!(
        serde_json::to_string(&stale)
            .unwrap()
            .contains("backend.generation.output_modified"),
        "{stale}"
    );
    std::fs::write(&readme, original).unwrap();

    // Shapes without reference inputs are refused before any tool runs.
    let plan = build(&sources_without_application_did(true)).unwrap();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/component-pipeline/applications");
    let applications: Vec<_> = plan
        .application_slot_descriptors()
        .unwrap()
        .iter()
        .map(|slot| ApplicationSource {
            component_instance: slot
                .producer_slot
                .strip_prefix("singlecore-ecu-v1:")
                .unwrap()
                .into(),
            path: fixtures.join(slot.source_paths[0].rsplit('/').next().unwrap()),
        })
        .collect();
    let files =
        prepare_ecu_project_with_applications(&plan, tooling::native_target(), &applications)
            .unwrap()
            .into_files();
    let three = scratch.0.join("three");
    write_files(&three, &files);
    let refused = verify_ecu_project(&files, &three, &settings, &owner).unwrap_err();
    assert!(
        serde_json::to_string(&refused)
            .unwrap()
            .contains("reference_verification_inputs_missing"),
        "{refused}"
    );
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn contract_headers_refuse_mismatched_consumers() {
    let plan = build(&single_component_sources()).unwrap();
    let scratch = Scratch::new();
    let directory = scratch.0.join("consumer");
    std::fs::create_dir_all(directory.join("include")).unwrap();
    for (file, bytes) in plan.component_contract_files().unwrap().files() {
        if file.ends_with(".h") {
            std::fs::write(directory.join(file), bytes).unwrap();
        }
    }
    let compiler = tooling::execution_settings().compiler;
    let compile = |name: &str| {
        let mut command = std::process::Command::new(&compiler);
        command.current_dir(&directory).args([
            "-std=c99",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
            "-Iinclude",
            "-c",
            name,
            "-o",
            "consumer.o",
        ]);
        tooling::run_public_command(
            &mut command,
            &directory,
            "contract-consumer",
            std::time::Duration::from_secs(60),
        )
    };
    // Each SWC translation unit includes its own application header.
    for (name, source) in [
        (
            "matching-sr.c",
            "#include \"Rte_Ingress.h\"\nint test(void);\nint test(void) { uint32 data=0U; return (Rte_Read_RxValue_Value(&data)==E_OK && Rte_Write_TxValue_Value(data)==E_OK) ? 0 : 1; }\n",
        ),
        (
            "matching-cs.c",
            "#include \"Rte_DcmService.h\"\nint test(void);\nint test(void) { uint8 bytes[4]={0U,0U,0U,0U}; return Rte_Call_DataServices_ApplicationValue_ReadData(bytes)==E_OK ? 0 : 1; }\n",
        ),
    ] {
        std::fs::write(directory.join(name), source).unwrap();
        let output = compile(name);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    for (name, source, expected) in [
        (
            "private-valid.c",
            "#include \"Rte_Ingress.h\"\nvoid test(void);\nvoid test(void) { uint32 data=0U; uint8 valid=0U; (void)Rte_Read_RxValue_Value(&data, &valid); }\n",
            "too many arguments",
        ),
        (
            "invalidate.c",
            "#include \"Rte_Ingress.h\"\nvoid test(void);\nvoid test(void) { (void)Rte_Invalidate_TxValue_Value(); }\n",
            "implicit declaration",
        ),
        (
            "private-service.c",
            "#include \"Rte_DcmService.h\"\nvoid test(void);\nvoid test(void) { uint8 data[4]={0U}; uint8 nrc=0U; (void)Rte_Call_DataServices_ApplicationValue_ReadData(0U, data, &nrc); }\n",
            "too many arguments",
        ),
        (
            "wrong-write.c",
            "#include \"Rte_Ingress.h\"\nvoid test(void);\nvoid test(void) { uint32 data=0U; (void)Rte_Write_TxValue_Value(&data); }\n",
            "pointer",
        ),
        (
            "wrong-operation.c",
            "#include \"Rte_DcmService.h\"\nvoid test(void);\nvoid test(void) { uint8 data[4]={0U}; (void)Rte_Call_DataServices_ApplicationValue_WriteData(data); }\n",
            "implicit declaration",
        ),
        (
            "wrong-callback.c",
            "#include \"Rte_Ingress.h\"\nStd_ReturnType (*callback)(uint32 *) = &Ingress_ReadData;\n",
            "incompatible pointer",
        ),
    ] {
        std::fs::write(directory.join(name), source).unwrap();
        let output = compile(name);
        assert!(!output.status.success(), "{name} unexpectedly compiled");
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(diagnostic.contains(expected), "{name}: {diagnostic}");
    }
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn production_preflight_and_binary_exclude_private_test_code() {
    use autosar_config_core::prepared::PreflightStatus;
    use autosar_config_core::{ApplicationSource, prepare_ecu_project_with_applications};

    let scratch = Scratch::new();
    let (project, files) = single_component_project(&scratch.0, tooling::native_target());
    let settings = tooling::execution_settings();
    let owner = tooling::execution_owner();

    // Preflight is bound to the prepared fingerprint; a missing compiler
    // fails it and the caller has nothing installed or changed.
    let plan = build(&single_component_sources()).unwrap();
    let prepared = prepare_ecu_project_with_applications(
        &plan,
        tooling::native_target(),
        &[ApplicationSource {
            component_instance: "/Application/Pipeline/IngressInstance".into(),
            path: scratch.0.join("Ingress.c"),
        }],
    )
    .unwrap();
    let application = std::fs::read(scratch.0.join("Ingress.c")).unwrap();
    let passed = prepared.native_preflight(&settings, &owner);
    assert_eq!(passed.status, PreflightStatus::Passed, "{passed:?}");
    assert_eq!(passed.fingerprint, prepared.fingerprint());
    let missing = autosar_config_core::target::ExecutionSettings::new(
        scratch.0.join(if cfg!(windows) {
            "missing-gcc.exe"
        } else {
            "missing-gcc"
        }),
        settings.objdump.clone(),
        settings.git.clone(),
        settings.python.clone(),
    )
    .unwrap();
    let failed = prepared.native_preflight(&missing, &owner);
    assert_eq!(failed.status, PreflightStatus::Failed, "{failed:?}");
    assert_eq!(failed.fingerprint, prepared.fingerprint());
    // A failed preflight keeps its source and logs for diagnosis; the test removes its copy.
    let capture = failed
        .logs
        .iter()
        .find_map(|log| match log {
            autosar_config_core::LocalizedText::Message(message)
                if message.key == "backend.prepared.preflight_capture_directory" =>
            {
                message.params["path"]
                    .as_str()
                    .map(std::path::PathBuf::from)
            }
            _ => None,
        })
        .expect("a failed preflight reports its capture directory");
    std::fs::remove_dir_all(&capture).unwrap();
    assert_eq!(
        std::fs::read(scratch.0.join("Ingress.c")).unwrap(),
        application
    );

    let output = scratch.0.join("production");
    let result = tooling::ecu_build_command(&project, &output, "host-batch", None)
        .current_dir(&scratch.0)
        .env_remove("PYTHONPATH")
        .env_remove("PYTHONHOME")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let binary = tooling::native_binary(&output, "ecu_host_batch");
    let symbols = tooling::symbol_sections(&settings.objdump, &binary);
    assert!(
        symbols
            .iter()
            .any(|(symbol, _)| symbol == "Ingress_Periodic")
    );
    for (symbol, _) in &symbols {
        assert!(
            ![
                "Os_Test",
                "Os_StackTest",
                "Os_TimeTest",
                "Os_TargetTest",
                "Os_ArtiTest",
                "Ecu_TargetTest",
                "vTaskOsTest",
            ]
            .iter()
            .any(|prefix| symbol.starts_with(prefix)),
            "Private test symbol linked into production: {symbol}"
        );
    }
    // A production build never accepts a caller control source.
    let mixed = scratch.0.join("refused-production-control");
    let result = tooling::ecu_build_command(
        &project,
        &mixed,
        "host-batch",
        Some(&scratch.0.join("Ingress.c")),
    )
    .current_dir(&scratch.0)
    .env_remove("PYTHONPATH")
    .env_remove("PYTHONHOME")
    .output()
    .unwrap();
    assert!(!result.status.success());
    assert!(!mixed.exists(), "Rejected mixed build installed an output");
    for (name, bytes) in files {
        assert_eq!(
            std::fs::read(project.join(&name)).unwrap(),
            bytes,
            "{name}: offline builds must not mutate the sealed package"
        );
    }
}
