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

mod host_profile;
mod integration_editor;
mod persistence;
mod xml;

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
    schema_zip: PathBuf,
    integration_input_root: Option<PathBuf>,
}

impl Workspace {
    pub fn set_validation_schema(&mut self, archive: PathBuf) {
        self.schema_zip = archive;
    }

    pub fn verify_saved_sources(&self) -> Result<(), String> {
        if self.files.iter().any(|file| file.text != file.saved) {
            return Err("请先保存全部 ARXML，再执行交付操作".into());
        }
        self.ensure_sources_current()
    }
}

fn load_sources(files: Vec<PathBuf>, schema_zip: PathBuf) -> Result<Workspace, String> {
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
            || !unique.insert(path.clone())
        {
            return Err(format!("文件不是唯一的 .arxml: {}", path.display()));
        }
        if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 50 * 1024 * 1024 {
            return Err(format!("单份 ARXML 不得超过 50 MiB: {}", path.display()));
        }
        let text = fs::read_to_string(&path)
            .map_err(|e| format!("{} 必须是 UTF-8 ARXML: {e}", path.display()))?;
        let doc = Document::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if doc.root_element().tag_name().namespace() != Some(NS)
            || doc.root_element().tag_name().name() != "AUTOSAR"
        {
            return Err(format!("{} 不是 R24-11 AUTOSAR 文档", path.display()));
        }
        if !doc
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
    let schema_issues = schema::validate_files(
        &schema_zip,
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
    pub fn create(directory: &Path, name: &str, schema_zip: PathBuf) -> Result<Self, String> {
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
        load_sources(vec![path], schema_zip)
    }

    pub fn open(paths: Vec<PathBuf>, schema_zip: PathBuf) -> Result<Self, String> {
        load_sources(paths, schema_zip)
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
            integration_candidate: self.files.iter().any(|file| {
                Document::parse(&file.text).ok().is_some_and(|document| {
                    document.descendants().any(|node| {
                        node.is_element()
                            && node.tag_name().namespace() == Some(NS)
                            && node.tag_name().name() == "SYSTEM"
                            && child_text(node, "CATEGORY").as_deref() == Some("ECU_EXTRACT")
                    })
                })
            }),
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
            dirty: self.files.iter().any(|file| file.saved != file.text),
        }
    }

    fn replace_managed(
        &mut self,
        frames: Vec<FrameView>,
        signals: Vec<SignalView>,
        diagnostic: Option<DiagnosticView>,
    ) -> Result<(), String> {
        let index = self
            .files
            .iter()
            .position(|f| self.is_managed_file(f))
            .ok_or(
                "当前项目没有可安全重建的配置文件；已保留导入内容，不执行可能破坏未知项的新增操作",
            )?;
        let mut frames = frames;
        let mut signals = signals;
        frames.sort_by(|a, b| a.path.cmp(&b.path));
        signals.sort_by(|a, b| a.path.cmp(&b.path));
        let mut issues = validate_profile(&frames, &signals);
        if let Some(diagnostic) = &diagnostic {
            issues.extend(validate_diagnostic(diagnostic, &frames, &signals));
        }
        if let Some(first) = issues.first() {
            return Err(format!("{}: {}", first.code, first.message));
        }
        let previous = self.files[index].text.clone();
        self.files[index].text = render_profile(&self.name, &frames, &signals, diagnostic.as_ref());
        if let Err(error) = self.refresh() {
            self.files[index].text = previous;
            let _ = self.refresh();
            return Err(error);
        }
        // A new Rx frame has no ComSignal/ComTimeout until its first signal is added.
        // Keep the requested timeout in the editing model; validation/save still reject
        // the incomplete on-disk profile if the user stops before adding that signal.
        for frame in &mut self.frames {
            if matches!(frame.direction, Direction::Rx)
                && !signals.iter().any(|signal| signal.frame_path == frame.path)
            {
                frame.timeout_ms = frames
                    .iter()
                    .find(|source| source.path == frame.path)
                    .and_then(|source| source.timeout_ms);
            }
        }
        self.issues.retain(|issue| {
            !(issue.code == "RX_TIMEOUT"
                && frames.iter().any(|frame| {
                    matches!(frame.direction, Direction::Rx)
                        && issue.path.as_deref() == Some(frame.path.as_str())
                        && !signals.iter().any(|signal| signal.frame_path == frame.path)
                }))
        });
        if let Some(issue) = self
            .issues
            .iter()
            .find(|i| matches!(i.severity, Severity::Error))
        {
            let error = format!("{}: {}", issue.code, issue.message);
            self.files[index].text = previous;
            self.refresh()?;
            return Err(error);
        }
        Ok(())
    }

    pub fn add_frame(
        &mut self,
        name: String,
        id: u32,
        dlc: u8,
        direction: Direction,
        period_ms: Option<u32>,
        timeout_ms: Option<u32>,
    ) -> Result<WorkspaceView, String> {
        if !valid_name(&name) {
            return Err("帧名称只能包含 ASCII 字母、数字与下划线，且须以字母开头".into());
        }
        let path = format!("/{}/Pdu_{}", self.name, name);
        if self.frames.iter().any(|f| f.path == path) {
            return Err("同名帧已经存在".into());
        }
        let mut frames = self.frames.clone();
        frames.push(FrameView {
            path,
            name,
            id,
            dlc,
            direction,
            period_ms,
            timeout_ms,
        });
        self.replace_managed(frames, self.signals.clone(), self.diagnostic.clone())?;
        Ok(self.view())
    }

    pub fn add_signal(
        &mut self,
        frame_path: String,
        name: String,
        start_bit: u8,
        length: u8,
        initial_value: u32,
    ) -> Result<WorkspaceView, String> {
        if !valid_name(&name) {
            return Err("信号名称只能包含 ASCII 字母、数字与下划线，且须以字母开头".into());
        }
        if !self.frames.iter().any(|f| f.path == frame_path) {
            return Err("关联帧不存在".into());
        }
        let path = format!("/{}/ComCfg/ComConfig/{}", self.name, name);
        if self.signals.iter().any(|s| s.path == path) {
            return Err("同名信号已经存在".into());
        }
        let mut signals = self.signals.clone();
        signals.push(SignalView {
            path,
            name,
            frame_path,
            start_bit,
            length,
            initial_value,
        });
        self.replace_managed(self.frames.clone(), signals, self.diagnostic.clone())?;
        Ok(self.view())
    }
    pub fn configure_diagnostic(
        &mut self,
        settings: DiagnosticSettings,
    ) -> Result<WorkspaceView, String> {
        let diagnostic = DiagnosticView {
            path: format!("/{}/DcmCfg/DcmConfigSet/DcmDsp/Did", self.name),
            request_id: settings.request_id,
            response_id: settings.response_id,
            s3_ms: settings.s3_ms,
            n_as_ms: settings.n_as_ms.unwrap_or(settings.n_bs_ms),
            n_bs_ms: settings.n_bs_ms,
            n_cr_ms: settings.n_cr_ms,
            did: settings.did,
            signal_paths: settings.signal_paths,
            write_enabled: settings.write_enabled,
            reset_routine_id: settings.reset_routine_id,
            security_enabled: settings.security_enabled,
            dtc: self
                .diagnostic
                .as_ref()
                .and_then(|existing| existing.dtc.clone()),
        };
        self.replace_managed(self.frames.clone(), self.signals.clone(), Some(diagnostic))?;
        Ok(self.view())
    }
    pub fn configure_dtc(
        &mut self,
        code: u32,
        monitor_frame_path: String,
    ) -> Result<WorkspaceView, String> {
        let mut diagnostic = self
            .diagnostic
            .clone()
            .ok_or("须先配置诊断服务，再配置 UDS DTC")?;
        diagnostic.dtc = Some(DtcView {
            path: format!("/{}/DemCfg/DemConfigSet/DTC", self.name),
            code,
            monitor_frame_path,
        });
        if let Some(issue) = validate_diagnostic(&diagnostic, &self.frames, &self.signals).first() {
            return Err(format!("{}: {}", issue.code, issue.message));
        }
        self.replace_managed(self.frames.clone(), self.signals.clone(), Some(diagnostic))?;
        Ok(self.view())
    }

    pub fn clear_dtc(&mut self) -> Result<WorkspaceView, String> {
        let mut diagnostic = self.diagnostic.clone().ok_or("当前工程没有诊断配置")?;
        if diagnostic.dtc.is_none() {
            return Err("当前工程没有可移除的受支持 DTC".into());
        }
        if diagnostic.security_enabled && !diagnostic.write_enabled {
            return Err("先关闭 0x27 安全档案，再移除唯一受保护的 DTC".into());
        }
        diagnostic.dtc = None;
        if self.files.iter().any(|file| self.is_managed_file(file)) {
            self.replace_managed(self.frames.clone(), self.signals.clone(), Some(diagnostic))?;
            return Ok(self.view());
        }
        let targets: BTreeSet<_> = [
            format!("/{}/DemCfg", self.name),
            format!("/{}/NvMCfg", self.name),
            format!(
                "/{}/DcmCfg/DcmConfigSet/DcmDsd/Services/ClearDiagnosticInformation",
                self.name
            ),
            format!(
                "/{}/DcmCfg/DcmConfigSet/DcmDsd/Services/ReadDTCInformation",
                self.name
            ),
            format!(
                "/{}/DcmCfg/DcmConfigSet/DcmDsd/Services/ControlDTCSetting",
                self.name
            ),
            format!("/{}/DcmCfg/DcmConfigSet/DcmDsp/ClearDTC", self.name),
            format!(
                "/{}/DcmCfg/DcmConfigSet/DcmDsp/ReadDTCInformation",
                self.name
            ),
            format!(
                "/{}/DcmCfg/DcmConfigSet/DcmDsp/ControlDTCSetting",
                self.name
            ),
        ]
        .into_iter()
        .collect();
        let expected = render_profile(
            &self.name,
            &self.frames,
            &self.signals,
            self.diagnostic.as_ref(),
        );
        let expected_doc = Document::parse(&expected).map_err(|e| e.to_string())?;
        let expected_nodes: BTreeMap<_, _> = expected_doc
            .descendants()
            .filter(|n| {
                n.is_element()
                    && child_text(*n, "SHORT-NAME").is_some()
                    && targets.contains(&path_of(*n))
            })
            .map(|n| (path_of(n), structural_node(n)))
            .collect();
        let mut patches: Vec<Vec<Patch>> = (0..self.files.len()).map(|_| Vec::new()).collect();
        let mut removed = BTreeSet::new();
        let mut owned_paths = BTreeSet::new();
        let row_path = format!("/{}/DcmCfg/DcmConfigSet/DcmDsl/Protocol/UdsCan", self.name);
        let client_path = format!("/{}/DemCfg/DemGeneral/DcmClient", self.name);
        let client_definition = "/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsl/DcmDslProtocol/DcmDslProtocolRow/DcmDemClientRef";
        let mut removed_client_ref: Option<(usize, std::ops::Range<usize>)> = None;
        for (index, file) in self.files.iter().enumerate() {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for row in doc.descendants().filter(|node| {
                node.is_element()
                    && node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                    && path_of(*node) == row_path
            }) {
                if definition(row).as_deref()
                    != Some(
                        "/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsl/DcmDslProtocol/DcmDslProtocolRow",
                    )
                {
                    return Err(format!("{row_path} 定义不匹配，拒绝移除 DTC"));
                }
                let mut references = row
                    .children()
                    .filter(|node| {
                        node.is_element() && node.tag_name().name() == "REFERENCE-VALUES"
                    })
                    .flat_map(|group| group.children().filter(|node| node.is_element()))
                    .filter(|node| definition(*node).as_deref() == Some(client_definition));
                let link = references
                    .next()
                    .ok_or_else(|| format!("{row_path} 缺少 DcmDemClientRef"))?;
                if references.next().is_some() || removed_client_ref.is_some() {
                    return Err(format!("{row_path} 有重复的 DcmDemClientRef"));
                }
                let target = link
                    .children()
                    .find(|node| node.is_element() && node.tag_name().name() == "VALUE-REF")
                    .ok_or_else(|| format!("{row_path} 的 DcmDemClientRef 缺少目标"))?;
                if link.tag_name().name() != "ECUC-REFERENCE-VALUE"
                    || target.attribute("DEST") != Some("ECUC-CONTAINER-VALUE")
                    || target.text() != Some(client_path.as_str())
                {
                    return Err(format!("{row_path} 的 DcmDemClientRef 不属于当前 DTC 配置"));
                }
                let range = link.range();
                patches[index].push(Patch {
                    range: range.clone(),
                    value: String::new(),
                });
                removed_client_ref = Some((index, range));
            }
            for node in doc.descendants().filter(|n| {
                n.is_element()
                    && child_text(*n, "SHORT-NAME").is_some()
                    && targets.contains(&path_of(*n))
            }) {
                let path = path_of(node);
                if expected_nodes.get(&path) != Some(&structural_node(node)) {
                    return Err(format!("{path} 含未知或非工具所有的内容，拒绝删除"));
                }
                if !removed.insert(path.clone()) {
                    return Err(format!("重复的 DTC 元素 {path}"));
                }
                owned_paths.extend(
                    node.descendants()
                        .filter(|n| n.is_element() && child_text(*n, "SHORT-NAME").is_some())
                        .map(path_of),
                );
                patches[index].push(Patch {
                    range: node.range(),
                    value: String::new(),
                });
            }
        }
        if removed_client_ref.is_none() {
            return Err(format!("{row_path} 缺少唯一的 DcmDemClientRef"));
        }
        if removed != targets {
            return Err("DTC ARXML 节点不完整，拒绝部分删除".into());
        }
        for (index, file) in self.files.iter().enumerate() {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| {
                n.is_element()
                    && n.tag_name().name().ends_with("-REF")
                    && n.tag_name().name() != "DEFINITION-REF"
            }) {
                if let Some(target) = node.text()
                    && owned_paths.contains(target)
                    && !node
                        .ancestors()
                        .any(|ancestor| removed.contains(&path_of(ancestor)))
                {
                    if removed_client_ref
                        .as_ref()
                        .is_some_and(|(file_index, range)| {
                            *file_index == index
                                && range.start <= node.range().start
                                && node.range().end <= range.end
                        })
                    {
                        continue;
                    }
                    return Err(format!("外部引用 {target} 仍依赖 DTC 配置，拒绝删除"));
                }
            }
        }
        self.commit_patches(patches, |workspace| {
            workspace
                .diagnostic
                .as_ref()
                .is_some_and(|d| d.dtc.is_none())
        })?;
        Ok(self.view())
    }

    pub fn clear_diagnostic(&mut self) -> Result<WorkspaceView, String> {
        if self.diagnostic.is_none() {
            return Err("当前工程没有可移除的受支持诊断配置".into());
        }
        if self.files.iter().any(|file| self.is_managed_file(file)) {
            self.replace_managed(self.frames.clone(), self.signals.clone(), None)?;
            return Ok(self.view());
        }
        let mut patches: Vec<Vec<Patch>> = (0..self.files.len()).map(|_| Vec::new()).collect();
        let mut removed = BTreeSet::new();
        let mut owned_paths = BTreeSet::new();
        let mut targets: BTreeSet<_> = [
            format!("/{}/DcmCfg", self.name),
            format!("/{}/CanTpCfg", self.name),
            format!("/{}/NPdu_DiagRequest", self.name),
            format!("/{}/NPdu_DiagResponse", self.name),
            format!("/{}/DcmPdu_DiagRequest", self.name),
            format!("/{}/DcmPdu_DiagResponse", self.name),
        ]
        .into_iter()
        .collect();
        for name in [
            "NPdu_DiagRequest",
            "NPdu_DiagResponse",
            "DcmPdu_DiagRequest",
            "DcmPdu_DiagResponse",
        ] {
            targets.insert(format!("/{}/EcuCCfg/EcucConfigSet/Pdus/{name}", self.name));
        }
        if self.diagnostic.as_ref().is_some_and(|d| d.dtc.is_some()) {
            targets.insert(format!("/{}/DemCfg", self.name));
            targets.insert(format!("/{}/NvMCfg", self.name));
        }
        let canif_targets: BTreeSet<_> = [
            format!("/{}/CanIfCfg/CanIfInitCfg/Can_DiagRequest", self.name),
            format!("/{}/CanIfCfg/CanIfInitCfg/Can_DiagResponse", self.name),
        ]
        .into_iter()
        .collect();
        let expected = render_profile(
            &self.name,
            &self.frames,
            &self.signals,
            self.diagnostic.as_ref(),
        );
        let expected_doc = Document::parse(&expected).map_err(|e| e.to_string())?;
        let expected_nodes: BTreeMap<_, _> = expected_doc
            .descendants()
            .filter(|n| {
                n.is_element()
                    && child_text(*n, "SHORT-NAME").is_some()
                    && (targets.contains(&path_of(*n)) || canif_targets.contains(&path_of(*n)))
            })
            .map(|n| (path_of(n), structural_node(n)))
            .collect();
        for file in &self.files {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element()) {
                let path = path_of(node);
                if (matches!(
                    node.tag_name().name(),
                    "ECUC-MODULE-CONFIGURATION-VALUES"
                        | "ECUC-CONTAINER-VALUE"
                        | "N-PDU"
                        | "DCM-I-PDU"
                ) && targets.contains(&path))
                    || (node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                        && canif_targets.contains(&path))
                {
                    owned_paths.extend(
                        node.descendants()
                            .filter(|n| n.is_element() && child_text(*n, "SHORT-NAME").is_some())
                            .map(path_of),
                    );
                }
            }
        }
        let mut updated_pdu_length_type = false;
        for (index, file) in self.files.iter().enumerate() {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element()) {
                let path = path_of(node);
                if node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                    && definition(node).as_deref()
                        == Some("/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection")
                    && path == format!("/{}/EcuCCfg/EcucConfigSet/Pdus", self.name)
                {
                    if updated_pdu_length_type {
                        return Err("重复的 EcuC 全局 PDU 集合".into());
                    }
                    patch_param(
                        node,
                        "PduLengthTypeEnum",
                        "UINT8".into(),
                        &mut patches[index],
                    )?;
                    updated_pdu_length_type = true;
                }
                let owned = (matches!(
                    node.tag_name().name(),
                    "ECUC-MODULE-CONFIGURATION-VALUES"
                        | "ECUC-CONTAINER-VALUE"
                        | "N-PDU"
                        | "DCM-I-PDU"
                ) && targets.contains(&path))
                    || (node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                        && canif_targets.contains(&path));
                if !owned {
                    continue;
                }
                if expected_nodes.get(&path) != Some(&structural_node(node)) {
                    return Err(format!("{path} 含非工具所有的 ARXML 内容，拒绝删除"));
                }
                if !removed.insert(path.clone()) {
                    return Err(format!("重复的诊断元素 {path}"));
                }
                patches[index].push(Patch {
                    range: node.range(),
                    value: String::new(),
                });
            }
        }
        if !updated_pdu_length_type || removed.len() != targets.len() + canif_targets.len() {
            return Err("诊断 ARXML 或全局 PDU 节点不完整，拒绝部分删除".into());
        }
        for file in &self.files {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants() {
                let target = if node.is_text() || node.is_comment() {
                    node.text().and_then(|text| {
                        owned_paths.iter().find(|path| text.contains(path.as_str()))
                    })
                } else if node.is_element() {
                    node.attributes().find_map(|attribute| {
                        owned_paths
                            .iter()
                            .find(|path| attribute.value().contains(path.as_str()))
                    })
                } else {
                    None
                };
                if let Some(target) = target {
                    if node
                        .ancestors()
                        .any(|ancestor| removed.contains(&path_of(ancestor)))
                    {
                        continue;
                    }
                    return Err(format!(
                        "{}: 外部内容 {} 仍引用诊断配置 {target}，拒绝删除",
                        file.path.display(),
                        path_of(node.parent_element().unwrap_or(node))
                    ));
                }
            }
        }
        self.commit_patches(patches, |workspace| workspace.diagnostic.is_none())?;
        Ok(self.view())
    }

    pub fn update_frame(&mut self, path: &str, changes: Value) -> Result<WorkspaceView, String> {
        let old = self
            .frames
            .iter()
            .find(|f| f.path == path)
            .ok_or("帧不存在")?
            .clone();
        let mut frames = self.frames.clone();
        let frame = frames
            .iter_mut()
            .find(|f| f.path == path)
            .ok_or("帧不存在")?;
        if changes
            .get("name")
            .is_some_and(|v| v.as_str() != Some(&frame.name))
        {
            return Err("重命名可能破坏跨文件引用；当前不允许重命名现有帧".into());
        }
        if let Some(v) = changes.get("id") {
            frame.id = v
                .as_u64()
                .ok_or("CAN 标识符须为无符号整数")?
                .try_into()
                .map_err(|_| "CAN 标识符超出范围")?;
        }
        if let Some(v) = changes.get("dlc") {
            frame.dlc = v
                .as_u64()
                .ok_or("DLC 须为整数")?
                .try_into()
                .map_err(|_| "DLC 超出范围")?;
        }
        if let Some(v) = changes.get("periodMs") {
            frame.period_ms = if v.is_null() {
                None
            } else {
                Some(
                    v.as_u64()
                        .ok_or("周期须为整数")?
                        .try_into()
                        .map_err(|_| "周期超出范围")?,
                )
            };
        }
        if let Some(v) = changes.get("timeoutMs") {
            frame.timeout_ms = if v.is_null() {
                None
            } else {
                Some(
                    v.as_u64()
                        .ok_or("超时须为整数")?
                        .try_into()
                        .map_err(|_| "超时超出范围")?,
                )
            };
        }
        if let Some(v) = changes.get("direction") {
            let current = match frame.direction {
                Direction::Tx => "tx",
                Direction::Rx => "rx",
            };
            if v.as_str() != Some(current) {
                return Err("切换方向需要重新建立 Com/CanIf 映射；请创建新帧".into());
            }
        }
        if let Some(issue) = validate_profile(&frames, &self.signals).first() {
            return Err(format!("{}: {}", issue.code, issue.message));
        }
        if let Some(diagnostic) = &self.diagnostic
            && let Some(issue) = validate_diagnostic(diagnostic, &frames, &self.signals).first()
        {
            return Err(format!("{}: {}", issue.code, issue.message));
        }
        let updated = frames.iter().find(|f| f.path == path).unwrap().clone();
        if self.files.iter().any(|file| self.is_managed_file(file)) {
            self.replace_managed(frames, self.signals.clone(), self.diagnostic.clone())?;
        } else {
            self.patch_imported_frame(&old, &updated)?;
        }
        Ok(self.view())
    }

    pub fn update_signal(&mut self, path: &str, changes: Value) -> Result<WorkspaceView, String> {
        let old = self
            .signals
            .iter()
            .find(|s| s.path == path)
            .ok_or("信号不存在")?
            .clone();
        let mut signals = self.signals.clone();
        let signal = signals
            .iter_mut()
            .find(|s| s.path == path)
            .ok_or("信号不存在")?;
        if changes
            .get("name")
            .is_some_and(|v| v.as_str() != Some(&signal.name))
            || changes
                .get("framePath")
                .is_some_and(|v| v.as_str() != Some(&signal.frame_path))
        {
            return Err("重命名或迁移信号可能破坏跨文件引用；请创建新信号".into());
        }
        if let Some(v) = changes.get("startBit") {
            signal.start_bit = v
                .as_u64()
                .ok_or("起始位须为整数")?
                .try_into()
                .map_err(|_| "起始位超出范围")?;
        }
        if let Some(v) = changes.get("length") {
            signal.length = v
                .as_u64()
                .ok_or("位长须为整数")?
                .try_into()
                .map_err(|_| "位长超出范围")?;
        }
        if let Some(v) = changes.get("initialValue") {
            signal.initial_value = v
                .as_u64()
                .ok_or("初始值须为整数")?
                .try_into()
                .map_err(|_| "初始值超出范围")?;
        }
        if let Some(issue) = validate_profile(&self.frames, &signals).first() {
            return Err(format!("{}: {}", issue.code, issue.message));
        }
        if let Some(diagnostic) = &self.diagnostic
            && let Some(issue) = validate_diagnostic(diagnostic, &self.frames, &signals).first()
        {
            return Err(format!("{}: {}", issue.code, issue.message));
        }
        let updated = signals.iter().find(|s| s.path == path).unwrap().clone();
        if self.files.iter().any(|file| self.is_managed_file(file)) {
            self.replace_managed(self.frames.clone(), signals, self.diagnostic.clone())?;
        } else {
            self.patch_imported_signal(&old, &updated)?;
        }
        Ok(self.view())
    }

    pub(crate) fn handoff_sources(&mut self) -> Result<Vec<HandoffSource>, String> {
        if self.files.iter().any(|file| file.text != file.saved) {
            return Err("请先保存全部 ARXML，再导出可重建交付包".into());
        }
        self.ensure_sources_current()?;
        self.checked_profile()?;
        let mut sources = Vec::with_capacity(self.files.len());
        for file in &self.files {
            let original_name = file
                .original_name
                .as_deref()
                .or_else(|| file.path.file_name().and_then(|name| name.to_str()))
                .ok_or("ARXML 来源文件名不是 UTF-8")?
                .to_owned();
            let doc = Document::parse(&file.saved).map_err(|e| e.to_string())?;
            let package_roots = doc
                .descendants()
                .filter(|node| {
                    node.is_element()
                        && node.tag_name().name() == "AR-PACKAGE"
                        && node.parent_element().is_some_and(|parent| {
                            parent.tag_name().name() == "AR-PACKAGES"
                                && parent.parent_element().is_some_and(|grandparent| {
                                    grandparent.tag_name().name() == "AUTOSAR"
                                })
                        })
                })
                .filter_map(|node| child_text(node, "SHORT-NAME"))
                .collect();
            sources.push(HandoffSource {
                original_name,
                package_roots,
                contents: file.saved.as_bytes().to_vec(),
            });
        }
        sources.sort_by(|left, right| {
            left.contents
                .cmp(&right.contents)
                .then_with(|| left.original_name.cmp(&right.original_name))
        });
        Ok(sources)
    }

    /// Check the current standard inputs through the single Epic 4 plan path.
    /// Unsaved in-memory edits can be previewed, but external changes to their
    /// saved sources are rejected before the plan is constructed.
    pub fn integration_plan(
        &self,
        runtime: &crate::integration::RuntimeCatalog,
        mod_archive: PathBuf,
    ) -> Result<crate::integration::ValidatedIntegrationPlan, Vec<crate::integration::PlanDiagnostic>>
    {
        use crate::integration::{
            DiagnosticCategory, InputSource, PlanDependencies, PlanDiagnostic,
        };
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
        crate::integration::build_plan(
            &sources,
            &PlanDependencies {
                xsd_archive: self.schema_zip.clone(),
                mod_archive,
            },
            runtime,
        )
    }

    pub fn validate(&mut self) -> Result<WorkspaceView, String> {
        self.refresh()?;
        self.issues.extend(schema::validate_files(
            &self.schema_zip,
            &self
                .files
                .iter()
                .map(|f| (f.path.as_path(), f.text.as_str()))
                .collect::<Vec<_>>(),
        )?);
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
