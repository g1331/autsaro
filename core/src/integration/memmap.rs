//! Memory Mapping outputs for the RTE, integration configuration and SWC contracts.
use super::c_layout;
use super::{PlanDescription, PlanDiagnostic};

/// The same selected Dcm mode owner supplies its storage, code and placement alias.
pub(super) fn dcm_scope(composition: &super::composition::CompositionContract) -> &str {
    composition
        .components
        .iter()
        .find(|component| component.diagnostic_session_port.is_some())
        .map(|component| super::short_name(&component.component))
        .unwrap_or("Dcm")
}

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
    let receivers = super::rte_memory::receivers(composition);
    for component in &composition.components {
        let scope = super::short_name(&component.component);
        if !component.data_ports.is_empty()
            || component.operations.iter().any(|operation| operation.read)
            || component
                .runnables
                .iter()
                .any(|runnable| runnable.event.is_none())
        {
            sections.push((scope.into(), "CODE"));
        }
        sections.push((scope.into(), "CONST_UNSPECIFIED"));
        if receivers.iter().any(|receiver| receiver.scope == scope) {
            sections.push((scope.into(), "VAR_INIT_UNSPECIFIED"));
        }
        if receivers
            .iter()
            .any(|receiver| receiver.scope == scope && (receiver.received || receiver.expired))
        {
            sections.push((scope.into(), "VAR_CLEARED_UNSPECIFIED"));
        }
    }
    let mode_scope = dcm_scope(composition);
    sections.push((mode_scope.into(), "CODE"));
    sections.push((mode_scope.into(), "VAR_CLEARED_UNSPECIFIED"));
    sections.push((
        plan.ecu_communication()?.partition.name.clone(),
        "CALLOUT_CODE",
    ));
    // Integration configuration and RTE/SchM share RTE_<module> scopes. One header
    // owns every scope so a prefix has one unambiguous BSWMD artifact. Only the
    // allocations emitted for this configuration belong to its private table.
    for scope in [
        "Can",
        "CanIf",
        "Com",
        "PduR",
        "LSduR",
        "ComM",
        "BswM",
        "Det",
        "Ecu_HostBusSM",
        "Ecu",
    ] {
        sections.push((scope.into(), "CONFIG_DATA_PREBUILD_UNSPECIFIED"));
    }
    for scope in ["Com", "ComM", "Ecu"] {
        sections.push((scope.into(), "CODE"));
    }
    if plan.diagnostic_transport.is_some() {
        for scope in ["CanTp", "Dcm"] {
            sections.push((scope.into(), "CONFIG_DATA_PREBUILD_UNSPECIFIED"));
        }
        sections.push(("Dcm".into(), "CODE"));
    }
    sections.sort();
    sections.dedup();
    Ok(sections)
}

/// `code_section` places every code segment in one executable section while
/// keeping the source-scoped AUTOSAR keywords; `None` keeps per-scope sections.
pub(super) fn rte(
    scopes: &[(String, &str)],
    code_section: Option<&str>,
    mode_scope: &str,
) -> String {
    let mut mapping = String::from(
        "/** @file Source-derived RTE scopes; controlled x64 section placement. */\n#include \"MemMap_Version.h\"\n",
    );
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
        let mut entries = vec![(
            format!("RTE_{scope}_{segment}"),
            format!("__attribute__((section(\"{placement}\")))"),
        )];
        // Existing SchM mode functions consume this short placement keyword.
        // Other scopes and memory classes have no consumer for a generic alias.
        let mode_alias = scope == mode_scope && *segment == "CODE";
        if mode_alias {
            entries.push(("RTE_CODE".into(), format!("RTE_{scope}_{segment}")));
        }
        let macros = c_layout::defines(&entries);
        mapping.push_str(&mapping_branch(
            index == 0,
            &format!("RTE_{scope}"),
            "RTE",
            segment,
            &macros,
            "RTE",
        ));
        if mode_alias {
            mapping.push_str("#undef RTE_CODE\n");
        }
    }
    mapping.push_str(
        r#"#else
#error "Unknown RTE memory section; Rte_MemMap requires exactly one supported marker"
#endif
"#,
    );
    let requests = scopes
        .iter()
        .flat_map(|(scope, segment)| {
            ["START", "STOP"]
                .map(|boundary| format!("defined(RTE_{scope}_{boundary}_SEC_{segment})"))
        })
        .collect::<Vec<_>>();
    mapping.push_str(&c_layout::condition("if", &requests));
    mapping.push_str("#error \"Rte_MemMap requires exactly one supported marker\"\n#endif\n");
    mapping
}

/// The default SWC sections (SWS_MemMap_00027/00029, SWS_Rte_07194).
/// The controlled target places runnable code beside RTE code; ownership stays with the SWC.
pub(super) fn component(scope: &str) -> String {
    let mut output = String::from(
        "/** @file Default SWC sections for the controlled x64 target. */\n#include \"MemMap_Version.h\"\n",
    );
    for (index, (segment, placement)) in
        [("CODE", ".rte_code"), ("VAR_CLEARED_UNSPECIFIED", ".bss")]
            .iter()
            .enumerate()
    {
        let define = c_layout::defines(&[(
            format!("{scope}_{segment}"),
            format!("__attribute__((section(\"{placement}\")))"),
        )]);
        output.push_str(&mapping_branch(
            index == 0,
            scope,
            scope,
            segment,
            &define,
            "SWC",
        ));
    }
    output.push_str(&format!(
        "#else\n#error \"Unknown SWC memory section; {scope}_MemMap requires exactly one supported marker\"\n#endif\n"
    ));
    let requests = [
        "START_SEC_CODE",
        "STOP_SEC_CODE",
        "START_SEC_VAR_CLEARED_UNSPECIFIED",
        "STOP_SEC_VAR_CLEARED_UNSPECIFIED",
    ]
    .map(|request| format!("defined({scope}_{request})"));
    output.push_str(&c_layout::condition("if", &requests));
    output.push_str(&format!(
        "#error \"{scope}_MemMap requires exactly one supported marker\"\n#endif\n"
    ));
    output
}

/// RTE scopes share one active section; each SWC owns its own active section.
fn mapping_branch(
    first: bool,
    prefix: &str,
    owner: &str,
    segment: &str,
    definitions: &str,
    kind: &str,
) -> String {
    let directive = if first { "if" } else { "elif" };
    format!(
        r#"#{directive} defined({prefix}_START_SEC_{segment})
#undef {prefix}_START_SEC_{segment}
#ifdef {owner}_MEMMAP_ACTIVE
#error "Nested {kind} memory section"
#endif
#define {owner}_MEMMAP_ACTIVE
#define {prefix}_{segment}_ACTIVE
{definitions}#elif defined({prefix}_STOP_SEC_{segment})
#undef {prefix}_STOP_SEC_{segment}
#ifndef {prefix}_{segment}_ACTIVE
#error "Unmatched {kind} memory section"
#endif
#undef {prefix}_{segment}_ACTIVE
#undef {owner}_MEMMAP_ACTIVE
#undef {prefix}_{segment}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mode_code_alias_ends_at_its_own_stop_boundary() {
        let text = rte(
            &[("Modes".into(), "CODE"), ("Other".into(), "CODE")],
            Some(".rte_code"),
            "Modes",
        );
        assert!(text.contains("#define RTE_CODE"));
        assert!(text.contains(
            "#undef RTE_Modes_CODE\n#undef RTE_CODE\n#elif defined(RTE_Other_START_SEC_CODE)"
        ));
        assert_eq!(text.matches("#undef RTE_CODE\n").count(), 1);
        assert!(!component("Modes").contains("RTE_CODE"));
    }
}
