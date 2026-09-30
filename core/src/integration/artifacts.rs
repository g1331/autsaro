use super::catalog::SymbolContract;
use super::component::c_name;
use super::{ContractArgument, PlanDescription};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn artifact(path: &str) -> String {
    format!(
        "<SHORT-LABEL>{}</SHORT-LABEL><CATEGORY>{}</CATEGORY><DOMAIN>{}</DOMAIN>",
        xml(path.rsplit('/').next().unwrap()),
        if path.ends_with(".c") {
            "SWSRC"
        } else {
            "SWHDR"
        },
        xml(&path.rsplit_once('/').unwrap().0.replace('/', "."))
    )
}
fn argument(name: &str, native: &str, direction: &str, types: &BTreeMap<String, usize>) -> String {
    format!(
        r#"<SW-SERVICE-ARG><SHORT-NAME>{}</SHORT-NAME><CATEGORY>VALUE</CATEGORY><DIRECTION>{direction}</DIRECTION><SW-DATA-DEF-PROPS><SW-DATA-DEF-PROPS-VARIANTS><SW-DATA-DEF-PROPS-CONDITIONAL><BASE-TYPE-REF DEST="SW-BASE-TYPE">/HostArtifacts/Native{}</BASE-TYPE-REF></SW-DATA-DEF-PROPS-CONDITIONAL></SW-DATA-DEF-PROPS-VARIANTS></SW-DATA-DEF-PROPS></SW-SERVICE-ARG>"#,
        xml(name),
        types[native]
    )
}
fn rte_symbol(name: &str, return_type: &str, args: &[(&str, &str, &str)]) -> SymbolContract {
    SymbolContract {
        symbol: name.into(),
        return_type: return_type.into(),
        arguments: args
            .iter()
            .map(|(name, native_type, direction)| ContractArgument {
                name: (*name).into(),
                native_type: (*native_type).into(),
                direction: (*direction).into(),
            })
            .collect(),
        declaration_owner: "generated RTE headers".into(),
        definition_owner: "src/Rte.c".into(),
        consumers: Vec::new(),
    }
}

/// Product implementation descriptions reference only the sealed project's actual files.
/// They describe this host profile, not an ICC3 or complete standard API certification.
pub(super) fn files(
    plan: &PlanDescription,
    sources: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut entries: BTreeMap<String, Vec<SymbolContract>> = BTreeMap::new();
    for symbol in &plan.symbols {
        if let Some(path) = symbol.definition_owner.strip_prefix("runtime/src/") {
            entries
                .entry(path.trim_end_matches(".c").into())
                .or_default()
                .push(symbol.clone());
        } else if symbol.definition_owner.contains("generated Rte.c")
            || symbol
                .definition_owner
                .contains("generated Dcm service binding")
            || symbol.definition_owner.contains("BSW signature adapter")
        {
            entries
                .entry("Rte".into())
                .or_default()
                .push(symbol.clone());
        }
    }
    let rte = entries.entry("Rte".into()).or_default();
    rte.push(rte_symbol("Ecu_TargetInitializeRte", "uint8_t", &[]));
    rte.push(rte_symbol(
        "Ecu_TargetReadDid",
        "Std_ReturnType",
        &[("data", "uint8_t *", "OUT")],
    ));
    let counter = c_name(plan.schedule.counter.rsplit('/').next().unwrap());
    rte.push(rte_symbol(
        &format!("OsService_{counter}_GetCounterValue"),
        "StatusType",
        &[
            ("CounterID", "CounterType", "IN"),
            ("Value", "TimeInMicrosecondsType *", "OUT"),
        ],
    ));
    rte.push(rte_symbol(
        &format!("OsService_{counter}_GetElapsedValue"),
        "StatusType",
        &[
            ("CounterID", "CounterType", "IN"),
            ("Value", "TimeInMicrosecondsType *", "INOUT"),
            ("ElapsedValue", "TimeInMicrosecondsType *", "OUT"),
        ],
    ));
    rte.push(rte_symbol(
        "Rte_Call_OsService_GetCounterValue",
        "StatusType",
        &[("Value", "TimeInMicrosecondsType *", "OUT")],
    ));
    rte.push(rte_symbol(
        "Rte_Call_OsService_GetElapsedValue",
        "StatusType",
        &[
            ("Value", "TimeInMicrosecondsType *", "INOUT"),
            ("ElapsedValue", "TimeInMicrosecondsType *", "OUT"),
        ],
    ));
    let natives: BTreeSet<_> = entries
        .values()
        .flatten()
        .flat_map(|s| {
            std::iter::once(s.return_type.clone())
                .chain(s.arguments.iter().map(|a| a.native_type.clone()))
        })
        .filter(|t| t != "void")
        .collect();
    let types: BTreeMap<_, _> = natives
        .into_iter()
        .enumerate()
        .map(|(i, t)| (t, i))
        .collect();
    let mut elements = String::new();
    for (native, i) in &types {
        // Base types retain their C object size; array parameters decay only in the signature.
        let bits = if native.contains('*') || matches!(native.as_str(), "size_t" | "uint64_t") {
            64
        } else if matches!(native.as_str(), "Std_ReturnType" | "uint8_t" | "StatusType") {
            8
        } else if matches!(native.as_str(), "Com_SignalIdType" | "uint16_t") {
            16
        } else if matches!(
            native.as_str(),
            "EcuStatus" | "uint32" | "uint32_t" | "CounterType"
        ) {
            32
        } else if native
            == &c_name(
                plan.component
                    .service
                    .array_type
                    .rsplit('/')
                    .next()
                    .unwrap(),
            )
        {
            plan.component.service.array_length * 8
        } else {
            return Err(format!("No target C object size is defined for {native}"));
        };
        write!(elements, "<SW-BASE-TYPE><SHORT-NAME>Native{i}</SHORT-NAME><CATEGORY>FIXED_LENGTH</CATEGORY><BASE-TYPE-SIZE>{bits}</BASE-TYPE-SIZE><BASE-TYPE-ENCODING>NONE</BASE-TYPE-ENCODING><NATIVE-DECLARATION>{}</NATIVE-DECLARATION></SW-BASE-TYPE>", xml(native)).unwrap();
    }
    let mut docs = String::from(
        "# Generated host module implementation\n\nThis is the selected single-core Windows x64 C99 profile. The R24-11 BSW/RTE description locates delivered code and selected actual contracts; it does not certify a complete standard module API or MISRA compliance. Unassigned vendor identity is 0. Host implementation version follows this workbench; Arti uses its actual 1.0.0 version API. Source paths are relative to this project. Engineering-object SHORT-LABEL is the filename; DOMAIN maps directory components with dots (os.src means os/src).\n\n| Module | Delivered code and headers |\n| --- | --- |\n",
    );
    for module in [
        "Can", "CanIf", "PduR", "Com", "CanTp", "Dcm", "Dem", "NvM", "LSduR", "Security", "Os",
        "Arti", "Rte",
    ] {
        let paths: Vec<_> = sources
            .keys()
            .filter(|p| {
                if module == "Os" {
                    (p.starts_with("os/src/") && !p.ends_with("/Arti.c"))
                        || (p.starts_with("os/include/") && !p.ends_with("/Arti.h"))
                        || p.as_str() == "os/Os.h"
                } else if module == "Arti" {
                    p.as_str() == "os/src/Arti.c" || p.as_str() == "os/include/Arti.h"
                } else if module == "Rte" {
                    p.starts_with("include/Rte")
                        || p.as_str() == "src/Rte.c"
                        || p.as_str() == "src/Rte_OsService.c"
                } else {
                    p.as_str() == format!("src/{module}.c")
                        || p.as_str() == format!("include/{module}.h")
                        || (module == "Can" && p.as_str() == "include/SchM_Can.h")
                }
            })
            .cloned()
            .collect();
        if !paths.iter().any(|p| p.ends_with(".c")) {
            return Err(format!("No delivered C implementation for {module}"));
        }
        let contracts = entries.get(module).map(Vec::as_slice).unwrap_or(&[]);
        let mut refs = String::new();
        let mut entities = String::new();
        let mut events = String::new();
        for entry in contracts {
            let name = &entry.symbol;
            let mut fields = String::new();
            if entry.return_type != "void" {
                fields.push_str("<RETURN-TYPE>");
                fields.push_str(
                    argument("result", &entry.return_type, "OUT", &types)
                        .strip_prefix("<SW-SERVICE-ARG>")
                        .unwrap()
                        .strip_suffix("</SW-SERVICE-ARG>")
                        .unwrap(),
                );
                fields.push_str("</RETURN-TYPE>");
            }
            if !entry.arguments.is_empty() {
                fields.push_str("<ARGUMENTS>");
                for a in &entry.arguments {
                    fields.push_str(&argument(&a.name, &a.native_type, &a.direction, &types));
                }
                fields.push_str("</ARGUMENTS>");
            }
            let scheduled = plan
                .schedule
                .entities
                .iter()
                .find(|entity| !entity.application && entity.symbol == *name);
            let call_type = if scheduled.is_some() {
                "SCHEDULED"
            } else {
                "REGULAR"
            };
            let entity_type = if scheduled.is_some() {
                "BSW-SCHEDULABLE-ENTITY"
            } else {
                "BSW-CALLED-ENTITY"
            };
            write!(elements, "<BSW-MODULE-ENTRY><SHORT-NAME>{name}</SHORT-NAME><IS-REENTRANT>false</IS-REENTRANT><IS-SYNCHRONOUS>true</IS-SYNCHRONOUS><CALL-TYPE>{call_type}</CALL-TYPE><EXECUTION-CONTEXT>UNSPECIFIED</EXECUTION-CONTEXT>{fields}</BSW-MODULE-ENTRY>").unwrap();
            write!(refs, r#"<BSW-MODULE-ENTRY-REF-CONDITIONAL><BSW-MODULE-ENTRY-REF DEST="BSW-MODULE-ENTRY">/HostArtifacts/{name}</BSW-MODULE-ENTRY-REF></BSW-MODULE-ENTRY-REF-CONDITIONAL>"#).unwrap();
            write!(entities, r#"<{entity_type}><SHORT-NAME>{name}Entity</SHORT-NAME><IMPLEMENTED-ENTRY-REF DEST="BSW-MODULE-ENTRY">/HostArtifacts/{name}</IMPLEMENTED-ENTRY-REF></{entity_type}>"#).unwrap();
            if let Some(scheduled) = scheduled {
                let seconds = format!(
                    "{}.{:03}",
                    scheduled.period_ms / 1000,
                    scheduled.period_ms % 1000
                );
                write!(events, r#"<BSW-TIMING-EVENT><SHORT-NAME>{name}Timing</SHORT-NAME><STARTS-ON-EVENT-REF DEST="BSW-SCHEDULABLE-ENTITY">/HostArtifacts/{module}/HostBehavior/{name}Entity</STARTS-ON-EVENT-REF><PERIOD>{seconds}</PERIOD></BSW-TIMING-EVENT>"#).unwrap();
            }
        }
        write!(elements, "<BSW-MODULE-DESCRIPTION><SHORT-NAME>{module}</SHORT-NAME><CATEGORY>BSW_MODULE</CATEGORY><IMPLEMENTED-ENTRYS>{refs}</IMPLEMENTED-ENTRYS><INTERNAL-BEHAVIORS><BSW-INTERNAL-BEHAVIOR><SHORT-NAME>HostBehavior</SHORT-NAME><ENTITYS>{entities}</ENTITYS><EVENTS>{events}</EVENTS></BSW-INTERNAL-BEHAVIOR></INTERNAL-BEHAVIORS></BSW-MODULE-DESCRIPTION>").unwrap();
        let artifacts: String = paths
            .iter()
            .map(|p| {
                format!(
                    "<AUTOSAR-ENGINEERING-OBJECT>{}</AUTOSAR-ENGINEERING-OBJECT>",
                    artifact(p)
                )
            })
            .collect();
        let generated = if module == "Rte" {
            format!("<GENERATED-ARTIFACTS>{}</GENERATED-ARTIFACTS>", paths.iter().map(|p| format!("<DEPENDENCY-ON-ARTIFACT><SHORT-NAME>{}</SHORT-NAME><ARTIFACT-DESCRIPTOR>{}</ARTIFACT-DESCRIPTOR><USAGES><USAGE>COMPILE</USAGE></USAGES></DEPENDENCY-ON-ARTIFACT>", c_name(p), artifact(p))).collect::<String>())
        } else {
            String::new()
        };
        let resource = if module == "Os" || module == "Rte" {
            format!(
                "<RESOURCE-CONSUMPTION><SHORT-NAME>CodeMapping</SHORT-NAME><MEMORY-SECTIONS><MEMORY-SECTION><SHORT-NAME>CODE</SHORT-NAME><SYMBOL>CODE</SYMBOL></MEMORY-SECTION></MEMORY-SECTIONS></RESOURCE-CONSUMPTION>"
            )
        } else {
            String::new()
        };
        let version = if module == "Arti" {
            "1.0.0"
        } else {
            env!("CARGO_PKG_VERSION")
        };
        write!(elements, r#"<BSW-IMPLEMENTATION><SHORT-NAME>{module}HostImplementation</SHORT-NAME><CODE-DESCRIPTORS><CODE><SHORT-NAME>DeliveredCode</SHORT-NAME><ARTIFACT-DESCRIPTORS>{artifacts}</ARTIFACT-DESCRIPTORS></CODE></CODE-DESCRIPTORS>{generated}<PROGRAMMING-LANGUAGE>C</PROGRAMMING-LANGUAGE>{resource}<SW-VERSION>{}</SW-VERSION><USED-CODE-GENERATOR>Autosar host ECU generator</USED-CODE-GENERATOR><VENDOR-ID>0</VENDOR-ID><AR-RELEASE-VERSION>4.10.0</AR-RELEASE-VERSION><BEHAVIOR-REF DEST="BSW-INTERNAL-BEHAVIOR">/HostArtifacts/{module}/HostBehavior</BEHAVIOR-REF></BSW-IMPLEMENTATION>"#, version).unwrap();
        writeln!(
            docs,
            "| {module} | {} |",
            paths
                .iter()
                .map(|p| format!("`{p}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
    }
    docs.push_str("\nRTE initialization is the actual Ecu_TargetInitializeRte called by StartupHook; synchronous S/R and DID calls run on the one owner task. Rte_OsService forwards Counter calls to OS. RTE owns no independent static application data; Application.c owns its state. RTE CODE maps to .rte_code and OS Task/Hook CODE to .os_code. No configured SwAddrMethod hardware location override exists. Native host adapters (Ecu_Target, Ecu_Status, locks, storage and transport bridges) remain host helpers, not additional standard BSW modules. Original source identities are in integration.json; build.ps1 verifies files.list/files.sha256 before building a separate copy.\n");
    let description = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>HostArtifacts</SHORT-NAME><ELEMENTS>{elements}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>
"#
    );
    Ok(BTreeMap::from([
        (
            "descriptions/Host_Implementation.arxml".into(),
            description.into_bytes(),
        ),
        ("MODULES.md".into(), docs.into_bytes()),
    ]))
}
