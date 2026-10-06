use super::{DefinitionCatalog, NS, parent};
use crate::model::Severity;
use crate::project_model::{
    ConfigurationDiagnostic, DefinitionDescriptor, RuleCoverage, ScopeValidation, TypedValue,
    ValidationScope, ValidationStatus, ValidationWitness, ValueKind,
};
use roxmltree::{Document, Node};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(crate) fn identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn integer(raw: &str) -> Result<i128, String> {
    let (negative, raw) = match raw.strip_prefix('-') {
        Some(raw) => (true, raw),
        None => (false, raw.strip_prefix('+').unwrap_or(raw)),
    };
    let result = if let Some(raw) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        i128::from_str_radix(raw, 16)
    } else {
        raw.parse::<i128>()
    }
    .map_err(|_| "Value must be an exact integer lexeme".to_string())?;
    if negative {
        result
            .checked_neg()
            .ok_or_else(|| "Integer overflow".into())
    } else {
        Ok(result)
    }
}

pub(super) fn value(definition: &DefinitionDescriptor, value: &TypedValue) -> Result<(), String> {
    if definition.kind != Some(value.kind) {
        return Err("Value kind differs from the definition kind".into());
    }
    let raw = value.lexeme.trim();
    match value.kind {
        ValueKind::Integer => {
            let parsed = integer(raw)?;
            if definition
                .minimum
                .as_deref()
                .map(integer)
                .transpose()?
                .is_some_and(|min| parsed < min)
                || definition
                    .maximum
                    .as_deref()
                    .map(integer)
                    .transpose()?
                    .is_some_and(|max| parsed > max)
            {
                return Err("Integer lies outside its definition range".into());
            }
        }
        ValueKind::Float => {
            let parsed = raw
                .parse::<f64>()
                .map_err(|_| "Value must be a float lexeme")?;
            if !parsed.is_finite() {
                return Err("Nonfinite floating-point values are forbidden".into());
            }
            let bound = |raw: &str| {
                raw.parse::<f64>()
                    .map_err(|_| "Invalid metadata float bound".to_string())
            };
            if definition
                .minimum
                .as_deref()
                .map(bound)
                .transpose()?
                .is_some_and(|min| parsed < min)
                || definition
                    .maximum
                    .as_deref()
                    .map(bound)
                    .transpose()?
                    .is_some_and(|max| parsed > max)
            {
                return Err("Float lies outside its definition range".into());
            }
        }
        ValueKind::Boolean if !matches!(raw, "true" | "false" | "1" | "0") => {
            return Err("Boolean must be true, false, 1 or 0".into());
        }
        ValueKind::Enumeration if !definition.enumeration.iter().any(|literal| literal == raw) => {
            return Err("Value is not a declared enumeration literal".into());
        }
        ValueKind::FunctionName
            if !identifier(raw)
                || matches!(
                    raw,
                    "auto"
                        | "break"
                        | "case"
                        | "char"
                        | "const"
                        | "continue"
                        | "default"
                        | "do"
                        | "double"
                        | "else"
                        | "enum"
                        | "extern"
                        | "float"
                        | "for"
                        | "goto"
                        | "if"
                        | "inline"
                        | "int"
                        | "long"
                        | "register"
                        | "restrict"
                        | "return"
                        | "short"
                        | "signed"
                        | "sizeof"
                        | "static"
                        | "struct"
                        | "switch"
                        | "typedef"
                        | "union"
                        | "unsigned"
                        | "void"
                        | "volatile"
                        | "while"
                        | "_Bool"
                        | "_Complex"
                        | "_Imaginary"
                ) =>
        {
            return Err(
                "Function name must be a C identifier, not an expression or keyword".into(),
            );
        }
        ValueKind::Reference
            if !raw.starts_with('/') || raw.split('/').skip(1).any(|part| !identifier(part)) =>
        {
            return Err("Reference must be an absolute AUTOSAR object path".into());
        }
        _ => {}
    }
    Ok(())
}

fn child<'a, 'input>(node: Node<'a, 'input>, tag: &str) -> Option<Node<'a, 'input>> {
    node.children().find(|n| {
        n.is_element() && n.tag_name().namespace() == Some(NS) && n.tag_name().name() == tag
    })
}

fn text<'a, 'input>(node: Node<'a, 'input>, tag: &str) -> Option<std::borrow::Cow<'a, str>> {
    child(node, tag).map(super::xml_text_trimmed)
}

fn object(node: Node<'_, '_>) -> String {
    let mut names: Vec<_> = node
        .ancestors()
        .filter_map(|node| text(node, "SHORT-NAME"))
        .collect();
    names.reverse();
    format!("/{}", names.join("/"))
}

pub(super) fn diagnostic(
    file: Option<String>,
    path: Option<String>,
    code: &str,
    severity: Severity,
    message: String,
    remedy: &str,
    constraint: &str,
    counterexample: &str,
) -> ConfigurationDiagnostic {
    ConfigurationDiagnostic {
        scope: ValidationScope::Definition,
        rule_id: format!("native.definition.{code}"),
        severity,
        code: code.into(),
        message,
        remedy: remedy.into(),
        file,
        path,
        source_id: None,
        object_id: None,
        field_id: None,
        witness: Some(ValidationWitness {
            rule_id: format!("native.definition.{code}"),
            subjects: Vec::new(), // Workspace binds these to stable subject identities.
            constraint: constraint.into(),
            counterexample: counterexample.into(),
        }),
    }
}

pub(super) fn documents(
    catalog: &DefinitionCatalog,
    files: &[(&Path, &str)],
) -> Result<ScopeValidation, String> {
    let documents: Vec<_> = files
        .iter()
        .map(|(file, source)| {
            if source.len() > 50 * 1024 * 1024
                || source.contains("<!DOCTYPE")
                || source.contains("<!ENTITY")
            {
                return Err(format!(
                    "Unsafe definition validation source: {}",
                    file.display()
                ));
            }
            let document =
                Document::parse(source).map_err(|error| format!("{}: {error}", file.display()))?;
            if document.root_element().tag_name().name() != "AUTOSAR"
                || document.root_element().tag_name().namespace() != Some(NS)
            {
                return Err(format!(
                    "Definition validation requires AUTOSAR CP XML: {}",
                    file.display()
                ));
            }
            Ok(document)
        })
        .collect::<Result<_, _>>()?;
    let mut result = ScopeValidation {
        scope: ValidationScope::Definition,
        status: ValidationStatus::Passed,
        coverage: Vec::new(),
        diagnostics: Vec::new(),
    };
    let mut objects = BTreeMap::new();
    for (document, (file, _)) in documents.iter().zip(files) {
        for node in document
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().namespace() == Some(NS))
        {
            if text(node, "SHORT-NAME").is_some() && node.tag_name().name() != "AR-PACKAGE" {
                let path = object(node);
                if objects.insert(path.clone(), node).is_some() {
                    result.diagnostics.push(diagnostic(
                        Some(file.display().to_string()),
                        Some(path.clone()),
                        "OBJECT_DUPLICATE",
                        Severity::Error,
                        "Duplicate complete AUTOSAR object path".into(),
                        "Give each source object a unique full path.",
                        "unique object paths",
                        &path,
                    ));
                }
            }
        }
    }
    let mut module_counts: BTreeMap<&str, u32> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut references = BTreeSet::new();
    for (source_index, (document, (file, _))) in documents.iter().zip(files).enumerate() {
        for node in document
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().namespace() == Some(NS))
        {
            let Some(def_ref) = child(node, "DEFINITION-REF") else {
                continue;
            };
            let definition_text = super::xml_text_trimmed(def_ref);
            let id = definition_text.as_ref();
            let file = Some(file.display().to_string());
            let path = Some(object(node));
            let mut issue = |code: &str,
                             severity,
                             message: &str,
                             constraint: &str,
                             example: &str| {
                let mut issue = diagnostic(
                    file.clone(),
                    path.clone(),
                    code,
                    severity,
                    message.into(),
                    "Repair the value, structure or typed reference using its declared definition.",
                    constraint,
                    example,
                );
                if let Some(witness) = &mut issue.witness {
                    let range = node.range();
                    witness
                        .subjects
                        .push(format!("entry-range:{}:{}", range.start, range.end));
                    if let Some(target) = text(node, "VALUE-REF")
                        .filter(|target| objects.contains_key(target.as_ref()))
                    {
                        witness.subjects.push(target.into_owned());
                    }
                }
                result.diagnostics.push(issue);
            };
            let Some(definition) = catalog.get(id) else {
                issue(
                    "DEFINITION_UNKNOWN",
                    Severity::Warning,
                    "Definition is unavailable; this consumer is readonly",
                    "accepted definition identity",
                    id,
                );
                result.coverage.push(RuleCoverage {
                    rule_id: "native.definition.coverage".into(),
                    scope: ValidationScope::Definition,
                    subjects: vec![id.into()],
                    supported: false,
                    reason: Some("No explicitly accepted definition".into()),
                });
                continue;
            };
            let metadata = serde_json::to_string(definition).map_err(|error| error.to_string())?;
            if seen.insert(id.to_string()) {
                result.coverage.push(RuleCoverage {
                    rule_id: "native.definition.constraints".into(),
                    scope: ValidationScope::Definition,
                    subjects: vec![id.into()],
                    supported: definition.writable,
                    reason: definition.reason.clone(),
                });
            }
            let definition_refs = node
                .children()
                .filter(|child| {
                    child.is_element()
                        && child.tag_name().namespace() == Some(NS)
                        && child.tag_name().name() == "DEFINITION-REF"
                })
                .count();
            if definition_refs != 1 {
                issue(
                    "DEFINITION_CARDINALITY",
                    Severity::Error,
                    "Configuration entry requires one definition reference",
                    "1",
                    &definition_refs.to_string(),
                );
            }
            if def_ref.attribute("DEST") != Some(definition.element_kind.as_str()) {
                issue(
                    "DEFINITION_DEST",
                    Severity::Error,
                    "DEFINITION-REF DEST differs from the metadata kind",
                    &metadata,
                    def_ref.attribute("DEST").unwrap_or(""),
                );
            }
            let tag = node.tag_name().name();
            let expected = match definition.kind {
                None if definition.element_kind == "ECUC-MODULE-DEF" => {
                    "ECUC-MODULE-CONFIGURATION-VALUES"
                }
                None => "ECUC-CONTAINER-VALUE",
                Some(ValueKind::Integer | ValueKind::Float | ValueKind::Boolean) => {
                    "ECUC-NUMERICAL-PARAM-VALUE"
                }
                Some(ValueKind::Reference)
                    if definition.element_kind == "ECUC-INSTANCE-REFERENCE-DEF" =>
                {
                    "ECUC-INSTANCE-REFERENCE-VALUE"
                }
                Some(ValueKind::Reference) => "ECUC-REFERENCE-VALUE",
                Some(_) => "ECUC-TEXTUAL-PARAM-VALUE",
            };
            if tag != expected {
                issue(
                    "DEFINITION_KIND",
                    Severity::Error,
                    "Configuration entry kind differs from its definition",
                    expected,
                    tag,
                );
            }
            let wrapper = node
                .parent()
                .map(|parent| parent.tag_name().name())
                .unwrap_or("");
            let placement = match tag {
                "ECUC-MODULE-CONFIGURATION-VALUES" => wrapper == "ELEMENTS",
                "ECUC-CONTAINER-VALUE" => matches!(wrapper, "CONTAINERS" | "SUB-CONTAINERS"),
                "ECUC-REFERENCE-VALUE" | "ECUC-INSTANCE-REFERENCE-VALUE" => {
                    wrapper == "REFERENCE-VALUES"
                }
                _ => wrapper == "PARAMETER-VALUES",
            };
            if !placement {
                issue(
                    "DEFINITION_STRUCTURE",
                    Severity::Error,
                    "Entry is outside its required XML owner wrapper",
                    expected,
                    wrapper,
                );
            }
            if !definition.writable
                || node.children().any(|n| {
                    matches!(
                        n.tag_name().name(),
                        "VARIATION-POINT" | "VALUE-IREF" | "VALUE-EXPR"
                    )
                })
                || node
                    .children()
                    .filter(|child| matches!(child.tag_name().name(), "VALUE" | "VALUE-REF"))
                    .any(|value| value.children().any(|child| child.is_element()))
                || node.ancestors().skip(1).any(|owner| {
                    owner
                        .children()
                        .any(|n| n.tag_name().name() == "VARIATION-POINT")
                })
                || node.ancestors().any(|owner| {
                    text(owner, "ECUC-DEF-EDITION").is_some_and(|edition| edition != "4.10.0")
                })
            {
                issue(
                    "SEMANTICS_UNSUPPORTED",
                    Severity::Warning,
                    "Conditional, expression or instance-reference semantics are readonly",
                    &metadata,
                    id,
                );
                result.coverage.push(RuleCoverage {
                    rule_id: "native.definition.opaque".into(),
                    scope: ValidationScope::Definition,
                    subjects: vec![id.into()],
                    supported: false,
                    reason: Some("Unselected or unsupported value semantics".into()),
                });
                continue;
            }
            if definition.element_kind == "ECUC-MODULE-DEF" {
                let count = module_counts
                    .entry(definition.definition_id.as_str())
                    .or_default();
                *count += 1;
                if definition
                    .upper_multiplicity
                    .is_some_and(|upper| *count > upper)
                {
                    issue(
                        "MODULE_MULTIPLICITY",
                        Severity::Error,
                        "Module configuration multiplicity is invalid",
                        &metadata,
                        &count.to_string(),
                    );
                }
            }
            if let Some(owner) = node
                .ancestors()
                .skip(1)
                .find(|n| child(*n, "DEFINITION-REF").is_some())
            {
                let owner_text = text(owner, "DEFINITION-REF").unwrap_or_default();
                let owner_id = owner_text.as_ref();
                let mut expected_parent = parent(id);
                while expected_parent != Some(owner_id)
                    && expected_parent
                        .and_then(|id| catalog.get(id))
                        .is_some_and(|d| d.element_kind == "ECUC-CHOICE-CONTAINER-DEF")
                {
                    expected_parent = expected_parent.and_then(parent);
                }
                if expected_parent != Some(owner_id) {
                    issue(
                        "DEFINITION_PARENT",
                        Severity::Error,
                        "Entry is placed beneath the wrong definition owner",
                        parent(id).unwrap_or(""),
                        owner_id,
                    );
                }
            }
            if let Some(kind) = definition.kind {
                if kind == ValueKind::Reference {
                    let Some(reference) = child(node, "VALUE-REF") else {
                        issue(
                            "REFERENCE_MISSING",
                            Severity::Error,
                            "Ordinary reference has no VALUE-REF",
                            &metadata,
                            "absent",
                        );
                        continue;
                    };
                    let target_text = super::xml_text_trimmed(reference);
                    let dest = reference.attribute("DEST").unwrap_or("");
                    let owner = node
                        .ancestors()
                        .skip(1)
                        .find(|owner| child(*owner, "DEFINITION-REF").is_some())
                        .map(|owner| owner.range().start)
                        .unwrap_or(node.range().start);
                    let key = (
                        source_index,
                        owner,
                        definition.definition_id.as_str(),
                        target_text,
                        dest,
                    );
                    let raw = key.3.as_ref();
                    if references.contains(&key) {
                        issue(
                            "REFERENCE_DUPLICATE",
                            Severity::Error,
                            "A repeated reference duplicates the same target",
                            &metadata,
                            dest,
                        );
                    }
                    if !definition
                        .reference_destinations
                        .iter()
                        .any(|allowed| allowed == dest)
                    {
                        issue(
                            "REFERENCE_DESTINATION",
                            Severity::Error,
                            "Reference DEST is not allowed by its definition",
                            &metadata,
                            dest,
                        );
                    }
                    match objects.get(raw) {
                        None => issue(
                            "REFERENCE_UNRESOLVED",
                            Severity::Error,
                            "Reference target is absent from this source set",
                            &metadata,
                            raw,
                        ),
                        Some(target) => {
                            if target.tag_name().name() != dest {
                                issue(
                                    "REFERENCE_DEST",
                                    Severity::Error,
                                    "Reference DEST differs from the actual target kind",
                                    target.tag_name().name(),
                                    dest,
                                );
                            }
                            if let Some(targets) = catalog.targets.get(id) {
                                let target_id = text(*target, "DEFINITION-REF").unwrap_or_default();
                                if !targets.iter().any(|allowed| allowed == target_id.as_ref()) {
                                    issue(
                                        "REFERENCE_TARGET",
                                        Severity::Error,
                                        "Target configuration definition is not allowed",
                                        &targets.join("|"),
                                        &target_id,
                                    );
                                }
                            }
                        }
                    }
                    references.insert(key);
                } else {
                    let values: Vec<_> = node
                        .children()
                        .filter(|n| n.tag_name().name() == "VALUE")
                        .collect();
                    if values.len() != 1 {
                        issue(
                            "VALUE_CARDINALITY",
                            Severity::Error,
                            "Parameter requires exactly one explicit VALUE",
                            "1",
                            &values.len().to_string(),
                        );
                    } else {
                        let typed = TypedValue {
                            kind,
                            lexeme: super::xml_text(values[0]).into_owned(),
                        };
                        if let Err(error) = value(definition, &typed) {
                            issue(
                                "VALUE_RANGE",
                                Severity::Error,
                                &error,
                                &metadata,
                                &typed.lexeme,
                            );
                        }
                    }
                }
            } else {
                let mut counts: BTreeMap<std::borrow::Cow<'_, str>, u32> = BTreeMap::new();
                let mut opaque_entries = BTreeSet::new();
                for descendant in node
                    .descendants()
                    .skip(1)
                    .filter(|n| child(*n, "DEFINITION-REF").is_some())
                {
                    if descendant
                        .ancestors()
                        .skip(1)
                        .find(|n| child(*n, "DEFINITION-REF").is_some())
                        != Some(node)
                    {
                        continue;
                    }
                    if let Some(id) = text(descendant, "DEFINITION-REF") {
                        *counts.entry(id.clone()).or_default() += 1;
                        if descendant
                            .children()
                            .any(|child| child.tag_name().name() == "VARIATION-POINT")
                        {
                            opaque_entries.insert(id.clone());
                        }
                        if let Some(choice) = parent(&id)
                            .filter(|parent| *parent != definition.definition_id)
                            .and_then(|parent| catalog.get(parent))
                            .filter(|d| d.element_kind == "ECUC-CHOICE-CONTAINER-DEF")
                        {
                            *counts
                                .entry(std::borrow::Cow::Borrowed(&choice.definition_id))
                                .or_default() += 1;
                            if opaque_entries.contains(id.as_ref()) {
                                opaque_entries
                                    .insert(std::borrow::Cow::Borrowed(&choice.definition_id));
                            }
                        }
                    }
                }
                for child in catalog.children(id) {
                    let count = counts
                        .get(child.definition_id.as_str())
                        .copied()
                        .unwrap_or(0);
                    if opaque_entries.contains(child.definition_id.as_str())
                        || (!child.writable && (count > 0 || child.lower_multiplicity > 0))
                    {
                        result.coverage.push(RuleCoverage {
                            rule_id: "native.definition.conditional-cardinality".into(),
                            scope: ValidationScope::Definition,
                            subjects: vec![child.definition_id.clone()],
                            supported: false,
                            reason: Some(
                                "Unselected or unsupported entry multiplicity cannot be certified"
                                    .into(),
                            ),
                        });
                        continue;
                    }
                    if count < child.lower_multiplicity
                        || child.upper_multiplicity.is_some_and(|max| count > max)
                    {
                        // A declared default is display metadata, never a synthesized
                        // explicit entry and never a waiver for required cardinality.
                        issue(
                            "MULTIPLICITY",
                            Severity::Error,
                            "Required or repeated entry cardinality is invalid",
                            &serde_json::to_string(child).map_err(|error| error.to_string())?,
                            &count.to_string(),
                        );
                    }
                }
                if definition.element_kind == "ECUC-CHOICE-CONTAINER-DEF"
                    && opaque_entries.is_empty()
                {
                    let count: u32 = counts.values().sum();
                    if count != 1 {
                        issue(
                            "CHOICE_CARDINALITY",
                            Severity::Error,
                            "A choice container must select exactly one child",
                            "1",
                            &count.to_string(),
                        );
                    }
                }
            }
        }
    }
    cross_constraints(&documents, files, &objects, &mut result);
    legacy_constraints(catalog, &documents, files, &objects, &mut result)?;
    result
        .coverage
        .sort_by(|a, b| (&a.rule_id, &a.subjects).cmp(&(&b.rule_id, &b.subjects)));
    if result
        .diagnostics
        .iter()
        .any(|issue| matches!(issue.severity, Severity::Error))
    {
        result.status = ValidationStatus::Failed;
    } else if result.coverage.iter().any(|coverage| !coverage.supported) {
        result.status = ValidationStatus::Unsupported;
    }
    Ok(result)
}

fn parameter<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<std::borrow::Cow<'a, str>> {
    node.descendants()
        .find(|n| text(*n, "DEFINITION-REF").is_some_and(|id| id.rsplit('/').next() == Some(name)))
        .and_then(|n| text(n, "VALUE"))
}

fn cross_constraints<'a, 'input>(
    documents: &[Document<'input>],
    files: &[(&Path, &str)],
    objects: &BTreeMap<String, Node<'a, 'input>>,
    result: &mut ScopeValidation,
) where
    'input: 'a,
{
    for (document, (file, _)) in documents.iter().zip(files) {
        for node in document
            .descendants()
            .filter(|n| text(*n, "DEFINITION-REF").is_some_and(|id| id.ends_with("/ComIPdu")))
        {
            result.coverage.push(RuleCoverage {
                rule_id: "native.definition.CAN_SIGNAL_OVERLAP".into(),
                scope: ValidationScope::Definition,
                subjects: vec!["/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu".into()],
                supported: true,
                reason: None,
            });
            let mut occupied: BTreeMap<i128, usize> = BTreeMap::new();
            let mut claims: Vec<(i128, i128, bool, Node<'a, 'input>, Option<usize>)> = Vec::new();
            let mut selected_signals = Vec::new();
            for reference in node.descendants().filter(|n| {
                text(*n, "DEFINITION-REF").is_some_and(|id| id.ends_with("/ComIPduSignalRef"))
            }) {
                let Some(signal) =
                    text(reference, "VALUE-REF").and_then(|path| objects.get(path.as_ref()))
                else {
                    continue;
                };
                if selected_signals.contains(signal) {
                    continue;
                }
                selected_signals.push(*signal);
                let Some((start, length)) = parameter(*signal, "ComBitPosition")
                    .and_then(|v| integer(&v).ok())
                    .zip(parameter(*signal, "ComBitSize").and_then(|v| integer(&v).ok()))
                else {
                    continue;
                };
                // A bounded CAN-oriented rule, not an unbounded iteration over malformed values.
                if !(0..=i128::from(u32::MAX)).contains(&start) || !(0..=64).contains(&length) {
                    continue;
                }
                let big_endian =
                    parameter(*signal, "ComSignalEndianness").as_deref() == Some("BIG_ENDIAN");
                let mut bit = start;
                let mut overlaps: Vec<(i128, i128, bool, Node<'a, 'input>, Vec<i128>)> = Vec::new();
                for _ in 0..length {
                    let head = occupied.insert(bit, claims.len());
                    let mut previous = head;
                    while let Some(index) = previous {
                        let (previous_start, previous_length, previous_endian, previous_node, next) =
                            claims[index];
                        if let Some(overlap) = overlaps
                            .iter_mut()
                            .find(|overlap| overlap.3 == previous_node)
                        {
                            overlap.4.push(bit);
                        } else {
                            overlaps.push((
                                previous_start,
                                previous_length,
                                previous_endian,
                                previous_node,
                                vec![bit],
                            ));
                        }
                        previous = next;
                    }
                    claims.push((start, length, big_endian, *signal, head));
                    bit = if big_endian {
                        if bit % 8 == 0 { bit + 15 } else { bit - 1 }
                    } else {
                        bit + 1
                    };
                }
                for (previous_start, previous_length, previous_endian, previous, bits) in overlaps {
                    let mut issue = diagnostic(
                        Some(file.display().to_string()),
                        Some(object(node)),
                        "CAN_SIGNAL_OVERLAP",
                        Severity::Error,
                        "Signals in one ComIPdu overlap".into(),
                        "Assign disjoint signal bit ranges.",
                        "disjoint bit ranges",
                        &format!(
                            "{previous_start}:{previous_length}:{start}:{length}:{previous_endian}:{big_endian}:{bits:?}"
                        ),
                    );
                    if let Some(witness) = &mut issue.witness {
                        witness.subjects.push(object(previous));
                        witness.subjects.push(object(*signal));
                        let range = reference.range();
                        witness
                            .subjects
                            .push(format!("entry-range:{}:{}", range.start, range.end));
                    }
                    result.diagnostics.push(issue);
                }
            }
        }
    }
}

fn legacy_constraints(
    catalog: &DefinitionCatalog,
    documents: &[Document<'_>],
    files: &[(&Path, &str)],
    objects: &BTreeMap<String, Node<'_, '_>>,
    result: &mut ScopeValidation,
) -> Result<(), String> {
    let modules: BTreeSet<_> = documents
        .iter()
        .flat_map(|document| document.descendants())
        .filter(|node| node.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES")
        .filter_map(|node| text(node, "DEFINITION-REF"))
        .collect();
    let standard = documents.iter().any(|document| {
        document.descendants().any(|node| {
            node.tag_name().name() == "SYSTEM"
                && text(node, "CATEGORY").as_deref() == Some("ECU_EXTRACT")
        })
    });
    let required: Vec<_> = [
        "Can", "CanIf", "CanTp", "Com", "Dcm", "EcuC", "Os", "PduR", "Rte",
    ]
    .iter()
    .map(|module| format!("/AUTOSAR/EcucDefs/{module}"))
    .collect();
    if !standard || !required.iter().all(|id| modules.contains(id.as_str())) {
        return Ok(());
    }
    let Some((graph_valid, issues)) = catalog.legacy_definition_constraints(
        documents
            .iter()
            .zip(files)
            .map(|(document, (file, _))| (file.to_str().unwrap_or("<non-UTF8-source>"), document)),
    ) else {
        return Ok(());
    };
    let registered = |code: &str| {
        matches!(
            code,
            "CONTROLLER_MAPPING"
                | "BAUDRATE_CONFLICT"
                | "PERIOD_ALARM_CONFLICT"
                | "PERIOD_SCHEDULE_TABLE_CONFLICT"
                | "PARAMETER_NOT_UNIQUE"
                | "INSTANCE_MAPPING"
                | "DIRECTION_CONFLICT"
                | "TIMEOUT_CONFLICT"
                | "DIAGNOSTIC_TIMING"
                | "DIAGNOSTIC_REFERENCE"
        )
    };
    let supported = graph_valid;
    result.coverage.push(RuleCoverage {
        rule_id: "native.definition.legacy-cross-module".into(),
        scope: ValidationScope::Definition,
        subjects: required,
        supported,
        reason: (!supported).then(|| {
            "Actual cross-module consumers contain unresolved or unqualified references".into()
        }),
    });
    let rates: Vec<_> = if issues.iter().any(|issue| issue.code == "BAUDRATE_CONFLICT") {
        documents
            .iter()
            .flat_map(|document| document.descendants())
            .filter(|node| node.tag_name().name() == "BAUDRATE")
            .map(super::xml_text_trimmed)
            .collect()
    } else {
        Vec::new()
    };
    for issue in issues.into_iter().filter(|issue| registered(&issue.code)) {
        let values: Vec<_> = issue
            .object
            .as_ref()
            .and_then(|path| objects.get(path))
            .into_iter()
            .flat_map(|node| node.descendants())
            .filter(|node| node.tag_name().name() == "VALUE")
            .map(super::xml_text_trimmed)
            .collect();
        let dependent_rates = if issue.code == "BAUDRATE_CONFLICT" {
            rates.as_slice()
        } else {
            &[]
        };
        let counterexample =
            serde_json::to_string(&(values, dependent_rates)).map_err(|error| error.to_string())?;
        let subjects: Vec<_> = issue
            .object
            .as_ref()
            .and_then(|path| objects.get(path))
            .into_iter()
            .flat_map(|node| node.descendants())
            .filter(|node| node.tag_name().name() == "VALUE-REF")
            .map(super::xml_text_trimmed)
            .filter(|path| objects.contains_key(path.as_ref()))
            .map(std::borrow::Cow::into_owned)
            .collect();
        let mut diagnostic = diagnostic(
            issue.file,
            issue.object,
            &issue.code,
            Severity::Error,
            issue.message,
            &issue.remedy,
            &issue.code,
            &counterexample,
        );
        if let Some(witness) = &mut diagnostic.witness {
            witness.subjects = subjects;
        }
        result.diagnostics.push(diagnostic);
    }
    Ok(())
}
