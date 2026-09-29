use super::component::{ComponentContract, milliseconds};
use super::graph::Graph;
use super::schedule::{definition_is, value, values};
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalChannel {
    pub port: String,
    pub system_signal: String,
    pub signal: String,
    pub signal_mapping: String,
    pub system_pdu: String,
    pub global_pdu: String,
    pub com_signal: String,
    pub com_pdu: String,
    pub frame: String,
    pub trigger: String,
    pub can_if_pdu: String,
    pub can_id: u32,
    pub can_if_handle: u16,
    pub receive: bool,
    pub dlc: u8,
    pub start_bit: u16,
    pub bit_length: u16,
    pub deadline_ms: Option<u32>,
    pub transmit_period_ms: Option<u32>,
}

fn reject(graph: &Graph, index: usize, code: &str, message: &str) -> Vec<PlanDiagnostic> {
    vec![graph.diagnostic(index, DiagnosticCategory::Input, code, message,
        "Correct the explicit Extract/SWC/Com/CanIf/PDU relationship and preserve the selected single-network Classical CAN uint32 profile.")]
}

fn one(
    graph: &Graph,
    context: usize,
    found: Vec<usize>,
    code: &str,
) -> Result<usize, Vec<PlanDiagnostic>> {
    if found.len() == 1 {
        Ok(found[0])
    } else {
        Err(reject(
            graph,
            context,
            code,
            "The communication relationship is missing or not unique.",
        ))
    }
}

fn containers(graph: &Graph, kind: &str) -> Vec<usize> {
    graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, kind))
        .collect()
}

pub(super) fn check_trigger(
    graph: &Graph,
    trigger: usize,
    pdu: usize,
    signal: Option<usize>,
    receive: bool,
) -> Result<(), Vec<PlanDiagnostic>> {
    let direction = if receive { "IN" } else { "OUT" };
    let channel = graph
        .ancestor(trigger, "CAN-PHYSICAL-CHANNEL")
        .ok_or_else(|| {
            reject(
                graph,
                trigger,
                "SIGNAL_MAPPING",
                "The frame trigger has no physical CAN channel.",
            )
        })?;
    let reference = one(
        graph,
        trigger,
        graph.descendants(trigger, "PDU-TRIGGERING-REF"),
        "SIGNAL_MAPPING",
    )?;
    let pdu_trigger = *graph.objects.get(&graph.elements[reference].text).unwrap();
    if graph.target(pdu_trigger, "I-PDU-REF") != Some(pdu)
        || graph.ancestor(pdu_trigger, "CAN-PHYSICAL-CHANNEL") != Some(channel)
    {
        return Err(reject(
            graph,
            trigger,
            "SIGNAL_MAPPING",
            "The frame trigger refers to a different PDU or physical channel.",
        ));
    }
    for (owner, tag) in [(trigger, "FRAME-PORT-REF"), (pdu_trigger, "I-PDU-PORT-REF")] {
        let reference = one(
            graph,
            owner,
            graph.descendants(owner, tag),
            "DIRECTION_CONFLICT",
        )?;
        let port = *graph.objects.get(&graph.elements[reference].text).unwrap();
        if graph.text(port, "COMMUNICATION-DIRECTION") != Some(direction) {
            return Err(reject(
                graph,
                port,
                "DIRECTION_CONFLICT",
                "The selected frame/PDU port has a conflicting communication direction.",
            ));
        }
        let connector = graph
            .ancestor(port, "CAN-COMMUNICATION-CONNECTOR")
            .ok_or_else(|| {
                reject(
                    graph,
                    port,
                    "SIGNAL_MAPPING",
                    "The communication port has no CAN connector.",
                )
            })?;
        let channel_connectors = graph.descendants(channel, "COMMUNICATION-CONNECTOR-REF");
        if channel_connectors.len() != 1
            || graph.elements[channel_connectors[0]].text != graph.elements[connector].object
        {
            return Err(reject(
                graph,
                port,
                "SIGNAL_MAPPING",
                "The communication port and channel connector differ.",
            ));
        }
    }
    if let Some(signal) = signal {
        let reference = one(
            graph,
            pdu_trigger,
            graph.descendants(pdu_trigger, "I-SIGNAL-TRIGGERING-REF"),
            "SIGNAL_MAPPING",
        )?;
        let signal_trigger = *graph.objects.get(&graph.elements[reference].text).unwrap();
        if graph.target(signal_trigger, "I-SIGNAL-REF") != Some(signal)
            || graph.ancestor(signal_trigger, "CAN-PHYSICAL-CHANNEL") != Some(channel)
        {
            return Err(reject(
                graph,
                signal_trigger,
                "SIGNAL_MAPPING",
                "The PDU trigger refers to a different signal or channel.",
            ));
        }
        let reference = one(
            graph,
            signal_trigger,
            graph.descendants(signal_trigger, "I-SIGNAL-PORT-REF"),
            "DIRECTION_CONFLICT",
        )?;
        let port = *graph.objects.get(&graph.elements[reference].text).unwrap();
        if graph.text(port, "COMMUNICATION-DIRECTION") != Some(direction) {
            return Err(reject(
                graph,
                port,
                "DIRECTION_CONFLICT",
                "The signal port and selected channel direction disagree.",
            ));
        }
    } else if !graph
        .descendants(pdu_trigger, "I-SIGNAL-TRIGGERING-REF")
        .is_empty()
    {
        return Err(reject(
            graph,
            pdu_trigger,
            "SIGNAL_MAPPING",
            "A diagnostic N-PDU must not acquire an S/R signal trigger.",
        ));
    }
    Ok(())
}

pub(super) fn inspect(
    graph: &Graph,
    component: &ComponentContract,
) -> Result<Vec<SignalChannel>, Vec<PlanDiagnostic>> {
    let system = graph
        .of_kind("SYSTEM")
        .into_iter()
        .find(|index| graph.text(*index, "CATEGORY") == Some("ECU_EXTRACT"))
        .unwrap();
    let mut channels = Vec::new();
    for port in &component.data_ports {
        let context = *graph.objects.get(&port.path).unwrap();
        let mapping = one(
            graph,
            context,
            graph
                .descendants(system, "SENDER-RECEIVER-TO-SIGNAL-MAPPING")
                .into_iter()
                .filter(|mapping| {
                    graph
                        .descendants(*mapping, "CONTEXT-PORT-REF")
                        .into_iter()
                        .any(|reference| graph.elements[reference].text == port.path)
                })
                .collect(),
            "SIGNAL_MAPPING",
        )?;
        let iref = one(
            graph,
            mapping,
            graph.children(mapping, "DATA-ELEMENT-IREF"),
            "SIGNAL_MAPPING",
        )?;
        if graph.text(iref, "CONTEXT-COMPONENT-REF") != Some(component.instance.as_str())
            || graph.text(iref, "TARGET-DATA-PROTOTYPE-REF") != Some(port.element.as_str())
            || graph.text(mapping, "COMMUNICATION-DIRECTION")
                != Some(if port.read { "IN" } else { "OUT" })
        {
            return Err(reject(
                graph,
                mapping,
                "DIRECTION_CONFLICT",
                "The Extract data mapping differs from the selected component instance, element or port direction.",
            ));
        }
        let system_signal = graph.target(mapping, "SYSTEM-SIGNAL-REF").ok_or_else(|| {
            reject(
                graph,
                mapping,
                "SIGNAL_MAPPING",
                "The application data mapping lacks its system signal.",
            )
        })?;
        let signal = one(
            graph,
            system_signal,
            graph
                .of_kind("I-SIGNAL")
                .into_iter()
                .filter(|signal| graph.target(*signal, "SYSTEM-SIGNAL-REF") == Some(system_signal))
                .collect(),
            "SIGNAL_MAPPING",
        )?;
        if !graph.descendants(signal, "DATA-TRANSFORMATIONS").is_empty() {
            return Err(vec![graph.diagnostic(
                signal,
                DiagnosticCategory::Unsupported,
                "TRANSFORMER_UNSUPPORTED",
                "A transformer affects the selected communication signal.",
                "Use the explicit untransformed uint32 communication profile.",
            )]);
        }
        if graph
            .text(signal, "LENGTH")
            .and_then(|value| value.parse::<u16>().ok())
            != Some(32)
        {
            return Err(reject(
                graph,
                signal,
                "LENGTH_CONFLICT",
                "The uint32 application value and I-SIGNAL bit length disagree.",
            ));
        }
        let signal_mapping = one(
            graph,
            signal,
            graph
                .of_kind("I-SIGNAL-TO-I-PDU-MAPPING")
                .into_iter()
                .filter(|mapping| graph.target(*mapping, "I-SIGNAL-REF") == Some(signal))
                .collect(),
            "SIGNAL_MAPPING",
        )?;
        let pdu = graph
            .ancestor(signal_mapping, "I-SIGNAL-I-PDU")
            .ok_or_else(|| {
                reject(
                    graph,
                    signal_mapping,
                    "SIGNAL_MAPPING",
                    "The signal mapping has no owning I-PDU.",
                )
            })?;
        if graph.text(signal_mapping, "START-POSITION") != Some("0")
            || graph.text(signal_mapping, "PACKING-BYTE-ORDER")
                != Some("MOST-SIGNIFICANT-BYTE-LAST")
            || graph.text(pdu, "LENGTH") != Some("4")
        {
            return Err(reject(
                graph,
                signal_mapping,
                "LENGTH_CONFLICT",
                "The selected uint32 signal requires one four-byte little-endian PDU mapping at bit zero.",
            ));
        }
        let frame_mapping = one(
            graph,
            pdu,
            graph
                .of_kind("PDU-TO-FRAME-MAPPING")
                .into_iter()
                .filter(|mapping| graph.target(*mapping, "PDU-REF") == Some(pdu))
                .collect(),
            "SIGNAL_MAPPING",
        )?;
        let frame = graph.ancestor(frame_mapping, "CAN-FRAME").ok_or_else(|| {
            reject(
                graph,
                frame_mapping,
                "SIGNAL_MAPPING",
                "The PDU mapping has no owning CAN frame.",
            )
        })?;
        if graph.text(frame, "FRAME-LENGTH") != Some("4")
            || graph.text(frame_mapping, "START-POSITION") != Some("0")
            || graph.text(frame_mapping, "PACKING-BYTE-ORDER") != Some("MOST-SIGNIFICANT-BYTE-LAST")
        {
            return Err(reject(
                graph,
                frame,
                "LENGTH_CONFLICT",
                "The CAN frame length or PDU position differs from the supported four-byte profile.",
            ));
        }
        let trigger = one(
            graph,
            frame,
            graph
                .of_kind("CAN-FRAME-TRIGGERING")
                .into_iter()
                .filter(|trigger| graph.target(*trigger, "FRAME-REF") == Some(frame))
                .collect(),
            "SIGNAL_MAPPING",
        )?;
        check_trigger(graph, trigger, pdu, Some(signal), port.read)?;
        let can_id = graph
            .text(trigger, "IDENTIFIER")
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|id| *id <= 0x7ff)
            .ok_or_else(|| {
                reject(
                    graph,
                    trigger,
                    "CAN_ID_CONFLICT",
                    "The Classical CAN identifier must be a valid standard 11-bit value.",
                )
            })?;
        if graph.text(trigger, "CAN-ADDRESSING-MODE") != Some("STANDARD") {
            return Err(reject(
                graph,
                trigger,
                "CAN_ID_CONFLICT",
                "Only standard Classical CAN addressing is supported.",
            ));
        }
        let frame_port_ref = one(
            graph,
            trigger,
            graph.descendants(trigger, "FRAME-PORT-REF"),
            "DIRECTION_CONFLICT",
        )?;
        let frame_port = *graph
            .objects
            .get(&graph.elements[frame_port_ref].text)
            .unwrap();
        if graph.text(frame_port, "COMMUNICATION-DIRECTION")
            != Some(if port.read { "IN" } else { "OUT" })
        {
            return Err(reject(
                graph,
                frame_port,
                "DIRECTION_CONFLICT",
                "The frame port and SWC/Com direction disagree.",
            ));
        }
        let com_signal = one(
            graph,
            signal_mapping,
            containers(graph, "ComSignal")
                .into_iter()
                .filter(|signal| {
                    value(graph, *signal, "ComSystemTemplateSystemSignalRef", true)
                        == Some(graph.elements[signal_mapping].object.as_str())
                })
                .collect(),
            "SIGNAL_MAPPING",
        )?;
        if value(graph, com_signal, "ComBitPosition", false) != Some("0")
            || value(graph, com_signal, "ComBitSize", false) != Some("32")
            || value(graph, com_signal, "ComSignalType", false) != Some("UINT32")
            || value(graph, com_signal, "ComSignalEndianness", false) != Some("LITTLE_ENDIAN")
            || value(graph, com_signal, "ComSignalInitValue", false) != Some("0")
            || value(graph, com_signal, "ComTransferProperty", false) != Some("PENDING")
        {
            return Err(reject(
                graph,
                com_signal,
                "TYPE_CONFLICT",
                "The Com signal bit layout, type, initial value or transfer policy differs from its explicit application/Extract mapping.",
            ));
        }
        let com_pdu = one(
            graph,
            com_signal,
            containers(graph, "ComIPdu")
                .into_iter()
                .filter(|pdu| {
                    values(graph, *pdu, "ComIPduSignalRef", true)
                        .contains(&graph.elements[com_signal].object.as_str())
                })
                .collect(),
            "SIGNAL_MAPPING",
        )?;
        if value(graph, com_pdu, "ComIPduDirection", false)
            != Some(if port.read { "RECEIVE" } else { "SEND" })
            || values(graph, com_pdu, "ComIPduSignalRef", true).len() != 1
        {
            return Err(reject(
                graph,
                com_pdu,
                "DIRECTION_CONFLICT",
                "The Com I-PDU direction or signal membership differs from the selected channel.",
            ));
        }
        let global_pdu = value(graph, com_pdu, "ComPduIdRef", true)
            .and_then(|path| graph.objects.get(path).copied())
            .ok_or_else(|| {
                reject(
                    graph,
                    com_pdu,
                    "SIGNAL_MAPPING",
                    "The Com I-PDU has no unique canonical ECUC PDU reference.",
                )
            })?;
        if !definition_is(graph, global_pdu, "Pdu")
            || value(graph, global_pdu, "PduLength", false) != Some("4")
        {
            return Err(reject(
                graph,
                global_pdu,
                "LENGTH_CONFLICT",
                "The canonical ECUC PDU length differs from the mapped four-byte frame.",
            ));
        }
        let can_prefix = if port.read {
            "CanIfRxPdu"
        } else {
            "CanIfTxPdu"
        };
        let can_if = one(
            graph,
            global_pdu,
            containers(graph, &format!("{can_prefix}Cfg"))
                .into_iter()
                .filter(|pdu| {
                    value(graph, *pdu, &format!("{can_prefix}Ref"), true)
                        == Some(graph.elements[global_pdu].object.as_str())
                })
                .collect(),
            "SIGNAL_MAPPING",
        )?;
        if value(graph, can_if, &format!("{can_prefix}CanId"), false)
            .and_then(|value| value.parse::<u32>().ok())
            != Some(can_id)
            || value(graph, can_if, &format!("{can_prefix}CanIdType"), false)
                != Some("STANDARD_CAN")
        {
            return Err(reject(
                graph,
                can_if,
                "CAN_ID_CONFLICT",
                "The CanIf identifier or addressing type differs from the selected Extract frame.",
            ));
        }
        let can_handle = value(graph, can_if, &format!("{can_prefix}Id"), false)
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| {
                reject(
                    graph,
                    can_if,
                    "CAN_ID_CONFLICT",
                    "The CanIf PDU handle is absent or outside its supported range.",
                )
            })?;
        let deadline = if port.read {
            if value(graph, can_if, "CanIfRxPduDataLength", false) != Some("4")
                || !matches!(
                    value(graph, can_if, "CanIfRxPduDataLengthCheck", false),
                    Some("true" | "1")
                )
                || value(graph, com_signal, "ComFirstTimeout", false) != Some("0")
                || value(graph, com_signal, "ComRxDataTimeoutAction", false) != Some("NONE")
                || value(graph, com_signal, "ComTimeout", false).and_then(milliseconds)
                    != port.alive_timeout_ms
            {
                return Err(reject(
                    graph,
                    com_signal,
                    "TIMEOUT_CONFLICT",
                    "The Rx DLC/deadline/first-timeout/action differs from the declared receive contract.",
                ));
            }
            port.alive_timeout_ms
        } else {
            None
        };
        let transmit_period = if port.read {
            None
        } else {
            let mode = one(
                graph,
                com_pdu,
                graph
                    .descendants(com_pdu, "ECUC-CONTAINER-VALUE")
                    .into_iter()
                    .filter(|index| definition_is(graph, *index, "ComTxMode"))
                    .collect(),
                "PERIOD_ALARM_CONFLICT",
            )?;
            if value(graph, mode, "ComTxModeMode", false) != Some("PERIODIC")
                || value(graph, mode, "ComTxModeTimePeriod", false).and_then(milliseconds)
                    != Some(component.period_ms)
            {
                return Err(reject(
                    graph,
                    mode,
                    "PERIOD_ALARM_CONFLICT",
                    "The Com transmit period differs from the application period.",
                ));
            }
            Some(component.period_ms)
        };
        channels.push(SignalChannel {
            port: port.path.clone(),
            system_signal: graph.elements[system_signal].object.clone(),
            signal: graph.elements[signal].object.clone(),
            signal_mapping: graph.elements[signal_mapping].object.clone(),
            system_pdu: graph.elements[pdu].object.clone(),
            global_pdu: graph.elements[global_pdu].object.clone(),
            com_signal: graph.elements[com_signal].object.clone(),
            com_pdu: graph.elements[com_pdu].object.clone(),
            frame: graph.elements[frame].object.clone(),
            trigger: graph.elements[trigger].object.clone(),
            can_if_pdu: graph.elements[can_if].object.clone(),
            can_id,
            can_if_handle: can_handle,
            receive: port.read,
            dlc: 4,
            start_bit: 0,
            bit_length: 32,
            deadline_ms: deadline,
            transmit_period_ms: transmit_period,
        });
    }
    if channels[0].can_id == channels[1].can_id {
        return Err(reject(
            graph,
            system,
            "CAN_ID_CONFLICT",
            "Distinct S/R channels cannot share the same physical CAN identifier.",
        ));
    }
    channels.sort_by(|left, right| left.port.cmp(&right.port));
    Ok(channels)
}
