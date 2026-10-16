//! Memory Mapping outputs for the RTE, integration configuration and SWC contracts.
use super::c_layout;
use super::{PlanDescription, PlanDiagnostic};

/// RTE allocations shared by source generation and BSWMD memory descriptions.
pub(super) fn rte_sections(
    plan: &PlanDescription,
) -> Result<Vec<(String, &'static str)>, Vec<PlanDiagnostic>> {
    let composition = plan.ecu_composition()?;
    let mut sections = vec![
        ("Rte".into(), "CODE"),
        ("Rte".into(), "VAR_CLEARED_UNSPECIFIED"),
        ("Rte".into(), "CONFIG_DATA_PREBUILD_UNSPECIFIED"),
    ];
    for component in &composition.components {
        let scope = super::short_name(&component.component);
        sections.push((scope.into(), "CODE"));
        sections.push((scope.into(), "CONST_UNSPECIFIED"));
        sections.push((scope.into(), "VAR_CLEARED_UNSPECIFIED"));
        if component.data_ports.iter().any(|port| port.read) {
            sections.push((scope.into(), "VAR_INIT_UNSPECIFIED"));
        }
    }
    if !composition
        .components
        .iter()
        .any(|component| component.diagnostic_session_port.is_some())
    {
        sections.push(("Dcm".into(), "CODE"));
        sections.push(("Dcm".into(), "VAR_CLEARED_UNSPECIFIED"));
    }
    sections.push((
        plan.ecu_communication()?.partition.name.clone(),
        "CALLOUT_CODE",
    ));
    // Integration configuration and RTE/SchM share RTE_<module> scopes. One header
    // owns every scope so a prefix has one unambiguous BSWMD artifact.
    for scope in [
        "Can",
        "CanIf",
        "Com",
        "CanTp",
        "Dcm",
        "PduR",
        "LSduR",
        "ComM",
        "BswM",
        "Det",
        "Ecu_HostBusSM",
        "Ecu",
    ] {
        sections.push((scope.into(), "CONFIG_DATA_PREBUILD_UNSPECIFIED"));
        sections.push((scope.into(), "CODE"));
    }
    sections.sort();
    sections.dedup();
    Ok(sections)
}

/// `code_section` places every code segment in one executable section while
/// keeping the source-scoped AUTOSAR keywords; `None` keeps per-scope sections.
pub(super) fn rte(scopes: &[(String, &str)], code_section: Option<&str>) -> String {
    let mut mapping =
        String::from("/** @file Source-derived RTE scopes; controlled x64 section placement. */\n");
    for (index, (scope, segment)) in scopes.iter().enumerate() {
        let class = if *segment == "VAR_CLEARED_UNSPECIFIED" {
            "VAR_CLEARED"
        } else if *segment == "CONST_UNSPECIFIED" {
            "CONST"
        } else if *segment == "VAR_INIT_UNSPECIFIED" {
            "VAR_INIT"
        } else if *segment == "CONFIG_DATA_PREBUILD_UNSPECIFIED" {
            "CONFIG_DATA_PREBUILD"
        } else {
            "CODE"
        };
        let section_name = if class == "VAR_CLEARED" {
            "bss"
        } else if class == "VAR_INIT" {
            "data"
        } else if class == "CONST" || class == "CONFIG_DATA_PREBUILD" {
            "data.rel.ro"
        } else {
            "text"
        };
        let placement = match code_section {
            Some(section) if class == "CODE" => section.to_owned(),
            _ => format!(".{section_name}.rte.{scope}.{segment}"),
        };
        let macros = c_layout::defines(&[
            (
                format!("RTE_{scope}_{segment}"),
                format!("__attribute__((section(\"{placement}\")))"),
            ),
            (format!("RTE_{class}"), format!("RTE_{scope}_{segment}")),
        ]);
        mapping.push_str(&format!("#{} defined(RTE_{scope}_START_SEC_{segment})\n#undef RTE_{scope}_START_SEC_{segment}\n#ifdef RTE_MEMMAP_ACTIVE\n#error \"Nested RTE memory section\"\n#endif\n#define RTE_MEMMAP_ACTIVE\n#define RTE_{scope}_{segment}_ACTIVE\n{macros}#elif defined(RTE_{scope}_STOP_SEC_{segment})\n#undef RTE_{scope}_STOP_SEC_{segment}\n#ifndef RTE_{scope}_{segment}_ACTIVE\n#error \"Unmatched RTE memory section\"\n#endif\n#undef RTE_{scope}_{segment}_ACTIVE\n#undef RTE_MEMMAP_ACTIVE\n#undef RTE_{scope}_{segment}\n#undef RTE_{class}\n", if index == 0 { "if" } else { "elif" }));
    }
    mapping.push_str("#else\n#error \"Unknown RTE memory section\"\n#endif\n");
    mapping
}

/// The default SWC sections (SWS_MemMap_00027/00029, SWS_Rte_07194).
/// The controlled target places runnable code beside RTE code; ownership stays with the SWC.
pub(super) fn component(scope: &str) -> String {
    let mut output =
        String::from("/** @file Default SWC sections for the controlled x64 target. */\n");
    for (index, (segment, placement)) in
        [("CODE", ".rte_code"), ("VAR_CLEARED_UNSPECIFIED", ".bss")]
            .iter()
            .enumerate()
    {
        let define = c_layout::defines(&[(
            format!("{scope}_{segment}"),
            format!("__attribute__((section(\"{placement}\")))"),
        )]);
        output.push_str(&format!("#{} defined({scope}_START_SEC_{segment})\n#undef {scope}_START_SEC_{segment}\n#ifdef {scope}_MEMMAP_ACTIVE\n#error \"Nested SWC memory section\"\n#endif\n#define {scope}_MEMMAP_ACTIVE\n#define {scope}_{segment}_ACTIVE\n{define}#elif defined({scope}_STOP_SEC_{segment})\n#undef {scope}_STOP_SEC_{segment}\n#ifndef {scope}_{segment}_ACTIVE\n#error \"Unmatched SWC memory section\"\n#endif\n#undef {scope}_{segment}_ACTIVE\n#undef {scope}_MEMMAP_ACTIVE\n#undef {scope}_{segment}\n", if index == 0 { "if" } else { "elif" }));
    }
    output.push_str("#else\n#error \"Unknown SWC memory section\"\n#endif\n");
    output
}
