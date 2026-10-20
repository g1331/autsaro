//! Shared standard RTE status declarations, independent of contract/generation phase.
use super::c_layout;

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
        &[super::outputs::Edit::Once(
            "@STATUSES@",
            &c_layout::defines(STATUSES),
        )],
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
    use std::collections::BTreeMap;
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
            // This profile imports only its trusted Dcm mode type producer.
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
                || !modes
                    .iter()
                    .any(|mode| symbol == format!("Rte_ModeType_{}", mode.group))
            {
                return Err(invalid(datatype));
            }
            continue;
        }
        if symbol.starts_with("Rte_") || symbol.starts_with("SchM_") {
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
            if graph.text(base, "BASE-TYPE-SIZE") != Some(bits)
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
    // OS service types share Rte_Type.h with application types. Refuse a source
    // declaration that would change their fixed standard representation.
    for (symbol, native) in [
        ("CounterType", "uint32"),
        ("TimeInMicrosecondsType", "uint64"),
    ] {
        if let Some(item) = primitives.get(symbol).or_else(|| arrays.get(symbol))
            && (item.native_type != native || item.array_length.is_some())
        {
            return Err(invalid(graph.index(&item.path)?));
        }
    }
    Ok(primitives
        .into_values()
        .chain(arrays.into_values())
        .collect())
}
