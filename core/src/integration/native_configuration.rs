use super::graph::Graph;
use crate::definitions::DefinitionCatalog;
use crate::project_model::{TypedValue, ValueKind};

// These fields describe physical modules or disabled optional services. They do
// not add a runtime implementation to the controlled virtual ECU target.
const DESCRIPTOR_PARAMETERS: &str = "
CanControllerBaseAddress CanControllerBaudRateConfigID CanControllerPropSeg
CanControllerSeg1 CanControllerSeg2 CanControllerSyncJumpWidth CanIndex
CanMainFunctionModePeriod CanTimeoutDuration CanIfInitCfgSet
CanIfDispatchUserCtrlBusOffUL CanIfDispatchUserCtrlModeIndicationUL
CanIfPrivateSoftwareFilterType CanIfPublicHandleTypeEnum
PduIdTypeEnum PduLengthTypeEnum DcmDspDataDefaultEndianness";

const DISABLED_FEATURES: &str = "
CanDevErrorDetect CanEnableSecurityEventReporting CanGlobalTimeSupport
CanMultiplexedTransmission CanVersionInfoApi CanIfCtrlWakeupSupport
CanIfFixedBuffer CanIfSupportTTCAN CanIfBusMirroringSupport CanIfDevErrorDetect
CanIfEnableSecurityEventReporting CanIfGlobalTimeSupport CanIfPublicCtrlPnEnable
CanIfPublicMultipleDrvSupport CanIfPublicPnSupport CanIfPublicReadRxPduDataApi
CanIfPublicReadRxPduNotifyStatusApi CanIfPublicReadTxPduNotifyStatusApi
CanIfPublicSetDynamicTxIdApi CanIfPublicTrcvPnEnable CanIfPublicTxBuffering
CanIfPublicTxConfirmPollingSupport CanIfPublicWakeupCheckValidSupport
CanIfTriggerTransmitSupport CanIfTxOfflineActiveSupport CanIfVersionInfoApi
CanIfWakeupSupport CanTpChangeParameterApi CanTpDevErrorDetect
CanTpEnableSecurityEventReporting CanTpReadParameterApi CanTpVersionInfoApi
ComEnableSecurityEventReporting ComEnableSignalGroupArrayApi ComVersionInfoApi
DcmDspEnableObdMirror DcmDevErrorDetect DcmVersionInfoApi
DcmEnableSecurityEventReporting DcmRespondAllRequest DcmPagedBufferEnabled
DcmDslDiagRespOnSecondDeclinedRequest";

pub(super) fn descriptor_only(
    graph: &Graph,
    index: usize,
    catalog: &DefinitionCatalog,
    reference: bool,
) -> bool {
    let Some(id) = graph.text(index, "DEFINITION-REF") else {
        return false;
    };
    if !id.starts_with("/AUTOSAR/EcucDefs/") {
        return false;
    }
    let Some(descriptor) = catalog.get(id) else {
        return false;
    };
    let name = id.rsplit('/').next().unwrap_or("");
    if reference {
        return matches!(
            name,
            "CanCpuClockRef"
                | "CanSupportTTCANRef"
                | "CanIfCtrlDrvInitHohConfigRef"
                | "CanIfCtrlDrvNameRef"
                | "DcmDemClientRef"
        );
    }
    let Some(kind) = descriptor.kind else {
        return false;
    };
    let Some(lexeme) = graph.text(index, "VALUE") else {
        return false;
    };
    if catalog
        .validate_value(
            id,
            &TypedValue {
                kind,
                lexeme: lexeme.into(),
            },
        )
        .is_err()
    {
        return false;
    }
    if DESCRIPTOR_PARAMETERS
        .split_ascii_whitespace()
        .any(|candidate| candidate == name)
    {
        return true;
    }
    if kind == ValueKind::Boolean
        && DISABLED_FEATURES
            .split_ascii_whitespace()
            .any(|candidate| candidate == name)
    {
        return matches!(lexeme.trim(), "false" | "0");
    }
    match name {
        // The existing controlled virtual ECU contract has one core, ID 0.
        // Physical core numbering is metadata, not a multicore runtime capability.
        "EcucCoreId" => {
            if lexeme.parse::<u16>() != Ok(0) {
                return false;
            }
            let mut cores = graph.objects.values().filter(|index| {
                graph.text(**index, "DEFINITION-REF")
                    == Some("/AUTOSAR/EcucDefs/EcuC/EcucHardware/EcucCoreDefinition")
            });
            cores.next().is_some() && cores.next().is_none()
        }
        "CanIfPrivateDataLengthCheck" => matches!(lexeme.trim(), "true" | "1"),
        "ComIPduSignalProcessing" => lexeme == "DEFERRED",
        "ComIPduType" => lexeme == "NORMAL",
        "ComTxIPduUnusedAreasDefault" | "ComSupportedIPduGroups" => lexeme.parse::<u32>() == Ok(0),
        "DcmDspDidUsed" => matches!(lexeme.trim(), "true" | "1"),
        "DcmDspDidUsePort" => lexeme == "USE_DATA_ELEMENT_SPECIFIC_INTERFACES",
        _ => false,
    }
}

pub(super) fn baudrate_kbit(value: &str) -> Option<u32> {
    let value = value.parse::<f64>().ok()?;
    if !value.is_finite() || value <= 0.0 || value > f64::from(u32::MAX) || value.fract() != 0.0 {
        return None;
    }
    Some(value as u32)
}

pub(super) fn consumer_objects(graph: &Graph) -> std::collections::BTreeSet<String> {
    let mut roots = Vec::new();
    for kind in [
        "APPLICATION-SW-COMPONENT-TYPE",
        "SERVICE-SW-COMPONENT-TYPE",
        "COMPOSITION-SW-COMPONENT-TYPE",
        "IMPLEMENTATION-DATA-TYPE",
        "SW-BASE-TYPE",
        "SENDER-RECEIVER-INTERFACE",
        "CLIENT-SERVER-INTERFACE",
        "BSW-MODULE-DESCRIPTION",
        "BSW-IMPLEMENTATION",
        "CAN-CLUSTER",
        "ECU-INSTANCE",
    ] {
        roots.extend(graph.of_kind(kind));
    }
    roots.extend(
        graph
            .of_kind("SYSTEM")
            .into_iter()
            .filter(|index| graph.text(*index, "CATEGORY") == Some("ECU_EXTRACT")),
    );
    for module in graph.of_kind("ECUC-MODULE-CONFIGURATION-VALUES") {
        if graph
            .text(module, "DEFINITION-REF")
            .is_some_and(|definition| {
                [
                    "Can", "CanIf", "CanTp", "Com", "ComM", "BswM", "Dcm", "EcuC", "Os", "PduR",
                    "Rte",
                ]
                .iter()
                .any(|name| definition.strip_prefix("/AUTOSAR/EcucDefs/") == Some(*name))
            })
        {
            roots.push(module);
        }
    }
    let mut selected = vec![false; graph.elements.len()];
    let mut pending = roots;
    while let Some(index) = pending.pop() {
        if selected[index] {
            continue;
        }
        selected[index] = true;
        let element = &graph.elements[index];
        pending.extend(element.children.iter().copied());
        if element.attributes.contains_key("DEST") {
            if let Some(target) = graph.objects.get(&element.text) {
                pending.push(*target);
            }
        }
    }
    // A referenced container also depends on its enclosing module's edition
    // and ownership, without consuming unrelated siblings of that container.
    for index in 0..selected.len() {
        if !selected[index] {
            continue;
        }
        let mut parent = graph.elements[index].parent;
        while let Some(index) = parent {
            if selected[index] {
                break;
            }
            selected[index] = true;
            parent = graph.elements[index].parent;
        }
    }
    graph
        .objects
        .iter()
        .filter_map(|(path, index)| selected[*index].then(|| path.clone()))
        .collect()
}
