use crate::arxml_render::render_profile;
use crate::model::{validate_diagnostic, validate_profile, DiagnosticView, Direction, DtcView, FileView, FrameView, Issue, Severity, SignalView, WorkspaceView};
use crate::schema;
use roxmltree::{Document, Node};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

const NS: &str = "http://autosar.org/schema/r4.0";
const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

#[derive(Clone)]
struct SourceFile {
    path: PathBuf,
    text: String,
    saved: String,
}

struct PduInfo {
    length: Result<u32, Issue>,
    mappings: Vec<SignalMapping>,
}

struct SignalMapping {
    signal_ref: Option<String>,
    start: Option<String>,
    byte_order: Option<String>,
}

struct CanFrameInfo {
    length: Option<String>,
    mappings: Vec<FrameMapping>,
}

struct FrameMapping {
    pdu_ref: Option<String>,
    start: Option<String>,
    byte_order: Option<String>,
}

pub struct Workspace {
    name: String,
    files: Vec<SourceFile>,
    frames: Vec<FrameView>,
    signals: Vec<SignalView>,
    diagnostic: Option<DiagnosticView>,
    issues: Vec<Issue>,
    schema_zip: PathBuf,
}


fn child_text(node: Node<'_, '_>, name: &str) -> Option<String> {
    node.children().find(|n| n.is_element() && n.tag_name().namespace() == Some(NS) && n.tag_name().name() == name)?.text().map(str::to_owned)
}

fn definition(node: Node<'_, '_>) -> Option<String> {
    child_text(node, "DEFINITION-REF")
}


fn path_of(node: Node<'_, '_>) -> String {
    let mut names: Vec<String> = node.ancestors().filter_map(|n| child_text(n, "SHORT-NAME")).collect();
    names.reverse();
    format!("/{}", names.join("/"))
}

fn param(node: Node<'_, '_>, name: &str) -> Option<String> {
    node.descendants().filter(|n| n.is_element() && matches!(n.tag_name().name(), "ECUC-NUMERICAL-PARAM-VALUE" | "ECUC-TEXTUAL-PARAM-VALUE"))
        .find(|n| definition(*n).is_some_and(|p| p.ends_with(&format!("/{name}"))))
        .and_then(|n| child_text(n, "VALUE"))
}

fn ref_value(node: Node<'_, '_>, suffix: &str) -> Option<String> {
    node.descendants().filter(|n| n.is_element() && n.tag_name().name() == "ECUC-REFERENCE-VALUE")
        .find(|n| definition(*n).is_some_and(|p| p.ends_with(&format!("/{suffix}"))))
        .and_then(|n| child_text(n, "VALUE-REF"))
}

fn parse_u32(value: Option<String>, field: &str, path: &str) -> Result<u32, Issue> {
    value.ok_or_else(|| Issue::error("MISSING_PARAMETER", format!("缺少 {field}"), Some(path.into())))?
        .parse::<u32>().map_err(|_| Issue::error("INVALID_PARAMETER", format!("{field} 须为无符号整数"), Some(path.into())))
}

fn parse_milliseconds(value: Option<String>, field: &str, path: &str) -> Result<u32, Issue> {
    let text = value.ok_or_else(|| Issue::error("MISSING_PARAMETER", format!("缺少 {field}"), Some(path.into())))?;
    let (seconds, fraction) = text.split_once('.').unwrap_or((&text, ""));
    if fraction.len() > 3 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Issue::error("TIME_PRECISION", format!("{field} 仅支持整毫秒"), Some(path.into())));
    }
    let seconds = seconds.parse::<u32>().map_err(|_| Issue::error("INVALID_PARAMETER", field, Some(path.into())))?;
    let fraction = if fraction.is_empty() { 0 } else { fraction.parse::<u32>().map_err(|_| Issue::error("INVALID_PARAMETER", field, Some(path.into())))? * 10u32.pow((3 - fraction.len()) as u32) };
    seconds.checked_mul(1000).and_then(|s| s.checked_add(fraction)).ok_or_else(|| Issue::error("TIME_RANGE", field, Some(path.into())))
}

fn valid_name(name: &str) -> bool {
    name.len() <= 128 && name.bytes().next().is_some_and(|c| c.is_ascii_alphabetic()) && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

fn load_sources(files: Vec<PathBuf>, schema_zip: PathBuf) -> Result<Workspace, String> {
    if files.is_empty() {
        return Err("请选择至少一份 .arxml 文件".into());
    }
    let mut sources = Vec::new();
    let mut unique = BTreeSet::new();
    for path in files {
        let path = fs::canonicalize(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("arxml")) || !unique.insert(path.clone()) {
            return Err(format!("文件不是唯一的 .arxml: {}", path.display()));
        }
        if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 50 * 1024 * 1024 {
            return Err(format!("单份 ARXML 不得超过 50 MiB: {}", path.display()));
        }
        let text = fs::read_to_string(&path).map_err(|e| format!("{} 必须是 UTF-8 ARXML: {e}", path.display()))?;
        let doc = Document::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if doc.root_element().tag_name().namespace() != Some(NS) || doc.root_element().tag_name().name() != "AUTOSAR" {
            return Err(format!("{} 不是 R24-11 AUTOSAR 文档", path.display()));
        }
        if !doc.root_element().attribute((XSI, "schemaLocation")).is_some_and(|s| s.contains("AUTOSAR_00053.xsd")) {
            return Err(format!("{} 未声明 AUTOSAR_00053.xsd；不猜测 ARXML 发布版本", path.display()));
        }
        sources.push(SourceFile { path, saved: text.clone(), text });
    }
    sources.sort_by(|a, b| a.path.cmp(&b.path));
    let schema_issues = schema::validate_files(&schema_zip, &sources.iter().map(|s| (s.path.as_path(), s.text.as_str())).collect::<Vec<_>>())?;
    if let Some(first) = schema_issues.first() {
        return Err(format!("导入前 XSD 校验失败 {}: {}", first.file.as_deref().unwrap_or(""), first.message));
    }
    let name = sources.iter().filter_map(|s| Document::parse(&s.text).ok()).flat_map(|d| d.root_element().descendants().filter(|n| n.is_element() && n.tag_name().name() == "AR-PACKAGE").filter_map(|n| child_text(n, "SHORT-NAME")).collect::<Vec<_>>()).next().unwrap_or_else(|| "ImportedEcu".into());
    let mut workspace = Workspace { name, files: sources, frames: Vec::new(), signals: Vec::new(), diagnostic: None, issues: Vec::new(), schema_zip };
    workspace.refresh()?;
    Ok(workspace)
}

struct Patch {
    range: std::ops::Range<usize>,
    value: String,
}

fn patch_child(node: Node<'_, '_>, child_name: &str, value: String, patches: &mut Vec<Patch>) -> Result<(), String> {
    let leaf = node.children().find(|n| n.is_element() && n.tag_name().name() == child_name)
        .ok_or_else(|| format!("{} 缺少 {child_name}，拒绝不安全编辑", path_of(node)))?;
    let mut content = leaf.children();
    let text = content.next().filter(|n| n.is_text()).ok_or("值不是简单文本；拒绝不安全编辑")?;
    if content.next().is_some() { return Err("值有混合内容；拒绝不安全编辑".into()); }
    patches.push(Patch { range: text.range(), value });
    Ok(())
}

fn patch_param(node: Node<'_, '_>, name: &str, value: String, patches: &mut Vec<Patch>) -> Result<(), String> {
    let mut found = node.descendants().filter(|n| n.is_element() && matches!(n.tag_name().name(), "ECUC-NUMERICAL-PARAM-VALUE" | "ECUC-TEXTUAL-PARAM-VALUE"))
        .filter(|n| definition(*n).is_some_and(|d| d.ends_with(&format!("/{name}"))));
    let parameter = found.next().ok_or_else(|| format!("缺少 {name}，拒绝不安全编辑"))?;
    if found.next().is_some() { return Err(format!("{name} 存在多个变体，拒绝不安全编辑")); }
    patch_child(parameter, "VALUE", value, patches)
}

fn seconds(milliseconds: u32) -> String {
    format!("{}.{:03}", milliseconds / 1000, milliseconds % 1000)
}

fn signal_type(length: u8) -> &'static str {
    if length == 1 { "BOOLEAN" } else if length <= 8 { "UINT8" } else if length <= 16 { "UINT16" } else { "UINT32" }
}

fn apply_patches(text: &mut String, patches: &mut Vec<Patch>) -> Result<(), String> {
    patches.sort_by(|a,b| b.range.start.cmp(&a.range.start));
    let mut next_start = text.len();
    for patch in patches {
        if patch.range.end > next_start { return Err("编辑范围重叠；保留原 ARXML".into()); }
        next_start = patch.range.start;
        text.replace_range(patch.range.clone(), &patch.value);
    }
    Ok(())
}

fn restore_backup(original: &Path, backup: &Path, installed: Option<&str>) -> Result<(), String> {
    if original.exists() {
        let owned = installed.is_some_and(|text| fs::read_to_string(original).is_ok_and(|current| current == text));
        if !owned {
            return Err(format!("外部文件 {} 未覆盖；原备份保留在 {}", original.display(), backup.display()));
        }
        fs::remove_file(original).map_err(|e| format!("{}: {e}", original.display()))?;
    }
    fs::rename(backup, original).map_err(|e| format!("{} -> {}: {e}", backup.display(), original.display()))
}

fn install_staged(file: &SourceFile, stage: &Path, backup: &Path) -> Result<(), String> {
    fs::rename(&file.path, backup).map_err(|e| format!("{} -> {}: {e}", file.path.display(), backup.display()))?;
    let result = fs::read_to_string(backup).map_err(|e| format!("无法复核原文件 {}: {e}", backup.display()))
        .and_then(|actual| {
            if actual != file.saved {
                return Err(format!("文件已被外部修改，拒绝覆盖: {}", file.path.display()));
            }
            fs::rename(stage, &file.path).map_err(|e| format!("{} -> {}: {e}", stage.display(), file.path.display()))
        });
    if let Err(error) = result {
        return Err(match restore_backup(&file.path, backup, None) {
            Ok(()) => error,
            Err(rollback) => format!("{error}; 回滚问题: {rollback}"),
        });
    }
    Ok(())
}

fn structural_node(node: Node<'_, '_>) -> String {
    fn append(node: Node<'_, '_>, out: &mut String) {
        if node.is_comment() {
            out.push_str("<!--");
            out.push_str(node.text().unwrap_or(""));
            out.push_str("-->");
        } else if node.is_element() {
            out.push('<');
            out.push_str(node.tag_name().name());
            let mut attributes: Vec<_> = node.attributes().map(|a| (a.namespace().unwrap_or(""), a.name(), a.value())).collect();
            attributes.sort_unstable();
            for attribute in attributes { write!(out, " {:?}", attribute).unwrap(); }
            out.push('>');
            for child in node.children() { append(child, out); }
            out.push_str("</");
            out.push_str(node.tag_name().name());
            out.push('>');
        } else if node.is_text() {
            let text = node.text().unwrap_or("").trim();
            if !text.is_empty() { out.push_str(text); }
        }
    }
    let mut out = String::new();
    append(node, &mut out);
    out
}

type DiagnosticShape = BTreeMap<String, (String, String, Vec<(String, String, String, String)>, String)>;

fn diagnostic_shape(docs: &[Document<'_>]) -> Result<DiagnosticShape, String> {
    let mut shape = BTreeMap::new();
    for doc in docs {
        for module in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
            && matches!(definition(*n).as_deref(), Some("/AUTOSAR/EcucDefs/Dcm" | "/AUTOSAR/EcucDefs/CanTp" | "/AUTOSAR/EcucDefs/Dem" | "/AUTOSAR/EcucDefs/NvM"))) {
            for node in module.descendants().filter(|n| n.is_element()
                && matches!(n.tag_name().name(), "ECUC-MODULE-CONFIGURATION-VALUES" | "ECUC-CONTAINER-VALUE")) {
                let mut values = Vec::new();
                for group in node.children().filter(|n| n.is_element() && matches!(n.tag_name().name(), "PARAMETER-VALUES" | "REFERENCE-VALUES")) {
                    for item in group.children().filter(|n| n.is_element()) {
                        let value = child_text(item, if group.tag_name().name() == "REFERENCE-VALUES" { "VALUE-REF" } else { "VALUE" })
                            .ok_or_else(|| format!("{} 缺少 ECUC 值", path_of(node)))?;
                        let definition = definition(item).ok_or_else(|| format!("{} 缺少 ECUC 参数定义", path_of(node)))?;
                        let definition_dest = item.children().find(|n| n.is_element() && n.tag_name().name() == "DEFINITION-REF")
                            .and_then(|n| n.attribute("DEST")).unwrap_or("").to_owned();
                        let value_dest = item.children().find(|n| n.is_element() && n.tag_name().name() == "VALUE-REF")
                            .and_then(|n| n.attribute("DEST")).unwrap_or("").to_owned();
                        values.push((definition, definition_dest, value, value_dest));
                    }
                }
                values.sort();
                let path = path_of(node);
                let definition_dest = node.children().find(|n| n.is_element() && n.tag_name().name() == "DEFINITION-REF")
                    .and_then(|n| n.attribute("DEST")).unwrap_or("").to_owned();
                let strict = matches!(definition(module).as_deref(), Some("/AUTOSAR/EcucDefs/Dem" | "/AUTOSAR/EcucDefs/NvM"));
                if shape.insert(path.clone(), (definition(node).unwrap_or_default(), definition_dest, values,
                    if strict { structural_node(node) } else { String::new() })).is_some() {
                    return Err(format!("重复的诊断 ECUC 容器 {path}"));
                }
            }
        }
    }
    Ok(shape)
}

fn parse_diagnostic(files: &[SourceFile], project: &str, frames: &[FrameView], signals: &[SignalView]) -> Result<Option<DiagnosticView>, String> {
    let docs: Vec<_> = files.iter().map(|file| Document::parse(&file.text).map_err(|e| e.to_string())).collect::<Result<_, _>>()?;
    let actual = diagnostic_shape(&docs)?;
    let nodes: Vec<_> = docs.iter().flat_map(|doc| doc.descendants().filter(|n| n.is_element())).collect();
    if actual.is_empty() {
        let request = format!("/{project}/NPdu_DiagRequest");
        let response = format!("/{project}/NPdu_DiagResponse");
        if nodes.iter().any(|n| n.tag_name().name() == "N-PDU" &&
            (path_of(*n) == request || path_of(*n) == response)) {
            return Err("诊断 N-PDU 存在，但缺少对应的 CanTp/Dcm 配置".into());
        }
        return Ok(None);
    }
    let unique = |def: &str| -> Result<Node<'_, '_>, String> {
        let mut found = nodes.iter().copied().filter(|n| n.tag_name().name() == "ECUC-CONTAINER-VALUE" && definition(*n).as_deref() == Some(def));
        let node = found.next().ok_or_else(|| format!("诊断配置缺少 {def}"))?;
        if found.next().is_some() { return Err(format!("诊断配置含多个 {def}")); }
        Ok(node)
    };
    let general = unique("/AUTOSAR/EcucDefs/Dcm/DcmGeneral")?;
    let rx_sdu = unique("/AUTOSAR/EcucDefs/CanTp/CanTpConfig/CanTpChannel/CanTpRxNSdu")?;
    let tx_sdu = unique("/AUTOSAR/EcucDefs/CanTp/CanTpConfig/CanTpChannel/CanTpTxNSdu")?;
    let did_node = unique("/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsp/DcmDspDid")?;
    let s3_ms = parse_milliseconds(param(general, "DcmS3ServerTimeoutOverwrite"), "DcmS3ServerTimeoutOverwrite", &path_of(general)).map_err(|e| e.message)?;
    let n_bs_ms = parse_milliseconds(param(tx_sdu, "CanTpNbs"), "CanTpNbs", &path_of(tx_sdu)).map_err(|e| e.message)?;
    let n_cr_ms = parse_milliseconds(param(rx_sdu, "CanTpNcr"), "CanTpNcr", &path_of(rx_sdu)).map_err(|e| e.message)?;
    let did = parse_u32(param(did_node, "DcmDspDidIdentifier"), "DcmDspDidIdentifier", &path_of(did_node)).map_err(|e| e.message)?;
    let did: u16 = did.try_into().map_err(|_| "诊断 DID 超出 16 位范围")?;
    let request = format!("/{project}/NPdu_DiagRequest");
    let response = format!("/{project}/NPdu_DiagResponse");
    let mut ids = Vec::new();
    for name in ["DcmPdu_DiagRequest", "DcmPdu_DiagResponse"] {
        let path = format!("/{project}/{name}");
        let pdus: Vec<_> = nodes.iter().copied().filter(|n| n.tag_name().name() == "DCM-I-PDU" && path_of(*n) == path).collect();
        if pdus.len() != 1 || child_text(pdus[0], "LENGTH").as_deref() != Some("256") {
            return Err(format!("{path} 必须是唯一的最大 256 字节 DCM-I-PDU"));
        }
    }
    for (pdu, tx) in [(&request, false), (&response, true)] {
        let pdus: Vec<_> = nodes.iter().copied().filter(|n| n.tag_name().name() == "N-PDU" && path_of(*n) == *pdu).collect();
        if pdus.len() != 1 || child_text(pdus[0], "LENGTH").as_deref() != Some("8") ||
            child_text(pdus[0], "HAS-DYNAMIC-LENGTH").as_deref() != Some("true") {
            return Err(format!("{pdu} 必须是唯一的动态长度 8 字节 N-PDU"));
        }
        let def = if tx { "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfTxPduCfg" } else { "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfRxPduCfg" };
        let ref_name = if tx { "CanIfTxPduRef" } else { "CanIfRxPduRef" };
        let canif: Vec<_> = nodes.iter().copied().filter(|n| n.tag_name().name() == "ECUC-CONTAINER-VALUE" && definition(*n).as_deref() == Some(def) && ref_value(*n, ref_name).as_deref() == Some(pdu)).collect();
        if canif.len() != 1 { return Err(format!("{pdu} 缺少唯一且方向正确的 CanIf 映射")); }
        let field = if tx { "CanIfTxPduCanId" } else { "CanIfRxPduCanId" };
        if tx && param(canif[0], "CanIfTxPduCanIdType").as_deref() != Some("STANDARD_CAN") ||
            !tx && (param(canif[0], "CanIfRxPduDataLength").as_deref() != Some("8") ||
                param(canif[0], "CanIfRxPduCanIdType").as_deref() != Some("STANDARD_NO_FD_CAN") ||
                param(canif[0], "CanIfRxPduDataLengthCheck").as_deref() != Some("false")) {
            return Err(format!("{pdu} 须配置不检查变长 DLC 的经典 11 位 CAN"));
        }
        let expected_params: &[&str] = if tx {
            &["CanIfTxPduCanId", "CanIfTxPduId", "CanIfTxPduCanIdType"]
        } else {
            &["CanIfRxPduCanId", "CanIfRxPduId", "CanIfRxPduDataLength", "CanIfRxPduCanIdType", "CanIfRxPduDataLengthCheck"]
        };
        let parameter_values: Vec<_> = canif[0].children().filter(|n| n.is_element() && n.tag_name().name() == "PARAMETER-VALUES")
            .flat_map(|group| group.children().filter(|n| n.is_element())).collect();
        let reference_values: Vec<_> = canif[0].children().filter(|n| n.is_element() && n.tag_name().name() == "REFERENCE-VALUES")
            .flat_map(|group| group.children().filter(|n| n.is_element())).collect();
        if parameter_values.len() != expected_params.len() ||
            expected_params.iter().any(|name| parameter_values.iter().filter(|n| definition(**n).as_deref() == Some(format!("{def}/{name}").as_str())).count() != 1) ||
            reference_values.len() != 1 || definition(reference_values[0]).as_deref() != Some(format!("{def}/{ref_name}").as_str()) ||
            reference_values[0].children().find(|n| n.is_element() && n.tag_name().name() == "VALUE-REF")
                .and_then(|n| n.attribute("DEST")) != Some("N-PDU") ||
            param(canif[0], if tx { "CanIfTxPduId" } else { "CanIfRxPduId" }).as_deref() != Some(frames.len().to_string().as_str()) {
            return Err(format!("{pdu} 的诊断 CanIf 参数或引用不属于受支持的静态配置"));
        }
        let wrong_direction = nodes.iter().any(|n| n.tag_name().name() == "ECUC-CONTAINER-VALUE"
            && definition(*n).as_deref() == Some(if tx { "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfRxPduCfg" } else { "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfTxPduCfg" })
            && ref_value(*n, if tx { "CanIfRxPduRef" } else { "CanIfTxPduRef" }).as_deref() == Some(pdu));
        if wrong_direction { return Err(format!("{pdu} 同时配置了反向 CanIf 映射")); }
        ids.push(parse_u32(param(canif[0], field), field, pdu).map_err(|e| e.message)?);
    }
    let data_def = "/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsp/DcmDspData";
    let did_signal_def = "/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsp/DcmDspDid/DcmDspDidSignal";
    let mut bindings = Vec::new();
    for item in did_node.descendants().filter(|n| n.is_element() && definition(*n).as_deref() == Some(did_signal_def)) {
        let offset = parse_u32(param(item, "DcmDspDidByteOffset"), "DcmDspDidByteOffset", &path_of(item)).map_err(|e| e.message)?;
        let data_ref = ref_value(item, "DcmDspDidDataRef").ok_or("诊断 DID 信号缺少数据引用")?;
        let data: Vec<_> = nodes.iter().copied().filter(|n| n.tag_name().name() == "ECUC-CONTAINER-VALUE" && definition(*n).as_deref() == Some(data_def) && path_of(*n) == data_ref).collect();
        if data.len() != 1 { return Err(format!("{data_ref} 诊断数据引用未唯一解析")); }
        let metadata: Vec<_> = data[0].descendants().filter(|n| n.is_element() && n.tag_name().name() == "SDG"
            && n.attribute("GID") == Some("AutosarWorkbenchDiagnostic")).collect();
        if metadata.len() != 1 { return Err(format!("{data_ref} 必须有唯一的工具域绑定")); }
        let refs: Vec<_> = metadata[0].children().filter(|n| n.is_element()).collect();
        if refs.len() != 1 || refs[0].tag_name().name() != "SD" ||
            refs[0].attribute("GID") != Some("ComSignalRef") || refs[0].text().is_none_or(str::is_empty) {
            return Err(format!("{data_ref} 缺少唯一的工具域 ComSignalRef 绑定"));
        }
        bindings.push((offset, refs[0].text().unwrap().to_owned()));
    }
    bindings.sort_by_key(|(offset, _)| *offset);
    if bindings.is_empty() || bindings.iter().enumerate().any(|(i, (offset, _))| *offset != i as u32 * 4) {
        return Err("诊断 DID 数据字节偏移必须是从零开始的连续 32 位信号".into());
    }
    let dem_dtc_def = "/AUTOSAR/EcucDefs/Dem/DemConfigSet/DemDTC";
    let dem_event_def = "/AUTOSAR/EcucDefs/Dem/DemConfigSet/DemEventParameter";
    let dtc_nodes: Vec<_> = nodes.iter().copied().filter(|n| n.tag_name().name() == "ECUC-CONTAINER-VALUE"
        && definition(*n).as_deref() == Some(dem_dtc_def)).collect();
    let dtc = if dtc_nodes.is_empty() {
        None
    } else {
        if dtc_nodes.len() != 1 { return Err("仅支持一个 Dem UDS DTC".into()); }
        let node = dtc_nodes[0];
        let code = parse_u32(param(node, "DemDtcValue"), "DemDtcValue", &path_of(node)).map_err(|e| e.message)?;
        let event_nodes: Vec<_> = nodes.iter().copied().filter(|n| n.tag_name().name() == "ECUC-CONTAINER-VALUE"
            && definition(*n).as_deref() == Some(dem_event_def)).collect();
        if event_nodes.len() != 1 || ref_value(event_nodes[0], "DemDTCRef").as_deref() != Some(path_of(node).as_str()) {
            return Err("Dem 事件须唯一地关联 UDS DTC".into());
        }
        let event = event_nodes[0];
        let metadata: Vec<_> = event.descendants().filter(|n| n.is_element() && n.tag_name().name() == "SDG"
            && n.attribute("GID") == Some("AutosarWorkbenchDtc")).collect();
        if metadata.len() != 1 { return Err("Dem 事件缺少唯一的工具域 Rx 帧绑定".into()); }
        let refs: Vec<_> = metadata[0].children().filter(|n| n.is_element()).collect();
        if refs.len() != 1 || refs[0].tag_name().name() != "SD"
            || refs[0].attribute("GID") != Some("MonitorFrameRef") {
            return Err("Dem 事件须有唯一 MonitorFrameRef 工具域绑定".into());
        }
        let monitor_frame_path = refs[0].text().filter(|s| !s.is_empty())
            .ok_or("Dem 事件缺少监控 Rx 帧绝对路径")?.to_owned();
        Some(DtcView { path: path_of(node), code, monitor_frame_path })
    };
    if dtc.is_none() && nodes.iter().any(|n| n.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
        && matches!(definition(*n).as_deref(), Some("/AUTOSAR/EcucDefs/Dem" | "/AUTOSAR/EcucDefs/NvM"))) {
        return Err("Dem/NvM 配置存在但缺少受支持的单个 UDS DTC".into());
    }
    let diagnostic = DiagnosticView {
        path: path_of(did_node), request_id: ids[0], response_id: ids[1], s3_ms, n_bs_ms, n_cr_ms, did,
        signal_paths: bindings.into_iter().map(|(_, path)| path).collect(), dtc,
    };
    if let Some(issue) = validate_diagnostic(&diagnostic, frames, signals).first() {
        return Err(format!("{}: {}", issue.code, issue.message));
    }
    let expected = render_profile(project, frames, signals, Some(&diagnostic));
    let expected_doc = Document::parse(&expected).map_err(|e| e.to_string())?;
    if actual != diagnostic_shape(&[expected_doc])? {
        return Err("CanTp/Dcm/Dem/NvM 配置含未知、不一致或不受支持的参数、引用、方向、会话或变体".into());
    }
    Ok(Some(diagnostic))
}

impl Workspace {
    pub fn create(directory: &Path, name: &str, schema_zip: PathBuf) -> Result<Self, String> {
        if !valid_name(name) {
            return Err("工程名须以 ASCII 字母开头，且仅含字母、数字和下划线（最多 128 字节）".into());
        }
        fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let path = directory.join(format!("{name}.arxml"));
        let text = render_profile(name, &[], &[], None);
        let issues = schema::validate_files(&schema_zip, &[(path.as_path(), text.as_str())])?;
        if let Some(first) = issues.first() {
            return Err(format!("空项目 XSD 校验失败: {}", first.message));
        }
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| format!("不会覆盖已有 ARXML {}: {e}", path.display()))?;
        std::io::Write::write_all(&mut file, text.as_bytes()).map_err(|e| e.to_string())?;
        load_sources(vec![path], schema_zip)
    }

    pub fn open(paths: Vec<PathBuf>, schema_zip: PathBuf) -> Result<Self, String> {
        load_sources(paths, schema_zip)
    }

    pub fn view(&self) -> WorkspaceView {
        WorkspaceView {
            name: self.name.clone(),
            files: self.files.iter().map(|file| {
                let managed = self.is_managed_file(file);
                let doc = Document::parse(&file.text).ok();
                let supported = doc.as_ref().is_some_and(|d| d.descendants().any(|n| n.is_element() &&
                    ((n.tag_name().name() == "I-SIGNAL-I-PDU" && self.frames.iter().any(|f| f.path == path_of(n))) ||
                    (n.tag_name().name() == "ECUC-CONTAINER-VALUE" && self.signals.iter().any(|s| s.path == path_of(n))))));
                let retained_count = if managed { 0 } else { doc.as_ref().map(|d| d.descendants().filter(|n| n.is_element() &&
                    n.parent_element().is_some_and(|p| p.tag_name().name() == "ELEMENTS") &&
                    !matches!(n.tag_name().name(), "I-SIGNAL-I-PDU" | "ECUC-MODULE-CONFIGURATION-VALUES") &&
                    !(n.tag_name().name() == "I-SIGNAL" && self.signals.iter().any(|s| path_of(*n) == format!("/{}/ISignal_{}", self.name, s.name)))).count()).unwrap_or(0) };
                FileView { path: file.path.display().to_string(), readonly: !managed && !supported, retained_count }
            }).collect(),
            frames: self.frames.clone(), signals: self.signals.clone(), diagnostic: self.diagnostic.clone(), issues: self.issues.clone(),
            dirty: self.files.iter().any(|file| file.saved != file.text),
        }
    }

    fn is_managed_file(&self, file: &SourceFile) -> bool {
        file.text == render_profile(&self.name, &self.frames, &self.signals, self.diagnostic.as_ref())
    }

    fn refresh(&mut self) -> Result<(), String> {
        let mut issues = Vec::new();
        let mut pdus = BTreeMap::new();
        let mut signal_lengths = BTreeMap::new();
        let mut com = Vec::new();
        let mut canif = BTreeMap::new();
        let mut network_frames = BTreeMap::new();
        let mut triggers = Vec::new();
        let mut signal_nodes = BTreeMap::new();
        let mut paths = BTreeMap::new();
        for file in &self.files {
            let doc = Document::parse(&file.text).map_err(|e| format!("{}: {e}", file.path.display()))?;
            for node in doc.descendants().filter(|n| n.is_element() && n.tag_name().namespace() == Some(NS)) {
                if child_text(node, "SHORT-NAME").is_some() {
                    let path = path_of(node);
                    if let Some(previous) = paths.insert(path.clone(), node.tag_name().name().to_owned()) {
                        if previous != "AR-PACKAGE" || node.tag_name().name() != "AR-PACKAGE" {
                            issues.push(Issue::error("DUPLICATE_PATH", "ARXML 绝对路径重复", Some(path)));
                        }
                    }
                }
                if node.tag_name().name() == "I-SIGNAL-I-PDU" {
                    let mappings = node.descendants().filter(|n| n.is_element() && n.tag_name().name() == "I-SIGNAL-TO-I-PDU-MAPPING")
                        .map(|mapping| SignalMapping {
                            signal_ref: child_text(mapping, "I-SIGNAL-REF"),
                            start: child_text(mapping, "START-POSITION"),
                            byte_order: child_text(mapping, "PACKING-BYTE-ORDER"),
                        }).collect();
                    pdus.insert(path_of(node), PduInfo { length: parse_u32(child_text(node, "LENGTH"), "LENGTH", &path_of(node)), mappings });
                }
                if node.tag_name().name() == "I-SIGNAL" {
                    signal_lengths.insert(path_of(node), parse_u32(child_text(node, "LENGTH"), "LENGTH", &path_of(node)));
                }
                if node.tag_name().name() == "CAN-FRAME" {
                    let mappings = node.descendants().filter(|n| n.is_element() && n.tag_name().name() == "PDU-TO-FRAME-MAPPING")
                        .map(|mapping| FrameMapping {
                            pdu_ref: child_text(mapping, "PDU-REF"),
                            start: child_text(mapping, "START-POSITION"),
                            byte_order: child_text(mapping, "PACKING-BYTE-ORDER"),
                        }).collect();
                    network_frames.insert(path_of(node), CanFrameInfo { length: child_text(node, "FRAME-LENGTH"), mappings });
                }
                if node.tag_name().name() == "CAN-FRAME-TRIGGERING" {
                    triggers.push((child_text(node, "FRAME-REF"), child_text(node, "IDENTIFIER"), path_of(node)));
                }
                if node.tag_name().name() == "ECUC-CONTAINER-VALUE" {
                    match definition(node).as_deref() {
                        Some("/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu") => {
                            let path = path_of(node);
                            com.push((path, ref_value(node, "ComPduIdRef"), param(node, "ComIPduDirection"), param(node, "ComTxModeTimePeriod"), node.descendants().filter(|n| n.is_element() && definition(*n).is_some_and(|d| d.ends_with("/ComIPduSignalRef"))).filter_map(|n| child_text(n, "VALUE-REF")).collect::<Vec<_>>()));
                        }
                        Some("/AUTOSAR/EcucDefs/Com/ComConfig/ComSignal") => {
                            signal_nodes.insert(path_of(node), (param(node, "ComBitPosition"), param(node, "ComBitSize"), param(node, "ComSignalInitValue"), param(node, "ComSignalEndianness"), param(node, "ComSignalType"), param(node, "ComTimeout")));
                        }
                        Some("/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfTxPduCfg") => {
                            if let Some(pdu) = ref_value(node, "CanIfTxPduRef") {
                                if canif.insert(pdu.clone(), (param(node, "CanIfTxPduCanId"), None, param(node, "CanIfTxPduCanIdType"), true)).is_some() {
                                    issues.push(Issue::error("CANIF_PDU_DUPLICATE", "同一 I-PDU 有多个 CanIf 映射", Some(pdu)));
                                }
                            }
                        }
                        Some("/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfRxPduCfg") => {
                            if let Some(pdu) = ref_value(node, "CanIfRxPduRef") {
                                if canif.insert(pdu.clone(), (param(node, "CanIfRxPduCanId"), param(node, "CanIfRxPduDataLength"), param(node, "CanIfRxPduCanIdType"), false)).is_some() {
                                    issues.push(Issue::error("CANIF_PDU_DUPLICATE", "同一 I-PDU 有多个 CanIf 映射", Some(pdu)));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        let mut frames = Vec::new();
        let mut signals = Vec::new();
        for (path, pdu_ref, direction, period, signal_refs) in com {
            let Some(pdu_ref) = pdu_ref else { issues.push(Issue::error("COM_PDU_REF", "ComIPdu 未关联 I-SIGNAL-I-PDU", Some(path))); continue; };
            let Some(pdu) = pdus.get(&pdu_ref) else { issues.push(Issue::error("COM_PDU_REF", "ComPduIdRef 未解析", Some(pdu_ref))); continue; };
            let Some((can_id, can_dlc, id_type, is_tx)) = canif.get(&pdu_ref) else { issues.push(Issue::error("CANIF_PDU_REF", "CanIf 没有引用该 I-PDU", Some(pdu_ref))); continue; };
            let direction = match direction.as_deref() { Some("SEND") if *is_tx => Direction::Tx, Some("RECEIVE") if !*is_tx => Direction::Rx, _ => { issues.push(Issue::error("PDU_DIRECTION", "Com 与 CanIf 方向不一致或未知", Some(pdu_ref))); continue; } };
            if id_type.as_deref().is_some_and(|kind| kind != "STANDARD_CAN") {
                issues.push(Issue::error("CAN_ID_TYPE", "仅支持 STANDARD_CAN", Some(pdu_ref.clone())));
            }
            let Ok(dlc) = pdu.length.as_ref() else { issues.push(pdu.length.as_ref().unwrap_err().clone()); continue; };
            let id = match parse_u32(can_id.clone(), "CanIfPduCanId", &pdu_ref) { Ok(id) => id, Err(e) => { issues.push(e); continue; } };
            let period_ms = if matches!(direction, Direction::Tx) { match parse_milliseconds(period, "ComTxModeTimePeriod", &pdu_ref) { Ok(value) => Some(value), Err(e) => { issues.push(e); None } } } else { None };
            let mut timeout_ms = None;
            let mut com_layout = Vec::new();
            for signal_ref in signal_refs {
                let Some((start, length, initial, endian, ty, timeout)) = signal_nodes.get(&signal_ref) else { issues.push(Issue::error("COM_SIGNAL_REF", "ComIPduSignalRef 未解析", Some(signal_ref))); continue; };
                if endian.as_deref() != Some("LITTLE_ENDIAN") || !ty.as_deref().is_some_and(|t| matches!(t, "BOOLEAN" | "UINT8" | "UINT16" | "UINT32")) {
                    issues.push(Issue::error("SIGNAL_VARIANT", "仅支持小端无符号标量信号", Some(signal_ref.clone())));
                }
                let start = match parse_u32(start.clone(), "ComBitPosition", &signal_ref) { Ok(v) => v, Err(e) => { issues.push(e); continue; } };
                let length = match parse_u32(length.clone(), "ComBitSize", &signal_ref) { Ok(v) => v, Err(e) => { issues.push(e); continue; } };
                let initial = match parse_u32(initial.clone(), "ComSignalInitValue", &signal_ref) { Ok(v) => v, Err(e) => { issues.push(e); continue; } };
                if start > u8::MAX as u32 || length > u8::MAX as u32 { issues.push(Issue::error("SIGNAL_RANGE", "信号位位置或长度超出范围", Some(signal_ref))); continue; }
                com_layout.push((start, length));
                if matches!(direction, Direction::Rx) {
                    match parse_milliseconds(timeout.clone(), "ComTimeout", &signal_ref) {
                        Ok(value) if timeout_ms.is_none() || timeout_ms == Some(value) => timeout_ms = Some(value),
                        Ok(_) => issues.push(Issue::error("RX_TIMEOUT_MISMATCH", "同帧接收信号超时须一致", Some(signal_ref.clone()))),
                        Err(e) => issues.push(e),
                    }
                }
                let name = signal_ref.rsplit('/').next().unwrap_or("").to_owned();
                signals.push(SignalView { path: signal_ref, name, frame_path: pdu_ref.clone(), start_bit: start as u8, length: length as u8, initial_value: initial });
            }
            if !pdu.mappings.is_empty() {
                let mut mapped_layout = Vec::with_capacity(pdu.mappings.len());
                let supported = pdu.mappings.iter().all(|mapping| {
                    if mapping.byte_order.as_deref() != Some("MOST-SIGNIFICANT-BYTE-LAST") { return false; }
                    let Some(start) = mapping.start.as_deref().and_then(|value| value.parse::<u32>().ok()) else { return false; };
                    let Some(length) = mapping.signal_ref.as_deref().and_then(|path| signal_lengths.get(path)).and_then(|value| value.as_ref().ok()) else { return false; };
                    mapped_layout.push((start, *length));
                    true
                });
                mapped_layout.sort_unstable();
                com_layout.sort_unstable();
                if !supported || mapped_layout != com_layout {
                    issues.push(Issue::error("PDU_MAPPING", "I-PDU 信号映射的位序或位段与 Com 配置不一致", Some(pdu_ref.clone())));
                }
            }
            for (frame_ref, network_id, trigger_path) in &triggers {
                let Some(frame) = frame_ref.as_deref().and_then(|path| network_frames.get(path)) else { continue; };
                if let Some(mapping) = frame.mappings.iter().find(|mapping| mapping.pdu_ref.as_deref() == Some(pdu_ref.as_str())) {
                    if frame.mappings.len() != 1 || frame.length.as_deref().and_then(|value| value.parse::<u32>().ok()) != Some(*dlc)
                        || mapping.start.as_deref() != Some("0") || mapping.byte_order.as_deref() != Some("MOST-SIGNIFICANT-BYTE-LAST") {
                        issues.push(Issue::error("CAN_FRAME_MAPPING", "网络帧须以小端、零偏移完整承载该 I-PDU", Some(trigger_path.clone())));
                    }
                    if network_id.as_deref().and_then(|value| value.parse::<u32>().ok()) != Some(id) {
                        issues.push(Issue::error("CAN_ID_MISMATCH", "CAN-FRAME-TRIGGERING 与 CanIf 的 CAN 标识符不一致", Some(trigger_path.clone())));
                    }
                }
            }
            if *dlc > u8::MAX as u32 || can_dlc.as_ref().is_some_and(|v| v.parse::<u32>().ok() != Some(*dlc)) {
                issues.push(Issue::error("PDU_DLC", "I-PDU 与 CanIf 的数据长度不一致", Some(pdu_ref.clone())));
            }
            frames.push(FrameView { name: pdu_ref.rsplit('/').next().unwrap_or("").trim_start_matches("Pdu_").into(), path: pdu_ref, id, dlc: (*dlc).min(u8::MAX as u32) as u8, direction, period_ms, timeout_ms });
        }
        frames.sort_by(|a,b| a.path.cmp(&b.path));
        signals.sort_by(|a,b| a.path.cmp(&b.path));
        issues.extend(validate_profile(&frames, &signals));
        let diagnostic = match parse_diagnostic(&self.files, &self.name, &frames, &signals) {
            Ok(diagnostic) => diagnostic,
            Err(message) => {
                issues.push(Issue::error("DIAG_UNSUPPORTED", message, None));
                None
            }
        };
        self.frames = frames;
        self.signals = signals;
        self.diagnostic = diagnostic;
        self.issues = issues;
        Ok(())
    }

    fn replace_managed(&mut self, frames: Vec<FrameView>, signals: Vec<SignalView>, diagnostic: Option<DiagnosticView>) -> Result<(), String> {
        let index = self.files.iter().position(|f| self.is_managed_file(f)).ok_or("当前项目没有可安全重建的配置文件；已保留导入内容，不执行可能破坏未知项的新增操作")?;
        let mut frames = frames;
        let mut signals = signals;
        frames.sort_by(|a,b| a.path.cmp(&b.path));
        signals.sort_by(|a,b| a.path.cmp(&b.path));
        let mut issues = validate_profile(&frames, &signals);
        if let Some(diagnostic) = &diagnostic { issues.extend(validate_diagnostic(diagnostic, &frames, &signals)); }
        if let Some(first) = issues.first() { return Err(format!("{}: {}", first.code, first.message)); }
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
            if matches!(frame.direction, Direction::Rx) && !signals.iter().any(|signal| signal.frame_path == frame.path) {
                frame.timeout_ms = frames.iter().find(|source| source.path == frame.path).and_then(|source| source.timeout_ms);
            }
        }
        self.issues.retain(|issue| !(issue.code == "RX_TIMEOUT" && frames.iter().any(|frame|
            matches!(frame.direction, Direction::Rx) && issue.path.as_deref() == Some(frame.path.as_str()) &&
            !signals.iter().any(|signal| signal.frame_path == frame.path))));
        if let Some(issue) = self.issues.iter().find(|i| matches!(i.severity, Severity::Error)) {
            let error = format!("{}: {}", issue.code, issue.message);
            self.files[index].text = previous;
            self.refresh()?;
            return Err(error);
        }
        Ok(())
    }

    fn commit_patches(&mut self, mut patches: Vec<Vec<Patch>>, matches: impl FnOnce(&Workspace) -> bool) -> Result<(), String> {
        let mut previous = Vec::new();
        for (index, edits) in patches.iter_mut().enumerate() {
            if edits.is_empty() { continue; }
            previous.push((index, self.files[index].text.clone()));
            if let Err(error) = apply_patches(&mut self.files[index].text, edits) {
                for (i, text) in previous { self.files[i].text = text; }
                return Err(error);
            }
        }
        let checked = self.refresh().and_then(|_| {
            if let Some(issue) = self.issues.iter().find(|i| matches!(i.severity, Severity::Error)) {
                Err(format!("{}: {}", issue.code, issue.message))
            } else if matches(self) { Ok(()) } else { Err("编辑后的 ARXML 与配置模型不一致".into()) }
        });
        if let Err(error) = checked {
            for (i, text) in previous { self.files[i].text = text; }
            let _ = self.refresh();
            return Err(error);
        }
        Ok(())
    }

    fn patch_imported_frame(&mut self, old: &FrameView, new: &FrameView) -> Result<(), String> {
        let mut patches: Vec<Vec<Patch>> = (0..self.files.len()).map(|_| Vec::new()).collect();
        let mut found_id = false;
        let mut found_dlc = false;
        let mut found_period = false;
        let mut found_timeout = 0usize;
        let receive_signals: BTreeSet<_> = self.signals.iter().filter(|s| s.frame_path == old.path).map(|s| s.path.as_str()).collect();
        for (index, file) in self.files.iter().enumerate() {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element()) {
                if old.dlc != new.dlc && node.tag_name().name() == "I-SIGNAL-I-PDU" && path_of(node) == old.path {
                    patch_child(node, "LENGTH", new.dlc.to_string(), &mut patches[index])?;
                    found_dlc = true;
                }
                if node.tag_name().name() == "ECUC-CONTAINER-VALUE" {
                    let def = definition(node).unwrap_or_default();
                    if def.ends_with("/CanIfTxPduCfg") && ref_value(node, "CanIfTxPduRef").as_deref() == Some(&old.path) {
                        if old.id != new.id { patch_param(node, "CanIfTxPduCanId", new.id.to_string(), &mut patches[index])?; found_id = true; }
                    }
                    if def.ends_with("/CanIfRxPduCfg") && ref_value(node, "CanIfRxPduRef").as_deref() == Some(&old.path) {
                        if old.id != new.id { patch_param(node, "CanIfRxPduCanId", new.id.to_string(), &mut patches[index])?; found_id = true; }
                        if old.dlc != new.dlc { patch_param(node, "CanIfRxPduDataLength", new.dlc.to_string(), &mut patches[index])?; }
                    }
                    if old.period_ms != new.period_ms && def.ends_with("/ComIPdu") && ref_value(node, "ComPduIdRef").as_deref() == Some(&old.path) {
                        patch_param(node, "ComTxModeTimePeriod", seconds(new.period_ms.ok_or("发送周期不可为空")?), &mut patches[index])?;
                        found_period = true;
                    }
                    if old.timeout_ms != new.timeout_ms && receive_signals.contains(path_of(node).as_str()) {
                        patch_param(node, "ComTimeout", seconds(new.timeout_ms.ok_or("接收超时不可为空")?), &mut patches[index])?;
                        found_timeout += 1;
                    }
                }
                if old.id != new.id && node.tag_name().name() == "CAN-FRAME-TRIGGERING" {
                    return Err("导入项目包含 CAN 网络触发配置，修改标识符需同步网络模型；已阻止不安全编辑".into());
                }
                if old.dlc != new.dlc && node.tag_name().name() == "CAN-FRAME" {
                    return Err("导入项目包含 CAN-FRAME，修改 DLC 需同步网络模型；已阻止不安全编辑".into());
                }
            }
        }
        if (old.id != new.id && !found_id) || (old.dlc != new.dlc && !found_dlc) || (old.period_ms != new.period_ms && !found_period) ||
            (old.timeout_ms != new.timeout_ms && found_timeout != receive_signals.len()) {
            return Err("导入项目缺少可定位的标准参数，已拒绝修改且保留原文件".into());
        }
        self.commit_patches(patches, |w| w.frames.iter().any(|f| f.path == old.path && f.id == new.id && f.dlc == new.dlc && f.period_ms == new.period_ms && f.timeout_ms == new.timeout_ms))?;
        Ok(())
    }

    fn patch_imported_signal(&mut self, old: &SignalView, new: &SignalView) -> Result<(), String> {
        let mut patches: Vec<Vec<Patch>> = (0..self.files.len()).map(|_| Vec::new()).collect();
        let mut found_com = false;
        let mut found_mapping = false;
        let mut found_system_signal = false;
        let layout_changed = old.start_bit != new.start_bit || old.length != new.length;
        let system_path = format!("/{}/ISignal_{}", self.name, old.name);
        for (index, file) in self.files.iter().enumerate() {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element()) {
                if node.tag_name().name() == "ECUC-CONTAINER-VALUE" && path_of(node) == old.path {
                    if old.start_bit != new.start_bit { patch_param(node, "ComBitPosition", new.start_bit.to_string(), &mut patches[index])?; }
                    if old.length != new.length {
                        patch_param(node, "ComBitSize", new.length.to_string(), &mut patches[index])?;
                        patch_param(node, "ComSignalType", signal_type(new.length).into(), &mut patches[index])?;
                    }
                    if old.initial_value != new.initial_value { patch_param(node, "ComSignalInitValue", new.initial_value.to_string(), &mut patches[index])?; }
                    found_com = true;
                }
                if layout_changed && node.tag_name().name() == "I-SIGNAL-TO-I-PDU-MAPPING" &&
                    child_text(node, "I-SIGNAL-REF").as_deref() == Some(&system_path) &&
                    node.ancestors().any(|a| a.tag_name().name() == "I-SIGNAL-I-PDU" && path_of(a) == old.frame_path) {
                    if old.start_bit != new.start_bit { patch_child(node, "START-POSITION", new.start_bit.to_string(), &mut patches[index])?; }
                    found_mapping = true;
                }
                if old.length != new.length && node.tag_name().name() == "I-SIGNAL" && path_of(node) == system_path {
                    patch_child(node, "LENGTH", new.length.to_string(), &mut patches[index])?;
                    found_system_signal = true;
                }
            }
        }
        if !found_com || (layout_changed && !found_mapping) || (old.length != new.length && !found_system_signal) {
            return Err("信号系统映射不完整；拒绝破坏未知引用或位布局".into());
        }
        self.commit_patches(patches, |w| w.signals.iter().any(|s| s.path == old.path && s.start_bit == new.start_bit && s.length == new.length && s.initial_value == new.initial_value))?;
        Ok(())
    }

    pub fn add_frame(&mut self, name: String, id: u32, dlc: u8, direction: Direction, period_ms: Option<u32>, timeout_ms: Option<u32>) -> Result<WorkspaceView, String> {
        if !valid_name(&name) { return Err("帧名称只能包含 ASCII 字母、数字与下划线，且须以字母开头".into()); }
        let path = format!("/{}/Pdu_{}", self.name, name);
        if self.frames.iter().any(|f| f.path == path) { return Err("同名帧已经存在".into()); }
        let mut frames = self.frames.clone();
        frames.push(FrameView { path, name, id, dlc, direction, period_ms, timeout_ms });
        self.replace_managed(frames, self.signals.clone(), self.diagnostic.clone())?;
        Ok(self.view())
    }

    pub fn add_signal(&mut self, frame_path: String, name: String, start_bit: u8, length: u8, initial_value: u32) -> Result<WorkspaceView, String> {
        if !valid_name(&name) { return Err("信号名称只能包含 ASCII 字母、数字与下划线，且须以字母开头".into()); }
        if !self.frames.iter().any(|f| f.path == frame_path) { return Err("关联帧不存在".into()); }
        let path = format!("/{}/ComCfg/ComConfig/{}", self.name, name);
        if self.signals.iter().any(|s| s.path == path) { return Err("同名信号已经存在".into()); }
        let mut signals = self.signals.clone();
        signals.push(SignalView { path, name, frame_path, start_bit, length, initial_value });
        self.replace_managed(self.frames.clone(), signals, self.diagnostic.clone())?;
        Ok(self.view())
    }
    pub fn configure_diagnostic(
        &mut self, request_id: u32, response_id: u32, s3_ms: u32, n_bs_ms: u32, n_cr_ms: u32,
        did: u16, signal_paths: Vec<String>,
    ) -> Result<WorkspaceView, String> {
        let diagnostic = DiagnosticView {
            path: format!("/{}/DcmCfg/DcmConfigSet/DcmDsp/Did", self.name),
            request_id, response_id, s3_ms, n_bs_ms, n_cr_ms, did, signal_paths,
            dtc: self.diagnostic.as_ref().and_then(|existing| existing.dtc.clone()),
        };
        self.replace_managed(self.frames.clone(), self.signals.clone(), Some(diagnostic))?;
        Ok(self.view())
    }
    pub fn configure_dtc(&mut self, code: u32, monitor_frame_path: String) -> Result<WorkspaceView, String> {
        let mut diagnostic = self.diagnostic.clone().ok_or("须先配置诊断服务，再配置 UDS DTC")?;
        diagnostic.dtc = Some(DtcView {
            path: format!("/{}/DemCfg/DemConfigSet/DTC", self.name), code, monitor_frame_path,
        });
        if let Some(issue) = validate_diagnostic(&diagnostic, &self.frames, &self.signals).first() {
            return Err(format!("{}: {}", issue.code, issue.message));
        }
        self.replace_managed(self.frames.clone(), self.signals.clone(), Some(diagnostic))?;
        Ok(self.view())
    }

    pub fn clear_dtc(&mut self) -> Result<WorkspaceView, String> {
        let mut diagnostic = self.diagnostic.clone().ok_or("当前工程没有诊断配置")?;
        if diagnostic.dtc.is_none() { return Err("当前工程没有可移除的受支持 DTC".into()); }
        diagnostic.dtc = None;
        if self.files.iter().any(|file| self.is_managed_file(file)) {
            self.replace_managed(self.frames.clone(), self.signals.clone(), Some(diagnostic))?;
            return Ok(self.view());
        }
        let targets: BTreeSet<_> = [
            format!("/{}/DemCfg", self.name),
            format!("/{}/NvMCfg", self.name),
            format!("/{}/DcmCfg/DcmConfigSet/DcmDsd/Services/ClearDiagnosticInformation", self.name),
            format!("/{}/DcmCfg/DcmConfigSet/DcmDsd/Services/ReadDTCInformation", self.name),
            format!("/{}/DcmCfg/DcmConfigSet/DcmDsp/ClearDTC", self.name),
            format!("/{}/DcmCfg/DcmConfigSet/DcmDsp/ReadDTCInformation", self.name),
        ].into_iter().collect();
        let expected = render_profile(&self.name, &self.frames, &self.signals, self.diagnostic.as_ref());
        let expected_doc = Document::parse(&expected).map_err(|e| e.to_string())?;
        let expected_nodes: BTreeMap<_, _> = expected_doc.descendants().filter(|n| n.is_element()
            && child_text(*n, "SHORT-NAME").is_some() && targets.contains(&path_of(*n)))
            .map(|n| (path_of(n), structural_node(n))).collect();
        let mut patches: Vec<Vec<Patch>> = (0..self.files.len()).map(|_| Vec::new()).collect();
        let mut removed = BTreeSet::new();
        let mut owned_paths = BTreeSet::new();
        for (index, file) in self.files.iter().enumerate() {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element()
                && child_text(*n, "SHORT-NAME").is_some() && targets.contains(&path_of(*n))) {
                let path = path_of(node);
                if expected_nodes.get(&path) != Some(&structural_node(node)) {
                    return Err(format!("{path} 含未知或非工具所有的内容，拒绝删除"));
                }
                if !removed.insert(path.clone()) { return Err(format!("重复的 DTC 元素 {path}")); }
                owned_paths.extend(node.descendants().filter(|n| n.is_element() && child_text(*n, "SHORT-NAME").is_some()).map(path_of));
                patches[index].push(Patch { range: node.range(), value: String::new() });
            }
        }
        if removed != targets { return Err("DTC ARXML 节点不完整，拒绝部分删除".into()); }
        for file in &self.files {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element() && n.tag_name().name().ends_with("-REF") && n.tag_name().name() != "DEFINITION-REF") {
                if let Some(target) = node.text() {
                    if owned_paths.contains(target) && !node.ancestors().any(|ancestor| removed.contains(&path_of(ancestor))) {
                        return Err(format!("外部引用 {target} 仍依赖 DTC 配置，拒绝删除"));
                    }
                }
            }
        }
        self.commit_patches(patches, |workspace| workspace.diagnostic.as_ref().is_some_and(|d| d.dtc.is_none()))?;
        Ok(self.view())
    }


    pub fn clear_diagnostic(&mut self) -> Result<WorkspaceView, String> {
        if self.diagnostic.is_none() { return Err("当前工程没有可移除的受支持诊断配置".into()); }
        if self.files.iter().any(|file| self.is_managed_file(file)) {
            self.replace_managed(self.frames.clone(), self.signals.clone(), None)?;
            return Ok(self.view());
        }
        let mut patches: Vec<Vec<Patch>> = (0..self.files.len()).map(|_| Vec::new()).collect();
        let mut removed = BTreeSet::new();
        let mut owned_paths = BTreeSet::new();
        let mut targets: BTreeSet<_> = [
            format!("/{}/DcmCfg", self.name), format!("/{}/CanTpCfg", self.name),
            format!("/{}/NPdu_DiagRequest", self.name), format!("/{}/NPdu_DiagResponse", self.name),
            format!("/{}/DcmPdu_DiagRequest", self.name), format!("/{}/DcmPdu_DiagResponse", self.name),
        ].into_iter().collect();
        if self.diagnostic.as_ref().is_some_and(|d| d.dtc.is_some()) {
            targets.insert(format!("/{}/DemCfg", self.name));
            targets.insert(format!("/{}/NvMCfg", self.name));
        }
        let canif_targets: BTreeSet<_> = [
            format!("/{}/CanIfCfg/CanIfInitCfg/Can_DiagRequest", self.name),
            format!("/{}/CanIfCfg/CanIfInitCfg/Can_DiagResponse", self.name),
        ].into_iter().collect();
        let expected = render_profile(&self.name, &self.frames, &self.signals, self.diagnostic.as_ref());
        let expected_doc = Document::parse(&expected).map_err(|e| e.to_string())?;
        let expected_nodes: BTreeMap<_, _> = expected_doc.descendants().filter(|n| n.is_element()
            && child_text(*n, "SHORT-NAME").is_some()
            && (targets.contains(&path_of(*n)) || canif_targets.contains(&path_of(*n))))
            .map(|n| (path_of(n), structural_node(n))).collect();
        for file in &self.files {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element()) {
                let path = path_of(node);
                if (matches!(node.tag_name().name(), "ECUC-MODULE-CONFIGURATION-VALUES" | "N-PDU" | "DCM-I-PDU") &&
                    targets.contains(&path)) || (node.tag_name().name() == "ECUC-CONTAINER-VALUE" &&
                    canif_targets.contains(&path)) {
                    owned_paths.extend(node.descendants().filter(|n| n.is_element() && child_text(*n, "SHORT-NAME").is_some()).map(path_of));
                }
            }
        }
        for (index, file) in self.files.iter().enumerate() {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element()) {
                let path = path_of(node);
                let owned = (matches!(node.tag_name().name(), "ECUC-MODULE-CONFIGURATION-VALUES" | "N-PDU" | "DCM-I-PDU") &&
                    targets.contains(&path)) || (node.tag_name().name() == "ECUC-CONTAINER-VALUE" &&
                    canif_targets.contains(&path));
                if !owned { continue; }
                if expected_nodes.get(&path) != Some(&structural_node(node)) {
                    return Err(format!("{path} 含非工具所有的 ARXML 内容，拒绝删除"));
                }
                if !removed.insert(path.clone()) { return Err(format!("重复的诊断元素 {path}")); }
                patches[index].push(Patch { range: node.range(), value: String::new() });
            }
        }
        if removed.len() != targets.len() + canif_targets.len() { return Err("诊断 ARXML 节点不完整，拒绝部分删除".into()); }
        for file in &self.files {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element() && n.tag_name().name().ends_with("-REF") && n.tag_name().name() != "DEFINITION-REF") {
                if let Some(target) = node.text() {
                    if owned_paths.contains(target) && !node.ancestors().any(|ancestor| removed.contains(&path_of(ancestor))) {
                        return Err(format!("外部引用 {target} 仍依赖诊断配置，拒绝删除"));
                    }
                }
            }
        }
        self.commit_patches(patches, |workspace| workspace.diagnostic.is_none())?;
        Ok(self.view())
    }


    pub fn update_frame(&mut self, path: &str, changes: Value) -> Result<WorkspaceView, String> {
        let old = self.frames.iter().find(|f| f.path == path).ok_or("帧不存在")?.clone();
        let mut frames = self.frames.clone();
        let frame = frames.iter_mut().find(|f| f.path == path).ok_or("帧不存在")?;
        if changes.get("name").is_some_and(|v| v.as_str() != Some(&frame.name)) { return Err("重命名可能破坏跨文件引用；当前不允许重命名现有帧".into()); }
        if let Some(v) = changes.get("id") { frame.id = v.as_u64().ok_or("CAN 标识符须为无符号整数")?.try_into().map_err(|_| "CAN 标识符超出范围")?; }
        if let Some(v) = changes.get("dlc") { frame.dlc = v.as_u64().ok_or("DLC 须为整数")?.try_into().map_err(|_| "DLC 超出范围")?; }
        if let Some(v) = changes.get("periodMs") { frame.period_ms = if v.is_null() { None } else { Some(v.as_u64().ok_or("周期须为整数")?.try_into().map_err(|_| "周期超出范围")?) }; }
        if let Some(v) = changes.get("timeoutMs") { frame.timeout_ms = if v.is_null() { None } else { Some(v.as_u64().ok_or("超时须为整数")?.try_into().map_err(|_| "超时超出范围")?) }; }
        if let Some(v) = changes.get("direction") { let current = match frame.direction { Direction::Tx => "tx", Direction::Rx => "rx" }; if v.as_str() != Some(current) { return Err("切换方向需要重新建立 Com/CanIf 映射；请创建新帧".into()); } }
        if let Some(issue) = validate_profile(&frames, &self.signals).first() { return Err(format!("{}: {}", issue.code, issue.message)); }
        if let Some(diagnostic) = &self.diagnostic {
            if let Some(issue) = validate_diagnostic(diagnostic, &frames, &self.signals).first() {
                return Err(format!("{}: {}", issue.code, issue.message));
            }
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
        let old = self.signals.iter().find(|s| s.path == path).ok_or("信号不存在")?.clone();
        let mut signals = self.signals.clone();
        let signal = signals.iter_mut().find(|s| s.path == path).ok_or("信号不存在")?;
        if changes.get("name").is_some_and(|v| v.as_str() != Some(&signal.name)) || changes.get("framePath").is_some_and(|v| v.as_str() != Some(&signal.frame_path)) {
            return Err("重命名或迁移信号可能破坏跨文件引用；请创建新信号".into());
        }
        if let Some(v) = changes.get("startBit") { signal.start_bit = v.as_u64().ok_or("起始位须为整数")?.try_into().map_err(|_| "起始位超出范围")?; }
        if let Some(v) = changes.get("length") { signal.length = v.as_u64().ok_or("位长须为整数")?.try_into().map_err(|_| "位长超出范围")?; }
        if let Some(v) = changes.get("initialValue") { signal.initial_value = v.as_u64().ok_or("初始值须为整数")?.try_into().map_err(|_| "初始值超出范围")?; }
        if let Some(issue) = validate_profile(&self.frames, &signals).first() { return Err(format!("{}: {}", issue.code, issue.message)); }
        if let Some(diagnostic) = &self.diagnostic {
            if let Some(issue) = validate_diagnostic(diagnostic, &self.frames, &signals).first() {
                return Err(format!("{}: {}", issue.code, issue.message));
            }
        }
        let updated = signals.iter().find(|s| s.path == path).unwrap().clone();
        if self.files.iter().any(|file| self.is_managed_file(file)) {
            self.replace_managed(self.frames.clone(), signals, self.diagnostic.clone())?;
        } else {
            self.patch_imported_signal(&old, &updated)?;
        }
        Ok(self.view())
    }

    pub fn validate(&mut self) -> Result<WorkspaceView, String> {
        self.refresh()?;
        self.issues.extend(schema::validate_files(&self.schema_zip, &self.files.iter().map(|f| (f.path.as_path(), f.text.as_str())).collect::<Vec<_>>())?);
        let paths = self.all_paths()?;
        for file in &self.files {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element() && n.tag_name().name().ends_with("-REF") && n.tag_name().name() != "DEFINITION-REF") {
                if let Some(reference) = node.text() {
                    if reference.starts_with('/') && !paths.contains(reference) && !reference.starts_with("/AUTOSAR/EcucDefs/") {
                        self.issues.push(Issue { file: Some(file.path.display().to_string()), ..Issue::error("UNRESOLVED_REF", format!("跨文件引用未解析: {reference}"), Some(path_of(node.parent_element().unwrap_or(node)))) });
                    }
                }
            }
            for node in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "VARIATION-POINT") {
                let active_package = node.parent_element().filter(|parent| parent.tag_name().name() == "AR-PACKAGE").is_some_and(|package| {
                    let prefix = path_of(package);
                    self.frames.iter().any(|frame| frame.path.strip_prefix(&prefix).is_some_and(|suffix| suffix.starts_with('/')))
                });
                let active_root = node.parent_element().is_some_and(|parent| parent.tag_name().name() == "AUTOSAR") && !self.frames.is_empty();
                if active_package || active_root || node.ancestors().any(|ancestor| matches!(ancestor.tag_name().name(), "I-SIGNAL-I-PDU" | "ECUC-MODULE-CONFIGURATION-VALUES")) {
                    self.issues.push(Issue { severity: Severity::Warning, code: "VARIANT_DEPENDENCY".into(), message: "配置含未解析变体，可保存但禁止生成".into(), path: Some(path_of(node)), file: Some(file.path.display().to_string()) });
                }
            }
        }
        Ok(self.view())
    }

    fn all_paths(&self) -> Result<BTreeSet<String>, String> {
        let mut paths = BTreeSet::new();
        for file in &self.files {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element() && child_text(*n, "SHORT-NAME").is_some()) {
                paths.insert(path_of(node));
            }
        }
        Ok(paths)
    }

    pub fn save(&mut self) -> Result<WorkspaceView, String> {
        self.validate()?;
        if let Some(issue) = self.issues.iter().find(|i| matches!(i.severity, Severity::Error)) { return Err(format!("{}: {}", issue.code, issue.message)); }
        let dirty = self.files.iter().filter(|f| f.text != f.saved).collect::<Vec<_>>();
        let mut staged = Vec::new();
        for (index, file) in dirty.iter().enumerate() {
            let suffix = format!("arxml.autosar-config-{}-{index}", std::process::id());
            let stage = file.path.with_extension(format!("{suffix}.tmp"));
            let backup = file.path.with_extension(format!("{suffix}.bak"));
            let prepared = (|| -> Result<(), String> {
                if backup.exists() { return Err(format!("待恢复备份已存在，拒绝覆盖: {}", backup.display())); }
                if fs::read_to_string(&file.path).map_err(|e| e.to_string())? != file.saved {
                    return Err(format!("文件已被外部修改，拒绝覆盖: {}", file.path.display()));
                }
                let mut handle = fs::OpenOptions::new().write(true).create_new(true).open(&stage).map_err(|e| format!("暂存文件不可创建 {}: {e}", stage.display()))?;
                let written = std::io::Write::write_all(&mut handle, file.text.as_bytes()).and_then(|_| handle.sync_all());
                drop(handle);
                if written.is_err() { let _ = fs::remove_file(&stage); }
                written.map_err(|e| format!("暂存文件写入失败 {}: {e}", stage.display()))
            })();
            if let Err(error) = prepared {
                for (_, stage, _) in &staged { let _ = fs::remove_file(stage); }
                return Err(error);
            }
            staged.push((*file, stage, backup));
        }
        for index in 0..staged.len() {
            let (file, stage, backup) = &staged[index];
            if let Err(error) = install_staged(file, stage, backup) {
                let mut rollback_errors = Vec::new();
                for (file, _, backup) in staged[..index].iter().rev() {
                    if let Err(rollback) = restore_backup(&file.path, backup, Some(&file.text)) {
                        rollback_errors.push(rollback);
                    }
                }
                for (_, stage, _) in &staged { let _ = fs::remove_file(stage); }
                return Err(format!("ARXML 保存失败: {error}; 回滚问题: {}", rollback_errors.join("; ")));
            }
        }
        let mut cleanup_error = None;
        for (file, _, backup) in &staged {
            if fs::read_to_string(backup).ok().as_deref() != Some(&file.saved) {
                cleanup_error = Some(format!("备份内容发生变化，保留备份供检查: {}", backup.display()));
                break;
            }
            if let Err(error) = fs::remove_file(backup) {
                cleanup_error = Some(format!("已保存 ARXML，但无法清理备份 {}: {error}", backup.display()));
                break;
            }
        }
        for file in &mut self.files { file.saved = file.text.clone(); }
        if let Some(error) = cleanup_error { return Err(error); }
        Ok(self.view())
    }

    pub fn checked_profile(&mut self) -> Result<(Vec<FrameView>, Vec<SignalView>), String> {
        self.validate()?;
        if let Some(issue) = self.issues.iter().find(|i| matches!(i.severity, Severity::Error)) { return Err(format!("{}: {}", issue.code, issue.message)); }
        if let Some(issue) = self.issues.iter().find(|i| i.code == "VARIANT_DEPENDENCY") { return Err(format!("{}: {}", issue.code, issue.message)); }
        if self.frames.is_empty() || self.signals.is_empty() { return Err("NO_SIGNALS: 至少需要一帧和一个信号才能生成".into()); }
        if let Some(frame) = self.frames.iter().find(|f| !self.signals.iter().any(|s| s.frame_path == f.path)) {
            return Err(format!("FRAME_EMPTY: {} 没有信号，C99 运行代码无法初始化", frame.path));
        }
        Ok((self.frames.clone(), self.signals.clone()))
    }

    pub fn name(&self) -> &str { &self.name }
}

#[cfg(test)]
mod tests {
    use super::{install_staged, restore_backup, SourceFile};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let path = std::env::temp_dir().join(format!("autosar-save-{}-{nonce}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }

    #[test]
    fn staged_save_preserves_external_edit_between_preparation_and_installation() {
        let root = Scratch::new();
        let original = root.0.join("Ecu.arxml");
        let stage = root.0.join("Ecu.tmp");
        let backup = root.0.join("Ecu.bak");
        let file = SourceFile { path: original.clone(), saved: "original".into(), text: "ours".into() };
        fs::write(&original, &file.saved).unwrap();
        fs::write(&stage, &file.text).unwrap();
        fs::write(&original, "external").unwrap();
        let conflict = install_staged(&file, &stage, &backup).unwrap_err();
        assert!(conflict.contains("外部修改"), "{conflict}");
        assert_eq!(fs::read_to_string(&original).unwrap(), "external");
        assert!(!backup.exists());
        assert!(stage.exists());

        fs::write(&original, &file.saved).unwrap();
        install_staged(&file, &stage, &backup).unwrap();
        assert_eq!(fs::read_to_string(&original).unwrap(), "ours");
        assert_eq!(fs::read_to_string(&backup).unwrap(), "original");
        restore_backup(&original, &backup, Some(&file.text)).unwrap();
        assert_eq!(fs::read_to_string(&original).unwrap(), "original");
    }

    #[test]
    fn rollback_never_removes_a_new_external_original() {
        let root = Scratch::new();
        let original = root.0.join("Ecu.arxml");
        let backup = root.0.join("Ecu.bak");
        fs::write(&original, "external").unwrap();
        fs::write(&backup, "original").unwrap();
        assert!(restore_backup(&original, &backup, Some("ours")).is_err());
        assert_eq!(fs::read_to_string(&original).unwrap(), "external");
        assert_eq!(fs::read_to_string(&backup).unwrap(), "original");
    }
}
