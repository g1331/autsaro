use super::catalog::SymbolContract;
use super::component::c_name;
use super::{ContractArgument, PlanDescription};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn artifact(path: &str) -> String {
    format!(
        "<SHORT-LABEL>{}</SHORT-LABEL><CATEGORY>{}</CATEGORY><DOMAIN>{}</DOMAIN>",
        xml(path.rsplit('/').next().unwrap()),
        if path.ends_with(".c") {
            "SWSRC"
        } else {
            "SWHDR"
        },
        xml(&path.rsplit_once('/').unwrap().0.replace('/', "."))
    )
}
fn argument(name: &str, native: &str, direction: &str, types: &BTreeMap<String, usize>) -> String {
    format!(
        r#"<SW-SERVICE-ARG><SHORT-NAME>{}</SHORT-NAME><CATEGORY>VALUE</CATEGORY><DIRECTION>{direction}</DIRECTION><SW-DATA-DEF-PROPS><SW-DATA-DEF-PROPS-VARIANTS><SW-DATA-DEF-PROPS-CONDITIONAL><BASE-TYPE-REF DEST="SW-BASE-TYPE">/HostArtifacts/Native{}</BASE-TYPE-REF></SW-DATA-DEF-PROPS-CONDITIONAL></SW-DATA-DEF-PROPS-VARIANTS></SW-DATA-DEF-PROPS></SW-SERVICE-ARG>"#,
        xml(name),
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

/// Product implementation descriptions reference only the sealed project's actual files.
/// They describe this host profile, not an ICC3 or complete standard API certification.
pub(super) fn files<T>(
    plan: &PlanDescription,
    sources: &BTreeMap<String, T>,
) -> Result<BTreeMap<String, Vec<u8>>, crate::message::LocalizedText> {
    let component = plan.component.as_ref();
    let mut entries: BTreeMap<String, Vec<SymbolContract>> = BTreeMap::new();
    for symbol in &plan.symbols {
        if plan.multi.is_some() && symbol.definition_owner.starts_with("application/") {
            continue;
        }
        if symbol.definition_owner == "src/Rte.c" {
            entries
                .entry("Rte".into())
                .or_default()
                .push(symbol.clone());
        } else if plan.multi.is_some() && symbol.definition_owner == "src/Ecu_RuntimeConfig.c" {
            let module = if symbol.symbol.starts_with("ComM_") {
                "ComM"
            } else {
                "Com"
            };
            entries
                .entry(module.into())
                .or_default()
                .push(symbol.clone());
        } else if let Some(path) = symbol
            .definition_owner
            .strip_prefix("runtime/multi/src/")
            .or_else(|| symbol.definition_owner.strip_prefix("runtime/src/"))
        {
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
    if plan.multi.is_some() {
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
            symbol.declaration_owner = "runtime/include/Os.h".into();
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
                    symbol.definition_owner = if module == "Can" {
                        "runtime/src/Can.c".into()
                    } else {
                        format!("runtime/multi/src/{module}.c")
                    };
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
            "runtime/include/Can.h",
        );
        imported(
            "Can",
            "Can_SetControllerMode",
            "Std_ReturnType",
            &[
                ("Controller", "uint8", "IN"),
                ("Transition", "Can_ControllerStateType", "IN"),
            ],
            "runtime/include/Can.h",
        );
        imported(
            "Can",
            "Can_GetControllerMode",
            "Std_ReturnType",
            &[
                ("Controller", "uint8", "IN"),
                ("Value", "Can_ControllerStateType *", "OUT"),
            ],
            "runtime/include/Can.h",
        );
        imported(
            "Can",
            "Can_GetControllerErrorState",
            "Std_ReturnType",
            &[
                ("Controller", "uint8", "IN"),
                ("Value", "Can_ErrorStateType *", "OUT"),
            ],
            "runtime/include/Can.h",
        );
        imported(
            "Can",
            "Can_GetControllerRxErrorCounter",
            "Std_ReturnType",
            &[("Controller", "uint8", "IN"), ("Value", "uint8 *", "OUT")],
            "runtime/include/Can.h",
        );
        imported(
            "Can",
            "Can_GetControllerTxErrorCounter",
            "Std_ReturnType",
            &[("Controller", "uint8", "IN"), ("Value", "uint8 *", "OUT")],
            "runtime/include/Can.h",
        );
        imported(
            "CanIf",
            "CanIf_RxIndication",
            "void",
            &[
                ("Mailbox", "const Can_HwType *", "IN"),
                ("PduInfoPtr", "const PduInfoType *", "IN"),
            ],
            "runtime/multi/include/CanIf.h",
        );
        imported(
            "CanIf",
            "CanIf_TxConfirmation",
            "void",
            &[("CanTxPduId", "PduIdType", "IN")],
            "runtime/multi/include/CanIf.h",
        );
        imported(
            "CanIf",
            "CanIf_ControllerModeIndication",
            "void",
            &[
                ("ControllerId", "uint8", "IN"),
                ("ControllerMode", "Can_ControllerStateType", "IN"),
            ],
            "runtime/multi/include/CanIf.h",
        );
        imported(
            "CanIf",
            "CanIf_ControllerBusOff",
            "void",
            &[("ControllerId", "uint8", "IN")],
            "runtime/multi/include/CanIf.h",
        );
        imported(
            "ComM",
            "ComM_DCM_ActiveDiagnostic",
            "void",
            &[("Channel", "NetworkHandleType", "IN")],
            "runtime/multi/include/ComM_Dcm.h",
        );
        imported(
            "ComM",
            "ComM_DCM_InactiveDiagnostic",
            "void",
            &[("Channel", "NetworkHandleType", "IN")],
            "runtime/multi/include/ComM_Dcm.h",
        );
        imported(
            "Com",
            "Com_RxIndication",
            "void",
            &[
                ("PduId", "PduIdType", "IN"),
                ("PduInfoPtr", "const PduInfoType *", "IN"),
            ],
            "runtime/multi/include/Com.h",
        );
        imported(
            "Com",
            "Com_TxConfirmation",
            "void",
            &[
                ("PduId", "PduIdType", "IN"),
                ("Result", "Std_ReturnType", "IN"),
            ],
            "runtime/multi/include/Com.h",
        );
        imported(
            "Com",
            "Com_TriggerTransmit",
            "Std_ReturnType",
            &[
                ("PduId", "PduIdType", "IN"),
                ("PduInfoPtr", "PduInfoType *", "INOUT"),
            ],
            "runtime/multi/include/Com.h",
        );
        imported(
            "ComM",
            "ComM_BusSM_ModeIndication",
            "void",
            &[
                ("Channel", "NetworkHandleType", "IN"),
                ("ComMode", "ComM_ModeType", "IN"),
            ],
            "runtime/multi/include/ComM.h",
        );
        imported(
            "BswM",
            "BswM_ComM_CurrentMode",
            "void",
            &[
                ("Network", "NetworkHandleType", "IN"),
                ("RequestedMode", "ComM_ModeType", "IN"),
            ],
            "runtime/multi/include/BswM_ComM.h",
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
                "runtime/multi/include/CanTp.h",
            );
            imported(
                "CanTp",
                "CanTp_RxIndication",
                "void",
                &[
                    ("RxPduId", "PduIdType", "IN"),
                    ("PduInfoPtr", "const PduInfoType *", "IN"),
                ],
                "runtime/multi/include/CanTp.h",
            );
            imported(
                "CanTp",
                "CanTp_TxConfirmation",
                "void",
                &[
                    ("TxPduId", "PduIdType", "IN"),
                    ("Result", "Std_ReturnType", "IN"),
                ],
                "runtime/multi/include/CanTp.h",
            );
            imported(
                "PduR",
                "PduR_DcmTransmit",
                "Std_ReturnType",
                &[
                    ("TxPduId", "PduIdType", "IN"),
                    ("PduInfoPtr", "const PduInfoType *", "IN"),
                ],
                "runtime/multi/include/PduR_Dcm.h",
            );
        }
        if plan.diagnostic_transport.is_some() {
            for (owner, prefix, header) in [
                ("PduR", "PduR_CanTp", "runtime/multi/include/PduR_CanTp.h"),
                ("Dcm", "Dcm_", "runtime/multi/include/Dcm.h"),
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
                "runtime/multi/include/PduR_CanTp.h",
            );
        }
        for (owner, prefix) in [("LSduR", "LSduR_CanIf"), ("PduR", "PduR_CanIf")] {
            let header = format!("runtime/multi/include/{owner}_CanIf.h");
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
                &format!("runtime/multi/include/LSduR_{upper}.h"),
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
                "runtime/multi/include/CanIf.h",
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
                "runtime/multi/include/Dcm_ComM.h"
            } else {
                "runtime/multi/include/Dcm.h"
            }
            .into();
            symbol.definition_owner = "runtime/multi/src/Dcm.c".into();
            dcm.push(symbol);
        }
    }
    let rte = entries.entry("Rte".into()).or_default();
    if plan.multi.is_some() {
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
            symbol.declaration_owner = "runtime/multi/include/SchM_Dcm.h".into();
            symbol.definition_owner = "src/SchM.c".into();
            rte.push(symbol);
        }
        let authentication = plan
            .multi
            .as_ref()
            .unwrap()
            .dcm_modes
            .iter()
            .find(|definition| definition.group.starts_with("DcmAuthenticationState_"))
            .unwrap();
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
            symbol.definition_owner = "src/SchM.c".into();
            rte.push(symbol);
        }
    } else {
        rte.push(rte_symbol("Ecu_TargetInitializeRte", "uint8_t", &[]));
        rte.push(rte_symbol(
            "Ecu_TargetReadDid",
            "Std_ReturnType",
            &[("data", "uint8_t *", "OUT")],
        ));
    }
    let counter = c_name(plan.schedule.counter.rsplit('/').next().unwrap());
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
    rte.push(rte_symbol(
        "Rte_Call_OsService_GetCounterValue",
        "StatusType",
        &[("Value", "TimeInMicrosecondsType *", "OUT")],
    ));
    rte.push(rte_symbol(
        "Rte_Call_OsService_GetElapsedValue",
        "StatusType",
        &[
            ("Value", "TimeInMicrosecondsType *", "INOUT"),
            ("ElapsedValue", "TimeInMicrosecondsType *", "OUT"),
        ],
    ));
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
    let mut elements = String::new();
    for (native, i) in &types {
        // Base types retain their C object size; array parameters decay only in the signature.
        let bits = if native.contains('*') || matches!(native.as_str(), "size_t" | "uint64_t") {
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
            match plan.communication_runtime.as_ref().unwrap().pdu_length_type {
                super::multi_bsw::CommunicationIntegerType::Uint8 => 8,
                super::multi_bsw::CommunicationIntegerType::Uint16 => 16,
                super::multi_bsw::CommunicationIntegerType::Uint32 => 32,
            }
        } else if native == "PduIdType" {
            match plan.communication_runtime.as_ref().unwrap().pdu_id_type {
                super::multi_bsw::CommunicationIntegerType::Uint8 => 8,
                super::multi_bsw::CommunicationIntegerType::Uint16 => 16,
                super::multi_bsw::CommunicationIntegerType::Uint32 => 32,
            }
        } else if matches!(
            native.as_str(),
            "Com_SignalIdType" | "CbkHandleIdType" | "uint16_t" | "Can_HwHandleType"
        ) {
            16
        } else if matches!(
            native.as_str(),
            "EcuStatus"
                | "uint32"
                | "uint32_t"
                | "CounterType"
                | "TickType"
                | "Can_ControllerStateType"
                | "CanIf_PduModeType"
                | "BufReq_ReturnType"
        ) {
            32
        } else if let Some(component) = component.filter(|component| {
            native == &c_name(component.service.array_type.rsplit('/').next().unwrap())
        }) {
            component.service.array_length * 8
        } else if plan
            .multi
            .as_ref()
            .is_some_and(|multi| multi.array_types.values().any(|name| name == native))
        {
            // native_type validates a fixed four-byte UINT8 array in this profile.
            32
        } else {
            return Err(
                crate::product_message!("backend.integration.artifacts.native_object_size_missing", "native" => native),
            );
        };
        write!(elements, "<SW-BASE-TYPE><SHORT-NAME>Native{i}</SHORT-NAME><CATEGORY>FIXED_LENGTH</CATEGORY><BASE-TYPE-SIZE>{bits}</BASE-TYPE-SIZE><BASE-TYPE-ENCODING>NONE</BASE-TYPE-ENCODING><NATIVE-DECLARATION>{}</NATIVE-DECLARATION></SW-BASE-TYPE>", xml(native)).unwrap();
    }
    let mut docs = String::from(
        "# Generated host module implementation\n\nThis is the selected single-core C99 host profile. The native target is recorded in target.json. The R24-11 BSW/RTE description locates delivered code and selected actual contracts; it does not certify a complete standard module API or MISRA compliance. Unassigned vendor identity is 0. Host implementation version follows this workbench; Arti uses its actual 1.0.0 version API. Source paths are relative to this project. Engineering-object SHORT-LABEL is the filename; DOMAIN maps directory components with dots (os.src means os/src).\n\n| Module | Delivered code and headers |\n| --- | --- |\n",
    );
    let modules: &[&str] = if plan.multi.is_some() {
        &[
            "Can",
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
        ]
    } else {
        &[
            "Can", "CanIf", "PduR", "Com", "CanTp", "Dcm", "Dem", "NvM", "LSduR", "Security", "Os",
            "Arti", "Rte",
        ]
    };
    let mut selected_packages = String::new();
    let mut private_entries = String::new();
    for &module in modules {
        let package = if plan.multi.is_some() {
            if module == "Ecu_HostBusSM" {
                "Autsaro_HostBusSM".into()
            } else {
                format!("AUTOSAR_{module}")
            }
        } else {
            "HostArtifacts".into()
        };
        let mut module_entries = String::new();
        let module_start = elements.len();
        let paths: Vec<_> = sources
            .keys()
            .filter(|p| {
                if module == "Os" {
                    (p.starts_with("os/src/") && !p.ends_with("/Arti.c"))
                        || (p.starts_with("os/include/") && !p.ends_with("/Arti.h"))
                        || matches!(p.as_str(), "os/Os.h" | "include/Os.h" | "bsw-origin/Os.h")
                } else if module == "Arti" {
                    p.as_str() == "os/src/Arti.c" || p.as_str() == "os/include/Arti.h"
                } else if module == "Rte" {
                    p.starts_with("include/Rte")
                        || p.as_str() == "src/Rte.c"
                        || p.as_str() == "src/Rte_OsService.c"
                        || (plan.multi.is_some()
                            && matches!(
                                p.as_str(),
                                "src/SchM.c"
                                    | "src/SchM_Diagnostic_Host.c"
                                    | "src/SchM_Host.c"
                                    | "src/Ecu_RuntimeConfig.c"
                                    | "include/Ecu_MemMap.h"
                            ))
                } else {
                    p.as_str() == format!("src/{module}.c")
                        || p.as_str() == format!("src/{module}_PBcfg.c")
                        || p.as_str() == format!("include/{module}_Cfg.h")
                        || (matches!(module, "LSduR" | "PduR")
                            && p.as_str() == "include/Ecu_MemMap.h")
                        || p.as_str() == format!("include/{module}.h")
                        || p.as_str() == format!("include/SchM_{module}.h")
                        || (plan.multi.is_some()
                            && matches!(module, "Com" | "ComM")
                            && p.as_str() == "src/Ecu_RuntimeConfig.c")
                }
            })
            .cloned()
            .collect();
        if !paths.iter().any(|p| p.ends_with(".c")) {
            return Err(
                crate::product_message!("backend.integration.artifacts.module_implementation_missing", "module" => module),
            );
        }
        let contracts = entries.get(module).map(Vec::as_slice).unwrap_or(&[]);
        let mut refs = String::new();
        let mut entities = String::new();
        let mut events = String::new();
        for entry in contracts {
            let name = &entry.symbol;
            let entry_package = if plan.multi.is_some()
                && module == "Rte"
                && !name.starts_with("Rte_")
                && !name.starts_with("SchM_")
            {
                "Autsaro_Rte"
            } else {
                &package
            };
            let entry_path = if plan.multi.is_some() {
                format!("/{entry_package}/BswModuleEntrys/{name}")
            } else {
                format!("/HostArtifacts/{name}")
            };
            let mut fields = String::new();
            if entry.return_type != "void" {
                fields.push_str("<RETURN-TYPE>");
                fields.push_str(
                    argument("result", &entry.return_type, "OUT", &types)
                        .strip_prefix("<SW-SERVICE-ARG>")
                        .unwrap()
                        .strip_suffix("</SW-SERVICE-ARG>")
                        .unwrap(),
                );
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
            let callback = plan.multi.is_some()
                && (name.starts_with("Rte_COMCbk")
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
                    ));
            let call_type = if callback {
                "CALLBACK"
            } else if scheduled.is_some() {
                "SCHEDULED"
            } else {
                "REGULAR"
            };
            let entity_type = if scheduled.is_some() {
                "BSW-SCHEDULABLE-ENTITY"
            } else {
                "BSW-CALLED-ENTITY"
            };
            let (reentrant, synchronous) = if plan.multi.is_some() {
                super::catalog::selected_behavior(name)
            } else {
                (false, true)
            };
            let mut entry_xml = String::new();
            let implementation_policy = if plan.multi.is_some() {
                "<SW-SERVICE-IMPL-POLICY>STANDARD</SW-SERVICE-IMPL-POLICY><BSW-ENTRY-KIND>CONCRETE</BSW-ENTRY-KIND>"
            } else {
                ""
            };
            write!(entry_xml, "<BSW-MODULE-ENTRY><SHORT-NAME>{name}</SHORT-NAME><IS-REENTRANT>{reentrant}</IS-REENTRANT><IS-SYNCHRONOUS>{synchronous}</IS-SYNCHRONOUS><CALL-TYPE>{call_type}</CALL-TYPE><EXECUTION-CONTEXT>UNSPECIFIED</EXECUTION-CONTEXT>{implementation_policy}{fields}</BSW-MODULE-ENTRY>").unwrap();
            if plan.multi.is_some() && entry_package == "Autsaro_Rte" {
                private_entries.push_str(&entry_xml);
            } else if plan.multi.is_some() {
                module_entries.push_str(&entry_xml);
            } else {
                elements.push_str(&entry_xml);
            }
            write!(refs, r#"<BSW-MODULE-ENTRY-REF-CONDITIONAL><BSW-MODULE-ENTRY-REF DEST="BSW-MODULE-ENTRY">{entry_path}</BSW-MODULE-ENTRY-REF></BSW-MODULE-ENTRY-REF-CONDITIONAL>"#).unwrap();
            let area = if plan.multi.is_some() {
                match module {
                    "CanIf" => Some("CANIF_STATE"),
                    "CanTp" => Some("CANTP_STATE"),
                    "ComM" => Some("COMM_STATE"),
                    "Dcm" => Some("DCM_STATE"),
                    _ => None,
                }
            } else {
                None
            };
            let can_enter = area.map(|area| format!("<CAN-ENTERS><EXCLUSIVE-AREA-REF-CONDITIONAL><EXCLUSIVE-AREA-REF DEST=\"EXCLUSIVE-AREA\">/{package}/{module}/HostBehavior/{area}</EXCLUSIVE-AREA-REF></EXCLUSIVE-AREA-REF-CONDITIONAL></CAN-ENTERS>")).unwrap_or_default();
            let managed = if plan.multi.is_some()
                && matches!(
                    name.as_str(),
                    "Dcm_Init"
                        | "Dcm_ResetToDefaultSession"
                        | "Dcm_MainFunction"
                        | "Dcm_TpTxConfirmation"
                ) {
                "<MANAGED-MODE-GROUPS><MODE-DECLARATION-GROUP-PROTOTYPE-REF-CONDITIONAL><MODE-DECLARATION-GROUP-PROTOTYPE-REF DEST=\"MODE-DECLARATION-GROUP-PROTOTYPE\">/AUTOSAR_Dcm/Dcm/DcmDiagnosticSessionControl</MODE-DECLARATION-GROUP-PROTOTYPE-REF></MODE-DECLARATION-GROUP-PROTOTYPE-REF-CONDITIONAL></MANAGED-MODE-GROUPS>"
            } else {
                ""
            };
            let managed = if plan.multi.is_some() && name == "Dcm_Init" {
                let authentication = plan
                    .multi
                    .as_ref()
                    .unwrap()
                    .dcm_modes
                    .iter()
                    .find(|definition| definition.group.starts_with("DcmAuthenticationState_"))
                    .unwrap();
                managed.replace("</MANAGED-MODE-GROUPS>", &format!("<MODE-DECLARATION-GROUP-PROTOTYPE-REF-CONDITIONAL><MODE-DECLARATION-GROUP-PROTOTYPE-REF DEST=\"MODE-DECLARATION-GROUP-PROTOTYPE\">/AUTOSAR_Dcm/Dcm/{}</MODE-DECLARATION-GROUP-PROTOTYPE-REF></MODE-DECLARATION-GROUP-PROTOTYPE-REF-CONDITIONAL></MANAGED-MODE-GROUPS>", authentication.group))
            } else {
                managed.to_string()
            };
            let scheduler_prefix = if plan.multi.is_some() && module == "Dcm" {
                "<SCHEDULER-NAME-PREFIX-REF DEST=\"BSW-SCHEDULER-NAME-PREFIX\">/AUTOSAR_Dcm/Dcm/HostBehavior/Dcm</SCHEDULER-NAME-PREFIX-REF>"
            } else if plan.multi.is_some() && module == "Rte" && name.starts_with("SchM_") {
                "<SCHEDULER-NAME-PREFIX-REF DEST=\"BSW-SCHEDULER-NAME-PREFIX\">/AUTOSAR_Rte/Rte/HostBehavior/SchM</SCHEDULER-NAME-PREFIX-REF>"
            } else {
                ""
            };
            write!(entities, r#"<{entity_type}><SHORT-NAME>{name}Entity</SHORT-NAME>{can_enter}<IMPLEMENTED-ENTRY-REF DEST="BSW-MODULE-ENTRY">{entry_path}</IMPLEMENTED-ENTRY-REF>{managed}{scheduler_prefix}</{entity_type}>"#).unwrap();
            if let Some(scheduled) = scheduled {
                let seconds = format!(
                    "{}.{:03}",
                    scheduled.period_ms / 1000,
                    scheduled.period_ms % 1000
                );
                write!(events, r#"<BSW-TIMING-EVENT><SHORT-NAME>{name}Timing</SHORT-NAME><STARTS-ON-EVENT-REF DEST="BSW-SCHEDULABLE-ENTITY">/{package}/{module}/HostBehavior/{name}Entity</STARTS-ON-EVENT-REF><PERIOD>{seconds}</PERIOD></BSW-TIMING-EVENT>"#).unwrap();
            }
        }
        let mut expected = String::new();
        let mut dependencies = String::new();
        if plan.multi.is_some() {
            let mut imported_entries: Vec<(&str, &str)> = match module {
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
            let mut owners = BTreeSet::new();
            for (owner, name) in imported_entries {
                owners.insert(owner);
                write!(expected, "<BSW-MODULE-ENTRY-REF-CONDITIONAL><BSW-MODULE-ENTRY-REF DEST=\"BSW-MODULE-ENTRY\">/AUTOSAR_{owner}/BswModuleEntrys/{name}</BSW-MODULE-ENTRY-REF></BSW-MODULE-ENTRY-REF-CONDITIONAL>").unwrap();
            }
            if module == "Dcm" && plan.diagnostic_transport.is_some() {
                let authentication = plan
                    .multi
                    .as_ref()
                    .unwrap()
                    .dcm_modes
                    .iter()
                    .find(|mode| mode.group.starts_with("DcmAuthenticationState_"))
                    .unwrap();
                write!(expected, "<BSW-MODULE-ENTRY-REF-CONDITIONAL><BSW-MODULE-ENTRY-REF DEST=\"BSW-MODULE-ENTRY\">/AUTOSAR_Rte/BswModuleEntrys/SchM_Switch_Dcm_{}</BSW-MODULE-ENTRY-REF></BSW-MODULE-ENTRY-REF-CONDITIONAL>", authentication.group).unwrap();
            }
            if !expected.is_empty() {
                expected = format!("<EXPECTED-ENTRYS>{expected}</EXPECTED-ENTRYS>");
            }
            for owner in owners {
                let id = super::catalog::module_id(owner).unwrap();
                write!(dependencies, "<BSW-MODULE-DEPENDENCY><SHORT-NAME>{owner}Dependency</SHORT-NAME><TARGET-MODULE-ID>{id}</TARGET-MODULE-ID><TARGET-MODULE-REFS><BSW-MODULE-DESCRIPTION-REF-CONDITIONAL><BSW-MODULE-DESCRIPTION-REF DEST=\"BSW-MODULE-DESCRIPTION\">/AUTOSAR_{owner}/{owner}</BSW-MODULE-DESCRIPTION-REF></BSW-MODULE-DESCRIPTION-REF-CONDITIONAL></TARGET-MODULE-REFS></BSW-MODULE-DEPENDENCY>").unwrap();
            }
            if !dependencies.is_empty() {
                dependencies =
                    format!("<BSW-MODULE-DEPENDENCYS>{dependencies}</BSW-MODULE-DEPENDENCYS>");
            }
        }
        let area = if plan.multi.is_some() {
            match module {
                "CanIf" => Some("CANIF_STATE"),
                "CanTp" => Some("CANTP_STATE"),
                "ComM" => Some("COMM_STATE"),
                "Dcm" => Some("DCM_STATE"),
                _ => None,
            }
        } else {
            None
        };
        let areas = area.map(|area| format!("<EXCLUSIVE-AREAS><EXCLUSIVE-AREA><SHORT-NAME>{area}</SHORT-NAME></EXCLUSIVE-AREA></EXCLUSIVE-AREAS>")).unwrap_or_default();
        let module_id = if plan.multi.is_some() {
            super::catalog::module_id(module)
                .map(|id| format!("<MODULE-ID>{id}</MODULE-ID>"))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let provided = if plan.multi.is_some() && module == "Dcm" {
            let mut xml = String::from("<PROVIDED-MODE-GROUPS>");
            for definition in &plan.multi.as_ref().unwrap().dcm_modes {
                let group = &definition.group;
                write!(xml, "<MODE-DECLARATION-GROUP-PROTOTYPE><SHORT-NAME>{group}</SHORT-NAME><TYPE-TREF DEST=\"MODE-DECLARATION-GROUP\">/AUTOSAR_Dcm/{group}</TYPE-TREF></MODE-DECLARATION-GROUP-PROTOTYPE>").unwrap();
            }
            xml.push_str("</PROVIDED-MODE-GROUPS>");
            xml
        } else {
            String::new()
        };
        let mode_types = if plan.multi.is_some() && module == "Dcm" {
            "<DATA-TYPE-MAPPING-REFS><DATA-TYPE-MAPPING-REF DEST=\"DATA-TYPE-MAPPING-SET\">/AUTOSAR_Dcm/DcmModeTypes</DATA-TYPE-MAPPING-REF></DATA-TYPE-MAPPING-REFS>"
        } else {
            ""
        };
        let prefixes = if plan.multi.is_some() && module == "Dcm" {
            "<SCHEDULER-NAME-PREFIXS><BSW-SCHEDULER-NAME-PREFIX><SHORT-NAME>Dcm</SHORT-NAME><SYMBOL>Dcm</SYMBOL></BSW-SCHEDULER-NAME-PREFIX></SCHEDULER-NAME-PREFIXS>"
        } else if plan.multi.is_some() && module == "Rte" {
            "<SCHEDULER-NAME-PREFIXS><BSW-SCHEDULER-NAME-PREFIX><SHORT-NAME>SchM</SHORT-NAME><SYMBOL>SchM</SYMBOL></BSW-SCHEDULER-NAME-PREFIX></SCHEDULER-NAME-PREFIXS>"
        } else {
            ""
        };
        write!(elements, "<BSW-MODULE-DESCRIPTION><SHORT-NAME>{module}</SHORT-NAME><CATEGORY>BSW_MODULE</CATEGORY>{expected}<IMPLEMENTED-ENTRYS>{refs}</IMPLEMENTED-ENTRYS>{module_id}{dependencies}{provided}<INTERNAL-BEHAVIORS><BSW-INTERNAL-BEHAVIOR><SHORT-NAME>HostBehavior</SHORT-NAME>{mode_types}{areas}<ENTITYS>{entities}</ENTITYS><EVENTS>{events}</EVENTS>{prefixes}</BSW-INTERNAL-BEHAVIOR></INTERNAL-BEHAVIORS></BSW-MODULE-DESCRIPTION>").unwrap();
        if plan.multi.is_some() && module == "Dcm" {
            let mode_native = types["Rte_ModeType_DcmDiagnosticSessionControl"];
            let mut maps = String::new();
            for definition in &plan.multi.as_ref().unwrap().dcm_modes {
                let group = &definition.group;
                let initial = definition.modes[0];
                write!(elements, "<IMPLEMENTATION-DATA-TYPE><SHORT-NAME>Rte_ModeType_{group}</SHORT-NAME><CATEGORY>VALUE</CATEGORY><SW-DATA-DEF-PROPS><SW-DATA-DEF-PROPS-VARIANTS><SW-DATA-DEF-PROPS-CONDITIONAL><BASE-TYPE-REF DEST=\"SW-BASE-TYPE\">/HostArtifacts/Native{mode_native}</BASE-TYPE-REF></SW-DATA-DEF-PROPS-CONDITIONAL></SW-DATA-DEF-PROPS-VARIANTS></SW-DATA-DEF-PROPS></IMPLEMENTATION-DATA-TYPE><MODE-DECLARATION-GROUP><SHORT-NAME>{group}</SHORT-NAME><CATEGORY>EXPLICIT_ORDER</CATEGORY><INITIAL-MODE-REF DEST=\"MODE-DECLARATION\">/AUTOSAR_Dcm/{group}/{initial}</INITIAL-MODE-REF><MODE-DECLARATIONS>").unwrap();
                for (value, mode) in definition.modes.iter().enumerate() {
                    write!(elements, "<MODE-DECLARATION><SHORT-NAME>{mode}</SHORT-NAME><VALUE>{value}</VALUE></MODE-DECLARATION>").unwrap();
                }
                elements.push_str("</MODE-DECLARATIONS><ON-TRANSITION-VALUE>255</ON-TRANSITION-VALUE></MODE-DECLARATION-GROUP>");
                write!(maps, "<MODE-REQUEST-TYPE-MAP><IMPLEMENTATION-DATA-TYPE-REF DEST=\"IMPLEMENTATION-DATA-TYPE\">/AUTOSAR_Dcm/Rte_ModeType_{group}</IMPLEMENTATION-DATA-TYPE-REF><MODE-GROUP-REF DEST=\"MODE-DECLARATION-GROUP\">/AUTOSAR_Dcm/{group}</MODE-GROUP-REF></MODE-REQUEST-TYPE-MAP>").unwrap();
            }
            write!(elements, "<DATA-TYPE-MAPPING-SET><SHORT-NAME>DcmModeTypes</SHORT-NAME><MODE-REQUEST-TYPE-MAPS>{maps}</MODE-REQUEST-TYPE-MAPS></DATA-TYPE-MAPPING-SET>").unwrap();
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
        let generated = if module == "Rte"
            || (plan.multi.is_some() && matches!(module, "LSduR" | "PduR"))
        {
            format!("<GENERATED-ARTIFACTS>{}</GENERATED-ARTIFACTS>", paths.iter().map(|p| format!("<DEPENDENCY-ON-ARTIFACT><SHORT-NAME>{}</SHORT-NAME><ARTIFACT-DESCRIPTOR>{}</ARTIFACT-DESCRIPTOR><USAGES><USAGE>COMPILE</USAGE></USAGES></DEPENDENCY-ON-ARTIFACT>", c_name(p), artifact(p))).collect::<String>())
        } else {
            String::new()
        };
        let resource = if plan.multi.is_some() && module == "Rte" {
            let multi = plan.multi.as_ref().unwrap();
            let mut scopes = BTreeSet::from([
                ("Rte".to_string(), "CODE"),
                ("Rte".to_string(), "VAR_CLEARED_UNSPECIFIED"),
                ("Rte".to_string(), "CONFIG_DATA_PREBUILD_UNSPECIFIED"),
                ("Ecu".to_string(), "CODE"),
                ("Ecu".to_string(), "CONFIG_DATA_PREBUILD_UNSPECIFIED"),
                ("Com".to_string(), "CODE"),
                ("ComM".to_string(), "CODE"),
            ]);
            for component in &multi.components {
                let scope = component.component.rsplit('/').next().unwrap().to_string();
                scopes.insert((scope.clone(), "CODE"));
                if component.data_ports.iter().any(|port| port.read)
                    || component.diagnostic_session_port.is_some()
                {
                    scopes.insert((scope, "VAR_CLEARED_UNSPECIFIED"));
                }
            }
            if !multi
                .components
                .iter()
                .any(|component| component.diagnostic_session_port.is_some())
            {
                scopes.insert(("Dcm".into(), "CODE"));
                scopes.insert(("Dcm".into(), "VAR_CLEARED_UNSPECIFIED"));
            }
            scopes.insert((
                plan.communication_runtime
                    .as_ref()
                    .unwrap()
                    .partition
                    .path
                    .rsplit('/')
                    .next()
                    .unwrap()
                    .into(),
                "CALLOUT_CODE",
            ));
            for scope in ["Can", "CanIf", "Com", "ComM", "BswM", "Ecu_HostBusSM"] {
                scopes.insert((scope.into(), "CONFIG_DATA_PREBUILD_UNSPECIFIED"));
            }
            if plan.diagnostic_transport.is_some() {
                for scope in ["CanTp", "Dcm"] {
                    scopes.insert((scope.into(), "CONFIG_DATA_PREBUILD_UNSPECIFIED"));
                }
            }
            let mut sections = String::new();
            let mut prefixes = String::new();
            for scope in scopes
                .iter()
                .map(|(scope, _)| scope)
                .collect::<BTreeSet<_>>()
            {
                let header = if matches!(
                    scope.as_str(),
                    "Ecu"
                        | "Can"
                        | "CanIf"
                        | "Com"
                        | "CanTp"
                        | "Dcm"
                        | "PduR"
                        | "LSduR"
                        | "ComM"
                        | "BswM"
                        | "Ecu_HostBusSM"
                ) {
                    "include/Ecu_MemMap.h"
                } else {
                    "include/Rte_MemMap.h"
                };
                write!(prefixes, "<SECTION-NAME-PREFIX><SHORT-NAME>RTE_{scope}</SHORT-NAME><IMPLEMENTED-IN-REF DEST=\"DEPENDENCY-ON-ARTIFACT\">/{package}/RteHostImplementation/{}</IMPLEMENTED-IN-REF></SECTION-NAME-PREFIX>", c_name(header)).unwrap();
            }
            for (scope, segment) in scopes {
                write!(sections, "<MEMORY-SECTION><SHORT-NAME>{scope}_{segment}</SHORT-NAME><PREFIX-REF DEST=\"SECTION-NAME-PREFIX\">/{package}/RteHostImplementation/CodeMapping/RTE_{scope}</PREFIX-REF><SYMBOL>{segment}</SYMBOL></MEMORY-SECTION>").unwrap();
            }
            format!(
                "<RESOURCE-CONSUMPTION><SHORT-NAME>CodeMapping</SHORT-NAME><MEMORY-SECTIONS>{sections}</MEMORY-SECTIONS><SECTION-NAME-PREFIXS>{prefixes}</SECTION-NAME-PREFIXS></RESOURCE-CONSUMPTION>"
            )
        } else if plan.multi.is_some() && matches!(module, "LSduR" | "PduR") {
            format!(
                "<RESOURCE-CONSUMPTION><SHORT-NAME>CodeMapping</SHORT-NAME><MEMORY-SECTIONS><MEMORY-SECTION><SHORT-NAME>{module}_CONFIG_DATA_PREBUILD_UNSPECIFIED</SHORT-NAME><PREFIX-REF DEST=\"SECTION-NAME-PREFIX\">/{package}/{module}HostImplementation/CodeMapping/RTE_{module}</PREFIX-REF><SYMBOL>CONFIG_DATA_PREBUILD_UNSPECIFIED</SYMBOL></MEMORY-SECTION></MEMORY-SECTIONS><SECTION-NAME-PREFIXS><SECTION-NAME-PREFIX><SHORT-NAME>RTE_{module}</SHORT-NAME><IMPLEMENTED-IN-REF DEST=\"DEPENDENCY-ON-ARTIFACT\">/{package}/{module}HostImplementation/include_Ecu_MemMap_h</IMPLEMENTED-IN-REF></SECTION-NAME-PREFIX></SECTION-NAME-PREFIXS></RESOURCE-CONSUMPTION>"
            )
        } else if module == "Os" || module == "Rte" {
            format!(
                "<RESOURCE-CONSUMPTION><SHORT-NAME>CodeMapping</SHORT-NAME><MEMORY-SECTIONS><MEMORY-SECTION><SHORT-NAME>CODE</SHORT-NAME><SYMBOL>CODE</SYMBOL></MEMORY-SECTION></MEMORY-SECTIONS></RESOURCE-CONSUMPTION>"
            )
        } else {
            String::new()
        };
        let version = if module == "Arti" {
            "1.0.0"
        } else {
            env!("CARGO_PKG_VERSION")
        };
        write!(elements, r#"<BSW-IMPLEMENTATION><SHORT-NAME>{module}HostImplementation</SHORT-NAME><CODE-DESCRIPTORS><CODE><SHORT-NAME>DeliveredCode</SHORT-NAME><ARTIFACT-DESCRIPTORS>{artifacts}</ARTIFACT-DESCRIPTORS></CODE></CODE-DESCRIPTORS>{generated}<PROGRAMMING-LANGUAGE>C</PROGRAMMING-LANGUAGE>{resource}<SW-VERSION>{}</SW-VERSION><USED-CODE-GENERATOR>Autosar host ECU generator</USED-CODE-GENERATOR><VENDOR-ID>0</VENDOR-ID><AR-RELEASE-VERSION>4.10.0</AR-RELEASE-VERSION><BEHAVIOR-REF DEST="BSW-INTERNAL-BEHAVIOR">/{package}/{module}/HostBehavior</BEHAVIOR-REF></BSW-IMPLEMENTATION>"#, version).unwrap();
        if plan.multi.is_some() {
            let body = elements.split_off(module_start);
            let entry_package = if module_entries.is_empty() {
                String::new()
            } else {
                format!(
                    "<AR-PACKAGES><AR-PACKAGE><SHORT-NAME>BswModuleEntrys</SHORT-NAME><ELEMENTS>{module_entries}</ELEMENTS></AR-PACKAGE></AR-PACKAGES>"
                )
            };
            write!(selected_packages, "<AR-PACKAGE><SHORT-NAME>{package}</SHORT-NAME><ELEMENTS>{body}</ELEMENTS>{entry_package}</AR-PACKAGE>").unwrap();
        }
        writeln!(
            docs,
            "| {module} | {} |",
            paths
                .iter()
                .map(|p| format!("`{p}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
    }
    if let Some(multi) = &plan.multi {
        for component in &multi.components {
            for mode in &component.diagnostic_mode_ports {
                let group = mode.mode_group.rsplit('/').next().unwrap();
                write!(elements, "<SWC-BSW-MAPPING><SHORT-NAME>ModeMapping{}{group}</SHORT-NAME><BSW-BEHAVIOR-REF DEST=\"BSW-INTERNAL-BEHAVIOR\">/AUTOSAR_Dcm/Dcm/HostBehavior</BSW-BEHAVIOR-REF><SWC-BEHAVIOR-REF DEST=\"SWC-INTERNAL-BEHAVIOR\">{}</SWC-BEHAVIOR-REF><SYNCHRONIZED-MODE-GROUPS><SWC-BSW-SYNCHRONIZED-MODE-GROUP-PROTOTYPE><BSW-MODE-GROUP-REF DEST=\"MODE-DECLARATION-GROUP-PROTOTYPE\">/AUTOSAR_Dcm/Dcm/{group}</BSW-MODE-GROUP-REF><SWC-MODE-GROUP-IREF><CONTEXT-P-PORT-REF DEST=\"P-PORT-PROTOTYPE\">{}</CONTEXT-P-PORT-REF><TARGET-MODE-GROUP-REF DEST=\"MODE-DECLARATION-GROUP-PROTOTYPE\">{}</TARGET-MODE-GROUP-REF></SWC-MODE-GROUP-IREF></SWC-BSW-SYNCHRONIZED-MODE-GROUP-PROTOTYPE></SYNCHRONIZED-MODE-GROUPS></SWC-BSW-MAPPING>", c_name(&component.component), component.behavior, mode.port, mode.prototype).unwrap();
            }
        }
    }
    if plan.multi.is_some() {
        docs.push_str("\nRte_Start/Stop and SchM_Init/Start/StartTiming/Deinit own the actual local/freshness resources and authorize the OS bootstrap release. Ordinary RTE calls run on the one owner task. COM/ComM configured mains are defined in Ecu_RuntimeConfig.c and scheduled by the same source-derived rows as their BSW entries. Partially reentrant communication APIs permit concurrent calls on different PduIds/signals; the same object requires synchronization. Com_SendSignal is asynchronous; Rte_COMCbk and Rte_COMCbkRxTOut are synchronous and non-reentrant. Input BSW descriptions remain source provenance, while this file describes selected delivered producers.\n");
    } else {
        docs.push_str("\nRTE initialization is the actual Ecu_TargetInitializeRte called by StartupHook; synchronous S/R and DID calls run on the one owner task. Rte_OsService forwards Counter calls to OS. RTE owns no independent static application data; Application.c owns its state. RTE CODE maps to .rte_code and OS Task/Hook CODE to .os_code. No configured SwAddrMethod hardware location override exists. Native host adapters (Ecu_Target, Ecu_Status, locks, storage and transport bridges) remain host helpers, not additional standard BSW modules. Original source identities are in integration.json; tools/ecu-tool.py verifies files.list/files.sha256 before building in a separate output directory.\n");
    }
    if !private_entries.is_empty() {
        write!(selected_packages, "<AR-PACKAGE><SHORT-NAME>Autsaro_Rte</SHORT-NAME><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>BswModuleEntrys</SHORT-NAME><ELEMENTS>{private_entries}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AR-PACKAGE>").unwrap();
    }
    let description = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>HostArtifacts</SHORT-NAME><ELEMENTS>{elements}</ELEMENTS></AR-PACKAGE>{selected_packages}</AR-PACKAGES></AUTOSAR>
"#
    );
    Ok(BTreeMap::from([
        (
            "descriptions/Host_Implementation.arxml".into(),
            description.into_bytes(),
        ),
        ("MODULES.md".into(), docs.into_bytes()),
    ]))
}
