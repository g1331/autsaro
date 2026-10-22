//! Integrator manuals consume the same selected contracts and definition metadata as generation.
use super::{PlanDescription, PlanDiagnostic, catalog::SymbolContract};
use crate::project_model::ValueKind;
use std::collections::BTreeMap;

fn cell(value: &str) -> String {
    value
        .chars()
        .flat_map(|c| match c {
            '|' => "&#124;".chars().collect::<Vec<_>>(),
            '\n' | '\r' => " ".chars().collect(),
            '<' => "&lt;".chars().collect(),
            '>' => "&gt;".chars().collect(),
            _ => vec![c],
        })
        .collect()
}

fn overview(module: &str) -> (&'static str, &'static str) {
    match module {
        "Can" => (
            "Classical CAN controller and hardware-object access through the controlled host driver.",
            "One controller, contiguous controller/HOH IDs, 11-bit Classical CAN identifiers and 8-byte payloads. Polling functions must be bound to their configured time bases; this driver provides no physical MCU or transceiver.",
        ),
        "CanIf" => (
            "CAN controller/PDU-mode management and configured CAN frame routing.",
            "Controller, HRH/HTH, buffer and PDU references must resolve to the same selected driver. CAN identifiers, direction and lengths must agree with the referenced PDU. Unsupported optional APIs remain disabled.",
        ),
        "CanTp" => (
            "Bounded ISO-TP reception, transmission, cancellation and transport timers.",
            "Normal-addressing physical Classical CAN transport with consistent N-SDU/N-PDU handles, padding and time bases. Buffers and timeout relationships are checked before generation; no CAN FD or dynamic connection allocation.",
        ),
        "Com" => (
            "Configured signal packing, reception, deadline monitoring and periodic transmission.",
            "Signal widths, positions, endianness, initial values and I-PDU lengths must fit and agree with the system mappings. Reception deadlines apply only to Rx signals. The selected periodic Tx and Rx mains have explicit task/period mappings; unsupported transfer modes are rejected.",
        ),
        "Dcm" => (
            "Bounded DoCAN session control, TesterPresent and configured ReadDataByIdentifier services.",
            "Service tables, sessions, buffers, physical transport, P2/P2*/S3 and transport timers must form a supported connected configuration. Application DID bindings require matching runnable/data contracts. DCMServices SW-C ports remain tracked by #187.",
        ),
        "ComM" => (
            "Single CDD channel communication requests, diagnostic activity and minimum FULL hold time.",
            "NM variant NONE; every configured user maps to the selected channel. PNC, inhibition/NvM and unsupported notification features are rejected. Channel IDs, hold periods and BswM/host BusSM references must agree. SW-C user service ports remain tracked by #195.",
        ),
        "BswM" => (
            "Immediate ComM-mode arbitration and configured action lists.",
            "Only supported immediate requests, conditions and action lists are accepted. References, indices, initial modes and action targets must resolve. The delivered deferred main has no work in this all-IMMEDIATE configuration.",
        ),
        "PduR" => (
            "Configured IF/TP routes, routing groups and configuration identity.",
            "Source/destination module, API, direction, handle and PDU lengths must agree along each route. No zero-cost operation or metadata support. Routing group membership and default enable state are fixed at generation.",
        ),
        "LSduR" => (
            "Fixed target lower-layer dispatch between CanIf, PduR and CanTp.",
            "LSduR has a fixed target implementation contract derived from the validated route graph; an independent LSduR ECUC module is rejected. Its immutable route tables are generated with PduR.",
        ),
        "Det" => (
            "Development/runtime error hooks and the configured runtime callout.",
            "The selected error-hook/callout configuration is immutable while initialized. Hook counts and references must match the supported host contract; runtime callbacks run outside DET_STATE. No mode declaration group is provided.",
        ),
        "Os" => (
            "Single-core controlled host OS services, tasks, events, alarms, counters and schedule tables.",
            "Only the supported single-core OS configuration is accepted. Task/event/alarm and schedule-table mappings, priorities, activation limits, masks and counter bounds must agree. Interrupt source injection is a host adaptation; no physical interrupt controller or MCU timing is claimed.",
        ),
        "Arti" => (
            "ARTI access to the delivered OS object, class and hook descriptions.",
            "Descriptions refer to actual selected OS objects and storage. ARTI is enabled by the supported OS option and uses the fixed delivered ARTI version interface. Host tracing does not establish hardware debugger interoperability.",
        ),
        "Rte" => (
            "Configured RTE data/client-server interfaces, SchM lifecycle and Dcm mode publication.",
            "One OS owner executes ordinary RTE calls; callback paths obey their public contracts. Ports, types, runnable accesses, initialization policies and event/task positions must agree. Generation options outside the selected profile are rejected. Service-port support is limited to actual selected bindings.",
        ),
        _ => (
            "Controlled host CDD BusSM adapter for the selected ComM channel.",
            "A host substitute for a physical bus state manager; not a delivered CanSM or real network-management implementation. Its channel and callbacks follow the selected ComM plan.",
        ),
    }
}

pub(super) fn module(
    plan: &PlanDescription,
    module: &str,
    paths: &[String],
    entries: &[SymbolContract],
    comments: &BTreeMap<String, String>,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<String, Vec<PlanDiagnostic>> {
    let base = format!("generated/descriptions/modules/{module}");
    let (function, constraints) = overview(module);
    let version = if module == "Rte" {
        plan.ecu_composition()?.generation.software_version.as_str()
    } else if module == "Arti" {
        env!("ARTI_SOFTWARE_VERSION")
    } else {
        env!("AUTSARO_SOFTWARE_VERSION")
    };
    let version_scope = if module == "Rte" {
        format!(
            "BSWMD/configuration version above comes from source DOC-REVISION. RTE header software macros identify generator implementation {} (Autsaro_Version.h). The disabled SchM_GetVersionInfo API would require a build version depending on both generator and input configuration; neither value alone is that build version.\n\n",
            env!("AUTSARO_SOFTWARE_VERSION")
        )
    } else if module == "Arti" {
        "Software version is owned by Arti_Version.h and shared by Arti_GetVersionInfo and this description.\n\n".into()
    } else {
        "Software version is owned by Autsaro_Version.h and shared by module headers and this description, independently of the Cargo package version.\n\n".into()
    };
    let activation = if matches!(module, "Dcm" | "CanTp") && plan.diagnostic_transport.is_none() {
        "Diagnostic activation is disabled in this selected plan. Public implementation entries are delivered, but the generated bootstrap neither initializes nor schedules this module.\n\n"
    } else {
        ""
    };
    let mut text = format!(
        "# {module} module manual\n\nDocument: `{module}/README.md`; document revision: 1; date: 2026-10-18; status: generated selected implementation. Company/author: Autsaro project (no assigned AUTOSAR vendor identity). Implementation version: `{version}`; standard baseline: R24-11. Configuration and source identities: [integration.json](../../../../meta/integration.json). AUTOSAR release 4.10.0 is published by each header owner; applicable direct imports check major/minor. Arti uses standard module ID 5; vendor identity remains unassigned. Formal vendor identity and remaining API obligations remain [#59](https://github.com/g1331/autsaro/issues/59).\n\n[Overview](#overview) · [History](#history) · [Files](#files) · [Resources and integration](#resources-and-integration) · [Configuration](#configuration) · [API](#api) · [Requirements and limitations](#requirements-and-limitations)\n\n## Overview\n\n{version_scope}{function}\n\n{activation}## History\n\n| Document revision | Date | Company/author | Change | Status |\n| --- | --- | --- | --- | --- |\n| 1 | 2026-10-18 | Autsaro project | Generated manual for the selected implementation and configuration above | Generated |\n\nRegeneration replaces this manual; integration.json and the delivery seal record the selected source/configuration identities.\n\n## Files\n\n| File | Role |\n| --- | --- |\n"
    );
    for path in paths {
        let role = if path.ends_with(".h") {
            "Public/configuration header; consult its contract comments"
        } else if path.contains("PBcfg") || path.contains("RuntimeConfig") {
            "Selected configuration and bindings"
        } else {
            "Implementation or target adapter"
        };
        text.push_str(&format!("| [{path}](../../../../{path}) | {role} |\n"));
    }
    let area = super::artifacts::exclusive_area(module).unwrap_or("None declared");
    if module == "Ecu_HostBusSM" {
        text.push_str("\nThe host BusSM uses its private CAN_HOST_AREA_BUSSM host lock through Ecu_HostBusSM_Enter/Exit; it exports no SchM interlink header. The BSWMD records this actual adapter area without claiming a CanSM implementation.\n");
    }
    text.push_str(&format!("\n## Resources and integration\n\nExclusive area: `{area}`. Module areas serialize their own state, never nest or span cross-module callbacks. BSWMD canEnter relations describe actual entering entities; callout prototypes have no BswCalledEntity (constr_4016) and their locking contract remains in the public API comments; generated SchM headers are checked before delivery. OS resources/interrupt suspension are separate OS services. No physical MCU vendor/family/derivative/stepping is selected.\n\nThe target, compiler identity/version/options, source list and host libraries are recorded in [target.json](../../../../target.json); use the [normal offline build](../../../../README.md#independent-native-build). The delivered controlled x64 target uses C99 and its selected FreeRTOS port. RAM/ROM/stack footprint and hardware timing have not been qualified. Static buffers and module state are owned by the delivered implementation; API comments below specify borrowing and retained configuration.\n\nInitialize through the generated SchM lifecycle before normal calls; start periodic work only after configuration and application initialization. `Rte_Start`/`SchM_Start`/`SchM_StartTiming` authorize the owner-task release; stop through the matching lifecycle. [Ecu_RuntimeConfig.c](../../../src/Ecu_RuntimeConfig.c) is the executable initialization/API-use example with the actual configuration objects; [SchM_Main.c](../../../src/SchM_Main.c) implements the lifecycle. Callback APIs are invoked by their lower/upper layer, never substituted for initialization.\n\nWhen several scheduled entities are due on one task, execute their configured positions in order; different tasks have no invented global sequence. The complete selected schedule follows, including application dependencies:\n\n| Function | Task | Position | Period (ms) | Trigger |\n| --- | --- | --- | --- | --- |\n"));
    for entity in &plan.schedule.entities {
        text.push_str(&format!(
            "| `{}` | `{}` | {} | {} | `{}` |\n",
            entity.symbol,
            cell(&entity.task),
            entity.position,
            entity.period_ms,
            cell(entity.trigger())
        ));
    }
    text.push_str(&format!("\n## Configuration\n\n{constraints}\n\nEdit the live ECUC inputs and regenerate; do not edit sealed generated files. The parameter pages below are configuration examples from this exact accepted plan and metadata from the same DefinitionCatalog used by value validation. Bounds are definition bounds; connected profile constraints above also apply. An absent default means no default is declared, never an implicit zero. Omitted optional values retain the generator's explicitly supported behavior. Original ARXML examples are sealed in [inputs](../../../../inputs); references and all selected values also appear in integration.json.\n\n"));
    let catalog = crate::definitions::DefinitionCatalog::builtin().map_err(super::ecu::reject)?;
    let prefix = format!("/AUTOSAR/EcucDefs/{module}");
    let records: Vec<_> = plan
        .configuration
        .iter()
        .filter(|record| {
            record.definition == prefix || record.definition.starts_with(&format!("{prefix}/"))
        })
        .collect();
    let mut parameters = Vec::new();
    for record in &records {
        for (name, values) in record.parameters.iter().chain(record.references.iter()) {
            let id = name.clone();
            let name = super::short_name(name);
            let descriptor = catalog.get(&id);
            let description = descriptor.and_then(|entry| entry.description.as_ref()).map(ToString::to_string).unwrap_or_else(|| "Configured reference or integration value; resolve against the declared target type.".into());
            let range = descriptor
                .map(|entry| {
                    if entry.kind == Some(ValueKind::Boolean) {
                        "false, true (0, 1)".into()
                    } else if entry.kind == Some(ValueKind::Reference) || record.references.contains_key(&id) {
                        format!("Reference destinations: {}; target definitions: {}. Target must resolve and satisfy the connected module constraints",
                            entry.reference_destinations.join(", "), catalog.reference_target_definitions(&id).join(", "))
                    } else if !entry.enumeration.is_empty() {
                        entry.enumeration.join(", ")
                    } else if matches!(entry.kind, Some(ValueKind::Integer | ValueKind::Float)) {
                        format!(
                            "{} .. {}",
                            entry.minimum.as_deref().unwrap_or("unbounded"),
                            entry.maximum.as_deref().unwrap_or("unbounded")
                        )
                    } else {
                        format!("{:?}; must satisfy the definition type and connected module constraints", entry.kind)
                    }
                })
                .unwrap_or_else(|| "Validated connected target".into());
            let default = descriptor
                .and_then(|entry| entry.default_value.as_ref())
                .map(|value| value.lexeme.as_str())
                .unwrap_or("No declared default");
            let unit = descriptor
                .and_then(|entry| entry.unit.as_deref())
                .unwrap_or("—");
            let profile = if matches!(
                name,
                "CanMainFunctionPeriod"
                    | "CanMainFunctionModePeriod"
                    | "CanMainFunctionBusoffPeriod"
            ) {
                "Convert seconds to whole milliseconds; must equal the OS counter tick and each corresponding Can_MainFunction Read/Write, Mode or BusOff mapped entity period (CAN_POLLING_TIMEBASE)"
            } else if super::native_configuration::DISABLED_FEATURES
                .split_ascii_whitespace()
                .any(|field| field == name)
            {
                "Must be false/0; enabling is rejected"
            } else if super::configuration::RUNTIME_PARAMETERS
                .split_ascii_whitespace()
                .any(|field| field == name)
            {
                "Consumed by the selected runtime/configuration validator; must satisfy the connected module/profile constraints"
            } else if super::native_configuration::DESCRIPTOR_PARAMETERS
                .split_ascii_whitespace()
                .any(|field| field == name)
            {
                "Descriptor only; does not enable physical hardware"
            } else {
                "Must satisfy the connected module/profile constraints"
            };
            parameters.push(format!("### `{}` / `{name}`\n\nDefinition: `{id}`.\n\n{description}\n\nUnit: {}; definition range: {}; default: {}.\n\nSelected value/reference: `{}`. Profile rule: {profile}.\n\n", cell(&record.path), cell(unit), cell(&range), cell(default), cell(&values.join(", "))));
        }
    }
    for descriptor in catalog
        .definitions()
        .filter(|entry| entry.definition_id.starts_with(&format!("{prefix}/")))
    {
        if records
            .iter()
            .any(|record| record.parameters.contains_key(&descriptor.definition_id))
        {
            continue;
        }
        let name = super::short_name(&descriptor.definition_id);
        if super::native_configuration::DISABLED_FEATURES
            .split_ascii_whitespace()
            .any(|field| field == name)
        {
            let description = descriptor
                .description
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            parameters.push(format!("### `{name}` fixed profile option\n\nDefinition: `{}`.\n\n{description}\n\nUnit: none; definition range: false/true; profile range: false/0. Default: {}. Must be false/0; enabling is rejected. This capability is not implemented by the controlled profile.\n\n", descriptor.definition_id, descriptor.default_value.as_ref().map(|value| value.lexeme.as_str()).unwrap_or("No declared default")));
        }
    }
    if parameters.is_empty() {
        text.push_str("No independent parameter values are selected for this implementation. Its generated bindings follow the module constraints and connected plan above.\n");
    }
    for (index, chunk) in parameters.chunks(12).enumerate() {
        let file = format!("configuration-{}.md", index + 1);
        text.push_str(&format!(
            "- [Selected configuration {}]({file})\n",
            index + 1
        ));
        files.insert(format!("{base}/{file}"), format!("# {module} selected configuration {}\n\n[Module manual](README.md#configuration)\n\n{}", index + 1, chunk.concat()).into_bytes());
    }
    text.push_str("\n## API\n\nUse the exact public signatures below. Preconditions, call contexts, reentrancy restrictions, outputs and ownership come from the delivered public header comments. BSWMD records selected synchronous/reentrant flags; partial reentrancy permits different objects only. Internal/host adaptation APIs are identified by their public headers and are not additional standardized capabilities.\n\n");
    for (index, chunk) in entries.chunks(8).enumerate() {
        let file = format!("api-{}.md", index + 1);
        text.push_str(&format!("- [API contracts {}]({file})\n", index + 1));
        let mut api = format!(
            "# {module} API contracts {}\n\n[Module manual](README.md#api)\n\n",
            index + 1
        );
        for entry in chunk {
            let (reentrant, synchronous) = super::catalog::selected_behavior(&entry.symbol);
            let arguments = if entry.arguments.is_empty() {
                "void".into()
            } else {
                entry
                    .arguments
                    .iter()
                    .map(|arg| format!("{} {}", arg.native_type, arg.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            api.push_str(&format!("## `{}`\n\n```c\n{} {}({arguments});\n```\n\nBSWMD: reentrant={reentrant}, synchronous={synchronous}. Declaration: `{}`.\n\n{}\n\n", entry.symbol, entry.return_type, entry.symbol, entry.declaration_owner, comments.get(&entry.symbol).map(String::as_str).unwrap_or("Generated binding: consult its delivered public header for preconditions, context, ownership and return semantics; use the generated owner lifecycle and mapped task.")));
        }
        files.insert(format!("{base}/{file}"), api.into_bytes());
    }
    text.push_str("\n## Requirements and limitations\n\nThis manual documents the selected implementation; it does not approve specification/requirement deviations or assert complete AUTOSAR/MISRA compliance. The controlled host has no MCU, physical ISR timing, safety qualification or certification evidence. Source analyzer findings and the 221 manual MISRA assessments remain separately tracked by [#58](https://github.com/g1331/autsaro/issues/58); [#57](https://github.com/g1331/autsaro/issues/57) tracks original normative material gaps. [#59](https://github.com/g1331/autsaro/issues/59) retains unassigned vendor/version identity; [#187](https://github.com/g1331/autsaro/issues/187) and [#195](https://github.com/g1331/autsaro/issues/195) retain Dcm/ComM SW-C service boundaries. Module-specific requirement dispositions and target restrictions remain in the project's [standard baseline](https://github.com/g1331/autsaro/tree/master/docs/standards). No unsupported service is enabled by a description entry.\n");
    files.insert(format!("{base}/README.md"), text.into_bytes());
    Ok(format!("[{}](modules/{module}/README.md)", module))
}
