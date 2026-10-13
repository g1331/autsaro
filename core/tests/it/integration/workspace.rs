//! Mainline workspace initialization, reopening and source safety.
use super::*;
use std::path::Path;

#[test]
fn multi_workspace_initialization_reopen_and_regeneration_preserve_every_user_source() {
    use autosar_config_core::{
        Workspace, prepared::prepare_ecu_project_for_workspace, target::BuildTarget,
    };
    for renamed in [false, true] {
        let scratch = Scratch::new();
        let root = scratch.0.join("live");
        let mut arxml = inputs();
        if renamed {
            for (before, after) in [
                ("Process</", "Compute</"),
                ("Process/", "Compute/"),
                ("Process_", "Compute_"),
                ("ProcessInstance", "ComputeInstance"),
                ("ResultService", "Calculation"),
                ("Transform", "Calculate"),
            ] {
                replace_all(&mut arxml, before, after);
            }
        }
        let mut workspace = live_multi_workspace(&root, &arxml);
        let preview = workspace.preview_application_initialization().unwrap();
        assert_eq!(preview.slots.len(), 3);
        assert_eq!(preview.files.len(), 3);
        let mut stale = preview.clone();
        stale.slots[1].component_path.push_str("/untrusted");
        assert!(workspace.initialize_application_previewed(&stale).is_err());
        assert!(
            preview
                .files
                .iter()
                .all(|file| !root.join(&file.path).exists())
        );
        for path in [
            root.join("workbench-project.json"),
            root.join(arxml[0].logical_path()),
        ] {
            let before = std::fs::read(&path).unwrap();
            let mut edited = before.clone();
            edited.push(b'\n');
            std::fs::write(&path, &edited).unwrap();
            assert!(
                workspace
                    .initialize_application_previewed(&preview)
                    .is_err()
            );
            assert_eq!(std::fs::read(&path).unwrap(), edited);
            assert!(
                preview
                    .files
                    .iter()
                    .all(|file| !root.join(&file.path).exists())
            );
            std::fs::write(&path, before).unwrap();
        }
        let outcome = workspace
            .initialize_application_previewed(&preview)
            .unwrap();
        assert!(outcome.warnings.is_empty());
        assert!(outcome.retained_recovery_files.is_empty());
        let mut user_sources = std::collections::BTreeMap::new();
        for (index, file) in preview.files.iter().enumerate() {
            let mut bytes = file.contents.as_bytes().to_vec();
            bytes.extend_from_slice(
                format!("\r\n/* User edit {index}: 用户源码. */\r\n").as_bytes(),
            );
            std::fs::write(root.join(&file.path), &bytes).unwrap();
            user_sources.insert(file.path.clone(), bytes);
        }
        assert!(workspace.generation_snapshot().is_err());
        let workspace = Workspace::open_project_manifest(
            &root.join("workbench-project.json"),
            &root.join("cache"),
        )
        .unwrap();
        let plan = workspace.saved_integration_plan().unwrap();
        let output = scratch.0.join("sealed");
        let prepared = || {
            prepare_ecu_project_for_workspace(
                &workspace,
                &plan,
                BuildTarget::LinuxX64ControlledV1,
                true,
            )
            .unwrap()
        };
        let generation = prepared().preview(&output).unwrap();
        prepared()
            .generate_previewed(&output, &generation.revision)
            .unwrap();
        for (path, bytes) in &user_sources {
            assert_eq!(&std::fs::read(root.join(path)).unwrap(), bytes);
            assert_eq!(
                &std::fs::read(output.join(path.replace("application/", "src/"))).unwrap(),
                bytes
            );
        }
        for source in &arxml {
            assert_eq!(
                std::fs::read(root.join(source.logical_path())).unwrap(),
                source.bytes()
            );
        }
        let original_output = std::fs::read(output.join("files.sha256")).unwrap();
        let old_preview = prepared().preview(&output).unwrap();
        let mut guarded = user_sources
            .iter()
            .map(|(path, bytes)| (root.join(path), bytes.clone()))
            .collect::<Vec<_>>();
        guarded.push((
            root.join("workbench-project.json"),
            std::fs::read(root.join("workbench-project.json")).unwrap(),
        ));
        guarded.push((
            root.join(arxml[0].logical_path()),
            arxml[0].bytes().to_vec(),
        ));
        for (path, bytes) in guarded {
            let pending = prepared();
            let mut changed = bytes.clone();
            changed.push(b'\n');
            std::fs::write(&path, &changed).unwrap();
            assert!(
                pending
                    .generate_previewed(&output, &old_preview.revision)
                    .is_err()
            );
            assert_eq!(std::fs::read(&path).unwrap(), changed);
            assert_eq!(
                std::fs::read(output.join("files.sha256")).unwrap(),
                original_output
            );
            for (source, bytes) in &user_sources {
                assert_eq!(
                    &std::fs::read(output.join(source.replace("application/", "src/"))).unwrap(),
                    bytes
                );
            }
            std::fs::write(&path, bytes).unwrap();
        }
    }
}

#[test]
fn multi_workspace_reopen_rejects_incomplete_unknown_and_wrong_slot_membership() {
    use autosar_config_core::Workspace;
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut workspace = live_multi_workspace(&root, &inputs());
    let preview = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&preview)
        .unwrap();
    let original = workspace.project_manifest().unwrap().clone();
    for mutation in 0..5 {
        let mut manifest = original.clone();
        match mutation {
            0 => {
                manifest.application_inputs.pop();
            }
            1 => manifest.application_inputs[0]
                .producer_slot
                .push_str("/unknown"),
            2 => manifest.application_inputs.swap(0, 1),
            3 => manifest
                .application_inputs
                .push(manifest.application_inputs[0].clone()),
            4 => manifest.application_inputs[0].path = "application/Unknown.c".into(),
            _ => unreachable!(),
        }
        if mutation == 2 {
            let slot = manifest.application_inputs[0].producer_slot.clone();
            manifest.application_inputs[0].producer_slot =
                manifest.application_inputs[1].producer_slot.clone();
            manifest.application_inputs[1].producer_slot = slot;
        }
        if mutation == 4 {
            std::fs::write(root.join("application/Unknown.c"), "/* unknown */").unwrap();
        }
        let path = root.join("workbench-project.json");
        std::fs::write(&path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        assert!(
            Workspace::open_project_manifest(&path, &root.join("cache")).is_err(),
            "mutation {mutation}"
        );
    }
    std::fs::write(
        root.join("workbench-project.json"),
        serde_json::to_vec_pretty(&original).unwrap(),
    )
    .unwrap();
    // Ownership remains structurally valid while an unrelated target restriction blocks generation.
    let ecuc_path = root.join("ecuc.arxml");
    let ecuc = std::fs::read_to_string(&ecuc_path).unwrap();
    let changed = ecuc.replacen("<VALUE>IMMEDIATE</VALUE>", "<VALUE>DEFERRED</VALUE>", 1);
    assert_ne!(ecuc, changed);
    std::fs::write(&ecuc_path, changed).unwrap();
    let reopened =
        Workspace::open_project_manifest(&root.join("workbench-project.json"), &root.join("cache"))
            .unwrap();
    assert!(reopened.saved_integration_plan().is_err());
}

#[cfg(unix)]
#[test]
fn multi_workspace_initialization_and_reopen_refuse_symlink_members() {
    use autosar_config_core::{Workspace, prepared::prepare_ecu_project_for_workspace};
    use std::os::unix::fs::symlink;
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut workspace = live_multi_workspace(&root, &inputs());
    let preview = workspace.preview_application_initialization().unwrap();
    let source = root.join(&preview.files[1].path);
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    let external = scratch.0.join("external.c");
    let bytes = b"/* external user source */\n";
    std::fs::write(&external, bytes).unwrap();
    symlink(&external, &source).unwrap();
    assert!(
        workspace
            .initialize_application_previewed(&preview)
            .is_err()
    );
    assert_eq!(std::fs::read(&external).unwrap(), bytes);
    assert!(
        preview
            .files
            .iter()
            .filter(|file| root.join(&file.path) != source)
            .all(|file| !root.join(&file.path).exists())
    );
    std::fs::remove_file(&source).unwrap();
    workspace
        .initialize_application_previewed(&preview)
        .unwrap();
    let plan = workspace.saved_integration_plan().unwrap();
    let prepared = prepare_ecu_project_for_workspace(
        &workspace,
        &plan,
        autosar_config_core::target::BuildTarget::LinuxX64ControlledV1,
        true,
    )
    .unwrap();
    std::fs::remove_file(&source).unwrap();
    symlink(&external, &source).unwrap();
    assert!(
        Workspace::open_project_manifest(&root.join("workbench-project.json"), &root.join("cache"))
            .is_err()
    );
    let output = scratch.0.join("sealed");
    assert!(prepared.generate(&output).is_err());
    assert!(!output.exists());
    assert_eq!(std::fs::read(&external).unwrap(), bytes);
}

fn link_directory(link: &Path, target: &Path) {
    #[cfg(windows)]
    {
        let linked = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .unwrap();
        assert!(
            linked.status.success(),
            "{}",
            String::from_utf8_lossy(&linked.stderr)
        );
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
}

#[test]
fn tampered_trusted_runtime_asset_is_refused_by_identity() {
    use autosar_config_core::resources::AssetInventory;
    let scratch = Scratch::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let tampered = "runtime/multi/src/Can.c";
    let destination = scratch.0.join(tampered);
    std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
    let mut bytes = std::fs::read(root.join(tampered)).unwrap();
    std::fs::write(&destination, &bytes).unwrap();
    AssetInventory::verify_directory_asset(&scratch.0, tampered).unwrap();
    bytes[0] ^= 1;
    std::fs::write(&destination, &bytes).unwrap();
    let error = AssetInventory::verify_directory_asset(&scratch.0, tampered).unwrap_err();
    assert_eq!(
        serde_json::to_value(&error).unwrap()["key"],
        "backend.resources.identity_differs"
    );
}

#[test]
fn mainline_sources_through_a_directory_link_or_junction_are_refused() {
    use autosar_config_core::Workspace;
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    live_multi_workspace(&root, &inputs());
    let alias = scratch.0.join("alias");
    link_directory(&alias, &root);
    assert!(
        Workspace::open_project_manifest(
            &alias.join("workbench-project.json"),
            &scratch.0.join("cache")
        )
        .is_err()
    );
    assert!(Workspace::open(vec![alias.join("ingress.arxml")]).is_err());
    for source in inputs() {
        assert_eq!(
            std::fs::read(root.join(source.logical_path())).unwrap(),
            source.bytes()
        );
    }
}

#[test]
fn mainline_generation_rejects_trailing_dot_and_linked_output_without_touching_owner() {
    use autosar_config_core::prepared::prepare_ecu_project_for_workspace;
    let scratch = Scratch::new();
    let workspace = initialized_multi_user_workspace(&scratch.0.join("live"));
    let plan = workspace.saved_integration_plan().unwrap();
    let prepare = || {
        prepare_ecu_project_for_workspace(
            &workspace,
            &plan,
            autosar_config_core::target::BuildTarget::LinuxX64ControlledV1,
            false,
        )
        .unwrap()
    };
    let owner = scratch.0.join("OwnerData");
    std::fs::create_dir(&owner).unwrap();
    std::fs::write(owner.join("sentinel.txt"), b"owner content").unwrap();
    let untouched = |owner: &Path| {
        let names: Vec<_> = std::fs::read_dir(owner)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["sentinel.txt"]);
        assert_eq!(
            std::fs::read(owner.join("sentinel.txt")).unwrap(),
            b"owner content"
        );
    };
    assert!(prepare().generate(&owner.join(".")).is_err());
    untouched(&owner);
    let output = scratch.0.join("Generated");
    link_directory(&output, &owner);
    assert!(prepare().generate(&output).is_err());
    untouched(&owner);
}

#[test]
fn multi_workspace_missing_or_directory_source_refuses_reopen_and_prepared_generation() {
    use autosar_config_core::{
        Workspace, prepared::prepare_ecu_project_for_workspace, target::BuildTarget,
    };
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut workspace = live_multi_workspace(&root, &inputs());
    let initialization = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&initialization)
        .unwrap();
    let plan = workspace.saved_integration_plan().unwrap();
    let prepare = || {
        prepare_ecu_project_for_workspace(
            &workspace,
            &plan,
            BuildTarget::LinuxX64ControlledV1,
            true,
        )
        .unwrap()
    };
    let output = scratch.0.join("sealed");
    let report = prepare().generate(&output).unwrap();
    let sealed_before = report
        .files
        .iter()
        .map(|file| (file.clone(), std::fs::read(output.join(file)).unwrap()))
        .collect::<Vec<_>>();
    let user_before = initialization
        .files
        .iter()
        .map(|file| {
            (
                file.path.clone(),
                std::fs::read(root.join(&file.path)).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let manifest = root.join("workbench-project.json");
    let manifest_before = std::fs::read(&manifest).unwrap();
    let source = root.join(&user_before[1].0);
    for directory in [false, true] {
        let pending = prepare();
        std::fs::remove_file(&source).unwrap();
        if directory {
            std::fs::create_dir(&source).unwrap();
            std::fs::write(source.join("external"), b"external directory contents").unwrap();
        }
        assert!(Workspace::open_project_manifest(&manifest, &root.join("cache")).is_err());
        assert!(pending.generate(&output).is_err());
        assert_eq!(std::fs::read(&manifest).unwrap(), manifest_before);
        for (path, bytes) in user_before
            .iter()
            .filter(|(path, _)| root.join(path) != source)
        {
            assert_eq!(&std::fs::read(root.join(path)).unwrap(), bytes);
        }
        for (path, bytes) in &sealed_before {
            assert_eq!(&std::fs::read(output.join(path)).unwrap(), bytes);
        }
        if directory {
            assert_eq!(
                std::fs::read(source.join("external")).unwrap(),
                b"external directory contents"
            );
            std::fs::remove_file(source.join("external")).unwrap();
            std::fs::remove_dir(&source).unwrap();
        } else {
            assert!(!source.exists());
        }
        std::fs::write(&source, &user_before[1].1).unwrap();
    }
}

#[test]
fn multi_workspace_ownership_rejects_existing_foreign_signal_mapping_member() {
    use autosar_config_core::Workspace;
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut arxml = inputs();
    change(
        &mut arxml,
        "types.arxml",
        "</SENDER-RECEIVER-INTERFACE>",
        r#"</SENDER-RECEIVER-INTERFACE>
        <SENDER-RECEIVER-INTERFACE>
          <SHORT-NAME>ForeignInterface</SHORT-NAME>
          <IS-SERVICE>false</IS-SERVICE>
          <DATA-ELEMENTS>
            <VARIABLE-DATA-PROTOTYPE>
              <SHORT-NAME>ForeignValue</SHORT-NAME>
              <TYPE-TREF DEST="APPLICATION-PRIMITIVE-DATA-TYPE">/Types/ApplicationUint32</TYPE-TREF>
            </VARIABLE-DATA-PROTOTYPE>
          </DATA-ELEMENTS>
        </SENDER-RECEIVER-INTERFACE>"#,
    );
    let mut workspace = live_multi_workspace(&root, &arxml);
    let initialization = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&initialization)
        .unwrap();
    let extract = root.join("extract.arxml");
    let before = std::fs::read_to_string(&extract).unwrap();
    let member = r#"<TARGET-DATA-PROTOTYPE-REF DEST="VARIABLE-DATA-PROTOTYPE">/Types/ValueInterface/Value</TARGET-DATA-PROTOTYPE-REF>"#;
    assert!(before.contains(member));
    std::fs::write(&extract, before.replacen(member,
        r#"<TARGET-DATA-PROTOTYPE-REF DEST="VARIABLE-DATA-PROTOTYPE">/Types/ForeignInterface/ForeignValue</TARGET-DATA-PROTOTYPE-REF>"#, 1)).unwrap();
    let paths = arxml
        .iter()
        .map(|source| root.join(source.logical_path()))
        .chain(
            initialization
                .files
                .iter()
                .map(|file| root.join(&file.path)),
        )
        .chain(std::iter::once(root.join("workbench-project.json")));
    let bytes_before = paths
        .map(|path| {
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect::<Vec<_>>();
    let error =
        Workspace::open_project_manifest(&root.join("workbench-project.json"), &root.join("cache"))
            .err()
            .unwrap();
    assert!(error.to_string().contains("SIGNAL_MAPPING"), "{error}");
    assert!(
        !error.to_string().contains("REFERENCE_UNRESOLVED"),
        "{error}"
    );
    for (path, bytes) in bytes_before {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}
