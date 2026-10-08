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

fn reject(
    graph: &Graph,
    index: usize,
    code: &str,
    message: crate::message::LocalizedText,
) -> Vec<PlanDiagnostic> {
    vec![graph.diagnostic(
        index,
        DiagnosticCategory::Input,
        code,
        message,
        crate::product_message!(
            "backend.integration.communication.communication_relationship_correct"
        ),
    )]
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
            crate::product_message!(
                "backend.integration.communication.communication_relationship_missing_or_ambiguous"
            ),
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
                crate::product_message!(
                    "backend.integration.communication.frame_trigger_physical_can_channel_missing"
                ),
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
            crate::product_message!(
                "backend.integration.communication.frame_trigger_pdu_or_channel_mismatch"
            ),
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
                crate::product_message!(
                    "backend.integration.communication.frame_pdu_port_direction_conflict"
                ),
            ));
        }
        let connector = graph
            .ancestor(port, "CAN-COMMUNICATION-CONNECTOR")
            .ok_or_else(|| {
                reject(
                    graph,
                    port,
                    "SIGNAL_MAPPING",
                    crate::product_message!(
                        "backend.integration.communication.communication_port_can_connector_missing"
                    ),
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
                crate::product_message!(
                    "backend.integration.communication.communication_port_channel_connector_mismatch"
                ),
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
                crate::product_message!(
                    "backend.integration.communication.pdu_trigger_signal_or_channel_mismatch"
                ),
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
                crate::product_message!(
                    "backend.integration.communication.signal_port_channel_direction_mismatch"
                ),
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
            crate::product_message!(
                "backend.integration.communication.diagnostic_npdu_sr_signal_trigger_forbidden"
            ),
        ));
    }
    Ok(())
}

pub(super) fn inspect(
    graph: &Graph,
    component: &ComponentContract,
) -> Result<Vec<SignalChannel>, Vec<PlanDiagnostic>> {
    inspect_ports(
        graph,
        &component.data_ports,
        &component.instance,
        component.period_ms,
    )
}

pub(super) fn inspect_ports(
    graph: &Graph,
    ports: &[super::component::DataPort],
    instance: &str,
    period_ms: u32,
) -> Result<Vec<SignalChannel>, Vec<PlanDiagnostic>> {
    let system = graph
        .of_kind("SYSTEM")
        .into_iter()
        .find(|index| graph.text(*index, "CATEGORY") == Some("ECU_EXTRACT"))
        .unwrap();
    let mut channels = Vec::new();
    for port in ports {
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
        if graph.text(iref, "CONTEXT-COMPONENT-REF") != Some(instance)
            || graph.text(iref, "TARGET-DATA-PROTOTYPE-REF") != Some(port.element.as_str())
            || graph.text(mapping, "COMMUNICATION-DIRECTION")
                != Some(if port.read { "IN" } else { "OUT" })
        {
            return Err(reject(
                graph,
                mapping,
                "DIRECTION_CONFLICT",
                crate::product_message!(
                    "backend.integration.communication.extract_data_mapping_mismatch"
                ),
            ));
        }
        let system_signal = graph.target(mapping, "SYSTEM-SIGNAL-REF").ok_or_else(|| {
            reject(
                graph,
                mapping,
                "SIGNAL_MAPPING",
                crate::product_message!("backend.integration.communication.application_data_mapping_system_signal_missing"),
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
                crate::product_message!("backend.integration.communication.communication_signal_transformer_present"),
                crate::product_message!("backend.integration.communication.untransformed_uint32_communication_profile_required"),
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
                crate::product_message!(
                    "backend.integration.communication.uint32_application_signal_bit_length_mismatch"
                ),
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
                    crate::product_message!(
                        "backend.integration.communication.signal_mapping_owning_ipdu_missing"
                    ),
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
                crate::product_message!(
                    "backend.integration.communication.uint32_four_byte_little_endian_pdu_mapping_required"
                ),
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
                crate::product_message!(
                    "backend.integration.communication.pdu_mapping_owning_can_frame_missing"
                ),
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
                crate::product_message!(
                    "backend.integration.communication.can_frame_length_or_pdu_position_mismatch"
                ),
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
                    crate::product_message!("backend.integration.communication.classical_can_standard_identifier_invalid"),
                )
            })?;
        if graph.text(trigger, "CAN-ADDRESSING-MODE") != Some("STANDARD") {
            return Err(reject(
                graph,
                trigger,
                "CAN_ID_CONFLICT",
                crate::product_message!(
                    "backend.integration.communication.classical_can_addressing_unsupported"
                ),
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
                crate::product_message!(
                    "backend.integration.communication.frame_port_swc_com_direction_mismatch"
                ),
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
                crate::product_message!(
                    "backend.integration.communication.com_signal_application_extract_mapping_mismatch"
                ),
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
                crate::product_message!(
                    "backend.integration.communication.com_ipdu_direction_or_signal_membership_mismatch"
                ),
            ));
        }
        let global_pdu = value(graph, com_pdu, "ComPduIdRef", true)
            .and_then(|path| graph.objects.get(path).copied())
            .ok_or_else(|| {
                reject(
                    graph,
                    com_pdu,
                    "SIGNAL_MAPPING",
                    crate::product_message!("backend.integration.communication.com_ipdu_canonical_ecuc_pdu_reference_not_unique"),
                )
            })?;
        if !definition_is(graph, global_pdu, "Pdu")
            || value(graph, global_pdu, "PduLength", false) != Some("4")
        {
            return Err(reject(
                graph,
                global_pdu,
                "LENGTH_CONFLICT",
                crate::product_message!(
                    "backend.integration.communication.canonical_ecuc_pdu_length_mismatch"
                ),
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
                crate::product_message!(
                    "backend.integration.communication.canif_identifier_or_addressing_type_mismatch"
                ),
            ));
        }
        let can_handle = value(graph, can_if, &format!("{can_prefix}Id"), false)
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| {
                reject(
                    graph,
                    can_if,
                    "CAN_ID_CONFLICT",
                    crate::product_message!(
                        "backend.integration.communication.canif_pdu_handle_missing_or_out_of_range"
                    ),
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
                || value(graph, com_signal, "ComTimeout", false).and_then(|value| {
                    if super::multi::selected(graph) && super::multi::zero_seconds(value) {
                        Some(0)
                    } else {
                        milliseconds(value)
                    }
                }) != port.alive_timeout_ms
            {
                return Err(reject(
                    graph,
                    com_signal,
                    "TIMEOUT_CONFLICT",
                    crate::product_message!(
                        "backend.integration.communication.rx_timing_or_action_contract_mismatch"
                    ),
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
                    != Some(period_ms)
            {
                return Err(reject(
                    graph,
                    mode,
                    "PERIOD_ALARM_CONFLICT",
                    crate::product_message!(
                        "backend.integration.communication.com_transmit_application_period_mismatch"
                    ),
                ));
            }
            Some(period_ms)
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
    if channels
        .iter()
        .map(|channel| channel.can_id)
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != channels.len()
    {
        return Err(reject(
            graph,
            system,
            "CAN_ID_CONFLICT",
            crate::product_message!(
                "backend.integration.communication.sr_channels_physical_can_identifier_conflict"
            ),
        ));
    }
    channels.sort_by(|left, right| left.port.cmp(&right.port));
    Ok(channels)
}
