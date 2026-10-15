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
        |node| named(node, "ECUC-CONTAINER-VALUE", "DefaultMode"),
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
            "REFERENCE:/Configuration/Os/DefaultMode",
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
        ("DefaultMode", "ServiceMode"),
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
