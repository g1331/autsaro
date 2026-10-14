//! A mainline project with a single application component.
use super::*;
use autosar_config_core::integration::InputSource;
use std::path::Path;

#[test]
fn single_application_component_is_planned_by_the_mainline() {
    let sources = single_component_sources();
    let validation = normal_validation(&sources);
    assert!(
        validation.diagnostics.is_empty(),
        "{:?}",
        validation.diagnostics
    );
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(plan.description().profile, "singlecore-ecu-v1");
    let slots = plan.application_slot_descriptors().unwrap();
    assert_eq!(
        slots
            .iter()
            .map(|slot| slot.producer_slot.as_str())
            .collect::<Vec<_>>(),
        ["singlecore-ecu-v1:/Application/Pipeline/IngressInstance"]
    );
}

#[test]
fn single_application_component_rejections_locate_source_objects() {
    fn rejected(sources: &[InputSource], code: &str, file: &str, object: &str) {
        let issues = build(sources)
            .err()
            .unwrap_or_else(|| panic!("{code} accepted"));
        assert!(
            issues.iter().any(|issue| issue.code == code
                && issue.file.as_deref() == Some(file)
                && issue.object.as_deref() == Some(object)),
            "{issues:?}"
        );
        let validation = normal_validation(sources);
        assert!(
            validation.diagnostics.iter().any(|issue| issue.code == code
                && issue.file.as_deref() == Some(file)
                && issue.path.as_deref() == Some(object)),
            "{code}: {:?}",
            validation.diagnostics
        );
    }

    let mut dangling = single_component_sources();
    change(
        &mut dangling,
        "composition.arxml",
        "/Application/Ingress/ApplicationValue</TARGET-P-PORT-REF>",
        "/Application/Ingress/MissingValue</TARGET-P-PORT-REF>",
    );
    rejected(
        &dangling,
        "REFERENCE_UNRESOLVED",
        "composition.arxml",
        "/Application/Pipeline/Diagnostic",
    );

    let mut missing_port = single_component_sources();
    change(
        &mut missing_port,
        "extract.arxml",
        "/Application/Ingress/RxValue</CONTEXT-PORT-REF>",
        "/Application/Ingress/MissingRx</CONTEXT-PORT-REF>",
    );
    rejected(
        &missing_port,
        "REFERENCE_UNRESOLVED",
        "extract.arxml",
        "/Extract/ReferenceExtract/Mapping",
    );

    let mut conflicting = single_component_sources();
    change(
        &mut conflicting,
        "ingress.arxml",
        "<OPERATION-INVOKED-EVENT>",
        "<TIMING-EVENT><SHORT-NAME>Periodic20ms</SHORT-NAME>\
         <START-ON-EVENT-REF DEST=\"RUNNABLE-ENTITY\">/Application/Ingress/Behavior/Periodic</START-ON-EVENT-REF>\
         <PERIOD>0.02</PERIOD></TIMING-EVENT><OPERATION-INVOKED-EVENT>",
    );
    rejected(
        &conflicting,
        "SCHEDULE_NOT_UNIQUE",
        "ingress.arxml",
        "/Application/Ingress/Behavior/Periodic",
    );

    // Unconnected ports stay rejected until #51 settles their RTE semantics.
    let mut unconnected = single_component_sources();
    xml_edit(
        &mut unconnected,
        "extract.arxml",
        |node| {
            node.has_tag_name("SENDER-RECEIVER-TO-SIGNAL-MAPPING")
                && node
                    .descendants()
                    .any(|child| child.text() == Some("/Application/Ingress/RxValue"))
        },
        |_| String::new(),
    );
    rejected(
        &unconnected,
        "ENDPOINT_BINDING",
        "ingress.arxml",
        "/Application/Ingress/RxValue",
    );

    // Only the Dcm service component remains: nothing owns a periodic schedule.
    let mut services_only = single_component_sources();
    services_only.retain(|source| source.logical_path() != "ingress.arxml");
    for (file, tag) in [
        ("composition.arxml", "CONNECTORS"),
        ("extract.arxml", "DATA-MAPPINGS"),
    ] {
        xml_edit(
            &mut services_only,
            file,
            |node| node.has_tag_name(tag),
            |_| String::new(),
        );
    }
    xml_edit(
        &mut services_only,
        "composition.arxml",
        |node| named(node, "SW-COMPONENT-PROTOTYPE", "IngressInstance"),
        |_| String::new(),
    );
    for file in ["extract.arxml", "ecuc.arxml"] {
        xml_edit(
            &mut services_only,
            file,
            |node| {
                matches!(
                    node.tag_name().name(),
                    "COMPONENT-IREF" | "ECUC-INSTANCE-REFERENCE-VALUE"
                ) && node
                    .descendants()
                    .any(|child| child.text() == Some("/Application/Pipeline/IngressInstance"))
            },
            |_| String::new(),
        );
    }
    xml_edit(
        &mut services_only,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "Ingress")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/RteSwComponentInstance"))
                })
        },
        |_| String::new(),
    );
    rejected(
        &services_only,
        "APPLICATION_COMPONENT_MISSING",
        "composition.arxml",
        "/Application/Pipeline",
    );
}

#[test]
fn reference_shape_records_independent_verification_inputs() {
    let scratch = Scratch::new();
    let (project, _) = single_component_project(
        &scratch.0,
        autosar_config_core::target::BuildTarget::LinuxX64ControlledV1,
    );
    let inputs: serde_json::Value = serde_json::from_slice(
        &std::fs::read(project.join("meta/verification-inputs.json")).unwrap(),
    )
    .unwrap();
    // Configured endpoints of the fixture, not values read back from the plan.
    for (name, value) in [
        ("periodMs", 10),
        ("receiveCanId", 0x320),
        ("transmitCanId", 0x321),
        ("requestCanId", 0x700),
        ("responseCanId", 0x708),
        ("did", 0x1234),
        ("initialReceiveValue", 0),
        ("receiveTimeoutMs", 200),
    ] {
        assert_eq!(inputs[name], value, "{name}");
    }
    assert_eq!(inputs["format"], "autosar-ecu-test-inputs-v1");
    let readme = std::fs::read_to_string(project.join("README.md")).unwrap();
    assert!(readme.contains("tools/ecu-tool.py verify"), "{readme}");
    let config =
        std::fs::read_to_string(project.join("generated/src/Ecu_RuntimeConfig.c")).unwrap();
    assert!(
        config.contains("Ecu_DcmAuthenticationMode,\n    2u,\n};"),
        "DcmDspMaxDidToRead"
    );
    assert!(config.contains("Det_Init(&det_config);"));

    // The verifier checks one reference echo application with an application
    // DID; other shapes get no inputs (3-SWC with DID: see the README test).
    use autosar_config_core::{ApplicationSource, prepare_ecu_project_with_applications};
    let plan = build(&sources_without_application_did(true)).unwrap();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/component-pipeline/applications");
    let applications: Vec<_> = plan
        .application_slot_descriptors()
        .unwrap()
        .iter()
        .map(|slot| ApplicationSource {
            component_instance: slot
                .producer_slot
                .strip_prefix("singlecore-ecu-v1:")
                .unwrap()
                .into(),
            path: fixtures.join(slot.source_paths[0].rsplit('/').next().unwrap()),
        })
        .collect();
    let files = prepare_ecu_project_with_applications(
        &plan,
        autosar_config_core::target::BuildTarget::LinuxX64ControlledV1,
        &applications,
    )
    .unwrap()
    .into_files();
    assert!(
        !files
            .iter()
            .any(|(path, _)| path.starts_with("meta/verification"))
    );
    let readme = &files
        .iter()
        .find(|(path, _)| path == "README.md")
        .unwrap()
        .1;
    assert!(!String::from_utf8_lossy(readme).contains("ecu-tool.py verify"));
}

#[test]
fn mainline_enforces_fixed_os_class_and_did_read_limit() {
    for (owner, definition, value) in [
        ("Dsp", "DcmDspMaxDidToRead", "3"),
        ("OperatingSystem", "OsScalabilityClass", "SC2"),
    ] {
        let mut sources = single_component_sources();
        parameter(&mut sources, owner, definition, value);
        rejects_in_both(&sources, "TARGET_PARAMETER_UNSUPPORTED");
    }
    let plan = build(&single_component_sources()).unwrap();
    assert_eq!(
        plan.description()
            .diagnostic_transport
            .as_ref()
            .unwrap()
            .max_did_to_read,
        Some(2)
    );
}

/// The defect is rejected by plan construction and by normal definition
/// validation at the same source file and object.
fn rejected_at(sources: &[InputSource], code: &str, file: &str, object: &str) {
    rejected_with(sources, (code, code), file, object);
}

/// As `rejected_at`, with distinct plan and normal-validation codes.
fn rejected_with(sources: &[InputSource], (plan, normal): (&str, &str), file: &str, object: &str) {
    let issues = build(sources)
        .err()
        .unwrap_or_else(|| panic!("{plan} accepted"));
    assert!(
        issues.iter().any(|issue| issue.code == plan
            && issue.file.as_deref() == Some(file)
            && issue.object.as_deref() == Some(object)),
        "{plan}: {issues:?}"
    );
    let validation = normal_validation(sources);
    assert!(
        validation
            .diagnostics
            .iter()
            .any(|issue| issue.code == normal
                && issue.file.as_deref() == Some(file)
                && issue.path.as_deref() == Some(object)),
        "{normal}: {:?}",
        validation.diagnostics
    );
}

fn text<'a>(sources: &'a [InputSource], file: &str) -> &'a str {
    std::str::from_utf8(
        sources
            .iter()
            .find(|source| source.logical_path() == file)
            .unwrap()
            .bytes(),
    )
    .unwrap()
}

fn is_parameter(node: roxmltree::Node<'_, '_>, definition: &str) -> bool {
    node.children().any(|child| {
        child.has_tag_name("DEFINITION-REF")
            && child
                .text()
                .is_some_and(|text| text.ends_with(&format!("/{definition}")))
    })
}

#[test]
fn network_rejections_locate_extract_objects() {
    // A second ECU instance in the ECU extract. The ECU_EXTRACT system is the
    // object that must describe exactly one ECU, so the mainline locates the
    // defect there rather than at the first ECU instance.
    let mut second_ecu = single_component_sources();
    let extract = text(&second_ecu, "extract.arxml");
    let ecu = extract[extract.find("<ECU-INSTANCE>").unwrap()
        ..extract.find("</ECU-INSTANCE>").unwrap() + "</ECU-INSTANCE>".len()]
        .replacen(
            "<SHORT-NAME>ReferenceEcu</SHORT-NAME>",
            "<SHORT-NAME>ReferenceEcuSecond</SHORT-NAME>",
            1,
        );
    change(
        &mut second_ecu,
        "extract.arxml",
        "<CAN-CLUSTER>",
        &format!("{ecu}<CAN-CLUSTER>"),
    );
    rejected_at(
        &second_ecu,
        "TARGET_NOT_UNIQUE",
        "extract.arxml",
        "/Extract/ReferenceExtract",
    );

    // The received frame's port claims to transmit.
    let mut direction = single_component_sources();
    xml_edit(
        &mut direction,
        "extract.arxml",
        |node| named(node, "FRAME-PORT", "RxValueFramePort"),
        |text| text.replace(">IN<", ">OUT<"),
    );
    rejected_at(
        &direction,
        "DIRECTION_CONFLICT",
        "extract.arxml",
        "/Extract/ReferenceEcu/CanConnector/RxValueFramePort",
    );

    // A 16-bit I-signal cannot carry the 32-bit application value.
    let mut length = single_component_sources();
    xml_edit(
        &mut length,
        "extract.arxml",
        |node| named(node, "I-SIGNAL", "RxValueSignal"),
        |text| text.replace("<LENGTH>32</LENGTH>", "<LENGTH>16</LENGTH>"),
    );
    rejected_at(
        &length,
        "LENGTH_CONFLICT",
        "extract.arxml",
        "/Extract/RxValueSignal",
    );

    // An I-signal data transformation. Plan construction stops at the
    // product rule scope (the transformation is outside the supported
    // structure); normal validation names the transformer explicitly.
    let mut transformer = single_component_sources();
    xml_edit(
        &mut transformer,
        "extract.arxml",
        |node| named(node, "I-SIGNAL", "RxValueSignal"),
        |text| {
            text.replace(
                "<DATA-TYPE-POLICY>",
                "<DATA-TRANSFORMATIONS><DATA-TRANSFORMATION-REF-CONDITIONAL><DATA-TRANSFORMATION-REF DEST=\"DATA-TRANSFORMATION\">/Extract/UnsupportedTransformationSet/UnsupportedTransformation</DATA-TRANSFORMATION-REF></DATA-TRANSFORMATION-REF-CONDITIONAL></DATA-TRANSFORMATIONS><DATA-TYPE-POLICY>",
            )
        },
    );
    change(
        &mut transformer,
        "extract.arxml",
        "<SYSTEM>",
        "<DATA-TRANSFORMATION-SET><SHORT-NAME>UnsupportedTransformationSet</SHORT-NAME><DATA-TRANSFORMATIONS><DATA-TRANSFORMATION><SHORT-NAME>UnsupportedTransformation</SHORT-NAME><TRANSFORMER-CHAIN-REFS><TRANSFORMER-CHAIN-REF DEST=\"TRANSFORMATION-TECHNOLOGY\">/Extract/UnsupportedTransformationSet/UnsupportedTechnology</TRANSFORMER-CHAIN-REF></TRANSFORMER-CHAIN-REFS></DATA-TRANSFORMATION></DATA-TRANSFORMATIONS><TRANSFORMATION-TECHNOLOGYS><TRANSFORMATION-TECHNOLOGY><SHORT-NAME>UnsupportedTechnology</SHORT-NAME><PROTOCOL>ReferenceUnsupported</PROTOCOL></TRANSFORMATION-TECHNOLOGY></TRANSFORMATION-TECHNOLOGYS></DATA-TRANSFORMATION-SET><SYSTEM>",
    );
    rejected_with(
        &transformer,
        ("NATIVE_UNSUPPORTED", "TRANSFORMER_UNSUPPORTED"),
        "extract.arxml",
        "/Extract/RxValueSignal",
    );
}

#[test]
fn rejections_locate_application_objects() {
    // Unsupported component features: plan construction reports them as
    // outside the product rule scope, normal validation as an unsupported
    // component feature, both at the declaring object.
    let unsupported = ("NATIVE_UNSUPPORTED", "UNSUPPORTED_COMPONENT_FEATURE");
    let mut queued = single_component_sources();
    xml_edit(
        &mut queued,
        "ingress.arxml",
        |node| node.has_tag_name("NONQUEUED-RECEIVER-COM-SPEC"),
        |_| {
            "<QUEUED-RECEIVER-COM-SPEC><DATA-ELEMENT-REF DEST=\"VARIABLE-DATA-PROTOTYPE\">/Types/ValueInterface/Value</DATA-ELEMENT-REF><QUEUE-LENGTH>2</QUEUE-LENGTH></QUEUED-RECEIVER-COM-SPEC>".into()
        },
    );
    rejected_with(
        &queued,
        unsupported,
        "ingress.arxml",
        "/Application/Ingress/RxValue",
    );

    let mut implicit = single_component_sources();
    xml_edit(
        &mut implicit,
        "ingress.arxml",
        |node| node.has_tag_name("DATA-RECEIVE-POINT-BY-ARGUMENTS"),
        |text| text.replace("DATA-RECEIVE-POINT-BY-ARGUMENTS", "DATA-READ-ACCESSS"),
    );
    rejected_with(
        &implicit,
        unsupported,
        "ingress.arxml",
        "/Application/Ingress/Behavior/Periodic",
    );

    let mut mode = single_component_sources();
    change(
        &mut mode,
        "ingress.arxml",
        "</PORTS>",
        "<R-PORT-PROTOTYPE><SHORT-NAME>RequiredMode</SHORT-NAME><REQUIRED-INTERFACE-TREF DEST=\"MODE-SWITCH-INTERFACE\">/Application/UnsupportedModeInterface</REQUIRED-INTERFACE-TREF></R-PORT-PROTOTYPE></PORTS>",
    );
    change(
        &mut mode,
        "ingress.arxml",
        "<SYMBOL>Ingress_Periodic</SYMBOL>",
        "<MODE-ACCESS-POINTS><MODE-ACCESS-POINT><IDENT><SHORT-NAME>ReadMode</SHORT-NAME></IDENT><MODE-GROUP-IREF><R-MODE-GROUP-IN-ATOMIC-SWC-INSTANCE-REF><CONTEXT-R-PORT-REF DEST=\"R-PORT-PROTOTYPE\">/Application/Ingress/RequiredMode</CONTEXT-R-PORT-REF><TARGET-MODE-GROUP-REF DEST=\"MODE-DECLARATION-GROUP-PROTOTYPE\">/Application/UnsupportedModeInterface/Mode</TARGET-MODE-GROUP-REF></R-MODE-GROUP-IN-ATOMIC-SWC-INSTANCE-REF></MODE-GROUP-IREF></MODE-ACCESS-POINT></MODE-ACCESS-POINTS><SYMBOL>Ingress_Periodic</SYMBOL>",
    );
    change(
        &mut mode,
        "ingress.arxml",
        "</ELEMENTS>",
        "<MODE-DECLARATION-GROUP><SHORT-NAME>UnsupportedModes</SHORT-NAME><INITIAL-MODE-REF DEST=\"MODE-DECLARATION\">/Application/UnsupportedModes/Idle</INITIAL-MODE-REF><MODE-DECLARATIONS><MODE-DECLARATION><SHORT-NAME>Idle</SHORT-NAME></MODE-DECLARATION></MODE-DECLARATIONS></MODE-DECLARATION-GROUP><MODE-SWITCH-INTERFACE><SHORT-NAME>UnsupportedModeInterface</SHORT-NAME><IS-SERVICE>false</IS-SERVICE><MODE-GROUP><SHORT-NAME>Mode</SHORT-NAME><TYPE-TREF DEST=\"MODE-DECLARATION-GROUP\">/Application/UnsupportedModes</TYPE-TREF></MODE-GROUP></MODE-SWITCH-INTERFACE></ELEMENTS>",
    );
    rejected_with(
        &mode,
        unsupported,
        "ingress.arxml",
        "/Application/Ingress/Behavior/Periodic",
    );

    let mut variant = single_component_sources();
    change(
        &mut variant,
        "ingress.arxml",
        "<PORTS>",
        "<VARIATION-POINT><SHORT-LABEL>Unselected</SHORT-LABEL><SW-SYSCOND BINDING-TIME=\"CODE-GENERATION-TIME\"><SYSC-REF DEST=\"SW-SYSTEMCONST\">/Application/UnassignedSelection</SYSC-REF></SW-SYSCOND></VARIATION-POINT><PORTS>",
    );
    change(
        &mut variant,
        "ingress.arxml",
        "</ELEMENTS>",
        "<SW-SYSTEMCONST><SHORT-NAME>UnassignedSelection</SHORT-NAME></SW-SYSTEMCONST></ELEMENTS>",
    );
    rejected_with(
        &variant,
        unsupported,
        "ingress.arxml",
        "/Application/Ingress",
    );

    // Ingress declares SUPPORTS-MULTIPLE-INSTANTIATION false; the second
    // prototype of the type is the violating object.
    let mut multiple = single_component_sources();
    change(
        &mut multiple,
        "composition.arxml",
        "</COMPONENTS>",
        "<SW-COMPONENT-PROTOTYPE><SHORT-NAME>IngressSecond</SHORT-NAME><TYPE-TREF DEST=\"APPLICATION-SW-COMPONENT-TYPE\">/Application/Ingress</TYPE-TREF></SW-COMPONENT-PROTOTYPE></COMPONENTS>",
    );
    rejected_at(
        &multiple,
        "MULTIPLE_INSTANCES",
        "composition.arxml",
        "/Application/Pipeline/IngressSecond",
    );

    // The mapping set no longer maps the application type. The product rules
    // require a non-empty DATA-TYPE-MAPS wrapper, so the wrapper goes with its
    // only map; an empty wrapper is a structural (NATIVE_SCHEMA) rejection.
    let mut type_map = single_component_sources();
    xml_edit(
        &mut type_map,
        "types.arxml",
        |node| node.has_tag_name("DATA-TYPE-MAPS"),
        |_| String::new(),
    );
    rejected_at(
        &type_map,
        "TYPE_MAP_MISSING",
        "types.arxml",
        "/Types/ApplicationTypes",
    );

    // A 20 ms timing event against the 10 ms alarm. With only the event
    // changed, the 10 ms periodic Com PDU conflicts first; changing it too
    // isolates the event/alarm conflict at the timing event.
    let mut period = single_component_sources();
    change(
        &mut period,
        "ingress.arxml",
        "<PERIOD>0.01</PERIOD>",
        "<PERIOD>0.02</PERIOD>",
    );
    rejects_in_both(&period, "PERIOD_ALARM_CONFLICT");
    parameter(&mut period, "Periodic", "ComTxModeTimePeriod", "0.02");
    rejected_at(
        &period,
        "PERIOD_ALARM_CONFLICT",
        "ingress.arxml",
        "/Application/Ingress/Behavior/Periodic10ms",
    );

    // A standard BSW entry removed from its module description. The missing
    // entry is reported at the reference that names it, for a regular API
    // and for a scheduled main function.
    for (module, entry) in [("Com", "Com_SendSignal"), ("Dcm", "Dcm_MainFunction")] {
        let mut missing = single_component_sources();
        xml_edit(
            &mut missing,
            "bsw.arxml",
            |node| named(node, "BSW-MODULE-ENTRY", entry),
            |_| String::new(),
        );
        rejected_at(
            &missing,
            "BSW_ENTRY_MISSING",
            "bsw.arxml",
            &format!("/AUTOSAR_{module}/BswModuleEntrys/{entry}"),
        );
    }

    // A structurally invalid timing event. The official entrance reports
    // R24_XSD; the native mainline entrance has no XSD and rejects the unknown
    // element as outside the rule scope, normal validation the missing period.
    let mut structure = single_component_sources();
    change(
        &mut structure,
        "ingress.arxml",
        "<PERIOD>0.01</PERIOD>",
        "<NOT-A-PERIOD>0.01</NOT-A-PERIOD>",
    );
    rejected_with(
        &structure,
        ("NATIVE_UNSUPPORTED", "PERIOD_UNSUPPORTED"),
        "ingress.arxml",
        "/Application/Ingress/Behavior/Periodic10ms",
    );
}

#[test]
fn configuration_rejections_keep_identifier_width_and_order() {
    // SWS_CanTp_00067: BS is not supported (only BS 0 is selected).
    let mut block_size = single_component_sources();
    parameter(&mut block_size, "DiagRequest", "CanTpBs", "1");
    rejected_at(
        &block_size,
        "SERVICE_UNSUPPORTED",
        "ecuc.arxml",
        "/Configuration/CanTp/Config/PhysicalChannel/DiagRequest",
    );

    // A negative N-SDU identifier is outside the ECUC integer range; the
    // mainline reports the definition range before any handle derivation.
    let mut identifier = single_component_sources();
    parameter(&mut identifier, "DiagRequest", "CanTpRxNSduId", "-1");
    rejected_at(
        &identifier,
        "VALUE_RANGE",
        "ecuc.arxml",
        "/Configuration/CanTp/Config/PhysicalChannel/DiagRequest",
    );

    // The delivered ComStack_Types.h defines PduIdType as uint8_t.
    let mut wide = single_component_sources();
    parameter(&mut wide, "TxValue", "CanIfTxPduId", "256");
    rejected_at(
        &wide,
        "CANIF_HANDLE_WIDTH",
        "ecuc.arxml",
        "/Configuration/CanIf/Config/TxValue",
    );
    // SWS_Comtype_00007: a handle inside uint8 but past the PDU count leaves a gap
    // (Verifies: ECUC_CanIf_00591).
    let mut gap = single_component_sources();
    parameter(&mut gap, "TxValue", "CanIfTxPduId", "255");
    rejected_at(
        &gap,
        "CANIF_HANDLE_SEQUENCE",
        "ecuc.arxml",
        "/Configuration/CanIf/Config/TxValue",
    );

    // Without CanSM both controller notifications go to the configured host CDD
    // BusSM by UL and name (Verifies: SWS_CANIF_00014, 00450, 00524, 00558, 00560,
    // 00687, 00689, 00690, 00692, ECUC_CanIf_00250／00525／00547／00683／00684).
    // Rejects: ECUC_CanIf_00547, ECUC_CanIf_00684
    for (definition, value) in [
        ("CanIfDispatchUserCtrlBusOffUL", "CAN_SM"),
        ("CanIfDispatchUserCtrlModeIndicationUL", "CAN_SM"),
        (
            "CanIfDispatchUserCtrlModeIndicationName",
            "Other_ModeIndication",
        ),
    ] {
        let mut dispatch = single_component_sources();
        parameter(&mut dispatch, "Dispatch", definition, value);
        rejected_at(
            &dispatch,
            "CANIF_DISPATCH",
            "ecuc.arxml",
            "/Configuration/CanIf/Dispatch",
        );
    }

    // Each HOH names its own-direction Can hardware object (Verifies: SWS_CANIF_00467／00468),
    // and each Rx L-PDU its HRH (Tx L-PDUs reach the HTH through their buffer,
    // Verifies: SWS_CANIF_00466). Verifies: ECUC_CanIf_00257／00258／00259／00634／00602
    let mut hrh = single_component_sources();
    xml_edit(
        &mut hrh,
        "ecuc.arxml",
        |node| {
            node.has_tag_name("ECUC-REFERENCE-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/CanIfHrhIdSymRef"))
                })
        },
        |text| text.replace("/Can/Config/Receive<", "/Can/Config/Transmit<"),
    );
    rejected_at(
        &hrh,
        "CONTROLLER_MAPPING",
        "ecuc.arxml",
        "/Configuration/CanIf/Config/Handles/Receive",
    );
    let mut rx_pdu = single_component_sources();
    xml_edit(
        &mut rx_pdu,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "RxValue")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/CanIfRxPduCfg"))
                })
        },
        |text| text.replace("/Handles/Receive<", "/Handles/Transmit<"),
    );
    rejected_at(
        &rx_pdu,
        "CONTROLLER_MAPPING",
        "ecuc.arxml",
        "/Configuration/CanIf/Config/RxValue",
    );

    // Can_MainFunction_Write moved before Can_MainFunction_Read.
    let mut reordered = single_component_sources();
    parameter(
        &mut reordered,
        "Can_MainFunction_Read",
        "RteBswPositionInTask",
        "10",
    );
    parameter(
        &mut reordered,
        "Can_MainFunction_Write",
        "RteBswPositionInTask",
        "9",
    );
    rejects_in_both(&reordered, "SCHEDULE_ORDER");

    // A runnable symbol naming a selected BSW producer.
    let mut producer = single_component_sources();
    change(
        &mut producer,
        "ingress.arxml",
        "<SYMBOL>Ingress_ReadData</SYMBOL>",
        "<SYMBOL>Com_Init</SYMBOL>",
    );
    rejects_in_both(&producer, "SYMBOL_PRODUCER_DUPLICATE");

    // The DID array type renamed to the platform scalar name `uint32`. The
    // mainline rejects the clash with the Platform_Types name itself.
    let mut scalar = single_component_sources();
    change(
        &mut scalar,
        "types.arxml",
        "<SHORT-NAME>uint32</SHORT-NAME>",
        "<SHORT-NAME>U32</SHORT-NAME>",
    );
    replace_all(&mut scalar, "/Types/uint32", "/Types/U32");
    replace_all(
        &mut scalar,
        "Dcm_DataElement_ApplicationValueType",
        "uint32",
    );
    rejected_at(
        &scalar,
        "CONTRACT_NAME_COLLISION",
        "types.arxml",
        "/Types/uint32",
    );
}

#[test]
fn omitted_event_masks_and_unrelated_sources_are_planned() {
    // R24-11 ECUC_Os_00034: an omitted OsEventMask is AUTO. The plan assigns
    // distinct single bits in event path order. The product rules require a
    // non-empty PARAMETER-VALUES wrapper, so the wrapper goes with the mask.
    let mut automatic = single_component_sources();
    for _ in 0..3 {
        xml_edit(
            &mut automatic,
            "ecuc.arxml",
            |node| {
                node.has_tag_name("PARAMETER-VALUES")
                    && node.children().filter(|child| child.is_element()).count() == 1
                    && node
                        .children()
                        .any(|child| child.is_element() && is_parameter(child, "OsEventMask"))
            },
            |_| String::new(),
        );
    }
    assert!(!text(&automatic, "ecuc.arxml").contains("OsEventMask"));
    let plan = build(&automatic).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(
        plan.description()
            .events
            .iter()
            .map(|event| (event.path.as_str(), event.mask))
            .collect::<Vec<_>>(),
        [
            ("/Configuration/Os/Ev_App", 1),
            ("/Configuration/Os/Ev_IO", 2),
            ("/Configuration/Os/Ev_Work", 4),
        ]
    );

    // A source no integration object consumes, including a second CAN
    // cluster, is retained byte for byte without changing the plan's network.
    let unrelated = InputSource::new(
        "unrelated.arxml",
        br#"<?xml version="1.0" encoding="UTF-8"?>
<AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd">
  <AR-PACKAGES>
    <AR-PACKAGE>
      <SHORT-NAME>Unrelated</SHORT-NAME>
      <ELEMENTS>
        <SW-BASE-TYPE>
          <SHORT-NAME>Value</SHORT-NAME>
          <CATEGORY>FIXED_LENGTH</CATEGORY>
          <BASE-TYPE-SIZE>16</BASE-TYPE-SIZE>
          <BASE-TYPE-ENCODING>NONE</BASE-TYPE-ENCODING>
          <NATIVE-DECLARATION>uint16</NATIVE-DECLARATION>
        </SW-BASE-TYPE>
        <CAN-CLUSTER>
          <SHORT-NAME>OtherNetwork</SHORT-NAME>
          <CAN-CLUSTER-VARIANTS>
            <CAN-CLUSTER-CONDITIONAL>
              <BAUDRATE>250000</BAUDRATE>
              <PHYSICAL-CHANNELS>
                <CAN-PHYSICAL-CHANNEL>
                  <SHORT-NAME>OtherChannel</SHORT-NAME>
                </CAN-PHYSICAL-CHANNEL>
              </PHYSICAL-CHANNELS>
            </CAN-CLUSTER-CONDITIONAL>
          </CAN-CLUSTER-VARIANTS>
        </CAN-CLUSTER>
      </ELEMENTS>
    </AR-PACKAGE>
  </AR-PACKAGES>
</AUTOSAR>
"#
        .to_vec(),
    )
    .unwrap();
    let mut retained = single_component_sources();
    retained.push(unrelated.clone());
    let validation = normal_validation(&retained);
    assert!(
        validation.diagnostics.is_empty(),
        "{:?}",
        validation.diagnostics
    );
    let plan = build(&retained).unwrap_or_else(|issues| panic!("{issues:?}"));
    let identity = plan
        .description()
        .sources
        .iter()
        .find(|source| source.logical_path == "unrelated.arxml")
        .unwrap();
    assert_eq!(identity.roles, ["retained"]);
    for path in ["/Unrelated/Value", "/Unrelated/OtherNetwork/OtherChannel"] {
        assert!(
            plan.description()
                .objects
                .iter()
                .any(|object| object.path == path),
            "{path}"
        );
    }
    assert_eq!(
        plan.sources()
            .iter()
            .find(|source| source.logical_path() == "unrelated.arxml")
            .unwrap()
            .bytes(),
        unrelated.bytes()
    );
    assert_eq!(
        plan.description()
            .signals
            .iter()
            .map(|signal| signal.can_id)
            .collect::<Vec<_>>(),
        build(&single_component_sources())
            .unwrap()
            .description()
            .signals
            .iter()
            .map(|signal| signal.can_id)
            .collect::<Vec<_>>()
    );
}

#[test]
fn input_sources_reject_unportable_aliases_and_missing_archives() {
    let sources = single_component_sources();
    let bytes = text(&sources, "ingress.arxml").as_bytes().to_vec();
    for name in [
        "../input.arxml",
        "a/../input.arxml",
        "a//input.arxml",
        "a/./input.arxml",
        "C:/input.arxml",
        "a\\input.arxml",
        "input.arxml.",
        "a/CON.arxml",
        "a/LPT1.arxml",
        "a?/input.arxml",
        "a\u{0}/input.arxml",
    ] {
        assert!(InputSource::new(name, bytes.clone()).is_err(), "{name}");
    }
    // Case-insensitive input filesystems: ASCII and Unicode case aliases.
    let mut duplicate = sources.clone();
    duplicate.push(InputSource::new("INGRESS.ARXML", bytes.clone()).unwrap());
    let issues = build(&duplicate).err().unwrap();
    assert_eq!(issues[0].code, "SOURCE_DUPLICATE", "{issues:?}");
    assert_eq!(issues[0].file.as_deref(), Some("INGRESS.ARXML"));
    for (first, second) in [("Ä.arxml", "ä.arxml"), ("Σ.arxml", "ς.arxml")] {
        let aliases = [
            InputSource::new(first, bytes.clone()).unwrap(),
            InputSource::new(second, bytes.clone()).unwrap(),
        ];
        let issues = build(&aliases).err().unwrap();
        assert_eq!(issues[0].code, "SOURCE_DUPLICATE", "{issues:?}");
    }
    assert_eq!(text(&sources, "ingress.arxml").as_bytes(), bytes.as_slice());
}

#[test]
fn contract_names_reject_c_keywords_and_reserved_identifiers() {
    // Runnable symbols are external C producers: a keyword or reserved
    // identifier fails the producer identity check at the runnable itself.
    let runnable = Some((
        "SYMBOL_PRODUCER_DUPLICATE",
        "/Application/Ingress/Behavior/Periodic",
    ));
    for (before, after, producer) in [
        // Component `Type` would own the standard `Rte_Type.h` name.
        ("Ingress", "Type", None),
        ("Dcm_DataElement_ApplicationValueType", "RTE_H", None),
        ("Dcm_DataElement_ApplicationValueType", "restrict", None),
        ("Ingress_Periodic", "while", runnable),
        ("Ingress_Periodic", "_ReservedRunnable", runnable),
    ] {
        let mut sources = single_component_sources();
        replace_all(&mut sources, before, after);
        let issues = match build(&sources) {
            Err(issues) => issues,
            Ok(plan) => plan
                .component_contract_files()
                .err()
                .unwrap_or_else(|| panic!("{after} accepted")),
        };
        assert!(
            issues.iter().any(|issue| match producer {
                Some((code, object)) => {
                    issue.code == code
                        && issue.file.as_deref() == Some("ingress.arxml")
                        && issue.object.as_deref() == Some(object)
                }
                None => {
                    issue.code == "CONTRACT_NAME_COLLISION"
                        && issue.file.is_some()
                        && issue.object.is_some()
                }
            }),
            "{after}: {issues:?}"
        );
        assert!(
            !issues
                .iter()
                .any(|issue| issue.code == "REFERENCE_UNRESOLVED"),
            "{after}: dangling references mask the collision"
        );
    }
}

#[test]
fn source_preparation_is_target_complete_without_native_tools() {
    use autosar_config_core::prepared::PreflightStatus;
    use autosar_config_core::resources::AssetInventory;
    use autosar_config_core::target::BuildTarget;
    use autosar_config_core::{ApplicationSource, prepare_ecu_project_with_applications};
    use sha2::{Digest, Sha256};

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let inventory = AssetInventory::from_directory(root).unwrap();
    let plan = build(&single_component_sources()).unwrap();
    let applications = [ApplicationSource {
        component_instance: "/Application/Pipeline/IngressInstance".into(),
        path: Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/component-pipeline/applications/Ingress.c"),
    }];
    let mut targets = Vec::new();
    for target in BuildTarget::ALL {
        let first = prepare_ecu_project_with_applications(&plan, target, &applications).unwrap();
        let repeated = prepare_ecu_project_with_applications(&plan, target, &applications).unwrap();
        assert_eq!(first.fingerprint(), repeated.fingerprint());
        assert_eq!(first.preflight().status, PreflightStatus::NotRun);
        assert_eq!(first.preflight().fingerprint, first.fingerprint());
        assert!(first.preflight().logs.is_empty());
        let has = |path: &str| first.files().iter().any(|file| file.path == path);
        let selected: Vec<_> = first
            .files()
            .iter()
            .filter_map(|file| {
                file.source
                    .filter(|asset| asset.role == "bsw")
                    .map(|asset| asset.relative_path)
            })
            .collect();
        // The mainline owns every BSW producer, including the host CAN driver.
        for asset in [
            "runtime/bsw/src/Can.c",
            "runtime/bsw/src/Com.c",
            "runtime/bsw/src/Dcm.c",
        ] {
            assert!(selected.contains(&asset), "{asset}: {selected:?}");
        }
        let delivered = |path: &str| {
            first
                .files()
                .iter()
                .find(|file| file.path == path)
                .unwrap_or_else(|| panic!("{path} is not delivered"))
        };
        // Recorded original sources are exactly the delivered bytes.
        let integration: serde_json::Value =
            serde_json::from_slice(&delivered("meta/integration.json").bytes).unwrap();
        let originals = integration["originalBswSources"].as_object().unwrap();
        for (original, path) in originals {
            assert_eq!(
                delivered(path.as_str().unwrap()).bytes.as_ref(),
                std::fs::read(root.join(original)).unwrap(),
                "{original}"
            );
        }
        // Every runtime producer/consumer named by the contract is a selected source.
        let contract: serde_json::Value =
            serde_json::from_slice(&delivered("meta/contract.json").bytes).unwrap();
        let selected_sources: Vec<_> = first
            .files()
            .iter()
            .filter_map(|file| file.source.map(|asset| asset.relative_path))
            .collect();
        for symbol in contract["symbols"].as_array().unwrap() {
            let owners = ["declarationOwner", "definitionOwner"]
                .into_iter()
                .filter_map(|key| symbol[key].as_str())
                .chain(
                    symbol["consumers"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|value| value.as_str()),
                );
            for owner in owners.filter(|owner| owner.starts_with("runtime/")) {
                assert!(
                    selected_sources.contains(&owner),
                    "{}: {owner}",
                    symbol["symbol"]
                );
            }
        }
        let service = contract["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .find(|symbol| symbol["symbol"] == "DcmService_ReadData")
            .expect("the selection has a DID service without an event");
        assert_eq!(
            service["consumers"],
            serde_json::json!(["runtime/bsw/src/Dcm.c"])
        );
        assert_eq!(
            originals["runtime/bsw/src/Dcm.c"], "bsw/src/Dcm.c",
            "the consumer must be the delivered Dcm implementation"
        );
        assert!(has("os/src/Os.c") && has("kernel/tasks.c") && has("generated/src/Rte.c"));
        let (native_host, native_kernel) = match target {
            BuildTarget::WindowsX64ControlledV1 => (
                "os/src/host/windows/Os_HostWindows.c",
                "kernel/portable/MSVC-MingW/port.c",
            ),
            BuildTarget::LinuxX64ControlledV1 => (
                "os/src/host/linux/Os_HostLinux.c",
                "kernel/portable/ThirdParty/GCC/Posix/port.c",
            ),
        };
        assert!(
            has(native_host) && has(native_kernel),
            "{}",
            target.spec().id
        );
        // Every delivered asset is the trusted inventory byte sequence.
        for file in first.files() {
            if let Some(asset) = file.source {
                let trusted = inventory.get(asset.relative_path).unwrap();
                assert_eq!(trusted.sha256, asset.sha256, "{}", file.path);
                assert_eq!(
                    format!("{:x}", Sha256::digest(&file.bytes)),
                    asset.sha256,
                    "{}",
                    file.path
                );
            }
        }
        let metadata: serde_json::Value = serde_json::from_slice(
            &first
                .files()
                .iter()
                .find(|file| file.path == "target.json")
                .unwrap()
                .bytes,
        )
        .unwrap();
        assert_eq!(metadata["format"], "autosar-build-target-v2");
        // Each mode's main is sealed, recorded once and never a shared translation unit.
        for (mode, entry) in metadata["entries"].as_object().unwrap() {
            assert!(has(entry.as_str().unwrap()), "{mode}");
            assert!(!metadata["sources"].as_array().unwrap().contains(entry));
        }
        assert_eq!(metadata["entries"].as_object().unwrap().len(), 2);
        assert_eq!(metadata["target"], target.spec().id);
        assert_eq!(metadata["logicalClock"], target.spec().logical_clock);
        assert_eq!(metadata["objectFormat"], target.spec().object_format);
        assert_eq!(
            metadata["kernelPatches"].as_array().unwrap().len(),
            target.spec().kernel_patches.len()
        );
        targets.push((selected, first.fingerprint().to_owned()));
    }
    assert_eq!(targets[0].0, targets[1].0);
    assert_ne!(targets[0].1, targets[1].1);
    assert!(serde_json::from_str::<BuildTarget>("\"windows-x64\"").is_err());
}

/// Every CanIf parameter outside its selected value, and every CanIf reference
/// to the wrong object, is refused at the CanIf container that holds it.
/// Containers (Verifies: ECUC_CanIf_00244／00245／00246／00247／00248／00249／00253／00546／00832)
#[test]
fn canif_parameters_outside_the_selection_are_refused_at_their_source() {
    let public = "/Configuration/CanIf/Public";
    let private = "/Configuration/CanIf/Private";
    let rx = "/Configuration/CanIf/Config/RxValue";
    let tx = "/Configuration/CanIf/Config/TxValue";
    for (owner, definition, value, code, object) in [
        // Unselected optional features (Verifies: ECUC_CanIf_00847／00614／00848／00854／00866／
        // 00612／00772／00607／00608／00609／00610／00865／00618／00733／00611／00844／
        // 00837／00613／00843).
        // Rejects: ECUC_CanIf_00847
        (
            "Public",
            "CanIfBusMirroringSupport",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00614
        (
            "Public",
            "CanIfDevErrorDetect",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00848
        (
            "Public",
            "CanIfEnableSecurityEventReporting",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00854
        (
            "Public",
            "CanIfGlobalTimeSupport",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00866
        (
            "Public",
            "CanIfPublicCtrlPnEnable",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00612
        (
            "Public",
            "CanIfPublicMultipleDrvSupport",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00772
        (
            "Public",
            "CanIfPublicPnSupport",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00607
        (
            "Public",
            "CanIfPublicReadRxPduDataApi",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00608
        (
            "Public",
            "CanIfPublicReadRxPduNotifyStatusApi",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00609
        (
            "Public",
            "CanIfPublicReadTxPduNotifyStatusApi",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00610
        (
            "Public",
            "CanIfPublicSetDynamicTxIdApi",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00865
        (
            "Public",
            "CanIfPublicTrcvPnEnable",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00618
        (
            "Public",
            "CanIfPublicTxBuffering",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00733
        (
            "Public",
            "CanIfPublicTxConfirmPollingSupport",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00611
        (
            "Public",
            "CanIfPublicWakeupCheckValidSupport",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00844
        (
            "Public",
            "CanIfTriggerTransmitSupport",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00837
        (
            "Public",
            "CanIfTxOfflineActiveSupport",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00613
        (
            "Public",
            "CanIfVersionInfoApi",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Rejects: ECUC_CanIf_00843
        (
            "Public",
            "CanIfWakeupSupport",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // No fixed buffer or TTCAN, and the data length check stays on
        // (Verifies: ECUC_CanIf_00827／00675／00617); the filter and handle type are
        // the implemented ones (Verifies: ECUC_CanIf_00619／00742).
        // Rejects: ECUC_CanIf_00827
        (
            "Private",
            "CanIfFixedBuffer",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            private,
        ),
        // Rejects: ECUC_CanIf_00675
        (
            "Private",
            "CanIfSupportTTCAN",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            private,
        ),
        // Rejects: ECUC_CanIf_00617
        (
            "Private",
            "CanIfPrivateDataLengthCheck",
            "false",
            "TARGET_PARAMETER_UNSUPPORTED",
            private,
        ),
        // Rejects: ECUC_CanIf_00619
        (
            "Private",
            "CanIfPrivateSoftwareFilterType",
            "BINARY",
            "TARGET_PARAMETER_UNSUPPORTED",
            private,
        ),
        // Rejects: ECUC_CanIf_00742
        (
            "Public",
            "CanIfPublicHandleTypeEnum",
            "UINT8",
            "TARGET_PARAMETER_UNSUPPORTED",
            public,
        ),
        // Verifies: ECUC_CanIf_00637
        // Rejects: ECUC_CanIf_00637
        (
            "Controller",
            "CanIfCtrlWakeupSupport",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            "/Configuration/CanIf/Controller/Controller",
        ),
        // No CanIf Tx buffer (Verifies: ECUC_CanIf_00834).
        // Rejects: ECUC_CanIf_00834
        (
            "TransmitBuffer",
            "CanIfBufferSize",
            "1",
            "TARGET_PARAMETER_UNSUPPORTED",
            "/Configuration/CanIf/Config/TransmitBuffer",
        ),
        // Static Classical CAN L-PDUs without read access or truncation
        // (Verifies: ECUC_CanIf_00596／00590／00593／00600／00595／00589／00845).
        (
            "RxValue",
            "CanIfRxPduCanIdType",
            "STANDARD_CAN",
            "CAN_ID_CONFLICT",
            rx,
        ),
        (
            "TxValue",
            "CanIfTxPduCanIdType",
            "STANDARD_FD_CAN",
            "CAN_ID_CONFLICT",
            tx,
        ),
        // Rejects: ECUC_CanIf_00593
        (
            "TxValue",
            "CanIfTxPduType",
            "DYNAMIC",
            "TARGET_PARAMETER_UNSUPPORTED",
            tx,
        ),
        // Rejects: ECUC_CanIf_00600
        (
            "RxValue",
            "CanIfRxPduReadData",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            rx,
        ),
        // Rejects: ECUC_CanIf_00595
        (
            "RxValue",
            "CanIfRxPduReadNotifyStatus",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            rx,
        ),
        // Rejects: ECUC_CanIf_00589
        (
            "TxValue",
            "CanIfTxPduReadNotifyStatus",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            tx,
        ),
        // Rejects: ECUC_CanIf_00845
        (
            "TxValue",
            "CanIfTxPduTruncation",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            tx,
        ),
        // The L-PDU CAN IDs and data length match the selected frame
        // (Verifies: ECUC_CanIf_00598／00592／00599).
        ("RxValue", "CanIfRxPduCanId", "801", "CAN_ID_CONFLICT", rx),
        ("TxValue", "CanIfTxPduCanId", "800", "CAN_ID_CONFLICT", tx),
        (
            "RxValue",
            "CanIfRxPduDataLength",
            "3",
            "LENGTH_CONFLICT",
            rx,
        ),
    ] {
        let mut sources = single_component_sources();
        parameter(&mut sources, owner, definition, value);
        rejected_at(&sources, code, "ecuc.arxml", object);
    }
    // Verifies: ECUC_CanIf_00601／00603／00627／00831／00833／00631／00625／00636／00638／00642
    for (definition, from, to, code, object) in [
        (
            "/CanIfTxPduBufferRef",
            "/Config/TransmitBuffer<",
            "/Config/Handles<",
            "REFERENCE_TARGET",
            "/Configuration/CanIf/Config/TxValue",
        ),
        (
            "/CanIfRxPduRef",
            "/Pdus/RxValuePdu<",
            "/Pdus/TxValuePdu<",
            "SIGNAL_MAPPING",
            "/Configuration/EcuC/Config/Pdus/RxValuePdu",
        ),
        (
            "/CanIfTxPduRef",
            "/Pdus/TxValuePdu<",
            "/Pdus/RxValuePdu<",
            "SIGNAL_MAPPING",
            "/Configuration/EcuC/Config/Pdus/TxValuePdu",
        ),
        (
            "/CanIfHthIdSymRef",
            "/Can/Config/Transmit<",
            "/Can/Config/Receive<",
            "CONTROLLER_MAPPING",
            "/Configuration/CanIf/Config/Handles/Transmit",
        ),
        (
            "/CanIfBufferHthRef",
            "/Handles/Transmit<",
            "/Handles/Receive<",
            "REFERENCE_TARGET",
            "/Configuration/CanIf/Config/TransmitBuffer",
        ),
        (
            "/CanIfHrhCanCtrlIdRef",
            "/Controller/Controller<",
            "/Controller<",
            "REFERENCE_TARGET",
            "/Configuration/CanIf/Config/Handles/Receive",
        ),
        (
            "/CanIfHthCanCtrlIdRef",
            "/Controller/Controller<",
            "/Controller<",
            "REFERENCE_TARGET",
            "/Configuration/CanIf/Config/Handles/Transmit",
        ),
        (
            "/CanIfCtrlCanCtrlRef",
            "/Can/Config/Controller<",
            "/Can/Config/Receive<",
            "REFERENCE_TARGET",
            "/Configuration/CanIf/Controller/Controller",
        ),
        (
            "/CanIfCtrlDrvNameRef",
            "/Can/General<",
            "/Can/Config<",
            "REFERENCE_TARGET",
            "/Configuration/CanIf/Controller",
        ),
        (
            "/CanIfCtrlDrvInitHohConfigRef",
            "/CanIf/Config/Handles<",
            "/CanIf/Config<",
            "REFERENCE_TARGET",
            "/Configuration/CanIf/Controller",
        ),
    ] {
        let mut sources = single_component_sources();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                node.has_tag_name("ECUC-REFERENCE-VALUE")
                    && node.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF")
                            && child.text().is_some_and(|text| text.ends_with(definition))
                    })
            },
            |text| text.replacen(from, to, 1),
        );
        rejected_at(&sources, code, "ecuc.arxml", object);
    }
}

/// Containers and parameters of unselected CanIf features have no definition in
/// the selected catalog, so their source is refused before generation.
#[test]
fn canif_definitions_outside_the_selection_are_refused_at_their_source() {
    let owners = [
        ("CanIf", "", "/Configuration/CanIf"),
        ("Config", "/CanIfInitCfg", "/Configuration/CanIf/Config"),
        (
            "Receive",
            "/CanIfInitCfg/CanIfInitHohCfg/CanIfHrhCfg",
            "/Configuration/CanIf/Config/Handles/Receive",
        ),
        (
            "RxValue",
            "/CanIfInitCfg/CanIfRxPduCfg",
            "/Configuration/CanIf/Config/RxValue",
        ),
        (
            "TxValue",
            "/CanIfInitCfg/CanIfTxPduCfg",
            "/Configuration/CanIf/Config/TxValue",
        ),
        ("Public", "/CanIfPublicCfg", "/Configuration/CanIf/Public"),
        (
            "Dispatch",
            "/CanIfDispatchCfg",
            "/Configuration/CanIf/Dispatch",
        ),
    ];
    // (owner, definition, "" for a container or "KIND:value" for a parameter)
    for (owner, name, value) in [
        // Rejects: ECUC_CanIf_00273
        ("CanIf", "CanIfTrcvDrvCfg", ""),
        // Rejects: ECUC_CanIf_00849
        ("CanIf", "CanIfSecurityEventRefs", ""),
        // Rejects: ECUC_CanIf_00628
        ("Receive", "CanIfHrhRangeCfg", ""),
        // Rejects: ECUC_CanIf_00743
        ("RxValue", "CanIfRxPduCanIdRange", ""),
        // Rejects: ECUC_CanIf_00860
        ("RxValue", "CanIfRxPduXLParams", ""),
        // Rejects: ECUC_CanIf_00855
        ("TxValue", "CanIfTxPduXLParams", ""),
        // Rejects: ECUC_CanIf_00822
        ("RxValue", "CanIfRxPduCanIdMask", "INTEGER:2047"),
        // Rejects: ECUC_CanIf_00823
        ("TxValue", "CanIfTxPduCanIdMask", "INTEGER:2047"),
        // Rejects: ECUC_CanIf_00840
        ("TxValue", "CanIfTxPduTriggerTransmit", "BOOLEAN:false"),
        // Rejects: ECUC_CanIf_00773
        ("TxValue", "CanIfTxPduPnFilterPdu", "BOOLEAN:false"),
        // Rejects: ECUC_CanIf_00838
        ("Public", "CanIfSetBaudrateApi", "BOOLEAN:false"),
        // Rejects: ECUC_CanIf_00824
        ("Public", "CanIfMetaDataSupport", "BOOLEAN:false"),
        // Rejects: ECUC_CanIf_00741
        ("Public", "CanIfPublicWakeupCheckValidByNM", "BOOLEAN:false"),
        // Rejects: ECUC_CanIf_00671
        ("Public", "CanIfPublicCddHeaderFile", "STRING:Cdd.h"),
        // Rejects: ECUC_CanIf_00828
        ("Config", "CanIfMaxBufferSize", "INTEGER:0"),
        // Rejects: ECUC_CanIf_00829
        ("Config", "CanIfMaxTxPduCfg", "INTEGER:2"),
        // Rejects: ECUC_CanIf_00830
        ("Config", "CanIfMaxRxPduCfg", "INTEGER:2"),
        // Rejects: ECUC_CanIf_00686
        (
            "Dispatch",
            "CanIfDispatchUserTrcvModeIndicationUL",
            "ENUMERATION:CDD",
        ),
        // Rejects: ECUC_CanIf_00790
        (
            "Dispatch",
            "CanIfDispatchUserClearTrcvWufFlagIndicationUL",
            "ENUMERATION:CDD",
        ),
        // Rejects: ECUC_CanIf_00792
        (
            "Dispatch",
            "CanIfDispatchUserCheckTrcvWakeFlagIndicationUL",
            "ENUMERATION:CDD",
        ),
        // Rejects: ECUC_CanIf_00549
        (
            "Dispatch",
            "CanIfDispatchUserValidateWakeupEventUL",
            "ENUMERATION:ECUM",
        ),
        // Rejects: ECUC_CanIf_00820
        (
            "Dispatch",
            "CanIfDispatchUserConfirmPnAvailabilityUL",
            "ENUMERATION:CAN_SM",
        ),
        // Rejects: ECUC_CanIf_00868
        (
            "Dispatch",
            "CanIfDispatchUserConfirmCtrlPnAvailabilityUL",
            "ENUMERATION:CAN_SM",
        ),
    ] {
        let (_, path, object) = owners.iter().find(|row| row.0 == owner).unwrap();
        let definition = format!("/AUTOSAR/EcucDefs/CanIf{path}/{name}");
        let mut sources = single_component_sources();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                matches!(
                    node.tag_name().name(),
                    "ECUC-CONTAINER-VALUE" | "ECUC-MODULE-CONFIGURATION-VALUES"
                ) && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text == format!("/AUTOSAR/EcucDefs/CanIf{path}"))
                })
            },
            |text| match value.split_once(':') {
                Some((kind, value)) => {
                    let element = match kind {
                        "INTEGER" | "BOOLEAN" => "NUMERICAL",
                        _ => "TEXTUAL",
                    };
                    let parameter = format!(
                        "<ECUC-{element}-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-{kind}-PARAM-DEF\">{definition}</DEFINITION-REF><VALUE>{value}</VALUE></ECUC-{element}-PARAM-VALUE>"
                    );
                    text.replacen(
                        "<PARAMETER-VALUES>",
                        &format!("<PARAMETER-VALUES>{parameter}"),
                        1,
                    )
                }
                None => {
                    let container = format!(
                        "<ECUC-CONTAINER-VALUE><SHORT-NAME>{name}</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">{definition}</DEFINITION-REF></ECUC-CONTAINER-VALUE>"
                    );
                    if owner == "CanIf" {
                        text.replacen("<CONTAINERS>", &format!("<CONTAINERS>{container}"), 1)
                    } else {
                        // After the owner's references, before its closing tag.
                        let at = text.rfind("</REFERENCE-VALUES>").unwrap()
                            + "</REFERENCE-VALUES>".len();
                        format!(
                            "{}<SUB-CONTAINERS>{container}</SUB-CONTAINERS>{}",
                            &text[..at],
                            &text[at..]
                        )
                    }
                }
            },
        );
        let object = if value.is_empty() {
            format!("{object}/{name}")
        } else {
            (*object).to_owned()
        };
        rejected_with(
            &sources,
            ("REFERENCE_UNRESOLVED", "DEFINITION_UNKNOWN"),
            "ecuc.arxml",
            &object,
        );
    }
}

/// The selected controller has one HRH and one HTH; a second one is refused.
#[test]
fn second_canif_hardware_object_is_refused() {
    // Rejects: ECUC_CanIf_00259, ECUC_CanIf_00258
    for (name, definition) in [("Receive", "/CanIfHrhCfg"), ("Transmit", "/CanIfHthCfg")] {
        let mut sources = single_component_sources();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                named(node, "ECUC-CONTAINER-VALUE", name)
                    && node.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF")
                            && child.text().is_some_and(|text| text.ends_with(definition))
                    })
            },
            |text| {
                let copy = text.replacen(
                    &format!("<SHORT-NAME>{name}</SHORT-NAME>"),
                    &format!("<SHORT-NAME>{name}Second</SHORT-NAME>"),
                    1,
                );
                format!("{text}{copy}")
            },
        );
        rejected_at(
            &sources,
            "CONFIGURATION_NOT_UNIQUE",
            "ecuc.arxml",
            "/Configuration/Can/Config/Controller",
        );
    }
}

/// Configuration is generated before compilation, so a module declaring the
/// link-time or post-build variant is refused (link-time CanIf: SWS_CANIF_00378).
#[test]
fn link_time_and_post_build_variants_are_refused_at_the_module() {
    // Rejects: ECUC_CanIf_00244, ECUC_PduR_00293, ECUC_Com_00540
    for (module, variant) in [
        ("CanIf", "VARIANT-LINK-TIME"),
        ("CanIf", "VARIANT-POST-BUILD"),
        ("PduR", "VARIANT-POST-BUILD"),
        ("Com", "VARIANT-LINK-TIME"),
        ("Com", "VARIANT-POST-BUILD"),
    ] {
        let mut sources = single_component_sources();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| named(node, "ECUC-MODULE-CONFIGURATION-VALUES", module),
            |text| text.replacen("VARIANT-PRE-COMPILE", variant, 1),
        );
        rejected_at(
            &sources,
            "TARGET_PARAMETER_UNSUPPORTED",
            "ecuc.arxml",
            &format!("/Configuration/{module}"),
        );
    }
}

/// LSduR routes are derived from the PduR, CanIf and CanTp configuration; an LSduR
/// module is refused where it is declared.
#[test]
fn lsdur_module_configuration_is_refused_at_the_module() {
    // Rejects: ECUC_LSduR_00001
    let mut sources = single_component_sources();
    change(
        &mut sources,
        "ecuc.arxml",
        "<ECUC-MODULE-CONFIGURATION-VALUES>",
        "<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>LSduR</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-MODULE-DEF\">/AUTOSAR/EcucDefs/LSduR</DEFINITION-REF></ECUC-MODULE-CONFIGURATION-VALUES><ECUC-MODULE-CONFIGURATION-VALUES>",
    );
    rejected_at(
        &sources,
        "MODULE_UNSUPPORTED",
        "ecuc.arxml",
        "/Configuration/LSduR",
    );
}

/// PduR features outside one-to-one IF and TP routing between the selected modules
/// are refused where they are configured.
#[test]
fn pdur_features_outside_the_selection_are_refused_at_their_source() {
    // Unselected switches (Verifies: ECUC_PduR_00302／00347／00316／00317).
    for definition in [
        // Rejects: ECUC_PduR_00302
        "PduRDevErrorDetect",
        // Rejects: ECUC_PduR_00347
        "PduRMetaDataSupport",
        // Rejects: ECUC_PduR_00316
        "PduRVersionInfoApi",
        // Rejects: ECUC_PduR_00317
        "PduRZeroCostOperation",
    ] {
        let mut sources = single_component_sources();
        parameter(&mut sources, "General", definition, "true");
        rejected_at(
            &sources,
            "TARGET_PARAMETER_UNSUPPORTED",
            "ecuc.arxml",
            "/Configuration/PduR/General",
        );
    }
    // (owner, definition below the owner's, "" for a container or "KIND:value")
    for (owner, name, value) in [
        // Rejects: ECUC_PduR_00295
        ("PduR", "PduRBswModules", ""),
        // Rejects: ECUC_PduR_00365
        ("PduR", "PduRDemEventParameterRefs", ""),
        // Rejects: ECUC_PduR_00308
        ("Routes", "PduRRoutingPathGroup", ""),
        // Rejects: ECUC_PduR_00336
        ("Routes", "PduRBuffer", ""),
        // Rejects: ECUC_PduR_00350
        ("Routes", "PduRMaxRoutingPathCnt", "INTEGER:4"),
        // Rejects: ECUC_PduR_00348
        ("Routes", "PduRMaxRoutingPathGroupCnt", "INTEGER:1"),
        // Rejects: ECUC_PduR_00355
        ("TxValueSource", "PduRSourcePduBlockSize", "INTEGER:8"),
        // Rejects: ECUC_PduR_00289
        (
            "TxValueDestination",
            "PduRDestPduDataProvision",
            "ENUMERATION:PDUR_TRIGGERTRANSMIT",
        ),
        // Rejects: ECUC_PduR_00299
        ("TxValue", "PduRDefaultValue", ""),
        // Rejects: ECUC_PduR_00356
        ("TxValue", "PduRQueueDepth", "INTEGER:1"),
        // Rejects: ECUC_PduR_00367
        ("TxValue", "PduRQueueingStrategy", "ENUMERATION:FIFO"),
        // Rejects: ECUC_PduR_00320
        ("TxValue", "PduRTpThreshold", "INTEGER:8"),
    ] {
        let (path, object) = match owner {
            "PduR" => ("", "/Configuration/PduR".to_owned()),
            "Routes" => ("/PduRRoutingPaths", "/Configuration/PduR/Routes".to_owned()),
            "TxValueSource" => (
                "/PduRRoutingPaths/PduRSrcPdu",
                format!("/Configuration/PduR/Routes/{owner}"),
            ),
            "TxValueDestination" => (
                "/PduRRoutingPaths/PduRDestPdu",
                format!("/Configuration/PduR/Routes/{owner}"),
            ),
            _ => (
                "/PduRRoutingPaths/PduRRoutingPath",
                format!("/Configuration/PduR/Routes/{owner}"),
            ),
        };
        let definition = format!("/AUTOSAR/EcucDefs/PduR{path}/{name}");
        let mut sources = single_component_sources();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            // Short names such as TxValue recur in Com and CanIf; match the definition too.
            |node| {
                named(node, node.tag_name().name(), owner)
                    && node.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF")
                            && child.text() == Some(&*format!("/AUTOSAR/EcucDefs/PduR{path}"))
                    })
            },
            |text| match value.split_once(':') {
                Some((kind, value)) => {
                    let element = if kind == "INTEGER" {
                        "NUMERICAL"
                    } else {
                        "TEXTUAL"
                    };
                    let parameter = format!(
                        "<ECUC-{element}-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-{kind}-PARAM-DEF\">{definition}</DEFINITION-REF><VALUE>{value}</VALUE></ECUC-{element}-PARAM-VALUE>"
                    );
                    if text.contains("<PARAMETER-VALUES>") {
                        text.replacen(
                            "<PARAMETER-VALUES>",
                            &format!("<PARAMETER-VALUES>{parameter}"),
                            1,
                        )
                    } else {
                        text.replacen(
                            "<REFERENCE-VALUES>",
                            &format!(
                                "<PARAMETER-VALUES>{parameter}</PARAMETER-VALUES><REFERENCE-VALUES>"
                            ),
                            1,
                        )
                    }
                }
                None => {
                    let container = format!(
                        "<ECUC-CONTAINER-VALUE><SHORT-NAME>{name}</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">{definition}</DEFINITION-REF></ECUC-CONTAINER-VALUE>"
                    );
                    match owner {
                        "PduR" => {
                            text.replacen("<CONTAINERS>", &format!("<CONTAINERS>{container}"), 1)
                        }
                        "Routes" => text.replacen(
                            "<SUB-CONTAINERS>",
                            &format!("<SUB-CONTAINERS>{container}"),
                            1,
                        ),
                        // After the routing path's references, before its closing tag.
                        _ => {
                            let at = text.rfind("</REFERENCE-VALUES>").unwrap()
                                + "</REFERENCE-VALUES>".len();
                            format!(
                                "{}<SUB-CONTAINERS>{container}</SUB-CONTAINERS>{}",
                                &text[..at],
                                &text[at..]
                            )
                        }
                    }
                }
            },
        );
        let object = if value.is_empty() {
            format!("{object}/{name}")
        } else {
            object
        };
        rejected_with(
            &sources,
            ("REFERENCE_UNRESOLVED", "DEFINITION_UNKNOWN"),
            "ecuc.arxml",
            &object,
        );
    }
}

/// Com values outside the selected immediate UINT32 profile are refused at the Com
/// container that holds them.
#[test]
fn com_parameters_outside_the_selection_are_refused_at_their_source() {
    let general = "/Configuration/Com/General";
    let rx = "/Configuration/Com/Config/RxValue";
    let tx = "/Configuration/Com/Config/TxValue";
    let rx_pdu = "/Configuration/Com/Config/RxValuePdu";
    let transmit = "/Configuration/Com/Config/TxValuePdu/Transmit";
    let mode = "/Configuration/Com/Config/TxValuePdu/Transmit/ModeTrue/Periodic";
    let callbacks = "/Configuration/Rte/ComUser/Callbacks";
    for (owner, definition, value, code, object) in [
        // Rejects: ECUC_Com_10042
        (
            "General",
            "ComEnableSecurityEventReporting",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            general,
        ),
        // Rejects: ECUC_Com_10002
        (
            "General",
            "ComEnableSignalGroupArrayApi",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            general,
        ),
        // Rejects: ECUC_Com_00438
        (
            "General",
            "ComVersionInfoApi",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            general,
        ),
        // Rejects: ECUC_Com_00127
        ("RxValue", "ComSignalType", "UINT8_DYN", "TYPE_CONFLICT", rx),
        ("RxValue", "ComSignalType", "UINT8_N", "TYPE_CONFLICT", rx),
        ("RxValue", "ComSignalType", "SINT32", "TYPE_CONFLICT", rx),
        ("RxValue", "ComSignalType", "FLOAT32", "TYPE_CONFLICT", rx),
        // Rejects: ECUC_Com_00157
        (
            "RxValue",
            "ComSignalEndianness",
            "BIG_ENDIAN",
            "TYPE_CONFLICT",
            rx,
        ),
        (
            "RxValue",
            "ComSignalEndianness",
            "OPAQUE",
            "TYPE_CONFLICT",
            rx,
        ),
        // Rejects: ECUC_Com_00158
        ("RxValue", "ComBitSize", "0", "TYPE_CONFLICT", rx),
        // Rejects: ECUC_Com_00560
        (
            "TxValue",
            "ComTransferProperty",
            "TRIGGERED",
            "TYPE_CONFLICT",
            tx,
        ),
        (
            "TxValue",
            "ComTransferProperty",
            "TRIGGERED_ON_CHANGE",
            "TYPE_CONFLICT",
            tx,
        ),
        // Rejects: ECUC_Com_00412
        (
            "RxValue",
            "ComRxDataTimeoutAction",
            "REPLACE",
            "TIMEOUT_CONFLICT",
            rx,
        ),
        (
            "RxValue",
            "ComRxDataTimeoutAction",
            "SUBSTITUTE",
            "TIMEOUT_CONFLICT",
            rx,
        ),
        // Rejects: ECUC_Com_00263
        ("RxValue", "ComTimeout", "0.031", "TIMEOUT_CONFLICT", rx),
        // Rejects: ECUC_Com_00183
        ("RxValue", "ComFirstTimeout", "0.05", "TIMEOUT_CONFLICT", rx),
        // Rejects: ECUC_Com_00119
        (
            "RxValuePdu",
            "ComIPduSignalProcessing",
            "DEFERRED",
            "COM_FEATURE_UNSUPPORTED",
            rx_pdu,
        ),
        // Rejects: ECUC_Com_00761
        (
            "RxValuePdu",
            "ComIPduType",
            "TP",
            "TARGET_PARAMETER_UNSUPPORTED",
            rx_pdu,
        ),
        // Rejects: ECUC_Com_00181
        (
            "Transmit",
            "ComMinimumDelayTime",
            "0.02",
            "COM_FEATURE_UNSUPPORTED",
            transmit,
        ),
        // Rejects: ECUC_Com_00137
        (
            "Periodic",
            "ComTxModeMode",
            "DIRECT",
            "PERIOD_ALARM_CONFLICT",
            mode,
        ),
        (
            "Periodic",
            "ComTxModeMode",
            "MIXED",
            "PERIOD_ALARM_CONFLICT",
            mode,
        ),
        (
            "Periodic",
            "ComTxModeMode",
            "NONE",
            "PERIOD_ALARM_CONFLICT",
            mode,
        ),
        // Rejects: ECUC_Com_10034
        (
            "Reception",
            "ComUserCallbackType",
            "COM_TX_ACK",
            "COM_CALLBACK",
            callbacks,
        ),
        (
            "Reception",
            "ComUserCallbackType",
            "COM_TX_ERR",
            "COM_CALLBACK",
            callbacks,
        ),
        (
            "Reception",
            "ComUserCallbackType",
            "COM_TX_TOUT",
            "COM_CALLBACK",
            callbacks,
        ),
        (
            "Reception",
            "ComUserCallbackType",
            "COM_INV",
            "COM_CALLBACK",
            callbacks,
        ),
    ] {
        let mut sources = single_component_sources();
        parameter(&mut sources, owner, definition, value);
        rejected_at(&sources, code, "ecuc.arxml", object);
    }
}

/// Containers and parameters of unselected Com features have no definition in the
/// selected catalog, or are refused where the selected profile cannot honour them.
#[test]
fn com_features_outside_the_selection_are_refused_at_their_source() {
    let com = "/AUTOSAR/EcucDefs/Com";
    let owners = [
        (
            "General",
            format!("{com}/ComGeneral"),
            "/Configuration/Com/General",
        ),
        (
            "Config",
            format!("{com}/ComConfig"),
            "/Configuration/Com/Config",
        ),
        (
            "Tx",
            format!("{com}/ComConfig/ComMainFunctionTx"),
            "/Configuration/Com/Config/Tx",
        ),
        (
            "RxValuePdu",
            format!("{com}/ComConfig/ComIPdu"),
            "/Configuration/Com/Config/RxValuePdu",
        ),
        (
            "Transmit",
            format!("{com}/ComConfig/ComIPdu/ComTxIPdu"),
            "/Configuration/Com/Config/TxValuePdu/Transmit",
        ),
        (
            "Periodic",
            format!("{com}/ComConfig/ComIPdu/ComTxIPdu/ComTxModeTrue/ComTxMode"),
            "/Configuration/Com/Config/TxValuePdu/Transmit/ModeTrue/Periodic",
        ),
        (
            "RxValue",
            format!("{com}/ComConfig/ComSignal"),
            "/Configuration/Com/Config/RxValue",
        ),
        (
            "TxValue",
            format!("{com}/ComConfig/ComSignal"),
            "/Configuration/Com/Config/TxValue",
        ),
        (
            "Callbacks",
            "/AUTOSAR/EcucDefs/Rte/RteComUser/ComUserModuleCnf".to_owned(),
            "/Configuration/Rte/ComUser/Callbacks",
        ),
    ];
    let unknown = ("REFERENCE_UNRESOLVED", "DEFINITION_UNKNOWN");
    // (owner, definition below the owner's, "" for a container or "KIND:value", codes)
    for (owner, name, value, codes) in [
        // Rejects: ECUC_Com_10000
        (
            "General",
            "ComCancellationSupport",
            "BOOLEAN:false",
            unknown,
        ),
        // Rejects: ECUC_Com_00141
        (
            "General",
            "ComConfigurationUseDet",
            "BOOLEAN:false",
            unknown,
        ),
        // Rejects: ECUC_Com_00788
        (
            "General",
            "ComEnableMDTForCyclicTransmission",
            "BOOLEAN:false",
            unknown,
        ),
        // Rejects: ECUC_Com_10004
        ("General", "ComMetaDataSupport", "BOOLEAN:false", unknown),
        // Rejects: ECUC_Com_00780
        (
            "General",
            "ComRetryFailedTransmitRequests",
            "BOOLEAN:false",
            unknown,
        ),
        // Rejects: ECUC_Com_10001
        ("General", "ComUserCbkHeaderFile", "STRING:User.h", unknown),
        // Rejects: ECUC_Com_10041
        ("General", "ComSecurityEventRefs", "", unknown),
        // Rejects: ECUC_Com_00783
        ("Config", "ComDataMemSize", "INTEGER:64", unknown),
        // Rejects: ECUC_Com_00782
        ("Config", "ComMaxIPduCnt", "INTEGER:2", unknown),
        // Rejects: ECUC_Com_10013
        ("Config", "ComMainFunctionRouteSignals", "", unknown),
        // Rejects: ECUC_Com_00345
        ("Config", "ComSignalGroup", "", unknown),
        // Rejects: ECUC_Com_00544
        ("Config", "ComGwMapping", "", unknown),
        // Rejects: ECUC_Com_10031
        ("Config", "ComUserModule", "", unknown),
        // Rejects: ECUC_Com_10020
        (
            "Tx",
            "ComPreparationNotification",
            "FUNCTION-NAME:Prepare",
            unknown,
        ),
        // Rejects: ECUC_Com_00709
        (
            "RxValuePdu",
            "ComIPduCancellationSupport",
            "BOOLEAN:false",
            unknown,
        ),
        // Rejects: ECUC_Com_00765
        (
            "RxValuePdu",
            "ComIPduTriggerTransmitCallout",
            "FUNCTION-NAME:Callout",
            unknown,
        ),
        // Rejects: ECUC_Com_00387
        (
            "RxValuePdu",
            "ComIPduCallout",
            "FUNCTION-NAME:Callout",
            ("COM_FEATURE_UNSUPPORTED", "COM_FEATURE_UNSUPPORTED"),
        ),
        // Rejects: ECUC_Com_00576
        (
            "Transmit",
            "ComTxIPduClearUpdateBit",
            "ENUMERATION:TRANSMIT",
            unknown,
        ),
        // Rejects: ECUC_Com_10022
        ("Transmit", "ComMetaDataDefaultItem", "", unknown),
        // Rejects: ECUC_Com_00454
        (
            "Transmit",
            "ComTxModeFalse",
            "",
            ("PERIOD_ALARM_CONFLICT", "PERIOD_ALARM_CONFLICT"),
        ),
        // Rejects: ECUC_Com_00281
        (
            "Periodic",
            "ComTxModeNumberOfRepetitions",
            "INTEGER:1",
            unknown,
        ),
        // Rejects: ECUC_Com_00282
        (
            "Periodic",
            "ComTxModeRepetitionPeriod",
            "FLOAT:0.01",
            unknown,
        ),
        // Rejects: ECUC_Com_00180
        ("Periodic", "ComTxModeTimeOffset", "FLOAT:0.01", unknown),
        // Rejects: ECUC_Com_00339
        ("RxValue", "ComFilter", "", unknown),
        // Rejects: ECUC_Com_00314
        (
            "RxValue",
            "ComDataInvalidAction",
            "ENUMERATION:NOTIFY",
            unknown,
        ),
        // Rejects: ECUC_Com_00811
        ("RxValue", "ComInitialValueOnly", "BOOLEAN:false", unknown),
        // Rejects: ECUC_Com_00391
        ("RxValue", "ComSignalDataInvalidValue", "STRING:0", unknown),
        // Rejects: ECUC_Com_00437
        ("RxValue", "ComSignalLength", "INTEGER:4", unknown),
        // Rejects: ECUC_Com_10006
        (
            "RxValue",
            "ComTimeoutSubstitutionValue",
            "STRING:0",
            unknown,
        ),
        // Rejects: ECUC_Com_00257
        ("RxValue", "ComUpdateBitPosition", "INTEGER:32", unknown),
        // Rejects: ECUC_Com_00263, ECUC_Com_00183
        (
            "TxValue",
            "ComTimeout",
            "FLOAT:0.05",
            ("TIMEOUT_CONFLICT", "TIMEOUT_CONFLICT"),
        ),
        (
            "TxValue",
            "ComFirstTimeout",
            "FLOAT:0.05",
            ("TIMEOUT_CONFLICT", "TIMEOUT_CONFLICT"),
        ),
        // Rejects: ECUC_Com_00165
        (
            "RxValue",
            "ComHandleId",
            "INTEGER:0",
            (
                "TARGET_PARAMETER_UNSUPPORTED",
                "TARGET_PARAMETER_UNSUPPORTED",
            ),
        ),
        // Rejects: ECUC_Com_00175
        (
            "RxValuePdu",
            "ComIPduHandleId",
            "INTEGER:0",
            (
                "TARGET_PARAMETER_UNSUPPORTED",
                "TARGET_PARAMETER_UNSUPPORTED",
            ),
        ),
        // Rejects: ECUC_Com_10027
        ("Callbacks", "ComUserSignalGroup", "", unknown),
    ] {
        let (_, path, object) = owners.iter().find(|row| row.0 == owner).unwrap();
        let definition = format!("{path}/{name}");
        let mut sources = single_component_sources();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            // Short names such as RxValue recur in CanIf and PduR; match the definition too.
            |node| {
                named(node, "ECUC-CONTAINER-VALUE", owner)
                    && node.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF") && child.text() == Some(path.as_str())
                    })
            },
            |text| {
                let close = "</ECUC-CONTAINER-VALUE>";
                let body = text.trim_end().strip_suffix(close).unwrap();
                match value.split_once(':') {
                    Some((kind, value)) => {
                        let element = match kind {
                            "INTEGER" | "FLOAT" | "BOOLEAN" => "NUMERICAL",
                            _ => "TEXTUAL",
                        };
                        let dest = match kind {
                            "FUNCTION-NAME" => "ECUC-FUNCTION-NAME-DEF".to_owned(),
                            _ => format!("ECUC-{kind}-PARAM-DEF"),
                        };
                        let parameter = format!(
                            "<ECUC-{element}-PARAM-VALUE><DEFINITION-REF DEST=\"{dest}\">{definition}</DEFINITION-REF><VALUE>{value}</VALUE></ECUC-{element}-PARAM-VALUE>"
                        );
                        // The owner's own parameters precede its sub-containers.
                        let at =
                            text.find("</DEFINITION-REF>").unwrap() + "</DEFINITION-REF>".len();
                        if text[at..].trim_start().starts_with("<PARAMETER-VALUES>") {
                            text.replacen(
                                "<PARAMETER-VALUES>",
                                &format!("<PARAMETER-VALUES>{parameter}"),
                                1,
                            )
                        } else {
                            format!(
                                "{}<PARAMETER-VALUES>{parameter}</PARAMETER-VALUES>{}",
                                &text[..at],
                                &text[at..]
                            )
                        }
                    }
                    None => {
                        // ComTxModeFalse is complete with the mode the runtime assumes when absent.
                        let content = if name == "ComTxModeFalse" {
                            format!(
                                "<SUB-CONTAINERS><ECUC-CONTAINER-VALUE><SHORT-NAME>None</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">{definition}/ComTxMode</DEFINITION-REF><PARAMETER-VALUES><ECUC-TEXTUAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-ENUMERATION-PARAM-DEF\">{definition}/ComTxMode/ComTxModeMode</DEFINITION-REF><VALUE>NONE</VALUE></ECUC-TEXTUAL-PARAM-VALUE></PARAMETER-VALUES></ECUC-CONTAINER-VALUE></SUB-CONTAINERS>"
                            )
                        } else {
                            String::new()
                        };
                        let container = format!(
                            "<ECUC-CONTAINER-VALUE><SHORT-NAME>{name}</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">{definition}</DEFINITION-REF>{content}</ECUC-CONTAINER-VALUE>"
                        );
                        // The owner's own sub-containers close last.
                        match body.trim_end().strip_suffix("</SUB-CONTAINERS>") {
                            Some(inner) => format!("{inner}{container}</SUB-CONTAINERS>{close}"),
                            None => {
                                format!("{body}<SUB-CONTAINERS>{container}</SUB-CONTAINERS>{close}")
                            }
                        }
                    }
                }
            },
        );
        let object = if name == "ComTxModeFalse" {
            // A second transmission mode is refused at the I-PDU that selects between them.
            "/Configuration/Com/Config/TxValuePdu".to_owned()
        } else if value.is_empty() {
            format!("{object}/{name}")
        } else {
            (*object).to_owned()
        };
        rejected_with(&sources, codes, "ecuc.arxml", &object);
    }
}

#[test]
fn pdur_mandatory_general_and_configuration_id_are_required_and_generated() {
    // Verifies: ECUC_PduR_00305, ECUC_PduR_00327
    let mut sources = single_component_sources();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "General")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text() == Some("/AUTOSAR/EcucDefs/PduR/PduRGeneral")
                })
        },
        |_| String::new(),
    );
    rejected_at(
        &sources,
        "MULTIPLICITY",
        "ecuc.arxml",
        "/Configuration/PduR",
    );
    let mut sources = single_component_sources();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.has_tag_name("ECUC-NUMERICAL-PARAM-VALUE")
                && node.children().any(|child| {
                    child.text()
                        == Some("/AUTOSAR/EcucDefs/PduR/PduRRoutingPaths/PduRConfigurationId")
                })
        },
        |_| String::new(),
    );
    rejected_with(
        &sources,
        ("NATIVE_SCHEMA", "MULTIPLICITY"),
        "ecuc.arxml",
        "/Configuration/PduR/Routes",
    );
    // The configuration is identified by PduRConfigurationId and lives in PduR_PBcfg.c
    // (Verifies: SWS_PduR_00281, 00241).
    let mut sources = single_component_sources();
    parameter(&mut sources, "Routes", "PduRConfigurationId", "7");
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    let files = prepared_pipeline_files(&plan);
    let config = files
        .iter()
        .find(|(name, _)| name == "generated/src/PduR_PBcfg.c")
        .map(|(_, bytes)| std::str::from_utf8(bytes).unwrap())
        .unwrap();
    assert!(
        config.contains("PduR_Config RTE_PduR_CONFIG_DATA_PREBUILD_UNSPECIFIED = {\n    7u,\n"),
        "{config}"
    );
}

#[test]
fn pdur_fan_out_fan_in_and_gateway_routes_are_refused_at_their_source() {
    // Rejects: ECUC_PduR_00354
    let mut sources = single_component_sources();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "TxValue")
                && node.children().any(|child| {
                    child.text() == Some("/AUTOSAR/EcucDefs/PduR/PduRRoutingPaths/PduRRoutingPath")
                })
        },
        |text| {
            text.replacen(
                "</REFERENCE-VALUES>",
                "<ECUC-REFERENCE-VALUE><DEFINITION-REF DEST=\"ECUC-REFERENCE-DEF\">/AUTOSAR/EcucDefs/PduR/PduRRoutingPaths/PduRRoutingPath/PduRDestPduRRef</DEFINITION-REF><VALUE-REF DEST=\"ECUC-CONTAINER-VALUE\">/Configuration/PduR/Routes/RxValueDestination</VALUE-REF></ECUC-REFERENCE-VALUE></REFERENCE-VALUES>",
                1,
            )
        },
    );
    rejected_at(
        &sources,
        "PDU_ROUTE_NOT_UNIQUE",
        "ecuc.arxml",
        "/Configuration/PduR/Routes/TxValueSource",
    );
    // Two routing paths into one destination (n:1).
    let mut sources = single_component_sources();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "RxValue")
                && node.children().any(|child| {
                    child.text() == Some("/AUTOSAR/EcucDefs/PduR/PduRRoutingPaths/PduRRoutingPath")
                })
        },
        |text| {
            text.replacen(
                "/Routes/RxValueDestination<",
                "/Routes/TxValueDestination<",
                1,
            )
        },
    );
    rejected_at(
        &sources,
        "PDU_ROUTE_NOT_UNIQUE",
        "ecuc.arxml",
        "/Configuration/PduR/Routes/RxValueSource",
    );
    // Rejects: ECUC_PduR_00291
    let mut sources = single_component_sources();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "TxValueDestination"),
        |text| text.replacen("/Pdus/TxValuePdu<", "/Pdus/RxValuePdu<", 1),
    );
    rejected_at(
        &sources,
        "PDU_ROUTE_NOT_UNIQUE",
        "ecuc.arxml",
        "/Configuration/EcuC/Config/Pdus/RxValuePdu",
    );
}
