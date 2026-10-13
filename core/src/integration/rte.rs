//! Actual bounded RTE producers from the immutable component composition contract.
use super::composition::{Endpoint, data_symbol};
use super::{PlanDescription, PlanDiagnostic, ValidatedIntegrationPlan};
use std::collections::BTreeMap;

fn signal(plan: &PlanDescription, path: &str) -> Result<u32, Vec<PlanDiagnostic>> {
    plan.handles
        .iter()
        .find(|handle| handle.domain == "com_signal" && handle.path == path)
        .map(|handle| handle.handle)
        .ok_or_else(|| PlanDiagnostic::plan_premise(path))
}

fn section(source: &mut String, scope: &str, segment: &str, start: bool) {
    source.push_str(&format!(
        "#define RTE_{scope}_{}_SEC_{segment}\n#include \"Rte_MemMap.h\"\n",
        if start { "START" } else { "STOP" }
    ));
}

/// `code_section` places every code segment in one executable section while
/// keeping the source-scoped AUTOSAR keywords; `None` keeps per-scope sections.
pub(super) fn memory_map(scopes: &[(String, &str)], code_section: Option<&str>) -> String {
    let mut mapping =
        String::from("/** @file Source-derived RTE scopes; controlled x64 section placement. */\n");
    for (index, (scope, segment)) in scopes.iter().enumerate() {
        let class = if *segment == "VAR_CLEARED_UNSPECIFIED" {
            "VAR_CLEARED"
        } else if *segment == "CONFIG_DATA_PREBUILD_UNSPECIFIED" {
            "CONFIG_DATA_PREBUILD"
        } else {
            "CODE"
        };
        let section_name = if class == "VAR_CLEARED" {
            "bss"
        } else if class == "CONFIG_DATA_PREBUILD" {
            "data.rel.ro"
        } else {
            "text"
        };
        let placement = match code_section {
            Some(section) if class == "CODE" => section.to_owned(),
            _ => format!(".{section_name}.rte.{scope}.{segment}"),
        };
        mapping.push_str(&format!("#{} defined(RTE_{scope}_START_SEC_{segment})\n#undef RTE_{scope}_START_SEC_{segment}\n#ifdef RTE_MEMMAP_ACTIVE\n#error \"Nested RTE memory section\"\n#endif\n#define RTE_MEMMAP_ACTIVE\n#define RTE_{scope}_{segment}_ACTIVE\n#define RTE_{scope}_{segment} __attribute__((section(\"{placement}\")))\n#define RTE_{class} RTE_{scope}_{segment}\n#elif defined(RTE_{scope}_STOP_SEC_{segment})\n#undef RTE_{scope}_STOP_SEC_{segment}\n#ifndef RTE_{scope}_{segment}_ACTIVE\n#error \"Unmatched RTE memory section\"\n#endif\n#undef RTE_{scope}_{segment}_ACTIVE\n#undef RTE_MEMMAP_ACTIVE\n#undef RTE_{scope}_{segment}\n#undef RTE_{class}\n", if index == 0 { "if" } else { "elif" }));
    }
    mapping.push_str("#else\n#ifndef RTE_MEMMAP_HEADER_CHECK\n#define RTE_MEMMAP_HEADER_CHECK\ntypedef unsigned char Rte_MemMap_HeaderCheck;\n#endif\n#endif\n");
    mapping
}

pub(super) fn files(
    plan: &ValidatedIntegrationPlan,
) -> Result<BTreeMap<String, Vec<u8>>, Vec<PlanDiagnostic>> {
    let description = plan.description();
    let composition = description.ecu_composition()?;
    let mut source = String::from(
        "/** @file Generated RTE for the checked single-owner component graph. */\n#define RTE_CORE\n#include \"Rte_Main.h\"\n#include \"Rte_Com.h\"\n#include \"Com.h\"\n#include \"Ecu_Target.h\"\n",
    );
    for component in &composition.components {
        source.push_str(&format!(
            "#include \"{}\"\n",
            component.header.trim_start_matches("include/")
        ));
    }
    source.push_str("#define RTE_Rte_START_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\nstatic boolean rte_started RTE_Rte_VAR_CLEARED_UNSPECIFIED;\nstatic boolean rte_allocated RTE_Rte_VAR_CLEARED_UNSPECIFIED;\n");
    let mut states = BTreeMap::new();
    let mut initialization = String::new();
    let mut scopes = vec![
        ("Rte".to_string(), "VAR_CLEARED_UNSPECIFIED"),
        ("Rte".to_string(), "CODE"),
        ("Rte".to_string(), "CONFIG_DATA_PREBUILD_UNSPECIFIED"),
    ];
    if !composition
        .components
        .iter()
        .any(|component| component.diagnostic_session_port.is_some())
    {
        scopes.extend([
            ("Dcm".to_string(), "VAR_CLEARED_UNSPECIFIED"),
            ("Dcm".to_string(), "CODE"),
        ]);
    }
    section(&mut source, "Rte", "VAR_CLEARED_UNSPECIFIED", false);
    for component in &composition.components {
        let scope = super::short_name(&component.component);
        if component.data_ports.iter().any(|port| port.read)
            || component.diagnostic_session_port.is_some()
        {
            scopes.push((scope.to_string(), "VAR_CLEARED_UNSPECIFIED"));
        }
        scopes.push((scope.to_string(), "CODE"));
        section(&mut source, scope, "VAR_CLEARED_UNSPECIFIED", true);
        for port in component.data_ports.iter().filter(|port| port.read) {
            let endpoint = Endpoint {
                instance: component.instance.clone(),
                port: port.path.clone(),
                member: port.element.clone(),
            };
            let name = format!("rte_value_{}", states.len());
            let network = composition
                .network_endpoints
                .iter()
                .find(|network| network.endpoint == endpoint);
            source.push_str(&format!(
                "static uint32 {name} RTE_{scope}_VAR_CLEARED_UNSPECIFIED;\n"
            ));
            initialization.push_str(&format!("    {name} = {}u;\n", port.initial_value));
            if network.is_some() {
                source.push_str(&format!("static boolean {name}_received RTE_{scope}_VAR_CLEARED_UNSPECIFIED;\nstatic boolean {name}_expired RTE_{scope}_VAR_CLEARED_UNSPECIFIED;\n"));
                initialization.push_str(&format!(
                    "    {name}_received = FALSE;\n    {name}_expired = FALSE;\n"
                ));
            }
            states.insert(endpoint, name);
        }
        section(&mut source, scope, "VAR_CLEARED_UNSPECIFIED", false);
    }
    source.push_str("#define RTE_Rte_START_SEC_CODE\n#include \"Rte_MemMap.h\"\nRTE_Rte_CODE Std_ReturnType Rte_Start(void) {\n    if (Ecu_TargetCheckLifecycleContext(TRUE) != E_OK) { return RTE_E_LIMIT; }\n    if (rte_allocated == TRUE) { return RTE_E_LIMIT; }\n");
    source.push_str(&initialization);
    source.push_str("    rte_allocated = TRUE;\n    rte_started = TRUE;\n    return E_OK;\n}\nRTE_Rte_CODE Std_ReturnType Rte_Stop(void) {\n    if (Ecu_TargetCheckLifecycleContext(FALSE) != E_OK) { return RTE_E_LIMIT; }\n    rte_started = FALSE;\n    return E_OK;\n}\n");
    section(&mut source, "Rte", "CODE", false);
    for component in &composition.components {
        let scope = super::short_name(&component.component);
        section(&mut source, scope, "CODE", true);
        for port in &component.data_ports {
            let endpoint = Endpoint {
                instance: component.instance.clone(),
                port: port.path.clone(),
                member: port.element.clone(),
            };
            let network = composition
                .network_endpoints
                .iter()
                .find(|network| network.endpoint == endpoint);
            let symbol = data_symbol(component, port);
            if port.read {
                let name = &states[&endpoint];
                source.push_str(&format!("RTE_{scope}_CODE Std_ReturnType {symbol}(uint32 *data) {{\n    if (data == NULL_PTR) {{ return E_NOT_OK; }}\n    if (rte_started == FALSE) {{ return RTE_E_COM_STOPPED; }}\n    if (Ecu_TargetIsOwner() == 0) {{ return E_NOT_OK; }}\n"));
                if let Some(network) = network {
                    let id = signal(description, &network.transport.com_signal)?;
                    source.push_str(&format!("    {{\n        uint32 value = {name};\n        const uint8 status = Com_ReceiveSignal({id}u, &value);\n        if (status == COM_SERVICE_NOT_AVAILABLE) {{\n            *data = {name};\n            return RTE_E_COM_STOPPED;\n        }}\n        if (status != E_OK) {{ return E_NOT_OK; }}\n        if ({name}_received == FALSE) {{\n            *data = {name};\n            return RTE_E_NEVER_RECEIVED;\n        }}\n        {name} = value;\n        *data = value;\n        return ({name}_expired == TRUE) ? RTE_E_MAX_AGE_EXCEEDED : E_OK;\n    }}\n"));
                } else {
                    source.push_str(&format!("    *data = {name};\n    return E_OK;\n"));
                }
            } else {
                source.push_str(&format!("RTE_{scope}_CODE Std_ReturnType {symbol}(uint32 data) {{\n    if (rte_started == FALSE) {{ return RTE_E_COM_STOPPED; }}\n    if (Ecu_TargetIsOwner() == 0) {{ return E_NOT_OK; }}\n"));
                if let Some(network) = network {
                    let id = signal(description, &network.transport.com_signal)?;
                    source.push_str(&format!("    const uint8 status = Com_SendSignal({id}u, &data);\n    return (status == COM_SERVICE_NOT_AVAILABLE) ? RTE_E_COM_STOPPED : status;\n"));
                } else {
                    for connection in composition
                        .connections
                        .iter()
                        .filter(|connection| !connection.service && connection.provider == endpoint)
                    {
                        source
                            .push_str(&format!("    {} = data;\n", states[&connection.requester]));
                    }
                    source.push_str("    return E_OK;\n");
                }
            }
            source.push_str("}\n");
        }
        for operation in component
            .operations
            .iter()
            .filter(|operation| operation.read)
        {
            let connection = composition
                .connections
                .iter()
                .find(|connection| {
                    connection.service
                        && connection.requester.instance == component.instance
                        && connection.requester.port == operation.port
                        && connection.requester.member == operation.operation
                })
                .ok_or_else(|| PlanDiagnostic::plan_premise(&operation.operation))?;
            let provider = composition
                .components
                .iter()
                .find(|component| component.instance == connection.provider.instance)
                .ok_or_else(|| PlanDiagnostic::plan_premise(&connection.provider.instance))?;
            let server = provider
                .operations
                .iter()
                .find(|operation| {
                    !operation.read
                        && operation.port == connection.provider.port
                        && operation.operation == connection.provider.member
                })
                .ok_or_else(|| PlanDiagnostic::plan_premise(&connection.provider.member))?;
            let runnable = provider
                .runnables
                .iter()
                .find(|runnable| runnable.path == server.runnable)
                .ok_or_else(|| PlanDiagnostic::plan_premise(&server.runnable))?;
            let parameters = operation
                .arguments
                .iter()
                .map(|arg| format!("{} {}", arg.native_type, arg.name))
                .collect::<Vec<_>>()
                .join(", ");
            source.push_str(&format!("RTE_{scope}_CODE Std_ReturnType {}({}) {{\n    if (rte_started == FALSE) {{ return RTE_E_COM_STOPPED; }}\n    if (Ecu_TargetIsOwner() == 0) {{ return E_NOT_OK; }}\n", operation.implementation_symbol, if parameters.is_empty() { "void" } else { &parameters }));
            for argument in &operation.arguments {
                if argument.native_type.contains('*')
                    || composition
                        .array_types
                        .values()
                        .any(|name| name == &argument.native_type)
                {
                    source.push_str(&format!(
                        "    if ({} == NULL_PTR) {{ return E_NOT_OK; }}\n",
                        argument.name
                    ));
                }
            }
            let names = operation
                .arguments
                .iter()
                .map(|arg| arg.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            source.push_str(&format!(
                "    {}({names});\n    return E_OK;\n}}\n",
                runnable.symbol
            ));
        }
        for runnable in component
            .runnables
            .iter()
            .filter(|runnable| runnable.event.is_none())
        {
            let operation = component
                .operations
                .iter()
                .find(|operation| operation.runnable == runnable.path)
                .ok_or_else(|| PlanDiagnostic::plan_premise(&runnable.path))?;
            let parameters = operation
                .arguments
                .iter()
                .map(|arg| format!("{} {}", arg.native_type, arg.name))
                .collect::<Vec<_>>()
                .join(", ");
            let names = operation
                .arguments
                .iter()
                .map(|arg| arg.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            source.push_str(&format!(
                "RTE_{scope}_CODE Std_ReturnType {}({}) {{\n    return {}({names});\n}}\n",
                runnable.symbol,
                if parameters.is_empty() {
                    "void"
                } else {
                    &parameters
                },
                operation.implementation_symbol
            ));
        }
        section(&mut source, scope, "CODE", false);
    }
    let partition = &description.ecu_communication()?.partition.name;
    scopes.push((partition.clone(), "CALLOUT_CODE"));
    section(&mut source, partition, "CALLOUT_CODE", true);
    let com = description.ecu_com()?;
    for (callback, timeout) in [("Rte_COMCbk", false), ("Rte_COMCbkRxTOut", true)] {
        source.push_str(&format!("RTE_{partition}_CALLOUT_CODE void {callback}(CbkHandleIdType handle) {{\n    if (rte_started == FALSE) {{ return; }}\n    if (Ecu_TargetIsOwner() == 0) {{ return; }}\n"));
        for reception in &com.receptions {
            for network in composition.network_endpoints.iter().filter(|network| {
                network.transport.receive && network.transport.com_signal == reception.signal
            }) {
                let name = &states[&network.endpoint];
                source.push_str(&format!(
                    "    if (handle == {}u) {{\n",
                    reception.callback_handle
                ));
                if timeout {
                    source.push_str(&format!(
                        "        if ({name}_received == TRUE) {{ {name}_expired = TRUE; }}\n"
                    ));
                } else {
                    let id = signal(description, &reception.signal)?;
                    source.push_str(&format!("        if (Com_ReceiveSignal({id}u, &{name}) == E_OK) {{\n            {name}_received = TRUE;\n            {name}_expired = FALSE;\n        }}\n"));
                }
                source.push_str("    }\n");
            }
        }
        source.push_str("}\n");
    }
    section(&mut source, partition, "CALLOUT_CODE", false);
    Ok(BTreeMap::from([
        // The delivered targets place all RTE code in their executable RTE section.
        ("include/Rte_MemMap.h".into(), memory_map(&scopes, Some(".rte_code")).into_bytes()),
        ("src/Rte.c".into(), source.into_bytes()),
        ("include/Rte_Main.h".into(), b"/** @file RTE trusted-context lifecycle; Start follows BSW/SchM Init. */\n#ifndef RTE_MAIN_H\n#define RTE_MAIN_H\n#include \"Std_Types.h\"\nStd_ReturnType Rte_Start(void);\nStd_ReturnType Rte_Stop(void);\ntypedef struct { uint16 partition; } SchM_ConfigType;\nvoid SchM_Init(const SchM_ConfigType *config);\nvoid SchM_Start(void);\nvoid SchM_StartTiming(void);\nvoid SchM_Deinit(void);\nextern const SchM_ConfigType Ecu_SchMConfig;\nboolean Ecu_SchMReady(void);\nboolean Ecu_SchMTimingActive(void);\n#endif\n".to_vec()),
        ("include/Rte_Com.h".into(), b"/** @file Handle-bearing COM reception and timeout notifications. */\n#ifndef RTE_COM_H\n#define RTE_COM_H\n#include \"ComStack_Types.h\"\nvoid Rte_COMCbk(CbkHandleIdType handle);\nvoid Rte_COMCbkRxTOut(CbkHandleIdType handle);\n#endif\n".to_vec()),
    ]))
}
