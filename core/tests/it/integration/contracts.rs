//! Component, port and service contracts derived from the sources.
use super::*;
use autosar_config_core::integration::InputSource;
use std::path::Path;

#[test]
fn source_derived_multi_contract_is_deterministic_and_keeps_local_identity() {
    let sources = inputs();
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    let description = plan.description();
    let composition = description.composition.as_ref().unwrap();
    assert_eq!(description.profile, "singlecore-ecu-v1");
    assert_eq!(composition.components.len(), 4);
    assert_eq!(composition.connections.len(), 5);
    assert_eq!(composition.network_endpoints.len(), 2);
    assert_eq!(
        description
            .schedule
            .entities
            .iter()
            .filter(|entity| entity.application)
            .map(|entity| entity.symbol.as_str())
            .collect::<Vec<_>>(),
        ["Ingress_Periodic", "Process_Periodic", "Observe_Periodic"]
    );
    for component in &composition.components {
        assert_ne!(component.component, component.instance);
        assert!(component.rte_instance.starts_with("/Configuration/Rte/"));
        if component.component.ends_with("Process") || component.component.ends_with("Observe") {
            assert!(
                !composition
                    .network_endpoints
                    .iter()
                    .any(|endpoint| endpoint.endpoint.instance == component.instance)
            );
        }
    }
    let read_values = composition
        .components
        .iter()
        .flat_map(|component| &component.data_ports)
        .filter(|port| port.name == "Value" && port.read)
        .map(|port| port.initial_value)
        .collect::<Vec<_>>();
    assert_eq!(read_values, [7, 9]);
    let server = description
        .symbols
        .iter()
        .find(|symbol| symbol.symbol == "Process_Transform")
        .unwrap();
    assert_eq!(server.return_type, "void");
    assert_eq!(
        server
            .arguments
            .iter()
            .map(|argument| (argument.native_type.as_str(), argument.direction.as_str()))
            .collect::<Vec<_>>(),
        [("uint32", "IN"), ("uint32 *", "OUT"), ("uint32 *", "INOUT")]
    );
    let files = plan.component_contract_files().unwrap();
    let mut reversed = sources.clone();
    reversed.reverse();
    assert_eq!(
        files.files(),
        build(&reversed)
            .unwrap()
            .component_contract_files()
            .unwrap()
            .files()
    );
    assert!(
        plan.ecu_integration_files(autosar_config_core::target::BuildTarget::LinuxX64ControlledV1)
            .is_err()
    );
    if let Ok(directory) = std::env::var("AUTOSAR_MULTI_CONTRACT_OUTPUT") {
        let output = Path::new(&directory);
        let preview = files.preview(output).unwrap();
        files.generate_previewed(output, &preview.revision).unwrap();
    }
}

#[test]
fn malformed_multi_contracts_are_rejected_at_real_source_objects() {
    let vectors = [
        (
            "composition.arxml",
            "/Application/Process/Value</TARGET-R-PORT-REF>",
            "/Application/Process/Result</TARGET-R-PORT-REF>",
            "direction",
        ),
        (
            "composition.arxml",
            "/Application/Process/Value</TARGET-R-PORT-REF>",
            "/Application/Observe/Value</TARGET-R-PORT-REF>",
            "ownership",
        ),
        (
            "composition.arxml",
            "/Application/Process/Value</TARGET-R-PORT-REF>",
            "/Application/Missing/Value</TARGET-R-PORT-REF>",
            "dangling",
        ),
        (
            "composition.arxml",
            "<SHORT-NAME>ProcessInstance</SHORT-NAME>",
            "<SHORT-NAME>IngressInstance</SHORT-NAME>",
            "duplicate instance",
        ),
        (
            "process.arxml",
            "<SHORT-NAME>Process</SHORT-NAME>",
            "<SHORT-NAME>ingress</SHORT-NAME>",
            "case collision",
        ),
        (
            "process.arxml",
            "/Types/ApplicationTypes</DATA-TYPE-MAPPING-REF>",
            "/Types/MissingMapping</DATA-TYPE-MAPPING-REF>",
            "mapping",
        ),
        (
            "types.arxml",
            "<DIRECTION>INOUT</DIRECTION>",
            "<DIRECTION>INVALID</DIRECTION>",
            "parameter direction",
        ),
        (
            "types.arxml",
            "<ARRAY-SIZE>4</ARRAY-SIZE>",
            "<ARRAY-SIZE>5</ARRAY-SIZE>",
            "array length",
        ),
        (
            "process.arxml",
            "<ALIVE-TIMEOUT>0</ALIVE-TIMEOUT>",
            "<ALIVE-TIMEOUT>1e-999</ALIVE-TIMEOUT>",
            "timeout underflow",
        ),
        (
            "process.arxml",
            "<ALIVE-TIMEOUT>0</ALIVE-TIMEOUT>",
            "<ALIVE-TIMEOUT>invalid</ALIVE-TIMEOUT>",
            "malformed timeout",
        ),
        (
            "process.arxml",
            "<HANDLE-NEVER-RECEIVED>false</HANDLE-NEVER-RECEIVED>",
            "<HANDLE-NEVER-RECEIVED>true</HANDLE-NEVER-RECEIVED>",
            "local freshness",
        ),
        (
            "process.arxml",
            "<VALUE>7</VALUE>",
            "<VALUE>-1</VALUE>",
            "initial value",
        ),
        (
            "process.arxml",
            "<PERIOD>0.01</PERIOD>",
            "<PERIOD>0.02</PERIOD>",
            "period mismatch",
        ),
        (
            "ecuc.arxml",
            "<VALUE>5</VALUE>",
            "<VALUE>4</VALUE>",
            "position",
        ),
        (
            "ecuc.arxml",
            "RteEventIsMappedToTask</DEFINITION-REF>\n                      <VALUE>false</VALUE>",
            "RteEventIsMappedToTask</DEFINITION-REF>\n                      <VALUE>true</VALUE>",
            "server task mapping",
        ),
    ];
    for (file, before, after, label) in vectors {
        let mut sources = inputs();
        change(&mut sources, file, before, after);
        let original = sources
            .iter()
            .map(|source| source.bytes().to_vec())
            .collect::<Vec<_>>();
        let issues = build(&sources)
            .err()
            .unwrap_or_else(|| panic!("{label} accepted"));
        assert!(
            issues
                .iter()
                .any(|issue| issue.file.is_some() && issue.object.is_some()),
            "{label}: {issues:?}"
        );
        assert_eq!(
            original,
            sources
                .iter()
                .map(|source| source.bytes().to_vec())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn collisions_are_rejected_with_all_references_still_valid() {
    let vectors = [
        ("Dcm_DataElement_ApplicationValueType", "boolean"),
        (
            "Dcm_DataElement_ApplicationValueType",
            "Std_VersionInfoType",
        ),
        ("Dcm_DataElement_ApplicationValueType", "UINT32_MAX"),
        ("Dcm_DataElement_ApplicationValueType", "uint_fast32_t"),
        ("Process", "ingress"),
        ("Process_Periodic", "Rte_Read_Value_Value"),
        ("Process_Periodic", "Rte_COMCbk"),
        ("Process_Periodic", "Rte_COMCbkRxTOut"),
        ("Process_Periodic", "Com_MainFunctionRx_Rx"),
        ("Process_Periodic", "Com_MainFunctionTx_Tx"),
        (
            "<SHORT-NAME>Input</SHORT-NAME>",
            "<SHORT-NAME>uint32</SHORT-NAME>",
        ),
        (
            "<SHORT-NAME>Input</SHORT-NAME>",
            "<SHORT-NAME>Std_ReturnType</SHORT-NAME>",
        ),
        (
            "<SHORT-NAME>Input</SHORT-NAME>",
            "<SHORT-NAME>E_OK</SHORT-NAME>",
        ),
    ];
    for (before, after) in vectors {
        let mut sources = inputs();
        if before == "Process" {
            replace_all(&mut sources, "/Application/Process", "/Application/ingress");
            replace_all(
                &mut sources,
                "<SHORT-NAME>Process</SHORT-NAME>",
                "<SHORT-NAME>ingress</SHORT-NAME>",
            );
        } else {
            replace_all(&mut sources, before, after);
        }
        let issues = build(&sources)
            .err()
            .unwrap_or_else(|| panic!("{after} accepted"));
        assert!(
            issues.iter().any(|issue| matches!(
                issue.code.as_str(),
                "CONTRACT_NAME_COLLISION"
                    | "SYMBOL_NORMALIZATION_COLLISION"
                    | "SYMBOL_PRODUCER_DUPLICATE"
            )),
            "{after}: {issues:?}"
        );
        assert!(
            !issues
                .iter()
                .any(|issue| issue.code == "REFERENCE_UNRESOLVED"),
            "{after}: dangling references mask collision"
        );
    }
}

#[test]
fn service_argument_names_cannot_shadow_selected_runtime_and_component_producers() {
    let baseline = build(&inputs()).unwrap();
    let bridge = baseline
        .description()
        .composition
        .as_ref()
        .unwrap()
        .components
        .iter()
        .flat_map(|component| &component.operations)
        .find(|operation| operation.read && operation.implementation_symbol.contains("DcmService"))
        .unwrap()
        .implementation_symbol
        .clone();
    for name in [
        "Ecu_TargetIsOwner",
        "Process_Transform",
        "DcmService_ReadData",
        &bridge,
        "RTE_INGRESS_H",
        "NULL_PTR",
        "RTE_Process_CODE",
        "RTE_MEMMAP_ACTIVE",
        "RTE_Process_CODE_ACTIVE",
        "RTE_MEMMAP_HEADER_CHECK",
        "COM_H",
        "ECU_TARGET_H",
        "COM_SERVICE_NOT_AVAILABLE",
        "STD_ON",
        "COMSTACK_TYPES_H",
        "ECU_TARGET_READY",
        "RTE_Ingress_VAR_CLEARED_UNSPECIFIED",
    ] {
        for reverse in [false, true] {
            let mut sources = inputs();
            change(
                &mut sources,
                "types.arxml",
                "<SHORT-NAME>Input</SHORT-NAME>",
                &format!("<SHORT-NAME>{name}</SHORT-NAME>"),
            );
            if reverse {
                sources.reverse();
            }
            rejects_in_both(&sources, "CONTRACT_NAME_COLLISION");
            if name == "Ecu_TargetIsOwner" && !reverse {
                assert_rejected_multi_sources_preserved(&sources);
            }
            assert!(
                !normal_validation(&sources)
                    .diagnostics
                    .iter()
                    .any(|issue| issue.code == "REFERENCE_UNRESOLVED")
            );
        }
    }
}

#[test]
fn service_argument_runtime_like_prefixes_without_producers_remain_legal() {
    for name in ["Ecu_sample", "Os_sample", "xTask_sample"] {
        let mut sources = inputs();
        change(
            &mut sources,
            "types.arxml",
            "<SHORT-NAME>Input</SHORT-NAME>",
            &format!("<SHORT-NAME>{name}</SHORT-NAME>"),
        );
        assert!(normal_validation(&sources).diagnostics.is_empty());
        build(&sources).unwrap();
    }
}

#[test]
fn missing_and_duplicate_relations_are_rejected_before_emission() {
    let cases = [
        ("process.arxml", "INIT-VALUE", None, false),
        ("process.arxml", "DATA-TYPE-MAPPING-REFS", None, false),
        ("process.arxml", "TIMING-EVENT", Some("Periodic10ms"), false),
        ("process.arxml", "TIMING-EVENT", Some("Periodic10ms"), true),
        ("ecuc.arxml", "ECUC-CONTAINER-VALUE", Some("Process"), false),
        (
            "ecuc.arxml",
            "ECUC-CONTAINER-VALUE",
            Some("ReadApplicationValue"),
            false,
        ),
        (
            "composition.arxml",
            "ASSEMBLY-SW-CONNECTOR",
            Some("IngressProcess"),
            false,
        ),
        (
            "composition.arxml",
            "ASSEMBLY-SW-CONNECTOR",
            Some("IngressProcess"),
            true,
        ),
    ];
    for (file, tag, short_name, duplicate) in cases {
        let mut sources = inputs();
        let source = sources
            .iter_mut()
            .find(|source| source.logical_path() == file)
            .unwrap();
        let text = std::str::from_utf8(source.bytes()).unwrap();
        let document = roxmltree::Document::parse(text).unwrap();
        let node = document
            .descendants()
            .find(|node| {
                node.tag_name().name() == tag
                    && short_name.is_none_or(|name| {
                        node.children().any(|child| {
                            child.tag_name().name() == "SHORT-NAME" && child.text() == Some(name)
                        })
                    })
            })
            .unwrap();
        let range = node.range();
        let replacement = if duplicate {
            format!(
                "{}{}",
                &text[range.clone()],
                text[range.clone()].replacen(
                    &format!("<SHORT-NAME>{}</SHORT-NAME>", short_name.unwrap()),
                    "<SHORT-NAME>Duplicate</SHORT-NAME>",
                    1
                )
            )
        } else {
            String::new()
        };
        let mutated = format!(
            "{}{}{}",
            &text[..range.start],
            replacement,
            &text[range.end..]
        );
        *source = InputSource::new(file, mutated.into_bytes()).unwrap();
        let issues = build(&sources)
            .err()
            .unwrap_or_else(|| panic!("{file}/{tag} duplicate={duplicate} accepted"));
        assert!(
            issues
                .iter()
                .any(|issue| issue.file.is_some() && issue.object.is_some()),
            "{issues:?}"
        );
    }
}

#[test]
fn renamed_component_and_port_contracts_follow_full_source_identity() {
    let mut sources = inputs();
    replace_all(&mut sources, "Ingress", "Gateway");
    replace_all(
        &mut sources,
        "/Application/Process/Value",
        "/Application/Process/InputValue",
    );
    change(
        &mut sources,
        "process.arxml",
        "<SHORT-NAME>Value</SHORT-NAME>",
        "<SHORT-NAME>InputValue</SHORT-NAME>",
    );
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    let composition = plan.description().composition.as_ref().unwrap();
    let gateway = composition
        .components
        .iter()
        .find(|component| component.component == "/Application/Gateway")
        .unwrap();
    assert_eq!(gateway.instance, "/Application/Pipeline/GatewayInstance");
    assert_eq!(gateway.header, "include/Rte_Gateway.h");
    let process = composition
        .components
        .iter()
        .find(|component| component.component == "/Application/Process")
        .unwrap();
    assert_eq!(
        process
            .data_ports
            .iter()
            .find(|port| port.read)
            .unwrap()
            .api_symbol,
        "Rte_Read_InputValue_Value"
    );
    assert!(
        composition
            .connections
            .iter()
            .any(|connection| connection.requester.port == "/Application/Process/InputValue")
    );
    assert!(
        plan.component_contract_files()
            .unwrap()
            .files()
            .iter()
            .any(|(file, _)| file == "include/Rte_Gateway.h")
    );
}

#[test]
fn every_call_point_and_actual_server_recursion_are_checked() {
    let mut sources = inputs();
    let call = subtree(
        &sources,
        "observe.arxml",
        "SYNCHRONOUS-SERVER-CALL-POINT",
        "CallTransform",
    );
    let foreign = call
        .replace("CallTransform", "ForeignCall")
        .replace(
            "/Application/Observe/ResultService",
            "/Services/DcmService/DataServices_ApplicationValue",
        )
        .replace(
            "/Types/ScalarService/Transform",
            "/Types/DataServices_ApplicationValue/ReadData",
        );
    change(
        &mut sources,
        "observe.arxml",
        "</SERVER-CALL-POINTS>",
        &format!("{foreign}</SERVER-CALL-POINTS>"),
    );
    let issues = build(&sources)
        .err()
        .expect("Foreign second call point accepted");
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "CALL_POINT_OWNERSHIP"),
        "{issues:?}"
    );

    let mut sources = inputs();
    let port = subtree(
        &sources,
        "observe.arxml",
        "R-PORT-PROTOTYPE",
        "ResultService",
    )
    .replace(
        "<SHORT-NAME>ResultService</SHORT-NAME>",
        "<SHORT-NAME>LoopClient</SHORT-NAME>",
    );
    change(
        &mut sources,
        "process.arxml",
        "</PORTS>",
        &format!("{port}</PORTS>"),
    );
    let recursive_call = call.replace("CallTransform", "RecursiveCall").replace(
        "/Application/Observe/ResultService",
        "/Application/Process/LoopClient",
    );
    change(
        &mut sources,
        "process.arxml",
        "<SYMBOL>Process_Transform</SYMBOL>",
        &format!(
            "<SERVER-CALL-POINTS>{recursive_call}</SERVER-CALL-POINTS><SYMBOL>Process_Transform</SYMBOL>"
        ),
    );
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "Scalar",
    )
    .replace(
        "<SHORT-NAME>Scalar</SHORT-NAME>",
        "<SHORT-NAME>Recursive</SHORT-NAME>",
    )
    .replace(
        "/Application/Pipeline/ObserveInstance",
        "/Application/Pipeline/ProcessInstance",
    )
    .replace(
        "/Application/Observe/ResultService",
        "/Application/Process/LoopClient",
    );
    change(
        &mut sources,
        "composition.arxml",
        "</CONNECTORS>",
        &format!("{connector}</CONNECTORS>"),
    );
    let issues = build(&sources)
        .err()
        .expect("Recursive server call accepted");
    assert!(
        issues.iter().any(|issue| issue.code == "SERVICE_RECURSION"),
        "{issues:?}"
    );
    assert!(
        normal_validation(&sources)
            .diagnostics
            .iter()
            .any(|issue| issue.code == "SERVICE_RECURSION")
    );
}

#[test]
fn equal_width_implementation_types_and_network_local_double_binding_are_rejected() {
    let mut sources = inputs();
    let implementation = subtree(
        &sources,
        "types.arxml",
        "IMPLEMENTATION-DATA-TYPE",
        "uint32",
    )
    .replace(
        "<SHORT-NAME>uint32</SHORT-NAME>",
        "<SHORT-NAME>OtherUint32</SHORT-NAME>",
    );
    let mapping = subtree(
        &sources,
        "types.arxml",
        "DATA-TYPE-MAPPING-SET",
        "ApplicationTypes",
    )
    .replace(
        "<SHORT-NAME>ApplicationTypes</SHORT-NAME>",
        "<SHORT-NAME>OtherTypes</SHORT-NAME>",
    )
    .replace(
        "/Types/uint32</IMPLEMENTATION-DATA-TYPE-REF>",
        "/Types/OtherUint32</IMPLEMENTATION-DATA-TYPE-REF>",
    );
    change(
        &mut sources,
        "types.arxml",
        "</ELEMENTS>",
        &format!("{implementation}{mapping}</ELEMENTS>"),
    );
    change(
        &mut sources,
        "process.arxml",
        "/Types/ApplicationTypes</DATA-TYPE-MAPPING-REF>",
        "/Types/OtherTypes</DATA-TYPE-MAPPING-REF>",
    );
    let issues = build(&sources)
        .err()
        .expect("Equal-width distinct implementation types accepted");
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "TYPE_CONFLICT" || issue.code == "SERVICE_TYPE_CONFLICT"),
        "{issues:?}"
    );
    assert!(
        !issues
            .iter()
            .any(|issue| issue.code == "REFERENCE_UNRESOLVED")
    );

    let mut sources = inputs();
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "IngressProcess",
    )
    .replace(
        "/Application/Pipeline/ProcessInstance",
        "/Application/Pipeline/IngressInstance",
    )
    .replace("/Application/Process/Value", "/Application/Ingress/RxValue");
    let original = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "IngressProcess",
    );
    change(&mut sources, "composition.arxml", &original, &connector);
    let issues = build(&sources)
        .err()
        .expect("Local and network producer accepted");
    assert!(
        issues.iter().any(|issue| issue.code == "ENDPOINT_BINDING"),
        "{issues:?}"
    );
    assert!(
        normal_validation(&sources)
            .diagnostics
            .iter()
            .any(|issue| issue.code == "ENDPOINT_BINDING")
    );
}

#[test]
fn unsigned_contract_rejects_conflicting_base_encodings_with_valid_references() {
    for native in ["uint8", "uint32"] {
        for encoding in ["2C", "IEEE754"] {
            let mut sources = inputs();
            change(
                &mut sources,
                "types.arxml",
                &format!(
                    "<BASE-TYPE-ENCODING>NONE</BASE-TYPE-ENCODING>\n          <NATIVE-DECLARATION>{native}</NATIVE-DECLARATION>"
                ),
                &format!(
                    "<BASE-TYPE-ENCODING>{encoding}</BASE-TYPE-ENCODING>\n          <NATIVE-DECLARATION>{native}</NATIVE-DECLARATION>"
                ),
            );
            let issues = build(&sources)
                .err()
                .expect("conflicting encoding rejected");
            assert!(
                issues.iter().any(|issue| issue.code == "TYPE_CONFLICT"),
                "{issues:?}"
            );
            assert!(
                !issues.iter().any(|issue| issue.code.contains("UNRESOLVED")),
                "{issues:?}"
            );
        }
    }
}

#[test]
fn client_server_sets_close_in_both_directions_and_empty_interfaces_are_rejected() {
    for file in ["process.arxml", "observe.arxml"] {
        for foreign in [false, true] {
            let mut sources = inputs();
            let tag = if file == "process.arxml" {
                "SERVER-COM-SPEC"
            } else {
                "CLIENT-COM-SPEC"
            };
            xml_edit(
                &mut sources,
                file,
                |node| node.tag_name().name() == tag,
                |original| {
                    format!(
                        "{original}{}",
                        if foreign {
                            original.replace(
                                "/Types/ScalarService/Transform",
                                "/Types/DataServices_ApplicationValue/ReadData",
                            )
                        } else {
                            original.into()
                        }
                    )
                },
            );
            rejects_in_both(&sources, "SERVICE_TYPE_CONFLICT");
        }
    }
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "process.arxml",
        |node| named(node, "OPERATION-INVOKED-EVENT", "InvokeTransform"),
        |original| {
            format!(
                "{original}<OPERATION-INVOKED-EVENT><SHORT-NAME>InvokeUnbound</SHORT-NAME><START-ON-EVENT-REF DEST=\"RUNNABLE-ENTITY\">/Application/Process/Behavior/Unbound</START-ON-EVENT-REF><OPERATION-IREF><CONTEXT-P-PORT-REF DEST=\"P-PORT-PROTOTYPE\">/Application/Process/ResultService</CONTEXT-P-PORT-REF><TARGET-PROVIDED-OPERATION-REF DEST=\"CLIENT-SERVER-OPERATION\">/Types/DataServices_ApplicationValue/ReadData</TARGET-PROVIDED-OPERATION-REF></OPERATION-IREF></OPERATION-INVOKED-EVENT>"
            )
        },
    );
    change(
        &mut sources,
        "process.arxml",
        "</RUNNABLES>",
        "<RUNNABLE-ENTITY><SHORT-NAME>Unbound</SHORT-NAME><CAN-BE-INVOKED-CONCURRENTLY>false</CAN-BE-INVOKED-CONCURRENTLY><SYMBOL>Process_Unbound</SYMBOL></RUNNABLE-ENTITY></RUNNABLES>",
    );
    rejects_in_both(&sources, "EVENT_OWNERSHIP");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "types.arxml",
        |node| named(node, "CLIENT-SERVER-INTERFACE", "ScalarService"),
        |_| {
            "<CLIENT-SERVER-INTERFACE><SHORT-NAME>ScalarService</SHORT-NAME><IS-SERVICE>false</IS-SERVICE></CLIENT-SERVER-INTERFACE>".into()
        },
    );
    for (file, tag) in [
        ("process.arxml", "PROVIDED-COM-SPECS"),
        ("observe.arxml", "REQUIRED-COM-SPECS"),
        ("process.arxml", "OPERATION-IREF"),
        ("observe.arxml", "OPERATION-IREF"),
    ] {
        xml_edit(
            &mut sources,
            file,
            |node| {
                node.tag_name().name() == tag
                    && (tag == "OPERATION-IREF"
                        || node.ancestors().any(|ancestor| {
                            named(ancestor, "P-PORT-PROTOTYPE", "ResultService")
                                || named(ancestor, "R-PORT-PROTOTYPE", "ResultService")
                        }))
            },
            |_| String::new(),
        );
    }
    rejects_in_both(&sources, "SERVICE_TYPE_CONFLICT");
}

#[test]
fn every_operation_has_its_own_spec_call_and_event_and_void_is_valid() {
    let sources = multi_operation_inputs();
    assert!(normal_validation(&sources).diagnostics.is_empty());
    let plan = build(&sources).unwrap();
    let files = plan.component_contract_files().unwrap();
    assert!(
        std::str::from_utf8(
            &files
                .files()
                .iter()
                .find(|(path, _)| path == "include/Rte_Process.h")
                .unwrap()
                .1
        )
        .unwrap()
        .contains("void Process_Ping(void);")
    );
    for (file, tag, code) in [
        ("process.arxml", "SERVER-COM-SPEC", "SERVICE_TYPE_CONFLICT"),
        ("observe.arxml", "CLIENT-COM-SPEC", "SERVICE_TYPE_CONFLICT"),
        (
            "observe.arxml",
            "SYNCHRONOUS-SERVER-CALL-POINT",
            "SERVICE_CLIENT_MISSING",
        ),
        (
            "process.arxml",
            "OPERATION-INVOKED-EVENT",
            "SERVICE_CLIENT_MISSING",
        ),
    ] {
        let mut changed = sources.clone();
        xml_edit(
            &mut changed,
            file,
            |node| {
                node.tag_name().name() == tag
                    && node.descendants().any(|child| {
                        child
                            .text()
                            .is_some_and(|text| text.ends_with("/Ping") || text == "InvokePing")
                    })
            },
            |_| String::new(),
        );
        if tag == "OPERATION-INVOKED-EVENT" {
            xml_edit(
                &mut changed,
                "ecuc.arxml",
                |node| named(node, "ECUC-CONTAINER-VALUE", "InvokePing"),
                |_| String::new(),
            );
            xml_edit(
                &mut changed,
                "process.arxml",
                |node| named(node, "RUNNABLE-ENTITY", "ServerPing"),
                |_| String::new(),
            );
        }
        rejects_in_both(&changed, code);
    }
}

#[test]
fn argument_policy_explicit_task_flags_offset_and_concurrency_are_checked() {
    let mut sources = inputs();
    change(&mut sources, "types.arxml", "USE-ARGUMENT-TYPE", "USE-VOID");
    rejects_in_both(&sources, "SERVICE_TYPE_CONFLICT");
    for prefix in ["Rte", "RteBsw"] {
        for remove in [false, true] {
            let mut sources = inputs();
            xml_edit(
                &mut sources,
                "ecuc.arxml",
                |node| {
                    node.tag_name().name() == "ECUC-NUMERICAL-PARAM-VALUE"
                        && node.children().any(|child| {
                            child.text().is_some_and(|text| {
                                text.ends_with(&format!("/{prefix}EventIsMappedToTask"))
                            })
                        })
                        && node.children().any(|child| child.text() == Some("true"))
                },
                |original| {
                    if remove {
                        String::new()
                    } else {
                        original.replace("<VALUE>true</VALUE>", "<VALUE>false</VALUE>")
                    }
                },
            );
            rejects_in_both(&sources, "INSTANCE_MAPPING");
        }
    }
    for name in ["Periodic", "ServerTransform"] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "process.arxml",
            |node| named(node, "RUNNABLE-ENTITY", name),
            |original| {
                original.replace(
                    "<CAN-BE-INVOKED-CONCURRENTLY>false",
                    "<CAN-BE-INVOKED-CONCURRENTLY>true",
                )
            },
        );
        rejects_in_both(&sources, "REENTRANCY_UNSUPPORTED");
    }
    let mut sources = inputs();
    change(
        &mut sources,
        "process.arxml",
        "<PERIOD>0.01</PERIOD>",
        "<OFFSET>0</OFFSET><PERIOD>0.01</PERIOD>",
    );
    assert!(build(&sources).is_ok());
    assert!(normal_validation(&sources).diagnostics.is_empty());
    change(
        &mut sources,
        "process.arxml",
        "<OFFSET>0</OFFSET>",
        "<OFFSET>0.001</OFFSET>",
    );
    rejects_in_both(&sources, "PERIOD_UNSUPPORTED");
}

#[test]
fn contract_provenance_preserves_complete_rule_and_extension_identities() {
    let plan = build(&inputs()).unwrap();
    let files = plan.component_contract_files().unwrap();
    let json: serde_json::Value = serde_json::from_slice(
        &files
            .files()
            .iter()
            .find(|(path, _)| path == "contract.json")
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(
        json["ruleSetIdentity"],
        serde_json::to_value(&plan.description().rule_set_identity).unwrap()
    );
    assert_eq!(
        json["requiredExtensionDefinitions"],
        serde_json::to_value(&plan.description().required_extension_definitions).unwrap()
    );
    assert_eq!(
        json["validationDependencies"],
        serde_json::to_value(&plan.description().validation_dependencies).unwrap()
    );
}

#[test]
fn type_categories_are_required_with_valid_references_in_both_entrances() {
    for category in [None, Some("VALUE")] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "types.arxml",
            |node| node.tag_name().name() == "IMPLEMENTATION-DATA-TYPE-ELEMENT",
            |original| {
                original.replace(
                    "<CATEGORY>TYPE_REFERENCE</CATEGORY>",
                    &category
                        .map(|value| format!("<CATEGORY>{value}</CATEGORY>"))
                        .unwrap_or_default(),
                )
            },
        );
        rejects_in_both(&sources, "SERVICE_TYPE_CONFLICT");
    }
    for base in ["Base_uint8", "Base_uint32"] {
        for category in [None, Some("VARIABLE_LENGTH")] {
            let mut sources = inputs();
            xml_edit(
                &mut sources,
                "types.arxml",
                |node| named(node, "SW-BASE-TYPE", base),
                |original| {
                    original.replace(
                        "<CATEGORY>FIXED_LENGTH</CATEGORY>",
                        &category
                            .map(|value| format!("<CATEGORY>{value}</CATEGORY>"))
                            .unwrap_or_default(),
                    )
                },
            );
            rejects_in_both(&sources, "TYPE_CONFLICT");
        }
    }
}

#[test]
fn serialized_symbol_consumers_include_all_runnable_accesses_and_client_callers() {
    let mut sources = inputs();
    let accesses = format!(
        "<DATA-RECEIVE-POINT-BY-ARGUMENTS>{}</DATA-RECEIVE-POINT-BY-ARGUMENTS>",
        subtree(&sources, "process.arxml", "VARIABLE-ACCESS", "RValue")
    );
    xml_edit(
        &mut sources,
        "process.arxml",
        |node| named(node, "RUNNABLE-ENTITY", "ServerTransform"),
        |original| original.replace("<SYMBOL>", &format!("{accesses}<SYMBOL>")),
    );
    xml_edit(
        &mut sources,
        "observe.arxml",
        |node| named(node, "RUNNABLE-ENTITY", "Periodic"),
        |original| {
            format!(
                "{original}{}",
                original
                    .replace(
                        "<SHORT-NAME>Periodic</SHORT-NAME>",
                        "<SHORT-NAME>ExtraPeriodic</SHORT-NAME>"
                    )
                    .replace(
                        "<SYMBOL>Observe_Periodic</SYMBOL>",
                        "<SYMBOL>Observe_ExtraPeriodic</SYMBOL>"
                    )
            )
        },
    );
    xml_edit(
        &mut sources,
        "observe.arxml",
        |node| named(node, "TIMING-EVENT", "Periodic10ms"),
        |original| {
            format!(
                "{original}{}",
                original
                    .replace("Periodic10ms", "Extra10ms")
                    .replace("/Behavior/Periodic", "/Behavior/ExtraPeriodic")
            )
        },
    );
    parameter(&mut sources, "Transmit10ms", "RteBswPositionInTask", "8");
    parameter(&mut sources, "Dcm", "RteBswPositionInTask", "9");
    for (name, position) in [
        ("Can_MainFunction_Read", "10"),
        ("Can_MainFunction_Write", "11"),
        ("Can_MainFunction_Mode", "12"),
        ("Can_MainFunction_BusOff", "13"),
    ] {
        parameter(&mut sources, name, "RteBswPositionInTask", position);
    }
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "Periodic10ms")
                && node
                    .ancestors()
                    .any(|ancestor| named(ancestor, "ECUC-CONTAINER-VALUE", "Observe"))
        },
        |original| {
            format!(
                "{original}{}",
                original
                    .replace(
                        "<SHORT-NAME>Periodic10ms</SHORT-NAME>",
                        "<SHORT-NAME>Extra10ms</SHORT-NAME>"
                    )
                    .replace(
                        "/Observe/Behavior/Periodic10ms",
                        "/Observe/Behavior/Extra10ms"
                    )
                    .replace("<VALUE>6</VALUE>", "<VALUE>7</VALUE>")
            )
        },
    );
    assert!(normal_validation(&sources).diagnostics.is_empty());
    let files = build(&sources).unwrap().component_contract_files().unwrap();
    let json: serde_json::Value = serde_json::from_slice(
        &files
            .files()
            .iter()
            .find(|(file, _)| file == "contract.json")
            .unwrap()
            .1,
    )
    .unwrap();
    for (symbol, expected) in [
        (
            "Rte_Application_Pipeline_ProcessInstance_Read_Value_Value",
            vec![
                "/Application/Process/Behavior/Periodic",
                "/Application/Process/Behavior/ServerTransform",
            ],
        ),
        (
            "Rte_Application_Pipeline_ObserveInstance_Call_ResultService_Transform",
            vec![
                "/Application/Observe/Behavior/ExtraPeriodic",
                "/Application/Observe/Behavior/Periodic",
            ],
        ),
    ] {
        let actual = json["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["symbol"] == symbol)
            .unwrap()["consumers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry.as_str().unwrap())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual, expected.into_iter().collect());
    }
}

#[test]
fn actual_runtime_source_owners_derive_all_application_slots_and_keep_service_generated() {
    let plan = build(&inputs()).unwrap();
    let slots = plan.application_slot_descriptors().unwrap();
    assert_eq!(slots.len(), 3);
    assert_eq!(
        slots
            .iter()
            .map(|slot| slot.source_paths[0].as_str())
            .collect::<Vec<_>>(),
        [
            "application/Ingress.c",
            "application/Process.c",
            "application/Observe.c"
        ]
    );
    for slot in &slots {
        assert!(
            slot.producer_slot
                .starts_with("singlecore-ecu-v1:/Application/Pipeline/")
        );
        assert!(!slot.entry_symbols.is_empty());
        assert!(
            slot.entry_symbols.iter().all(|entry| plan
                .description()
                .symbols
                .iter()
                .any(|symbol| symbol.symbol == *entry
                    && symbol.definition_owner == slot.source_paths[0]))
        );
    }
    assert!(
        plan.description()
            .symbols
            .iter()
            .any(|symbol| symbol.symbol == "DcmService_ReadData"
                && symbol.definition_owner == "src/Rte.c")
    );
    let files = prepared_multi_files(&plan);
    assert!(files.iter().any(|(path, _)| path == "src/Rte.c"));
}

#[test]
fn component_memory_scopes_cannot_alias_selected_runtime_producers() {
    for name in [
        "Rte",
        "Ecu",
        "Com",
        "ComM",
        "Can",
        "CanIf",
        "CanTp",
        "Dcm",
        "PduR",
        "LSduR",
        "BswM",
        "Ecu_HostBusSM",
    ] {
        let mut sources = inputs();
        replace_all(
            &mut sources,
            "/Application/Process",
            &format!("/Application/{name}"),
        );
        change(
            &mut sources,
            "process.arxml",
            "<SHORT-NAME>Process</SHORT-NAME>",
            &format!("<SHORT-NAME>{name}</SHORT-NAME>"),
        );
        rejects_in_both(&sources, "CONTRACT_NAME_COLLISION");
        let mut partition_sources = inputs();
        replace_all(&mut partition_sources, "OwnerPartition", name);
        rejects_in_both(&partition_sources, "CONTRACT_NAME_COLLISION");
    }
}

#[test]
fn shared_rte_lifecycle_os_target_kernel_and_c_runtime_producers_cannot_be_shadowed() {
    for symbol in [
        "Rte_Start",
        "Rte_Stop",
        "SchM_Init",
        "SchM_StartTiming",
        "Dcm_GetSesCtrlType",
        "CanTp_CancelReceive",
        "StartOS",
        "ShutdownOS",
        "Ecu_TargetTask",
        "xTaskCreate",
        "Os_TargetPrepare",
        "Arti_Trace",
        "main",
        "memcpy",
        "printf",
        "abort",
        "TaskType",
        "SCHEDULETABLE_STOPPED",
    ] {
        let mut sources = inputs();
        change(
            &mut sources,
            "ingress.arxml",
            "<SYMBOL>Ingress_Periodic</SYMBOL>",
            &format!("<SYMBOL>{symbol}</SYMBOL>"),
        );
        rejects_in_both(&sources, "SYMBOL_PRODUCER_DUPLICATE");
    }
}

#[test]
fn multi_selected_catalog_binds_standard_signatures_and_actual_periodic_producers() {
    let plan = build(&inputs()).unwrap();
    let description = plan.description();
    assert_eq!(
        description
            .schedule
            .entities
            .iter()
            .map(|entity| (entity.position, entity.symbol.as_str()))
            .collect::<Vec<_>>(),
        [
            (0, "ComM_MainFunction_Host"),
            (1, "Can_MainFunction_Wakeup"),
            (2, "CanTp_MainFunction"),
            (3, "Com_MainFunctionRx_Rx"),
            (4, "Ingress_Periodic"),
            (5, "Process_Periodic"),
            (6, "Observe_Periodic"),
            (7, "Com_MainFunctionTx_Tx"),
            (8, "Dcm_MainFunction"),
            (9, "Can_MainFunction_Read"),
            (10, "Can_MainFunction_Write"),
            (11, "Can_MainFunction_Mode"),
            (12, "Can_MainFunction_BusOff"),
        ]
    );
    assert!(
        description
            .runtime_sources
            .contains_key("runtime/bsw/src/Com.c")
    );
    assert!(
        description
            .runtime_sources
            .contains_key("core/src/integration/ecu_config.rs")
    );
    for name in [
        "Com_MainFunctionRx_Rx",
        "Com_MainFunctionTx_Tx",
        "ComM_MainFunction_Host",
    ] {
        let symbol = description
            .symbols
            .iter()
            .find(|symbol| symbol.symbol == name)
            .unwrap();
        assert_eq!(symbol.return_type, "void");
        assert!(symbol.arguments.is_empty());
        assert_eq!(symbol.definition_owner, "src/Ecu_RuntimeConfig.c");
    }
    let transmit = description
        .symbols
        .iter()
        .find(|symbol| symbol.symbol == "CanIf_Transmit")
        .unwrap();
    assert_eq!(transmit.return_type, "Std_ReturnType");
    assert_eq!(
        transmit
            .arguments
            .iter()
            .map(|argument| argument.native_type.as_str())
            .collect::<Vec<_>>(),
        ["PduIdType", "const PduInfoType *"]
    );
    for (name, field, old, new) in [
        ("Com_SendSignal", "IS-SYNCHRONOUS", "false", "true"),
        ("Com_ReceiveSignal", "IS-REENTRANT", "true", "false"),
        ("CanIf_Transmit", "IS-REENTRANT", "true", "false"),
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "bsw.arxml",
            |node| named(node, "BSW-MODULE-ENTRY", name),
            |text| {
                text.replace(
                    &format!("<{field}>{old}</{field}>"),
                    &format!("<{field}>{new}</{field}>"),
                )
            },
        );
        rejects_in_both(&sources, "BSW_SIGNATURE_CONFLICT");
    }
    for (field, value) in [
        ("IS-REENTRANT", "false"),
        ("IS-SYNCHRONOUS", "true"),
        ("CALL-TYPE", "REGULAR"),
        ("EXECUTION-CONTEXT", "UNSPECIFIED"),
        ("SW-SERVICE-IMPL-POLICY", "STANDARD"),
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "bsw.arxml",
            |node| named(node, "BSW-MODULE-ENTRY", "Can_Init"),
            |text| text.replace(&format!("<{field}>{value}</{field}>"), ""),
        );
        rejects_in_both(&sources, "BSW_SIGNATURE_CONFLICT");
    }
    for (field, old, new) in [
        ("CALL-TYPE", "REGULAR", "CALLBACK"),
        ("SW-SERVICE-IMPL-POLICY", "STANDARD", "MACRO"),
        ("BSW-ENTRY-KIND", "CONCRETE", "ABSTRACT"),
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "bsw.arxml",
            |node| named(node, "BSW-MODULE-ENTRY", "Can_Init"),
            |text| {
                text.replace(
                    &format!("<{field}>{old}</{field}>"),
                    &format!("<{field}>{new}</{field}>"),
                )
            },
        );
        rejects_in_both(&sources, "BSW_SIGNATURE_CONFLICT");
    }
    for (old, new) in [
        ("<MODULE-ID>80</MODULE-ID>", "<MODULE-ID>60</MODULE-ID>"),
        ("<CATEGORY>BSW_MODULE</CATEGORY>", "<CATEGORY></CATEGORY>"),
    ] {
        let mut sources = inputs();
        change(&mut sources, "bsw.arxml", old, new);
        rejects_in_both(&sources, "BSW_ENTRY_IDENTITY");
    }
    let pdu_router = description
        .symbols
        .iter()
        .find(|symbol| symbol.symbol == "PduR_ComTransmit")
        .unwrap();
    assert_eq!(
        pdu_router.declaration_owner,
        "runtime/bsw/include/PduR_Com.h"
    );
    for path in [
        "core/src/integration/ecu.rs",
        "core/src/integration/contracts.rs",
    ] {
        assert!(description.runtime_sources.contains_key(path));
    }
    let mut sources = inputs();
    replace_all(
        &mut sources,
        "AUTOSAR_Com</SHORT-NAME>",
        "Vendor_Com</SHORT-NAME>",
    );
    replace_all(&mut sources, "/AUTOSAR_Com/", "/Vendor_Com/");
    rejects_in_both(&sources, "BSW_ENTRY_IDENTITY");
}

#[test]
fn plural_application_preparation_freezes_every_real_producer_and_refuses_bad_members() {
    use autosar_config_core::{ApplicationSource, prepare_ecu_project_with_applications};
    let scratch = Scratch::new();
    let plan = build(&inputs()).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi-application");
    let sources: Vec<_> = plan
        .application_slot_descriptors()
        .unwrap()
        .iter()
        .map(|slot| {
            let file = slot.source_paths[0].rsplit('/').next().unwrap();
            let path = scratch.0.join(file);
            std::fs::copy(fixture.join(file), &path).unwrap();
            ApplicationSource {
                component_instance: slot
                    .producer_slot
                    .strip_prefix("singlecore-ecu-v1:")
                    .unwrap()
                    .into(),
                path,
            }
        })
        .collect();
    let prepare = |members: &[ApplicationSource]| {
        prepare_ecu_project_with_applications(
            &plan,
            autosar_config_core::target::BuildTarget::LinuxX64ControlledV1,
            members,
        )
    };
    assert!(prepare(&sources[..2]).is_err());
    let mut wrong = sources.clone();
    wrong[1].component_instance = wrong[0].component_instance.clone();
    assert!(prepare(&wrong).is_err());
    wrong = sources.clone();
    wrong[1].path = wrong[0].path.clone();
    assert!(prepare(&wrong).is_err());
    wrong = sources.clone();
    wrong[0].component_instance = "/Application/Pipeline/Unexpected".into();
    assert!(prepare(&wrong).is_err());
    wrong = sources.clone();
    wrong.push(sources[0].clone());
    assert!(prepare(&wrong).is_err());
    let first: std::collections::BTreeMap<_, _> = prepare(&sources)
        .unwrap()
        .into_files()
        .into_iter()
        .collect();
    let mut reversed = sources.clone();
    reversed.reverse();
    assert_eq!(
        first,
        prepare(&reversed)
            .unwrap()
            .into_files()
            .into_iter()
            .collect()
    );
    for source in &sources {
        let file = source.path.file_name().unwrap().to_str().unwrap();
        assert_eq!(
            first[&format!("src/{file}")],
            std::fs::read(&source.path).unwrap()
        );
        assert!(!first.contains_key(&format!("application/{file}")));
    }
    // Each independent caller source is checked again at the normal staging
    // boundary; a stale later member must not publish a partial project.
    for (index, source) in sources.iter().enumerate() {
        let prepared = prepare(&sources).unwrap();
        let original = std::fs::read(&source.path).unwrap();
        let mut changed = original.clone();
        changed.extend_from_slice(b"\n/* changed after preparation */\n");
        std::fs::write(&source.path, changed).unwrap();
        let output = scratch.0.join(format!("stale-{index}"));
        assert!(prepared.generate(&output).is_err());
        assert!(!output.exists());
        std::fs::write(&source.path, original).unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let linked = scratch.0.join("linked.c");
        symlink(&sources[0].path, &linked).unwrap();
        wrong = sources.clone();
        wrong[1].path = linked;
        assert!(prepare(&wrong).is_err());
    }
}

#[test]
fn prepared_multi_bsw_entries_describe_actual_call_mechanisms_and_implementation_policy() {
    let plan = build(&inputs()).unwrap();
    let files = prepared_multi_files(&plan);
    let bytes = &files
        .iter()
        .find(|(path, _)| path == "descriptions/Host_Implementation.arxml")
        .unwrap()
        .1;
    let document = roxmltree::Document::parse(std::str::from_utf8(bytes).unwrap()).unwrap();
    let imported_refs: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name("EXPECTED-ENTRYS"))
        .flat_map(|node| {
            node.descendants()
                .filter(|child| child.has_tag_name("BSW-MODULE-ENTRY-REF"))
        })
        .map(|node| node.text().unwrap())
        .collect();
    for required in [
        "/AUTOSAR_Can/BswModuleEntrys/Can_Write",
        "/AUTOSAR_CanIf/BswModuleEntrys/CanIf_RxIndication",
        "/AUTOSAR_CanTp/BswModuleEntrys/CanTp_Transmit",
        "/AUTOSAR_Dcm/BswModuleEntrys/Dcm_StartOfReception",
        "/AUTOSAR_ComM/BswModuleEntrys/ComM_DCM_ActiveDiagnostic",
        "/AUTOSAR_PduR/BswModuleEntrys/PduR_CanTpCopyTxData",
        "/AUTOSAR_LSduR/BswModuleEntrys/LSduR_CanTpTransmit",
        "/AUTOSAR_Os/BswModuleEntrys/GetElapsedValue",
    ] {
        assert!(imported_refs.contains(&required), "{required}");
    }
    let actual_entries: std::collections::BTreeMap<_, _> = document
        .descendants()
        .filter(|node| node.has_tag_name("BSW-MODULE-ENTRY"))
        .map(|node| {
            let mut packages: Vec<_> = node
                .ancestors()
                .filter(|ancestor| ancestor.has_tag_name("AR-PACKAGE"))
                .map(|package| {
                    package
                        .children()
                        .find(|child| child.has_tag_name("SHORT-NAME"))
                        .unwrap()
                        .text()
                        .unwrap()
                })
                .collect();
            packages.reverse();
            let name = node
                .children()
                .find(|child| child.has_tag_name("SHORT-NAME"))
                .unwrap()
                .text()
                .unwrap();
            (format!("/{}/{name}", packages.join("/")), node)
        })
        .collect();
    for reference in document
        .descendants()
        .filter(|node| node.has_tag_name("BSW-MODULE-ENTRY-REF"))
    {
        assert_eq!(reference.attribute("DEST"), Some("BSW-MODULE-ENTRY"));
        assert!(
            actual_entries.contains_key(reference.text().unwrap()),
            "{}",
            reference.text().unwrap()
        );
    }
    for (module, name, reentrant, synchronous, call_type) in [
        ("Can", "Can_Write", "true", "true", "REGULAR"),
        ("Can", "Can_SetControllerMode", "false", "false", "REGULAR"),
        ("Can", "Can_GetControllerMode", "false", "true", "REGULAR"),
        (
            "Can",
            "Can_GetControllerErrorState",
            "true",
            "true",
            "REGULAR",
        ),
        (
            "Can",
            "Can_GetControllerRxErrorCounter",
            "true",
            "true",
            "REGULAR",
        ),
        (
            "Can",
            "Can_GetControllerTxErrorCounter",
            "true",
            "true",
            "REGULAR",
        ),
        (
            "CanIf",
            "CanIf_SetControllerMode",
            "true",
            "false",
            "REGULAR",
        ),
        (
            "CanIf",
            "CanIf_GetControllerMode",
            "false",
            "true",
            "REGULAR",
        ),
        (
            "CanIf",
            "CanIf_GetControllerErrorState",
            "true",
            "true",
            "REGULAR",
        ),
        ("CanIf", "CanIf_GetPduMode", "true", "true", "REGULAR"),
        ("CanIf", "CanIf_SetPduMode", "false", "true", "REGULAR"),
        ("CanIf", "CanIf_RxIndication", "true", "true", "CALLBACK"),
        ("CanIf", "CanIf_TxConfirmation", "true", "true", "CALLBACK"),
        (
            "CanIf",
            "CanIf_ControllerModeIndication",
            "true",
            "true",
            "CALLBACK",
        ),
        (
            "CanIf",
            "CanIf_ControllerBusOff",
            "true",
            "true",
            "CALLBACK",
        ),
        ("CanTp", "CanTp_Transmit", "true", "true", "REGULAR"),
        ("CanTp", "CanTp_RxIndication", "true", "true", "CALLBACK"),
        ("CanTp", "CanTp_TxConfirmation", "true", "true", "CALLBACK"),
        ("Dcm", "Dcm_CopyTxData", "true", "true", "CALLBACK"),
        ("PduR", "PduR_CanTpCopyTxData", "true", "true", "CALLBACK"),
        ("PduR", "PduR_CanIfRxIndication", "true", "true", "CALLBACK"),
        ("PduR", "PduR_DcmTransmit", "true", "true", "REGULAR"),
        ("LSduR", "LSduR_CanTpTransmit", "true", "true", "REGULAR"),
        (
            "LSduR",
            "LSduR_CanIfTxConfirmation",
            "true",
            "true",
            "CALLBACK",
        ),
        ("Com", "Com_SendSignal", "true", "false", "REGULAR"),
        ("Com", "Com_TriggerTransmit", "true", "true", "CALLBACK"),
        (
            "ComM",
            "ComM_DCM_ActiveDiagnostic",
            "true",
            "true",
            "CALLBACK",
        ),
        (
            "ComM",
            "ComM_BusSM_ModeIndication",
            "true",
            "false",
            "CALLBACK",
        ),
        ("BswM", "BswM_ComM_CurrentMode", "true", "true", "CALLBACK"),
        ("Rte", "Rte_COMCbk", "false", "true", "CALLBACK"),
    ] {
        let path = format!("/AUTOSAR_{module}/BswModuleEntrys/{name}");
        let entry = actual_entries[&path];
        for (tag, expected) in [
            ("IS-REENTRANT", reentrant),
            ("IS-SYNCHRONOUS", synchronous),
            ("CALL-TYPE", call_type),
        ] {
            assert_eq!(
                entry
                    .children()
                    .find(|child| child.has_tag_name(tag))
                    .unwrap()
                    .text(),
                Some(expected),
                "{path}: {tag}"
            );
        }
    }
    for entry in document
        .descendants()
        .filter(|node| node.has_tag_name("BSW-MODULE-ENTRY"))
    {
        let field = |tag| {
            entry
                .children()
                .find(|child| child.has_tag_name(tag))
                .and_then(|child| child.text())
                .unwrap()
        };
        let name = field("SHORT-NAME");
        assert_eq!(field("SW-SERVICE-IMPL-POLICY"), "STANDARD", "{name}");
        assert_eq!(field("BSW-ENTRY-KIND"), "CONCRETE", "{name}");
        for tag in [
            "IS-REENTRANT",
            "IS-SYNCHRONOUS",
            "CALL-TYPE",
            "EXECUTION-CONTEXT",
        ] {
            assert!(!field(tag).is_empty(), "{name}: {tag}");
        }
        if matches!(
            name,
            "Rte_COMCbk"
                | "Rte_COMCbkRxTOut"
                | "Dcm_TpTxConfirmation"
                | "Dcm_ComM_NoComModeEntered"
                | "Dcm_ComM_SilentComModeEntered"
                | "Dcm_ComM_FullComModeEntered"
        ) {
            assert_eq!(field("CALL-TYPE"), "CALLBACK", "{name}");
        }
        if matches!(
            name,
            "Dcm_ResetToDefaultSession" | "GetCounterValue" | "GetElapsedValue"
        ) {
            assert_eq!(field("IS-REENTRANT"), "true", "{name}");
            assert_eq!(field("IS-SYNCHRONOUS"), "true", "{name}");
            assert_eq!(field("CALL-TYPE"), "REGULAR", "{name}");
        }
        if name == "Dcm_MainFunction" {
            assert_eq!(field("CALL-TYPE"), "SCHEDULED");
        }
    }
}
