use super::graph::Graph;
use super::{DiagnosticCategory, PROFILE, PlanDiagnostic};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

const DESCRIPTION: &str = include_str!("../../../runtime/contracts/bsw-v1.json");

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContractArgument {
    pub name: String,
    pub native_type: String,
    pub direction: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Entry {
    module: String,
    return_type: String,
    arguments: Vec<ContractArgument>,
    header: String,
    source: String,
    stage: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Description {
    format_version: u32,
    profile: String,
    scope: String,
    entries: BTreeMap<String, Entry>,
    sources: BTreeMap<String, String>,
}

/// Source-backed inventory compiled into this workbench version. An edited
/// on-disk manifest cannot replace its trusted contracts at runtime.
pub struct RuntimeCatalog {
    description: Description,
}

fn tool(code: &str, message: crate::message::LocalizedText) -> Vec<PlanDiagnostic> {
    vec![PlanDiagnostic {
        category: DiagnosticCategory::Tool,
        code: code.into(),
        file: None,
        object: None,
        message: message.into(),
        remedy: crate::product_message!(
            "backend.integration.catalog.runtime_source_inventory_restore"
        )
        .into(),
    }]
}

impl RuntimeCatalog {
    pub fn embedded() -> Result<Self, Vec<PlanDiagnostic>> {
        let description: Description = serde_json::from_str(DESCRIPTION).map_err(|error| {
            tool(
                "CATALOG_TOOL",
                crate::product_message!("backend.integration.catalog.compiled_bsw_inventory_invalid", "error" => error),
            )
        })?;
        if description.format_version != 1
            || description.profile != PROFILE
            || description.scope.is_empty()
            || description.entries.iter().any(|(symbol, entry)| {
                entry.stage != "current_host_bsw"
                    || !description.sources.contains_key(&entry.header)
                    || !description.sources.contains_key(&entry.source)
                    || entry.module.is_empty()
                    || !c_identifier(symbol)
            })
        {
            return Err(tool(
                "CATALOG_TOOL",
                crate::product_message!(
                    "backend.integration.catalog.compiled_bsw_inventory_identity_unsupported"
                ),
            ));
        }
        Ok(Self { description })
    }

    pub fn from_repository(root: &Path) -> Result<Self, Vec<PlanDiagnostic>> {
        let Self { description } = Self::embedded()?;
        let manifest =
            std::fs::read(root.join("runtime/contracts/bsw-v1.json")).map_err(|error| {
                tool(
                    "CATALOG_MISSING",
                    crate::product_message!("backend.integration.catalog.bsw_inventory_missing", "error" => error),
                )
            })?;
        if manifest != DESCRIPTION.as_bytes() {
            return Err(tool(
                "CATALOG_IDENTITY",
                crate::product_message!("backend.integration.catalog.disk_bsw_inventory_mismatch"),
            ));
        }
        for (relative, expected) in &description.sources {
            let path = Path::new(relative);
            if !relative.starts_with("runtime/")
                || relative.contains('\\')
                || relative.contains(':')
                || !path
                    .components()
                    .all(|part| matches!(part, Component::Normal(_)))
            {
                return Err(tool(
                    "CATALOG_TOOL",
                    crate::product_message!(
                        "backend.integration.catalog.compiled_inventory_source_identity_invalid"
                    ),
                ));
            }
            let bytes = std::fs::read(root.join(path)).map_err(|error| {
                tool(
                    "CATALOG_SOURCE",
                    crate::product_message!("backend.integration.catalog.bsw_source_read_failed", "relative" => relative, "error" => error),
                )
            })?;
            if format!("{:x}", Sha256::digest(bytes)) != *expected {
                return Err(tool(
                    "CATALOG_SOURCE",
                    crate::product_message!("backend.integration.catalog.bsw_source_identity_mismatch", "relative" => relative),
                ));
            }
        }
        for (symbol, entry) in &description.entries {
            if entry.stage != "current_host_bsw"
                || !description.sources.contains_key(&entry.header)
                || !description.sources.contains_key(&entry.source)
                || entry.module.is_empty()
                || !c_identifier(symbol)
            {
                return Err(tool(
                    "CATALOG_TOOL",
                    crate::product_message!("backend.integration.catalog.bsw_producer_contract_invalid", "symbol" => symbol),
                ));
            }
        }
        Ok(Self { description })
    }

    pub fn source_identities(&self) -> &BTreeMap<String, String> {
        &self.description.sources
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolContract {
    pub symbol: String,
    pub return_type: String,
    pub arguments: Vec<ContractArgument>,
    pub declaration_owner: String,
    pub definition_owner: String,
    pub consumers: Vec<String>,
}

pub(super) fn c_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    matches!(bytes.next(), Some(b'a'..=b'z' | b'A'..=b'Z' | b'_'))
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn native_type(graph: &Graph, context: usize) -> Option<String> {
    let references = graph.descendants(context, "BASE-TYPE-REF");
    if references.len() != 1 {
        return None;
    }
    let base = graph.objects.get(&graph.elements[references[0]].text)?;
    graph
        .text(*base, "NATIVE-DECLARATION")
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
}

pub(super) fn inspect(
    graph: &Graph,
    catalog: &RuntimeCatalog,
) -> Result<Vec<SymbolContract>, Vec<PlanDiagnostic>> {
    let mut symbols = Vec::new();
    let mut used_owners = BTreeSet::new();
    let context = *graph.objects.values().next().unwrap();
    let all_entries = graph.of_kind("BSW-MODULE-ENTRY");
    for (symbol, current) in &catalog.description.entries {
        let found: Vec<_> = all_entries
            .iter()
            .copied()
            .filter(|index| graph.text(*index, "SHORT-NAME") == Some(symbol))
            .collect();
        if found.len() != 1 {
            return Err(vec![graph.diagnostic(found.first().copied().unwrap_or(context),
                DiagnosticCategory::Input, if found.is_empty() { "BSW_ENTRY_MISSING" } else { "SYMBOL_PRODUCER_DUPLICATE" },
                crate::product_message!("backend.integration.catalog.bsw_entry_description_and_producer_required", "symbol" => symbol),
                crate::product_message!("backend.integration.catalog.bsw_entry_description_supply"))]);
        }
        let entry = found[0];
        let returns = graph.children(entry, "RETURN-TYPE");
        let return_type = if returns.is_empty() {
            Some("void".into())
        } else if returns.len() == 1 {
            native_type(graph, returns[0])
        } else {
            None
        };
        let arguments = graph.descendants(entry, "SW-SERVICE-ARG");
        let mut actual = Vec::new();
        for argument in arguments {
            let native = native_type(graph, argument).ok_or_else(|| {
                vec![graph.diagnostic(
                    argument,
                    DiagnosticCategory::Input,
                    "BSW_SIGNATURE_CONFLICT",
                    crate::product_message!(
                        "backend.integration.catalog.bsw_argument_native_type_not_unique"
                    ),
                    crate::product_message!(
                        "backend.integration.catalog.bsw_matching_base_type_declaration_supply"
                    ),
                )]
            })?;
            actual.push(ContractArgument {
                name: graph.text(argument, "SHORT-NAME").unwrap_or("").into(),
                native_type: native,
                direction: graph.text(argument, "DIRECTION").unwrap_or("").into(),
            });
        }
        if return_type.as_deref() != Some(current.return_type.as_str())
            || actual.len() != current.arguments.len()
            || actual
                .iter()
                .zip(&current.arguments)
                .any(|(actual, expected)| {
                    actual.native_type != expected.native_type
                        || actual.direction != expected.direction
                })
            || !matches!(graph.text(entry, "IS-SYNCHRONOUS"), Some("true" | "1"))
        {
            return Err(vec![graph.diagnostic(entry, DiagnosticCategory::Input, "BSW_SIGNATURE_CONFLICT",
                crate::product_message!("backend.integration.catalog.bsw_description_source_contract_mismatch", "symbol" => symbol),
                crate::product_message!("backend.integration.catalog.bsw_abi_contract_correct"))]);
        }
        let owners: Vec<_> = graph
            .of_kind("BSW-MODULE-DESCRIPTION")
            .into_iter()
            .filter(|owner| {
                graph
                    .descendants(*owner, "BSW-MODULE-ENTRY-REF")
                    .into_iter()
                    .any(|reference| graph.elements[reference].text == graph.elements[entry].object)
            })
            .collect();
        if owners.len() != 1 {
            return Err(vec![graph.diagnostic(
                entry,
                DiagnosticCategory::Input,
                "SYMBOL_PRODUCER_DUPLICATE",
                crate::product_message!(
                    "backend.integration.catalog.bsw_entry_module_description_required"
                ),
                crate::product_message!(
                    "backend.integration.catalog.bsw_implemented_entry_relationship_supply"
                ),
            )]);
        }
        let owner = owners[0];
        used_owners.insert(owner);
        let implementations: Vec<_> = graph
            .of_kind("BSW-IMPLEMENTATION")
            .into_iter()
            .filter(|implementation| {
                graph
                    .target(*implementation, "BEHAVIOR-REF")
                    .is_some_and(|behavior| graph.within(behavior, owner))
            })
            .collect();
        if implementations.len() != 1
            || graph.text(implementations[0], "PROGRAMMING-LANGUAGE") != Some("C")
        {
            return Err(vec![graph.diagnostic(
                owner,
                DiagnosticCategory::Input,
                "SYMBOL_PRODUCER_DUPLICATE",
                crate::product_message!(
                    "backend.integration.catalog.bsw_matching_c_implementation_required"
                ),
                crate::product_message!(
                    "backend.integration.catalog.bsw_c_implementation_and_behavior_supply"
                ),
            )]);
        }
        symbols.push(SymbolContract {
            symbol: symbol.clone(),
            return_type: current.return_type.clone(),
            arguments: actual,
            declaration_owner: current.header.clone(),
            definition_owner: current.source.clone(),
            consumers: vec![graph.elements[entry].object.clone()],
        });
    }
    for owner in used_owners {
        for reference in graph.descendants(owner, "BSW-MODULE-ENTRY-REF") {
            let entry = *graph.objects.get(&graph.elements[reference].text).unwrap();
            let symbol = graph.text(entry, "SHORT-NAME").unwrap_or("");
            if !catalog.description.entries.contains_key(symbol) {
                return Err(vec![graph.diagnostic(
                    entry,
                    DiagnosticCategory::Input,
                    "BSW_ENTRY_MISSING",
                    crate::product_message!(
                        "backend.integration.catalog.bsw_selected_entry_source_producer_missing"
                    ),
                    crate::product_message!(
                        "backend.integration.catalog.bsw_entry_source_type_contract_supply"
                    ),
                )]);
            }
        }
    }
    Ok(symbols)
}
