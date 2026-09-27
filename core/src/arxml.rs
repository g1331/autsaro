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
struct GlobalPduBinding {
    system_path: String,
    length: u32,
    file: String,
}
struct ComPduRecord {
    path: String,
    target: Option<String>,
    dest: Option<String>,
    direction: Option<String>,
    period: Option<String>,
    processing: Option<String>,
    kind: Option<String>,
    unused: Option<String>,
    signals: Vec<String>,
    file: String,
}

struct CanIfPduRecord {
    can_id: Option<String>,
    dlc: Option<String>,
    id_type: Option<String>,
    tx: bool,
    dest: Option<String>,
    path: String,
    file: String,
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

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
struct EcucRecord {
    path: String,
    kind: String,
    definition: String,
    value: String,
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
    node.children()
        .find(|n| {
            n.is_element() && n.tag_name().namespace() == Some(NS) && n.tag_name().name() == name
        })?
        .text()
        .map(str::to_owned)
}

fn definition(node: Node<'_, '_>) -> Option<String> {
    child_text(node, "DEFINITION-REF")
}

fn path_of(node: Node<'_, '_>) -> String {
    let mut names: Vec<String> = node
        .ancestors()
        .filter_map(|n| child_text(n, "SHORT-NAME"))
        .collect();
    names.reverse();
    format!("/{}", names.join("/"))
}

fn param(node: Node<'_, '_>, name: &str) -> Option<String> {
    node.descendants()
        .filter(|n| {
            n.is_element()
                && matches!(
                    n.tag_name().name(),
                    "ECUC-NUMERICAL-PARAM-VALUE" | "ECUC-TEXTUAL-PARAM-VALUE"
                )
        })
        .find(|n| definition(*n).is_some_and(|p| p.ends_with(&format!("/{name}"))))
        .and_then(|n| child_text(n, "VALUE"))
}

fn ref_value(node: Node<'_, '_>, suffix: &str) -> Option<String> {
    node.descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "ECUC-REFERENCE-VALUE")
        .find(|n| definition(*n).is_some_and(|p| p.ends_with(&format!("/{suffix}"))))
        .and_then(|n| child_text(n, "VALUE-REF"))
}
fn ref_dest(node: Node<'_, '_>, suffix: &str) -> Option<String> {
    node.descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "ECUC-REFERENCE-VALUE")
        .find(|n| definition(*n).is_some_and(|path| path.ends_with(&format!("/{suffix}"))))
        .and_then(|reference| {
            reference
                .children()
                .find(|child| child.is_element() && child.tag_name().name() == "VALUE-REF")
        })
        .and_then(|value| value.attribute("DEST"))
        .map(str::to_owned)
}

fn host_ecuc_records(module: Node<'_, '_>) -> Vec<EcucRecord> {
    let mut records = Vec::new();
    for node in module.descendants().filter(|n| n.is_element()) {
        let kind = node.tag_name().name();
        if !matches!(
            kind,
            "ECUC-MODULE-CONFIGURATION-VALUES"
                | "ECUC-CONTAINER-VALUE"
                | "ECUC-NUMERICAL-PARAM-VALUE"
                | "ECUC-TEXTUAL-PARAM-VALUE"
                | "ECUC-REFERENCE-VALUE"
        ) {
            continue;
        }
        let owner = if matches!(
            kind,
            "ECUC-MODULE-CONFIGURATION-VALUES" | "ECUC-CONTAINER-VALUE"
        ) {
            node
        } else {
            node.ancestors()
                .find(|ancestor| {
                    ancestor.is_element() && ancestor.tag_name().name() == "ECUC-CONTAINER-VALUE"
                })
                .unwrap_or(module)
        };
        let value = if kind == "ECUC-REFERENCE-VALUE" {
            node.children()
                .find(|child| child.is_element() && child.tag_name().name() == "VALUE-REF")
                .map(|child| {
                    format!(
                        "{}:{}",
                        child.attribute("DEST").unwrap_or(""),
                        child.text().unwrap_or("")
                    )
                })
                .unwrap_or_default()
        } else if matches!(
            kind,
            "ECUC-NUMERICAL-PARAM-VALUE" | "ECUC-TEXTUAL-PARAM-VALUE"
        ) {
            child_text(node, "VALUE").unwrap_or_default()
        } else {
            String::new()
        };
        records.push(EcucRecord {
            path: path_of(owner),
            kind: kind.to_owned(),
            definition: definition(node).unwrap_or_default(),
            value,
        });
    }
    records.sort();
    records
}

fn validate_host_can_ecuc(
    files: &[SourceFile],
    project: &str,
    frames: &[FrameView],
    signals: &[SignalView],
    diagnostic: Option<&DiagnosticView>,
) -> Vec<Issue> {
    let expected_xml = render_profile(project, frames, signals, diagnostic);
    let expected_doc = match Document::parse(&expected_xml) {
        Ok(doc) => doc,
        Err(error) => {
            return vec![Issue::error(
                "PDU_UNSUPPORTED",
                format!("当前输入无法形成固定主机 CAN 剖面: {error}"),
                None,
            )];
        }
    };
    let expected_paths = ["McuCfg", "CanCfg", "CanIfCfg"].map(|name| format!("/{project}/{name}"));
    let mut expected = BTreeMap::new();
    for module in expected_doc.descendants().filter(|node| {
        node.is_element() && node.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
    }) {
        let path = path_of(module);
        if expected_paths.contains(&path) {
            expected.insert(path, host_ecuc_records(module));
        }
    }
    let mut actual = BTreeMap::new();
    let mut issues = Vec::new();
    for file in files {
        let doc = match Document::parse(&file.text) {
            Ok(doc) => doc,
            Err(_) => continue,
        };
        for module in doc.descendants().filter(|node| {
            node.is_element() && node.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
        }) {
            let path = path_of(module);
            let own_module = expected_paths.contains(&path);
            let can_definition = matches!(
                definition(module).as_deref(),
                Some("/AUTOSAR/EcucDefs/Mcu" | "/AUTOSAR/EcucDefs/Can" | "/AUTOSAR/EcucDefs/CanIf")
            );
            if !own_module && !(path.starts_with(&format!("/{project}/")) && can_definition) {
                continue;
            }
            if !own_module {
                issues.push(Issue {
                    file: Some(file.path.display().to_string()),
                    ..Issue::error(
                        "PDU_UNSUPPORTED",
                        "额外的 Mcu/Can/CanIf 模块不属于固定主机剖面",
                        Some(path),
                    )
                });
                continue;
            }
            if actual
                .insert(
                    path.clone(),
                    (host_ecuc_records(module), file.path.display().to_string()),
                )
                .is_some()
            {
                issues.push(Issue {
                    file: Some(file.path.display().to_string()),
                    ..Issue::error(
                        "PDU_UNSUPPORTED",
                        "主机 Mcu/Can/CanIf 模块路径重复",
                        Some(path),
                    )
                });
            }
        }
    }
    for path in expected_paths {
        match (expected.get(&path), actual.get(&path)) {
            (None, None) => {}
            (Some(_), None) => issues.push(Issue {
                file: files.first().map(|file| file.path.display().to_string()),
                ..Issue::error(
                    "PDU_UNSUPPORTED",
                    "缺少固定主机 Mcu/Can/CanIf ECUC 模块",
                    Some(path),
                )
            }),
            (None, Some((_, file))) => issues.push(Issue {
                file: Some(file.clone()),
                ..Issue::error(
                    "PDU_UNSUPPORTED",
                    "无 CAN PDU 的工程不应含主机 Mcu/Can/CanIf 配置",
                    Some(path),
                )
            }),
            (Some(want), Some((got, file))) if want != got => {
                let missing = want.iter().find(|record| !got.contains(record));
                let extra = got.iter().find(|record| !want.contains(record));
                let record = missing.or(extra).unwrap();
                let field = record.definition.rsplit('/').next().unwrap_or("配置");
                let message = if missing.is_some() {
                    format!("主机 CAN ECUC 缺少或不匹配必需项 {field}")
                } else {
                    format!("主机 CAN ECUC 含未支持的配置项 {field}")
                };
                issues.push(Issue {
                    file: Some(file.clone()),
                    ..Issue::error("PDU_UNSUPPORTED", message, Some(record.path.clone()))
                });
            }
            _ => {}
        }
    }
    issues
}
fn tool_global_pdu(node: Node<'_, '_>, file: &SourceFile) -> Result<GlobalPduBinding, String> {
    let groups: Vec<_> = node
        .descendants()
        .filter(|child| {
            child.is_element()
                && child.tag_name().name() == "SDG"
                && child
                    .attribute("GID")
                    .is_some_and(|gid| gid.starts_with("AutosarWorkbenchGlobalPdu"))
        })
        .collect();
    let [group] = groups.as_slice() else {
        return Err("缺少唯一版本化全局 PDU 绑定".into());
    };
    if group.attribute("GID") != Some("AutosarWorkbenchGlobalPduV1")
        || !group.parent_element().is_some_and(|sdgs| {
            sdgs.tag_name().name() == "SDGS"
                && sdgs.parent_element().is_some_and(|admin| {
                    admin.tag_name().name() == "ADMIN-DATA" && admin.parent_element() == Some(node)
                })
        })
    {
        return Err("全局 PDU 绑定版本或位置不受支持".into());
    }
    let all_groups: Vec<_> = node
        .descendants()
        .filter(|child| child.is_element() && child.tag_name().name() == "SDG")
        .collect();
    let values: Vec<_> = node
        .children()
        .filter(|child| child.is_element() && child.tag_name().name() == "PARAMETER-VALUES")
        .flat_map(|group| group.children().filter(|child| child.is_element()))
        .collect();
    if all_groups.len() != 1
        || group.attributes().len() != 1
        || values.len() != 1
        || node.children().any(|child| {
            child.is_element()
                && matches!(
                    child.tag_name().name(),
                    "REFERENCE-VALUES" | "SUB-CONTAINERS"
                )
        })
        || values[0].tag_name().name() != "ECUC-NUMERICAL-PARAM-VALUE"
        || values[0]
            .children()
            .find(|child| child.is_element() && child.tag_name().name() == "DEFINITION-REF")
            .is_none_or(|definition| {
                definition.attribute("DEST") != Some("ECUC-INTEGER-PARAM-DEF")
                    || definition.text()
                        != Some(
                            "/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu/PduLength",
                        )
            })
    {
        return Err("全局 PDU 只接受 PduLength，不支持 DynamicLength、引用或未知工具数据".into());
    }
    let fields: Vec<_> = group
        .children()
        .filter(|child| child.is_element())
        .collect();
    if fields.len() != 1
        || fields[0].tag_name().name() != "SD"
        || fields[0].attribute("GID") != Some("SystemPduRef")
    {
        return Err("全局 PDU 绑定必须有唯一 SystemPduRef".into());
    }
    let system_path = fields[0]
        .text()
        .filter(|value| value.starts_with('/'))
        .ok_or("全局 PDU SystemPduRef 必须为绝对路径")?
        .to_owned();
    let length = param(node, "PduLength")
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or("全局 PDU 缺少有效 PduLength")?;
    Ok(GlobalPduBinding {
        system_path,
        length,
        file: file.path.display().to_string(),
    })
}
fn child_containers<'a, 'input>(node: Node<'a, 'input>) -> Vec<Node<'a, 'input>> {
    node.children()
        .filter(|child| {
            child.is_element() && matches!(child.tag_name().name(), "CONTAINERS" | "SUB-CONTAINERS")
        })
        .flat_map(|group| {
            group.children().filter(|child| {
                child.is_element() && child.tag_name().name() == "ECUC-CONTAINER-VALUE"
            })
        })
        .collect()
}
fn module_definition(node: Node<'_, '_>, expected: &str) -> bool {
    let mut refs = node
        .children()
        .filter(|child| child.is_element() && child.tag_name().name() == "DEFINITION-REF");
    refs.next().is_some_and(|reference| {
        reference.attribute("DEST") == Some("ECUC-MODULE-DEF") && reference.text() == Some(expected)
    }) && refs.next().is_none()
}

fn tool_com_root(node: Node<'_, '_>) -> bool {
    let configs: Vec<_> = node
        .descendants()
        .filter(|child| {
            child.is_element()
                && child.tag_name().name() == "ECUC-CONTAINER-VALUE"
                && definition(*child).as_deref() == Some("/AUTOSAR/EcucDefs/Com/ComConfig")
        })
        .collect();
    let generals: Vec<_> = node
        .descendants()
        .filter(|child| {
            child.is_element()
                && child.tag_name().name() == "ECUC-CONTAINER-VALUE"
                && definition(*child).as_deref() == Some("/AUTOSAR/EcucDefs/Com/ComGeneral")
        })
        .collect();
    if configs.len() != 1 || generals.len() != 1 {
        return false;
    }
    let roots = child_containers(node);
    if roots.len() != 2
        || !roots.contains(&configs[0])
        || !roots.contains(&generals[0])
        || !child_containers(generals[0]).is_empty()
        || child_containers(configs[0]).iter().any(|child| {
            !matches!(
                definition(*child).as_deref(),
                Some(
                    "/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu"
                        | "/AUTOSAR/EcucDefs/Com/ComConfig/ComSignal"
                )
            )
        })
        || generals[0]
            .children()
            .any(|child| child.is_element() && child.tag_name().name() == "REFERENCE-VALUES")
    {
        return false;
    }
    let general = generals[0];
    let values: Vec<_> = general
        .children()
        .filter(|child| child.is_element() && child.tag_name().name() == "PARAMETER-VALUES")
        .flat_map(|group| group.children().filter(|child| child.is_element()))
        .collect();
    values.len() == 4
        && param(general, "ComEnableSecurityEventReporting").as_deref() == Some("false")
        && param(general, "ComEnableSignalGroupArrayApi").as_deref() == Some("false")
        && param(general, "ComSupportedIPduGroups").as_deref() == Some("0")
        && param(general, "ComVersionInfoApi").as_deref() == Some("false")
}
fn tool_com_parent(node: Node<'_, '_>, module_path: &str, config_path: &str) -> bool {
    let Some(subcontainers) = node.parent_element() else {
        return false;
    };
    let Some(config) = subcontainers.parent_element() else {
        return false;
    };
    let Some(containers) = config.parent_element() else {
        return false;
    };
    let Some(module) = containers.parent_element() else {
        return false;
    };
    subcontainers.tag_name().name() == "SUB-CONTAINERS"
        && config.tag_name().name() == "ECUC-CONTAINER-VALUE"
        && definition(config).as_deref() == Some("/AUTOSAR/EcucDefs/Com/ComConfig")
        && path_of(config) == config_path
        && containers.tag_name().name() == "CONTAINERS"
        && module.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
        && path_of(module) == module_path
        && module_definition(module, "/AUTOSAR/EcucDefs/Com")
}

fn tool_ecuc_root(node: Node<'_, '_>) -> bool {
    let find = |definition_path: &str| -> Vec<_> {
        node.descendants()
            .filter(|child| {
                child.is_element()
                    && child.tag_name().name() == "ECUC-CONTAINER-VALUE"
                    && definition(*child).as_deref() == Some(definition_path)
            })
            .collect()
    };
    let configs = find("/AUTOSAR/EcucDefs/EcuC/EcucConfigSet");
    let hardware = find("/AUTOSAR/EcucDefs/EcuC/EcucHardware");
    let cores = find("/AUTOSAR/EcucDefs/EcuC/EcucHardware/EcucCoreDefinition");
    let collections = find("/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection");
    let ([config], [hardware], [core], [collection]) = (
        configs.as_slice(),
        hardware.as_slice(),
        cores.as_slice(),
        collections.as_slice(),
    ) else {
        return false;
    };
    let roots = child_containers(node);
    let hardware_children = child_containers(*hardware);
    let config_children = child_containers(*config);
    if roots.len() != 2
        || !roots.contains(config)
        || !roots.contains(hardware)
        || hardware_children.as_slice() != [*core]
        || config_children.as_slice() != [*collection]
        || !child_containers(*core).is_empty()
        || param(*core, "EcucCoreId").as_deref() != Some("0")
        || param(*collection, "PduIdTypeEnum").as_deref() != Some("UINT8")
    {
        return false;
    }
    let values: Vec<_> = collection
        .children()
        .filter(|child| child.is_element() && child.tag_name().name() == "PARAMETER-VALUES")
        .flat_map(|group| group.children().filter(|child| child.is_element()))
        .collect();
    let pdus = find("/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu");
    let children = child_containers(*collection);
    if children.len() != pdus.len() || children.iter().any(|child| !pdus.contains(child)) {
        return false;
    }
    let wide = pdus.iter().any(|pdu| {
        param(*pdu, "PduLength")
            .and_then(|value| value.parse::<u32>().ok())
            .is_some_and(|length| length > 255)
    });
    values.len() == 2
        && param(*collection, "PduLengthTypeEnum").as_deref()
            == Some(if wide { "UINT16" } else { "UINT8" })
}

fn parse_u32(value: Option<String>, field: &str, path: &str) -> Result<u32, Issue> {
    value
        .ok_or_else(|| {
            Issue::error(
                "MISSING_PARAMETER",
                format!("缺少 {field}"),
                Some(path.into()),
            )
        })?
        .parse::<u32>()
        .map_err(|_| {
            Issue::error(
                "INVALID_PARAMETER",
                format!("{field} 须为无符号整数"),
                Some(path.into()),
            )
        })
}

fn parse_milliseconds(value: Option<String>, field: &str, path: &str) -> Result<u32, Issue> {
    let text = value.ok_or_else(|| {
        Issue::error(
            "MISSING_PARAMETER",
            format!("缺少 {field}"),
            Some(path.into()),
        )
    })?;
    let (seconds, fraction) = text.split_once('.').unwrap_or((&text, ""));
    if fraction.len() > 3 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Issue::error(
            "TIME_PRECISION",
            format!("{field} 仅支持整毫秒"),
            Some(path.into()),
        ));
    }
    let seconds = seconds
        .parse::<u32>()
        .map_err(|_| Issue::error("INVALID_PARAMETER", field, Some(path.into())))?;
    let fraction = if fraction.is_empty() {
        0
    } else {
        fraction
            .parse::<u32>()
            .map_err(|_| Issue::error("INVALID_PARAMETER", field, Some(path.into())))?
            * 10u32.pow((3 - fraction.len()) as u32)
    };
    seconds
        .checked_mul(1000)
        .and_then(|s| s.checked_add(fraction))
        .ok_or_else(|| Issue::error("TIME_RANGE", field, Some(path.into())))
}

fn valid_name(name: &str) -> bool {
    name.len() <= 128
        && name.bytes().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

fn load_sources(files: Vec<PathBuf>, schema_zip: PathBuf) -> Result<Workspace, String> {
    if files.is_empty() {
        return Err("请选择至少一份 .arxml 文件".into());
    }
    let mut sources = Vec::new();
    let mut unique = BTreeSet::new();
    for path in files {
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

struct Patch {
    range: std::ops::Range<usize>,
    value: String,
}

fn patch_child(
    node: Node<'_, '_>,
    child_name: &str,
    value: String,
    patches: &mut Vec<Patch>,
) -> Result<(), String> {
    let leaf = node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == child_name)
        .ok_or_else(|| format!("{} 缺少 {child_name}，拒绝不安全编辑", path_of(node)))?;
    let mut content = leaf.children();
    let text = content
        .next()
        .filter(|n| n.is_text())
        .ok_or("值不是简单文本；拒绝不安全编辑")?;
    if content.next().is_some() {
        return Err("值有混合内容；拒绝不安全编辑".into());
    }
    patches.push(Patch {
        range: text.range(),
        value,
    });
    Ok(())
}

fn patch_param(
    node: Node<'_, '_>,
    name: &str,
    value: String,
    patches: &mut Vec<Patch>,
) -> Result<(), String> {
    let mut found = node
        .descendants()
        .filter(|n| {
            n.is_element()
                && matches!(
                    n.tag_name().name(),
                    "ECUC-NUMERICAL-PARAM-VALUE" | "ECUC-TEXTUAL-PARAM-VALUE"
                )
        })
        .filter(|n| definition(*n).is_some_and(|d| d.ends_with(&format!("/{name}"))));
    let parameter = found
        .next()
        .ok_or_else(|| format!("缺少 {name}，拒绝不安全编辑"))?;
    if found.next().is_some() {
        return Err(format!("{name} 存在多个变体，拒绝不安全编辑"));
    }
    patch_child(parameter, "VALUE", value, patches)
}

fn seconds(milliseconds: u32) -> String {
    format!("{}.{:03}", milliseconds / 1000, milliseconds % 1000)
}

fn signal_type(length: u8) -> &'static str {
    if length == 1 {
        "BOOLEAN"
    } else if length <= 8 {
        "UINT8"
    } else if length <= 16 {
        "UINT16"
    } else {
        "UINT32"
    }
}

fn apply_patches(text: &mut String, patches: &mut Vec<Patch>) -> Result<(), String> {
    patches.sort_by_key(|patch| std::cmp::Reverse(patch.range.start));
    let mut next_start = text.len();
    for patch in patches {
        if patch.range.end > next_start {
            return Err("编辑范围重叠；保留原 ARXML".into());
        }
        next_start = patch.range.start;
        text.replace_range(patch.range.clone(), &patch.value);
    }
    Ok(())
}

fn restore_backup(original: &Path, backup: &Path, installed: Option<&str>) -> Result<(), String> {
    if original.exists() {
        let owned = installed
            .is_some_and(|text| fs::read_to_string(original).is_ok_and(|current| current == text));
        if !owned {
            return Err(format!(
                "外部文件 {} 未覆盖；原备份保留在 {}",
                original.display(),
                backup.display()
            ));
        }
        fs::remove_file(original).map_err(|e| format!("{}: {e}", original.display()))?;
    }
    fs::rename(backup, original)
        .map_err(|e| format!("{} -> {}: {e}", backup.display(), original.display()))
}

fn install_staged(file: &SourceFile, stage: &Path, backup: &Path) -> Result<(), String> {
    fs::rename(&file.path, backup)
        .map_err(|e| format!("{} -> {}: {e}", file.path.display(), backup.display()))?;
    let result = fs::read_to_string(backup)
        .map_err(|e| format!("无法复核原文件 {}: {e}", backup.display()))
        .and_then(|actual| {
            if actual != file.saved {
                return Err(format!(
                    "文件已被外部修改，拒绝覆盖: {}",
                    file.path.display()
                ));
            }
            fs::rename(stage, &file.path)
                .map_err(|e| format!("{} -> {}: {e}", stage.display(), file.path.display()))
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
            let mut attributes: Vec<_> = node
                .attributes()
                .map(|a| (a.namespace().unwrap_or(""), a.name(), a.value()))
                .collect();
            attributes.sort_unstable();
            for attribute in attributes {
                write!(out, " {:?}", attribute).unwrap();
            }
            out.push('>');
            for child in node.children() {
                append(child, out);
            }
            out.push_str("</");
            out.push_str(node.tag_name().name());
            out.push('>');
        } else if node.is_text() {
            let text = node.text().unwrap_or("").trim();
            if !text.is_empty() {
                out.push_str(text);
            }
        }
    }
    let mut out = String::new();
    append(node, &mut out);
    out
}

type DiagnosticShape = BTreeMap<
    String,
    (
        String,
        String,
        Vec<(String, String, String, String)>,
        String,
    ),
>;

fn diagnostic_shape(docs: &[Document<'_>]) -> Result<DiagnosticShape, String> {
    let mut shape = BTreeMap::new();
    for doc in docs {
        for module in doc.descendants().filter(|n| {
            n.is_element()
                && n.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
                && matches!(
                    definition(*n).as_deref(),
                    Some(
                        "/AUTOSAR/EcucDefs/Dcm"
                            | "/AUTOSAR/EcucDefs/CanTp"
                            | "/AUTOSAR/EcucDefs/Dem"
                            | "/AUTOSAR/EcucDefs/NvM"
                    )
                )
        }) {
            for node in module.descendants().filter(|n| {
                n.is_element()
                    && matches!(
                        n.tag_name().name(),
                        "ECUC-MODULE-CONFIGURATION-VALUES" | "ECUC-CONTAINER-VALUE"
                    )
            }) {
                let mut values = Vec::new();
                for group in node.children().filter(|n| {
                    n.is_element()
                        && matches!(n.tag_name().name(), "PARAMETER-VALUES" | "REFERENCE-VALUES")
                }) {
                    for item in group.children().filter(|n| n.is_element()) {
                        let value = child_text(
                            item,
                            if group.tag_name().name() == "REFERENCE-VALUES" {
                                "VALUE-REF"
                            } else {
                                "VALUE"
                            },
                        )
                        .ok_or_else(|| format!("{} 缺少 ECUC 值", path_of(node)))?;
                        let definition = definition(item)
                            .ok_or_else(|| format!("{} 缺少 ECUC 参数定义", path_of(node)))?;
                        let definition_dest = item
                            .children()
                            .find(|n| n.is_element() && n.tag_name().name() == "DEFINITION-REF")
                            .and_then(|n| n.attribute("DEST"))
                            .unwrap_or("")
                            .to_owned();
                        let value_dest = item
                            .children()
                            .find(|n| n.is_element() && n.tag_name().name() == "VALUE-REF")
                            .and_then(|n| n.attribute("DEST"))
                            .unwrap_or("")
                            .to_owned();
                        values.push((definition, definition_dest, value, value_dest));
                    }
                }
                values.sort();
                let path = path_of(node);
                let definition_dest = node
                    .children()
                    .find(|n| n.is_element() && n.tag_name().name() == "DEFINITION-REF")
                    .and_then(|n| n.attribute("DEST"))
                    .unwrap_or("")
                    .to_owned();
                let strict = matches!(
                    definition(module).as_deref(),
                    Some("/AUTOSAR/EcucDefs/Dem" | "/AUTOSAR/EcucDefs/NvM")
                );
                if shape
                    .insert(
                        path.clone(),
                        (
                            definition(node).unwrap_or_default(),
                            definition_dest,
                            values,
                            if strict {
                                structural_node(node)
                            } else {
                                String::new()
                            },
                        ),
                    )
                    .is_some()
                {
                    return Err(format!("重复的诊断 ECUC 容器 {path}"));
                }
            }
        }
    }
    Ok(shape)
}

const HOST_ROUTINE_GID: &str = "AutosarWorkbenchHostRestoreDidV1";

fn parse_host_routine(
    did: Node<'_, '_>,
    project: &str,
    nodes: &[Node<'_, '_>],
) -> Result<Option<u16>, String> {
    let label = path_of(did);
    let mut groups = nodes.iter().copied().filter(|node| {
        node.tag_name().name() == "SDG"
            && node
                .attribute("GID")
                .is_some_and(|gid| gid.starts_with("AutosarWorkbenchHostRestoreDid"))
    });
    let group = groups.next();
    if groups.next().is_some() {
        return Err(format!("{label}: 主机复位例程 SDG 重复"));
    }
    let Some(group) = group else {
        return Ok(None);
    };
    if group.attribute("GID") != Some(HOST_ROUTINE_GID)
        || group.attributes().len() != 1
        || !group.parent_element().is_some_and(|parent| {
            parent.tag_name().name() == "SDGS"
                && parent.parent_element().is_some_and(|admin| {
                    admin.tag_name().name() == "ADMIN-DATA" && admin.parent_element() == Some(did)
                })
        })
    {
        return Err(format!("{label}: 主机复位例程 SDG 版本或所在 DID 不受支持"));
    }
    let fields: Vec<_> = group.children().filter(|node| node.is_element()).collect();
    if fields.len() != 2
        || fields
            .iter()
            .any(|node| node.tag_name().name() != "SD" || node.attributes().len() != 1)
    {
        return Err(format!("{label}: 主机复位例程须有唯一 Rid 和 SessionRef"));
    }
    let field = |name: &str| {
        let mut found = fields
            .iter()
            .filter(|node| node.attribute("GID") == Some(name));
        let value = found.next().and_then(|node| node.text());
        if found.next().is_some() { None } else { value }
    };
    let rid = field("Rid")
        .ok_or_else(|| format!("{label}: 主机复位例程缺少唯一 Rid"))?
        .parse::<u16>()
        .map_err(|_| format!("{label}: 主机复位例程 Rid 须为 16 位十进制整数"))?;
    let session = format!("/{project}/DcmCfg/DcmConfigSet/DcmDsp/Sessions/Extended");
    if field("SessionRef") != Some(session.as_str()) {
        return Err(format!("{label}: 主机复位例程 SessionRef 须指向 {session}"));
    }
    Ok(Some(rid))
}

fn parse_diagnostic(
    files: &[SourceFile],
    project: &str,
    frames: &[FrameView],
    signals: &[SignalView],
) -> Result<Option<DiagnosticView>, String> {
    let docs: Vec<_> = files
        .iter()
        .map(|file| Document::parse(&file.text).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let actual = diagnostic_shape(&docs)?;
    let nodes: Vec<_> = docs
        .iter()
        .flat_map(|doc| doc.descendants().filter(|n| n.is_element()))
        .collect();
    if actual.is_empty() {
        if nodes.iter().any(|node| {
            node.tag_name().name() == "SDG"
                && node
                    .attribute("GID")
                    .is_some_and(|gid| gid.starts_with("AutosarWorkbenchHostRestoreDid"))
        }) {
            return Err("主机例程工具记录存在，但缺少 DID 与诊断 ECUC 配置".into());
        }
        let request = format!("/{project}/NPdu_DiagRequest");
        let response = format!("/{project}/NPdu_DiagResponse");
        if nodes.iter().any(|n| {
            n.tag_name().name() == "N-PDU" && (path_of(*n) == request || path_of(*n) == response)
        }) {
            return Err("诊断 N-PDU 存在，但缺少对应的 CanTp/Dcm 配置".into());
        }
        return Ok(None);
    }
    let unique = |def: &str| -> Result<Node<'_, '_>, String> {
        let mut found = nodes.iter().copied().filter(|n| {
            n.tag_name().name() == "ECUC-CONTAINER-VALUE" && definition(*n).as_deref() == Some(def)
        });
        let node = found.next().ok_or_else(|| format!("诊断配置缺少 {def}"))?;
        if found.next().is_some() {
            return Err(format!("诊断配置含多个 {def}"));
        }
        Ok(node)
    };
    let general = unique("/AUTOSAR/EcucDefs/Dcm/DcmGeneral")?;
    let rx_sdu = unique("/AUTOSAR/EcucDefs/CanTp/CanTpConfig/CanTpChannel/CanTpRxNSdu")?;
    let tx_sdu = unique("/AUTOSAR/EcucDefs/CanTp/CanTpConfig/CanTpChannel/CanTpTxNSdu")?;
    let did_node = unique("/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsp/DcmDspDid")?;
    let s3_ms = parse_milliseconds(
        param(general, "DcmS3ServerTimeoutOverwrite"),
        "DcmS3ServerTimeoutOverwrite",
        &path_of(general),
    )
    .map_err(|e| e.message)?;
    let n_as_ms = parse_milliseconds(param(tx_sdu, "CanTpNas"), "CanTpNas", &path_of(tx_sdu))
        .map_err(|e| e.message)?;
    let n_bs_ms = parse_milliseconds(param(tx_sdu, "CanTpNbs"), "CanTpNbs", &path_of(tx_sdu))
        .map_err(|e| e.message)?;
    let n_cr_ms = parse_milliseconds(param(rx_sdu, "CanTpNcr"), "CanTpNcr", &path_of(rx_sdu))
        .map_err(|e| e.message)?;
    let did = parse_u32(
        param(did_node, "DcmDspDidIdentifier"),
        "DcmDspDidIdentifier",
        &path_of(did_node),
    )
    .map_err(|e| e.message)?;
    let did: u16 = did.try_into().map_err(|_| "诊断 DID 超出 16 位范围")?;
    let request = format!("/{project}/NPdu_DiagRequest");
    let response = format!("/{project}/NPdu_DiagResponse");
    let mut ids = Vec::new();
    for name in ["DcmPdu_DiagRequest", "DcmPdu_DiagResponse"] {
        let path = format!("/{project}/{name}");
        let pdus: Vec<_> = nodes
            .iter()
            .copied()
            .filter(|n| n.tag_name().name() == "DCM-I-PDU" && path_of(*n) == path)
            .collect();
        if pdus.len() != 1 || child_text(pdus[0], "LENGTH").as_deref() != Some("256") {
            return Err(format!("{path} 必须是唯一的最大 256 字节 DCM-I-PDU"));
        }
    }
    for (pdu, tx) in [(&request, false), (&response, true)] {
        let pdus: Vec<_> = nodes
            .iter()
            .copied()
            .filter(|n| n.tag_name().name() == "N-PDU" && path_of(*n) == *pdu)
            .collect();
        if pdus.len() != 1 || child_text(pdus[0], "LENGTH").as_deref() != Some("8") {
            return Err(format!("{pdu} 必须是唯一的 8 字节 N-PDU"));
        }
        if child_text(pdus[0], "HAS-DYNAMIC-LENGTH").is_some() {
            return Err(format!("{pdu} 不适用 HAS-DYNAMIC-LENGTH"));
        }
        let global = format!(
            "/{project}/EcuCCfg/EcucConfigSet/Pdus/{}",
            pdu.rsplit('/').next().unwrap()
        );
        let configured = nodes.iter().any(|node| {
            node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                && definition(*node).as_deref()
                    == Some("/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu")
                && path_of(*node) == global
        });
        if !configured {
            return Err(format!("{pdu} 缺少对应的 EcuC 全局 Pdu"));
        }
        let def = if tx {
            "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfTxPduCfg"
        } else {
            "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfRxPduCfg"
        };
        let ref_name = if tx { "CanIfTxPduRef" } else { "CanIfRxPduRef" };
        let canif: Vec<_> = nodes
            .iter()
            .copied()
            .filter(|n| {
                n.tag_name().name() == "ECUC-CONTAINER-VALUE"
                    && definition(*n).as_deref() == Some(def)
                    && ref_value(*n, ref_name).as_deref() == Some(global.as_str())
            })
            .collect();
        if canif.len() != 1 {
            return Err(format!("{pdu} 缺少唯一且方向正确的 CanIf 全局 Pdu 映射"));
        }
        let field = if tx {
            "CanIfTxPduCanId"
        } else {
            "CanIfRxPduCanId"
        };
        if tx && param(canif[0], "CanIfTxPduCanIdType").as_deref() != Some("STANDARD_CAN")
            || !tx
                && (param(canif[0], "CanIfRxPduDataLength").as_deref() != Some("8")
                    || param(canif[0], "CanIfRxPduCanIdType").as_deref()
                        != Some("STANDARD_NO_FD_CAN")
                    || param(canif[0], "CanIfRxPduDataLengthCheck").as_deref() != Some("false"))
        {
            return Err(format!("{pdu} 须配置不检查变长 DLC 的经典 11 位 CAN"));
        }
        let expected_params: &[&str] = if tx {
            &[
                "CanIfTxPduCanId",
                "CanIfTxPduId",
                "CanIfTxPduCanIdType",
                "CanIfTxPduReadNotifyStatus",
                "CanIfTxPduTruncation",
                "CanIfTxPduType",
            ]
        } else {
            &[
                "CanIfRxPduCanId",
                "CanIfRxPduId",
                "CanIfRxPduDataLength",
                "CanIfRxPduCanIdType",
                "CanIfRxPduDataLengthCheck",
                "CanIfRxPduReadData",
                "CanIfRxPduReadNotifyStatus",
            ]
        };
        let parameter_values: Vec<_> = canif[0]
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "PARAMETER-VALUES")
            .flat_map(|group| group.children().filter(|n| n.is_element()))
            .collect();
        let reference_values: Vec<_> = canif[0]
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "REFERENCE-VALUES")
            .flat_map(|group| group.children().filter(|n| n.is_element()))
            .collect();
        let expected_refs = [
            ref_name,
            if tx {
                "CanIfTxPduBufferRef"
            } else {
                "CanIfRxPduHrhIdRef"
            },
        ];
        if parameter_values.len() != expected_params.len()
            || expected_params.iter().any(|name| {
                parameter_values
                    .iter()
                    .filter(|n| {
                        definition(**n).as_deref() == Some(format!("{def}/{name}").as_str())
                    })
                    .count()
                    != 1
            })
            || reference_values.len() != 2
            || expected_refs.iter().any(|name| {
                reference_values
                    .iter()
                    .filter(|n| {
                        definition(**n).as_deref() == Some(format!("{def}/{name}").as_str())
                    })
                    .count()
                    != 1
            })
            || reference_values.iter().any(|value| {
                value
                    .children()
                    .find(|n| n.is_element() && n.tag_name().name() == "VALUE-REF")
                    .and_then(|n| n.attribute("DEST"))
                    != Some("ECUC-CONTAINER-VALUE")
            })
            || param(canif[0], if tx { "CanIfTxPduId" } else { "CanIfRxPduId" }).as_deref()
                != Some(frames.len().to_string().as_str())
        {
            return Err(format!(
                "{pdu} 的诊断 CanIf 参数或引用不属于受支持的静态配置"
            ));
        }
        let wrong_direction = nodes.iter().any(|n| {
            n.tag_name().name() == "ECUC-CONTAINER-VALUE"
                && definition(*n).as_deref()
                    == Some(if tx {
                        "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfRxPduCfg"
                    } else {
                        "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfTxPduCfg"
                    })
                && ref_value(*n, if tx { "CanIfRxPduRef" } else { "CanIfTxPduRef" }).as_deref()
                    == Some(global.as_str())
        });
        if wrong_direction {
            return Err(format!("{pdu} 同时配置了反向 CanIf 映射"));
        }
        ids.push(parse_u32(param(canif[0], field), field, pdu).map_err(|e| e.message)?);
    }
    let data_def = "/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsp/DcmDspData";
    let did_signal_def = "/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsp/DcmDspDid/DcmDspDidSignal";
    let mut bindings = Vec::new();
    for item in did_node
        .descendants()
        .filter(|n| n.is_element() && definition(*n).as_deref() == Some(did_signal_def))
    {
        let offset = parse_u32(
            param(item, "DcmDspDidByteOffset"),
            "DcmDspDidByteOffset",
            &path_of(item),
        )
        .map_err(|e| e.message)?;
        let data_ref = ref_value(item, "DcmDspDidDataRef").ok_or("诊断 DID 信号缺少数据引用")?;
        let data: Vec<_> = nodes
            .iter()
            .copied()
            .filter(|n| {
                n.tag_name().name() == "ECUC-CONTAINER-VALUE"
                    && definition(*n).as_deref() == Some(data_def)
                    && path_of(*n) == data_ref
            })
            .collect();
        if data.len() != 1 {
            return Err(format!("{data_ref} 诊断数据引用未唯一解析"));
        }
        let metadata: Vec<_> = data[0]
            .descendants()
            .filter(|n| {
                n.is_element()
                    && n.tag_name().name() == "SDG"
                    && n.attribute("GID") == Some("AutosarWorkbenchDiagnostic")
            })
            .collect();
        if metadata.len() != 1 {
            return Err(format!("{data_ref} 必须有唯一的工具域绑定"));
        }
        let refs: Vec<_> = metadata[0].children().filter(|n| n.is_element()).collect();
        if refs.len() != 1
            || refs[0].tag_name().name() != "SD"
            || refs[0].attribute("GID") != Some("ComSignalRef")
            || refs[0].text().is_none_or(str::is_empty)
        {
            return Err(format!("{data_ref} 缺少唯一的工具域 ComSignalRef 绑定"));
        }
        bindings.push((offset, refs[0].text().unwrap().to_owned()));
    }
    bindings.sort_by_key(|(offset, _)| *offset);
    if bindings.is_empty()
        || bindings
            .iter()
            .enumerate()
            .any(|(i, (offset, _))| *offset != i as u32 * 4)
    {
        return Err("诊断 DID 数据字节偏移必须是从零开始的连续 32 位信号".into());
    }
    let dem_dtc_def = "/AUTOSAR/EcucDefs/Dem/DemConfigSet/DemDTC";
    let dem_event_def = "/AUTOSAR/EcucDefs/Dem/DemConfigSet/DemEventParameter";
    let dtc_nodes: Vec<_> = nodes
        .iter()
        .copied()
        .filter(|n| {
            n.tag_name().name() == "ECUC-CONTAINER-VALUE"
                && definition(*n).as_deref() == Some(dem_dtc_def)
        })
        .collect();
    let dtc = if dtc_nodes.is_empty() {
        None
    } else {
        if dtc_nodes.len() != 1 {
            return Err("仅支持一个 Dem UDS DTC".into());
        }
        let node = dtc_nodes[0];
        let code = parse_u32(param(node, "DemDtcValue"), "DemDtcValue", &path_of(node))
            .map_err(|e| e.message)?;
        let event_nodes: Vec<_> = nodes
            .iter()
            .copied()
            .filter(|n| {
                n.tag_name().name() == "ECUC-CONTAINER-VALUE"
                    && definition(*n).as_deref() == Some(dem_event_def)
            })
            .collect();
        if event_nodes.len() != 1
            || ref_value(event_nodes[0], "DemDTCRef").as_deref() != Some(path_of(node).as_str())
        {
            return Err("Dem 事件须唯一地关联 UDS DTC".into());
        }
        let event = event_nodes[0];
        let metadata: Vec<_> = event
            .descendants()
            .filter(|n| {
                n.is_element()
                    && n.tag_name().name() == "SDG"
                    && n.attribute("GID") == Some("AutosarWorkbenchDtc")
            })
            .collect();
        if metadata.len() != 1 {
            return Err("Dem 事件缺少唯一的工具域 Rx 帧绑定".into());
        }
        let refs: Vec<_> = metadata[0].children().filter(|n| n.is_element()).collect();
        if refs.len() != 1
            || refs[0].tag_name().name() != "SD"
            || refs[0].attribute("GID") != Some("MonitorFrameRef")
        {
            return Err("Dem 事件须有唯一 MonitorFrameRef 工具域绑定".into());
        }
        let monitor_frame_path = refs[0]
            .text()
            .filter(|s| !s.is_empty())
            .ok_or("Dem 事件缺少监控 Rx 帧绝对路径")?
            .to_owned();
        Some(DtcView {
            path: path_of(node),
            code,
            monitor_frame_path,
        })
    };
    if dtc.is_none()
        && nodes.iter().any(|n| {
            n.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
                && matches!(
                    definition(*n).as_deref(),
                    Some("/AUTOSAR/EcucDefs/Dem" | "/AUTOSAR/EcucDefs/NvM")
                )
        })
    {
        return Err("Dem/NvM 配置存在但缺少受支持的单个 UDS DTC".into());
    }
    let reset_routine_id = parse_host_routine(did_node, project, &nodes)?;
    let security_enabled = nodes.iter().any(|node| {
        node.tag_name().name() == "ECUC-CONTAINER-VALUE"
            && definition(*node).as_deref()
                == Some(
                    "/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsp/DcmDspSecurity/DcmDspSecurityRow",
                )
    });
    let diagnostic = DiagnosticView {
        path: path_of(did_node),
        request_id: ids[0],
        response_id: ids[1],
        s3_ms,
        n_as_ms,
        n_bs_ms,
        n_cr_ms,
        did,
        signal_paths: bindings.into_iter().map(|(_, path)| path).collect(),
        dtc,
        reset_routine_id,
        security_enabled,
        write_enabled: nodes.iter().any(|node| {
            node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                && definition(*node).as_deref()
                    == Some(
                        "/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsp/DcmDspDidInfo/DcmDspDidWrite",
                    )
        }),
    };
    if let Some(issue) = validate_diagnostic(&diagnostic, frames, signals).first() {
        return Err(format!("{}: {}", issue.code, issue.message));
    }
    let expected = render_profile(project, frames, signals, Some(&diagnostic));
    let expected_doc = Document::parse(&expected).map_err(|e| e.to_string())?;
    let expected_shape = diagnostic_shape(&[expected_doc])?;
    if actual != expected_shape {
        return Err(
            "CanTp/Dcm/Dem/NvM 配置含未知、不一致或不受支持的参数、引用、方向、会话或变体".into(),
        );
    }
    Ok(Some(diagnostic))
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

    pub fn view(&self) -> WorkspaceView {
        WorkspaceView {
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

    fn is_managed_file(&self, file: &SourceFile) -> bool {
        file.text
            == render_profile(
                &self.name,
                &self.frames,
                &self.signals,
                self.diagnostic.as_ref(),
            )
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
        let mut global_pdus = BTreeMap::new();
        let mut system_pdu_lengths = BTreeMap::new();
        let own_pdu_prefix = format!("/{}/EcuCCfg/EcucConfigSet/Pdus/", self.name);
        let own_com_module_path = format!("/{}/ComCfg", self.name);
        let own_com_config_path = format!("{own_com_module_path}/ComConfig");
        let mut consumed_com_modules = 0usize;
        let mut com_owner_issues = Vec::new();
        for file in &self.files {
            let doc =
                Document::parse(&file.text).map_err(|e| format!("{}: {e}", file.path.display()))?;
            for node in doc
                .descendants()
                .filter(|n| n.is_element() && n.tag_name().namespace() == Some(NS))
            {
                if child_text(node, "SHORT-NAME").is_some() {
                    let path = path_of(node);
                    if let Some(previous) =
                        paths.insert(path.clone(), node.tag_name().name().to_owned())
                        && (previous != "AR-PACKAGE" || node.tag_name().name() != "AR-PACKAGE")
                    {
                        issues.push(Issue::error(
                            "DUPLICATE_PATH",
                            "ARXML 绝对路径重复",
                            Some(path),
                        ));
                    }
                }
                if node.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES" {
                    let path = path_of(node);
                    let consumed = node.descendants().any(|child| {
                        child.is_element()
                            && child.tag_name().name() == "ECUC-CONTAINER-VALUE"
                            && matches!(
                                definition(child).as_deref(),
                                Some(
                                    "/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu"
                                        | "/AUTOSAR/EcucDefs/Com/ComConfig/ComSignal"
                                )
                            )
                    });
                    if consumed {
                        consumed_com_modules += 1;
                        if consumed_com_modules > 1 {
                            com_owner_issues.push(Issue {
                                file: Some(file.path.display().to_string()),
                                ..Issue::error(
                                    "PDU_UNSUPPORTED",
                                    "消耗的 Com 配置必须有唯一的模块所有者",
                                    Some(path.clone()),
                                )
                            });
                        }
                    }
                    if (consumed || path == own_com_module_path)
                        && (path != own_com_module_path
                            || !module_definition(node, "/AUTOSAR/EcucDefs/Com")
                            || !tool_com_root(node))
                    {
                        com_owner_issues.push(Issue {
                            file: Some(file.path.display().to_string()),
                            ..Issue::error(
                                "PDU_UNSUPPORTED",
                                "Com 模块所有者、ComConfig/ComGeneral 必需容器或参数不受支持",
                                Some(path),
                            )
                        });
                    }
                }
                if node.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
                    && path_of(node) == format!("/{}/EcuCCfg", self.name)
                    && (!module_definition(node, "/AUTOSAR/EcucDefs/EcuC") || !tool_ecuc_root(node))
                {
                    issues.push(Issue {
                        file: Some(file.path.display().to_string()),
                        ..Issue::error(
                            "PDU_UNSUPPORTED",
                            "EcuCCfg 根定义、虚拟核心、Pdu 集合或必需类型参数不完整",
                            Some(path_of(node)),
                        )
                    });
                }
                if node.tag_name().name() == "I-SIGNAL-I-PDU" {
                    let mappings = node
                        .descendants()
                        .filter(|n| {
                            n.is_element() && n.tag_name().name() == "I-SIGNAL-TO-I-PDU-MAPPING"
                        })
                        .map(|mapping| SignalMapping {
                            signal_ref: child_text(mapping, "I-SIGNAL-REF"),
                            start: child_text(mapping, "START-POSITION"),
                            byte_order: child_text(mapping, "PACKING-BYTE-ORDER"),
                        })
                        .collect();
                    pdus.insert(
                        path_of(node),
                        PduInfo {
                            length: parse_u32(child_text(node, "LENGTH"), "LENGTH", &path_of(node)),
                            mappings,
                        },
                    );
                }
                if matches!(node.tag_name().name(), "N-PDU" | "DCM-I-PDU") {
                    system_pdu_lengths.insert(
                        path_of(node),
                        child_text(node, "LENGTH").and_then(|value| value.parse::<u32>().ok()),
                    );
                }
                if node.tag_name().name() == "I-SIGNAL" {
                    signal_lengths.insert(
                        path_of(node),
                        parse_u32(child_text(node, "LENGTH"), "LENGTH", &path_of(node)),
                    );
                }
                if node.tag_name().name() == "CAN-FRAME" {
                    let mappings = node
                        .descendants()
                        .filter(|n| n.is_element() && n.tag_name().name() == "PDU-TO-FRAME-MAPPING")
                        .map(|mapping| FrameMapping {
                            pdu_ref: child_text(mapping, "PDU-REF"),
                            start: child_text(mapping, "START-POSITION"),
                            byte_order: child_text(mapping, "PACKING-BYTE-ORDER"),
                        })
                        .collect();
                    network_frames.insert(
                        path_of(node),
                        CanFrameInfo {
                            length: child_text(node, "FRAME-LENGTH"),
                            mappings,
                        },
                    );
                }
                if node.tag_name().name() == "CAN-FRAME-TRIGGERING" {
                    triggers.push((
                        child_text(node, "FRAME-REF"),
                        child_text(node, "IDENTIFIER"),
                        path_of(node),
                    ));
                }
                if node.tag_name().name() == "ECUC-CONTAINER-VALUE" {
                    let container_definition = definition(node);
                    if matches!(
                        container_definition.as_deref(),
                        Some(
                            "/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu"
                                | "/AUTOSAR/EcucDefs/Com/ComConfig/ComSignal"
                        )
                    ) && !tool_com_parent(node, &own_com_module_path, &own_com_config_path)
                    {
                        com_owner_issues.push(Issue {
                            file: Some(file.path.display().to_string()),
                            ..Issue::error(
                                "PDU_UNSUPPORTED",
                                "Com 容器必须直接属于唯一受支持的 ComCfg/ComConfig",
                                Some(path_of(node)),
                            )
                        });
                    }
                    match container_definition.as_deref() {
                        Some("/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu")
                            if path_of(node).starts_with(&own_pdu_prefix) =>
                        {
                            let path = path_of(node);
                            match tool_global_pdu(node, file) {
                                Ok(binding) => {
                                    if global_pdus.insert(path.clone(), binding).is_some() {
                                        issues.push(Issue {
                                            file: Some(file.path.display().to_string()),
                                            ..Issue::error(
                                                "PDU_UNSUPPORTED",
                                                "全局 PDU 路径重复",
                                                Some(path),
                                            )
                                        });
                                    }
                                }
                                Err(message) => issues.push(Issue {
                                    file: Some(file.path.display().to_string()),
                                    ..Issue::error("PDU_UNSUPPORTED", message, Some(path))
                                }),
                            }
                        }
                        Some("/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu") => {
                            com.push(ComPduRecord {
                                path: path_of(node),
                                target: ref_value(node, "ComPduIdRef"),
                                dest: ref_dest(node, "ComPduIdRef"),
                                direction: param(node, "ComIPduDirection"),
                                period: param(node, "ComTxModeTimePeriod"),
                                processing: param(node, "ComIPduSignalProcessing"),
                                kind: param(node, "ComIPduType"),
                                unused: param(node, "ComTxIPduUnusedAreasDefault"),
                                signals: node
                                    .descendants()
                                    .filter(|n| {
                                        n.is_element()
                                            && definition(*n)
                                                .is_some_and(|d| d.ends_with("/ComIPduSignalRef"))
                                    })
                                    .filter_map(|n| child_text(n, "VALUE-REF"))
                                    .collect(),
                                file: file.path.display().to_string(),
                            });
                        }
                        Some("/AUTOSAR/EcucDefs/Com/ComConfig/ComSignal") => {
                            signal_nodes.insert(
                                path_of(node),
                                (
                                    param(node, "ComBitPosition"),
                                    param(node, "ComBitSize"),
                                    param(node, "ComSignalInitValue"),
                                    param(node, "ComSignalEndianness"),
                                    param(node, "ComSignalType"),
                                    param(node, "ComTimeout"),
                                ),
                            );
                        }
                        Some("/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfTxPduCfg") => {
                            if let Some(pdu) = ref_value(node, "CanIfTxPduRef")
                                && canif
                                    .insert(
                                        pdu.clone(),
                                        CanIfPduRecord {
                                            can_id: param(node, "CanIfTxPduCanId"),
                                            dlc: None,
                                            id_type: param(node, "CanIfTxPduCanIdType"),
                                            tx: true,
                                            dest: ref_dest(node, "CanIfTxPduRef"),
                                            path: path_of(node),
                                            file: file.path.display().to_string(),
                                        },
                                    )
                                    .is_some()
                            {
                                issues.push(Issue::error(
                                    "CANIF_PDU_DUPLICATE",
                                    "同一 I-PDU 有多个 CanIf 映射",
                                    Some(pdu),
                                ));
                            }
                        }
                        Some("/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfRxPduCfg") => {
                            if let Some(pdu) = ref_value(node, "CanIfRxPduRef")
                                && canif
                                    .insert(
                                        pdu.clone(),
                                        CanIfPduRecord {
                                            can_id: param(node, "CanIfRxPduCanId"),
                                            dlc: param(node, "CanIfRxPduDataLength"),
                                            id_type: param(node, "CanIfRxPduCanIdType"),
                                            tx: false,
                                            dest: ref_dest(node, "CanIfRxPduRef"),
                                            path: path_of(node),
                                            file: file.path.display().to_string(),
                                        },
                                    )
                                    .is_some()
                            {
                                issues.push(Issue::error(
                                    "CANIF_PDU_DUPLICATE",
                                    "同一 I-PDU 有多个 CanIf 映射",
                                    Some(pdu),
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        for (target, pdu) in &canif {
            if target.starts_with(&own_pdu_prefix)
                && (!global_pdus.contains_key(target)
                    || pdu.dest.as_deref() != Some("ECUC-CONTAINER-VALUE"))
            {
                issues.push(Issue {
                    file: Some(pdu.file.clone()),
                    ..Issue::error(
                        "PDU_UNSUPPORTED",
                        format!("CanIf 引用没有正确解析到 EcuC Pdu: {target}"),
                        Some(pdu.path.clone()),
                    )
                });
            }
        }
        let mut bound_system_pdus = BTreeSet::new();
        for (path, binding) in &global_pdus {
            let length = pdus
                .get(&binding.system_path)
                .and_then(|system| system.length.as_ref().ok().copied())
                .or_else(|| {
                    system_pdu_lengths
                        .get(&binding.system_path)
                        .copied()
                        .flatten()
                });
            if !bound_system_pdus.insert(binding.system_path.as_str())
                || length != Some(binding.length)
            {
                issues.push(Issue {
                    file: Some(binding.file.clone()),
                    ..Issue::error(
                        "PDU_UNSUPPORTED",
                        format!(
                            "全局 PDU 与系统 PDU 必须一一对应且长度一致: {}",
                            binding.system_path
                        ),
                        Some(path.clone()),
                    )
                });
            }
        }
        let mut frames = Vec::new();
        let mut signals = Vec::new();
        for record in com {
            let path = record.path;
            let Some(pdu_ref) = record.target else {
                issues.push(Issue::error(
                    "COM_PDU_REF",
                    "ComIPdu 缺少 PDU 引用",
                    Some(path),
                ));
                continue;
            };
            let system_path = if let Some(binding) = global_pdus.get(&pdu_ref) {
                if record.dest.as_deref() != Some("ECUC-CONTAINER-VALUE")
                    || record.processing.as_deref() != Some("IMMEDIATE")
                    || record.kind.as_deref() != Some("NORMAL")
                    || record.period.is_some() && record.unused.as_deref() != Some("0")
                {
                    issues.push(Issue {
                        file: Some(record.file.clone()),
                        ..Issue::error(
                            "PDU_UNSUPPORTED",
                            "Com PDU 引用 DEST 或必需参数不属于受支持的全局 PDU 配置",
                            Some(path.clone()),
                        )
                    });
                }
                binding.system_path.as_str()
            } else {
                issues.push(Issue {
                    file: Some(record.file),
                    ..Issue::error(
                        "PDU_UNSUPPORTED",
                        format!("ComPduIdRef 必须指向绑定的 EcuC 全局 Pdu: {pdu_ref}"),
                        Some(path),
                    )
                });
                continue;
            };
            let Some(pdu) = pdus.get(system_path) else {
                issues.push(Issue::error(
                    "PDU_UNSUPPORTED",
                    format!("全局 PDU 未绑定可解析的系统 I-PDU: {system_path}"),
                    Some(pdu_ref),
                ));
                continue;
            };
            let Some(canif_pdu) = canif.get(&pdu_ref) else {
                issues.push(Issue::error(
                    "CANIF_PDU_REF",
                    "CanIf 没有引用同一个 PDU",
                    Some(pdu_ref),
                ));
                continue;
            };
            if canif_pdu.dest.as_deref() != Some("ECUC-CONTAINER-VALUE") {
                issues.push(Issue::error(
                    "PDU_UNSUPPORTED",
                    "CanIf PDU 引用 DEST 必须指向 EcuC Pdu 容器",
                    Some(pdu_ref.clone()),
                ));
            }
            let direction = match record.direction.as_deref() {
                Some("SEND") if canif_pdu.tx => Direction::Tx,
                Some("RECEIVE") if !canif_pdu.tx => Direction::Rx,
                _ => {
                    issues.push(Issue::error(
                        "PDU_DIRECTION",
                        "Com 与 CanIf 方向不一致或未知",
                        Some(pdu_ref),
                    ));
                    continue;
                }
            };
            let expected_id_type = if canif_pdu.tx {
                "STANDARD_CAN"
            } else {
                "STANDARD_NO_FD_CAN"
            };
            if canif_pdu.id_type.as_deref() != Some(expected_id_type) {
                issues.push(Issue::error(
                    "CAN_ID_TYPE",
                    "仅支持经典 11 位 CAN",
                    Some(pdu_ref.clone()),
                ));
            }
            let Ok(dlc) = pdu.length.as_ref() else {
                issues.push(pdu.length.as_ref().unwrap_err().clone());
                continue;
            };
            if global_pdus
                .get(&pdu_ref)
                .is_some_and(|binding| binding.length != *dlc)
            {
                issues.push(Issue::error(
                    "PDU_UNSUPPORTED",
                    "全局 PDU 长度与系统 I-PDU 不一致",
                    Some(pdu_ref.clone()),
                ));
            }
            let id = match parse_u32(canif_pdu.can_id.clone(), "CanIfPduCanId", &pdu_ref) {
                Ok(id) => id,
                Err(issue) => {
                    issues.push(issue);
                    continue;
                }
            };
            let period_ms = if matches!(direction, Direction::Tx) {
                match parse_milliseconds(record.period, "ComTxModeTimePeriod", &pdu_ref) {
                    Ok(value) => Some(value),
                    Err(issue) => {
                        issues.push(issue);
                        None
                    }
                }
            } else {
                None
            };
            let mut timeout_ms = None;
            let mut com_layout = Vec::new();
            for signal_ref in record.signals {
                let Some((start, length, initial, endian, ty, timeout)) =
                    signal_nodes.get(&signal_ref)
                else {
                    issues.push(Issue::error(
                        "COM_SIGNAL_REF",
                        "ComIPduSignalRef 未解析",
                        Some(signal_ref),
                    ));
                    continue;
                };
                if endian.as_deref() != Some("LITTLE_ENDIAN")
                    || !ty
                        .as_deref()
                        .is_some_and(|t| matches!(t, "BOOLEAN" | "UINT8" | "UINT16" | "UINT32"))
                {
                    issues.push(Issue::error(
                        "SIGNAL_VARIANT",
                        "仅支持小端无符号标量信号",
                        Some(signal_ref.clone()),
                    ));
                }
                let start = match parse_u32(start.clone(), "ComBitPosition", &signal_ref) {
                    Ok(v) => v,
                    Err(e) => {
                        issues.push(e);
                        continue;
                    }
                };
                let length = match parse_u32(length.clone(), "ComBitSize", &signal_ref) {
                    Ok(v) => v,
                    Err(e) => {
                        issues.push(e);
                        continue;
                    }
                };
                let initial = match parse_u32(initial.clone(), "ComSignalInitValue", &signal_ref) {
                    Ok(v) => v,
                    Err(e) => {
                        issues.push(e);
                        continue;
                    }
                };
                if start > u8::MAX as u32 || length > u8::MAX as u32 {
                    issues.push(Issue::error(
                        "SIGNAL_RANGE",
                        "信号位位置或长度超出范围",
                        Some(signal_ref),
                    ));
                    continue;
                }
                com_layout.push((start, length));
                if matches!(direction, Direction::Rx) {
                    match parse_milliseconds(timeout.clone(), "ComTimeout", &signal_ref) {
                        Ok(value) if timeout_ms.is_none() || timeout_ms == Some(value) => {
                            timeout_ms = Some(value)
                        }
                        Ok(_) => issues.push(Issue::error(
                            "RX_TIMEOUT_MISMATCH",
                            "同帧接收信号超时须一致",
                            Some(signal_ref.clone()),
                        )),
                        Err(e) => issues.push(e),
                    }
                }
                let name = signal_ref.rsplit('/').next().unwrap_or("").to_owned();
                signals.push(SignalView {
                    path: signal_ref,
                    name,
                    frame_path: system_path.to_owned(),
                    start_bit: start as u8,
                    length: length as u8,
                    initial_value: initial,
                });
            }
            if !pdu.mappings.is_empty() {
                let mut mapped_layout = Vec::with_capacity(pdu.mappings.len());
                let supported = pdu.mappings.iter().all(|mapping| {
                    if mapping.byte_order.as_deref() != Some("MOST-SIGNIFICANT-BYTE-LAST") {
                        return false;
                    }
                    let Some(start) = mapping
                        .start
                        .as_deref()
                        .and_then(|value| value.parse::<u32>().ok())
                    else {
                        return false;
                    };
                    let Some(length) = mapping
                        .signal_ref
                        .as_deref()
                        .and_then(|path| signal_lengths.get(path))
                        .and_then(|value| value.as_ref().ok())
                    else {
                        return false;
                    };
                    mapped_layout.push((start, *length));
                    true
                });
                mapped_layout.sort_unstable();
                com_layout.sort_unstable();
                if !supported || mapped_layout != com_layout {
                    issues.push(Issue::error(
                        "PDU_MAPPING",
                        "I-PDU 信号映射的位序或位段与 Com 配置不一致",
                        Some(system_path.to_owned()),
                    ));
                }
            }
            for (frame_ref, network_id, trigger_path) in &triggers {
                let Some(frame) = frame_ref
                    .as_deref()
                    .and_then(|path| network_frames.get(path))
                else {
                    continue;
                };
                if let Some(mapping) = frame
                    .mappings
                    .iter()
                    .find(|mapping| mapping.pdu_ref.as_deref() == Some(system_path))
                {
                    if frame.mappings.len() != 1
                        || frame
                            .length
                            .as_deref()
                            .and_then(|value| value.parse::<u32>().ok())
                            != Some(*dlc)
                        || mapping.start.as_deref() != Some("0")
                        || mapping.byte_order.as_deref() != Some("MOST-SIGNIFICANT-BYTE-LAST")
                    {
                        issues.push(Issue::error(
                            "CAN_FRAME_MAPPING",
                            "网络帧须以小端、零偏移完整承载该 I-PDU",
                            Some(trigger_path.clone()),
                        ));
                    }
                    if network_id
                        .as_deref()
                        .and_then(|value| value.parse::<u32>().ok())
                        != Some(id)
                    {
                        issues.push(Issue::error(
                            "CAN_ID_MISMATCH",
                            "CAN-FRAME-TRIGGERING 与 CanIf 的 CAN 标识符不一致",
                            Some(trigger_path.clone()),
                        ));
                    }
                }
            }
            if *dlc > u8::MAX as u32
                || canif_pdu
                    .dlc
                    .as_ref()
                    .is_some_and(|value| value.parse::<u32>().ok() != Some(*dlc))
            {
                issues.push(Issue::error(
                    "PDU_DLC",
                    "I-PDU 与 CanIf 的数据长度不一致",
                    Some(system_path.to_owned()),
                ));
            }
            frames.push(FrameView {
                name: system_path
                    .rsplit('/')
                    .next()
                    .unwrap_or("")
                    .trim_start_matches("Pdu_")
                    .into(),
                path: system_path.to_owned(),
                id,
                dlc: (*dlc).min(u8::MAX as u32) as u8,
                direction,
                period_ms,
                timeout_ms,
            });
        }
        issues.extend(com_owner_issues);
        frames.sort_by(|a, b| a.path.cmp(&b.path));
        signals.sort_by(|a, b| a.path.cmp(&b.path));
        issues.extend(validate_profile(&frames, &signals));
        let diagnostic = match parse_diagnostic(&self.files, &self.name, &frames, &signals) {
            Ok(diagnostic) => diagnostic,
            Err(message) => {
                issues.push(Issue::error("DIAG_UNSUPPORTED", message, None));
                None
            }
        };
        issues.extend(validate_host_can_ecuc(
            &self.files,
            &self.name,
            &frames,
            &signals,
            diagnostic.as_ref(),
        ));
        if !global_pdus.is_empty() {
            let mut expected: BTreeSet<_> = frames
                .iter()
                .map(|frame| {
                    format!(
                        "/{}/EcuCCfg/EcucConfigSet/Pdus/Pdu_{}",
                        self.name, frame.name
                    )
                })
                .collect();
            if diagnostic.is_some() {
                for name in [
                    "NPdu_DiagRequest",
                    "NPdu_DiagResponse",
                    "DcmPdu_DiagRequest",
                    "DcmPdu_DiagResponse",
                ] {
                    let path = format!("/{}/EcuCCfg/EcucConfigSet/Pdus/{name}", self.name);
                    expected.insert(path.clone());
                    if global_pdus.get(&path).is_none_or(|binding| {
                        binding.system_path != format!("/{}/{name}", self.name)
                    }) {
                        issues.push(Issue {
                            file: self
                                .files
                                .first()
                                .map(|file| file.path.display().to_string()),
                            ..Issue::error(
                                "PDU_UNSUPPORTED",
                                format!("诊断全局 PDU {path} 缺失或未绑定对应系统 PDU"),
                                Some(path),
                            )
                        });
                    }
                }
            }
            for (path, binding) in &global_pdus {
                if !expected.contains(path) {
                    issues.push(Issue {
                        file: Some(binding.file.clone()),
                        ..Issue::error(
                            "PDU_UNSUPPORTED",
                            "全局 PDU 没有唯一受支持的 Com/诊断用途",
                            Some(path.clone()),
                        )
                    });
                }
            }
        }
        self.frames = frames;
        self.signals = signals;
        self.diagnostic = diagnostic;
        self.issues = issues;
        Ok(())
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

    fn commit_patches(
        &mut self,
        mut patches: Vec<Vec<Patch>>,
        matches: impl FnOnce(&Workspace) -> bool,
    ) -> Result<(), String> {
        let mut previous = Vec::new();
        for (index, edits) in patches.iter_mut().enumerate() {
            if edits.is_empty() {
                continue;
            }
            previous.push((index, self.files[index].text.clone()));
            if let Err(error) = apply_patches(&mut self.files[index].text, edits) {
                for (i, text) in previous {
                    self.files[i].text = text;
                }
                return Err(error);
            }
        }
        let checked = self.refresh().and_then(|_| {
            if let Some(issue) = self
                .issues
                .iter()
                .find(|i| matches!(i.severity, Severity::Error))
            {
                Err(format!("{}: {}", issue.code, issue.message))
            } else if matches(self) {
                Ok(())
            } else {
                Err("编辑后的 ARXML 与配置模型不一致".into())
            }
        });
        if let Err(error) = checked {
            for (i, text) in previous {
                self.files[i].text = text;
            }
            let _ = self.refresh();
            return Err(error);
        }
        Ok(())
    }

    fn global_pdu_for(&self, system_path: &str) -> Result<String, String> {
        let prefix = format!("/{}/EcuCCfg/", self.name);
        let mut found = None;
        for file in &self.files {
            let doc = Document::parse(&file.text).map_err(|error| error.to_string())?;
            for node in doc.descendants().filter(|node| {
                node.is_element()
                    && node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                    && definition(*node).as_deref()
                        == Some("/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu")
            }) {
                if !path_of(node).starts_with(&prefix) {
                    continue;
                }
                let binding = tool_global_pdu(node, file)?;
                if binding.system_path == system_path && found.replace(path_of(node)).is_some() {
                    return Err(format!("{system_path} 有多个全局 PDU 绑定"));
                }
            }
        }
        found.ok_or_else(|| format!("{system_path} 缺少可定位的全局 PDU 绑定"))
    }

    fn patch_imported_frame(&mut self, old: &FrameView, new: &FrameView) -> Result<(), String> {
        let global_pdu = self.global_pdu_for(&old.path)?;
        let mut patches: Vec<Vec<Patch>> = (0..self.files.len()).map(|_| Vec::new()).collect();
        let mut found_id = false;
        let mut found_dlc = false;
        let mut found_global_length = false;
        let mut found_period = false;
        let mut found_timeout = 0usize;
        let receive_signals: BTreeSet<_> = self
            .signals
            .iter()
            .filter(|s| s.frame_path == old.path)
            .map(|s| s.path.as_str())
            .collect();
        for (index, file) in self.files.iter().enumerate() {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element()) {
                if old.dlc != new.dlc
                    && node.tag_name().name() == "I-SIGNAL-I-PDU"
                    && path_of(node) == old.path
                {
                    patch_child(node, "LENGTH", new.dlc.to_string(), &mut patches[index])?;
                    found_dlc = true;
                }
                if old.dlc != new.dlc
                    && node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                    && path_of(node) == global_pdu
                {
                    patch_param(node, "PduLength", new.dlc.to_string(), &mut patches[index])?;
                    found_global_length = true;
                }
                if node.tag_name().name() == "ECUC-CONTAINER-VALUE" {
                    let def = definition(node).unwrap_or_default();
                    if def.ends_with("/CanIfTxPduCfg")
                        && ref_value(node, "CanIfTxPduRef").as_deref() == Some(&global_pdu)
                        && old.id != new.id
                    {
                        patch_param(
                            node,
                            "CanIfTxPduCanId",
                            new.id.to_string(),
                            &mut patches[index],
                        )?;
                        found_id = true;
                    }
                    if def.ends_with("/CanIfRxPduCfg")
                        && ref_value(node, "CanIfRxPduRef").as_deref() == Some(&global_pdu)
                    {
                        if old.id != new.id {
                            patch_param(
                                node,
                                "CanIfRxPduCanId",
                                new.id.to_string(),
                                &mut patches[index],
                            )?;
                            found_id = true;
                        }
                        if old.dlc != new.dlc {
                            patch_param(
                                node,
                                "CanIfRxPduDataLength",
                                new.dlc.to_string(),
                                &mut patches[index],
                            )?;
                        }
                    }
                    if old.period_ms != new.period_ms
                        && def.ends_with("/ComIPdu")
                        && ref_value(node, "ComPduIdRef").as_deref() == Some(&global_pdu)
                    {
                        patch_param(
                            node,
                            "ComTxModeTimePeriod",
                            seconds(new.period_ms.ok_or("发送周期不可为空")?),
                            &mut patches[index],
                        )?;
                        found_period = true;
                    }
                    if old.timeout_ms != new.timeout_ms
                        && receive_signals.contains(path_of(node).as_str())
                    {
                        patch_param(
                            node,
                            "ComTimeout",
                            seconds(new.timeout_ms.ok_or("接收超时不可为空")?),
                            &mut patches[index],
                        )?;
                        found_timeout += 1;
                    }
                }
                if old.id != new.id && node.tag_name().name() == "CAN-FRAME-TRIGGERING" {
                    return Err(
                        "导入项目包含 CAN 网络触发配置，修改标识符需同步网络模型；已阻止不安全编辑"
                            .into(),
                    );
                }
                if old.dlc != new.dlc && node.tag_name().name() == "CAN-FRAME" {
                    return Err(
                        "导入项目包含 CAN-FRAME，修改 DLC 需同步网络模型；已阻止不安全编辑".into(),
                    );
                }
            }
        }
        if (old.id != new.id && !found_id)
            || (old.dlc != new.dlc && (!found_dlc || !found_global_length))
            || (old.period_ms != new.period_ms && !found_period)
            || (old.timeout_ms != new.timeout_ms && found_timeout != receive_signals.len())
        {
            return Err("导入项目缺少可定位的标准参数，已拒绝修改且保留原文件".into());
        }
        self.commit_patches(patches, |w| {
            w.frames.iter().any(|f| {
                f.path == old.path
                    && f.id == new.id
                    && f.dlc == new.dlc
                    && f.period_ms == new.period_ms
                    && f.timeout_ms == new.timeout_ms
            })
        })?;
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
                    if old.start_bit != new.start_bit {
                        patch_param(
                            node,
                            "ComBitPosition",
                            new.start_bit.to_string(),
                            &mut patches[index],
                        )?;
                    }
                    if old.length != new.length {
                        patch_param(
                            node,
                            "ComBitSize",
                            new.length.to_string(),
                            &mut patches[index],
                        )?;
                        patch_param(
                            node,
                            "ComSignalType",
                            signal_type(new.length).into(),
                            &mut patches[index],
                        )?;
                    }
                    if old.initial_value != new.initial_value {
                        patch_param(
                            node,
                            "ComSignalInitValue",
                            new.initial_value.to_string(),
                            &mut patches[index],
                        )?;
                    }
                    found_com = true;
                }
                if layout_changed
                    && node.tag_name().name() == "I-SIGNAL-TO-I-PDU-MAPPING"
                    && child_text(node, "I-SIGNAL-REF").as_deref() == Some(&system_path)
                    && node.ancestors().any(|a| {
                        a.tag_name().name() == "I-SIGNAL-I-PDU" && path_of(a) == old.frame_path
                    })
                {
                    if old.start_bit != new.start_bit {
                        patch_child(
                            node,
                            "START-POSITION",
                            new.start_bit.to_string(),
                            &mut patches[index],
                        )?;
                    }
                    found_mapping = true;
                }
                if old.length != new.length
                    && node.tag_name().name() == "I-SIGNAL"
                    && path_of(node) == system_path
                {
                    patch_child(node, "LENGTH", new.length.to_string(), &mut patches[index])?;
                    found_system_signal = true;
                }
            }
        }
        if !found_com
            || (layout_changed && !found_mapping)
            || (old.length != new.length && !found_system_signal)
        {
            return Err("信号系统映射不完整；拒绝破坏未知引用或位布局".into());
        }
        self.commit_patches(patches, |w| {
            w.signals.iter().any(|s| {
                s.path == old.path
                    && s.start_bit == new.start_bit
                    && s.length == new.length
                    && s.initial_value == new.initial_value
            })
        })?;
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

    fn ensure_sources_current(&self) -> Result<(), String> {
        for file in &self.files {
            let disk = fs::read(&file.path)
                .map_err(|error| format!("无法读取来源文件 {}: {error}", file.path.display()))?;
            if disk != file.saved.as_bytes() {
                return Err(format!(
                    "文件已被外部修改，拒绝基于过期配置继续；请重新导入项目: {}",
                    file.path.display()
                ));
            }
        }
        Ok(())
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

    fn save_revision(&self) -> String {
        let mut digest = Sha256::new();
        for file in &self.files {
            let path = file.path.to_string_lossy();
            for bytes in [path.as_bytes(), file.saved.as_bytes(), file.text.as_bytes()] {
                digest.update((bytes.len() as u64).to_le_bytes());
                digest.update(bytes);
            }
        }
        format!("{:x}", digest.finalize())
    }

    pub fn preview_save(&mut self) -> Result<SavePreview, String> {
        self.validate()?;
        if let Some(issue) = self
            .issues
            .iter()
            .find(|issue| matches!(issue.severity, Severity::Error))
        {
            return Err(format!("{}: {}", issue.code, issue.message));
        }
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

    pub fn save_previewed(&mut self, revision: &str) -> Result<WorkspaceView, String> {
        if self.save_revision() != revision {
            return Err("配置已在预览后改变，请重新查看 ARXML 改动再保存".into());
        }
        self.save()
    }

    pub fn save(&mut self) -> Result<WorkspaceView, String> {
        self.validate()?;
        if let Some(issue) = self
            .issues
            .iter()
            .find(|i| matches!(i.severity, Severity::Error))
        {
            return Err(format!("{}: {}", issue.code, issue.message));
        }
        // Validation includes references across every imported file, including files this
        // edit leaves untouched. A stale untouched file would invalidate that result.
        self.ensure_sources_current()?;
        let dirty = self
            .files
            .iter()
            .filter(|f| f.text != f.saved)
            .collect::<Vec<_>>();
        let mut staged = Vec::new();
        for (index, file) in dirty.iter().enumerate() {
            let suffix = format!("arxml.autosar-config-{}-{index}", std::process::id());
            let stage = file.path.with_extension(format!("{suffix}.tmp"));
            let backup = file.path.with_extension(format!("{suffix}.bak"));
            let prepared = (|| -> Result<(), String> {
                if backup.exists() {
                    return Err(format!("待恢复备份已存在，拒绝覆盖: {}", backup.display()));
                }
                if fs::read_to_string(&file.path).map_err(|e| e.to_string())? != file.saved {
                    return Err(format!(
                        "文件已被外部修改，拒绝覆盖: {}",
                        file.path.display()
                    ));
                }
                let mut handle = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&stage)
                    .map_err(|e| format!("暂存文件不可创建 {}: {e}", stage.display()))?;
                let written = std::io::Write::write_all(&mut handle, file.text.as_bytes())
                    .and_then(|_| handle.sync_all());
                drop(handle);
                if written.is_err() {
                    let _ = fs::remove_file(&stage);
                }
                written.map_err(|e| format!("暂存文件写入失败 {}: {e}", stage.display()))
            })();
            if let Err(error) = prepared {
                for (_, stage, _) in &staged {
                    let _ = fs::remove_file(stage);
                }
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
                for (_, stage, _) in &staged {
                    let _ = fs::remove_file(stage);
                }
                return Err(format!(
                    "ARXML 保存失败: {error}; 回滚问题: {}",
                    rollback_errors.join("; ")
                ));
            }
        }
        let mut cleanup_error = None;
        for (file, _, backup) in &staged {
            if fs::read_to_string(backup).ok().as_deref() != Some(&file.saved) {
                cleanup_error = Some(format!(
                    "备份内容发生变化，保留备份供检查: {}",
                    backup.display()
                ));
                break;
            }
            if let Err(error) = fs::remove_file(backup) {
                cleanup_error = Some(format!(
                    "已保存 ARXML，但无法清理备份 {}: {error}",
                    backup.display()
                ));
                break;
            }
        }
        for file in &mut self.files {
            file.saved = file.text.clone();
        }
        if let Some(error) = cleanup_error {
            return Err(error);
        }
        Ok(self.view())
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

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::{SourceFile, install_staged, restore_backup};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("autosar-save-{}-{nonce}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn staged_save_preserves_external_edit_between_preparation_and_installation() {
        let root = Scratch::new();
        let original = root.0.join("Ecu.arxml");
        let stage = root.0.join("Ecu.tmp");
        let backup = root.0.join("Ecu.bak");
        let file = SourceFile {
            path: original.clone(),
            saved: "original".into(),
            text: "ours".into(),
        };
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
