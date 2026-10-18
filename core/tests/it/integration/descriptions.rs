//! Independent expectations for the delivered BSWMD/manual boundary.
use super::*;

#[test]
fn module_descriptions_include_delivered_public_entries_and_manuals() {
    let plan = build(&inputs()).unwrap();
    let files = prepared_pipeline_files(&plan);
    let text = |path: &str| {
        std::str::from_utf8(&files.iter().find(|(file, _)| file == path).unwrap().1).unwrap()
    };
    let xml = roxmltree::Document::parse(text("generated/descriptions/Host_Implementation.arxml"))
        .unwrap();
    // R24-11 API tables, including APIs omitted from the former handwritten inventory.
    for (name, reentrant, synchronous) in [
        ("Can_DisableControllerInterrupts", true, true),
        ("Can_EnableControllerInterrupts", true, true),
        ("Can_CheckWakeup", false, true),
        ("CanIf_DeInit", false, true),
        ("CanIf_GetControllerRxErrorCounter", false, true),
        ("CanIf_GetControllerTxErrorCounter", false, true),
        ("CanTp_CancelTransmit", true, true),
        ("CanTp_CancelReceive", false, true),
        ("Com_GetStatus", true, true),
        ("Com_InvalidateSignal", true, false),
        ("Com_IpduGroupStart", true, true),
        ("Com_IpduGroupStop", true, true),
        ("ComM_GetRequestedComMode", true, true),
        ("ComM_GetCurrentPNCComMode", true, true),
        ("ComM_CommunicationAllowed", false, false),
        ("ComM_EcuM_WakeUpIndication", true, true),
        ("Dcm_GetSecurityLevel", true, true),
        ("Det_ReportError", true, true),
        ("PduR_EnableRouting", true, true),
        ("PduR_DisableRouting", true, true),
        ("LSduR_GetConfigurationId", true, true),
        ("ActivateTask", true, true),
        ("Os_SetEvent", true, true),
        ("GetAlarm", true, true),
        ("StartOS", false, true),
        ("Arti_GetVersionInfo", true, true),
        ("Ecu_HostBusSM_DeInit", false, true),
        ("Ecu_HostBusSM_ControllerBusOff", true, true),
    ] {
        let entry = xml
            .descendants()
            .find(|node| {
                node.has_tag_name("BSW-MODULE-ENTRY")
                    && node
                        .children()
                        .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some(name))
            })
            .unwrap_or_else(|| panic!("missing {name}"));
        for (tag, expected) in [("IS-REENTRANT", reentrant), ("IS-SYNCHRONOUS", synchronous)] {
            assert_eq!(
                entry
                    .children()
                    .find(|node| node.has_tag_name(tag))
                    .unwrap()
                    .text(),
                Some(if expected { "true" } else { "false" }),
                "{name}/{tag}"
            );
        }
    }
    for (name, call_type, context) in [
        ("Ecu_DetRuntimeCallout", "CALLOUT", "UNSPECIFIED"),
        ("ErrorHook", "CALLOUT", "HOOK"),
        ("BswM_MainFunction", "SCHEDULED", "TASK"),
        ("ComM_EcuM_WakeUpIndication", "CALLBACK", "UNSPECIFIED"),
    ] {
        let entry = xml
            .descendants()
            .find(|node| {
                node.has_tag_name("BSW-MODULE-ENTRY")
                    && node
                        .children()
                        .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some(name))
            })
            .unwrap_or_else(|| panic!("missing {name}"));
        for (tag, expected) in [("CALL-TYPE", call_type), ("EXECUTION-CONTEXT", context)] {
            assert_eq!(
                entry
                    .children()
                    .find(|node| node.has_tag_name(tag))
                    .unwrap()
                    .text(),
                Some(expected)
            );
        }
    }
    // SWS_CANIF_91003: the output is caller-owned uint8 storage, distinct from the controller value.
    let counter = xml
        .descendants()
        .find(|node| {
            node.has_tag_name("BSW-MODULE-ENTRY")
                && node.children().any(|child| {
                    child.has_tag_name("SHORT-NAME")
                        && child.text() == Some("CanIf_GetControllerRxErrorCounter")
                })
        })
        .unwrap();
    let arguments: Vec<_> = counter
        .children()
        .find(|node| node.has_tag_name("ARGUMENTS"))
        .unwrap()
        .children()
        .filter(|node| node.has_tag_name("SW-SERVICE-ARG"))
        .map(|arg| {
            let field = |tag| {
                arg.children()
                    .find(|node| node.has_tag_name(tag))
                    .unwrap()
                    .text()
                    .unwrap()
            };
            let base_ref = arg
                .descendants()
                .find(|node| node.has_tag_name("BASE-TYPE-REF"))
                .unwrap()
                .text()
                .unwrap()
                .rsplit('/')
                .next()
                .unwrap();
            let base = xml
                .descendants()
                .find(|node| {
                    node.has_tag_name("SW-BASE-TYPE")
                        && node.children().any(|child| {
                            child.has_tag_name("SHORT-NAME") && child.text() == Some(base_ref)
                        })
                })
                .unwrap();
            let native = base
                .children()
                .find(|node| node.has_tag_name("NATIVE-DECLARATION"))
                .unwrap()
                .text()
                .unwrap();
            (field("SHORT-NAME"), field("DIRECTION"), native)
        })
        .collect();
    assert_eq!(
        arguments,
        [
            ("ControllerId", "IN", "uint8"),
            ("RxErrorCounterPtr", "OUT", "uint8 *")
        ]
    );
    let det = xml
        .descendants()
        .find(|node| {
            node.has_tag_name("BSW-MODULE-DESCRIPTION")
                && node
                    .children()
                    .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some("Det"))
        })
        .unwrap();
    assert!(
        det.descendants()
            .any(|node| node.has_tag_name("BSW-MODULE-ENTRY-REF")
                && node.text() == Some("/Autsaro_Rte/BswModuleEntrys/Ecu_DetRuntimeCallout"))
    );
    // Verifies: constr_4016, constr_4017, TPS_BSWMDT_04002, TPS_BSWMDT_04130.
    // Callout prototypes remain interfaces; only ordinary/callback or scheduled
    // entries can acquire the corresponding executable entity type.
    for entity in xml.descendants().filter(|node| {
        matches!(
            node.tag_name().name(),
            "BSW-CALLED-ENTITY" | "BSW-SCHEDULABLE-ENTITY"
        )
    }) {
        let name = entity
            .children()
            .find(|node| node.has_tag_name("IMPLEMENTED-ENTRY-REF"))
            .unwrap()
            .text()
            .unwrap()
            .rsplit('/')
            .next()
            .unwrap();
        let entry = xml
            .descendants()
            .find(|node| {
                node.has_tag_name("BSW-MODULE-ENTRY")
                    && node
                        .children()
                        .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some(name))
            })
            .unwrap();
        let call_type = entry
            .children()
            .find(|node| node.has_tag_name("CALL-TYPE"))
            .unwrap()
            .text()
            .unwrap();
        if entity.has_tag_name("BSW-CALLED-ENTITY") {
            assert!(
                matches!(call_type, "REGULAR" | "CALLBACK"),
                "{name}: {call_type}"
            );
        } else {
            assert_eq!(call_type, "SCHEDULED", "{name}");
        }
    }
    let owner = |name| {
        xml.descendants()
            .find(|node| {
                node.has_tag_name("BSW-MODULE-DESCRIPTION")
                    && node
                        .children()
                        .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some(name))
            })
            .unwrap()
    };
    for name in [
        "ErrorHook",
        "PreTaskHook",
        "PostTaskHook",
        "StartupHook",
        "ShutdownHook",
    ] {
        let reference = format!("/Autsaro_Rte/BswModuleEntrys/{name}");
        for (module, role) in [("Os", "EXPECTED-ENTRYS"), ("Rte", "IMPLEMENTED-ENTRYS")] {
            assert!(
                owner(module)
                    .children()
                    .find(|node| node.has_tag_name(role))
                    .unwrap()
                    .descendants()
                    .any(|node| node.has_tag_name("BSW-MODULE-ENTRY-REF")
                        && node.text() == Some(reference.as_str())),
                "{module}/{role}/{name}"
            );
        }
        assert!(
            !owner("Os")
                .children()
                .find(|node| node.has_tag_name("IMPLEMENTED-ENTRYS"))
                .unwrap()
                .descendants()
                .any(|node| node
                    .text()
                    .is_some_and(|value| value.ends_with(&format!("/{name}"))))
        );
    }
    let rte_manual = text("generated/descriptions/modules/Rte/README.md");
    assert!(rte_manual.contains("integration/src/Ecu_OsHooks.c"));
    assert!(rte_manual.contains("integration/src/Ecu_Target.c"));
    let bus = xml
        .descendants()
        .find(|node| {
            node.has_tag_name("BSW-MODULE-DESCRIPTION")
                && node.children().any(|child| {
                    child.has_tag_name("SHORT-NAME") && child.text() == Some("Ecu_HostBusSM")
                })
        })
        .unwrap();
    assert!(bus.descendants().any(|node| {
        node.has_tag_name("EXCLUSIVE-AREA")
            && node.children().any(|child| {
                child.has_tag_name("SHORT-NAME") && child.text() == Some("CAN_HOST_AREA_BUSSM")
            })
    }));
    for module in [
        "Can",
        "CanIf",
        "CanTp",
        "Com",
        "Dcm",
        "Det",
        "PduR",
        "LSduR",
        "ComM",
        "BswM",
        "Os",
        "Arti",
        "Rte",
        "Ecu_HostBusSM",
    ] {
        let manual = text(&format!(
            "generated/descriptions/modules/{module}/README.md"
        ));
        for section in [
            "## Overview",
            "## History",
            "## Files",
            "## Resources and integration",
            "## Configuration",
            "## API",
            "## Requirements and limitations",
        ] {
            assert!(manual.contains(section), "{module}: {section}");
        }
        assert!(manual.contains("2026-10-18"));
        assert!(manual.contains("#59"));
        assert!(
            text("generated/descriptions/MODULES.md")
                .contains(&format!("modules/{module}/README.md"))
        );
    }
    for (path, bytes) in files.iter().filter(|(path, _)| {
        path.starts_with("generated/descriptions/modules/") && path.ends_with(".md")
    }) {
        assert!(bytes.len() <= 20 * 1024, "oversized manual: {path}");
    }
    let manuals = files
        .iter()
        .filter(|(path, _)| path.starts_with("generated/descriptions/modules/"))
        .map(|(_, bytes)| std::str::from_utf8(bytes).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(manuals.contains("CanControllerBaudRate"));
    assert!(manuals.contains("CanControllerBaseAddress"));
    assert!(manuals.contains("definition range: false, true (0, 1)"));
    assert!(manuals.contains("Reference destinations:"));
    assert!(manuals.contains("target definitions:"));
    let mode_period = manuals
        .split("### ")
        .find(|section| section.contains("/ `CanMainFunctionModePeriod`\n"))
        .unwrap();
    assert!(mode_period.contains("must equal the OS counter tick"));
    assert!(mode_period.contains("CAN_POLLING_TIMEBASE"));
    assert!(!mode_period.contains("Descriptor only"));
    assert!(manuals.contains("Unit:"));
    assert!(manuals.contains("definition range:"));
    assert!(manuals.contains("No declared default"));
    assert!(manuals.contains("Must be false/0; enabling is rejected"));
    assert!(
        manuals.contains("@param[out] SecurityLevel") || manuals.contains("@param[out] SecLevel")
    );
    assert!(manuals.contains(
        "Std_ReturnType ComM_RequestComMode(ComM_UserHandleType User, ComM_ModeType ComMode)"
    ));
    assert!(manuals.contains("PduR_PBConfigIdType PduR_GetConfigurationId(void)"));
    assert!(manuals.contains("Ecu_RuntimeConfig.c"));
}

#[test]
fn selected_configuration_manual_tracks_the_validated_input_value() {
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|text| text.ends_with("/CanControllerBaudRate"))
            })
        },
        |text| text.replace("<VALUE>500</VALUE>", "<VALUE>250</VALUE>"),
    );
    replace_all(
        &mut sources,
        "<BAUDRATE>500000</BAUDRATE>",
        "<BAUDRATE>250000</BAUDRATE>",
    );
    // The paired controller/cluster bitrate remains valid and is visible in the manual.
    let plan = build(&sources).unwrap();
    let files = prepared_pipeline_files(&plan);
    let pages = files
        .iter()
        .filter(|(path, _)| path.starts_with("generated/descriptions/modules/Can/configuration-"))
        .map(|(_, bytes)| std::str::from_utf8(bytes).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let start = pages.find("`CanControllerBaudRate`").unwrap();
    assert!(
        pages[start..]
            .split("###")
            .next()
            .unwrap()
            .contains("Selected value/reference: `250`")
    );
}

#[test]
fn unselected_diagnostic_mains_remain_described_without_fabricated_activation() {
    let plan = build(&sources_without_application_did(false)).unwrap();
    assert!(plan.description().diagnostic_transport.is_none());
    let files = prepared_pipeline_files(&plan);
    for module in ["Dcm", "CanTp"] {
        let path = format!("generated/descriptions/modules/{module}/README.md");
        let manual =
            std::str::from_utf8(&files.iter().find(|(file, _)| file == &path).unwrap().1).unwrap();
        assert!(manual.contains("Diagnostic activation is disabled"));
        assert!(manual.contains("neither initializes nor schedules this module"));
        let entry = format!("{module}_MainFunction");
        let description = files
            .iter()
            .find(|(file, _)| file == "generated/descriptions/Host_Implementation.arxml")
            .unwrap();
        let xml = roxmltree::Document::parse(std::str::from_utf8(&description.1).unwrap()).unwrap();
        assert!(xml.descendants().any(|node| {
            node.has_tag_name("BSW-MODULE-ENTRY")
                && node.children().any(|child| {
                    child.has_tag_name("SHORT-NAME") && child.text() == Some(entry.as_str())
                })
        }));
        assert!(
            !plan
                .description()
                .schedule
                .entities
                .iter()
                .any(|entity| entity.symbol == entry)
        );
    }
}
