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
    source.push_str(&c_layout::section(
        &format!("RTE_{scope}"),
        "Rte",
        segment,
        start,
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
        r#"/** @file Generated RTE for the checked single-owner component graph. */
#define RTE_CORE
#include "Rte_Main.h"
#include "Rte_Cfg.h"
#include "Rte_Com.h"
#include "Com.h"
#include "Com_Cfg.h"
#include "Os.h"
#include "Rte.h"
#include "Ecu_Target.h"
"#,
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
            r#"#error Incompatible {module} AUTOSAR release
#endif
"#
        ));
    }
    source.push_str(&c_layout::release_check("Rte.c", "Com_Cfg.h", "COM"));
    let diagnostic_types = description
        .diagnostic
        .as_ref()
        .map(|diagnostic| composition.diagnostic_types(diagnostic))
        .transpose()?;
    if diagnostic_types.is_some() {
        source.push_str("#include \"Rte_Dcm_Type.h\"\n");
        source.push_str(&c_layout::release_check("Rte.c", "Rte_Dcm_Type.h", "DCM"));
    }
    for component in &composition.components {
        source.push_str(&format!(
            "#include \"{}\"\n",
            component.header.trim_start_matches("include/")
        ));
    }
    source.push_str(&format!(
        r#"{start_var_cleared_unspecified}/* Owner only; cleared at Stop. */
static boolean rte_started RTE_Rte_VAR_CLEARED_UNSPECIFIED;
/* Owner only; allocated once. */
static boolean rte_allocated RTE_Rte_VAR_CLEARED_UNSPECIFIED;
"#,
        start_var_cleared_unspecified =
            c_layout::section("RTE_Rte", "Rte", "VAR_CLEARED_UNSPECIFIED", true)
    ));
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
                        source.push_str(&format!(
 r#"/* Owner-only reception status; starts FALSE and publication sets TRUE. */
static boolean {name}_received RTE_{scope}_VAR_CLEARED_UNSPECIFIED;
"#));
                    }
                    if receiver.expired {
                        source.push_str(&format!(
                            r#"/* Owner-only timeout status; a successful reception clears it. */
static boolean {name}_expired RTE_{scope}_VAR_CLEARED_UNSPECIFIED;
"#
                        ));
                    }
                }
                section(&mut source, scope, "VAR_CLEARED_UNSPECIFIED", false);
            }
        }
    }
    source.push_str(&format!(
        r#"{start_code}RTE_Rte_CODE Std_ReturnType Rte_Start(void)
{{
    if (Ecu_TargetCheckLifecycleContext(TRUE) != E_OK) {{
        return RTE_E_LIMIT;
    }}
    if (rte_allocated == TRUE) {{
        return RTE_E_LIMIT;
    }}
"#,
        start_code = c_layout::section("RTE_Rte", "Rte", "CODE", true)
    ));
    source.push_str(
        r#"    rte_allocated = TRUE;
    rte_started = TRUE;
    return E_OK;
}
RTE_Rte_CODE Std_ReturnType Rte_Stop(void)
{
    if (Ecu_TargetCheckLifecycleContext(FALSE) != E_OK) {
        return RTE_E_LIMIT;
    }
    rte_started = FALSE;
    return E_OK;
}
"#,
    );
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
                source.push_str(&format!(
                    r#"{head}
{{
    if (data == NULL_PTR) {{
        return E_NOT_OK;
    }}
    if (rte_started == FALSE) {{
        return RTE_E_COM_STOPPED;
    }}
    if (Ecu_TargetIsOwner() == 0) {{
        return E_NOT_OK;
    }}
"#
                ));
                if let Some(network) = network {
                    let id = signal(description, &network.transport.com_signal)?;
                    source.push_str(&format!(
                        r#"    {{
        uint32 value = {name};
        const uint8 status = Com_ReceiveSignal({id}, &value);
        if (status == COM_SERVICE_NOT_AVAILABLE) {{
            *data = {name};
            return RTE_E_COM_STOPPED;
        }}
        if (status != E_OK) {{
            return E_NOT_OK;
        }}
        if ({name}_received == FALSE) {{
            *data = {name};
            return {};
        }}
        {name} = value;
        *data = value;
        return ({name}_expired == TRUE) ? RTE_E_MAX_AGE_EXCEEDED : E_OK;
    }}
"#,
                        if port.handle_never_received {
                            "RTE_E_NEVER_RECEIVED"
                        } else {
                            "E_OK"
                        }
                    ));
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
                source.push_str(&format!(
                    r#"{head}
{{
    if (rte_started == FALSE) {{
        return RTE_E_COM_STOPPED;
    }}
    if (Ecu_TargetIsOwner() == 0) {{
        return E_NOT_OK;
    }}
"#
                ));
                if let Some(network) = network {
                    let id = signal(description, &network.transport.com_signal)?;
                    source.push_str(&format!(
                        r#"    const uint8 status = Com_SendSignal({id}, &data);
    return (status == COM_SERVICE_NOT_AVAILABLE) ? RTE_E_COM_STOPPED : status;
"#
                    ));
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
                source.push_str(&format!(
                    r#"{head}
{{
"#
                ));
                for argument in &operation.arguments {
                    source.push_str(&format!("    (void){};\n", argument.name));
                }
                source.push_str(
                    r#"    return RTE_E_UNCONNECTED;
}
"#,
                );
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
            source.push_str(&format!(
                r#"{head}
{{
    if (rte_started == FALSE) {{
        return RTE_E_COM_STOPPED;
    }}
    if (Ecu_TargetIsOwner() == 0) {{
        return E_NOT_OK;
    }}
"#
            ));
            for argument in &operation.arguments {
                if argument.native_type.contains('*')
                    || composition
                        .array_types
                        .values()
                        .any(|name| name == &argument.native_type)
                {
                    source.push_str(&format!(
                        r#"    if ({} == NULL_PTR) {{
        return {};
    }}
"#,
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
            source.push_str(&format!(
                r#"{head}
{{
{call}
}}
"#
            ));
        }
        section(&mut source, scope, "CODE", false);
    }
    let partition = &description.ecu_communication()?.partition.name;
    section(&mut source, partition, "CALLOUT_CODE", true);
    let com = description.ecu_com()?;
    for (callback, timeout) in [("Rte_COMCbk", false), ("Rte_COMCbkRxTOut", true)] {
        source.push_str(&format!(
            r#"RTE_{partition}_CALLOUT_CODE void {callback}(CbkHandleIdType handle)
{{
    if (rte_started == FALSE) {{
        return;
    }}
    if (Ecu_TargetIsOwner() == 0) {{
        return;
    }}
"#
        ));
        for reception in &com.receptions {
            for network in composition.network_endpoints.iter().filter(|network| {
                network.transport.receive && network.transport.com_signal == reception.signal
            }) {
                let name = &states[&network.endpoint];
                source.push_str(&format!(
                    "    if (handle == {}) {{\n",
                    super::symbolic_names::name("Rte", "ComUserSignal", &reception.user_signal)
                ));
                if timeout {
                    source.push_str(&format!(
                        r#"        if ({name}_received == TRUE) {{
            {name}_expired = TRUE;
        }}
"#
                    ));
                } else {
                    let id = signal(description, &reception.signal)?;
                    source.push_str(&format!(
                        r#"        if (Com_ReceiveSignal({id}, &{name}) == E_OK) {{
            {name}_received = TRUE;
            {name}_expired = FALSE;
        }}
"#
                    ));
                }
                source.push_str("    }\n");
            }
        }
        source.push_str("}\n");
    }
    section(&mut source, partition, "CALLOUT_CODE", false);
    source.push_str(&c_layout::release_check("Rte.c", "Rte_MemMap.h", "MEMMAP"));
    Ok(BTreeMap::from([
        // The delivered targets place all RTE code in their executable RTE section.
        (
            "include/Rte_MemMap.h".into(),
            super::memmap::rte(
                &scopes,
                Some(".rte_code"),
                super::memmap::dcm_scope(composition),
            )
            .into_bytes(),
        ),
        ("src/Rte.c".into(), source.into_bytes()),
    ]))
}
