//! Generated deliveries and handoff packages: refusals, ownership and preserved user bytes.
use super::*;
use crate::support::generated_packages::*;
use crate::support::packages::{package_tree, payload, reseal};
use autosar_config_core::Workspace;
use autosar_config_core::definitions::DefinitionCatalog;
use autosar_config_core::generator::delivery;
use autosar_config_core::prepared::prepare_ecu_project_for_workspace;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

fn signals(root: &Path) -> Workspace {
    standard_named(root, "Delivery")
}

#[test]
fn delivery_without_an_initialized_application_names_the_missing_initialization() {
    let scratch = Scratch::new();
    let creation =
        Workspace::preview_project_creation(&scratch.0.join("Live"), "Delivery", "standard-ecu")
            .unwrap();
    let workspace = Workspace::create_project_previewed(&creation).unwrap();
    let plan = workspace.saved_integration_plan().unwrap();
    let Err(issues) =
        prepare_ecu_project_for_workspace(&workspace, &plan, tooling::native_target(), false)
    else {
        panic!("delivery without application sources was accepted");
    };
    assert_eq!(issues.len(), 1, "{issues:?}");
    let issue = &issues[0];
    assert_eq!(issue.code, "APPLICATION_UNINITIALIZED");
    assert_eq!(
        issue.object.as_deref(),
        Some("/Application/ReferenceComposition")
    );
    assert_eq!(
        serde_json::to_value(&issue.message).unwrap()["key"],
        "backend.prepared.application_uninitialized"
    );
    assert_eq!(
        serde_json::to_value(&issue.remedy).unwrap()["key"],
        "backend.prepared.application_initialization_required"
    );
}

#[test]
fn builtin_handoff_reconstructs_independent_authoritative_members() {
    let scratch = Scratch::new();
    let live = scratch.0.join("Live");
    let workspace = signals(&live);
    let before = workspace.generation_snapshot().unwrap();
    let output = scratch.0.join("package");
    prepare(&workspace, true).generate(&output).unwrap();
    let imported = delivery::open_handoff(
        &output,
        &scratch.0.join("Receiver"),
        &DefinitionCatalog::builtin().unwrap(),
    )
    .unwrap();
    let actual = imported.workspace.generation_snapshot().unwrap();
    let expected: BTreeMap<_, _> = before
        .inputs
        .iter()
        .map(|input| (&input.logical_path, &input.bytes))
        .collect();
    let received: BTreeMap<_, _> = actual
        .inputs
        .iter()
        .map(|input| (&input.logical_path, &input.bytes))
        .collect();
    assert_eq!(received, expected);
    let live = fs::canonicalize(&live).unwrap();
    for input in actual.inputs {
        assert!(
            !fs::canonicalize(input.disk_path)
                .unwrap()
                .starts_with(&live)
        );
    }
    let reproduced = scratch.0.join("reproduced");
    prepare(&imported.workspace, true)
        .generate(&reproduced)
        .unwrap();
    assert_eq!(payload(&output), payload(&reproduced));
}

/// A configuration change rewrites only its `inputs/` snapshot, `generated/`,
/// `app/`, `meta/` and the root build/README files; a hand edit to any other
/// directory is refused.
#[test]
fn regeneration_changes_only_generated_directories() {
    use crate::support::os_variants::{counter_variant, prepared};
    use autosar_config_core::target::BuildTarget;
    let scratch = Scratch::new();
    let target = BuildTarget::LinuxX64ControlledV1;
    let files = |maximum| -> BTreeMap<String, Vec<u8>> {
        prepared(&counter_variant(false, maximum), &scratch.0, target)
            .into_files()
            .into_iter()
            .collect()
    };
    let (before, after) = (files(65535), files(4095));
    let changed: Vec<_> = before
        .keys()
        .chain(after.keys())
        .filter(|path| before.get(*path) != after.get(*path))
        .collect();
    assert!(changed.iter().any(|path| path.starts_with("generated/")));
    for path in changed {
        assert!(
            ["inputs/", "generated/", "app/", "meta/"]
                .iter()
                .any(|directory| path.starts_with(directory))
                || matches!(
                    path.as_str(),
                    "README.md" | "target.json" | "CMakeLists.txt"
                ),
            "{path}"
        );
    }
    let output = scratch.0.join("package");
    let sources = counter_variant(false, 65535);
    prepared(&sources, &scratch.0, target)
        .generate(&output)
        .unwrap();
    for path in [
        "bsw/src/Com.c",
        "integration/src/Ecu_Target.c",
        "os/src/Os.c",
        "kernel/tasks.c",
        "tools/ecu-tool.py",
    ] {
        let file = output.join(path);
        let original = fs::read(&file).unwrap();
        fs::write(&file, [original.as_slice(), b"\n"].concat()).unwrap();
        assert!(
            prepared(&sources, &scratch.0, target)
                .preview(&output)
                .is_err(),
            "{path}"
        );
        fs::write(&file, original).unwrap();
    }
    assert!(
        prepared(&sources, &scratch.0, target)
            .preview(&output)
            .is_ok()
    );
}

#[test]
fn builtin_resealed_generated_bytes_cannot_self_authorize_import_or_replacement() {
    let scratch = Scratch::new();
    let workspace = signals(&scratch.0.join("Live"));
    let output = scratch.0.join("package");
    prepare(&workspace, true).generate(&output).unwrap();
    let original = fs::read(output.join("generated/src/Ecu_Config.c")).unwrap();
    let mut changed = original.clone();
    changed.extend_from_slice(b"\n#error Unauthorized_generated_change\n");
    fs::write(output.join("generated/src/Ecu_Config.c"), &changed).unwrap();
    reseal(&output);
    let receiver = scratch.0.join("Receiver");
    assert!(
        delivery::open_handoff(&output, &receiver, &DefinitionCatalog::builtin().unwrap()).is_err()
    );
    assert!(!receiver.exists());
    assert!(prepare(&workspace, true).preview(&output).is_err());
    assert_eq!(
        fs::read(output.join("generated/src/Ecu_Config.c")).unwrap(),
        changed
    );
}

#[test]
fn builtin_resealed_rule_identity_is_not_a_trusted_native_resource() {
    let scratch = Scratch::new();
    let workspace = signals(&scratch.0.join("Live"));
    let output = scratch.0.join("package");
    prepare(&workspace, true).generate(&output).unwrap();
    for path in ["meta/handoff.json", "target.json"] {
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(output.join(path)).unwrap()).unwrap();
        let metadata = if path == "target.json" {
            &mut value["nativeDelivery"]
        } else {
            &mut value
        };
        metadata["resourceIdentities"]["ruleSetIdentity"]["rulesVersion"] =
            "untrusted-version".into();
        fs::write(
            output.join(path),
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();
    }
    reseal(&output);
    let receiver = scratch.0.join("Receiver");
    assert!(
        delivery::open_handoff(&output, &receiver, &DefinitionCatalog::builtin().unwrap()).is_err()
    );
    assert!(!receiver.exists());
    workspace.generation_snapshot().unwrap();
}

#[test]
fn builtin_generation_never_installs_inside_authoritative_source_tree() {
    let scratch = Scratch::new();
    let live = scratch.0.join("Live");
    let workspace = signals(&live);
    let before = workspace.generation_snapshot().unwrap();
    let output = live.join("generated");
    assert!(prepare(&workspace, false).preview(&output).is_err());
    assert!(prepare(&workspace, false).generate(&output).is_err());
    assert!(!output.exists());
    for input in before.inputs {
        assert_eq!(fs::read(input.disk_path).unwrap(), input.bytes);
    }
}

#[test]
fn builtin_late_source_edit_refuses_staged_commit_and_preserves_output() {
    let scratch = Scratch::new();
    let workspace = signals(&scratch.0.join("Live"));
    let output = scratch.0.join("package");
    prepare(&workspace, true).generate(&output).unwrap();
    let original = payload(&output);
    let prepared = prepare(&workspace, true);
    let preview = prepare(&workspace, true).preview(&output).unwrap();
    let staged = prepared
        .stage_previewed(&output, &preview.revision)
        .unwrap();
    let snapshot = workspace.generation_snapshot().unwrap();
    let input = &snapshot.inputs[0];
    let mut changed = input.bytes.clone();
    changed.extend_from_slice(b"\n<!-- external source edit -->\n");
    fs::write(&input.disk_path, &changed).unwrap();
    let error = staged.commit().unwrap_err();
    assert_eq!(
        serde_json::to_value(&error).unwrap(),
        serde_json::json!({
            "key": "backend.delivery.source_changed",
            "params": {"path": input.disk_path.display().to_string()},
        })
    );
    assert_eq!(payload(&output), original);
    assert_eq!(fs::read(&input.disk_path).unwrap(), changed);
}

#[test]
fn output_replaced_after_staging_refuses_confirmation_and_keeps_the_newer_output() {
    use crate::support::os_variants::{counter_variant, prepared};
    use autosar_config_core::target::BuildTarget;
    let scratch = Scratch::new();
    let target = BuildTarget::LinuxX64ControlledV1;
    let output = scratch.0.join("package");
    let first = || prepared(&counter_variant(false, 65535), &scratch.0, target);
    let preview = first().preview(&output).unwrap();
    let staged = first().stage_previewed(&output, &preview.revision).unwrap();
    // A valid generation of another configuration lands between staging and confirmation.
    prepared(&counter_variant(false, 4095), &scratch.0, target)
        .generate(&output)
        .unwrap();
    let newer = payload(&output);
    let error = staged.commit().unwrap_err();
    assert_eq!(message_key(&error), "backend.generation.confirmation_stale");
    assert_eq!(payload(&output), newer);
}

#[test]
fn builtin_ecu_delivers_project_license_and_preserves_kernel_notice() {
    use autosar_config_core::target::BuildTarget;

    let scratch = Scratch::new();
    let ecu = standard_with_application(&scratch.0.join("ECU"));
    let plan = ecu.saved_integration_plan().unwrap();
    for target in [
        BuildTarget::WindowsX64ControlledV1,
        BuildTarget::LinuxX64ControlledV1,
    ] {
        let output = scratch.0.join(format!("ecu-{}", target.spec().id));
        prepare_ecu_project_for_workspace(&ecu, &plan, target, true)
            .unwrap()
            .generate(&output)
            .unwrap();
        packages::assert_offline_build_readme(
            &output,
            target == BuildTarget::WindowsX64ControlledV1,
        );
        let text = fs::read_to_string(output.join("README.md")).unwrap();
        assert!(
            text.contains("The live project remains separate")
                && text.contains("not silently upgraded")
                && text.contains("tools/ecu-tool.py verify")
                && text.contains("--mode host-batch")
                && text.contains("Apache-2.0")
        );
        let names = fs::read_to_string(output.join("meta/files.list")).unwrap();
        let hashes = fs::read_to_string(output.join("meta/files.sha256")).unwrap();
        for (name, expected) in [
            ("LICENSE", include_bytes!("../../../LICENSE").as_slice()),
            ("NOTICE", include_bytes!("../../../NOTICE").as_slice()),
        ] {
            assert_eq!(fs::read(output.join(name)).unwrap(), expected);
            assert!(names.lines().any(|path| path == name));
            assert!(
                hashes
                    .lines()
                    .any(|line| { line == format!("{:x}  {name}", Sha256::digest(expected)) })
            );
        }
        assert_eq!(
            fs::read(output.join("kernel/LICENSE.md")).unwrap(),
            include_bytes!("../../../third_party/freertos/LICENSE.md")
        );
    }
}

#[test]
fn builtin_ecu_handoff_preserves_user_algorithm_in_independent_reconstructed_project() {
    let scratch = Scratch::new();
    let live = scratch.0.join("Live");
    let workspace = standard_with_application(&live);
    let application = &workspace.generation_snapshot().unwrap().applications[0];
    let original = String::from_utf8(application.bytes.clone()).unwrap();
    let changed = original
        .replace(
            "Data[3] = (uint8)committed;",
            "Data[3] = (uint8)(committed + 1u);",
        )
        .into_bytes();
    assert_ne!(changed, application.bytes);
    fs::write(&application.disk_path, &changed).unwrap();
    let workspace = Workspace::open_project_manifest(
        &live.join("workbench-project.json"),
        &scratch.0.join("cache"),
    )
    .unwrap();
    let plan = workspace.saved_integration_plan().unwrap();
    let output = scratch.0.join("package");
    prepare_ecu_project_for_workspace(&workspace, &plan, tooling::native_target(), true)
        .unwrap()
        .generate(&output)
        .unwrap();
    assert_eq!(
        fs::read(output.join("app/EchoApplication/EchoApplication.c")).unwrap(),
        changed
    );
    assert_eq!(fs::read(&application.disk_path).unwrap(), changed);
    let receiver = scratch.0.join("Receiver");
    let imported =
        delivery::open_handoff(&output, &receiver, &DefinitionCatalog::builtin().unwrap()).unwrap();
    let actual = imported.workspace.generation_snapshot().unwrap();
    assert_eq!(actual.applications[0].bytes, changed);
    assert!(
        fs::canonicalize(&actual.applications[0].disk_path)
            .unwrap()
            .starts_with(fs::canonicalize(&receiver).unwrap())
    );
    let receiver_plan = imported.workspace.saved_integration_plan().unwrap();
    let reproduced = scratch.0.join("reproduced");
    prepare_ecu_project_for_workspace(
        &imported.workspace,
        &receiver_plan,
        tooling::native_target(),
        true,
    )
    .unwrap()
    .generate(&reproduced)
    .unwrap();
    assert_eq!(payload(&output), payload(&reproduced));
}

#[test]
fn builtin_ecu_late_live_edit_and_resealed_snapshot_never_overwrite_user_bytes_or_prior_output() {
    let scratch = Scratch::new();
    let workspace = standard_with_application(&scratch.0.join("Live"));
    let plan = workspace.saved_integration_plan().unwrap();
    let output = scratch.0.join("package");
    prepare_ecu_project_for_workspace(&workspace, &plan, tooling::native_target(), true)
        .unwrap()
        .generate(&output)
        .unwrap();
    let original = payload(&output);
    let prepared =
        prepare_ecu_project_for_workspace(&workspace, &plan, tooling::native_target(), true)
            .unwrap();
    let preview =
        prepare_ecu_project_for_workspace(&workspace, &plan, tooling::native_target(), true)
            .unwrap()
            .preview(&output)
            .unwrap();
    let staged = prepared
        .stage_previewed(&output, &preview.revision)
        .unwrap();
    let snapshot = workspace.generation_snapshot().unwrap();
    let application = &snapshot.applications[0];
    let mut changed = application.bytes.clone();
    changed.extend_from_slice(b"\n/* external live user edit */\n");
    fs::write(&application.disk_path, &changed).unwrap();
    assert!(staged.commit().is_err());
    assert_eq!(payload(&output), original);
    assert_eq!(fs::read(&application.disk_path).unwrap(), changed);
    fs::write(
        output.join("app/EchoApplication/EchoApplication.c"),
        b"#error Altered_sealed_application_snapshot\n",
    )
    .unwrap();
    reseal(&output);
    let receiver = scratch.0.join("Receiver");
    assert!(
        delivery::open_handoff(&output, &receiver, &DefinitionCatalog::builtin().unwrap()).is_err()
    );
    assert!(!receiver.exists());
    assert_eq!(fs::read(&application.disk_path).unwrap(), changed);
}

#[test]
fn builtin_handoff_rejects_unknown_owner_and_ownership_version_without_publishing() {
    let scratch = Scratch::new();
    let workspace = signals(&scratch.0.join("Live"));
    for version in [false, true] {
        let output = scratch.0.join(if version { "version" } else { "owner" });
        prepare(&workspace, true).generate(&output).unwrap();
        let ledger_path = output.join("meta/workbench-ownership.json");
        let mut ledger: serde_json::Value =
            serde_json::from_slice(&fs::read(&ledger_path).unwrap()).unwrap();
        if version {
            ledger["formatVersion"] = 2.into();
        } else {
            ledger["files"][0]["owner"] = "unrecognized-authority".into();
        }
        fs::write(ledger_path, serde_json::to_vec_pretty(&ledger).unwrap()).unwrap();
        reseal(&output);
        let corrupted = payload(&output);
        let receiver = scratch.0.join(if version {
            "version-receiver"
        } else {
            "owner-receiver"
        });
        let error =
            delivery::open_handoff(&output, &receiver, &DefinitionCatalog::builtin().unwrap())
                .err()
                .unwrap();
        let evidence = serde_json::to_string(&error).unwrap();
        assert!(
            evidence.contains(if version {
                "backend.delivery.ownership_identity_invalid"
            } else {
                "unknown variant `unrecognized-authority`"
            }),
            "{evidence}"
        );
        assert!(!receiver.exists());
        assert_eq!(payload(&output), corrupted);
    }
}

#[test]
fn builtin_handoff_rejects_forged_declared_extension_identity_without_publishing() {
    let scratch = Scratch::new();
    let workspace = signals(&scratch.0.join("Live"));
    let output = scratch.0.join("package");
    prepare(&workspace, true).generate(&output).unwrap();
    let identity = serde_json::json!({
        "catalogId": "not-explicitly-accepted", "release": "R24-11",
        "version": "1.0.0", "sha256": "a".repeat(64),
    });
    for path in ["meta/handoff.json", "target.json"] {
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(output.join(path)).unwrap()).unwrap();
        let metadata = if path == "target.json" {
            &mut value["nativeDelivery"]
        } else {
            &mut value
        };
        metadata["resourceIdentities"]["requiredExtensionDefinitions"] =
            serde_json::json!([identity]);
        fs::write(
            output.join(path),
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();
    }
    reseal(&output);
    let receiver = scratch.0.join("Receiver");
    let before = workspace.generation_snapshot().unwrap();
    assert!(
        delivery::open_handoff(&output, &receiver, &DefinitionCatalog::builtin().unwrap()).is_err()
    );
    assert!(!receiver.exists());
    for input in before.inputs {
        assert_eq!(fs::read(input.disk_path).unwrap(), input.bytes);
    }
}

fn edit_standard_configuration(root: &Path, edit: impl FnOnce(&str) -> String) -> Workspace {
    let workspace = standard_named(root, "Qualified");
    workspace.saved_integration_plan().unwrap();
    let path = root.join("ecuc.arxml");
    let original = fs::read_to_string(&path).unwrap();
    let changed = edit(&original);
    assert_ne!(changed, original);
    fs::write(&path, &changed).unwrap();
    drop(workspace);
    Workspace::open_project_manifest(&root.join("workbench-project.json"), &root.join("cache"))
        .unwrap()
}

fn assert_native_generation_rejected(workspace: &Workspace, output: &Path) {
    let before = workspace.generation_snapshot().unwrap();
    let prior_output = output.exists().then(|| payload(output));
    let rejected = match workspace.saved_integration_plan() {
        Err(_) => true,
        Ok(plan) => match autosar_config_core::prepared::prepare_ecu_project_for_workspace(
            workspace,
            &plan,
            tooling::native_target(),
            true,
        ) {
            Err(_) => true,
            Ok(prepared) => prepared.generate(output).is_err(),
        },
    };
    assert!(
        rejected,
        "Unqualified target consumers must not produce native sources"
    );
    if let Some(prior) = prior_output {
        assert_eq!(payload(output), prior);
    } else {
        assert!(!output.exists());
    }
    for input in before.inputs {
        assert_eq!(fs::read(input.disk_path).unwrap(), input.bytes);
    }
}

fn ecuc_module<'a, 'input>(
    document: &'a roxmltree::Document<'input>,
    name: &str,
) -> roxmltree::Node<'a, 'input> {
    document
        .descendants()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
                && node.children().any(|child| {
                    child.tag_name().name() == "SHORT-NAME" && child.text() == Some(name)
                })
        })
        .unwrap()
}

#[test]
fn builtin_ecu_generation_rejects_unknown_container_inside_consumed_com_module() {
    let scratch = Scratch::new();
    let workspace = edit_standard_configuration(&scratch.0.join("Live"), |original| {
        let document = roxmltree::Document::parse(original).unwrap();
        let module = ecuc_module(&document, "Com");
        let containers = module
            .children()
            .find(|node| node.is_element() && node.tag_name().name() == "CONTAINERS")
            .unwrap();
        let end = containers.range().end - "</CONTAINERS>".len();
        let mut changed = original.to_owned();
        changed.insert_str(end, "<ECUC-CONTAINER-VALUE><SHORT-NAME>UnqualifiedConsumer</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">/AUTOSAR/EcucDefs/Com/UnmodeledConsumer</DEFINITION-REF></ECUC-CONTAINER-VALUE>");
        changed
    });
    assert_native_generation_rejected(&workspace, &scratch.0.join("output"));
}

#[test]
fn builtin_ecu_generation_qualifies_referenced_clock_semantics_and_module_edition() {
    let scratch = Scratch::new();
    for unsupported_edition in [false, true] {
        let root = scratch.0.join(if unsupported_edition {
            "edition"
        } else {
            "clock"
        });
        let workspace = edit_standard_configuration(&root, |original| {
            let document = roxmltree::Document::parse(original).unwrap();
            let module = ecuc_module(&document, "Mcu");
            let mut changed = original.to_owned();
            if unsupported_edition {
                let edition = module
                    .children()
                    .find(|node| node.is_element() && node.tag_name().name() == "ECUC-DEF-EDITION")
                    .unwrap();
                changed.replace_range(
                    edition.range(),
                    "<ECUC-DEF-EDITION>4.9.0</ECUC-DEF-EDITION>",
                );
            } else {
                let cpu_clock = document
                    .descendants()
                    .find(|node| {
                        node.is_element()
                            && node.tag_name().name() == "ECUC-REFERENCE-VALUE"
                            && node.children().any(|child| {
                                child.tag_name().name() == "DEFINITION-REF"
                                    && child
                                        .text()
                                        .is_some_and(|value| value.ends_with("/CanCpuClockRef"))
                            })
                    })
                    .unwrap();
                let path = cpu_clock
                    .children()
                    .find(|node| node.is_element() && node.tag_name().name() == "VALUE-REF")
                    .unwrap()
                    .text()
                    .unwrap();
                let reference = module
                    .descendants()
                    .find(|node| {
                        node.is_element()
                            && node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                            && node.children().any(|child| {
                                child.tag_name().name() == "SHORT-NAME"
                                    && child.text() == path.rsplit('/').next()
                            })
                    })
                    .unwrap();
                let parameters = reference
                    .children()
                    .find(|node| node.is_element() && node.tag_name().name() == "PARAMETER-VALUES")
                    .unwrap();
                let end = parameters.range().end - "</PARAMETER-VALUES>".len();
                changed.insert_str(end, "<ECUC-NUMERICAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-INTEGER-PARAM-DEF\">/AUTOSAR/EcucDefs/Mcu/McuModuleConfiguration/McuClockSettingConfig/McuClockReferencePoint/UnknownClockPolicy</DEFINITION-REF><VALUE>0</VALUE></ECUC-NUMERICAL-PARAM-VALUE>");
            }
            changed
        });
        assert_native_generation_rejected(
            &workspace,
            &scratch.0.join(if unsupported_edition {
                "edition-output"
            } else {
                "clock-output"
            }),
        );
    }
}

#[test]
fn builtin_ecu_generation_preserves_unrelated_retained_module_without_blocking_target() {
    let scratch = Scratch::new();
    let workspace = edit_standard_configuration(&scratch.0.join("Live"), |original| {
        let document = roxmltree::Document::parse(original).unwrap();
        let module = ecuc_module(&document, "Mcu");
        let name = module
            .children()
            .find(|node| node.is_element() && node.tag_name().name() == "SHORT-NAME")
            .unwrap();
        let edition = module
            .children()
            .find(|node| node.is_element() && node.tag_name().name() == "ECUC-DEF-EDITION")
            .unwrap();
        let range = module.range();
        let mut retained = original[range.clone()].to_owned();
        for (span, value) in [
            (
                edition.range(),
                "<ECUC-DEF-EDITION>4.9.0</ECUC-DEF-EDITION>",
            ),
            (name.range(), "<SHORT-NAME>RetainedMcu</SHORT-NAME>"),
        ] {
            retained.replace_range(span.start - range.start..span.end - range.start, value);
        }
        let mut changed = original.to_owned();
        changed.insert_str(range.end, &retained);
        changed
    });
    let before = workspace.generation_snapshot().unwrap();
    let plan = workspace.saved_integration_plan().unwrap();
    let output = scratch.0.join("output");
    autosar_config_core::prepared::prepare_ecu_project_for_workspace(
        &workspace,
        &plan,
        tooling::native_target(),
        true,
    )
    .unwrap()
    .generate(&output)
    .unwrap();
    let imported = delivery::open_handoff(
        &output,
        &scratch.0.join("Receiver"),
        &DefinitionCatalog::builtin().unwrap(),
    )
    .unwrap();
    let actual = imported.workspace.generation_snapshot().unwrap();
    let expected: BTreeMap<_, _> = before
        .inputs
        .iter()
        .map(|input| (&input.logical_path, &input.bytes))
        .collect();
    let received: BTreeMap<_, _> = actual
        .inputs
        .iter()
        .map(|input| (&input.logical_path, &input.bytes))
        .collect();
    assert_eq!(received, expected);
    for input in before.inputs {
        assert_eq!(fs::read(input.disk_path).unwrap(), input.bytes);
    }
}

fn reject_core_change_with_prior_output(edit: impl FnOnce(&str) -> String) {
    let scratch = Scratch::new();
    let root = scratch.0.join("Live");
    let workspace = standard_named(&root, "CorePolicy");
    let plan = workspace.saved_integration_plan().unwrap();
    let output = scratch.0.join("output");
    autosar_config_core::prepared::prepare_ecu_project_for_workspace(
        &workspace,
        &plan,
        tooling::native_target(),
        true,
    )
    .unwrap()
    .generate(&output)
    .unwrap();
    let path = root.join("ecuc.arxml");
    let original = fs::read_to_string(&path).unwrap();
    let changed = edit(&original);
    assert_ne!(changed, original);
    fs::write(&path, &changed).unwrap();
    drop(workspace);
    let workspace = Workspace::open_project_manifest(
        &root.join("workbench-project.json"),
        &scratch.0.join("cache"),
    )
    .unwrap();
    assert_native_generation_rejected(&workspace, &output);
    assert_eq!(fs::read_to_string(path).unwrap(), changed);
}

fn core_id_parameter<'a, 'input>(
    document: &'a roxmltree::Document<'input>,
) -> roxmltree::Node<'a, 'input> {
    document
        .descendants()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "ECUC-NUMERICAL-PARAM-VALUE"
                && node.children().any(|child| {
                    child.tag_name().name() == "DEFINITION-REF"
                        && child
                            .text()
                            .is_some_and(|value| value.ends_with("/EcucCoreId"))
                })
        })
        .unwrap()
}

#[test]
fn builtin_ecu_generation_rejects_definition_valid_nonzero_core_without_replacing_output() {
    reject_core_change_with_prior_output(|original| {
        let document = roxmltree::Document::parse(original).unwrap();
        let parameter = core_id_parameter(&document);
        let definition = parameter
            .children()
            .find(|node| node.is_element() && node.tag_name().name() == "DEFINITION-REF")
            .unwrap()
            .text()
            .unwrap();
        let catalog = DefinitionCatalog::builtin().unwrap();
        let kind = catalog.get(definition).unwrap().kind.unwrap();
        catalog
            .validate_value(
                definition,
                &autosar_config_core::project_model::TypedValue {
                    kind,
                    lexeme: "1".into(),
                },
            )
            .unwrap();
        let value = parameter
            .children()
            .find(|node| node.is_element() && node.tag_name().name() == "VALUE")
            .unwrap();
        let mut changed = original.to_owned();
        changed.replace_range(value.range(), "<VALUE>1</VALUE>");
        changed
    });
}

#[test]
fn builtin_ecu_generation_rejects_second_core_zero_without_replacing_output() {
    reject_core_change_with_prior_output(|original| {
        let document = roxmltree::Document::parse(original).unwrap();
        let parameter = core_id_parameter(&document);
        let core = parameter
            .ancestors()
            .find(|node| node.is_element() && node.tag_name().name() == "ECUC-CONTAINER-VALUE")
            .unwrap();
        let name = core
            .children()
            .find(|node| node.is_element() && node.tag_name().name() == "SHORT-NAME")
            .unwrap();
        let range = core.range();
        let mut second = original[range.clone()].to_owned();
        second.replace_range(
            name.range().start - range.start..name.range().end - range.start,
            "<SHORT-NAME>SecondCoreZero</SHORT-NAME>",
        );
        let mut changed = original.to_owned();
        changed.insert_str(range.end, &second);
        changed
    });
}

#[test]
fn generation_refusal_serializes_stable_key_and_verbatim_owner_path() {
    let scratch = Scratch::new();
    let workspace = signals(&scratch.0.join("Live"));
    let output = scratch.0.join("package");
    prepare(&workspace, true).generate(&output).unwrap();
    let original = payload(&output);
    let owner_name = "用户 {{name}}.txt";
    fs::write(output.join(owner_name), b"owner bytes").unwrap();
    let error = prepare(&workspace, true).preview(&output).unwrap_err();
    assert_eq!(
        serde_json::to_value(error).unwrap(),
        serde_json::json!({
            "key": "backend.generation.output_user_file_refused",
            "params": {"path": owner_name},
        })
    );
    assert_eq!(payload(&output), original);
    assert_eq!(fs::read(output.join(owner_name)).unwrap(), b"owner bytes");
}

#[test]
fn prepared_source_change_keeps_localized_identity_and_existing_payload() {
    let scratch = Scratch::new();
    let workspace = signals(&scratch.0.join("Live"));
    let output = scratch.0.join("package");
    prepare(&workspace, true).generate(&output).unwrap();
    let original = payload(&output);
    let source = workspace.generation_snapshot().unwrap().inputs[0]
        .disk_path
        .clone();
    let prepared = prepare(&workspace, true);
    let mut changed = fs::read(&source).unwrap();
    changed.extend_from_slice(b"\n<!-- external source evidence -->\n");
    fs::write(&source, &changed).unwrap();
    let error = prepared.generate(&output).unwrap_err();
    assert_eq!(
        serde_json::to_value(error).unwrap(),
        serde_json::json!({
            "key": "backend.delivery.source_changed",
            "params": {"path": source.display().to_string()},
        })
    );
    assert_eq!(payload(&output), original);
    assert_eq!(fs::read(source).unwrap(), changed);
}

#[test]
fn source_only_import_refusal_keeps_localized_contract_and_package_bytes() {
    let scratch = Scratch::new();
    let workspace = signals(&scratch.0.join("Live"));
    let output = scratch.0.join("package");
    prepare(&workspace, false).generate(&output).unwrap();
    let original = payload(&output);
    let receiver = scratch.0.join("Receiver");
    let error =
        match delivery::open_handoff(&output, &receiver, &DefinitionCatalog::builtin().unwrap()) {
            Err(error) => error,
            Ok(_) => panic!("Source-only output must not be accepted as an explicit handoff"),
        };
    assert_eq!(
        serde_json::to_value(error).unwrap(),
        serde_json::json!({
            "key": "backend.delivery.explicit_handoff_required",
            "params": {},
        })
    );
    assert!(!receiver.exists());
    assert_eq!(payload(&output), original);
}

#[test]
fn product_message_consumption_does_not_change_preparation_or_fixed_readme_bytes() {
    let scratch = Scratch::new();
    let workspace = signals(&scratch.0.join("Live"));
    let before = prepare(&workspace, true);
    let fingerprint = before.fingerprint().to_owned();
    let bytes = before.into_files();
    let readme = &bytes
        .iter()
        .find(|(path, _)| path == "README.md")
        .unwrap()
        .1;
    assert!(std::str::from_utf8(readme).unwrap().contains(
        "This package uses product-authored R24-11 rules, not official XSD/MOD certification."
    ));
    let refused_output = scratch.0.join("owned-output");
    fs::create_dir(&refused_output).unwrap();
    fs::write(refused_output.join("用户.txt"), b"owner bytes").unwrap();
    let error = prepare(&workspace, true)
        .preview(&refused_output)
        .unwrap_err();
    let descriptor = serde_json::to_value(error).unwrap();
    assert_eq!(
        descriptor["key"],
        "backend.generation.output_inventory_missing"
    );
    let after = prepare(&workspace, true);
    assert_eq!(after.fingerprint(), fingerprint);
    assert_eq!(after.into_files(), bytes);
}

#[test]
fn preflight_logs_keep_structured_envelopes_and_verbatim_external_evidence() {
    use autosar_config_core::LocalizedText;
    use autosar_config_core::prepared::{PreflightReport, PreflightStatus};
    let evidence = "工具 {{key}}: ERROR C123\r\n原始 stderr\n";
    let report = PreflightReport {
        status: PreflightStatus::Failed,
        fingerprint: "unchanged-machine-identity".into(),
        logs: vec![
            autosar_config_core::product_message!(
                "backend.prepared.preflight_capture_directory",
                "path" => "D:/用户/{{tool}}",
            ),
            LocalizedText::from(evidence),
        ],
    };
    let serialized = serde_json::to_value(report).unwrap();
    assert_eq!(serialized["fingerprint"], "unchanged-machine-identity");
    assert_eq!(
        serialized["logs"][0],
        serde_json::json!({
            "key": "backend.prepared.preflight_capture_directory",
            "params": {"path": "D:/用户/{{tool}}"},
        })
    );
    assert_eq!(serialized["logs"][1], evidence);
}

fn message_key(error: &autosar_config_core::LocalizedText) -> &str {
    match error {
        autosar_config_core::LocalizedText::Message(message) => &message.key,
        other => panic!("{other:?}"),
    }
}

#[test]
fn retired_project_identities_are_refused_without_loading_or_rewriting() {
    let scratch = Scratch::new();
    let live = scratch.0.join("Live");
    drop(standard_named(&live, "Retired"));
    let manifest_path = live.join("workbench-project.json");
    let current = fs::read(&manifest_path).unwrap();
    let current_value: serde_json::Value = serde_json::from_slice(&current).unwrap();
    assert_eq!(
        current_value["profileHint"],
        autosar_config_core::integration::PROFILE
    );
    let slot = current_value["applicationInputs"][0]["producerSlot"]
        .as_str()
        .unwrap()
        .to_owned();
    let instance = slot.split_once(':').unwrap().1.to_owned();
    let mut cases = Vec::new();
    for profile in [
        "host-can-v1",
        "epic4-win64-sr-cs-v1",
        "singlecore-multi-swc-v1",
    ] {
        let mut value = current_value.clone();
        value["profileHint"] = profile.into();
        cases.push((profile, value));
    }
    for (profile, retired_slot) in [
        (
            "epic4-win64-sr-cs-v1",
            "epic4-single-application-v1".to_owned(),
        ),
        (
            "singlecore-multi-swc-v1",
            format!("singlecore-multi-swc-v1:{instance}"),
        ),
    ] {
        let mut value = current_value.clone();
        value["applicationInputs"][0]["producerSlot"] = retired_slot.into();
        cases.push((profile, value));
    }
    for (profile, value) in cases {
        let bytes = serde_json::to_vec_pretty(&value).unwrap();
        fs::write(&manifest_path, &bytes).unwrap();
        let error = Workspace::open_project_manifest(&manifest_path, &scratch.0.join("cache"))
            .err()
            .unwrap();
        assert_eq!(message_key(&error), "backend.arxml.project.retired_profile");
        let autosar_config_core::LocalizedText::Message(message) = &error else {
            unreachable!()
        };
        assert_eq!(message.params["profile"], profile);
        assert_eq!(fs::read(&manifest_path).unwrap(), bytes);
    }
    for version in [0, 2] {
        let mut value = current_value.clone();
        value["formatVersion"] = version.into();
        let bytes = serde_json::to_vec_pretty(&value).unwrap();
        fs::write(&manifest_path, &bytes).unwrap();
        let error = Workspace::open_project_manifest(&manifest_path, &scratch.0.join("cache"))
            .err()
            .unwrap();
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            serde_json::json!({
                "key": "backend.arxml.project.format_version_unsupported",
                "params": {"version": version.to_string()},
            })
        );
        assert_eq!(fs::read(&manifest_path).unwrap(), bytes);
    }
    fs::write(&manifest_path, &current).unwrap();
    Workspace::open_project_manifest(&manifest_path, &scratch.0.join("cache")).unwrap();
}

#[test]
fn retired_delivery_profiles_are_refused_before_import() {
    let scratch = Scratch::new();
    let workspace = signals(&scratch.0.join("Live"));
    let output = scratch.0.join("package");
    prepare(&workspace, true).generate(&output).unwrap();
    for path in [
        "meta/handoff.json",
        "target.json",
        "meta/workbench-ownership.json",
    ] {
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(output.join(path)).unwrap()).unwrap();
        let metadata = if path == "target.json" {
            &mut value["nativeDelivery"]
        } else {
            &mut value
        };
        metadata["profileId"] = "singlecore-multi-swc-v1".into();
        fs::write(
            output.join(path),
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();
    }
    reseal(&output);
    assert_retired_package(
        &output,
        &scratch.0.join("Receiver"),
        "singlecore-multi-swc-v1",
    );

    // A frame/signal host package is recognized by its target identity.
    let output = scratch.0.join("host-package");
    prepare(&workspace, true).generate(&output).unwrap();
    let mut target: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("target.json")).unwrap()).unwrap();
    target["profile"] = "host".into();
    fs::write(
        output.join("target.json"),
        serde_json::to_vec_pretty(&target).unwrap(),
    )
    .unwrap();
    reseal(&output);
    assert_retired_package(&output, &scratch.0.join("HostReceiver"), "host-can-v1");
}

#[test]
fn retired_and_unknown_handoff_formats_are_refused_by_identity_without_rewriting() {
    let scratch = Scratch::new();
    let workspace = signals(&scratch.0.join("Live"));
    let refuse = |package: &Path, receiver: &Path| {
        let before = package_tree(package);
        let error =
            delivery::open_handoff(package, receiver, &DefinitionCatalog::builtin().unwrap())
                .err()
                .unwrap();
        assert_eq!(
            delivery::require_current_format(package).unwrap_err(),
            error
        );
        assert!(!receiver.exists());
        assert_eq!(package_tree(package), before);
        error
    };
    // Current-layout packages declaring an earlier workbench format, and the
    // retired root-level handoff.json layout of the host and single-component chains.
    for (format, root_layout) in [
        ("autosar-workbench-handoff-v2", false),
        ("autosar-workbench-handoff-v3", false),
        ("autosar-host-handoff-v1", true),
        ("autosar-ecu-handoff-v1", true),
    ] {
        let package = scratch.0.join(format);
        if root_layout {
            fs::create_dir(&package).unwrap();
            fs::write(package.join("files.list"), b"handoff.json\n").unwrap();
            fs::write(
                package.join("handoff.json"),
                serde_json::to_vec(&serde_json::json!({"format": format})).unwrap(),
            )
            .unwrap();
        } else {
            prepare(&workspace, true).generate(&package).unwrap();
            let path = package.join("meta/handoff.json");
            let mut metadata: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            metadata["format"] = format.into();
            fs::write(&path, serde_json::to_vec_pretty(&metadata).unwrap()).unwrap();
        }
        let error = refuse(&package, &scratch.0.join(format!("{format}-receiver")));
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            serde_json::json!({
                "key": "backend.project.retired_handoff_format",
                "params": {"format": format},
            })
        );
    }
    let package = scratch.0.join("unknown");
    prepare(&workspace, true).generate(&package).unwrap();
    let path = package.join("meta/handoff.json");
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    metadata["format"] = "autosar-workbench-handoff-v99".into();
    fs::write(&path, serde_json::to_vec_pretty(&metadata).unwrap()).unwrap();
    let error = refuse(&package, &scratch.0.join("unknown-receiver"));
    assert_eq!(
        message_key(&error),
        "backend.project.handoff_format_unsupported"
    );
}

fn assert_retired_package(output: &Path, receiver: &Path, profile: &str) {
    let before = payload(output);
    let error = delivery::open_handoff(output, receiver, &DefinitionCatalog::builtin().unwrap())
        .err()
        .unwrap();
    assert_eq!(message_key(&error), "backend.delivery.retired_profile");
    let autosar_config_core::LocalizedText::Message(message) = &error else {
        unreachable!()
    };
    assert_eq!(message.params["profile"], profile);
    assert!(!receiver.exists());
    assert_eq!(payload(output), before);
}

#[test]
fn runtime_source_inventory_has_no_manual_digests_and_delivery_has_computed_digests() {
    use autosar_config_core::resources::AssetInventory;
    use sha2::{Digest, Sha256};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let inventory = AssetInventory::from_directory(root).unwrap();
    let source: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("runtime/contracts/assets.json")).unwrap())
            .unwrap();
    let delivered: serde_json::Value = serde_json::from_slice(
        inventory
            .get("runtime/contracts/assets.json")
            .unwrap()
            .bytes,
    )
    .unwrap();
    let source_rows = source["assets"].as_array().unwrap();
    let delivered_rows = delivered["assets"].as_array().unwrap();
    assert_eq!(source_rows.len(), delivered_rows.len());
    for (source, delivered) in source_rows.iter().zip(delivered_rows) {
        assert!(source.get("sha256").is_none());
        let path = source["path"].as_str().unwrap();
        let bytes = fs::read(root.join(path)).unwrap();
        let expected = format!("{:x}", Sha256::digest(&bytes));
        assert_eq!(delivered["sha256"], expected, "{path}");
        let asset = inventory.get(path).unwrap();
        assert_eq!(asset.sha256, expected, "{path}");
        assert_eq!(asset.bytes, bytes, "{path}");
        let mut metadata = delivered.clone();
        metadata.as_object_mut().unwrap().remove("sha256");
        assert_eq!(&metadata, source, "{path}");
    }
}
