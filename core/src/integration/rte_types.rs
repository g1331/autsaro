//! Shared standard RTE status declarations, independent of contract/generation phase.
use super::{PlanDiagnostic, c_layout};
use std::collections::BTreeMap;

pub(super) const STATUSES: &[(&str, &str)] = &[
    ("RTE_E_OK", "0U"),
    ("RTE_E_INVALID", "1U"),
    ("RTE_E_COM_STOPPED", "128U"),
    ("RTE_E_TIMEOUT", "129U"),
    ("RTE_E_LIMIT", "130U"),
    ("RTE_E_NO_DATA", "131U"),
    ("RTE_E_TRANSMIT_ACK", "132U"),
    ("RTE_E_NEVER_RECEIVED", "133U"),
    ("RTE_E_UNCONNECTED", "134U"),
    ("RTE_E_IN_EXCLUSIVE_AREA", "135U"),
    ("RTE_E_SEG_FAULT", "136U"),
    ("RTE_E_OUT_OF_RANGE", "137U"),
    ("RTE_E_SERIALIZATION_ERROR", "138U"),
    ("RTE_E_HARD_TRANSFORMER_ERROR", "138U"),
    ("RTE_E_SERIALIZATION_LIMIT", "139U"),
    ("RTE_E_TRANSFORMER_LIMIT", "139U"),
    ("RTE_E_SOFT_TRANSFORMER_ERROR", "140U"),
    ("RTE_E_COM_BUSY", "141U"),
    ("RTE_E_RAM_UNCHANGED", "142U"),
    ("RTE_E_LOST_DATA", "64U"),
    ("RTE_E_MAX_AGE_EXCEEDED", "64U"),
    ("SCHM_E_OK", "0U"),
    ("SCHM_E_LIMIT", "130U"),
    ("SCHM_E_NO_DATA", "131U"),
    ("SCHM_E_TRANSMIT_ACK", "132U"),
    ("SCHM_E_IN_EXCLUSIVE_AREA", "135U"),
    ("SCHM_E_TIMEOUT", "129U"),
    ("SCHM_E_LOST_DATA", "64U"),
];

pub(super) fn standard() -> Result<String, Vec<super::PlanDiagnostic>> {
    let path = "runtime/ecu/templates/Rte.h.in";
    super::outputs::render(
        super::outputs::trusted_text(path)?,
        path,
        &[
            super::outputs::Edit::Once(
                "@STATUSES@",
                &c_layout::retained_defines(
                    STATUSES,
                    "RTE status value.",
                    &[
                        "RTE_E_OK",
                        "RTE_E_INVALID",
                        "RTE_E_TIMEOUT",
                        "RTE_E_NO_DATA",
                        "RTE_E_TRANSMIT_ACK",
                        "RTE_E_UNCONNECTED",
                        "RTE_E_IN_EXCLUSIVE_AREA",
                        "RTE_E_SEG_FAULT",
                        "RTE_E_OUT_OF_RANGE",
                        "RTE_E_SERIALIZATION_ERROR",
                        "RTE_E_HARD_TRANSFORMER_ERROR",
                        "RTE_E_SERIALIZATION_LIMIT",
                        "RTE_E_TRANSFORMER_LIMIT",
                        "RTE_E_SOFT_TRANSFORMER_ERROR",
                        "RTE_E_COM_BUSY",
                        "RTE_E_RAM_UNCHANGED",
                        "RTE_E_LOST_DATA",
                        "SCHM_E_LIMIT",
                        "SCHM_E_NO_DATA",
                        "SCHM_E_TRANSMIT_ACK",
                        "SCHM_E_IN_EXCLUSIVE_AREA",
                        "SCHM_E_TIMEOUT",
                        "SCHM_E_LOST_DATA",
                    ],
                ),
            ),
            super::outputs::Edit::Once(
                "/** Test the infrastructure bit (SWS_Rte_07404); the argument is evaluated once.",
                "/** Test the infrastructure bit (SWS_Rte_07404); the argument is evaluated once.\n * MR12 RULE 2.5 VIOLATION: SWS_Rte_07404 publishes this macro even when unused.",
            ),
            super::outputs::Edit::Once(
                "/** Test the overlay bit (SWS_Rte_07405); the argument is evaluated once.",
                "/** Test the overlay bit (SWS_Rte_07405); the argument is evaluated once.\n * MR12 RULE 2.5 VIOLATION: SWS_Rte_07405 publishes this macro even when unused.",
            ),
            super::outputs::Edit::Once(
                "/** Read the application error (SWS_Rte_07406); the argument is evaluated once.",
                "/** Read the application error (SWS_Rte_07406); the argument is evaluated once.\n * MR12 RULE 2.5 VIOLATION: SWS_Rte_07406 publishes this macro even when unused.",
            ),
        ],
    )
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct RteDataType {
    pub path: String,
    pub symbol: String,
    pub native_type: String,
    pub array_length: Option<u32>,
}

/// All accepted RTE-owned input types, including types not used by a port.
pub(super) fn inspect(
    graph: &super::graph::Graph,
) -> Result<Vec<RteDataType>, Vec<super::PlanDiagnostic>> {
    let mut primitives = BTreeMap::new();
    let mut arrays = BTreeMap::new();
    let invalid = |index| {
        vec![graph.diagnostic(index, super::DiagnosticCategory::Unsupported, "RTE_TYPE_UNSUPPORTED",
        crate::product_message!("backend.integration.composition.contract_invalid", "code" => "RTE_TYPE_UNSUPPORTED"),
        crate::product_message!("backend.integration.composition.repair_contract"))]
    };
    let insert = |table: &mut BTreeMap<String, RteDataType>, index, native_type, array_length| {
        let symbol = graph.text(index, "SHORT-NAME").unwrap_or("").to_owned();
        let item = RteDataType {
            path: graph.elements[index].object.clone(),
            symbol: symbol.clone(),
            native_type,
            array_length,
        };
        if let Some(existing) = table.get(&symbol) {
            if existing.native_type != item.native_type
                || existing.array_length != item.array_length
            {
                return Err(invalid(index));
            }
        } else {
            table.insert(symbol, item);
        }
        Ok(())
    };
    let mut emitters = BTreeMap::new();
    for datatype in graph.of_kind("IMPLEMENTATION-DATA-TYPE") {
        let symbol = graph.text(datatype, "SHORT-NAME").unwrap_or("");
        let emitter = graph.text(datatype, "TYPE-EMITTER").unwrap_or("RTE");
        if emitters
            .insert(symbol, emitter)
            .is_some_and(|existing| existing != emitter)
        {
            return Err(invalid(datatype));
        }
        if emitter != "RTE" {
            // Import the trusted Dcm type producer; definitions stay in its header.
            let modes = super::composition::dcm_mode_definitions(graph, datatype)?;
            let bases = graph.descendants(datatype, "BASE-TYPE-REF");
            let valid_base = if let [base] = bases.as_slice() {
                let base = graph.index(&graph.elements[*base].text)?;
                graph.text(base, "NATIVE-DECLARATION") == Some("uint8")
                    && graph.text(base, "BASE-TYPE-SIZE") == Some("8")
                    && graph.text(base, "BASE-TYPE-ENCODING") == Some("NONE")
            } else {
                false
            };
            if emitter != "Rte_Dcm_Type.h"
                || graph.text(datatype, "CATEGORY") != Some("VALUE")
                || !valid_base
                || !(matches!(symbol, "Dcm_SecLevelType" | "Dcm_SesCtrlType")
                    || modes
                        .iter()
                        .any(|mode| symbol == format!("Rte_ModeType_{}", mode.group)))
            {
                return Err(invalid(datatype));
            }
            if matches!(symbol, "Dcm_SecLevelType" | "Dcm_SesCtrlType") {
                insert(&mut primitives, datatype, symbol.into(), None)?;
            }
            continue;
        }
        if symbol.starts_with("Rte_")
            || symbol.starts_with("SchM_")
            || matches!(symbol, "Dcm_SecLevelType" | "Dcm_SesCtrlType")
        {
            return Err(invalid(datatype));
        }
        if graph.text(datatype, "CATEGORY") == Some("VALUE") {
            let bases = graph.descendants(datatype, "BASE-TYPE-REF");
            let [base] = bases.as_slice() else {
                return Err(invalid(datatype));
            };
            let base = graph.index(&graph.elements[*base].text)?;
            let native = graph.text(base, "NATIVE-DECLARATION").unwrap_or("");
            let (bits, encoding) = match native {
                "uint8" => ("8", "NONE"),
                "uint16" => ("16", "NONE"),
                "uint32" => ("32", "NONE"),
                "uint64" => ("64", "NONE"),
                "sint8" => ("8", "2C"),
                "sint16" => ("16", "2C"),
                "sint32" => ("32", "2C"),
                "sint64" => ("64", "2C"),
                "boolean" => ("8", "BOOLEAN"),
                "float32" => ("32", "IEEE754"),
                "float64" => ("64", "IEEE754"),
                _ => return Err(invalid(datatype)),
            };
            if graph.text(base, "CATEGORY") != Some("FIXED_LENGTH")
                || graph.text(base, "BASE-TYPE-SIZE") != Some(bits)
                || graph.text(base, "BASE-TYPE-ENCODING") != Some(encoding)
            {
                return Err(invalid(datatype));
            }
            insert(&mut primitives, datatype, native.to_owned(), None)?;
        } else if graph.text(datatype, "CATEGORY") == Some("ARRAY") {
            let elements = graph.descendants(datatype, "IMPLEMENTATION-DATA-TYPE-ELEMENT");
            let [element] = elements.as_slice() else {
                return Err(invalid(datatype));
            };
            let references = graph.descendants(*element, "IMPLEMENTATION-DATA-TYPE-REF");
            let [reference] = references.as_slice() else {
                return Err(invalid(datatype));
            };
            let leaf = graph.index(&graph.elements[*reference].text)?;
            let length = graph
                .text(*element, "ARRAY-SIZE")
                .and_then(|size| size.parse::<u32>().ok())
                .filter(|size| *size == 4);
            if graph.text(*element, "CATEGORY") != Some("TYPE_REFERENCE")
                || graph.text(*element, "ARRAY-SIZE-SEMANTICS") != Some("FIXED-SIZE")
                || length.is_none()
                || graph.text(leaf, "CATEGORY") != Some("VALUE")
            {
                return Err(invalid(datatype));
            }
            insert(
                &mut arrays,
                datatype,
                graph.text(leaf, "SHORT-NAME").unwrap_or("").into(),
                length,
            )?;
        } else {
            return Err(invalid(datatype));
        }
    }
    for (symbol, array) in &arrays {
        if primitives.contains_key(symbol) || !primitives.contains_key(&array.native_type) {
            return Err(invalid(graph.index(&array.path)?));
        }
    }
    Ok(primitives
        .into_values()
        .chain(arrays.into_values())
        .collect())
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct PortDefinedArgument {
    pub native_type: String,
    pub type_identity: String,
    pub value: String,
}

/// P-port constants retain source order and checked type identity; clients never own them.
pub(super) fn port_arguments(
    graph: &super::graph::Graph,
    behavior: usize,
    component: usize,
    types: &[RteDataType],
) -> Result<BTreeMap<String, Vec<PortDefinedArgument>>, Vec<PlanDiagnostic>> {
    use super::component::{one, referenced};
    let mut ports = BTreeMap::new();
    let invalid = |index| {
        let code = "SERVICE_TYPE_CONFLICT";
        vec![graph.diagnostic(
            index,
            super::DiagnosticCategory::Input,
            code,
            crate::product_message!("backend.integration.composition.contract_invalid", "code" => code),
            crate::product_message!("backend.integration.composition.repair_contract"),
        )]
    };
    for option in graph.descendants(behavior, "PORT-API-OPTION") {
        let port = referenced(graph, option, "PORT-REF", "SERVICE_TYPE_CONFLICT")?;
        if !graph.within(port, component)
            || graph.elements[port].tag != "P-PORT-PROTOTYPE"
            || graph
                .target(port, "PROVIDED-INTERFACE-TREF")
                .is_none_or(|interface| graph.elements[interface].tag != "CLIENT-SERVER-INTERFACE")
            || ["ENABLE-TAKE-ADDRESS", "INDIRECT-API"].iter().any(|field| {
                graph
                    .text(option, field)
                    .is_some_and(|value| !matches!(value, "false" | "0"))
            })
        {
            return Err(invalid(option));
        }
        let mut values = Vec::new();
        for argument in graph.descendants(option, "PORT-DEFINED-ARGUMENT-VALUE") {
            let datatype = referenced(graph, argument, "VALUE-TYPE-TREF", "SERVICE_TYPE_CONFLICT")?;
            let checked = types
                .iter()
                .find(|item| {
                    Some(item.symbol.as_str()) == graph.text(datatype, "SHORT-NAME")
                        && item.array_length.is_none()
                })
                .ok_or_else(|| invalid(argument))?;
            let value = one(
                graph,
                argument,
                graph.descendants(argument, "NUMERICAL-VALUE-SPECIFICATION"),
                "SERVICE_TYPE_CONFLICT",
            )?;
            let raw = graph
                .text(value, "VALUE")
                .ok_or_else(|| invalid(argument))?;
            let literal =
                numeric_constant(&checked.native_type, raw).ok_or_else(|| invalid(argument))?;
            values.push(PortDefinedArgument {
                native_type: checked.symbol.clone(),
                type_identity: graph.elements[datatype].object.clone(),
                value: literal,
            });
        }
        if ports
            .insert(graph.elements[port].object.clone(), values)
            .is_some()
        {
            return Err(invalid(option));
        }
    }
    Ok(ports)
}

fn numeric_constant(native: &str, raw: &str) -> Option<String> {
    if native == "boolean" {
        return match raw {
            "0" | "false" => Some("FALSE".into()),
            "1" | "true" => Some("TRUE".into()),
            _ => None,
        };
    }
    if matches!(native, "float32" | "float64") {
        let value = raw.parse::<f64>().ok()?;
        if !value.is_finite() || (native == "float32" && !(value as f32).is_finite()) {
            return None;
        }
        return Some(if native == "float32" {
            format!("{value:e}F")
        } else {
            format!("{value:e}")
        });
    }
    let value = integer_constant(raw)?;
    let (signed, bits) = if let Some(bits) = native.strip_prefix("uint") {
        (false, bits.parse::<u32>().ok()?)
    } else {
        (true, native.strip_prefix("sint")?.parse::<u32>().ok()?)
    };
    let (min, max) = if signed {
        (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
    } else {
        (0, (1i128 << bits) - 1)
    };
    if !(min..=max).contains(&value) {
        return None;
    }
    let suffix = match (signed, bits) {
        (false, 64) => "ULL",
        (false, _) => "U",
        (true, 64) => "LL",
        _ => "",
    };
    // Spell the minimum as two representable signed constants, avoiding an unsigned literal.
    if signed && value == min {
        Some(format!("(-{}{suffix} - 1{suffix})", max))
    } else {
        Some(format!("{value}{suffix}"))
    }
}

/// The radix forms already admitted by the native XML integer grammar.
pub(super) fn integer_constant(raw: &str) -> Option<i128> {
    if let Some(digits) = raw.strip_prefix("0b").or_else(|| raw.strip_prefix("0B")) {
        i128::from_str_radix(digits, 2).ok()
    } else if raw.starts_with('0')
        && raw.len() > 1
        && !raw.starts_with("0x")
        && !raw.starts_with("0X")
    {
        i128::from_str_radix(raw, 8).ok()
    } else {
        crate::definitions::integer(raw).ok()
    }
}
