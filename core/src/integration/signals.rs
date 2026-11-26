use super::component::{SignalInteger, milliseconds};
use super::graph::Graph;
use super::schedule::{definition_is, value, values};
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
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
    pub extended: bool,
    pub accepts_fd: bool,
    pub fd: bool,
    pub can_if_handle: u16,
    pub receive: bool,
    pub dlc: u8,
    pub start_bit: u16,
    pub bit_length: u16,
    pub big_endian: bool,
    pub native_type: String,
    pub initial_value: i64,
    pub pdu_length: u8,
    pub frame_mapping: String,
    pub frame_start_bit: u16,
    pub frame_big_endian: bool,
    pub frame_offset: u8,
    pub unused_byte: u8,
    pub deadline_ms: Option<u32>,
    pub transmit_period_ms: Option<u32>,
}

impl SignalChannel {
    pub(super) fn encoded_id(&self) -> u32 {
        super::can_id::encoded(self.can_id, self.extended)
            | if self.fd && !self.accepts_fd {
                super::can_id::FD
            } else {
                0
            }
    }
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
    let pdu_trigger = graph.index(&graph.elements[reference].text)?;
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
        let port = graph.index(&graph.elements[reference].text)?;
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
        let signal_trigger = one(
            graph,
            pdu_trigger,
            graph
                .descendants(pdu_trigger, "I-SIGNAL-TRIGGERING-REF")
                .into_iter()
                .filter_map(|reference| graph.objects.get(&graph.elements[reference].text).copied())
                .filter(|target| graph.target(*target, "I-SIGNAL-REF") == Some(signal))
                .collect(),
            "SIGNAL_MAPPING",
        )?;
        if graph.ancestor(signal_trigger, "CAN-PHYSICAL-CHANNEL") != Some(channel) {
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
        let port = graph.index(&graph.elements[reference].text)?;
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

pub(super) fn inspect_ports(
    graph: &Graph,
    ports: &[super::component::DataPort],
    instance: &str,
    period_ms: u32,
) -> Result<Vec<SignalChannel>, Vec<PlanDiagnostic>> {
    let Some(system) = graph
        .of_kind("SYSTEM")
        .into_iter()
        .find(|index| graph.text(*index, "CATEGORY") == Some("ECU_EXTRACT"))
    else {
        return Err(reject(
            graph,
            graph.context()?,
            "SIGNAL_MAPPING",
            crate::product_message!(
                "backend.integration.communication.communication_relationship_missing_or_ambiguous"
            ),
        ));
    };
    let mut channels = Vec::new();
    for port in ports {
        let context = graph.index(&port.path)?;
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
                crate::product_message!("backend.integration.communication.untransformed_integer_communication_profile_required"),
            )]);
        }
        let implementation = SignalInteger::read(graph, graph.index(&port.implementation_type)?)?;
        let network = SignalInteger::read(graph, signal)?;
        if implementation != network {
            return Err(field(
                graph,
                signal,
                "TYPE_CONFLICT",
                "NETWORK-REPRESENTATION-PROPS",
            ));
        }
        let bit_length = number(graph.text(signal, "LENGTH"))
            .filter(|width| (1..=network.bits).contains(width))
            .ok_or_else(|| field(graph, signal, "LENGTH_CONFLICT", "LENGTH"))?;
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
        let (start_bit, big_endian) = mapping_position(graph, signal_mapping)?;
        let pdu_length = payload_length(graph, pdu, "LENGTH")?;
        if occupied_bits(start_bit, bit_length, big_endian)
            .any(|bit| bit >= u32::from(pdu_length) * 8)
        {
            return Err(field(
                graph,
                signal_mapping,
                "LENGTH_CONFLICT",
                "START-POSITION",
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
        let dlc = payload_length(graph, frame, "FRAME-LENGTH")?;
        let (frame_start_bit, frame_big_endian) = mapping_position(graph, frame_mapping)?;
        let mut frame_bits = BTreeSet::new();
        let frame_mappings = graph.descendants(frame, "PDU-TO-FRAME-MAPPING");
        for mapping in &frame_mappings {
            let target = graph
                .target(*mapping, "PDU-REF")
                .ok_or_else(|| field(graph, *mapping, "SIGNAL_MAPPING", "PDU-REF"))?;
            let (start, big) = mapping_position(graph, *mapping)?;
            let length = payload_length(graph, target, "LENGTH")?;
            if start % 8 != if big { 7 } else { 0 } {
                return Err(field(graph, *mapping, "LENGTH_CONFLICT", "START-POSITION"));
            }
            for bit in occupied_bits(start, u16::from(length) * 8, big) {
                if bit >= u32::from(dlc) * 8 {
                    return Err(field(graph, *mapping, "LENGTH_CONFLICT", "START-POSITION"));
                }
                if !frame_bits.insert(bit) {
                    return Err(field(
                        graph,
                        *mapping,
                        "PDU_FRAME_OVERLAP",
                        "START-POSITION",
                    ));
                }
            }
        }
        // The selected CanIf profile has one I-PDU route per frame.
        if frame_mappings.len() != 1 {
            return Err(field(
                graph,
                frame_mapping,
                "PDU_FRAME_MULTIPLEX_UNSUPPORTED",
                "PDU-REF",
            ));
        }
        let frame_offset = (frame_start_bit / 8) as u8;
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
        let can_id = super::can_id::trigger_id(graph, trigger).ok_or_else(|| {
            reject(
                graph,
                trigger,
                "CAN_ID_CONFLICT",
                crate::product_message!(
                    "backend.integration.communication.classical_can_identifier_or_format_invalid"
                ),
            )
        })?;
        if !super::can_id::wire_length(can_id, dlc) {
            return Err(field(graph, frame, "LENGTH_CONFLICT", "FRAME-LENGTH"));
        }
        let frame_port_ref = one(
            graph,
            trigger,
            graph.descendants(trigger, "FRAME-PORT-REF"),
            "DIRECTION_CONFLICT",
        )?;
        let frame_port = graph.index(&graph.elements[frame_port_ref].text)?;
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
        for (name, matches) in [
            (
                "ComBitPosition",
                number(value(graph, com_signal, "ComBitPosition", false)) == Some(start_bit),
            ),
            (
                "ComBitSize",
                number(value(graph, com_signal, "ComBitSize", false)) == Some(bit_length),
            ),
            (
                "ComSignalType",
                value(graph, com_signal, "ComSignalType", false)
                    .is_some_and(|name| name.eq_ignore_ascii_case(network.native)),
            ),
            (
                "ComSignalEndianness",
                value(graph, com_signal, "ComSignalEndianness", false)
                    == Some(if big_endian {
                        "BIG_ENDIAN"
                    } else {
                        "LITTLE_ENDIAN"
                    }),
            ),
            (
                "ComTransferProperty",
                value(graph, com_signal, "ComTransferProperty", false) == Some("PENDING"),
            ),
        ] {
            if !matches {
                return Err(field(graph, com_signal, "TYPE_CONFLICT", name));
            }
        }
        let initial_value = value(graph, com_signal, "ComSignalInitValue", false)
            .and_then(|text| i64::try_from(super::rte_types::integer_constant(text)?).ok())
            .filter(|value| network.contains(*value, bit_length))
            .ok_or_else(|| {
                field(
                    graph,
                    com_signal,
                    "INIT_VALUE_REQUIRED",
                    "ComSignalInitValue",
                )
            })?;
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
            || number(value(graph, global_pdu, "PduLength", false)) != Some(u16::from(pdu_length))
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
        let Some((_, accepts_fd)) = super::can_id::pdu_id(graph, can_if, port.read)
            .filter(|(id, dual)| super::can_id::accepts(*id, *dual, can_id))
        else {
            return Err(reject(
                graph,
                can_if,
                "CAN_ID_CONFLICT",
                crate::product_message!(
                    "backend.integration.communication.canif_identifier_or_addressing_type_mismatch"
                ),
            ));
        };
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
        // The Rx data length check is enabled per PDU (SWS_CANIF_00026, 00902).
        if port.read
            && (number(value(graph, can_if, "CanIfRxPduDataLength", false))
                != Some(u16::from(pdu_length))
                || !matches!(
                    value(graph, can_if, "CanIfRxPduDataLengthCheck", false),
                    Some("true" | "1")
                ))
        {
            return Err(reject(
                graph,
                can_if,
                "LENGTH_CONFLICT",
                crate::product_message!("backend.integration.communication.signal_rx_dlc_required"),
            ));
        }
        let deadline = if port.read {
            if !value(graph, com_signal, "ComFirstTimeout", false)
                .is_some_and(super::composition::zero_seconds)
                || value(graph, com_signal, "ComRxDataTimeoutAction", false) != Some("NONE")
                || value(graph, com_signal, "ComTimeout", false).and_then(|value| {
                    if super::composition::zero_seconds(value) {
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
            // The selected Tx PDUs have no transmission deadline monitoring (SWS_Com_00333).
            if ["ComTimeout", "ComFirstTimeout"].into_iter().any(|name| {
                value(graph, com_signal, name, false)
                    .is_some_and(|text| !super::composition::zero_seconds(text))
            }) {
                return Err(reject(
                    graph,
                    com_signal,
                    "TIMEOUT_CONFLICT",
                    crate::product_message!(
                        "backend.integration.communication.tx_deadline_monitoring_unsupported"
                    ),
                ));
            }
            None
        };
        let (transmit_period, unused_byte) = if port.read {
            (None, 0)
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
            let tx = one(
                graph,
                com_pdu,
                graph
                    .descendants(com_pdu, "ECUC-CONTAINER-VALUE")
                    .into_iter()
                    .filter(|index| definition_is(graph, *index, "ComTxIPdu"))
                    .collect(),
                "SIGNAL_MAPPING",
            )?;
            let unused = value(graph, tx, "ComTxIPduUnusedAreasDefault", false)
                .and_then(|text| text.parse::<u8>().ok())
                .ok_or_else(|| field(graph, tx, "TYPE_CONFLICT", "ComTxIPduUnusedAreasDefault"))?;
            (Some(period_ms), unused)
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
            can_id: can_id & super::can_id::EXTENDED_MAX,
            extended: can_id & super::can_id::EXTENDED != 0,
            accepts_fd,
            fd: can_id & super::can_id::FD != 0,
            can_if_handle: can_handle,
            receive: port.read,
            dlc,
            start_bit,
            bit_length,
            big_endian,
            native_type: network.native.into(),
            initial_value,
            pdu_length,
            frame_mapping: graph.elements[frame_mapping].object.clone(),
            frame_start_bit,
            frame_big_endian,
            frame_offset,
            unused_byte,
            deadline_ms: deadline,
            transmit_period_ms: transmit_period,
        });
    }
    channels.sort_by(|left, right| left.port.cmp(&right.port));
    Ok(channels)
}

pub(super) fn field(graph: &Graph, index: usize, code: &str, name: &str) -> Vec<PlanDiagnostic> {
    reject(
        graph,
        index,
        code,
        crate::product_message!("backend.integration.communication.invalid_signal_layout_field", "field" => name),
    )
}

fn number(value: Option<&str>) -> Option<u16> {
    u16::try_from(crate::definitions::integer(value?).ok()?).ok()
}

fn payload_length(graph: &Graph, object: usize, tag: &str) -> Result<u8, Vec<PlanDiagnostic>> {
    graph
        .text(object, tag)
        .and_then(|value| value.parse::<u8>().ok())
        .filter(|length| (1..=64).contains(length))
        .ok_or_else(|| field(graph, object, "LENGTH_CONFLICT", tag))
}

fn mapping_position(graph: &Graph, object: usize) -> Result<(u16, bool), Vec<PlanDiagnostic>> {
    let start = number(graph.text(object, "START-POSITION"))
        .ok_or_else(|| field(graph, object, "LENGTH_CONFLICT", "START-POSITION"))?;
    let big_endian = match graph.text(object, "PACKING-BYTE-ORDER") {
        Some("MOST-SIGNIFICANT-BYTE-FIRST") => true,
        Some("MOST-SIGNIFICANT-BYTE-LAST") => false,
        _ => return Err(field(graph, object, "TYPE_CONFLICT", "PACKING-BYTE-ORDER")),
    };
    Ok((start, big_endian))
}

/// AUTOSAR System Template R24-11, TPS_SYST_01068/01069, figures 6.10–6.12.
fn occupied_bits(start: u16, length: u16, big_endian: bool) -> impl Iterator<Item = u32> {
    let first = u32::from(if big_endian { start ^ 7 } else { start });
    (first..first + u32::from(length)).map(move |bit| if big_endian { bit ^ 7 } else { bit })
}

pub(super) fn validate_layouts(
    graph: &Graph,
    channels: &[SignalChannel],
) -> Result<(), Vec<PlanDiagnostic>> {
    struct Layout<'a> {
        first: &'a SignalChannel,
        mapped: BTreeSet<&'a str>,
        members: BTreeSet<&'a str>,
    }
    let mut pdus = BTreeMap::new();
    let mut owners = BTreeMap::new();
    for signal in channels {
        let index = graph.index(&signal.signal_mapping)?;
        let Layout {
            first,
            mapped,
            members,
        } = pdus
            .entry(signal.com_pdu.as_str())
            .or_insert_with(|| Layout {
                first: signal,
                mapped: BTreeSet::new(),
                members: BTreeSet::new(),
            });
        if first.system_pdu != signal.system_pdu || first.receive != signal.receive {
            return Err(field(graph, index, "SIGNAL_MAPPING", "I-SIGNAL-REF"));
        }
        if let Some(previous) = owners.insert(&signal.system_pdu, &signal.com_pdu)
            && previous != &signal.com_pdu
        {
            return Err(field(graph, index, "SIGNAL_MAPPING", "I-SIGNAL-REF"));
        }
        mapped.insert(signal.signal_mapping.as_str());
        members.insert(signal.com_signal.as_str());
    }
    for (
        path,
        Layout {
            first,
            mapped,
            members,
            ..
        },
    ) in pdus
    {
        let com_pdu = graph.index(path)?;
        let actual = values(graph, com_pdu, "ComIPduSignalRef", true);
        if members.len() != actual.len()
            || !actual.into_iter().all(|member| members.contains(member))
        {
            return Err(field(graph, com_pdu, "SIGNAL_MAPPING", "ComIPduSignalRef"));
        }
        let mappings =
            graph.descendants(graph.index(&first.system_pdu)?, "I-SIGNAL-TO-I-PDU-MAPPING");
        let mut used = BTreeSet::new();
        for mapping in &mappings {
            let signal = graph
                .target(*mapping, "I-SIGNAL-REF")
                .ok_or_else(|| field(graph, *mapping, "SIGNAL_MAPPING", "I-SIGNAL-REF"))?;
            let length = number(graph.text(signal, "LENGTH"))
                .filter(|length| (1..=64).contains(length))
                .ok_or_else(|| field(graph, signal, "LENGTH_CONFLICT", "LENGTH"))?;
            let (start, big_endian) = mapping_position(graph, *mapping)?;
            for bit in occupied_bits(start, length, big_endian) {
                if bit >= u32::from(first.pdu_length) * 8 {
                    return Err(field(graph, *mapping, "LENGTH_CONFLICT", "START-POSITION"));
                }
                if !used.insert(bit) {
                    return Err(field(
                        graph,
                        *mapping,
                        "CAN_SIGNAL_OVERLAP",
                        "START-POSITION",
                    ));
                }
            }
        }
        for mapping in mappings {
            if !mapped.contains(graph.elements[mapping].object.as_str()) {
                return Err(field(graph, mapping, "SIGNAL_MAPPING", "I-SIGNAL-REF"));
            }
        }
    }
    Ok(())
}

/// Route producers and handle validation use each PDU once, even for a shared payload.
pub(super) fn pdus(channels: &[SignalChannel]) -> impl Iterator<Item = &SignalChannel> {
    let mut seen = BTreeSet::new();
    channels
        .iter()
        .filter(move |signal| seen.insert(signal.com_pdu.as_str()))
}
