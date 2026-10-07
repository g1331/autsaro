mod legacy_editor;
use crate::arxml_render::render_profile;
use crate::model::{
    DiagnosticSettings, DiagnosticView, Direction, DtcView, FileView, FrameView, Issue,
    SavePreview, SavePreviewFile, Severity, SignalView, WorkspaceView, validate_diagnostic,
    validate_profile,
};
use crate::schema;
use roxmltree::{Document, Node};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

mod application;
mod changes;
mod host_profile;
mod integration_editor;
mod persistence;
mod project;
mod projection;
mod standard_template;
mod xml;

pub use application::{ApplicationInitializationOutcome, ApplicationInitializationPreview};

pub use project::{
    ApplicationInput, GenerationInputSnapshot, GenerationSnapshot, ProjectCreationPreview,
    ProjectFilePreview, ProjectInput, ProjectManifest,
};

pub use persistence::{PreparedSave, SaveFailure};

use host_profile::parse_host_routine;
use persistence::{Patch, apply_patches, patch_child, patch_param};
use xml::{
    child_containers, child_text, definition, param, parse_milliseconds, parse_u32, path_of,
    ref_dest, ref_value, seconds, signal_type, structural_node, valid_name,
};

const NS: &str = "http://autosar.org/schema/r4.0";
const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

#[derive(Clone)]
struct SourceFile {
    path: PathBuf,
    text: String,
    saved: String,
    original_name: Option<String>,
}

pub(crate) struct HandoffSource {
    pub original_name: String,
    pub package_roots: Vec<String>,
    pub contents: Vec<u8>,
}


#[derive(Clone)]
pub struct Workspace {
    name: String,
    files: Vec<SourceFile>,
    frames: Vec<FrameView>,
    signals: Vec<SignalView>,
    diagnostic: Option<DiagnosticView>,
    issues: Vec<Issue>,
    schema_zip: Option<PathBuf>,
    integration_input_root: Option<PathBuf>,
    catalog: std::sync::Arc<crate::definitions::DefinitionCatalog>,
    snapshot: std::sync::Arc<projection::SourceSnapshot>,
    epoch: String,
    next_identity: u64,
    revision: u64,
    project: Option<project::ProjectMembership>,
}

impl Workspace {
    pub fn set_legacy_validation_schema(&mut self, archive: PathBuf) -> Result<(), String> {
        if self.schema_zip.is_none() {
            return Err("Builtin validation cannot be replaced by external resources.".into());
        }
        self.schema_zip = Some(archive);
        Ok(())
    }

    pub fn uses_legacy_validation(&self) -> bool {
        self.schema_zip.is_some()
    }

    pub fn definition_fingerprint(&self) -> Result<String, String> {
        crate::rules::rule_set_identity()?;
        Ok(self.snapshot.definition_fingerprint.clone())
    }

    pub fn verify_saved_sources(&self) -> Result<(), String> {
        if self.files.iter().any(|file| file.text != file.saved)
            || self
                .project
                .as_ref()
                .is_some_and(|project| project.current != project.saved)
        {
            return Err("请先保存全部 ARXML，再执行交付操作".into());
        }
        self.ensure_sources_current()
    }
}

fn load_sources(files: Vec<PathBuf>, schema_zip: Option<PathBuf>) -> Result<Workspace, String> {
    if files.is_empty() {
        return Err("请选择至少一份 .arxml 文件".into());
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
            return Err(format!(
                "ARXML 来源路径包含 ..，拒绝导入: {}",
                path.display()
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
                return Err(format!(
                    "ARXML 来源路径包含链接或重解析点: {}",
                    ancestor.display()
                ));
            }
        }
        let path = fs::canonicalize(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("arxml"))
            || !unique.insert(path.to_string_lossy().to_uppercase())
        {
            return Err(format!("文件不是唯一的 .arxml: {}", path.display()));
        }
        if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 50 * 1024 * 1024 {
            return Err(format!("单份 ARXML 不得超过 50 MiB: {}", path.display()));
        }
        let text = String::from_utf8(project::read_bounded(&path)?)
            .map_err(|e| format!("{} 必须是 UTF-8 ARXML: {e}", path.display()))?;
        if text.contains("<!DOCTYPE") || text.contains("<!ENTITY") {
            return Err(format!(
                "{}: DTD and entity declarations are not accepted.",
                path.display()
            ));
        }
        let doc = Document::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if doc.root_element().tag_name().namespace() != Some(NS)
            || doc.root_element().tag_name().name() != "AUTOSAR"
        {
            return Err(format!("{} 不是 R24-11 AUTOSAR 文档", path.display()));
        }
        if schema_zip.is_some()
            && !doc
                .root_element()
                .attribute((XSI, "schemaLocation"))
                .is_some_and(|s| s.contains("AUTOSAR_00053.xsd"))
        {
            return Err(format!(
                "{} 未声明 AUTOSAR_00053.xsd；不猜测 ARXML 发布版本",
                path.display()
            ));
        }
        sources.push(SourceFile {
            path,
            saved: text.clone(),
            text,
            original_name: None,
        });
    }
    sources.sort_by(|a, b| a.path.cmp(&b.path));
    if let Some(archive) = &schema_zip {
        let schema_issues = schema::validate_files(
            archive,
            &sources
                .iter()
                .map(|s| (s.path.as_path(), s.text.as_str()))
                .collect::<Vec<_>>(),
        )?;
        if let Some(first) = schema_issues.first() {
            return Err(format!(
                "导入前 XSD 校验失败 {}: {}",
                first.file.as_deref().unwrap_or(""),
                first.message
            ));
        }
    }
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
        frames: Vec::new(),
        signals: Vec::new(),
        diagnostic: None,
        issues: Vec::new(),
        schema_zip,
        integration_input_root: None,
        catalog: std::sync::Arc::new(crate::definitions::DefinitionCatalog::builtin()?),
        snapshot: std::sync::Arc::new(projection::SourceSnapshot::default()),
        epoch: projection::new_epoch(),
        next_identity: 0,
        revision: 0,
        project: None,
    };
    workspace.refresh()?;
    if workspace
        .issues
        .iter()
        .any(|issue| issue.code == "DIAG_UNSUPPORTED")
    {
        for file in &workspace.files {
            if !file.text.contains("AutosarWorkbenchHostRestoreDid") {
                continue;
            }
            let doc = Document::parse(&file.text).map_err(|error| error.to_string())?;
            let nodes: Vec<_> = doc.descendants().filter(|node| node.is_element()).collect();
            for group in nodes.iter().copied().filter(|node| {
                node.tag_name().name() == "SDG"
                    && node
                        .attribute("GID")
                        .is_some_and(|gid| gid.starts_with("AutosarWorkbenchHostRestoreDid"))
            }) {
                let did = group
                    .ancestors()
                    .find(|node| {
                        node.is_element()
                            && node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                            && definition(*node).as_deref()
                                == Some("/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsp/DcmDspDid")
                    })
                    .ok_or_else(|| {
                        format!("{}: 主机复位例程不在 DID 容器中", file.path.display())
                    })?;
                parse_host_routine(did, &workspace.name, &nodes)
                    .map_err(|error| format!("{}: {error}", file.path.display()))?;
            }
        }
    }
    Ok(workspace)
}

impl Workspace {
    pub fn create(directory: &Path, name: &str) -> Result<Self, String> {
        let preview = Self::preview_project_creation(directory, name, "can-empty-v1")?;
        Self::create_project_previewed(&preview)
    }

    pub fn create_legacy(
        directory: &Path,
        name: &str,
        schema_zip: PathBuf,
    ) -> Result<Self, String> {
        if !valid_name(name) {
            return Err(
                "工程名须以 ASCII 字母开头，且仅含字母、数字和下划线（最多 128 字节）".into(),
            );
        }
        fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let path = directory.join(format!("{name}.arxml"));
        let text = render_profile(name, &[], &[], None);
        let issues = schema::validate_files(&schema_zip, &[(path.as_path(), text.as_str())])?;
        if let Some(first) = issues.first() {
            return Err(format!("空项目 XSD 校验失败: {}", first.message));
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| format!("不会覆盖已有 ARXML {}: {e}", path.display()))?;
        std::io::Write::write_all(&mut file, text.as_bytes()).map_err(|e| e.to_string())?;
        load_sources(vec![path], Some(schema_zip))
    }

    pub fn open(paths: Vec<PathBuf>) -> Result<Self, String> {
        load_sources(paths, None)
    }

    pub fn open_legacy(paths: Vec<PathBuf>, schema_zip: PathBuf) -> Result<Self, String> {
        load_sources(paths, Some(schema_zip))
    }

    pub(crate) fn preparation_input_identity(&self) -> String {
        let mut inputs: Vec<_> = self
            .files
            .iter()
            .map(|file| {
                (
                    file.original_name
                        .as_deref()
                        .map(std::borrow::Cow::Borrowed)
                        .unwrap_or_else(|| {
                            file.path
                                .file_name()
                                .unwrap_or_else(|| file.path.as_os_str())
                                .to_string_lossy()
                        }),
                    file.text.as_bytes(),
                )
            })
            .collect();
        inputs.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(right.1)));
        let mut digest = Sha256::new();
        for (name, contents) in inputs {
            digest.update((name.len() as u64).to_le_bytes());
            digest.update(name.as_bytes());
            digest.update((contents.len() as u64).to_le_bytes());
            digest.update(contents);
        }
        format!("{:x}", digest.finalize())
    }

    pub(crate) fn restore_handoff_source_names(
        &mut self,
        names: BTreeMap<PathBuf, String>,
    ) -> Result<(), String> {
        if names.len() != self.files.len()
            || self
                .files
                .iter()
                .any(|file| !names.contains_key(&file.path))
        {
            return Err("交付输入名称映射与实际 ARXML 文件不一致".into());
        }
        for file in &mut self.files {
            file.original_name = names.get(&file.path).cloned();
        }
        Ok(())
    }

    pub fn view(&self) -> WorkspaceView {
        WorkspaceView {
            integration_candidate: self.snapshot.integration_candidate,
            name: self.name.clone(),
            files: self
                .files
                .iter()
                .map(|file| {
                    let managed = self.is_managed_file(file);
                    let doc = Document::parse(&file.text).ok();
                    let supported = doc.as_ref().is_some_and(|d| {
                        d.descendants().any(|n| {
                            n.is_element()
                                && ((n.tag_name().name() == "I-SIGNAL-I-PDU"
                                    && self.frames.iter().any(|f| f.path == path_of(n)))
                                    || (n.tag_name().name() == "ECUC-CONTAINER-VALUE"
                                        && self.signals.iter().any(|s| s.path == path_of(n))))
                        })
                    });
                    let retained_count = if managed {
                        0
                    } else {
                        doc.as_ref()
                            .map(|d| {
                                d.descendants()
                                    .filter(|n| {
                                        n.is_element()
                                            && n.parent_element()
                                                .is_some_and(|p| p.tag_name().name() == "ELEMENTS")
                                            && !matches!(
                                                n.tag_name().name(),
                                                "I-SIGNAL-I-PDU"
                                                    | "ECUC-MODULE-CONFIGURATION-VALUES"
                                            )
                                            && !(n.tag_name().name() == "I-SIGNAL"
                                                && self.signals.iter().any(|s| {
                                                    path_of(*n)
                                                        == format!(
                                                            "/{}/ISignal_{}",
                                                            self.name, s.name
                                                        )
                                                }))
                                    })
                                    .count()
                            })
                            .unwrap_or(0)
                    };
                    FileView {
                        path: file.path.display().to_string(),
                        readonly: self.issues.iter().any(|issue| {
                            issue.code == "DIAG_UNSUPPORTED" || issue.code.starts_with("PDU_")
                        }) || !managed && !supported,
                        retained_count,
                    }
                })
                .collect(),
            frames: self.frames.clone(),
            signals: self.signals.clone(),
            diagnostic: self.diagnostic.clone(),
            issues: self.issues.clone(),
            dirty: self.is_dirty(),
        }
    }

    fn integration_sources(
        &self,
    ) -> Result<Vec<crate::integration::InputSource>, Vec<crate::integration::PlanDiagnostic>> {
        use crate::integration::{DiagnosticCategory, InputSource, PlanDiagnostic};
        let issue = |code: &str, message: String| {
            vec![PlanDiagnostic {
            category: DiagnosticCategory::Input, code: code.into(), file: None, object: None,
            message, remedy: "Reopen the original input set and resolve external changes before checking its integration plan.".into(),
        }]
        };
        self.ensure_sources_current()
            .map_err(|error| issue("SOURCE_CHANGED", error))?;
        let mut root = self
            .integration_input_root
            .as_deref()
            .or_else(|| self.files.first().and_then(|file| file.path.parent()))
            .ok_or_else(|| {
                issue(
                    "INPUT_MISSING",
                    "No input source directory is available.".into(),
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
                    "A delivered source escaped its declared logical input root.".into(),
                ));
            }
            root = root.parent().ok_or_else(|| {
                issue(
                    "SOURCE_IDENTITY",
                    "The source files do not share a portable input root.".into(),
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
                    .map_err(|error| issue("SOURCE_IDENTITY", error.to_string()))?;
                let logical = relative
                    .to_str()
                    .ok_or_else(|| {
                        issue(
                            "SOURCE_IDENTITY",
                            "The source identity is not UTF-8.".into(),
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
        runtime: &crate::integration::RuntimeCatalog,
    ) -> Result<crate::integration::ValidatedIntegrationPlan, Vec<crate::integration::PlanDiagnostic>>
    {
        if self.uses_legacy_validation() {
            return Err(integration_editor::failure(
                "VALIDATION_MODE",
                "Use explicit legacy resources for a legacy workspace.",
            ));
        }
        crate::integration::build_plan_native(&self.integration_sources()?, &self.catalog, runtime)
    }

    pub fn integration_plan_legacy(
        &self,
        runtime: &crate::integration::RuntimeCatalog,
        mod_archive: PathBuf,
    ) -> Result<crate::integration::ValidatedIntegrationPlan, Vec<crate::integration::PlanDiagnostic>>
    {
        let archive = self.schema_zip.clone().ok_or_else(|| {
            integration_editor::failure(
                "VALIDATION_MODE",
                "Legacy integration requires explicit official resources.",
            )
        })?;
        crate::integration::build_plan(
            &self.integration_sources()?,
            &crate::integration::PlanDependencies {
                xsd_archive: archive,
                mod_archive,
            },
            runtime,
        )
    }

    pub fn validate(&mut self) -> Result<WorkspaceView, String> {
        self.refresh()?;
        if let Some(archive) = &self.schema_zip {
            self.issues.extend(schema::validate_files(
                archive,
                &self
                    .files
                    .iter()
                    .map(|f| (f.path.as_path(), f.text.as_str()))
                    .collect::<Vec<_>>(),
            )?);
        } else {
            self.issues
                .extend(self.snapshot.validation.iter().flat_map(|scope| {
                    scope.diagnostics.iter().map(|diagnostic| Issue {
                        severity: diagnostic.severity.clone(),
                        code: diagnostic.code.clone(),
                        message: diagnostic.message.clone(),
                        file: diagnostic.file.clone(),
                        path: diagnostic.path.clone(),
                    })
                }));
        }
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
                            format!("跨文件引用未解析: {reference}"),
                            Some(path_of(node.parent_element().unwrap_or(node))),
                        )
                    });
                }
            }
            for node in doc
                .descendants()
                .filter(|n| n.is_element() && n.tag_name().name() == "VARIATION-POINT")
            {
                let active_package = node
                    .parent_element()
                    .filter(|parent| parent.tag_name().name() == "AR-PACKAGE")
                    .is_some_and(|package| {
                        let prefix = path_of(package);
                        self.frames.iter().any(|frame| {
                            frame
                                .path
                                .strip_prefix(&prefix)
                                .is_some_and(|suffix| suffix.starts_with('/'))
                        })
                    });
                let active_root = node
                    .parent_element()
                    .is_some_and(|parent| parent.tag_name().name() == "AUTOSAR")
                    && !self.frames.is_empty();
                if active_package
                    || active_root
                    || node.ancestors().any(|ancestor| {
                        matches!(
                            ancestor.tag_name().name(),
                            "I-SIGNAL-I-PDU" | "ECUC-MODULE-CONFIGURATION-VALUES"
                        )
                    })
                {
                    self.issues.push(Issue {
                        severity: Severity::Warning,
                        code: "VARIANT_DEPENDENCY".into(),
                        message: "配置含未解析变体，可保存但禁止生成".into(),
                        path: Some(path_of(node)),
                        file: Some(file.path.display().to_string()),
                    });
                }
            }
        }
        self.ensure_sources_current()?;
        Ok(self.view())
    }

    fn all_paths(&self) -> Result<BTreeSet<String>, String> {
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

    pub fn checked_profile(&mut self) -> Result<(Vec<FrameView>, Vec<SignalView>), String> {
        self.validate()?;
        if let Some(issue) = self
            .issues
            .iter()
            .find(|i| matches!(i.severity, Severity::Error))
        {
            return Err(format!("{}: {}", issue.code, issue.message));
        }
        if let Some(issue) = self.issues.iter().find(|i| i.code == "VARIANT_DEPENDENCY") {
            return Err(format!("{}: {}", issue.code, issue.message));
        }
        if self.frames.is_empty() || self.signals.is_empty() {
            return Err("NO_SIGNALS: 至少需要一帧和一个信号才能生成".into());
        }
        if let Some(frame) = self
            .frames
            .iter()
            .find(|f| !self.signals.iter().any(|s| s.frame_path == f.path))
        {
            return Err(format!(
                "FRAME_EMPTY: {} 没有信号，C99 运行代码无法初始化",
                frame.path
            ));
        }
        Ok((self.frames.clone(), self.signals.clone()))
    }

    pub(crate) fn diagnostic_profile(&self) -> Option<&DiagnosticView> {
        self.diagnostic.as_ref()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
