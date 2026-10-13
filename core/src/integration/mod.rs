//! The validated, immutable source of truth for ECU integration planning.
//!
//! Construction is all-or-nothing. Input bytes and identities are retained;
//! component contracts and ECU integration consume the same checked plan.

// Generation and integration report PlanDiagnostic instead of panicking and
// render each output once; core/clippy.toml lists the disallowed text patching.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_methods,
    clippy::disallowed_macros
)]

mod arti;
mod artifacts;
mod can_runtime;
mod catalog;
mod com;
mod component;
mod composition;
mod configuration;
mod contracts;
mod diagnostic;
pub(crate) mod ecu;
mod ecu_config;
pub(crate) mod editor;
mod graph;
pub(crate) mod handoff;
mod mode;
mod native_configuration;
mod os_service;
mod outputs;
mod plan;
mod routing;
mod rte;
mod schedule;
mod signals;

pub use can_runtime::{CanRuntimeContract, CommunicationIntegerType, CommunicationRuntimeContract};
pub use catalog::{ContractArgument, SymbolContract};
pub use com::{ComMainFunction, ComReception, ComRuntimeContract, ComRxGroup};
pub use component::{ComponentContract, DataPort, ServicePort};
pub use composition::{
    CompositionContract, Connection as ComponentConnection, Endpoint as ComponentEndpoint,
    Operation as ComponentOperation, PROFILE,
};
pub use configuration::{ConfigurationRecord, EventAssignment};
pub use contracts::ComponentContractFiles;
pub use diagnostic::DiagnosticContract;
pub use ecu::EcuIntegrationFiles;
pub use editor::{IntegrationEdit, IntegrationInspection};
pub use handoff::{build_ecu_project, verify_ecu_project};
pub use mode::{ModeRule, ModeRuntimeContract, ModeUser};
pub use outputs::INTEGRATION_PATH;
pub use plan::{HandleAssignment, PlanDescription, ValidatedIntegrationPlan, build_plan_native};
pub use routing::PduRoute;
pub use schedule::{ScheduleContract, ScheduledEntity};
pub use signals::SignalChannel;

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Component, Path};

/// The removed single-component chain.
const SINGLE_COMPONENT_PROFILE: &str = "epic4-win64-sr-cs-v1";
/// The single application slot recorded by retired single-component projects.
const SINGLE_COMPONENT_SLOT: &str = "epic4-single-application-v1";
/// The removed frame/signal host chain.
pub const HOST_PROFILE: &str = "host-can-v1";
/// Product profiles whose projects and deliveries are refused on open rather
/// than reinterpreted: the frame/signal host chain, the historical
/// single-component chain and the earlier name of the current mainline. ARXML
/// sources remain importable.
pub(crate) const RETIRED_PROFILES: [&str; 3] = [
    HOST_PROFILE,
    SINGLE_COMPONENT_PROFILE,
    "singlecore-multi-swc-v1",
];

/// Application slots of a retired profile, recorded in project manifests.
pub(crate) fn retired_profile_of_slot(slot: &str) -> Option<&'static str> {
    if slot == SINGLE_COMPONENT_SLOT {
        return Some(SINGLE_COMPONENT_PROFILE);
    }
    RETIRED_PROFILES.into_iter().find(|profile| {
        slot.strip_prefix(profile)
            .is_some_and(|rest| rest.starts_with(':'))
    })
}
pub const FORMAT_VERSION: u32 = 2;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCategory {
    Input,
    Unsupported,
    Dependency,
    Tool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDiagnostic {
    pub category: DiagnosticCategory,
    pub code: String,
    pub file: Option<String>,
    pub object: Option<String>,
    pub message: crate::message::LocalizedText,
    pub remedy: crate::message::LocalizedText,
}

/// XML character data and attribute text.
fn xml_text(value: &str) -> String {
    let mut text = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => text.push_str("&amp;"),
            '<' => text.push_str("&lt;"),
            '>' => text.push_str("&gt;"),
            '"' => text.push_str("&quot;"),
            other => text.push(other),
        }
    }
    text
}

/// The last segment of a complete AUTOSAR path.
fn short_name(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

impl PlanDiagnostic {
    fn dependency(
        code: &str,
        message: crate::message::LocalizedText,
        remedy: crate::message::LocalizedText,
    ) -> Self {
        Self {
            category: DiagnosticCategory::Dependency,
            code: code.into(),
            file: None,
            object: None,
            message,
            remedy,
        }
    }

    fn output(code: &str, path: &str, message: crate::message::LocalizedText) -> Vec<Self> {
        vec![Self {
            category: DiagnosticCategory::Tool,
            code: code.into(),
            file: None,
            object: Some(path.into()),
            message,
            remedy: crate::product_message!(
                "backend.integration.ecu.generation_identity_contract_repair_required"
            ),
        }]
    }

    fn output_conflict(path: &str, first: &str, second: &str) -> Vec<Self> {
        Self::output(
            "OUTPUT_PRODUCER_CONFLICT",
            path,
            crate::product_message!("backend.integration.outputs.producer_conflict", "path" => path, "first" => first, "second" => second),
        )
    }

    fn output_missing(path: &str) -> Vec<Self> {
        Self::output(
            "OUTPUT_MISSING",
            path,
            crate::product_message!("backend.integration.outputs.output_missing", "path" => path),
        )
    }

    /// A premise the validated plan should guarantee, such as a route or
    /// handle for a planned object, is missing when an emitter needs it.
    fn plan_premise(object: &str) -> Vec<Self> {
        Self::output(
            "PLAN_PREMISE",
            object,
            crate::product_message!("backend.integration.outputs.plan_premise_missing", "object" => object),
        )
    }

    fn template_anchor(path: &str, anchor: &str) -> Vec<Self> {
        Self::output(
            "TEMPLATE_ANCHOR",
            path,
            crate::product_message!("backend.integration.outputs.template_anchor_missing", "path" => path, "anchor" => anchor),
        )
    }

    fn at_source(
        source: &InputSource,
        code: &str,
        message: crate::message::LocalizedText,
        remedy: crate::message::LocalizedText,
    ) -> Self {
        Self {
            category: DiagnosticCategory::Input,
            code: code.into(),
            file: Some(source.logical_path.clone()),
            object: Some("/".into()),
            message,
            remedy,
        }
    }
}

/// Original file bytes with a portable logical identity, independent of the
/// current checkout or the receiving machine's absolute path.
#[derive(Clone, Debug)]
pub struct InputSource {
    logical_path: String,
    bytes: Vec<u8>,
}

impl InputSource {
    pub fn new(
        logical_path: impl Into<String>,
        bytes: Vec<u8>,
    ) -> Result<Self, Box<PlanDiagnostic>> {
        let logical_path = logical_path.into();
        let source = Self {
            logical_path,
            bytes,
        };
        let path = Path::new(&source.logical_path);
        if source.logical_path.is_empty()
            || source.logical_path.contains('\\')
            || source.logical_path.contains(':')
            || source
                .logical_path
                .split('/')
                .any(|part| !portable_component(part))
            || path.is_absolute()
            || !path
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
            || !path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("arxml"))
        {
            return Err(Box::new(PlanDiagnostic::at_source(
                &source,
                "SOURCE_IDENTITY",
                crate::product_message!(
                    "backend.integration.mod.input_identity_relative_arxml_required"
                ),
                crate::product_message!(
                    "backend.integration.mod.use_unique_portable_relative_path"
                ),
            )));
        }
        let text = source.text()?;
        if text.contains("<!DOCTYPE") || text.contains("<!ENTITY") {
            return Err(Box::new(PlanDiagnostic::at_source(
                &source,
                "XML_DTD",
                crate::product_message!(
                    "backend.integration.mod.external_entities_and_dtd_rejected"
                ),
                crate::product_message!(
                    "backend.integration.mod.supply_self_contained_autosar_xml"
                ),
            )));
        }
        Ok(source)
    }

    pub fn logical_path(&self) -> &str {
        &self.logical_path
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn sha256(&self) -> String {
        format!("{:x}", Sha256::digest(&self.bytes))
    }

    fn text(&self) -> Result<&str, Box<PlanDiagnostic>> {
        std::str::from_utf8(&self.bytes).map_err(|error| Box::new(PlanDiagnostic::at_source(self,
            "XML_ENCODING", crate::product_message!("backend.integration.mod.input_not_utf8_xml", "error" => error),
            crate::product_message!("backend.integration.mod.convert_source_to_utf8_arxml"))))
    }
}

fn portable_component(part: &str) -> bool {
    if part.is_empty()
        || part == "."
        || part == ".."
        || part.ends_with('.')
        || part.ends_with(' ')
        || part
            .chars()
            .any(|character| character.is_control() || "<>\"|?*".contains(character))
    {
        return false;
    }
    let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return false;
    }
    if let Some(number) = stem
        .strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"))
        && matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
    {
        return false;
    }
    true
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceIdentity {
    pub logical_path: String,
    pub raw_sha256: String,
    pub roles: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectIdentity {
    pub path: String,
    pub kind: String,
    pub file: String,
}

/// Structural preflight only. This is intentionally distinct from the final
/// ValidatedIntegrationPlan: reference/XSD closure cannot prove semantic
/// type, communication, scheduling or producer closure.
pub struct InputInspection {
    sources: Vec<InputSource>,
    identities: Vec<SourceIdentity>,
    graph: graph::Graph,
    validation_dependencies: std::collections::BTreeMap<String, String>,
    rule_set_identity: Option<crate::project_model::RuleSetIdentity>,
    required_extension_definitions: Vec<crate::project_model::ExtensionDefinitionIdentity>,
    definition_catalog: Option<crate::definitions::DefinitionCatalog>,
}

impl InputInspection {
    pub fn sources(&self) -> &[InputSource] {
        &self.sources
    }
    pub fn identities(&self) -> &[SourceIdentity] {
        &self.identities
    }
    pub fn objects(&self) -> Vec<ObjectIdentity> {
        self.graph
            .objects
            .iter()
            .map(|(path, index)| {
                let element = &self.graph.elements[*index];
                ObjectIdentity {
                    path: path.clone(),
                    kind: element.tag.clone(),
                    file: element.file.clone(),
                }
            })
            .collect()
    }
}

fn check_native_scope(
    validation: crate::project_model::ScopeValidation,
    consumers: &BTreeSet<String>,
) -> Result<(), Vec<PlanDiagnostic>> {
    let errors: Vec<_> = validation
        .diagnostics
        .into_iter()
        .filter(|diagnostic| {
            matches!(diagnostic.severity, crate::model::Severity::Error)
                || (matches!(
                    diagnostic.code.as_str(),
                    "NATIVE_UNSUPPORTED" | "DEFINITION_UNKNOWN" | "SEMANTICS_UNSUPPORTED"
                ) && diagnostic
                    .path
                    .as_ref()
                    .is_some_and(|path| consumers.contains(path)))
        })
        .map(|diagnostic| PlanDiagnostic {
            category: if matches!(
                diagnostic.code.as_str(),
                "NATIVE_UNSUPPORTED" | "DEFINITION_UNKNOWN" | "SEMANTICS_UNSUPPORTED"
            ) {
                DiagnosticCategory::Unsupported
            } else {
                DiagnosticCategory::Input
            },
            code: diagnostic.code,
            file: diagnostic.file,
            object: diagnostic.path,
            message: diagnostic.message,
            remedy: diagnostic.remedy,
        })
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate product-supported inputs with the trusted compiled rule inventory
/// and this project's explicitly accepted definitions. No official archive is read.
pub fn inspect_inputs_native(
    sources: &[InputSource],
    catalog: &crate::definitions::DefinitionCatalog,
) -> Result<InputInspection, Vec<PlanDiagnostic>> {
    let tool_error = |message: crate::message::LocalizedText| {
        vec![PlanDiagnostic {
            category: DiagnosticCategory::Tool,
            code: "BUILTIN_RULES".into(),
            file: None,
            object: None,
            message,
            remedy: crate::product_message!(
                "backend.integration.mod.repair_installed_product_rule_inventory"
            ),
        }]
    };
    let identity = crate::rules::rule_set_identity().map_err(&tool_error)?;
    if sources.is_empty() {
        return Err(vec![PlanDiagnostic::dependency(
            "INPUT_MISSING",
            crate::product_message!("backend.integration.mod.integration_arxml_inputs_missing"),
            crate::product_message!("backend.integration.mod.supply_integration_arxml_inputs"),
        )]);
    }
    let mut names = BTreeSet::new();
    for source in sources {
        if source.bytes.len() > 50 * 1024 * 1024 {
            return Err(vec![PlanDiagnostic::at_source(
                source,
                "SOURCE_SIZE",
                crate::product_message!("backend.integration.mod.arxml_input_size_exceeded"),
                crate::product_message!(
                    "backend.integration.mod.split_input_into_explicit_source_members"
                ),
            )]);
        }
        if !names.insert(source.logical_path.to_uppercase()) {
            return Err(vec![PlanDiagnostic::at_source(
                source,
                "SOURCE_DUPLICATE",
                crate::product_message!(
                    "backend.integration.mod.portable_source_identity_duplicate"
                ),
                crate::product_message!(
                    "backend.integration.mod.select_distinct_portable_source_paths"
                ),
            )]);
        }
    }
    let mut sorted = sources.to_vec();
    sorted.sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
    let files: Vec<_> = sorted
        .iter()
        .map(|source| Ok((Path::new(&source.logical_path), source.text()?)))
        .collect::<Result<_, Box<PlanDiagnostic>>>()
        .map_err(|diagnostic| vec![*diagnostic])?;
    let native_schema = crate::rules::validate_native(&files).map_err(&tool_error)?;
    let graph = graph::Graph::from_catalog_target_scope(&sorted, catalog)?;
    let consumers = native_configuration::consumer_objects(&graph);
    check_native_scope(native_schema, &consumers)?;
    let reference_errors = graph.reference_diagnostics_for(&consumers);
    if !reference_errors.is_empty() {
        return Err(reference_errors);
    }
    let mut definition_validation = catalog.validate_documents(&files).map_err(&tool_error)?;
    definition_validation.diagnostics.retain(|diagnostic| {
        diagnostic
            .path
            .as_ref()
            .is_some_and(|path| consumers.contains(path))
    });
    check_native_scope(definition_validation, &consumers)?;
    let identities = sorted
        .iter()
        .map(|source| {
            let kinds: BTreeSet<_> = graph
                .objects
                .values()
                .map(|index| &graph.elements[*index])
                .filter(|element| element.file == source.logical_path)
                .map(|element| element.tag.as_str())
                .collect();
            let mut roles = Vec::new();
            for (kind, role) in [
                ("SYSTEM", "ecu_extract"),
                ("APPLICATION-SW-COMPONENT-TYPE", "application"),
                ("SERVICE-SW-COMPONENT-TYPE", "service_client"),
                ("IMPLEMENTATION-DATA-TYPE", "types"),
                ("BSW-MODULE-DESCRIPTION", "bsw_description"),
                ("BSW-IMPLEMENTATION", "bsw_implementation"),
                ("ECUC-MODULE-CONFIGURATION-VALUES", "ecuc_values"),
            ] {
                if kinds.contains(kind) {
                    roles.push(role.into());
                }
            }
            if roles.is_empty() {
                roles.push("retained".into());
            }
            SourceIdentity {
                logical_path: source.logical_path.clone(),
                raw_sha256: source.sha256(),
                roles,
            }
        })
        .collect();
    let definition_ids: Vec<_> = graph
        .elements
        .iter()
        .filter(|element| element.tag == "DEFINITION-REF")
        .filter(|element| consumers.contains(&element.object))
        .map(|element| element.text.clone())
        .collect();
    let required_extension_definitions = catalog.required_extensions(&definition_ids);
    let validation_dependencies = std::collections::BTreeMap::from([
        ("rules.release".into(), identity.release.clone()),
        ("rules.version".into(), identity.rules_version.clone()),
        ("rules.sha256".into(), identity.sha256.clone()),
        ("definitions.sha256".into(), catalog.fingerprint(&identity)),
    ]);
    Ok(InputInspection {
        sources: sorted,
        identities,
        graph,
        validation_dependencies,
        rule_set_identity: Some(identity),
        required_extension_definitions,
        definition_catalog: Some(catalog.clone()),
    })
}
