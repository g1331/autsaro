use super::catalog::SymbolContract;
use super::component::c_name;
use super::{ContractArgument, PlanDescription, PlanDiagnostic};
use std::collections::{BTreeMap, BTreeSet};

fn os_hook(name: &str) -> bool {
    matches!(
        name,
        "StartupHook" | "ShutdownHook" | "ErrorHook" | "PreTaskHook" | "PostTaskHook"
    )
}

/// The one exclusive area a module declares and enters (ADR 0018). A module is added
/// here when its code stops using another module's area.
pub(super) fn exclusive_area(module: &str) -> Option<&'static str> {
    match module {
        "BswM" => Some("BSWM_STATE"),
        "Can" => Some("CAN_STATE"),
        "CanIf" => Some("CANIF_STATE"),
        "CanTp" => Some("CANTP_STATE"),
        "Com" => Some("COM_STATE"),
        "ComM" => Some("COMM_STATE"),
        "Dcm" => Some("DCM_STATE"),
        "Det" => Some("DET_STATE"),
        "LSduR" => Some("LSDUR_STATE"),
        "PduR" => Some("PDUR_STATE"),
        "Ecu_HostBusSM" => Some("CAN_HOST_AREA_BUSSM"),
        "Rte" => Some("SCHM_MODE"),
        _ => None,
    }
}

/// Functions defined in the delivered module source that do not enter its area,
/// directly or through a helper of the same file. ADR 0018 attaches `canEnter`
/// only to entries that enter; generated wrappers keep it.
fn non_entering_functions(module: &str) -> BTreeSet<String> {
    let path = if module == "Ecu_HostBusSM" {
        "runtime/host/src/Ecu_HostBusSM.c".to_owned()
    } else {
        format!("runtime/bsw/src/{module}.c")
    };
    let Some(asset) = crate::resources::AssetInventory::embedded().get(&path) else {
        return BTreeSet::new();
    };
    // `lines` also drops the CR of CRLF sources.
    let source = String::from_utf8_lossy(asset.bytes);
    // clang-format puts the braces of every function definition at column 0,
    // or keeps a short definition on its one line.
    let name = |head: &str| {
        let signature = head.split('(').next().unwrap_or("");
        let mut words = signature.rsplit(|c: char| !(c.is_alphanumeric() || c == '_'));
        words.next().unwrap_or("").to_owned()
    };
    let mut bodies = Vec::new();
    let mut head = "";
    let mut body: Option<(String, String)> = None;
    for line in source.lines() {
        match (&mut body, line) {
            (None, "{") => body = Some((name(head), String::new())),
            (None, _) if line.contains('(') && line.contains(") {") && line.ends_with('}') => {
                bodies.push((name(line), line.to_owned()));
            }
            (None, _) if !line.starts_with(char::is_whitespace) => head = line,
            (Some(_), "}") => bodies.extend(body.take()),
            (Some((_, text)), _) => {
                text.push_str(line);
                text.push('\n');
            }
            (None, _) => {}
        }
    }
    let calls = |text: &str, callee: &str| {
        text.match_indices(&format!("{callee}("))
            .any(|(at, _)| !text[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_'))
    };
    let mut entering: BTreeSet<String> = bodies
        .iter()
        .filter(|(_, text)| {
            text.contains(&format!("SchM_Enter_{module}_"))
                || (module == "Ecu_HostBusSM" && text.contains("Ecu_HostBusSM_Enter("))
        })
        .map(|(name, _)| name.clone())
        .collect();
    loop {
        let reached: Vec<String> = bodies
            .iter()
            .filter(|(name, text)| {
                !entering.contains(name) && entering.iter().any(|callee| calls(text, callee))
            })
            .map(|(name, _)| name.clone())
            .collect();
        if reached.is_empty() {
            return bodies
                .into_iter()
                .map(|(name, _)| name)
                .filter(|name| !entering.contains(name))
                .collect();
        }
        entering.extend(reached);
    }
}

#[allow(
    clippy::disallowed_methods,
    reason = "DOMAIN writes path separators as dots; no rendered text is patched"
)]
fn artifact(path: &str) -> String {
    format!(
        "<SHORT-LABEL>{}</SHORT-LABEL><CATEGORY>{}</CATEGORY><DOMAIN>{}</DOMAIN>",
        super::xml_text(super::short_name(path)),
        if path.ends_with(".c") {
            "SWSRC"
        } else {
            "SWHDR"
        },
        super::xml_text(
            &path
                .rsplit_once('/')
                .map_or("", |(directory, _)| directory)
                .replace('/', ".")
        )
    )
}
fn argument(name: &str, native: &str, direction: &str, types: &BTreeMap<String, usize>) -> String {
    format!(
        "<SW-SERVICE-ARG>{}</SW-SERVICE-ARG>",
        argument_fields(name, native, direction, types)
    )
}
fn argument_fields(
    name: &str,
    native: &str,
    direction: &str,
    types: &BTreeMap<String, usize>,
) -> String {
    format!(
        r#"<SHORT-NAME>{}</SHORT-NAME><CATEGORY>VALUE</CATEGORY><DIRECTION>{direction}</DIRECTION><SW-DATA-DEF-PROPS><SW-DATA-DEF-PROPS-VARIANTS><SW-DATA-DEF-PROPS-CONDITIONAL><BASE-TYPE-REF DEST="SW-BASE-TYPE">/HostArtifacts/Native{}</BASE-TYPE-REF></SW-DATA-DEF-PROPS-CONDITIONAL></SW-DATA-DEF-PROPS-VARIANTS></SW-DATA-DEF-PROPS>"#,
        super::xml_text(name),
        types[native]
    )
}
fn rte_symbol(name: &str, return_type: &str, args: &[(&str, &str, &str)]) -> SymbolContract {
    SymbolContract {
        symbol: name.into(),
        return_type: return_type.into(),
        arguments: args
            .iter()
            .map(|(name, native_type, direction)| ContractArgument {
                name: (*name).into(),
                native_type: (*native_type).into(),
                direction: (*direction).into(),
            })
            .collect(),
        declaration_owner: "generated RTE headers".into(),
        definition_owner: "src/Rte.c".into(),
        consumers: Vec::new(),
    }
}

fn validate_areas(delivered: &BTreeMap<&str, &[u8]>) -> Result<(), Vec<PlanDiagnostic>> {
    for module in [
        "BswM", "Can", "CanIf", "CanTp", "Com", "ComM", "Dcm", "Det", "LSduR", "PduR",
    ] {
        let path = format!("generated/include/SchM_{module}.h");
        let bytes = delivered
            .get(path.as_str())
            .ok_or_else(|| PlanDiagnostic::plan_premise(&path))?;
        let text = std::str::from_utf8(bytes).map_err(|_| PlanDiagnostic::plan_premise(&path))?;
        let actual: BTreeSet<_> = super::module_contracts::declarations(text)?
            .into_iter()
            .filter(|entry| {
                entry.name.starts_with("SchM_Enter_") || entry.name.starts_with("SchM_Exit_")
            })
            .map(|entry| entry.name)
            .collect();
        let area = exclusive_area(module).ok_or_else(|| PlanDiagnostic::plan_premise(module))?;
        let expected = BTreeSet::from([
            format!("SchM_Enter_{module}_{area}"),
            format!("SchM_Exit_{module}_{area}"),
        ]);
        if actual != expected {
            return Err(PlanDiagnostic::plan_premise(&path));
        }
        for (&source_path, bytes) in delivered.iter().filter(|(path, _)| path.ends_with(".c")) {
            let source = std::str::from_utf8(bytes)
                .map_err(|_| PlanDiagnostic::plan_premise(source_path))?;
            for prefix in [
                format!("SchM_Enter_{module}_"),
                format!("SchM_Exit_{module}_"),
            ] {
                for (at, _) in source.match_indices(&prefix) {
                    let symbol: String = source[at..]
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    if !expected.contains(&symbol) {
                        return Err(PlanDiagnostic::plan_premise(source_path));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Product implementation descriptions reference only the sealed project's actual files.
/// They describe this host profile, not an ICC3 or complete standard API certification.
pub(super) fn files(
    plan: &PlanDescription,
    delivered: &BTreeMap<&str, &[u8]>,
) -> Result<BTreeMap<String, Vec<u8>>, Vec<PlanDiagnostic>> {
    let sources: BTreeSet<String> = delivered.keys().map(|path| (*path).to_owned()).collect();
    validate_areas(delivered)?;
    let mut entries: BTreeMap<String, Vec<SymbolContract>> = BTreeMap::new();
    for symbol in &plan.symbols {
        if symbol.definition_owner.starts_with("application/") {
            continue;
        }
        if symbol.definition_owner == "src/Rte.c" {
            entries
                .entry("Rte".into())
                .or_default()
                .push(symbol.clone());
        } else if symbol.definition_owner == "src/Ecu_RuntimeConfig.c" {
            let module = if symbol.symbol.starts_with("ComM_") {
                "ComM"
            } else {
                "Com"
            };
            entries
                .entry(module.into())
                .or_default()
                .push(symbol.clone());
        } else if let Some(path) = symbol.definition_owner.strip_prefix("runtime/bsw/src/") {
            entries
                .entry(path.trim_end_matches(".c").into())
                .or_default()
                .push(symbol.clone());
        } else if symbol.definition_owner.contains("generated Rte.c")
            || symbol
                .definition_owner
                .contains("generated Dcm service binding")
            || symbol.definition_owner.contains("BSW signature adapter")
        {
            entries
                .entry("Rte".into())
                .or_default()
                .push(symbol.clone());
        }
    }
    for (module, name, result, args, declaration, definition) in [
        (
            "Can",
            "Can_DeInit",
            "void",
            vec![],
            "runtime/bsw/include/Can.h",
            "runtime/bsw/src/Can.c",
        ),
        (
            "Det",
            "Det_ReportRuntimeError",
            "Std_ReturnType",
            vec![
                ("ModuleId", "uint16_t", "IN"),
                ("InstanceId", "uint8_t", "IN"),
                ("ApiId", "uint8_t", "IN"),
                ("ErrorId", "uint8_t", "IN"),
            ],
            "runtime/bsw/include/Det.h",
            "runtime/bsw/src/Det.c",
        ),
    ] {
        let mut symbol = rte_symbol(name, result, &args);
        symbol.declaration_owner = declaration.into();
        symbol.definition_owner = definition.into();
        entries.entry(module.into()).or_default().push(symbol);
    }
    let os = entries.entry("Os".into()).or_default();
    for (name, args) in [
        (
            "GetCounterValue",
            vec![
                ("CounterID", "CounterType", "IN"),
                ("Value", "TickType *", "OUT"),
            ],
        ),
        (
            "GetElapsedValue",
            vec![
                ("CounterID", "CounterType", "IN"),
                ("Value", "TickType *", "INOUT"),
                ("ElapsedValue", "TickType *", "OUT"),
            ],
        ),
    ] {
        let mut symbol = rte_symbol(name, "StatusType", &args);
        symbol.declaration_owner = "runtime/os/include/Os.h".into();
        symbol.definition_owner = "runtime/os/src/Os_ArtiServices.c".into();
        os.push(symbol);
    }
    // Actual imported standard interfaces of the selected runtime chain.
    // These contracts are explicit R24-11 signatures, not inferred from C bodies.
    let mut imported =
        |module: &str, name: &str, result: &str, args: &[(&str, &str, &str)], header: &str| {
            let module_entries = entries.entry(module.into()).or_default();
            if !module_entries.iter().any(|entry| entry.symbol == name) {
                let mut symbol = rte_symbol(name, result, args);
                symbol.declaration_owner = header.into();
                symbol.definition_owner = format!("runtime/bsw/src/{module}.c");
                module_entries.push(symbol);
            }
        };
    imported(
        "Can",
        "Can_Write",
        "Std_ReturnType",
        &[
            ("Hth", "Can_HwHandleType", "IN"),
            ("PduInfo", "const Can_PduType *", "IN"),
        ],
        "runtime/bsw/include/Can.h",
    );
    imported(
        "Can",
        "Can_SetControllerMode",
        "Std_ReturnType",
        &[
            ("Controller", "uint8", "IN"),
            ("Transition", "Can_ControllerStateType", "IN"),
        ],
        "runtime/bsw/include/Can.h",
    );
    imported(
        "Can",
        "Can_GetControllerMode",
        "Std_ReturnType",
        &[
            ("Controller", "uint8", "IN"),
            ("Value", "Can_ControllerStateType *", "OUT"),
        ],
        "runtime/bsw/include/Can.h",
    );
    imported(
        "Can",
        "Can_GetControllerErrorState",
        "Std_ReturnType",
        &[
            ("Controller", "uint8", "IN"),
            ("Value", "Can_ErrorStateType *", "OUT"),
        ],
        "runtime/bsw/include/Can.h",
    );
    imported(
        "Can",
        "Can_GetControllerRxErrorCounter",
        "Std_ReturnType",
        &[("Controller", "uint8", "IN"), ("Value", "uint8 *", "OUT")],
        "runtime/bsw/include/Can.h",
    );
    imported(
        "Can",
        "Can_GetControllerTxErrorCounter",
        "Std_ReturnType",
        &[("Controller", "uint8", "IN"), ("Value", "uint8 *", "OUT")],
        "runtime/bsw/include/Can.h",
    );
    imported(
        "CanIf",
        "CanIf_RxIndication",
        "void",
        &[
            ("Mailbox", "const Can_HwType *", "IN"),
            ("PduInfoPtr", "const PduInfoType *", "IN"),
        ],
        "runtime/bsw/include/CanIf.h",
    );
    imported(
        "CanIf",
        "CanIf_TxConfirmation",
        "void",
        &[("CanTxPduId", "PduIdType", "IN")],
        "runtime/bsw/include/CanIf.h",
    );
    imported(
        "CanIf",
        "CanIf_ControllerModeIndication",
        "void",
        &[
            ("ControllerId", "uint8", "IN"),
            ("ControllerMode", "Can_ControllerStateType", "IN"),
        ],
        "runtime/bsw/include/CanIf.h",
    );
    imported(
        "CanIf",
        "CanIf_ControllerBusOff",
        "void",
        &[("ControllerId", "uint8", "IN")],
        "runtime/bsw/include/CanIf.h",
    );
    imported(
        "ComM",
        "ComM_DCM_ActiveDiagnostic",
        "void",
        &[("Channel", "NetworkHandleType", "IN")],
        "runtime/bsw/include/ComM_Dcm.h",
    );
    imported(
        "ComM",
        "ComM_DCM_InactiveDiagnostic",
        "void",
        &[("Channel", "NetworkHandleType", "IN")],
        "runtime/bsw/include/ComM_Dcm.h",
    );
    imported(
        "Com",
        "Com_RxIndication",
        "void",
        &[
            ("PduId", "PduIdType", "IN"),
            ("PduInfoPtr", "const PduInfoType *", "IN"),
        ],
        "runtime/bsw/include/Com.h",
    );
    imported(
        "Com",
        "Com_TxConfirmation",
        "void",
        &[
            ("PduId", "PduIdType", "IN"),
            ("Result", "Std_ReturnType", "IN"),
        ],
        "runtime/bsw/include/Com.h",
    );
    imported(
        "Com",
        "Com_TriggerTransmit",
        "Std_ReturnType",
        &[
            ("PduId", "PduIdType", "IN"),
            ("PduInfoPtr", "PduInfoType *", "INOUT"),
        ],
        "runtime/bsw/include/Com.h",
    );
    imported(
        "ComM",
        "ComM_BusSM_ModeIndication",
        "void",
        &[
            ("Channel", "NetworkHandleType", "IN"),
            ("ComMode", "ComM_ModeType", "IN"),
        ],
        "runtime/bsw/include/ComM.h",
    );
    imported(
        "BswM",
        "BswM_ComM_CurrentMode",
        "void",
        &[
            ("Network", "NetworkHandleType", "IN"),
            ("RequestedMode", "ComM_ModeType", "IN"),
        ],
        "runtime/bsw/include/BswM_ComM.h",
    );
    if plan.diagnostic_transport.is_some() {
        imported(
            "CanTp",
            "CanTp_Transmit",
            "Std_ReturnType",
            &[
                ("TxPduId", "PduIdType", "IN"),
                ("PduInfoPtr", "const PduInfoType *", "IN"),
            ],
            "runtime/bsw/include/CanTp.h",
        );
        imported(
            "CanTp",
            "CanTp_RxIndication",
            "void",
            &[
                ("RxPduId", "PduIdType", "IN"),
                ("PduInfoPtr", "const PduInfoType *", "IN"),
            ],
            "runtime/bsw/include/CanTp.h",
        );
        imported(
            "CanTp",
            "CanTp_TxConfirmation",
            "void",
            &[
                ("TxPduId", "PduIdType", "IN"),
                ("Result", "Std_ReturnType", "IN"),
            ],
            "runtime/bsw/include/CanTp.h",
        );
        imported(
            "PduR",
            "PduR_DcmTransmit",
            "Std_ReturnType",
            &[
                ("TxPduId", "PduIdType", "IN"),
                ("PduInfoPtr", "const PduInfoType *", "IN"),
            ],
            "runtime/bsw/include/PduR_Dcm.h",
        );
    }
    if plan.diagnostic_transport.is_some() {
        for (owner, prefix, header) in [
            ("PduR", "PduR_CanTp", "runtime/bsw/include/PduR_CanTp.h"),
            ("Dcm", "Dcm_", "runtime/bsw/include/Dcm.h"),
        ] {
            for (suffix, result, args) in [
                (
                    "StartOfReception",
                    "BufReq_ReturnType",
                    vec![
                        ("Id", "PduIdType", "IN"),
                        ("Info", "const PduInfoType *", "IN"),
                        ("TpSduLength", "PduLengthType", "IN"),
                        ("Available", "PduLengthType *", "OUT"),
                    ],
                ),
                (
                    "CopyRxData",
                    "BufReq_ReturnType",
                    vec![
                        ("Id", "PduIdType", "IN"),
                        ("Info", "const PduInfoType *", "IN"),
                        ("Available", "PduLengthType *", "OUT"),
                    ],
                ),
                (
                    if owner == "Dcm" {
                        "TpRxIndication"
                    } else {
                        "RxIndication"
                    },
                    "void",
                    vec![
                        ("Id", "PduIdType", "IN"),
                        ("Result", "Std_ReturnType", "IN"),
                    ],
                ),
                (
                    "CopyTxData",
                    "BufReq_ReturnType",
                    vec![
                        ("Id", "PduIdType", "IN"),
                        ("Info", "const PduInfoType *", "IN"),
                        ("Retry", "const RetryInfoType *", "IN"),
                        ("Available", "PduLengthType *", "OUT"),
                    ],
                ),
            ] {
                imported(owner, &format!("{prefix}{suffix}"), result, &args, header);
            }
        }
        imported(
            "PduR",
            "PduR_CanTpTxConfirmation",
            "void",
            &[
                ("Id", "PduIdType", "IN"),
                ("Result", "Std_ReturnType", "IN"),
            ],
            "runtime/bsw/include/PduR_CanTp.h",
        );
    }
    for (owner, prefix) in [("LSduR", "LSduR_CanIf"), ("PduR", "PduR_CanIf")] {
        let header = format!("runtime/bsw/include/{owner}_CanIf.h");
        for (suffix, result, args) in [
            (
                "RxIndication",
                "void",
                vec![
                    ("RxPduId", "PduIdType", "IN"),
                    ("PduInfoPtr", "const PduInfoType *", "IN"),
                ],
            ),
            (
                "TxConfirmation",
                "void",
                vec![
                    ("TxPduId", "PduIdType", "IN"),
                    ("Result", "Std_ReturnType", "IN"),
                ],
            ),
            (
                "TriggerTransmit",
                "Std_ReturnType",
                vec![
                    ("TxPduId", "PduIdType", "IN"),
                    ("PduInfoPtr", "PduInfoType *", "INOUT"),
                ],
            ),
        ] {
            imported(owner, &format!("{prefix}{suffix}"), result, &args, &header);
        }
    }
    for upper in ["PduR", "CanTp"] {
        if upper == "CanTp" && plan.diagnostic_transport.is_none() {
            continue;
        }
        imported(
            "LSduR",
            &format!("LSduR_{upper}Transmit"),
            "Std_ReturnType",
            &[
                ("TxPduId", "PduIdType", "IN"),
                ("PduInfoPtr", "const PduInfoType *", "IN"),
            ],
            &format!("runtime/bsw/include/LSduR_{upper}.h"),
        );
    }
    for (name, args) in [
        (
            "CanIf_SetControllerMode",
            vec![
                ("ControllerId", "uint8", "IN"),
                ("ControllerMode", "Can_ControllerStateType", "IN"),
            ],
        ),
        (
            "CanIf_GetControllerMode",
            vec![
                ("ControllerId", "uint8", "IN"),
                ("ControllerModePtr", "Can_ControllerStateType *", "OUT"),
            ],
        ),
        (
            "CanIf_GetControllerErrorState",
            vec![
                ("ControllerId", "uint8", "IN"),
                ("ErrorStatePtr", "Can_ErrorStateType *", "OUT"),
            ],
        ),
        (
            "CanIf_SetPduMode",
            vec![
                ("ControllerId", "uint8", "IN"),
                ("PduModeRequest", "CanIf_PduModeType", "IN"),
            ],
        ),
        (
            "CanIf_GetPduMode",
            vec![
                ("ControllerId", "uint8", "IN"),
                ("PduModePtr", "CanIf_PduModeType *", "OUT"),
            ],
        ),
    ] {
        imported(
            "CanIf",
            name,
            "Std_ReturnType",
            &args,
            "runtime/bsw/include/CanIf.h",
        );
    }
    let dcm = entries.entry("Dcm".into()).or_default();
    for (name, result, args) in [
        ("Dcm_ResetToDefaultSession", "Std_ReturnType", vec![]),
        (
            "Dcm_TpTxConfirmation",
            "void",
            vec![
                ("Id", "PduIdType", "IN"),
                ("Result", "Std_ReturnType", "IN"),
            ],
        ),
        (
            "Dcm_ComM_NoComModeEntered",
            "void",
            vec![("NetworkId", "uint8", "IN")],
        ),
        (
            "Dcm_ComM_SilentComModeEntered",
            "void",
            vec![("NetworkId", "uint8", "IN")],
        ),
        (
            "Dcm_ComM_FullComModeEntered",
            "void",
            vec![("NetworkId", "uint8", "IN")],
        ),
    ] {
        let mut symbol = rte_symbol(name, result, &args);
        symbol.declaration_owner = if name.starts_with("Dcm_ComM_") {
            "runtime/bsw/include/Dcm_ComM.h"
        } else {
            "runtime/bsw/include/Dcm.h"
        }
        .into();
        symbol.definition_owner = "runtime/bsw/src/Dcm.c".into();
        dcm.push(symbol);
    }

    let rte = entries.entry("Rte".into()).or_default();
    rte.push(rte_symbol("Rte_Start", "Std_ReturnType", &[]));
    rte.push(rte_symbol("Rte_Stop", "Std_ReturnType", &[]));
    rte.push(rte_symbol(
        "SchM_Init",
        "void",
        &[("Config", "const SchM_ConfigType *", "IN")],
    ));
    for name in ["SchM_Start", "SchM_StartTiming", "SchM_Deinit"] {
        rte.push(rte_symbol(name, "void", &[]));
    }
    for (name, result, arguments) in [
        (
            "SchM_Switch_Dcm_DcmDiagnosticSessionControl",
            "Std_ReturnType",
            vec![("Mode", "Rte_ModeType_DcmDiagnosticSessionControl", "IN")],
        ),
        (
            "SchM_Mode_Dcm_DcmDiagnosticSessionControl",
            "Rte_ModeType_DcmDiagnosticSessionControl",
            vec![],
        ),
    ] {
        let mut symbol = rte_symbol(name, result, &arguments);
        symbol.declaration_owner = "include/SchM_Dcm.h".into();
        symbol.definition_owner = "src/SchM_Main.c".into();
        rte.push(symbol);
    }
    let authentication = plan.ecu_composition()?.authentication_modes()?;
    let group = &authentication.group;
    let native = format!("Rte_ModeType_{group}");
    for (name, result, args) in [
        (
            format!("SchM_Switch_Dcm_{group}"),
            "Std_ReturnType",
            vec![("Mode", native.as_str(), "IN")],
        ),
        (format!("SchM_Mode_Dcm_{group}"), native.as_str(), vec![]),
    ] {
        let mut symbol = rte_symbol(&name, result, &args);
        symbol.declaration_owner = "include/SchM_Dcm.h".into();
        symbol.definition_owner = "src/SchM_Main.c".into();
        rte.push(symbol);
    }

    let counter = c_name(super::short_name(&plan.schedule.counter));
    rte.push(rte_symbol(
        &format!("OsService_{counter}_GetCounterValue"),
        "StatusType",
        &[
            ("CounterID", "CounterType", "IN"),
            ("Value", "TimeInMicrosecondsType *", "OUT"),
        ],
    ));
    rte.push(rte_symbol(
        &format!("OsService_{counter}_GetElapsedValue"),
        "StatusType",
        &[
            ("CounterID", "CounterType", "IN"),
            ("Value", "TimeInMicrosecondsType *", "INOUT"),
            ("ElapsedValue", "TimeInMicrosecondsType *", "OUT"),
        ],
    ));
    // Counter-bound client conveniences are header-local inline functions;
    // only the actual external service servers are linkable BSW module entries.
    for symbol in rte.iter_mut() {
        if symbol.symbol.starts_with("OsService_") {
            symbol.declaration_owner = "include/Rte_Os.h".into();
            symbol.definition_owner = "src/Rte_OsService.c".into();
        } else if matches!(
            symbol.symbol.as_str(),
            "SchM_Init" | "SchM_Start" | "SchM_StartTiming" | "SchM_Deinit"
        ) {
            symbol.declaration_owner = "include/Rte_Main.h".into();
            symbol.definition_owner = "src/SchM_Main.c".into();
        }
    }
    let mut public_contracts = BTreeMap::new();
    for (&path, bytes) in delivered {
        if !(path.starts_with("bsw/include/") && path.ends_with(".h"))
            && !matches!(
                path,
                "os/include/Os.h"
                    | "os/include/Os_Hooks.h"
                    | "os/include/Arti.h"
                    | "generated/include/Det_Externals.h"
                    | "integration/include/Ecu_HostBusSM.h"
            )
        {
            continue;
        }
        let source = std::str::from_utf8(bytes).map_err(|_| PlanDiagnostic::plan_premise(path))?;
        for declaration in super::module_contracts::declarations(source)? {
            if declaration.name.starts_with("SchM_") {
                continue;
            }
            let module = if path == "integration/include/Ecu_HostBusSM.h" {
                "Ecu_HostBusSM"
            } else if path == "generated/include/Det_Externals.h" || os_hook(&declaration.name) {
                "Rte"
            } else if matches!(path, "os/include/Os.h" | "os/include/Os_Hooks.h") {
                "Os"
            } else if path == "os/include/Arti.h" {
                "Arti"
            } else {
                declaration.name.split('_').next().unwrap_or("")
            };
            let name = declaration.name.clone();
            let contract = entries.entry(module.into()).or_default();
            let signature = super::catalog::SymbolContract {
                symbol: name.clone(),
                return_type: declaration.result.clone(),
                arguments: declaration.arguments.clone(),
                declaration_owner: path.into(),
                definition_owner: if os_hook(&name) {
                    if name == "ShutdownHook" {
                        "integration/src/Ecu_Target.c".into()
                    } else {
                        "integration/src/Ecu_OsHooks.c".into()
                    }
                } else if name == "Ecu_DetRuntimeCallout" {
                    "integration/src/Det_Callouts.c".into()
                } else {
                    format!("bsw/src/{module}.c")
                },
                consumers: Vec::new(),
            };
            if let Some(existing) = contract.iter_mut().find(|entry| entry.symbol == name) {
                existing.return_type = signature.return_type;
                existing.arguments = signature.arguments;
                existing.declaration_owner = signature.declaration_owner;
            } else {
                contract.push(signature);
            }
            public_contracts.insert(name, declaration.documentation);
        }
    }
    for contracts in entries.values_mut() {
        contracts.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        contracts.dedup_by(|a, b| a.symbol == b.symbol);
    }
    for (&path, bytes) in delivered.iter().filter(|(path, _)| {
        path.starts_with("generated/include/Rte") || path.starts_with("generated/include/SchM_")
    }) {
        let source = std::str::from_utf8(bytes).map_err(|_| PlanDiagnostic::plan_premise(path))?;
        for declaration in super::module_contracts::declarations(source)? {
            if declaration.name.contains("_MainFunction") {
                let module = declaration.name.split('_').next().unwrap_or("");
                let contracts = entries.entry(module.into()).or_default();
                if !contracts
                    .iter()
                    .any(|entry| entry.symbol == declaration.name)
                {
                    contracts.push(SymbolContract {
                        symbol: declaration.name.clone(),
                        return_type: declaration.result.clone(),
                        arguments: declaration.arguments.clone(),
                        declaration_owner: path.into(),
                        definition_owner: format!("bsw/src/{module}.c"),
                        consumers: Vec::new(),
                    });
                }
            }
            if entries
                .values()
                .flatten()
                .any(|entry| entry.symbol == declaration.name)
            {
                public_contracts.insert(declaration.name, declaration.documentation);
            }
        }
    }
    let natives: BTreeSet<_> = entries
        .values()
        .flatten()
        .flat_map(|s| {
            std::iter::once(s.return_type.clone())
                .chain(s.arguments.iter().map(|a| a.native_type.clone()))
        })
        .filter(|t| t != "void")
        .collect();
    let types: BTreeMap<_, _> = natives
        .into_iter()
        .enumerate()
        .map(|(i, t)| (t, i))
        .collect();
    let aliases = super::module_contracts::native_aliases(delivered)
        .ok_or_else(|| PlanDiagnostic::plan_premise("delivered type aliases"))?;
    let mut elements = String::new();
    for (native, i) in &types {
        // Base types retain their C object size; array parameters decay only in the signature.
        let underlying =
            plan.composition
                .as_ref()
                .and_then(|composition| {
                    composition.types.iter().find(|datatype| {
                        datatype.symbol == *native && datatype.array_length.is_none()
                    })
                })
                .map(|datatype| datatype.native_type.as_str())
                .unwrap_or(native);
        let bits = if let Some(bits) = super::module_contracts::native_bits(native, &aliases) {
            bits
        } else if native.contains('*') || matches!(native.as_str(), "size_t" | "uint64_t") {
            64
        } else if matches!(
            native.as_str(),
            "Std_ReturnType"
                | "uint8"
                | "uint8_t"
                | "StatusType"
                | "NetworkHandleType"
                | "ComM_ModeType"
                | "Rte_ModeType_DcmDiagnosticSessionControl"
        ) || native.starts_with("Rte_ModeType_DcmAuthenticationState_")
        {
            8
        } else if native == "PduLengthType" {
            match plan.communication_runtime.as_ref().map_or(
                super::can_runtime::CommunicationIntegerType::Uint16,
                |runtime| runtime.pdu_length_type,
            ) {
                super::can_runtime::CommunicationIntegerType::Uint8 => 8,
                super::can_runtime::CommunicationIntegerType::Uint16 => 16,
                super::can_runtime::CommunicationIntegerType::Uint32 => 32,
            }
        } else if native == "PduIdType" {
            match plan.communication_runtime.as_ref().map_or(
                super::can_runtime::CommunicationIntegerType::Uint8,
                |runtime| runtime.pdu_id_type,
            ) {
                super::can_runtime::CommunicationIntegerType::Uint8 => 8,
                super::can_runtime::CommunicationIntegerType::Uint16 => 16,
                super::can_runtime::CommunicationIntegerType::Uint32 => 32,
            }
        } else if matches!(
            native.as_str(),
            "Com_SignalIdType" | "CbkHandleIdType" | "uint16_t" | "Can_HwHandleType"
        ) {
            16
        } else if underlying == "uint32"
            || matches!(
                native.as_str(),
                "EcuStatus"
                    | "uint32"
                    | "uint32_t"
                    | "CounterType"
                    | "TickType"
                    | "Can_ControllerStateType"
                    | "CanIf_PduModeType"
                    | "BufReq_ReturnType"
            )
        {
            32
        } else if plan
            .composition
            .as_ref()
            .is_some_and(|composition| composition.array_types.values().any(|name| name == native))
        {
            // native_type validates a fixed four-byte UINT8 array in this profile.
            32
        } else {
            return Err(super::ecu::reject(
                crate::product_message!("backend.integration.artifacts.native_object_size_missing", "native" => native),
            ));
        };
        elements.push_str(&format!("<SW-BASE-TYPE><SHORT-NAME>Native{i}</SHORT-NAME><CATEGORY>FIXED_LENGTH</CATEGORY><BASE-TYPE-SIZE>{bits}</BASE-TYPE-SIZE><BASE-TYPE-ENCODING>NONE</BASE-TYPE-ENCODING><NATIVE-DECLARATION>{}</NATIVE-DECLARATION></SW-BASE-TYPE>", super::xml_text(native)));
    }
    let mut docs = String::from(
        "# Generated host module implementation\n\nThis is the selected single-core C99 host profile. The native target is recorded in target.json. The R24-11 BSW/RTE description locates delivered code and selected actual contracts; it does not certify a complete standard module API or MISRA compliance. Unassigned vendor identity is 0. RTE implementation version follows the latest configuration revision (0.0.0 without a usable revision); other host implementation versions follow this workbench and Arti uses its actual 1.0.0 version API. Source paths are relative to this project. Engineering-object SHORT-LABEL is the filename; DOMAIN maps directory components with dots (os.src means os/src).\n\n| Module | Delivered code and headers |\n| --- | --- |\n",
    );
    let modules: &[&str] = &[
        "Can",
        "Det",
        "CanIf",
        "PduR",
        "Com",
        "CanTp",
        "Dcm",
        "LSduR",
        "ComM",
        "BswM",
        "Ecu_HostBusSM",
        "Os",
        "Arti",
        "Rte",
    ];
    let composition = plan.ecu_composition()?;
    let mut selected_packages = String::new();
    let mut manuals = BTreeMap::new();
    let mut private_entries = String::new();
    for &module in modules {
        let package = if module == "Ecu_HostBusSM" {
            "Autsaro_HostBusSM".into()
        } else {
            format!("AUTOSAR_{module}")
        };
        let mut module_entries = String::new();
        let module_start = elements.len();
        let paths: Vec<_> = sources
            .iter()
            .filter(|p| {
                // Header names are unique across the package's include paths.
                let name = p.rsplit('/').next().unwrap_or(p);
                let os = p.starts_with("os/src/") || p.starts_with("os/include/");
                let generated =
                    p.starts_with("generated/include/") || p.starts_with("generated/src/");
                if name == "Autsaro_Version.h" {
                    module != "Arti"
                } else if module == "Os" {
                    (os && !matches!(name, "Arti.c" | "Arti.h" | "Arti_Version.h"))
                        || (generated
                            && (name.starts_with("Os_")
                                || matches!(
                                    name,
                                    "Rte_Os_Type.h" | "Ecu_Config.c" | "Ecu_TargetConfig.h"
                                )))
                } else if module == "Arti" {
                    os && matches!(name, "Arti.c" | "Arti.h" | "Arti_Version.h")
                } else if module == "Rte" {
                    name == "Rte_Version.h"
                        || (generated && (name.starts_with("Rte") || name.starts_with("SchM_")))
                        || (generated
                            && composition.components.iter().any(|component| {
                                name == format!(
                                    "{}_MemMap.h",
                                    super::short_name(&component.component)
                                )
                            }))
                        || matches!(
                            name,
                            "Ecu_Config.c"
                                | "Ecu_Config.h"
                                | "Ecu_TargetConfig.h"
                                | "Ecu_ProfileConfig.h"
                                | "ComStack_Cfg.h"
                                | "SchM_Main.c"
                                | "SchM_Diagnostic_Host.c"
                                | "SchM_Host.c"
                                | "Ecu_RuntimeConfig.c"
                                | "Ecu_OsHooks.c"
                                | "Ecu_Target.c"
                                | "Os_Hooks.h"
                                | "Det_Callouts.c"
                                | "Det_Externals.h"
                                | "Rte_MemMap.h"
                        )
                } else {
                    (module == "Can"
                        && matches!(
                            p.as_str(),
                            "bsw/include/Can.h"
                                | "bsw/include/Can_CfgTypes.h"
                                | "integration/include/Can_Host.h"
                                | "bsw/include/Can_GeneralTypes.h"
                                | "generated/include/Can_MemMap.h"
                                | "integration/src/Can_Host.c"
                                | "bsw/src/Can_Internal.h"
                                | "integration/src/Can_HostLock.c"
                                | "integration/src/Can_HostLock.h"
                        ))
                        || (module == "Det" && matches!(name, "Det_Callouts.c" | "Det_Externals.h"))
                        || *name == format!("{module}.c")
                        || *name == format!("{module}_PBcfg.c")
                        || *name == format!("{module}_Cfg.h")
                        || (matches!(module, "CanIf" | "LSduR" | "PduR") && name == "Rte_MemMap.h")
                        || *name == format!("{module}.h")
                        || (name.starts_with(&format!("{module}_")) && name.ends_with(".h"))
                        || name.starts_with(&format!("SchM_{module}_Host"))
                        || *name == format!("SchM_{module}.h")
                        || (matches!(module, "Com" | "ComM") && name == "Ecu_RuntimeConfig.c")
                }
            })
            .cloned()
            .collect();
        if !paths.iter().any(|p| p.ends_with(".c")) {
            return Err(super::ecu::reject(
                crate::product_message!("backend.integration.artifacts.module_implementation_missing", "module" => module),
            ));
        }
        let contracts = entries.get(module).map(Vec::as_slice).unwrap_or(&[]);
        let non_entering = non_entering_functions(module);
        let mut refs = String::new();
        let mut entities = String::new();
        let mut events = String::new();
        for entry in contracts {
            let name = &entry.symbol;
            let entry_package =
                if module == "Rte" && !name.starts_with("Rte_") && !name.starts_with("SchM_") {
                    "Autsaro_Rte"
                } else {
                    &package
                };
            let entry_path = { format!("/{entry_package}/BswModuleEntrys/{name}") };
            let mut fields = String::new();
            if entry.return_type != "void" {
                fields.push_str("<RETURN-TYPE>");
                fields.push_str(&argument_fields(
                    "result",
                    &entry.return_type,
                    "OUT",
                    &types,
                ));
                fields.push_str("</RETURN-TYPE>");
            }
            if !entry.arguments.is_empty() {
                fields.push_str("<ARGUMENTS>");
                for a in &entry.arguments {
                    fields.push_str(&argument(&a.name, &a.native_type, &a.direction, &types));
                }
                fields.push_str("</ARGUMENTS>");
            }
            let scheduled = plan
                .schedule
                .entities
                .iter()
                .find(|entity| !entity.application && entity.symbol == *name);
            let callback = name.starts_with("Rte_COMCbk")
                || name.starts_with("Dcm_ComM_")
                || name.starts_with("ComM_DCM_")
                || name.starts_with("PduR_CanTp")
                || name.starts_with("PduR_CanIf")
                || name.starts_with("LSduR_CanIf")
                || name.starts_with("Dcm_Copy")
                || name == "Dcm_StartOfReception"
                || name.starts_with("Dcm_Tp")
                || matches!(
                    name.as_str(),
                    "CanIf_RxIndication"
                        | "CanIf_TxConfirmation"
                        | "CanIf_ControllerModeIndication"
                        | "CanIf_ControllerBusOff"
                        | "CanTp_RxIndication"
                        | "CanTp_TxConfirmation"
                        | "Com_RxIndication"
                        | "Com_TxConfirmation"
                        | "Com_TriggerTransmit"
                        | "ComM_BusSM_ModeIndication"
                        | "BswM_ComM_CurrentMode"
                        | "ComM_EcuM_WakeUpIndication"
                        | "ComM_CommunicationAllowed"
                        | "Ecu_HostBusSM_ControllerModeIndication"
                        | "Ecu_HostBusSM_ControllerBusOff"
                );
            let hook = os_hook(name);
            let callout = hook
                || matches!(
                    name.as_str(),
                    "Ecu_DetRuntimeCallout" | "Ecu_HostBusSM_ApplyMode"
                );
            let main = scheduled.is_some() || name.contains("_MainFunction");
            let call_type = if callout {
                "CALLOUT"
            } else if callback {
                "CALLBACK"
            } else if main {
                "SCHEDULED"
            } else {
                "REGULAR"
            };
            let entity_type = if main {
                "BSW-SCHEDULABLE-ENTITY"
            } else {
                "BSW-CALLED-ENTITY"
            };
            let (reentrant, synchronous) = super::catalog::selected_behavior(name);
            let mut entry_xml = String::new();
            let context = if hook {
                "HOOK"
            } else if main {
                "TASK"
            } else {
                "UNSPECIFIED"
            };
            let implementation_policy = "<SW-SERVICE-IMPL-POLICY>STANDARD</SW-SERVICE-IMPL-POLICY><BSW-ENTRY-KIND>CONCRETE</BSW-ENTRY-KIND>";
            entry_xml.push_str(&format!("<BSW-MODULE-ENTRY><SHORT-NAME>{name}</SHORT-NAME><IS-REENTRANT>{reentrant}</IS-REENTRANT><IS-SYNCHRONOUS>{synchronous}</IS-SYNCHRONOUS><CALL-TYPE>{call_type}</CALL-TYPE><EXECUTION-CONTEXT>{context}</EXECUTION-CONTEXT>{implementation_policy}{fields}</BSW-MODULE-ENTRY>"));
            if entry_package == "Autsaro_Rte" {
                private_entries.push_str(&entry_xml);
            } else {
                module_entries.push_str(&entry_xml);
            }
            refs.push_str(&format!(r#"<BSW-MODULE-ENTRY-REF-CONDITIONAL><BSW-MODULE-ENTRY-REF DEST="BSW-MODULE-ENTRY">{entry_path}</BSW-MODULE-ENTRY-REF></BSW-MODULE-ENTRY-REF-CONDITIONAL>"#));
            let area = exclusive_area(module).filter(|_| {
                if module == "Rte" {
                    name.starts_with("SchM_Switch_Dcm_") || name.starts_with("SchM_Mode_Dcm_")
                } else {
                    !non_entering.contains(name)
                }
            });
            let can_enter = area.map(|area| format!("<CAN-ENTERS><EXCLUSIVE-AREA-REF-CONDITIONAL><EXCLUSIVE-AREA-REF DEST=\"EXCLUSIVE-AREA\">/{package}/{module}/HostBehavior/{area}</EXCLUSIVE-AREA-REF></EXCLUSIVE-AREA-REF-CONDITIONAL></CAN-ENTERS>")).unwrap_or_default();
            let mode_group = |group: &str| {
                format!(
                    "<MODE-DECLARATION-GROUP-PROTOTYPE-REF-CONDITIONAL><MODE-DECLARATION-GROUP-PROTOTYPE-REF DEST=\"MODE-DECLARATION-GROUP-PROTOTYPE\">/AUTOSAR_Dcm/Dcm/{group}</MODE-DECLARATION-GROUP-PROTOTYPE-REF></MODE-DECLARATION-GROUP-PROTOTYPE-REF-CONDITIONAL>"
                )
            };
            let managed = match name.as_str() {
                "Dcm_Init"
                | "Dcm_ResetToDefaultSession"
                | "Dcm_MainFunction"
                | "Dcm_TpRxIndication"
                | "Dcm_TpTxConfirmation" => format!(
                    "<MANAGED-MODE-GROUPS>{}{}</MANAGED-MODE-GROUPS>",
                    mode_group("DcmDiagnosticSessionControl"),
                    mode_group(&plan.ecu_composition()?.authentication_modes()?.group)
                ),
                _ => String::new(),
            };
            let scheduler_prefix = if module == "Dcm" {
                "<SCHEDULER-NAME-PREFIX-REF DEST=\"BSW-SCHEDULER-NAME-PREFIX\">/AUTOSAR_Dcm/Dcm/HostBehavior/Dcm</SCHEDULER-NAME-PREFIX-REF>"
            } else if module == "Rte" && name.starts_with("SchM_") {
                "<SCHEDULER-NAME-PREFIX-REF DEST=\"BSW-SCHEDULER-NAME-PREFIX\">/AUTOSAR_Rte/Rte/HostBehavior/SchM</SCHEDULER-NAME-PREFIX-REF>"
            } else {
                ""
            };
            // constr_4016 only permits REGULAR/CALLBACK for BswCalledEntity;
            // callout prototypes are provided/expected entries, not scheduler entities.
            if !callout {
                entities.push_str(&format!(r#"<{entity_type}><SHORT-NAME>{name}Entity</SHORT-NAME>{can_enter}<IMPLEMENTED-ENTRY-REF DEST="BSW-MODULE-ENTRY">{entry_path}</IMPLEMENTED-ENTRY-REF>{managed}{scheduler_prefix}</{entity_type}>"#));
            }
            if let Some(scheduled) = scheduled {
                let seconds = format!(
                    "{}.{:03}",
                    scheduled.period_ms / 1000,
                    scheduled.period_ms % 1000
                );
                events.push_str(&format!(r#"<BSW-TIMING-EVENT><SHORT-NAME>{name}Timing</SHORT-NAME><STARTS-ON-EVENT-REF DEST="BSW-SCHEDULABLE-ENTITY">/{package}/{module}/HostBehavior/{name}Entity</STARTS-ON-EVENT-REF><PERIOD>{seconds}</PERIOD></BSW-TIMING-EVENT>"#));
            }
        }
        let mut expected = String::new();
        let mut dependencies = String::new();
        let mut imported_entries: Vec<(&str, &str)> = match module {
            "Det" => vec![("Rte", "Ecu_DetRuntimeCallout")],
            "Os" => vec![
                ("Rte", "ErrorHook"),
                ("Rte", "PreTaskHook"),
                ("Rte", "PostTaskHook"),
                ("Rte", "StartupHook"),
                ("Rte", "ShutdownHook"),
            ],
            "Rte" => vec![
                ("Com", "Com_ReceiveSignal"),
                ("Com", "Com_SendSignal"),
                ("Os", "GetCounterValue"),
                ("Os", "GetElapsedValue"),
            ],
            "Can" => vec![
                ("CanIf", "CanIf_RxIndication"),
                ("CanIf", "CanIf_TxConfirmation"),
                ("CanIf", "CanIf_ControllerModeIndication"),
                ("CanIf", "CanIf_ControllerBusOff"),
                ("Det", "Det_ReportRuntimeError"),
            ],
            "CanIf" => vec![
                ("Can", "Can_Write"),
                ("Can", "Can_SetControllerMode"),
                ("Can", "Can_GetControllerMode"),
                ("Can", "Can_GetControllerErrorState"),
                ("Can", "Can_GetControllerRxErrorCounter"),
                ("Can", "Can_GetControllerTxErrorCounter"),
                ("LSduR", "LSduR_CanIfRxIndication"),
                ("LSduR", "LSduR_CanIfTxConfirmation"),
                ("Det", "Det_ReportRuntimeError"),
            ],
            "LSduR" => vec![
                ("CanIf", "CanIf_Transmit"),
                ("PduR", "PduR_CanIfRxIndication"),
                ("PduR", "PduR_CanIfTxConfirmation"),
                ("PduR", "PduR_CanIfTriggerTransmit"),
            ],
            "PduR" => vec![
                ("LSduR", "LSduR_PduRTransmit"),
                ("Com", "Com_RxIndication"),
                ("Com", "Com_TxConfirmation"),
                ("Com", "Com_TriggerTransmit"),
            ],
            "Com" => vec![
                ("Rte", "Rte_COMCbk"),
                ("Rte", "Rte_COMCbkRxTOut"),
                ("PduR", "PduR_ComTransmit"),
            ],
            "ComM" => vec![("BswM", "BswM_ComM_CurrentMode")],
            "Ecu_HostBusSM" => vec![
                ("CanIf", "CanIf_SetControllerMode"),
                ("CanIf", "CanIf_GetControllerMode"),
                ("CanIf", "CanIf_GetControllerErrorState"),
                ("CanIf", "CanIf_SetPduMode"),
                ("CanIf", "CanIf_GetPduMode"),
                ("ComM", "ComM_BusSM_ModeIndication"),
            ],
            _ => vec![],
        };
        if plan.diagnostic_transport.is_some() {
            imported_entries.extend(match module {
                "LSduR" => vec![
                    ("CanTp", "CanTp_RxIndication"),
                    ("CanTp", "CanTp_TxConfirmation"),
                ],
                "PduR" => vec![
                    ("CanTp", "CanTp_Transmit"),
                    ("Dcm", "Dcm_StartOfReception"),
                    ("Dcm", "Dcm_CopyRxData"),
                    ("Dcm", "Dcm_TpRxIndication"),
                    ("Dcm", "Dcm_CopyTxData"),
                    ("Dcm", "Dcm_TpTxConfirmation"),
                ],
                "CanTp" => vec![
                    ("LSduR", "LSduR_CanTpTransmit"),
                    ("PduR", "PduR_CanTpStartOfReception"),
                    ("PduR", "PduR_CanTpCopyRxData"),
                    ("PduR", "PduR_CanTpRxIndication"),
                    ("PduR", "PduR_CanTpCopyTxData"),
                    ("PduR", "PduR_CanTpTxConfirmation"),
                ],
                "Dcm" => vec![
                    ("ComM", "ComM_DCM_ActiveDiagnostic"),
                    ("ComM", "ComM_DCM_InactiveDiagnostic"),
                    ("PduR", "PduR_DcmTransmit"),
                    ("Rte", "SchM_Switch_Dcm_DcmDiagnosticSessionControl"),
                ],
                "ComM" => vec![
                    ("Dcm", "Dcm_ComM_NoComModeEntered"),
                    ("Dcm", "Dcm_ComM_SilentComModeEntered"),
                    ("Dcm", "Dcm_ComM_FullComModeEntered"),
                ],
                _ => vec![],
            });
        }
        // The RTE SWS names the old implementedEntry dependency role. The fixed
        // R24-11 XSD still declares requiredEntry/expectedCallback for this relation.
        let entry_reference = |owner: &str, name: &str| {
            format!(
                "<BSW-MODULE-ENTRY-REF-CONDITIONAL><BSW-MODULE-ENTRY-REF DEST=\"BSW-MODULE-ENTRY\">/AUTOSAR_{owner}/BswModuleEntrys/{name}</BSW-MODULE-ENTRY-REF></BSW-MODULE-ENTRY-REF-CONDITIONAL>"
            )
        };
        let dependency = |owner: &str,
                          name: &str,
                          role: &str,
                          entry_owner: &str|
         -> Result<String, Vec<PlanDiagnostic>> {
            let id = super::catalog::module_id(owner).ok_or_else(|| {
                PlanDiagnostic::plan_premise(&format!("/AUTOSAR_{owner}/{owner}"))
            })?;
            Ok(format!(
                "<BSW-MODULE-DEPENDENCY><SHORT-NAME>{name}Dependency</SHORT-NAME><TARGET-MODULE-ID>{id}</TARGET-MODULE-ID><TARGET-MODULE-REFS><BSW-MODULE-DESCRIPTION-REF-CONDITIONAL><BSW-MODULE-DESCRIPTION-REF DEST=\"BSW-MODULE-DESCRIPTION\">/AUTOSAR_{owner}/{owner}</BSW-MODULE-DESCRIPTION-REF></BSW-MODULE-DESCRIPTION-REF-CONDITIONAL></TARGET-MODULE-REFS><{role}>{}</{role}></BSW-MODULE-DEPENDENCY>",
                entry_reference(entry_owner, name)
            ))
        };
        let mut owners = BTreeSet::new();
        for (owner, name) in imported_entries {
            if module == "Rte" {
                dependencies.push_str(&dependency(owner, name, "REQUIRED-ENTRYS", owner)?);
            } else {
                owners.insert(owner);
            }
            let entry_package =
                if owner == "Rte" && (name == "Ecu_DetRuntimeCallout" || os_hook(name)) {
                    "Autsaro_Rte".to_owned()
                } else {
                    format!("AUTOSAR_{owner}")
                };
            let entry_ref = format!("/{entry_package}/BswModuleEntrys/{name}");
            expected.push_str(&format!("<BSW-MODULE-ENTRY-REF-CONDITIONAL><BSW-MODULE-ENTRY-REF DEST=\"BSW-MODULE-ENTRY\">{entry_ref}</BSW-MODULE-ENTRY-REF></BSW-MODULE-ENTRY-REF-CONDITIONAL>"));
        }
        if module == "Dcm" && plan.diagnostic_transport.is_some() {
            let authentication = plan.ecu_composition()?.authentication_modes()?;
            expected.push_str(&format!("<BSW-MODULE-ENTRY-REF-CONDITIONAL><BSW-MODULE-ENTRY-REF DEST=\"BSW-MODULE-ENTRY\">/AUTOSAR_Rte/BswModuleEntrys/SchM_Switch_Dcm_{}</BSW-MODULE-ENTRY-REF></BSW-MODULE-ENTRY-REF-CONDITIONAL>", authentication.group));
        }
        if !expected.is_empty() {
            expected = format!("<EXPECTED-ENTRYS>{expected}</EXPECTED-ENTRYS>");
        }
        for owner in owners {
            let id = super::catalog::module_id(owner).ok_or_else(|| {
                PlanDiagnostic::plan_premise(&format!("/AUTOSAR_{owner}/{owner}"))
            })?;
            let owner_ref = { format!("/AUTOSAR_{owner}/{owner}") };
            dependencies.push_str(&format!("<BSW-MODULE-DEPENDENCY><SHORT-NAME>{owner}Dependency</SHORT-NAME><TARGET-MODULE-ID>{id}</TARGET-MODULE-ID><TARGET-MODULE-REFS><BSW-MODULE-DESCRIPTION-REF-CONDITIONAL><BSW-MODULE-DESCRIPTION-REF DEST=\"BSW-MODULE-DESCRIPTION\">{owner_ref}</BSW-MODULE-DESCRIPTION-REF></BSW-MODULE-DESCRIPTION-REF-CONDITIONAL></TARGET-MODULE-REFS></BSW-MODULE-DEPENDENCY>"));
        }
        if module == "Rte" {
            for name in ["Rte_COMCbk", "Rte_COMCbkRxTOut"] {
                dependencies.push_str(&dependency("Com", name, "EXPECTED-CALLBACKS", "Rte")?);
            }
            if plan.diagnostic_transport.is_some() {
                for name in [
                    "SchM_Switch_Dcm_DcmDiagnosticSessionControl".to_owned(),
                    format!(
                        "SchM_Switch_Dcm_{}",
                        plan.ecu_composition()?.authentication_modes()?.group
                    ),
                ] {
                    dependencies.push_str(&dependency("Dcm", &name, "EXPECTED-CALLBACKS", "Rte")?);
                }
            }
        }
        if !dependencies.is_empty() {
            dependencies =
                format!("<BSW-MODULE-DEPENDENCYS>{dependencies}</BSW-MODULE-DEPENDENCYS>");
        }

        let area = exclusive_area(module);
        let areas = area.map(|area| format!("<EXCLUSIVE-AREAS><EXCLUSIVE-AREA><SHORT-NAME>{area}</SHORT-NAME></EXCLUSIVE-AREA></EXCLUSIVE-AREAS>")).unwrap_or_default();
        let module_id = super::catalog::module_id(module)
            .map(|id| format!("<MODULE-ID>{id}</MODULE-ID>"))
            .unwrap_or_default();
        let provided = if module == "Dcm" {
            let mut xml = String::from("<PROVIDED-MODE-GROUPS>");
            for definition in &plan.ecu_composition()?.dcm_modes {
                let group = &definition.group;
                xml.push_str(&format!("<MODE-DECLARATION-GROUP-PROTOTYPE><SHORT-NAME>{group}</SHORT-NAME><TYPE-TREF DEST=\"MODE-DECLARATION-GROUP\">/AUTOSAR_Dcm/{group}</TYPE-TREF></MODE-DECLARATION-GROUP-PROTOTYPE>"));
            }
            xml.push_str("</PROVIDED-MODE-GROUPS>");
            xml
        } else {
            String::new()
        };
        let mode_types = if module == "Dcm" {
            "<DATA-TYPE-MAPPING-REFS><DATA-TYPE-MAPPING-REF DEST=\"DATA-TYPE-MAPPING-SET\">/AUTOSAR_Dcm/DcmModeTypes</DATA-TYPE-MAPPING-REF></DATA-TYPE-MAPPING-REFS>"
        } else {
            ""
        };
        let prefixes = if module == "Dcm" {
            "<SCHEDULER-NAME-PREFIXS><BSW-SCHEDULER-NAME-PREFIX><SHORT-NAME>Dcm</SHORT-NAME><SYMBOL>Dcm</SYMBOL></BSW-SCHEDULER-NAME-PREFIX></SCHEDULER-NAME-PREFIXS>"
        } else if module == "Rte" {
            "<SCHEDULER-NAME-PREFIXS><BSW-SCHEDULER-NAME-PREFIX><SHORT-NAME>SchM</SHORT-NAME><SYMBOL>SchM</SYMBOL></BSW-SCHEDULER-NAME-PREFIX></SCHEDULER-NAME-PREFIXS>"
        } else {
            ""
        };
        let super::rte_memory::Description {
            constants,
            variables,
            types: rte_types,
        } = if module == "Rte" {
            super::rte_memory::description(plan)?
        } else {
            super::rte_memory::Description::default()
        };
        elements.push_str(&format!("<BSW-MODULE-DESCRIPTION><SHORT-NAME>{module}</SHORT-NAME><CATEGORY>BSW_MODULE</CATEGORY>{expected}<IMPLEMENTED-ENTRYS>{refs}</IMPLEMENTED-ENTRYS>{module_id}{dependencies}{provided}<INTERNAL-BEHAVIORS><BSW-INTERNAL-BEHAVIOR><SHORT-NAME>HostBehavior</SHORT-NAME>{constants}{mode_types}{areas}{variables}<ENTITYS>{entities}</ENTITYS><EVENTS>{events}</EVENTS>{prefixes}</BSW-INTERNAL-BEHAVIOR></INTERNAL-BEHAVIORS></BSW-MODULE-DESCRIPTION>"));
        elements.push_str(&rte_types);
        if module == "Dcm" {
            let mode_native = types["Rte_ModeType_DcmDiagnosticSessionControl"];
            let mut maps = String::new();
            for definition in &plan.ecu_composition()?.dcm_modes {
                let group = &definition.group;
                let initial = definition.modes[0];
                elements.push_str(&format!("<IMPLEMENTATION-DATA-TYPE><SHORT-NAME>Rte_ModeType_{group}</SHORT-NAME><CATEGORY>VALUE</CATEGORY><SW-DATA-DEF-PROPS><SW-DATA-DEF-PROPS-VARIANTS><SW-DATA-DEF-PROPS-CONDITIONAL><BASE-TYPE-REF DEST=\"SW-BASE-TYPE\">/HostArtifacts/Native{mode_native}</BASE-TYPE-REF></SW-DATA-DEF-PROPS-CONDITIONAL></SW-DATA-DEF-PROPS-VARIANTS></SW-DATA-DEF-PROPS></IMPLEMENTATION-DATA-TYPE><MODE-DECLARATION-GROUP><SHORT-NAME>{group}</SHORT-NAME><CATEGORY>EXPLICIT_ORDER</CATEGORY><INITIAL-MODE-REF DEST=\"MODE-DECLARATION\">/AUTOSAR_Dcm/{group}/{initial}</INITIAL-MODE-REF><MODE-DECLARATIONS>"));
                for (value, mode) in definition.modes.iter().enumerate() {
                    elements.push_str(&format!("<MODE-DECLARATION><SHORT-NAME>{mode}</SHORT-NAME><VALUE>{value}</VALUE></MODE-DECLARATION>"));
                }
                elements.push_str("</MODE-DECLARATIONS><ON-TRANSITION-VALUE>255</ON-TRANSITION-VALUE></MODE-DECLARATION-GROUP>");
                maps.push_str(&format!("<MODE-REQUEST-TYPE-MAP><IMPLEMENTATION-DATA-TYPE-REF DEST=\"IMPLEMENTATION-DATA-TYPE\">/AUTOSAR_Dcm/Rte_ModeType_{group}</IMPLEMENTATION-DATA-TYPE-REF><MODE-GROUP-REF DEST=\"MODE-DECLARATION-GROUP\">/AUTOSAR_Dcm/{group}</MODE-GROUP-REF></MODE-REQUEST-TYPE-MAP>"));
            }
            elements.push_str(&format!("<DATA-TYPE-MAPPING-SET><SHORT-NAME>DcmModeTypes</SHORT-NAME><MODE-REQUEST-TYPE-MAPS>{maps}</MODE-REQUEST-TYPE-MAPS></DATA-TYPE-MAPPING-SET>"));
        }
        let artifacts: String = paths
            .iter()
            .map(|p| {
                format!(
                    "<AUTOSAR-ENGINEERING-OBJECT>{}</AUTOSAR-ENGINEERING-OBJECT>",
                    artifact(p)
                )
            })
            .collect();
        let generated = if matches!(module, "Rte" | "Can" | "Os")
            || (matches!(module, "LSduR" | "PduR"))
        {
            format!("<GENERATED-ARTIFACTS>{}</GENERATED-ARTIFACTS>", paths.iter().map(|p| format!("<DEPENDENCY-ON-ARTIFACT><SHORT-NAME>{}</SHORT-NAME><ARTIFACT-DESCRIPTOR>{}</ARTIFACT-DESCRIPTOR><USAGES><USAGE>COMPILE</USAGE></USAGES></DEPENDENCY-ON-ARTIFACT>", c_name(p), artifact(p))).collect::<String>())
        } else {
            String::new()
        };
        let resource = if module == "Can" {
            format!(
                "<RESOURCE-CONSUMPTION><SHORT-NAME>CodeMapping</SHORT-NAME><MEMORY-SECTIONS><MEMORY-SECTION><SHORT-NAME>Can_CODE</SHORT-NAME><PREFIX-REF DEST=\"SECTION-NAME-PREFIX\">/{package}/CanHostImplementation/CodeMapping/CAN</PREFIX-REF><SYMBOL>CODE</SYMBOL></MEMORY-SECTION><MEMORY-SECTION><SHORT-NAME>Can_VAR_CLEARED_UNSPECIFIED</SHORT-NAME><PREFIX-REF DEST=\"SECTION-NAME-PREFIX\">/{package}/CanHostImplementation/CodeMapping/CAN</PREFIX-REF><SYMBOL>VAR_CLEARED_UNSPECIFIED</SYMBOL></MEMORY-SECTION></MEMORY-SECTIONS><SECTION-NAME-PREFIXS><SECTION-NAME-PREFIX><SHORT-NAME>CAN</SHORT-NAME><IMPLEMENTED-IN-REF DEST=\"DEPENDENCY-ON-ARTIFACT\">/{package}/CanHostImplementation/generated_include_Can_MemMap_h</IMPLEMENTED-IN-REF></SECTION-NAME-PREFIX></SECTION-NAME-PREFIXS></RESOURCE-CONSUMPTION>"
            )
        } else if module == "Rte" {
            let scopes: BTreeSet<_> = super::memmap::rte_sections(plan)?.into_iter().collect();
            let mut sections = String::new();
            let mut prefixes = String::new();
            for scope in scopes
                .iter()
                .map(|(scope, _)| scope)
                .collect::<BTreeSet<_>>()
            {
                let header = "generated/include/Rte_MemMap.h";
                prefixes.push_str(&format!("<SECTION-NAME-PREFIX><SHORT-NAME>RTE_{scope}</SHORT-NAME><IMPLEMENTED-IN-REF DEST=\"DEPENDENCY-ON-ARTIFACT\">/{package}/RteHostImplementation/{}</IMPLEMENTED-IN-REF></SECTION-NAME-PREFIX>", c_name(header)));
            }
            for (scope, segment) in scopes {
                sections.push_str(&format!("<MEMORY-SECTION><SHORT-NAME>{scope}_{segment}</SHORT-NAME><PREFIX-REF DEST=\"SECTION-NAME-PREFIX\">/{package}/RteHostImplementation/CodeMapping/RTE_{scope}</PREFIX-REF><SYMBOL>{segment}</SYMBOL></MEMORY-SECTION>"));
            }
            format!(
                "<RESOURCE-CONSUMPTION><SHORT-NAME>CodeMapping</SHORT-NAME><MEMORY-SECTIONS>{sections}</MEMORY-SECTIONS><SECTION-NAME-PREFIXS>{prefixes}</SECTION-NAME-PREFIXS></RESOURCE-CONSUMPTION>"
            )
        } else if matches!(module, "LSduR" | "PduR") {
            format!(
                "<RESOURCE-CONSUMPTION><SHORT-NAME>CodeMapping</SHORT-NAME><MEMORY-SECTIONS><MEMORY-SECTION><SHORT-NAME>{module}_CONFIG_DATA_PREBUILD_UNSPECIFIED</SHORT-NAME><PREFIX-REF DEST=\"SECTION-NAME-PREFIX\">/{package}/{module}HostImplementation/CodeMapping/RTE_{module}</PREFIX-REF><SYMBOL>CONFIG_DATA_PREBUILD_UNSPECIFIED</SYMBOL></MEMORY-SECTION></MEMORY-SECTIONS><SECTION-NAME-PREFIXS><SECTION-NAME-PREFIX><SHORT-NAME>RTE_{module}</SHORT-NAME><IMPLEMENTED-IN-REF DEST=\"DEPENDENCY-ON-ARTIFACT\">/{package}/{module}HostImplementation/generated_include_Rte_MemMap_h</IMPLEMENTED-IN-REF></SECTION-NAME-PREFIX></SECTION-NAME-PREFIXS></RESOURCE-CONSUMPTION>"
            )
        } else if matches!(module, "Os" | "Arti") {
            // The native ARTI binding shares the OS mapping header, without owning a second copy.
            let sections: &[&str] = if module == "Os" {
                &[
                    "CODE",
                    "CONFIG_DATA_PREBUILD_8",
                    "CONFIG_DATA_PREBUILD_64",
                    "CONST_64",
                    "CONST_8",
                    "CONST_PTR",
                    "VAR_CLEARED_32",
                    "VAR_CLEARED_64",
                    "VAR_CLEARED_8",
                    "VAR_CLEARED_PTR",
                    "VAR_CLEARED_TLS_32",
                    "VAR_CLEARED_TLS_64",
                    "VAR_CLEARED_TLS_8",
                    "VAR_CLEARED_TLS_PTR",
                    "VAR_CLEARED_TLS_UNSPECIFIED",
                    "VAR_CLEARED_UNSPECIFIED",
                    "VAR_INIT_32",
                    "VAR_INIT_8",
                    "VAR_INIT_UNSPECIFIED",
                    "VAR_INIT_VECTOR_PTR",
                ]
            } else {
                &[
                    "CODE",
                    "VAR_CLEARED_32",
                    "VAR_CLEARED_64",
                    "VAR_CLEARED_TLS_64",
                ]
            };
            let sections: String = sections.iter().map(|section| format!("<MEMORY-SECTION><SHORT-NAME>{module}_{section}</SHORT-NAME><PREFIX-REF DEST=\"SECTION-NAME-PREFIX\">/{package}/{module}HostImplementation/CodeMapping/OS</PREFIX-REF><SYMBOL>{section}</SYMBOL></MEMORY-SECTION>")).collect();
            format!(
                "<RESOURCE-CONSUMPTION><SHORT-NAME>CodeMapping</SHORT-NAME><MEMORY-SECTIONS>{sections}</MEMORY-SECTIONS><SECTION-NAME-PREFIXS><SECTION-NAME-PREFIX><SHORT-NAME>OS</SHORT-NAME><IMPLEMENTED-IN-REF DEST=\"DEPENDENCY-ON-ARTIFACT\">/AUTOSAR_Os/OsHostImplementation/generated_include_Os_MemMap_h</IMPLEMENTED-IN-REF></SECTION-NAME-PREFIX></SECTION-NAME-PREFIXS></RESOURCE-CONSUMPTION>"
            )
        } else {
            String::new()
        };
        let version = if module == "Rte" {
            &composition.generation.software_version
        } else if module == "Arti" {
            env!("ARTI_SOFTWARE_VERSION")
        } else {
            env!("AUTSARO_SOFTWARE_VERSION")
        };
        let revision = if module == "Rte" {
            composition
                .generation
                .revision_suffix
                .as_ref()
                .map(|suffix| {
                    format!(
                        "<DESC><L-2 L=\"EN\">Configuration revision suffix: {}</L-2></DESC>",
                        super::xml_text(suffix)
                    )
                })
                .unwrap_or_default()
        } else {
            String::new()
        };
        let mc_support = if module == "Rte" {
            "<MC-SUPPORT><EMULATION-SUPPORTS><MC-SW-EMULATION-METHOD-SUPPORT><CATEGORY>NONE</CATEGORY></MC-SW-EMULATION-METHOD-SUPPORT></EMULATION-SUPPORTS></MC-SUPPORT>"
        } else {
            ""
        };
        elements.push_str(&format!(r#"<BSW-IMPLEMENTATION><SHORT-NAME>{module}HostImplementation</SHORT-NAME>{revision}<CODE-DESCRIPTORS><CODE><SHORT-NAME>DeliveredCode</SHORT-NAME><ARTIFACT-DESCRIPTORS>{artifacts}</ARTIFACT-DESCRIPTORS></CODE></CODE-DESCRIPTORS>{generated}{mc_support}<PROGRAMMING-LANGUAGE>C</PROGRAMMING-LANGUAGE>{resource}<SW-VERSION>{}</SW-VERSION><USED-CODE-GENERATOR>Autosar host ECU generator</USED-CODE-GENERATOR><VENDOR-ID>0</VENDOR-ID><AR-RELEASE-VERSION>4.10.0</AR-RELEASE-VERSION><BEHAVIOR-REF DEST="BSW-INTERNAL-BEHAVIOR">/{package}/{module}/HostBehavior</BEHAVIOR-REF></BSW-IMPLEMENTATION>"#, version));
        let body = elements.split_off(module_start);
        let entry_package = if module_entries.is_empty() {
            String::new()
        } else {
            format!(
                "<AR-PACKAGES><AR-PACKAGE><SHORT-NAME>BswModuleEntrys</SHORT-NAME><ELEMENTS>{module_entries}</ELEMENTS></AR-PACKAGE></AR-PACKAGES>"
            )
        };
        selected_packages.push_str(&format!("<AR-PACKAGE><SHORT-NAME>{package}</SHORT-NAME><ELEMENTS>{body}</ELEMENTS>{entry_package}</AR-PACKAGE>"));

        let manual = super::module_documentation::module(
            plan,
            module,
            &paths,
            contracts,
            &public_contracts,
            &mut manuals,
        )?;
        docs.push_str(&format!(
            "| {manual} | {} |\n",
            paths
                .iter()
                .map(|p| format!("`{p}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some(composition) = &plan.composition {
        for component in &composition.components {
            for mode in &component.diagnostic_mode_ports {
                let group = super::short_name(&mode.mode_group);
                elements.push_str(&format!("<SWC-BSW-MAPPING><SHORT-NAME>ModeMapping{}{group}</SHORT-NAME><BSW-BEHAVIOR-REF DEST=\"BSW-INTERNAL-BEHAVIOR\">/AUTOSAR_Dcm/Dcm/HostBehavior</BSW-BEHAVIOR-REF><SWC-BEHAVIOR-REF DEST=\"SWC-INTERNAL-BEHAVIOR\">{}</SWC-BEHAVIOR-REF><SYNCHRONIZED-MODE-GROUPS><SWC-BSW-SYNCHRONIZED-MODE-GROUP-PROTOTYPE><BSW-MODE-GROUP-REF DEST=\"MODE-DECLARATION-GROUP-PROTOTYPE\">/AUTOSAR_Dcm/Dcm/{group}</BSW-MODE-GROUP-REF><SWC-MODE-GROUP-IREF><CONTEXT-P-PORT-REF DEST=\"P-PORT-PROTOTYPE\">{}</CONTEXT-P-PORT-REF><TARGET-MODE-GROUP-REF DEST=\"MODE-DECLARATION-GROUP-PROTOTYPE\">{}</TARGET-MODE-GROUP-REF></SWC-MODE-GROUP-IREF></SWC-BSW-SYNCHRONIZED-MODE-GROUP-PROTOTYPE></SYNCHRONIZED-MODE-GROUPS></SWC-BSW-MAPPING>", c_name(&component.component), component.behavior, mode.port, mode.prototype));
            }
        }
    }
    docs.push_str("\nRte_Start/Stop and SchM_Init/Start/StartTiming/Deinit own the actual local/freshness resources and authorize the OS bootstrap release. Ordinary RTE calls run on the one owner task. COM/ComM configured mains are defined in Ecu_RuntimeConfig.c and scheduled by the same source-derived rows as their BSW entries. Partially reentrant communication APIs permit concurrent calls on different PduIds/signals; the same object requires synchronization. Com_SendSignal is asynchronous; Rte_COMCbk and Rte_COMCbkRxTOut are synchronous and non-reentrant. Input BSW descriptions remain source provenance, while this file describes selected delivered producers.\n");

    if !private_entries.is_empty() {
        selected_packages.push_str(&format!("<AR-PACKAGE><SHORT-NAME>Autsaro_Rte</SHORT-NAME><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>BswModuleEntrys</SHORT-NAME><ELEMENTS>{private_entries}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AR-PACKAGE>"));
    }
    let description = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>HostArtifacts</SHORT-NAME><ELEMENTS>{elements}</ELEMENTS></AR-PACKAGE>{selected_packages}</AR-PACKAGES></AUTOSAR>
"#
    );
    manuals.extend([
        (
            "generated/descriptions/Host_Implementation.arxml".into(),
            description.into_bytes(),
        ),
        (
            "generated/descriptions/MODULES.md".into(),
            docs.into_bytes(),
        ),
    ]);
    Ok(manuals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mismatched_schm_declarations_and_calls_are_rejected_before_delivery() {
        let fixture = [
            ("BswM", "BSWM_STATE"),
            ("Can", "CAN_STATE"),
            ("CanIf", "CANIF_STATE"),
            ("CanTp", "CANTP_STATE"),
            ("Com", "COM_STATE"),
            ("ComM", "COMM_STATE"),
            ("Dcm", "DCM_STATE"),
            ("Det", "DET_STATE"),
            ("LSduR", "LSDUR_STATE"),
            ("PduR", "PDUR_STATE"),
        ];
        let mut headers: BTreeMap<String, Vec<u8>> = fixture.into_iter().map(|(module, area)| (
            format!("generated/include/SchM_{module}.h"),
            format!("void SchM_Enter_{module}_{area}(void);\nvoid SchM_Exit_{module}_{area}(void);\n").into_bytes(),
        )).collect();
        let validate = |headers: &BTreeMap<String, Vec<u8>>| {
            validate_areas(
                &headers
                    .iter()
                    .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
                    .collect(),
            )
        };
        assert!(validate(&headers).is_ok());
        let path = "generated/include/SchM_Can.h";
        let original = headers[path].clone();
        headers.insert(
            path.into(),
            b"void SchM_Enter_Can_CAN_STATE(void);".to_vec(),
        );
        assert!(validate(&headers).is_err());
        headers.insert(path.into(), original);
        headers.insert(
            "bsw/src/Can.c".into(),
            b"void Bad(void) { SchM_Enter_Can_WRONG(); }".to_vec(),
        );
        assert!(validate(&headers).is_err());
    }
}
