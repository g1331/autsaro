//! Source-backed COM group, timebase and RTE notification configuration.
use super::component::milliseconds;
use super::graph::Graph;
use super::schedule::{ScheduleContract, definition_is, value, values};
use super::{DiagnosticCategory, PlanDiagnostic, SignalChannel};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComMainFunction {
    pub path: String,
    pub symbol: String,
    pub period_ms: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComRxGroup {
    pub path: String,
    pub handle: u16,
    pub members: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComReception {
    pub signal: String,
    pub user_signal: String,
    pub callback_handle: u16,
    pub first_timeout_ms: u32,
    pub timeout_ms: u32,
    pub receive_callback: String,
    pub timeout_callback: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComRuntimeContract {
    pub callback_header: String,
    pub receive_main: ComMainFunction,
    pub transmit_main: ComMainFunction,
    pub receive_group: ComRxGroup,
    pub receptions: Vec<ComReception>,
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
fn containers(graph: &Graph, kind: &str) -> Vec<usize> {
    graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, kind))
        .collect()
}
fn unique(
    graph: &Graph,
    context: usize,
    indices: Vec<usize>,
    code: &str,
) -> Result<usize, Vec<PlanDiagnostic>> {
    if indices.len() == 1 {
        Ok(indices[0])
    } else {
        Err(fail(graph, context, code))
    }
}
fn time(value: Option<&str>) -> Option<u32> {
    value.and_then(|text| {
        if super::multi::zero_seconds(text) {
            Some(0)
        } else {
            milliseconds(text)
        }
    })
}

pub(super) fn inspect(
    graph: &Graph,
    signals: &[SignalChannel],
    schedule: &ScheduleContract,
) -> Result<Option<ComRuntimeContract>, Vec<PlanDiagnostic>> {
    if signals.is_empty() {
        return Ok(None);
    }
    let context = *graph.objects.get(&signals[0].com_pdu).unwrap();
    let config = unique(
        graph,
        context,
        containers(graph, "ComConfig"),
        "COM_CONFIGURATION",
    )?;
    let general = unique(
        graph,
        context,
        containers(graph, "ComGeneral"),
        "COM_CONFIGURATION",
    )?;
    let group = unique(
        graph,
        config,
        containers(graph, "ComIPduGroup"),
        "COM_RX_GROUP",
    )?;
    let handle = value(graph, group, "ComIPduGroupHandleId", false)
        .and_then(|text| text.parse::<u16>().ok());
    if !graph.within(group, config)
        || handle != Some(0)
        || value(graph, general, "ComSupportedIPduGroups", false)
            .and_then(|text| text.parse::<u16>().ok())
            != Some(1)
        || !values(graph, group, "ComIPduGroupGroupRef", true).is_empty()
    {
        return Err(fail(graph, group, "COM_RX_GROUP"));
    }
    let main = |receive: bool| -> Result<(usize, ComMainFunction), Vec<PlanDiagnostic>> {
        let (kind, timebase, role, prefix) = if receive {
            (
                "ComMainFunctionRx",
                "ComMainRxTimeBase",
                "Com_AdvanceTime",
                "Com_MainFunctionRx",
            )
        } else {
            (
                "ComMainFunctionTx",
                "ComMainTxTimeBase",
                "Com_TriggerTransmit",
                "Com_MainFunctionTx",
            )
        };
        let index = unique(graph, config, containers(graph, kind), "COM_TIMEBASE")?;
        let period = value(graph, index, timebase, false)
            .and_then(milliseconds)
            .ok_or_else(|| fail(graph, index, "COM_TIMEBASE"))?;
        let name = graph.text(index, "SHORT-NAME").unwrap_or("");
        let symbol = format!("{prefix}_{name}");
        // The current BSW description names the logical role; the selected
        // configured instance symbol is retained for the runtime consumer.
        if !graph.within(index, config)
            || !super::catalog::c_identifier(&symbol)
            || super::contracts::reserved_identifier(name)
            || !schedule.entities.iter().any(|entity| {
                !entity.application && entity.symbol == role && entity.period_ms == period
            })
        {
            return Err(fail(graph, index, "COM_TIMEBASE"));
        }
        Ok((
            index,
            ComMainFunction {
                path: graph.elements[index].object.clone(),
                symbol,
                period_ms: period,
            },
        ))
    };
    let (rx_main, receive_main) = main(true)?;
    let (tx_main, transmit_main) = main(false)?;
    // This first profile emits one PERIODIC request per configured Tx main call.
    // A different PDU period needs counters and is not silently approximated.
    for signal in signals.iter().filter(|signal| !signal.receive) {
        if signal.transmit_period_ms != Some(transmit_main.period_ms) {
            return Err(fail(
                graph,
                *graph.objects.get(&signal.com_pdu).unwrap(),
                "COM_TIMEBASE",
            ));
        }
    }
    let rx_paths: BTreeSet<_> = signals
        .iter()
        .filter(|signal| signal.receive)
        .map(|signal| signal.com_pdu.as_str())
        .collect();
    if rx_paths.is_empty() {
        return Err(fail(graph, group, "COM_RX_GROUP"));
    }
    let selected_paths: BTreeSet<_> = signals
        .iter()
        .map(|signal| signal.com_pdu.as_str())
        .collect();
    for pdu in containers(graph, "ComIPdu") {
        let path = graph.elements[pdu].object.as_str();
        let receive = rx_paths.contains(path);
        let groups = values(graph, pdu, "ComIPduGroupRef", true);
        let required = if receive {
            vec![graph.elements[group].object.as_str()]
        } else {
            Vec::new()
        };
        let expected_main = if receive { rx_main } else { tx_main };
        if !selected_paths.contains(path)
            || groups != required
            || value(graph, pdu, "ComIPduMainFunctionRef", true)
                != Some(graph.elements[expected_main].object.as_str())
        {
            return Err(fail(graph, pdu, "COM_RX_GROUP"));
        }
    }
    let user = unique(
        graph,
        config,
        containers(graph, "ComUserModuleCnf"),
        "COM_CALLBACK",
    )?;
    if graph.text(user, "DEFINITION-REF")
        != Some("/AUTOSAR/EcucDefs/Rte/RteComUser/ComUserModuleCnf")
        || value(graph, user, "ComUserHeaderInclude", false) != Some("Rte_Com.h")
    {
        return Err(fail(graph, user, "COM_CALLBACK"));
    }
    let callbacks = containers(graph, "ComUserCallback");
    if callbacks.len() != 2 || callbacks.iter().any(|index| !graph.within(*index, user)) {
        return Err(fail(graph, user, "COM_CALLBACK"));
    }
    let callback = |kind: &str, symbol: &str| -> Result<usize, Vec<PlanDiagnostic>> {
        let index = unique(
            graph,
            user,
            callbacks
                .iter()
                .copied()
                .filter(|index| value(graph, *index, "ComUserCallbackType", false) == Some(kind))
                .collect(),
            "COM_CALLBACK",
        )?;
        if value(graph, index, "ComUserCallbackName", false) != Some(symbol) {
            return Err(fail(graph, index, "COM_CALLBACK"));
        }
        Ok(index)
    };
    let ack = callback("COM_RX_ACK", "Rte_COMCbk")?;
    let timeout = callback("COM_RX_TOUT", "Rte_COMCbkRxTOut")?;
    let mut seen = BTreeSet::new();
    let mut receptions = Vec::new();
    let mut accepted = BTreeSet::new();
    for signal in signals.iter().filter(|signal| signal.receive) {
        let index = *graph.objects.get(&signal.com_signal).unwrap();
        let user_signal = unique(
            graph,
            index,
            containers(graph, "ComUserSignal")
                .into_iter()
                .filter(|user_signal| {
                    value(
                        graph,
                        *user_signal,
                        "ComUserSystemTemplateSystemSignalRef",
                        true,
                    ) == Some(signal.signal_mapping.as_str())
                })
                .collect(),
            "COM_CALLBACK",
        )?;
        let handle = value(graph, user_signal, "ComUserCbkHandleId", false)
            .and_then(|text| text.parse::<u16>().ok())
            .ok_or_else(|| fail(graph, user_signal, "COM_CALLBACK"))?;
        let refs = values(graph, user_signal, "ComUserCallbackRef", true);
        let required = BTreeSet::from([
            graph.elements[ack].object.as_str(),
            graph.elements[timeout].object.as_str(),
        ]);
        if !graph.within(user_signal, user)
            || !seen.insert(handle)
            || refs.len() != 2
            || refs.into_iter().collect::<BTreeSet<_>>() != required
        {
            return Err(fail(graph, user_signal, "COM_CALLBACK"));
        }
        accepted.insert(user_signal);
        let first = time(value(graph, index, "ComFirstTimeout", false))
            .ok_or_else(|| fail(graph, index, "COM_TIMEOUT"))?;
        let regular = time(value(graph, index, "ComTimeout", false))
            .ok_or_else(|| fail(graph, index, "COM_TIMEOUT"))?;
        if first != 0
            || regular != signal.deadline_ms.unwrap_or(0)
            || regular % receive_main.period_ms != 0
        {
            return Err(fail(graph, index, "COM_TIMEOUT"));
        }
        receptions.push(ComReception {
            signal: signal.com_signal.clone(),
            user_signal: graph.elements[user_signal].object.clone(),
            callback_handle: handle,
            first_timeout_ms: first,
            timeout_ms: regular,
            receive_callback: "Rte_COMCbk".into(),
            timeout_callback: "Rte_COMCbkRxTOut".into(),
        });
    }
    if containers(graph, "ComUserSignal")
        .into_iter()
        .collect::<BTreeSet<_>>()
        != accepted
    {
        return Err(fail(graph, user, "COM_CALLBACK"));
    }
    receptions.sort_by(|left, right| left.signal.cmp(&right.signal));
    Ok(Some(ComRuntimeContract {
        callback_header: value(graph, user, "ComUserHeaderInclude", false)
            .unwrap()
            .into(),
        receive_main,
        transmit_main,
        receive_group: ComRxGroup {
            path: graph.elements[group].object.clone(),
            handle: 0,
            members: rx_paths.into_iter().map(str::to_owned).collect(),
        },
        receptions,
    }))
}
