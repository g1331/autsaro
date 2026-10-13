use super::graph::Graph;
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataPort {
    pub path: String,
    pub name: String,
    pub element: String,
    pub element_name: String,
    pub application_type: String,
    pub implementation_type: String,
    pub c_type: String,
    pub read: bool,
    pub api_symbol: String,
    pub runnable: String,
    pub initial_value: u32,
    pub alive_timeout_ms: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServicePort {
    pub path: String,
    pub name: String,
    pub operation: String,
    pub operation_name: String,
    pub array_type: String,
    pub array_length: u32,
    pub runnable: String,
    pub runnable_symbol: String,
    pub client_instance: String,
    pub client_port: String,
    pub client_runnable: String,
    pub client_symbol: String,
}

/// Checked component contract, without claiming that the ECU integration
/// graph or runtime link has already passed.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentContract {
    pub component: String,
    pub instance: String,
    pub ecu: String,
    pub behavior: String,
    pub data_ports: Vec<DataPort>,
    pub service: ServicePort,
    pub timing_event: String,
    pub periodic_runnable: String,
    pub periodic_symbol: String,
    pub period_ms: u32,
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
            "backend.integration.component.sr_cs_profile_unique_relationship_restore"
        ),
    )]
}

pub(super) fn one(
    graph: &Graph,
    context: usize,
    values: Vec<usize>,
    code: &str,
) -> Result<usize, Vec<PlanDiagnostic>> {
    if values.len() == 1 {
        Ok(values[0])
    } else {
        Err(reject(
            graph,
            context,
            code,
            crate::product_message!(
                "backend.integration.component.selected_relationship_unique_object_required"
            ),
        ))
    }
}

pub(super) fn referenced(
    graph: &Graph,
    context: usize,
    tag: &str,
    code: &str,
) -> Result<usize, Vec<PlanDiagnostic>> {
    graph.target(context, tag).ok_or_else(|| {
        reject(
            graph,
            context,
            code,
            crate::product_message!(
                "backend.integration.component.required_typed_reference_missing"
            ),
        )
    })
}

pub(super) fn name(graph: &Graph, index: usize) -> String {
    graph.text(index, "SHORT-NAME").unwrap_or("").into()
}

pub(super) fn c_name(value: &str) -> String {
    let mut result: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect();
    if result.starts_with(|character: char| character.is_ascii_digit()) {
        result.insert(0, '_');
    }
    result
}

pub(super) fn milliseconds(value: &str) -> Option<u32> {
    let (mantissa, exponent) = match value.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i32>().ok()?),
        None => (value, 0),
    };
    let mantissa = mantissa.strip_prefix('+').unwrap_or(mantissa);
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    let digits = format!("{whole}{fraction}");
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let mut amount = digits.parse::<u128>().ok()?;
    let scale = exponent
        .checked_add(3)?
        .checked_sub(i32::try_from(fraction.len()).ok()?)?;
    if scale >= 0 {
        amount = amount.checked_mul(10u128.checked_pow(scale as u32)?)?;
    } else {
        let divisor = 10u128.checked_pow(scale.checked_neg()? as u32)?;
        if amount % divisor != 0 {
            return None;
        }
        amount /= divisor;
    }
    u32::try_from(amount).ok().filter(|amount| *amount != 0)
}

pub(super) fn implementation(
    graph: &Graph,
    context: usize,
    behavior: usize,
    application: usize,
) -> Result<usize, Vec<PlanDiagnostic>> {
    let sets: Vec<_> = graph
        .descendants(behavior, "DATA-TYPE-MAPPING-REF")
        .iter()
        .filter_map(|index| graph.objects.get(&graph.elements[*index].text).copied())
        .collect();
    let mapping_context = sets.first().copied().unwrap_or(context);
    let maps: Vec<_> = sets
        .into_iter()
        .flat_map(|set| graph.descendants(set, "DATA-TYPE-MAP"))
        .filter(|map| graph.target(*map, "APPLICATION-DATA-TYPE-REF") == Some(application))
        .collect();
    let mapping = one(graph, mapping_context, maps, "TYPE_MAP_MISSING")?;
    let implementation = referenced(
        graph,
        mapping,
        "IMPLEMENTATION-DATA-TYPE-REF",
        "TYPE_MAP_MISSING",
    )?;
    let base = one(
        graph,
        implementation,
        graph.descendants(implementation, "BASE-TYPE-REF"),
        "TYPE_CONFLICT",
    )?;
    let base = graph.index(&graph.elements[base].text)?;
    if graph.text(base, "BASE-TYPE-SIZE") != Some("32")
        || graph.text(base, "NATIVE-DECLARATION") != Some("uint32")
        || graph.text(implementation, "CATEGORY") != Some("VALUE")
    {
        return Err(reject(
            graph,
            context,
            "TYPE_CONFLICT",
            crate::product_message!(
                "backend.integration.component.sr_mapped_uint32_implementation_type_required"
            ),
        ));
    }
    Ok(implementation)
}

pub(super) fn selected_channel(graph: &Graph, ecu: usize) -> Result<usize, Vec<PlanDiagnostic>> {
    let channels = graph
        .of_kind("CAN-PHYSICAL-CHANNEL")
        .into_iter()
        .filter(|channel| {
            graph
                .descendants(*channel, "COMMUNICATION-CONNECTOR-REF")
                .into_iter()
                .any(|reference| {
                    graph
                        .objects
                        .get(&graph.elements[reference].text)
                        .is_some_and(|connector| graph.within(*connector, ecu))
                })
        })
        .collect();
    one(graph, ecu, channels, "TARGET_NOT_UNIQUE")
}
