use super::{Scratch, archive, create_pair, tooling};
use autosar_config_core::{DiagnosticSettings, Direction, Workspace, generator};
use std::fs;
use std::io::Read;
#[cfg(windows)]
use std::io::Write;
use std::path::Path;
#[cfg(windows)]
use std::process::{Command, Stdio};

fn message_has_key(text: &autosar_config_core::LocalizedText, key: &str) -> bool {
    match text {
        autosar_config_core::LocalizedText::Message(message) => message.key == key,
        autosar_config_core::LocalizedText::Messages(messages) => {
            messages.iter().any(|text| message_has_key(text, key))
        }
        autosar_config_core::LocalizedText::Raw(_) => false,
    }
}

fn message_has_parameter(
    text: &autosar_config_core::LocalizedText,
    name: &str,
    expected: &str,
) -> bool {
    match text {
        autosar_config_core::LocalizedText::Message(message) => {
            message.params.get(name).and_then(serde_json::Value::as_str) == Some(expected)
        }
        autosar_config_core::LocalizedText::Messages(messages) => messages
            .iter()
            .any(|text| message_has_parameter(text, name, expected)),
        autosar_config_core::LocalizedText::Raw(_) => false,
    }
}

#[cfg(windows)]
pub(super) fn global_ecuc_pdu_binding_roundtrips_and_rejects_wrong_com_reference_type() {
    let temp = Scratch::new();
    let source = temp.0.join("Closure/Closure.arxml");
    let mut project =
        Workspace::create_legacy(source.parent().unwrap(), "Closure", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 1, Direction::Tx, Some(10), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    project
        .add_signal(frame.clone(), "Value".into(), 0, 8, 7)
        .unwrap();
    project.save().unwrap();
    let saved = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&saved).unwrap();
    let global = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-CONTAINER-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("SHORT-NAME") && child.text() == Some("Pdu_Live")
                })
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text()
                            == Some("/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu")
                })
        })
        .unwrap();
    assert!(
        global
            .descendants()
            .any(|node| node.has_tag_name("VALUE") && node.text() == Some("1"))
    );
    assert!(
        !global
            .descendants()
            .any(|node| node.has_tag_name("DEFINITION-REF")
                && node
                    .text()
                    .is_some_and(|value| value.ends_with("/DynamicLength"))),
        "System Template constr_3448 excludes DynamicLength for system I-PDU bindings"
    );
    assert!(global.descendants().any(|node| node.has_tag_name("SDG")
        && node.attribute("GID") == Some("AutosarWorkbenchGlobalPduV1")
        && node.descendants().any(|child| child.has_tag_name("SD")
            && child.attribute("GID") == Some("SystemPduRef")
            && child.text() == Some(frame.as_str()))));
    let com_ref = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-REFERENCE-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|value| value.ends_with("/ComPduIdRef"))
                })
        })
        .unwrap();
    let reference = com_ref
        .children()
        .find(|node| node.has_tag_name("VALUE-REF"))
        .unwrap();
    assert_eq!(reference.attribute("DEST"), Some("ECUC-CONTAINER-VALUE"));
    assert_eq!(
        reference.text(),
        Some("/Closure/EcuCCfg/EcucConfigSet/Pdus/Pdu_Live")
    );
    let mut reopened = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    assert!(reopened.validate().unwrap().issues.is_empty());
    let generated = temp.0.join("Generated");
    generator::generate(&mut reopened, &generated, tooling::native_target()).unwrap();
    let binary = tooling::build_host(&generated).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin.take().unwrap().write_all(b"T 10\n").unwrap();
    assert_eq!(
        String::from_utf8(ecu.wait_with_output().unwrap().stdout)
            .unwrap()
            .trim(),
        "X 801 1 07"
    );

    let wrong = saved.replacen(
        &saved[reference.range()],
        &saved[reference.range()].replace("ECUC-CONTAINER-VALUE", "I-SIGNAL-I-PDU"),
        1,
    );
    fs::write(&source, &wrong).unwrap();
    let mut unsupported = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    let issue = unsupported
        .validate()
        .unwrap()
        .issues
        .into_iter()
        .find(|issue| issue.code == "PDU_UNSUPPORTED")
        .expect("wrong ECUC destination must block use");
    assert!(
        issue
            .file
            .as_deref()
            .is_some_and(|file| file.contains("Closure.arxml"))
    );
    assert!(
        issue
            .path
            .as_deref()
            .is_some_and(|path| path.contains("Pdu_Live"))
    );
    assert!(message_has_parameter(
        &unsupported.save().unwrap_err(),
        "code",
        "PDU_UNSUPPORTED"
    ));
    assert!(message_has_parameter(
        &generator::generate(
            &mut unsupported,
            &temp.0.join("Unsafe"),
            tooling::native_target()
        )
        .unwrap_err(),
        "code",
        "PDU_UNSUPPORTED"
    ));
    assert_eq!(fs::read_to_string(source).unwrap(), wrong);
}

pub(super) fn host_can_ecuc_closes_required_mod_fields_and_rejects_broken_links() {
    use roxmltree::Node;
    use std::collections::BTreeMap;

    fn field(node: Node<'_, '_>, name: &str) -> String {
        node.children()
            .find(|child| child.is_element() && child.tag_name().name() == name)
            .and_then(|child| child.text())
            .unwrap_or("")
            .to_owned()
    }
    fn path(node: Node<'_, '_>) -> String {
        let mut names: Vec<_> = node
            .ancestors()
            .filter_map(|parent| {
                let name = field(parent, "SHORT-NAME");
                if name.is_empty() { None } else { Some(name) }
            })
            .collect();
        names.reverse();
        format!("/{}", names.join("/"))
    }
    fn named<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
        node.children()
            .find(|child| child.is_element() && child.tag_name().name() == name)
    }

    let temp = Scratch::new();
    let mut project =
        Workspace::create_legacy(&temp.0.join("CanClosure"), "CanClosure", archive()).unwrap();
    let tx = project
        .add_frame("Transmit".into(), 0x321, 4, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(tx, "LiveValue".into(), 0, 32, 7)
        .unwrap()
        .signals[0]
        .path
        .clone();
    let rx = project
        .add_frame("Receive".into(), 0x456, 1, Direction::Rx, None, Some(50))
        .unwrap()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Receive")
        .unwrap()
        .path;
    project
        .add_signal(rx, "ReceivedValue".into(), 0, 8, 0)
        .unwrap();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("CanClosure/CanClosure.arxml");
    let xml = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();

    let mod_zip = autosar_config_core::schema::reference_archive(
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap(),
        "AUTOSAR_MOD_ARCHIVE",
        autosar_config_core::schema::MOD_ZIP,
    );
    let mut mod_archive = zip::ZipArchive::new(
        fs::File::open(mod_zip).expect("R24-11 ECUC MOD archive is required for this test"),
    )
    .unwrap();
    let mut mod_xml = String::new();
    mod_archive
        .by_name("AUTOSAR_CP_MOD_ECUConfigurationParameters.arxml")
        .unwrap()
        .read_to_string(&mut mod_xml)
        .unwrap();
    let mod_doc = roxmltree::Document::parse(&mod_xml).unwrap();
    let definitions: BTreeMap<_, _> = mod_doc
        .descendants()
        .filter(|node| {
            node.is_element()
                && (node.tag_name().name().ends_with("-PARAM-DEF")
                    || node.tag_name().name().ends_with("-REFERENCE-DEF")
                    || node.tag_name().name().ends_with("-CONTAINER-DEF")
                    || node.tag_name().name() == "ECUC-MODULE-DEF")
        })
        .map(|node| (path(node), node))
        .collect();
    let containers: BTreeMap<_, _> = doc
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "ECUC-CONTAINER-VALUE")
        .map(|node| (path(node), node))
        .collect();
    let modules: Vec<_> = doc
        .descendants()
        .filter(|node| {
            node.is_element()
                && node.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
                && matches!(
                    field(*node, "SHORT-NAME").as_str(),
                    "McuCfg" | "CanCfg" | "CanIfCfg"
                )
        })
        .collect();
    assert_eq!(
        modules.len(),
        3,
        "fixed host CAN needs Mcu, Can and CanIf configuration"
    );
    for module in modules {
        for value in module.descendants().filter(|node| {
            node.is_element()
                && matches!(
                    node.tag_name().name(),
                    "ECUC-MODULE-CONFIGURATION-VALUES" | "ECUC-CONTAINER-VALUE"
                )
        }) {
            let definition = field(value, "DEFINITION-REF");
            let spec = definitions
                .get(&definition)
                .unwrap_or_else(|| panic!("missing MOD definition {definition}"));
            for (spec_group, value_group) in [
                ("CONTAINERS", "CONTAINERS"),
                ("SUB-CONTAINERS", "SUB-CONTAINERS"),
                ("PARAMETERS", "PARAMETER-VALUES"),
                ("REFERENCES", "REFERENCE-VALUES"),
            ] {
                if let Some(group) = named(*spec, spec_group) {
                    for required in group.children().filter(|child| {
                        child.is_element() && field(*child, "LOWER-MULTIPLICITY") == "1"
                    }) {
                        let required_path = path(required);
                        let count = named(value, value_group)
                            .into_iter()
                            .flat_map(|group| group.children())
                            .filter(|child| {
                                child.is_element()
                                    && field(*child, "DEFINITION-REF") == required_path
                            })
                            .count();
                        assert_eq!(
                            count,
                            1,
                            "{} requires exactly one {}",
                            path(value),
                            required_path
                        );
                    }
                }
            }
        }
        for reference in module
            .descendants()
            .filter(|node| node.is_element() && node.tag_name().name() == "ECUC-REFERENCE-VALUE")
        {
            let definition = field(reference, "DEFINITION-REF");
            let spec = definitions
                .get(&definition)
                .unwrap_or_else(|| panic!("missing MOD reference {definition}"));
            let target = named(reference, "VALUE-REF").unwrap();
            assert_eq!(
                target.attribute("DEST"),
                Some("ECUC-CONTAINER-VALUE"),
                "{definition}"
            );
            let target_path = target.text().unwrap();
            let resolved = containers
                .get(target_path)
                .unwrap_or_else(|| panic!("unresolved {definition} -> {target_path}"));
            let target_def = field(*resolved, "DEFINITION-REF");
            let allowed = spec
                .descendants()
                .filter(|node| node.is_element() && node.tag_name().name() == "DESTINATION-REF")
                .any(|node| node.text() == Some(target_def.as_str()));
            assert!(allowed, "{definition} cannot point to {target_def}");
        }
        for parameter in module.descendants().filter(|node| {
            node.is_element()
                && matches!(
                    node.tag_name().name(),
                    "ECUC-NUMERICAL-PARAM-VALUE" | "ECUC-TEXTUAL-PARAM-VALUE"
                )
        }) {
            let definition = field(parameter, "DEFINITION-REF");
            let spec = definitions
                .get(&definition)
                .unwrap_or_else(|| panic!("missing MOD parameter {definition}"));
            let value = field(parameter, "VALUE");
            if let Some(literals) = named(*spec, "LITERALS") {
                assert!(
                    literals.children().any(
                        |literal| literal.is_element() && field(literal, "SHORT-NAME") == value
                    ),
                    "{definition} has unsupported value {value}"
                );
            }
            if spec.tag_name().name() == "ECUC-BOOLEAN-PARAM-DEF" {
                assert!(
                    matches!(value.as_str(), "true" | "false"),
                    "{definition} has invalid boolean {value}"
                );
            }
            if let Ok(number) = value.parse::<f64>() {
                if let Ok(minimum) = field(*spec, "MIN").parse::<f64>() {
                    assert!(number >= minimum, "{definition} is below the MOD minimum");
                }
                if let Ok(maximum) = field(*spec, "MAX").parse::<f64>() {
                    assert!(number <= maximum, "{definition} is above the MOD maximum");
                }
            }
        }
    }

    let mcu_module = doc
        .descendants()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
                && field(*node, "SHORT-NAME") == "McuCfg"
        })
        .unwrap();
    let split_source = temp.0.join("CanClosure/Clock.arxml");
    let split_first = format!(
        "{}{}",
        &xml[..mcu_module.range().start],
        &xml[mcu_module.range().end..]
    );
    let split_second = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>CanClosure</SHORT-NAME><ELEMENTS>{}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>",
        &xml[mcu_module.range()]
    );
    fs::write(&source, &split_first).unwrap();
    fs::write(&split_source, &split_second).unwrap();
    let mut split =
        Workspace::open_legacy(vec![source.clone(), split_source.clone()], archive()).unwrap();
    assert!(
        split.validate().unwrap().issues.is_empty(),
        "split Mcu clock must resolve across files"
    );
    generator::generate(
        &mut split,
        &temp.0.join("GeneratedSplit"),
        tooling::native_target(),
    )
    .unwrap();
    split.save().unwrap();
    assert_eq!(fs::read_to_string(&source).unwrap(), split_first);
    assert_eq!(fs::read_to_string(&split_source).unwrap(), split_second);
    fs::write(&source, &xml).unwrap();

    let private = containers
        .get("/CanClosure/CanIfCfg/CanIfPrivateCfg")
        .unwrap();
    let removed_private = format!(
        "{}{}",
        &xml[..private.range().start],
        &xml[private.range().end..]
    );
    let broken_clock = xml.replacen(
        "/CanClosure/McuCfg/McuModuleConfiguration/HostClockSetting/HostClock</VALUE-REF>",
        "/CanClosure/McuCfg/McuModuleConfiguration/HostClockSetting/MissingClock</VALUE-REF>",
        1,
    );
    let buffer_ref = doc
        .descendants()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "ECUC-REFERENCE-VALUE"
                && field(*node, "DEFINITION-REF").ends_with("/CanIfTxPduBufferRef")
        })
        .unwrap();
    let removed_buffer_ref = format!(
        "{}{}",
        &xml[..buffer_ref.range().start],
        &xml[buffer_ref.range().end..]
    );
    let wrong_hoh = xml.replacen(
        "/CanClosure/CanCfg/CanConfigSet/HostRxObject</VALUE-REF>",
        "/CanClosure/CanCfg/CanConfigSet/HostTxObject</VALUE-REF>",
        1,
    );
    for (name, modified) in [
        ("missing-private", removed_private),
        ("clock-ref", broken_clock),
        ("missing-buffer-ref", removed_buffer_ref),
        ("wrong-hoh", wrong_hoh),
    ] {
        assert_ne!(modified, xml, "{name} must change the fixture");
        fs::write(&source, &modified).unwrap();
        let mut imported = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
        assert!(
            imported
                .validate()
                .unwrap()
                .issues
                .iter()
                .any(|issue| issue.code == "PDU_UNSUPPORTED"),
            "{name}"
        );
        assert!(imported.view().files[0].readonly, "{name}");
        assert!(imported.save().is_err(), "{name}");
        assert!(
            generator::generate(
                &mut imported,
                &temp.0.join(format!("Unsafe{name}")),
                tooling::native_target()
            )
            .is_err(),
            "{name}"
        );
        assert_eq!(fs::read_to_string(&source).unwrap(), modified, "{name}");
    }
}

pub(super) fn missing_required_com_or_ecuc_root_is_read_only_and_cannot_generate() {
    let temp = Scratch::new();
    let source = temp.0.join("Closure/Closure.arxml");
    let mut project =
        Workspace::create_legacy(source.parent().unwrap(), "Closure", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 1, Direction::Tx, Some(10), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    project.add_signal(frame, "Value".into(), 0, 8, 7).unwrap();
    project.save().unwrap();
    let xml = fs::read_to_string(&source).unwrap();
    for name in ["ComGeneral", "Hardware"] {
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let required = doc
            .descendants()
            .find(|node| {
                node.has_tag_name("ECUC-CONTAINER-VALUE")
                    && node
                        .children()
                        .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some(name))
            })
            .unwrap();
        let modified = format!(
            "{}{}",
            &xml[..required.range().start],
            &xml[required.range().end..]
        );
        fs::write(&source, &modified).unwrap();
        let mut imported = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
        let issue = imported
            .validate()
            .unwrap()
            .issues
            .into_iter()
            .find(|issue| issue.code == "PDU_UNSUPPORTED")
            .unwrap_or_else(|| panic!("missing {name} must block generation"));
        assert!(
            issue
                .file
                .as_deref()
                .is_some_and(|file| file.contains("Closure.arxml"))
        );
        assert!(message_has_parameter(
            &imported.save().unwrap_err(),
            "code",
            "PDU_UNSUPPORTED"
        ));
        assert!(message_has_parameter(
            &generator::generate(
                &mut imported,
                &temp.0.join(format!("Unsafe{name}")),
                tooling::native_target()
            )
            .unwrap_err(),
            "code",
            "PDU_UNSUPPORTED"
        ));
        assert_eq!(fs::read_to_string(&source).unwrap(), modified);
    }
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let module = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-MODULE-CONFIGURATION-VALUES")
                && node.children().any(|child| {
                    child.has_tag_name("SHORT-NAME") && child.text() == Some("EcuCCfg")
                })
        })
        .unwrap();
    let insertion = module.range().end - "</CONTAINERS></ECUC-MODULE-CONFIGURATION-VALUES>".len();
    let ecuc_extra = format!(
        "{}<ECUC-CONTAINER-VALUE><SHORT-NAME>Partitions</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">/AUTOSAR/EcucDefs/EcuC/EcucPartitionCollection</DEFINITION-REF></ECUC-CONTAINER-VALUE>{}",
        &xml[..insertion],
        &xml[insertion..]
    );
    let config = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-CONTAINER-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text() == Some("/AUTOSAR/EcucDefs/Com/ComConfig")
                })
        })
        .unwrap();
    let insertion = config.range().end - "</SUB-CONTAINERS></ECUC-CONTAINER-VALUE>".len();
    let com_extra = format!(
        "{}<ECUC-CONTAINER-VALUE><SHORT-NAME>Groups</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">/AUTOSAR/EcucDefs/Com/ComConfig/ComIPduGroup</DEFINITION-REF></ECUC-CONTAINER-VALUE>{}",
        &xml[..insertion],
        &xml[insertion..]
    );
    for (name, modified) in [("Partitions", ecuc_extra), ("Groups", com_extra)] {
        fs::write(&source, &modified).unwrap();
        let mut imported = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
        let issue = imported
            .validate()
            .unwrap()
            .issues
            .into_iter()
            .find(|issue| issue.code == "PDU_UNSUPPORTED")
            .unwrap_or_else(|| panic!("unknown {name} must be read-only"));
        assert!(
            issue
                .file
                .as_deref()
                .is_some_and(|file| file.contains("Closure.arxml"))
        );
        assert!(message_has_parameter(
            &imported.save().unwrap_err(),
            "code",
            "PDU_UNSUPPORTED"
        ));
        assert!(message_has_parameter(
            &generator::generate(
                &mut imported,
                &temp.0.join(format!("Unsafe{name}")),
                tooling::native_target()
            )
            .unwrap_err(),
            "code",
            "PDU_UNSUPPORTED"
        ));
        assert_eq!(fs::read_to_string(&source).unwrap(), modified);
    }
    for name in ["ComCfg", "EcuCCfg"] {
        let module = doc
            .descendants()
            .find(|node| {
                node.has_tag_name("ECUC-MODULE-CONFIGURATION-VALUES")
                    && node
                        .children()
                        .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some(name))
            })
            .unwrap();
        let definition = module
            .children()
            .find(|child| child.has_tag_name("DEFINITION-REF"))
            .unwrap();
        let modified = format!(
            "{}<DEFINITION-REF DEST=\"ECUC-MODULE-DEF\">/AUTOSAR/EcucDefs/CanIf</DEFINITION-REF>{}",
            &xml[..definition.range().start],
            &xml[definition.range().end..]
        );
        fs::write(&source, &modified).unwrap();
        let mut imported = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
        let issue = imported
            .validate()
            .unwrap()
            .issues
            .into_iter()
            .find(|issue| issue.code == "PDU_UNSUPPORTED")
            .unwrap_or_else(|| panic!("{name} parent definition must block use"));
        assert_eq!(
            issue.path.as_deref(),
            Some(format!("/Closure/{name}").as_str())
        );
        assert!(
            issue
                .file
                .as_deref()
                .is_some_and(|file| file.contains("Closure.arxml"))
        );
        assert!(message_has_parameter(
            &imported.save().unwrap_err(),
            "code",
            "PDU_UNSUPPORTED"
        ));
        assert!(message_has_parameter(
            &generator::generate(
                &mut imported,
                &temp.0.join(format!("Unsafe{name}")),
                tooling::native_target()
            )
            .unwrap_err(),
            "code",
            "PDU_UNSUPPORTED"
        ));
        assert_eq!(fs::read_to_string(&source).unwrap(), modified);
    }
    let renamed = xml
        .replacen(
            "<SHORT-NAME>ComCfg</SHORT-NAME>",
            "<SHORT-NAME>AltCom</SHORT-NAME>",
            1,
        )
        .replace("/Closure/ComCfg/", "/Closure/AltCom/");
    let renamed_doc = roxmltree::Document::parse(&renamed).unwrap();
    let general = renamed_doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-CONTAINER-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("SHORT-NAME") && child.text() == Some("ComGeneral")
                })
        })
        .unwrap();
    let missing_general = format!(
        "{}{}",
        &renamed[..general.range().start],
        &renamed[general.range().end..]
    );
    let module = renamed_doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-MODULE-CONFIGURATION-VALUES")
                && node
                    .children()
                    .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some("AltCom"))
        })
        .unwrap();
    let definition = module
        .children()
        .find(|child| child.has_tag_name("DEFINITION-REF"))
        .unwrap();
    let foreign_parent = format!(
        "{}<DEFINITION-REF DEST=\"ECUC-MODULE-DEF\">/AUTOSAR/EcucDefs/CanIf</DEFINITION-REF>{}",
        &renamed[..definition.range().start],
        &renamed[definition.range().end..]
    );
    for (name, modified) in [
        ("Renamed", renamed),
        ("MissingGeneral", missing_general),
        ("ForeignParent", foreign_parent),
    ] {
        fs::write(&source, &modified).unwrap();
        let mut imported = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
        assert_eq!(
            imported.view().frames.len(),
            1,
            "the altered ComIPdu remains consumed"
        );
        let issue = imported
            .validate()
            .unwrap()
            .issues
            .into_iter()
            .find(|issue| issue.code == "PDU_UNSUPPORTED")
            .unwrap_or_else(|| panic!("{name} Com owner must be rejected"));
        assert_eq!(issue.path.as_deref(), Some("/Closure/AltCom"));
        assert!(
            issue
                .file
                .as_deref()
                .is_some_and(|file| file.contains("Closure.arxml"))
        );
        assert!(message_has_parameter(
            &imported.save().unwrap_err(),
            "code",
            "PDU_UNSUPPORTED"
        ));
        assert!(message_has_parameter(
            &generator::generate(
                &mut imported,
                &temp.0.join(format!("Unsafe{name}")),
                tooling::native_target()
            )
            .unwrap_err(),
            "code",
            "PDU_UNSUPPORTED"
        ));
        assert_eq!(fs::read_to_string(&source).unwrap(), modified);
    }
}

pub(super) fn multiple_consumed_com_modules_cannot_generate() {
    let temp = Scratch::new();
    let source = temp.0.join("Closure/Closure.arxml");
    let mut project =
        Workspace::create_legacy(source.parent().unwrap(), "Closure", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 1, Direction::Tx, Some(10), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    project.add_signal(frame, "Value".into(), 0, 8, 7).unwrap();
    project.save().unwrap();
    let xml = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let module = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-MODULE-CONFIGURATION-VALUES")
                && node
                    .children()
                    .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some("ComCfg"))
        })
        .unwrap();
    let duplicate = xml[module.range()]
        .replacen(
            "<SHORT-NAME>ComCfg</SHORT-NAME>",
            "<SHORT-NAME>AltCom</SHORT-NAME>",
            1,
        )
        .replace("/Closure/ComCfg/", "/Closure/AltCom/");
    let insertion = xml.find("</ELEMENTS>").unwrap();
    let modified = format!("{}{}{}", &xml[..insertion], duplicate, &xml[insertion..]);
    fs::write(&source, &modified).unwrap();
    let mut imported = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    let issue = imported
        .validate()
        .unwrap()
        .issues
        .into_iter()
        .find(|issue| {
            issue.code == "PDU_UNSUPPORTED"
                && message_has_key(
                    &issue.message,
                    "backend.arxml.host_profile.consumed_com_unique_module_owner_required",
                )
        })
        .expect("two consumed Com modules must not produce a host profile");
    assert_eq!(issue.path.as_deref(), Some("/Closure/AltCom"));
    assert!(
        issue
            .file
            .as_deref()
            .is_some_and(|file| file.contains("Closure.arxml"))
    );
    assert!(imported.save().is_err());
    let output = temp.0.join("UnsafeDuplicateComOwner");
    assert!(generator::generate(&mut imported, &output, tooling::native_target()).is_err());
    assert!(!output.exists());
    assert_eq!(fs::read_to_string(source).unwrap(), modified);
}

pub(super) fn imported_global_pdu_cannot_duplicate_system_binding_or_misstate_diagnostic_length() {
    let temp = Scratch::new();
    let source = temp.0.join("Diag/Diag.arxml");
    let mut project =
        Workspace::create_legacy(source.parent().unwrap(), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 4, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(frame, "Value".into(), 0, 32, 7)
        .unwrap()
        .signals[0]
        .path
        .clone();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: true,
            reset_routine_id: Some(0xf001),
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let xml = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let find_pdu = |name| {
        doc.descendants()
            .find(|node| {
                node.has_tag_name("ECUC-CONTAINER-VALUE")
                    && node
                        .children()
                        .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some(name))
                    && node.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF")
                            && child.text()
                                == Some(
                                    "/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu",
                                )
                    })
            })
            .unwrap()
    };
    let live = find_pdu("Pdu_Live");
    let shadow = xml[live.range()].replacen(
        "<SHORT-NAME>Pdu_Live</SHORT-NAME>",
        "<SHORT-NAME>Pdu_Shadow</SHORT-NAME>",
        1,
    );
    let duplicate = format!(
        "{}{}{}",
        &xml[..live.range().end],
        shadow,
        &xml[live.range().end..]
    );
    let diagnostic = find_pdu("DcmPdu_DiagRequest");
    let short = &xml[diagnostic.range()];
    let wrong_length = format!(
        "{}{}{}",
        &xml[..diagnostic.range().start],
        short.replacen("<VALUE>256</VALUE>", "<VALUE>8</VALUE>", 1),
        &xml[diagnostic.range().end..]
    );
    let request = find_pdu("NPdu_DiagRequest");
    let missing = format!(
        "{}{}",
        &xml[..request.range().start],
        &xml[request.range().end..]
    );
    let insertion = live.range().end - "</ECUC-CONTAINER-VALUE>".len();
    let dependent = format!(
        "{}<REFERENCE-VALUES><ECUC-REFERENCE-VALUE><DEFINITION-REF DEST=\"ECUC-REFERENCE-DEF\">/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu/PduTriggeredByRef</DEFINITION-REF><VALUE-REF DEST=\"PDU-TRIGGERING\">/Diag/UnknownTrigger</VALUE-REF></ECUC-REFERENCE-VALUE></REFERENCE-VALUES>{}",
        &xml[..insertion],
        &xml[insertion..]
    );
    let insertion = live.range().start + xml[live.range()].find("</PARAMETER-VALUES>").unwrap();
    let dynamic = format!(
        "{}<ECUC-NUMERICAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-BOOLEAN-PARAM-DEF\">/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu/DynamicLength</DEFINITION-REF><VALUE>false</VALUE></ECUC-NUMERICAL-PARAM-VALUE>{}",
        &xml[..insertion],
        &xml[insertion..]
    );
    for (name, modified) in [
        ("duplicate", duplicate),
        ("length", wrong_length),
        ("missing", missing),
        ("dependent", dependent),
        ("dynamic", dynamic),
    ] {
        fs::write(&source, &modified).unwrap();
        let mut imported = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
        let issue = imported
            .validate()
            .unwrap()
            .issues
            .into_iter()
            .find(|issue| issue.code == "PDU_UNSUPPORTED")
            .unwrap_or_else(|| {
                panic!("{name} must not generate an ECU from ambiguous PDU bindings")
            });
        assert!(
            issue
                .file
                .as_deref()
                .is_some_and(|file| file.contains("Diag.arxml"))
        );
        assert!(message_has_parameter(
            &imported.save().unwrap_err(),
            "code",
            "PDU_UNSUPPORTED"
        ));
        assert!(message_has_parameter(
            &generator::generate(
                &mut imported,
                &temp.0.join(format!("Unsafe{name}")),
                tooling::native_target()
            )
            .unwrap_err(),
            "code",
            "PDU_UNSUPPORTED"
        ));
        assert_eq!(fs::read_to_string(&source).unwrap(), modified);
    }
}

pub(super) fn diagnostic_ecuc_refs_reject_old_system_destinations_and_dynamic_npdu() {
    let temp = Scratch::new();
    let source = temp.0.join("Diag/Diag.arxml");
    let mut project =
        Workspace::create_legacy(source.parent().unwrap(), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 4, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(frame, "Value".into(), 0, 32, 7)
        .unwrap()
        .signals[0]
        .path
        .clone();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let xml = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    for (name, system, old_dest) in [
        ("CanTpTxNPduRef", "/Diag/NPdu_DiagResponse", "N-PDU"),
        (
            "DcmDslProtocolRxPduRef",
            "/Diag/DcmPdu_DiagRequest",
            "DCM-I-PDU",
        ),
    ] {
        let reference = doc
            .descendants()
            .find(|node| {
                node.has_tag_name("ECUC-REFERENCE-VALUE")
                    && node.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF")
                            && child
                                .text()
                                .is_some_and(|value| value.ends_with(&format!("/{name}")))
                    })
            })
            .unwrap();
        let value = reference
            .children()
            .find(|child| child.has_tag_name("VALUE-REF"))
            .unwrap();
        assert_eq!(value.attribute("DEST"), Some("ECUC-CONTAINER-VALUE"));
        let modified = format!(
            "{}<VALUE-REF DEST=\"{old_dest}\">{system}</VALUE-REF>{}",
            &xml[..value.range().start],
            &xml[value.range().end..]
        );
        fs::write(&source, &modified).unwrap();
        let mut imported = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
        assert!(
            imported
                .validate()
                .unwrap()
                .issues
                .iter()
                .any(|issue| issue.code == "DIAG_UNSUPPORTED"),
            "{name} must not silently bind a system PDU"
        );
        assert!(imported.save().is_err());
        assert!(
            generator::generate(
                &mut imported,
                &temp.0.join(format!("Unsafe{name}")),
                tooling::native_target()
            )
            .is_err()
        );
        assert_eq!(fs::read_to_string(&source).unwrap(), modified);
    }
    let n_pdu = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("N-PDU")
                && node.children().any(|child| {
                    child.has_tag_name("SHORT-NAME") && child.text() == Some("NPdu_DiagRequest")
                })
        })
        .unwrap();
    let insertion = n_pdu.range().start
        + xml[n_pdu.range()].find("</SHORT-NAME>").unwrap()
        + "</SHORT-NAME>".len();
    let modified = format!(
        "{}<HAS-DYNAMIC-LENGTH>true</HAS-DYNAMIC-LENGTH>{}",
        &xml[..insertion],
        &xml[insertion..]
    );
    fs::write(&source, &modified).unwrap();
    let mut imported = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    assert!(
        imported
            .validate()
            .unwrap()
            .issues
            .iter()
            .any(|issue| issue.code == "DIAG_UNSUPPORTED")
    );
    assert!(imported.save().is_err());
    assert!(
        generator::generate(
            &mut imported,
            &temp.0.join("UnsafeDynamicNPdu"),
            tooling::native_target()
        )
        .is_err()
    );
    assert_eq!(fs::read_to_string(source).unwrap(), modified);
}

pub(super) fn imported_unknown_content_survives_supported_edit_without_rewriting_other_file() {
    let temp = Scratch::new();
    let (a, _) = create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    let retained =
        "<I-SIGNAL><SHORT-NAME>RetainedUnknown</SHORT-NAME><LENGTH>1</LENGTH></I-SIGNAL>";
    fs::write(
        &source,
        text.replacen("</ELEMENTS>", &format!("{retained}</ELEMENTS>"), 1),
    )
    .unwrap();
    let other = temp.0.join("Unrelated.arxml");
    let unrelated = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Other</SHORT-NAME><ELEMENTS><I-SIGNAL><SHORT-NAME>Extra</SHORT-NAME><LENGTH>1</LENGTH></I-SIGNAL></ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>";
    fs::write(&other, unrelated).unwrap();
    let mut imported =
        Workspace::open_legacy(vec![source.clone(), other.clone()], archive()).unwrap();
    assert_eq!(imported.view().files.len(), 2);
    let frame = imported
        .view()
        .frames
        .into_iter()
        .find(|f| f.name == "Command")
        .unwrap();
    imported
        .update_frame(&frame.path, serde_json::json!({"id": 802, "dlc": 3}))
        .unwrap();
    let original_source = fs::read_to_string(&source).unwrap();
    let preview = imported.preview_save().unwrap();
    assert_eq!(preview.files.len(), 2);
    let changed_path = fs::canonicalize(&source).unwrap().display().to_string();
    let changed = preview
        .files
        .iter()
        .find(|file| file.path == changed_path)
        .unwrap();
    assert!(changed.changed);
    assert_eq!(changed.before.as_deref(), Some(original_source.as_str()));
    assert!(changed.after.as_deref().unwrap().contains(retained));
    assert_eq!(
        fs::read_to_string(&source).unwrap(),
        original_source,
        "preview must not write ARXML"
    );
    let unchanged_path = fs::canonicalize(&other).unwrap().display().to_string();
    let unchanged = preview
        .files
        .iter()
        .find(|file| file.path == unchanged_path)
        .unwrap();
    assert!(!unchanged.changed);
    assert!(unchanged.before.is_none() && unchanged.after.is_none());
    imported.save_previewed(&preview.revision).unwrap();
    assert_eq!(
        fs::read_to_string(&source).unwrap(),
        changed.after.as_deref().unwrap()
    );
    let reopened = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    let updated = reopened
        .view()
        .frames
        .into_iter()
        .find(|item| item.path == frame.path)
        .unwrap();
    assert_eq!(
        (updated.id, updated.dlc),
        (802, 3),
        "imported frame must retain ID and global PDU length"
    );
    assert!(fs::read_to_string(&source).unwrap().contains(retained));
    assert_eq!(
        unrelated,
        fs::read_to_string(other).unwrap(),
        "unmodified ARXML must stay byte-identical"
    );
    assert_eq!(a.view().frames.len(), 2);
}

pub(super) fn save_does_not_overwrite_external_changes_to_managed_arxml() {
    let temp = Scratch::new();
    let (mut project, _) = create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let frame = project
        .view()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Command")
        .unwrap();
    project
        .update_frame(&frame.path, serde_json::json!({"id": 802}))
        .unwrap();
    let original = fs::read_to_string(&source).unwrap();
    let external = original.replacen("<VALUE>801</VALUE>", "<VALUE>803</VALUE>", 1);
    assert_ne!(original, external);
    fs::write(&source, &external).unwrap();
    assert!(message_has_key(
        &project.save().unwrap_err(),
        "backend.arxml.persistence.stale_source_reimport_required"
    ));
    assert_eq!(fs::read_to_string(&source).unwrap(), external);
    assert!(project.view().dirty);
    assert_eq!(
        fs::read_dir(source.parent().unwrap()).unwrap().count(),
        1,
        "failed save left staging files"
    );
}

pub(super) fn save_preview_rejects_edits_and_external_changes_after_preview() {
    let temp = Scratch::new();
    let (mut project, _) = create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let frame = project
        .view()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Command")
        .unwrap();
    project
        .update_frame(&frame.path, serde_json::json!({"id": 802}))
        .unwrap();
    let first = project.preview_save().unwrap();
    project
        .update_frame(&frame.path, serde_json::json!({"id": 804}))
        .unwrap();
    assert!(message_has_key(
        &project.save_previewed(&first.revision).unwrap_err(),
        "backend.arxml.persistence.save_preview_stale"
    ));
    assert!(
        fs::read_to_string(&source)
            .unwrap()
            .contains("<VALUE>801</VALUE>")
    );

    let second = project.preview_save().unwrap();
    let external = fs::read_to_string(&source).unwrap().replacen(
        "<VALUE>801</VALUE>",
        "<VALUE>803</VALUE>",
        1,
    );
    fs::write(&source, &external).unwrap();
    assert!(message_has_key(
        &project.save_previewed(&second.revision).unwrap_err(),
        "backend.arxml.persistence.stale_source_reimport_required"
    ));
    assert!(message_has_key(
        &project.preview_save().unwrap_err(),
        "backend.arxml.persistence.stale_source_reimport_required"
    ));
    assert_eq!(fs::read_to_string(&source).unwrap(), external);
}

pub(super) fn split_package_save_preserves_sources_and_rejects_stale_reference_file() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let other = temp.0.join("Alpha/Signals.arxml");
    let original = fs::read_to_string(&source).unwrap();
    let start = original
        .find("<I-SIGNAL><SHORT-NAME>ISignal_SendCount</SHORT-NAME>")
        .unwrap();
    let end = start + original[start..].find("</I-SIGNAL>").unwrap() + "</I-SIGNAL>".len();
    let signal = &original[start..end];
    fs::write(&source, original.replacen(signal, "", 1)).unwrap();
    fs::write(&other, format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Alpha</SHORT-NAME><ELEMENTS>{signal}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>\n")).unwrap();

    let mut project =
        Workspace::open_legacy(vec![source.clone(), other.clone()], archive()).unwrap();
    assert!(project.validate().unwrap().issues.is_empty());
    let frame = project
        .view()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Command")
        .unwrap();
    let untouched = fs::read(&other).unwrap();
    project
        .update_frame(&frame.path, serde_json::json!({"id": 802}))
        .unwrap();
    project.save().unwrap();
    assert_eq!(
        fs::read(&other).unwrap(),
        untouched,
        "supported edit must leave the other file byte-identical"
    );
    let mut project =
        Workspace::open_legacy(vec![source.clone(), other.clone()], archive()).unwrap();
    assert!(project.validate().unwrap().issues.is_empty());
    assert_eq!(
        project
            .view()
            .frames
            .iter()
            .find(|item| item.name == "Command")
            .unwrap()
            .id,
        802
    );
    assert!(!project.view().dirty);
    let external =
        fs::read_to_string(&other)
            .unwrap()
            .replacen("<LENGTH>8</LENGTH>", "<LENGTH>7</LENGTH>", 1);
    fs::write(&other, &external).unwrap();
    assert!(message_has_key(
        &project.validate().unwrap_err(),
        "backend.arxml.persistence.stale_source_reimport_required"
    ));
    let unsafe_output = temp.0.join("StaleGeneration");
    assert!(message_has_key(
        &generator::generate(&mut project, &unsafe_output, tooling::native_target()).unwrap_err(),
        "backend.arxml.persistence.stale_source_reimport_required"
    ));
    assert!(
        !unsafe_output.exists(),
        "stale sources must not materialize C99 output"
    );
    project
        .update_frame(&frame.path, serde_json::json!({"id": 803}))
        .unwrap();
    let before_save = fs::read(&source).unwrap();

    assert!(message_has_key(
        &project.save().unwrap_err(),
        "backend.arxml.persistence.stale_source_reimport_required"
    ));
    assert_eq!(
        fs::read(&source).unwrap(),
        before_save,
        "dirty source must not be partly committed"
    );
    assert_eq!(
        fs::read_to_string(&other).unwrap(),
        external,
        "external source must be preserved"
    );
    assert!(project.view().dirty);
}

pub(super) fn three_file_host_can_edit_preserves_retained_and_untouched_sources() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let clock = temp.0.join("Alpha/Clock.arxml");
    let signals = temp.0.join("Alpha/Signals.arxml");
    let original = fs::read_to_string(&source).unwrap();
    let signal_start = original
        .find("<I-SIGNAL><SHORT-NAME>ISignal_SendCount</SHORT-NAME>")
        .unwrap();
    let signal_end =
        signal_start + original[signal_start..].find("</I-SIGNAL>").unwrap() + "</I-SIGNAL>".len();
    let system_signal = &original[signal_start..signal_end];
    let without_signal = original.replacen(system_signal, "", 1);
    let clock_start = without_signal
        .find("<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>McuCfg</SHORT-NAME>")
        .unwrap();
    let clock_end = clock_start
        + without_signal[clock_start..]
            .find("</ECUC-MODULE-CONFIGURATION-VALUES>")
            .unwrap()
        + "</ECUC-MODULE-CONFIGURATION-VALUES>".len();
    let mcu = &without_signal[clock_start..clock_end];
    fs::write(&source, without_signal.replacen(mcu, "", 1)).unwrap();
    let wrap = |content: &str| {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Alpha</SHORT-NAME><ELEMENTS>{content}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>\n"
        )
    };
    fs::write(&clock, wrap(mcu)).unwrap();
    let retained =
        "<I-SIGNAL><SHORT-NAME>RetainedUnknown</SHORT-NAME><LENGTH>1</LENGTH></I-SIGNAL>";
    fs::write(&signals, wrap(&(system_signal.to_owned() + retained))).unwrap();

    let mut project = Workspace::open_legacy(
        vec![source.clone(), clock.clone(), signals.clone()],
        archive(),
    )
    .unwrap();
    assert!(project.validate().unwrap().issues.is_empty());
    assert_eq!(
        project
            .view()
            .files
            .iter()
            .map(|file| file.retained_count)
            .sum::<usize>(),
        1
    );
    let unchanged_clock = fs::read(&clock).unwrap();
    let signal = project
        .view()
        .signals
        .into_iter()
        .find(|signal| signal.name == "SendCount")
        .unwrap();
    project
        .update_signal(&signal.path, serde_json::json!({"length": 7}))
        .unwrap();
    project.save().unwrap();
    assert_eq!(fs::read(&clock).unwrap(), unchanged_clock);
    assert!(fs::read_to_string(&signals).unwrap().contains(retained));
    let mut reopened = Workspace::open_legacy(vec![source, clock, signals], archive()).unwrap();
    assert!(reopened.validate().unwrap().issues.is_empty());
    assert_eq!(
        reopened
            .view()
            .signals
            .into_iter()
            .find(|item| item.name == "SendCount")
            .unwrap()
            .length,
        7
    );
}

pub(super) fn official_r24_sample_imports_as_one_split_package_without_rewriting_sources() {
    let temp = Scratch::new();
    let zip_path = autosar_config_core::schema::reference_archive(
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap(),
        "AUTOSAR_SAMPLE_ARCHIVE",
        autosar_config_core::schema::SAMPLE_ZIP,
    );
    let mut zip = zip::ZipArchive::new(fs::File::open(zip_path).unwrap()).unwrap();
    let prefix =
        "AUTOSAR_CP_EXP_ModelingShowCases/30_MeasurementCalibration/10_Introductory/model/";
    let mut paths = Vec::new();
    let mut originals = Vec::new();
    for index in 0..zip.len() {
        let mut member = zip.by_index(index).unwrap();
        if !member.name().starts_with(prefix) || !member.name().ends_with(".arxml") {
            continue;
        }
        let name = Path::new(member.name()).file_name().unwrap();
        let path = temp.0.join(name);
        let mut content = Vec::new();
        member.read_to_end(&mut content).unwrap();
        fs::write(&path, &content).unwrap();
        originals.push((path.clone(), content));
        paths.push(path);
    }
    assert_eq!(
        paths.len(),
        15,
        "the official R24 example must have all model files"
    );
    let mut project = Workspace::open_legacy(paths, archive()).unwrap();
    assert!(
        project.validate().unwrap().issues.is_empty(),
        "split AR-PACKAGE paths may merge across files"
    );
    assert!(!project.view().dirty);
    project.save().unwrap();
    for (path, content) in originals {
        assert_eq!(content, fs::read(path).unwrap());
    }
    assert!(
        generator::generate(
            &mut project,
            &temp.0.join("Generated"),
            tooling::native_target()
        )
        .is_err(),
        "unconfigured CAN profile must not produce a misleading ECU"
    );
}

pub(super) fn unresolved_r24_variant_is_preserved_but_blocks_generation() {
    let temp = Scratch::new();
    let (a, _) = create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    let anchor = "</START-POSITION></I-SIGNAL-TO-I-PDU-MAPPING>";
    let variant = "<VARIATION-POINT><SHORT-LABEL>UnboundVariant</SHORT-LABEL></VARIATION-POINT>";
    assert!(text.contains(anchor));
    fs::write(
        &source,
        text.replacen(
            anchor,
            &format!("</START-POSITION>{variant}</I-SIGNAL-TO-I-PDU-MAPPING>"),
            1,
        ),
    )
    .unwrap();
    let mut imported = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    let checked = imported.validate().unwrap();
    assert!(
        checked
            .issues
            .iter()
            .any(|issue| issue.code == "VARIANT_DEPENDENCY")
    );
    imported.save().unwrap();
    assert!(fs::read_to_string(source).unwrap().contains(variant));
    let generated = temp.0.join("UnsafeOutput");
    assert!(generator::generate(&mut imported, &generated, tooling::native_target()).is_err());
    assert!(
        !generated.exists(),
        "variant-dependent C99 output must not be materialized"
    );
    assert_eq!(a.view().frames.len(), 2);
}

pub(super) fn package_variant_affecting_profile_blocks_generation() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    let variant = "<VARIATION-POINT><SHORT-LABEL>UnboundPackage</SHORT-LABEL></VARIATION-POINT>";
    fs::write(
        &source,
        text.replacen("</AR-PACKAGE>", &format!("{variant}</AR-PACKAGE>"), 1),
    )
    .unwrap();
    let mut imported = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    let checked = imported.validate().unwrap();
    assert!(
        checked
            .issues
            .iter()
            .any(|issue| issue.code == "VARIANT_DEPENDENCY"),
        "{:?}",
        checked.issues
    );
    let generated = temp.0.join("UnsafePackageOutput");
    assert!(generator::generate(&mut imported, &generated, tooling::native_target()).is_err());
    assert!(!generated.exists());
}

pub(super) fn conflicting_canif_entries_for_one_pdu_block_generation() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    let start = text
        .find("<ECUC-CONTAINER-VALUE><SHORT-NAME>Can_Command</SHORT-NAME>")
        .unwrap();
    let end = start
        + text[start..].find("</ECUC-CONTAINER-VALUE>").unwrap()
        + "</ECUC-CONTAINER-VALUE>".len();
    let duplicate = text[start..end]
        .replace(
            "<SHORT-NAME>Can_Command</SHORT-NAME>",
            "<SHORT-NAME>Can_Conflict</SHORT-NAME>",
        )
        .replace("<VALUE>801</VALUE>", "<VALUE>1536</VALUE>");
    assert!(duplicate.contains("<VALUE>1536</VALUE>"));
    fs::write(
        &source,
        format!("{}{}{}", &text[..end], duplicate, &text[end..]),
    )
    .unwrap();
    let mut imported = Workspace::open_legacy(vec![source], archive()).unwrap();
    let checked = imported.validate().unwrap();
    assert!(
        checked
            .issues
            .iter()
            .any(|issue| issue.code == "CANIF_PDU_DUPLICATE"),
        "{:?}",
        checked.issues
    );
    let generated = temp.0.join("UnsafeCanIfOutput");
    assert!(generator::generate(&mut imported, &generated, tooling::native_target()).is_err());
    assert!(!generated.exists());
}

pub(super) fn ipdu_mapping_disagreement_with_com_blocks_generation() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    for (field, original, changed) in [
        (
            "byte order",
            "<PACKING-BYTE-ORDER>MOST-SIGNIFICANT-BYTE-LAST</PACKING-BYTE-ORDER>",
            "<PACKING-BYTE-ORDER>MOST-SIGNIFICANT-BYTE-FIRST</PACKING-BYTE-ORDER>",
        ),
        (
            "start position",
            "<START-POSITION>3</START-POSITION>",
            "<START-POSITION>4</START-POSITION>",
        ),
    ] {
        assert!(text.contains(original), "missing {field} fixture");
        fs::write(&source, text.replacen(original, changed, 1)).unwrap();
        let mut imported = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
        let checked = imported.validate().unwrap();
        assert!(
            checked
                .issues
                .iter()
                .any(|issue| issue.code == "PDU_MAPPING"),
            "{field}: {:?}",
            checked.issues
        );
        let generated = temp.0.join(format!("Unsafe{field}Output"));
        assert!(
            generator::generate(&mut imported, &generated, tooling::native_target()).is_err(),
            "{field} generated incompatible C99"
        );
        assert!(!generated.exists());
    }
}

pub(super) fn linked_can_frame_must_match_canif_and_pdu_layout() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    let topology = r#"
<CAN-FRAME><SHORT-NAME>CommandFrame</SHORT-NAME><FRAME-LENGTH>2</FRAME-LENGTH>
  <PDU-TO-FRAME-MAPPINGS><PDU-TO-FRAME-MAPPING><SHORT-NAME>CommandMapping</SHORT-NAME>
    <PACKING-BYTE-ORDER>MOST-SIGNIFICANT-BYTE-LAST</PACKING-BYTE-ORDER>
    <PDU-REF DEST="I-SIGNAL-I-PDU">/Alpha/Pdu_Command</PDU-REF><START-POSITION>0</START-POSITION>
  </PDU-TO-FRAME-MAPPING></PDU-TO-FRAME-MAPPINGS></CAN-FRAME>
<CAN-CLUSTER><SHORT-NAME>Network</SHORT-NAME><CAN-CLUSTER-VARIANTS><CAN-CLUSTER-CONDITIONAL>
  <PHYSICAL-CHANNELS><CAN-PHYSICAL-CHANNEL><SHORT-NAME>Bus</SHORT-NAME>
    <FRAME-TRIGGERINGS><CAN-FRAME-TRIGGERING><SHORT-NAME>CommandOnBus</SHORT-NAME>
      <FRAME-REF DEST="CAN-FRAME">/Alpha/CommandFrame</FRAME-REF><IDENTIFIER>801</IDENTIFIER>
    </CAN-FRAME-TRIGGERING></FRAME-TRIGGERINGS>
  </CAN-PHYSICAL-CHANNEL></PHYSICAL-CHANNELS>
</CAN-CLUSTER-CONDITIONAL></CAN-CLUSTER-VARIANTS></CAN-CLUSTER>"#;
    let compatible = text.replacen("</ELEMENTS>", &format!("{topology}</ELEMENTS>"), 1);
    fs::write(&source, &compatible).unwrap();
    let mut imported = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    assert!(imported.validate().unwrap().issues.is_empty());
    generator::generate(
        &mut imported,
        &temp.0.join("CompatibleNetworkOutput"),
        tooling::native_target(),
    )
    .unwrap();
    for (original, changed) in [
        (
            "<START-POSITION>0</START-POSITION>",
            "<START-POSITION>8</START-POSITION>",
        ),
        (
            "<FRAME-LENGTH>2</FRAME-LENGTH>",
            "<FRAME-LENGTH>3</FRAME-LENGTH>",
        ),
    ] {
        let altered_topology = topology.replacen(original, changed, 1);
        fs::write(
            &source,
            text.replacen("</ELEMENTS>", &format!("{altered_topology}</ELEMENTS>"), 1),
        )
        .unwrap();
        let mut mismatched = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
        let checked = mismatched.validate().unwrap();
        assert!(
            checked
                .issues
                .iter()
                .any(|issue| issue.code == "CAN_FRAME_MAPPING"),
            "{:?}",
            checked.issues
        );
        assert!(
            generator::generate(
                &mut mismatched,
                &temp.0.join("UnsafeFrameOutput"),
                tooling::native_target()
            )
            .is_err()
        );
    }
    fs::write(
        &source,
        compatible.replacen(
            "<IDENTIFIER>801</IDENTIFIER>",
            "<IDENTIFIER>802</IDENTIFIER>",
            1,
        ),
    )
    .unwrap();
    imported = Workspace::open_legacy(vec![source], archive()).unwrap();
    let checked = imported.validate().unwrap();
    assert!(
        checked
            .issues
            .iter()
            .any(|issue| issue.code == "CAN_ID_MISMATCH"),
        "{:?}",
        checked.issues
    );
    let generated = temp.0.join("UnsafeNetworkOutput");
    assert!(generator::generate(&mut imported, &generated, tooling::native_target()).is_err());
    assert!(!generated.exists());
}

pub(super) fn noncanonical_pdu_input_is_not_rewritten_or_generated() {
    let temp = Scratch::new();
    let source = temp.0.join("Unsupported.arxml");
    let original = include_str!("../fixtures/legacy-signal.arxml");
    fs::write(&source, original).unwrap();
    let mut project = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    let view = project.validate().unwrap();
    assert!(!view.dirty);
    assert!(view.files.iter().all(|file| file.readonly));
    assert!(view.issues.iter().any(|issue| {
        issue.code == "PDU_UNSUPPORTED"
            && issue
                .path
                .as_deref()
                .is_some_and(|path| path.contains("ComCfg"))
    }));
    assert!(message_has_parameter(
        &project.preview_save().unwrap_err(),
        "code",
        "PDU_UNSUPPORTED"
    ));
    assert!(project.save().is_err());
    let output = temp.0.join("Rejected");
    assert!(generator::generate(&mut project, &output, tooling::native_target()).is_err());
    assert!(!output.exists());
    assert_eq!(fs::read_to_string(source).unwrap(), original);
}

pub(super) fn host_routine_metadata_rejects_unknown_version_and_wrong_session() {
    let temp = Scratch::new();
    let mut project = Workspace::create_legacy(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 4, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(frame, "Value".into(), 0, 32, 1)
        .unwrap()
        .signals[0]
        .path
        .clone();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: true,
            reset_routine_id: Some(0xf001),
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("Diag/Diag.arxml");
    let saved = fs::read_to_string(&source).unwrap();
    let start = saved
        .find("<SDG GID=\"AutosarWorkbenchHostRestoreDidV1\">")
        .unwrap();
    let end = start + saved[start..].find("</SDG>").unwrap() + "</SDG>".len();
    let group = &saved[start..end];
    for (altered, reason) in [
        (
            saved.replace(
                "AutosarWorkbenchHostRestoreDidV1",
                "AutosarWorkbenchHostRestoreDidV2",
            ),
            "backend.arxml.host_profile.unsupported_host_restore_routine_group",
        ),
        (
            saved.replace(
                "<SD GID=\"SessionRef\">/Diag/DcmCfg/DcmConfigSet/DcmDsp/Sessions/Extended</SD>",
                "<SD GID=\"SessionRef\">/Diag/DcmCfg/DcmConfigSet/DcmDsp/Sessions/Default</SD>",
            ),
            "backend.arxml.host_profile.host_restore_routine_session_reference_required",
        ),
        (
            saved.replace("<SD GID=\"Rid\">61441</SD>", ""),
            "backend.arxml.host_profile.host_restore_routine_unique_fields_required",
        ),
        (
            saved.replacen(group, &format!("{group}{group}"), 1),
            "backend.arxml.host_profile.duplicate_host_restore_routine_group",
        ),
    ] {
        assert_ne!(saved, altered);
        fs::write(&source, &altered).unwrap();
        let error = Workspace::open_legacy(vec![source.clone()], archive())
            .err()
            .expect("invalid host metadata must not be accepted");
        assert!(
            message_has_key(&error, reason)
                && message_has_parameter(
                    &error,
                    "path",
                    &fs::canonicalize(&source).unwrap().display().to_string()
                ),
            "{error}"
        );
        assert_eq!(fs::read_to_string(&source).unwrap(), altered);
    }
}
