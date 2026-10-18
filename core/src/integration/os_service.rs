use super::c_layout;
use std::collections::BTreeMap;

pub(super) const HEADER: &str = "Rte_Os.h";

/// The selected Counter's service description and actual synchronous binding.
/// Counter state and service error handling remain exclusively in the OS.
pub(super) fn files(counter: &str, counter_symbol: &str) -> BTreeMap<String, Vec<u8>> {
    let interface = format!("OsService_{counter}");
    let mut elements = String::new();
    for (name, bits, native) in [
        ("CounterType", 32, "uint32_t"),
        ("TimeInMicrosecondsType", 64, "uint64_t"),
    ] {
        elements.push_str(&format!(
 r#"<SW-BASE-TYPE><SHORT-NAME>Base_{name}</SHORT-NAME><CATEGORY>FIXED_LENGTH</CATEGORY><BASE-TYPE-SIZE>{bits}</BASE-TYPE-SIZE><BASE-TYPE-ENCODING>NONE</BASE-TYPE-ENCODING><NATIVE-DECLARATION>{native}</NATIVE-DECLARATION></SW-BASE-TYPE><IMPLEMENTATION-DATA-TYPE><SHORT-NAME>{name}</SHORT-NAME><CATEGORY>VALUE</CATEGORY><SW-DATA-DEF-PROPS><SW-DATA-DEF-PROPS-VARIANTS><SW-DATA-DEF-PROPS-CONDITIONAL><BASE-TYPE-REF DEST="SW-BASE-TYPE">/OsServices/Base_{name}</BASE-TYPE-REF></SW-DATA-DEF-PROPS-CONDITIONAL></SW-DATA-DEF-PROPS-VARIANTS></SW-DATA-DEF-PROPS><TYPE-EMITTER>Rte_Os_Type.h</TYPE-EMITTER></IMPLEMENTATION-DATA-TYPE>"#));
    }
    let mut operations = String::new();
    let mut com_specs = String::new();
    let mut events = String::new();
    let mut runnables = String::new();
    let mut header = format!(
        r#"/** @file Generated synchronous Counter service bindings.
 * Values are OS ticks, as specified by SWS_Os_00560, despite the type name.
 */
{guard_start}#include "Os.h"
#include "Rte_Os_Type.h"
{linkage_begin}"#,
        guard_start = c_layout::header_start("AUTOSAR_GENERATED_RTE_OS_H"),
        linkage_begin = c_layout::linkage(true),
    );
    let mut clients = String::new();
    let mut source = format!(
        r#"#include "Rte_Os.h"
{start_code}"#,
        start_code = c_layout::section("RTE_Rte", "Rte", "CODE", true),
    );
    for (operation, arguments, parameters, forwarded, errors) in [
        (
            "GetCounterValue",
            vec![("Value", "OUT")],
            "TimeInMicrosecondsType *Value",
            "Value",
            vec!["E_OS_ID"],
        ),
        (
            "GetElapsedValue",
            vec![("Value", "INOUT"), ("ElapsedValue", "OUT")],
            "TimeInMicrosecondsType *Value, TimeInMicrosecondsType *ElapsedValue",
            "Value, ElapsedValue",
            vec!["E_OS_ID", "E_OS_VALUE"],
        ),
    ] {
        let symbol = format!("{interface}_{operation}");
        operations.push_str(&format!(
            "<CLIENT-SERVER-OPERATION><SHORT-NAME>{operation}</SHORT-NAME><ARGUMENTS>"
        ));
        for (name, direction) in arguments {
            operations.push_str(&format!(
 r#"<ARGUMENT-DATA-PROTOTYPE><SHORT-NAME>{name}</SHORT-NAME><TYPE-TREF DEST="IMPLEMENTATION-DATA-TYPE">/OsServices/TimeInMicrosecondsType</TYPE-TREF><DIRECTION>{direction}</DIRECTION><SERVER-ARGUMENT-IMPL-POLICY>USE-ARGUMENT-TYPE</SERVER-ARGUMENT-IMPL-POLICY></ARGUMENT-DATA-PROTOTYPE>"#));
        }
        operations.push_str("</ARGUMENTS><POSSIBLE-ERROR-REFS>");
        for error in errors {
            operations.push_str(&format!(
 r#"<POSSIBLE-ERROR-REF DEST="APPLICATION-ERROR">/OsServices/{interface}/{error}</POSSIBLE-ERROR-REF>"#));
        }
        operations.push_str("</POSSIBLE-ERROR-REFS></CLIENT-SERVER-OPERATION>");
        com_specs.push_str(&format!(
 r#"<SERVER-COM-SPEC><OPERATION-REF DEST="CLIENT-SERVER-OPERATION">/OsServices/{interface}/{operation}</OPERATION-REF><QUEUE-LENGTH>1</QUEUE-LENGTH></SERVER-COM-SPEC>"#));
        events.push_str(&format!(
 r#"<OPERATION-INVOKED-EVENT><SHORT-NAME>On{operation}</SHORT-NAME><START-ON-EVENT-REF DEST="RUNNABLE-ENTITY">/OsServices/Os/Behavior/{operation}</START-ON-EVENT-REF><OPERATION-IREF><CONTEXT-P-PORT-REF DEST="P-PORT-PROTOTYPE">/OsServices/Os/OsService</CONTEXT-P-PORT-REF><TARGET-PROVIDED-OPERATION-REF DEST="CLIENT-SERVER-OPERATION">/OsServices/{interface}/{operation}</TARGET-PROVIDED-OPERATION-REF></OPERATION-IREF></OPERATION-INVOKED-EVENT>"#));
        runnables.push_str(&format!("<RUNNABLE-ENTITY><SHORT-NAME>{operation}</SHORT-NAME><CAN-BE-INVOKED-CONCURRENTLY>true</CAN-BE-INVOKED-CONCURRENTLY><SYMBOL>{symbol}</SYMBOL></RUNNABLE-ENTITY>"));
        let elapsed_doc = if operation == "GetElapsedValue" {
            " * @param[out] ElapsedValue Tick difference; unchanged on a rejected call.\n"
        } else {
            ""
        };
        let port = format!("Rte_Call_OsService_{operation}");
        let parameters: Vec<String> = parameters.split(", ").map(str::to_owned).collect();
        let served: Vec<String> = std::iter::once("CounterType CounterID".to_owned())
            .chain(parameters.iter().cloned())
            .collect();
        let forwarded: Vec<String> = forwarded.split(", ").map(str::to_owned).collect();
        let declaration = c_layout::signature("StatusType", &symbol, &served, ";", false);
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
            &port,
            &parameters,
            "",
            true,
        );
        let arguments: Vec<String> = std::iter::once(counter_symbol.to_owned())
            .chain(forwarded.iter().cloned())
            .collect();
        let call = c_layout::call("    ", &format!("return {symbol}"), &arguments, ";");
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
        let head = c_layout::signature("RTE_Rte_CODE StatusType", &symbol, &served, "", true);
        let arguments: Vec<String> = std::iter::once("CounterID".to_owned())
            .chain(forwarded.iter().cloned())
            .collect();
        let call = c_layout::call("    ", &format!("return {operation}"), &arguments, ";");
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
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>OsServices</SHORT-NAME><ELEMENTS>{elements}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>
"#
    );
    header.push_str(&format!(
        r#"{start_code}{clients}{stop_code}{linkage_end}#endif
"#,
        start_code = c_layout::section("RTE_Rte", "Rte", "CODE", true),
        stop_code = c_layout::section("RTE_Rte", "Rte", "CODE", false),
        linkage_end = c_layout::linkage(false)
    ));
    BTreeMap::from([
        (
            "generated/descriptions/Os_Service.arxml".into(),
            xml.into_bytes(),
        ),
        (format!("include/{HEADER}"), header.into_bytes()),
        ("src/Rte_OsService.c".into(), source.into_bytes()),
    ])
}
