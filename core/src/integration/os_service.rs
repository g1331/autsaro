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
        elements.push_str(&format!(r#"<SW-BASE-TYPE><SHORT-NAME>Base_{name}</SHORT-NAME><CATEGORY>FIXED_LENGTH</CATEGORY><BASE-TYPE-SIZE>{bits}</BASE-TYPE-SIZE><BASE-TYPE-ENCODING>NONE</BASE-TYPE-ENCODING><NATIVE-DECLARATION>{native}</NATIVE-DECLARATION></SW-BASE-TYPE><IMPLEMENTATION-DATA-TYPE><SHORT-NAME>{name}</SHORT-NAME><CATEGORY>VALUE</CATEGORY><SW-DATA-DEF-PROPS><SW-DATA-DEF-PROPS-VARIANTS><SW-DATA-DEF-PROPS-CONDITIONAL><BASE-TYPE-REF DEST="SW-BASE-TYPE">/OsServices/Base_{name}</BASE-TYPE-REF></SW-DATA-DEF-PROPS-CONDITIONAL></SW-DATA-DEF-PROPS-VARIANTS></SW-DATA-DEF-PROPS><TYPE-EMITTER>Rte_Os_Type.h</TYPE-EMITTER></IMPLEMENTATION-DATA-TYPE>"#));
    }
    let mut operations = String::new();
    let mut com_specs = String::new();
    let mut events = String::new();
    let mut runnables = String::new();
    let mut header = String::from(
        "/** @file Generated synchronous Counter service bindings.\n * Values are OS ticks, as specified by SWS_Os_00560, despite the type name.\n */\n#ifndef AUTOSAR_GENERATED_RTE_OS_H\n#define AUTOSAR_GENERATED_RTE_OS_H\n#include \"Os.h\"\n#include \"Rte_Os_Type.h\"\n#ifdef __cplusplus\nextern \"C\" {\n#endif\n",
    );
    let mut clients = String::new();
    let mut source = String::from(
        "#include \"Rte_Os.h\"\n#define RTE_Rte_START_SEC_CODE\n#include \"Rte_MemMap.h\"\n",
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
            operations.push_str(&format!(r#"<ARGUMENT-DATA-PROTOTYPE><SHORT-NAME>{name}</SHORT-NAME><TYPE-TREF DEST="IMPLEMENTATION-DATA-TYPE">/OsServices/TimeInMicrosecondsType</TYPE-TREF><DIRECTION>{direction}</DIRECTION><SERVER-ARGUMENT-IMPL-POLICY>USE-ARGUMENT-TYPE</SERVER-ARGUMENT-IMPL-POLICY></ARGUMENT-DATA-PROTOTYPE>"#));
        }
        operations.push_str("</ARGUMENTS><POSSIBLE-ERROR-REFS>");
        for error in errors {
            operations.push_str(&format!(r#"<POSSIBLE-ERROR-REF DEST="APPLICATION-ERROR">/OsServices/{interface}/{error}</POSSIBLE-ERROR-REF>"#));
        }
        operations.push_str("</POSSIBLE-ERROR-REFS></CLIENT-SERVER-OPERATION>");
        com_specs.push_str(&format!(r#"<SERVER-COM-SPEC><OPERATION-REF DEST="CLIENT-SERVER-OPERATION">/OsServices/{interface}/{operation}</OPERATION-REF><QUEUE-LENGTH>1</QUEUE-LENGTH></SERVER-COM-SPEC>"#));
        events.push_str(&format!(r#"<OPERATION-INVOKED-EVENT><SHORT-NAME>On{operation}</SHORT-NAME><START-ON-EVENT-REF DEST="RUNNABLE-ENTITY">/OsServices/Os/Behavior/{operation}</START-ON-EVENT-REF><OPERATION-IREF><CONTEXT-P-PORT-REF DEST="P-PORT-PROTOTYPE">/OsServices/Os/OsService</CONTEXT-P-PORT-REF><TARGET-PROVIDED-OPERATION-REF DEST="CLIENT-SERVER-OPERATION">/OsServices/{interface}/{operation}</TARGET-PROVIDED-OPERATION-REF></OPERATION-IREF></OPERATION-INVOKED-EVENT>"#));
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
        header.push_str(&format!("/** @brief Synchronous {operation}; preserves native errors and output semantics.\n * @param CounterID Port-defined Counter handle for the server.\n * @param Value Counter ticks; OUT for GetCounterValue, INOUT for GetElapsedValue.\n{elapsed_doc} * @return Native OS service result.\n */\n{declaration}\n"));
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
        clients.push_str(&format!("/** @brief Counter-bound {operation}; native tick and error semantics. */\n{head}\n{{\n{call}\n}}\n"));
        for (name, parameters, callee, first) in [(&symbol, &served, operation, "CounterID")] {
            let head = c_layout::signature("RTE_Rte_CODE StatusType", name, parameters, "", true);
            let arguments: Vec<String> = std::iter::once(first.to_owned())
                .chain(forwarded.iter().cloned())
                .collect();
            let call = c_layout::call("    ", &format!("return {callee}"), &arguments, ";");
            source.push_str(&format!("{head}\n{{\n{call}\n}}\n"));
        }
    }
    source.push_str("#define RTE_Rte_STOP_SEC_CODE\n#include \"Rte_MemMap.h\"\n");
    let mut errors = String::new();
    for (name, code) in [
        ("E_OS_ACCESS", 1),
        ("E_OS_ID", 3),
        ("E_OS_STATE", 7),
        ("E_OS_VALUE", 8),
    ] {
        errors.push_str(&format!("<APPLICATION-ERROR><SHORT-NAME>{name}</SHORT-NAME><ERROR-CODE>{code}</ERROR-CODE></APPLICATION-ERROR>"));
    }
    elements.push_str(&format!(r#"<CLIENT-SERVER-INTERFACE><SHORT-NAME>{interface}</SHORT-NAME><IS-SERVICE>true</IS-SERVICE><SERVICE-KIND>OPERATING-SYSTEM</SERVICE-KIND><OPERATIONS>{operations}</OPERATIONS><POSSIBLE-ERRORS>{errors}</POSSIBLE-ERRORS></CLIENT-SERVER-INTERFACE><SERVICE-SW-COMPONENT-TYPE><SHORT-NAME>Os</SHORT-NAME><PORTS><P-PORT-PROTOTYPE><SHORT-NAME>OsService</SHORT-NAME><PROVIDED-COM-SPECS>{com_specs}</PROVIDED-COM-SPECS><PROVIDED-INTERFACE-TREF DEST="CLIENT-SERVER-INTERFACE">/OsServices/{interface}</PROVIDED-INTERFACE-TREF></P-PORT-PROTOTYPE></PORTS><INTERNAL-BEHAVIORS><SWC-INTERNAL-BEHAVIOR><SHORT-NAME>Behavior</SHORT-NAME><EVENTS>{events}</EVENTS><PORT-API-OPTIONS><PORT-API-OPTION><ENABLE-TAKE-ADDRESS>false</ENABLE-TAKE-ADDRESS><INDIRECT-API>false</INDIRECT-API><PORT-ARG-VALUES><PORT-DEFINED-ARGUMENT-VALUE><VALUE><NUMERICAL-VALUE-SPECIFICATION><VALUE>0</VALUE></NUMERICAL-VALUE-SPECIFICATION></VALUE><VALUE-TYPE-TREF DEST="IMPLEMENTATION-DATA-TYPE">/OsServices/CounterType</VALUE-TYPE-TREF></PORT-DEFINED-ARGUMENT-VALUE></PORT-ARG-VALUES><PORT-REF DEST="P-PORT-PROTOTYPE">/OsServices/Os/OsService</PORT-REF></PORT-API-OPTION></PORT-API-OPTIONS><RUNNABLES>{runnables}</RUNNABLES><SUPPORTS-MULTIPLE-INSTANTIATION>false</SUPPORTS-MULTIPLE-INSTANTIATION></SWC-INTERNAL-BEHAVIOR></INTERNAL-BEHAVIORS></SERVICE-SW-COMPONENT-TYPE>"#));
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>OsServices</SHORT-NAME><ELEMENTS>{elements}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>\n"
    );
    header.push_str(&format!("#define RTE_Rte_START_SEC_CODE\n#include \"Rte_MemMap.h\"\n{clients}#define RTE_Rte_STOP_SEC_CODE\n#include \"Rte_MemMap.h\"\n#ifdef __cplusplus\n}}\n#endif\n#endif\n"));
    BTreeMap::from([
        (
            "generated/descriptions/Os_Service.arxml".into(),
            xml.into_bytes(),
        ),
        (format!("include/{HEADER}"), header.into_bytes()),
        ("src/Rte_OsService.c".into(), source.into_bytes()),
    ])
}
