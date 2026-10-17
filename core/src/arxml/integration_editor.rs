use super::persistence::Patch;
use super::{Workspace, apply_patches, child_text, patch_child, patch_param, path_of};
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
        crate::integration::build_plan_native(&self.integration_source_snapshot()?, &self.catalog)
    }

    pub fn inspect_integration(&self) -> IntegrationInspection {
        inspection(self.integration_plan())
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
                    found += 1;
                    let result = if field.parameter {
                        patch_param(node, &field.field, field.value.clone(), &mut patches[index])
                    } else {
                        patch_child(node, &field.field, field.value.clone(), &mut patches[index])
                    };
                    result.map_err(|error| {
                        editor::issue(&plan, "EDIT_UNSAFE", &field.object, error)
                    })?;
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
        let checked = candidate.integration_plan()?;
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
            profile: plan.description().profile.clone(),
            description: Some(plan.description().clone()),
            diagnostics: Vec::new(),
        },
        Err(diagnostics) => IntegrationInspection {
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
                    },
                )
            })
            .collect();
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
                },
            ));
        }
        edits
            .into_iter()
            .filter_map(|(id, edit)| {
                let edits = editor::fields_including_unchanged(&plan, &edit, true).ok()?;
                let fields: Option<Vec<_>> = edits
                    .iter()
                    .map(|edit| {
                        let mut found = self.snapshot.fields.iter().filter(|field| {
                            let owner = &self.snapshot.objects
                                [self.snapshot.object_by_id[&field.view.object_id]];
                            owner.view.path == edit.object
                                && if edit.parameter {
                                    field.view.definition.definition_id.rsplit('/').next()
                                        == Some(edit.field.as_str())
                                } else {
                                    field.element == edit.field
                                }
                        });
                        let field = found.next()?;
                        if found.next().is_some() {
                            return None;
                        }
                        Some((field.view.field_id.clone(), edit.value.clone()))
                    })
                    .collect();
                let kind = if edit.application_period_ms.is_none() {
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
        } else {
            let id = crate::definitions::unsigned_integer(raw).ok_or_else(invalid)?;
            for value in edit.can_ids.values_mut() {
                *value = id;
            }
        }
        let plan = self.integration_plan().map_err(|issues| {
            crate::message::LocalizedText::messages(issues.into_iter().map(|issue| issue.message))
        })?;
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
            edits[index].value = value.lexeme.clone();
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
