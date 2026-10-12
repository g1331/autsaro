//! Generated OS contracts (`ecu::configure_os`) on the mainline single-application
//! project: counter conversions and timing report, the OsStatus guard,
//! RES_SCHEDULER, the OS counter service description, ARTI, schedule-table
//! validation and the public OS delivery. Expected values are SWS/ECUC literals
//! or independent consumers in `tests/fixtures`, never read back from the
//! generator.

use super::{
    Scratch, build, change, named, parameter, rejects_in_both, single_component_sources,
    write_files, xml_edit,
};
use autosar_config_core::integration::InputSource;
use autosar_config_core::target::BuildTarget;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn replaced(sources: &[InputSource], before: &str, after: &str) -> Vec<InputSource> {
    let mut changed = false;
    let result = sources
        .iter()
        .map(|source| {
            let text = std::str::from_utf8(source.bytes()).unwrap();
            changed |= text.contains(before);
            InputSource::new(
                source.logical_path(),
                text.replace(before, after).into_bytes(),
            )
            .unwrap()
        })
        .collect();
    assert!(changed, "{before} is absent from the real input");
    result
}

/// Prepared files of `sources` with the reference echo application (the same
/// application as `single_component_project`), written to `root/project`.
fn project(
    sources: &[InputSource],
    root: &Path,
    target: BuildTarget,
) -> (PathBuf, Vec<(String, Vec<u8>)>) {
    use autosar_config_core::{ApplicationSource, prepare_ecu_project_with_applications};
    let plan = build(sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    let fixture_text = std::fs::read_to_string(fixture("multi-application/Ingress.c")).unwrap();
    let application = root.join("Ingress.c");
    std::fs::write(
        &application,
        fixture_text.replace("        (void)Rte_Write_Value_Value(value);\n", ""),
    )
    .unwrap();
    let prepared = prepare_ecu_project_with_applications(
        &plan,
        target,
        &[ApplicationSource {
            component_instance: "/Application/Pipeline/IngressInstance".into(),
            path: application,
        }],
    )
    .unwrap_or_else(|issues| panic!("{issues:?}"));
    let directory = root.join("project");
    let files = prepared.into_files();
    write_files(&directory, &files);
    (directory, files)
}

fn file<'a>(files: &'a [(String, Vec<u8>)], name: &str) -> &'a str {
    std::str::from_utf8(
        &files
            .iter()
            .find(|(path, _)| path == name)
            .unwrap_or_else(|| panic!("{name} is not delivered"))
            .1,
    )
    .unwrap()
}

fn codes(sources: &[InputSource]) -> Vec<String> {
    build(sources)
        .err()
        .expect("rejected input was accepted")
        .into_iter()
        .map(|issue| issue.code)
        .collect()
}

fn counter_variant(renamed: bool, maximum: u32) -> Vec<InputSource> {
    let mut sources = if renamed {
        replaced(
            &single_component_sources(),
            "SystemCounter",
            "RenamedCounter",
        )
    } else {
        single_component_sources()
    };
    if maximum != 65535 {
        parameter(
            &mut sources,
            if renamed {
                "RenamedCounter"
            } else {
                "SystemCounter"
            },
            "OsCounterMaxAllowedValue",
            &maximum.to_string(),
        );
    }
    sources
}

/// SWS_Os_00370 timing report and the counter profile accepted by the selected
/// controlled-time target: one logical millisecond per tick (ECUC
/// OsSecondsPerTick 0.001), base 1, minimum cycle 1.
pub(super) fn counter_timing_report_and_profile() {
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
            serde_json::from_str(file(&files, "os-generation-timing.json")).unwrap();
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
        let header = file(&files, "os/include/Os_Counter.h");
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
pub(super) fn status_guard_and_rejections() {
    for (status, define) in [
        ("STANDARD", "#define OS_STATUS_EXTENDED 0"),
        ("EXTENDED", "#define OS_STATUS_EXTENDED 1"),
    ] {
        let mut sources = single_component_sources();
        parameter(&mut sources, "OperatingSystem", "OsStatus", status);
        let scratch = Scratch::new();
        let (_, files) = project(&sources, &scratch.0, BuildTarget::LinuxX64ControlledV1);
        assert!(
            file(&files, "os/include/Os_Cfg.h").contains(define),
            "{status}"
        );
    }
    for status in ["UNKNOWN", "standard"] {
        let mut sources = single_component_sources();
        parameter(&mut sources, "OperatingSystem", "OsStatus", status);
        rejects_in_both(&sources, "TARGET_PARAMETER_UNSUPPORTED");
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

const RES_SCHEDULER: &str = "<ECUC-CONTAINER-VALUE><SHORT-NAME>RES_SCHEDULER</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">/AUTOSAR/EcucDefs/Os/OsResource</DEFINITION-REF><PARAMETER-VALUES><ECUC-TEXTUAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-ENUMERATION-PARAM-DEF\">/AUTOSAR/EcucDefs/Os/OsResource/OsResourceProperty</DEFINITION-REF><VALUE>STANDARD</VALUE></ECUC-TEXTUAL-PARAM-VALUE></PARAMETER-VALUES></ECUC-CONTAINER-VALUE>";

/// OsUseResScheduler (ECUC_Os_00040) with an optional explicit RES_SCHEDULER
/// OsResource in the selected OS module.
fn scheduler_variant(value: &str, resource: bool) -> Vec<InputSource> {
    let mut sources = single_component_sources();
    parameter(&mut sources, "OperatingSystem", "OsUseResScheduler", value);
    if resource {
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| named(node, "ECUC-MODULE-CONFIGURATION-VALUES", "Os"),
            |text| {
                let at = text.rfind("</CONTAINERS>").unwrap();
                format!("{}{RES_SCHEDULER}{}", &text[..at], &text[at..])
            },
        );
    }
    sources
}

/// Boolean literal forms of OsUseResScheduler and the OS configuration
/// parameters the selected SC1 kernel requires (ECUC Os module definition).
pub(super) fn scheduler_configuration_rejections() {
    for (value, resource) in [("false", false), ("true", false), ("0", false), ("1", true)] {
        build(&scheduler_variant(value, resource))
            .unwrap_or_else(|issues| panic!("{value}: {issues:?}"));
    }
    // An explicit RES_SCHEDULER needs OsUseResScheduler TRUE (SWS_Os_00801).
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
pub(super) fn counter_service_description() {
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
        let xml = file(&files, "os/Os_Service.arxml");
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
    assert_rejected(&component, "SYMBOL_PRODUCER_DUPLICATE");
}

/// Every OS trigger of the single-application project moved from alarms to
/// repeating schedule tables (expiry point offset 0, sync strategy NONE), as
/// in `epic4_timing/schedule_tables.arxml`.
fn schedule_table_variant() -> Vec<InputSource> {
    let tables = [InputSource::new(
        "tables.arxml",
        std::fs::read(fixture("epic4_timing/schedule_tables.arxml")).unwrap(),
    )
    .unwrap()];
    let mut sources = single_component_sources();
    for (alarm, table) in [("Alarm_Work", "Table_Work"), ("Alarm_App", "Table_App")] {
        let replacement = super::subtree(&tables, "tables.arxml", "ECUC-CONTAINER-VALUE", table);
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| named(node, "ECUC-CONTAINER-VALUE", alarm),
            |_| replacement.clone(),
        );
        sources = replaced(
            &sources,
            &format!("/Configuration/Os/{alarm}</VALUE-REF>"),
            &format!("/Configuration/Os/{table}/Point</VALUE-REF>"),
        );
    }
    sources = replaced(
        &sources,
        "/RteUsedOsAlarmRef<",
        "/RteUsedOsSchTblExpiryPointRef<",
    );
    replaced(
        &sources,
        "/RteBswUsedOsAlarmRef<",
        "/RteBswUsedOsSchTblExpiryPointRef<",
    )
}

/// The selected kernel drives schedule tables only as repeating, implicitly
/// unsynchronized tables started at a first expiry point of offset 0, owned
/// by the mapped task, on the software SystemCounter.
pub(super) fn schedule_table_rejections() {
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
            "wrong-strategy",
            "OsScheduleTblSyncStrategy</DEFINITION-REF><VALUE>NONE</VALUE>",
            "OsScheduleTblSyncStrategy</DEFINITION-REF><VALUE>IMPLICIT</VALUE>",
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
pub(super) fn handle_width_and_schedule_order() {
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
pub(super) fn public_os_headers_are_delivered_unchanged() {
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
    for header in ["Os.h", "Os_Types.h", "Rte_Os_Type.h"] {
        assert_eq!(
            file(&files, &format!("os/include/{header}")).as_bytes(),
            std::fs::read(runtime.join(header)).unwrap(),
            "{header}"
        );
    }
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
fn checked(
    command: &mut std::process::Command,
    directory: &Path,
    name: &str,
) -> std::process::Output {
    let output = super::tooling::run_public_command(
        command,
        directory,
        name,
        std::time::Duration::from_secs(180),
    );
    assert!(
        output.status.success(),
        "{name}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// Builds `project` with the delivered `ecu-tool.py` and returns the binary.
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
fn build_ecu(project: &Path, output: &Path, mode: &str, control: Option<&Path>) -> PathBuf {
    checked(
        &mut super::tooling::ecu_build_command(project, output, mode, control),
        output.parent().unwrap(),
        &format!(
            "build-{}",
            output
                .file_name()
                .unwrap()
                .to_string_lossy()
                .replace(' ', "-")
        ),
    );
    super::tooling::native_binary(output, "ecu_probe")
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
fn run(binary: &Path, arguments: &[&str]) -> String {
    let mut command = std::process::Command::new(binary);
    command.args(arguments);
    let output = checked(
        &mut command,
        binary.parent().unwrap(),
        &format!("run-{}", arguments.join("-")),
    );
    String::from_utf8(output.stdout).unwrap()
}

/// SWS_Os_00393 conversion macros and the ECUC counter constants, checked by
/// the independent literal oracle `ecu_counter_time.c` and by header mutants
/// that the oracle must reject (exit 1: scale, 2: double evaluation, 3: constant).
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
pub(super) fn counter_header_oracles() {
    let scratch = Scratch::new();
    let compile = |project: &Path, binary: &Path, renamed: bool, maximum: u32| {
        let mut command = std::process::Command::new("gcc");
        command
            .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-I"])
            .arg(project.join("os/include"))
            .arg("-I")
            .arg(project.join("include"))
            .arg(format!("-DEXPECTED_COUNTER_MAX={maximum}u"));
        if renamed {
            command.arg("-DCOUNTER_TIME_RENAMED=1");
        }
        command
            .arg(fixture("ecu_counter_time.c"))
            .arg("-o")
            .arg(binary);
        checked(&mut command, &scratch.0, "counter-time-compile");
    };
    for (label, renamed, maximum) in [
        ("default", false, 65535u32),
        ("renamed", true, 65535),
        ("alternate-maximum", false, 4095),
    ] {
        let root = scratch.0.join(label);
        std::fs::create_dir(&root).unwrap();
        let (project, _) = project(
            &counter_variant(renamed, maximum),
            &root,
            super::tooling::native_target(),
        );
        let counter = if renamed {
            "RenamedCounter"
        } else {
            "SystemCounter"
        };
        let binary = super::tooling::native_binary(&root, "counter_time");
        compile(&project, &binary, renamed, maximum);
        let stdout = run(&binary, &[]);
        assert_eq!(
            stdout.trim(),
            "counter_time PASS: 24 literal unit values; 4 single evaluations; 4 expressions"
        );
        let header_path = project.join("os/include/Os_Counter.h");
        let header = std::fs::read_to_string(&header_path).unwrap();
        for (name, mutant, expected_exit) in [
            (
                "wrong-nanosecond-scale",
                header.replace("* UINT64_C(1000000)", "* UINT64_C(2000000)"),
                1,
            ),
            (
                "duplicate-argument-evaluation",
                header.replace(
                    &format!("(Os_TicksToNs_{counter}((ticks)))"),
                    &format!("((void)(ticks), Os_TicksToNs_{counter}((ticks)))"),
                ),
                2,
            ),
            (
                "wrong-counter-maximum",
                header.replace(
                    &format!("OSMAXALLOWEDVALUE_{counter} UINT64_C({maximum})"),
                    &format!("OSMAXALLOWEDVALUE_{counter} UINT64_C({})", maximum - 1),
                ),
                3,
            ),
            (
                "wrong-counter-base",
                header.replace(
                    &format!("OSTICKSPERBASE_{counter} UINT64_C(1)"),
                    &format!("OSTICKSPERBASE_{counter} UINT64_C(2)"),
                ),
                3,
            ),
            (
                "wrong-counter-minimum-cycle",
                header.replace(
                    &format!("OSMINCYCLE_{counter} UINT64_C(1)"),
                    &format!("OSMINCYCLE_{counter} UINT64_C(2)"),
                ),
                3,
            ),
            (
                "wrong-system-tick-duration",
                header.replace(
                    "OSTICKDURATION UINT64_C(1000000)",
                    "OSTICKDURATION UINT64_C(2000000)",
                ),
                3,
            ),
            (
                "wrong-counter-id-maximum-alias",
                header.replace(
                    &format!(
                        "OSMAXALLOWEDVALUE_OS_COUNTER_ID_{counter} OSMAXALLOWEDVALUE_{counter}"
                    ),
                    &format!("OSMAXALLOWEDVALUE_OS_COUNTER_ID_{counter} OSTICKSPERBASE_{counter}"),
                ),
                3,
            ),
        ] {
            assert_ne!(header, mutant, "{name}");
            std::fs::write(&header_path, &mutant).unwrap();
            let mutated = super::tooling::native_binary(&root, name);
            compile(&project, &mutated, renamed, maximum);
            let result = super::tooling::run_public_command(
                &mut std::process::Command::new(&mutated),
                &root,
                name,
                std::time::Duration::from_secs(30),
            );
            assert_eq!(result.status.code(), Some(expected_exit), "{label} {name}");
        }
        std::fs::write(&header_path, &header).unwrap();
    }
}

/// A moved STANDARD delivery supplies its own OsStatus guard: a conflicting
/// consumer definition is refused at preprocessing, and the build runs the
/// independent `ecu_control.c` vectors.
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
pub(super) fn standard_status_build() {
    let scratch = Scratch::new();
    let mut sources = single_component_sources();
    parameter(&mut sources, "OperatingSystem", "OsStatus", "STANDARD");
    let (original, _) = project(&sources, &scratch.0, super::tooling::native_target());
    let moved = scratch.0.join("moved standard target");
    std::fs::rename(&original, &moved).unwrap();
    let mut conflict = std::process::Command::new("gcc");
    conflict
        .args(["-std=c99", "-E", "-x", "c", "-DOS_STATUS_EXTENDED=1", "-I"])
        .arg(moved.join("os/include"))
        .arg("-I")
        .arg(moved.join("include"))
        .arg(fixture("ecu_standard_control.c"));
    let result = super::tooling::run_public_command(
        &mut conflict,
        &scratch.0,
        "standard-conflict",
        std::time::Duration::from_secs(60),
    );
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("OsStatus differs from the validated plan"),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let binary = build_ecu(
        &moved,
        &scratch.0.join("standard-build"),
        "probe",
        Some(&fixture("ecu_standard_control.c")),
    );
    let stdout = run(&binary, &[]);
    assert!(
        stdout.contains("independent_control signal=2 default_session_did=2 completed=20"),
        "{stdout}"
    );
}

/// RES_SCHEDULER (SWS_Os_00801): with OsUseResScheduler TRUE the generated
/// resource has ceiling of the task, task access only and GetResource returns
/// E_OK; FALSE configures no resource and GetResource is E_OS_ID.
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
pub(super) fn scheduler_resource_runs() {
    let scratch = Scratch::new();
    for (value, resource, enabled) in [
        ("false", false, false),
        ("true", false, true),
        ("0", false, false),
        ("1", true, true),
    ] {
        let root = scratch.0.join(format!("scheduler {value}"));
        std::fs::create_dir(&root).unwrap();
        let (project, _) = project(
            &scheduler_variant(value, resource),
            &root,
            super::tooling::native_target(),
        );
        let binary = build_ecu(
            &project,
            &root.join("build"),
            "test",
            Some(&fixture("ecu_scheduler_control.c")),
        );
        let stdout = run(&binary, &[if enabled { "true" } else { "false" }]);
        assert!(
            stdout.contains(&format!(
                "scheduler_consumer enabled={} calls=2 reason=0",
                usize::from(enabled)
            )),
            "{value}: {stdout}"
        );
    }
}

/// An OS module renamed to `OsAlt` with OsUseResScheduler TRUE.
#[cfg(any(
    feature = "official-oracles",
    all(feature = "native-tests", any(windows, target_os = "linux"))
))]
fn renamed_os_variant() -> Vec<InputSource> {
    let mut sources = scheduler_variant("true", false);
    sources = replaced(
        &sources,
        "<SHORT-NAME>Os</SHORT-NAME>",
        "<SHORT-NAME>OsAlt</SHORT-NAME>",
    );
    replaced(&sources, "/Configuration/Os/", "/Configuration/OsAlt/")
}

/// The debugger expressions of an ARTI description as an independent C probe
/// (`arti_reader.py` writes the same probe after its official checks).
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
fn arti_expression_probe(project: &Path, header: &Path) {
    let xml = std::fs::read_to_string(project.join("os/Os_Arti.arxml")).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let expressions = doc
        .descendants()
        .filter(|node| {
            node.has_tag_name("ECUC-CONTAINER-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/ArtiExpression"))
                })
        })
        .map(|node| {
            node.descendants()
                .find(|child| child.has_tag_name("ECUC-TEXTUAL-PARAM-VALUE"))
                .and_then(|value| value.children().find(|child| child.has_tag_name("VALUE")))
                .and_then(|value| value.text())
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert!(!expressions.is_empty());
    let mut probe = String::from(
        "static void arti_expression_probe(void) {\n    volatile uintptr_t observed = 0u;\n",
    );
    for expression in expressions {
        assert!(
            !expression.contains('('),
            "call in debugger expression: {expression}"
        );
        probe.push_str(&format!("    observed = (uintptr_t)({expression});\n"));
    }
    probe.push_str("    (void)observed;\n}\n");
    std::fs::write(header, probe).unwrap();
}

/// ARTI R24-11 hooks observed at runtime by the independent consumer
/// `ecu_arti_control.c`: task/alarm/table/resource state agrees with the OS
/// services, every service error reaches the ErrorHook, no internal service
/// masquerades as application SetEvent, nothing is dropped.
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
pub(super) fn arti_consumer() {
    let scratch = Scratch::new();
    for (label, os, sources) in [
        ("alarms", "Os", single_component_sources()),
        ("renamed", "OsAlt", renamed_os_variant()),
        ("tables", "Os", schedule_table_variant()),
    ] {
        let root = scratch.0.join(format!("arti {label}"));
        std::fs::create_dir(&root).unwrap();
        let (project, _) = project(&sources, &root, super::tooling::native_target());
        let consumer = root.join("consumer");
        std::fs::create_dir(&consumer).unwrap();
        for name in ["ecu_arti_control.c", "ecu_control.c"] {
            std::fs::copy(fixture(name), consumer.join(name)).unwrap();
        }
        arti_expression_probe(&project, &consumer.join("Arti_ExpressionProbe.h"));
        let binary = build_ecu(
            &project,
            &root.join("build"),
            "test",
            Some(&consumer.join("ecu_arti_control.c")),
        );
        let stdout = run(&binary, &[os]);
        assert!(
            stdout.contains(&format!(
                "arti_consumer os={os} observations=2 getters=2 errors=2 internal_services=0 dropped=0"
            )),
            "{label}: {stdout}"
        );
    }
}

/// (address, section VMA, size) of `symbol` and its section in a linked binary,
/// for ELF (absolute symbol values) and PE/COFF (section-relative values).
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
fn section_offset(binary: &Path, symbol: &str, section: &str) -> (u64, u64) {
    let objdump = super::tooling::execution_settings().objdump;
    let dump = |flag: &str| {
        let output = std::process::Command::new(&objdump)
            .arg(flag)
            .arg(binary)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    };
    let headers = dump("-h");
    let row = headers
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .find(|fields| fields.get(1) == Some(&section))
        .unwrap_or_else(|| panic!("{section} is not linked: {headers}"));
    let size = u64::from_str_radix(row[2], 16).unwrap();
    let vma = u64::from_str_radix(row[3], 16).unwrap();
    let symbols = dump("-t");
    let lines = symbols
        .lines()
        .filter(|line| line.split_whitespace().last() == Some(symbol))
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 1, "{symbol}: {symbols}");
    let offset = if let Some((_, value)) = lines[0].rsplit_once(" 0x") {
        u64::from_str_radix(value.split_whitespace().next().unwrap(), 16).unwrap()
    } else {
        u64::from_str_radix(lines[0].split_whitespace().next().unwrap(), 16).unwrap() - vma
    };
    (offset, size)
}

/// A renamed and renumbered single-application variant: task, ECU, CAN
/// identifiers, schedule positions and a 20 ms application/transmission period.
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
fn renumbered_variant() -> Vec<InputSource> {
    let mut sources = replaced(&single_component_sources(), "Task_Ecu", "OwnerTask");
    sources = replaced(&sources, "ReferenceEcu", "LocalEcu");
    for (old, new) in [(800, 1100), (801, 1101), (1792, 1793), (1800, 1801)] {
        sources = replaced(
            &sources,
            &format!("<IDENTIFIER>{old}</IDENTIFIER>"),
            &format!("<IDENTIFIER>{new}</IDENTIFIER>"),
        );
        sources = replaced(
            &sources,
            &format!("<VALUE>{old}</VALUE>"),
            &format!("<VALUE>{new}</VALUE>"),
        );
    }
    sources = replaced(&sources, "<PERIOD>0.01</PERIOD>", "<PERIOD>0.02</PERIOD>");
    parameter(&mut sources, "Alarm_App", "OsAlarmAlarmTime", "20");
    parameter(&mut sources, "Alarm_App", "OsAlarmCycleTime", "20");
    parameter(&mut sources, "TxValuePdu", "ComTxModeTimePeriod", "0.02");
    parameter(&mut sources, "Tx", "ComMainTxTimeBase", "0.02");
    for (owner, definition, handle) in [
        ("RxValue", "CanIfRxPduId", "4"),
        ("TxValue", "CanIfTxPduId", "5"),
        ("DiagRequest", "CanIfRxPduId", "6"),
        ("DiagResponse", "CanIfTxPduId", "7"),
    ] {
        parameter(&mut sources, owner, definition, handle);
    }
    // Spread every RTE/BSW position by 10 while keeping the order.
    let ecuc = sources
        .iter_mut()
        .find(|source| source.logical_path() == "ecuc.arxml")
        .unwrap();
    let mut text = std::str::from_utf8(ecuc.bytes()).unwrap().to_owned();
    let ranges = {
        let document = roxmltree::Document::parse(&text).unwrap();
        document
            .descendants()
            .filter(|node| {
                node.has_tag_name("VALUE")
                    && node.parent().is_some_and(|parent| {
                        parent.children().any(|child| {
                            child.has_tag_name("DEFINITION-REF")
                                && child
                                    .text()
                                    .is_some_and(|text| text.ends_with("PositionInTask"))
                        })
                    })
            })
            .map(|node| (node.range(), node.text().unwrap().parse::<u32>().unwrap()))
            .collect::<Vec<_>>()
    };
    assert_eq!(ranges.len(), 11);
    for (range, position) in ranges.into_iter().rev() {
        text.replace_range(range, &format!("<VALUE>{}</VALUE>", position * 10));
    }
    *ecuc = InputSource::new("ecuc.arxml", text.into_bytes()).unwrap();
    sources
}

/// The delivered public OS API compiles for an independent legacy consumer;
/// the vector table and OS entries are linked in their mapped sections; the
/// independent `ecu_control.c` vectors pass with alarms and schedule tables;
/// a renamed and renumbered variant runs with its own identities.
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
pub(super) fn delivery_and_independent_control() {
    let scratch = Scratch::new();
    let (original, _) = project(
        &single_component_sources(),
        &scratch.0,
        super::tooling::native_target(),
    );
    let moved = scratch.0.join("moved delivery");
    std::fs::rename(&original, &moved).unwrap();
    // No repository header path and no OS object: only the moved delivery.
    let public = super::tooling::native_binary(&scratch.0, "public_compatibility");
    let mut compile = std::process::Command::new("gcc");
    compile
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic", "-I"])
        .arg(moved.join("os/include"))
        .arg("-I")
        .arg(moved.join("include"))
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("runtime/os/tests/public_compatibility.c"),
        )
        .arg("-o")
        .arg(&public);
    checked(&mut compile, &scratch.0, "public-compile");
    assert_eq!(
        run(&public, &[]).replace("\r\n", "\n"),
        "public_compatibility declarations=16 evaluations=0 error_codes=23 unique=pass\n"
    );

    let probe = build_ecu(&moved, &scratch.0.join("probe build"), "test", None);
    // The 256-byte vector table starts its own section; OS entries and hooks
    // are mapped through Os_MemMap.h into the OS code section.
    assert_eq!(
        section_offset(&probe, "Os_InterruptVectorTable", ".os_vec"),
        (0, 256)
    );
    let sections = super::tooling::symbol_sections(
        Path::new(&super::tooling::execution_settings().objdump),
        &probe,
    );
    for name in [
        "Os_TaskEntry_OS_TASK_ID_Task_Ecu",
        "ErrorHook",
        "PreTaskHook",
        "PostTaskHook",
        "StartupHook",
        "ShutdownHook",
    ] {
        let found = sections
            .iter()
            .filter(|(symbol, _)| symbol == name)
            .map(|(_, section)| section.as_str())
            .collect::<Vec<_>>();
        assert_eq!(found, [".os_code"], "{name}");
    }
    let stdout = run(&probe, &[]);
    assert!(stdout.contains("ecu_probe completed=20"), "{stdout}");

    for (label, sources) in [
        ("alarms", single_component_sources()),
        ("tables", schedule_table_variant()),
    ] {
        let root = scratch.0.join(format!("control {label}"));
        std::fs::create_dir(&root).unwrap();
        let (project, _) = project(&sources, &root, super::tooling::native_target());
        let binary = build_ecu(
            &project,
            &root.join("build"),
            "probe",
            Some(&fixture("ecu_control.c")),
        );
        let stdout = run(&binary, &[]);
        assert!(
            stdout.contains("independent_control signal=2 default_session_did=2 completed=20"),
            "{label}: {stdout}"
        );
    }

    let root = scratch.0.join("renumbered");
    std::fs::create_dir(&root).unwrap();
    let sources = renumbered_variant();
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert!(
        plan.description()
            .schedule
            .entities
            .iter()
            .filter(|entity| entity.application)
            .all(|entity| entity.period_ms == 20)
    );
    let (project, files) = project(&sources, &root, super::tooling::native_target());
    let config = file(&files, "src/Ecu_Config.c");
    assert!(config.contains("\"OwnerTask\""), "{config}");
    let binary = build_ecu(&project, &root.join("build"), "test", None);
    let stdout = run(&binary, &[]);
    // Com periodic transmission (SWS_Com_00495) at the 20 ms period with the
    // renumbered CanIf handle 5 and CAN identifier 1101; nothing at 10 ms.
    let outputs = stdout
        .lines()
        .filter(|line| line.starts_with("ecu_output "))
        .collect::<Vec<_>>();
    assert_eq!(outputs.len(), 1, "{stdout}");
    assert!(
        outputs[0].starts_with("ecu_output epoch=20 ")
            && outputs[0].ends_with("pdu=5 id=1101 dlc=4"),
        "{stdout}"
    );
}

/// Os_Arti.arxml and Os_Service.arxml validate against the official R24-11
/// schema, and the independent ARTI reader resolves every ARTI reference
/// against the official ECUC MOD definitions and the delivered inputs.
#[cfg(feature = "official-oracles")]
pub(super) fn official_os_descriptions() {
    use autosar_config_core::schema;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let archive = schema::schema_archive(root);
    let python = std::env::var_os("AUTOSAR_PYTHON")
        .expect("Set AUTOSAR_PYTHON to the absolute Python interpreter");
    let scratch = Scratch::new();
    let reader = |project: &Path, os: &str, header: &Path| {
        std::process::Command::new(&python)
            .arg(fixture("arti_reader.py"))
            .arg(project)
            .arg(root)
            .arg(os)
            .arg(header)
            .output()
            .unwrap()
    };
    for (label, os, sources) in [
        ("alarms", "Os", single_component_sources()),
        ("renamed", "OsAlt", renamed_os_variant()),
        ("tables", "Os", schedule_table_variant()),
    ] {
        let directory = scratch.0.join(label);
        std::fs::create_dir(&directory).unwrap();
        let (project, _) = project(&sources, &directory, BuildTarget::LinuxX64ControlledV1);
        for name in ["os/Os_Arti.arxml", "os/Os_Service.arxml"] {
            let path = project.join(name);
            let xml = std::fs::read_to_string(&path).unwrap();
            let issues = schema::validate_files(&archive, &[(&path, &xml)]).unwrap();
            assert!(issues.is_empty(), "{label} {name}: {issues:?}");
        }
        let header = directory.join("Arti_ExpressionProbe.h");
        let output = reader(&project, os, &header);
        assert!(
            output.status.success(),
            "{label}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(header.is_file());
        if label != "alarms" {
            continue;
        }
        let path = project.join("os/Os_Arti.arxml");
        let xml = std::fs::read_to_string(&path).unwrap();
        for (fault, changed) in [
            (
                "broken reference",
                xml.replace("/Configuration/Os/Task_Ecu", "/Configuration/Os/AbsentTask"),
            ),
            (
                "missing/extra hook",
                xml.replace("OsTask_Start", "OsTask_Unknown"),
            ),
        ] {
            assert_ne!(changed, xml, "{fault}");
            std::fs::write(&path, changed).unwrap();
            let output = reader(&project, os, &directory.join("rejected.h"));
            assert!(
                !output.status.success() && String::from_utf8_lossy(&output.stderr).contains(fault),
                "{fault}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::fs::write(&path, &xml).unwrap();
        let binding_path = project.join("os/src/Os_Arti.c");
        let binding = std::fs::read_to_string(&binding_path).unwrap();
        let changed = binding.replace("0u, OsTask_Start,", "0u, OsTask_Absent,");
        assert_ne!(binding, changed);
        std::fs::write(&binding_path, changed).unwrap();
        let output = reader(&project, os, &directory.join("rejected.h"));
        assert!(
            !output.status.success()
                && String::from_utf8_lossy(&output.stderr).contains("missing literal hook"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// The delivered OS counter service links into the counter-service harness
/// (`tests/python/native/os_suites.py`) and rejects wrong bindings, units and
/// signatures.
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
pub(super) fn counter_service_native() {
    let scratch = Scratch::new();
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
        let root = scratch.0.join(counter);
        std::fs::create_dir(&root).unwrap();
        let (project, _) = project(&sources, &root, super::tooling::native_target());
        let stage = root.join("native");
        std::fs::create_dir(&stage).unwrap();
        let mut command = super::tooling::python_command();
        command
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/python/native/os_suites.py"))
            .arg("counter-service")
            .arg(&stage)
            .arg(&project)
            .arg(counter);
        checked(&mut command, &stage, "counter-service");
    }
}
