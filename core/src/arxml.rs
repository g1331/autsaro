use crate::model::{FileView, Issue, SavePreview, SavePreviewFile, Severity, WorkspaceView};
use roxmltree::{Document, Node};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

mod application;
mod changes;
mod integration_editor;
mod persistence;
mod project;
mod projection;
mod xml;

pub use application::{ApplicationInitializationOutcome, ApplicationInitializationPreview};

pub use project::{
    ApplicationInput, GenerationInputSnapshot, GenerationSnapshot, ProjectCreationPreview,
    ProjectFilePreview, ProjectInput, ProjectManifest,
};

pub use persistence::{PreparedSave, SaveFailure};

use persistence::{Patch, apply_patches, patch_child, patch_param};
use xml::{child_text, definition, path_of, valid_name};

const NS: &str = "http://autosar.org/schema/r4.0";

#[derive(Clone)]
struct SourceFile {
    path: PathBuf,
    text: String,
    saved: String,
}

#[derive(Clone)]
pub struct Workspace {
    name: String,
    files: Vec<SourceFile>,
    issues: Vec<Issue>,
    integration_input_root: Option<PathBuf>,
    catalog: std::sync::Arc<crate::definitions::DefinitionCatalog>,
    snapshot: std::sync::Arc<projection::SourceSnapshot>,
    epoch: String,
    next_identity: u64,
    revision: u64,
    project: Option<project::ProjectMembership>,
}

impl Workspace {
    pub fn definition_fingerprint(&self) -> Result<String, crate::message::LocalizedText> {
        crate::rules::rule_set_identity()?;
        Ok(self.snapshot.definition_fingerprint.clone())
    }

    pub fn verify_saved_sources(&self) -> Result<(), crate::message::LocalizedText> {
        if self.files.iter().any(|file| file.text != file.saved)
            || self
                .project
                .as_ref()
                .is_some_and(|project| project.current != project.saved)
        {
            return Err(crate::product_message!("backend.arxml.save_before_handoff"));
        }
        self.ensure_sources_current()
    }
}

fn integration_errors(
    diagnostics: Vec<crate::integration::PlanDiagnostic>,
) -> crate::message::LocalizedText {
    crate::message::LocalizedText::messages(diagnostics.into_iter().map(|diagnostic| {
        crate::message::LocalizedText::messages([
            crate::product_message!(
                "backend.arxml.integration_diagnostic_context",
                "code" => &diagnostic.code,
                "category" => serde_json::to_string(&diagnostic.category).expect("Diagnostic category serializes"),
                "file" => diagnostic.file.as_deref().unwrap_or_default(),
                "object" => diagnostic.object.as_deref().unwrap_or_default()
            ),
            diagnostic.message,
            diagnostic.remedy,
        ])
    }))
}

fn load_sources(files: Vec<PathBuf>) -> Result<Workspace, crate::message::LocalizedText> {
    load_sources_with(files, |_| {})
}

fn load_sources_with(
    files: Vec<PathBuf>,
    initialize: impl FnOnce(&mut Workspace),
) -> Result<Workspace, crate::message::LocalizedText> {
    if files.is_empty() {
        return Err(crate::product_message!(
            "backend.arxml.input_files_required"
        ));
    }
    let mut sources = Vec::new();
    let mut unique = BTreeSet::new();
    for path in files {
        let selected = if path.is_absolute() {
            path.clone()
        } else {
            std::env::current_dir()
                .map_err(|e| e.to_string())?
                .join(&path)
        };
        if selected
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(crate::product_message!(
                "backend.arxml.source_parent_traversal",
                "path" => path.display()
            ));
        }
        for ancestor in selected.ancestors() {
            let metadata = fs::symlink_metadata(ancestor)
                .map_err(|e| format!("{}: {e}", ancestor.display()))?;
            #[cfg(windows)]
            let linked = {
                use std::os::windows::fs::MetadataExt;
                metadata.file_attributes() & 0x400 != 0
            };
            #[cfg(not(windows))]
            let linked = metadata.file_type().is_symlink();
            if linked {
                return Err(crate::product_message!(
                    "backend.arxml.source_link_rejected",
                    "path" => ancestor.display()
                ));
            }
        }
        let path = fs::canonicalize(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("arxml"))
            || !unique.insert(path.to_string_lossy().to_uppercase())
        {
            return Err(crate::product_message!(
                "backend.arxml.unique_arxml_required",
                "path" => path.display()
            ));
        }
        if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 50 * 1024 * 1024 {
            return Err(crate::product_message!(
                "backend.arxml.arxml_size_limit",
                "path" => path.display()
            ));
        }
        let text = String::from_utf8(project::read_bounded(&path)?).map_err(|e| {
            crate::message::LocalizedText::messages([
                crate::product_message!(
                    "backend.arxml.utf8_arxml_required",
                    "path" => path.display()
                ),
                e.to_string().into(),
            ])
        })?;
        if text.contains("<!DOCTYPE") || text.contains("<!ENTITY") {
            return Err(crate::product_message!(
                "backend.arxml.xml_declarations_rejected",
                "path" => path.display()
            ));
        }
        let doc = Document::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if doc.root_element().tag_name().namespace() != Some(NS)
            || doc.root_element().tag_name().name() != "AUTOSAR"
        {
            return Err(crate::product_message!(
                "backend.arxml.autosar_document_required",
                "path" => path.display()
            ));
        }
        sources.push(SourceFile {
            path,
            saved: text.clone(),
            text,
        });
    }
    sources.sort_by(|a, b| a.path.cmp(&b.path));
    let name = sources
        .iter()
        .filter_map(|s| Document::parse(&s.text).ok())
        .flat_map(|d| {
            d.root_element()
                .descendants()
                .filter(|n| n.is_element() && n.tag_name().name() == "AR-PACKAGE")
                .filter_map(|n| child_text(n, "SHORT-NAME"))
                .collect::<Vec<_>>()
        })
        .next()
        .unwrap_or_else(|| "ImportedEcu".into());
    let mut workspace = Workspace {
        name,
        files: sources,
        issues: Vec::new(),
        integration_input_root: None,
        catalog: std::sync::Arc::new(crate::definitions::DefinitionCatalog::builtin()?),
        snapshot: std::sync::Arc::new(projection::SourceSnapshot::default()),
        epoch: projection::new_epoch(),
        next_identity: 0,
        revision: 0,
        project: None,
    };
    // Membership and its accepted catalogue belong to the first validated snapshot.
    initialize(&mut workspace);
    workspace.refresh()?;
    Ok(workspace)
}

impl Workspace {
    pub fn create(directory: &Path, name: &str) -> Result<Self, crate::message::LocalizedText> {
        let preview = Self::preview_project_creation(directory, name, "standard-ecu")?;
        Self::create_project_previewed(&preview)
    }

    pub fn open(paths: Vec<PathBuf>) -> Result<Self, crate::message::LocalizedText> {
        load_sources(paths)
    }

    pub fn view(&self) -> WorkspaceView {
        WorkspaceView {
            integration_candidate: self.snapshot.integration_candidate,
            name: self.name.clone(),
            files: self
                .files
                .iter()
                .map(|file| FileView {
                    path: file.path.display().to_string(),
                    // Sources change only through validated structured edits.
                    readonly: true,
                    retained_count: Document::parse(&file.text)
                        .map(|d| {
                            d.descendants()
                                .filter(|n| {
                                    n.is_element()
                                        && n.parent_element()
                                            .is_some_and(|p| p.tag_name().name() == "ELEMENTS")
                                        && !matches!(
                                            n.tag_name().name(),
                                            "I-SIGNAL-I-PDU" | "ECUC-MODULE-CONFIGURATION-VALUES"
                                        )
                                })
                                .count()
                        })
                        .unwrap_or(0),
                })
                .collect(),
            issues: self.issues.clone(),
            dirty: self.is_dirty(),
        }
    }

    fn integration_sources(
        &self,
    ) -> Result<Vec<crate::integration::InputSource>, Vec<crate::integration::PlanDiagnostic>> {
        self.ensure_sources_current().map_err(|error| {
            vec![crate::integration::PlanDiagnostic {
                category: crate::integration::DiagnosticCategory::Input,
                code: "SOURCE_CHANGED".into(),
                file: None,
                object: None,
                message: error,
                remedy: crate::product_message!("backend.arxml.reopen_inputs_remedy"),
            }]
        })?;
        self.integration_source_snapshot()
    }

    fn integration_source_snapshot(
        &self,
    ) -> Result<Vec<crate::integration::InputSource>, Vec<crate::integration::PlanDiagnostic>> {
        use crate::integration::{DiagnosticCategory, InputSource, PlanDiagnostic};
        let issue = |code: &str, message: crate::message::LocalizedText| {
            vec![PlanDiagnostic {
                category: DiagnosticCategory::Input,
                code: code.into(),
                file: None,
                object: None,
                message,
                remedy: crate::product_message!("backend.arxml.reopen_inputs_remedy"),
            }]
        };
        let mut root = self
            .integration_input_root
            .as_deref()
            .or_else(|| self.files.first().and_then(|file| file.path.parent()))
            .ok_or_else(|| {
                issue(
                    "INPUT_MISSING",
                    crate::product_message!("backend.arxml.input_directory_missing"),
                )
            })?;
        while !self
            .files
            .iter()
            .all(|file| file.path.strip_prefix(root).is_ok())
        {
            if self.integration_input_root.is_some() {
                return Err(issue(
                    "SOURCE_IDENTITY",
                    crate::product_message!("backend.arxml.source_outside_input_root"),
                ));
            }
            root = root.parent().ok_or_else(|| {
                issue(
                    "SOURCE_IDENTITY",
                    crate::product_message!("backend.arxml.portable_input_root_required"),
                )
            })?;
        }
        let sources: Vec<_> = self
            .files
            .iter()
            .map(|file| {
                let relative = file
                    .path
                    .strip_prefix(root)
                    .map_err(|error| issue("SOURCE_IDENTITY", error.to_string().into()))?;
                let logical = relative
                    .to_str()
                    .ok_or_else(|| {
                        issue(
                            "SOURCE_IDENTITY",
                            crate::product_message!("backend.arxml.source_identity_utf8_required"),
                        )
                    })?
                    .replace('\\', "/");
                InputSource::new(logical, file.text.as_bytes().to_vec())
                    .map_err(|issue| vec![issue])
            })
            .collect::<Result<_, _>>()?;
        Ok(sources)
    }

    pub fn integration_plan(
        &self,
    ) -> Result<crate::integration::ValidatedIntegrationPlan, Vec<crate::integration::PlanDiagnostic>>
    {
        crate::integration::build_plan_native(&self.integration_sources()?, &self.catalog)
    }

    pub fn validate(&mut self) -> Result<WorkspaceView, crate::message::LocalizedText> {
        self.refresh()?;
        let paths = self.all_paths()?;
        for file in &self.files {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| {
                n.is_element()
                    && n.tag_name().name().ends_with("-REF")
                    && n.tag_name().name() != "DEFINITION-REF"
            }) {
                if let Some(reference) = node.text()
                    && reference.starts_with('/')
                    && !paths.contains(reference)
                    && !reference.starts_with("/AUTOSAR/EcucDefs/")
                {
                    self.issues.push(Issue {
                        file: Some(file.path.display().to_string()),
                        ..Issue::error(
                            "UNRESOLVED_REF",
                            crate::product_message!(
                                "backend.arxml.unresolved_reference",
                                "reference" => reference
                            ),
                            Some(path_of(node.parent_element().unwrap_or(node))),
                        )
                    });
                }
            }
            for node in doc
                .descendants()
                .filter(|n| n.is_element() && n.tag_name().name() == "VARIATION-POINT")
            {
                if node.ancestors().any(|ancestor| {
                    matches!(
                        ancestor.tag_name().name(),
                        "I-SIGNAL-I-PDU" | "ECUC-MODULE-CONFIGURATION-VALUES"
                    )
                }) {
                    self.issues.push(Issue {
                        severity: Severity::Warning,
                        code: "VARIANT_DEPENDENCY".into(),
                        message: crate::product_message!("backend.arxml.unresolved_variant"),
                        path: Some(path_of(node)),
                        file: Some(file.path.display().to_string()),
                    });
                }
            }
        }
        self.ensure_sources_current()?;
        Ok(self.view())
    }

    fn all_paths(&self) -> Result<BTreeSet<String>, crate::message::LocalizedText> {
        let mut paths = BTreeSet::new();
        for file in &self.files {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc
                .descendants()
                .filter(|n| n.is_element() && child_text(*n, "SHORT-NAME").is_some())
            {
                paths.insert(path_of(node));
            }
        }
        Ok(paths)
    }

    pub(super) fn refresh(&mut self) -> Result<(), crate::message::LocalizedText> {
        self.rebuild_snapshot()?;
        self.issues = self
            .snapshot
            .validation
            .iter()
            .flat_map(|scope| &scope.diagnostics)
            .map(|issue| Issue {
                severity: issue.severity.clone(),
                code: issue.code.clone(),
                message: issue.message.clone(),
                file: issue.file.clone(),
                path: issue.path.clone(),
            })
            .collect();
        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
