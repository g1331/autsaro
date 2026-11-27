//! R24-11 reference projects used by the desktop and independent package consumer.
#![allow(dead_code)]

mod support;

use autosar_config_core::{
    Workspace,
    arxml::*,
    integration::{InputSource, IntegrationEdit},
    project_model::{TypedValue, ValueKind},
};
use roxmltree::{Document, Node};
use std::{collections::BTreeMap, fs, path::Path};
use support::{
    mainline::{live_pipeline_workspace, named, parameter, subtree, xml_edit},
    multichannel::gateway_ring_sources,
    workspace::Scratch,
};

const COM: &str = "/AUTOSAR/EcucDefs/Com/ComConfig/ComSignal";
const CAN: &str = "/AUTOSAR/EcucDefs/Can/CanConfigSet/CanController/CanControllerBaudrateConfig/CanControllerFdBaudrateConfig";

fn child<'a>(node: Node<'a, '_>, tag: &str) -> Option<&'a str> {
    node.children().find(|item| item.has_tag_name(tag))?.text()
}

fn short(node: Node<'_, '_>, prefix: &str) -> bool {
    child(node, "SHORT-NAME").is_some_and(|name| name.starts_with(prefix))
}

fn variant(option: &str) -> BTreeMap<String, std::collections::BTreeSet<String>> {
    [("calibration".into(), [option.into()].into())].into()
}

fn set(path: &str, definition: &str, field: &str, kind: ValueKind, value: &str) -> LayerOperation {
    LayerOperation {
        target: LayerTarget {
            object: LayerObject {
                release: "R24-11".into(),
                definition_type: definition.into(),
                path: path.into(),
                instance_context: vec![],
            },
            field: Some(format!("{definition}/{field}")),
        },
        action: LayerAction::Set {
            values: vec![TypedValue {
                kind,
                lexeme: value.into(),
            }],
            default_origin: None,
        },
    }
}

fn remove_periodic_transmit(sources: &mut [InputSource], port: &str) {
    let source = format!("/Application/Ingress/{port}");
    for file in ["ingress.arxml", "extract.arxml", "ecuc.arxml"] {
        let input = sources
            .iter_mut()
            .find(|input| input.logical_path() == file)
            .unwrap();
        let mut xml = std::str::from_utf8(input.bytes()).unwrap().to_owned();
        let document = Document::parse(&xml).unwrap();
        let selected = |node: Node<'_, '_>| {
            short(node, port)
                || named(node, "VARIABLE-ACCESS", &format!("P{port}"))
                || (node.has_tag_name("SENDER-RECEIVER-TO-SIGNAL-MAPPING")
                    && node.descendants().any(|item| {
                        item.has_tag_name("CONTEXT-PORT-REF") && item.text() == Some(&source)
                    }))
        };
        let ranges = document
            .descendants()
            .filter(|node| selected(*node) && !node.ancestors().skip(1).any(selected))
            .map(|node| node.range())
            .collect::<Vec<_>>();
        for range in ranges.into_iter().rev() {
            xml.replace_range(range, "");
        }
        *input = InputSource::new(file, xml.into_bytes()).unwrap();
    }
}

fn add_signal_types(sources: &mut [InputSource]) {
    let base = subtree(sources, "types.arxml", "SW-BASE-TYPE", "Base_uint32");
    let implementation = subtree(sources, "types.arxml", "IMPLEMENTATION-DATA-TYPE", "uint32");
    let app = subtree(
        sources,
        "types.arxml",
        "APPLICATION-PRIMITIVE-DATA-TYPE",
        "ApplicationUint32",
    );
    let interface = subtree(
        sources,
        "types.arxml",
        "SENDER-RECEIVER-INTERFACE",
        "ValueInterface",
    );
    let sint = |xml: &str| xml.replace("uint32", "sint16").replace("Uint32", "Sint16");
    let byte = |xml: &str| xml.replace("uint32", "uint8").replace("Uint32", "Uint8");
    xml_edit(
        sources,
        "types.arxml",
        |node| named(node, "SENDER-RECEIVER-INTERFACE", "ValueInterface"),
        |xml| {
            format!(
                "{xml}{}{}{}{}{}{}",
                sint(&base)
                    .replace(">32<", ">16<")
                    .replace(">NONE<", ">2C<"),
                sint(&implementation),
                sint(&app),
                byte(&app),
                sint(&interface).replace("ValueInterface", "SignedValueInterface"),
                byte(&interface).replace("ValueInterface", "ByteValueInterface"),
            )
        },
    );
    xml_edit(
        sources,
        "types.arxml",
        |node| {
            node.has_tag_name("DATA-TYPE-MAP")
                && node
                    .ancestors()
                    .any(|item| named(item, "DATA-TYPE-MAPPING-SET", "ApplicationTypes"))
        },
        |xml| format!("{xml}{}{}", sint(xml), byte(xml)),
    );
}

fn set_signal_layout(sources: &mut [InputSource], port: &str, signed: bool, big: bool) {
    let (interface, width, native) = if signed {
        ("SignedValueInterface", "16", "SINT16")
    } else {
        ("ByteValueInterface", "8", "UINT8")
    };
    for (file, tag) in [
        (
            "ingress.arxml",
            if port.starts_with("Rx") {
                "R-PORT-PROTOTYPE"
            } else {
                "P-PORT-PROTOTYPE"
            },
        ),
        ("ingress.arxml", "VARIABLE-ACCESS"),
        ("extract.arxml", "SENDER-RECEIVER-TO-SIGNAL-MAPPING"),
    ] {
        let port_path = format!("/Application/Ingress/{port}");
        xml_edit(
            sources,
            file,
            |node| {
                named(node, tag, port)
                    || (node.has_tag_name(tag)
                        && node.descendants().any(|item| {
                            matches!(
                                item.tag_name().name(),
                                "CONTEXT-PORT-REF" | "PORT-PROTOTYPE-REF"
                            ) && item.text() == Some(&port_path)
                        }))
            },
            |xml| xml.replace("/Types/ValueInterface", &format!("/Types/{interface}")),
        );
    }
    xml_edit(
        sources,
        "extract.arxml",
        |node| named(node, "I-SIGNAL", &format!("{port}Signal")),
        |xml| {
            xml.replace(">32<", &format!(">{width}<")).replace(
                "Base_uint32",
                if signed { "Base_sint16" } else { "Base_uint8" },
            )
        },
    );
    let start = if big { "7" } else { "0" };
    if big {
        xml_edit(
            sources,
            "extract.arxml",
            |node| named(node, "I-SIGNAL-TO-I-PDU-MAPPING", &format!("{port}Map")),
            |xml| {
                xml.replace("MOST-SIGNIFICANT-BYTE-LAST", "MOST-SIGNIFICANT-BYTE-FIRST")
                    .replace("<START-POSITION>0<", "<START-POSITION>7<")
            },
        );
    }
    for (name, value) in [
        ("ComBitPosition", start),
        ("ComBitSize", width),
        ("ComSignalType", native),
        (
            "ComSignalEndianness",
            if big { "BIG_ENDIAN" } else { "LITTLE_ENDIAN" },
        ),
    ] {
        parameter(sources, port, name, value);
    }
}

fn fd_transport(sources: &mut [InputSource]) {
    for (name, receive) in [("DiagRequest", true), ("DiagResponse", false)] {
        parameter(sources, &format!("{name}Pdu"), "PduLength", "12");
        xml_edit(
            sources,
            "ecuc.arxml",
            |node| named(node, "ECUC-CONTAINER-VALUE", &format!("{name}Pdu")),
            |xml| {
                xml.replace("</PARAMETER-VALUES>", &format!(r#"</PARAMETER-VALUES><REFERENCE-VALUES>
              <ECUC-REFERENCE-VALUE>
                <DEFINITION-REF DEST="ECUC-FOREIGN-REFERENCE-DEF">/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu/SysTPduToFrameTriggeringRef</DEFINITION-REF>
                <VALUE-REF DEST="CAN-FRAME-TRIGGERING">/Extract/ReferenceCan/Channel/{name}Trigger</VALUE-REF>
              </ECUC-REFERENCE-VALUE>
            </REFERENCE-VALUES>"#))
            },
        );
        parameter(
            sources,
            name,
            if receive {
                "CanIfRxPduCanIdType"
            } else {
                "CanIfTxPduCanIdType"
            },
            "STANDARD_FD_CAN",
        );
        if receive {
            parameter(sources, name, "CanIfRxPduDataLengthCheck", "false");
        }
        for (tag, ending) in [
            ("CAN-FRAME-TRIGGERING", "Trigger"),
            ("CAN-FRAME", "Frame"),
            ("N-PDU", "Pdu"),
        ] {
            xml_edit(
                sources,
                "extract.arxml",
                |node| named(node, tag, &format!("{name}{ending}")),
                |xml| {
                    xml.replace("<CAN-FD-FRAME-SUPPORT>false", "<CAN-FD-FRAME-SUPPORT>true")
                        .replace("<FRAME-LENGTH>8<", "<FRAME-LENGTH>12<")
                        .replace("<LENGTH>8<", "<LENGTH>12<")
                },
            );
        }
    }
    parameter(sources, "General", "CanTpPaddingByte", "204");
}

fn separate_receive(sources: &mut [InputSource], suffix: &str, pdu: &str, object_id: usize) {
    let original = format!("Receive{suffix}");
    let dedicated = format!("Receive{pdu}");
    for definition in ["CanHardwareObject", "CanIfHrhCfg"] {
        xml_edit(
            sources,
            "ecuc.arxml",
            |node| {
                named(node, "ECUC-CONTAINER-VALUE", &original)
                    && child(node, "DEFINITION-REF").is_some_and(|path| path.ends_with(definition))
            },
            |xml| {
                let copy = xml
                    .replace(
                        &format!("<SHORT-NAME>{original}<"),
                        &format!("<SHORT-NAME>{dedicated}<"),
                    )
                    .replace(
                        &format!("/Can/Config/{original}<"),
                        &format!("/Can/Config/{dedicated}<"),
                    );
                format!("{xml}{copy}")
            },
        );
    }
    parameter(sources, &dedicated, "CanObjectId", &object_id.to_string());
    xml_edit(
        sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", pdu)
                && child(node, "DEFINITION-REF")
                    .is_some_and(|path| path.ends_with("/CanIfRxPduCfg"))
        },
        |xml| {
            xml.replace(
                &format!("/Handles/{original}<"),
                &format!("/Handles/{dedicated}<"),
            )
        },
    );
}

fn separate_diagnostic_transmit(sources: &mut [InputSource], object_id: usize) {
    for (definition, name) in [
        ("CanHardwareObject", "Transmit"),
        ("CanIfHthCfg", "Transmit"),
        ("CanIfBufferCfg", "TransmitBuffer"),
    ] {
        xml_edit(
            sources,
            "ecuc.arxml",
            |node| {
                named(node, "ECUC-CONTAINER-VALUE", name)
                    && child(node, "DEFINITION-REF").is_some_and(|path| path.ends_with(definition))
            },
            |xml| {
                let copy = xml
                    .replace("<SHORT-NAME>Transmit", "<SHORT-NAME>TransmitDiagnostic")
                    .replace("/Can/Config/Transmit<", "/Can/Config/TransmitDiagnostic<")
                    .replace("/Handles/Transmit<", "/Handles/TransmitDiagnostic<");
                format!("{xml}{copy}")
            },
        );
    }
    parameter(
        sources,
        "TransmitDiagnostic",
        "CanObjectId",
        &object_id.to_string(),
    );
    xml_edit(
        sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "DiagResponse")
                && child(node, "DEFINITION-REF")
                    .is_some_and(|path| path.ends_with("/CanIfTxPduCfg"))
        },
        |xml| xml.replace("/TransmitBuffer<", "/TransmitDiagnosticBuffer<"),
    );
}

fn sources(count: usize) -> Vec<InputSource> {
    let mut sources = gateway_ring_sources(count, if count == 2 { 2 } else { 3 });
    if count == 4 {
        for port in ["TxValue1", "TxValue2", "TxValue3"] {
            remove_periodic_transmit(&mut sources, port);
        }
        for (gateway, handle) in [
            ("TxGateway1", "2"),
            ("TxGateway2", "3"),
            ("TxGateway3", "4"),
        ] {
            parameter(&mut sources, gateway, "CanIfTxPduId", handle);
        }
        parameter(
            &mut sources,
            "TxValue",
            "CanIfTxPduCanIdType",
            "EXTENDED_FD_CAN",
        );
        xml_edit(
            &mut sources,
            "extract.arxml",
            |node| named(node, "CAN-FRAME-TRIGGERING", "TxValueTrigger"),
            |xml| {
                xml.replace("STANDARD", "EXTENDED")
                    .replace(">false<", ">true<")
            },
        );
    }
    add_signal_types(&mut sources);
    set_signal_layout(&mut sources, "RxValue", true, false);
    set_signal_layout(&mut sources, "TxValue", true, true);
    set_signal_layout(&mut sources, "RxValue1", false, false);
    if count == 2 {
        set_signal_layout(&mut sources, "TxValue1", false, true);
    }
    fd_transport(&mut sources);
    let dedicated: &[(&str, &str)] = if count == 2 {
        &[("", "DiagRequest"), ("", "RxGateway0"), ("1", "RxGateway1")]
    } else {
        &[
            ("", "DiagRequest"),
            ("", "RxGateway0"),
            ("1", "RxGateway1"),
            ("2", "RxGateway2"),
            ("", "RxExtended"),
            ("1", "RxExtended1"),
            ("2", "RxExtended2"),
            ("3", "RxExtended3"),
        ]
    };
    for (slot, (suffix, pdu)) in dedicated.iter().enumerate() {
        separate_receive(&mut sources, suffix, pdu, 2 * count + slot);
    }
    if count == 4 {
        separate_diagnostic_transmit(&mut sources, 2 * count + dedicated.len());
    }
    sources
}

fn project(root: &Path, count: usize, init: &str) -> Workspace {
    let sources = sources(count);
    let workspace = live_pipeline_workspace(root, &sources);
    let mut manifest = workspace.project_manifest().unwrap().clone();
    let (base, overlay): (Vec<_>, Vec<_>) = sources
        .iter()
        .partition(|source| matches!(source.logical_path(), "composition.arxml" | "types.arxml"));
    let reference = BaseReference {
        base_id: "reference-communication-platform".into(),
        revision: "1".into(),
        source_sha256: base_source_digest(
            base.iter()
                .map(|source| (source.logical_path(), source.bytes())),
        ),
    };
    manifest.layers = Some(ProjectLayers {
        base_reference: reference.clone(),
        base: ProjectBase {
            reference,
            release: "R24-11".into(),
            rules: autosar_config_core::rules::rule_set_identity().unwrap(),
            extensions: vec![],
            members: base
                .iter()
                .map(|source| source.logical_path().into())
                .collect(),
            migrations: vec![],
        },
        project: ProjectLayer {
            members: overlay
                .iter()
                .map(|source| source.logical_path().into())
                .collect(),
            operations: vec![set(
                "/Configuration/Can/Config/Controller/Baudrate/Fd",
                CAN,
                "CanControllerTxBitRateSwitch",
                ValueKind::Boolean,
                if count == 2 { "true" } else { "false" },
            )],
        },
        groups: [(
            "calibration".into(),
            VariantGroup {
                exactly_one: true,
                options: [
                    ("platform".into(), ProjectLayer::default()),
                    (
                        "commissioned".into(),
                        ProjectLayer {
                            members: vec![],
                            operations: vec![set(
                                "/Configuration/Com/Config/TxValue",
                                COM,
                                "ComSignalInitValue",
                                ValueKind::String,
                                init,
                            )],
                        },
                    ),
                    (
                        "invalid".into(),
                        ProjectLayer {
                            members: vec![],
                            operations: vec![set(
                                "/Configuration/Com/Config/TxValue",
                                COM,
                                if count == 2 {
                                    "ComBitPosition"
                                } else {
                                    "ComBitSize"
                                },
                                ValueKind::Integer,
                                if count == 2 { "25" } else { "17" },
                            )],
                        },
                    ),
                ]
                .into(),
            },
        )]
        .into(),
        selection: variant("platform"),
        conflicts: vec![],
    });
    let file = root.join("workbench-project.json");
    fs::write(&file, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    let mut workspace = open(root);
    let report = workspace.inspect_integration();
    assert!(
        report.description.is_some(),
        "{count}: {:#?}",
        report.diagnostics
    );
    let preview = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&preview)
        .unwrap();
    assert_eq!(preview.files.len(), 1);
    let source = include_str!("fixtures/r11-reference/Ingress.c");
    let features = if count == 2 {
        "#define REFERENCE_TWO_BUSES\n"
    } else {
        ""
    };
    fs::write(
        root.join(&preview.files[0].path),
        format!("{features}{source}"),
    )
    .unwrap();
    open(root)
}

fn open(root: &Path) -> Workspace {
    Workspace::open_project_manifest(&root.join("workbench-project.json"), &root.join("cache"))
        .unwrap()
}

#[test]
fn reference_projects() {
    let scratch = Scratch::new();
    let staged = std::env::var_os("AUTOSAR_R11_PROJECTS").map(std::path::PathBuf::from);
    let parent = staged.as_deref().unwrap_or(&scratch.0);
    let two = project(&parent.join("two-bus"), 2, "17");
    let four = project(&parent.join("four-bus"), 4, "-33");
    let diff = two.compare_project(&four).unwrap();
    assert!(
        diff.semantics
            .iter()
            .any(|row| row.target.object.path.ends_with("/ReferenceCan3")
                && row.changes.contains(&SemanticChange::Added))
    );
    assert!(diff.semantics.iter().any(|row| {
        row.target.object.path.ends_with("/Controller/Baudrate/Fd")
            && row
                .target
                .field
                .as_deref()
                .is_some_and(|field| field.ends_with("CanControllerTxBitRateSwitch"))
            && row.changes.contains(&SemanticChange::Value)
    }));

    // The desktop consumes the original project inputs; the core round trip uses a copy.
    for (name, init) in [("two-bus", 17), ("four-bus", -33)] {
        let root = if staged.is_some() {
            scratch.0.join(name)
        } else {
            parent.join(name)
        };
        let mut workspace = if staged.is_some() {
            project(
                &root,
                if name == "two-bus" { 2 } else { 4 },
                &init.to_string(),
            )
        } else {
            open(&root)
        };
        let invalid = workspace
            .preview_variant_selection(variant("invalid"))
            .unwrap_err();
        assert!(
            invalid.iter().any(|issue| issue.code == "TYPE_CONFLICT"
                && issue.object.as_deref() == Some("/Configuration/Com/Config/TxValue")),
            "{invalid:#?}"
        );
        let changes = workspace
            .preview_variant_selection(variant("commissioned"))
            .unwrap();
        assert_eq!(changes.len(), 1);
        assert!(
            changes[0]
                .field
                .as_deref()
                .unwrap()
                .ends_with("/ComSignalInitValue")
        );
        workspace
            .edit_integration(IntegrationEdit {
                variant_selection: Some(variant("commissioned")),
                ..IntegrationEdit::default()
            })
            .unwrap();
        workspace.save().unwrap();
        let reopened = open(&root).saved_integration_plan().unwrap();
        let reopened = reopened.description();
        let topology = reopened.communication_runtime.as_ref().unwrap();
        let expected = if name == "two-bus" { (2, 7) } else { (4, 17) };
        assert_eq!((topology.channels.len(), topology.hardware.len()), expected);
        for (native, width) in [("uint8", 8), ("sint16", 16), ("uint32", 32)] {
            assert!(
                reopened
                    .signals
                    .iter()
                    .any(|signal| signal.native_type == native && signal.bit_length == width)
            );
        }
        let signal = reopened
            .signals
            .iter()
            .find(|item| item.port.ends_with("/TxValue"))
            .unwrap();
        assert_eq!(signal.initial_value, init);
    }
}
