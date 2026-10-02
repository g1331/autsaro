use super::{
    Document, Patch, SavePreview, SavePreviewFile, Workspace, apply_patches, child_text,
    patch_child, patch_param, path_of,
};
use crate::integration::{
    DiagnosticCategory, IntegrationEdit, IntegrationInspection, PlanDiagnostic, RuntimeCatalog,
    ValidatedIntegrationPlan, editor,
};
use std::path::PathBuf;

fn failure(code: &str, message: impl Into<String>) -> Vec<PlanDiagnostic> {
    vec![PlanDiagnostic {
        category: DiagnosticCategory::Input, code: code.into(), file: None, object: None,
        message: message.into(), remedy: "Resolve the named source/preview conflict and reopen or preview the original inputs again; recovery files must be preserved.".into(),
    }]
}

impl Workspace {
    pub fn prepare_integration_save_previewed(
        self,
        runtime: &RuntimeCatalog,
        mod_archive: PathBuf,
        revision: &str,
    ) -> Result<super::PreparedSave, Vec<PlanDiagnostic>> {
        self.integration_plan(runtime, mod_archive)?;
        super::PreparedSave::validated(self, revision)
            .map_err(|error| failure("SAVE_PREVIEW_STALE", error))
    }

    pub fn saved_integration_plan(
        &self,
        runtime: &RuntimeCatalog,
        mod_archive: PathBuf,
    ) -> Result<ValidatedIntegrationPlan, Vec<PlanDiagnostic>> {
        if self.files.iter().any(|file| file.text != file.saved) {
            return Err(failure(
                "SOURCE_DIRTY",
                "Save all standard inputs before generating or exporting the ECU project.",
            ));
        }
        self.integration_plan(runtime, mod_archive)
    }

    pub fn open_ecu_handoff(
        output: &std::path::Path,
        dependencies: &crate::integration::PlanDependencies,
        runtime: &RuntimeCatalog,
    ) -> Result<Self, Vec<PlanDiagnostic>> {
        let package = crate::integration::open_ecu_handoff(output, dependencies, runtime)?;
        let mut workspace = Self::open(package.input_paths(), dependencies.xsd_archive.clone())
            .map_err(|e| failure("ECU_HANDOFF", e))?;
        workspace.integration_input_root = Some(package.input_root());
        let actual = workspace.saved_integration_plan(runtime, dependencies.mod_archive.clone())?;
        if serde_json::to_value(actual.description()).unwrap()
            != serde_json::to_value(package.plan().description()).unwrap()
        {
            return Err(failure(
                "SOURCE_CHANGED",
                "The delivered inputs changed while opening the ECU package.",
            ));
        }
        Ok(workspace)
    }

    pub fn inspect_integration(
        &self,
        runtime: &RuntimeCatalog,
        mod_archive: PathBuf,
    ) -> IntegrationInspection {
        match self.integration_plan(runtime, mod_archive) {
            Ok(plan) => IntegrationInspection {
                profile: crate::integration::PROFILE.into(),
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

    /// Apply only text-range edits derived from the current validated plan.
    /// A failed final plan check restores all in-memory source bytes.
    pub fn edit_integration(
        &mut self,
        runtime: &RuntimeCatalog,
        mod_archive: PathBuf,
        changes: IntegrationEdit,
    ) -> Result<IntegrationInspection, Vec<PlanDiagnostic>> {
        let plan = self.integration_plan(runtime, mod_archive.clone())?;
        let fields = editor::fields(&plan, &changes)?;
        let mut patches: Vec<Vec<Patch>> = self.files.iter().map(|_| Vec::new()).collect();
        for field in &fields {
            let mut found = 0;
            for (index, file) in self.files.iter().enumerate() {
                let document = Document::parse(&file.text).map_err(|error| {
                    editor::issue(&plan, "EDIT_UNSAFE", &field.object, error.to_string())
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
                    "The selected text-range edit has no unique original source object.",
                ));
            }
        }
        let previous: Vec<_> = self.files.iter().map(|file| file.text.clone()).collect();
        let checked: Result<ValidatedIntegrationPlan, Vec<PlanDiagnostic>> = (|| {
            for (index, edits) in patches.iter_mut().enumerate() {
                apply_patches(&mut self.files[index].text, edits)
                    .map_err(|error| failure("EDIT_UNSAFE", error))?;
            }
            self.integration_plan(runtime, mod_archive)
        })();
        match checked {
            Ok(plan) => {
                if let Err(error) = self.refresh() {
                    for (file, text) in self.files.iter_mut().zip(previous) {
                        file.text = text;
                    }
                    let _ = self.refresh();
                    return Err(failure("EDIT_UNSAFE", error));
                }
                Ok(IntegrationInspection {
                    profile: crate::integration::PROFILE.into(),
                    description: Some(plan.description().clone()),
                    diagnostics: Vec::new(),
                })
            }
            Err(issues) => {
                for (file, text) in self.files.iter_mut().zip(previous) {
                    file.text = text;
                }
                Err(issues)
            }
        }
    }

    /// Standard-input preview validates the same plan, without passing through
    /// the unrelated legacy host-v1 profile or serializing the source graph.
    pub fn preview_integration_save(
        &self,
        runtime: &RuntimeCatalog,
        mod_archive: PathBuf,
    ) -> Result<SavePreview, Vec<PlanDiagnostic>> {
        self.integration_plan(runtime, mod_archive)?;
        Ok(SavePreview {
            revision: self.save_revision(),
            files: self
                .files
                .iter()
                .map(|file| {
                    let changed = file.saved != file.text;
                    SavePreviewFile {
                        path: file.path.display().to_string(),
                        changed,
                        before: changed.then(|| file.saved.clone()),
                        after: changed.then(|| file.text.clone()),
                    }
                })
                .collect(),
        })
    }

    pub fn save_integration_previewed(
        &mut self,
        runtime: &RuntimeCatalog,
        mod_archive: PathBuf,
        revision: &str,
    ) -> Result<IntegrationInspection, Vec<PlanDiagnostic>> {
        if self.save_revision() != revision {
            return Err(failure(
                "SAVE_PREVIEW_STALE",
                "The in-memory inputs changed after preview; preview again before saving.",
            ));
        }
        self.integration_plan(runtime, mod_archive.clone())?;
        self.save_sources()
            .map_err(|error| failure("SAVE_FAILED", error))?;
        Ok(self.inspect_integration(runtime, mod_archive))
    }
}
