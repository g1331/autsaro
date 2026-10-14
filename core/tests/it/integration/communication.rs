//! COM, signal and network planning of the mainline.
use super::*;

#[test]
fn com_plan_keeps_real_group_timebase_and_notification_identity() {
    let plan = build(&inputs()).unwrap();
    let com = plan.description().com_runtime.as_ref().unwrap();
    assert_eq!(com.callback_header, "Rte_Com.h");
    assert_eq!(com.receive_group.path, "/Configuration/Com/Config/RxGroup");
    assert_eq!(com.receive_group.handle, 0);
    assert_eq!(
        com.receive_group.members,
        ["/Configuration/Com/Config/RxValuePdu"]
    );
    assert_eq!(com.receive_main.symbol, "Com_MainFunctionRx_Rx");
    assert_eq!(com.receive_main.period_ms, 1);
    assert_eq!(com.transmit_main.symbol, "Com_MainFunctionTx_Tx");
    assert_eq!(com.transmit_main.period_ms, 10);
    let reception = &com.receptions[0];
    assert_eq!(reception.signal, "/Configuration/Com/Config/RxValue");
    assert_eq!(
        reception.user_signal,
        "/Configuration/Rte/ComUser/Callbacks/RxValue"
    );
    assert_eq!(reception.callback_handle, 17);
    assert_eq!(reception.first_timeout_ms, 0);
    assert_eq!(reception.timeout_ms, 30);
    assert_eq!(reception.receive_callback, "Rte_COMCbk");
    assert_eq!(reception.timeout_callback, "Rte_COMCbkRxTOut");
    let mut zero = inputs();
    xml_edit(
        &mut zero,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|text| text.ends_with("/ComFirstTimeout"))
            })
        },
        |text| text.replace("<VALUE>0</VALUE>", "<VALUE>0.0e3</VALUE>"),
    );
    assert_eq!(
        build(&zero)
            .unwrap()
            .description()
            .com_runtime
            .as_ref()
            .unwrap()
            .receptions[0]
            .first_timeout_ms,
        0
    );
    let mut renamed = inputs();
    replace_all(
        &mut renamed,
        "/Com/Config/RxGroup",
        "/Com/Config/ReceptionGroup",
    );
    xml_edit(
        &mut renamed,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "RxGroup"),
        |text| {
            text.replace(
                "<SHORT-NAME>RxGroup</SHORT-NAME>",
                "<SHORT-NAME>ReceptionGroup</SHORT-NAME>",
            )
        },
    );
    assert_eq!(
        build(&renamed)
            .unwrap()
            .description()
            .com_runtime
            .as_ref()
            .unwrap()
            .receive_group
            .path,
        "/Configuration/Com/Config/ReceptionGroup"
    );
}

#[test]
fn com_rejects_tx_main_period_different_from_periodic_pdu() {
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "Transmit10ms"),
        |text| {
            text.replace("Alarm_App", "Alarm_Work")
                .replace("Ev_App", "Ev_Work")
        },
    );
    xml_edit(
        &mut sources,
        "bsw.arxml",
        |node| named(node, "BSW-TIMING-EVENT", "Com_MainFunctionTx_Tx_10ms"),
        |text| text.replace("<PERIOD>0.01</PERIOD>", "<PERIOD>0.001</PERIOD>"),
    );
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|value| value.ends_with("/ComMainTxTimeBase"))
            })
        },
        |text| text.replace("<VALUE>0.01</VALUE>", "<VALUE>0.001</VALUE>"),
    );
    rejects_in_both(&sources, "COM_TIMEBASE");
}

#[test]
fn com_control_configuration_rejects_inconsistent_group_timebase_and_callbacks() {
    for (field, original, replacement, code) in [
        ("ComSupportedIPduGroups", "1", "0", "COM_RX_GROUP"),
        ("ComIPduGroupHandleId", "0", "1", "COM_RX_GROUP"),
        ("ComMainRxTimeBase", "0.001", "0.002", "COM_TIMEBASE"),
        ("ComMainTxTimeBase", "0.01", "0.001", "COM_TIMEBASE"),
        (
            "ComUserHeaderInclude",
            "Rte_Com.h",
            "Wrong.h",
            "COM_CALLBACK",
        ),
        ("ComUserCallbackName", "Rte_COMCbk", "Wrong", "COM_CALLBACK"),
        (
            "ComUserCallbackType",
            "COM_RX_ACK",
            "COM_TX_ACK",
            "COM_CALLBACK",
        ),
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with(&format!("/{field}")))
                })
            },
            |text| {
                text.replace(
                    &format!("<VALUE>{original}</VALUE>"),
                    &format!("<VALUE>{replacement}</VALUE>"),
                )
            },
        );
        rejects_in_both(&sources, code);
    }
    for (field, code) in [
        ("ComIPduGroupRef", "COM_RX_GROUP"),
        ("ComIPduMainFunctionRef", "COM_RX_GROUP"),
        ("ComUserCallbackRef", "COM_CALLBACK"),
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with(&format!("/{field}")))
                })
            },
            |_| String::new(),
        );
        rejects_in_both(&sources, code);
    }
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.has_tag_name("PARAMETER-VALUES")
                && node.parent().is_some_and(|parent| {
                    parent.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF")
                            && child
                                .text()
                                .is_some_and(|text| text.ends_with("/ComUserSignal"))
                    })
                })
        },
        |_| String::new(),
    );
    rejects_in_both(&sources, "COM_CALLBACK");
    // A live target exists, so rejection proves direction/ownership rather than a dangling ref.
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|text| text.ends_with("/ComIPduGroupRef"))
            })
        },
        |text| {
            text.replace(
                "/Configuration/Com/Config/RxGroup",
                "/Configuration/Com/Config/TxValuePdu",
            )
        },
    );
    rejects_in_both(&sources, "COM_RX_GROUP");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|text| text.ends_with("/ComUserCallbackRef"))
            })
        },
        |text| format!("{text}{text}"),
    );
    rejects_in_both(&sources, "COM_CALLBACK");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "RxGroup"),
        |text| {
            format!(
                "{text}{}",
                text.replace(
                    "<SHORT-NAME>RxGroup</SHORT-NAME>",
                    "<SHORT-NAME>DuplicateGroup</SHORT-NAME>"
                )
            )
        },
    );
    rejects_in_both(&sources, "COM_RX_GROUP");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|text| text.ends_with("/ComIPduGroupRef"))
            })
        },
        |text| format!("{text}{text}"),
    );
    rejects_in_both(&sources, "COM_RX_GROUP");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "TxValuePdu")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text().is_some_and(|text| text.ends_with("/ComIPdu"))
                })
        },
        |text| {
            text.replacen("<REFERENCE-VALUES>", r#"<REFERENCE-VALUES><ECUC-REFERENCE-VALUE><DEFINITION-REF DEST="ECUC-REFERENCE-DEF">/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu/ComIPduGroupRef</DEFINITION-REF><VALUE-REF DEST="ECUC-CONTAINER-VALUE">/Configuration/Com/Config/RxGroup</VALUE-REF></ECUC-REFERENCE-VALUE>"#, 1)
        },
    );
    rejects_in_both(&sources, "COM_RX_GROUP");
}

#[test]
fn selected_network_profile_rejects_all_local_endpoints_without_panicking() {
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| node.has_tag_name("DATA-MAPPINGS"),
        |_| String::new(),
    );
    xml_edit(
        &mut sources,
        "ingress.arxml",
        |node| named(node, "R-PORT-PROTOTYPE", "RxValue"),
        |text| {
            text.replace(
                "<HANDLE-NEVER-RECEIVED>true</HANDLE-NEVER-RECEIVED>",
                "<HANDLE-NEVER-RECEIVED>false</HANDLE-NEVER-RECEIVED>",
            )
            .replace(
                "<ALIVE-TIMEOUT>0.03</ALIVE-TIMEOUT>",
                "<ALIVE-TIMEOUT>0</ALIVE-TIMEOUT>",
            )
        },
    );
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "IngressProcess",
    )
    .replace("IngressProcess", "LocalIngress")
    .replace("/Application/Ingress/Value", "/Application/Ingress/TxValue")
    .replace(
        "/Application/Pipeline/ProcessInstance",
        "/Application/Pipeline/IngressInstance",
    )
    .replace("/Application/Process/Value", "/Application/Ingress/RxValue");
    change(
        &mut sources,
        "composition.arxml",
        "</CONNECTORS>",
        &format!("{connector}</CONNECTORS>"),
    );
    rejects_in_both(&sources, "COM_CONFIGURATION");
}

#[test]
fn flat_signal_context_and_ecu_members_are_unique_and_nonempty() {
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| node.tag_name().name() == "DATA-ELEMENT-IREF",
        |original| {
            original.replace("</CONTEXT-COMPONENT-REF>", "</CONTEXT-COMPONENT-REF><CONTEXT-COMPONENT-REF DEST=\"SW-COMPONENT-PROTOTYPE\">/Application/Pipeline/ProcessInstance</CONTEXT-COMPONENT-REF>")
        },
    );
    rejects_in_both(&sources, "REFERENCE_UNRESOLVED");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| node.tag_name().name() == "COMPONENT-IREF",
        |original| format!("{original}{original}"),
    );
    rejects_in_both(&sources, "ECU_MAPPING");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| node.tag_name().name() == "SWC-TO-ECU-MAPPING",
        |original| {
            format!(
                "{original}<SWC-TO-ECU-MAPPING><SHORT-NAME>Empty</SHORT-NAME><ECU-INSTANCE-REF DEST=\"ECU-INSTANCE\">/Extract/ReferenceEcu</ECU-INSTANCE-REF></SWC-TO-ECU-MAPPING>"
            )
        },
    );
    rejects_in_both(&sources, "ECU_MAPPING");
}

#[test]
fn network_zero_timeout_matches_without_changing_runtime_scope() {
    let mut sources = inputs();
    change(
        &mut sources,
        "ingress.arxml",
        "<ALIVE-TIMEOUT>0.03</ALIVE-TIMEOUT>",
        "<ALIVE-TIMEOUT>0</ALIVE-TIMEOUT>",
    );
    parameter(&mut sources, "RxValue", "ComTimeout", "0");
    assert!(normal_validation(&sources).diagnostics.is_empty());
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(
        plan.description()
            .signals
            .iter()
            .find(|signal| signal.receive)
            .unwrap()
            .deadline_ms,
        Some(0)
    );
    parameter(&mut sources, "RxValue", "ComTimeout", "0.02");
    rejects_in_both(&sources, "TIMEOUT_CONFLICT");
}

#[test]
fn communication_types_and_polling_periods_are_source_derived() {
    let plan = build(&inputs()).unwrap_or_else(|issues| panic!("{issues:?}"));
    let runtime = plan.description().communication_runtime.as_ref().unwrap();
    let json = serde_json::to_value(runtime).unwrap();
    assert_eq!(json["pduIdType"], "UINT16");
    assert_eq!(json["pduLengthType"], "UINT16");
    assert_eq!(
        runtime.can.read_write_period,
        "/Configuration/Can/General/Polling"
    );
    assert_eq!(runtime.can.read_write_period_ms, 1);
    assert_eq!(runtime.can.controller_id, 0);
    assert_eq!(runtime.can.can_if_controller_id, 0);
    assert_eq!(runtime.can.receive_handle, 0);
    assert_eq!(runtime.can.transmit_handle, 1);
    assert_eq!(runtime.can.busoff_period_ms, 1);
    assert_eq!(runtime.can.mode_period_ms, 1);
    assert!(runtime.can.receive_polling && runtime.can.transmit_polling);
    assert!(runtime.can.busoff_polling && runtime.can.wakeup_polling);
    let mut sources = inputs();
    parameter(&mut sources, "Pdus", "PduIdTypeEnum", "UINT8");
    parameter(&mut sources, "Pdus", "PduLengthTypeEnum", "UINT32");
    replace_all(&mut sources, "/General/Polling", "/General/PollCycle");
    change(
        &mut sources,
        "ecuc.arxml",
        "<SHORT-NAME>Polling</SHORT-NAME>",
        "<SHORT-NAME>PollCycle</SHORT-NAME>",
    );
    parameter(&mut sources, "Receive", "CanObjectId", "1");
    parameter(&mut sources, "Transmit", "CanObjectId", "0");
    let renamed = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    let runtime = renamed
        .description()
        .communication_runtime
        .as_ref()
        .unwrap();
    let json = serde_json::to_value(runtime).unwrap();
    assert_eq!(json["pduIdType"], "UINT8");
    assert_eq!(json["pduLengthType"], "UINT32");
    assert!(runtime.can.read_write_period.ends_with("/PollCycle"));
    assert_eq!(runtime.can.controller_id, 0);
    assert_eq!(runtime.can.can_if_controller_id, 0);
    assert_eq!(runtime.can.receive_handle, 1);
    assert_eq!(runtime.can.transmit_handle, 0);
    assert!(normal_validation(&sources).diagnostics.is_empty());
    let files = prepared_pipeline_files(&renamed);
    let text = |name: &str| {
        std::str::from_utf8(
            &files
                .iter()
                .find(|(path, _)| path == name)
                .unwrap_or_else(|| panic!("missing {name}"))
                .1,
        )
        .unwrap()
    };
    let defines = |name: &str, value: &str| {
        text("generated/include/Ecu_ProfileConfig.h")
            .lines()
            .any(|line| line.split_whitespace().eq(["#define", name, value]))
    };
    assert!(defines("CAN_RX_HOH", "1u"));
    assert!(defines("CAN_TX_HOH", "0u"));
    let description = text("generated/descriptions/Host_Implementation.arxml");
    assert!(description.contains("Can_VAR_CLEARED_UNSPECIFIED"));
    assert!(description.contains(
        "<DEPENDENCY-ON-ARTIFACT><SHORT-NAME>generated_include_Can_MemMap_h</SHORT-NAME>"
    ));
    assert!(description.contains("Det_ReportRuntimeError"));
    // The CanIf post-build set (Verifies: SWS_CANIF_00144) lives in CanIf_PBcfg.c (SWS_BSW_00015).
    assert!(text("generated/src/CanIf_PBcfg.c").contains("const CanIf_ConfigType CanIf_Config"));
    assert!(
        text("generated/include/CanIf_Cfg.h")
            .contains("extern const CanIf_ConfigType CanIf_Config;")
    );
    for name in [
        "bsw/include/standard/Can.h",
        "integration/include/Can_Host.h",
        "integration/src/Can_Host.c",
        "bsw/src/Can_Internal.h",
        "bsw/src/Det.c",
        "integration/src/SchM_CanIf_Host.c",
    ] {
        assert!(files.iter().any(|(path, _)| path == name), "missing {name}");
    }
}

#[test]
fn communication_rejects_truncation_and_unbound_polling_in_both_entries() {
    for (owner, field, new_value, code) in [
        (
            "Polling",
            "CanMainFunctionPeriod",
            "0.002",
            "CAN_POLLING_TIMEBASE",
        ),
        (
            "General",
            "CanMainFunctionBusoffPeriod",
            "0.002",
            "CAN_POLLING_TIMEBASE",
        ),
        (
            "General",
            "CanMainFunctionModePeriod",
            "0.002",
            "CAN_POLLING_TIMEBASE",
        ),
        (
            "Controller",
            "CanBusoffProcessing",
            "INTERRUPT",
            "CAN_PROCESSING",
        ),
        (
            "General",
            "CanDevErrorDetect",
            "true",
            "CAN_FEATURE_UNSUPPORTED",
        ),
        (
            "General",
            "CanVersionInfoApi",
            "true",
            "CAN_FEATURE_UNSUPPORTED",
        ),
        // One HOH numbering area from zero (Verifies: SWS_CANIF_00115, ECUC_Can_00326) and
        // controller IDs from zero (Verifies: SWS_CANIF_00653／00433, ECUC_CanIf_00647).
        ("Transmit", "CanObjectId", "0", "CAN_HARDWARE_HANDLES"),
        ("Transmit", "CanObjectId", "17", "CAN_HARDWARE_HANDLES"),
        ("Receive", "CanObjectId", "17", "CAN_HARDWARE_HANDLES"),
        ("Controller", "CanControllerId", "7", "CAN_CONTROLLER_ID"),
        ("Controller", "CanIfCtrlId", "9", "CAN_CONTROLLER_ID"),
        // The BasicCAN HRH filters in software (Verifies: SWS_CANIF_00663／00469,
        // ECUC_CanIf_00632), and the Rx data length check stays enabled per PDU
        // (Verifies: SWS_CANIF_00902, ECUC_CanIf_00846).
        (
            "Receive",
            "CanIfHrhSoftwareFilter",
            "false",
            "TARGET_PARAMETER_UNSUPPORTED",
        ),
        // Rejects: ECUC_CanIf_00846
        (
            "RxValue",
            "CanIfRxPduDataLengthCheck",
            "false",
            "LENGTH_CONFLICT",
        ),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, owner, field, new_value);
        rejects_in_both(&sources, code);
    }
    let mut sources = inputs();
    parameter(&mut sources, "Pdus", "PduIdTypeEnum", "UINT8");
    parameter(&mut sources, "RxValue", "CanIfRxPduId", "256");
    rejects_in_both(&sources, "COMMUNICATION_HANDLE_RANGE");
    let mut sources = inputs();
    parameter(&mut sources, "Pdus", "PduLengthTypeEnum", "UINT8");
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "RxValuePdu"),
        |text| {
            format!(
                "{}\n{}",
                text,
                text.replace(
                    "<SHORT-NAME>RxValuePdu</SHORT-NAME>",
                    "<SHORT-NAME>AdditionalPdu</SHORT-NAME>"
                )
                .replace("<VALUE>4</VALUE>", "<VALUE>256</VALUE>")
            )
        },
    );
    rejects_in_both(&sources, "COMMUNICATION_LENGTH_RANGE");
    let mut sources = inputs();
    change(
        &mut sources,
        "ecuc.arxml",
        "/Configuration/Can/General/Polling</VALUE-REF>",
        "/Configuration/Can/General</VALUE-REF>",
    );
    rejects_in_both(&sources, "CAN_POLLING_REFERENCE");
}

#[test]
fn selected_com_manual_trigger_requires_zero_minimum_delay_and_no_callout() {
    let mut sources = inputs();
    parameter(&mut sources, "Transmit", "ComMinimumDelayTime", "0.001");
    rejects_in_both(&sources, "COM_FEATURE_UNSUPPORTED");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "TxValuePdu")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text().is_some_and(|text| text.ends_with("/ComIPdu"))
                })
        },
        |text| {
            text.replacen("<PARAMETER-VALUES>", r#"<PARAMETER-VALUES><ECUC-TEXTUAL-PARAM-VALUE><DEFINITION-REF DEST="ECUC-FUNCTION-NAME-DEF">/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu/ComIPduCallout</DEFINITION-REF><VALUE>ApplicationCallout</VALUE></ECUC-TEXTUAL-PARAM-VALUE>"#, 1)
        },
    );
    rejects_in_both(&sources, "COM_FEATURE_UNSUPPORTED");
}

#[test]
fn signal_processing_matches_actual_immediate_notification_contract() {
    for pdu in ["RxValuePdu", "TxValuePdu"] {
        let mut sources = inputs();
        parameter(&mut sources, pdu, "ComIPduSignalProcessing", "DEFERRED");
        rejects_in_both(&sources, "COM_FEATURE_UNSUPPORTED");
    }
}

#[test]
fn actual_partition_owns_all_instances_and_all_com_producers() {
    let plan = build(&inputs()).unwrap();
    let partition = &plan
        .description()
        .communication_runtime
        .as_ref()
        .unwrap()
        .partition;
    assert_eq!(partition.name, "OwnerPartition");
    assert_eq!(partition.id, 0);
    assert_eq!(partition.instances.len(), 4);
    assert!(
        partition
            .instances
            .iter()
            .any(|instance| instance.ends_with("/DcmService"))
    );
    let files = prepared_pipeline_files(&plan);
    let source = String::from_utf8(
        files
            .iter()
            .find(|(name, _)| name == "generated/src/Rte.c")
            .unwrap()
            .1
            .clone(),
    )
    .unwrap();
    assert!(source.contains("RTE_Ingress_START_SEC_CODE"));
    assert!(source.contains("RTE_Observe_START_SEC_VAR_CLEARED_UNSPECIFIED"));
    assert!(source.contains("RTE_OwnerPartition_START_SEC_CALLOUT_CODE"));
    let mut sources = inputs();
    parameter(&mut sources, "OwnerPartition", "EcucPartitionId", "1");
    rejects_in_both(&sources, "RTE_PARTITION_CONFIGURATION");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.has_tag_name("REFERENCE-VALUES")
                && node.descendants().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/RteComUserEcucPartitionRef"))
                })
        },
        |_| String::new(),
    );
    rejects_in_both(&sources, "RTE_PARTITION_CONFIGURATION");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.has_tag_name("ECUC-INSTANCE-REFERENCE-VALUE")
                && node.descendants().any(|child| {
                    child.has_tag_name("TARGET-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/DcmService"))
                })
        },
        |_| String::new(),
    );
    rejects_in_both(&sources, "RTE_PARTITION_CONFIGURATION");
}
