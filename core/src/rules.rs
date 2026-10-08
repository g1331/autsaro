//! Offline, product-authored R24-11 checks. The official oracle remains in schema.rs.
//! The inventory binds the actual implementation and every authored definition source.

mod grammar;

use crate::model::Severity;
use crate::project_model::{
    ConfigurationDiagnostic, RuleCoverage, RuleSetIdentity, ScopeValidation, ValidationScope,
    ValidationStatus, ValidationWitness,
};
use roxmltree::{Document, Node};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};
use std::sync::LazyLock;

const NS: &str = "http://autosar.org/schema/r4.0";
const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";
const MAX_SOURCE_BYTES: usize = 50 * 1024 * 1024;

struct InventorySource {
    path: &'static str,
    sha256: &'static str,
    bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/native_rule_index.rs"));

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Inventory {
    format: String,
    release: String,
    rules_version: String,
    sources: Vec<SourceIdentity>,
    coverage: Vec<RuleCoverage>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceIdentity {
    path: String,
    sha256: String,
}

/// Compiled authority, available even when an installation inventory is corrupt.
/// This identity alone is not evidence that validation executed successfully.
pub fn trusted_rule_set_identity() -> RuleSetIdentity {
    RuleSetIdentity {
        release: grammar::RELEASE.into(),
        rules_version: grammar::RULES_VERSION.into(),
        sha256: TRUSTED_INVENTORY_SHA256.into(),
    }
}

/// Immutable inventory bytes; no caller-provided metadata can replace them.
pub fn inventory_bytes() -> &'static [u8] {
    INVENTORY_BYTES
}

fn read_inventory(bytes: &[u8]) -> Result<Inventory, crate::LocalizedText> {
    if format!("{:x}", Sha256::digest(bytes)) != TRUSTED_INVENTORY_SHA256 {
        return Err(crate::product_message!("backend.rules.inventory_hash"));
    }
    let inventory: Inventory = serde_json::from_slice(bytes).map_err(
        |error| crate::product_message!("backend.rules.inventory_parse", "error" => error),
    )?;
    if inventory.format != "autosar-native-rule-inventory-v1"
        || inventory.release != grammar::RELEASE
        || inventory.rules_version != grammar::RULES_VERSION
        || inventory.sources.len() != INVENTORY_SOURCES.len()
    {
        return Err(crate::product_message!("backend.rules.inventory_members"));
    }
    for (declared, source) in inventory.sources.iter().zip(INVENTORY_SOURCES) {
        if declared.path != source.path
            || declared.sha256 != source.sha256
            || format!("{:x}", Sha256::digest(source.bytes)) != source.sha256
        {
            return Err(
                crate::product_message!("backend.rules.inventory_source", "path" => source.path),
            );
        }
    }
    let supported: Vec<_> = inventory
        .coverage
        .iter()
        .filter(|row| row.supported)
        .collect();
    let expected = grammar::coverage_rows();
    if supported.len() != expected.len()
        || supported
            .iter()
            .zip(expected)
            .any(|(row, (scope, id, subject))| {
                row.rule_id != id
                    || row.subjects != [subject]
                    || row.reason.is_some()
                    || row.scope
                        != if scope == "schema" {
                            ValidationScope::Schema
                        } else {
                            ValidationScope::SourceSafety
                        }
            })
    {
        return Err(crate::product_message!("backend.rules.inventory_coverage"));
    }
    Ok(inventory)
}

/// Validate an installed/copied inventory against compiled authority, not its own hashes.
pub fn verify_inventory_bytes(bytes: &[u8]) -> Result<RuleSetIdentity, crate::LocalizedText> {
    read_inventory(bytes)?;
    Ok(trusted_rule_set_identity())
}

fn verified_inventory() -> Result<&'static Inventory, crate::LocalizedText> {
    static VERIFIED: LazyLock<Result<Inventory, crate::LocalizedText>> = LazyLock::new(|| {
        #[cfg(feature = "verification-metrics")]
        crate::verification::record_rule_load();
        read_inventory(INVENTORY_BYTES)
    });
    VERIFIED.as_ref().map_err(Clone::clone)
}

pub fn rule_set_identity() -> Result<RuleSetIdentity, crate::LocalizedText> {
    verified_inventory()?;
    Ok(trusted_rule_set_identity())
}

pub fn coverage() -> Result<Vec<RuleCoverage>, crate::LocalizedText> {
    let mut coverage = verified_inventory()?.coverage.clone();
    for row in &mut coverage {
        if row.rule_id == "native.unsupported" {
            row.reason = Some(crate::product_message!(
                "backend.rules.inventory_partial_coverage"
            ));
        }
    }
    Ok(coverage)
}

struct ChildGroup {
    names: Vec<&'static str>,
    min: usize,
    max: usize,
}

struct NativeGrammar {
    structures: BTreeMap<&'static str, Vec<ChildGroup>>,
    scalars: BTreeMap<&'static str, &'static str>,
}

fn native_grammar() -> &'static NativeGrammar {
    static GRAMMAR: LazyLock<NativeGrammar> = LazyLock::new(|| {
        #[cfg(feature = "verification-metrics")]
        crate::verification::record_grammar_load();
        NativeGrammar {
            structures: grammar::STRUCTURES
                .iter()
                .map(|(name, shape)| {
                    let groups = shape
                        .split_ascii_whitespace()
                        .map(|token| {
                            let (names, min, max) = match token.as_bytes().last() {
                                Some(b'?') => (&token[..token.len() - 1], 0, 1),
                                Some(b'*') => (&token[..token.len() - 1], 0, usize::MAX),
                                Some(b'+') => (&token[..token.len() - 1], 1, usize::MAX),
                                _ => (token, 1, 1),
                            };
                            ChildGroup {
                                names: names.split('|').collect(),
                                min,
                                max,
                            }
                        })
                        .collect();
                    (*name, groups)
                })
                .collect(),
            scalars: grammar::SCALARS.iter().copied().collect(),
        }
    });
    &GRAMMAR
}

fn check_source(path: &Path, text: &str) -> Result<(), crate::LocalizedText> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
        || !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("arxml"))
    {
        return Err(crate::product_message!("backend.rules.source_path", "path" => path.display()));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| crate::LocalizedText::from(error.to_string()))?
            .join(path)
    };
    // Prospective/new source paths need not exist. Existing ancestors still
    // cannot be links; validation never reads source bytes back from disk.
    for ancestor in absolute.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => {
                #[cfg(windows)]
                let linked = {
                    use std::os::windows::fs::MetadataExt;
                    metadata.file_attributes() & 0x400 != 0
                };
                #[cfg(not(windows))]
                let linked = metadata.file_type().is_symlink();
                if linked || (ancestor == absolute && !metadata.is_file()) {
                    return Err(
                        crate::product_message!("backend.rules.source_link", "path" => ancestor.display()),
                    );
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("{}: {error}", ancestor.display()).into()),
        }
    }
    if text.len() > MAX_SOURCE_BYTES {
        return Err(crate::product_message!("backend.rules.source_size", "path" => path.display()));
    }
    if text.contains("<!DOCTYPE") || text.contains("<!ENTITY") {
        return Err(
            crate::product_message!("backend.rules.source_entities", "path" => path.display()),
        );
    }
    if let Some(declaration) = text.trim_start_matches('\u{feff}').strip_prefix("<?xml") {
        let declaration = declaration.split("?>").next().unwrap_or(declaration);
        if let Some(encoding) = declaration.split("encoding").nth(1) {
            let encoding = encoding
                .trim_start()
                .strip_prefix('=')
                .unwrap_or(encoding)
                .trim_start();
            let quote = encoding.chars().next().unwrap_or(' ');
            let value = encoding
                .get(1..)
                .unwrap_or("")
                .split(quote)
                .next()
                .unwrap_or("");
            if !value.eq_ignore_ascii_case("UTF-8") {
                return Err(
                    crate::product_message!("backend.rules.source_encoding", "path" => path.display()),
                );
            }
        }
    }
    Ok(())
}

fn literal<'a>(node: Node<'a, '_>) -> Cow<'a, str> {
    let mut fragments = node
        .children()
        .filter(Node::is_text)
        .filter_map(|child| child.text());
    let first = fragments.next().unwrap_or("");
    let Some(second) = fragments.next() else {
        return Cow::Borrowed(first);
    };
    let mut joined = String::with_capacity(first.len() + second.len());
    joined.push_str(first);
    joined.push_str(second);
    for fragment in fragments {
        joined.push_str(fragment);
    }
    Cow::Owned(joined)
}

fn xml_path(node: Node<'_, '_>) -> String {
    // Use the same AUTOSAR named-object locator as the Workspace projection.
    // Unnamed fields locate their owner; ruleId identifies the violated XML field.
    let mut parts = Vec::new();
    for ancestor in node.ancestors().filter(Node::is_element) {
        if let Some(name) = ancestor
            .children()
            .find(|child| child.has_tag_name((NS, "SHORT-NAME")))
            .map(literal)
        {
            parts.push(name);
        }
    }
    parts.reverse();
    format!("/{}", parts.join("/"))
}

fn diagnostic(
    file: &Path,
    node: Node<'_, '_>,
    rule_id: String,
    constraint: crate::LocalizedText,
    counterexample: crate::LocalizedText,
    unsupported: bool,
) -> ConfigurationDiagnostic {
    let path = xml_path(node);
    ConfigurationDiagnostic {
        scope: ValidationScope::Schema,
        rule_id: rule_id.clone(),
        severity: if unsupported {
            Severity::Warning
        } else {
            Severity::Error
        },
        code: if unsupported {
            "NATIVE_UNSUPPORTED"
        } else {
            "NATIVE_SCHEMA"
        }
        .into(),
        message: crate::LocalizedText::messages([constraint.clone(), counterexample.clone()]),
        remedy: if unsupported {
            crate::product_message!("backend.rules.remedy_unsupported")
        } else {
            crate::product_message!("backend.rules.remedy_schema")
        },
        file: Some(file.display().to_string()),
        path: Some(path.clone()),
        source_id: None,
        object_id: None,
        field_id: None,
        witness: Some(ValidationWitness {
            rule_id,
            // The Workspace replaces source locators with its opaque subject IDs.
            subjects: vec![path],
            constraint,
            counterexample,
        }),
    }
}

fn identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        && value.len() <= 128
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn integer(value: &str, unsigned: bool) -> bool {
    let value = value.trim();
    let digits = if unsigned {
        value.strip_prefix('+').unwrap_or(value)
    } else {
        value.strip_prefix(['+', '-']).unwrap_or(value)
    };
    if let Some(hex) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        return value == digits
            && !hex.is_empty()
            && hex.bytes().all(|byte| byte.is_ascii_hexdigit());
    }
    if let Some(binary) = digits
        .strip_prefix("0b")
        .or_else(|| digits.strip_prefix("0B"))
    {
        return value == digits
            && !binary.is_empty()
            && binary.bytes().all(|byte| matches!(byte, b'0' | b'1'));
    }
    if digits.starts_with('0') {
        return value == digits && digits.bytes().all(|byte| (b'0'..=b'7').contains(&byte));
    }
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn float_literal(value: &str) -> bool {
    let value = value.trim();
    if matches!(value, "INF" | "-INF" | "NaN") {
        return true;
    }
    let value = value.strip_prefix(['+', '-']).unwrap_or(value);
    let (mantissa, exponent) = value.split_once(['e', 'E']).unwrap_or((value, ""));
    if value.contains(['e', 'E']) {
        let exponent = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        if exponent.is_empty() || !exponent.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
    }
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    (!whole.is_empty() || !fraction.is_empty())
        && whole.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
}

fn number(value: &str) -> bool {
    integer(value, false) || float_literal(value)
}

fn destination_allowed(node: Node<'_, '_>, dest: &str) -> bool {
    let name = node.tag_name().name();
    match name {
        "DEFINITION-REF" => match node.parent_element().map(|parent| parent.tag_name().name()) {
            Some("ECUC-MODULE-CONFIGURATION-VALUES") => dest == "ECUC-MODULE-DEF",
            Some("ECUC-CONTAINER-VALUE") => matches!(
                dest,
                "ECUC-PARAM-CONF-CONTAINER-DEF" | "ECUC-CHOICE-CONTAINER-DEF"
            ),
            Some("ECUC-NUMERICAL-PARAM-VALUE") => matches!(
                dest,
                "ECUC-INTEGER-PARAM-DEF" | "ECUC-FLOAT-PARAM-DEF" | "ECUC-BOOLEAN-PARAM-DEF"
            ),
            Some("ECUC-TEXTUAL-PARAM-VALUE") => matches!(
                dest,
                "ECUC-STRING-PARAM-DEF"
                    | "ECUC-MULTILINE-STRING-PARAM-DEF"
                    | "ECUC-ENUMERATION-PARAM-DEF"
                    | "ECUC-FUNCTION-NAME-DEF"
                    | "ECUC-LINKER-SYMBOL-DEF"
            ),
            Some("ECUC-REFERENCE-VALUE") => matches!(
                dest,
                "ECUC-REFERENCE-DEF"
                    | "ECUC-CHOICE-REFERENCE-DEF"
                    | "ECUC-SYMBOLIC-NAME-REFERENCE-DEF"
                    | "ECUC-FOREIGN-REFERENCE-DEF"
            ),
            _ => false,
        },
        "VALUE-REF" => {
            !dest.is_empty()
                && dest
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
        }
        "TYPE-TREF" => matches!(
            dest,
            "APPLICATION-PRIMITIVE-DATA-TYPE"
                | "IMPLEMENTATION-DATA-TYPE"
                | "APPLICATION-SW-COMPONENT-TYPE"
                | "SERVICE-SW-COMPONENT-TYPE"
                | "COMPOSITION-SW-COMPONENT-TYPE"
        ),
        "PROVIDED-INTERFACE-TREF" | "REQUIRED-INTERFACE-TREF" => matches!(
            dest,
            "SENDER-RECEIVER-INTERFACE" | "CLIENT-SERVER-INTERFACE"
        ),
        "CONTEXT-COMPONENT-REF" | "TARGET-COMPONENT-REF" => dest == "SW-COMPONENT-PROTOTYPE",
        "CONTEXT-COMPOSITION-REF" => matches!(
            dest,
            "ROOT-SW-COMPOSITION-PROTOTYPE" | "SW-COMPONENT-PROTOTYPE"
        ),
        "CONTEXT-PORT-REF" | "PORT-PROTOTYPE-REF" => {
            matches!(dest, "P-PORT-PROTOTYPE" | "R-PORT-PROTOTYPE")
        }
        "TARGET-DATA-PROTOTYPE-REF" | "DATA-ELEMENT-REF" => dest == "VARIABLE-DATA-PROTOTYPE",
        "TARGET-P-PORT-REF" | "CONTEXT-P-PORT-REF" => dest == "P-PORT-PROTOTYPE",
        "TARGET-R-PORT-REF" | "CONTEXT-R-PORT-REF" => dest == "R-PORT-PROTOTYPE",
        "OPERATION-REF" | "TARGET-PROVIDED-OPERATION-REF" | "TARGET-REQUIRED-OPERATION-REF" => {
            dest == "CLIENT-SERVER-OPERATION"
        }
        "START-ON-EVENT-REF" => dest == "RUNNABLE-ENTITY",
        "STARTS-ON-EVENT-REF" => dest == "BSW-SCHEDULABLE-ENTITY",
        "IMPLEMENTED-ENTRY-REF" => dest == "BSW-MODULE-ENTRY",
        "APPLICATION-DATA-TYPE-REF" => dest == "APPLICATION-PRIMITIVE-DATA-TYPE",
        "BASE-TYPE-REF" => dest == "SW-BASE-TYPE",
        "DATA-TYPE-MAPPING-REF" => dest == "DATA-TYPE-MAPPING-SET",
        "SOFTWARE-COMPOSITION-TREF" => dest == "COMPOSITION-SW-COMPONENT-TYPE",
        "FIBEX-ELEMENT-REF" => matches!(dest, "CAN-CLUSTER" | "ECU-INSTANCE"),
        "COMMUNICATION-CONNECTOR-REF" => dest == "CAN-COMMUNICATION-CONNECTOR",
        "COMM-CONTROLLER-REF" => dest == "CAN-COMMUNICATION-CONTROLLER",
        "FRAME-REF" => dest == "CAN-FRAME",
        "I-PDU-REF" | "PDU-REF" => matches!(dest, "I-SIGNAL-I-PDU" | "N-PDU" | "DCM-I-PDU"),
        "BEHAVIOR-REF" => dest == "BSW-INTERNAL-BEHAVIOR",
        _ => name
            .strip_suffix("-REF")
            .is_some_and(|target| dest == target),
    }
}

fn scalar_valid(value: &str, kind: &str) -> bool {
    match kind {
        "text" => true,
        "identifier" => identifier(value),
        "symbol" => {
            value
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        }
        "revision" => {
            let mut parts = value.splitn(3, '.');
            let first = parts.next().unwrap_or("");
            let second = parts.next().unwrap_or("");
            let last = parts
                .next()
                .unwrap_or("")
                .split(['.', '_', ';'])
                .next()
                .unwrap_or("");
            [first, second, last]
                .iter()
                .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        }
        "unsigned" => integer(value, true),
        "integer" => integer(value, false),
        "number" => float_literal(value),
        "boolean" => matches!(value.trim(), "true" | "false" | "0" | "1"),
        // Numerical VALUE is a mixed expression type in XML. Nonliteral
        // expression semantics are reported unsupported before this check;
        // ECUC boolean/integer/float restrictions belong to definition scope.
        "context" => true,
        _ => kind
            .strip_prefix("enum:")
            .is_some_and(|choices| choices.split('|').any(|choice| choice == value.trim())),
    }
}

fn context_accepts(container: &str, owner: &str, child: &str) -> bool {
    match (container, owner) {
        ("CONNECTORS", "ECU-INSTANCE") => child == "CAN-COMMUNICATION-CONNECTOR",
        ("CONNECTORS", "COMPOSITION-SW-COMPONENT-TYPE") => child == "ASSEMBLY-SW-CONNECTOR",
        ("ARGUMENTS", "BSW-MODULE-ENTRY") => child == "SW-SERVICE-ARG",
        ("ARGUMENTS", "CLIENT-SERVER-OPERATION") => child == "ARGUMENT-DATA-PROTOTYPE",
        ("INTERNAL-BEHAVIORS", "BSW-MODULE-DESCRIPTION") => child == "BSW-INTERNAL-BEHAVIOR",
        ("INTERNAL-BEHAVIORS", _) => child == "SWC-INTERNAL-BEHAVIOR",
        ("EVENTS", "BSW-INTERNAL-BEHAVIOR") => child == "BSW-TIMING-EVENT",
        ("EVENTS", "SWC-INTERNAL-BEHAVIOR") => {
            matches!(child, "TIMING-EVENT" | "OPERATION-INVOKED-EVENT")
        }
        ("I-SIGNAL-TRIGGERINGS", "CAN-PHYSICAL-CHANNEL") => child == "I-SIGNAL-TRIGGERING",
        ("I-SIGNAL-TRIGGERINGS", "PDU-TRIGGERING") => {
            child == "I-SIGNAL-TRIGGERING-REF-CONDITIONAL"
        }
        ("PDU-TRIGGERINGS", "CAN-PHYSICAL-CHANNEL") => child == "PDU-TRIGGERING",
        ("PDU-TRIGGERINGS", "CAN-FRAME-TRIGGERING") => child == "PDU-TRIGGERING-REF-CONDITIONAL",
        _ => true,
    }
}

/// Execute native rules on authoritative/prospective UTF-8 text, without fetching
/// schemaLocation or consulting XSD/MOD settings. Safety failures are hard errors.
pub fn validate_native(files: &[(&Path, &str)]) -> Result<ScopeValidation, crate::LocalizedText> {
    #[cfg(feature = "verification-metrics")]
    crate::verification::before(crate::verification::Phase::SchemaCheck);
    let inventory = verified_inventory()?;
    if files.is_empty() {
        return Err(crate::product_message!("backend.rules.source_required"));
    }
    let grammar = native_grammar();
    let mut diagnostics = Vec::new();
    let mut actual_coverage: Vec<_> = inventory
        .coverage
        .iter()
        .filter(|row| row.scope == ValidationScope::Schema && row.supported)
        .cloned()
        .collect();
    let mut unknown = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for (file, text) in files {
        check_source(file, text)?;
        let absolute = if file.is_absolute() {
            file.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|error| crate::LocalizedText::from(error.to_string()))?
                .join(file)
        };
        let identity = if absolute.exists() {
            fs::canonicalize(&absolute)
                .map_err(|error| crate::LocalizedText::from(error.to_string()))?
        } else {
            absolute
        };
        if !paths.insert(identity) {
            return Err(
                crate::product_message!("backend.rules.source_duplicate", "path" => file.display()),
            );
        }
        let doc = Document::parse(text)
            .map_err(|error| crate::product_message!("backend.rules.source_parse", "path" => file.display(), "error" => error))?;
        let root = doc.root_element();
        if !root.has_tag_name((NS, "AUTOSAR")) {
            return Err(
                crate::product_message!("backend.rules.source_namespace", "path" => file.display()),
            );
        }
        let location = root.attribute((XSI, "schemaLocation")).unwrap_or("");
        let mut locations = location.split_ascii_whitespace();
        let mut release_declared = false;
        while let Some(namespace) = locations.next() {
            let Some(location) = locations.next() else {
                return Err(
                    crate::product_message!("backend.rules.schema_location_pair", "path" => file.display()),
                );
            };
            if namespace == NS {
                if release_declared
                    || location.rsplit(['/', '\\']).next() != Some("AUTOSAR_00053.xsd")
                {
                    return Err(
                        crate::product_message!("backend.rules.schema_release_ambiguous", "path" => file.display()),
                    );
                }
                release_declared = true;
            }
        }
        if !release_declared {
            return Err(
                crate::product_message!("backend.rules.schema_release_missing", "path" => file.display()),
            );
        }
        let mut skip_end = 0;
        for node in doc.descendants().filter(Node::is_element) {
            if node.range().start < skip_end {
                continue;
            }
            let name = node.tag_name().name();
            let structure = grammar.structures.get(name);
            let scalar = grammar.scalars.get(name);
            let reference = grammar::REFERENCES.contains(&name);
            if node.tag_name().namespace() != Some(NS)
                || (structure.is_none() && scalar.is_none() && !reference)
            {
                unknown.insert(format!(
                    "{{{}}}{name}",
                    node.tag_name().namespace().unwrap_or("")
                ));
                diagnostics.push(diagnostic(
                    file,
                    node,
                    "native.unsupported".into(),
                    crate::product_message!("backend.rules.constraint_unknown_subtree"),
                    name.into(),
                    true,
                ));
                skip_end = node.range().end;
                continue;
            }
            for attribute in node.attributes() {
                let allowed = match (attribute.namespace(), attribute.name()) {
                    (Some(XSI), "schemaLocation") => node == root,
                    (None, "DEST") => reference,
                    (None, "GID") => matches!(name, "SDG" | "SD"),
                    (None, "UUID") => structure.is_some_and(|groups| {
                        groups
                            .first()
                            .is_some_and(|group| group.names.contains(&"SHORT-NAME"))
                    }),
                    _ => false,
                };
                if !allowed {
                    let subject = format!("{name}/@{}", attribute.name());
                    unknown.insert(subject.clone());
                    diagnostics.push(diagnostic(
                        file,
                        node,
                        "native.unsupported".into(),
                        crate::product_message!("backend.rules.constraint_unknown_attribute"),
                        subject.into(),
                        true,
                    ));
                }
            }
            if matches!(name, "SD" | "SDG") && node.attribute("GID").is_none() {
                diagnostics.push(diagnostic(
                    file,
                    node,
                    format!(
                        "native.{}.{name}",
                        if structure.is_some() {
                            "structure"
                        } else {
                            "type"
                        }
                    ),
                    crate::product_message!("backend.rules.constraint_gid"),
                    crate::product_message!("backend.rules.witness_gid_absent"),
                    false,
                ));
            }
            if let Some(groups) = structure {
                let rule_id = format!("native.structure.{name}");
                let mut counts = [0usize; 32];
                let mut previous = 0;
                let mut unknown_choices = 0;
                for child in node.children() {
                    if child.is_text() && !child.text().unwrap_or("").trim().is_empty() {
                        diagnostics.push(diagnostic(
                            file,
                            node,
                            rule_id.clone(),
                            crate::product_message!("backend.rules.constraint_structure_text"),
                            child.text().unwrap_or("").into(),
                            false,
                        ));
                    }
                    if !child.is_element() {
                        continue;
                    }
                    if child.tag_name().namespace() != Some(NS) {
                        unknown_choices += 1;
                        continue;
                    }
                    let child_name = child.tag_name().name();
                    if let Some(index) = groups
                        .iter()
                        .position(|group| group.names.contains(&child_name))
                    {
                        counts[index] += 1;
                        let owner = node
                            .parent_element()
                            .map(|parent| parent.tag_name().name())
                            .unwrap_or("");
                        if !context_accepts(name, owner, child_name) {
                            diagnostics.push(diagnostic(
                                file,
                                child,
                                rule_id.clone(),
                                crate::product_message!("backend.rules.constraint_context_child", "name" => name, "owner" => owner),
                                child_name.into(),
                                false,
                            ));
                        }
                        if index < previous {
                            diagnostics.push(diagnostic(
                                file,
                                child,
                                rule_id.clone(),
                                crate::product_message!("backend.rules.constraint_child_order"),
                                child_name.into(),
                                false,
                            ));
                        }
                        previous = previous.max(index);
                    } else if grammar.structures.contains_key(child_name)
                        || grammar.scalars.contains_key(child_name)
                        || grammar::REFERENCES.contains(&child_name)
                    {
                        diagnostics.push(diagnostic(
                            file,
                            child,
                            rule_id.clone(),
                            crate::product_message!("backend.rules.constraint_known_child", "name" => name),
                            child_name.into(),
                            false,
                        ));
                    } else {
                        unknown_choices += 1;
                    }
                }
                for (index, group) in groups.iter().enumerate() {
                    let count = counts[index]
                        + if index == 0 && grammar::OPEN_CHOICES.contains(&name) {
                            unknown_choices
                        } else {
                            0
                        };
                    if count < group.min || count > group.max {
                        diagnostics.push(diagnostic(
                            file,
                            node,
                            rule_id.clone(),
                            crate::product_message!("backend.rules.constraint_cardinality",
                                "names" => group.names.join("|"),
                                "min" => group.min,
                                "max" => if group.max == usize::MAX { "n".into() } else { group.max.to_string() }),
                            count.to_string().into(),
                            false,
                        ));
                    }
                }
            } else {
                let value = literal(node);
                let value = value.as_ref();
                let mixed_value = name == "VALUE"
                    && node.parent_element().is_some_and(|parent| {
                        parent.has_tag_name("ECUC-NUMERICAL-PARAM-VALUE")
                            || parent.has_tag_name("NUMERICAL-VALUE-SPECIFICATION")
                    });
                let known_nested_value = node.children().filter(Node::is_element).any(|child| {
                    let child_name = child.tag_name().name();
                    child.tag_name().namespace() == Some(NS)
                        && (grammar.structures.contains_key(child_name)
                            || grammar.scalars.contains_key(child_name)
                            || grammar::REFERENCES.contains(&child_name))
                });
                if mixed_value
                    && !known_nested_value
                    && (node.children().any(|child| child.is_element())
                        || (!number(value) && !matches!(value.trim(), "true" | "false")))
                {
                    unknown.insert("VALUE/expression".into());
                    diagnostics.push(diagnostic(
                        file,
                        node,
                        "native.unsupported".into(),
                        crate::product_message!("backend.rules.constraint_expression"),
                        value.into(),
                        true,
                    ));
                    skip_end = node.range().end;
                    continue;
                }
                let rule_id = format!("native.type.{name}");
                if node.children().any(|child| child.is_element()) {
                    diagnostics.push(diagnostic(
                        file,
                        node,
                        rule_id.clone(),
                        crate::product_message!("backend.rules.constraint_scalar_children"),
                        name.into(),
                        false,
                    ));
                }
                if let Some(kind) = scalar {
                    if !scalar_valid(value, kind) {
                        diagnostics.push(diagnostic(
                            file,
                            node,
                            rule_id,
                            crate::product_message!("backend.rules.constraint_xml_type", "kind" => kind),
                            value.into(),
                            false,
                        ));
                    }
                } else if reference {
                    let value = value.trim();
                    if !value.starts_with('/')
                        || value[1..].split('/').any(|part| !identifier(part))
                        || !node
                            .attribute("DEST")
                            .is_some_and(|dest| destination_allowed(node, dest))
                    {
                        diagnostics.push(diagnostic(
                            file,
                            node,
                            rule_id,
                            crate::product_message!("backend.rules.constraint_reference"),
                            crate::product_message!("backend.rules.witness_reference",
                                "value" => value,
                                "dest" => node.attribute("DEST").unwrap_or("<absent>")),
                            false,
                        ));
                    }
                }
            }
        }
    }
    if !unknown.is_empty() {
        actual_coverage.push(RuleCoverage {
            rule_id: "native.unsupported".into(),
            scope: ValidationScope::Schema,
            subjects: unknown.into_iter().collect(),
            supported: false,
            reason: Some(crate::product_message!("backend.rules.reason_uncovered")),
        });
    }
    let status = if diagnostics
        .iter()
        .any(|issue| matches!(issue.severity, Severity::Error))
    {
        ValidationStatus::Failed
    } else if diagnostics
        .iter()
        .any(|issue| issue.code == "NATIVE_UNSUPPORTED")
    {
        ValidationStatus::Unsupported
    } else {
        ValidationStatus::Passed
    };
    Ok(ScopeValidation {
        scope: ValidationScope::Schema,
        status,
        coverage: actual_coverage,
        diagnostics,
    })
}
