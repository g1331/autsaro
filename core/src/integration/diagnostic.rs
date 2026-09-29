use super::communication::check_trigger;
use super::component::{ComponentContract, milliseconds};
use super::graph::Graph;
use super::schedule::{definition_is, value, values};
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticContract {
    pub data: String,
    pub did_object: String,
    pub did: u16,
    pub client_port: String,
    pub rx_sdu: String,
    pub tx_sdu: String,
    pub rx_npdu: String,
    pub tx_npdu: String,
    pub request_can_id: u32,
    pub response_can_id: u32,
    pub request_can_if_handle: u16,
    pub response_can_if_handle: u16,
    pub p2_ms: u32,
    pub p2_star_ms: u32,
    pub s3_ms: u32,
    pub n_ar_ms: u32,
    pub n_br_ms: u32,
    pub n_cr_ms: u32,
    pub n_as_ms: u32,
    pub n_bs_ms: u32,
    pub n_cs_ms: u32,
    pub buffer_bytes: u32,
    pub sessions: Vec<u8>,
}

fn reject(graph: &Graph, index: usize, code: &str, message: &str) -> Vec<PlanDiagnostic> {
    vec![graph.diagnostic(index, DiagnosticCategory::Input, code, message,
        "Use one physical normal-addressed DoCAN connection, the same synchronous four-byte service, explicit canonical PDUs and matching transport/diagnostic declarations.")]
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
            "DIAGNOSTIC_NOT_UNIQUE",
            "A required diagnostic/transport object is missing or duplicated.",
        ))
    }
}

fn reference(graph: &Graph, context: usize, parameter: &str) -> Result<usize, Vec<PlanDiagnostic>> {
    value(graph, context, parameter, true)
        .and_then(|path| graph.objects.get(path).copied())
        .ok_or_else(|| {
            reject(
                graph,
                context,
                "DIAGNOSTIC_REFERENCE",
                "A required diagnostic reference is missing or duplicated.",
            )
        })
}

fn timer(graph: &Graph, context: usize, parameter: &str) -> Result<u32, Vec<PlanDiagnostic>> {
    value(graph, context, parameter, false)
        .and_then(milliseconds)
        .ok_or_else(|| {
            reject(
                graph,
                context,
                "DIAGNOSTIC_TIMING",
                "The configured timer must be a positive whole logical millisecond.",
            )
        })
}

fn can_link(graph: &Graph, pdu: usize, receive: bool) -> Result<(u32, u16), Vec<PlanDiagnostic>> {
    let prefix = if receive { "CanIfRxPdu" } else { "CanIfTxPdu" };
    let found: Vec<_> = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| {
            definition_is(graph, *index, &format!("{prefix}Cfg"))
                && value(graph, *index, &format!("{prefix}Ref"), true)
                    == Some(graph.elements[pdu].object.as_str())
        })
        .collect();
    if found.len() != 1 {
        return Err(reject(
            graph,
            pdu,
            "DIAGNOSTIC_REFERENCE",
            "The diagnostic N-PDU requires one matching CanIf route.",
        ));
    }
    let can_if = found[0];
    let id = value(graph, can_if, &format!("{prefix}CanId"), false)
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|id| *id <= 0x7ff)
        .ok_or_else(|| {
            reject(
                graph,
                can_if,
                "CAN_ID_CONFLICT",
                "The diagnostic CAN identifier is outside the standard 11-bit range.",
            )
        })?;
    let handle = value(graph, can_if, &format!("{prefix}Id"), false)
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| {
            reject(
                graph,
                can_if,
                "CAN_ID_CONFLICT",
                "The diagnostic CanIf handle is missing or outside its supported range.",
            )
        })?;
    if value(graph, can_if, &format!("{prefix}CanIdType"), false) != Some("STANDARD_CAN")
        || value(graph, pdu, "PduLength", false) != Some("8")
    {
        return Err(reject(
            graph,
            pdu,
            "LENGTH_CONFLICT",
            "The diagnostic link requires a standard Classical CAN eight-byte N-PDU.",
        ));
    }
    let triggers: Vec<_> = graph
        .of_kind("CAN-FRAME-TRIGGERING")
        .into_iter()
        .filter(|trigger| {
            graph
                .text(*trigger, "IDENTIFIER")
                .and_then(|value| value.parse::<u32>().ok())
                == Some(id)
        })
        .collect();
    if triggers.len() != 1 {
        return Err(reject(
            graph,
            can_if,
            "CAN_ID_CONFLICT",
            "The diagnostic identifier has no unique matching Extract trigger.",
        ));
    }
    let trigger = triggers[0];
    let frame = graph.target(trigger, "FRAME-REF").unwrap();
    let mappings = graph.descendants(frame, "PDU-TO-FRAME-MAPPING");
    if mappings.len() != 1 || graph.text(mappings[0], "START-POSITION") != Some("0") {
        return Err(reject(
            graph,
            frame,
            "LENGTH_CONFLICT",
            "The diagnostic frame requires one complete N-PDU mapping at byte zero.",
        ));
    }
    let system_pdu = graph.target(mappings[0], "PDU-REF").unwrap();
    if graph.elements[system_pdu].tag != "N-PDU" || graph.text(system_pdu, "LENGTH") != Some("8") {
        return Err(reject(
            graph,
            system_pdu,
            "LENGTH_CONFLICT",
            "The diagnostic frame maps a different N-PDU kind or length.",
        ));
    }
    check_trigger(graph, trigger, system_pdu, None, receive)?;
    let ports = graph.descendants(trigger, "FRAME-PORT-REF");
    if ports.len() != 1
        || graph.text(frame, "FRAME-LENGTH") != Some("8")
        || graph.text(trigger, "CAN-ADDRESSING-MODE") != Some("STANDARD")
    {
        return Err(reject(
            graph,
            trigger,
            "LENGTH_CONFLICT",
            "The diagnostic Extract trigger/frame differs from its eight-byte Classical CAN link.",
        ));
    }
    let port = *graph.objects.get(&graph.elements[ports[0]].text).unwrap();
    if graph.text(port, "COMMUNICATION-DIRECTION") != Some(if receive { "IN" } else { "OUT" }) {
        return Err(reject(
            graph,
            port,
            "DIRECTION_CONFLICT",
            "The diagnostic Extract frame port and CanIf direction disagree.",
        ));
    }
    if receive
        && (value(graph, can_if, "CanIfRxPduDataLength", false) != Some("8")
            || !matches!(
                value(graph, can_if, "CanIfRxPduDataLengthCheck", false),
                Some("true" | "1")
            ))
    {
        return Err(reject(
            graph,
            can_if,
            "LENGTH_CONFLICT",
            "Diagnostic Rx must enforce the declared eight-byte DLC.",
        ));
    }
    Ok((id, handle))
}

pub(super) fn inspect(
    graph: &Graph,
    component: &ComponentContract,
) -> Result<DiagnosticContract, Vec<PlanDiagnostic>> {
    let context = *graph.objects.get(&component.component).unwrap();
    let data = one(graph, context, "DcmDspData")?;
    let did = one(graph, data, "DcmDspDid")?;
    if value(graph, data, "DcmDspDataUsePort", false) != Some("USE_DATA_SYNCH_CLIENT_SERVER")
        || value(graph, data, "DcmDspDataType", false) != Some("UINT8_N")
        || value(graph, data, "DcmDspDataByteSize", false) != Some("4")
        || value(graph, data, "DcmDspDataEndianness", false) != Some("OPAQUE")
        || !matches!(
            value(graph, data, "DcmDspDataConditionCheckReadFncUsed", false),
            Some("false" | "0")
        )
        || graph.text(data, "SHORT-NAME") != Some(component.service.name.as_str())
    {
        return Err(reject(
            graph,
            data,
            "SERVICE_TYPE_CONFLICT",
            "Dcm data must bind the declared synchronous UINT8_N four-byte port without condition-check or byte swapping.",
        ));
    }
    let bindings: Vec<_> = graph
        .descendants(did, "ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, "DcmDspDidSignal"))
        .collect();
    if bindings.len() != 1
        || reference(graph, bindings[0], "DcmDspDidDataRef")? != data
        || value(graph, bindings[0], "DcmDspDidByteOffset", false) != Some("0")
    {
        return Err(reject(
            graph,
            did,
            "DIAGNOSTIC_REFERENCE",
            "The DID must contain the same four-byte data at byte offset zero.",
        ));
    }
    let did_id = value(graph, did, "DcmDspDidIdentifier", false)
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| {
            reject(
                graph,
                did,
                "DIAGNOSTIC_IDENTIFIER",
                "The DID identifier is absent or outside uint16.",
            )
        })?;
    let info = reference(graph, did, "DcmDspDidInfoRef")?;
    if !matches!(
        value(graph, info, "DcmDspDidDynamicallyDefined", false),
        Some("false" | "0")
    ) || graph
        .descendants(info, "ECUC-CONTAINER-VALUE")
        .into_iter()
        .any(|index| {
            definition_is(graph, index, "DcmDspDidWrite")
                || definition_is(graph, index, "DcmDspDidControl")
        })
    {
        return Err(reject(
            graph,
            info,
            "SERVICE_UNSUPPORTED",
            "Dynamic, write or control DID operations are outside the selected synchronous read profile.",
        ));
    }
    let read = one(graph, info, "DcmDspDidRead")?;
    let mut sessions = BTreeSet::new();
    let mut timing = None;
    for path in values(graph, read, "DcmDspDidReadSessionRef", true) {
        let session = *graph.objects.get(path).unwrap();
        let level = value(graph, session, "DcmDspSessionLevel", false)
            .and_then(|value| value.parse::<u8>().ok())
            .ok_or_else(|| {
                reject(
                    graph,
                    session,
                    "DIAGNOSTIC_SESSION",
                    "The declared session has no supported numeric level.",
                )
            })?;
        if !matches!(level, 1 | 3) || !sessions.insert(level) {
            return Err(reject(
                graph,
                session,
                "DIAGNOSTIC_SESSION",
                "Only distinct default and extended sessions are supported.",
            ));
        }
        let pair = (
            timer(graph, session, "DcmDspSessionP2ServerMax")?,
            timer(graph, session, "DcmDspSessionP2StarServerMax")?,
        );
        if timing.is_some_and(|expected| expected != pair) {
            return Err(reject(
                graph,
                session,
                "DIAGNOSTIC_TIMING",
                "The selected sessions require one shared explicit P2/P2* timing contract.",
            ));
        }
        timing = Some(pair);
    }
    if sessions != BTreeSet::from([1u8, 3u8]) {
        return Err(reject(
            graph,
            read,
            "DIAGNOSTIC_SESSION",
            "The application DID must be readable in the declared default and extended sessions.",
        ));
    }
    let protocol = one(graph, data, "DcmDslProtocolRow")?;
    let table = reference(graph, protocol, "DcmDslProtocolSIDTable")?;
    let services: Vec<_> = graph
        .descendants(table, "ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, "DcmDsdService"))
        .collect();
    let mut service_ids = BTreeSet::new();
    for service in services {
        let id = value(graph, service, "DcmDsdSidTabServiceId", false)
            .and_then(|value| value.parse::<u8>().ok())
            .ok_or_else(|| {
                reject(
                    graph,
                    service,
                    "SERVICE_UNSUPPORTED",
                    "The diagnostic service identifier is not a supported uint8 value.",
                )
            })?;
        let expected_subfunctions = match id {
            0x10 => BTreeSet::from([1u8, 3u8]),
            0x22 => BTreeSet::new(),
            0x3e => BTreeSet::from([0u8]),
            _ => {
                return Err(reject(
                    graph,
                    service,
                    "SERVICE_UNSUPPORTED",
                    "The selected read-only profile supports SessionControl, ReadDataByIdentifier and TesterPresent only.",
                ));
            }
        };
        let subfunctions: Vec<_> = graph
            .descendants(service, "ECUC-CONTAINER-VALUE")
            .into_iter()
            .filter(|index| definition_is(graph, *index, "DcmDsdSubService"))
            .collect();
        let mut actual_subfunctions = BTreeSet::new();
        for subfunction in subfunctions {
            let id = value(graph, subfunction, "DcmDsdSubServiceId", false)
                .and_then(|value| value.parse::<u8>().ok())
                .ok_or_else(|| {
                    reject(
                        graph,
                        subfunction,
                        "SERVICE_UNSUPPORTED",
                        "The declared subfunction is not a supported uint8 value.",
                    )
                })?;
            if !actual_subfunctions.insert(id)
                || !matches!(
                    value(graph, subfunction, "DcmDsdSubServiceUsed", false),
                    Some("true" | "1")
                )
            {
                return Err(reject(
                    graph,
                    subfunction,
                    "SERVICE_UNSUPPORTED",
                    "Subfunctions must be enabled and unique within their service.",
                ));
            }
        }
        if !service_ids.insert(id)
            || actual_subfunctions != expected_subfunctions
            || !matches!(
                value(graph, service, "DcmDsdServiceUsed", false),
                Some("true" | "1")
            )
            || match value(graph, service, "DcmDsdSidTabSubfuncAvail", false) {
                Some("true" | "1") => id == 0x22,
                Some("false" | "0") => id != 0x22,
                _ => true,
            }
        {
            return Err(reject(
                graph,
                service,
                "SERVICE_UNSUPPORTED",
                "Service availability, subfunction policy or numeric identity differs from the supported read-only profile.",
            ));
        }
    }
    if service_ids != BTreeSet::from([0x10u8, 0x22u8, 0x3eu8]) {
        return Err(reject(
            graph,
            table,
            "SERVICE_UNSUPPORTED",
            "The selected diagnostic service table is incomplete.",
        ));
    }
    let rx = one(graph, protocol, "DcmDslProtocolRx")?;
    let tx = one(graph, protocol, "DcmDslProtocolTx")?;
    if value(graph, protocol, "DcmDslProtocolType", false) != Some("DCM_UDS_ON_CAN")
        || !matches!(
            value(graph, protocol, "DcmDslProtocolRowUsed", false),
            Some("true" | "1")
        )
        || value(graph, rx, "DcmDslProtocolRxAddrType", false) != Some("DCM_PHYSICAL_TYPE")
    {
        return Err(reject(
            graph,
            protocol,
            "SERVICE_UNSUPPORTED",
            "The protocol must declare one enabled physical UDS-on-CAN connection.",
        ));
    }
    let rx_sdu = reference(graph, rx, "DcmDslProtocolRxPduRef")?;
    let tx_sdu = reference(graph, tx, "DcmDslProtocolTxPduRef")?;
    for sdu in [rx_sdu, tx_sdu] {
        if !definition_is(graph, sdu, "Pdu") || value(graph, sdu, "PduLength", false) != Some("64")
        {
            return Err(reject(
                graph,
                sdu,
                "LENGTH_CONFLICT",
                "The diagnostic N-SDU capacity must be explicitly 64 bytes.",
            ));
        }
    }
    for parameter in ["DcmDslProtocolRxBufferRef", "DcmDslProtocolTxBufferRef"] {
        let buffer = reference(graph, protocol, parameter)?;
        if value(graph, buffer, "DcmDslBufferSize", false) != Some("64") {
            return Err(reject(
                graph,
                buffer,
                "LENGTH_CONFLICT",
                "The selected diagnostic buffer must match its 64-byte N-SDU capacity.",
            ));
        }
    }
    let rx_transport = one(graph, data, "CanTpRxNSdu")?;
    let tx_transport = one(graph, data, "CanTpTxNSdu")?;
    let general = one(graph, data, "CanTpGeneral")?;
    if value(graph, general, "CanTpPaddingByte", false) != Some("0")
        || !matches!(
            value(graph, tx_transport, "CanTpTc", false),
            Some("false" | "0")
        )
        || !matches!(
            value(graph, protocol, "DcmSendRespPendOnRestart", false),
            Some("false" | "0")
        )
        || value(graph, protocol, "DcmTimStrP2ServerAdjust", false) != Some("0")
        || value(graph, protocol, "DcmTimStrP2StarServerAdjust", false) != Some("0")
    {
        return Err(reject(
            graph,
            protocol,
            "SERVICE_UNSUPPORTED",
            "Nonzero padding, timer adjustment, transport cancellation or restart PENDING behavior is outside the selected profile.",
        ));
    }
    if reference(graph, rx_transport, "CanTpRxNSduRef")? != rx_sdu
        || reference(graph, tx_transport, "CanTpTxNSduRef")? != tx_sdu
        || value(graph, rx_transport, "CanTpRxAddressingFormat", false) != Some("CANTP_STANDARD")
        || value(graph, tx_transport, "CanTpTxAddressingFormat", false) != Some("CANTP_STANDARD")
        || value(graph, rx_transport, "CanTpRxTaType", false) != Some("CANTP_PHYSICAL")
        || value(graph, tx_transport, "CanTpTxTaType", false) != Some("CANTP_PHYSICAL")
        || value(graph, rx_transport, "CanTpRxPaddingActivation", false) != Some("CANTP_ON")
        || value(graph, tx_transport, "CanTpTxPaddingActivation", false) != Some("CANTP_ON")
        || value(graph, rx_transport, "CanTpBs", false) != Some("0")
        || value(graph, rx_transport, "CanTpSTmin", false) != Some("0")
        || value(graph, rx_transport, "CanTpRxWftMax", false) != Some("0")
    {
        return Err(reject(
            graph,
            rx_transport,
            "SERVICE_UNSUPPORTED",
            "Transport requires matching physical normal-addressed padded SDUs with the selected zero Bs/STmin/Wft profile.",
        ));
    }
    let rx_npdu_cfg = one(graph, rx_transport, "CanTpRxNPdu")?;
    let tx_npdu_cfg = one(graph, tx_transport, "CanTpTxNPdu")?;
    let rx_npdu = reference(graph, rx_npdu_cfg, "CanTpRxNPduRef")?;
    let tx_npdu = reference(graph, tx_npdu_cfg, "CanTpTxNPduRef")?;
    let tx_flow = one(graph, rx_transport, "CanTpTxFcNPdu")?;
    let rx_flow = one(graph, tx_transport, "CanTpRxFcNPdu")?;
    if reference(graph, tx_flow, "CanTpTxFcNPduRef")? != tx_npdu
        || reference(graph, rx_flow, "CanTpRxFcNPduRef")? != rx_npdu
    {
        return Err(reject(
            graph,
            rx_transport,
            "DIAGNOSTIC_REFERENCE",
            "Flow-control frames must use the same paired physical NPDU routes.",
        ));
    }
    let (request_can_id, request_handle) = can_link(graph, rx_npdu, true)?;
    let (response_can_id, response_handle) = can_link(graph, tx_npdu, false)?;
    if request_can_id == response_can_id {
        return Err(reject(
            graph,
            protocol,
            "CAN_ID_CONFLICT",
            "Diagnostic request and response routes must be distinct.",
        ));
    }
    let (p2_ms, p2_star_ms) = timing.unwrap();
    if p2_ms > u16::MAX as u32
        || p2_star_ms > (u16::MAX as u32 * 10)
        || p2_star_ms % 10 != 0
        || p2_star_ms < p2_ms
    {
        return Err(reject(
            graph,
            protocol,
            "DIAGNOSTIC_TIMING",
            "P2/P2* must fit their UDS response fields and P2* ten-millisecond resolution, with P2* at least P2.",
        ));
    }
    Ok(DiagnosticContract {
        data: graph.elements[data].object.clone(),
        did_object: graph.elements[did].object.clone(),
        did: did_id,
        client_port: component.service.client_port.clone(),
        rx_sdu: graph.elements[rx_sdu].object.clone(),
        tx_sdu: graph.elements[tx_sdu].object.clone(),
        rx_npdu: graph.elements[rx_npdu].object.clone(),
        tx_npdu: graph.elements[tx_npdu].object.clone(),
        request_can_id,
        response_can_id,
        request_can_if_handle: request_handle,
        response_can_if_handle: response_handle,
        p2_ms,
        p2_star_ms,
        s3_ms: 5000,
        n_ar_ms: timer(graph, rx_transport, "CanTpNar")?,
        n_br_ms: timer(graph, rx_transport, "CanTpNbr")?,
        n_cr_ms: timer(graph, rx_transport, "CanTpNcr")?,
        n_as_ms: timer(graph, tx_transport, "CanTpNas")?,
        n_bs_ms: timer(graph, tx_transport, "CanTpNbs")?,
        n_cs_ms: timer(graph, tx_transport, "CanTpNcs")?,
        buffer_bytes: 64,
        sessions: sessions.into_iter().collect(),
    })
}
