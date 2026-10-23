//! External RTE generator switches and the source configuration revision.
use super::graph::Graph;
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct RteGenerationContract {
    pub strict_unconnected_r_port_check: bool,
    pub strict_initial_values_check: bool,
    pub strict_configuration_check: bool,
    pub code_vendor_id: u16,
    pub software_version: String,
    pub revision_suffix: Option<String>,
}

fn invalid(graph: &Graph, index: usize) -> Vec<PlanDiagnostic> {
    vec![graph.diagnostic(index, DiagnosticCategory::Input, "RTE_GENERATION_OPTION",
        crate::product_message!("backend.integration.composition.contract_invalid", "code" => "RTE_GENERATION_OPTION"),
        crate::product_message!("backend.integration.composition.repair_contract"))]
}

pub(super) fn inspect(graph: &Graph) -> Result<RteGenerationContract, Vec<PlanDiagnostic>> {
    let mut configuration = RteGenerationContract {
        strict_unconnected_r_port_check: false,
        strict_initial_values_check: true,
        strict_configuration_check: true,
        code_vendor_id: 0,
        software_version: "0.0.0".into(),
        revision_suffix: None,
    };
    let mut selected = BTreeSet::new();
    let mut group_seen = false;
    for (group, element) in graph.elements.iter().enumerate().filter(|(_, element)| {
        element.tag == "SDG"
            && element.attributes.get("GID").map(String::as_str) == Some("Autsaro.RteGeneration")
    }) {
        let module = element
            .parent
            .and_then(|parent| graph.elements[parent].parent)
            .and_then(|admin| graph.elements[admin].parent);
        if group_seen
            || module.is_none_or(|module| {
                graph.elements[module].tag != "ECUC-MODULE-CONFIGURATION-VALUES"
                    || graph.text(module, "DEFINITION-REF") != Some("/AUTOSAR/EcucDefs/Rte")
            })
        {
            return Err(invalid(graph, group));
        }
        group_seen = true;
        for index in &element.children {
            let option = &graph.elements[*index];
            let name = option
                .attributes
                .get("GID")
                .map(String::as_str)
                .unwrap_or("");
            if option.tag != "SD" || !selected.insert(name) {
                return Err(invalid(graph, *index));
            }
            let value = match option.text.as_str() {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => return Err(invalid(graph, *index)),
            };
            match name {
                "strictUnconnectedRPortCheck" => {
                    configuration.strict_unconnected_r_port_check = value
                }
                "strictInitialValuesCheck" if value => {
                    configuration.strict_initial_values_check = true
                }
                "strictConfigurationCheck" if value => {
                    configuration.strict_configuration_check = true
                }
                // Automatic completion of missing configuration is outside the explicit-input profile.
                _ => return Err(invalid(graph, *index)),
            }
        }
    }
    let modules = graph.of_kind("ECUC-MODULE-CONFIGURATION-VALUES");
    if let Some(module) = modules
        .into_iter()
        .find(|index| graph.text(*index, "DEFINITION-REF") == Some("/AUTOSAR/EcucDefs/Rte"))
    {
        // ECUC_Rte_09086 has no default; metadata validates the exact source
        // definition/cardinality before this shared composition inspection.
        let generation = graph
            .descendants(module, "ECUC-CONTAINER-VALUE")
            .into_iter()
            .find(|index| {
                graph.text(*index, "DEFINITION-REF") == Some("/AUTOSAR/EcucDefs/Rte/RteGeneration")
            })
            .ok_or_else(|| invalid(graph, module))?;
        configuration.code_vendor_id =
            super::schedule::value(graph, generation, "RteCodeVendorId", false)
                .and_then(|value| crate::definitions::unsigned_integer(value.trim()))
                .and_then(|value| u16::try_from(value).ok())
                .ok_or_else(|| invalid(graph, generation))?;
        let revisions: Vec<_> = graph
            .children(module, "ADMIN-DATA")
            .into_iter()
            .flat_map(|admin| graph.descendants(admin, "DOC-REVISION"))
            .collect();
        if let Some((version, suffix)) = revision(graph, &revisions) {
            configuration.software_version = version;
            configuration.revision_suffix = suffix;
        }
    }
    Ok(configuration)
}

fn revision(graph: &Graph, revisions: &[usize]) -> Option<(String, Option<String>)> {
    let selected = if let [only] = revisions {
        *only
    } else {
        let mut dated: Vec<_> = revisions
            .iter()
            .map(|index| Some((date(graph.text(*index, "DATE")?)?, *index)))
            .collect::<Option<_>>()?;
        dated.sort_by_key(|(date, _)| *date);
        let (newest, selected) = dated.pop()?;
        if dated.last().is_some_and(|(date, _)| *date == newest) {
            return None;
        }
        selected
    };
    let label = graph.text(selected, "REVISION-LABEL")?;
    let integer = |text: &str| {
        (!text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| text.parse::<u32>().ok())
            .flatten()
    };
    let mut parts = label.splitn(3, '.');
    let major = integer(parts.next()?)?;
    let minor = integer(parts.next()?)?;
    let last = parts.next()?;
    let end = last.find(['.', '_', ';']).unwrap_or(last.len());
    let patch = integer(&last[..end])?;
    let suffix = last.get(end + 1..).map(str::to_owned);
    Some((format!("{major}.{minor}.{patch}"), suffix))
}

// AUTOSAR DateTime has day precision or second precision with a mandatory time zone.
// Compare instants, so different offsets and input order cannot select an older revision.
fn date(text: &str) -> Option<i64> {
    fn number(text: &str, start: usize, end: usize) -> Option<i64> {
        let value = text.get(start..end)?;
        value
            .bytes()
            .all(|byte| byte.is_ascii_digit())
            .then(|| value.parse().ok())
            .flatten()
    }
    let year = number(text, 0, 4)?;
    let month = number(text, 5, 7)?;
    let day = number(text, 8, 10)?;
    if text.get(4..5)? != "-" || text.get(7..8)? != "-" || !(1..=12).contains(&month) {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=months[(month - 1) as usize]).contains(&day) {
        return None;
    }
    let days = year * 365 + (year + 3) / 4 - (year + 99) / 100
        + (year + 399) / 400
        + months[..(month - 1) as usize].iter().sum::<i64>()
        + day
        - 1;
    if text.len() == 10 {
        return Some(days * 86400);
    }
    if text.get(10..11)? != "T" || text.get(13..14)? != ":" || text.get(16..17)? != ":" {
        return None;
    }
    let hours = number(text, 11, 13)?;
    let minutes = number(text, 14, 16)?;
    let seconds = number(text, 17, 19)?;
    if hours > 23 || minutes > 59 || seconds > 59 {
        return None;
    }
    let offset = match text.get(19..)? {
        "Z" => 0,
        zone if zone.len() == 6
            && matches!(zone.get(..1)?, "+" | "-")
            && zone.get(3..4)? == ":" =>
        {
            let hours = number(zone, 1, 3)?;
            let minutes = number(zone, 4, 6)?;
            if hours > 23 || minutes > 59 {
                return None;
            }
            (hours * 3600 + minutes * 60) * if zone.starts_with('-') { -1 } else { 1 }
        }
        _ => return None,
    };
    Some(days * 86400 + hours * 3600 + minutes * 60 + seconds - offset)
}
