//! Actual bounded RTE producers from the immutable multi-component contract.
use super::multi::{Endpoint, data_symbol};
use super::{PlanDescription, ValidatedIntegrationPlan};
use std::collections::BTreeMap;
use std::fmt::Write;

fn signal(plan: &PlanDescription, path: &str) -> u32 {
    plan.handles
        .iter()
        .find(|handle| handle.domain == "com_signal" && handle.path == path)
        .unwrap()
        .handle
}

fn section(source: &mut String, scope: &str, segment: &str, start: bool) {
    writeln!(
        source,
        "#define RTE_{scope}_{}_SEC_{segment}\n#include \"Rte_MemMap.h\"",
        if start { "START" } else { "STOP" }
    )
    .unwrap();
}

pub(super) fn memory_map(scopes: &[(String, &str)]) -> String {
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
        writeln!(mapping, "#{} defined(RTE_{scope}_START_SEC_{segment})\n#undef RTE_{scope}_START_SEC_{segment}\n#ifdef RTE_MEMMAP_ACTIVE\n#error \"Nested RTE memory section\"\n#endif\n#define RTE_MEMMAP_ACTIVE\n#define RTE_{scope}_{segment}_ACTIVE\n#define RTE_{scope}_{segment} __attribute__((section(\".{section_name}.rte.{scope}.{segment}\")))\n#define RTE_{class} RTE_{scope}_{segment}\n#elif defined(RTE_{scope}_STOP_SEC_{segment})\n#undef RTE_{scope}_STOP_SEC_{segment}\n#ifndef RTE_{scope}_{segment}_ACTIVE\n#error \"Unmatched RTE memory section\"\n#endif\n#undef RTE_{scope}_{segment}_ACTIVE\n#undef RTE_MEMMAP_ACTIVE\n#undef RTE_{scope}_{segment}\n#undef RTE_{class}", if index == 0 { "if" } else { "elif" }).unwrap();
    }
    mapping.push_str("#else\n#ifndef RTE_MEMMAP_HEADER_CHECK\n#define RTE_MEMMAP_HEADER_CHECK\ntypedef unsigned char Rte_MemMap_HeaderCheck;\n#endif\n#endif\n");
    mapping
}

pub(super) fn files(plan: &ValidatedIntegrationPlan) -> BTreeMap<String, Vec<u8>> {
    let description = plan.description();
    let multi = description.multi.as_ref().unwrap();
    let mut source = String::from(
        "/** @file Generated RTE for the checked single-owner component graph. */\n#define RTE_CORE\n#include \"Rte_Main.h\"\n#include \"Rte_Com.h\"\n#include \"Com.h\"\n#include \"Ecu_Target.h\"\n",
    );
    for component in &multi.components {
        writeln!(
            source,
            "#include \"{}\"",
            component.header.trim_start_matches("include/")
        )
        .unwrap();
    }
    source.push_str("#define RTE_Rte_START_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\nstatic boolean rte_started RTE_Rte_VAR_CLEARED_UNSPECIFIED;\nstatic boolean rte_allocated RTE_Rte_VAR_CLEARED_UNSPECIFIED;\n");
    let mut states = BTreeMap::new();
    let mut initialization = String::new();
    let mut scopes = vec![
        ("Rte".to_string(), "VAR_CLEARED_UNSPECIFIED"),
        ("Rte".to_string(), "CODE"),
        ("Rte".to_string(), "CONFIG_DATA_PREBUILD_UNSPECIFIED"),
    ];
    if !multi
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
    for component in &multi.components {
        let scope = component.component.rsplit('/').next().unwrap();
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
            let network = multi
                .network_endpoints
                .iter()
                .find(|network| network.endpoint == endpoint);
            writeln!(
                source,
                "static uint32 {name} RTE_{scope}_VAR_CLEARED_UNSPECIFIED;"
            )
            .unwrap();
            writeln!(initialization, "    {name} = {}u;", port.initial_value).unwrap();
            if network.is_some() {
                writeln!(
                    source,
                    "static boolean {name}_received RTE_{scope}_VAR_CLEARED_UNSPECIFIED;\nstatic boolean {name}_expired RTE_{scope}_VAR_CLEARED_UNSPECIFIED;"
                )
                .unwrap();
                writeln!(
                    initialization,
                    "    {name}_received = FALSE;\n    {name}_expired = FALSE;"
                )
                .unwrap();
            }
            states.insert(endpoint, name);
        }
        section(&mut source, scope, "VAR_CLEARED_UNSPECIFIED", false);
    }
    source.push_str("#define RTE_Rte_START_SEC_CODE\n#include \"Rte_MemMap.h\"\nRTE_Rte_CODE Std_ReturnType Rte_Start(void) {\n    if (Ecu_TargetCheckLifecycleContext(TRUE) != E_OK) { return RTE_E_LIMIT; }\n    if (rte_allocated == TRUE) { return RTE_E_LIMIT; }\n");
    source.push_str(&initialization);
    source.push_str("    rte_allocated = TRUE;\n    rte_started = TRUE;\n    return E_OK;\n}\nRTE_Rte_CODE Std_ReturnType Rte_Stop(void) {\n    if (Ecu_TargetCheckLifecycleContext(FALSE) != E_OK) { return RTE_E_LIMIT; }\n    rte_started = FALSE;\n    return E_OK;\n}\n");
    section(&mut source, "Rte", "CODE", false);
    for component in &multi.components {
        let scope = component.component.rsplit('/').next().unwrap();
        section(&mut source, scope, "CODE", true);
        for port in &component.data_ports {
            let endpoint = Endpoint {
                instance: component.instance.clone(),
                port: port.path.clone(),
                member: port.element.clone(),
            };
            let network = multi
                .network_endpoints
                .iter()
                .find(|network| network.endpoint == endpoint);
            let symbol = data_symbol(component, port);
            if port.read {
                let name = &states[&endpoint];
                writeln!(source, "RTE_{scope}_CODE Std_ReturnType {symbol}(uint32 *data) {{\n    if (data == NULL_PTR) {{ return E_NOT_OK; }}\n    if (rte_started == FALSE) {{ return RTE_E_COM_STOPPED; }}\n    if (Ecu_TargetIsOwner() == 0) {{ return E_NOT_OK; }}").unwrap();
                if let Some(network) = network {
                    let id = signal(description, &network.transport.com_signal);
                    writeln!(source, "    {{\n        uint32 value = {name};\n        const uint8 status = Com_ReceiveSignal({id}u, &value);\n        if (status == COM_SERVICE_NOT_AVAILABLE) {{\n            *data = {name};\n            return RTE_E_COM_STOPPED;\n        }}\n        if (status != E_OK) {{ return E_NOT_OK; }}\n        if ({name}_received == FALSE) {{\n            *data = {name};\n            return RTE_E_NEVER_RECEIVED;\n        }}\n        {name} = value;\n        *data = value;\n        return ({name}_expired == TRUE) ? RTE_E_MAX_AGE_EXCEEDED : E_OK;\n    }}").unwrap();
                } else {
                    writeln!(source, "    *data = {name};\n    return E_OK;").unwrap();
                }
            } else {
                writeln!(source, "RTE_{scope}_CODE Std_ReturnType {symbol}(uint32 data) {{\n    if (rte_started == FALSE) {{ return RTE_E_COM_STOPPED; }}\n    if (Ecu_TargetIsOwner() == 0) {{ return E_NOT_OK; }}").unwrap();
                if let Some(network) = network {
                    let id = signal(description, &network.transport.com_signal);
                    writeln!(source, "    const uint8 status = Com_SendSignal({id}u, &data);\n    return (status == COM_SERVICE_NOT_AVAILABLE) ? RTE_E_COM_STOPPED : status;").unwrap();
                } else {
                    for connection in multi
                        .connections
                        .iter()
                        .filter(|connection| !connection.service && connection.provider == endpoint)
                    {
                        writeln!(source, "    {} = data;", states[&connection.requester]).unwrap();
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
            let connection = multi
                .connections
                .iter()
                .find(|connection| {
                    connection.service
                        && connection.requester.instance == component.instance
                        && connection.requester.port == operation.port
                        && connection.requester.member == operation.operation
                })
                .unwrap();
            let provider = multi
                .components
                .iter()
                .find(|component| component.instance == connection.provider.instance)
                .unwrap();
            let server = provider
                .operations
                .iter()
                .find(|operation| {
                    !operation.read
                        && operation.port == connection.provider.port
                        && operation.operation == connection.provider.member
                })
                .unwrap();
            let runnable = provider
                .runnables
                .iter()
                .find(|runnable| runnable.path == server.runnable)
                .unwrap();
            let parameters = operation
                .arguments
                .iter()
                .map(|arg| format!("{} {}", arg.native_type, arg.name))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(source, "RTE_{scope}_CODE Std_ReturnType {}({}) {{\n    if (rte_started == FALSE) {{ return RTE_E_COM_STOPPED; }}\n    if (Ecu_TargetIsOwner() == 0) {{ return E_NOT_OK; }}", operation.implementation_symbol, if parameters.is_empty() { "void" } else { &parameters }).unwrap();
            for argument in &operation.arguments {
                if argument.native_type.contains('*')
                    || multi
                        .array_types
                        .values()
                        .any(|name| name == &argument.native_type)
                {
                    writeln!(
                        source,
                        "    if ({} == NULL_PTR) {{ return E_NOT_OK; }}",
                        argument.name
                    )
                    .unwrap();
                }
            }
            let names = operation
                .arguments
                .iter()
                .map(|arg| arg.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(
                source,
                "    {}({names});\n    return E_OK;\n}}",
                runnable.symbol
            )
            .unwrap();
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
                .unwrap();
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
            writeln!(
                source,
                "RTE_{scope}_CODE Std_ReturnType {}({}) {{\n    return {}({names});\n}}",
                runnable.symbol,
                if parameters.is_empty() {
                    "void"
                } else {
                    &parameters
                },
                operation.implementation_symbol
            )
            .unwrap();
        }
        section(&mut source, scope, "CODE", false);
    }
    let partition = &description
        .communication_runtime
        .as_ref()
        .unwrap()
        .partition
        .name;
    scopes.push((partition.clone(), "CALLOUT_CODE"));
    section(&mut source, partition, "CALLOUT_CODE", true);
    let com = description.com_runtime.as_ref().unwrap();
    for (callback, timeout) in [("Rte_COMCbk", false), ("Rte_COMCbkRxTOut", true)] {
        writeln!(source, "RTE_{partition}_CALLOUT_CODE void {callback}(CbkHandleIdType handle) {{\n    if (rte_started == FALSE) {{ return; }}\n    if (Ecu_TargetIsOwner() == 0) {{ return; }}").unwrap();
        for reception in &com.receptions {
            for network in multi.network_endpoints.iter().filter(|network| {
                network.transport.receive && network.transport.com_signal == reception.signal
            }) {
                let name = &states[&network.endpoint];
                writeln!(
                    source,
                    "    if (handle == {}u) {{",
                    reception.callback_handle
                )
                .unwrap();
                if timeout {
                    writeln!(
                        source,
                        "        if ({name}_received == TRUE) {{ {name}_expired = TRUE; }}"
                    )
                    .unwrap();
                } else {
                    let id = signal(description, &reception.signal);
                    writeln!(source, "        if (Com_ReceiveSignal({id}u, &{name}) == E_OK) {{\n            {name}_received = TRUE;\n            {name}_expired = FALSE;\n        }}").unwrap();
                }
                source.push_str("    }\n");
            }
        }
        source.push_str("}\n");
    }
    section(&mut source, partition, "CALLOUT_CODE", false);
    BTreeMap::from([
        ("include/Rte_MemMap.h".into(), memory_map(&scopes).into_bytes()),
        ("src/Rte.c".into(), source.into_bytes()),
        ("include/Rte_Main.h".into(), b"/** @file RTE trusted-context lifecycle; Start follows BSW/SchM Init. */\n#ifndef RTE_MAIN_H\n#define RTE_MAIN_H\n#include \"Std_Types.h\"\nStd_ReturnType Rte_Start(void);\nStd_ReturnType Rte_Stop(void);\n#endif\n".to_vec()),
        ("include/Rte_Com.h".into(), b"/** @file Handle-bearing COM reception and timeout notifications. */\n#ifndef RTE_COM_H\n#define RTE_COM_H\n#include \"ComStack_Types.h\"\nvoid Rte_COMCbk(CbkHandleIdType handle);\nvoid Rte_COMCbkRxTOut(CbkHandleIdType handle);\n#endif\n".to_vec()),
    ])
}
