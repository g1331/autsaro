//! OS counters, scheduling and status configuration of the mainline.
use super::*;
use autosar_config_core::integration::InputSource;
use autosar_config_core::target::BuildTarget;
use std::path::Path;

fn codes(sources: &[InputSource]) -> Vec<String> {
    build(sources)
        .err()
        .expect("rejected input was accepted")
        .into_iter()
        .map(|issue| issue.code)
        .collect()
}

/// The generator's report of OS-internal timers and the counter profile
/// accepted by the selected controlled-time target: one logical millisecond
/// per tick (ECUC OsSecondsPerTick 0.001), base 1, minimum cycle 1.
/// Verifies: SWS_Os_00370
#[test]
fn counter_timing_report_and_profile() {
    for (renamed, maximum) in [(false, 65535u32), (true, 65535), (false, 4095)] {
        let sources = counter_variant(renamed, maximum);
        let plan = build(&sources).unwrap();
        let schedule = &plan.description().schedule;
        assert_eq!(schedule.counter_tick_ms, 1);
        assert_eq!(schedule.counter_ticks_per_base, 1);
        assert_eq!(schedule.counter_minimum_cycle, 1);
        let scratch = Scratch::new();
        let (_, files) = project(&sources, &scratch.0, BuildTarget::LinuxX64ControlledV1);
        let report: serde_json::Value =
            serde_json::from_str(file(&files, "meta/os-generation-timing.json")).unwrap();
        let counter = if renamed {
            "RenamedCounter"
        } else {
            "SystemCounter"
        };
        assert_eq!(report["counter"]["name"], counter);
        assert_eq!(report["counter"]["tickNanoseconds"], 1_000_000);
        assert_eq!(report["counter"]["maximum"], maximum);
        assert_eq!(report["internalPeriodicTimers"], serde_json::json!([]));
        assert_eq!(report["kernelSoftwareTimers"]["enabled"], false);
        assert_eq!(report["kernelSoftwareTimers"]["configUSE_TIMERS"], 0);
        assert_eq!(report["kernelTick"]["periodicHostThread"], false);
        assert_eq!(report["kernelTick"]["logicalMillisecondsPerRequest"], 1);
        assert_eq!(
            report["hostTimeouts"][0]["rangeMilliseconds"],
            serde_json::json!([1, 5000])
        );
        assert_eq!(report["hostTimeouts"][0]["advancesAutomotiveTime"], false);
        assert_eq!(report["hostTimeouts"][1]["limitMilliseconds"], 5000);
        assert_eq!(report["hostTimeouts"][1]["advancesAutomotiveTime"], false);
        let header = file(&files, "generated/include/Os_Counter.h");
        assert!(header.contains(&format!("OS_TICKS2NS_{counter}(ticks)")));
        if renamed {
            assert!(!header.contains("SystemCounter"), "{header}");
        }
    }
    for resolution in ["0.002", "0.0005", "0"] {
        let mut sources = single_component_sources();
        parameter(
            &mut sources,
            "SystemCounter",
            "OsSecondsPerTick",
            resolution,
        );
        assert!(
            codes(&sources).iter().any(|code| code == "COUNTER_PROFILE"),
            "{resolution}"
        );
    }
}

/// OsStatus selects the delivered `OS_STATUS_EXTENDED` guard; literals outside
/// the ECUC enumeration (case-sensitive) are refused before generation.
#[test]
fn status_guard_and_rejections() {
    for (status, define) in [
        ("STANDARD", "#define OS_STATUS_EXTENDED 0"),
        ("EXTENDED", "#define OS_STATUS_EXTENDED 1"),
    ] {
        let mut sources = single_component_sources();
        parameter(&mut sources, "OperatingSystem", "OsStatus", status);
        let scratch = Scratch::new();
        let (_, files) = project(&sources, &scratch.0, BuildTarget::LinuxX64ControlledV1);
        assert!(
            file(&files, "generated/include/Os_Cfg.h").contains(define),
            "{status}"
        );
    }
    for status in ["UNKNOWN", "standard"] {
        let mut sources = single_component_sources();
        parameter(&mut sources, "OperatingSystem", "OsStatus", status);
        rejects_in_both(&sources, "TARGET_PARAMETER_UNSUPPORTED");
    }
}

/// OsUseGetServiceId and OsUseParameterAccess select whether the delivered
/// `Os_Cfg.h` provides the ErrorHook access macros.
// Verifies: SWS_Os_00439
#[test]
fn error_hook_access_switches_follow_the_configuration() {
    for (service_id, parameter_access) in [("false", "true"), ("true", "false")] {
        let mut sources = single_component_sources();
        parameter(
            &mut sources,
            "OperatingSystem",
            "OsUseGetServiceId",
            service_id,
        );
        parameter(
            &mut sources,
            "OperatingSystem",
            "OsUseParameterAccess",
            parameter_access,
        );
        let scratch = Scratch::new();
        let (_, files) = project(&sources, &scratch.0, BuildTarget::LinuxX64ControlledV1);
        let header = file(&files, "generated/include/Os_Cfg.h");
        for (name, value) in [
            ("OS_USE_GET_SERVICE_ID", service_id),
            ("OS_USE_PARAMETER_ACCESS", parameter_access),
        ] {
            let define = format!("#define {name} {}", u8::from(value == "true"));
            assert!(header.contains(&define), "{define}\n{header}");
        }
    }
}

fn os_parameter(node: roxmltree::Node<'_, '_>, definition: &str) -> bool {
    (node.has_tag_name("ECUC-NUMERICAL-PARAM-VALUE")
        || node.has_tag_name("ECUC-TEXTUAL-PARAM-VALUE"))
        && node.children().any(|child| {
            child.has_tag_name("DEFINITION-REF")
                && child.text().is_some_and(|text| {
                    text.starts_with("/AUTOSAR/EcucDefs/Os/")
                        && text.ends_with(&format!("/{definition}"))
                })
        })
}

fn edit_os_parameter(
    sources: &mut [InputSource],
    definition: &str,
    replacement: impl Fn(&str) -> String,
) {
    xml_edit(
        sources,
        "ecuc.arxml",
        |node| os_parameter(node, definition),
        replacement,
    );
}

/// Boolean literal forms of OsUseResScheduler and the OS configuration
/// parameters the selected SC1 kernel requires (ECUC Os module definition).
#[test]
fn scheduler_configuration_rejections() {
    for (value, resource) in [("false", false), ("true", false), ("0", false), ("1", true)] {
        build(&scheduler_variant(value, resource))
            .unwrap_or_else(|issues| panic!("{value}: {issues:?}"));
    }
    // With OsUseResScheduler FALSE an explicit RES_SCHEDULER is an ordinary
    // resource (SWS_Os_00854), which the selected profile does not accept.
    assert_rejected(&scheduler_variant("0", true), "OS_RESOURCE_PROFILE");
    // XSD boolean literals are case-sensitive; other integers are outside it.
    let mut upper = single_component_sources();
    parameter(&mut upper, "OperatingSystem", "OsUseResScheduler", "TRUE");
    assert_rejected(&upper, "NATIVE_UNSUPPORTED");
    let mut two = single_component_sources();
    parameter(&mut two, "OperatingSystem", "OsUseResScheduler", "2");
    assert_rejected(&two, "OS_CONFIGURATION");
    let mut missing = single_component_sources();
    edit_os_parameter(&mut missing, "OsUseResScheduler", |_| String::new());
    assert_rejected(&missing, "OS_CONFIGURATION");
    let mut duplicated = single_component_sources();
    edit_os_parameter(&mut duplicated, "OsUseResScheduler", |text| text.repeat(2));
    assert_rejected(&duplicated, "OS_CONFIGURATION");
    let wrong_definition = replaced(
        &single_component_sources(),
        "/Os/OsOS/OsUseResScheduler<",
        "/Os/OsOS/OsHooks/OsUseResScheduler<",
    );
    assert_rejected(&wrong_definition, "REFERENCE_UNRESOLVED");
    // A valid OsOS definition reference placed inside the OsHooks container.
    let mut misplaced = single_component_sources();
    let mut moved = String::new();
    {
        let text = std::str::from_utf8(
            misplaced
                .iter()
                .find(|source| source.logical_path() == "ecuc.arxml")
                .unwrap()
                .bytes(),
        )
        .unwrap();
        let document = roxmltree::Document::parse(text).unwrap();
        let node = document
            .descendants()
            .find(|node| os_parameter(*node, "OsUseResScheduler"))
            .unwrap();
        moved.push_str(&text[node.range()]);
    }
    edit_os_parameter(&mut misplaced, "OsUseResScheduler", |_| String::new());
    edit_os_parameter(&mut misplaced, "OsStartupHook", |text| {
        format!("{moved}{text}")
    });
    assert_rejected(&misplaced, "DEFINITION_PARENT");
    assert_rejected(&misplaced, "OS_CONFIGURATION");
    for name in [
        "OsStatus",
        "OsScalabilityClass",
        "OsUseGetServiceId",
        "OsUseParameterAccess",
        "OsLockTrustedFunctionCall",
        "OsStackMonitoring",
        "OsUseArti",
        "OsStartupHook",
        "OsErrorHook",
    ] {
        let mut sources = single_component_sources();
        edit_os_parameter(&mut sources, name, |_| String::new());
        assert_rejected(&sources, "OS_CONFIGURATION");
    }
    for parameter_name in ["OsUseResScheduler", "OsUseGetServiceId", "OsErrorHook"] {
        let definition = format!(
            r#"<ECUC-MODULE-DEF><SHORT-NAME>VendorOs</SHORT-NAME><LOWER-MULTIPLICITY>0</LOWER-MULTIPLICITY><UPPER-MULTIPLICITY>1</UPPER-MULTIPLICITY><POST-BUILD-VARIANT-SUPPORT>false</POST-BUILD-VARIANT-SUPPORT><SUPPORTED-CONFIG-VARIANTS><SUPPORTED-CONFIG-VARIANT>VARIANT-PRE-COMPILE</SUPPORTED-CONFIG-VARIANT></SUPPORTED-CONFIG-VARIANTS><CONTAINERS><ECUC-PARAM-CONF-CONTAINER-DEF><SHORT-NAME>OsOS</SHORT-NAME><LOWER-MULTIPLICITY>1</LOWER-MULTIPLICITY><UPPER-MULTIPLICITY>1</UPPER-MULTIPLICITY><ORIGIN>Vendor</ORIGIN><PARAMETERS><ECUC-BOOLEAN-PARAM-DEF><SHORT-NAME>{parameter_name}</SHORT-NAME><LOWER-MULTIPLICITY>1</LOWER-MULTIPLICITY><UPPER-MULTIPLICITY>1</UPPER-MULTIPLICITY><SCOPE>LOCAL</SCOPE><ORIGIN>Vendor</ORIGIN><POST-BUILD-VARIANT-VALUE>false</POST-BUILD-VARIANT-VALUE><VALUE-CONFIG-CLASSES><ECUC-VALUE-CONFIGURATION-CLASS><CONFIG-CLASS>PRE-COMPILE</CONFIG-CLASS><CONFIG-VARIANT>VARIANT-PRE-COMPILE</CONFIG-VARIANT></ECUC-VALUE-CONFIGURATION-CLASS></VALUE-CONFIG-CLASSES><SYMBOLIC-NAME-VALUE>false</SYMBOLIC-NAME-VALUE><DEFAULT-VALUE>true</DEFAULT-VALUE></ECUC-BOOLEAN-PARAM-DEF></PARAMETERS></ECUC-PARAM-CONF-CONTAINER-DEF></CONTAINERS></ECUC-MODULE-DEF>"#
        );
        let mut added = single_component_sources();
        change(
            &mut added,
            "ecuc.arxml",
            "</ELEMENTS>",
            &format!("{definition}</ELEMENTS>"),
        );
        // A vendor module with the same parameter names is retained unchanged.
        build(&added).unwrap_or_else(|issues| panic!("{parameter_name}: {issues:?}"));
        let official = format!(
            "/AUTOSAR/EcucDefs/Os/OsOS/{}{parameter_name}<",
            if parameter_name == "OsErrorHook" {
                "OsHooks/"
            } else {
                ""
            }
        );
        let aliased = replaced(
            &added,
            &official,
            &format!("/Configuration/VendorOs/OsOS/{parameter_name}<"),
        );
        // A vendor definition cannot stand in for the standard OS policy. The
        // normal (native) catalog refuses the foreign definition reference;
        // the official MOD path reports OS_CONFIGURATION for the same input.
        assert_rejected(&aliased, "NATIVE_UNSUPPORTED");
    }
}

fn assert_rejected(sources: &[InputSource], code: &str) {
    let actual = codes(sources);
    assert!(
        actual.iter().any(|actual| actual == code),
        "{code}: {actual:?}"
    );
}

/// Os_Service.arxml: the OS counter service SWC description (IS-SERVICE
/// OPERATING-SYSTEM; GetCounterValue/GetElapsedValue with
/// TimeInMicrosecondsType arguments and the E_OS_ID/E_OS_VALUE errors of
/// SWS_Os_00377/SWS_Os_00381/SWS_Os_00391).
// Verifies: SWS_Os_00560, 91027
#[test]
fn counter_service_description() {
    for counter in ["SystemCounter", "AuxClock"] {
        let mut sources = single_component_sources();
        parameter(
            &mut sources,
            "SystemCounter",
            "OsCounterMaxAllowedValue",
            "15",
        );
        if counter != "SystemCounter" {
            sources = replaced(&sources, "SystemCounter", counter);
        }
        let scratch = Scratch::new();
        let (_, files) = project(&sources, &scratch.0, BuildTarget::LinuxX64ControlledV1);
        let xml = file(&files, "generated/descriptions/Os_Service.arxml");
        let doc = roxmltree::Document::parse(xml).unwrap();
        let text = |tag: &str| {
            doc.descendants()
                .filter(|node| node.has_tag_name(tag))
                .filter_map(|node| node.text())
                .collect::<Vec<_>>()
        };
        assert_eq!(text("IS-SERVICE"), ["true"]);
        assert_eq!(text("SERVICE-KIND"), ["OPERATING-SYSTEM"]);
        assert_eq!(text("DIRECTION"), ["OUT", "INOUT", "OUT"]);
        assert_eq!(text("TYPE-TREF"), ["/OsServices/TimeInMicrosecondsType"; 3]);
        assert_eq!(text("VALUE-TYPE-TREF"), ["/OsServices/CounterType"]);
        assert_eq!(
            text("PROVIDED-INTERFACE-TREF"),
            [format!("/OsServices/OsService_{counter}")]
        );
        assert_eq!(text("PORT-REF"), ["/OsServices/Os/OsService"]);
        assert_eq!(
            text("POSSIBLE-ERROR-REF"),
            [
                format!("/OsServices/OsService_{counter}/E_OS_ID"),
                format!("/OsServices/OsService_{counter}/E_OS_ID"),
                format!("/OsServices/OsService_{counter}/E_OS_VALUE"),
            ]
        );
        assert_eq!(text("BASE-TYPE-SIZE"), ["32", "64"]);
        assert_eq!(text("ERROR-CODE"), ["1", "3", "7", "8"]);
        assert_eq!(text("VALUE"), ["0"]);
        assert_eq!(
            text("SYMBOL"),
            [
                format!("OsService_{counter}_GetCounterValue"),
                format!("OsService_{counter}_GetElapsedValue")
            ]
        );
        for reference in doc
            .descendants()
            .filter(|node| node.is_element() && node.attribute("DEST").is_some())
        {
            let path = reference.text().unwrap();
            let matches = doc
                .descendants()
                .filter(|node| node.has_tag_name(reference.attribute("DEST").unwrap()))
                .filter(|target| {
                    let names = target
                        .ancestors()
                        .filter_map(|parent| {
                            parent
                                .children()
                                .find(|child| child.has_tag_name("SHORT-NAME"))
                                .and_then(|child| child.text())
                        })
                        .collect::<Vec<_>>();
                    format!("/{}", names.into_iter().rev().collect::<Vec<_>>().join("/")) == path
                })
                .count();
            assert_eq!(matches, 1, "unresolved or ambiguous reference: {path}");
        }
    }
    // The generated service producers own these external names (os_service.rs);
    // a runnable or component may not shadow them.
    for symbol in [
        "OsService_SystemCounter_GetCounterValue",
        "OsService_SystemCounter_GetElapsedValue",
        "Rte_Call_OsService_GetCounterValue",
    ] {
        let sources = replaced(
            &single_component_sources(),
            "<SYMBOL>Ingress_Periodic</SYMBOL>",
            &format!("<SYMBOL>{symbol}</SYMBOL>"),
        );
        rejects_in_both(&sources, "SYMBOL_PRODUCER_DUPLICATE");
    }
    let component = replaced(&single_component_sources(), "Ingress", "Os");
    assert_rejected(&component, "CONTRACT_NAME_COLLISION");
}

/// The selected kernel drives schedule tables only as repeating, implicitly
/// unsynchronized tables started at a first expiry point of offset 0, owned
/// by the mapped task, on the software SystemCounter.
#[test]
fn schedule_table_rejections() {
    let sources = schedule_table_variant();
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert!(
        plan.description()
            .schedule
            .entities
            .iter()
            .all(|entity| entity.alarm.is_empty() && entity.expiry_point.is_some())
    );
    // Each table no longer yields one fixed period/phase per OS event.
    for (name, before, after, code) in [
        (
            "not-repeating",
            "OsScheduleTableRepeating</DEFINITION-REF><VALUE>true</VALUE>",
            "OsScheduleTableRepeating</DEFINITION-REF><VALUE>false</VALUE>",
            "PERIOD_SCHEDULE_TABLE_CONFLICT",
        ),
        (
            "wrong-first-offset",
            "OsScheduleTblExpPointOffset</DEFINITION-REF><VALUE>0</VALUE>",
            "OsScheduleTblExpPointOffset</DEFINITION-REF><VALUE>1</VALUE>",
            "PERIOD_SCHEDULE_TABLE_CONFLICT",
        ),
        (
            "wrong-owner",
            "OsScheduleTableSetEventTaskRef</DEFINITION-REF><VALUE-REF DEST=\"ECUC-CONTAINER-VALUE\">/Configuration/Os/Task_Ecu<",
            "OsScheduleTableSetEventTaskRef</DEFINITION-REF><VALUE-REF DEST=\"ECUC-CONTAINER-VALUE\">/Configuration/Os/SystemCounter<",
            "REFERENCE_TARGET",
        ),
    ] {
        let mut changed = sources.clone();
        change(&mut changed, "ecuc.arxml", before, after);
        assert!(build(&changed).is_err(), "accepted {name}");
        rejects_in_both(&changed, code);
    }
    let mut hardware = sources.clone();
    parameter(&mut hardware, "SystemCounter", "OsCounterType", "HARDWARE");
    rejects_in_both(&hardware, "COUNTER_PROFILE");
}

/// CanIf PDU handles stay inside the selected handle width, and
/// RteBswPositionInTask keeps the BSW main-function chain order (receive
/// processing before transmission, SWS_Rte_07800 position ordering).
#[test]
fn handle_width_and_schedule_order() {
    // The selected host CAN driver keeps 8-bit CanIf PDU handles.
    let mut wide = single_component_sources();
    parameter(&mut wide, "TxValue", "CanIfTxPduId", "256");
    rejects_in_both(&wide, "CANIF_HANDLE_WIDTH");
    let issues = build(&wide).err().unwrap();
    assert!(
        issues.iter().any(|issue| issue.code == "CANIF_HANDLE_WIDTH"
            && issue.object.as_deref() == Some("/Configuration/CanIf/Config/TxValue")),
        "{issues:?}"
    );
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
}

/// The public OS/RTE type headers are delivered byte-identical to the runtime.
#[test]
fn public_headers_are_delivered_unchanged() {
    let scratch = Scratch::new();
    let (_, files) = project(
        &single_component_sources(),
        &scratch.0,
        BuildTarget::LinuxX64ControlledV1,
    );
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("runtime/os/include");
    // Rte_Os_Type.h is an RTE-generated header (SWS_Rte_07120), delivered by the RTE emitter.
    for (header, directory) in [
        ("Os.h", "os/include"),
        ("Os_Types.h", "os/include"),
        ("Rte_Os_Type.h", "generated/include"),
    ] {
        assert_eq!(
            file(&files, &format!("{directory}/{header}")).as_bytes(),
            std::fs::read(runtime.join(header)).unwrap(),
            "{header}"
        );
    }
}

/// Every generated Task (and ISR) body is wrapped in the OS code section markers,
/// so `Os_MemMap.h` places it with the OS entries.
// Verifies: SWS_Os_00815
#[test]
fn generated_task_entries_are_wrapped_in_os_memory_mapping() {
    let plan = build(&inputs()).unwrap();
    let files = prepared_pipeline_files(&plan);
    let config = files
        .iter()
        .find(|(path, _)| path == "generated/src/Ecu_Config.c")
        .map(|(_, bytes)| String::from_utf8(bytes.clone()).unwrap())
        .unwrap();
    let lines: Vec<_> = config.lines().collect();
    let entries: Vec<_> = (0..lines.len())
        .filter(|at| lines[*at].starts_with("TASK(") || lines[*at].starts_with("ISR("))
        .collect();
    assert!(!entries.is_empty(), "{config}");
    for at in entries {
        assert_eq!(
            lines[at - 2..at],
            ["#define OS_START_SEC_CODE", "#include \"Os_MemMap.h\""],
            "{config}"
        );
        assert_eq!(
            lines[at + 1..at + 3],
            ["#define OS_STOP_SEC_CODE", "#include \"Os_MemMap.h\""],
            "{config}"
        );
    }
}
