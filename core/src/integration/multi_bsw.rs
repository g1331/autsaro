//! Source-backed communication types and selected CAN polling configuration.
use super::component::milliseconds;
use super::graph::Graph;
use super::schedule::{ScheduleContract, definition_is, value, values};
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
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
pub struct CanRuntimeContract {
    pub controller: String,
    pub controller_id: u8,
    pub can_if_controller_id: u8,
    pub receive_hardware: String,
    pub receive_handle: u16,
    pub transmit_hardware: String,
    pub transmit_handle: u16,
    pub read_write_period: String,
    pub read_write_period_ms: u32,
    pub busoff_period_ms: u32,
    pub mode_period_ms: u32,
    pub receive_polling: bool,
    pub transmit_polling: bool,
    pub busoff_polling: bool,
    pub wakeup_polling: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunicationRuntimeContract {
    pub pdu_collection: String,
    pub pdu_id_type: CommunicationIntegerType,
    pub pdu_length_type: CommunicationIntegerType,
    pub can: CanRuntimeContract,
}

fn fail(graph: &Graph, index: usize, code: &str) -> Vec<PlanDiagnostic> {
    vec![graph.diagnostic(
        index,
        DiagnosticCategory::Input,
        code,
        crate::product_message!("backend.integration.multi.contract_invalid", "code" => code),
        crate::product_message!("backend.integration.multi.repair_contract"),
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

pub(super) fn inspect(
    graph: &Graph,
    schedule: &ScheduleContract,
) -> Result<CommunicationRuntimeContract, Vec<PlanDiagnostic>> {
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
        if selected
            .replace((graph.elements[index].object.clone(), handle))
            .is_some()
        {
            return Err(fail(graph, index, "CAN_HARDWARE_HANDLES"));
        }
    }
    let (receive_hardware, receive_handle) =
        receive.ok_or_else(|| fail(graph, controller, "CAN_HARDWARE_HANDLES"))?;
    let (transmit_hardware, transmit_handle) =
        transmit.ok_or_else(|| fail(graph, controller, "CAN_HARDWARE_HANDLES"))?;
    if receive_handle == transmit_handle {
        return Err(fail(graph, controller, "CAN_HARDWARE_HANDLES"));
    }
    let controller_id = value(graph, controller, "CanControllerId", false)
        .and_then(|text| text.parse::<u8>().ok())
        .ok_or_else(|| fail(graph, controller, "CAN_CONTROLLER_ID"))?;
    let can_if_controller = one(graph, "CanIfCtrlCfg")?;
    let can_if_controller_id = value(graph, can_if_controller, "CanIfCtrlId", false)
        .and_then(|text| text.parse::<u8>().ok())
        .ok_or_else(|| fail(graph, can_if_controller, "CAN_CONTROLLER_ID"))?;
    Ok(CommunicationRuntimeContract {
        pdu_collection: graph.elements[collection].object.clone(),
        pdu_id_type,
        pdu_length_type,
        can: CanRuntimeContract {
            controller: graph.elements[controller].object.clone(),
            controller_id,
            can_if_controller_id,
            receive_hardware,
            receive_handle,
            transmit_hardware,
            transmit_handle,
            read_write_period: rw_path.into(),
            read_write_period_ms: period(graph, rw, "CanMainFunctionPeriod", schedule)?,
            busoff_period_ms: period(graph, general, "CanMainFunctionBusoffPeriod", schedule)?,
            mode_period_ms: period(graph, general, "CanMainFunctionModePeriod", schedule)?,
            receive_polling: true,
            transmit_polling: true,
            busoff_polling: true,
            wakeup_polling: true,
        },
    })
}
