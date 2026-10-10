//! Bounded standard SWC editing vocabulary; XML remains the source of truth.
use super::{DefinitionCatalog, descriptor};
use crate::project_model::ValueKind;

pub(crate) fn parent_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "APPLICATION-SW-COMPONENT-TYPE"
        | "COMPOSITION-SW-COMPONENT-TYPE"
        | "SENDER-RECEIVER-INTERFACE"
        | "CLIENT-SERVER-INTERFACE"
        | "APPLICATION-PRIMITIVE-DATA-TYPE"
        | "IMPLEMENTATION-DATA-TYPE"
        | "DATA-TYPE-MAPPING-SET" => Some("AR-PACKAGE"),
        "SW-COMPONENT-PROTOTYPE" | "ASSEMBLY-SW-CONNECTOR" => Some("COMPOSITION-SW-COMPONENT-TYPE"),
        "SWC-INTERNAL-BEHAVIOR" => Some("APPLICATION-SW-COMPONENT-TYPE"),
        "RUNNABLE-ENTITY" | "TIMING-EVENT" | "OPERATION-INVOKED-EVENT" => {
            Some("SWC-INTERNAL-BEHAVIOR")
        }
        "VARIABLE-DATA-PROTOTYPE" => Some("SENDER-RECEIVER-INTERFACE"),
        "CLIENT-SERVER-OPERATION" => Some("CLIENT-SERVER-INTERFACE"),
        "ARGUMENT-DATA-PROTOTYPE" => Some("CLIENT-SERVER-OPERATION"),
        _ => None,
    }
}
pub(crate) fn child_group(kind: &str) -> Option<&'static str> {
    match kind {
        "APPLICATION-SW-COMPONENT-TYPE"
        | "COMPOSITION-SW-COMPONENT-TYPE"
        | "SENDER-RECEIVER-INTERFACE"
        | "CLIENT-SERVER-INTERFACE"
        | "APPLICATION-PRIMITIVE-DATA-TYPE"
        | "IMPLEMENTATION-DATA-TYPE"
        | "DATA-TYPE-MAPPING-SET" => Some("ELEMENTS"),
        "SW-COMPONENT-PROTOTYPE" => Some("COMPONENTS"),
        "ASSEMBLY-SW-CONNECTOR" => Some("CONNECTORS"),
        "SWC-INTERNAL-BEHAVIOR" => Some("INTERNAL-BEHAVIORS"),
        "RUNNABLE-ENTITY" => Some("RUNNABLES"),
        "TIMING-EVENT" | "OPERATION-INVOKED-EVENT" => Some("EVENTS"),
        "VARIABLE-DATA-PROTOTYPE" => Some("DATA-ELEMENTS"),
        "CLIENT-SERVER-OPERATION" => Some("OPERATIONS"),
        "ARGUMENT-DATA-PROTOTYPE" => Some("ARGUMENTS"),
        "P-PORT-PROTOTYPE" | "R-PORT-PROTOTYPE" => Some("PORTS"),
        _ => None,
    }
}
pub(crate) fn populate(
    catalog: &mut DefinitionCatalog,
) -> Result<(), crate::message::LocalizedText> {
    for kind in [
        "APPLICATION-SW-COMPONENT-TYPE",
        "SERVICE-SW-COMPONENT-TYPE",
        "COMPOSITION-SW-COMPONENT-TYPE",
        "SW-COMPONENT-PROTOTYPE",
        "ASSEMBLY-SW-CONNECTOR",
        "P-PORT-PROTOTYPE",
        "R-PORT-PROTOTYPE",
        "SWC-INTERNAL-BEHAVIOR",
        "RUNNABLE-ENTITY",
        "TIMING-EVENT",
        "OPERATION-INVOKED-EVENT",
        "VARIABLE-ACCESS",
        "SYNCHRONOUS-SERVER-CALL-POINT",
        "SENDER-RECEIVER-INTERFACE",
        "CLIENT-SERVER-INTERFACE",
        "VARIABLE-DATA-PROTOTYPE",
        "CLIENT-SERVER-OPERATION",
        "ARGUMENT-DATA-PROTOTYPE",
        "APPLICATION-PRIMITIVE-DATA-TYPE",
        "IMPLEMENTATION-DATA-TYPE",
        "DATA-TYPE-MAPPING-SET",
    ] {
        let mut entry = descriptor(kind.into(), kind, None);
        entry.upper_multiplicity = None;
        catalog.insert(entry, Vec::new())?;
    }
    for (owner, tag, kind, destinations) in [
        (
            "SW-COMPONENT-PROTOTYPE",
            "TYPE-TREF",
            ValueKind::Reference,
            "APPLICATION-SW-COMPONENT-TYPE SERVICE-SW-COMPONENT-TYPE",
        ),
        (
            "P-PORT-PROTOTYPE",
            "PROVIDED-INTERFACE-TREF",
            ValueKind::Reference,
            "SENDER-RECEIVER-INTERFACE CLIENT-SERVER-INTERFACE",
        ),
        (
            "R-PORT-PROTOTYPE",
            "REQUIRED-INTERFACE-TREF",
            ValueKind::Reference,
            "SENDER-RECEIVER-INTERFACE CLIENT-SERVER-INTERFACE",
        ),
        (
            "ASSEMBLY-SW-CONNECTOR",
            "PROVIDER-IREF/CONTEXT-COMPONENT-REF",
            ValueKind::Reference,
            "SW-COMPONENT-PROTOTYPE",
        ),
        (
            "ASSEMBLY-SW-CONNECTOR",
            "REQUESTER-IREF/CONTEXT-COMPONENT-REF",
            ValueKind::Reference,
            "SW-COMPONENT-PROTOTYPE",
        ),
        (
            "ASSEMBLY-SW-CONNECTOR",
            "PROVIDER-IREF/TARGET-P-PORT-REF",
            ValueKind::Reference,
            "P-PORT-PROTOTYPE",
        ),
        (
            "ASSEMBLY-SW-CONNECTOR",
            "REQUESTER-IREF/TARGET-R-PORT-REF",
            ValueKind::Reference,
            "R-PORT-PROTOTYPE",
        ),
        (
            "TIMING-EVENT",
            "START-ON-EVENT-REF",
            ValueKind::Reference,
            "RUNNABLE-ENTITY",
        ),
        ("TIMING-EVENT", "PERIOD", ValueKind::Float, ""),
        ("TIMING-EVENT", "OFFSET", ValueKind::Float, ""),
        (
            "OPERATION-INVOKED-EVENT",
            "START-ON-EVENT-REF",
            ValueKind::Reference,
            "RUNNABLE-ENTITY",
        ),
        (
            "OPERATION-INVOKED-EVENT",
            "CONTEXT-P-PORT-REF",
            ValueKind::Reference,
            "P-PORT-PROTOTYPE",
        ),
        (
            "OPERATION-INVOKED-EVENT",
            "TARGET-PROVIDED-OPERATION-REF",
            ValueKind::Reference,
            "CLIENT-SERVER-OPERATION",
        ),
        ("RUNNABLE-ENTITY", "SYMBOL", ValueKind::FunctionName, ""),
        (
            "RUNNABLE-ENTITY",
            "CAN-BE-INVOKED-CONCURRENTLY",
            ValueKind::Boolean,
            "",
        ),
        (
            "SWC-INTERNAL-BEHAVIOR",
            "SUPPORTS-MULTIPLE-INSTANTIATION",
            ValueKind::Boolean,
            "",
        ),
        (
            "SWC-INTERNAL-BEHAVIOR",
            "DATA-TYPE-MAPPING-REF",
            ValueKind::Reference,
            "DATA-TYPE-MAPPING-SET",
        ),
        (
            "VARIABLE-DATA-PROTOTYPE",
            "TYPE-TREF",
            ValueKind::Reference,
            "APPLICATION-PRIMITIVE-DATA-TYPE IMPLEMENTATION-DATA-TYPE",
        ),
        (
            "ARGUMENT-DATA-PROTOTYPE",
            "TYPE-TREF",
            ValueKind::Reference,
            "APPLICATION-PRIMITIVE-DATA-TYPE IMPLEMENTATION-DATA-TYPE",
        ),
        (
            "DATA-TYPE-MAPPING-SET",
            "APPLICATION-DATA-TYPE-REF",
            ValueKind::Reference,
            "APPLICATION-PRIMITIVE-DATA-TYPE",
        ),
        (
            "DATA-TYPE-MAPPING-SET",
            "IMPLEMENTATION-DATA-TYPE-REF",
            ValueKind::Reference,
            "IMPLEMENTATION-DATA-TYPE",
        ),
        (
            "IMPLEMENTATION-DATA-TYPE",
            "BASE-TYPE-REF",
            ValueKind::Reference,
            "SW-BASE-TYPE",
        ),
        (
            "VARIABLE-ACCESS",
            "PORT-PROTOTYPE-REF",
            ValueKind::Reference,
            "P-PORT-PROTOTYPE R-PORT-PROTOTYPE",
        ),
        (
            "VARIABLE-ACCESS",
            "TARGET-DATA-PROTOTYPE-REF",
            ValueKind::Reference,
            "VARIABLE-DATA-PROTOTYPE",
        ),
        (
            "SYNCHRONOUS-SERVER-CALL-POINT",
            "CONTEXT-R-PORT-REF",
            ValueKind::Reference,
            "R-PORT-PROTOTYPE",
        ),
        (
            "SYNCHRONOUS-SERVER-CALL-POINT",
            "TARGET-REQUIRED-OPERATION-REF",
            ValueKind::Reference,
            "CLIENT-SERVER-OPERATION",
        ),
    ] {
        let mut entry = descriptor(format!("{owner}#{tag}"), tag, Some(kind));
        entry.element_kind = match (owner, tag) {
            ("OPERATION-INVOKED-EVENT", "CONTEXT-P-PORT-REF" | "TARGET-PROVIDED-OPERATION-REF") => {
                format!("OPERATION-IREF/{tag}")
            }
            ("SWC-INTERNAL-BEHAVIOR", "DATA-TYPE-MAPPING-REF") => {
                format!("DATA-TYPE-MAPPING-REFS/{tag}")
            }
            ("DATA-TYPE-MAPPING-SET", _) => format!("DATA-TYPE-MAPS/DATA-TYPE-MAP/{tag}"),
            ("IMPLEMENTATION-DATA-TYPE", "BASE-TYPE-REF") => format!(
                "SW-DATA-DEF-PROPS/SW-DATA-DEF-PROPS-VARIANTS/SW-DATA-DEF-PROPS-CONDITIONAL/{tag}"
            ),
            _ => tag.into(),
        };
        entry.reference_destinations = destinations.split_whitespace().map(str::to_owned).collect();
        if kind == ValueKind::Float {
            entry.minimum = Some("0".into());
        }
        catalog.insert(entry, Vec::new())?;
    }
    for (owner, tag, literals) in [
        ("APPLICATION-PRIMITIVE-DATA-TYPE", "CATEGORY", "VALUE"),
        ("IMPLEMENTATION-DATA-TYPE", "CATEGORY", "VALUE ARRAY"),
        ("ARGUMENT-DATA-PROTOTYPE", "DIRECTION", "IN OUT INOUT"),
        (
            "ARGUMENT-DATA-PROTOTYPE",
            "SERVER-ARGUMENT-IMPL-POLICY",
            "USE-ARGUMENT-TYPE USE-VOID",
        ),
    ] {
        let mut entry = descriptor(format!("{owner}#{tag}"), tag, Some(ValueKind::Enumeration));
        entry.enumeration = literals.split_whitespace().map(str::to_owned).collect();
        catalog.insert(entry, Vec::new())?;
    }
    for owner in ["P-PORT-PROTOTYPE", "R-PORT-PROTOTYPE"] {
        for (tag, kind, destinations) in [
            (
                "DATA-ELEMENT-REF",
                ValueKind::Reference,
                "VARIABLE-DATA-PROTOTYPE",
            ),
            (
                "OPERATION-REF",
                ValueKind::Reference,
                "CLIENT-SERVER-OPERATION",
            ),
            ("VALUE", ValueKind::Integer, ""),
            ("ALIVE-TIMEOUT", ValueKind::Float, ""),
            ("HANDLE-NEVER-RECEIVED", ValueKind::Boolean, ""),
            ("HANDLE-TIMEOUT-TYPE", ValueKind::Enumeration, ""),
            ("QUEUE-LENGTH", ValueKind::Integer, ""),
        ] {
            let mut entry = descriptor(format!("{owner}#{tag}"), tag, Some(kind));
            entry.reference_destinations =
                destinations.split_whitespace().map(str::to_owned).collect();
            if matches!(kind, ValueKind::Integer | ValueKind::Float) {
                entry.minimum = Some("0".into());
            }
            if kind == ValueKind::Integer {
                entry.maximum = Some("4294967295".into());
            }
            if kind == ValueKind::Enumeration {
                entry.enumeration = vec!["NONE".into(), "REPLACE".into()];
            }
            catalog.insert(entry, Vec::new())?;
        }
    }
    Ok(())
}

pub(crate) fn rank(kind: &str, tag: &str) -> usize {
    crate::rules::child_rank(kind, tag)
}

pub(crate) fn descendant(mut kind: &str, parent: &str) -> bool {
    while let Some(ancestor) = parent_kind(kind) {
        if ancestor == parent {
            return true;
        }
        kind = ancestor;
    }
    false
}

pub(crate) fn direct_field(owner: &str, tag: &str) -> bool {
    if tag.contains('/') {
        return true;
    }
    match owner {
        "SW-COMPONENT-PROTOTYPE" => tag == "TYPE-TREF",
        "P-PORT-PROTOTYPE" => tag == "PROVIDED-INTERFACE-TREF",
        "R-PORT-PROTOTYPE" => tag == "REQUIRED-INTERFACE-TREF",
        "TIMING-EVENT" => matches!(tag, "START-ON-EVENT-REF" | "PERIOD" | "OFFSET"),
        "OPERATION-INVOKED-EVENT" => tag == "START-ON-EVENT-REF",
        "RUNNABLE-ENTITY" => matches!(tag, "SYMBOL" | "CAN-BE-INVOKED-CONCURRENTLY"),
        "SWC-INTERNAL-BEHAVIOR" => tag == "SUPPORTS-MULTIPLE-INSTANTIATION",
        "VARIABLE-DATA-PROTOTYPE" => tag == "TYPE-TREF",
        "ARGUMENT-DATA-PROTOTYPE" => matches!(
            tag,
            "TYPE-TREF" | "DIRECTION" | "SERVER-ARGUMENT-IMPL-POLICY"
        ),
        "APPLICATION-PRIMITIVE-DATA-TYPE" | "IMPLEMENTATION-DATA-TYPE" => tag == "CATEGORY",
        _ => false,
    }
}
