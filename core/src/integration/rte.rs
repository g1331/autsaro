//! Actual bounded RTE producers from the immutable component composition contract.
use super::c_layout;
use super::composition::{Endpoint, data_symbol};
use super::{ContractArgument, PlanDescription, PlanDiagnostic, ValidatedIntegrationPlan};
use std::collections::{BTreeMap, BTreeSet};

fn signal(plan: &PlanDescription, path: &str) -> Result<String, Vec<PlanDiagnostic>> {
    plan.handles
        .iter()
        .find(|handle| handle.domain == "com_signal" && handle.path == path)
        .map(|handle| super::com::symbolic_name(&handle.path))
        .ok_or_else(|| PlanDiagnostic::plan_premise(path))
}

fn section(source: &mut String, scope: &str, segment: &str, start: bool) {
    source.push_str(&format!(
        "#define RTE_{scope}_{}_SEC_{segment}\n#include \"Rte_MemMap.h\"\n",
        if start { "START" } else { "STOP" }
    ));
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
        "/** @file Generated RTE for the checked single-owner component graph. */\n#define RTE_CORE\n#include \"Rte_Main.h\"\n#include \"Rte_Com.h\"\n#include \"Com.h\"\n#include \"Com_Cfg.h\"\n#include \"Os.h\"\n#include \"Rte.h\"\n#include \"Ecu_Target.h\"\n",
    );
    for (module, prefix) in [
        ("Com", "COM"),
        ("Os", "OS"),
        ("Std_Types", "STD_TYPES"),
        ("Platform", "PLATFORM"),
        ("ComStack_Types", "COMSTACKTYPE"),
    ] {
        source.push_str(&c_layout::condition(
            "if",
            &[
                format!("!defined({prefix}_AR_RELEASE_MAJOR_VERSION)"),
                format!("!defined({prefix}_AR_RELEASE_MINOR_VERSION)"),
            ],
        ));
        source.push_str(&format!("#error Missing {module} AUTOSAR release\n"));
        source.push_str(&c_layout::condition(
            "elif",
            &[
                format!("({prefix}_AR_RELEASE_MAJOR_VERSION != 4U)"),
                format!("({prefix}_AR_RELEASE_MINOR_VERSION != 10U)"),
            ],
        ));
        source.push_str(&format!(
            "#error Incompatible {module} AUTOSAR release\n#endif\n"
        ));
    }
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
    source.push_str("#define RTE_Rte_START_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n/* Owner only; cleared at Stop. */\nstatic boolean rte_started RTE_Rte_VAR_CLEARED_UNSPECIFIED;\n/* Owner only; allocated once. */\nstatic boolean rte_allocated RTE_Rte_VAR_CLEARED_UNSPECIFIED;\n");
    let receivers = super::rte_memory::receivers(composition);
    let states: BTreeMap<_, _> = receivers
        .iter()
        .map(|receiver| (receiver.endpoint.clone(), receiver.name.clone()))
        .collect();
    let received_states: BTreeSet<_> = receivers
        .iter()
        .filter(|receiver| receiver.received)
        .map(|receiver| receiver.endpoint.clone())
        .collect();
    let scopes = super::memmap::rte_sections(description)?;
    section(&mut source, "Rte", "VAR_CLEARED_UNSPECIFIED", false);
    for component in &composition.components {
        let scope = super::short_name(&component.component);
        section(&mut source, scope, "CONST_UNSPECIFIED", true);
        // SWS_Rte_03712 keeps the CDS object private; 03793 exposes its handle.
        source.push_str(&c_layout::initializer(
            &format!("static const Rte_CDS_{scope} Rte_Instance_{scope}"),
            &format!("RTE_{scope}_CONST_UNSPECIFIED"),
            &[c_layout::Item::Value("0u".into())],
        ));
        source.push('\n');
        let declaration = format!("const Rte_CDS_{scope} *const Rte_Inst_{scope}");
        let attribute = format!("RTE_{scope}_CONST_UNSPECIFIED");
        let value = format!("&Rte_Instance_{scope};");
        let assignment = format!("{declaration} {attribute} = {value}");
        if assignment.len() <= 100 {
            source.push_str(&assignment);
        } else if declaration.len() + attribute.len() + 4 <= 100 {
            source.push_str(&format!("{declaration} {attribute} =\n    {value}"));
        } else {
            source.push_str(&format!("{declaration}\n    {attribute} = {value}"));
        }
        source.push('\n');
        section(&mut source, scope, "CONST_UNSPECIFIED", false);
        // Default SWC mode storage belongs to the RTE, not to the SWC application.
        if component.data_ports.iter().any(|port| port.read) {
            section(&mut source, scope, "VAR_INIT_UNSPECIFIED", true);
            for receiver in receivers.iter().filter(|receiver| receiver.scope == scope) {
                let definition = c_layout::initializer(
                    &format!("static {} {}", receiver.native_type, receiver.name),
                    &format!("RTE_{scope}_VAR_INIT_UNSPECIFIED"),
                    &[c_layout::Item::Value(format!(
                        "{}u",
                        receiver.initial_value
                    ))],
                );
                source.push_str("/* Owner-only receiver value, initialized from its ComSpec. */\n");
                source.push_str(&definition);
                source.push('\n');
            }
            section(&mut source, scope, "VAR_INIT_UNSPECIFIED", false);
            if receivers
                .iter()
                .any(|receiver| receiver.scope == scope && (receiver.received || receiver.expired))
            {
                section(&mut source, scope, "VAR_CLEARED_UNSPECIFIED", true);
                for receiver in receivers.iter().filter(|receiver| receiver.scope == scope) {
                    let name = &receiver.name;
                    if receiver.received {
                        source.push_str(&format!("/* Owner-only reception status; starts FALSE and publication sets TRUE. */\nstatic boolean {name}_received RTE_{scope}_VAR_CLEARED_UNSPECIFIED;\n"));
                    }
                    if receiver.expired {
                        source.push_str(&format!("/* Owner-only timeout status; a successful reception clears it. */\nstatic boolean {name}_expired RTE_{scope}_VAR_CLEARED_UNSPECIFIED;\n"));
                    }
                }
                section(&mut source, scope, "VAR_CLEARED_UNSPECIFIED", false);
            }
        }
    }
    source.push_str("#define RTE_Rte_START_SEC_CODE\n#include \"Rte_MemMap.h\"\nRTE_Rte_CODE Std_ReturnType Rte_Start(void)\n{\n    if (Ecu_TargetCheckLifecycleContext(TRUE) != E_OK) {\n        return RTE_E_LIMIT;\n    }\n    if (rte_allocated == TRUE) {\n        return RTE_E_LIMIT;\n    }\n");
    source.push_str("    rte_allocated = TRUE;\n    rte_started = TRUE;\n    return E_OK;\n}\nRTE_Rte_CODE Std_ReturnType Rte_Stop(void)\n{\n    if (Ecu_TargetCheckLifecycleContext(FALSE) != E_OK) {\n        return RTE_E_LIMIT;\n    }\n    rte_started = FALSE;\n    return E_OK;\n}\n");
    section(&mut source, "Rte", "CODE", false);
    for component in &composition.components {
        let scope = super::short_name(&component.component);
        if !scopes.contains(&(scope.to_owned(), "CODE")) {
            continue;
        }
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
                let head = function(scope, &symbol, &[format!("{} *data", port.c_type)]);
                source.push_str(&format!("{head}\n{{\n    if (data == NULL_PTR) {{\n        return E_NOT_OK;\n    }}\n    if (rte_started == FALSE) {{\n        return RTE_E_COM_STOPPED;\n    }}\n    if (Ecu_TargetIsOwner() == 0) {{\n        return E_NOT_OK;\n    }}\n"));
                if let Some(network) = network {
                    let id = signal(description, &network.transport.com_signal)?;
                    source.push_str(&format!("    {{\n        uint32 value = {name};\n        const uint8 status = Com_ReceiveSignal({id}, &value);\n        if (status == COM_SERVICE_NOT_AVAILABLE) {{\n            *data = {name};\n            return RTE_E_COM_STOPPED;\n        }}\n        if (status != E_OK) {{\n            return E_NOT_OK;\n        }}\n        if ({name}_received == FALSE) {{\n            *data = {name};\n            return {};\n        }}\n        {name} = value;\n        *data = value;\n        return ({name}_expired == TRUE) ? RTE_E_MAX_AGE_EXCEEDED : E_OK;\n    }}\n", if port.handle_never_received { "RTE_E_NEVER_RECEIVED" } else { "E_OK" }));
                } else {
                    source.push_str(&format!("    *data = {name};\n"));
                    if !composition.data_connected(&endpoint) {
                        source.push_str("    return RTE_E_UNCONNECTED;\n");
                    } else if port.handle_never_received {
                        source.push_str(&format!(
                            "    return ({name}_received == TRUE) ? E_OK : RTE_E_NEVER_RECEIVED;\n"
                        ));
                    } else {
                        source.push_str("    return E_OK;\n");
                    }
                }
            } else {
                let head = function(scope, &symbol, &[format!("{} data", port.c_type)]);
                source.push_str(&format!("{head}\n{{\n    if (rte_started == FALSE) {{\n        return RTE_E_COM_STOPPED;\n    }}\n    if (Ecu_TargetIsOwner() == 0) {{\n        return E_NOT_OK;\n    }}\n"));
                if let Some(network) = network {
                    let id = signal(description, &network.transport.com_signal)?;
                    source.push_str(&format!("    const uint8 status = Com_SendSignal({id}, &data);\n    return (status == COM_SERVICE_NOT_AVAILABLE) ? RTE_E_COM_STOPPED : status;\n"));
                } else {
                    if !composition.data_connected(&endpoint) {
                        source.push_str("    (void)data;\n");
                    }
                    for connection in composition
                        .connections
                        .iter()
                        .filter(|connection| !connection.service && connection.provider == endpoint)
                    {
                        source
                            .push_str(&format!("    {} = data;\n", states[&connection.requester]));
                        if received_states.contains(&connection.requester) {
                            source.push_str(&format!(
                                "    {}_received = TRUE;\n",
                                states[&connection.requester]
                            ));
                        }
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
            let connection = composition.connections.iter().find(|connection| {
                connection.service
                    && connection.requester.instance == component.instance
                    && connection.requester.port == operation.port
                    && connection.requester.member == operation.operation
            });
            let Some(connection) = connection else {
                let head = function(
                    scope,
                    &operation.implementation_symbol,
                    &parameters(&operation.arguments),
                );
                source.push_str(&format!("{head}\n{{\n"));
                for argument in &operation.arguments {
                    source.push_str(&format!("    (void){};\n", argument.name));
                }
                source.push_str("    return RTE_E_UNCONNECTED;\n}\n");
                continue;
            };
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
                    source.push_str(&format!("        if (Com_ReceiveSignal({id}, &{name}) == E_OK) {{\n            {name}_received = TRUE;\n            {name}_expired = FALSE;\n        }}\n"));
                }
                source.push_str("    }\n");
            }
        }
        source.push_str("}\n");
    }
    section(&mut source, partition, "CALLOUT_CODE", false);
    Ok(BTreeMap::from([
        // The delivered targets place all RTE code in their executable RTE section.
        ("include/Rte_MemMap.h".into(), super::memmap::rte(&scopes, Some(".rte_code"), super::memmap::dcm_scope(composition)).into_bytes()),
        ("src/Rte.c".into(), source.into_bytes()),
        ("include/Rte_Com.h".into(), b"/** @file Handle-bearing COM reception and timeout notifications. */\n#ifndef RTE_COM_H\n#define RTE_COM_H\n#include \"Rte_Type.h\"\n#ifdef __cplusplus\nextern \"C\" {\n#endif\n/** Refresh receiver values after a successful COM reception.\n * @param[in] handle Configured ComUserCbkHandleId; unknown handles are ignored.\n * @par Return\n * None.\n *\n * Preconditions: RTE started and ECU owner context; otherwise no action.\n *\n * Context: COM callback on the ECU owner task, outside exclusive areas.\n *\n * Reentrancy: not reentrant for the same ECU receiver state.\n *\n * Ownership: No pointer is retained; receiver storage remains owned by the RTE.\n */\nvoid Rte_COMCbk(CbkHandleIdType handle);\n/** Mark previously received values as expired after a COM Rx deadline.\n * @param[in] handle Configured ComUserCbkHandleId; unknown handles are ignored.\n * @par Return\n * None.\n *\n * Preconditions: RTE started and ECU owner context; otherwise no action.\n *\n * Context: COM callback on the ECU owner task, outside exclusive areas.\n *\n * Reentrancy: not reentrant for the same ECU receiver state.\n *\n * Ownership: No pointer is retained; receiver storage remains owned by the RTE.\n */\nvoid Rte_COMCbkRxTOut(CbkHandleIdType handle);\n#ifdef __cplusplus\n}\n#endif\n#endif\n".to_vec()),
    ]))
}

pub(super) const MAIN_HEADER: &[u8] =
    br#"/** @file RTE trusted-context lifecycle; Start follows BSW/SchM Init. */
#ifndef RTE_MAIN_H
#define RTE_MAIN_H
#include "Rte.h"
#ifdef __cplusplus
extern "C" {
#endif
/** Start the RTE once for this ECU lifetime.
 * @return E_OK after start; RTE_E_LIMIT on an invalid lifecycle context or repeated allocation.
 *
 * Preconditions: BSW and SchM initialized; StartupHook context, otherwise RTE_E_LIMIT.
 *
 * Context: StartupHook, outside exclusive areas.
 *
 * Reentrancy: not reentrant.
 *
 * Ownership: No pointer is retained.
 */
Std_ReturnType Rte_Start(void);
/** Stop component execution without permitting another allocation.
 * @return E_OK after stop; RTE_E_LIMIT for an invalid lifecycle context.
 *
 * Preconditions: Trusted shutdown context; rejected calls preserve the started state.
 *
 * Context: ShutdownHook, outside exclusive areas.
 *
 * Reentrancy: not reentrant.
 *
 * Ownership: No pointer is retained.
 */
Std_ReturnType Rte_Stop(void);
/** Immutable SchM partition selection for this generated ECU. */
typedef struct {
    uint16 partition; /**< Validated partition handle; no other handle is accepted. */
} SchM_ConfigType;
/** Initialize the selected SchM partition once.
 * @param[in] ConfigPtr Non-NULL configuration with the generated partition handle.
 * @par Return
 * None.
 *
 * Preconditions: Trusted startup context and no prior allocation; invalid or repeated calls have no
 * effect.
 *
 * Context: StartupHook, outside exclusive areas.
 *
 * Reentrancy: not reentrant.
 *
 * Ownership: No pointer is retained.
 */
void SchM_Init(const SchM_ConfigType *ConfigPtr);
/** Enable SchM processing after initialization.
 * @par Return
 * None.
 *
 * Preconditions: Trusted startup context and SchM initialized; otherwise no effect.
 *
 * Context: StartupHook, outside exclusive areas.
 *
 * Reentrancy: not reentrant.
 *
 * Ownership: No pointer is retained.
 */
void SchM_Start(void);
/** Enable configured timing after SchM start.
 * @par Return
 * None.
 *
 * Preconditions: Trusted startup context and SchM started; otherwise no effect.
 *
 * Context: StartupHook, outside exclusive areas.
 *
 * Reentrancy: not reentrant.
 *
 * Ownership: No pointer is retained.
 */
void SchM_StartTiming(void);
/** Retire SchM permanently for this ECU lifetime.
 * @par Return
 * None.
 *
 * Preconditions: Trusted shutdown context; unallocated state stays unchanged.
 *
 * Context: ShutdownHook, outside exclusive areas.
 *
 * Reentrancy: not reentrant.
 *
 * Ownership: No pointer is retained.
 */
void SchM_Deinit(void);
extern const SchM_ConfigType Ecu_SchMConfig;
/** Read whether SchM processing has started.
 * @return TRUE in started or timing state; FALSE otherwise.
 *
 * Preconditions: Lifecycle changes are serialized by the owner; no concurrent lifecycle writer.
 *
 * Context: StartupHook, ShutdownHook or ECU owner task; no concurrent lifecycle writer.
 *
 * Reentrancy: reentrant when lifecycle state is stable.
 *
 * Ownership: No pointer is retained.
 */
boolean Ecu_SchMReady(void);
/** Read whether SchM timing has started.
 * @return TRUE only in timing state; FALSE otherwise.
 *
 * Preconditions: Lifecycle changes are serialized by the owner; no concurrent lifecycle writer.
 *
 * Context: StartupHook, ShutdownHook or ECU owner task; no concurrent lifecycle writer.
 *
 * Reentrancy: reentrant when lifecycle state is stable.
 *
 * Ownership: No pointer is retained.
 */
boolean Ecu_SchMTimingActive(void);
#ifdef __cplusplus
}
#endif
#endif
"#;
