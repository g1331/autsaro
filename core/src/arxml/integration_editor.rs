use super::persistence::Patch;
use super::{Workspace, apply_patches, child_text, patch_child, patch_param, path_of};
use crate::integration::{
    DiagnosticCategory, IntegrationEdit, IntegrationInspection, PlanDiagnostic, RuntimeCatalog,
    ValidatedIntegrationPlan, editor,
};
use crate::model::SavePreview;
use roxmltree::Document;
use std::path::PathBuf;

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
        _runtime: &RuntimeCatalog,
        revision: &str,
    ) -> Result<super::PreparedSave, Vec<PlanDiagnostic>> {
        self.prepare_save_previewed(revision)
            .map_err(|error| failure("SAVE_PREVIEW_STALE", error))
    }

    pub fn prepare_integration_save_previewed_legacy(
        self,
        runtime: &RuntimeCatalog,
        mod_archive: PathBuf,
        revision: &str,
    ) -> Result<super::PreparedSave, Vec<PlanDiagnostic>> {
        self.integration_plan_legacy(runtime, mod_archive)?;
        super::PreparedSave::validated(self, revision)
            .map_err(|error| failure("SAVE_PREVIEW_STALE", error))
    }

    pub fn saved_integration_plan(
        &self,
        runtime: &RuntimeCatalog,
    ) -> Result<ValidatedIntegrationPlan, Vec<PlanDiagnostic>> {
        self.verify_saved_sources().map_err(|error| {
            let code = if self.is_dirty() {
                "SOURCE_DIRTY"
            } else {
                "SOURCE_CHANGED"
            };
            failure(code, error)
        })?;
        if self.uses_legacy_validation() {
            return Err(failure(
                "VALIDATION_MODE",
                crate::product_message!("backend.arxml.legacy_resources_required"),
            ));
        }
        // The saved-source check above seals this same in-memory snapshot.
        // integration_plan would immediately read every source a second time.
        crate::integration::build_plan_native(
            &self.integration_source_snapshot()?,
            &self.catalog,
            runtime,
        )
    }

    pub fn saved_integration_plan_legacy(
        &self,
        runtime: &RuntimeCatalog,
        mod_archive: PathBuf,
    ) -> Result<ValidatedIntegrationPlan, Vec<PlanDiagnostic>> {
        self.verify_saved_sources().map_err(|error| {
            let code = if self.is_dirty() {
                "SOURCE_DIRTY"
            } else {
                "SOURCE_CHANGED"
            };
            failure(code, error)
        })?;
        self.integration_plan_legacy(runtime, mod_archive)
    }

    pub fn open_ecu_handoff(
        output: &std::path::Path,
        dependencies: &crate::integration::PlanDependencies,
        runtime: &RuntimeCatalog,
    ) -> Result<Self, Vec<PlanDiagnostic>> {
        let package = crate::integration::open_ecu_handoff(output, dependencies, runtime)?;
        let mut workspace =
            Self::open_legacy(package.input_paths(), dependencies.xsd_archive.clone())
                .map_err(|error| failure("ECU_HANDOFF", error))?;
        workspace.integration_input_root = Some(package.input_root());
        let actual =
            workspace.saved_integration_plan_legacy(runtime, dependencies.mod_archive.clone())?;
        if serde_json::to_value(actual.description()).unwrap()
            != serde_json::to_value(package.plan().description()).unwrap()
        {
            return Err(failure(
                "SOURCE_CHANGED",
                crate::product_message!(
                    "backend.arxml.integration_editor.delivered_inputs_changed"
                ),
            ));
        }
        Ok(workspace)
    }

    pub fn inspect_integration(&self, runtime: &RuntimeCatalog) -> IntegrationInspection {
        inspection(self.integration_plan(runtime))
    }

    pub fn inspect_integration_legacy(
        &self,
        runtime: &RuntimeCatalog,
        mod_archive: PathBuf,
    ) -> IntegrationInspection {
        inspection(self.integration_plan_legacy(runtime, mod_archive))
    }

    pub fn edit_integration(
        &mut self,
        runtime: &RuntimeCatalog,
        changes: IntegrationEdit,
    ) -> Result<IntegrationInspection, Vec<PlanDiagnostic>> {
        self.edit_integration_checked(runtime, None, changes)
    }

    pub fn edit_integration_legacy(
        &mut self,
        runtime: &RuntimeCatalog,
        mod_archive: PathBuf,
        changes: IntegrationEdit,
    ) -> Result<IntegrationInspection, Vec<PlanDiagnostic>> {
        self.edit_integration_checked(runtime, Some(mod_archive), changes)
    }

    fn edit_integration_checked(
        &mut self,
        runtime: &RuntimeCatalog,
        legacy_mod: Option<PathBuf>,
        changes: IntegrationEdit,
    ) -> Result<IntegrationInspection, Vec<PlanDiagnostic>> {
        let plan = match &legacy_mod {
            Some(archive) => self.integration_plan_legacy(runtime, archive.clone())?,
            None => self.integration_plan(runtime)?,
        };
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
        let checked = match legacy_mod {
            Some(archive) => candidate.integration_plan_legacy(runtime, archive)?,
            None => {
                candidate
                    .check_configuration_transition(self)
                    .map_err(|error| failure("EDIT_UNSAFE", error))?;
                candidate.integration_plan(runtime)?
            }
        };
        candidate.revision += 1;
        let result = inspection(Ok(checked));
        *self = candidate;
        Ok(result)
    }

    pub fn preview_integration_save(
        &self,
        _runtime: &RuntimeCatalog,
    ) -> Result<SavePreview, Vec<PlanDiagnostic>> {
        let mut candidate = self.clone();
        candidate
            .preview_save()
            .map_err(|error| failure("SAVE_UNSAFE", error))
    }

    pub fn preview_integration_save_legacy(
        &self,
        runtime: &RuntimeCatalog,
        mod_archive: PathBuf,
    ) -> Result<SavePreview, Vec<PlanDiagnostic>> {
        self.integration_plan_legacy(runtime, mod_archive)?;
        Ok(SavePreview {
            revision: self.save_revision(),
            files: self.save_preview_files(),
        })
    }

    pub fn save_integration_previewed(
        &mut self,
        runtime: &RuntimeCatalog,
        revision: &str,
    ) -> Result<IntegrationInspection, Vec<PlanDiagnostic>> {
        self.save_previewed(revision)
            .map_err(|error| failure("SAVE_FAILED", error))?;
        Ok(self.inspect_integration(runtime))
    }

    pub fn save_integration_previewed_legacy(
        &mut self,
        runtime: &RuntimeCatalog,
        mod_archive: PathBuf,
        revision: &str,
    ) -> Result<IntegrationInspection, Vec<PlanDiagnostic>> {
        if self.save_revision() != revision {
            return Err(failure(
                "SAVE_PREVIEW_STALE",
                crate::product_message!(
                    "backend.arxml.integration_editor.inputs_changed_after_preview"
                ),
            ));
        }
        self.integration_plan_legacy(runtime, mod_archive.clone())?;
        self.save_sources()
            .map_err(|error| failure("SAVE_FAILED", error))?;
        Ok(self.inspect_integration_legacy(runtime, mod_archive))
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
    fn saved_native_plan_preserves_source_rejections_before_legacy_mode() {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi-component");
        let files = std::fs::read_dir(root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        let mut workspace = Workspace::open(files).unwrap();
        let runtime = RuntimeCatalog::embedded().unwrap();
        workspace.saved_integration_plan(&runtime).unwrap();
        // Exercise the legacy route without requiring an official archive:
        // these saved-native gates must run before schema/legacy planning.
        workspace.schema_zip = Some(PathBuf::from("unused-legacy-schema.zip"));
        assert_eq!(
            workspace.saved_integration_plan(&runtime).err().unwrap()[0].code,
            "VALIDATION_MODE"
        );
        workspace.files[0].text.push_str("\n<!-- unsaved -->\n");
        assert_eq!(
            workspace.saved_integration_plan(&runtime).err().unwrap()[0].code,
            "SOURCE_DIRTY"
        );
        workspace.files[0].text = workspace.files[0].saved.clone();
        // Change only the expected baseline; real fixture bytes stay untouched.
        workspace.files[0]
            .saved
            .push_str("\n<!-- stale baseline -->\n");
        workspace.files[0].text = workspace.files[0].saved.clone();
        assert_eq!(
            workspace.saved_integration_plan(&runtime).err().unwrap()[0].code,
            "SOURCE_CHANGED"
        );
    }
}
