//! Mainline handoff packages: README, reconstruction and forgery refusal.
use super::*;
use autosar_config_core::definitions::DefinitionCatalog;

#[test]
fn handoff_readme_supports_both_native_targets_without_build_tools() {
    use autosar_config_core::{prepared::prepare_ecu_project_for_workspace, target::BuildTarget};
    let scratch = Scratch::new();
    let workspace = initialized_pipeline_workspace(&scratch.0.join("live"));
    let before = workspace.generation_snapshot().unwrap();
    let plan = workspace.saved_integration_plan().unwrap();
    for target in [
        BuildTarget::WindowsX64ControlledV1,
        BuildTarget::LinuxX64ControlledV1,
    ] {
        let output = scratch.0.join(target.spec().id);
        prepare_ecu_project_for_workspace(&workspace, &plan, target, true)
            .unwrap()
            .generate(&output)
            .unwrap();
        packages::assert_offline_build_readme(
            &output,
            target == BuildTarget::WindowsX64ControlledV1,
        );
        let text = std::fs::read_to_string(output.join("README.md")).unwrap();
        assert!(text.contains("--mode host-batch"));
        assert!(!text.contains("tools/ecu-tool.py verify"));
        for expected in [
            "never writes the live source",
            "not silently upgraded",
            "No official archive, compiler, checkout or network",
            "AUTOSAR/MISRA compliance",
        ] {
            assert!(text.contains(expected), "missing {expected}");
        }
    }
    for source in before.applications {
        assert_eq!(std::fs::read(source.disk_path).unwrap(), source.bytes);
    }
}

#[test]
fn moved_handoff_reconstructs_every_user_source_and_exact_producer() {
    use autosar_config_core::{generator::delivery, prepared::prepare_ecu_project_for_workspace};
    let scratch = Scratch::new();
    let live = scratch.0.join("author");
    let workspace = initialized_pipeline_workspace(&live);
    let expected = workspace.generation_snapshot().unwrap();
    let plan = workspace.saved_integration_plan().unwrap();
    let prepared =
        prepare_ecu_project_for_workspace(&workspace, &plan, native_delivery_target(), true)
            .unwrap();
    let package = scratch.0.join("original-package");
    prepared.generate(&package).unwrap();
    let moved = scratch.0.join("consumer/package");
    std::fs::create_dir(moved.parent().unwrap()).unwrap();
    std::fs::rename(&package, &moved).unwrap();
    std::fs::remove_dir_all(&live).unwrap();
    assert!(!live.exists() && !package.exists());
    let receiver = scratch.0.join("consumer/live");
    let imported =
        delivery::open_handoff(&moved, &receiver, &DefinitionCatalog::builtin().unwrap()).unwrap();
    let actual = imported.workspace.generation_snapshot().unwrap();
    assert_eq!(actual.manifest, expected.manifest);
    for (before, after) in [
        (&expected.inputs, &actual.inputs),
        (&expected.applications, &actual.applications),
    ] {
        let bytes = |sources: &[autosar_config_core::arxml::GenerationInputSnapshot]| {
            sources
                .iter()
                .map(|source| (source.logical_path.clone(), source.bytes.clone()))
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        assert_eq!(bytes(before), bytes(after));
        assert!(after.iter().all(|source| {
            source
                .disk_path
                .canonicalize()
                .unwrap()
                .starts_with(receiver.canonicalize().unwrap())
        }));
    }
    let applications = imported
        .metadata
        .input_snapshots
        .iter()
        .filter(|input| input.kind == delivery::InputKind::Application)
        .map(|input| {
            (
                input.producer_slot.as_deref().unwrap(),
                input.logical_path.as_str(),
                input.package_path.as_str(),
            )
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        applications,
        std::collections::BTreeSet::from([
            (
                "singlecore-ecu-v1:/Application/Pipeline/IngressInstance",
                "application/Ingress.c",
                "src/Ingress.c"
            ),
            (
                "singlecore-ecu-v1:/Application/Pipeline/ProcessInstance",
                "application/Process.c",
                "src/Process.c"
            ),
            (
                "singlecore-ecu-v1:/Application/Pipeline/ObserveInstance",
                "application/Observe.c",
                "src/Observe.c"
            ),
        ])
    );
    let plan = imported.workspace.saved_integration_plan().unwrap();
    let regenerated = scratch.0.join("consumer/regenerated");
    let prepared = prepare_ecu_project_for_workspace(
        &imported.workspace,
        &plan,
        native_delivery_target(),
        true,
    )
    .unwrap();
    let slots = prepared.application_slots().to_vec();
    prepared.generate(&regenerated).unwrap();
    assert_eq!(packages::payload(&moved), packages::payload(&regenerated));
    let readme = std::fs::read_to_string(regenerated.join("README.md")).unwrap();
    assert!(
        !readme.contains("epic4-single-application-v1") && !readme.contains("src/Application.c")
    );
    for slot in &slots {
        for identity in std::iter::once(&slot.component_path)
            .chain(std::iter::once(&slot.producer_slot))
            .chain(slot.source_paths.iter())
            .chain(slot.generated_headers.iter())
            .chain(slot.entry_symbols.iter())
        {
            assert!(
                readme.contains(identity),
                "missing README identity: {identity}"
            );
        }
    }
    for (producer, logical, compiled) in applications {
        let ledger: serde_json::Value = serde_json::from_slice(
            &std::fs::read(regenerated.join("workbench-ownership.json")).unwrap(),
        )
        .unwrap();
        let row = ledger["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == compiled)
            .unwrap();
        assert_eq!(row["owner"], "user-application");
        assert_eq!(row["producerId"], producer);
        assert_eq!(row["snapshotOf"], logical);
    }
    let payload = packages::payload(&regenerated);
    assert_eq!(payload, packages::payload(&moved));
    for bytes in payload.values() {
        assert!(
            !bytes
                .windows(live.to_str().unwrap().len())
                .any(|window| window == live.to_str().unwrap().as_bytes())
        );
        assert!(
            !bytes
                .windows(package.to_str().unwrap().len())
                .any(|window| window == package.to_str().unwrap().as_bytes())
        );
    }
}

#[test]
fn handoff_rejects_resealed_authority_and_snapshot_forgery() {
    use autosar_config_core::generator::delivery;
    let scratch = Scratch::new();
    let workspace = initialized_pipeline_workspace(&scratch.0.join("live"));
    let plan = workspace.saved_integration_plan().unwrap();
    let source = workspace.generation_snapshot().unwrap();
    let sentinel = scratch.0.join("outside.c");
    std::fs::write(&sentinel, b"outside owner bytes").unwrap();
    for (case, rust_reason, python_reason) in HANDOFF_FORGERIES {
        let package = forged_handoff(case, &workspace, &plan, &scratch.0);
        let bytes = packages::payload(&package);
        let receiver = scratch.0.join(format!("{case}-receiver"));
        let error =
            delivery::open_handoff(&package, &receiver, &DefinitionCatalog::builtin().unwrap())
                .err()
                .unwrap();
        let evidence = serde_json::to_string(&error).unwrap();
        assert!(evidence.contains(rust_reason), "{case}: {evidence}");
        assert!(!receiver.exists());
        std::fs::create_dir(&receiver).unwrap();
        std::fs::write(receiver.join("user.txt"), b"receiver owner bytes").unwrap();
        let retained =
            delivery::open_handoff(&package, &receiver, &DefinitionCatalog::builtin().unwrap())
                .err()
                .unwrap();
        assert_eq!(retained, error);
        assert_eq!(
            std::fs::read(receiver.join("user.txt")).unwrap(),
            b"receiver owner bytes"
        );
        assert_eq!(std::fs::read_dir(&receiver).unwrap().count(), 1);
        let _ = python_reason;
        assert_eq!(packages::payload(&package), bytes);
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"outside owner bytes");
        for input in source.inputs.iter().chain(source.applications.iter()) {
            assert_eq!(std::fs::read(&input.disk_path).unwrap(), input.bytes);
        }
    }
}

#[test]
fn unsupported_edit_and_handoff_preserve_live_inputs_and_existing_output() {
    use autosar_config_core::{Workspace, integration::IntegrationEdit, target::BuildTarget};
    let scratch = Scratch::new();
    let source_directory = scratch.0.join("source");
    std::fs::create_dir(&source_directory).unwrap();
    let inputs = inputs();
    let paths: Vec<_> = inputs
        .iter()
        .map(|source| {
            let path = source_directory.join(source.logical_path());
            std::fs::write(&path, source.bytes()).unwrap();
            path
        })
        .collect();
    let mut workspace = Workspace::open(paths.clone()).unwrap();
    let before = serde_json::to_value(workspace.project_projection("same").unwrap()).unwrap();
    let errors = workspace
        .edit_integration(IntegrationEdit {
            can_ids: [("/Application/NotASignalPort".to_owned(), 1)].into(),
            ..IntegrationEdit::default()
        })
        .err()
        .unwrap();
    assert!(errors.iter().any(|issue| issue.code == "EDIT_UNSUPPORTED"));
    assert_eq!(
        before,
        serde_json::to_value(workspace.project_projection("same").unwrap()).unwrap()
    );
    let plan = build(&inputs).unwrap();
    let contract = plan.component_contract_files().unwrap();
    let output = scratch.0.join("contract");
    let preview = contract.preview(&output).unwrap();
    contract
        .generate_previewed(&output, &preview.revision)
        .unwrap();
    assert!(
        plan.ecu_handoff_files(BuildTarget::LinuxX64ControlledV1)
            .is_err()
    );
    assert!(
        plan.ecu_integration_files(BuildTarget::LinuxX64ControlledV1)
            .is_err()
    );
    for (file, bytes) in contract.files() {
        assert_eq!(&std::fs::read(output.join(file)).unwrap(), bytes);
    }
    for (source, path) in inputs.iter().zip(paths) {
        assert_eq!(std::fs::read(path).unwrap(), source.bytes());
    }
}
