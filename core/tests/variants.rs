//! The project manifest and native delivery are the entry points for layered inputs.
#![allow(dead_code)]

#[path = "support/mod.rs"]
mod support;
use crate::support::generated_packages::{prepare, standard_named};
use autosar_config_core::Workspace;
use autosar_config_core::arxml::*;
use autosar_config_core::project_model::{TypedValue, ValueKind};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use support::{tooling, workspace::Scratch};

const PRIORITY: &str = "/AUTOSAR/EcucDefs/Os/OsTask/OsTaskPriority";

fn task(field: &str) -> LayerTarget {
    LayerTarget {
        object: LayerObject {
            release: "R24-11".into(),
            definition_type: "/AUTOSAR/EcucDefs/Os/OsTask".into(),
            path: "/Configuration/Os/Task_Ecu".into(),
            instance_context: Vec::new(),
        },
        field: Some(field.into()),
    }
}

fn priority(value: &str) -> LayerOperation {
    LayerOperation {
        target: task(PRIORITY),
        action: LayerAction::Set {
            values: vec![TypedValue {
                kind: ValueKind::Integer,
                lexeme: value.into(),
            }],
            default_origin: None,
        },
    }
}

fn event_delete() -> LayerOperation {
    LayerOperation {
        target: LayerTarget {
            object: LayerObject {
                release: "R24-11".into(),
                definition_type: "/AUTOSAR/EcucDefs/Os/OsEvent".into(),
                path: "/Configuration/Os/Ev_Work".into(),
                instance_context: Vec::new(),
            },
            field: None,
        },
        action: LayerAction::Delete,
    }
}

fn write(path: &Path, manifest: &ProjectManifest) {
    fs::write(
        path.join("workbench-project.json"),
        serde_json::to_vec_pretty(manifest).unwrap(),
    )
    .unwrap();
}

fn open(path: &Path) -> Workspace {
    Workspace::open_project_manifest(&path.join("workbench-project.json"), &path.join("cache"))
        .unwrap()
}

fn fixture(path: &Path, priority_value: &str) -> Workspace {
    let old = standard_named(path, "Shared");
    let mut manifest = old.project_manifest().unwrap().clone();
    let file = path.join("ecuc.arxml");
    let xml = fs::read_to_string(&file).unwrap();
    fs::write(
        file,
        replace_priority(&xml, priority_value).replace(
            "<AR-PACKAGES>",
            "<AR-PACKAGES><!-- preserve across layer edits -->",
        ),
    )
    .unwrap();
    let members = manifest
        .inputs
        .iter()
        .map(|input| input.path.clone())
        .collect::<Vec<_>>();
    let base = ProjectBase {
        reference: BaseReference {
            base_id: "shared-reference".into(),
            revision: String::from("first"),
            source_sha256: base_source_digest(
                members
                    .iter()
                    .map(|name| (name, fs::read(path.join(name)).unwrap())),
            ),
        },
        release: "R24-11".into(),
        rules: autosar_config_core::rules::rule_set_identity().unwrap(),
        extensions: Vec::new(),
        members,
        migrations: Vec::new(),
    };
    manifest.layers = Some(ProjectLayers {
        base_reference: base.reference.clone(),
        base,
        project: ProjectLayer::default(),
        groups: BTreeMap::from([(
            "scheduler".into(),
            VariantGroup {
                exactly_one: true,
                options: BTreeMap::from([
                    ("platform".into(), ProjectLayer::default()),
                    (
                        "urgent".into(),
                        ProjectLayer {
                            members: Vec::new(),
                            operations: vec![priority("8")],
                        },
                    ),
                ]),
            },
        )]),
        selection: selection("platform"),
        conflicts: Vec::new(),
    });
    write(path, &manifest);
    open(path)
}

fn selection(option: &str) -> BTreeMap<String, BTreeSet<String>> {
    BTreeMap::from([("scheduler".into(), BTreeSet::from([option.into()]))])
}

fn replace_priority(xml: &str, value: &str) -> String {
    let start = xml.find(PRIORITY).unwrap();
    let start = start + xml[start..].find("<VALUE>").unwrap() + "<VALUE>".len();
    let end = start + xml[start..].find("</VALUE>").unwrap();
    format!("{}{value}{}", &xml[..start], &xml[end..])
}

fn upgrade(path: &Path, workspace: &Workspace, value: &str) -> ProjectBase {
    let previous = &workspace
        .project_manifest()
        .unwrap()
        .layers
        .as_ref()
        .unwrap()
        .base;
    let mut incoming = previous.clone();
    incoming.reference.revision = "next".into();
    fs::create_dir(path.join("next")).unwrap();
    incoming.members = previous
        .members
        .iter()
        .map(|member| {
            let next = format!("next/{member}");
            let mut bytes = fs::read(path.join(member)).unwrap();
            if member == "ecuc.arxml" {
                bytes = replace_priority(std::str::from_utf8(&bytes).unwrap(), value).into_bytes();
            }
            fs::write(path.join(&next), bytes).unwrap();
            next
        })
        .collect();
    incoming.reference.source_sha256 = base_source_digest(
        incoming
            .members
            .iter()
            .map(|member| (member, fs::read(path.join(member)).unwrap())),
    );
    incoming
}

#[test]
fn selection_invalidates_save_and_generation_and_survives_offline_reopen() {
    let scratch = Scratch::new();
    let project = scratch.0.join("live");
    let mut workspace = fixture(&project, "1");
    let original = fs::read(project.join("ecuc.arxml")).unwrap();
    let plan = workspace.saved_integration_plan().unwrap();
    let old_preview = workspace.preview_save().unwrap();
    let package = scratch.0.join("moved-handoff");
    let old_delivery = prepare(&workspace, true).preview(&package).unwrap();

    let report = workspace
        .edit_integration(autosar_config_core::integration::IntegrationEdit {
            variant_selection: Some(selection("urgent")),
            ..Default::default()
        })
        .unwrap();
    assert!(
        report
            .layers
            .unwrap()
            .differences
            .iter()
            .any(|row| row.field.as_deref() == Some(PRIORITY))
    );
    assert!(workspace.save_previewed(&old_preview.revision).is_err());
    workspace.save().unwrap();
    assert!(
        autosar_config_core::prepared::prepare_ecu_project_for_workspace(
            &workspace,
            &plan,
            tooling::native_target(),
            true,
        )
        .is_err()
    );
    assert_eq!(fs::read(project.join("ecuc.arxml")).unwrap(), original);

    assert!(
        prepare(&workspace, true)
            .generate_previewed(&package, &old_delivery.revision)
            .is_err()
    );
    let workspace = open(&project);
    let application = workspace.generation_snapshot().unwrap().applications;
    prepare(&workspace, true).generate(&package).unwrap();
    #[cfg(feature = "native-tests")]
    {
        let result = tooling::run_public_command(
            &mut tooling::ecu_production_build_command(
                &package,
                &scratch.0.join("variant-build"),
                "host-batch",
                None,
            ),
            &scratch.0,
            "variant-build",
            std::time::Duration::from_secs(180),
        );
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
    fs::remove_dir_all(&project).unwrap();
    let receiver = scratch.0.join("receiver");
    let reopened = autosar_config_core::generator::delivery::open_handoff(
        &package,
        &receiver,
        &autosar_config_core::definitions::DefinitionCatalog::builtin().unwrap(),
    )
    .unwrap()
    .workspace;
    assert_eq!(
        reopened
            .project_manifest()
            .unwrap()
            .layers
            .as_ref()
            .unwrap()
            .selection,
        selection("urgent")
    );
    for source in application {
        assert_eq!(
            fs::read(receiver.join(source.logical_path)).unwrap(),
            source.bytes
        );
    }
    assert_eq!(fs::read(receiver.join("ecuc.arxml")).unwrap(), original);
}

#[test]
fn incomplete_selection_saves_but_does_not_generate() {
    let scratch = Scratch::new();
    let project = scratch.0.join("live");
    let workspace = fixture(&project, "1");
    let mut broken = workspace.project_manifest().unwrap().clone();
    let signal = workspace
        .saved_integration_plan()
        .unwrap()
        .description()
        .signals
        .iter()
        .find(|s| !s.receive)
        .unwrap()
        .clone();
    let mut op = priority("1024");
    op.target.object.path = signal.can_if_pdu;
    op.target.object.definition_type = "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfTxPduCfg".into();
    op.target.field = Some(format!(
        "{}/CanIfTxPduCanId",
        op.target.object.definition_type
    ));
    broken
        .layers
        .as_mut()
        .unwrap()
        .groups
        .get_mut("scheduler")
        .unwrap()
        .options
        .insert(
            "unpaired".into(),
            ProjectLayer {
                members: vec![],
                operations: vec![op],
            },
        );
    write(&project, &broken);
    let mut workspace = open(&project);
    let unpaired = selection("unpaired");
    let errors = workspace
        .preview_variant_selection(unpaired.clone())
        .unwrap_err();
    assert!(errors.iter().any(|issue| issue.code == "CAN_ID_CONFLICT"));
    assert_eq!(
        errors[0].code,
        workspace
            .edit_integration(autosar_config_core::integration::IntegrationEdit {
                variant_selection: Some(unpaired),
                ..Default::default()
            })
            .unwrap_err()[0]
            .code
    );
    let invalid = selection("missing");
    assert_eq!(
        workspace
            .preview_variant_selection(invalid.clone())
            .unwrap_err()[0]
            .code,
        workspace
            .edit_integration(autosar_config_core::integration::IntegrationEdit {
                variant_selection: Some(invalid),
                ..Default::default()
            })
            .unwrap_err()[0]
            .code
    );
    workspace.select_project_variants(BTreeMap::new()).unwrap();
    assert_eq!(
        workspace
            .preview_variant_selection(selection("urgent"))
            .unwrap_err()[0]
            .code,
        "VARIANT_BASELINE_UNAVAILABLE"
    );
    workspace
        .edit_integration(autosar_config_core::integration::IntegrationEdit {
            variant_selection: Some(selection("urgent")),
            ..Default::default()
        })
        .unwrap();
    for (selection, code) in [
        (BTreeMap::new(), "VARIANT_SELECTION_REQUIRED"),
        (
            BTreeMap::from([(
                "scheduler".into(),
                BTreeSet::from(["urgent".into(), "platform".into()]),
            )]),
            "VARIANT_SELECTION_REQUIRED",
        ),
        (selection("missing"), "VARIANT_OPTION_UNKNOWN"),
    ] {
        workspace.select_project_variants(selection).unwrap();
        workspace.save().unwrap();
        let issues = open(&project).saved_integration_plan().err().unwrap();
        assert!(
            issues.iter().any(|issue| issue.code == code
                && issue.object.as_deref().unwrap().starts_with("scheduler"))
        );
    }
}

#[test]
fn setting_an_equal_value_is_a_source_and_state_change_and_removal_inherits() {
    let scratch = Scratch::new();
    let mut workspace = fixture(&scratch.0.join("live"), "1");
    let base = workspace.clone();
    workspace
        .replace_layer_operations(None, vec![priority("1")])
        .unwrap();
    let diff = base.compare_project(&workspace).unwrap();
    assert_eq!(diff.sources.len(), 1);
    assert_eq!(diff.sources[0].path, "workbench-project.json");
    let priority_difference = diff
        .semantics
        .iter()
        .find(|row| row.target == task(PRIORITY))
        .unwrap();
    assert_eq!(
        priority_difference.changes,
        [SemanticChange::Origin, SemanticChange::State]
    );
    workspace
        .replace_layer_operations(None, Vec::new())
        .unwrap();
    assert!(
        base.compare_project(&workspace)
            .unwrap()
            .semantics
            .is_empty()
    );
}

#[test]
fn equal_integer_lexemes_change_source_only() {
    let scratch = Scratch::new();
    let left = fixture(&scratch.0.join("left"), "16");
    let right = fixture(&scratch.0.join("right"), "0x10");
    let diff = left.compare_project(&right).unwrap();
    assert!(diff.sources.iter().any(|row| row.path == "ecuc.arxml"));
    assert!(diff.semantics.is_empty(), "{:?}", diff.semantics);
    let left = prepare(&left, false);
    let right = prepare(&right, false);
    let artifacts = compare_project_artifacts(&left, &right).unwrap();
    assert!(
        artifacts
            .iter()
            .any(|row| row.path == "target.json" && row.change == ArtifactChange::Identity)
    );
    assert!(
        artifacts
            .iter()
            .all(|row| row.change == ArtifactChange::Identity),
        "{artifacts:?}"
    );
}

#[test]
fn deleting_a_referenced_object_reports_its_surviving_inbound_reference() {
    let scratch = Scratch::new();
    let path = scratch.0.join("live");
    let mut workspace = fixture(&path, "1");
    let original = fs::read(path.join("ecuc.arxml")).unwrap();
    let original_manifest = workspace.project_manifest().unwrap().clone();
    let issues = workspace
        .replace_layer_operations(None, vec![event_delete()])
        .unwrap_err();
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "LAYER_DELETE_REFERENCED"
                && issue.object.as_deref() == Some("/Configuration/Os/Task_Ecu"))
    );
    assert_eq!(workspace.project_manifest(), Some(&original_manifest));
    assert_eq!(fs::read(path.join("ecuc.arxml")).unwrap(), original);
    assert!(workspace.integration_plan().is_ok());
}

#[test]
fn base_upgrade_reports_same_field_conflict_and_blocks_generation_until_resolved() {
    let scratch = Scratch::new();
    let path = scratch.0.join("live");
    let mut workspace = fixture(&path, "3");
    workspace
        .replace_layer_operations(None, vec![priority("5")])
        .unwrap();
    workspace.save().unwrap();
    let incoming = upgrade(&path, &workspace, "4");
    let preview = workspace.preview_base_upgrade(incoming.clone()).unwrap();
    assert_eq!(preview.conflicts.len(), 2); // The inactive urgent variant also sets priority.
    let conflict = &preview.conflicts[0];
    assert_eq!(conflict.reason, UpgradeConflictKind::FieldChanged);
    assert_eq!(conflict.previous.as_ref().unwrap().values, ["3"]);
    assert_eq!(conflict.incoming.as_ref().unwrap().values, ["4"]);
    assert_eq!(conflict.operation, priority("5"));
    workspace
        .apply_base_upgrade(incoming, &preview.revision)
        .unwrap();
    assert!(
        workspace
            .saved_integration_plan()
            .err()
            .unwrap()
            .iter()
            .any(|issue| issue.code == "BASE_UPGRADE_CONFLICT")
    );
    workspace
        .resolve_base_conflict(0, ConflictResolution::UseBase)
        .unwrap();
    workspace
        .resolve_base_conflict(0, ConflictResolution::KeepProject)
        .unwrap();
    workspace.save().unwrap();
    assert!(open(&path).saved_integration_plan().is_ok());
}

#[test]
fn two_projects_generate_distinct_network_tasks_and_component_bindings_from_one_base() {
    let scratch = Scratch::new();
    let mut left = pipeline_variants(&scratch.0.join("left"));
    let mut right = pipeline_variants(&scratch.0.join("right"));
    right.select_project_variants(selection("reduced")).unwrap();
    right.save().unwrap();
    for workspace in [&mut left, &mut right] {
        let preview = workspace.preview_application_initialization().unwrap();
        workspace
            .initialize_application_previewed(&preview)
            .unwrap();
    }
    let base = |workspace: &Workspace| {
        workspace
            .project_manifest()
            .unwrap()
            .layers
            .as_ref()
            .unwrap()
            .base_reference
            .clone()
    };
    assert_eq!(base(&left), base(&right));
    let plan = right.saved_integration_plan().unwrap();
    let signal = &plan.description().signals[0];
    let next_id = 0x555;
    let parameter = if signal.receive {
        "CanIfRxPduCanId"
    } else {
        "CanIfTxPduCanId"
    };
    let row = plan
        .description()
        .configuration
        .iter()
        .find(|row| row.path == signal.can_if_pdu)
        .unwrap();
    let task = plan.description().schedule.task.clone();
    let mut priority = priority("4");
    priority.target.object.path = task;
    let int_action = |number: u32| LayerAction::Set {
        values: vec![TypedValue {
            kind: ValueKind::Integer,
            lexeme: number.to_string(),
        }],
        default_origin: None,
    };
    right
        .replace_layer_operations(
            None,
            vec![
                priority,
                LayerOperation {
                    target: LayerTarget {
                        object: LayerObject {
                            release: "R24-11".into(),
                            definition_type: "CAN-FRAME-TRIGGERING".into(),
                            path: signal.trigger.clone(),
                            instance_context: Vec::new(),
                        },
                        field: Some("IDENTIFIER".into()),
                    },
                    action: int_action(next_id),
                },
                LayerOperation {
                    target: LayerTarget {
                        object: LayerObject {
                            release: "R24-11".into(),
                            definition_type: row.definition.clone(),
                            path: row.path.clone(),
                            instance_context: Vec::new(),
                        },
                        field: Some(format!("{}/{parameter}", row.definition)),
                    },
                    action: int_action(next_id),
                },
            ],
        )
        .unwrap();
    right.save().unwrap();
    let previous = left.saved_integration_plan().unwrap();
    let current = right.saved_integration_plan().unwrap();
    assert_ne!(
        previous.description().schedule.task_priority,
        current.description().schedule.task_priority
    );
    assert_eq!(current.description().schedule.task_priority, 4);
    assert_eq!(previous.application_slot_descriptors().unwrap().len(), 3);
    assert_eq!(current.application_slot_descriptors().unwrap().len(), 1);
    assert_eq!(current.description().signals[0].can_id, next_id);
    let left_artifacts = prepare(&left, false);
    let right_artifacts = prepare(&right, false);
    let difference = compare_project_artifacts(&left_artifacts, &right_artifacts).unwrap();
    assert!(
        difference
            .iter()
            .any(|row| row.path.ends_with(".c") && row.change == ArtifactChange::Content)
    );
    assert!(difference.iter().any(|row| row.path == "application/Observe.c" && row.change == ArtifactChange::Removed));
    left_artifacts
        .generate(&scratch.0.join("left-package"))
        .unwrap();
    right_artifacts
        .generate(&scratch.0.join("right-package"))
        .unwrap();
}

#[test]
fn upgraded_base_flows_into_fields_that_inherit() {
    let scratch = Scratch::new();
    let path = scratch.0.join("live");
    let mut workspace = fixture(&path, "3");
    workspace
        .replace_layer_operations(None, vec![priority("5")])
        .unwrap();
    workspace
        .replace_layer_operations(None, Vec::new())
        .unwrap();
    workspace
        .replace_layer_operations(Some(("scheduler", "urgent")), Vec::new())
        .unwrap();
    workspace.save().unwrap();
    let current = workspace
        .project_manifest()
        .unwrap()
        .layers
        .as_ref()
        .unwrap()
        .base
        .clone();
    assert!(workspace.preview_base_upgrade(current).is_err());
    let incoming = upgrade(&path, &workspace, "4");
    let preview = workspace.preview_base_upgrade(incoming.clone()).unwrap();
    assert!(preview.conflicts.is_empty());
    workspace
        .apply_base_upgrade(incoming, &preview.revision)
        .unwrap();
    assert_eq!(
        open(&path)
            .saved_integration_plan()
            .unwrap()
            .description()
            .schedule
            .task_priority,
        4
    );
    assert!(
        fs::read_to_string(path.join("ecuc.arxml"))
            .unwrap()
            .contains("preserve across layer edits")
    );
}

fn pipeline_variants(path: &Path) -> Workspace {
    let complete = crate::support::mainline::inputs();
    let reduced = crate::support::mainline::single_component_sources();
    let mut base_members = Vec::new();
    let mut complete_members = Vec::new();
    let mut reduced_members = Vec::new();
    fs::create_dir_all(path.join("complete")).unwrap();
    fs::create_dir_all(path.join("reduced")).unwrap();
    for source in &complete {
        let simplified = reduced
            .iter()
            .find(|candidate| candidate.logical_path() == source.logical_path());
        if simplified.is_some_and(|candidate| candidate.bytes() == source.bytes()) {
            fs::write(path.join(source.logical_path()), source.bytes()).unwrap();
            base_members.push(source.logical_path().to_owned());
        } else {
            let name = format!("complete/{}", source.logical_path());
            fs::write(path.join(&name), source.bytes()).unwrap();
            complete_members.push(name);
            if let Some(simplified) = simplified {
                let name = format!("reduced/{}", simplified.logical_path());
                fs::write(path.join(&name), simplified.bytes()).unwrap();
                reduced_members.push(name);
            }
        }
    }
    assert!(!base_members.is_empty());
    let base = ProjectBase {
        reference: BaseReference {
            base_id: "pipeline".into(),
            revision: "first".into(),
            source_sha256: base_source_digest(
                base_members
                    .iter()
                    .map(|member| (member, fs::read(path.join(member)).unwrap())),
            ),
        },
        release: "R24-11".into(),
        rules: autosar_config_core::rules::rule_set_identity().unwrap(),
        extensions: Vec::new(),
        members: base_members.clone(),
        migrations: Vec::new(),
    };
    let manifest = ProjectManifest {
        format_version: 2,
        declared_release: "R24-11".into(),
        profile_hint: autosar_config_core::integration::PROFILE.into(),
        inputs: base_members
            .into_iter()
            .chain(complete_members.iter().cloned())
            .chain(reduced_members.iter().cloned())
            .map(|path| ProjectInput {
                path,
                role_hint: "source".into(),
            })
            .collect(),
        application_inputs: Vec::new(),
        accepted_extension_definitions: Vec::new(),
        layers: Some(ProjectLayers {
            base_reference: base.reference.clone(),
            base,
            project: ProjectLayer::default(),
            groups: BTreeMap::from([(
                "scheduler".into(),
                VariantGroup {
                    exactly_one: true,
                    options: BTreeMap::from([
                        (
                            "complete".into(),
                            ProjectLayer {
                                members: complete_members,
                                operations: Vec::new(),
                            },
                        ),
                        (
                            "reduced".into(),
                            ProjectLayer {
                                members: reduced_members,
                                operations: Vec::new(),
                            },
                        ),
                    ]),
                },
            )]),
            selection: selection("complete"),
            conflicts: Vec::new(),
        }),
    };
    write(path, &manifest);
    open(path)
}

#[test]
fn dormant_pipeline_application_survives_relocated_handoff_and_reactivation() {
    let scratch = Scratch::new();
    let live = scratch.0.join("live");
    let mut workspace = pipeline_variants(&live);
    let seed = workspace.preview_application_initialization().unwrap();
    workspace.initialize_application_previewed(&seed).unwrap();
    let authored = live.join("application/Observe.c");
    let mut implementation = fs::read(&authored).unwrap();
    implementation.extend_from_slice(b"\n/* user observations */\n");
    fs::write(&authored, &implementation).unwrap();

    let mut workspace = open(&live);
    assert_eq!(
        workspace
            .saved_integration_plan()
            .unwrap()
            .application_slot_descriptors()
            .unwrap()
            .len(),
        3
    );
    workspace
        .select_project_variants(selection("reduced"))
        .unwrap();
    workspace.save().unwrap();
    assert_eq!(
        workspace
            .saved_integration_plan()
            .unwrap()
            .application_slot_descriptors()
            .unwrap()
            .len(),
        1
    );
    let package = scratch.0.join("handoff");
    prepare(&workspace, true).generate(&package).unwrap();
    assert_eq!(
        fs::read(package.join("inputs/inactive/application/Observe.c")).unwrap(),
        implementation
    );
    let target: serde_json::Value =
        serde_json::from_slice(&fs::read(package.join("target.json")).unwrap()).unwrap();
    assert!(
        !target["sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry.as_str().unwrap().ends_with("/Observe.c"))
    );

    let moved = scratch.0.join("moved");
    fs::rename(&package, &moved).unwrap();
    fs::remove_dir_all(&live).unwrap();
    let receiver = scratch.0.join("receiver");
    let mut reopened = autosar_config_core::generator::delivery::open_handoff(
        &moved,
        &receiver,
        &autosar_config_core::definitions::DefinitionCatalog::builtin().unwrap(),
    )
    .unwrap()
    .workspace;
    assert_eq!(
        fs::read(receiver.join("application/Observe.c")).unwrap(),
        implementation
    );
    reopened
        .select_project_variants(selection("complete"))
        .unwrap();
    reopened.save().unwrap();
    let reactivated = scratch.0.join("reactivated");
    prepare(&reopened, false).generate(&reactivated).unwrap();
    assert_eq!(
        fs::read(reactivated.join("app/Observe/Observe.c")).unwrap(),
        implementation
    );
}

#[test]
fn base_upgrade_reports_new_inbound_reference_to_a_project_tombstone() {
    let scratch = Scratch::new();
    let path = scratch.0.join("live");
    let workspace = fixture(&path, "1");
    let original = r#"<AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:vendor="urn:test:vendor" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd">
<AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Unused</SHORT-NAME><ELEMENTS>
<SW-ADDR-METHOD><SHORT-NAME>Spare</SHORT-NAME></SW-ADDR-METHOD>
<!-- original platform marker -->
</ELEMENTS></AR-PACKAGE></AR-PACKAGES>
</AUTOSAR>"#;
    let file = "unused.arxml";
    fs::write(path.join(file), original).unwrap();
    let mut manifest = workspace.project_manifest().unwrap().clone();
    manifest.inputs.push(ProjectInput {
        path: file.into(),
        role_hint: "source".into(),
    });
    let layers = manifest.layers.as_mut().unwrap();
    layers.base.members.push(file.into());
    layers.base.reference.source_sha256 = base_source_digest(
        layers
            .base
            .members
            .iter()
            .map(|member| (member, fs::read(path.join(member)).unwrap())),
    );
    layers.base_reference = layers.base.reference.clone();
    write(&path, &manifest);
    let mut workspace = open(&path);
    let removal = LayerOperation {
        target: LayerTarget {
            object: LayerObject {
                release: "R24-11".into(),
                definition_type: "SW-ADDR-METHOD".into(),
                path: "/Unused/Spare".into(),
                instance_context: Vec::new(),
            },
            field: None,
        },
        action: LayerAction::Delete,
    };
    workspace
        .replace_layer_operations(None, vec![removal.clone()])
        .unwrap();
    workspace.save().unwrap();
    assert_eq!(fs::read_to_string(path.join(file)).unwrap(), original);
    assert!(
        !workspace
            .saved_integration_plan()
            .unwrap()
            .description()
            .objects
            .iter()
            .any(|object| object.path == "/Unused/Spare")
    );

    let mut incoming = upgrade(&path, &workspace, "1");
    let next_file = path.join("next").join(file);
    fs::write(
        &next_file,
        original.replace(
            "<SW-ADDR-METHOD><SHORT-NAME>Spare</SHORT-NAME></SW-ADDR-METHOD>",
            "",
        ),
    )
    .unwrap();
    incoming.reference.source_sha256 = base_source_digest(
        incoming
            .members
            .iter()
            .map(|member| (member, fs::read(path.join(member)).unwrap())),
    );
    let agreement = workspace.preview_base_upgrade(incoming.clone()).unwrap();
    assert_eq!(agreement.agreements, std::slice::from_ref(&removal.target));
    assert!(agreement.conflicts.is_empty());

    fs::write(
        path.join("next").join(file),
        original.replace(
            "<!-- original platform marker -->",
            "<!-- original platform marker --><vendor:METHOD-REF>/Unused/Spare</vendor:METHOD-REF>",
        ),
    )
    .unwrap();
    incoming.reference.source_sha256 = base_source_digest(
        incoming
            .members
            .iter()
            .map(|name| (name, fs::read(path.join(name)).unwrap())),
    );
    let preview = workspace.preview_base_upgrade(incoming.clone()).unwrap();
    assert_eq!(preview.conflicts.len(), 1);
    assert_eq!(preview.conflicts[0].operation, removal);
    assert_eq!(
        preview.conflicts[0].reason,
        UpgradeConflictKind::DeleteReferenced
    );
    workspace
        .apply_base_upgrade(incoming, &preview.revision)
        .unwrap();
    assert!(workspace.saved_integration_plan().is_err());
    workspace
        .resolve_base_conflict(0, ConflictResolution::UseBase)
        .unwrap();
    workspace.save().unwrap();
    assert!(open(&path).saved_integration_plan().is_ok());
}

#[test]
fn removing_did_and_route_dependencies_in_one_layer_keeps_the_effective_graph_closed() {
    let scratch = Scratch::new();
    let mut workspace = pipeline_variants(&scratch.0.join("live"));
    let previous = workspace.saved_integration_plan().unwrap();
    assert!(previous.description().diagnostic.is_some());
    assert!(previous.description().diagnostic_transport.is_some());
    let removed_containers = [
        "ApplicationValue",
        "ApplicationDid",
        "DcmService",
        "DiagRequest",
        "DiagResponse",
        "DiagRequestSource",
        "DiagRequestDestination",
        "DiagResponseSource",
        "DiagResponseDestination",
    ];
    let operations = workspace
        .project_semantics()
        .unwrap()
        .into_keys()
        .filter(|target| {
            if target.field.is_some() {
                return false;
            }
            let object = &target.object;
            let name = object.path.rsplit('/').next().unwrap();
            (removed_containers.contains(&name)
                && object.path.starts_with("/Configuration/")
                && (!name.starts_with("Diag") || object.definition_type.contains("/PduR/")))
                || [
                    "/Application/Pipeline/DcmService",
                    "/Application/Pipeline/Diagnostic",
                ]
                .contains(&object.path.as_str())
                || object
                    .instance_context
                    .iter()
                    .any(|context| context == "/Application/Pipeline/DcmService")
        })
        .map(|target| LayerOperation {
            target,
            action: LayerAction::Delete,
        })
        .collect::<Vec<_>>();
    assert!(
        operations
            .iter()
            .any(|operation| !operation.target.object.instance_context.is_empty())
    );
    workspace
        .replace_layer_operations(None, operations)
        .unwrap();
    workspace.save().unwrap();
    let current = workspace.saved_integration_plan().unwrap();
    assert!(current.description().diagnostic.is_none());
    assert!(current.description().diagnostic_transport.is_none());
}

#[test]
fn an_unselected_member_does_not_block_saving_the_active_variant() {
    let scratch = Scratch::new();
    let path = scratch.0.join("live");
    pipeline_variants(&path);
    let inactive = path.join("reduced/ecuc.arxml");
    let before = fs::read_to_string(&inactive).unwrap().replacen(
        "<SHORT-NAME>Configuration</SHORT-NAME>",
        "<SHORT-NAME>Configuration</SHORT-NAME><SHORT-NAME>Duplicated</SHORT-NAME>",
        1,
    );
    assert!(before.contains("<SHORT-NAME>Duplicated</SHORT-NAME>"));
    fs::write(&inactive, &before).unwrap();
    let mut workspace = open(&path);
    workspace
        .select_project_variants(selection("complete"))
        .unwrap();
    workspace.save().unwrap();
    assert!(workspace.saved_integration_plan().is_ok());
    assert_eq!(fs::read_to_string(inactive).unwrap(), before);
    let unsafe_encoding = before.replacen("encoding='utf-8'", "encoding='ISO-8859-1'", 1);
    assert_ne!(unsafe_encoding, before);
    fs::write(path.join("reduced/ecuc.arxml"), unsafe_encoding).unwrap();
    let error =
        Workspace::open_project_manifest(&path.join("workbench-project.json"), &path.join("cache"))
            .err()
            .unwrap();
    assert!(
        serde_json::to_string(&error)
            .unwrap()
            .contains("backend.rules.source_encoding")
    );
}

#[test]
fn setting_the_inherited_reference_list_changes_origin_without_retargeting() {
    let scratch = Scratch::new();
    let mut workspace = fixture(&scratch.0.join("live"), "1");
    let base = workspace.clone();
    let target = task("/AUTOSAR/EcucDefs/Os/OsTask/OsTaskEventRef");
    let rows = base.project_semantics().unwrap();
    let values = rows[&target]
        .values
        .iter()
        .map(|value| TypedValue {
            kind: ValueKind::Reference,
            lexeme: value.split_once(' ').unwrap().1.into(),
        })
        .collect();
    workspace
        .replace_layer_operations(
            None,
            vec![LayerOperation {
                target: target.clone(),
                action: LayerAction::Set {
                    values,
                    default_origin: None,
                },
            }],
        )
        .unwrap();
    let difference = base.compare_project(&workspace).unwrap();
    let row = difference
        .semantics
        .iter()
        .find(|row| row.target == target)
        .unwrap();
    assert_eq!(row.changes, [SemanticChange::Origin, SemanticChange::State]);
}

#[test]
fn a_layered_extension_field_survives_preparation_and_offline_reopen() {
    use autosar_config_core::definitions::DefinitionCatalog;
    use sha2::{Digest, Sha256};

    let scratch = Scratch::new();
    let path = scratch.0.join("live");
    let workspace = fixture(&path, "1");
    let authored = scratch.0.join("vendor");
    fs::create_dir(&authored).unwrap();
    let definitions = r#"<AUTOSAR xmlns="http://autosar.org/schema/r4.0"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Vendor</SHORT-NAME><ELEMENTS>
<ECUC-MODULE-DEF><SHORT-NAME>Lab</SHORT-NAME><CONTAINERS><ECUC-PARAM-CONF-CONTAINER-DEF><SHORT-NAME>Test</SHORT-NAME>
<PARAMETERS><ECUC-INTEGER-PARAM-DEF><SHORT-NAME>Threshold</SHORT-NAME><MIN>0</MIN><MAX>10</MAX></ECUC-INTEGER-PARAM-DEF></PARAMETERS>
</ECUC-PARAM-CONF-CONTAINER-DEF></CONTAINERS></ECUC-MODULE-DEF></ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>"#;
    fs::write(authored.join("definitions.arxml"), definitions).unwrap();
    let inventory = serde_json::json!({
        "formatVersion": 1, "catalogId": "vendor-threshold", "release": "R24-11",
        "version": "1.0", "files": [{"path": "definitions.arxml", "sha256": hex::encode(Sha256::digest(definitions))}],
    });
    let catalog_path = authored.join("catalog.json");
    fs::write(&catalog_path, serde_json::to_vec(&inventory).unwrap()).unwrap();
    let mut catalog = DefinitionCatalog::builtin().unwrap();
    let identity = catalog
        .accept_extension(&catalog_path, &scratch.0.join("cache"))
        .unwrap();
    let member = "vendor.arxml";
    fs::write(path.join(member), r#"<AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd">
<AR-PACKAGES><AR-PACKAGE><SHORT-NAME>VendorConfig</SHORT-NAME><ELEMENTS>
<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>Lab</SHORT-NAME><DEFINITION-REF DEST="ECUC-MODULE-DEF">/Vendor/Lab</DEFINITION-REF>
<CONTAINERS><ECUC-CONTAINER-VALUE><SHORT-NAME>Test</SHORT-NAME><DEFINITION-REF DEST="ECUC-PARAM-CONF-CONTAINER-DEF">/Vendor/Lab/Test</DEFINITION-REF>
<PARAMETER-VALUES><ECUC-NUMERICAL-PARAM-VALUE><DEFINITION-REF DEST="ECUC-INTEGER-PARAM-DEF">/Vendor/Lab/Test/Threshold</DEFINITION-REF><VALUE>1</VALUE></ECUC-NUMERICAL-PARAM-VALUE></PARAMETER-VALUES>
</ECUC-CONTAINER-VALUE></CONTAINERS></ECUC-MODULE-CONFIGURATION-VALUES></ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>"#).unwrap();
    let mut manifest = workspace.project_manifest().unwrap().clone();
    manifest
        .accepted_extension_definitions
        .push(identity.clone());
    manifest.inputs.push(ProjectInput {
        path: member.into(),
        role_hint: "source".into(),
    });
    let layers = manifest.layers.as_mut().unwrap();
    layers.base.extensions.push(identity);
    layers.project.members.push(member.into());
    write(&path, &manifest);
    let mut workspace =
        Workspace::open_project_with_catalog(&path.join("workbench-project.json"), &catalog)
            .unwrap();
    let target = LayerTarget {
        object: LayerObject {
            release: "R24-11".into(),
            definition_type: "/Vendor/Lab/Test".into(),
            path: "/VendorConfig/Lab/Test".into(),
            instance_context: Vec::new(),
        },
        field: Some("/Vendor/Lab/Test/Threshold".into()),
    };
    workspace
        .replace_layer_operations(
            None,
            vec![LayerOperation {
                target: target.clone(),
                action: LayerAction::Set {
                    values: vec![TypedValue {
                        kind: ValueKind::Integer,
                        lexeme: "7".into(),
                    }],
                    default_origin: None,
                },
            }],
        )
        .unwrap();
    workspace.save().unwrap();
    let package = scratch.0.join("sealed");
    prepare(&workspace, true).generate(&package).unwrap();
    let reopened = autosar_config_core::generator::delivery::open_handoff(
        &package,
        &scratch.0.join("reopened"),
        &catalog,
    )
    .unwrap()
    .workspace;
    assert_eq!(reopened.project_semantics().unwrap()[&target].values, ["7"]);
}
