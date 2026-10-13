//! Sealed mainline projects built offline and run with the production owner.
use super::*;
use std::path::Path;

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn copied_package_builds_offline_without_changing_source_bytes() {
    use autosar_config_core::prepared::prepare_ecu_project_for_workspace;
    let scratch = Scratch::new();
    let live = scratch.0.join("live");
    let workspace = initialized_pipeline_workspace(&live);
    let plan = workspace.saved_integration_plan().unwrap();
    let original = scratch.0.join("original");
    prepare_ecu_project_for_workspace(&workspace, &plan, tooling::native_target(), true)
        .unwrap()
        .generate(&original)
        .unwrap();
    let received = scratch.0.join("recipient/package");
    packages::copy_package(&original, &received);
    std::fs::remove_dir_all(&original).unwrap();
    std::fs::remove_dir_all(&live).unwrap();
    let before = packages::package_tree(&received);
    let output = scratch.0.join("recipient/build");
    let result = tooling::ecu_build_command(&received, &output, "host-batch", None)
        .current_dir(&received)
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
    assert!(tooling::native_binary(&output, "ecu_host_batch").is_file());
    assert_eq!(packages::package_tree(&received), before);
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn rejected_composition_contracts_refuse_stale_generation_and_preserve_sealed_output() {
    use autosar_config_core::prepared::prepare_ecu_project_for_workspace;
    for schedule in [false, true] {
        let scratch = Scratch::new();
        let live = scratch.0.join("live");
        let mut workspace = live_pipeline_workspace(&live, &inputs());
        let preview = workspace.preview_application_initialization().unwrap();
        workspace
            .initialize_application_previewed(&preview)
            .unwrap();
        let plan = workspace.saved_integration_plan().unwrap();
        let project = scratch.0.join("sealed");
        prepare_ecu_project_for_workspace(&workspace, &plan, tooling::native_target(), true)
            .unwrap()
            .generate(&project)
            .unwrap();
        let stale =
            prepare_ecu_project_for_workspace(&workspace, &plan, tooling::native_target(), true)
                .unwrap();
        let original = packages::payload(&project);
        let mut changed = if schedule {
            conflicting_schedule_sources()
        } else {
            inputs()
        };
        if !schedule {
            change(
                &mut changed,
                "types.arxml",
                "<SHORT-NAME>Input</SHORT-NAME>",
                "<SHORT-NAME>Ecu_TargetIsOwner</SHORT-NAME>",
            );
        }
        for source in &changed {
            std::fs::write(live.join(source.logical_path()), source.bytes()).unwrap();
        }
        if let Ok(reopened) = autosar_config_core::Workspace::open_project_manifest(
            &live.join("workbench-project.json"),
            &scratch.0.join("cache"),
        ) {
            assert!(reopened.saved_integration_plan().is_err());
            assert!(
                prepare_ecu_project_for_workspace(&reopened, &plan, tooling::native_target(), true)
                    .is_err()
            );
        }
        assert!(workspace.saved_integration_plan().is_err());
        assert!(
            prepare_ecu_project_for_workspace(&workspace, &plan, tooling::native_target(), true)
                .is_err()
        );
        assert!(stale.generate(&project).is_err());
        assert_eq!(packages::payload(&project), original);
        for source in &changed {
            assert_eq!(
                std::fs::read(live.join(source.logical_path())).unwrap(),
                source.bytes()
            );
        }
    }
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn sealed_project_builds_and_runs_production_owner() {
    use autosar_config_core::{Workspace, prepared::prepare_ecu_project_for_workspace};
    for (renamed, swapped, p2_star, triggers) in [
        (false, false, None, None),
        (true, false, None, None),
        (false, true, Some(("100", "2710")), None),
        (false, false, Some(("65.54", "199a")), None),
        (false, false, None, Some(false)),
        (false, false, None, Some(true)),
    ] {
        let scratch = Scratch::new();
        let mut arxml = triggers
            .map(equivalent_schedule_triggers)
            .unwrap_or_else(inputs);
        if swapped {
            parameter(&mut arxml, "Receive", "CanObjectId", "1");
            parameter(&mut arxml, "Transmit", "CanObjectId", "0");
        }
        if let Some((seconds, _)) = p2_star {
            for session in ["DCM_DEFAULT_SESSION", "DCM_EXTENDED_DIAGNOSTIC_SESSION"] {
                parameter(&mut arxml, session, "DcmDspSessionP2StarServerMax", seconds);
            }
        }
        if renamed {
            for (before, after) in [
                ("Ingress", "Gateway"),
                ("Process</", "Compute</"),
                ("Process/", "Compute/"),
                ("Process_", "Compute_"),
                ("ProcessInstance", "ComputeInstance"),
                ("ResultService", "Calculation"),
                ("Transform", "Calculate"),
            ] {
                replace_all(&mut arxml, before, after);
            }
            arxml.reverse();
        }
        let live = scratch.0.join("live");
        let mut workspace = live_pipeline_workspace(&live, &arxml);
        let initialization = workspace.preview_application_initialization().unwrap();
        workspace
            .initialize_application_previewed(&initialization)
            .unwrap();
        let plan = build(&arxml).unwrap();
        for slot in plan.application_slot_descriptors().unwrap().iter() {
            let file = slot.source_paths[0].rsplit('/').next().unwrap();
            let original = file
                .replace("Gateway", "Ingress")
                .replace("Compute", "Process");
            let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/component-pipeline/applications")
                .join(original);
            let mut text = std::fs::read_to_string(fixture).unwrap();
            if renamed {
                for (before, after) in [
                    ("Ingress", "Gateway"),
                    ("Process", "Compute"),
                    ("ResultService", "Calculation"),
                    ("Transform", "Calculate"),
                ] {
                    text = text.replace(before, after);
                }
            }
            std::fs::write(live.join(&slot.source_paths[0]), text).unwrap();
        }
        let workspace = Workspace::open_project_manifest(
            &live.join("workbench-project.json"),
            &scratch.0.join("cache"),
        )
        .unwrap();
        let plan = workspace.saved_integration_plan().unwrap();
        let prepared =
            prepare_ecu_project_for_workspace(&workspace, &plan, tooling::native_target(), true)
                .unwrap_or_else(|issues| panic!("{issues:?}"));
        assert_eq!(prepared.application_slots().len(), 3);
        let project = scratch.0.join("project");
        prepared.generate(&project).unwrap();
        verify_production_owner(&project, &scratch.0, p2_star);
    }
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn sealed_owner_preserves_network_init_and_deadline_phase() {
    use autosar_config_core::{ApplicationSource, prepare_ecu_project_with_applications};
    let scratch = Scratch::new();
    let mut source = inputs();
    change(
        &mut source,
        "ingress.arxml",
        "<VALUE>0</VALUE>",
        "<VALUE>7</VALUE>",
    );
    let plan = build(&source).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
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
            path: fixture.join(slot.source_paths[0].rsplit('/').next().unwrap()),
        })
        .collect();
    let prepared =
        prepare_ecu_project_with_applications(&plan, tooling::native_target(), &applications)
            .unwrap();
    let project = scratch.0.join("project");
    for (path, bytes) in prepared.into_files() {
        let path = project.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    let output = scratch.0.join("build");
    let built = tooling::ecu_build_command(
        &project,
        &output,
        "test",
        Some(&fixture.join("owner_probe.c")),
    )
    .output()
    .unwrap();
    assert!(
        built.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
    for phase in ["before", "after", "canonical", "controls", "epoch0", "late"] {
        let logs = scratch.0.join(phase);
        std::fs::create_dir(&logs).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let spec = autosar_config_core::execution::ProcessSpec::for_duration(
            vec![
                tooling::native_binary(&output, "ecu_probe").into_os_string(),
                phase.into(),
            ],
            scratch.0.clone(),
            vec![],
            std::time::Duration::from_secs(30),
            logs,
        )
        .unwrap();
        let result = tooling::execution_owner()
            .spawn(spec, None)
            .unwrap()
            .wait()
            .unwrap();
        let text = std::fs::read_to_string(&result.stdout).unwrap();
        assert!(
            result.success(),
            "{phase}: {result:?}\n{text}\n{}",
            std::fs::read_to_string(&result.stderr).unwrap()
        );
        let observations: Vec<_> = text
            .lines()
            .filter_map(|line| {
                let fields: Vec<_> = line.split_whitespace().collect();
                (fields.first() == Some(&"OBS")).then(|| {
                    (
                        fields[1].to_string(),
                        fields[2].parse::<u64>().unwrap(),
                        fields[3].parse::<u8>().unwrap(),
                        fields[4].parse::<u32>().unwrap(),
                    )
                })
            })
            .collect();
        let expected: Vec<_> = if phase == "late" {
            assert_eq!(
                text.lines()
                    .filter(|line| line.starts_with("LATE "))
                    .collect::<Vec<_>>(),
                [
                    "LATE 1 1800 0762123400000000",
                    "LATE 2 1800 0762123400000000"
                ]
            );
            vec![("rx".into(), 1, 0, 21), ("rx".into(), 2, 0, 21)]
        } else if phase == "epoch0" {
            std::iter::once(("rx".to_string(), 0, 0, 21))
                .chain((1..=30u64).map(|epoch| {
                    (
                        "main".to_string(),
                        epoch,
                        if epoch == 30 { 64 } else { 0 },
                        21,
                    )
                }))
                .collect()
        } else if phase == "controls" {
            (1..=120u64)
                .flat_map(|epoch| {
                    let (status, value) = match epoch {
                        1 => (133, 7),
                        2..=20 => (0, 21),
                        21..=42 => (0, 42),
                        43..=71 => (0, 43),
                        72 | 74 => (64, 43),
                        73 => (128, 43),
                        75 => (0, 0),
                        76..=104 => (0, 45),
                        105..=109 => (64, 45),
                        _ => (128, 45),
                    };
                    let mut rows = vec![("main".to_string(), epoch, status, value)];
                    match epoch {
                        1 => rows.push(("rx".into(), epoch, 0, 21)),
                        20 => rows.push(("rx".into(), epoch, 0, 42)),
                        42 => rows.push(("rx".into(), epoch, 0, 43)),
                        73 => rows.push(("rx".into(), epoch, 128, 43)),
                        74 => rows.push(("rx".into(), epoch, 0, 44)),
                        75 => rows.push(("rx".into(), epoch, 0, 45)),
                        _ => {}
                    }
                    rows
                })
                .collect()
        } else {
            (1..=if phase == "canonical" { 20u64 } else { 61u64 })
                .flat_map(|epoch| {
                    let value = if epoch == 1 {
                        7
                    } else if phase == "canonical" {
                        0x12345678
                    } else if epoch < 31 || (epoch == 31 && phase == "after") {
                        21
                    } else {
                        42
                    };
                    let status = if epoch == 1 {
                        133
                    } else if epoch == 61 || (epoch == 31 && phase == "after") {
                        64
                    } else {
                        0
                    };
                    let mut rows = vec![("main".to_string(), epoch, status, value)];
                    if epoch == 1 {
                        rows.push((
                            "rx".into(),
                            epoch,
                            0,
                            if phase == "canonical" { 0x12345678 } else { 21 },
                        ));
                    }
                    if epoch == 31 && phase == "after" {
                        rows.push(("rx".into(), epoch, 0, 42));
                        rows.push(("rx".into(), epoch, 0, 42));
                    }
                    if epoch == 30 && phase == "before" {
                        rows.push(("rx".into(), 31, 0, 42));
                        rows.push(("rx".into(), 31, 0, 42));
                    }
                    rows
                })
                .collect()
        };
        assert_eq!(observations, expected, "{phase}: {text}");
        assert!(std::fs::read(&result.stderr).unwrap().is_empty());
    }
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn sealed_optional_did_and_network_only_routes_use_real_producers() {
    use autosar_config_core::{ApplicationSource, prepare_ecu_project_with_applications};
    for diagnostic in [true, false] {
        let scratch = Scratch::new();
        let plan = build(&sources_without_application_did(diagnostic)).unwrap();
        assert!(plan.description().diagnostic.is_none());
        assert_eq!(
            plan.description().diagnostic_transport.is_some(),
            diagnostic
        );
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
        let prepared =
            prepare_ecu_project_with_applications(&plan, tooling::native_target(), &applications)
                .unwrap();
        let project = scratch.0.join("project");
        for (path, bytes) in prepared.into_files() {
            let path = project.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        let output = scratch.0.join("build");
        let built = tooling::ecu_build_command(&project, &output, "host-batch", None)
            .output()
            .unwrap();
        assert!(
            built.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
        let logs = scratch.0.join("logs");
        std::fs::create_dir(&logs).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut spec = autosar_config_core::execution::ProcessSpec::for_duration(
            vec![tooling::native_binary(&output, "ecu_host_batch").into_os_string()],
            scratch.0.clone(),
            vec![],
            std::time::Duration::from_secs(30),
            logs,
        )
        .unwrap();
        spec.stdin_stream = true;
        let owner = tooling::execution_owner();
        let mut child = owner.spawn(spec, None).unwrap();
        let input_result = child.write_stdin(b"BEGIN 0\nRX 800 4 2A000000\nRX 1792 8 0322F18600000000\nCOMMIT\nBEGIN 1\nCOMMIT\nBEGIN 1\nRX 1792 8 0322123400000000\nCOMMIT\nBEGIN 2\nCOMMIT\nBEGIN 10\nCOMMIT\n");
        child.close_stdin();
        let result = child.wait().unwrap();
        let text = std::fs::read_to_string(&result.stdout).unwrap();
        assert!(
            result.success(),
            "{diagnostic}: {result:?} input={input_result:?}\n{text}\n{}",
            std::fs::read_to_string(&result.stderr).unwrap()
        );
        let records: Vec<_> = text
            .lines()
            .filter(|line| line.starts_with("OUT "))
            .map(|line| {
                let fields: std::collections::BTreeMap<_, _> = line
                    .split_whitespace()
                    .skip(1)
                    .map(|item| item.split_once('=').unwrap())
                    .collect();
                (
                    fields["epoch"].parse::<u64>().unwrap(),
                    fields["id"].parse::<u32>().unwrap(),
                    fields["dlc"].parse::<u8>().unwrap(),
                    fields["data"].to_string(),
                )
            })
            .collect();
        let expected = if diagnostic {
            vec![
                (1, 1800, 8, "0462f18601000000".to_string()),
                (2, 1800, 8, "037f223100000000".into()),
                (10, 801, 4, "2a000000".into()),
            ]
        } else {
            vec![(10, 801, 4, "2a000000".into())]
        };
        assert_eq!(records, expected, "{diagnostic}: {text}");
        assert!(
            !text
                .lines()
                .any(|line| line.starts_with("REJECT ") || line.starts_with("COMMIT_ERROR ")),
            "{text}"
        );
        assert!(std::fs::read(&result.stderr).unwrap().is_empty());
        let config =
            std::fs::read_to_string(project.join("generated/src/Ecu_RuntimeConfig.c")).unwrap();
        let metadata = std::fs::read_to_string(
            project.join("generated/descriptions/Host_Implementation.arxml"),
        )
        .unwrap();
        if !diagnostic {
            assert!(!config.contains("CanTp_MainFunction();"));
            assert!(!config.contains("Dcm_MainFunction();"));
            let document = roxmltree::Document::parse(&metadata).unwrap();
            assert!(
                !document
                    .descendants()
                    .filter(|node| node.has_tag_name("BSW-TIMING-EVENT"))
                    .any(|node| node.descendants().any(|child| child
                        .text()
                        .is_some_and(|text| text.contains("CanTp_MainFunction")
                            || text.contains("Dcm_MainFunction"))))
            );
        }
    }
}

#[test]
fn resealed_handoff_forgeries_are_refused_by_the_delivered_tool() {
    let scratch = Scratch::new();
    let workspace = initialized_pipeline_workspace(&scratch.0.join("live"));
    let plan = workspace.saved_integration_plan().unwrap();
    for (case, _, python_reason) in HANDOFF_FORGERIES {
        let package = forged_handoff(case, &workspace, &plan, &scratch.0);
        let bytes = packages::payload(&package);
        {
            let output = scratch.0.join(format!("{case}-build"));
            let result = tooling::ecu_build_command(&package, &output, "host-batch", None)
                .current_dir(&scratch.0)
                .env_remove("PYTHONPATH")
                .env_remove("PYTHONHOME")
                .output()
                .unwrap();
            assert!(!result.status.success());
            let stderr = String::from_utf8_lossy(&result.stderr);
            assert!(stderr.contains(python_reason), "{case}: {stderr}");
            assert!(!output.exists());
        }
        assert_eq!(packages::payload(&package), bytes);
    }
}

#[test]
fn received_handoff_builds_and_runs_production_owner() {
    use autosar_config_core::{generator::delivery, prepared::prepare_ecu_project_for_workspace};
    let scratch = Scratch::new();
    let live = scratch.0.join("author");
    let workspace = initialized_pipeline_workspace(&live);
    let plan = workspace.saved_integration_plan().unwrap();
    let moved = scratch.0.join("consumer/package");
    std::fs::create_dir(moved.parent().unwrap()).unwrap();
    prepare_ecu_project_for_workspace(&workspace, &plan, native_delivery_target(), true)
        .unwrap()
        .generate(&moved)
        .unwrap();
    std::fs::remove_dir_all(&live).unwrap();
    let imported = delivery::open_handoff(
        &moved,
        &scratch.0.join("consumer/live"),
        &autosar_config_core::definitions::DefinitionCatalog::builtin().unwrap(),
    )
    .unwrap();
    let plan = imported.workspace.saved_integration_plan().unwrap();
    let regenerated = scratch.0.join("consumer/regenerated");
    prepare_ecu_project_for_workspace(&imported.workspace, &plan, native_delivery_target(), true)
        .unwrap()
        .generate(&regenerated)
        .unwrap();
    assert_eq!(packages::payload(&moved), packages::payload(&regenerated));
    verify_received_owner(&regenerated, moved.parent().unwrap());
}

fn verify_received_owner(project: &Path, consumer: &Path) {
    verify_production_owner(project, consumer, None);
    let control = consumer.join("owner_probe.c");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/component-pipeline/applications/owner_probe.c"),
        &control,
    )
    .unwrap();
    let output = consumer.join("probe-build");
    let result = tooling::ecu_build_command(project, &output, "test", Some(&control))
        .current_dir(consumer)
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
    let logs = consumer.join("probe-logs");
    std::fs::create_dir(&logs).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let spec = autosar_config_core::execution::ProcessSpec::for_duration(
        vec![
            tooling::native_binary(&output, "ecu_probe").into_os_string(),
            "canonical".into(),
        ],
        consumer.to_path_buf(),
        vec![],
        std::time::Duration::from_secs(30),
        logs,
    )
    .unwrap();
    let result = tooling::execution_owner()
        .spawn(spec, None)
        .unwrap()
        .wait()
        .unwrap();
    let text = std::fs::read_to_string(&result.stdout).unwrap();
    assert!(
        result.success(),
        "{result:?}\n{text}\n{}",
        std::fs::read_to_string(&result.stderr).unwrap()
    );
    let observations = text
        .lines()
        .filter_map(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            (fields.first() == Some(&"OBS")).then(|| {
                (
                    fields[1].to_owned(),
                    fields[2].parse::<u64>().unwrap(),
                    fields[3].parse::<u8>().unwrap(),
                    fields[4].parse::<u32>().unwrap(),
                )
            })
        })
        .collect::<Vec<_>>();
    let expected = (1..=20u64)
        .flat_map(|epoch| {
            let mut records = vec![(
                "main".to_owned(),
                epoch,
                if epoch == 1 { 133 } else { 0 },
                if epoch == 1 { 7 } else { 0x12345678 },
            )];
            if epoch == 1 {
                records.push(("rx".to_owned(), epoch, 0, 0x12345678));
            }
            records
        })
        .collect::<Vec<_>>();
    assert_eq!(observations, expected, "{text}");
    assert!(std::fs::read(&result.stderr).unwrap().is_empty());
}
