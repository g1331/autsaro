//! Source-backed communication types and selected CAN polling configuration.
use super::c_layout;
use super::component::milliseconds;
use super::graph::Graph;
use super::schedule::{ScheduleContract, definition_is, value, values};
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub enum CommunicationIntegerType {
    Uint8,
    Uint16,
    Uint32,
}
impl CommunicationIntegerType {
    fn maximum(self) -> u32 {
        match self {
            Self::Uint8 => u32::from(u8::MAX),
            Self::Uint16 => u32::from(u16::MAX),
            Self::Uint32 => u32::MAX,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct CanRuntimeContract {
    pub physical_channel: String,
    pub controller: String,
    pub controller_id: u8,
    pub can_if_controller_id: u8,
    pub receive_hardware: String,
    pub receive_id_type: String,
    pub receive_handle: u16,
    pub transmit_hardware: String,
    pub transmit_id_type: String,
    pub transmit_handle: u16,
    pub read_write_period: String,
    pub read_write_period_ms: u32,
    pub busoff_period_ms: u32,
    pub mode_period_ms: u32,
    pub receive_polling: bool,
    pub transmit_polling: bool,
    pub busoff_polling: bool,
    pub wakeup_polling: bool,
    pub nominal_bitrate: u32,
    pub data_bitrate: Option<u32>,
    pub brs: bool,
    pub fd_padding: u8,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct PartitionRuntimeContract {
    pub path: String,
    pub name: String,
    pub id: u16,
    pub core: String,
    pub root_composition: String,
    pub instances: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct CommunicationRuntimeContract {
    pub pdu_collection: String,
    pub pdu_id_type: CommunicationIntegerType,
    pub pdu_length_type: CommunicationIntegerType,
    pub can: CanRuntimeContract,
    pub partition: PartitionRuntimeContract,
}

fn fail(graph: &Graph, index: usize, code: &str) -> Vec<PlanDiagnostic> {
    vec![graph.diagnostic(
        index,
        DiagnosticCategory::Input,
        code,
        crate::product_message!("backend.integration.composition.contract_invalid", "code" => code),
        crate::product_message!("backend.integration.composition.repair_contract"),
    )]
}
fn one(graph: &Graph, kind: &str) -> Result<usize, Vec<PlanDiagnostic>> {
    let found: Vec<_> = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, kind))
        .collect();
    if found.len() == 1 {
        Ok(found[0])
    } else {
        Err(fail(
            graph,
            found.first().copied().unwrap_or(0),
            "COMMUNICATION_CONFIGURATION",
        ))
    }
}
fn integer_type(
    graph: &Graph,
    collection: usize,
    name: &str,
) -> Result<CommunicationIntegerType, Vec<PlanDiagnostic>> {
    match value(graph, collection, name, false) {
        Some("UINT8") => Ok(CommunicationIntegerType::Uint8),
        Some("UINT16") => Ok(CommunicationIntegerType::Uint16),
        Some("UINT32") if name == "PduLengthTypeEnum" => Ok(CommunicationIntegerType::Uint32),
        _ => Err(fail(graph, collection, "COMMUNICATION_TYPE")),
    }
}
fn period(
    graph: &Graph,
    container: usize,
    name: &str,
    schedule: &ScheduleContract,
) -> Result<u32, Vec<PlanDiagnostic>> {
    value(graph, container, name, false)
        .and_then(milliseconds)
        .filter(|period| *period == schedule.counter_tick_ms)
        .ok_or_else(|| fail(graph, container, "CAN_POLLING_TIMEBASE"))
}

fn partition(
    graph: &Graph,
    composition: &super::composition::CompositionContract,
) -> Result<PartitionRuntimeContract, Vec<PlanDiagnostic>> {
    let index = one(graph, "EcucPartition")?;
    if super::composition::reserved_memory_scope(graph.text(index, "SHORT-NAME").unwrap_or("")) {
        return Err(fail(graph, index, "CONTRACT_NAME_COLLISION"));
    }

    let core = one(graph, "EcucCoreDefinition")?;
    if value(graph, index, "EcucPartitionId", false).and_then(|id| id.parse::<u16>().ok())
        != Some(0)
        || value(graph, core, "EcucCoreId", false).and_then(|id| id.parse::<u16>().ok()) != Some(0)
        || value(graph, index, "EcucPartitionCoreRef", true)
            != Some(graph.elements[core].object.as_str())
    {
        return Err(fail(graph, index, "RTE_PARTITION_CONFIGURATION"));
    }
    let roots = graph.of_kind("ROOT-SW-COMPOSITION-PROTOTYPE");
    let root = roots
        .iter()
        .copied()
        .filter(|root| {
            graph.text(*root, "SOFTWARE-COMPOSITION-TREF") == Some(composition.composition.as_str())
                && graph
                    .objects
                    .get(&composition.system)
                    .is_some_and(|system| graph.within(*root, *system))
        })
        .collect::<Vec<_>>();
    if root.len() != 1 {
        return Err(fail(graph, index, "RTE_PARTITION_CONFIGURATION"));
    }
    let root_path = graph.elements[root[0]].object.as_str();
    let mut instances = Vec::new();
    for group in graph.children(index, "REFERENCE-VALUES") {
        for reference in &graph.elements[group].children {
            if graph
                .text(*reference, "DEFINITION-REF")
                .is_some_and(|id| id.ends_with("/EcucPartitionSoftwareComponentInstanceRef"))
            {
                let irefs = graph.children(*reference, "VALUE-IREF");
                if irefs.len() != 1
                    || graph.elements[irefs[0]].children.len() != 2
                    || graph.text(irefs[0], "CONTEXT-ELEMENT-REF") != Some(root_path)
                {
                    return Err(fail(graph, *reference, "RTE_PARTITION_CONFIGURATION"));
                }
                let target = graph
                    .text(irefs[0], "TARGET-REF")
                    .ok_or_else(|| fail(graph, *reference, "RTE_PARTITION_CONFIGURATION"))?;
                instances.push(target.to_string());
            }
        }
    }
    instances.sort();
    let mut expected = composition
        .components
        .iter()
        .map(|component| component.instance.clone())
        .collect::<Vec<_>>();
    expected.sort();
    if instances != expected {
        return Err(fail(graph, index, "RTE_PARTITION_CONFIGURATION"));
    }
    for (kind, reference) in [
        ("RteComUser", "RteComUserEcucPartitionRef"),
        ("ComMainFunctionRx", "ComMainRxPartitionRef"),
        ("ComMainFunctionTx", "ComMainTxPartitionRef"),
    ] {
        let consumer = one(graph, kind)?;
        if values(graph, consumer, reference, true) != [graph.elements[index].object.as_str()] {
            return Err(fail(graph, consumer, "RTE_PARTITION_CONFIGURATION"));
        }
    }
    Ok(PartitionRuntimeContract {
        path: graph.elements[index].object.clone(),
        name: graph
            .text(index, "SHORT-NAME")
            .ok_or_else(|| fail(graph, index, "RTE_PARTITION_CONFIGURATION"))?
            .into(),
        id: 0,
        core: graph.elements[core].object.clone(),
        root_composition: root_path.into(),
        instances,
    })
}

fn bitrates(
    graph: &Graph,
    controller: usize,
) -> Result<(u32, Option<u32>, bool), Vec<PlanDiagnostic>> {
    let baud = value(graph, controller, "CanControllerDefaultBaudrate", true)
        .and_then(|path| graph.objects.get(path).copied())
        .filter(|index| {
            definition_is(graph, *index, "CanControllerBaudrateConfig")
                && graph.within(*index, controller)
        })
        .ok_or_else(|| fail(graph, controller, "CONTROLLER_MAPPING"))?;
    let nominal = value(graph, baud, "CanControllerBaudRate", false)
        .and_then(super::native_configuration::baudrate_kbit)
        .and_then(|value| value.checked_mul(1000))
        .ok_or_else(|| fail(graph, baud, "BAUDRATE_CONFLICT"))?;
    let fd: Vec<_> = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, "CanControllerFdBaudrateConfig"))
        .collect();
    let [config] = fd.as_slice() else {
        return if fd.is_empty() {
            Ok((nominal, None, false))
        } else {
            Err(fail(graph, fd[1], "CAN_FD_CONFIGURATION"))
        };
    };
    if graph.elements[*config]
        .parent
        .and_then(|parent| graph.elements[parent].parent)
        != Some(baud)
    {
        return Err(fail(graph, *config, "CAN_FD_CONFIGURATION"));
    }
    let data = value(graph, *config, "CanControllerFdBaudRate", false)
        .and_then(super::native_configuration::baudrate_kbit)
        .filter(|rate| *rate <= 16_000)
        .map(|rate| rate * 1000)
        .ok_or_else(|| fail(graph, *config, "CAN_FD_CONFIGURATION"))?;
    let brs = match value(graph, *config, "CanControllerTxBitRateSwitch", false) {
        Some("true" | "1") => true,
        Some("false" | "0") => false,
        _ => return Err(fail(graph, *config, "CAN_FD_CONFIGURATION")),
    };
    Ok((nominal, Some(data), brs))
}

pub(super) fn inspect(
    graph: &Graph,
    schedule: &ScheduleContract,
    composition: &super::composition::CompositionContract,
) -> Result<CommunicationRuntimeContract, Vec<PlanDiagnostic>> {
    let partition = partition(graph, composition)?;
    let collection = one(graph, "EcucPduCollection")?;
    let pdu_id_type = integer_type(graph, collection, "PduIdTypeEnum")?;
    let pdu_length_type = integer_type(graph, collection, "PduLengthTypeEnum")?;
    // Every configured handle must fit its actual public communication type.
    let identifiers = [
        "CanIfRxPduId",
        "CanIfTxPduId",
        "CanTpRxNPduId",
        "CanTpRxNSduId",
        "CanTpTxFcNPduConfirmationPduId",
        "CanTpRxFcNPduId",
        "CanTpTxNPduConfirmationPduId",
        "CanTpTxNSduId",
        "DcmDslProtocolRxPduId",
        "DcmDslTxConfirmationPduId",
        "PduRSourcePduHandleId",
        "PduRDestPduHandleId",
    ];
    for container in graph.of_kind("ECUC-CONTAINER-VALUE") {
        for name in identifiers {
            for text in values(graph, container, name, false) {
                if text
                    .parse::<u32>()
                    .ok()
                    .filter(|id| *id <= pdu_id_type.maximum())
                    .is_none()
                {
                    return Err(fail(graph, container, "COMMUNICATION_HANDLE_RANGE"));
                }
            }
        }
        for text in values(graph, container, "PduLength", false) {
            if text
                .parse::<u32>()
                .ok()
                .filter(|length| *length <= pdu_length_type.maximum())
                .is_none()
            {
                return Err(fail(graph, container, "COMMUNICATION_LENGTH_RANGE"));
            }
        }
    }
    let controller = one(graph, "CanController")?;
    let (nominal_bitrate, data_bitrate, brs) = bitrates(graph, controller)?;
    let mut fd_padding = 0;
    for name in [
        "CanRxProcessing",
        "CanTxProcessing",
        "CanBusoffProcessing",
        "CanWakeupProcessing",
    ] {
        if value(graph, controller, name, false) != Some("POLLING") {
            return Err(fail(graph, controller, "CAN_PROCESSING"));
        }
    }
    let general = one(graph, "CanGeneral")?;
    if !matches!(
        value(graph, general, "CanDevErrorDetect", false),
        Some("false" | "0")
    ) {
        return Err(fail(graph, general, "CAN_FEATURE_UNSUPPORTED"));
    }
    if !matches!(
        value(graph, general, "CanVersionInfoApi", false),
        None | Some("false" | "0")
    ) {
        return Err(fail(graph, general, "CAN_FEATURE_UNSUPPORTED"));
    }
    let rw = one(graph, "CanMainFunctionRWPeriods")?;
    let rw_path = graph.elements[rw].object.as_str();
    let hardware: Vec<_> = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, "CanHardwareObject"))
        .collect();
    let mut receive = None;
    let mut transmit = None;
    for index in hardware {
        if value(graph, index, "CanMainFunctionRWPeriodRef", true) != Some(rw_path) {
            return Err(fail(graph, index, "CAN_POLLING_REFERENCE"));
        }
        if !matches!(
            value(graph, index, "CanTriggerTransmitEnable", false),
            None | Some("false" | "0")
        ) {
            return Err(fail(graph, index, "CAN_FEATURE_UNSUPPORTED"));
        }
        let handle = value(graph, index, "CanObjectId", false)
            .and_then(|text| text.parse::<u16>().ok())
            .ok_or_else(|| fail(graph, index, "CAN_HARDWARE_HANDLES"))?;
        let selected = match value(graph, index, "CanObjectType", false) {
            Some("RECEIVE") => &mut receive,
            Some("TRANSMIT") => &mut transmit,
            _ => return Err(fail(graph, index, "CAN_HARDWARE_HANDLES")),
        };
        if let Some(padding) = value(graph, index, "CanFdPaddingValue", false) {
            if value(graph, index, "CanObjectType", false) != Some("TRANSMIT") {
                return Err(fail(graph, index, "CAN_FD_PADDING"));
            }
            fd_padding = crate::definitions::unsigned_integer(padding)
                .and_then(|padding| u8::try_from(padding).ok())
                .ok_or_else(|| fail(graph, index, "CAN_FD_PADDING"))?;
        }
        let id_type = match value(graph, index, "CanIdType", false) {
            Some(format @ ("STANDARD" | "EXTENDED" | "MIXED")) => format,
            _ => return Err(fail(graph, index, "CAN_ID_CONFLICT")),
        };
        if selected
            .replace((
                graph.elements[index].object.clone(),
                handle,
                id_type.to_owned(),
            ))
            .is_some()
        {
            return Err(fail(graph, index, "CAN_HARDWARE_HANDLES"));
        }
    }
    let (receive_hardware, receive_handle, receive_id_type) =
        receive.ok_or_else(|| fail(graph, controller, "CAN_HARDWARE_HANDLES"))?;
    let (transmit_hardware, transmit_handle, transmit_id_type) =
        transmit.ok_or_else(|| fail(graph, controller, "CAN_HARDWARE_HANDLES"))?;
    // The selected two-object common HRH/HTH range is contiguous from zero
    // (ECUC_Can_00326, SWS_CANIF_00115).
    if (receive_handle == transmit_handle) || (receive_handle > 1) || (transmit_handle > 1) {
        return Err(fail(graph, controller, "CAN_HARDWARE_HANDLES"));
    }
    // Both selected namespaces contain one controller, numbered from zero
    // (ECUC_Can_00316, SWS_CANIF_00653 and 00433), despite the wider metadata range.
    let controller_id = value(graph, controller, "CanControllerId", false)
        .and_then(|text| text.parse::<u8>().ok())
        .filter(|id| *id == 0)
        .ok_or_else(|| fail(graph, controller, "CAN_CONTROLLER_ID"))?;
    let can_if_controller = one(graph, "CanIfCtrlCfg")?;
    let can_if_controller_id = value(graph, can_if_controller, "CanIfCtrlId", false)
        .and_then(|text| text.parse::<u8>().ok())
        .filter(|id| *id == 0)
        .ok_or_else(|| fail(graph, can_if_controller, "CAN_CONTROLLER_ID"))?;
    for (symbol, configured) in [
        (
            "Can_MainFunction_Read",
            period(graph, rw, "CanMainFunctionPeriod", schedule)?,
        ),
        (
            "Can_MainFunction_Write",
            period(graph, rw, "CanMainFunctionPeriod", schedule)?,
        ),
        (
            "Can_MainFunction_Mode",
            period(graph, general, "CanMainFunctionModePeriod", schedule)?,
        ),
        (
            "Can_MainFunction_BusOff",
            period(graph, general, "CanMainFunctionBusoffPeriod", schedule)?,
        ),
    ] {
        if !schedule.entities.iter().any(|entity| {
            !entity.application && entity.symbol == symbol && entity.period_ms == configured
        }) {
            return Err(fail(graph, general, "CAN_POLLING_TIMEBASE"));
        }
    }
    let ecu = graph.of_kind("ECU-INSTANCE");
    let [ecu] = ecu.as_slice() else {
        return Err(fail(graph, controller, "CONTROLLER_MAPPING"));
    };
    let channel = super::component::selected_channel(graph, *ecu)?;
    Ok(CommunicationRuntimeContract {
        partition,
        pdu_collection: graph.elements[collection].object.clone(),
        pdu_id_type,
        pdu_length_type,
        can: CanRuntimeContract {
            physical_channel: graph.elements[channel].object.clone(),
            controller: graph.elements[controller].object.clone(),
            controller_id,
            can_if_controller_id,
            receive_hardware,
            receive_id_type,
            receive_handle,
            transmit_hardware,
            transmit_id_type,
            transmit_handle,
            read_write_period: rw_path.into(),
            read_write_period_ms: period(graph, rw, "CanMainFunctionPeriod", schedule)?,
            busoff_period_ms: period(graph, general, "CanMainFunctionBusoffPeriod", schedule)?,
            mode_period_ms: period(graph, general, "CanMainFunctionModePeriod", schedule)?,
            receive_polling: true,
            transmit_polling: true,
            busoff_polling: true,
            wakeup_polling: true,
            nominal_bitrate,
            data_bitrate,
            brs,
            fd_padding,
        },
    })
}

/// ECU-only CAN selection; component contracts do not consume driver configuration.
pub(super) fn profile_header(
    communication: &CommunicationRuntimeContract,
    can_if_controller: &str,
) -> (String, Vec<u8>) {
    let can = &communication.can;
    let profile = super::c_layout::commented_defines(
        &[
            ("CAN_HOST_QUEUED_COMPLETION", "STD_ON".to_owned()),
            ("CAN_RX_POLLING", "STD_ON".to_owned()),
            ("CAN_TX_POLLING", "STD_ON".to_owned()),
            ("CAN_BUSOFF_POLLING", "STD_ON".to_owned()),
            (
                "CAN_CONTROLLER_ID",
                super::symbolic_names::name("Can", "CanController", &can.controller),
            ),
            ("CAN_CANIF_CONTROLLER_ID", can_if_controller.to_owned()),
            (
                "CAN_RX_ID_TYPE",
                format!("CAN_HOH_ID_{}", can.receive_id_type),
            ),
            (
                "CAN_TX_ID_TYPE",
                format!("CAN_HOH_ID_{}", can.transmit_id_type),
            ),
            (
                "CAN_TX_HOH",
                super::symbolic_names::name("Can", "CanHardwareObject", &can.transmit_hardware),
            ),
            (
                "CAN_RX_HOH",
                super::symbolic_names::name("Can", "CanHardwareObject", &can.receive_hardware),
            ),
        ],
        "Generated CAN selection.",
    );
    (
        "include/Ecu_ProfileConfig.h".into(),
        format!(
            r#"/** @file Generated communication selection. */
{guard_start}#include "Std_Types.h"
#include "Can_Cfg.h"
#include "CanIf_Cfg.h"
{checks}/** ECUC CanRxProcessing, CanTxProcessing and CanBusoffProcessing select POLLING.
 * The generated host target owns queued completion and the controller/HOH handles.
 */
{profile}#endif
"#,
            checks = [
                ("Std_Types.h", "STD_TYPES"),
                ("Can_Cfg.h", "CAN"),
                ("CanIf_Cfg.h", "CANIF")
            ]
            .into_iter()
            .map(|(h, p)| c_layout::release_check("Ecu_ProfileConfig.h", h, p))
            .collect::<String>(),
            guard_start = c_layout::header_start("ECU_PROFILE_CONFIG_H")
        )
        .into_bytes(),
    )
}

/// One producer for the selected standard communication widths.
pub(super) fn type_headers(communication: &CommunicationRuntimeContract) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    let integer = |kind: super::CommunicationIntegerType| match kind {
        super::CommunicationIntegerType::Uint8 => "uint8",
        super::CommunicationIntegerType::Uint16 => "uint16",
        super::CommunicationIntegerType::Uint32 => "uint32",
    };
    files.push((
        "include/ComStack_Cfg.h",
        format!(
            r#"/** @file EcuC-selected communication widths. */
{guard_start}#include "ComStack_Version.h"
#include "Platform_Types.h"
/** PDU handle width selected by EcuC PduIdTypeEnum. */
typedef {} PduIdType;
/** PDU byte-count width selected by EcuC PduLengthTypeEnum. */
typedef {} PduLengthType;
#endif
"#,
            integer(communication.pdu_id_type),
            integer(communication.pdu_length_type),
            guard_start = c_layout::header_start("COMSTACK_CFG_H")
        ),
    ));
    files
        .into_iter()
        .map(|(path, text)| (path.into(), text.into_bytes()))
        .collect()
}
