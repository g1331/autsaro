use super::can_id;
use super::diagnostic::DiagnosticContract;
use super::graph::Graph;
use super::schedule::{definition_is, value, values};
use super::signals::SignalChannel;
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct PduRoute {
    pub path: String,
    pub pdu: String,
    pub source: String,
    pub destination: String,
    pub source_handle: u16,
    pub destination_handle: u16,
    pub receive: bool,
    pub transport: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<GatewayRoute>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct GatewayEndpoint {
    pub can_if_pdu: String,
    pub channel: String,
    pub can_id: u32,
    pub accepts_fd: bool,
    pub fd_trigger: bool,
    pub handle: u16,
    pub length: u8,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct GatewayRoute {
    pub receive: GatewayEndpoint,
    pub transmit: GatewayEndpoint,
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
        crate::product_message!("backend.integration.routing.supply_single_explicit_pdur_route"),
    )]
}

pub(super) fn inspect_optional(
    graph: &Graph,
    signals: &[SignalChannel],
    diagnostic: Option<&DiagnosticContract>,
) -> Result<Vec<PduRoute>, Vec<PlanDiagnostic>> {
    inspect_endpoints(
        graph,
        signals,
        diagnostic.map(|diagnostic| (diagnostic.rx_sdu.as_str(), diagnostic.tx_sdu.as_str())),
    )
}

pub(super) fn inspect_transport(
    graph: &Graph,
    signals: &[SignalChannel],
    transport: &super::diagnostic::DiagnosticTransportContract,
) -> Result<Vec<PduRoute>, Vec<PlanDiagnostic>> {
    inspect_endpoints(graph, signals, Some((&transport.rx_sdu, &transport.tx_sdu)))
}

fn inspect_endpoints(
    graph: &Graph,
    signals: &[SignalChannel],
    diagnostic: Option<(&str, &str)>,
) -> Result<Vec<PduRoute>, Vec<PlanDiagnostic>> {
    let mut selected: Vec<_> = super::signals::pdus(signals)
        .map(|signal| (signal.global_pdu.as_str(), signal.receive, false))
        .collect();
    if let Some((receive, transmit)) = diagnostic {
        selected.extend([(receive, true, true), (transmit, false, true)]);
    }
    let mut routes = Vec::new();
    let mut source_handles = BTreeSet::new();
    let mut destination_handles = BTreeSet::new();
    let mut direction_counts = [0u16; 2];
    for (pdu_path, receive, transport) in selected {
        let context = graph.index(pdu_path)?;
        let sources: Vec<_> = graph
            .of_kind("ECUC-CONTAINER-VALUE")
            .into_iter()
            .filter(|index| {
                definition_is(graph, *index, "PduRSrcPdu")
                    && value(graph, *index, "PduRSrcPduRef", true) == Some(pdu_path)
            })
            .collect();
        let destinations: Vec<_> = graph
            .of_kind("ECUC-CONTAINER-VALUE")
            .into_iter()
            .filter(|index| {
                definition_is(graph, *index, "PduRDestPdu")
                    && value(graph, *index, "PduRDestPduRef", true) == Some(pdu_path)
            })
            .collect();
        if sources.len() != 1 || destinations.len() != 1 {
            return Err(reject(
                graph,
                context,
                "PDU_ROUTE_NOT_UNIQUE",
                crate::product_message!(
                    "backend.integration.routing.canonical_pdu_endpoint_missing_or_duplicate"
                ),
            ));
        }
        let source = sources[0];
        let destination = destinations[0];
        let paths: Vec<_> = graph
            .of_kind("ECUC-CONTAINER-VALUE")
            .into_iter()
            .filter(|index| {
                definition_is(graph, *index, "PduRRoutingPath")
                    && value(graph, *index, "PduRSrcPduRRef", true)
                        == Some(graph.elements[source].object.as_str())
                    && values(graph, *index, "PduRDestPduRRef", true)
                        == [graph.elements[destination].object.as_str()]
            })
            .collect();
        if paths.len() != 1 {
            return Err(reject(
                graph,
                source,
                "PDU_ROUTE_NOT_UNIQUE",
                crate::product_message!(
                    "backend.integration.routing.canonical_pdu_requires_single_routing_path"
                ),
            ));
        }
        if !matches!(
            value(graph, source, "PduRSrcPduUpTxConf", false),
            Some("true" | "1")
        ) || !matches!(
            value(graph, destination, "PduRTransmissionConfirmation", false),
            Some("true" | "1")
        ) {
            return Err(reject(
                graph,
                paths[0],
                "PDU_ROUTE_CONFIRMATION",
                crate::product_message!(
                    "backend.integration.routing.routing_profile_requires_transmission_confirmation_path"
                ),
            ));
        }
        let source_handle = value(graph, source, "PduRSourcePduHandleId", false)
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| {
                reject(
                    graph,
                    source,
                    "PDU_ROUTE_HANDLE",
                    crate::product_message!(
                        "backend.integration.routing.source_handle_missing_or_out_of_range"
                    ),
                )
            })?;
        let destination_handle = value(graph, destination, "PduRDestPduHandleId", false)
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| {
                reject(
                    graph,
                    destination,
                    "PDU_ROUTE_HANDLE",
                    crate::product_message!(
                        "backend.integration.routing.destination_handle_missing_or_out_of_range"
                    ),
                )
            })?;
        if !source_handles.insert((receive, transport, source_handle))
            || !destination_handles.insert((receive, transport, destination_handle))
        {
            return Err(reject(
                graph,
                paths[0],
                "PDU_ROUTE_HANDLE",
                crate::product_message!("backend.integration.routing.endpoint_handle_conflict"),
            ));
        }
        let count = &mut direction_counts[usize::from(receive)];
        *count = count.checked_add(1).ok_or_else(|| {
            reject(
                graph,
                paths[0],
                "PDU_ROUTE_HANDLE",
                crate::product_message!("backend.integration.routing.endpoint_handle_conflict"),
            )
        })?;
        routes.push(PduRoute {
            path: graph.elements[paths[0]].object.clone(),
            pdu: pdu_path.into(),
            source: graph.elements[source].object.clone(),
            destination: graph.elements[destination].object.clone(),
            source_handle,
            destination_handle,
            receive,
            transport,
            gateway: None,
        });
    }
    let input_routes: Vec<_> = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, "PduRRoutingPath"))
        .collect();
    let mut gateway_pdus = BTreeSet::new();
    let owned_pdus: BTreeSet<_> = routes.iter().map(|route| route.pdu.clone()).collect();
    for path in input_routes {
        if !routes
            .iter()
            .any(|route| route.path == graph.elements[path].object)
        {
            let (route, destination_pdu) = gateway_route(graph, path, &owned_pdus)?;
            if !gateway_pdus.insert((true, route.pdu.clone()))
                || !gateway_pdus.insert((false, destination_pdu))
            {
                return Err(unsupported(graph, path));
            }
            for (object, unique) in [
                (
                    &route.source,
                    source_handles.insert((true, false, route.source_handle)),
                ),
                (
                    &route.destination,
                    destination_handles.insert((false, false, route.destination_handle)),
                ),
            ] {
                if !unique {
                    return Err(reject(
                        graph,
                        graph.index(object)?,
                        "PDU_ROUTE_HANDLE",
                        crate::product_message!(
                            "backend.integration.routing.endpoint_handle_conflict"
                        ),
                    ));
                }
            }
            routes.push(route);
        }
    }
    routes.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(routes)
}

fn unsupported(graph: &Graph, object: usize) -> Vec<PlanDiagnostic> {
    reject(
        graph,
        object,
        "PDU_ROUTE_UNSUPPORTED",
        crate::product_message!(
            "backend.integration.routing.routing_table_has_unsupported_extra_path"
        ),
    )
}

fn gateway_route(
    graph: &Graph,
    path: usize,
    owned_pdus: &BTreeSet<String>,
) -> Result<(PduRoute, String), Vec<PlanDiagnostic>> {
    let destinations = values(graph, path, "PduRDestPduRRef", true);
    let [destination] = destinations.as_slice() else {
        return Err(unsupported(graph, path));
    };
    let destination = graph.index(destination)?;
    let source = value(graph, path, "PduRSrcPduRRef", true)
        .ok_or_else(|| unsupported(graph, path))
        .and_then(|source| graph.index(source))?;
    let rx = gateway_endpoint(graph, source, true, owned_pdus)?;
    let tx = gateway_endpoint(graph, destination, false, owned_pdus)?;
    if rx.0 == tx.0 || rx.2.channel == tx.2.channel || rx.2.length != tx.2.length {
        return Err(unsupported(graph, destination));
    }
    Ok((
        PduRoute {
            path: graph.elements[path].object.clone(),
            pdu: rx.0,
            source: graph.elements[source].object.clone(),
            destination: graph.elements[destination].object.clone(),
            source_handle: rx.1,
            destination_handle: tx.1,
            receive: true,
            transport: false,
            gateway: Some(GatewayRoute {
                receive: rx.2,
                transmit: tx.2,
            }),
        },
        tx.0,
    ))
}

fn gateway_endpoint(
    graph: &Graph,
    endpoint: usize,
    receive: bool,
    owned_pdus: &BTreeSet<String>,
) -> Result<(String, u16, GatewayEndpoint), Vec<PlanDiagnostic>> {
    let (kind, reference, handle, confirmation, can_if_kind, can_if_ref, can_if_handle) = if receive
    {
        (
            "PduRSrcPdu",
            "PduRSrcPduRef",
            "PduRSourcePduHandleId",
            "PduRSrcPduUpTxConf",
            "CanIfRxPduCfg",
            "CanIfRxPduRef",
            "CanIfRxPduId",
        )
    } else {
        (
            "PduRDestPdu",
            "PduRDestPduRef",
            "PduRDestPduHandleId",
            "PduRTransmissionConfirmation",
            "CanIfTxPduCfg",
            "CanIfTxPduRef",
            "CanIfTxPduId",
        )
    };
    let invalid = || unsupported(graph, endpoint);
    let expected_confirmation = if receive {
        ["false", "0"]
    } else {
        ["true", "1"]
    };
    if !definition_is(graph, endpoint, kind)
        || !value(graph, endpoint, confirmation, false)
            .is_some_and(|flag| expected_confirmation.contains(&flag))
        || (!receive
            && value(graph, endpoint, "PduRDestPduDataProvision", false) != Some("PDUR_DIRECT"))
    {
        return Err(invalid());
    }
    let pdu = value(graph, endpoint, reference, true)
        .filter(|pdu| !owned_pdus.contains(*pdu))
        .ok_or_else(invalid)?;
    let pdu_index = graph.index(pdu)?;
    let route_handle = value(graph, endpoint, handle, false)
        .and_then(|handle| handle.parse::<u16>().ok())
        .ok_or_else(invalid)?;

    let can_if_pdus = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| {
            definition_is(graph, *index, can_if_kind)
                && value(graph, *index, can_if_ref, true) == Some(pdu)
        })
        .collect::<Vec<_>>();
    let [can_if] = can_if_pdus.as_slice() else {
        return Err(invalid());
    };
    gateway_can_if(graph, pdu_index, *can_if, receive, can_if_handle)
        .map(|bus| (pdu.into(), route_handle, bus))
}

fn gateway_can_if(
    graph: &Graph,
    pdu: usize,
    can_if: usize,
    receive: bool,
    handle: &str,
) -> Result<GatewayEndpoint, Vec<PlanDiagnostic>> {
    let invalid = || unsupported(graph, can_if);
    let (id, dual) = can_id::pdu_id(graph, can_if, receive).ok_or_else(invalid)?;
    let target =
        can_id::pdu_trigger_reference(graph, pdu)?.ok_or_else(|| unsupported(graph, pdu))?;
    let triggers = graph
        .of_kind("CAN-FRAME-TRIGGERING")
        .into_iter()
        .filter(|trigger| can_id::trigger_references(graph, *trigger, target))
        .collect::<Vec<_>>();
    let [trigger] = triggers.as_slice() else {
        return Err(unsupported(graph, pdu));
    };
    let frame = graph.target(*trigger, "FRAME-REF").ok_or_else(invalid)?;
    let mappings = graph.descendants(frame, "PDU-TO-FRAME-MAPPING");
    let [mapping] = mappings.as_slice() else {
        return Err(unsupported(graph, frame));
    };
    let (start, big_endian) = super::signals::mapping_position(graph, *mapping)?;
    if start != if big_endian { 7 } else { 0 } {
        return Err(super::signals::field(
            graph,
            *mapping,
            "LENGTH_CONFLICT",
            "START-POSITION",
        ));
    }
    let mapped = graph.target(*mapping, "PDU-REF").ok_or_else(invalid)?;
    let channel = super::signals::check_trigger(graph, *trigger, mapped, None, receive)?;
    let trigger_id = can_id::trigger_id(graph, *trigger)
        .filter(|trigger_id| can_id::accepts(id, dual, *trigger_id))
        .ok_or_else(invalid)?;
    let length = value(graph, pdu, "PduLength", false)
        .and_then(|length| length.parse::<u8>().ok())
        .filter(|length| *length != 0 && can_id::wire_length(trigger_id, *length))
        .ok_or_else(invalid)?;
    if graph
        .text(mapped, "LENGTH")
        .and_then(|length| length.parse::<u8>().ok())
        != Some(length)
        || graph
            .text(frame, "FRAME-LENGTH")
            .and_then(|length| length.parse::<u8>().ok())
            != Some(length)
        || (receive
            && (value(graph, can_if, "CanIfRxPduDataLength", false)
                .and_then(|length| length.parse::<u8>().ok())
                != Some(length)
                || !matches!(
                    value(graph, can_if, "CanIfRxPduDataLengthCheck", false),
                    Some("true" | "1")
                )))
    {
        return Err(invalid());
    }
    Ok(GatewayEndpoint {
        can_if_pdu: graph.elements[can_if].object.clone(),
        channel: graph.elements[channel].object.clone(),
        can_id: id,
        accepts_fd: dual,
        fd_trigger: trigger_id & can_id::FD != 0,
        handle: value(graph, can_if, handle, false)
            .and_then(|handle| handle.parse::<u16>().ok())
            .ok_or_else(invalid)?,
        length,
    })
}
