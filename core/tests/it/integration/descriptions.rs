//! Independent expectations for the delivered BSWMD/manual boundary.
use super::*;
use std::collections::BTreeSet;

#[path = "../../../build_support/software_version.rs"]
mod software_version;

#[test]
fn software_version_owner_requires_each_literal_uint8_component() {
    let source = "#define OWNER_SW_MAJOR_VERSION 2U\n#define OWNER_SW_MINOR_VERSION 3U\n#define OWNER_SW_PATCH_VERSION 4U\n";
    assert_eq!(software_version::read(source, "OWNER").unwrap(), "2.3.4");
    for (part, current) in [("MAJOR", 2), ("MINOR", 3), ("PATCH", 4)] {
        let name = format!("OWNER_SW_{part}_VERSION");
        let definition = format!("#define {name} {current}U\n");
        for replacement in [
            String::new(),
            format!("#define {name} 256U\n"),
            format!("#define {name} 01U\n"),
            format!("{definition}{definition}"),
            format!("#define {name} OTHER\n"),
        ] {
            let error = software_version::read(&source.replace(&definition, &replacement), "OWNER")
                .unwrap_err();
            assert!(error.starts_with(&format!("{name}:")), "{error}");
        }
    }
}

#[test]
fn software_versions_are_published_by_the_selected_description_and_manual() {
    let files = prepared_pipeline_files(&build(&inputs()).unwrap());
    let text = |path: &str| {
        std::str::from_utf8(&files.iter().find(|(file, _)| file == path).unwrap().1).unwrap()
    };
    let document =
        roxmltree::Document::parse(text("generated/descriptions/Host_Implementation.arxml"))
            .unwrap();
    for implementation in document
        .descendants()
        .filter(|node| node.has_tag_name("BSW-IMPLEMENTATION"))
    {
        let name = implementation
            .children()
            .find(|node| node.has_tag_name("SHORT-NAME"))
            .unwrap()
            .text()
            .unwrap();
        let module = name.strip_suffix("HostImplementation").unwrap();
        let expected = match module {
            "Rte" => "0.0.0",
            "Arti" => "1.0.0",
            _ => "0.1.0",
        };
        assert_eq!(
            implementation
                .children()
                .find(|node| node.has_tag_name("SW-VERSION"))
                .unwrap()
                .text(),
            Some(expected),
            "{module}"
        );
        let manual = text(&format!(
            "generated/descriptions/modules/{module}/README.md"
        ));
        let owner = if module == "Arti" {
            "Arti_Version.h"
        } else {
            "Autsaro_Version.h"
        };
        assert!(
            implementation
                .descendants()
                .any(|node| node.has_tag_name("SHORT-LABEL") && node.text() == Some(owner)),
            "{module}: missing {owner}"
        );
        assert!(
            manual.contains(owner),
            "{module}: missing owner documentation"
        );
        assert!(
            manual.contains(&format!("Implementation version: `{expected}`")),
            "{module}"
        );
    }
}

#[test]
fn os_memory_mapping_description_uses_delivered_sections_and_one_header() {
    let files = prepared_pipeline_files(&build(&inputs()).unwrap());
    let source = |path: &str| {
        std::str::from_utf8(&files.iter().find(|(file, _)| file == path).unwrap().1).unwrap()
    };
    let xml =
        roxmltree::Document::parse(source("generated/descriptions/Host_Implementation.arxml"))
            .unwrap();
    assert!(
        !xml.descendants()
            .filter_map(|node| node.text())
            .any(|text| text.contains("OS_MEMMAP"))
    );
    let os_sections = [
        "CODE",
        "CONFIG_DATA_PREBUILD_8",
        "CONFIG_DATA_PREBUILD_64",
        "CONST_64",
        "CONST_8",
        "CONST_PTR",
        "VAR_CLEARED_32",
        "VAR_CLEARED_64",
        "VAR_CLEARED_8",
        "VAR_CLEARED_PTR",
        "VAR_CLEARED_TLS_32",
        "VAR_CLEARED_TLS_64",
        "VAR_CLEARED_TLS_8",
        "VAR_CLEARED_TLS_PTR",
        "VAR_CLEARED_TLS_UNSPECIFIED",
        "VAR_CLEARED_UNSPECIFIED",
        "VAR_INIT_32",
        "VAR_INIT_8",
        "VAR_INIT_UNSPECIFIED",
        "VAR_INIT_VECTOR_PTR",
    ];
    let arti_sections = [
        "CODE",
        "VAR_CLEARED_32",
        "VAR_CLEARED_64",
        "VAR_CLEARED_TLS_64",
    ];
    // Verifies[Os]: SWS_BSW_00006.
    // Verifies: SWS_MemMap_00003.
    for (module, expected) in [
        ("Os", os_sections.as_slice()),
        ("Arti", arti_sections.as_slice()),
    ] {
        let name = format!("{module}HostImplementation");
        let implementation = xml
            .descendants()
            .find(|node| named(*node, "BSW-IMPLEMENTATION", &name))
            .unwrap();
        let sections: BTreeSet<_> = implementation
            .descendants()
            .filter(|node| node.has_tag_name("MEMORY-SECTION"))
            .map(|node| {
                node.children()
                    .find(|child| child.has_tag_name("SYMBOL"))
                    .unwrap()
                    .text()
                    .unwrap()
            })
            .collect();
        assert_eq!(sections, expected.iter().copied().collect(), "{module}");
        for section in expected {
            assert!(
                source("generated/include/Os_MemMap.h")
                    .contains(&format!("defined(OS_START_SEC_{section})")),
                "{section}"
            );
        }
        let owner = implementation
            .descendants()
            .find(|node| node.has_tag_name("IMPLEMENTED-IN-REF"))
            .unwrap();
        assert_eq!(
            owner.text(),
            Some(if module == "Arti" {
                "/AUTOSAR_Arti/ArtiHostImplementation/required_Os_MemMap_h"
            } else {
                "/AUTOSAR_Os/OsHostImplementation/generated_include_Os_MemMap_h"
            })
        );
    }
    let header = xml
        .descendants()
        .find(|node| {
            named(
                *node,
                "DEPENDENCY-ON-ARTIFACT",
                "generated_include_Os_MemMap_h",
            )
        })
        .unwrap();
    assert!(
        header
            .descendants()
            .any(|node| node.has_tag_name("SHORT-LABEL") && node.text() == Some("Os_MemMap.h"))
    );
    assert!(
        header
            .descendants()
            .any(|node| node.has_tag_name("DOMAIN") && node.text() == Some("generated.include"))
    );
    let config = source("generated/src/Ecu_Config.c");
    assert!(config.contains("OS_MEMMAP const Os_TargetConfigType Ecu_OsConfig"));
    assert!(config.contains("OS_MEMMAP static const Os_AlarmConfigType alarms[]"));
    assert!(config.contains("OS_START_SEC_CONFIG_DATA_PREBUILD_8"));
    assert!(config.contains("OS_START_SEC_CONFIG_DATA_PREBUILD_64"));
    let descriptor = xml
        .descendants()
        .find(|node| {
            named(
                *node,
                "DEPENDENCY-ON-ARTIFACT",
                "generated_src_Ecu_Config_c",
            )
        })
        .unwrap();
    assert!(
        descriptor
            .descendants()
            .any(|node| node.has_tag_name("SHORT-LABEL") && node.text() == Some("Ecu_Config.c"))
    );
}

#[test]
fn nondefault_mapping_headers_are_required_artifacts_of_the_using_implementation() {
    // Verifies: SWS_MemMap_00028, SWS_MemMap_00034, SWS_MemMap_00035.
    let files = prepared_pipeline_files(&build(&inputs()).unwrap());
    let bytes = &files
        .iter()
        .find(|(file, _)| file == "generated/descriptions/Host_Implementation.arxml")
        .unwrap()
        .1;
    let document = roxmltree::Document::parse(std::str::from_utf8(bytes).unwrap()).unwrap();
    for (module, expected) in [
        ("Arti", vec![("OS", "Os_MemMap.h")]),
        (
            "LSduR",
            vec![("LSDUR", "LSduR_MemMap.h"), ("RTE_LSduR", "Rte_MemMap.h")],
        ),
        (
            "PduR",
            vec![("PDUR", "PduR_MemMap.h"), ("RTE_PduR", "Rte_MemMap.h")],
        ),
    ] {
        let implementation = document
            .descendants()
            .find(|node| {
                named(
                    *node,
                    "BSW-IMPLEMENTATION",
                    &format!("{module}HostImplementation"),
                )
            })
            .unwrap();
        let required = implementation
            .children()
            .find(|node| node.has_tag_name("REQUIRED-ARTIFACTS"))
            .unwrap();
        assert_eq!(
            required.children().filter(|node| node.is_element()).count(),
            expected.len()
        );
        for (prefix, header) in expected {
            let dependency = required
                .children()
                .find(|node| {
                    node.descendants().any(|child| {
                        child.has_tag_name("SHORT-LABEL") && child.text() == Some(header)
                    })
                })
                .unwrap();
            assert!(
                dependency
                    .children()
                    .any(|node| { node.has_tag_name("CATEGORY") && node.text() == Some("MEMMAP") })
            );
            let descriptor = dependency
                .children()
                .find(|node| node.has_tag_name("ARTIFACT-DESCRIPTOR"))
                .unwrap();
            assert!(
                descriptor
                    .children()
                    .any(|node| { node.has_tag_name("CATEGORY") && node.text() == Some("SWHDR") })
            );
            assert!(
                dependency
                    .descendants()
                    .any(|node| { node.has_tag_name("USAGE") && node.text() == Some("COMPILE") })
            );
            let name = dependency
                .children()
                .find(|node| node.has_tag_name("SHORT-NAME"))
                .unwrap()
                .text()
                .unwrap();
            let section_prefix = implementation
                .descendants()
                .find(|node| named(*node, "SECTION-NAME-PREFIX", prefix))
                .unwrap();
            let reference = section_prefix
                .children()
                .find(|node| node.has_tag_name("IMPLEMENTED-IN-REF"))
                .unwrap();
            assert_eq!(
                reference.text(),
                Some(format!("/AUTOSAR_{module}/{module}HostImplementation/{name}").as_str())
            );
        }
        if module != "Arti" {
            let sections: BTreeSet<_> = implementation
                .descendants()
                .filter(|node| node.has_tag_name("MEMORY-SECTION"))
                .flat_map(|node| node.children().filter(|child| child.has_tag_name("SYMBOL")))
                .filter_map(|node| node.text())
                .collect();
            assert_eq!(
                sections,
                [
                    "CODE",
                    "VAR_CLEARED_UNSPECIFIED",
                    "CONFIG_DATA_PREBUILD_UNSPECIFIED"
                ]
                .into_iter()
                .collect()
            );
        }
    }
}

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
    // Verifies: SWS_MemMap_00026, SWS_MemMap_00033.
    let rte = xml
        .descendants()
        .find(|node| named(*node, "BSW-IMPLEMENTATION", "RteHostImplementation"))
        .unwrap();
    for source in [
        "SchM_CanIf_Host.c",
        "SchM_Mode_Host.c",
        "SchM_Diagnostic_Host.c",
    ] {
        assert!(
            rte.descendants()
                .any(|node| node.has_tag_name("SHORT-LABEL") && node.text() == Some(source)),
            "{source}"
        );
    }
    let implementation = xml
        .descendants()
        .find(|node| named(*node, "BSW-IMPLEMENTATION", "DetHostImplementation"))
        .unwrap();
    assert!(
        implementation
            .descendants()
            .any(|node| { node.has_tag_name("SHORT-LABEL") && node.text() == Some("Det_Host.c") })
    );
    let sections: BTreeSet<_> = implementation
        .descendants()
        .filter(|node| node.has_tag_name("MEMORY-SECTION"))
        .flat_map(|node| node.children().filter(|child| child.has_tag_name("SYMBOL")))
        .filter_map(|node| node.text())
        .collect();
    assert_eq!(
        sections,
        [
            "CODE",
            "VAR_CLEARED_UNSPECIFIED",
            "VAR_CLEARED_8",
            "ECU_DETRUNTIMECALLOUT_CODE"
        ]
        .into_iter()
        .collect()
    );
    assert!(
        !implementation
            .children()
            .any(|node| node.has_tag_name("REQUIRED-ARTIFACTS")),
        "Det uses its default Det_MemMap.h"
    );
    let prefix = implementation
        .descendants()
        .find(|node| named(*node, "SECTION-NAME-PREFIX", "DET"))
        .unwrap();
    assert!(
        !prefix
            .children()
            .any(|node| node.has_tag_name("IMPLEMENTED-IN-REF")),
        "the implicit default must not point at another module's header"
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
