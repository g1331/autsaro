//! Actual bounded RTE producers from the immutable component composition contract.
use super::c_layout;
use super::composition::{Endpoint, data_symbol};
use super::{ContractArgument, PlanDescription, PlanDiagnostic, ValidatedIntegrationPlan};
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
        let macros = c_layout::defines(&[
            (
                format!("RTE_{scope}_{segment}"),
                format!("__attribute__((section(\"{placement}\")))"),
            ),
            (format!("RTE_{class}"), format!("RTE_{scope}_{segment}")),
        ]);
        mapping.push_str(&format!("#{} defined(RTE_{scope}_START_SEC_{segment})\n#undef RTE_{scope}_START_SEC_{segment}\n#ifdef RTE_MEMMAP_ACTIVE\n#error \"Nested RTE memory section\"\n#endif\n#define RTE_MEMMAP_ACTIVE\n#define RTE_{scope}_{segment}_ACTIVE\n{macros}#elif defined(RTE_{scope}_STOP_SEC_{segment})\n#undef RTE_{scope}_STOP_SEC_{segment}\n#ifndef RTE_{scope}_{segment}_ACTIVE\n#error \"Unmatched RTE memory section\"\n#endif\n#undef RTE_{scope}_{segment}_ACTIVE\n#undef RTE_MEMMAP_ACTIVE\n#undef RTE_{scope}_{segment}\n#undef RTE_{class}\n", if index == 0 { "if" } else { "elif" }));
    }
    mapping.push_str("#else\n#ifndef RTE_MEMMAP_HEADER_CHECK\n#define RTE_MEMMAP_HEADER_CHECK\ntypedef unsigned char Rte_MemMap_HeaderCheck;\n#endif\n#endif\n");
    mapping
}

/// The head of an RTE function definition in the component's code section.
fn function(scope: &str, symbol: &str, parameters: &[String]) -> String {
    c_layout::signature(
        &format!("RTE_{scope}_CODE Std_ReturnType"),
        symbol,
        parameters,
        "",
        true,
    )
}

/// Operation parameters, or `void` for none.
fn parameters(arguments: &[ContractArgument]) -> Vec<String> {
    if arguments.is_empty() {
        return vec!["void".into()];
    }
    arguments
        .iter()
        .map(|arg| c_layout::parameter(&arg.native_type, &arg.name))
        .collect()
}

fn names(arguments: &[ContractArgument]) -> Vec<String> {
    arguments.iter().map(|arg| arg.name.clone()).collect()
}

pub(super) fn files(
    plan: &ValidatedIntegrationPlan,
) -> Result<BTreeMap<String, Vec<u8>>, Vec<PlanDiagnostic>> {
    let description = plan.description();
    let composition = description.ecu_composition()?;
    let mut source = String::from(
        "/** @file Generated RTE for the checked single-owner component graph. */\n#define RTE_CORE\n#include \"Rte_Main.h\"\n#include \"Rte_Com.h\"\n#include \"Com.h\"\n#include \"Ecu_Target.h\"\n",
    );
    let diagnostic_types = description
        .diagnostic
        .as_ref()
        .map(|diagnostic| composition.diagnostic_types(diagnostic))
        .transpose()?;
    if diagnostic_types.is_some() {
        source.push_str("#include \"Rte_Dcm_Type.h\"\n");
    }
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
    source.push_str("#define RTE_Rte_START_SEC_CODE\n#include \"Rte_MemMap.h\"\nRTE_Rte_CODE Std_ReturnType Rte_Start(void)\n{\n    if (Ecu_TargetCheckLifecycleContext(TRUE) != E_OK) {\n        return RTE_E_LIMIT;\n    }\n    if (rte_allocated == TRUE) {\n        return RTE_E_LIMIT;\n    }\n");
    source.push_str(&initialization);
    source.push_str("    rte_allocated = TRUE;\n    rte_started = TRUE;\n    return E_OK;\n}\nRTE_Rte_CODE Std_ReturnType Rte_Stop(void)\n{\n    if (Ecu_TargetCheckLifecycleContext(FALSE) != E_OK) {\n        return RTE_E_LIMIT;\n    }\n    rte_started = FALSE;\n    return E_OK;\n}\n");
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
                let head = function(scope, &symbol, &["uint32 *data".to_owned()]);
                source.push_str(&format!("{head}\n{{\n    if (data == NULL_PTR) {{\n        return E_NOT_OK;\n    }}\n    if (rte_started == FALSE) {{\n        return RTE_E_COM_STOPPED;\n    }}\n    if (Ecu_TargetIsOwner() == 0) {{\n        return E_NOT_OK;\n    }}\n"));
                if let Some(network) = network {
                    let id = signal(description, &network.transport.com_signal)?;
                    source.push_str(&format!("    {{\n        uint32 value = {name};\n        const uint8 status = Com_ReceiveSignal({id}u, &value);\n        if (status == COM_SERVICE_NOT_AVAILABLE) {{\n            *data = {name};\n            return RTE_E_COM_STOPPED;\n        }}\n        if (status != E_OK) {{\n            return E_NOT_OK;\n        }}\n        if ({name}_received == FALSE) {{\n            *data = {name};\n            return RTE_E_NEVER_RECEIVED;\n        }}\n        {name} = value;\n        *data = value;\n        return ({name}_expired == TRUE) ? RTE_E_MAX_AGE_EXCEEDED : E_OK;\n    }}\n"));
                } else {
                    source.push_str(&format!("    *data = {name};\n    return E_OK;\n"));
                }
            } else {
                let head = function(scope, &symbol, &["uint32 data".to_owned()]);
                source.push_str(&format!("{head}\n{{\n    if (rte_started == FALSE) {{\n        return RTE_E_COM_STOPPED;\n    }}\n    if (Ecu_TargetIsOwner() == 0) {{\n        return E_NOT_OK;\n    }}\n"));
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
            let head = function(
                scope,
                &operation.implementation_symbol,
                &parameters(&operation.arguments),
            );
            source.push_str(&format!("{head}\n{{\n    if (rte_started == FALSE) {{\n        return RTE_E_COM_STOPPED;\n    }}\n    if (Ecu_TargetIsOwner() == 0) {{\n        return E_NOT_OK;\n    }}\n"));
            for argument in &operation.arguments {
                if argument.native_type.contains('*')
                    || composition
                        .array_types
                        .values()
                        .any(|name| name == &argument.native_type)
                {
                    source.push_str(&format!(
                        "    if ({} == NULL_PTR) {{\n        return {};\n    }}\n",
                        argument.name,
                        operation.application_error.as_deref().unwrap_or("E_NOT_OK")
                    ));
                }
            }
            let callee = if server.application_error.is_some() {
                format!("return {}", runnable.symbol)
            } else {
                runnable.symbol.clone()
            };
            let call = c_layout::call("    ", &callee, &names(&operation.arguments), ";");
            source.push_str(&format!("{call}\n"));
            if server.application_error.is_none() {
                source.push_str("    return E_OK;\n");
            }
            source.push_str("}\n");
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
            if diagnostic_types.is_none() {
                return Err(PlanDiagnostic::plan_premise(&runnable.path));
            }
            // Same parameter type names as the component header declaration (MISRA 8.3).
            let arguments = operation
                .arguments
                .iter()
                .map(|argument| c_layout::parameter(&argument.native_type, &argument.name))
                .collect::<Vec<_>>();
            let head = function(scope, &runnable.symbol, &arguments);
            let call = c_layout::call(
                "    ",
                &format!("return {}", operation.implementation_symbol),
                &names(&operation.arguments),
                ";",
            );
            source.push_str(&format!("{head}\n{{\n{call}\n}}\n"));
        }
        section(&mut source, scope, "CODE", false);
    }
    let partition = &description.ecu_communication()?.partition.name;
    scopes.push((partition.clone(), "CALLOUT_CODE"));
    section(&mut source, partition, "CALLOUT_CODE", true);
    let com = description.ecu_com()?;
    for (callback, timeout) in [("Rte_COMCbk", false), ("Rte_COMCbkRxTOut", true)] {
        source.push_str(&format!("RTE_{partition}_CALLOUT_CODE void {callback}(CbkHandleIdType handle)\n{{\n    if (rte_started == FALSE) {{\n        return;\n    }}\n    if (Ecu_TargetIsOwner() == 0) {{\n        return;\n    }}\n"));
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
                        "        if ({name}_received == TRUE) {{\n            {name}_expired = TRUE;\n        }}\n"
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
        ("include/Rte_Main.h".into(), b"/** @file RTE trusted-context lifecycle; Start follows BSW/SchM Init. */\n#ifndef RTE_MAIN_H\n#define RTE_MAIN_H\n#include \"Std_Types.h\"\nStd_ReturnType Rte_Start(void);\nStd_ReturnType Rte_Stop(void);\ntypedef struct {\n    uint16 partition;\n} SchM_ConfigType;\nvoid SchM_Init(const SchM_ConfigType *ConfigPtr);\nvoid SchM_Start(void);\nvoid SchM_StartTiming(void);\nvoid SchM_Deinit(void);\nextern const SchM_ConfigType Ecu_SchMConfig;\nboolean Ecu_SchMReady(void);\nboolean Ecu_SchMTimingActive(void);\n#endif\n".to_vec()),
        ("include/Rte_Com.h".into(), b"/** @file Handle-bearing COM reception and timeout notifications. */\n#ifndef RTE_COM_H\n#define RTE_COM_H\n#include \"ComStack_Types.h\"\nvoid Rte_COMCbk(CbkHandleIdType handle);\nvoid Rte_COMCbkRxTOut(CbkHandleIdType handle);\n#endif\n".to_vec()),
    ]))
}
