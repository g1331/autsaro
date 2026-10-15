use super::graph::Graph;
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContractArgument {
    pub name: String,
    pub native_type: String,
    pub direction: String,
}

#[derive(Clone)]
struct Entry {
    module: String,
    return_type: String,
    arguments: Vec<ContractArgument>,
    header: String,
    source: String,
    reentrant: bool,
    synchronous: bool,
}

/// Source-derived producer inventory selected for one mainline plan.
pub(super) struct RuntimeCatalog {
    entries: BTreeMap<String, Entry>,
}

/// The composition producer selects its actual R24-11 interfaces independently of
/// the historical host ABI. Generated instance wrappers retain their real
/// declaration/definition owners; immutable BSW bytes come from AssetInventory.
pub(super) fn bsw_catalog(
    com: &super::com::ComRuntimeContract,
    mode: &super::mode::ModeRuntimeContract,
) -> RuntimeCatalog {
    let mut entries = BTreeMap::new();
    let mut add = |symbol: &str,
                   module: &str,
                   returns: &str,
                   arguments: &[(&str, &str, &str)],
                   header: &str,
                   source: &str| {
        entries.insert(
            symbol.into(),
            Entry {
                module: module.into(),
                return_type: returns.into(),
                arguments: arguments
                    .iter()
                    .map(|(name, native, direction)| ContractArgument {
                        name: (*name).into(),
                        native_type: (*native).into(),
                        direction: (*direction).into(),
                    })
                    .collect(),
                header: header.into(),
                source: source.into(),
                reentrant: selected_behavior(symbol).0,
                synchronous: selected_behavior(symbol).1,
            },
        );
    };
    add(
        "Can_Init",
        "Can",
        "void",
        &[("Config", "const Can_ConfigType *", "IN")],
        "runtime/bsw/include/Can.h",
        "runtime/bsw/src/Can.c",
    );
    // SchM_Can.h is RTE-generated (SWS_Rte_07501), delivered under generated/.
    for symbol in [
        "Can_MainFunction_Wakeup",
        "Can_MainFunction_Read",
        "Can_MainFunction_Write",
        "Can_MainFunction_Mode",
        "Can_MainFunction_BusOff",
    ] {
        add(
            symbol,
            "Can",
            "void",
            &[],
            "include/SchM_Can.h",
            "runtime/bsw/src/Can.c",
        );
    }
    for (module, config) in [
        ("CanIf", "const CanIf_ConfigType *"),
        ("CanTp", "const CanTp_ConfigType *"),
        ("Com", "const Com_ConfigType *"),
        ("Dcm", "const Dcm_ConfigType *"),
        ("PduR", "const PduR_PBConfigType *"),
    ] {
        add(
            &format!("{module}_Init"),
            module,
            "void",
            &[("Config", config, "IN")],
            &format!("runtime/bsw/include/{module}.h"),
            &format!("runtime/bsw/src/{module}.c"),
        );
    }
    for (symbol, module, header) in [
        ("CanIf_Transmit", "CanIf", "CanIf.h"),
        ("PduR_ComTransmit", "PduR", "PduR_Com.h"),
    ] {
        add(
            symbol,
            module,
            "Std_ReturnType",
            &[
                ("PduId", "PduIdType", "IN"),
                ("PduInfo", "const PduInfoType *", "IN"),
            ],
            &format!("runtime/bsw/include/{header}"),
            &format!("runtime/bsw/src/{module}.c"),
        );
    }
    add(
        "Com_ReceiveSignal",
        "Com",
        "uint8",
        &[
            ("SignalId", "Com_SignalIdType", "IN"),
            ("SignalData", "void *", "OUT"),
        ],
        "runtime/bsw/include/Com.h",
        "runtime/bsw/src/Com.c",
    );
    add(
        "Com_SendSignal",
        "Com",
        "uint8",
        &[
            ("SignalId", "Com_SignalIdType", "IN"),
            ("SignalData", "const void *", "IN"),
        ],
        "runtime/bsw/include/Com.h",
        "runtime/bsw/src/Com.c",
    );
    // Both scheduler headers are delivered as generated files.
    for module in ["CanTp", "Dcm"] {
        add(
            &format!("{module}_MainFunction"),
            module,
            "void",
            &[],
            &format!("include/SchM_{module}.h"),
            &format!("runtime/bsw/src/{module}.c"),
        );
    }
    for (symbol, module) in [
        (&com.receive_main.symbol, "Com"),
        (&com.transmit_main.symbol, "Com"),
        (&mode.main_symbol, "ComM"),
    ] {
        add(
            symbol,
            module,
            "void",
            &[],
            &format!("include/SchM_{module}.h"),
            "src/Ecu_RuntimeConfig.c",
        );
    }
    RuntimeCatalog { entries }
}

// Reentrancy is qualified by different PduIds/signals in the standard API
// contracts; source descriptions carry the boolean capability, not permission
// for unsynchronized calls on one object. COM SendSignal is asynchronous.
pub(super) fn module_id(module: &str) -> Option<u32> {
    match module {
        "Can" => Some(80),
        "Det" => Some(15),
        "CanIf" => Some(60),
        "CanTp" => Some(35),
        "Com" => Some(50),
        "ComM" => Some(12),
        "Dcm" => Some(53),
        "PduR" => Some(51),
        "Rte" => Some(2),
        "Os" => Some(1),
        "BswM" => Some(42),
        "LSduR" => Some(132),
        "Arti" => Some(5),
        "Ecu_HostBusSM" => Some(255),
        _ => None,
    }
}

pub(super) fn selected_behavior(symbol: &str) -> (bool, bool) {
    (
        matches!(
            symbol,
            "CanIf_Transmit"
                | "Can_Write"
                | "Can_GetControllerErrorState"
                | "Can_GetControllerRxErrorCounter"
                | "Can_GetControllerTxErrorCounter"
                | "Dcm_StartOfReception"
                | "Dcm_CopyRxData"
                | "Dcm_TpRxIndication"
                | "Dcm_CopyTxData"
                | "PduR_CanIfRxIndication"
                | "PduR_CanIfTxConfirmation"
                | "PduR_CanIfTriggerTransmit"
                | "PduR_CanTpStartOfReception"
                | "PduR_CanTpCopyRxData"
                | "PduR_CanTpRxIndication"
                | "PduR_CanTpCopyTxData"
                | "PduR_CanTpTxConfirmation"
                | "Com_RxIndication"
                | "Com_TxConfirmation"
                | "Com_TriggerTransmit"
                | "ComM_BusSM_ModeIndication"
                | "BswM_ComM_CurrentMode"
                | "LSduR_PduRTransmit"
                | "LSduR_CanTpTransmit"
                | "LSduR_CanIfRxIndication"
                | "LSduR_CanIfTxConfirmation"
                | "LSduR_CanIfTriggerTransmit"
                | "CanIf_SetControllerMode"
                | "CanIf_GetControllerErrorState"
                | "CanIf_GetPduMode"
                | "CanIf_RxIndication"
                | "CanIf_TxConfirmation"
                | "CanIf_ControllerModeIndication"
                | "CanIf_ControllerBusOff"
                | "CanTp_Transmit"
                | "CanTp_RxIndication"
                | "CanTp_TxConfirmation"
                | "PduR_DcmTransmit"
                | "ComM_DCM_ActiveDiagnostic"
                | "ComM_DCM_InactiveDiagnostic"
                | "PduR_ComTransmit"
                | "Com_ReceiveSignal"
                | "Com_SendSignal"
                | "Dcm_ResetToDefaultSession"
                | "Dcm_GetSesCtrlType"
                | "Dcm_SetActiveDiagnostic"
                | "GetCounterValue"
                | "GetElapsedValue"
                | "Dcm_TpTxConfirmation"
                | "Dcm_ComM_NoComModeEntered"
                | "Dcm_ComM_SilentComModeEntered"
                | "Dcm_ComM_FullComModeEntered"
        ),
        !matches!(
            symbol,
            "Com_SendSignal"
                | "Can_SetControllerMode"
                | "CanIf_SetControllerMode"
                | "ComM_BusSM_ModeIndication"
        ),
    )
}

#[allow(
    clippy::disallowed_macros,
    reason = "identifies the generator's own sources, which are not delivered"
)]
pub(super) fn runtime_source_identities() -> BTreeMap<String, String> {
    let mut identities: BTreeMap<String, String> = crate::resources::AssetInventory::embedded()
        .entries()
        .iter()
        .filter(|asset| {
            asset.relative_path.starts_with("runtime/bsw/")
                || asset.relative_path.starts_with("runtime/host/")
        })
        .map(|asset| (asset.relative_path.into(), asset.sha256.into()))
        .collect();
    for (path, bytes) in [
        (
            "core/src/integration/ecu_config.rs",
            include_bytes!("ecu_config.rs").as_slice(),
        ),
        (
            "core/src/integration/rte.rs",
            include_bytes!("rte.rs").as_slice(),
        ),
        (
            "core/src/integration/artifacts.rs",
            include_bytes!("artifacts.rs").as_slice(),
        ),
        (
            "core/src/integration/ecu.rs",
            include_bytes!("ecu.rs").as_slice(),
        ),
        (
            "core/src/integration/os_service.rs",
            include_bytes!("os_service.rs").as_slice(),
        ),
        (
            "core/src/integration/composition.rs",
            include_bytes!("composition.rs").as_slice(),
        ),
        (
            "core/src/integration/contracts.rs",
            include_bytes!("contracts.rs").as_slice(),
        ),
        (
            "core/src/integration/arti.rs",
            include_bytes!("arti.rs").as_slice(),
        ),
        (
            "core/src/integration/outputs.rs",
            include_bytes!("outputs.rs").as_slice(),
        ),
    ] {
        identities.insert(path.into(), format!("{:x}", Sha256::digest(bytes)));
    }
    identities
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolContract {
    pub symbol: String,
    pub return_type: String,
    pub arguments: Vec<ContractArgument>,
    pub declaration_owner: String,
    pub definition_owner: String,
    pub consumers: Vec<String>,
}

pub(super) fn c_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    matches!(bytes.next(), Some(b'a'..=b'z' | b'A'..=b'Z' | b'_'))
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn native_type(graph: &Graph, context: usize) -> Option<String> {
    let references = graph.descendants(context, "BASE-TYPE-REF");
    if references.len() != 1 {
        return None;
    }
    let base = graph.objects.get(&graph.elements[references[0]].text)?;
    graph
        .text(*base, "NATIVE-DECLARATION")
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
}

pub(super) fn inspect(
    graph: &Graph,
    catalog: &RuntimeCatalog,
) -> Result<Vec<SymbolContract>, Vec<PlanDiagnostic>> {
    let mut symbols = Vec::new();
    let mut used_owners = BTreeSet::new();
    let context = graph.context()?;
    let all_entries = graph.of_kind("BSW-MODULE-ENTRY");
    for (symbol, current) in &catalog.entries {
        let found: Vec<_> = all_entries
            .iter()
            .copied()
            .filter(|index| graph.text(*index, "SHORT-NAME") == Some(symbol))
            .collect();
        if found.is_empty() {
            // Locate the module description reference that names the missing
            // entry, as the reference audit does, rather than an unrelated object.
            let reference = graph.elements.iter().position(|element| {
                element.tag == "BSW-MODULE-ENTRY-REF"
                    && element.text.rsplit('/').next() == Some(symbol.as_str())
            });
            let mut issue = graph.diagnostic(
                reference.unwrap_or(context),
                DiagnosticCategory::Input,
                "BSW_ENTRY_MISSING",
                crate::product_message!("backend.integration.catalog.bsw_entry_description_and_producer_required", "symbol" => symbol),
                crate::product_message!("backend.integration.catalog.bsw_entry_description_supply"),
            );
            if let Some(reference) = reference {
                issue.object = Some(graph.elements[reference].text.clone());
            }
            return Err(vec![issue]);
        }
        if found.len() != 1 {
            return Err(vec![graph.diagnostic(found[0],
                DiagnosticCategory::Input, "SYMBOL_PRODUCER_DUPLICATE",
                crate::product_message!("backend.integration.catalog.bsw_entry_description_and_producer_required", "symbol" => symbol),
                crate::product_message!("backend.integration.catalog.bsw_entry_description_supply"))]);
        }
        let entry = found[0];
        if graph.elements[entry].object
            != format!("/AUTOSAR_{}/BswModuleEntrys/{symbol}", current.module)
        {
            return Err(vec![graph.diagnostic(entry, DiagnosticCategory::Input, "BSW_ENTRY_IDENTITY", crate::product_message!("backend.integration.catalog.bsw_entry_description_and_producer_required", "symbol" => symbol), crate::product_message!("backend.integration.catalog.bsw_entry_description_supply"))]);
        }
        // TPS_BSWMDT constr_10260..10264 require explicit interface properties.
        // The selected producers are ordinary concrete functions, not macros or inline APIs.
        if [
            "IS-REENTRANT",
            "IS-SYNCHRONOUS",
            "CALL-TYPE",
            "EXECUTION-CONTEXT",
        ]
        .iter()
        .any(|field| graph.text(entry, field).is_none())
            || graph.text(entry, "CALL-TYPE")
                != Some(if symbol.contains("_MainFunction") {
                    "SCHEDULED"
                } else {
                    "REGULAR"
                })
            || graph.text(entry, "SW-SERVICE-IMPL-POLICY") != Some("STANDARD")
            || graph
                .text(entry, "BSW-ENTRY-KIND")
                .is_some_and(|kind| kind != "CONCRETE")
        {
            return Err(vec![graph.diagnostic(entry, DiagnosticCategory::Input, "BSW_SIGNATURE_CONFLICT",
                crate::product_message!("backend.integration.catalog.bsw_description_source_contract_mismatch", "symbol" => symbol),
                crate::product_message!("backend.integration.catalog.bsw_abi_contract_correct"))]);
        }
        let returns = graph.children(entry, "RETURN-TYPE");
        let return_type = if returns.is_empty() {
            Some("void".into())
        } else if returns.len() == 1 {
            native_type(graph, returns[0])
        } else {
            None
        };
        let arguments = graph.descendants(entry, "SW-SERVICE-ARG");
        let mut actual = Vec::new();
        for argument in arguments {
            let native = native_type(graph, argument).ok_or_else(|| {
                vec![graph.diagnostic(
                    argument,
                    DiagnosticCategory::Input,
                    "BSW_SIGNATURE_CONFLICT",
                    crate::product_message!(
                        "backend.integration.catalog.bsw_argument_native_type_not_unique"
                    ),
                    crate::product_message!(
                        "backend.integration.catalog.bsw_matching_base_type_declaration_supply"
                    ),
                )]
            })?;
            actual.push(ContractArgument {
                name: graph.text(argument, "SHORT-NAME").unwrap_or("").into(),
                native_type: native,
                direction: graph.text(argument, "DIRECTION").unwrap_or("").into(),
            });
        }
        if return_type.as_deref() != Some(current.return_type.as_str())
            || actual.len() != current.arguments.len()
            || actual
                .iter()
                .zip(&current.arguments)
                .any(|(actual, expected)| {
                    actual.native_type != expected.native_type
                        || actual.direction != expected.direction
                })
            || matches!(graph.text(entry, "IS-SYNCHRONOUS"), Some("true" | "1"))
                != current.synchronous
            || matches!(graph.text(entry, "IS-REENTRANT"), Some("true" | "1")) != current.reentrant
        {
            return Err(vec![graph.diagnostic(entry, DiagnosticCategory::Input, "BSW_SIGNATURE_CONFLICT",
                crate::product_message!("backend.integration.catalog.bsw_description_source_contract_mismatch", "symbol" => symbol),
                crate::product_message!("backend.integration.catalog.bsw_abi_contract_correct"))]);
        }
        let owners: Vec<_> = graph
            .of_kind("BSW-MODULE-DESCRIPTION")
            .into_iter()
            .filter(|owner| {
                graph
                    .descendants(*owner, "BSW-MODULE-ENTRY-REF")
                    .into_iter()
                    .any(|reference| graph.elements[reference].text == graph.elements[entry].object)
            })
            .collect();
        if owners.len() != 1 {
            return Err(vec![graph.diagnostic(
                entry,
                DiagnosticCategory::Input,
                "SYMBOL_PRODUCER_DUPLICATE",
                crate::product_message!(
                    "backend.integration.catalog.bsw_entry_module_description_required"
                ),
                crate::product_message!(
                    "backend.integration.catalog.bsw_implemented_entry_relationship_supply"
                ),
            )]);
        }
        let owner = owners[0];
        if graph.text(owner, "CATEGORY") != Some("BSW_MODULE")
            || graph
                .text(owner, "MODULE-ID")
                .and_then(|value| value.parse::<u32>().ok())
                != module_id(&current.module)
        {
            return Err(vec![graph.diagnostic(owner, DiagnosticCategory::Input, "BSW_ENTRY_IDENTITY", crate::product_message!("backend.integration.catalog.bsw_entry_description_and_producer_required", "symbol" => symbol), crate::product_message!("backend.integration.catalog.bsw_entry_description_supply"))]);
        }
        used_owners.insert(owner);
        let implementations: Vec<_> = graph
            .of_kind("BSW-IMPLEMENTATION")
            .into_iter()
            .filter(|implementation| {
                graph
                    .target(*implementation, "BEHAVIOR-REF")
                    .is_some_and(|behavior| graph.within(behavior, owner))
            })
            .collect();
        if implementations.len() != 1
            || graph.text(implementations[0], "PROGRAMMING-LANGUAGE") != Some("C")
        {
            return Err(vec![graph.diagnostic(
                owner,
                DiagnosticCategory::Input,
                "SYMBOL_PRODUCER_DUPLICATE",
                crate::product_message!(
                    "backend.integration.catalog.bsw_matching_c_implementation_required"
                ),
                crate::product_message!(
                    "backend.integration.catalog.bsw_c_implementation_and_behavior_supply"
                ),
            )]);
        }
        symbols.push(SymbolContract {
            symbol: symbol.clone(),
            return_type: current.return_type.clone(),
            arguments: actual,
            declaration_owner: current.header.clone(),
            definition_owner: current.source.clone(),
            consumers: vec![graph.elements[entry].object.clone()],
        });
    }
    for owner in used_owners {
        for reference in graph.descendants(owner, "BSW-MODULE-ENTRY-REF") {
            let entry = graph.index(&graph.elements[reference].text)?;
            let symbol = graph.text(entry, "SHORT-NAME").unwrap_or("");
            if !catalog.entries.contains_key(symbol) {
                return Err(vec![graph.diagnostic(
                    entry,
                    DiagnosticCategory::Input,
                    "BSW_ENTRY_MISSING",
                    crate::product_message!(
                        "backend.integration.catalog.bsw_selected_entry_source_producer_missing"
                    ),
                    crate::product_message!(
                        "backend.integration.catalog.bsw_entry_source_type_contract_supply"
                    ),
                )]);
            }
        }
    }
    Ok(symbols)
}
