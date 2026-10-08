//! The validated, immutable source of truth for the Epic 4 target.
//!
//! Construction is all-or-nothing. Input bytes and identities are retained;
//! component contracts and ECU integration consume the same checked plan.

mod arti;
mod artifacts;
mod catalog;
mod communication;
mod component;
mod configuration;
mod contracts;
mod diagnostic;
mod ecu;
pub(crate) mod editor;
mod graph;
pub(crate) mod handoff;
mod multi;
mod native_configuration;
pub use multi::{
    Component as MultiComponent, Connection as ComponentConnection, Endpoint as ComponentEndpoint,
    MultiComponentContract, Operation as ComponentOperation,
};
pub const MULTI_PROFILE: &str = multi::PROFILE;
mod os_service;
mod plan;
mod routing;
mod schedule;

pub use catalog::{ContractArgument, RuntimeCatalog, SymbolContract};
pub use communication::SignalChannel;
pub use component::{ComponentContract, DataPort, ServicePort};
pub use configuration::{ConfigurationRecord, EventAssignment};
pub use contracts::ComponentContractFiles;
pub use diagnostic::DiagnosticContract;
pub(crate) use ecu::APPLICATION_SOURCE_PATH;
pub use ecu::EcuIntegrationFiles;
pub use editor::{IntegrationEdit, IntegrationInspection};
pub use handoff::{EcuHandoff, build_ecu_project, open_ecu_handoff, verify_ecu_project};
pub use plan::{
    HandleAssignment, PlanDescription, ValidatedIntegrationPlan, build_plan, build_plan_native,
};
pub use routing::PduRoute;
pub use schedule::{ScheduleContract, ScheduledEntity};

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

pub const PROFILE: &str = "epic4-win64-sr-cs-v1";
pub const FORMAT_VERSION: u32 = 1;
const MOD_PATH: &str = crate::schema::MOD_ZIP;
const MOD_SHA256: &str = "df1e3bc992e49de6e14e5c1a679d7ce7ca90d2450d66186cea0b4a0f1f6555fb";
const XSD_SHA256: &str = crate::schema::XSD_SHA256;

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
            message: message.into(),
            remedy: remedy.into(),
        }
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
            message: message.into(),
            remedy: remedy.into(),
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
    pub fn new(logical_path: impl Into<String>, bytes: Vec<u8>) -> Result<Self, PlanDiagnostic> {
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
            return Err(PlanDiagnostic::at_source(
                &source,
                "SOURCE_IDENTITY",
                crate::product_message!(
                    "backend.integration.mod.input_identity_relative_arxml_required"
                ),
                crate::product_message!(
                    "backend.integration.mod.use_unique_portable_relative_path"
                ),
            ));
        }
        let text = source.text()?;
        if text.contains("<!DOCTYPE") || text.contains("<!ENTITY") {
            return Err(PlanDiagnostic::at_source(
                &source,
                "XML_DTD",
                crate::product_message!(
                    "backend.integration.mod.external_entities_and_dtd_rejected"
                ),
                crate::product_message!(
                    "backend.integration.mod.supply_self_contained_autosar_xml"
                ),
            ));
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

    fn text(&self) -> Result<&str, PlanDiagnostic> {
        std::str::from_utf8(&self.bytes).map_err(|error| PlanDiagnostic::at_source(self,
            "XML_ENCODING", crate::product_message!("backend.integration.mod.input_not_utf8_xml", "error" => error),
            crate::product_message!("backend.integration.mod.convert_source_to_utf8_arxml")))
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
    {
        if matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9") {
            return false;
        }
    }
    true
}

/// Legally supplied, external validation references. These files are checked
/// by identity and are never copied into a generated project.
#[derive(Clone)]
pub struct PlanDependencies {
    pub xsd_archive: PathBuf,
    pub mod_archive: PathBuf,
}

/// External standards are never part of the embedded source inventory.
pub type ValidationResources = PlanDependencies;

impl PlanDependencies {
    pub fn from_repository(root: &Path) -> Self {
        Self {
            xsd_archive: crate::schema::schema_archive(root),
            mod_archive: crate::schema::reference_archive(root, "AUTOSAR_MOD_ARCHIVE", MOD_PATH),
        }
    }

    pub fn explicit(
        xsd_archive: PathBuf,
        mod_archive: PathBuf,
    ) -> Result<Self, crate::message::LocalizedText> {
        for (kind, path) in [("XSD", &xsd_archive), ("MOD", &mod_archive)] {
            if !path.is_absolute()
                || path
                    .components()
                    .any(|part| matches!(part, Component::ParentDir))
            {
                return Err(
                    crate::product_message!("backend.integration.mod.archive_normalized_absolute_path_required", "kind" => kind, "value0" => path.display()),
                );
            }
        }
        Ok(Self {
            xsd_archive,
            mod_archive,
        })
    }

    pub fn validate(&self) -> Result<(), Vec<PlanDiagnostic>> {
        read_archive(&self.xsd_archive, XSD_SHA256, false)?;
        read_archive(&self.mod_archive, MOD_SHA256, false)?;
        Ok(())
    }
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
    pub fn component_contract(&self) -> Result<ComponentContract, Vec<PlanDiagnostic>> {
        component::inspect(&self.graph)
    }
    pub fn schedule_contract(&self) -> Result<ScheduleContract, Vec<PlanDiagnostic>> {
        let component = self.component_contract()?;
        schedule::inspect(&self.graph, &component)
    }
    pub fn signal_channels(&self) -> Result<Vec<SignalChannel>, Vec<PlanDiagnostic>> {
        let component = self.component_contract()?;
        communication::inspect(&self.graph, &component)
    }
    pub fn current_bsw_contracts(
        &self,
        catalog: &RuntimeCatalog,
    ) -> Result<Vec<SymbolContract>, Vec<PlanDiagnostic>> {
        catalog::inspect(&self.graph, catalog)
    }
    pub fn diagnostic_contract(&self) -> Result<DiagnosticContract, Vec<PlanDiagnostic>> {
        let component = self.component_contract()?;
        diagnostic::inspect(&self.graph, &component)
    }
    pub fn pdu_routes(&self) -> Result<Vec<PduRoute>, Vec<PlanDiagnostic>> {
        let component = self.component_contract()?;
        let signals = communication::inspect(&self.graph, &component)?;
        let diagnostic = diagnostic::inspect(&self.graph, &component)?;
        routing::inspect(&self.graph, &signals, &diagnostic)
    }
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

/// Validate identities, the pinned R24-11 schema and all typed references.
/// No inputs or generated outputs are written. Semantic plan construction
/// consumes this same graph rather than reparsing a second source of truth.
pub fn inspect_inputs(
    sources: &[InputSource],
    dependencies: &PlanDependencies,
) -> Result<InputInspection, Vec<PlanDiagnostic>> {
    if sources.is_empty() {
        return Err(vec![PlanDiagnostic::dependency(
            "INPUT_MISSING",
            crate::product_message!("backend.integration.mod.integration_arxml_inputs_missing"),
            crate::product_message!("backend.integration.mod.supply_integration_arxml_inputs"),
        )]);
    }
    let mut names = BTreeSet::new();
    for source in sources {
        // Conservatively fold Unicode case aliases as well as ASCII on the
        // supported case-insensitive Windows input filesystem.
        if !names.insert(source.logical_path.to_uppercase()) {
            return Err(vec![PlanDiagnostic::at_source(
                source,
                "SOURCE_DUPLICATE",
                crate::product_message!(
                    "backend.integration.mod.portable_source_identity_duplicate"
                ),
                crate::product_message!(
                    "backend.integration.mod.select_distinct_sources_without_case_aliases"
                ),
            )]);
        }
    }
    let mut sorted = sources.to_vec();
    sorted.sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
    let archive = checked_archive(&dependencies.mod_archive, MOD_SHA256)?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(archive)).map_err(|error| {
        vec![PlanDiagnostic::dependency(
            "MOD_ARCHIVE",
            crate::product_message!("backend.integration.mod.mod_archive_invalid", "error" => error),
            crate::product_message!("backend.integration.mod.supply_pinned_unmodified_mod_archive"),
        )]
    })?;
    let mut definitions = String::new();
    archive
        .by_name("AUTOSAR_CP_MOD_ECUConfigurationParameters.arxml")
        .and_then(|mut member| {
            member
                .read_to_string(&mut definitions)
                .map_err(zip::result::ZipError::Io)
        })
        .map_err(|error| {
            vec![PlanDiagnostic::dependency(
                "MOD_CONTENT",
                crate::product_message!("backend.integration.mod.ecuc_definitions_read_failed", "error" => error),
                crate::product_message!("backend.integration.mod.restore_complete_external_mod_archive"),
            )]
        })?;
    checked_archive(&dependencies.xsd_archive, XSD_SHA256)?;
    let files: Vec<_> = sorted
        .iter()
        .map(|source| Ok((Path::new(&source.logical_path), source.text()?)))
        .collect::<Result<_, PlanDiagnostic>>()
        .map_err(|diagnostic| vec![diagnostic])?;
    let issues =
        crate::schema::validate_files(&dependencies.xsd_archive, &files).map_err(|error| {
            vec![PlanDiagnostic {
                category: DiagnosticCategory::Tool,
                code: "XSD_TOOL".into(),
                file: None,
                object: None,
                message: error,
                remedy: crate::product_message!(
                    "backend.integration.mod.check_schema_archive_and_validator"
                )
                .into(),
            }]
        })?;
    if !issues.is_empty() {
        return Err(issues
            .into_iter()
            .map(|issue| PlanDiagnostic {
                category: DiagnosticCategory::Input,
                code: issue.code,
                file: issue.file,
                object: issue.path.or_else(|| Some("/".into())),
                message: issue.message,
                remedy: crate::product_message!(
                    "backend.integration.mod.repair_input_against_autosar_schema"
                )
                .into(),
            })
            .collect());
    }
    let graph = graph::Graph::new(&sorted, &definitions)?;
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
    Ok(InputInspection {
        sources: sorted,
        identities,
        graph,
        validation_dependencies: std::collections::BTreeMap::from([
            ("R24-11 XSD".into(), XSD_SHA256.into()),
            ("R24-11 MOD".into(), MOD_SHA256.into()),
        ]),
        rule_set_identity: None,
        required_extension_definitions: Vec::new(),
        definition_catalog: None,
    })
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
            )
            .into(),
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
        .collect::<Result<_, PlanDiagnostic>>()
        .map_err(|diagnostic| vec![diagnostic])?;
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

fn checked_archive(path: &Path, expected: &str) -> Result<Vec<u8>, Vec<PlanDiagnostic>> {
    read_archive(path, expected, true)
}

fn read_archive(
    path: &Path,
    expected: &str,
    collect: bool,
) -> Result<Vec<u8>, Vec<PlanDiagnostic>> {
    let unreadable = |error: std::io::Error| {
        vec![PlanDiagnostic::dependency(
            "DEPENDENCY_MISSING",
            crate::product_message!("backend.integration.mod.external_reference_read_failed", "value0" => path.display(), "error" => error),
            crate::product_message!(
                "backend.integration.mod.supply_legal_pinned_validation_archive"
            ),
        )]
    };
    let mut file = std::fs::File::open(path).map_err(&unreadable)?;
    let mut bytes = Vec::new();
    if collect {
        let size = file.metadata().map_err(&unreadable)?.len();
        let capacity = usize::try_from(size).map_err(|error| {
            vec![PlanDiagnostic::dependency(
                "DEPENDENCY_IDENTITY",
                crate::product_message!("backend.integration.mod.archive_size_unrepresentable", "error" => error),
                crate::product_message!("backend.integration.mod.supply_exact_pinned_archive"),
            )]
        })?;
        bytes.reserve_exact(capacity);
    }
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = file.read(&mut buffer).map_err(&unreadable)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        if collect {
            bytes.extend_from_slice(&buffer[..count]);
        }
    }
    if format!("{:x}", digest.finalize()) != expected {
        return Err(vec![PlanDiagnostic::dependency(
            "DEPENDENCY_IDENTITY",
            crate::product_message!("backend.integration.mod.external_archive_identity_mismatch", "value0" => path.display()),
            crate::product_message!(
                "backend.integration.mod.restore_target_pinned_archive_baseline"
            ),
        )]);
    }
    Ok(bytes)
}
