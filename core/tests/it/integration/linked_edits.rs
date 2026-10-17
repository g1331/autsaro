use super::*;
use autosar_config_core::{Workspace, project_model::*};

fn projection(workspace: &Workspace) -> ProjectProjection {
    workspace
        .project_projection(&workspace.input_fingerprint().unwrap())
        .unwrap()
}

fn change(view: &ProjectProjection, field: &FieldDescriptor, lexeme: &str) -> ChangeSet {
    ChangeSet {
        workspace_epoch: view.workspace_epoch.clone(),
        input_fingerprint: view.input_fingerprint.clone(),
        definition_fingerprint: view.definition_fingerprint.clone(),
        changes: vec![ConfigurationChange::SetValue {
            change_id: field.field_id.clone(),
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
        }],
    }
}

#[test]
fn linked_grid_changes_cover_can_and_period_and_bind_every_location() {
    let scratch = Scratch::new();
    let preview =
        Workspace::preview_project_creation(&scratch.0.join("linked"), "Linked", "standard-ecu")
            .unwrap();
    let mut workspace = Workspace::create_project_previewed(&preview).unwrap();
    let original = workspace.preview_save().unwrap();
    for (definition, lexeme) in [
        ("CanIfRxPduCanId", "0x450"),
        ("CanIfTxPduCanId", "1105"),
        ("TIMING-EVENT#PERIOD", "2e-2"),
    ] {
        let view = projection(&workspace);
        let field = view
            .fields
            .iter()
            .find(|field| {
                field.linked_edit.is_some()
                    && (field
                        .definition
                        .definition_id
                        .ends_with(&format!("/{definition}"))
                        || field.definition.definition_id == definition)
            })
            .unwrap();
        let linked = field.linked_edit.as_ref().unwrap();
        assert_eq!(linked.operation, "set-value");
        assert!(!linked.locations.is_empty());
        let changes = change(&view, field, lexeme);
        let preview = workspace.prepare_change(&changes).unwrap_or_else(|error| {
            panic!("{} {lexeme}: {error:?}", field.definition.definition_id)
        });
        assert_eq!(preview.impacts.len(), linked.locations.len() + 1);
        assert!(
            preview
                .impacts
                .iter()
                .all(|impact| impact.change_id == field.field_id)
        );
        workspace
            .apply_change(&changes, &preview.change_revision)
            .unwrap();
        assert!(
            workspace
                .apply_change(&changes, &preview.change_revision)
                .is_err()
        );
        assert!(workspace.inspect_integration().diagnostics.is_empty());
    }
    let plan = workspace.integration_plan().unwrap();
    let description = plan.description();
    assert_eq!(
        description
            .signals
            .iter()
            .map(|signal| signal.can_id)
            .collect::<Vec<_>>(),
        [1104, 1105]
    );
    assert!(
        description
            .schedule
            .entities
            .iter()
            .filter(|entity| entity.application)
            .all(|entity| entity.period_ms == 20)
    );
    assert_eq!(
        description
            .com_runtime
            .as_ref()
            .unwrap()
            .transmit_main
            .period_ms,
        20
    );
    assert!(
        description
            .signals
            .iter()
            .filter(|signal| !signal.receive)
            .all(|signal| signal.transmit_period_ms == Some(20))
    );
    let view = projection(&workspace);
    for field in view.fields.iter().filter(|field| {
        field.linked_edit.is_some()
            && field
                .definition
                .definition_id
                .ends_with("/OsAlarmCycleTime")
    }) {
        assert!(matches!(&field.current, ValueState::Explicit { value } if value.lexeme == "20"));
    }
    assert_ne!(
        workspace.preview_save().unwrap().revision,
        original.revision
    );
}

#[test]
fn linked_grid_rejections_preserve_all_bytes_and_preview_identity() {
    let scratch = Scratch::new();
    let preview =
        Workspace::preview_project_creation(&scratch.0.join("refused"), "Refused", "standard-ecu")
            .unwrap();
    let mut workspace = Workspace::create_project_previewed(&preview).unwrap();
    let before = workspace.preview_save().unwrap().revision;
    let view = projection(&workspace);
    for (suffix, invalid) in [
        ("/CanIfRxPduCanId", "2048"),
        ("TIMING-EVENT#PERIOD", "0.0001"),
    ] {
        let field = view
            .fields
            .iter()
            .find(|field| {
                field.linked_edit.is_some() && field.definition.definition_id.ends_with(suffix)
            })
            .unwrap();
        let changes = change(&view, field, invalid);
        assert!(workspace.prepare_change(&changes).is_err());
        assert!(workspace.apply_change(&changes, "unprepared").is_err());
        assert_eq!(workspace.preview_save().unwrap().revision, before);
    }
    let period = view
        .fields
        .iter()
        .find(|field| {
            field.linked_edit.is_some() && field.definition.definition_id == "TIMING-EVENT#PERIOD"
        })
        .unwrap();
    let mut changes = change(&view, period, "0.020");
    let other = view
        .fields
        .iter()
        .find(|field| {
            field.field_id != period.field_id
                && field.definition.kind.is_some()
                && field.linked_edit.as_ref().is_some_and(|linked| {
                    linked.group_id == period.linked_edit.as_ref().unwrap().group_id
                })
        })
        .unwrap();
    changes.changes.extend(change(&view, other, "30").changes);
    assert!(
        serde_json::to_string(&workspace.prepare_change(&changes).unwrap_err())
            .unwrap()
            .contains("conflicting_target_operations")
    );
    assert_eq!(workspace.preview_save().unwrap().revision, before);
    let changes = change(&view, period, "0.020");
    let preview = workspace.prepare_change(&changes).unwrap();
    let altered = change(&view, period, "0.030");
    assert!(
        workspace
            .apply_change(&altered, &preview.change_revision)
            .is_err()
    );
    assert_eq!(workspace.preview_save().unwrap().revision, before);
}

#[test]
fn linked_period_accepts_alarm_side_edits_and_noop_keeps_revision() {
    let scratch = Scratch::new();
    let preview =
        Workspace::preview_project_creation(&scratch.0.join("alarm"), "Alarm", "standard-ecu")
            .unwrap();
    let mut workspace = Workspace::create_project_previewed(&preview).unwrap();
    let view = projection(&workspace);
    let alarm = view
        .fields
        .iter()
        .find(|field| {
            field.linked_edit.is_some()
                && field
                    .definition
                    .definition_id
                    .ends_with("/OsAlarmCycleTime")
        })
        .unwrap();
    let original = workspace.input_fingerprint().unwrap();
    let ValueState::Explicit { value } = &alarm.current else {
        panic!("explicit alarm period")
    };
    let noop = change(&view, alarm, &value.lexeme);
    let preview = workspace.prepare_change(&noop).unwrap();
    assert!(preview.impacts.is_empty());
    workspace
        .apply_change(&noop, &preview.change_revision)
        .unwrap();
    assert_eq!(workspace.input_fingerprint().unwrap(), original);
    let changes = change(&view, alarm, "0x1e");
    let preview = workspace.prepare_change(&changes).unwrap();
    workspace
        .apply_change(&changes, &preview.change_revision)
        .unwrap();
    let plan = workspace.integration_plan().unwrap();
    assert!(
        plan.description()
            .schedule
            .entities
            .iter()
            .filter(|entity| entity.application)
            .all(|entity| entity.period_ms == 30)
    );
}

#[test]
fn linked_can_groups_swap_identifiers_in_one_atomic_batch() {
    let scratch = Scratch::new();
    let preview =
        Workspace::preview_project_creation(&scratch.0.join("swap"), "Swap", "standard-ecu")
            .unwrap();
    let mut workspace = Workspace::create_project_previewed(&preview).unwrap();
    let view = projection(&workspace);
    let rx = view
        .fields
        .iter()
        .find(|field| field.definition.definition_id.ends_with("/CanIfRxPduCanId"))
        .unwrap();
    let tx = view
        .fields
        .iter()
        .find(|field| field.definition.definition_id.ends_with("/CanIfTxPduCanId"))
        .unwrap();
    let ValueState::Explicit { value: rx_value } = &rx.current else {
        panic!("rx")
    };
    let ValueState::Explicit { value: tx_value } = &tx.current else {
        panic!("tx")
    };
    let mut changes = change(&view, rx, &tx_value.lexeme);
    changes
        .changes
        .extend(change(&view, tx, &rx_value.lexeme).changes);
    let preview = workspace.prepare_change(&changes).unwrap();
    assert_eq!(preview.impacts.len(), 4);
    workspace
        .apply_change(&changes, &preview.change_revision)
        .unwrap();
    let updated = projection(&workspace);
    assert_eq!(
        updated
            .fields
            .iter()
            .find(|field| field.field_id == rx.field_id)
            .unwrap()
            .current,
        tx.current
    );
    assert_eq!(
        updated
            .fields
            .iter()
            .find(|field| field.field_id == tx.field_id)
            .unwrap()
            .current,
        rx.current
    );
    assert!(workspace.inspect_integration().diagnostics.is_empty());
}

#[test]
fn linked_grid_batch_rebinds_locations_after_instance_rename() {
    let scratch = Scratch::new();
    let preview =
        Workspace::preview_project_creation(&scratch.0.join("rename"), "Rename", "standard-ecu")
            .unwrap();
    let mut workspace = Workspace::create_project_previewed(&preview).unwrap();
    let view = projection(&workspace);
    let field = view
        .fields
        .iter()
        .find(|field| field.definition.definition_id.ends_with("/CanIfTxPduCanId"))
        .unwrap();
    let owner = view
        .objects
        .iter()
        .find(|owner| owner.object_id == field.object_id)
        .unwrap();
    let mut changes = change(&view, field, "1105");
    changes.changes.push(ConfigurationChange::RenameInstance {
        change_id: "rename-pdu".into(),
        object: ObjectRef::Existing {
            object_id: owner.object_id.clone(),
        },
        expected_short_name: owner.short_name.clone(),
        short_name: "RenamedTransmitPdu".into(),
    });
    let preview = workspace.prepare_change(&changes).unwrap();
    let outcome = workspace
        .apply_change(&changes, &preview.change_revision)
        .unwrap();
    assert_eq!(
        outcome
            .projection
            .objects
            .iter()
            .find(|object| object.object_id == owner.object_id)
            .unwrap()
            .short_name,
        "RenamedTransmitPdu"
    );
    let plan = workspace.integration_plan().unwrap();
    let signal = plan
        .description()
        .signals
        .iter()
        .find(|signal| !signal.receive)
        .unwrap();
    assert_eq!(signal.can_id, 1105);
    assert!(signal.can_if_pdu.ends_with("/RenamedTransmitPdu"));
}

#[test]
fn linked_grid_can_id_removal_preserves_incomplete_configuration_editing() {
    for definition in ["CanIfRxPduCanId", "CanIfTxPduCanId"] {
        let scratch = Scratch::new();
        let project = scratch.0.join("remove");
        let preview =
            Workspace::preview_project_creation(&project, "Remove", "standard-ecu").unwrap();
        let mut workspace = Workspace::create_project_previewed(&preview).unwrap();
        let view = projection(&workspace);
        let field = view
            .fields
            .iter()
            .find(|field| {
                field
                    .definition
                    .definition_id
                    .ends_with(&format!("/{definition}"))
            })
            .unwrap();
        assert!(field.linked_edit.is_some());
        let mut changes = change(&view, field, "1105");
        let ConfigurationChange::SetValue { value, .. } = &mut changes.changes[0] else {
            unreachable!()
        };
        *value = ValueState::Absent;
        let preview = workspace.prepare_change(&changes).unwrap();
        assert_eq!(preview.impacts.len(), 1);
        let outcome = workspace
            .apply_change(&changes, &preview.change_revision)
            .unwrap();
        assert!(!outcome.projection.fields.iter().any(|updated| {
            updated.object_id == field.object_id
                && updated.definition.definition_id == field.definition.definition_id
                && matches!(updated.current, ValueState::Explicit { .. })
        }));
        assert!(workspace.integration_plan().is_err());
        let save = workspace.preview_save().unwrap();
        workspace.save_previewed(&save.revision).unwrap();
        let reopened = Workspace::open_project_manifest(
            &project.join("workbench-project.json"),
            &scratch.0.join("cache"),
        )
        .unwrap();
        assert!(reopened.integration_plan().is_err());
        let explicit_count = |view: &ProjectProjection| {
            view.fields
                .iter()
                .filter(|updated| {
                    updated.definition.definition_id == field.definition.definition_id
                        && matches!(updated.current, ValueState::Explicit { .. })
                })
                .count()
        };
        assert_eq!(
            explicit_count(&projection(&reopened)),
            explicit_count(&view) - 1
        );
    }
}
