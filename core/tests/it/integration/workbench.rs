//! Workbench edits and saves of mainline integration sources.
use super::*;
use std::path::Path;

#[test]
fn multi_workbench_edits_standard_fields_and_paired_instance_references() {
    use autosar_config_core::{Workspace, project_model::*};
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut workspace = live_multi_workspace(&root, &inputs());
    let projection = workspace
        .project_projection(&workspace.input_fingerprint().unwrap())
        .unwrap();
    assert_eq!(projection.profile, "singlecore-ecu-v1");
    assert!(
        workspace.view().issues.is_empty(),
        "{:?}",
        workspace.view().issues
    );
    let connector = projection
        .objects
        .iter()
        .find(|object| object.path == "/Application/Pipeline/IngressObserve")
        .unwrap();
    assert!(connector.writable);
    let ref_change = |definition: &str, target_path: &str| {
        let field = projection
            .fields
            .iter()
            .find(|field| {
                field.object_id == connector.object_id
                    && field.definition.definition_id == definition
            })
            .unwrap();
        assert!(field.definition.writable, "{definition}");
        let target = projection
            .objects
            .iter()
            .find(|object| object.path == target_path)
            .unwrap();
        ConfigurationChange::SetReference {
            change_id: definition.into(),
            field: FieldRef::Existing {
                field_id: field.field_id.clone(),
            },
            expected: field.reference.clone().unwrap(),
            value: ReferenceState::Explicit {
                raw_path: target.path.clone(),
                dest: target.kind.clone(),
                target: Some(ObjectRef::Existing {
                    object_id: target.object_id.clone(),
                }),
            },
        }
    };
    let set = |changes| ChangeSet {
        workspace_epoch: projection.workspace_epoch.clone(),
        input_fingerprint: projection.input_fingerprint.clone(),
        definition_fingerprint: projection.definition_fingerprint.clone(),
        changes,
    };
    let context = ref_change(
        "ASSEMBLY-SW-CONNECTOR#PROVIDER-IREF/CONTEXT-COMPONENT-REF",
        "/Application/Pipeline/ProcessInstance",
    );
    assert!(
        workspace
            .prepare_change(&set(vec![context.clone()]))
            .is_err()
    );
    let paired = set(vec![
        context,
        ref_change(
            "ASSEMBLY-SW-CONNECTOR#PROVIDER-IREF/TARGET-P-PORT-REF",
            "/Application/Process/Result",
        ),
    ]);
    let preview = workspace.prepare_change(&paired).unwrap();
    workspace
        .apply_change(&paired, &preview.change_revision)
        .unwrap();
    assert!(workspace.integration_plan().is_ok());
    let save = workspace.preview_save().unwrap();
    workspace.save_previewed(&save.revision).unwrap();
    let reopened =
        Workspace::open_project_manifest(&root.join("workbench-project.json"), &root.join("cache"))
            .unwrap();
    assert!(reopened.integration_plan().is_ok());
    for source in inputs()
        .iter()
        .filter(|source| source.logical_path() != "composition.arxml")
    {
        assert_eq!(
            std::fs::read(root.join(source.logical_path())).unwrap(),
            source.bytes()
        );
    }
    let path = root.join("composition.arxml");
    let saved = std::fs::read(&path).unwrap();
    let mut changed = saved.clone();
    changed.extend_from_slice(b"\n<!-- independent external edit -->\n");
    std::fs::write(&path, &changed).unwrap();
    let rejected = reopened.saved_integration_plan().err().unwrap();
    assert_eq!(rejected[0].code, "SOURCE_CHANGED");
    assert_eq!(std::fs::read(&path).unwrap(), changed);
    std::fs::write(&path, &saved).unwrap();
}

#[test]
fn mainline_integration_save_refuses_stale_preview_prior_backup_and_unsafe_edit() {
    use autosar_config_core::{Workspace, integration::IntegrationEdit};
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let sources = inputs();
    let mut workspace = live_multi_workspace(&root, &sources);
    let unchanged = workspace.preview_integration_save().unwrap();
    workspace
        .edit_integration(IntegrationEdit {
            application_period_ms: Some(20),
            ..IntegrationEdit::default()
        })
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    let stale = workspace
        .clone()
        .prepare_integration_save_previewed(&unchanged.revision)
        .err()
        .unwrap();
    assert_eq!(stale[0].code, "SAVE_PREVIEW_STALE");
    let stale = workspace
        .save_integration_previewed(&unchanged.revision)
        .err()
        .unwrap();
    assert_eq!(stale[0].code, "SAVE_FAILED");

    // A prior interrupted save's backup is recovery data: refuse, keep it and
    // every source byte.
    let preview = workspace.preview_integration_save().unwrap();
    let changed =
        Path::new(&preview.files.iter().find(|file| file.changed).unwrap().path).to_path_buf();
    let backup =
        changed.with_extension(format!("arxml.autosar-config-{}-0.bak", std::process::id()));
    std::fs::write(&backup, b"prior interrupted save").unwrap();
    let refused = workspace
        .save_integration_previewed(&preview.revision)
        .err()
        .unwrap();
    assert_eq!(refused[0].code, "SAVE_FAILED");
    assert_eq!(std::fs::read(&backup).unwrap(), b"prior interrupted save");
    for source in &sources {
        assert_eq!(
            std::fs::read(root.join(source.logical_path())).unwrap(),
            source.bytes()
        );
    }
    std::fs::remove_file(&backup).unwrap();
    let saved = workspace
        .save_integration_previewed(&preview.revision)
        .unwrap();
    assert!(!workspace.view().dirty);
    let reopened =
        Workspace::open_project_manifest(&root.join("workbench-project.json"), &root.join("cache"))
            .unwrap();
    assert_eq!(
        serde_json::to_vec(&saved).unwrap(),
        serde_json::to_vec(&reopened.inspect_integration()).unwrap()
    );

    // A value that cannot be patched as one text node refuses the edit and
    // leaves the save preview unchanged.
    let mut commented = inputs();
    change(
        &mut commented,
        "extract.arxml",
        "<IDENTIFIER>800</IDENTIFIER>",
        "<IDENTIFIER>800<!-- retained numeric comment --></IDENTIFIER>",
    );
    let mut mixed = live_multi_workspace(&scratch.0.join("mixed"), &commented);
    let before = mixed.preview_integration_save().unwrap();
    let port = mixed
        .inspect_integration()
        .description
        .unwrap()
        .signals
        .iter()
        .find(|signal| signal.can_id == 800)
        .unwrap()
        .port
        .clone();
    let issues = mixed
        .edit_integration(IntegrationEdit {
            can_ids: [(port, 1100)].into(),
            application_period_ms: None,
        })
        .err()
        .unwrap();
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "EDIT_UNSAFE"
                && issue.file.as_deref() == Some("extract.arxml")),
        "{issues:?}"
    );
    assert_eq!(
        serde_json::to_vec(&before).unwrap(),
        serde_json::to_vec(&mixed.preview_integration_save().unwrap()).unwrap()
    );
}

#[test]
fn initialized_multi_workbench_rejects_component_and_entry_identity_changes() {
    use autosar_config_core::project_model::*;
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut workspace = live_multi_workspace(&root, &inputs());
    let initialization = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&initialization)
        .unwrap();
    let projection = workspace
        .project_projection(&workspace.input_fingerprint().unwrap())
        .unwrap();
    let component = projection
        .objects
        .iter()
        .find(|object| object.path == "/Application/Process")
        .unwrap();
    let changes = ChangeSet {
        workspace_epoch: projection.workspace_epoch.clone(),
        input_fingerprint: projection.input_fingerprint.clone(),
        definition_fingerprint: projection.definition_fingerprint.clone(),
        changes: vec![ConfigurationChange::RenameInstance {
            change_id: "rename".into(),
            object: ObjectRef::Existing {
                object_id: component.object_id.clone(),
            },
            expected_short_name: "Process".into(),
            short_name: "Compute".into(),
        }],
    };
    let symbol = projection
        .fields
        .iter()
        .find(|field| {
            field.definition.definition_id == "RUNNABLE-ENTITY#SYMBOL"
                && projection.objects.iter().any(|object| {
                    object.object_id == field.object_id
                        && object.path == "/Application/Process/Behavior/Periodic"
                })
        })
        .unwrap();
    let symbol_changes = ChangeSet {
        changes: vec![ConfigurationChange::SetValue {
            change_id: "symbol".into(),
            field: FieldRef::Existing {
                field_id: symbol.field_id.clone(),
            },
            expected: symbol.current.clone(),
            value: ValueState::Explicit {
                value: TypedValue {
                    kind: ValueKind::FunctionName,
                    lexeme: "Process_AlternatePeriodic".into(),
                },
            },
        }],
        ..changes.clone()
    };
    for changes in [&changes, &symbol_changes] {
        let rejection = workspace.prepare_change(changes).unwrap_err();
        assert_eq!(
            serde_json::to_value(&rejection).unwrap()["key"],
            "backend.arxml.changes.initialized_identity_change"
        );
        assert_eq!(
            workspace.apply_change(changes, "unapproved").unwrap_err(),
            rejection
        );
        assert!(!workspace.view().dirty);
        assert_eq!(
            serde_json::to_value(
                workspace
                    .project_projection(&projection.input_fingerprint)
                    .unwrap()
            )
            .unwrap(),
            serde_json::to_value(&projection).unwrap()
        );
        for source in inputs() {
            assert_eq!(
                std::fs::read(root.join(source.logical_path())).unwrap(),
                source.bytes()
            );
        }
        for file in &initialization.files {
            if file.path.ends_with(".c") {
                assert_eq!(
                    std::fs::read_to_string(root.join(&file.path)).unwrap(),
                    file.contents
                );
            }
        }
    }
}

#[test]
fn multi_workbench_standard_child_creation_uses_xml_syntax_and_save_is_not_a_generation_gate() {
    use autosar_config_core::{Workspace, project_model::*};
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut workspace = live_multi_workspace(&root, &inputs());
    let projection = workspace
        .project_projection(&workspace.input_fingerprint().unwrap())
        .unwrap();
    let owner = projection
        .objects
        .iter()
        .find(|object| object.path == "/Application/Process/Behavior")
        .unwrap();
    assert!(owner.writable);
    let runnable = projection
        .objects
        .iter()
        .find(|object| object.path == "/Application/Process/Behavior/Periodic")
        .unwrap();
    let created = ObjectRef::Created {
        change_id: "event".into(),
    };
    let changes = ChangeSet {
        workspace_epoch: projection.workspace_epoch.clone(),
        input_fingerprint: projection.input_fingerprint.clone(),
        definition_fingerprint: projection.definition_fingerprint.clone(),
        changes: vec![
            ConfigurationChange::CreateInstance {
                change_id: "event".into(),
                parent: ObjectRef::Existing {
                    object_id: owner.object_id.clone(),
                },
                source_id: owner.source_id.clone(),
                definition_id: "TIMING-EVENT".into(),
                short_name: "ExtraPeriodic".into(),
            },
            ConfigurationChange::SetValue {
                change_id: "period".into(),
                field: FieldRef::New {
                    object: created.clone(),
                    definition_id: "TIMING-EVENT#PERIOD".into(),
                    entry_key: "period".into(),
                },
                expected: ValueState::Absent,
                value: ValueState::Explicit {
                    value: TypedValue {
                        kind: ValueKind::Float,
                        lexeme: "0.01".into(),
                    },
                },
            },
            ConfigurationChange::SetReference {
                change_id: "start".into(),
                field: FieldRef::New {
                    object: created,
                    definition_id: "TIMING-EVENT#START-ON-EVENT-REF".into(),
                    entry_key: "start".into(),
                },
                expected: ReferenceState::Absent,
                value: ReferenceState::Explicit {
                    raw_path: runnable.path.clone(),
                    dest: runnable.kind.clone(),
                    target: Some(ObjectRef::Existing {
                        object_id: runnable.object_id.clone(),
                    }),
                },
            },
        ],
    };
    let preview = workspace.prepare_change(&changes).unwrap();
    workspace
        .apply_change(&changes, &preview.change_revision)
        .unwrap();
    assert!(workspace.integration_plan().is_err());
    let save = workspace.preview_save().unwrap();
    workspace.save_previewed(&save.revision).unwrap();
    let text = std::fs::read_to_string(root.join("process.arxml")).unwrap();
    assert!(text.contains("<TIMING-EVENT><SHORT-NAME>ExtraPeriodic</SHORT-NAME><START-ON-EVENT-REF DEST=\"RUNNABLE-ENTITY\">/Application/Process/Behavior/Periodic</START-ON-EVENT-REF><PERIOD>0.01</PERIOD></TIMING-EVENT>"));
    let reopened =
        Workspace::open_project_manifest(&root.join("workbench-project.json"), &root.join("cache"))
            .unwrap();
    assert!(reopened.integration_plan().is_err());
}

#[test]
fn multi_workbench_nested_reference_creation_preserves_standard_wrappers() {
    use autosar_config_core::project_model::*;
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut workspace = live_multi_workspace(&root, &inputs());
    for (parent_path, kind, name, references) in [
        (
            "/Application/Process/Behavior",
            "OPERATION-INVOKED-EVENT",
            "ExtraInvocation",
            vec![
                (
                    "START-ON-EVENT-REF",
                    "/Application/Process/Behavior/ServerTransform",
                ),
                ("CONTEXT-P-PORT-REF", "/Application/Process/ResultService"),
                (
                    "TARGET-PROVIDED-OPERATION-REF",
                    "/Types/ScalarService/Transform",
                ),
            ],
        ),
        (
            "/Application/Process",
            "SWC-INTERNAL-BEHAVIOR",
            "ExtraBehavior",
            vec![("DATA-TYPE-MAPPING-REF", "/Types/ApplicationTypes")],
        ),
        (
            "/Types",
            "IMPLEMENTATION-DATA-TYPE",
            "ExtraUint32",
            vec![("BASE-TYPE-REF", "/Types/Base_uint32")],
        ),
        (
            "/Types",
            "DATA-TYPE-MAPPING-SET",
            "ExtraMapping",
            vec![
                ("APPLICATION-DATA-TYPE-REF", "/Types/ApplicationUint32"),
                ("IMPLEMENTATION-DATA-TYPE-REF", "/Types/uint32"),
            ],
        ),
    ] {
        let projection = workspace
            .project_projection(&workspace.input_fingerprint().unwrap())
            .unwrap();
        let type_source = projection
            .objects
            .iter()
            .find(|object| object.path == "/Types/uint32")
            .unwrap()
            .source_id
            .clone();
        let parent = projection
            .objects
            .iter()
            .find(|object| {
                object.path == parent_path
                    && (parent_path != "/Types" || object.source_id == type_source)
            })
            .unwrap();
        let created = ObjectRef::Created {
            change_id: "create".into(),
        };
        let mut changes = vec![ConfigurationChange::CreateInstance {
            change_id: "create".into(),
            parent: ObjectRef::Existing {
                object_id: parent.object_id.clone(),
            },
            source_id: parent.source_id.clone(),
            definition_id: kind.into(),
            short_name: name.into(),
        }];
        if kind == "IMPLEMENTATION-DATA-TYPE" {
            changes.push(ConfigurationChange::SetValue {
                change_id: "category".into(),
                field: FieldRef::New {
                    object: created.clone(),
                    definition_id: format!("{kind}#CATEGORY"),
                    entry_key: "category".into(),
                },
                expected: ValueState::Absent,
                value: ValueState::Explicit {
                    value: TypedValue {
                        kind: ValueKind::Enumeration,
                        lexeme: "VALUE".into(),
                    },
                },
            });
        }
        for (tag, path) in references {
            let target = projection
                .objects
                .iter()
                .find(|object| object.path == path)
                .unwrap();
            changes.push(ConfigurationChange::SetReference {
                change_id: tag.into(),
                field: FieldRef::New {
                    object: created.clone(),
                    definition_id: format!("{kind}#{tag}"),
                    entry_key: tag.into(),
                },
                expected: ReferenceState::Absent,
                value: ReferenceState::Explicit {
                    raw_path: path.into(),
                    dest: target.kind.clone(),
                    target: Some(ObjectRef::Existing {
                        object_id: target.object_id.clone(),
                    }),
                },
            });
        }
        let changes = ChangeSet {
            workspace_epoch: projection.workspace_epoch,
            input_fingerprint: projection.input_fingerprint,
            definition_fingerprint: projection.definition_fingerprint,
            changes,
        };
        let preview = workspace.prepare_change(&changes).unwrap();
        workspace
            .apply_change(&changes, &preview.change_revision)
            .unwrap();
    }
    let save = workspace.preview_save().unwrap();
    workspace.save_previewed(&save.revision).unwrap();
    for (file, name, expected_parent) in [
        ("process.arxml", "ExtraInvocation", "OPERATION-IREF"),
        ("process.arxml", "ExtraBehavior", "DATA-TYPE-MAPPING-REFS"),
        (
            "types.arxml",
            "ExtraUint32",
            "SW-DATA-DEF-PROPS-CONDITIONAL",
        ),
        ("types.arxml", "ExtraMapping", "DATA-TYPE-MAP"),
    ] {
        let text = std::fs::read_to_string(root.join(file)).unwrap();
        let document = roxmltree::Document::parse(&text).unwrap();
        let object = document
            .descendants()
            .find(|node| {
                node.is_element()
                    && node.children().any(|child| {
                        child.tag_name().name() == "SHORT-NAME" && child.text() == Some(name)
                    })
            })
            .unwrap();
        let refs = object
            .descendants()
            .filter(|node| {
                node.is_element()
                    && node.tag_name().name().ends_with("-REF")
                    && node.tag_name().name() != "START-ON-EVENT-REF"
            })
            .collect::<Vec<_>>();
        assert!(!refs.is_empty());
        for reference in refs {
            assert_eq!(
                reference.parent_element().unwrap().tag_name().name(),
                expected_parent
            );
        }
    }
    let reopened = autosar_config_core::Workspace::open_project_manifest(
        &root.join("workbench-project.json"),
        &root.join("cache"),
    )
    .unwrap();
    let projection = reopened
        .project_projection(&reopened.input_fingerprint().unwrap())
        .unwrap();
    assert!(
        projection
            .fields
            .iter()
            .filter(|field| projection
                .objects
                .iter()
                .any(|object| object.object_id == field.object_id
                    && object.short_name.starts_with("Extra")))
            .all(|field| field.definition.writable)
    );
}

#[test]
fn multi_workbench_instance_rename_rebinds_known_source_irefs_and_preserves_unknown_irefs() {
    use autosar_config_core::project_model::*;
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut workspace = live_multi_workspace(&root, &inputs());
    let view = workspace
        .project_projection(&workspace.input_fingerprint().unwrap())
        .unwrap();
    let instance = view
        .objects
        .iter()
        .find(|object| object.path == "/Application/Pipeline/ProcessInstance")
        .unwrap();
    let long_name = "ProcessingComponentInstanceWithAnExplicitLongEngineeringIdentity";
    let set = ChangeSet {
        workspace_epoch: view.workspace_epoch.clone(),
        input_fingerprint: view.input_fingerprint.clone(),
        definition_fingerprint: view.definition_fingerprint.clone(),
        changes: vec![ConfigurationChange::RenameInstance {
            change_id: "rename".into(),
            object: ObjectRef::Existing {
                object_id: instance.object_id.clone(),
            },
            expected_short_name: instance.short_name.clone(),
            short_name: long_name.into(),
        }],
    };
    let preview = workspace.prepare_change(&set).unwrap();
    workspace
        .apply_change(&set, &preview.change_revision)
        .unwrap();
    assert!(workspace.integration_plan().is_ok());
    let view = workspace
        .project_projection(&workspace.input_fingerprint().unwrap())
        .unwrap();
    assert!(
        view.references
            .iter()
            .any(|edge| edge.raw_path.ends_with(long_name))
    );
    assert!(
        !view
            .references
            .iter()
            .any(|edge| edge.raw_path.ends_with("/ProcessInstance"))
    );
    let mut opaque = inputs();
    change(
        &mut opaque,
        "composition.arxml",
        "<PROVIDER-IREF>",
        "<UNKNOWN-IREF>",
    );
    change(
        &mut opaque,
        "composition.arxml",
        "</PROVIDER-IREF>",
        "</UNKNOWN-IREF>",
    );
    let opaque_root = scratch.0.join("opaque");
    let mut workspace = live_multi_workspace(&opaque_root, &opaque);
    let view = workspace
        .project_projection(&workspace.input_fingerprint().unwrap())
        .unwrap();
    assert!(
        !view
            .objects
            .iter()
            .find(|object| object.path == "/Application/Pipeline/IngressProcess")
            .unwrap()
            .writable
    );
    let connector = view
        .objects
        .iter()
        .find(|object| object.path == "/Application/Pipeline/IngressProcess")
        .unwrap();
    let changes = ChangeSet {
        workspace_epoch: view.workspace_epoch.clone(),
        input_fingerprint: view.input_fingerprint.clone(),
        definition_fingerprint: view.definition_fingerprint.clone(),
        changes: vec![ConfigurationChange::RenameInstance {
            change_id: "opaque-rename".into(),
            object: ObjectRef::Existing {
                object_id: connector.object_id.clone(),
            },
            expected_short_name: connector.short_name.clone(),
            short_name: "ChangedOpaqueConnector".into(),
        }],
    };
    let rejection = workspace.prepare_change(&changes).unwrap_err();
    assert_eq!(
        workspace.apply_change(&changes, "unapproved").unwrap_err(),
        rejection
    );
    assert!(!workspace.view().dirty);
    assert_eq!(
        serde_json::to_value(
            workspace
                .project_projection(&view.input_fingerprint)
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(view).unwrap()
    );
    for source in &opaque {
        assert_eq!(
            std::fs::read(opaque_root.join(source.logical_path())).unwrap(),
            source.bytes()
        );
    }

    let mut variant = inputs();
    change(
        &mut variant,
        "composition.arxml",
        "<SHORT-NAME>ProcessInstance</SHORT-NAME>",
        "<SHORT-NAME>ProcessInstance</SHORT-NAME><VARIATION-POINT><SHORT-LABEL>UnresolvedInstance</SHORT-LABEL></VARIATION-POINT>",
    );
    let variant_root = scratch.0.join("variant");
    let mut workspace = live_multi_workspace(&variant_root, &variant);
    let fingerprint = workspace.input_fingerprint().unwrap();
    let view = workspace.project_projection(&fingerprint).unwrap();
    let instance = view
        .objects
        .iter()
        .find(|object| object.path == "/Application/Pipeline/ProcessInstance")
        .unwrap();
    assert!(!instance.writable);
    let changes = ChangeSet {
        workspace_epoch: view.workspace_epoch.clone(),
        input_fingerprint: view.input_fingerprint.clone(),
        definition_fingerprint: view.definition_fingerprint.clone(),
        changes: vec![ConfigurationChange::RenameInstance {
            change_id: "variant-rename".into(),
            object: ObjectRef::Existing {
                object_id: instance.object_id.clone(),
            },
            expected_short_name: instance.short_name.clone(),
            short_name: long_name.into(),
        }],
    };
    let rejection = workspace.prepare_change(&changes).unwrap_err();
    assert_eq!(
        workspace.apply_change(&changes, "unapproved").unwrap_err(),
        rejection,
        "apply must reject the unsafe object before accepting a preview revision"
    );
    assert_eq!(workspace.input_fingerprint().unwrap(), fingerprint);
    assert!(!workspace.view().dirty);
    assert_eq!(
        serde_json::to_value(workspace.project_projection(&fingerprint).unwrap()).unwrap(),
        serde_json::to_value(view).unwrap(),
        "refused edits must preserve the complete projection and its revision"
    );
    for source in &variant {
        assert_eq!(
            std::fs::read(variant_root.join(source.logical_path())).unwrap(),
            source.bytes()
        );
    }
}

#[test]
fn multi_workbench_connector_creation_builds_paired_standard_irefs_atomically() {
    use autosar_config_core::project_model::*;
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut workspace = live_multi_workspace(&root, &inputs());
    let view = workspace
        .project_projection(&workspace.input_fingerprint().unwrap())
        .unwrap();
    let owner = view
        .objects
        .iter()
        .find(|object| object.path == "/Application/Pipeline")
        .unwrap();
    let mut changes = vec![ConfigurationChange::CreateInstance {
        change_id: "connector".into(),
        parent: ObjectRef::Existing {
            object_id: owner.object_id.clone(),
        },
        source_id: owner.source_id.clone(),
        definition_id: "ASSEMBLY-SW-CONNECTOR".into(),
        short_name: "ExtraIngressObserve".into(),
    }];
    for (index, (field, target_path)) in [
        (
            "PROVIDER-IREF/CONTEXT-COMPONENT-REF",
            "/Application/Pipeline/IngressInstance",
        ),
        (
            "PROVIDER-IREF/TARGET-P-PORT-REF",
            "/Application/Ingress/Value",
        ),
        (
            "REQUESTER-IREF/CONTEXT-COMPONENT-REF",
            "/Application/Pipeline/ObserveInstance",
        ),
        (
            "REQUESTER-IREF/TARGET-R-PORT-REF",
            "/Application/Observe/Value",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let target = view
            .objects
            .iter()
            .find(|object| object.path == target_path)
            .expect(target_path);
        changes.push(ConfigurationChange::SetReference {
            change_id: format!("endpoint-{index}"),
            field: FieldRef::New {
                object: ObjectRef::Created {
                    change_id: "connector".into(),
                },
                definition_id: format!("ASSEMBLY-SW-CONNECTOR#{field}"),
                entry_key: format!("endpoint-{index}"),
            },
            expected: ReferenceState::Absent,
            value: ReferenceState::Explicit {
                raw_path: target.path.clone(),
                dest: target.kind.clone(),
                target: Some(ObjectRef::Existing {
                    object_id: target.object_id.clone(),
                }),
            },
        });
    }
    let set = ChangeSet {
        workspace_epoch: view.workspace_epoch,
        input_fingerprint: view.input_fingerprint,
        definition_fingerprint: view.definition_fingerprint,
        changes,
    };
    let preview = workspace.prepare_change(&set).unwrap();
    workspace
        .apply_change(&set, &preview.change_revision)
        .unwrap();
    assert_eq!(
        workspace.saved_integration_plan().err().unwrap()[0].code,
        "SOURCE_DIRTY"
    );
    let save = workspace.preview_save().unwrap();
    workspace.save_previewed(&save.revision).unwrap();
    let bytes = std::fs::read_to_string(root.join("composition.arxml")).unwrap();
    assert!(bytes.contains("<ASSEMBLY-SW-CONNECTOR><SHORT-NAME>ExtraIngressObserve</SHORT-NAME><PROVIDER-IREF><CONTEXT-COMPONENT-REF DEST=\"SW-COMPONENT-PROTOTYPE\">/Application/Pipeline/IngressInstance</CONTEXT-COMPONENT-REF><TARGET-P-PORT-REF DEST=\"P-PORT-PROTOTYPE\">/Application/Ingress/Value</TARGET-P-PORT-REF></PROVIDER-IREF><REQUESTER-IREF>"));
}

#[test]
fn multi_workbench_restores_missing_type_category_in_declared_xml_order() {
    use autosar_config_core::project_model::*;
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut sources = inputs();
    change(
        &mut sources,
        "types.arxml",
        "<SHORT-NAME>uint8</SHORT-NAME>\n          <CATEGORY>VALUE</CATEGORY>",
        "<SHORT-NAME>uint8</SHORT-NAME>",
    );
    let before = sources
        .iter()
        .find(|source| source.logical_path() == "types.arxml")
        .unwrap()
        .bytes()
        .to_vec();
    let mut workspace = live_multi_workspace(&root, &sources);
    let view = workspace
        .project_projection(&workspace.input_fingerprint().unwrap())
        .unwrap();
    let object = view
        .objects
        .iter()
        .find(|object| object.path == "/Types/uint8")
        .unwrap();
    let field = view
        .fields
        .iter()
        .find(|field| {
            field.object_id == object.object_id
                && field.definition.definition_id == "IMPLEMENTATION-DATA-TYPE#CATEGORY"
        })
        .unwrap();
    assert!(field.definition.writable);
    assert_eq!(field.current, ValueState::Absent);
    let set = ChangeSet {
        workspace_epoch: view.workspace_epoch,
        input_fingerprint: view.input_fingerprint,
        definition_fingerprint: view.definition_fingerprint,
        changes: vec![ConfigurationChange::SetValue {
            change_id: "restore-category".into(),
            field: FieldRef::Existing {
                field_id: field.field_id.clone(),
            },
            expected: ValueState::Absent,
            value: ValueState::Explicit {
                value: TypedValue {
                    kind: ValueKind::Enumeration,
                    lexeme: "VALUE".into(),
                },
            },
        }],
    };
    let preview = workspace.prepare_change(&set).unwrap();
    workspace
        .apply_change(&set, &preview.change_revision)
        .unwrap();
    let save = workspace.preview_save().unwrap();
    workspace.save_previewed(&save.revision).unwrap();
    let text = std::fs::read_to_string(root.join("types.arxml")).unwrap();
    let doc = roxmltree::Document::parse(&text).unwrap();
    let owner = doc
        .descendants()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "IMPLEMENTATION-DATA-TYPE"
                && node
                    .children()
                    .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some("uint8"))
        })
        .unwrap();
    let tags = owner
        .children()
        .filter(|node| node.is_element())
        .map(|node| node.tag_name().name())
        .collect::<Vec<_>>();
    assert_eq!(tags, ["SHORT-NAME", "CATEGORY", "SW-DATA-DEF-PROPS"]);
    let category = owner
        .children()
        .find(|node| node.has_tag_name("CATEGORY"))
        .unwrap();
    let mut restored = text.clone();
    restored.replace_range(category.range(), "");
    assert_eq!(restored.as_bytes(), before);
    let reopened = autosar_config_core::Workspace::open_project_manifest(
        &root.join("workbench-project.json"),
        &root.join("cache"),
    )
    .unwrap();
    assert!(reopened.integration_plan().is_ok());
}

#[test]
fn multi_workbench_existing_invalid_connector_allows_unrelated_safe_edits() {
    use autosar_config_core::{Workspace, project_model::*};
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut sources = inputs();
    change(
        &mut sources,
        "composition.arxml",
        "/Application/Ingress/Value",
        "/Application/Process/Result",
    );
    let original = sources
        .iter()
        .find(|source| source.logical_path() == "composition.arxml")
        .unwrap()
        .bytes()
        .to_vec();
    let mut workspace = live_multi_workspace(&root, &sources);
    let view = workspace
        .project_projection(&workspace.input_fingerprint().unwrap())
        .unwrap();
    assert!(
        view.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.scope == ValidationScope::TargetGeneration)
    );
    let connector = view
        .objects
        .iter()
        .find(|object| object.path == "/Application/Pipeline/IngressProcess")
        .unwrap();
    let endpoint = view
        .fields
        .iter()
        .find(|field| {
            field.object_id == connector.object_id
                && field.definition.definition_id
                    == "ASSEMBLY-SW-CONNECTOR#PROVIDER-IREF/CONTEXT-COMPONENT-REF"
        })
        .unwrap();
    let other_instance = view
        .objects
        .iter()
        .find(|object| object.path == "/Application/Pipeline/ObserveInstance")
        .unwrap();
    let invalid_change = ChangeSet {
        workspace_epoch: view.workspace_epoch.clone(),
        input_fingerprint: view.input_fingerprint.clone(),
        definition_fingerprint: view.definition_fingerprint.clone(),
        changes: vec![ConfigurationChange::SetReference {
            change_id: "edited-invalid-pair".into(),
            field: FieldRef::Existing {
                field_id: endpoint.field_id.clone(),
            },
            expected: endpoint.reference.clone().unwrap(),
            value: ReferenceState::Explicit {
                raw_path: other_instance.path.clone(),
                dest: other_instance.kind.clone(),
                target: Some(ObjectRef::Existing {
                    object_id: other_instance.object_id.clone(),
                }),
            },
        }],
    };
    assert!(workspace.prepare_change(&invalid_change).is_err());
    let event = view
        .objects
        .iter()
        .find(|object| object.path == "/Application/Process/Behavior/Periodic10ms")
        .unwrap();
    let field = view
        .fields
        .iter()
        .find(|field| {
            field.object_id == event.object_id
                && field.definition.definition_id == "TIMING-EVENT#PERIOD"
        })
        .unwrap();
    assert!(field.definition.writable);
    let set = ChangeSet {
        workspace_epoch: view.workspace_epoch,
        input_fingerprint: view.input_fingerprint,
        definition_fingerprint: view.definition_fingerprint,
        changes: vec![ConfigurationChange::SetValue {
            change_id: "safe-period".into(),
            field: FieldRef::Existing {
                field_id: field.field_id.clone(),
            },
            expected: field.current.clone(),
            value: ValueState::Explicit {
                value: TypedValue {
                    kind: ValueKind::Float,
                    lexeme: "0.0100".into(),
                },
            },
        }],
    };
    let preview = workspace.prepare_change(&set).unwrap();
    workspace
        .apply_change(&set, &preview.change_revision)
        .unwrap();
    assert!(workspace.integration_plan().is_err());
    let save = workspace.preview_save().unwrap();
    workspace.save_previewed(&save.revision).unwrap();
    assert_eq!(
        std::fs::read(root.join("composition.arxml")).unwrap(),
        original
    );
    assert!(
        std::fs::read_to_string(root.join("process.arxml"))
            .unwrap()
            .contains("<PERIOD>0.0100</PERIOD>")
    );
    let reopened =
        Workspace::open_project_manifest(&root.join("workbench-project.json"), &root.join("cache"))
            .unwrap();
    assert!(reopened.integration_plan().is_err());
}

#[test]
fn template_quick_edit_changes_can_ids_and_application_period_on_the_mainline() {
    use autosar_config_core::{Workspace, integration::IntegrationEdit};
    let scratch = Scratch::new();
    let preview =
        Workspace::preview_project_creation(&scratch.0.join("quick"), "Quick", "standard-ecu")
            .unwrap();
    let mut workspace = Workspace::create_project_previewed(&preview).unwrap();
    let before = workspace.inspect_integration();
    let description = before.description.unwrap();
    assert_eq!(
        description.profile,
        autosar_config_core::integration::PROFILE
    );
    let can_ids = description
        .signals
        .iter()
        .map(|signal| {
            (
                signal.port.clone(),
                if signal.receive { 1104 } else { 1105 },
            )
        })
        .collect();
    let report = workspace
        .edit_integration(IntegrationEdit {
            can_ids,
            application_period_ms: Some(20),
        })
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    let after = report.description.unwrap();
    assert_eq!(after.profile, autosar_config_core::integration::PROFILE);
    let mut ids: Vec<_> = after.signals.iter().map(|signal| signal.can_id).collect();
    ids.sort();
    assert_eq!(ids, [1104, 1105]);
    let application: Vec<_> = after
        .schedule
        .entities
        .iter()
        .filter(|entity| entity.application)
        .collect();
    assert!(!application.is_empty());
    assert!(application.iter().all(|entity| entity.period_ms == 20));
    for component in &after.composition.as_ref().unwrap().components {
        for runnable in &component.runnables {
            assert!(runnable.period_ms.is_none_or(|period| period == 20));
        }
    }
    // The template transmits from the Com Tx main function on the
    // application trigger, so the PDU period moves with it.
    let com = after.com_runtime.as_ref().unwrap();
    assert_eq!(com.transmit_main.period_ms, 20);
    assert_eq!(com.receive_main.period_ms, 1);
    assert!(
        after
            .signals
            .iter()
            .filter(|signal| !signal.receive)
            .all(|signal| signal.transmit_period_ms == Some(20))
    );
    let errors = workspace
        .edit_integration(IntegrationEdit {
            application_period_ms: Some(0),
            ..IntegrationEdit::default()
        })
        .err()
        .unwrap();
    assert!(errors.iter().any(|issue| issue.code == "EDIT_RANGE"));
}
