//! Input-layer, semantic and source-delivery expectations of the historical
//! single-application chain, re-expressed as mutations of the mainline
//! single-component project (`single_component_sources`). Each expected
//! file/object is the mainline object equivalent to the historical negative
//! case; where the mainline reports a different code for the same defect the
//! reason is stated at the vector.
use super::*;

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

pub(super) fn network_rejections_locate_extract_objects() {
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

pub(super) fn component_rejections_locate_application_objects() {
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

pub(super) fn configuration_rejections_keep_identifier_width_and_order() {
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
    let mut widest = single_component_sources();
    parameter(&mut widest, "TxValue", "CanIfTxPduId", "255");
    build(&widest).unwrap_or_else(|issues| panic!("{issues:?}"));

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

pub(super) fn omitted_event_masks_and_unrelated_sources_are_planned() {
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

pub(super) fn input_sources_reject_unportable_aliases_and_missing_archives() {
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
    // The official entrance refuses unreadable pinned archives before any
    // profile selection; no input byte is consumed or changed.
    let scratch = Scratch::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let missing = autosar_config_core::integration::PlanDependencies {
        xsd_archive: scratch.0.join("missing-xsd.zip"),
        mod_archive: scratch.0.join("missing-mod.zip"),
    };
    let issues = autosar_config_core::integration::build_plan(
        &sources,
        &missing,
        &RuntimeCatalog::from_repository(root).unwrap(),
    )
    .err()
    .unwrap();
    assert_eq!(issues[0].code, "DEPENDENCY_MISSING", "{issues:?}");
    assert_eq!(text(&sources, "ingress.arxml").as_bytes(), bytes.as_slice());
}

pub(super) fn contract_names_reject_c_keywords_and_reserved_identifiers() {
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

pub(super) fn source_preparation_is_target_complete_without_native_tools() {
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
            .join("tests/fixtures/multi-application/Ingress.c"),
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
            "runtime/multi/src/Can.c",
            "runtime/multi/src/Com.c",
            "runtime/multi/src/Dcm.c",
        ] {
            assert!(selected.contains(&asset), "{asset}: {selected:?}");
        }
        let legacy: Vec<_> = first
            .files()
            .iter()
            .filter_map(|file| file.source.map(|asset| asset.relative_path))
            .filter(|path| path.starts_with("runtime/src/") || path.starts_with("runtime/include/"))
            .collect();
        assert!(legacy.is_empty(), "{legacy:?}");
        let delivered = |path: &str| {
            first
                .files()
                .iter()
                .find(|file| file.path == path)
                .unwrap_or_else(|| panic!("{path} is not delivered"))
        };
        // Recorded original sources are exactly the delivered bytes.
        let integration: serde_json::Value =
            serde_json::from_slice(&delivered("integration.json").bytes).unwrap();
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
            serde_json::from_slice(&delivered("contract.json").bytes).unwrap();
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
            serde_json::json!(["runtime/multi/src/Dcm.c"])
        );
        assert_eq!(
            originals["runtime/multi/src/Dcm.c"], "src/Dcm.c",
            "the consumer must be the delivered Dcm implementation"
        );
        assert!(has("os/src/Os.c") && has("kernel/tasks.c") && has("src/Rte.c"));
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
        assert_eq!(metadata["format"], "autosar-build-target-v1");
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

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
pub(super) fn generated_contract_headers_refuse_mismatched_consumers() {
    let plan = build(&single_component_sources()).unwrap();
    let scratch = Scratch::new();
    let directory = scratch.0.join("consumer");
    std::fs::create_dir_all(directory.join("include")).unwrap();
    for (file, bytes) in plan.component_contract_files().unwrap().files() {
        if file.ends_with(".h") {
            std::fs::write(directory.join(file), bytes).unwrap();
        }
    }
    let compiler = tooling::execution_settings().compiler;
    let compile = |name: &str| {
        let mut command = std::process::Command::new(&compiler);
        command.current_dir(&directory).args([
            "-std=c99",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
            "-Iinclude",
            "-c",
            name,
            "-o",
            "consumer.o",
        ]);
        tooling::run_public_command(
            &mut command,
            &directory,
            "contract-consumer",
            std::time::Duration::from_secs(60),
        )
    };
    // The matching consumer compiles, so each refusal below is the mismatch.
    std::fs::write(
        directory.join("matching.c"),
        "#include \"Rte_Ingress.h\"\n#include \"Rte_DcmService.h\"\nint test(void);\nint test(void) { uint32 data=0U; uint8 bytes[4]={0U,0U,0U,0U}; return (Rte_Read_RxValue_Value(&data)==E_OK && Rte_Write_TxValue_Value(data)==E_OK && Rte_Call_DataServices_ApplicationValue_ReadData(bytes)==E_OK) ? 0 : 1; }\n",
    )
    .unwrap();
    let output = compile("matching.c");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for (name, source, expected) in [
        (
            "private-valid.c",
            "#include \"Rte_Ingress.h\"\nvoid test(void);\nvoid test(void) { uint32 data=0U; uint8 valid=0U; (void)Rte_Read_RxValue_Value(&data, &valid); }\n",
            "too many arguments",
        ),
        (
            "invalidate.c",
            "#include \"Rte_Ingress.h\"\nvoid test(void);\nvoid test(void) { (void)Rte_Invalidate_TxValue_Value(); }\n",
            "implicit declaration",
        ),
        (
            "private-service.c",
            "#include \"Rte_DcmService.h\"\nvoid test(void);\nvoid test(void) { uint8 data[4]={0U}; uint8 nrc=0U; (void)Rte_Call_DataServices_ApplicationValue_ReadData(0U, data, &nrc); }\n",
            "too many arguments",
        ),
        (
            "wrong-write.c",
            "#include \"Rte_Ingress.h\"\nvoid test(void);\nvoid test(void) { uint32 data=0U; (void)Rte_Write_TxValue_Value(&data); }\n",
            "pointer",
        ),
        (
            "wrong-operation.c",
            "#include \"Rte_DcmService.h\"\nvoid test(void);\nvoid test(void) { uint8 data[4]={0U}; (void)Rte_Call_DataServices_ApplicationValue_WriteData(data); }\n",
            "implicit declaration",
        ),
        (
            "wrong-callback.c",
            "#include \"Rte_Ingress.h\"\nStd_ReturnType (*callback)(uint32 *) = &Ingress_ReadData;\n",
            "incompatible pointer",
        ),
    ] {
        std::fs::write(directory.join(name), source).unwrap();
        let output = compile(name);
        assert!(!output.status.success(), "{name} unexpectedly compiled");
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(diagnostic.contains(expected), "{name}: {diagnostic}");
    }
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
pub(super) fn production_preflight_and_binary_exclude_private_test_code() {
    use autosar_config_core::prepared::PreflightStatus;
    use autosar_config_core::{ApplicationSource, prepare_ecu_project_with_applications};

    let scratch = Scratch::new();
    let (project, files) = single_component_project(&scratch.0, tooling::native_target());
    let settings = tooling::execution_settings();
    let owner = tooling::execution_owner();

    // Preflight is bound to the prepared fingerprint; a missing compiler
    // fails it and the caller has nothing installed or changed.
    let plan = build(&single_component_sources()).unwrap();
    let prepared = prepare_ecu_project_with_applications(
        &plan,
        tooling::native_target(),
        &[ApplicationSource {
            component_instance: "/Application/Pipeline/IngressInstance".into(),
            path: scratch.0.join("Ingress.c"),
        }],
    )
    .unwrap();
    let application = std::fs::read(scratch.0.join("Ingress.c")).unwrap();
    let passed = prepared.native_preflight(&settings, &owner);
    assert_eq!(passed.status, PreflightStatus::Passed, "{passed:?}");
    assert_eq!(passed.fingerprint, prepared.fingerprint());
    let missing = autosar_config_core::target::ExecutionSettings::new(
        scratch.0.join(if cfg!(windows) {
            "missing-gcc.exe"
        } else {
            "missing-gcc"
        }),
        settings.objdump.clone(),
        settings.git.clone(),
        settings.python.clone(),
    )
    .unwrap();
    let failed = prepared.native_preflight(&missing, &owner);
    assert_eq!(failed.status, PreflightStatus::Failed, "{failed:?}");
    assert_eq!(failed.fingerprint, prepared.fingerprint());
    assert_eq!(
        std::fs::read(scratch.0.join("Ingress.c")).unwrap(),
        application
    );

    let output = scratch.0.join("production");
    let result = tooling::ecu_build_command(&project, &output, "host-batch", None)
        .current_dir(&scratch.0)
        .env_remove("PYTHONPATH")
        .env_remove("PYTHONHOME")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let binary = tooling::native_binary(&output, "ecu_host_batch");
    let symbols = tooling::symbol_sections(&settings.objdump, &binary);
    assert!(
        symbols
            .iter()
            .any(|(symbol, _)| symbol == "Ingress_Periodic")
    );
    for (symbol, _) in &symbols {
        assert!(
            ![
                "Os_Test",
                "Os_StackTest",
                "Os_TimeTest",
                "Os_TargetTest",
                "Os_ArtiTest",
                "Ecu_TargetTest",
                "vTaskOsTest",
            ]
            .iter()
            .any(|prefix| symbol.starts_with(prefix)),
            "Private test symbol linked into production: {symbol}"
        );
    }
    // A production build never accepts a caller control source.
    let mixed = scratch.0.join("refused-production-control");
    let result = tooling::ecu_build_command(
        &project,
        &mixed,
        "host-batch",
        Some(&scratch.0.join("Ingress.c")),
    )
    .current_dir(&scratch.0)
    .env_remove("PYTHONPATH")
    .env_remove("PYTHONHOME")
    .output()
    .unwrap();
    assert!(!result.status.success());
    assert!(!mixed.exists(), "Rejected mixed build installed an output");
    for (name, bytes) in files {
        assert_eq!(
            std::fs::read(project.join(&name)).unwrap(),
            bytes,
            "{name}: offline builds must not mutate the sealed package"
        );
    }
}
