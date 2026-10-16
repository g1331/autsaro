//! OS configuration outside the selected SC1 single-core profile is rejected
//! at its source object before generation, never ignored.
use super::single_component::{add_to_container, rejected_at, rejected_with};
use super::*;
use autosar_config_core::integration::InputSource;

const OS: &str = "/AUTOSAR/EcucDefs/Os";

/// Adds a top-level container of `definition` named `name` to the Os module.
fn add_os_container(sources: &mut [InputSource], name: &str, definition: &str, content: &str) {
    xml_edit(
        sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "OSDEFAULTAPPMODE"),
        |text| {
            format!(
                "<ECUC-CONTAINER-VALUE><SHORT-NAME>{name}</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">{OS}/{definition}</DEFINITION-REF>{content}</ECUC-CONTAINER-VALUE>{text}"
            )
        },
    );
}

/// Containers and parameters of unselected OS features (OS-Applications, ISRs,
/// IOC, memory/timing protection, spinlocks, multicore, alarm actions other
/// than SetEvent, hardware counter drivers, adjustable or precision-bound
/// schedule tables) have no definition in the selected catalog.
#[test]
fn unselected_os_features_have_no_definition() {
    for (name, definition) in [
        // Rejects: ECUC_Os_00114
        ("Application", "OsApplication"),
        // Rejects: ECUC_Os_00041
        ("Isr", "OsIsr"),
        // Rejects: ECUC_Os_01000
        ("Ioc", "OsIoc"),
        // Rejects: ECUC_Os_00397
        ("Peripheral", "OsPeripheralArea"),
        // Rejects: ECUC_Os_00258
        ("Spinlock", "OsSpinlock"),
    ] {
        let mut sources = single_component_sources();
        add_os_container(&mut sources, name, definition, "");
        rejected_with(
            &sources,
            ("REFERENCE_UNRESOLVED", "DEFINITION_UNKNOWN"),
            "ecuc.arxml",
            &format!("/Configuration/Os/{name}"),
        );
    }
    for (tables, owner, path, object, name, value) in [
        // Rejects: ECUC_Os_00325
        (
            false,
            "Task_Ecu",
            "OsTask",
            "/Configuration/Os/Task_Ecu",
            "OsTaskTimingProtection",
            "",
        ),
        // Rejects: ECUC_Os_00404
        (
            false,
            "Task_Ecu",
            "OsTask",
            "/Configuration/Os/Task_Ecu",
            "OsTaskPeriod",
            "FLOAT:0.01",
        ),
        // Rejects: ECUC_Os_00079
        (
            false,
            "Task_Ecu",
            "OsTask",
            "/Configuration/Os/Task_Ecu",
            "OsTaskResourceRef",
            "REFERENCE:/Configuration/Os/OSDEFAULTAPPMODE",
        ),
        // Rejects: ECUC_Os_01019
        (
            false,
            "OperatingSystem",
            "OsOS",
            "/Configuration/Os/OperatingSystem",
            "OsNumberOfCores",
            "INTEGER:1",
        ),
        // Rejects: ECUC_Os_00371
        (
            false,
            "SystemCounter",
            "OsCounter",
            "/Configuration/Os/SystemCounter",
            "OsDriver",
            "",
        ),
        // Rejects: ECUC_Os_00386
        (
            false,
            "SystemCounter",
            "OsCounter",
            "/Configuration/Os/SystemCounter",
            "OsTimeConstant",
            "",
        ),
        // Rejects: ECUC_Os_00007
        (
            false,
            "Action",
            "OsAlarm/OsAlarmAction",
            "/Configuration/Os/Alarm_Work/Action",
            "OsAlarmActivateTask",
            "",
        ),
        // Rejects: ECUC_Os_00014
        (
            false,
            "Action",
            "OsAlarm/OsAlarmAction",
            "/Configuration/Os/Alarm_Work/Action",
            "OsAlarmCallback",
            "",
        ),
        // Rejects: ECUC_Os_00302
        (
            false,
            "Action",
            "OsAlarm/OsAlarmAction",
            "/Configuration/Os/Alarm_Work/Action",
            "OsAlarmIncrementCounter",
            "",
        ),
        // Rejects: ECUC_Os_00068
        (
            true,
            "Point",
            "OsScheduleTable/OsScheduleTableExpiryPoint",
            "/Configuration/Os/Table_Work/Point",
            "OsScheduleTblAdjustableExpPoint",
            "",
        ),
        // Rejects: ECUC_Os_00064
        (
            true,
            "Sync",
            "OsScheduleTable/OsScheduleTableSync",
            "/Configuration/Os/Table_Work/Sync",
            "OsScheduleTblExplicitPrecision",
            "INTEGER:1",
        ),
    ] {
        let mut sources = if tables {
            schedule_table_variant()
        } else {
            single_component_sources()
        };
        add_to_container(
            &mut sources,
            owner,
            &format!("{OS}/{path}"),
            name,
            value,
            "",
        );
        let object = if value.is_empty() {
            format!("{object}/{name}")
        } else {
            object.to_owned()
        };
        rejected_with(
            &sources,
            ("REFERENCE_UNRESOLVED", "DEFINITION_UNKNOWN"),
            "ecuc.arxml",
            &object,
        );
    }
}

/// Values outside the selected profile are refused at the owning container:
/// SC1 only, no ProtectionHook, stack monitoring and ARTI always on, a
/// software counter, relative autostart and unsynchronized schedule tables.
#[test]
fn os_policy_values_outside_the_profile_are_rejected() {
    for (tables, owner, name, value, code, object) in [
        // Rejects: ECUC_Os_00259
        (
            false,
            "OperatingSystem",
            "OsScalabilityClass",
            "SC2",
            "TARGET_PARAMETER_UNSUPPORTED",
            "/Configuration/Os/OperatingSystem",
        ),
        (
            false,
            "OperatingSystem",
            "OsScalabilityClass",
            "SC4",
            "TARGET_PARAMETER_UNSUPPORTED",
            "/Configuration/Os/OperatingSystem",
        ),
        // Rejects: ECUC_Os_00214
        (
            false,
            "Hooks",
            "OsProtectionHook",
            "true",
            "TARGET_PARAMETER_UNSUPPORTED",
            "/Configuration/Os/OperatingSystem/Hooks",
        ),
        // Rejects: ECUC_Os_00307
        (
            false,
            "OperatingSystem",
            "OsStackMonitoring",
            "false",
            "TARGET_PARAMETER_UNSUPPORTED",
            "/Configuration/Os/OperatingSystem",
        ),
        // Rejects: ECUC_Os_00406
        (
            false,
            "OperatingSystem",
            "OsUseArti",
            "false",
            "TARGET_PARAMETER_UNSUPPORTED",
            "/Configuration/Os/OperatingSystem",
        ),
        // Rejects: ECUC_Os_00255
        (
            false,
            "SystemCounter",
            "OsCounterType",
            "HARDWARE",
            "COUNTER_PROFILE",
            "/Configuration/Os/SystemCounter",
        ),
        // Rejects: ECUC_Os_00011
        (
            false,
            "Autostart",
            "OsAlarmAutostartType",
            "ABSOLUTE",
            "TARGET_PARAMETER_UNSUPPORTED",
            "/Configuration/Os/Alarm_Work/Autostart",
        ),
        // Rejects: ECUC_Os_00056
        (
            true,
            "Autostart",
            "OsScheduleTableAutostartType",
            "SYNCHRON",
            "TARGET_PARAMETER_UNSUPPORTED",
            "/Configuration/Os/Table_Work/Autostart",
        ),
        (
            true,
            "Autostart",
            "OsScheduleTableAutostartType",
            "ABSOLUTE",
            "TARGET_PARAMETER_UNSUPPORTED",
            "/Configuration/Os/Table_Work/Autostart",
        ),
        // Rejects: ECUC_Os_00065
        (
            true,
            "Sync",
            "OsScheduleTblSyncStrategy",
            "EXPLICIT",
            "TARGET_PARAMETER_UNSUPPORTED",
            "/Configuration/Os/Table_Work/Sync",
        ),
        (
            true,
            "Sync",
            "OsScheduleTblSyncStrategy",
            "IMPLICIT",
            "TARGET_PARAMETER_UNSUPPORTED",
            "/Configuration/Os/Table_Work/Sync",
        ),
    ] {
        let mut sources = if tables {
            schedule_table_variant()
        } else {
            single_component_sources()
        };
        parameter(&mut sources, owner, name, value);
        rejected_at(&sources, code, "ecuc.arxml", object);
    }
    // OsLockTrustedFunctionCall has no effect without trusted functions.
    let mut unlocked = single_component_sources();
    parameter(
        &mut unlocked,
        "OperatingSystem",
        "OsLockTrustedFunctionCall",
        "false",
    );
    build(&unlocked).unwrap_or_else(|issues| panic!("{issues:?}"));
}

/// Explicit resources other than the virtual RES_SCHEDULER and schedule-table
/// task activation are refused at their own container.
#[test]
fn explicit_resources_and_table_task_activation_are_rejected() {
    let mut sources = single_component_sources();
    // Rejects: ECUC_Os_00252
    add_os_container(
        &mut sources,
        "UserResource",
        "OsResource",
        &format!(
            "<PARAMETER-VALUES><ECUC-TEXTUAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-ENUMERATION-PARAM-DEF\">{OS}/OsResource/OsResourceProperty</DEFINITION-REF><VALUE>STANDARD</VALUE></ECUC-TEXTUAL-PARAM-VALUE></PARAMETER-VALUES>"
        ),
    );
    rejected_at(
        &sources,
        "OS_RESOURCE_PROFILE",
        "ecuc.arxml",
        "/Configuration/Os/UserResource",
    );
    let mut sources = schedule_table_variant();
    // Rejects: ECUC_Os_00066
    add_to_container(
        &mut sources,
        "Point",
        &format!("{OS}/OsScheduleTable/OsScheduleTableExpiryPoint"),
        "OsScheduleTableTaskActivation",
        "",
        "",
    );
    rejected_with(
        &sources,
        ("REFERENCE_UNRESOLVED", "DEFINITION_UNKNOWN"),
        "ecuc.arxml",
        "/Configuration/Os/Table_Work/Point/OsScheduleTableTaskActivation",
    );
}

/// Appends a copy of the OS container `original` named `copy`.
fn copy_os_container(sources: &mut [InputSource], original: &str, copy: &str) {
    xml_edit(
        sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", original),
        |text| {
            let renamed = text.replacen(
                &format!("<SHORT-NAME>{original}</SHORT-NAME>"),
                &format!("<SHORT-NAME>{copy}</SHORT-NAME>"),
                1,
            );
            format!("{text}{renamed}")
        },
    );
}

/// The generator accepts the single-task SC1 shape of the reference ECU: one
/// Extended, fully preemptive task with one activation, three events and one
/// application mode; one 1 ms software counter; autostarted cyclic SetEvent
/// alarms or repeating one-point schedule tables. Other values of these
/// parameters are refused at the owning container.
#[test]
fn sc1_shapes_outside_the_reference_ecu_are_rejected() {
    const TASK: &str = "/Configuration/Os/Task_Ecu";
    const COUNTER: &str = "/Configuration/Os/SystemCounter";
    for (owner, name, value, code, object) in [
        // Rejects: ECUC_Os_00074
        ("Task_Ecu", "OsTaskActivation", "2", "TASK_PROFILE", TASK),
        // Rejects: ECUC_Os_00075
        ("Task_Ecu", "OsTaskPriority", "0", "TASK_PROFILE", TASK),
        ("Task_Ecu", "OsTaskPriority", "31", "TASK_PROFILE", TASK),
        // Rejects: ECUC_Os_00076
        ("Task_Ecu", "OsTaskSchedule", "NON", "TASK_PROFILE", TASK),
        // Rejects: ECUC_Os_00028
        (
            "SystemCounter",
            "OsCounterMinCycle",
            "2",
            "COUNTER_PROFILE",
            COUNTER,
        ),
        // Rejects: ECUC_Os_00029
        (
            "SystemCounter",
            "OsCounterTicksPerBase",
            "2",
            "COUNTER_PROFILE",
            COUNTER,
        ),
        // Rejects: ECUC_Os_00030
        (
            "SystemCounter",
            "OsSecondsPerTick",
            "0.01",
            "COUNTER_PROFILE",
            COUNTER,
        ),
    ] {
        let mut sources = single_component_sources();
        parameter(&mut sources, owner, name, value);
        rejected_at(&sources, code, "ecuc.arxml", object);
    }
    // The alarm period is the period of the RTE events mapped to it; a cycle
    // that differs is refused at the mapped event.
    // Rejects: ECUC_Os_00012
    let mut sources = single_component_sources();
    parameter(&mut sources, "Alarm_Work", "OsAlarmCycleTime", "2");
    rejected_at(
        &sources,
        "PERIOD_ALARM_CONFLICT",
        "bsw.arxml",
        "/AUTOSAR_Can/Can/Behavior/Can_MainFunction_BusOff_1ms",
    );
    // Rejects: ECUC_Os_00073
    let mut sources = single_component_sources();
    copy_os_container(&mut sources, "Task_Ecu", "Task_Spare");
    rejected_at(
        &sources,
        "SCHEDULE_NOT_UNIQUE",
        "ecuc.arxml",
        "/Configuration/Os/Task_Spare",
    );
    // Rejects: ECUC_Os_00026
    let mut sources = single_component_sources();
    copy_os_container(&mut sources, "SystemCounter", "SpareCounter");
    rejected_at(
        &sources,
        "SCHEDULE_NOT_UNIQUE",
        "ecuc.arxml",
        "/Configuration/Os/SpareCounter",
    );
    // Rejects: ECUC_Os_00078
    let mut sources = single_component_sources();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.has_tag_name("ECUC-REFERENCE-VALUE")
                && node.descendants().any(|child| {
                    child.has_tag_name("VALUE-REF")
                        && child.text() == Some("/Configuration/Os/Ev_IO")
                })
        },
        |_| String::new(),
    );
    rejected_at(&sources, "TASK_PROFILE", "ecuc.arxml", TASK);
    // Rejects: ECUC_Os_00081
    let mut sources = single_component_sources();
    copy_os_container(&mut sources, "OSDEFAULTAPPMODE", "ServiceMode");
    add_to_container(
        &mut sources,
        "Autostart",
        &format!("{OS}/OsTask/OsTaskAutostart"),
        "OsTaskAppModeRef",
        "REFERENCE:/Configuration/Os/ServiceMode",
        "",
    );
    rejected_at(
        &sources,
        "TASK_PROFILE",
        "ecuc.arxml",
        "/Configuration/Os/Task_Ecu/Autostart",
    );
    // Rejects: ECUC_Os_00003
    let mut sources = single_component_sources();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "Autostart")
                && node
                    .ancestors()
                    .any(|ancestor| named(ancestor, "ECUC-CONTAINER-VALUE", "Alarm_Work"))
        },
        |_| String::new(),
    );
    rejected_at(
        &sources,
        "PERIOD_ALARM_CONFLICT",
        "ecuc.arxml",
        "/Configuration/Os/Alarm_Work",
    );
    // Rejects: ECUC_Os_00144
    let mut sources = schedule_table_variant();
    parameter(
        &mut sources,
        "Table_Work",
        "OsScheduleTableRepeating",
        "false",
    );
    rejected_at(
        &sources,
        "PERIOD_SCHEDULE_TABLE_CONFLICT",
        "ecuc.arxml",
        "/Configuration/Os/Table_Work",
    );
    // Rejects: ECUC_Os_00143
    let mut sources = schedule_table_variant();
    copy_os_container(&mut sources, "Point", "SecondPoint");
    rejected_at(
        &sources,
        "PERIOD_SCHEDULE_TABLE_CONFLICT",
        "ecuc.arxml",
        "/Configuration/Os/Table_Work",
    );
    // Rejects: ECUC_Os_00059
    let mut sources = schedule_table_variant();
    copy_os_container(&mut sources, "SetEvent", "SecondSetEvent");
    rejected_at(
        &sources,
        "PERIOD_ALARM_CONFLICT",
        "ecuc.arxml",
        "/Configuration/Os/Table_Work/Point",
    );
}

/// The single application mode is the mandatory OSDEFAULTAPPMODE that the
/// generated ECU passes to StartOS; a mode under any other name is refused.
// Verifies: ECUC_Os_00022
#[test]
fn default_application_mode_is_required() {
    let mut sources = single_component_sources();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| node.has_tag_name("AUTOSAR"),
        |text| text.replace("OSDEFAULTAPPMODE", "NormalMode"),
    );
    rejected_at(
        &sources,
        "OS_DEFAULT_APP_MODE_REQUIRED",
        "ecuc.arxml",
        "/Configuration/Os/NormalMode",
    );
}

/// The generated OS holds only the scheduled alarms and tables, the owner
/// task's events and its autostart mode; any other object is refused at its
/// source rather than silently dropped.
#[test]
fn unused_os_objects_are_rejected() {
    let copy = |sources: &mut [InputSource], original: &str, copy: &str| {
        xml_edit(
            sources,
            "ecuc.arxml",
            |node| named(node, "ECUC-CONTAINER-VALUE", original),
            |text| {
                let renamed = text.replacen(
                    &format!("<SHORT-NAME>{original}</SHORT-NAME>"),
                    &format!("<SHORT-NAME>{copy}</SHORT-NAME>"),
                    1,
                );
                format!("{text}{renamed}")
            },
        );
    };
    for (original, extra) in [
        ("Alarm_Work", "Alarm_Spare"),
        ("Ev_Work", "Ev_Spare"),
        ("OSDEFAULTAPPMODE", "ServiceMode"),
    ] {
        let mut sources = single_component_sources();
        copy(&mut sources, original, extra);
        rejected_at(
            &sources,
            "OS_OBJECT_UNUSED",
            "ecuc.arxml",
            &format!("/Configuration/Os/{extra}"),
        );
    }
    let mut sources = schedule_table_variant();
    copy(&mut sources, "Table_Work", "Table_Spare");
    rejected_at(
        &sources,
        "SCHEDULE_TABLE_UNBOUND",
        "ecuc.arxml",
        "/Configuration/Os/Table_Spare",
    );
}
