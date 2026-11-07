use super::ContractArgument;
use super::c_layout;
use super::composition::{BswTarget, Component, Operation, Runnable};
use std::collections::BTreeMap;

pub(super) const HEADER: &str = "Rte_Os.h";

pub(super) fn contract(counter: &str) -> Component {
    let interface = format!("/OsServices/OsService_{counter}");
    let behavior = "/OsServices/Os/Behavior";
    let port = "/OsServices/Os/OsService";
    let mut operations = Vec::new();
    let mut runnables = Vec::new();
    for native in super::catalog::counter_entries() {
        let operation = native.symbol.as_str();
        let errors = if operation == "GetElapsedValue" {
            vec![("E_OS_ID", 3), ("E_OS_VALUE", 8)]
        } else {
            vec![("E_OS_ID", 3)]
        };
        let arguments = native
            .arguments
            .iter()
            .skip(1)
            .map(|argument| ContractArgument {
                name: argument.name.clone(),
                native_type: "TimeInMicrosecondsType *".into(),
                direction: argument.direction.clone(),
            })
            .collect::<Vec<_>>();
        runnables.push(Runnable {
            path: format!("{behavior}/{operation}"),
            symbol: operation.into(),
            event: Some(format!("{behavior}/On{operation}")),
            period_ms: None,
            bsw_target: Some(BswTarget {
                mapping: "/OsServices/ServiceMapping_Os_0".into(),
                behavior: "/AUTOSAR_Os/Os/HostBehavior".into(),
                entity: format!("/AUTOSAR_Os/Os/HostBehavior/{operation}Entity"),
                entry: format!("/AUTOSAR_Os/BswModuleEntrys/{operation}"),
                entity_kind: "BSW-CALLED-ENTITY".into(),
                reentrant: true,
                synchronous: true,
                minimum_start_interval: "0".into(),
                execution_context: "UNSPECIFIED".into(),
                contract: native.clone(),
            }),
        });
        operations.push(Operation {
            port: port.into(),
            interface: interface.clone(),
            operation: format!("{interface}/{operation}"),
            application_errors: errors
                .into_iter()
                .map(|(name, code)| (name.into(), code))
                .collect(),
            read: false,
            api_symbol: format!("Rte_Call_OsService_{operation}"),
            implementation_symbol: format!("OsService_{counter}_{operation}"),
            argument_types: vec!["/OsServices/TimeInMicrosecondsType".into(); arguments.len()],
            arguments,
            runnable: format!("{behavior}/{operation}"),
            callers: Vec::new(),
            call_points: Vec::new(),
            event: Some(format!("{behavior}/On{operation}")),
        });
    }
    Component {
        component: "/OsServices/Os".into(),
        instance: "/OsServices/OsInstance".into(),
        ecu: String::new(),
        ecu_mapping: String::new(),
        rte_instance: String::new(),
        behavior: behavior.into(),
        header: format!("include/{HEADER}"),
        interfaces: BTreeMap::from([(port.into(), interface)]),
        diagnostic_session_port: None,
        diagnostic_mode_ports: Vec::new(),
        data_ports: Vec::new(),
        data_accesses: Vec::new(),
        operations,
        runnables,
        port_arguments: BTreeMap::from([(
            port.into(),
            vec![super::rte_types::PortDefinedArgument {
                native_type: "CounterType".into(),
                type_identity: "/OsServices/CounterType".into(),
                value: "0".into(),
            }],
        )]),
    }
}

/// The selected Counter's service description and actual synchronous binding.
/// Counter state and service error handling remain exclusively in the OS.
pub(super) fn files(
    counter: &str,
    counter_symbol: &str,
    contract: &Component,
) -> Result<BTreeMap<String, Vec<u8>>, Vec<super::PlanDiagnostic>> {
    let interface = format!("OsService_{counter}");
    let mut elements = String::new();
    for (name, bits, native) in [
        ("CounterType", 32, "uint32"),
        ("TimeInMicrosecondsType", 64, "uint64"),
    ] {
        elements.push_str(&format!(
 r#"<SW-BASE-TYPE><SHORT-NAME>Base_{name}</SHORT-NAME><CATEGORY>FIXED_LENGTH</CATEGORY><BASE-TYPE-SIZE>{bits}</BASE-TYPE-SIZE><BASE-TYPE-ENCODING>NONE</BASE-TYPE-ENCODING><NATIVE-DECLARATION>{native}</NATIVE-DECLARATION></SW-BASE-TYPE><IMPLEMENTATION-DATA-TYPE><SHORT-NAME>{name}</SHORT-NAME><CATEGORY>VALUE</CATEGORY><SW-DATA-DEF-PROPS><SW-DATA-DEF-PROPS-VARIANTS><SW-DATA-DEF-PROPS-CONDITIONAL><BASE-TYPE-REF DEST="SW-BASE-TYPE">/OsServices/Base_{name}</BASE-TYPE-REF></SW-DATA-DEF-PROPS-CONDITIONAL></SW-DATA-DEF-PROPS-VARIANTS></SW-DATA-DEF-PROPS><TYPE-EMITTER>Rte_Type.h</TYPE-EMITTER></IMPLEMENTATION-DATA-TYPE>"#));
    }
    let mut operations = String::new();
    let mut com_specs = String::new();
    let mut events = String::new();
    let mut runnables = String::new();
    let mut header = format!(
        r#"/** @file Generated synchronous Counter service bindings.
 * Values are OS ticks, as specified by SWS_Os_00560, despite the type name.
 */
{guard_start}#include "Rte_Version.h"
#include "Os.h"
#include "Rte_Os_Type.h"
{release_check}{linkage_begin}{start_code}"#,
        guard_start = c_layout::header_start("AUTOSAR_GENERATED_RTE_OS_H"),
        release_check = c_layout::release_check("Rte_Os.h", "Os.h", "OS"),
        linkage_begin = c_layout::linkage(true),
        start_code = c_layout::section("RTE_Rte", "Rte", "CODE", true),
    );
    let mut clients = String::new();
    let mut source = format!(
        r#"#include "Rte_Os.h"
{start_code}"#,
        start_code = c_layout::section("RTE_Rte", "Rte", "CODE", true),
    );
    for binding in &contract.operations {
        let operation = super::short_name(&binding.operation);
        let target = contract
            .runnables
            .iter()
            .find(|runnable| runnable.path == binding.runnable)
            .and_then(|runnable| runnable.bsw_target.as_ref())
            .ok_or_else(|| super::PlanDiagnostic::plan_premise(&binding.runnable))?;
        let symbol = &binding.implementation_symbol;
        operations.push_str(&format!(
            "<CLIENT-SERVER-OPERATION><SHORT-NAME>{operation}</SHORT-NAME><ARGUMENTS>"
        ));
        for argument in &binding.arguments {
            let name = &argument.name;
            let direction = &argument.direction;
            operations.push_str(&format!(
 r#"<ARGUMENT-DATA-PROTOTYPE><SHORT-NAME>{name}</SHORT-NAME><TYPE-TREF DEST="IMPLEMENTATION-DATA-TYPE">/OsServices/TimeInMicrosecondsType</TYPE-TREF><DIRECTION>{direction}</DIRECTION><SERVER-ARGUMENT-IMPL-POLICY>USE-ARGUMENT-TYPE</SERVER-ARGUMENT-IMPL-POLICY></ARGUMENT-DATA-PROTOTYPE>"#));
        }
        operations.push_str("</ARGUMENTS><POSSIBLE-ERROR-REFS>");
        for error in binding.application_errors.keys() {
            operations.push_str(&format!(
 r#"<POSSIBLE-ERROR-REF DEST="APPLICATION-ERROR">/OsServices/{interface}/{error}</POSSIBLE-ERROR-REF>"#));
        }
        operations.push_str("</POSSIBLE-ERROR-REFS></CLIENT-SERVER-OPERATION>");
        com_specs.push_str(&format!(
 r#"<SERVER-COM-SPEC><OPERATION-REF DEST="CLIENT-SERVER-OPERATION">/OsServices/{interface}/{operation}</OPERATION-REF><QUEUE-LENGTH>1</QUEUE-LENGTH></SERVER-COM-SPEC>"#));
        events.push_str(&format!(
 r#"<OPERATION-INVOKED-EVENT><SHORT-NAME>On{operation}</SHORT-NAME><START-ON-EVENT-REF DEST="RUNNABLE-ENTITY">/OsServices/Os/Behavior/{operation}</START-ON-EVENT-REF><OPERATION-IREF><CONTEXT-P-PORT-REF DEST="P-PORT-PROTOTYPE">/OsServices/Os/OsService</CONTEXT-P-PORT-REF><TARGET-PROVIDED-OPERATION-REF DEST="CLIENT-SERVER-OPERATION">/OsServices/{interface}/{operation}</TARGET-PROVIDED-OPERATION-REF></OPERATION-IREF></OPERATION-INVOKED-EVENT>"#));
        runnables.push_str(&format!("<RUNNABLE-ENTITY><SHORT-NAME>{operation}</SHORT-NAME><CAN-BE-INVOKED-CONCURRENTLY>{}</CAN-BE-INVOKED-CONCURRENTLY><SYMBOL>{}</SYMBOL></RUNNABLE-ENTITY>", target.reentrant, target.contract.symbol));
        let elapsed_doc = if operation == "GetElapsedValue" {
            " * @param[out] ElapsedValue Tick difference; unchanged on a rejected call.\n"
        } else {
            ""
        };
        let port = &binding.api_symbol;
        let parameters: Vec<String> = binding
            .arguments
            .iter()
            .map(|argument| c_layout::parameter(&argument.native_type, &argument.name))
            .collect();
        let implicit = target
            .contract
            .arguments
            .first()
            .ok_or_else(|| super::PlanDiagnostic::plan_premise(&target.entry))?;
        let served: Vec<String> =
            std::iter::once(c_layout::parameter(&implicit.native_type, &implicit.name))
                .chain(parameters.iter().cloned())
                .collect();
        let forwarded: Vec<String> = binding
            .arguments
            .iter()
            .map(|argument| argument.name.clone())
            .collect();
        let declaration =
            c_layout::signature("RTE_Rte_CODE StatusType", symbol, &served, ";", false);
        let direction = if operation == "GetElapsedValue" {
            "in,out"
        } else {
            "out"
        };
        let conditions = r#" * Preconditions: OS initialized; CounterID and non-NULL output storage valid.
 * Context: task or ISR2, outside exclusive areas, subject to OS call-level checks.
 * Reentrancy: reentrant; counter state is serialized by the OS.
 * Ownership: caller retains pointers; rejected calls preserve outputs.
"#;
        header.push_str(&format!(
 r#"/** Synchronous {operation}; preserve native OS errors and output semantics.
 * @param[in] CounterID Port-defined Counter handle for the server.
 * @param[{direction}] Value Counter ticks; updated only on success.
{elapsed_doc} * @return Native OS result, including E_OS_ID and E_OS_VALUE for invalid elapsed input.
{conditions} */
{declaration}
"#));
        let head = c_layout::signature(
            "static inline RTE_Rte_CODE StatusType",
            port,
            &parameters,
            "",
            true,
        );
        let arguments: Vec<String> = std::iter::once(counter_symbol.to_owned())
            .chain(forwarded.iter().cloned())
            .collect();
        let call = binding.invocation(symbol, &arguments, true);
        clients.push_str(&format!(
            r#"/** Counter-bound {operation}; native tick and error semantics.
 * @param[{direction}] Value Counter ticks; updated only on success.
{elapsed_doc} * @return Native OS service result; rejected calls preserve output storage.
{conditions} */
{head}
{{
{call}
}}
"#
        ));
        let head = c_layout::signature("RTE_Rte_CODE StatusType", symbol, &served, "", true);
        let arguments: Vec<String> = std::iter::once(implicit.name.clone())
            .chain(forwarded.iter().cloned())
            .collect();
        let call = binding.invocation(&target.contract.symbol, &arguments, true);
        source.push_str(&format!(
            r#"{head}
{{
{call}
}}
"#
        ));
    }
    source.push_str(&c_layout::section("RTE_Rte", "Rte", "CODE", false));
    let mut errors = String::new();
    for (name, code) in [
        ("E_OS_ACCESS", 1),
        ("E_OS_ID", 3),
        ("E_OS_STATE", 7),
        ("E_OS_VALUE", 8),
    ] {
        errors.push_str(&format!("<APPLICATION-ERROR><SHORT-NAME>{name}</SHORT-NAME><ERROR-CODE>{code}</ERROR-CODE></APPLICATION-ERROR>"));
    }
    elements.push_str(&format!(
 r#"<CLIENT-SERVER-INTERFACE><SHORT-NAME>{interface}</SHORT-NAME><IS-SERVICE>true</IS-SERVICE><SERVICE-KIND>OPERATING-SYSTEM</SERVICE-KIND><OPERATIONS>{operations}</OPERATIONS><POSSIBLE-ERRORS>{errors}</POSSIBLE-ERRORS></CLIENT-SERVER-INTERFACE><SERVICE-SW-COMPONENT-TYPE><SHORT-NAME>Os</SHORT-NAME><PORTS><P-PORT-PROTOTYPE><SHORT-NAME>OsService</SHORT-NAME><PROVIDED-COM-SPECS>{com_specs}</PROVIDED-COM-SPECS><PROVIDED-INTERFACE-TREF DEST="CLIENT-SERVER-INTERFACE">/OsServices/{interface}</PROVIDED-INTERFACE-TREF></P-PORT-PROTOTYPE></PORTS><INTERNAL-BEHAVIORS><SWC-INTERNAL-BEHAVIOR><SHORT-NAME>Behavior</SHORT-NAME><EVENTS>{events}</EVENTS><PORT-API-OPTIONS><PORT-API-OPTION><ENABLE-TAKE-ADDRESS>false</ENABLE-TAKE-ADDRESS><INDIRECT-API>false</INDIRECT-API><PORT-ARG-VALUES><PORT-DEFINED-ARGUMENT-VALUE><VALUE><NUMERICAL-VALUE-SPECIFICATION><VALUE>0</VALUE></NUMERICAL-VALUE-SPECIFICATION></VALUE><VALUE-TYPE-TREF DEST="IMPLEMENTATION-DATA-TYPE">/OsServices/CounterType</VALUE-TYPE-TREF></PORT-DEFINED-ARGUMENT-VALUE></PORT-ARG-VALUES><PORT-REF DEST="P-PORT-PROTOTYPE">/OsServices/Os/OsService</PORT-REF></PORT-API-OPTION></PORT-API-OPTIONS><RUNNABLES>{runnables}</RUNNABLES><SUPPORTS-MULTIPLE-INSTANTIATION>false</SUPPORTS-MULTIPLE-INSTANTIATION></SWC-INTERNAL-BEHAVIOR></INTERNAL-BEHAVIORS></SERVICE-SW-COMPONENT-TYPE>"#));
    elements.push_str(&contract.bsw_mapping_xml()?);
    elements.push_str("<SWC-IMPLEMENTATION><SHORT-NAME>OsImplementation</SHORT-NAME><PROGRAMMING-LANGUAGE>C</PROGRAMMING-LANGUAGE><BEHAVIOR-REF DEST=\"SWC-INTERNAL-BEHAVIOR\">/OsServices/Os/Behavior</BEHAVIOR-REF></SWC-IMPLEMENTATION>");
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>OsServices</SHORT-NAME><ELEMENTS>{elements}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>
"#
    );
    header.push_str(&format!(
        r#"{clients}{stop_code}{release_check}{linkage_end}#endif
"#,
        stop_code = c_layout::section("RTE_Rte", "Rte", "CODE", false),
        release_check = c_layout::release_check(HEADER, "Rte_MemMap.h", "MEMMAP"),
        linkage_end = c_layout::linkage(false)
    ));
    source.push_str(&c_layout::release_check(
        "Rte_OsService.c",
        "Rte_MemMap.h",
        "MEMMAP",
    ));
    Ok(BTreeMap::from([
        (
            "generated/descriptions/Os_Service.arxml".into(),
            xml.into_bytes(),
        ),
        (format!("include/{HEADER}"), header.into_bytes()),
        ("src/Rte_OsService.c".into(), source.into_bytes()),
    ]))
}
