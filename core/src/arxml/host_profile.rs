use super::*;

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

pub(super) fn parse_host_routine(
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
    pub(super) fn is_managed_file(&self, file: &SourceFile) -> bool {
        file.text
            == render_profile(
                &self.name,
                &self.frames,
                &self.signals,
                self.diagnostic.as_ref(),
            )
    }

    pub(super) fn refresh(&mut self) -> Result<(), String> {
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
        self.rebuild_snapshot()
    }
    pub(super) fn global_pdu_for(&self, system_path: &str) -> Result<String, String> {
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
}
