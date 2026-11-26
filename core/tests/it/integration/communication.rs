//! COM, signal and network planning of the mainline.
use super::*;

#[test]
fn multichannel_topologies_preserve_controller_and_mode_bindings() {
    for count in [2, 3, 4, 6] {
        let sources = crate::support::multichannel::multichannel_sources(count);
        let plan = build(&sources).unwrap_or_else(|issues| panic!("{count}: {issues:#?}"));
        let description = plan.description();
        let communication = description.communication_runtime.as_ref().unwrap();
        assert_eq!(communication.channels.len(), count);
        assert_eq!(communication.hardware.len(), 2 * count);
        for (driver, channel) in communication.channels.iter().enumerate() {
            assert_eq!(usize::from(channel.controller_id), driver);
            assert_eq!(
                usize::from(channel.can_if_controller_id),
                count - 1 - driver
            );
        }
        let mode = description.mode_runtime.as_ref().unwrap();
        assert_eq!(mode.channels.len(), count);
        assert_eq!(mode.users[0].channels.len(), count);
        assert_eq!(mode.rules.len(), 3 * count);
        assert!(
            mode.dcm_connections
                .values()
                .all(|channel| channel.ends_with("/Host"))
        );
    }
}

#[test]
fn diagnostic_npdu_keeps_its_network_when_another_channel_reuses_its_id() {
    let mut sources = crate::support::multichannel::multichannel_sources(2);
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| named(node, "CAN-FRAME-TRIGGERING", "RxValue1Trigger"),
        |xml| {
            xml.replace(
                "<IDENTIFIER>800</IDENTIFIER>",
                "<IDENTIFIER>1792</IDENTIFIER>",
            )
        },
    );
    parameter(&mut sources, "RxValue1", "CanIfRxPduCanId", "1792");
    let plan = build(&sources).unwrap();
    let config = plan.description();
    let transport = config.diagnostic_transport.as_ref().unwrap();
    assert_eq!(transport.request_can_id, 1792);
    let ids = config
        .signals
        .iter()
        .filter(|signal| signal.receive && signal.can_id == 1792)
        .count();
    assert_eq!(ids, 1);
}

#[test]
fn multichannel_invalid_routes_are_rejected_at_configuration_objects() {
    use super::single_component::rejected_at;
    let sources = crate::support::multichannel::multichannel_sources(2);
    let mut mapping = sources.clone();
    xml_edit(
        &mut mapping,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "Controller1")
                && node.children().any(|child| {
                    child
                        .text()
                        .is_some_and(|value| value.ends_with("/CanIfCtrlCfg"))
                })
        },
        |xml| xml.replace("/Can/Config/Controller1<", "/Can/Config/Controller<"),
    );
    rejected_at(
        &mapping,
        "CONTROLLER_MAPPING",
        "ecuc.arxml",
        "/Configuration/CanIf/Controller/Controller1",
    );

    let mut wrong_direction = sources.clone();
    xml_edit(
        &mut wrong_direction,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "TransmitBuffer1"),
        |xml| xml.replace("/Handles/Transmit1<", "/Handles/Receive1<"),
    );
    let issues = build(&wrong_direction)
        .err()
        .expect("HRH accepted as an HTH");
    assert!(
        issues
            .iter()
            .any(|issue| issue.object.as_deref().is_some_and(|object| object
                == "/Configuration/CanIf/Config/TransmitBuffer1"
                || object == "/Configuration/CanIf/Config/TxValue1")),
        "{issues:#?}"
    );

    let mut gap = sources.clone();
    parameter(&mut gap, "Transmit", "CanObjectId", "99");
    rejected_at(
        &gap,
        "CAN_HARDWARE_HANDLES",
        "ecuc.arxml",
        "/Configuration/Can/Config/Transmit",
    );

    let mut duplicate = sources;
    parameter(&mut duplicate, "DiagRequest", "CanIfRxPduCanId", "800");
    xml_edit(
        &mut duplicate,
        "extract.arxml",
        |node| named(node, "CAN-FRAME-TRIGGERING", "DiagRequestTrigger"),
        |xml| {
            xml.replace(
                "<IDENTIFIER>1792</IDENTIFIER>",
                "<IDENTIFIER>800</IDENTIFIER>",
            )
        },
    );
    rejected_at(
        &duplicate,
        "CAN_ID_CONFLICT",
        "ecuc.arxml",
        "/Configuration/CanIf/Config/DiagRequest",
    );
}

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
            .any(|line| {
                let code = line.split_once("/**<").map_or(line, |(code, _)| code);
                code.split_whitespace().eq(["#define", name, value])
            })
    };
    assert!(defines("CAN_RX_HOH", "CanConf_CanHardwareObject_Receive"));
    assert!(defines("CAN_TX_HOH", "CanConf_CanHardwareObject_Transmit"));
    let can_symbols = text("generated/include/Can_Cfg.h");
    assert!(
        can_symbols
            .lines()
            .any(|line| line.split_whitespace().take(3).eq([
                "#define",
                "CanConf_CanHardwareObject_Receive",
                "1u"
            ]))
    );
    assert!(
        can_symbols
            .lines()
            .any(|line| line.split_whitespace().take(3).eq([
                "#define",
                "CanConf_CanHardwareObject_Transmit",
                "0u"
            ]))
    );
    let description = text("generated/descriptions/Host_Implementation.arxml");
    assert!(description.contains("Can_VAR_CLEARED_UNSPECIFIED"));
    assert!(description.contains(
        "<DEPENDENCY-ON-ARTIFACT><SHORT-NAME>generated_include_Can_MemMap_h</SHORT-NAME>"
    ));
    assert!(description.contains("Det_ReportRuntimeError"));
    // The CanIf post-build set (Verifies: SWS_CANIF_00144) lives in CanIf_PBcfg.c (SWS_BSW_00015).
    assert!(text("generated/src/CanIf_PBcfg.c").contains("const CanIf_ConfigType CanIf_Config"));
    assert!(text("generated/include/CanIf_Cfg.h").contains(
        "extern const CanIf_ConfigType CanIf_Config RTE_CanIf_CONFIG_DATA_PREBUILD_UNSPECIFIED;"
    ));
    for name in [
        "bsw/include/Can.h",
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
        // The polled main functions run at their configured periods
        // (Verifies: ECUC_Can_00484／00355／00376).
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
        // One HOH numbering area from zero (Verifies: SWS_CANIF_00115) (Verifies: ECUC_Can_00326)
        // and controller IDs from zero (Verifies: SWS_CANIF_00653／00433, ECUC_CanIf_00647)
        // (Verifies: ECUC_Can_00316).
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
    let object = "/Configuration/EcuC/Config/Pdus/AdditionalPdu";
    assert!(
        build(&sources)
            .err()
            .expect("UINT8 accepted 256 bytes")
            .iter()
            .any(|issue| issue.code == "COMMUNICATION_LENGTH_RANGE"
                && issue.object.as_deref() == Some(object))
    );
    assert!(
        normal_validation(&sources)
            .diagnostics
            .iter()
            .any(|issue| issue.code == "COMMUNICATION_LENGTH_RANGE"
                && issue.path.as_deref() == Some(object))
    );
    assert_rejected_sources_preserved(&sources);
    parameter(&mut sources, "AdditionalPdu", "PduLength", "255");
    assert!(normal_validation(&sources).diagnostics.is_empty());
    build(&sources).unwrap();
    let mut sources = inputs();
    change(
        &mut sources,
        "ecuc.arxml",
        "/Configuration/Can/General/Polling</VALUE-REF>",
        "/Configuration/Can/General</VALUE-REF>",
    );
    // Verifies: ECUC_Can_00438
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
    assert_eq!(partition.instances.len(), 5);
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
    assert!(source.contains("RTE_Observe_START_SEC_VAR_INIT_UNSPECIFIED"));
    assert!(source.contains("RTE_Ingress_START_SEC_VAR_CLEARED_UNSPECIFIED"));
    assert!(!source.contains("RTE_Observe_START_SEC_VAR_CLEARED_UNSPECIFIED"));
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

#[test]
fn com_integer_plan_keeps_two_levels_and_locates_invalid_fields() {
    let sources = integer_signal_sources();
    let plan = build(&sources).unwrap();
    // Assert the delivered plan fields through serialization. The current
    // suite also compiles against the first-parent C-analysis generator.
    let channels = serde_json::to_value(&plan.description().signals).unwrap();
    let channels = channels.as_array().unwrap();
    let rx = channels
        .iter()
        .find(|channel| channel["receive"] == true)
        .unwrap();
    let tx = channels
        .iter()
        .find(|channel| channel["receive"] == false)
        .unwrap();
    for (channel, big, start) in [(rx, false, 8), (tx, true, 15)] {
        assert_eq!(channel["startBit"], 5);
        assert_eq!(channel["bigEndian"], big);
        assert_eq!(channel["frameStartBit"], start);
        assert_eq!(channel["frameOffset"], 1);
        assert_eq!(channel["bitLength"], 10);
        assert_eq!(channel["pduLength"], 2);
        assert_eq!(channel["dlc"], 4);
    }
    assert_eq!(tx["initialValue"], -7);

    let mut inherited = sources.clone();
    xml_edit(
        &mut inherited,
        "extract.arxml",
        |node| named(node, "I-SIGNAL", "RxValueSignal"),
        |block| {
            let start = block.find("<NETWORK-REPRESENTATION-PROPS>").unwrap();
            let end = block.find("</NETWORK-REPRESENTATION-PROPS>").unwrap()
                + "</NETWORK-REPRESENTATION-PROPS>".len();
            format!("{}{}", &block[..start], &block[end..])
                .replace(">OVERRIDE<", ">NETWORK-REPRESENTATION-FROM-COM-SPEC<")
        },
    );
    let plan = build(&inherited).unwrap();
    let channels = serde_json::to_value(&plan.description().signals).unwrap();
    assert_eq!(
        channels
            .as_array()
            .unwrap()
            .iter()
            .find(|channel| channel["receive"] == true)
            .unwrap()["nativeType"],
        "sint16"
    );

    for (tag, object, field, replacement, path) in [
        (
            "I-SIGNAL",
            "TxValueSignal",
            "LENGTH",
            "17",
            "/Extract/TxValueSignal",
        ),
        (
            "I-SIGNAL-TO-I-PDU-MAPPING",
            "TxValueMap",
            "START-POSITION",
            "13",
            "/Extract/TxValuePdu/TxValueMap",
        ),
        (
            "PDU-TO-FRAME-MAPPING",
            "TxValuePduMap",
            "START-POSITION",
            "31",
            "/Extract/TxValueFrame/TxValuePduMap",
        ),
        (
            "PDU-TO-FRAME-MAPPING",
            "TxValuePduMap",
            "START-POSITION",
            "8",
            "/Extract/TxValueFrame/TxValuePduMap",
        ),
    ] {
        let mut invalid = sources.clone();
        xml_edit(
            &mut invalid,
            "extract.arxml",
            |node| named(node, tag, object),
            |block| {
                let start = block.find(&format!("<{field}>")).unwrap() + field.len() + 2;
                let end = start + block[start..].find('<').unwrap();
                format!("{}{replacement}{}", &block[..start], &block[end..])
            },
        );
        let issues = build(&invalid).err().expect("invalid layout accepted");
        assert!(
            issues.iter().any(|issue| issue.code == "LENGTH_CONFLICT"
                && issue.file.as_deref() == Some("extract.arxml")
                && issue.object.as_deref() == Some(path)),
            "{issues:?}"
        );
    }

    let mut overlap = sources.clone();
    xml_edit(
        &mut overlap,
        "extract.arxml",
        |node| named(node, "I-SIGNAL", "TxValueSignal"),
        |block| {
            let extra = block
                .replace("TxValueSignal", "ExtraSignal")
                .replace("TxValueSystemSignal", "ExtraSystemSignal");
            format!(
                "{block}{extra}<SYSTEM-SIGNAL><SHORT-NAME>ExtraSystemSignal</SHORT-NAME></SYSTEM-SIGNAL>"
            )
        },
    );
    xml_edit(
        &mut overlap,
        "extract.arxml",
        |node| named(node, "I-SIGNAL-TO-I-PDU-MAPPING", "TxValueMap"),
        |block| {
            let extra = block
                .replace("TxValueMap", "OverlappingMap")
                .replace("TxValueSignal", "ExtraSignal");
            format!("{block}{extra}")
        },
    );
    let issues = build(&overlap).err().expect("overlapping signal accepted");
    assert!(
        issues.iter().any(|issue| issue.code == "CAN_SIGNAL_OVERLAP"
            && issue.file.as_deref() == Some("extract.arxml")
            && issue.object.as_deref() == Some("/Extract/TxValuePdu/OverlappingMap")),
        "{issues:?}"
    );

    fn mapping_set(node: roxmltree::Node<'_, '_>) -> bool {
        named(node, "DATA-TYPE-MAPPING-SET", "ApplicationTypes")
    }
    let mut ambiguous = sources.clone();
    xml_edit(&mut ambiguous, "types.arxml", mapping_set, |block| {
        let start = block.find("<DATA-TYPE-MAP>").unwrap();
        let end = block.find("</DATA-TYPE-MAP>").unwrap() + "</DATA-TYPE-MAP>".len();
        format!("{}{}{}", &block[..end], &block[start..end], &block[end..])
    });
    let issues = build(&ambiguous)
        .err()
        .expect("ambiguous DataTypeMap accepted");
    assert!(
        issues.iter().any(|issue| issue.code == "TYPE_MAP_MISSING"
            && issue.file.as_deref() == Some("types.arxml")
            && issue.object.as_deref() == Some("/Types/ApplicationTypes")),
        "{issues:?}"
    );

    let mut missing = sources.clone();
    xml_edit(&mut missing, "types.arxml", mapping_set, |block| {
        let start = block.find("<DATA-TYPE-MAPS>").unwrap();
        let end = block.find("</DATA-TYPE-MAPS>").unwrap() + "</DATA-TYPE-MAPS>".len();
        format!("{}{}", &block[..start], &block[end..])
    });
    let issues = build(&missing).err().expect("missing DataTypeMap accepted");
    assert!(
        issues.iter().any(|issue| issue.code == "TYPE_MAP_MISSING"
            && issue.file.as_deref() == Some("types.arxml")
            && issue.object.as_deref() == Some("/Types/ApplicationTypes")),
        "{issues:?}"
    );
}

#[test]
fn com_shared_pdu_refuses_different_monitored_port_deadlines() {
    let mut sources = two_receiver_inputs();
    for name in ["RxValueSignal", "RxSecondSignal"] {
        xml_edit(
            &mut sources,
            "extract.arxml",
            |node| named(node, "I-SIGNAL", name),
            |block| block.replace("<LENGTH>32<", "<LENGTH>16<"),
        );
    }
    parameter(&mut sources, "RxValue", "ComBitSize", "16");
    parameter(&mut sources, "RxSecond", "ComBitSize", "16");
    parameter(&mut sources, "RxSecond", "ComBitPosition", "16");

    let extract = sources
        .iter()
        .find(|source| source.logical_path() == "extract.arxml")
        .unwrap();
    let source = std::str::from_utf8(extract.bytes()).unwrap();
    let document = roxmltree::Document::parse(source).unwrap();
    let second = document
        .descendants()
        .find(|node| named(*node, "I-SIGNAL-TO-I-PDU-MAPPING", "RxSecondMap"))
        .unwrap();
    let mapping = source[second.range()].replace("<START-POSITION>0<", "<START-POSITION>16<");
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| named(node, "I-SIGNAL-I-PDU", "RxValuePdu"),
        |block| {
            block.replace(
                "</I-SIGNAL-TO-PDU-MAPPINGS>",
                &format!("{mapping}</I-SIGNAL-TO-PDU-MAPPINGS>"),
            )
        },
    );
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| named(node, "PDU-TRIGGERING", "RxValuePduTrigger"),
        |block| {
            let reference = "<I-SIGNAL-TRIGGERING-REF-CONDITIONAL><I-SIGNAL-TRIGGERING-REF DEST=\"I-SIGNAL-TRIGGERING\">/Extract/ReferenceCan/Channel/RxSecondSignalTrigger</I-SIGNAL-TRIGGERING-REF></I-SIGNAL-TRIGGERING-REF-CONDITIONAL>";
            block.replace(
                "</I-SIGNAL-TRIGGERINGS>",
                &format!("{reference}</I-SIGNAL-TRIGGERINGS>"),
            )
        },
    );
    for (tag, name) in [
        ("I-SIGNAL-I-PDU", "RxSecondPdu"),
        ("CAN-FRAME", "RxSecondFrame"),
        ("CAN-FRAME-TRIGGERING", "RxSecondTrigger"),
        ("PDU-TRIGGERING", "RxSecondPduTrigger"),
    ] {
        xml_edit(
            &mut sources,
            "extract.arxml",
            |node| named(node, tag, name),
            |_| String::new(),
        );
    }
    replace_all(
        &mut sources,
        "/Extract/RxSecondPdu/RxSecondMap",
        "/Extract/RxValuePdu/RxSecondMap",
    );
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "RxValuePdu")
                && node.children().any(|field| {
                    field.has_tag_name("DEFINITION-REF")
                        && field
                            .text()
                            .is_some_and(|value| value.ends_with("/ComIPdu"))
                })
        },
        |block| {
            let definition = "/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu/ComIPduSignalRef";
            let reference = format!(
                "<ECUC-REFERENCE-VALUE><DEFINITION-REF DEST=\"ECUC-REFERENCE-DEF\">{definition}</DEFINITION-REF><VALUE-REF DEST=\"ECUC-CONTAINER-VALUE\">/Configuration/Com/Config/RxSecond</VALUE-REF></ECUC-REFERENCE-VALUE>"
            );
            block.replace(
                "</REFERENCE-VALUES>",
                &format!("{reference}</REFERENCE-VALUES>"),
            )
        },
    );
    for (name, definition) in [
        ("RxSecondPdu", "/EcucPduCollection/Pdu"),
        ("RxSecondPdu", "/ComConfig/ComIPdu"),
        ("RxSecond", "/CanIfInitCfg/CanIfRxPduCfg"),
        ("RxSecondSource", "/PduRRoutingPaths/PduRSrcPdu"),
        ("RxSecondDestination", "/PduRRoutingPaths/PduRDestPdu"),
        ("RxSecond", "/PduRRoutingPaths/PduRRoutingPath"),
    ] {
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                named(node, "ECUC-CONTAINER-VALUE", name)
                    && node.children().any(|field| {
                        field.has_tag_name("DEFINITION-REF")
                            && field
                                .text()
                                .is_some_and(|value| value.ends_with(definition))
                    })
            },
            |_| String::new(),
        );
    }

    let issues = build(&sources)
        .err()
        .expect("shared PDU accepted divergent deadlines");
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "COM_SHARED_DEADLINE"
                && issue.file.as_deref() == Some("ecuc.arxml")
                && issue.object.as_deref() == Some("/Configuration/Com/Config/RxValue")),
        "{issues:?}"
    );

    parameter(&mut sources, "RxSecond", "ComTimeout", "0.03");
    xml_edit(
        &mut sources,
        "ingress.arxml",
        |node| named(node, "R-PORT-PROTOTYPE", "RxSecond"),
        |block| block.replace("<ALIVE-TIMEOUT>0.05<", "<ALIVE-TIMEOUT>0.03<"),
    );
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| named(node, "I-SIGNAL-PORT", "RxSecondSignalPort"),
        |block| block.replace("<TIMEOUT>0.05<", "<TIMEOUT>0.03<"),
    );
    let plan = build(&sources).unwrap();
    assert_eq!(
        plan.description()
            .signals
            .iter()
            .filter(|signal| signal.receive)
            .count(),
        2
    );
    assert_eq!(
        plan.description()
            .routes
            .iter()
            .filter(|route| route.receive && !route.transport)
            .count(),
        1
    );
}
