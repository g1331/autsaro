use super::persistence::Patch;
use super::{Workspace, apply_patches, child_text, patch_text, path_of};
use crate::integration::{
    DiagnosticCategory, IntegrationEdit, IntegrationInspection, PlanDiagnostic,
    ValidatedIntegrationPlan, editor,
};
use crate::model::SavePreview;
use roxmltree::Document;

pub(super) fn failure(
    code: &str,
    message: impl Into<crate::message::LocalizedText>,
) -> Vec<PlanDiagnostic> {
    vec![PlanDiagnostic {
        category: DiagnosticCategory::Input,
        code: code.into(),
        file: None,
        object: None,
        message: message.into(),
        remedy: crate::product_message!(
            "backend.arxml.integration_editor.source_preview_conflict_remedy"
        ),
    }]
}

impl Workspace {
    pub fn prepare_integration_save_previewed(
        self,
        revision: &str,
    ) -> Result<super::PreparedSave, Vec<PlanDiagnostic>> {
        self.prepare_save_previewed(revision)
            .map_err(|error| failure("SAVE_PREVIEW_STALE", error))
    }

    pub fn saved_integration_plan(&self) -> Result<ValidatedIntegrationPlan, Vec<PlanDiagnostic>> {
        self.verify_saved_sources().map_err(|error| {
            let code = if self.is_dirty() {
                "SOURCE_DIRTY"
            } else {
                "SOURCE_CHANGED"
            };
            failure(code, error)
        })?;
        // The saved-source check above seals this same in-memory snapshot.
        // integration_plan would immediately read every source a second time.
        self.build_workspace_plan(&self.integration_source_snapshot()?, &self.catalog)
    }

    pub fn inspect_integration(&self) -> IntegrationInspection {
        let mut report = inspection(self.integration_plan());
        report.layers = self
            .project_manifest()
            .and_then(|manifest| manifest.layers.as_ref())
            .map(|layers| editor::VariantEditor {
                base: layers.base_reference.base_id.clone(),
                revision: layers.base_reference.revision.clone(),
                project_members: layers.project.members.clone(),
                project_fields: layers
                    .project
                    .operations
                    .iter()
                    .map(|op| {
                        if let Some(field) = &op.target.field {
                            format!("{} · {}", op.target.object.path, field)
                        } else {
                            op.target.object.path.clone()
                        }
                    })
                    .collect(),
                base_digest: layers.base_reference.source_sha256.clone(),
                groups: layers
                    .groups
                    .iter()
                    .map(|(name, group)| {
                        (
                            name.clone(),
                            editor::VariantChoices {
                                exactly_one: group.exactly_one,
                                options: group.options.keys().cloned().collect(),
                                selected: layers
                                    .selection
                                    .get(name)
                                    .into_iter()
                                    .flatten()
                                    .cloned()
                                    .collect(),
                            },
                        )
                    })
                    .collect(),
                differences: Vec::new(),
            });
        report
    }

    fn variant_differences(
        &self,
        candidate: &Self,
    ) -> Result<Vec<editor::VariantDifference>, Vec<PlanDiagnostic>> {
        Ok(self
            .compare_project(candidate)?
            .semantics
            .into_iter()
            .map(|item| editor::VariantDifference {
                object: item.target.object.path,
                field: item.target.field,
                changes: item
                    .changes
                    .iter()
                    .map(|change| format!("{change:?}"))
                    .collect(),
                before: item.before.map(|entry| entry.values).unwrap_or_default(),
                after: item.after.map(|entry| entry.values).unwrap_or_default(),
            })
            .collect())
    }

    pub fn preview_variant_selection(
        &self,
        selection: std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    ) -> Result<Vec<editor::VariantDifference>, Vec<PlanDiagnostic>> {
        let mut candidate = self.clone();
        candidate
            .select_project_variants(selection)
            .map_err(|error| failure("VARIANT_SELECTION", error))?;
        self.variant_differences(&candidate)
    }

    pub fn edit_integration(
        &mut self,
        changes: IntegrationEdit,
    ) -> Result<IntegrationInspection, Vec<PlanDiagnostic>> {
        self.edit_integration_checked(changes)
    }

    fn edit_integration_checked(
        &mut self,
        changes: IntegrationEdit,
    ) -> Result<IntegrationInspection, Vec<PlanDiagnostic>> {
        if let Some(selection) = changes.variant_selection {
            if !changes.connections.is_empty()
                || !changes.signal_settings.is_empty()
                || !changes.can_ids.is_empty()
                || changes.application_period_ms.is_some()
            {
                return Err(failure(
                    "LAYER_EDIT_REQUIRED",
                    crate::product_message!(
                        "backend.integration.editor.separate_variant_selection"
                    ),
                ));
            }
            let mut candidate = self.clone();
            candidate
                .select_project_variants(selection)
                .map_err(|error| failure("VARIANT_SELECTION", error))?;
            let mut report = candidate.inspect_integration();
            if report.description.is_none() {
                return Err(report.diagnostics);
            }
            if self.project_semantics().is_ok() {
                if let Some(layers) = report.layers.as_mut() {
                    layers.differences = self.variant_differences(&candidate)?;
                }
            }
            *self = candidate;
            return Ok(report);
        }
        if self
            .project_manifest()
            .is_some_and(|manifest| manifest.layers.is_some())
        {
            return Err(failure(
                "LAYER_EDIT_REQUIRED",
                "Use layer operations to edit a layered project.",
            ));
        }
        let plan = self.integration_plan()?;
        let fields = editor::fields(&plan, &changes)?;
        self.edit_integration_fields(&fields)
    }

    pub(super) fn edit_integration_fields(
        &mut self,
        fields: &[editor::FieldEdit],
    ) -> Result<IntegrationInspection, Vec<PlanDiagnostic>> {
        let plan = self.integration_plan()?;
        let mut patches: Vec<Vec<Patch>> = self.files.iter().map(|_| Vec::new()).collect();
        for field in fields {
            let mut found = 0;
            for (index, file) in self.files.iter().enumerate() {
                let document = Document::parse(&file.text).map_err(|error| {
                    failure("EDIT_XML", crate::message::LocalizedText::messages([
                        crate::product_message!("backend.arxml.integration_editor.xml_source_context", "path" => file.path.display()),
                        error.to_string().into(),
                    ]))
                })?;
                for node in document.descendants().filter(|node| {
                    node.is_element()
                        && child_text(*node, "SHORT-NAME").is_some()
                        && path_of(*node) == field.object
                }) {
                    // The validated plan already requires each edited scalar. Count
                    // actual leaves here so source selection has one owner.
                    let leaves: Vec<_> = if !field.parameter
                        && node.has_tag_name("DATA-TYPE-MAPPING-SET")
                        && field.field.starts_with('/')
                    {
                        node.descendants()
                            .filter(|item| {
                                item.has_tag_name("DATA-TYPE-MAP")
                                    && child_text(*item, "APPLICATION-DATA-TYPE-REF").as_deref()
                                        == Some(field.field.as_str())
                            })
                            .flat_map(|item| {
                                item.children().filter(|child| {
                                    child.has_tag_name("IMPLEMENTATION-DATA-TYPE-REF")
                                })
                            })
                            .collect()
                    } else if !field.parameter
                        && node.has_tag_name("I-SIGNAL")
                        && field.field == "BASE-TYPE-REF"
                    {
                        node.descendants()
                            .filter(|item| item.has_tag_name("BASE-TYPE-REF"))
                            .collect()
                    } else if field.parameter && field.field.starts_with('/') {
                        node.children()
                            .filter(|item| item.has_tag_name("REFERENCE-VALUES"))
                            .flat_map(|group| group.children())
                            .filter(|entry| {
                                super::definition(*entry).as_deref() == Some(field.field.as_str())
                            })
                            .flat_map(|entry| {
                                entry
                                    .children()
                                    .filter(|child| child.has_tag_name("VALUE-REF"))
                            })
                            .collect()
                    } else if field.parameter {
                        node.descendants()
                            .filter(|parameter| {
                                parameter.is_element()
                                    && matches!(
                                        parameter.tag_name().name(),
                                        "ECUC-NUMERICAL-PARAM-VALUE" | "ECUC-TEXTUAL-PARAM-VALUE"
                                    )
                                    && super::definition(*parameter).is_some_and(|definition| {
                                        definition.ends_with(&format!("/{}", field.field))
                                    })
                            })
                            .flat_map(|parameter| {
                                parameter.children().filter(|child| {
                                    child.is_element() && child.tag_name().name() == "VALUE"
                                })
                            })
                            .collect()
                    } else {
                        node.children()
                            .filter(|child| {
                                child.is_element() && child.tag_name().name() == field.field
                            })
                            .collect()
                    };
                    if leaves.is_empty()
                        && field.field == "CAN-FD-FRAME-SUPPORT"
                        && node.has_tag_name("CAN-FRAME-TRIGGERING")
                    {
                        found += 1;
                        if field.value == "true" {
                            super::changes::standard_insert(
                                &file.text,
                                node.range(),
                                &field.field,
                                "<CAN-FD-FRAME-SUPPORT>true</CAN-FD-FRAME-SUPPORT>",
                                &mut patches[index],
                            )
                            .map_err(|error| {
                                editor::issue(&plan, "EDIT_UNSAFE", &field.object, error)
                            })?;
                        }
                    }
                    if leaves.is_empty()
                        && field.field == "CanIfRxPduDataLength"
                        && field.value == "8"
                        && let Some(definition) = super::definition(node)
                        && definition.ends_with("/CanIfRxPduCfg")
                    {
                        let entry = format!(
                            r#"<ECUC-NUMERICAL-PARAM-VALUE><DEFINITION-REF DEST="ECUC-INTEGER-PARAM-DEF">{definition}/CanIfRxPduDataLength</DEFINITION-REF><VALUE>8</VALUE></ECUC-NUMERICAL-PARAM-VALUE>"#
                        );
                        super::changes::group_insert(
                            &file.text,
                            node.range(),
                            "PARAMETER-VALUES",
                            &entry,
                            &mut patches[index],
                        )
                        .map_err(|error| {
                            editor::issue(&plan, "EDIT_UNSAFE", &field.object, error)
                        })?;
                        found += 1;
                    }
                    for leaf in leaves {
                        found += 1;
                        patch_text(leaf, field.value.clone(), &mut patches[index]).map_err(
                            |error| editor::issue(&plan, "EDIT_UNSAFE", &field.object, error),
                        )?;
                    }
                }
            }
            if found != 1 {
                return Err(editor::issue(
                    &plan,
                    "EDIT_UNSAFE",
                    &field.object,
                    crate::product_message!(
                        "backend.arxml.integration_editor.edit_source_not_unique"
                    ),
                ));
            }
        }
        let mut candidate = self.clone();
        for (file, edits) in candidate.files.iter_mut().zip(&mut patches) {
            apply_patches(&mut file.text, edits).map_err(|error| failure("EDIT_UNSAFE", error))?;
        }
        candidate
            .refresh()
            .map_err(|error| failure("EDIT_UNSAFE", error))?;
        candidate
            .check_configuration_transition(self)
            .map_err(|error| failure("EDIT_UNSAFE", error))?;
        let checked = candidate.integration_plan().map_err(|mut issues| {
            for diagnostic in &mut issues {
                let mut affected: Vec<_> = fields.iter()
                    .filter(|field| diagnostic.object.as_ref() == Some(&field.object))
                    .collect();
                if affected.is_empty() {
                    affected.extend(fields);
                }
                let locations = affected.iter()
                    .map(|field| format!("{} · {}", field.object, field.field))
                    .collect::<Vec<_>>()
                    .join(", ");
                diagnostic.message = crate::message::LocalizedText::messages([
                    diagnostic.message.clone(),
                    crate::product_message!("backend.integration.editor.rejected_fields", "fields" => locations),
                ]);
            }
            issues
        })?;
        candidate.revision += 1;
        let result = inspection(Ok(checked));
        *self = candidate;
        Ok(result)
    }

    pub fn preview_integration_save(&self) -> Result<SavePreview, Vec<PlanDiagnostic>> {
        let mut candidate = self.clone();
        candidate
            .preview_save()
            .map_err(|error| failure("SAVE_UNSAFE", error))
    }

    pub fn save_integration_previewed(
        &mut self,
        revision: &str,
    ) -> Result<IntegrationInspection, Vec<PlanDiagnostic>> {
        self.save_previewed(revision)
            .map_err(|error| failure("SAVE_FAILED", error))?;
        Ok(self.inspect_integration())
    }
}

fn inspection(
    result: Result<ValidatedIntegrationPlan, Vec<PlanDiagnostic>>,
) -> IntegrationInspection {
    match result {
        Ok(plan) => IntegrationInspection {
            connections: crate::integration::editor_connections::choices(plan.description()),
            layers: None,
            profile: plan.description().profile.clone(),
            description: Some(plan.description().clone()),
            diagnostics: Vec::new(),
        },
        Err(diagnostics) => IntegrationInspection {
            connections: Vec::new(),
            layers: None,
            profile: crate::integration::PROFILE.into(),
            description: None,
            diagnostics,
        },
    }
}

pub(super) struct LinkedGroup {
    pub kind: String,
    pub id: String,
    edit: IntegrationEdit,
    pub fields: Vec<(String, String)>,
}

impl Workspace {
    /// Resolve identities from the same checked plan and edit closure as the standard editor.
    pub(super) fn linked_groups(&self) -> Vec<LinkedGroup> {
        let Ok(plan) = self.integration_plan() else {
            return Vec::new();
        };
        let description = plan.description();
        let mut edits: Vec<_> = description
            .signals
            .iter()
            .map(|signal| {
                (
                    signal.port.clone(),
                    IntegrationEdit {
                        can_ids: [(signal.port.clone(), signal.can_id)].into(),
                        application_period_ms: None,
                        ..IntegrationEdit::default()
                    },
                )
            })
            .collect();
        for signal in &description.signals {
            for setting in crate::integration::SignalSetting::current(signal) {
                edits.push((
                    format!("{}#{}", signal.port, setting.name()),
                    IntegrationEdit {
                        signal_settings: [(signal.port.clone(), vec![setting])].into(),
                        ..IntegrationEdit::default()
                    },
                ));
            }
        }
        let connections = crate::integration::editor_connections::choices(description);
        for choice in connections
            .iter()
            .filter(|choice| choice.definition == "wireLength")
        {
            edits.push((
                format!("{}#wire-length", choice.object),
                IntegrationEdit {
                    connections: vec![crate::integration::ConnectionEdit {
                        object: choice.object.clone(),
                        definition: choice.definition.clone(),
                        target: choice.current.clone(),
                    }],
                    ..Default::default()
                },
            ));
        }
        if let Some(application) = description
            .schedule
            .entities
            .iter()
            .find(|entity| entity.application)
        {
            edits.push((
                application.trigger().into(),
                IntegrationEdit {
                    can_ids: Default::default(),
                    application_period_ms: Some(application.period_ms),
                    ..IntegrationEdit::default()
                },
            ));
        }
        let mut source_fields = std::collections::BTreeMap::new();
        for field in &self.snapshot.fields {
            let owner = &self.snapshot.objects[self.snapshot.object_by_id[&field.view.object_id]];
            let is_parameter = matches!(
                field.element.as_str(),
                "ECUC-TEXTUAL-PARAM-VALUE" | "ECUC-NUMERICAL-PARAM-VALUE"
            );
            let name = if is_parameter {
                field
                    .view
                    .definition
                    .definition_id
                    .rsplit('/')
                    .next()
                    .unwrap_or("")
            } else {
                field.element.as_str()
            };
            source_fields
                .entry((owner.view.path.as_str(), name, is_parameter))
                .and_modify(|id| *id = None)
                .or_insert(Some(&field.view.field_id));
        }
        edits
            .into_iter()
            .filter_map(|(id, edit)| {
                let edits = if let Some(connection) = edit.connections.first() {
                    connection.resolve(description, &connections, &edit.signal_settings)?
                } else {
                    editor::fields_including_unchanged(&plan, &edit, true).ok()?
                };
                let fields: Option<Vec<_>> = edits
                    .iter()
                    .map(|edit| {
                        let field = source_fields
                            .get(&(edit.object.as_str(), edit.field.as_str(), edit.parameter))?
                            .as_ref()?;
                        Some(((*field).clone(), edit.value.clone()))
                    })
                    .collect();
                let kind = if !edit.connections.is_empty() {
                    "transport-length"
                } else if !edit.signal_settings.is_empty() {
                    "signal-setting"
                } else if edit.application_period_ms.is_none() {
                    "can-id"
                } else if edits.iter().any(|field| field.field.starts_with("Com")) {
                    "application-com-period"
                } else {
                    "application-period"
                };
                Some(LinkedGroup {
                    kind: kind.into(),
                    id,
                    edit,
                    fields: fields?,
                })
            })
            .collect()
    }

    pub(super) fn linked_fields(
        &self,
        group: &LinkedGroup,
        field_id: &str,
        value: &crate::project_model::ValueState,
    ) -> Result<Vec<editor::FieldEdit>, crate::message::LocalizedText> {
        use crate::project_model::{ValueKind, ValueState};
        let invalid =
            || crate::product_message!("backend.integration.editor.application_period_invalid");
        let field = &self.snapshot.fields[self.snapshot.field_by_id[field_id]];
        if !field.view.definition.writable || field.view.reference.is_some() {
            return Err(
                crate::product_message!("backend.arxml.changes.field_value_editing_unsupported", "change_id" => field_id),
            );
        }
        if field.view.current == *value {
            return Ok(Vec::new());
        }
        let ValueState::Explicit { value } = value else {
            return Err(
                crate::product_message!("backend.arxml.changes.field_value_editing_unsupported", "change_id" => field_id),
            );
        };
        self.catalog
            .validate_value(&field.view.definition.definition_id, value)?;
        let mut edit = group.edit.clone();
        let raw = value.lexeme.trim();
        let plan = self.integration_plan().map_err(|issues| {
            crate::message::LocalizedText::messages(issues.into_iter().map(|issue| issue.message))
        })?;
        if let Some(period) = edit.application_period_ms {
            let seconds = field.view.definition.kind == Some(ValueKind::Float);
            let numeric = |raw: &str| -> Option<u32> {
                if seconds {
                    crate::integration::milliseconds(raw)
                } else {
                    crate::definitions::unsigned_integer(raw)
                }
            };
            let current = group
                .fields
                .iter()
                .find(|(id, _)| id == field_id)
                .ok_or_else(invalid)?;
            // Autostart start values can differ from the period by an expiry offset.
            let offset = period
                .checked_sub(numeric(&current.1).ok_or_else(invalid)?)
                .ok_or_else(invalid)?;
            edit.application_period_ms = Some(
                numeric(raw)
                    .and_then(|value| value.checked_add(offset))
                    .ok_or_else(invalid)?,
            );
        } else if let Some(connection) = edit.connections.first_mut() {
            connection.target = crate::definitions::unsigned_integer(raw)
                .ok_or_else(|| crate::product_message!("backend.integration.communication.invalid_signal_layout_field", "field" => &connection.definition))?
                .to_string();
        } else if let Some((port, settings)) = edit.signal_settings.iter_mut().next() {
            let signal = plan
                .description()
                .signals
                .iter()
                .find(|signal| &signal.port == port)
                .ok_or_else(invalid)?;
            let name = if field.element.starts_with("ECUC-") {
                field
                    .view
                    .definition
                    .definition_id
                    .rsplit('/')
                    .next()
                    .unwrap_or("")
            } else {
                &field.element
            };
            let invalid_field = || crate::product_message!("backend.integration.communication.invalid_signal_layout_field", "field" => name);
            if matches!(name, "CanIfTxPduCanIdType" | "CanIfRxPduCanIdType") {
                *settings = crate::integration::SignalSetting::can_if_settings(raw, signal)
                    .ok_or_else(invalid_field)?;
            } else {
                settings[0] = settings[0]
                    .parse_cell(signal, name, raw)
                    .ok_or_else(invalid_field)?;
            }
        } else {
            let id = crate::definitions::unsigned_integer(raw).ok_or_else(invalid)?;
            for value in edit.can_ids.values_mut() {
                *value = id;
            }
        }
        let mut edits = editor::fields(&plan, &edit).map_err(|issues| {
            crate::message::LocalizedText::messages(issues.into_iter().map(|issue| issue.message))
        })?;
        let index = group
            .fields
            .iter()
            .position(|(id, _)| id == field_id)
            .ok_or_else(invalid)?;
        if edits.is_empty() {
            // Equivalent numeric input only changes this cell's spelling.
            let all = editor::fields_including_unchanged(&plan, &edit, true).map_err(|issues| {
                crate::message::LocalizedText::messages(
                    issues.into_iter().map(|issue| issue.message),
                )
            })?;
            let mut source = all.into_iter().nth(index).ok_or_else(invalid)?;
            source.value = value.lexeme.clone();
            edits.push(source);
        } else {
            let owner = &self.snapshot.objects[self.snapshot.object_by_id[&field.view.object_id]]
                .view
                .path;
            let edited = edits
                .iter_mut()
                .find(|edit| {
                    edit.object == *owner
                        && if edit.parameter {
                            field.view.definition.definition_id.rsplit('/').next()
                                == Some(edit.field.as_str())
                        } else {
                            edit.field == field.element
                        }
                })
                .ok_or_else(invalid)?;
            edited.value = value.lexeme.clone();
        }
        Ok(edits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_native_plan_refuses_dirty_or_changed_sources() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/component-pipeline/arxml");
        let files = std::fs::read_dir(root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        let mut workspace = Workspace::open(files).unwrap();
        workspace.saved_integration_plan().unwrap();
        workspace.files[0].text.push_str("\n<!-- unsaved -->\n");
        assert_eq!(
            workspace.saved_integration_plan().err().unwrap()[0].code,
            "SOURCE_DIRTY"
        );
        workspace.files[0].text = workspace.files[0].saved.clone();
        // Change only the expected baseline; real fixture bytes stay untouched.
        workspace.files[0]
            .saved
            .push_str("\n<!-- stale baseline -->\n");
        workspace.files[0].text = workspace.files[0].saved.clone();
        assert_eq!(
            workspace.saved_integration_plan().err().unwrap()[0].code,
            "SOURCE_CHANGED"
        );
    }
}
