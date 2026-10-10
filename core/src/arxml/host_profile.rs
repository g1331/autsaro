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
                crate::product_message!(
                    "backend.arxml.host_profile.host_can_profile_unrenderable",
                    "error" => error.to_string()
                ),
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
                        crate::product_message!("backend.arxml.host_profile.extra_host_can_module"),
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
                        crate::product_message!(
                            "backend.arxml.host_profile.duplicate_host_can_module_path"
                        ),
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
                    crate::product_message!(
                        "backend.arxml.host_profile.missing_host_can_ecuc_module"
                    ),
                    Some(path),
                )
            }),
            (None, Some((_, file))) => issues.push(Issue {
                file: Some(file.clone()),
                ..Issue::error(
                    "PDU_UNSUPPORTED",
                    crate::product_message!(
                        "backend.arxml.host_profile.host_can_configuration_without_pdu"
                    ),
                    Some(path),
                )
            }),
            (Some(want), Some((got, file))) if want != got => {
                let missing = want.iter().find(|record| !got.contains(record));
                let extra = got.iter().find(|record| !want.contains(record));
                let record = missing.or(extra).unwrap();
                let field = record.definition.rsplit('/').next().unwrap_or_default();
                let message = if missing.is_some() {
                    crate::product_message!(
                        "backend.arxml.host_profile.host_can_ecuc_required_item_mismatch",
                        "field" => field
                    )
                } else {
                    crate::product_message!(
                        "backend.arxml.host_profile.host_can_ecuc_unsupported_item",
                        "field" => field
                    )
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
fn tool_global_pdu(
    node: Node<'_, '_>,
    file: &SourceFile,
) -> Result<GlobalPduBinding, crate::message::LocalizedText> {
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
        return Err(crate::product_message!(
            "backend.arxml.host_profile.missing_unique_versioned_global_pdu_binding"
        ));
    };
    if group.attribute("GID") != Some("AutosarWorkbenchGlobalPduV1")
        || !group.parent_element().is_some_and(|sdgs| {
            sdgs.tag_name().name() == "SDGS"
                && sdgs.parent_element().is_some_and(|admin| {
                    admin.tag_name().name() == "ADMIN-DATA" && admin.parent_element() == Some(node)
                })
        })
    {
        return Err(crate::product_message!(
            "backend.arxml.host_profile.unsupported_global_pdu_binding_version_or_location"
        ));
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
        return Err(crate::product_message!(
            "backend.arxml.host_profile.global_pdu_length_only"
        ));
    }
    let fields: Vec<_> = group
        .children()
        .filter(|child| child.is_element())
        .collect();
    if fields.len() != 1
        || fields[0].tag_name().name() != "SD"
        || fields[0].attribute("GID") != Some("SystemPduRef")
    {
        return Err(crate::product_message!(
            "backend.arxml.host_profile.global_pdu_unique_system_reference_required"
        ));
    }
    let system_path = fields[0]
        .text()
        .filter(|value| value.starts_with('/'))
        .ok_or_else(|| {
            crate::product_message!(
                "backend.arxml.host_profile.global_pdu_system_reference_absolute_path_required"
            )
        })?
        .to_owned();
    let length = param(node, "PduLength")
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| {
            crate::product_message!("backend.arxml.host_profile.global_pdu_valid_length_required")
        })?;
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

fn diagnostic_shape(
    docs: &[Document<'_>],
) -> Result<DiagnosticShape, crate::message::LocalizedText> {
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
                        .ok_or_else(|| {
                            crate::product_message!(
                                "backend.arxml.host_profile.missing_ecuc_value",
                                "path" => path_of(node)
                            )
                        })?;
                        let definition = definition(item).ok_or_else(|| {
                            crate::product_message!(
                                "backend.arxml.host_profile.missing_ecuc_parameter_definition",
                                "path" => path_of(node)
                            )
                        })?;
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
                    return Err(crate::product_message!(
                        "backend.arxml.host_profile.duplicate_diagnostic_ecuc_container",
                        "path" => path
                    ));
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
) -> Result<Option<u16>, crate::message::LocalizedText> {
    let label = path_of(did);
    let mut groups = nodes.iter().copied().filter(|node| {
        node.tag_name().name() == "SDG"
            && node
                .attribute("GID")
                .is_some_and(|gid| gid.starts_with("AutosarWorkbenchHostRestoreDid"))
    });
    let group = groups.next();
    if groups.next().is_some() {
        return Err(crate::product_message!(
            "backend.arxml.host_profile.duplicate_host_restore_routine_group",
            "path" => label
        ));
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
        return Err(crate::product_message!(
            "backend.arxml.host_profile.unsupported_host_restore_routine_group",
            "path" => label
        ));
    }
    let fields: Vec<_> = group.children().filter(|node| node.is_element()).collect();
    if fields.len() != 2
        || fields
            .iter()
            .any(|node| node.tag_name().name() != "SD" || node.attributes().len() != 1)
    {
        return Err(crate::product_message!(
            "backend.arxml.host_profile.host_restore_routine_unique_fields_required",
            "path" => label
        ));
    }
    let field = |name: &str| {
        let mut found = fields
            .iter()
            .filter(|node| node.attribute("GID") == Some(name));
        let value = found.next().and_then(|node| node.text());
        if found.next().is_some() { None } else { value }
    };
    let rid = field("Rid")
        .ok_or_else(|| {
            crate::product_message!(
                "backend.arxml.host_profile.host_restore_routine_missing_unique_rid",
                "path" => label
            )
        })?
        .parse::<u16>()
        .map_err(|_| {
            crate::product_message!(
                "backend.arxml.host_profile.host_restore_routine_rid_range",
                "path" => label
            )
        })?;
    let session = format!("/{project}/DcmCfg/DcmConfigSet/DcmDsp/Sessions/Extended");
    if field("SessionRef") != Some(session.as_str()) {
        return Err(crate::product_message!(
            "backend.arxml.host_profile.host_restore_routine_session_reference_required",
            "path" => label,
            "session" => session
        ));
    }
    Ok(Some(rid))
}

fn parse_diagnostic(
    files: &[SourceFile],
    project: &str,
    frames: &[FrameView],
    signals: &[SignalView],
) -> Result<Option<DiagnosticView>, crate::message::LocalizedText> {
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
            return Err(crate::product_message!(
                "backend.arxml.host_profile.host_routine_without_diagnostic_configuration"
            ));
        }
        let request = format!("/{project}/NPdu_DiagRequest");
        let response = format!("/{project}/NPdu_DiagResponse");
        if nodes.iter().any(|n| {
            n.tag_name().name() == "N-PDU" && (path_of(*n) == request || path_of(*n) == response)
        }) {
            return Err(crate::product_message!(
                "backend.arxml.host_profile.diagnostic_network_pdu_without_configuration"
            ));
        }
        return Ok(None);
    }
    let unique = |def: &str| -> Result<Node<'_, '_>, crate::message::LocalizedText> {
        let mut found = nodes.iter().copied().filter(|n| {
            n.tag_name().name() == "ECUC-CONTAINER-VALUE" && definition(*n).as_deref() == Some(def)
        });
        let node = found.next().ok_or_else(|| {
            crate::product_message!(
                "backend.arxml.host_profile.missing_diagnostic_container",
                "definition" => def
            )
        })?;
        if found.next().is_some() {
            return Err(crate::product_message!(
                "backend.arxml.host_profile.multiple_diagnostic_containers",
                "definition" => def
            ));
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
    let did: u16 = did.try_into().map_err(|_| {
        crate::product_message!("backend.arxml.host_profile.diagnostic_did_out_of_range")
    })?;
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
            return Err(crate::product_message!(
                "backend.arxml.host_profile.diagnostic_dcm_pdu_shape_required",
                "path" => path
            ));
        }
    }
    for (pdu, tx) in [(&request, false), (&response, true)] {
        let pdus: Vec<_> = nodes
            .iter()
            .copied()
            .filter(|n| n.tag_name().name() == "N-PDU" && path_of(*n) == *pdu)
            .collect();
        if pdus.len() != 1 || child_text(pdus[0], "LENGTH").as_deref() != Some("8") {
            return Err(crate::product_message!(
                "backend.arxml.host_profile.diagnostic_network_pdu_shape_required",
                "path" => pdu
            ));
        }
        if child_text(pdus[0], "HAS-DYNAMIC-LENGTH").is_some() {
            return Err(crate::product_message!(
                "backend.arxml.host_profile.diagnostic_network_pdu_dynamic_length_unsupported",
                "path" => pdu
            ));
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
            return Err(crate::product_message!(
                "backend.arxml.host_profile.diagnostic_pdu_missing_global_pdu",
                "path" => pdu
            ));
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
            return Err(crate::product_message!(
                "backend.arxml.host_profile.diagnostic_pdu_missing_directional_canif_mapping",
                "path" => pdu
            ));
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
            return Err(crate::product_message!(
                "backend.arxml.host_profile.diagnostic_pdu_classic_can_required",
                "path" => pdu
            ));
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
            return Err(crate::product_message!(
                "backend.arxml.host_profile.unsupported_diagnostic_canif_configuration",
                "path" => pdu
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
            return Err(crate::product_message!(
                "backend.arxml.host_profile.diagnostic_pdu_reverse_canif_mapping",
                "path" => pdu
            ));
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
        let data_ref = ref_value(item, "DcmDspDidDataRef").ok_or_else(|| {
            crate::product_message!(
                "backend.arxml.host_profile.diagnostic_did_signal_missing_data_reference"
            )
        })?;
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
            return Err(crate::product_message!(
                "backend.arxml.host_profile.diagnostic_data_reference_not_unique",
                "path" => data_ref
            ));
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
            return Err(crate::product_message!(
                "backend.arxml.host_profile.diagnostic_data_unique_tool_binding_required",
                "path" => data_ref
            ));
        }
        let refs: Vec<_> = metadata[0].children().filter(|n| n.is_element()).collect();
        if refs.len() != 1
            || refs[0].tag_name().name() != "SD"
            || refs[0].attribute("GID") != Some("ComSignalRef")
            || refs[0].text().is_none_or(str::is_empty)
        {
            return Err(crate::product_message!(
                "backend.arxml.host_profile.diagnostic_data_missing_unique_signal_binding",
                "path" => data_ref
            ));
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
        return Err(crate::product_message!(
            "backend.arxml.host_profile.diagnostic_did_contiguous_signal_offsets_required"
        ));
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
            return Err(crate::product_message!(
                "backend.arxml.host_profile.single_dem_uds_dtc_supported"
            ));
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
            return Err(crate::product_message!(
                "backend.arxml.host_profile.dem_event_unique_dtc_association_required"
            ));
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
            return Err(crate::product_message!(
                "backend.arxml.host_profile.dem_event_missing_unique_rx_frame_binding"
            ));
        }
        let refs: Vec<_> = metadata[0].children().filter(|n| n.is_element()).collect();
        if refs.len() != 1
            || refs[0].tag_name().name() != "SD"
            || refs[0].attribute("GID") != Some("MonitorFrameRef")
        {
            return Err(crate::product_message!(
                "backend.arxml.host_profile.dem_event_unique_monitor_reference_required"
            ));
        }
        let monitor_frame_path = refs[0]
            .text()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                crate::product_message!(
                    "backend.arxml.host_profile.dem_event_missing_monitor_frame_path"
                )
            })?
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
        return Err(crate::product_message!(
            "backend.arxml.host_profile.dem_nvm_without_supported_dtc"
        ));
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
        return Err(crate::message::LocalizedText::messages([
            crate::product_message!(
                "backend.arxml.host_profile.diagnostic_validation_failure",
                "code" => issue.code
            ),
            issue.message.clone(),
        ]));
    }
    let expected = render_profile(project, frames, signals, Some(&diagnostic));
    let expected_doc = Document::parse(&expected).map_err(|e| e.to_string())?;
    let expected_shape = diagnostic_shape(&[expected_doc])?;
    if actual != expected_shape {
        return Err(crate::product_message!(
            "backend.arxml.host_profile.unsupported_diagnostic_configuration_shape"
        ));
    }
    Ok(Some(diagnostic))
}

struct HostSourceProfile<'a> {
    name: &'a str,
    files: &'a [SourceFile],
    frames: Vec<FrameView>,
    signals: Vec<SignalView>,
    diagnostic: Option<DiagnosticView>,
    issues: Vec<Issue>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct HostSourceElement {
    name: String,
    namespace: Option<String>,
    attributes: Vec<(Option<String>, String, String)>,
    text: String,
    children: Vec<HostSourceElement>,
}

// Whitespace, comments and ordering of independently named configuration
// entries do not change profile identity. Every element, attribute and value
// remains in this comparison, including unknown or additional business data.
fn host_source_element(node: Node<'_, '_>) -> HostSourceElement {
    let mut attributes: Vec<_> = node
        .attributes()
        .map(|attribute| {
            (
                attribute.namespace().map(str::to_owned),
                attribute.name().into(),
                attribute.value().into(),
            )
        })
        .collect();
    attributes.sort();
    let mut children: Vec<_> = node
        .children()
        .filter(|node| node.is_element())
        .map(host_source_element)
        .collect();
    children.sort();
    HostSourceElement {
        name: node.tag_name().name().into(),
        namespace: node.tag_name().namespace().map(str::to_owned),
        attributes,
        text: node
            .children()
            .filter(|node| node.is_text())
            .filter_map(|node| node.text())
            .map(str::trim)
            .collect(),
        children,
    }
}

/// Recognize the complete product host source shape, without opening a
/// workspace or entering definition validation again.
pub(crate) fn historical_host_mode_dependency(documents: &[&Document<'_>]) -> bool {
    if documents.len() != 1 {
        return false;
    }
    let document = &documents[0];
    let packages: Vec<_> = document
        .root_element()
        .children()
        .filter(|node| node.tag_name().name() == "AR-PACKAGES")
        .flat_map(|node| {
            node.children()
                .filter(|node| node.tag_name().name() == "AR-PACKAGE")
        })
        .collect();
    if packages.len() != 1 {
        return false;
    }
    let Some(name) = child_text(packages[0], "SHORT-NAME") else {
        return false;
    };
    let files = [SourceFile {
        path: PathBuf::from("host.arxml"),
        text: document.input_text().into(),
        saved: document.input_text().into(),
        original_name: None,
    }];
    let mut profile = HostSourceProfile {
        name: &name,
        files: &files,
        frames: Vec::new(),
        signals: Vec::new(),
        diagnostic: None,
        issues: Vec::new(),
    };
    if profile.refresh().is_err() {
        return false;
    }
    if !profile.issues.is_empty() || profile.diagnostic.is_none() {
        return false;
    }
    // Recover caller order because source-generated handle assignments depend
    // on it; the ordinary parsed workspace views are sorted for presentation.
    let positions: BTreeMap<_, _> = document
        .descendants()
        .filter(|node| child_text(*node, "SHORT-NAME").is_some())
        .map(|node| (path_of(node), node.range().start))
        .collect();
    profile
        .frames
        .sort_by_key(|frame| positions.get(&frame.path).copied());
    profile
        .signals
        .sort_by_key(|signal| positions.get(&signal.path).copied());
    let expected = render_profile(
        &name,
        &profile.frames,
        &profile.signals,
        profile.diagnostic.as_ref(),
    );
    let Ok(expected) = Document::parse(&expected) else {
        return false;
    };
    host_source_element(document.root_element()) == host_source_element(expected.root_element())
}

impl HostSourceProfile<'_> {
    fn refresh(&mut self) -> Result<(), crate::message::LocalizedText> {
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
        for file in self.files {
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
                            crate::product_message!(
                                "backend.arxml.host_profile.duplicate_arxml_absolute_path"
                            ),
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
                                    crate::product_message!(
                                        "backend.arxml.host_profile.consumed_com_unique_module_owner_required"
                                    ),
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
                                crate::product_message!(
                                    "backend.arxml.host_profile.unsupported_com_module_structure"
                                ),
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
                            crate::product_message!(
                                "backend.arxml.host_profile.incomplete_ecuc_root_configuration"
                            ),
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
                                crate::product_message!(
                                    "backend.arxml.host_profile.com_container_direct_owner_required"
                                ),
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
                                                crate::product_message!(
                                                    "backend.arxml.host_profile.duplicate_global_pdu_path"
                                                ),
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
                                    crate::product_message!(
                                        "backend.arxml.host_profile.multiple_canif_mappings"
                                    ),
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
                                    crate::product_message!(
                                        "backend.arxml.host_profile.multiple_canif_mappings"
                                    ),
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
                        crate::product_message!(
                            "backend.arxml.host_profile.canif_reference_unresolved_global_pdu",
                            "target" => target
                        ),
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
                        crate::product_message!(
                            "backend.arxml.host_profile.global_system_pdu_bijection_required",
                            "path" => binding.system_path
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
                    crate::product_message!(
                        "backend.arxml.host_profile.com_ipdu_missing_pdu_reference"
                    ),
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
                            crate::product_message!(
                                "backend.arxml.host_profile.unsupported_com_global_pdu_configuration"
                            ),
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
                        crate::product_message!(
                            "backend.arxml.host_profile.com_pdu_reference_bound_global_pdu_required",
                            "reference" => pdu_ref
                        ),
                        Some(path),
                    )
                });
                continue;
            };
            let Some(pdu) = pdus.get(system_path) else {
                issues.push(Issue::error(
                    "PDU_UNSUPPORTED",
                    crate::product_message!(
                        "backend.arxml.host_profile.global_pdu_unresolved_system_ipdu",
                        "path" => system_path
                    ),
                    Some(pdu_ref),
                ));
                continue;
            };
            let Some(canif_pdu) = canif.get(&pdu_ref) else {
                issues.push(Issue::error(
                    "CANIF_PDU_REF",
                    crate::product_message!(
                        "backend.arxml.host_profile.canif_same_pdu_reference_required"
                    ),
                    Some(pdu_ref),
                ));
                continue;
            };
            if canif_pdu.dest.as_deref() != Some("ECUC-CONTAINER-VALUE") {
                issues.push(Issue::error(
                    "PDU_UNSUPPORTED",
                    crate::product_message!(
                        "backend.arxml.host_profile.canif_pdu_reference_container_destination_required"
                    ),
                    Some(pdu_ref.clone()),
                ));
            }
            let direction = match record.direction.as_deref() {
                Some("SEND") if canif_pdu.tx => Direction::Tx,
                Some("RECEIVE") if !canif_pdu.tx => Direction::Rx,
                _ => {
                    issues.push(Issue::error(
                        "PDU_DIRECTION",
                        crate::product_message!(
                            "backend.arxml.host_profile.com_canif_direction_mismatch"
                        ),
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
                    crate::product_message!("backend.arxml.host_profile.classic_can_only"),
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
                    crate::product_message!(
                        "backend.arxml.host_profile.global_system_ipdu_length_mismatch"
                    ),
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
                        crate::product_message!(
                            "backend.arxml.host_profile.unresolved_com_ipdu_signal_reference"
                        ),
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
                        crate::product_message!(
                            "backend.arxml.host_profile.little_endian_unsigned_scalar_signals_only"
                        ),
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
                        crate::product_message!(
                            "backend.arxml.host_profile.signal_position_or_length_out_of_range"
                        ),
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
                            crate::product_message!(
                                "backend.arxml.host_profile.receive_signal_timeouts_must_match"
                            ),
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
                        crate::product_message!(
                            "backend.arxml.host_profile.ipdu_signal_mapping_layout_mismatch"
                        ),
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
                            crate::product_message!(
                                "backend.arxml.host_profile.network_frame_complete_ipdu_mapping_required"
                            ),
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
                            crate::product_message!(
                                "backend.arxml.host_profile.network_canif_identifier_mismatch"
                            ),
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
                    crate::product_message!(
                        "backend.arxml.host_profile.ipdu_canif_data_length_mismatch"
                    ),
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
                                crate::product_message!(
                                    "backend.arxml.host_profile.diagnostic_global_pdu_missing_system_binding",
                                    "path" => path
                                ),
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
                            crate::product_message!(
                                "backend.arxml.host_profile.global_pdu_unique_supported_usage_required"
                            ),
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

    pub(super) fn refresh(&mut self) -> Result<(), crate::message::LocalizedText> {
        let mut profile = HostSourceProfile {
            name: &self.name,
            files: &self.files,
            frames: Vec::new(),
            signals: Vec::new(),
            diagnostic: None,
            issues: Vec::new(),
        };
        profile.refresh()?;
        self.frames = profile.frames;
        self.signals = profile.signals;
        self.diagnostic = profile.diagnostic;
        self.issues = profile.issues;
        self.rebuild_snapshot()?;
        if self.snapshot.integration_candidate {
            self.frames.clear();
            self.signals.clear();
            self.diagnostic = None;
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
        }
        Ok(())
    }

    pub(super) fn global_pdu_for(
        &self,
        system_path: &str,
    ) -> Result<String, crate::message::LocalizedText> {
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
                    return Err(crate::product_message!(
                        "backend.arxml.host_profile.multiple_global_pdu_bindings",
                        "path" => system_path
                    ));
                }
            }
        }
        found.ok_or_else(|| {
            crate::product_message!(
                "backend.arxml.host_profile.missing_locatable_global_pdu_binding",
                "path" => system_path
            )
        })
    }
}
