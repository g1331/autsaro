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
        let mut patches: Vec<Vec<Patch>> = self.files.iter().map(|_| Vec::new()).collect();
        for field in &fields {
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
