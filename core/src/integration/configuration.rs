use super::component::milliseconds;
use super::graph::Graph;
use super::schedule::{definition_is, value, values};
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

// Explicit input grammar for the selected target. These are standard ECUC
// parameter identifiers, not reference names, IDs or expected test results.
const SUPPORTED_PARAMETERS: &str = "
CanBusoffProcessing CanControllerActivation CanControllerBaudRate CanControllerId
CanHwPnSupport CanRxProcessing CanTxProcessing CanWakeupProcessing CanWakeupSupport
CanHandleType CanHwObjectCount CanIdType CanObjectId CanObjectType
CanIfCtrlId CanIfBufferSize CanIfHrhSoftwareFilter CanIfRxPduCanId CanIfRxPduCanIdType
CanIfRxPduDataLength CanIfRxPduDataLengthCheck CanIfRxPduId CanIfRxPduReadData
CanIfRxPduReadNotifyStatus CanIfTxPduCanId CanIfTxPduCanIdType CanIfTxPduId
CanIfTxPduReadNotifyStatus CanIfTxPduTruncation CanIfTxPduType
CanTpBs CanTpNar CanTpNbr CanTpNcr CanTpRxAddressingFormat CanTpRxNPduId CanTpRxNSduId
CanTpRxPaddingActivation CanTpRxTaType CanTpRxWftMax CanTpSTmin CanTpTxFcNPduConfirmationPduId
CanTpNas CanTpNbs CanTpNcs CanTpRxFcNPduId CanTpTc CanTpTxAddressingFormat
CanTpTxNPduConfirmationPduId CanTpTxNSduId CanTpTxPaddingActivation CanTpTxTaType
CanTpMainFunctionPeriod CanTpPaddingByte
ComIPduDirection ComTxModeMode ComTxModeTimePeriod ComBitPosition ComBitSize ComFirstTimeout
ComRxDataTimeoutAction ComSignalEndianness ComSignalInitValue ComSignalType ComTimeout ComTransferProperty
DcmDsdServiceUsed DcmDsdSidTabServiceId DcmDsdSidTabSubfuncAvail DcmDsdSubServiceId
DcmDsdSubServiceUsed DcmDsdSidTabId DcmDslBufferSize DcmDslProtocolRxAddrType DcmDslProtocolRxPduId
DcmDslProtocolRxConnectionId DcmDslTxConfirmationPduId DcmDslProtocolPriority DcmDslProtocolRowUsed
DcmDslProtocolType DcmSendRespPendOnRestart DcmTimStrP2ServerAdjust DcmTimStrP2StarServerAdjust
DcmDspDataByteSize DcmDspDataConditionCheckReadFncUsed DcmDspDataEndianness DcmDspDataType
DcmDspDataUsePort DcmDspDataScalingInfoSize DcmDspDidIdentifier DcmDspDidByteOffset
DcmDspDidDynamicallyDefined DcmDspMaxDidToRead DcmDspSessionForBoot DcmDspSessionLevel
DcmDspSessionP2ServerMax DcmDspSessionP2StarServerMax DcmTaskTime PduLength
OsAlarmAlarmTime OsAlarmAutostartType OsAlarmCycleTime OsCounterMaxAllowedValue OsCounterMinCycle
OsScheduleTableDuration OsScheduleTableRepeating OsScheduleTableAutostartType OsScheduleTableStartValue OsScheduleTblExpPointOffset OsScheduleTblSyncStrategy
OsCounterTicksPerBase OsCounterType OsSecondsPerTick OsEventMask OsErrorHook OsPostTaskHook
OsPreTaskHook OsProtectionHook OsShutdownHook OsStartupHook OsScalabilityClass OsStatus
OsUseGetServiceId OsUseParameterAccess OsUseResScheduler OsTaskActivation OsTaskPriority OsTaskSchedule
PduRDestPduHandleId PduRTransmissionConfirmation PduRSourcePduHandleId PduRSrcPduUpTxConf
RteBswPositionInTask RtePositionInTask RteEventIsMappedToTask RteBswEventIsMappedToTask";

const MULTI_RUNTIME_PARAMETERS: &str = "ComMinimumDelayTime ComSupportedIPduGroups ComIPduGroupHandleId ComMainRxTimeBase ComMainTxTimeBase ComUserHeaderInclude ComUserCbkHandleId ComUserCallbackName ComUserCallbackType PduIdTypeEnum PduLengthTypeEnum CanDevErrorDetect CanMainFunctionPeriod CanMainFunctionModePeriod CanMainFunctionBusoffPeriod CanTriggerTransmitEnable ComMDevErrorDetect ComMDynamicPncToChannelMappingSupport ComMModeLimitationEnabled ComMPncSupport ComMResetAfterForcingNoComm ComMSynchronousWakeUp ComMVersionInfoApi ComMWakeupInhibitionEnabled ComMEcuGroupClassification ComMTMinFullComModeDuration ComMBusType ComMChannelId ComMCDDBusPrefix ComMMainFunctionPeriod ComMFullCommRequestNotificationEnabled ComMNoCom ComMNoWakeup ComMNoWakeUpInhibitionNvmStorage ComMNmVariant ComMUserIdentifier BswMCanSMEnabled BswMComMEnabled BswMDcmEnabled BswMDevErrorDetect BswMEcuMEnabled BswMEthIfEnabled BswMEthSMEnabled BswMFrSMEnabled BswMGenericRequestEnabled BswMJ1939DcmEnabled BswMJ1939NmEnabled BswMLinSMEnabled BswMLinTPEnabled BswMNmEnabled BswMNvMEnabled BswMSdControlEnabled BswMSdEnabled BswMVersionInfoApi BswMUserIncludeFile BswMRequestProcessing BswMBswModeInitValue BswMConditionType BswMBswRequestedMode BswMLogicalOperator BswMRuleInitState BswMNestedExecutionOnly BswMUserCalloutFunction BswMActionListExecution BswMActionListPriority BswMActionListItemIndex BswMAbortOnFail";
const MULTI_RUNTIME_REFERENCES: &str = "ComIPduGroupRef ComIPduMainFunctionRef ComUserCallbackRef ComUserSystemTemplateSystemSignalRef CanMainFunctionRWPeriodRef ComMUserChannel BswMComMChannelRef BswMConditionMode BswMArgumentRef BswMRuleExpressionRef BswMRuleTrueActionList BswMActionListItemRef DcmDslProtocolComMChannelRef";

const MODULES: &[&str] = &[
    "Can", "CanIf", "CanTp", "Com", "Dcm", "EcuC", "Os", "PduR", "Rte",
];

const SUPPORTED_REFERENCES: &str = "
CanControllerDefaultBaudrate CanControllerRef CanIfBufferHthRef CanIfCtrlCanCtrlRef
CanIfHrhCanCtrlIdRef CanIfHrhIdSymRef CanIfHthCanCtrlIdRef CanIfHthIdSymRef
CanIfRxPduHrhIdRef CanIfRxPduRef CanIfTxPduBufferRef CanIfTxPduRef
CanTpRxFcNPduRef CanTpRxNPduRef CanTpRxNSduRef CanTpTxFcNPduRef CanTpTxNPduRef CanTpTxNSduRef
ComIPduSignalRef ComPduIdRef ComSystemTemplateSystemSignalRef
DcmDslProtocolRxBufferRef DcmDslProtocolRxPduRef DcmDslProtocolSIDTable
DcmDslProtocolTxBufferRef DcmDslProtocolTxPduRef DcmDspDataInfoRef DcmDspDidDataRef
DcmDspDidInfoRef DcmDspDidReadSessionRef OsAlarmAppModeRef OsAlarmCounterRef
OsAlarmSetEventRef OsAlarmSetEventTaskRef OsTaskAppModeRef OsTaskEventRef
PduRDestPduRRef PduRDestPduRef PduRSrcPduRRef PduRSrcPduRef RteBswEventRef
RteBswImplementationRef RteBswMappedToTaskRef RteBswUsedOsAlarmRef RteBswUsedOsEventRef
RteEventRef RteMappedToTaskRef RteSoftwareComponentInstanceRef RteUsedOsAlarmRef RteUsedOsEventRef
RteUsedOsSchTblExpiryPointRef RteBswUsedOsSchTblExpiryPointRef OsScheduleTableCounterRef OsScheduleTableAppModeRef OsScheduleTableSetEventTaskRef OsScheduleTableSetEventRef";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationRecord {
    pub path: String,
    pub definition: String,
    pub parameters: BTreeMap<String, Vec<String>>,
    pub references: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventAssignment {
    pub path: String,
    pub handle: u8,
    pub mask: u32,
}

pub(super) struct Configuration {
    pub records: Vec<ConfigurationRecord>,
    pub events: Vec<EventAssignment>,
}

fn reject(
    graph: &Graph,
    index: usize,
    code: &str,
    message: crate::message::LocalizedText,
) -> Vec<PlanDiagnostic> {
    vec![graph.diagnostic(
        index,
        DiagnosticCategory::Unsupported,
        code,
        message,
        crate::product_message!(
            "backend.integration.configuration.fixed_target_implementation_contract_required"
        ),
    )]
}

fn one(graph: &Graph, context: usize, kind: &str) -> Result<usize, Vec<PlanDiagnostic>> {
    let found: Vec<_> = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, kind))
        .collect();
    if found.len() == 1 {
        Ok(found[0])
    } else {
        Err(reject(
            graph,
            context,
            "CONFIGURATION_NOT_UNIQUE",
            crate::product_message!(
                "backend.integration.configuration.physical_configuration_object_missing_or_duplicate"
            ),
        ))
    }
}

fn reference(graph: &Graph, index: usize, parameter: &str) -> Result<usize, Vec<PlanDiagnostic>> {
    value(graph, index, parameter, true)
        .and_then(|path| graph.objects.get(path).copied())
        .ok_or_else(|| {
            reject(
                graph,
                index,
                "CONFIGURATION_REFERENCE",
                crate::product_message!("backend.integration.configuration.physical_configuration_reference_missing_or_duplicate"),
            )
        })
}

fn physical(graph: &Graph, context: usize, native: bool) -> Result<(), Vec<PlanDiagnostic>> {
    let controller = one(graph, context, "CanController")?;
    let can_if_controller = one(graph, controller, "CanIfCtrlCfg")?;
    if reference(graph, can_if_controller, "CanIfCtrlCanCtrlRef")? != controller
        || value(graph, controller, "CanControllerId", false)
            .and_then(|value| value.parse::<u8>().ok())
            .is_none()
        || value(graph, can_if_controller, "CanIfCtrlId", false)
            .and_then(|value| value.parse::<u8>().ok())
            .is_none()
    {
        return Err(reject(
            graph,
            can_if_controller,
            "CONTROLLER_MAPPING",
            crate::product_message!(
                "backend.integration.configuration.canif_controller_binding_required"
            ),
        ));
    }
    let baudrate = reference(graph, controller, "CanControllerDefaultBaudrate")?;
    if !definition_is(graph, baudrate, "CanControllerBaudrateConfig")
        || !graph.within(baudrate, controller)
    {
        return Err(reject(
            graph,
            controller,
            "CONTROLLER_MAPPING",
            crate::product_message!(
                "backend.integration.configuration.default_baudrate_controller_mismatch"
            ),
        ));
    }
    let rate = value(graph, baudrate, "CanControllerBaudRate", false)
        .and_then(|value| {
            if native {
                super::native_configuration::baudrate_kbit(value)
            } else {
                value.parse::<u32>().ok()
            }
        })
        .and_then(|value| value.checked_mul(1000))
        .filter(|value| *value != 0);
    let ecus = graph.of_kind("ECU-INSTANCE");
    if ecus.len() != 1 {
        return Err(reject(
            graph,
            controller,
            "CONTROLLER_MAPPING",
            crate::product_message!(
                "backend.integration.configuration.single_selected_ecu_instance_required"
            ),
        ));
    }
    let channel = super::component::selected_channel(graph, ecus[0])?;
    let cluster = graph.ancestor(channel, "CAN-CLUSTER").unwrap();
    let rates = graph.descendants(cluster, "BAUDRATE");
    if rates.len() != 1 || graph.elements[rates[0]].text.parse::<u32>().ok() != rate {
        return Err(reject(
            graph,
            baudrate,
            "BAUDRATE_CONFLICT",
            crate::product_message!(
                "backend.integration.configuration.controller_cluster_bitrate_mismatch"
            ),
        ));
    }
    for (kind, symbol_ref, controller_ref, direction) in [
        (
            "CanIfHrhCfg",
            "CanIfHrhIdSymRef",
            "CanIfHrhCanCtrlIdRef",
            "RECEIVE",
        ),
        (
            "CanIfHthCfg",
            "CanIfHthIdSymRef",
            "CanIfHthCanCtrlIdRef",
            "TRANSMIT",
        ),
    ] {
        let handle = one(graph, controller, kind)?;
        let hardware = reference(graph, handle, symbol_ref)?;
        if !definition_is(graph, hardware, "CanHardwareObject")
            || reference(graph, hardware, "CanControllerRef")? != controller
            || reference(graph, handle, controller_ref)? != can_if_controller
            || value(graph, hardware, "CanObjectType", false) != Some(direction)
            || value(graph, hardware, "CanObjectId", false)
                .and_then(|value| value.parse::<u16>().ok())
                .is_none()
        {
            return Err(reject(
                graph,
                handle,
                "CONTROLLER_MAPPING",
                crate::product_message!(
                    "backend.integration.configuration.can_hardware_controller_direction_mismatch"
                ),
            ));
        }
        for pdu in graph.of_kind("ECUC-CONTAINER-VALUE") {
            if definition_is(
                graph,
                pdu,
                if direction == "RECEIVE" {
                    "CanIfRxPduCfg"
                } else {
                    "CanIfTxPduCfg"
                },
            ) {
                let actual = if direction == "RECEIVE" {
                    reference(graph, pdu, "CanIfRxPduHrhIdRef")?
                } else {
                    let buffer = reference(graph, pdu, "CanIfTxPduBufferRef")?;
                    reference(graph, buffer, "CanIfBufferHthRef")?
                };
                if actual != handle {
                    return Err(reject(
                        graph,
                        pdu,
                        "CONTROLLER_MAPPING",
                        crate::product_message!(
                            "backend.integration.configuration.pdu_hardware_handle_mismatch"
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn inspect_native(
    graph: &Graph,
    catalog: &crate::definitions::DefinitionCatalog,
) -> Result<Configuration, Vec<PlanDiagnostic>> {
    inspect_with_catalog(graph, Some(catalog))
}

pub(super) fn inspect(graph: &Graph) -> Result<Configuration, Vec<PlanDiagnostic>> {
    inspect_with_catalog(graph, None)
}

fn inspect_with_catalog(
    graph: &Graph,
    catalog: Option<&crate::definitions::DefinitionCatalog>,
) -> Result<Configuration, Vec<PlanDiagnostic>> {
    let multi = super::multi::selected(graph);
    let mut allowed: BTreeSet<_> = SUPPORTED_PARAMETERS.split_whitespace().collect();
    if multi {
        allowed.extend(MULTI_RUNTIME_PARAMETERS.split_ascii_whitespace());
    }
    let mut allowed_references: BTreeSet<_> = SUPPORTED_REFERENCES.split_whitespace().collect();
    if multi {
        allowed_references.extend(MULTI_RUNTIME_REFERENCES.split_ascii_whitespace());
    }
    let modules = graph.of_kind("ECUC-MODULE-CONFIGURATION-VALUES");
    let context = *graph.objects.values().next().unwrap();
    physical(graph, context, catalog.is_some())?;
    let mut selected = Vec::new();
    for module in MODULES {
        let definition = format!("/AUTOSAR/EcucDefs/{module}");
        let found: Vec<_> = modules
            .iter()
            .copied()
            .filter(|index| graph.text(*index, "DEFINITION-REF") == Some(definition.as_str()))
            .collect();
        if found.len() != 1 {
            return Err(reject(
                graph,
                found.first().copied().unwrap_or(context),
                "MODULE_NOT_UNIQUE",
                crate::product_message!(
                    "backend.integration.configuration.module_configuration_value_set_required"
                ),
            ));
        }
        selected.push(found[0]);
    }
    if multi {
        for name in ["ComM", "BswM"] {
            let definition = format!("/AUTOSAR/EcucDefs/{name}");
            let found: Vec<_> = modules
                .iter()
                .copied()
                .filter(|index| graph.text(*index, "DEFINITION-REF") == Some(definition.as_str()))
                .collect();
            if found.len() != 1 {
                return Err(reject(
                    graph,
                    context,
                    "MODE_CONFIGURATION",
                    crate::product_message!("backend.integration.multi.contract_invalid", "code" => "MODE_CONFIGURATION"),
                ));
            }
            selected.push(found[0]);
        }
    }
    let os = one(graph, context, "OsOS")?;
    let hooks = one(graph, os, "OsHooks")?;
    let os_module = selected[MODULES.iter().position(|module| *module == "Os").unwrap()];
    let owner = |index: usize| {
        graph.elements[index]
            .parent
            .and_then(|parent| graph.elements[parent].parent)
    };
    if owner(hooks) != Some(os) || owner(os) != Some(os_module) {
        return Err(reject(
            graph,
            hooks,
            "OS_CONFIGURATION",
            crate::product_message!(
                "backend.integration.configuration.os_hooks_ownership_mismatch"
            ),
        ));
    }
    let os_policies = [
        (
            os,
            &[
                "OsStatus",
                "OsScalabilityClass",
                "OsUseGetServiceId",
                "OsUseParameterAccess",
                "OsUseResScheduler",
            ][..],
        ),
        (
            hooks,
            &[
                "OsErrorHook",
                "OsPostTaskHook",
                "OsPreTaskHook",
                "OsProtectionHook",
                "OsShutdownHook",
                "OsStartupHook",
            ][..],
        ),
    ];
    for (container, parameters) in os_policies {
        if parameters
            .iter()
            .any(|parameter| value(graph, container, parameter, false).is_none())
        {
            return Err(reject(
                graph,
                container,
                "OS_CONFIGURATION",
                crate::product_message!(
                    "backend.integration.configuration.os_configuration_parameter_missing_or_duplicate"
                ),
            ));
        }
    }
    if !matches!(
        value(graph, os, "OsUseResScheduler", false),
        Some("true" | "false" | "1" | "0")
    ) {
        return Err(reject(
            graph,
            os,
            "OS_CONFIGURATION",
            crate::product_message!(
                "backend.integration.configuration.scheduler_resource_explicit_boolean_required"
            ),
        ));
    }
    let mut records = Vec::new();
    for container in graph.of_kind("ECUC-CONTAINER-VALUE") {
        if !selected
            .iter()
            .any(|module| graph.within(container, *module))
        {
            continue;
        }
        if definition_is(graph, container, "OsResource") {
            if matches!(
                value(graph, os, "OsUseResScheduler", false),
                Some("true" | "1")
            ) && graph.text(container, "SHORT-NAME") == Some("RES_SCHEDULER")
            {
                // SWS_Os_00850: the virtual scheduler instance replaces this
                // configured definition, including its declared properties.
                continue;
            }
            return Err(reject(
                graph,
                container,
                "OS_RESOURCE_PROFILE",
                crate::product_message!(
                    "backend.integration.configuration.explicit_scheduler_resource_unsupported"
                ),
            ));
        }
        let mut parameters = BTreeMap::<String, Vec<String>>::new();
        let mut references = BTreeMap::<String, Vec<String>>::new();
        for (group, target, reference) in [
            ("PARAMETER-VALUES", &mut parameters, false),
            ("REFERENCE-VALUES", &mut references, true),
        ] {
            for group in graph.children(container, group) {
                for index in &graph.elements[group].children {
                    let definition = graph.text(*index, "DEFINITION-REF").unwrap_or("");
                    let name = definition.rsplit('/').next().unwrap_or("");
                    let text = graph
                        .text(*index, if reference { "VALUE-REF" } else { "VALUE" })
                        .unwrap_or("");
                    if (!reference && !allowed.contains(name))
                        || (reference && !allowed_references.contains(name))
                    {
                        if catalog.is_some_and(|catalog| {
                            super::native_configuration::descriptor_only(
                                graph, *index, catalog, reference,
                            )
                        }) {
                            continue;
                        }
                        return Err(reject(
                            graph,
                            *index,
                            "TARGET_PARAMETER_UNSUPPORTED",
                            crate::product_message!(
                                "backend.integration.configuration.explicit_parameter_implementation_unsupported"
                            ),
                        ));
                    }
                    target
                        .entry(definition.into())
                        .or_default()
                        .push(text.into());
                }
            }
        }
        if parameters.values().any(|values| values.len() != 1) {
            return Err(reject(
                graph,
                container,
                "PARAMETER_NOT_UNIQUE",
                crate::product_message!(
                    "backend.integration.configuration.scalar_configuration_parameter_duplicate"
                ),
            ));
        }
        records.push(ConfigurationRecord {
            path: graph.elements[container].object.clone(),
            definition: graph.text(container, "DEFINITION-REF").unwrap_or("").into(),
            parameters,
            references,
        });
    }
    // Fixed policies are explicit target choices; numeric identifiers and
    // symbolic names remain user inputs, checked separately in their domains.
    let fixed = [
        ("CanBusoffProcessing", "POLLING"),
        ("CanRxProcessing", "POLLING"),
        ("CanTxProcessing", "POLLING"),
        ("CanWakeupProcessing", "POLLING"),
        ("CanHandleType", "BASIC"),
        ("CanIdType", "STANDARD"),
        ("CanHwObjectCount", "1"),
        ("CanIfBufferSize", "0"),
        ("CanIfRxPduCanIdType", "STANDARD_CAN"),
        ("CanIfTxPduCanIdType", "STANDARD_CAN"),
        ("CanIfTxPduType", "STATIC"),
        ("ComTransferProperty", "PENDING"),
        ("ComRxDataTimeoutAction", "NONE"),
        ("OsScalabilityClass", "SC1"),
        ("DcmDspSessionForBoot", "DCM_NO_BOOT"),
        ("DcmDspDataScalingInfoSize", "0"),
        ("DcmDspMaxDidToRead", "2"),
    ];
    let booleans = [
        ("CanControllerActivation", true),
        ("CanHwPnSupport", false),
        ("CanWakeupSupport", false),
        ("CanIfHrhSoftwareFilter", true),
        ("CanIfRxPduReadData", false),
        ("CanIfRxPduReadNotifyStatus", false),
        ("CanIfTxPduReadNotifyStatus", false),
        ("CanIfTxPduTruncation", false),
        ("OsErrorHook", true),
        ("OsPostTaskHook", true),
        ("OsPreTaskHook", true),
        ("OsProtectionHook", false),
        ("OsShutdownHook", true),
        ("OsStartupHook", true),
        ("OsUseGetServiceId", true),
        ("OsUseParameterAccess", true),
    ];
    for record in &records {
        let index = *graph.objects.get(&record.path).unwrap();
        for definition in record.parameters.keys().filter(|definition| {
            definition.starts_with("/AUTOSAR/EcucDefs/Os/OsOS/")
                || os_policies
                    .iter()
                    .any(|(_, names)| names.contains(&definition.rsplit('/').next().unwrap_or("")))
        }) {
            let expected = definition.rsplit_once('/').unwrap().0;
            if record.definition != expected {
                return Err(reject(
                    graph,
                    index,
                    "OS_CONFIGURATION",
                    crate::product_message!(
                        "backend.integration.configuration.os_policy_container_mismatch"
                    ),
                ));
            }
        }
        if let Some(status) = value(graph, index, "OsStatus", false) {
            if !matches!(status, "STANDARD" | "EXTENDED") {
                return Err(reject(
                    graph,
                    index,
                    "TARGET_PARAMETER_UNSUPPORTED",
                    crate::product_message!(
                        "backend.integration.configuration.sc1_os_status_unsupported"
                    ),
                ));
            }
        }
        for (parameter, expected) in fixed {
            if let Some(actual) = value(graph, index, parameter, false) {
                if actual != expected {
                    return Err(reject(
                        graph,
                        index,
                        "TARGET_PARAMETER_UNSUPPORTED",
                        crate::product_message!(
                            "backend.integration.configuration.fixed_target_policy_value_mismatch"
                        ),
                    ));
                }
            }
        }
        for (parameter, expected) in booleans {
            if let Some(actual) = value(graph, index, parameter, false) {
                let boolean = match actual {
                    "true" | "1" => Some(true),
                    "false" | "0" => Some(false),
                    _ => None,
                };
                if boolean != Some(expected) {
                    return Err(reject(
                        graph,
                        index,
                        "TARGET_PARAMETER_UNSUPPORTED",
                        crate::product_message!(
                            "backend.integration.configuration.fixed_target_boolean_policy_mismatch"
                        ),
                    ));
                }
            }
        }
        for parameter in ["CanTpMainFunctionPeriod", "DcmTaskTime"] {
            if value(graph, index, parameter, false).is_some()
                && value(graph, index, parameter, false).and_then(milliseconds) != Some(1)
            {
                return Err(reject(
                    graph,
                    index,
                    "PERIOD_ALARM_CONFLICT",
                    crate::product_message!(
                        "backend.integration.configuration.bsw_main_function_period_mismatch"
                    ),
                ));
            }
        }
        for (definition, literals) in &record.parameters {
            let name = definition.rsplit('/').next().unwrap_or("");
            if (name.ends_with("Id") || name.ends_with("HandleId"))
                && graph.external.get(definition).map(String::as_str)
                    == Some("ECUC-INTEGER-PARAM-DEF")
                && literals[0].parse::<u16>().is_err()
            {
                return Err(reject(
                    graph,
                    index,
                    "CONFIGURATION_IDENTIFIER",
                    crate::product_message!(
                        "backend.integration.configuration.public_identifier_out_of_range"
                    ),
                ));
            }
        }
    }
    let tasks: Vec<_> = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, "OsTask"))
        .collect();
    let task = tasks.first().copied().ok_or_else(|| {
        reject(
            graph,
            context,
            "TASK_PROFILE",
            crate::product_message!("backend.integration.configuration.ecu_owner_task_missing"),
        )
    })?;
    let task_events: BTreeSet<_> = values(graph, task, "OsTaskEventRef", true)
        .into_iter()
        .collect();
    let mut event_nodes: Vec<_> = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| {
            definition_is(graph, *index, "OsEvent")
                && task_events.contains(graph.elements[*index].object.as_str())
        })
        .collect();
    event_nodes.sort_by(|left, right| {
        graph.elements[*left]
            .object
            .cmp(&graph.elements[*right].object)
    });
    let mut assigned = BTreeMap::new();
    let mut used = 0u32;
    for event in &event_nodes {
        // R24-11 MOD ECUC_Os_00034: OIL AUTO is represented by omitting
        // this optional integer parameter, never by an XML string value.
        if let Some(text) = value(graph, *event, "OsEventMask", false) {
            let mask = text
                .parse::<u32>()
                .ok()
                .filter(|mask| mask.count_ones() == 1)
                .ok_or_else(|| {
                    reject(
                        graph,
                        *event,
                        "EVENT_MASK",
                        crate::product_message!(
                            "backend.integration.configuration.event_mask_single_bit_required"
                        ),
                    )
                })?;
            if used & mask != 0 {
                return Err(reject(
                    graph,
                    *event,
                    "EVENT_MASK",
                    crate::product_message!(
                        "backend.integration.configuration.owner_event_mask_bit_duplicate"
                    ),
                ));
            }
            used |= mask;
            assigned.insert(*event, mask);
        }
    }
    let mut events = Vec::new();
    for (handle, event) in event_nodes.into_iter().enumerate() {
        let mask = if let Some(mask) = assigned.get(&event) {
            *mask
        } else {
            let bit = (0..32)
                .find(|bit| used & (1u32 << bit) == 0)
                .ok_or_else(|| {
                    reject(
                        graph,
                        event,
                        "EVENT_MASK",
                        crate::product_message!(
                            "backend.integration.configuration.auto_event_bits_exhausted"
                        ),
                    )
                })?;
            used |= 1u32 << bit;
            1u32 << bit
        };
        events.push(EventAssignment {
            path: graph.elements[event].object.clone(),
            handle: handle as u8,
            mask,
        });
    }
    Ok(Configuration { records, events })
}
