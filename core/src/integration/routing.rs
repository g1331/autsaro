use super::communication::SignalChannel;
use super::diagnostic::DiagnosticContract;
use super::graph::Graph;
use super::schedule::{definition_is, value, values};
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PduRoute {
    pub path: String,
    pub pdu: String,
    pub source: String,
    pub destination: String,
    pub source_handle: u16,
    pub destination_handle: u16,
    pub receive: bool,
    pub transport: bool,
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

pub(super) fn inspect(
    graph: &Graph,
    signals: &[SignalChannel],
    diagnostic: &DiagnosticContract,
) -> Result<Vec<PduRoute>, Vec<PlanDiagnostic>> {
    let mut selected: Vec<_> = signals
        .iter()
        .map(|signal| (signal.global_pdu.as_str(), signal.receive, false))
        .collect();
    selected.extend([
        (diagnostic.rx_sdu.as_str(), true, true),
        (diagnostic.tx_sdu.as_str(), false, true),
    ]);
    let mut routes = Vec::new();
    let mut source_handles = BTreeSet::new();
    let mut destination_handles = BTreeSet::new();
    for (pdu_path, receive, transport) in selected {
        let context = *graph.objects.get(pdu_path).unwrap();
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
        routes.push(PduRoute {
            path: graph.elements[paths[0]].object.clone(),
            pdu: pdu_path.into(),
            source: graph.elements[source].object.clone(),
            destination: graph.elements[destination].object.clone(),
            source_handle,
            destination_handle,
            receive,
            transport,
        });
    }
    let input_routes: Vec<_> = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, "PduRRoutingPath"))
        .collect();
    for path in input_routes {
        if !routes
            .iter()
            .any(|route| route.path == graph.elements[path].object)
        {
            return Err(reject(
                graph,
                path,
                "PDU_ROUTE_UNSUPPORTED",
                crate::product_message!(
                    "backend.integration.routing.routing_table_has_unsupported_extra_path"
                ),
            ));
        }
    }
    routes.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(routes)
}
