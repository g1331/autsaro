//! RTE generation policy, connection semantics and independent source identities.
use super::single_component::rejected_at;
use super::*;
use autosar_config_core::integration::InputSource;

fn generation_option(sources: &mut [InputSource], value: &str) {
    xml_edit(
        sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-MODULE-CONFIGURATION-VALUES", "Rte"),
        |text| {
            let position = text.find("<DEFINITION-REF").unwrap();
            format!(
                "{}<ADMIN-DATA><SDGS><SDG GID=\"Autsaro.RteGeneration\"><SD GID=\"strictUnconnectedRPortCheck\">{value}</SD></SDG></SDGS></ADMIN-DATA>{}",
                &text[..position],
                &text[position..]
            )
        },
    );
}

fn disconnect(sources: &mut [InputSource], name: &str) {
    xml_edit(
        sources,
        "composition.arxml",
        |node| named(node, "ASSEMBLY-SW-CONNECTOR", name),
        |_| String::new(),
    );
}

#[test]
fn unconnected_ports_are_valid_and_strict_check_respects_the_port_exception() {
    // Verifies: SWS_Rte_01329, SWS_Rte_05099, SWS_Rte_03019
    for connector in ["IngressProcess", "Scalar"] {
        let mut sources = inputs();
        disconnect(&mut sources, connector);
        assert!(normal_validation(&sources).diagnostics.is_empty());
        assert!(build(&sources).is_ok());
        generation_option(&mut sources, "true");
        let port = if connector == "IngressProcess" {
            "/Application/Process/Value"
        } else {
            "/Application/Observe/ResultService"
        };
        rejected_at(
            &sources,
            "UNCONNECTED_R_PORT",
            if connector == "IngressProcess" {
                "process.arxml"
            } else {
                "observe.arxml"
            },
            port,
        );
        let (file, name) = if connector == "IngressProcess" {
            ("process.arxml", "Value")
        } else {
            ("observe.arxml", "ResultService")
        };
        xml_edit(
            &mut sources,
            file,
            |node| named(node, "R-PORT-PROTOTYPE", name),
            |text| {
                let position = text.find("<REQUIRED-INTERFACE-TREF").unwrap();
                format!(
                    "{}<MAY-BE-UNCONNECTED>true</MAY-BE-UNCONNECTED>{}",
                    &text[..position],
                    &text[position..]
                )
            },
        );
        assert!(normal_validation(&sources).diagnostics.is_empty());
        let plan = build(&sources).unwrap();
        assert!(
            plan.description()
                .composition
                .as_ref()
                .unwrap()
                .generation
                .strict_unconnected_r_port_check
        );
        let forward = plan.component_contract_files().unwrap();
        sources.reverse();
        assert_eq!(
            forward.files(),
            build(&sources)
                .unwrap()
                .component_contract_files()
                .unwrap()
                .files()
        );
    }
    let mut malformed = inputs();
    generation_option(&mut malformed, "invalid");
    rejected_at(
        &malformed,
        "RTE_GENERATION_OPTION",
        "ecuc.arxml",
        "/Configuration/Rte",
    );
    let mut duplicate = inputs();
    generation_option(&mut duplicate, "false");
    generation_option(&mut duplicate, "false");
    assert!(build(&duplicate).is_err());
}

#[test]
fn rte_generation_options_are_explicit_and_unsupported_values_are_refused() {
    // Verifies: ECUC_Rte_09000, ECUC_Rte_09009, ECUC_Rte_09061, ECUC_Rte_09087
    // Verifies: ECUC_Rte_09086, ECUC_Rte_09088, ECUC_Rte_09089
    // Rejects: ECUC_Rte_09007, ECUC_Rte_09008, ECUC_Rte_09085, ECUC_Rte_09010
    // Rejects: ECUC_Rte_09011, ECUC_Rte_09012, ECUC_Rte_09014, ECUC_Rte_09015
    // Rejects: ECUC_Rte_09113, ECUC_Rte_09126, ECUC_Rte_09163, ECUC_Rte_09062
    for (owner, name, invalid) in [
        ("Generation", "RteBypassSupport", "COMPONENT_WRAPPER"),
        ("Generation", "RteCalibrationSupport", "INITIALIZED_RAM"),
        ("Generation", "RteGenerationMode", "VENDOR_MODE"),
        ("Generation", "RteOptimizationMode", "MEMORY"),
        ("Generation", "RteDevErrorDetect", "true"),
        ("Generation", "RteDevErrorDetectUninit", "true"),
        ("Generation", "RteInExclusiveAreaCheckEnabled", "true"),
        ("Generation", "RteMeasurementSupport", "true"),
        ("Generation", "RteValueRangeCheckEnabled", "true"),
        ("Generation", "RteVfbTraceAnonymousClientEnabled", "true"),
        ("Generation", "RteVfbTraceEnabled", "true"),
        ("BswGeneral", "RteSchMVersionInfoApi", "true"),
        (
            "InitializeINIT",
            "RteInitializationStrategy",
            "RTE_INITIALIZATION_STRATEGY_NONE",
        ),
        (
            "InitializeCLEARED",
            "RteSectionInitializationPolicy",
            "POWER-ON-CLEARED",
        ),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, owner, name, invalid);
        rejected_at(
            &sources,
            "TARGET_PARAMETER_UNSUPPORTED",
            "ecuc.arxml",
            &format!("/Configuration/Rte/{owner}"),
        );
    }
    for name in [
        "Generation",
        "BswGeneral",
        "InitializeINIT",
        "InitializeCLEARED",
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| named(node, "ECUC-CONTAINER-VALUE", name),
            |_| String::new(),
        );
        if name.starts_with("Initialize") {
            // Each selected policy must be present even if the other container remains.
            assert!(build(&sources).is_err());
        } else {
            assert!(!normal_validation(&sources).diagnostics.is_empty());
        }
    }
}

fn description_with_revisions(revisions: &str) -> (String, String) {
    let mut sources = inputs();
    if !revisions.is_empty() {
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| named(node, "ECUC-MODULE-CONFIGURATION-VALUES", "Rte"),
            |text| {
                let position = text.find("<DEFINITION-REF").unwrap();
                format!(
                    "{}<ADMIN-DATA><DOC-REVISIONS>{revisions}</DOC-REVISIONS></ADMIN-DATA>{}",
                    &text[..position],
                    &text[position..]
                )
            },
        );
    }
    let files = prepared_pipeline_files(&build(&sources).unwrap());
    let text = |name: &str| {
        String::from_utf8(
            files
                .iter()
                .find(|(path, _)| path == name)
                .unwrap()
                .1
                .clone(),
        )
        .unwrap()
    };
    (
        text("generated/descriptions/Host_Implementation.arxml"),
        text("generated/descriptions/modules/Rte/README.md"),
    )
}

#[test]
fn rte_bswmd_uses_source_revision_and_describes_allocations_and_each_api_dependency() {
    // Verifies: SWS_Rte_05184, SWS_Rte_05185, SWS_Rte_05186
    // Verifies: SWS_Rte_05179, SWS_Rte_05180, SWS_Rte_05182, SWS_Rte_05183
    // Verifies: SWS_Rte_05087, SWS_Rte_05118, SWS_Rte_05137, SWS_Rte_05138
    let revision = |label: &str, date: &str| {
        format!(
            "<DOC-REVISION><REVISION-LABEL>{label}</REVISION-LABEL><DATE>{date}</DATE></DOC-REVISION>"
        )
    };
    for (revisions, expected, suffix) in [
        (String::new(), "0.0.0", None),
        (revision("3.12.45", "2026-10-16"), "3.12.45", None),
        (revision("4.12.45", "2026-10-16"), "4.12.45", None),
        (revision("3.13.45", "2026-10-16"), "3.13.45", None),
        (revision("3.12.46", "2026-10-16"), "3.12.46", None),
        (
            revision("3.12.45_rc1", "2026-10-16"),
            "3.12.45",
            Some("rc1"),
        ),
        (
            format!(
                "{}{}",
                revision("2.0.1", "2026-10-16T00:01:00+01:00"),
                revision("1.0.0", "2026-10-15T23:02:00Z")
            ),
            "1.0.0",
            None,
        ),
        (revision("+3.12.45", "2026-10-16"), "0.0.0", None),
        (revision("3.12", "2026-10-16"), "0.0.0", None),
        (
            format!(
                "{}{}",
                revision("3.0.0", "2026-10-16"),
                revision("4.0.0", "2026-10-16")
            ),
            "0.0.0",
            None,
        ),
        (
            format!(
                "{}{}",
                revision("3.0.0", "2026-02-29"),
                revision("4.0.0", "2026-10-16")
            ),
            "0.0.0",
            None,
        ),
    ] {
        let (xml, manual) = description_with_revisions(&revisions);
        assert!(manual.contains(&format!("Implementation version: `{expected}`")));
        assert!(
            manual.contains("RTE header software macros identify generator implementation 0.1.0")
        );
        assert!(manual.contains("both generator and input configuration"));
        let document = roxmltree::Document::parse(&xml).unwrap();
        let implementation = document
            .descendants()
            .find(|node| named(*node, "BSW-IMPLEMENTATION", "RteHostImplementation"))
            .unwrap();
        assert_eq!(
            implementation
                .children()
                .find(|node| node.has_tag_name("SW-VERSION"))
                .unwrap()
                .text(),
            Some(expected)
        );
        if let Some(suffix) = suffix {
            assert!(
                implementation
                    .descendants()
                    .any(|node| node.text().is_some_and(|text| text.contains(suffix)))
            );
        }
        assert!(
            implementation
                .descendants()
                .any(|node| node.has_tag_name("CATEGORY") && node.text() == Some("NONE"))
        );
        let rte = document
            .descendants()
            .find(|node| named(*node, "BSW-MODULE-DESCRIPTION", "Rte"))
            .unwrap();
        let area = rte
            .descendants()
            .find(|node| named(*node, "EXCLUSIVE-AREA", "SCHM_MODE"));
        assert!(area.is_some());
        for entity in rte
            .descendants()
            .filter(|node| node.has_tag_name("BSW-CALLED-ENTITY"))
        {
            let name = entity
                .children()
                .find(|node| node.has_tag_name("SHORT-NAME"))
                .unwrap()
                .text()
                .unwrap();
            let enters = entity
                .descendants()
                .any(|node| node.has_tag_name("EXCLUSIVE-AREA-REF"));
            assert_eq!(
                enters,
                name.starts_with("SchM_Mode_Dcm_") || name.starts_with("SchM_Switch_Dcm_"),
                "{name}"
            );
        }
        for (role, names) in [
            (
                "STATIC-MEMORYS",
                &[
                    "rte_started",
                    "rte_allocated",
                    "rte_value_0",
                    "rte_value_0_received",
                    "rte_value_0_expired",
                    "schm_lifecycle",
                    "diagnostic_session_mode",
                    "authentication_mode",
                ][..],
            ),
            (
                "CONSTANT-MEMORYS",
                &["Ecu_SchMConfig", "Rte_Instance_Ingress", "Rte_Inst_Ingress"][..],
            ),
        ] {
            let objects = rte
                .descendants()
                .find(|node| node.has_tag_name(role))
                .unwrap();
            for name in names {
                assert!(
                    objects
                        .descendants()
                        .any(|node| node.has_tag_name("SHORT-NAME") && node.text() == Some(name)),
                    "{role}: {name}"
                );
            }
        }
        for (role, module, name) in [
            ("REQUIRED-ENTRYS", "Com", "Com_ReceiveSignal"),
            ("REQUIRED-ENTRYS", "Com", "Com_SendSignal"),
            ("REQUIRED-ENTRYS", "Os", "GetCounterValue"),
            ("REQUIRED-ENTRYS", "Os", "GetElapsedValue"),
            ("EXPECTED-CALLBACKS", "Rte", "Rte_COMCbk"),
            ("EXPECTED-CALLBACKS", "Rte", "Rte_COMCbkRxTOut"),
        ] {
            let dependency = rte
                .descendants()
                .find(|node| named(*node, "BSW-MODULE-DEPENDENCY", &format!("{name}Dependency")))
                .unwrap();
            let relation = dependency
                .children()
                .find(|node| node.has_tag_name(role))
                .unwrap();
            assert!(
                relation
                    .descendants()
                    .any(|node| node.has_tag_name("BSW-MODULE-ENTRY-REF")
                        && node.text()
                            == Some(&format!("/AUTOSAR_{module}/BswModuleEntrys/{name}")))
            );
        }
    }
}

#[test]
fn all_rte_owned_input_types_have_one_declaration_and_external_emitters_stay_external() {
    // Verifies: SWS_Rte_01160, SWS_Rte_02648, SWS_Rte_06709, SWS_Rte_06710
    // Verifies: SWS_Rte_07104, SWS_Rte_07105, SWS_Rte_07111, SWS_Rte_07113
    let mut sources = inputs();
    replace_all(&mut sources, "/Types/uint32", "/Types/ValueType");
    xml_edit(
        &mut sources,
        "types.arxml",
        |node| named(node, "IMPLEMENTATION-DATA-TYPE", "uint32"),
        |text| {
            text.replace(
                "<SHORT-NAME>uint32</SHORT-NAME>",
                "<SHORT-NAME>ValueType</SHORT-NAME>",
            )
        },
    );
    xml_edit(
        &mut sources,
        "types.arxml",
        |node| named(node, "IMPLEMENTATION-DATA-TYPE", "uint8"),
        |text| {
            format!(
                "{text}{}",
                text.replace(
                    "<SHORT-NAME>uint8</SHORT-NAME>",
                    "<SHORT-NAME>UnusedByte</SHORT-NAME>"
                )
            )
        },
    );
    let plan = build(&sources).unwrap();
    let files = plan.component_contract_files().unwrap();
    let header = |name: &str| {
        std::str::from_utf8(
            &files
                .files()
                .iter()
                .find(|(path, _)| path == name)
                .unwrap()
                .1,
        )
        .unwrap()
    };
    let types = header("include/Rte_Type.h");
    // Verifies[Os]: SWS_BSW_00147.
    // Verifies: SWS_Os_00786, SWS_Os_00794.
    assert_eq!(types.matches("typedef uint32 CounterType;").count(), 1);
    assert_eq!(
        types
            .matches("typedef uint64 TimeInMicrosecondsType;")
            .count(),
        1
    );
    assert_eq!(types.matches("typedef uint32 ValueType;").count(), 1);
    assert!(types.contains("typedef uint8 UnusedByte;"));
    assert!(!types.contains("Rte_ModeType_"));
    assert!(!types.contains("#include \"Rte_Dcm_Type.h\""));
    assert!(header("include/Rte_Process_Type.h").contains("#include \"Rte_Dcm_Type.h\""));
    assert!(header("include/Rte_Process.h").contains("ValueType *data"));
    for reserved in ["CounterType", "TimeInMicrosecondsType"] {
        let mut wrong = sources.clone();
        xml_edit(
            &mut wrong,
            "types.arxml",
            |node| named(node, "IMPLEMENTATION-DATA-TYPE", "UnusedByte"),
            |text| text.replace("UnusedByte", reserved),
        );
        rejected_at(
            &wrong,
            "CONTRACT_NAME_COLLISION",
            "types.arxml",
            &format!("/Types/{reserved}"),
        );
    }
    let mut wrong = sources.clone();
    xml_edit(
        &mut wrong,
        "types.arxml",
        |node| named(node, "SW-BASE-TYPE", "Base_uint8"),
        |text| {
            text.replace(
                "<BASE-TYPE-SIZE>8</BASE-TYPE-SIZE>",
                "<BASE-TYPE-SIZE>16</BASE-TYPE-SIZE>",
            )
        },
    );
    assert!(build(&wrong).is_err());
}

#[test]
fn unsupported_completion_modes_and_foreign_application_type_owners_are_refused() {
    for name in ["strictInitialValuesCheck", "strictConfigurationCheck"] {
        for value in ["true", "false"] {
            let mut sources = inputs();
            generation_option(&mut sources, value);
            change(
                &mut sources,
                "ecuc.arxml",
                "strictUnconnectedRPortCheck",
                name,
            );
            if value == "true" {
                assert!(build(&sources).is_ok());
            } else {
                rejected_at(
                    &sources,
                    "RTE_GENERATION_OPTION",
                    "ecuc.arxml",
                    "/Configuration/Rte",
                );
            }
        }
    }
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "types.arxml",
        |node| named(node, "IMPLEMENTATION-DATA-TYPE", "uint32"),
        |text| {
            text.replace(
                "</IMPLEMENTATION-DATA-TYPE>",
                "<TYPE-EMITTER>External.h</TYPE-EMITTER></IMPLEMENTATION-DATA-TYPE>",
            )
        },
    );
    rejected_at(&sources, "TYPE_CONFLICT", "types.arxml", "/Types/uint32");
    let mut conflict = inputs();
    xml_edit(
        &mut conflict,
        "types.arxml",
        |node| {
            named(
                node,
                "IMPLEMENTATION-DATA-TYPE",
                "Dcm_DataElement_ApplicationValueType",
            )
        },
        |text| {
            format!(
                "{text}{}",
                text.replace(
                    "<SHORT-NAME>Dcm_DataElement_ApplicationValueType</SHORT-NAME>",
                    "<SHORT-NAME>Ingress_Periodic</SHORT-NAME>"
                )
            )
        },
    );
    rejected_at(
        &conflict,
        "SYMBOL_PRODUCER_DUPLICATE",
        "ingress.arxml",
        "/Application/Ingress/Behavior/Periodic",
    );
}

#[test]
fn generated_callback_values_use_explicit_function_addresses() {
    // MISRA AMD3.37 / R17.12: these interface slots hold addresses, not calls.
    let plan = build(&inputs()).unwrap();
    let files = prepared_pipeline_files(&plan);
    for (path, callbacks) in [
        (
            "generated/src/CanIf_PBcfg.c",
            &[
                "Ecu_HostBusSM_ControllerModeIndication",
                "Ecu_HostBusSM_ControllerBusOff",
            ][..],
        ),
        (
            "generated/src/LSduR_PBcfg.c",
            &[
                "PduR_CanIfRxIndication",
                "CanTp_RxIndication",
                "PduR_CanIfTxConfirmation",
                "PduR_CanIfTriggerTransmit",
                "CanTp_TxConfirmation",
            ][..],
        ),
        (
            "generated/src/PduR_PBcfg.c",
            &[
                "Com_RxIndication",
                "LSduR_PduRTransmit",
                "Com_TxConfirmation",
                "Com_TriggerTransmit",
                "CanTp_Transmit",
                "Dcm_StartOfReception",
                "Dcm_CopyRxData",
                "Dcm_TpRxIndication",
                "Dcm_CopyTxData",
                "Dcm_TpTxConfirmation",
            ][..],
        ),
        (
            "generated/src/Ecu_RuntimeConfig.c",
            &[
                "Rte_COMCbk",
                "Rte_COMCbkRxTOut",
                "PduR_ComTransmit",
                "DcmService_ReadData",
                "Ecu_DcmAuthenticationMode",
                "Ecu_HostBusSM_RequestComMode",
                "Ecu_HostBusSM_GetCurrentComMode",
                "Dcm_ComM_NoComModeEntered",
                "Dcm_ComM_SilentComModeEntered",
                "Dcm_ComM_FullComModeEntered",
                "Ecu_BswMAction",
                "Ecu_DetRuntimeCallout",
            ][..],
        ),
    ] {
        let source = files.iter().find(|(name, _)| name == path).unwrap();
        let source = std::str::from_utf8(&source.1).unwrap();
        for callback in callbacks {
            let mut values = 0;
            for (position, _) in source.match_indices(callback) {
                let following = source[position + callback.len()..].trim_start();
                if following.starts_with([',', '}']) {
                    values += 1;
                    assert!(
                        source[..position].trim_end().ends_with('&'),
                        "{path}: {callback}: {source}"
                    );
                }
            }
            assert!(values > 0, "{path}: missing callback value {callback}");
        }
    }
    let source = files
        .iter()
        .find(|(name, _)| name == "generated/src/Ecu_Config.c")
        .unwrap();
    assert!(
        std::str::from_utf8(&source.1)
            .unwrap()
            .contains(".entry = &OS_TASK_ENTRY(")
    );
}

#[test]
fn generated_lifecycle_sections_contain_no_other_preprocessing() {
    // Verifies: SWS_MemMap_00003.
    let plan = build(&inputs()).unwrap();
    let files = prepared_pipeline_files(&plan);
    for (name, prefix, segment, owned) in [
        (
            "SchM_Main.c",
            "RTE_Rte",
            "VAR_CLEARED_UNSPECIFIED",
            "schm_lifecycle",
        ),
        (
            "Ecu_RuntimeConfig.c",
            "RTE_Ecu",
            "CODE",
            "Ecu_StackBootstrap",
        ),
    ] {
        let path = format!("generated/src/{name}");
        let source = files
            .iter()
            .find(|(file, _)| file == &path)
            .map(|(_, bytes)| std::str::from_utf8(bytes).unwrap())
            .unwrap();
        let start = format!("#define {prefix}_START_SEC_{segment}\n#include \"Rte_MemMap.h\"\n");
        let stop = format!("#define {prefix}_STOP_SEC_{segment}");
        let bodies = source
            .split(&start)
            .skip(1)
            .map(|tail| tail.split_once(&stop).unwrap().0)
            .collect::<Vec<_>>();
        assert!(bodies.iter().any(|body| body.contains(owned)), "{path}");
        for body in bodies {
            assert!(
                body.lines().all(|line| !line.trim_start().starts_with('#')),
                "{path}: {body}"
            );
        }
    }
}

#[test]
fn task_memory_mapping_and_generator_option_scope_are_checked_at_the_source() {
    // Verifies: SWS_Rte_08923, SWS_Rte_04557
    let mut wrong = inputs();
    replace_all(
        &mut wrong,
        "/Configuration/CODE",
        "/Configuration/OTHER_CODE",
    );
    xml_edit(
        &mut wrong,
        "ecuc.arxml",
        |node| named(node, "SW-ADDR-METHOD", "CODE"),
        |text| {
            text.replace(
                "<SHORT-NAME>CODE</SHORT-NAME>",
                "<SHORT-NAME>OTHER_CODE</SHORT-NAME>",
            )
        },
    );
    rejected_at(
        &wrong,
        "RTE_OS_MEMORY_METHOD",
        "ecuc.arxml",
        "/Configuration/Os/Task_Ecu",
    );
    let mut missing = inputs();
    xml_edit(
        &mut missing,
        "ecuc.arxml",
        |node| {
            node.has_tag_name("ECUC-REFERENCE-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/OsMemoryMappingCodeLocationRef"))
                })
        },
        |_| String::new(),
    );
    rejected_at(
        &missing,
        "CONFIGURATION_REFERENCE",
        "ecuc.arxml",
        "/Configuration/Os/Task_Ecu",
    );
    let mut misplaced = inputs();
    generation_option(&mut misplaced, "true");
    let admin = "<ADMIN-DATA><SDGS><SDG GID=\"Autsaro.RteGeneration\"><SD GID=\"strictUnconnectedRPortCheck\">true</SD></SDG></SDGS></ADMIN-DATA>";
    xml_edit(
        &mut misplaced,
        "ecuc.arxml",
        |node| node.has_tag_name("ADMIN-DATA"),
        |_| String::new(),
    );
    xml_edit(
        &mut misplaced,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "Generation"),
        |text| {
            let position = text.find("<DEFINITION-REF").unwrap();
            format!("{}{admin}{}", &text[..position], &text[position..])
        },
    );
    rejected_at(
        &misplaced,
        "RTE_GENERATION_OPTION",
        "ecuc.arxml",
        "/Configuration/Rte/Generation",
    );
}

#[test]
fn disabled_rips_and_unconsumed_toolchain_identifier_options_are_refused() {
    // Rejects: ECUC_Rte_89000, ECUC_Rte_09013
    let mut rips = inputs();
    xml_edit(
        &mut rips,
        "ecuc.arxml",
        |node| named(node, "ECUC-MODULE-CONFIGURATION-VALUES", "Rte"),
        |text| {
            let position = text.rfind("</CONTAINERS>").unwrap();
            format!(
                "{}<ECUC-CONTAINER-VALUE><SHORT-NAME>Rips</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">/AUTOSAR/EcucDefs/Rte/RteRips</DEFINITION-REF><PARAMETER-VALUES><ECUC-TEXTUAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-ENUMERATION-PARAM-DEF\">/AUTOSAR/EcucDefs/Rte/RteRips/RteRipsSupport</DEFINITION-REF><VALUE>RTE_RIPS_ON</VALUE></ECUC-TEXTUAL-PARAM-VALUE></PARAMETER-VALUES></ECUC-CONTAINER-VALUE>{}",
                &text[..position],
                &text[position..]
            )
        },
    );
    let issues = build(&rips).err().unwrap();
    assert!(
        issues
            .iter()
            .any(|issue| issue.file.as_deref() == Some("ecuc.arxml")
                && issue.object.as_deref() == Some("/Configuration/Rte/Rips")),
        "{issues:?}"
    );
    let mut significant = inputs();
    xml_edit(
        &mut significant,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "Generation"),
        |text| {
            text.replace("</PARAMETER-VALUES>", "<ECUC-NUMERICAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-INTEGER-PARAM-DEF\">/AUTOSAR/EcucDefs/Rte/RteGeneration/RteToolChainSignificantCharacters</DEFINITION-REF><VALUE>31</VALUE></ECUC-NUMERICAL-PARAM-VALUE></PARAMETER-VALUES>")
        },
    );
    rejected_at(
        &significant,
        "TARGET_PARAMETER_UNSUPPORTED",
        "ecuc.arxml",
        "/Configuration/Rte/Generation",
    );
}

#[test]
fn generated_namespace_rejects_public_header_type_and_macro_collisions() {
    // Verifies: SWS_Rte_05149, SWS_Rte_01003, SWS_Rte_07122, SWS_Rte_07922
    for name in ["float32", "VoidPtr", "uint8_least", "RTE_RUNNABLE_Periodic"] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "types.arxml",
            |node| named(node, "IMPLEMENTATION-DATA-TYPE", "uint32"),
            |text| {
                format!(
                    "{text}{}",
                    text.replace(
                        "<SHORT-NAME>uint32</SHORT-NAME>",
                        &format!("<SHORT-NAME>{name}</SHORT-NAME>")
                    )
                )
            },
        );
        rejected_at(
            &sources,
            "CONTRACT_NAME_COLLISION",
            "types.arxml",
            &format!("/Types/{name}"),
        );
    }
    let mut runnable = inputs();
    change(
        &mut runnable,
        "ingress.arxml",
        "<SYMBOL>Ingress_Periodic</SYMBOL>",
        "<SYMBOL>float32</SYMBOL>",
    );
    rejected_at(
        &runnable,
        "SYMBOL_PRODUCER_DUPLICATE",
        "ingress.arxml",
        "/Application/Ingress/Behavior/Periodic",
    );
    for (before, after) in [("Ingress", "DataHandleType"), ("Ingress", "Process_Type")] {
        let mut sources = inputs();
        replace_all(
            &mut sources,
            &format!("/Application/{before}"),
            &format!("/Application/{after}"),
        );
        change(
            &mut sources,
            "ingress.arxml",
            &format!("<SHORT-NAME>{before}</SHORT-NAME>"),
            &format!("<SHORT-NAME>{after}</SHORT-NAME>"),
        );
        rejected_at(
            &sources,
            "CONTRACT_NAME_COLLISION",
            if after == "Process_Type" {
                "process.arxml"
            } else {
                "ingress.arxml"
            },
            if after == "Process_Type" {
                "/Application/Process"
            } else {
                "/Application/DataHandleType"
            },
        );
    }
}

#[test]
fn input_types_have_consistent_trusted_emitters_and_required_initial_values() {
    // Verifies: SWS_Rte_04632, SWS_Rte_06712, SWS_Rte_03893, SWS_Rte_CONSTR_09097
    // Verifies: SWS_Rte_06711, SWS_Rte_07642
    for name in ["Rte_UserType", "SchM_UserType"] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "types.arxml",
            |node| named(node, "IMPLEMENTATION-DATA-TYPE", "uint32"),
            |text| {
                format!(
                    "{text}{}",
                    text.replace(
                        "<SHORT-NAME>uint32</SHORT-NAME>",
                        &format!("<SHORT-NAME>{name}</SHORT-NAME>")
                    )
                )
            },
        );
        rejected_at(
            &sources,
            "RTE_TYPE_UNSUPPORTED",
            "types.arxml",
            &format!("/Types/{name}"),
        );
    }
    let mut missing = inputs();
    change(
        &mut missing,
        "types.arxml",
        "<NATIVE-DECLARATION>uint32</NATIVE-DECLARATION>",
        "",
    );
    assert!(build(&missing).is_err());
    let mut missing = inputs();
    xml_edit(
        &mut missing,
        "process.arxml",
        |node| node.has_tag_name("INIT-VALUE"),
        |_| String::new(),
    );
    rejected_at(
        &missing,
        "INIT_VALUE_REQUIRED",
        "process.arxml",
        "/Application/Process/Value",
    );
    let mut extra = inputs();
    xml_edit(
        &mut extra,
        "observe.arxml",
        |node| named(node, "APPLICATION-SW-COMPONENT-TYPE", "Observe"),
        |text| {
            format!(
                "{text}{}",
                text.replace(
                    "<SHORT-NAME>Observe</SHORT-NAME>",
                    "<SHORT-NAME>Unused</SHORT-NAME>"
                )
            )
        },
    );
    rejected_at(
        &extra,
        "UNINSTANTIATED_COMPONENT",
        "observe.arxml",
        "/Application/Unused",
    );
}

#[test]
fn private_rte_memory_sections_have_real_consumers_and_only_used_aliases() {
    let files = prepared_pipeline_files(&build(&inputs()).unwrap());
    let mapping = files
        .iter()
        .find(|(path, _)| path == "generated/include/Rte_MemMap.h")
        .map(|(_, bytes)| std::str::from_utf8(bytes).unwrap())
        .unwrap();
    for alias in [
        "RTE_CONST",
        "RTE_VAR_INIT",
        "RTE_VAR_CLEARED",
        "RTE_CONFIG_DATA_PREBUILD",
    ] {
        assert!(!mapping.contains(&format!("#define {alias} ")), "{alias}");
        assert!(!mapping.contains(&format!("#undef {alias}\n")), "{alias}");
    }
    assert_eq!(mapping.matches("#define RTE_CODE ").count(), 1);
    assert_eq!(mapping.matches("#undef RTE_CODE\n").count(), 1);
    let modes = files
        .iter()
        .find(|(path, _)| path == "generated/src/SchM_Main.c")
        .unwrap();
    assert!(
        std::str::from_utf8(&modes.1)
            .unwrap()
            .contains("RTE_CODE Std_ReturnType")
    );
    let mut used = std::collections::BTreeSet::new();
    for (path, bytes) in &files {
        if !path.ends_with(".c") && !path.ends_with(".h") {
            continue;
        }
        let source = std::str::from_utf8(bytes).unwrap();
        for (index, line) in source.lines().enumerate() {
            if let Some(keyword) = line.strip_prefix("#define RTE_")
                && keyword.contains("_START_SEC_")
            {
                used.insert(keyword.to_owned());
                let next = source.lines().nth(index + 2).unwrap();
                assert!(
                    !next.starts_with("#define RTE_"),
                    "empty section in {path}: {keyword}"
                );
            }
        }
    }
    let described: std::collections::BTreeSet<_> = mapping
        .lines()
        .filter_map(|line| line.split("defined(RTE_").nth(1))
        .filter_map(|line| line.strip_suffix(')'))
        .filter(|keyword| keyword.contains("_START_SEC_"))
        .map(str::to_owned)
        .collect();
    assert_eq!(described, used);
}

#[test]
fn version_assets_reserve_public_files_guards_and_macros_at_the_source_object() {
    // Verifies: SWS_Rte_01126, SWS_Rte_06713
    // Literal names independently cover the public version producer namespace.
    for (module, prefix) in [
        ("Can", "CAN"),
        ("CanIf", "CANIF"),
        ("CanTp", "CANTP"),
        ("Com", "COM"),
        ("ComM", "COMM"),
        ("BswM", "BSWM"),
        ("Dcm", "DCM"),
        ("Det", "DET"),
        ("PduR", "PDUR"),
        ("LSduR", "LSDUR"),
        ("Rte", "RTE"),
        ("MemMap", "MEMMAP"),
        ("ComStack", "COMSTACKTYPE"),
        ("Os", "OS"),
        ("Arti", "ARTI"),
    ] {
        let mut names = vec![format!("{}_VERSION_H", module.to_uppercase())];
        for part in ["MAJOR", "MINOR", "REVISION"] {
            names.push(format!("{prefix}_AR_RELEASE_{part}_VERSION"));
        }
        for part in ["MAJOR", "MINOR", "PATCH"] {
            names.push(format!("{prefix}_SW_{part}_VERSION"));
        }
        if module == "Arti" {
            names.push("ARTI_MODULE_ID".to_owned());
        }
        for name in names {
            let mut sources = inputs();
            change(
                &mut sources,
                "ingress.arxml",
                "<SYMBOL>Ingress_Periodic</SYMBOL>",
                &format!("<SYMBOL>{name}</SYMBOL>"),
            );
            rejected_at(
                &sources,
                "SYMBOL_PRODUCER_DUPLICATE",
                "ingress.arxml",
                "/Application/Ingress/Behavior/Periodic",
            );
        }
    }
    for name in [
        "AUTSARO_VERSION_H",
        "AUTSARO_SW_MAJOR_VERSION",
        "AUTSARO_SW_MINOR_VERSION",
        "AUTSARO_SW_PATCH_VERSION",
        "STD_TYPES_SW_MAJOR_VERSION",
        "STD_TYPES_SW_MINOR_VERSION",
        "STD_TYPES_SW_PATCH_VERSION",
        "PLATFORM_SW_MAJOR_VERSION",
        "PLATFORM_SW_MINOR_VERSION",
        "PLATFORM_SW_PATCH_VERSION",
    ] {
        let mut sources = inputs();
        change(
            &mut sources,
            "ingress.arxml",
            "<SYMBOL>Ingress_Periodic</SYMBOL>",
            &format!("<SYMBOL>{name}</SYMBOL>"),
        );
        rejected_at(
            &sources,
            "SYMBOL_PRODUCER_DUPLICATE",
            "ingress.arxml",
            "/Application/Ingress/Behavior/Periodic",
        );
    }
    for name in ["Version", "VERSION"] {
        let mut sources = inputs();
        replace_all(
            &mut sources,
            "/Application/Ingress",
            &format!("/Application/{name}"),
        );
        change(
            &mut sources,
            "ingress.arxml",
            "<SHORT-NAME>Ingress</SHORT-NAME>",
            &format!("<SHORT-NAME>{name}</SHORT-NAME>"),
        );
        rejected_at(
            &sources,
            "CONTRACT_NAME_COLLISION",
            "ingress.arxml",
            &format!("/Application/{name}"),
        );
    }
    let mut sources = inputs();
    change(
        &mut sources,
        "ingress.arxml",
        "<SYMBOL>Ingress_Periodic</SYMBOL>",
        "<SYMBOL>Control_ReleaseVersion</SYMBOL>",
    );
    let plan = build(&sources).unwrap();
    let contract = plan.component_contract_files().unwrap();
    let mut paths = std::collections::BTreeSet::new();
    for (path, _) in contract.files() {
        assert!(
            paths.insert(path.to_uppercase()),
            "duplicate producer: {path}"
        );
    }
    assert!(contract.files().iter().any(|(_, bytes)| {
        std::str::from_utf8(bytes)
            .unwrap()
            .contains("Control_ReleaseVersion")
    }));
}
