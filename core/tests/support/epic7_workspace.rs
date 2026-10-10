use super::workspace::Scratch;
use autosar_config_core::Workspace;
use autosar_config_core::project_model::*;
use std::fs;
use std::path::Path;

fn projection(workspace: &Workspace) -> ProjectProjection {
    workspace
        .project_projection(&workspace.input_fingerprint().unwrap())
        .unwrap()
}

fn batch(workspace: &Workspace, changes: Vec<ConfigurationChange>) -> ChangeSet {
    ChangeSet {
        workspace_epoch: projection(workspace).workspace_epoch,
        input_fingerprint: workspace.input_fingerprint().unwrap(),
        definition_fingerprint: workspace.definition_fingerprint().unwrap(),
        changes,
    }
}

fn value_change(field: &FieldDescriptor, id: &str, lexeme: &str) -> ConfigurationChange {
    ConfigurationChange::SetValue {
        change_id: id.into(),
        field: FieldRef::Existing {
            field_id: field.field_id.clone(),
        },
        expected: field.current.clone(),
        value: ValueState::Explicit {
            value: TypedValue {
                kind: field.definition.kind.unwrap(),
                lexeme: lexeme.into(),
            },
        },
    }
}

fn signals(directory: &Path, name: &str) -> Workspace {
    let preview = Workspace::preview_project_creation(directory, name, "can-signals-v1").unwrap();
    Workspace::create_project_previewed(&preview).unwrap()
}

#[test]
fn builtin_batch_preserves_existing_definition_violations_but_rejects_changed_witnesses() {
    let scratch = Scratch::new();
    let directory = scratch.0.join("Witness");
    let workspace = signals(&directory, "Witness");
    let original = projection(&workspace);
    let signal = original
        .objects
        .iter()
        .find(|object| {
            object.definition_id.as_deref() == Some("/AUTOSAR/EcucDefs/Com/ComConfig/ComSignal")
        })
        .unwrap();
    let path = &original.sources[0].path;
    let mut text = fs::read_to_string(path).unwrap();
    let document = roxmltree::Document::parse(&text).unwrap();
    let node = document
        .descendants()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                && node.children().any(|child| {
                    child.tag_name().name() == "SHORT-NAME"
                        && child.text() == Some(signal.short_name.as_str())
                })
        })
        .unwrap();
    let reference = document
        .descendants()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "ECUC-REFERENCE-VALUE"
                && node.children().any(|child| {
                    child.tag_name().name() == "VALUE-REF"
                        && child.text() == Some(signal.path.as_str())
                })
        })
        .unwrap();
    let node_range = node.range();
    let reference_range = reference.range();
    let mut nodes = String::new();
    let mut references = String::new();
    for name in ["OverlapB", "OverlapC"] {
        nodes.push_str(&text[node_range.clone()].replacen(
            &format!("<SHORT-NAME>{}</SHORT-NAME>", signal.short_name),
            &format!("<SHORT-NAME>{name}</SHORT-NAME>"),
            1,
        ));
        let parent = signal.path.rsplit_once('/').unwrap().0;
        references.push_str(
            &text[reference_range.clone()].replace(&signal.path, &format!("{parent}/{name}")),
        );
    }
    drop(document);
    let mut insertions = [(node_range.end, nodes), (reference_range.end, references)];
    insertions.sort_by_key(|(offset, _)| std::cmp::Reverse(*offset));
    for (offset, addition) in insertions {
        text.insert_str(offset, &addition);
    }
    fs::write(path, &text).unwrap();
    let mut workspace = Workspace::open_project_manifest(
        &directory.join("workbench-project.json"),
        &scratch.0.join("cache"),
    )
    .unwrap();
    let overlaps = |view: &ProjectProjection| {
        view.diagnostics
            .iter()
            .filter(|issue| issue.code == "CAN_SIGNAL_OVERLAP")
            .count()
    };
    let before = projection(&workspace);
    assert_eq!(overlaps(&before), 3);
    assert!(before.validation.iter().any(|scope| {
        scope.scope == ValidationScope::Schema && scope.status == ValidationStatus::Passed
    }));
    let initial = before
        .fields
        .iter()
        .find(|field| {
            field
                .definition
                .definition_id
                .ends_with("/ComSignalInitValue")
        })
        .unwrap();
    let unrelated = batch(&workspace, vec![value_change(initial, "initial", "17")]);
    let preview = workspace.prepare_change(&unrelated).unwrap();
    workspace
        .apply_change(&unrelated, &preview.change_revision)
        .unwrap();
    assert_eq!(overlaps(&projection(&workspace)), 3);
    let save = workspace.preview_save().unwrap();
    workspace.save_previewed(&save.revision).unwrap();

    let before = projection(&workspace);
    let saved = fs::read(path).unwrap();
    let changed = before
        .objects
        .iter()
        .find(|object| object.short_name == "OverlapC")
        .unwrap();
    let layout = |suffix: &str| {
        before
            .fields
            .iter()
            .find(|field| {
                field.object_id == changed.object_id
                    && field.definition.definition_id.ends_with(suffix)
            })
            .unwrap()
    };
    let replacement = batch(
        &workspace,
        vec![
            value_change(layout("/ComBitPosition"), "position", "1"),
            value_change(layout("/ComBitSize"), "length", "31"),
        ],
    );
    let error = workspace.prepare_change(&replacement).unwrap_err();
    assert_eq!(
        serde_json::to_value(&error).unwrap()[0]["params"]["code"],
        "CAN_SIGNAL_OVERLAP"
    );
    let error = workspace
        .apply_change(&replacement, "unissued-preview")
        .unwrap_err();
    assert_eq!(
        serde_json::to_value(&error).unwrap()[0]["params"]["code"],
        "CAN_SIGNAL_OVERLAP"
    );
    assert_eq!(
        serde_json::to_value(projection(&workspace)).unwrap(),
        serde_json::to_value(&before).unwrap()
    );
    assert_eq!(fs::read(path).unwrap(), saved);
    let removed = before
        .objects
        .iter()
        .find(|object| object.short_name == "OverlapB")
        .unwrap();
    let incoming = before
        .references
        .iter()
        .find(|edge| edge.target_id.as_deref() == Some(&removed.object_id))
        .unwrap();
    let field = before
        .fields
        .iter()
        .find(|field| field.field_id == incoming.field_id)
        .unwrap();
    let repair = batch(
        &workspace,
        vec![
            ConfigurationChange::SetReference {
                change_id: "remove-reference".into(),
                field: FieldRef::Existing {
                    field_id: field.field_id.clone(),
                },
                expected: field.reference.clone().unwrap(),
                value: ReferenceState::Absent,
            },
            ConfigurationChange::RemoveInstance {
                change_id: "partial-repair".into(),
                object: ObjectRef::Existing {
                    object_id: removed.object_id.clone(),
                },
                expected_short_name: removed.short_name.clone(),
            },
        ],
    );
    let preview = workspace.prepare_change(&repair).unwrap();
    workspace
        .apply_change(&repair, &preview.change_revision)
        .unwrap();
    assert_eq!(overlaps(&projection(&workspace)), 1);
    assert_eq!(fs::read(path).unwrap(), saved);
    let save = workspace.preview_save().unwrap();
    workspace.save_previewed(&save.revision).unwrap();
    assert!(!workspace.view().dirty);
    assert_ne!(fs::read(path).unwrap(), saved);
}

#[test]
fn builtin_source_projection_owns_bytes_and_changes_epoch_on_reopen() {
    let scratch = Scratch::new();
    let workspace = signals(&scratch.0.join("First"), "First");
    let view = projection(&workspace);
    let source = &view.sources[0];
    assert_eq!(
        workspace.source_text(&source.source_id).unwrap().as_bytes(),
        fs::read(&source.path).unwrap()
    );
    assert!(workspace.source_text(&source.path).is_err());
    let mut workspace = workspace;
    assert!(
        workspace
            .set_legacy_validation_schema(scratch.0.join("not-an-oracle.zip"))
            .is_err()
    );
    assert!(!workspace.uses_legacy_validation());
    let reopened = Workspace::open_project_manifest(
        &scratch.0.join("First/workbench-project.json"),
        &scratch.0.join("cache"),
    )
    .unwrap();
    let reopened_view = projection(&reopened);
    assert_ne!(view.workspace_epoch, reopened_view.workspace_epoch);
    assert!(reopened.source_text(&source.source_id).is_err());
    assert_ne!(
        view.objects[0].object_id,
        reopened_view.objects[0].object_id
    );
}

#[test]
fn builtin_batch_is_atomic_stale_safe_and_preserves_untouched_bytes() {
    let scratch = Scratch::new();
    let mut workspace = signals(&scratch.0.join("Batch"), "Batch");
    let original = projection(&workspace);
    let id = original
        .fields
        .iter()
        .find(|field| field.definition.definition_id.ends_with("/CanIfTxPduCanId"))
        .unwrap();
    let initial = original
        .fields
        .iter()
        .find(|field| {
            field
                .definition
                .definition_id
                .ends_with("/ComSignalInitValue")
        })
        .unwrap();
    let before = workspace
        .source_text(&original.sources[0].source_id)
        .unwrap();
    let valid = batch(
        &workspace,
        vec![
            value_change(id, "id", "802"),
            value_change(initial, "initial", "123456789"),
        ],
    );
    let preview = workspace.prepare_change(&valid).unwrap();
    assert_eq!(
        workspace.input_fingerprint().unwrap(),
        valid.input_fingerprint
    );
    assert_eq!(
        workspace
            .source_text(&original.sources[0].source_id)
            .unwrap(),
        before
    );
    assert_eq!(
        fs::read(&original.sources[0].path).unwrap(),
        before.as_bytes()
    );
    let mut modified = valid.clone();
    modified.changes[0] = value_change(id, "id", "803");
    assert!(
        workspace
            .apply_change(&modified, &preview.change_revision)
            .is_err()
    );
    assert_eq!(
        workspace
            .source_text(&original.sources[0].source_id)
            .unwrap(),
        before
    );
    let mut invalid = valid.clone();
    invalid.changes[0] = value_change(id, "id", "536870912");
    assert!(workspace.prepare_change(&invalid).is_err());
    assert_eq!(
        workspace.input_fingerprint().unwrap(),
        valid.input_fingerprint
    );
    let outcome = workspace
        .apply_change(&valid, &preview.change_revision)
        .unwrap();
    for (field, expected) in [(id, "802"), (initial, "123456789")] {
        let changed = outcome
            .projection
            .fields
            .iter()
            .find(|item| item.field_id == field.field_id)
            .unwrap();
        assert_eq!(
            changed.current,
            ValueState::Explicit {
                value: TypedValue {
                    kind: field.definition.kind.unwrap(),
                    lexeme: expected.into()
                }
            }
        );
        assert_eq!(changed.object_id, field.object_id);
    }
    assert_eq!(
        fs::read(&original.sources[0].path).unwrap(),
        before.as_bytes()
    );
    assert!(
        workspace
            .apply_change(&valid, &preview.change_revision)
            .is_err()
    );
    let save = workspace.preview_save().unwrap();
    workspace.save_previewed(&save.revision).unwrap();
    let reopened = Workspace::open_project_manifest(
        &scratch.0.join("Batch/workbench-project.json"),
        &scratch.0.join("cache"),
    )
    .unwrap();
    let actual = projection(&reopened);
    let saved = actual
        .fields
        .iter()
        .find(|field| {
            field
                .definition
                .definition_id
                .ends_with("/ComSignalInitValue")
        })
        .unwrap();
    assert_eq!(
        saved.current,
        ValueState::Explicit {
            value: TypedValue {
                kind: initial.definition.kind.unwrap(),
                lexeme: "123456789".into()
            }
        }
    );
}

#[test]
fn builtin_rename_updates_real_incoming_edges_without_rewriting_unrelated_source() {
    let scratch = Scratch::new();
    let mut workspace = signals(&scratch.0.join("Rename"), "Rename");
    let view = projection(&workspace);
    let signal = view
        .objects
        .iter()
        .find(|object| {
            object.definition_id.as_deref() == Some("/AUTOSAR/EcucDefs/Com/ComConfig/ComSignal")
        })
        .unwrap();
    let incoming: Vec<_> = view
        .references
        .iter()
        .filter(|edge| edge.target_id.as_deref() == Some(&signal.object_id))
        .collect();
    assert!(incoming.iter().any(|edge| edge.raw_path == signal.path));
    let rename = batch(
        &workspace,
        vec![ConfigurationChange::RenameInstance {
            change_id: "rename".into(),
            object: ObjectRef::Existing {
                object_id: signal.object_id.clone(),
            },
            expected_short_name: signal.short_name.clone(),
            short_name: "Counter".into(),
        }],
    );
    let before = workspace.source_text(&view.sources[0].source_id).unwrap();
    let preview = workspace.prepare_change(&rename).unwrap();
    let outcome = workspace
        .apply_change(&rename, &preview.change_revision)
        .unwrap();
    let renamed = outcome
        .projection
        .objects
        .iter()
        .find(|object| object.object_id == signal.object_id)
        .unwrap();
    assert_eq!(renamed.short_name, "Counter");
    for edge in incoming {
        let actual = outcome
            .projection
            .references
            .iter()
            .find(|item| item.field_id == edge.field_id)
            .unwrap();
        assert_eq!(actual.raw_path, renamed.path);
        assert_eq!(actual.target_id.as_deref(), Some(signal.object_id.as_str()));
    }
    assert_eq!(fs::read(&view.sources[0].path).unwrap(), before.as_bytes());
    // Renaming a configuration instance is not a global textual replacement of the system signal name.
    let after = workspace.source_text(&view.sources[0].source_id).unwrap();
    let before_document = roxmltree::Document::parse(&before).unwrap();
    let after_document = roxmltree::Document::parse(&after).unwrap();
    let names = |document: &roxmltree::Document<'_>| {
        document
            .descendants()
            .filter(|node| node.is_element() && node.tag_name().name() == "I-SIGNAL")
            .flat_map(|node| {
                node.children()
                    .filter(|node| node.tag_name().name() == "SHORT-NAME")
                    .filter_map(|node| node.text().map(str::to_string))
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(names(&before_document), names(&after_document));
}

#[test]
fn builtin_create_requires_complete_batch_and_deleted_ids_are_never_reused() {
    let scratch = Scratch::new();
    let mut workspace = signals(&scratch.0.join("Structure"), "Structure");
    let view = projection(&workspace);
    let parent = view
        .objects
        .iter()
        .find(|object| {
            object.definition_id.as_deref()
                == Some("/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection")
        })
        .unwrap();
    let create = ConfigurationChange::CreateInstance {
        change_id: "create".into(),
        parent: ObjectRef::Existing {
            object_id: parent.object_id.clone(),
        },
        source_id: parent.source_id.clone(),
        definition_id: "/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu".into(),
        short_name: "Spare".into(),
    };
    let incomplete = batch(&workspace, vec![create.clone()]);
    let original_fingerprint = workspace.input_fingerprint().unwrap();
    assert!(workspace.prepare_change(&incomplete).is_err());
    assert_eq!(workspace.input_fingerprint().unwrap(), original_fingerprint);
    let length = ConfigurationChange::SetValue {
        change_id: "length".into(),
        field: FieldRef::New {
            object: ObjectRef::Created {
                change_id: "create".into(),
            },
            definition_id: "/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu/PduLength"
                .into(),
            entry_key: "length".into(),
        },
        expected: ValueState::Absent,
        value: ValueState::Explicit {
            value: TypedValue {
                kind: ValueKind::Integer,
                lexeme: "4".into(),
            },
        },
    };
    let complete = batch(&workspace, vec![length.clone(), create.clone()]);
    let preview = workspace.prepare_change(&complete).unwrap();
    let outcome = workspace
        .apply_change(&complete, &preview.change_revision)
        .unwrap();
    let created = outcome
        .created_ids
        .iter()
        .find(|item| item.change_id == "create")
        .unwrap();
    let created_field = outcome
        .created_fields
        .iter()
        .find(|item| item.entry_key == "length")
        .unwrap();
    let actual = outcome
        .projection
        .fields
        .iter()
        .find(|field| field.field_id == created_field.field_id)
        .unwrap();
    assert_eq!(actual.object_id, created.object_id);
    assert_eq!(
        actual.current,
        ValueState::Explicit {
            value: TypedValue {
                kind: ValueKind::Integer,
                lexeme: "4".into()
            }
        }
    );
    let remove = batch(
        &workspace,
        vec![ConfigurationChange::RemoveInstance {
            change_id: "remove".into(),
            object: ObjectRef::Existing {
                object_id: created.object_id.clone(),
            },
            expected_short_name: "Spare".into(),
        }],
    );
    let preview = workspace.prepare_change(&remove).unwrap();
    let removed = workspace
        .apply_change(&remove, &preview.change_revision)
        .unwrap();
    assert!(
        !removed
            .projection
            .objects
            .iter()
            .any(|object| object.object_id == created.object_id)
    );
    let second = batch(&workspace, vec![create, length]);
    let preview = workspace.prepare_change(&second).unwrap();
    let recreated = workspace
        .apply_change(&second, &preview.change_revision)
        .unwrap();
    assert_ne!(recreated.created_ids[0].object_id, created.object_id);
    assert_ne!(recreated.created_fields[0].field_id, created_field.field_id);
}

#[test]
fn builtin_remove_rejects_incoming_references_and_batch_reference_can_target_created_instance() {
    let scratch = Scratch::new();
    let mut workspace = signals(&scratch.0.join("References"), "References");
    let view = projection(&workspace);
    let pdu = view
        .objects
        .iter()
        .find(|object| {
            object.definition_id.as_deref()
                == Some("/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu")
        })
        .unwrap();
    let parent = view
        .objects
        .iter()
        .find(|object| Some(&object.object_id) == pdu.parent_id.as_ref())
        .unwrap();
    let remove = ConfigurationChange::RemoveInstance {
        change_id: "remove".into(),
        object: ObjectRef::Existing {
            object_id: pdu.object_id.clone(),
        },
        expected_short_name: pdu.short_name.clone(),
    };
    assert!(
        workspace
            .prepare_change(&batch(&workspace, vec![remove.clone()]))
            .is_err()
    );
    let mut changes = vec![
        ConfigurationChange::CreateInstance {
            change_id: "new".into(),
            parent: ObjectRef::Existing {
                object_id: parent.object_id.clone(),
            },
            source_id: parent.source_id.clone(),
            definition_id: pdu.definition_id.clone().unwrap(),
            short_name: "Replacement".into(),
        },
        ConfigurationChange::SetValue {
            change_id: "length".into(),
            field: FieldRef::New {
                object: ObjectRef::Created {
                    change_id: "new".into(),
                },
                definition_id: format!("{}/PduLength", pdu.definition_id.as_ref().unwrap()),
                entry_key: "length".into(),
            },
            expected: ValueState::Absent,
            value: ValueState::Explicit {
                value: TypedValue {
                    kind: ValueKind::Integer,
                    lexeme: "4".into(),
                },
            },
        },
        remove,
    ];
    let path = format!("{}/Replacement", parent.path);
    for (index, edge) in view
        .references
        .iter()
        .filter(|edge| edge.target_id.as_deref() == Some(&pdu.object_id))
        .enumerate()
    {
        let field = view
            .fields
            .iter()
            .find(|field| field.field_id == edge.field_id)
            .unwrap();
        changes.push(ConfigurationChange::SetReference {
            change_id: format!("reference-{index}"),
            field: FieldRef::Existing {
                field_id: field.field_id.clone(),
            },
            expected: field.reference.clone().unwrap(),
            value: ReferenceState::Explicit {
                raw_path: path.clone(),
                dest: "ECUC-CONTAINER-VALUE".into(),
                target: Some(ObjectRef::Created {
                    change_id: "new".into(),
                }),
            },
        });
    }
    let changes = batch(&workspace, changes);
    let preview = workspace.prepare_change(&changes).unwrap();
    let result = workspace
        .apply_change(&changes, &preview.change_revision)
        .unwrap();
    let created = &result.created_ids[0].object_id;
    assert!(
        !result
            .projection
            .objects
            .iter()
            .any(|object| object.object_id == pdu.object_id)
    );
    for suffix in ["/CanIfTxPduRef", "/ComPduIdRef"] {
        let field = view
            .fields
            .iter()
            .find(|field| field.definition.definition_id.ends_with(suffix))
            .unwrap();
        let edge = result
            .projection
            .references
            .iter()
            .find(|edge| edge.field_id == field.field_id)
            .unwrap();
        assert_eq!(edge.raw_path, path);
        assert_eq!(edge.target_id.as_ref(), Some(created));
    }
}

#[test]
fn builtin_templates_are_previewed_portable_and_confirmation_rejects_tampering() {
    let scratch = Scratch::new();
    for (template, name) in [
        ("can-empty-v1", "Empty"),
        ("can-signals-v1", "Signals"),
        ("standard-ecu-v1", "Standard"),
    ] {
        let directory = scratch.0.join(name);
        let preview = Workspace::preview_project_creation(&directory, name, template).unwrap();
        assert!(!directory.exists());
        let mut altered = preview.clone();
        altered.files[0]
            .contents
            .push_str("\n<!-- changed outside trusted preview -->");
        assert!(Workspace::create_project_previewed(&altered).is_err());
        assert!(!directory.exists());
        let workspace = Workspace::create_project_previewed(&preview).unwrap();
        let manifest = workspace.project_manifest().unwrap();
        assert!(manifest.application_inputs.is_empty());
        assert!(manifest.accepted_extension_definitions.is_empty());
        for input in &manifest.inputs {
            assert!(!Path::new(&input.path).is_absolute());
            assert_eq!(
                fs::read(directory.join(&input.path)).unwrap(),
                preview
                    .files
                    .iter()
                    .find(|file| file.path == input.path)
                    .unwrap()
                    .contents
                    .as_bytes()
            );
        }
        let original_epoch = projection(&workspace).workspace_epoch;
        let moved = scratch.0.join(format!("Moved{name}"));
        fs::rename(&directory, &moved).unwrap();
        let reopened = Workspace::open_project_manifest(
            &moved.join("workbench-project.json"),
            &scratch.0.join("cache"),
        )
        .unwrap();
        assert_ne!(projection(&reopened).workspace_epoch, original_epoch);
        assert_eq!(reopened.project_manifest().unwrap().inputs, manifest.inputs);
        assert!(Workspace::preview_project_creation(&moved, name, template).is_err());
    }
}

#[test]
fn builtin_external_changes_and_manifest_escape_refuse_without_losing_owned_bytes() {
    let scratch = Scratch::new();
    let mut workspace = signals(&scratch.0.join("Safety"), "Safety");
    let view = projection(&workspace);
    let source = &view.sources[0];
    let owned = workspace.source_text(&source.source_id).unwrap();
    let save = workspace.preview_save().unwrap();
    fs::write(&source.path, format!("{owned}\n<!-- external -->")).unwrap();
    assert!(workspace.save_previewed(&save.revision).is_err());
    assert_eq!(workspace.source_text(&source.source_id).unwrap(), owned);
    assert_eq!(
        fs::read(&source.path).unwrap(),
        format!("{owned}\n<!-- external -->").as_bytes()
    );
    let manifest_path = scratch.0.join("Safety/workbench-project.json");
    let original_manifest = fs::read_to_string(&manifest_path).unwrap();
    let mut manifest: serde_json::Value = serde_json::from_str(&original_manifest).unwrap();
    manifest["inputs"][0]["path"] = "../Safety/Safety.arxml".into();
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(Workspace::open_project_manifest(&manifest_path, &scratch.0.join("cache")).is_err());
    assert_eq!(workspace.source_text(&source.source_id).unwrap(), owned);
}

#[test]
fn builtin_schema_failure_blocks_save_but_unknown_generation_does_not() {
    let scratch = Scratch::new();
    let directory = scratch.0.join("SavedUnknown");
    let workspace = signals(&directory, "SavedUnknown");
    let source = directory.join("SavedUnknown.arxml");
    let contents = fs::read_to_string(&source).unwrap();
    drop(workspace);
    let modified = contents.replacen("</ELEMENTS>", "<IMPLEMENTATION-DATA-TYPE><SHORT-NAME>Retained</SHORT-NAME><CATEGORY>VALUE</CATEGORY></IMPLEMENTATION-DATA-TYPE></ELEMENTS>", 1);
    fs::write(&source, &modified).unwrap();
    let mut workspace = Workspace::open(vec![source.clone()]).unwrap();
    let before = fs::read(&source).unwrap();
    let view = projection(&workspace);
    let field = view
        .fields
        .iter()
        .find(|field| {
            field
                .definition
                .definition_id
                .ends_with("/ComSignalInitValue")
        })
        .unwrap();
    let changes = batch(&workspace, vec![value_change(field, "repair", "17")]);
    let preview = workspace.prepare_change(&changes).unwrap();
    workspace
        .apply_change(&changes, &preview.change_revision)
        .unwrap();
    let save = workspace.preview_save().unwrap();
    workspace.save_previewed(&save.revision).unwrap();
    let after = fs::read_to_string(&source).unwrap();
    let before_doc = roxmltree::Document::parse(std::str::from_utf8(&before).unwrap()).unwrap();
    let after_doc = roxmltree::Document::parse(&after).unwrap();
    let retained_before = before_doc
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "IMPLEMENTATION-DATA-TYPE")
        .unwrap()
        .range();
    let retained_after = after_doc
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "IMPLEMENTATION-DATA-TYPE")
        .unwrap()
        .range();
    assert_eq!(&before[retained_before], &after.as_bytes()[retained_after]);
    let invalid = contents.replacen(
        "<SHORT-NAME>SavedUnknown</SHORT-NAME>",
        "<SHORT-NAME>bad-name</SHORT-NAME>",
        1,
    );
    fs::write(&source, &invalid).unwrap();
    let invalid_bytes = fs::read(&source).unwrap();
    let mut invalid_workspace = Workspace::open(vec![source.clone()]).unwrap();
    assert!(invalid_workspace.preview_save().is_err());
    assert_eq!(fs::read(&source).unwrap(), invalid_bytes);
}

fn author_extension(directory: &Path) -> std::path::PathBuf {
    use sha2::{Digest, Sha256};
    fs::create_dir(directory).unwrap();
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd">
<AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Vendor</SHORT-NAME><ELEMENTS>
<ECUC-MODULE-DEF><SHORT-NAME>Custom</SHORT-NAME><CONTAINERS>
<ECUC-PARAM-CONF-CONTAINER-DEF><SHORT-NAME>Config</SHORT-NAME><LOWER-MULTIPLICITY>1</LOWER-MULTIPLICITY><UPPER-MULTIPLICITY>1</UPPER-MULTIPLICITY><PARAMETERS>
<ECUC-INTEGER-PARAM-DEF><SHORT-NAME>Counter</SHORT-NAME><LOWER-MULTIPLICITY>1</LOWER-MULTIPLICITY><MIN>0</MIN><MAX>18446744073709551615</MAX></ECUC-INTEGER-PARAM-DEF>
<ECUC-FLOAT-PARAM-DEF><SHORT-NAME>Ratio</SHORT-NAME><MIN>0</MIN><MAX>100</MAX></ECUC-FLOAT-PARAM-DEF>
<ECUC-BOOLEAN-PARAM-DEF><SHORT-NAME>Enabled</SHORT-NAME><DEFAULT-VALUE>false</DEFAULT-VALUE></ECUC-BOOLEAN-PARAM-DEF>
<ECUC-STRING-PARAM-DEF><SHORT-NAME>Label</SHORT-NAME><DEFAULT-VALUE>not explicit</DEFAULT-VALUE></ECUC-STRING-PARAM-DEF>
<ECUC-FUNCTION-NAME-DEF><SHORT-NAME>Callback</SHORT-NAME></ECUC-FUNCTION-NAME-DEF>
<ECUC-ENUMERATION-PARAM-DEF><SHORT-NAME>Mode</SHORT-NAME><LITERALS><ECUC-ENUMERATION-LITERAL-DEF><SHORT-NAME>ONE</SHORT-NAME></ECUC-ENUMERATION-LITERAL-DEF><ECUC-ENUMERATION-LITERAL-DEF><SHORT-NAME>TWO</SHORT-NAME></ECUC-ENUMERATION-LITERAL-DEF></LITERALS></ECUC-ENUMERATION-PARAM-DEF>
</PARAMETERS></ECUC-PARAM-CONF-CONTAINER-DEF></CONTAINERS></ECUC-MODULE-DEF>
</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>
"#;
    fs::write(directory.join("custom.arxml"), xml).unwrap();
    let inventory = serde_json::json!({
        "formatVersion": 1, "catalogId": "vendor-custom", "release": "R24-11", "version": "1.0.0",
        "files": [{ "path": "custom.arxml", "sha256": format!("{:x}", Sha256::digest(xml.as_bytes())) }]
    });
    let path = directory.join("catalog.json");
    fs::write(&path, serde_json::to_vec_pretty(&inventory).unwrap()).unwrap();
    path
}

#[test]
fn builtin_workspace_types_defaults_and_exact_catalog_acceptance_roundtrip() {
    let scratch = Scratch::new();
    let directory = scratch.0.join("Typed");
    let mut workspace = signals(&directory, "Typed");
    let path = author_extension(&scratch.0.join("extension"));
    let cache = scratch.0.join("cache");
    let identity = workspace.accept_definition_catalog(&path, &cache).unwrap();
    assert!(projection(&workspace).dirty);
    let view = projection(&workspace);
    let package = view
        .objects
        .iter()
        .find(|object| object.kind == "AR-PACKAGE")
        .unwrap();
    let independent_definition = view
        .fields
        .iter()
        .find(|field| {
            field.definition.writable
                && field.definition.definition_id
                    == "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfTxPduCfg/CanIfTxPduCanId"
        })
        .unwrap()
        .definition
        .definition_id
        .clone();
    let mut changes = vec![
        ConfigurationChange::CreateInstance {
            change_id: "module".into(),
            parent: ObjectRef::Existing {
                object_id: package.object_id.clone(),
            },
            source_id: package.source_id.clone(),
            definition_id: "/Vendor/Custom".into(),
            short_name: "Custom".into(),
        },
        ConfigurationChange::CreateInstance {
            change_id: "config".into(),
            parent: ObjectRef::Created {
                change_id: "module".into(),
            },
            source_id: package.source_id.clone(),
            definition_id: "/Vendor/Custom/Config".into(),
            short_name: "Config".into(),
        },
    ];
    for (name, kind, lexeme) in [
        ("Counter", ValueKind::Integer, "18446744073709551615"),
        ("Ratio", ValueKind::Float, "0.125"),
        ("Callback", ValueKind::FunctionName, "Custom_Callback"),
        ("Mode", ValueKind::Enumeration, "TWO"),
    ] {
        changes.push(ConfigurationChange::SetValue {
            change_id: name.into(),
            field: FieldRef::New {
                object: ObjectRef::Created {
                    change_id: "config".into(),
                },
                definition_id: format!("/Vendor/Custom/Config/{name}"),
                entry_key: name.into(),
            },
            expected: ValueState::Absent,
            value: ValueState::Explicit {
                value: TypedValue {
                    kind,
                    lexeme: lexeme.into(),
                },
            },
        });
    }
    let complete = batch(&workspace, changes);
    let preview = workspace.prepare_change(&complete).unwrap();
    let outcome = workspace
        .apply_change(&complete, &preview.change_revision)
        .unwrap();
    let config_id = &outcome
        .created_ids
        .iter()
        .find(|item| item.change_id == "config")
        .unwrap()
        .object_id;
    let fields: Vec<_> = outcome
        .projection
        .fields
        .iter()
        .filter(|field| &field.object_id == config_id)
        .collect();
    let counter = fields
        .iter()
        .find(|field| field.definition.definition_id.ends_with("/Counter"))
        .unwrap();
    assert_eq!(
        counter.current,
        ValueState::Explicit {
            value: TypedValue {
                kind: ValueKind::Integer,
                lexeme: "18446744073709551615".into()
            }
        }
    );
    let label = fields
        .iter()
        .find(|field| field.definition.definition_id.ends_with("/Label"))
        .unwrap();
    let enabled = fields
        .iter()
        .find(|field| field.definition.definition_id.ends_with("/Enabled"))
        .unwrap();
    assert_eq!(label.current, ValueState::Absent);
    assert_eq!(
        label.definition.default_value,
        Some(TypedValue {
            kind: ValueKind::String,
            lexeme: "not explicit".into()
        })
    );
    assert_eq!(enabled.current, ValueState::Absent);
    let explicit = batch(
        &workspace,
        vec![
            value_change(label, "label", ""),
            value_change(enabled, "enabled", "true"),
        ],
    );
    let preview = workspace.prepare_change(&explicit).unwrap();
    let result = workspace
        .apply_change(&explicit, &preview.change_revision)
        .unwrap();
    assert_eq!(
        result
            .projection
            .fields
            .iter()
            .find(|field| field.field_id == label.field_id)
            .unwrap()
            .current,
        ValueState::Explicit {
            value: TypedValue {
                kind: ValueKind::String,
                lexeme: String::new()
            }
        }
    );
    let ratio = result
        .projection
        .fields
        .iter()
        .find(|field| field.definition.definition_id == "/Vendor/Custom/Config/Ratio")
        .unwrap();
    let callback = result
        .projection
        .fields
        .iter()
        .find(|field| field.definition.definition_id == "/Vendor/Custom/Config/Callback")
        .unwrap();
    let before_failure = workspace.input_fingerprint().unwrap();
    for invalid in [
        value_change(ratio, "invalid", "NaN"),
        value_change(callback, "invalid", "callback()"),
    ] {
        let invalid = batch(
            &workspace,
            vec![
                value_change(counter, "valid-first", "9007199254740993"),
                invalid,
            ],
        );
        assert!(workspace.prepare_change(&invalid).is_err());
        assert_eq!(workspace.input_fingerprint().unwrap(), before_failure);
    }
    let save = workspace.preview_save().unwrap();
    workspace.save_previewed(&save.revision).unwrap();
    let manifest_path = directory.join("workbench-project.json");
    let restored = Workspace::open_project_manifest(&manifest_path, &cache).unwrap();
    assert_eq!(
        projection(&restored).accepted_extension_definitions,
        vec![identity.clone()]
    );
    assert!(
        restored
            .definition_catalog()
            .get("/Vendor/Custom/Config/Counter")
            .is_some()
    );
    let missing =
        Workspace::open_project_manifest(&manifest_path, &scratch.0.join("missing-cache")).unwrap();
    assert_eq!(
        projection(&missing).accepted_extension_definitions,
        vec![identity.clone()]
    );
    assert!(
        missing
            .definition_catalog()
            .get("/Vendor/Custom/Config/Counter")
            .is_none()
    );
    assert!(
        projection(&missing)
            .fields
            .iter()
            .find(|field| field.definition.definition_id == independent_definition)
            .unwrap()
            .definition
            .writable
    );
    let source_before_remove = fs::read(directory.join("Typed.arxml")).unwrap();
    workspace
        .remove_definition_catalog(&identity.catalog_id)
        .unwrap();
    assert!(projection(&workspace).dirty);
    assert_eq!(
        fs::read(directory.join("Typed.arxml")).unwrap(),
        source_before_remove
    );
    let save = workspace.preview_save().unwrap();
    workspace.save_previewed(&save.revision).unwrap();
    let reopened = Workspace::open_project_manifest(&manifest_path, &cache).unwrap();
    assert!(
        projection(&reopened)
            .accepted_extension_definitions
            .is_empty()
    );
    assert!(
        reopened
            .definition_catalog()
            .get("/Vendor/Custom/Config/Counter")
            .is_none()
    );
}

#[test]
fn builtin_save_as_preserves_original_sources_and_rejects_name_collisions() {
    let scratch = Scratch::new();
    let original = signals(&scratch.0.join("Original"), "Original");
    let source = scratch.0.join("Original/Original.arxml");
    let before = fs::read(&source).unwrap();
    let direct = Workspace::open(vec![source.clone()]).unwrap();
    let preview = direct
        .preview_save_as_project(&scratch.0.join("Portable"), "Portable")
        .unwrap();
    assert!(!scratch.0.join("Portable").exists());
    let portable = direct.save_as_project_previewed(&preview).unwrap();
    assert_eq!(fs::read(&source).unwrap(), before);
    assert_eq!(
        portable
            .source_text(&projection(&portable).sources[0].source_id)
            .unwrap()
            .as_bytes(),
        before
    );
    assert_ne!(
        projection(&portable).workspace_epoch,
        projection(&original).workspace_epoch
    );
    fs::create_dir(scratch.0.join("Other")).unwrap();
    let other = scratch.0.join("Other/Original.arxml");
    fs::write(&other, r#"<?xml version="1.0"?><AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Other</SHORT-NAME><ELEMENTS/></AR-PACKAGE></AR-PACKAGES></AUTOSAR>"#).unwrap();
    let collision = Workspace::open(vec![source.clone(), other]).unwrap();
    assert!(
        collision
            .preview_save_as_project(&scratch.0.join("Collision"), "Collision")
            .is_err()
    );
    assert!(!scratch.0.join("Collision").exists());
    assert_eq!(fs::read(&source).unwrap(), before);
}

#[test]
fn builtin_cross_file_batch_keeps_members_and_untouched_original_bytes() {
    let scratch = Scratch::new();
    let directory = scratch.0.join("Split");
    let preview =
        Workspace::preview_project_creation(&directory, "Split", "standard-ecu-v1").unwrap();
    Workspace::create_project_previewed(&preview).unwrap();
    let ecuc = directory.join("ecuc.arxml");
    let original = fs::read_to_string(&ecuc).unwrap();
    let document = roxmltree::Document::parse(&original).unwrap();
    let module = document
        .descendants()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
                && node.children().any(|node| {
                    node.tag_name().name() == "SHORT-NAME" && node.text() == Some("CanIf")
                })
        })
        .unwrap();
    let range = module.range();
    let part = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Configuration</SHORT-NAME><ELEMENTS>{}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>"#,
        &original[range.clone()]
    );
    let mut remainder = original.clone();
    remainder.replace_range(range, "");
    fs::write(&ecuc, remainder).unwrap();
    fs::write(directory.join("canif.arxml"), part).unwrap();
    let manifest_path = directory.join("workbench-project.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["inputs"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"path": "canif.arxml", "roleHint": "ecuc_values"}));
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let mut workspace =
        Workspace::open_project_manifest(&manifest_path, &scratch.0.join("cache")).unwrap();
    let view = projection(&workspace);
    let com = view
        .fields
        .iter()
        .find(|field| {
            field
                .definition
                .definition_id
                .ends_with("/ComSignalInitValue")
        })
        .unwrap();
    let canif = view
        .fields
        .iter()
        .find(|field| field.definition.definition_id.ends_with("/CanIfInitCfgSet"))
        .unwrap();
    let com_source = &view
        .objects
        .iter()
        .find(|object| object.object_id == com.object_id)
        .unwrap()
        .source_id;
    let canif_source = &view
        .objects
        .iter()
        .find(|object| object.object_id == canif.object_id)
        .unwrap()
        .source_id;
    assert_ne!(com_source, canif_source);
    let before: std::collections::BTreeMap<_, _> = view
        .sources
        .iter()
        .map(|source| (source.source_id.clone(), fs::read(&source.path).unwrap()))
        .collect();
    let manifest_before = fs::read(&manifest_path).unwrap();
    let changes = batch(
        &workspace,
        vec![
            value_change(com, "com", "77"),
            value_change(canif, "canif", "RenamedSet"),
        ],
    );
    let preview = workspace.prepare_change(&changes).unwrap();
    let outcome = workspace
        .apply_change(&changes, &preview.change_revision)
        .unwrap();
    for (field, expected) in [(com, "77"), (canif, "RenamedSet")] {
        assert_eq!(
            outcome
                .projection
                .fields
                .iter()
                .find(|actual| actual.field_id == field.field_id)
                .unwrap()
                .current,
            ValueState::Explicit {
                value: TypedValue {
                    kind: field.definition.kind.unwrap(),
                    lexeme: expected.into()
                }
            }
        );
    }
    for source in &view.sources {
        assert_eq!(fs::read(&source.path).unwrap(), before[&source.source_id]);
    }
    let save = workspace.preview_save().unwrap();
    workspace.save_previewed(&save.revision).unwrap();
    for source in view
        .sources
        .iter()
        .filter(|source| source.source_id != *com_source && source.source_id != *canif_source)
    {
        assert_eq!(fs::read(&source.path).unwrap(), before[&source.source_id]);
    }
    assert_eq!(fs::read(&manifest_path).unwrap(), manifest_before);
    let reopened =
        Workspace::open_project_manifest(&manifest_path, &scratch.0.join("cache")).unwrap();
    let reopened_view = projection(&reopened);
    for (definition, expected) in [
        (&com.definition.definition_id, "77"),
        (&canif.definition.definition_id, "RenamedSet"),
    ] {
        let actual = reopened_view
            .fields
            .iter()
            .find(|field| &field.definition.definition_id == definition)
            .unwrap();
        assert_eq!(
            actual.current,
            ValueState::Explicit {
                value: TypedValue {
                    kind: actual.definition.kind.unwrap(),
                    lexeme: expected.into()
                }
            }
        );
    }
}

#[test]
fn builtin_duplicate_source_paths_keep_distinct_owned_identities_across_refresh() {
    let scratch = Scratch::new();
    let source = scratch.0.join("duplicates.arxml");
    let xml = r#"<AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Duplicates</SHORT-NAME><ELEMENTS><IMPLEMENTATION-DATA-TYPE><SHORT-NAME>Retained</SHORT-NAME><CATEGORY>VALUE</CATEGORY></IMPLEMENTATION-DATA-TYPE><IMPLEMENTATION-DATA-TYPE><SHORT-NAME>Retained</SHORT-NAME><CATEGORY>VALUE</CATEGORY></IMPLEMENTATION-DATA-TYPE></ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>"#;
    fs::write(&source, xml).unwrap();
    let mut workspace = Workspace::open(vec![source.clone()]).unwrap();
    let first = projection(&workspace);
    let duplicated: Vec<_> = first
        .objects
        .iter()
        .filter(|object| object.path == "/Duplicates/Retained")
        .collect();
    assert_eq!(duplicated.len(), 2);
    assert_ne!(duplicated[0].object_id, duplicated[1].object_id);
    workspace.validate().unwrap();
    let refreshed = projection(&workspace);
    for object in duplicated {
        let actual = refreshed
            .objects
            .iter()
            .find(|item| item.object_id == object.object_id)
            .unwrap();
        assert_eq!(actual.path, object.path);
        assert_eq!(actual.source_id, object.source_id);
        assert!(!actual.writable);
    }
    assert_eq!(
        workspace.source_text(&first.sources[0].source_id).unwrap(),
        xml
    );
    assert_eq!(fs::read_to_string(&source).unwrap(), xml);
}

#[test]
fn builtin_split_packages_allow_owned_creation_but_refuse_ambiguous_identity_changes() {
    let scratch = Scratch::new();
    let first_path = scratch.0.join("first.arxml");
    let second_path = scratch.0.join("second.arxml");
    let xml = r#"<AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Shared</SHORT-NAME><ELEMENTS/></AR-PACKAGE></AR-PACKAGES></AUTOSAR>"#;
    fs::write(&first_path, xml).unwrap();
    fs::write(&second_path, xml).unwrap();
    let mut workspace = Workspace::open(vec![first_path.clone(), second_path.clone()]).unwrap();
    let view = projection(&workspace);
    let packages = view
        .objects
        .iter()
        .filter(|object| object.path == "/Shared")
        .collect::<Vec<_>>();
    assert_eq!(packages.len(), 2);
    assert!(
        packages.iter().all(|object| object.writable),
        "{packages:?}"
    );
    assert_ne!(packages[0].object_id, packages[1].object_id);
    let parent = packages
        .iter()
        .find(|object| {
            object.source_id
                == view
                    .sources
                    .iter()
                    .find(|source| source.path == first_path)
                    .unwrap()
                    .source_id
        })
        .unwrap();
    for change in [
        ConfigurationChange::RenameInstance {
            change_id: "ambiguous-rename".into(),
            object: ObjectRef::Existing {
                object_id: parent.object_id.clone(),
            },
            expected_short_name: "Shared".into(),
            short_name: "Renamed".into(),
        },
        ConfigurationChange::RemoveInstance {
            change_id: "ambiguous-remove".into(),
            object: ObjectRef::Existing {
                object_id: parent.object_id.clone(),
            },
            expected_short_name: "Shared".into(),
        },
    ] {
        assert!(
            workspace
                .prepare_change(&batch(&workspace, vec![change]))
                .is_err()
        );
    }
    let create = ConfigurationChange::CreateInstance {
        change_id: "owned-component".into(),
        parent: ObjectRef::Existing {
            object_id: parent.object_id.clone(),
        },
        source_id: parent.source_id.clone(),
        definition_id: "APPLICATION-SW-COMPONENT-TYPE".into(),
        short_name: "LocalComponent".into(),
    };
    let mut mismatched = create.clone();
    if let ConfigurationChange::CreateInstance { source_id, .. } = &mut mismatched {
        *source_id = packages
            .iter()
            .find(|object| object.source_id != parent.source_id)
            .unwrap()
            .source_id
            .clone();
    }
    assert!(
        workspace
            .prepare_change(&batch(&workspace, vec![mismatched]))
            .is_err()
    );
    let changes = batch(&workspace, vec![create]);
    let preview = workspace.prepare_change(&changes).unwrap();
    workspace
        .apply_change(&changes, &preview.change_revision)
        .unwrap();
    let save = workspace.preview_save().unwrap();
    workspace.save_previewed(&save.revision).unwrap();
    assert_eq!(fs::read_to_string(&second_path).unwrap(), xml);
    let first_bytes = fs::read_to_string(&first_path).unwrap();
    assert!(first_bytes.contains("<APPLICATION-SW-COMPONENT-TYPE><SHORT-NAME>LocalComponent</SHORT-NAME></APPLICATION-SW-COMPONENT-TYPE>"));
    let reopened = Workspace::open(vec![first_path.clone(), second_path]).unwrap();
    let reopened_view = projection(&reopened);
    let component = reopened_view
        .objects
        .iter()
        .find(|object| object.path == "/Shared/LocalComponent")
        .unwrap();
    assert_eq!(
        component.source_id,
        reopened_view
            .sources
            .iter()
            .find(|source| source.path == first_path)
            .unwrap()
            .source_id
    );
}

#[test]
fn builtin_application_initialization_is_reviewed_create_only_and_owns_real_live_bytes() {
    let scratch = Scratch::new();
    let directory = scratch.0.join("ApplicationOwned");
    let creation =
        Workspace::preview_project_creation(&directory, "ApplicationOwned", "standard-ecu-v1")
            .unwrap();
    let mut workspace = Workspace::create_project_previewed(&creation).unwrap();
    let before = projection(&workspace);
    let original_sources: Vec<_> = before
        .sources
        .iter()
        .map(|source| (source.path.clone(), fs::read(&source.path).unwrap()))
        .collect();
    let old_identity = workspace.input_fingerprint().unwrap();
    let preview = workspace.preview_application_initialization().unwrap();
    assert_eq!(preview.slots.len(), 1);
    let application = directory.join(&preview.slots[0].source_paths[0]);
    let mut altered = preview.clone();
    altered.files[0].contents = "/* untrusted replacement */\n".into();
    assert!(
        workspace
            .initialize_application_previewed(&altered)
            .is_err()
    );
    assert!(!application.exists());
    assert_eq!(workspace.input_fingerprint().unwrap(), old_identity);
    let outcome = workspace
        .initialize_application_previewed(&preview)
        .unwrap();
    assert!(!outcome.projection.dirty);
    assert_eq!(outcome.projection.workspace_epoch, before.workspace_epoch);
    assert_ne!(workspace.input_fingerprint().unwrap(), old_identity);
    for (path, bytes) in original_sources {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    let snapshot = workspace.generation_snapshot().unwrap();
    assert_eq!(
        snapshot.manifest.application_inputs,
        vec![autosar_config_core::arxml::ApplicationInput {
            path: preview.slots[0].source_paths[0].clone(),
            producer_slot: "epic4-single-application-v1".into(),
        }]
    );
    assert_eq!(
        snapshot.manifest_bytes,
        fs::read(directory.join("workbench-project.json")).unwrap()
    );
    let mut user_bytes = fs::read(&application).unwrap();
    user_bytes
        .extend_from_slice(b"\n/* Live user code: preserve through delivery and re-open. */\n");
    fs::write(&application, &user_bytes).unwrap();
    assert!(workspace.generation_snapshot().is_err());
    assert!(
        workspace
            .initialize_application_previewed(&preview)
            .is_err()
    );
    assert_eq!(fs::read(&application).unwrap(), user_bytes);
    let restored = Workspace::open_project_manifest(
        &directory.join("workbench-project.json"),
        &scratch.0.join("cache"),
    )
    .unwrap();
    let current = restored.generation_snapshot().unwrap();
    assert_eq!(current.applications[0].bytes, user_bytes);
    assert!(restored.preview_application_initialization().is_err());
}

#[test]
fn builtin_application_initialization_refuses_a_late_user_path_without_touching_membership() {
    let scratch = Scratch::new();
    let directory = scratch.0.join("ApplicationCollision");
    let creation =
        Workspace::preview_project_creation(&directory, "ApplicationCollision", "standard-ecu-v1")
            .unwrap();
    let mut workspace = Workspace::create_project_previewed(&creation).unwrap();
    let preview = workspace.preview_application_initialization().unwrap();
    let manifest = directory.join("workbench-project.json");
    let original_manifest = fs::read(&manifest).unwrap();
    let before = workspace.input_fingerprint().unwrap();
    assert_eq!(preview.slots.len(), 1);
    let application = directory.join(&preview.slots[0].source_paths[0]);
    fs::create_dir_all(application.parent().unwrap()).unwrap();
    let user_bytes = b"/* This pre-existing source is owned by the user, not the initializer. */\n";
    fs::write(&application, user_bytes).unwrap();
    assert!(
        workspace
            .initialize_application_previewed(&preview)
            .is_err()
    );
    assert_eq!(fs::read(&application).unwrap(), user_bytes);
    assert_eq!(fs::read(&manifest).unwrap(), original_manifest);
    assert_eq!(workspace.input_fingerprint().unwrap(), before);
    assert!(
        workspace
            .project_manifest()
            .unwrap()
            .application_inputs
            .is_empty()
    );
}

#[test]
fn builtin_unsupported_ecuc_edition_keeps_owned_bytes_and_only_that_module_readonly() {
    let scratch = Scratch::new();
    let directory = scratch.0.join("WrongEdition");
    let workspace = signals(&directory, "WrongEdition");
    let initial = projection(&workspace);
    let editable = initial
        .objects
        .iter()
        .find(|object| {
            object.kind == "ECUC-MODULE-CONFIGURATION-VALUES"
                && initial.fields.iter().any(|field| {
                    field.definition.writable
                        && field.definition.kind == Some(ValueKind::Boolean)
                        && initial.objects.iter().any(|owner| {
                            owner.object_id == field.object_id
                                && owner.path.starts_with(&format!("{}/", object.path))
                        })
                })
        })
        .unwrap();
    let module_name = editable.short_name.clone();
    let source = directory.join("WrongEdition.arxml");
    let original = fs::read_to_string(&source).unwrap();
    let document = roxmltree::Document::parse(&original).unwrap();
    let module = document
        .descendants()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
                && node.children().any(|child| {
                    child.is_element()
                        && child.tag_name().name() == "SHORT-NAME"
                        && child.text() == Some(module_name.as_str())
                })
        })
        .unwrap();
    let edition = module
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "ECUC-DEF-EDITION");
    let range = edition.map(|node| node.range()).unwrap_or_else(|| {
        let end = module
            .children()
            .find(|node| node.is_element() && node.tag_name().name() == "DEFINITION-REF")
            .unwrap()
            .range()
            .end;
        end..end
    });
    let mut changed = original.clone();
    changed.replace_range(range, "<ECUC-DEF-EDITION>4.9.0</ECUC-DEF-EDITION>");
    fs::write(&source, &changed).unwrap();
    drop(workspace);
    let owned = Workspace::open(vec![source.clone()]).unwrap();
    let view = projection(&owned);
    let module = view
        .objects
        .iter()
        .find(|object| {
            object.kind == "ECUC-MODULE-CONFIGURATION-VALUES" && object.short_name == module_name
        })
        .unwrap();
    assert!(!module.writable);
    let descendant = format!("{}/", module.path);
    let blocked: std::collections::BTreeSet<_> = view
        .objects
        .iter()
        .filter(|object| object.path == module.path || object.path.starts_with(&descendant))
        .map(|object| object.object_id.as_str())
        .collect();
    let field = view
        .fields
        .iter()
        .find(|field| {
            blocked.contains(field.object_id.as_str())
                && field.definition.kind == Some(ValueKind::Boolean)
        })
        .unwrap();
    assert!(
        view.fields
            .iter()
            .filter(|field| blocked.contains(field.object_id.as_str()))
            .all(|field| !field.definition.writable)
    );
    assert!(
        view.fields
            .iter()
            .any(|field| !blocked.contains(field.object_id.as_str()) && field.definition.writable)
    );
    let changed_value = match &field.current {
        ValueState::Explicit { value } if matches!(value.lexeme.as_str(), "true" | "1") => "false",
        _ => "true",
    };
    let invalid = batch(
        &owned,
        vec![value_change(field, "unsupported-edition", changed_value)],
    );
    assert!(owned.prepare_change(&invalid).is_err());
    assert_eq!(
        owned.source_text(&view.sources[0].source_id).unwrap(),
        changed
    );
    assert_eq!(fs::read(source).unwrap(), changed.as_bytes());
}

#[test]
fn builtin_live_application_reopens_generic_configuration_without_authorizing_target_generation() {
    let scratch = Scratch::new();
    let directory = scratch.0.join("GenericLive");
    let creation =
        Workspace::preview_project_creation(&directory, "GenericLive", "standard-ecu-v1").unwrap();
    let mut workspace = Workspace::create_project_previewed(&creation).unwrap();
    let initialization = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&initialization)
        .unwrap();
    let before = workspace.generation_snapshot().unwrap();
    let view = projection(&workspace);
    let field = view
        .fields
        .iter()
        .find(|field| {
            field
                .definition
                .definition_id
                .ends_with("/OsScalabilityClass")
        })
        .unwrap();
    let changes = batch(
        &workspace,
        vec![value_change(field, "generic-class", "SC2")],
    );
    let preview = workspace.prepare_change(&changes).unwrap();
    workspace
        .apply_change(&changes, &preview.change_revision)
        .unwrap();
    let save = workspace.preview_save().unwrap();
    workspace.save_previewed(&save.revision).unwrap();
    let expected = workspace.generation_snapshot().unwrap();
    assert_eq!(expected.applications[0].bytes, before.applications[0].bytes);
    let reopened = Workspace::open_project_manifest(
        &directory.join("workbench-project.json"),
        &scratch.0.join("cache"),
    )
    .unwrap();
    let actual = reopened.generation_snapshot().unwrap();
    assert_eq!(actual.manifest, expected.manifest);
    assert_eq!(actual.manifest_bytes, expected.manifest_bytes);
    assert_eq!(actual.applications[0].bytes, expected.applications[0].bytes);
    for (actual, expected) in actual.inputs.iter().zip(&expected.inputs) {
        assert_eq!(actual.logical_path, expected.logical_path);
        assert_eq!(actual.bytes, expected.bytes);
    }
    assert_eq!(actual.inputs.len(), expected.inputs.len());
    let view = projection(&reopened);
    let field = view
        .fields
        .iter()
        .find(|field| {
            field
                .definition
                .definition_id
                .ends_with("/OsScalabilityClass")
        })
        .unwrap();
    assert!(matches!(&field.current, ValueState::Explicit { value } if value.lexeme == "SC2"));
    assert_eq!(
        view.validation
            .iter()
            .find(|scope| scope.scope == ValidationScope::Definition)
            .unwrap()
            .status,
        ValidationStatus::Unsupported,
    );
    let definition = view
        .validation
        .iter()
        .find(|scope| scope.scope == ValidationScope::Definition)
        .unwrap();
    assert!(definition.diagnostics.is_empty());
    assert!(definition.coverage.iter().any(|rule| rule.rule_id
        == "native.definition.legacy-dcm-mode-dependency"
        && !rule.supported));
    let target = view
        .validation
        .iter()
        .find(|scope| scope.scope == ValidationScope::TargetGeneration)
        .unwrap();
    assert_ne!(target.status, ValidationStatus::Passed);
    let runtime = autosar_config_core::integration::RuntimeCatalog::embedded().unwrap();
    assert!(reopened.integration_plan(&runtime).is_err());
    assert!(reopened.saved_integration_plan(&runtime).is_err());
}

#[test]
fn builtin_live_application_reopen_refuses_membership_and_producer_drift() {
    let scratch = Scratch::new();
    let directory = scratch.0.join("LiveOwnership");
    let creation =
        Workspace::preview_project_creation(&directory, "LiveOwnership", "standard-ecu-v1")
            .unwrap();
    let mut workspace = Workspace::create_project_previewed(&creation).unwrap();
    let initialization = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&initialization)
        .unwrap();
    let snapshot = workspace.generation_snapshot().unwrap();
    let manifest_path = directory.join("workbench-project.json");
    let open = || Workspace::open_project_manifest(&manifest_path, &scratch.0.join("cache"));
    assert!(open().is_ok());
    let mut foreign_path = snapshot.manifest.clone();
    foreign_path.application_inputs[0].path = "application/Foreign.c".into();
    fs::write(
        directory.join("application/Foreign.c"),
        &snapshot.applications[0].bytes,
    )
    .unwrap();
    let mut foreign_slot = snapshot.manifest.clone();
    foreign_slot.application_inputs[0].producer_slot = "foreign-producer".into();
    let mut duplicate = snapshot.manifest.clone();
    duplicate
        .application_inputs
        .push(duplicate.application_inputs[0].clone());
    let mut missing = snapshot.manifest.clone();
    missing.application_inputs[0].path = "application/Missing.c".into();
    for manifest in [foreign_path, foreign_slot, duplicate, missing] {
        let bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        fs::write(&manifest_path, &bytes).unwrap();
        assert!(open().is_err());
        assert_eq!(fs::read(&manifest_path).unwrap(), bytes);
        assert_eq!(
            fs::read(&snapshot.applications[0].disk_path).unwrap(),
            snapshot.applications[0].bytes
        );
    }
    fs::write(&manifest_path, &snapshot.manifest_bytes).unwrap();
    let component = snapshot
        .inputs
        .iter()
        .find(|source| {
            std::str::from_utf8(&source.bytes)
                .unwrap()
                .contains("<APPLICATION-SW-COMPONENT-TYPE>")
        })
        .unwrap();
    let changed = std::str::from_utf8(&component.bytes).unwrap().replace(
        "APPLICATION-SW-COMPONENT-TYPE",
        "SENSOR-ACTUATOR-SW-COMPONENT-TYPE",
    );
    assert_ne!(changed.as_bytes(), component.bytes);
    fs::write(&component.disk_path, &changed).unwrap();
    assert!(open().is_err());
    assert_eq!(fs::read(&component.disk_path).unwrap(), changed.as_bytes());
    assert_eq!(
        fs::read(&snapshot.applications[0].disk_path).unwrap(),
        snapshot.applications[0].bytes
    );
}

#[test]
fn builtin_remove_preserves_unsupported_subtrees_and_attributes_but_allows_known_edits() {
    let scratch = Scratch::new();
    for (case, child, attribute) in [
        ("Supported", "", ""),
        ("UnknownChild", "<VENDOR-DATA>calibration</VENDOR-DATA>", ""),
        (
            "UnknownAttribute",
            "",
            " VENDOR-CALIBRATION=\"calibration\"",
        ),
    ] {
        let directory = scratch.0.join(case);
        let workspace = signals(&directory, case);
        let view = projection(&workspace);
        let source = &view.sources[0];
        let original = workspace.source_text(&source.source_id).unwrap();
        let document = roxmltree::Document::parse(&original).unwrap();
        let node = document
            .descendants()
            .find(|node| {
                node.is_element()
                    && node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                    && node.children().any(|child| {
                        child.tag_name().name() == "DEFINITION-REF"
                            && child.text()
                                == Some(
                                    "/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu",
                                )
                    })
            })
            .unwrap();
        let name = node
            .children()
            .find(|child| child.tag_name().name() == "SHORT-NAME")
            .unwrap();
        let mut spare = original[node.range()].to_string();
        spare.replace_range(
            name.range().start - node.range().start..name.range().end - node.range().start,
            "<SHORT-NAME>Spare</SHORT-NAME>",
        );
        let end = spare.rfind("</").unwrap();
        spare.insert_str(end, child);
        let start = spare.find('>').unwrap();
        spare.insert_str(start, attribute);
        let mut raw = original.clone();
        raw.insert_str(node.range().end, &spare);
        fs::write(&source.path, &raw).unwrap();
        let mut workspace = Workspace::open_project_manifest(
            &directory.join("workbench-project.json"),
            &scratch.0.join("cache"),
        )
        .unwrap();
        let view = projection(&workspace);
        let owner = view
            .objects
            .iter()
            .find(|object| object.short_name == "Spare")
            .unwrap();
        let remove = batch(
            &workspace,
            vec![ConfigurationChange::RemoveInstance {
                change_id: "remove-spare".into(),
                object: ObjectRef::Existing {
                    object_id: owner.object_id.clone(),
                },
                expected_short_name: owner.short_name.clone(),
            }],
        );
        if child.is_empty() && attribute.is_empty() {
            let preview = workspace.prepare_change(&remove).unwrap();
            let outcome = workspace
                .apply_change(&remove, &preview.change_revision)
                .unwrap();
            assert!(
                !outcome
                    .projection
                    .objects
                    .iter()
                    .any(|object| object.object_id == owner.object_id)
            );
            assert_eq!(
                workspace.source_text(&view.sources[0].source_id).unwrap(),
                original
            );
        } else {
            assert!(view.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "NATIVE_UNSUPPORTED"
                    && diagnostic.object_id.as_deref() == Some(owner.object_id.as_str())
            }));
            assert!(workspace.prepare_change(&remove).is_err());
            assert!(workspace.apply_change(&remove, "unreviewed").is_err());
            assert_eq!(
                workspace.input_fingerprint().unwrap(),
                remove.input_fingerprint
            );
            assert_eq!(
                workspace.source_text(&view.sources[0].source_id).unwrap(),
                raw
            );
            let field = view
                .fields
                .iter()
                .find(|field| {
                    field.object_id == owner.object_id
                        && field.definition.definition_id.ends_with("/PduLength")
                })
                .unwrap();
            let ValueState::Explicit { value } = &field.current else {
                panic!("The cloned PDU must retain its explicit length.");
            };
            let length = if value.lexeme == "5" { "6" } else { "5" };
            let expected_spare = spare.replace(
                &format!("<VALUE>{}</VALUE>", value.lexeme),
                &format!("<VALUE>{length}</VALUE>"),
            );
            assert_ne!(expected_spare, spare);
            let changes = batch(
                &workspace,
                vec![value_change(field, "known-length", length)],
            );
            let preview = workspace.prepare_change(&changes).unwrap();
            workspace
                .apply_change(&changes, &preview.change_revision)
                .unwrap();
            let edited = workspace.source_text(&view.sources[0].source_id).unwrap();
            assert!(edited.contains(&expected_spare));
        }
        assert_eq!(fs::read(&source.path).unwrap(), raw.as_bytes());
    }
}
