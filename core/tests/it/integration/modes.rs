//! Mode management configuration of the mainline.
use super::single_component::{add_to_container, rejected_at, rejected_with};
use super::*;
use autosar_config_core::integration::InputSource;

#[test]
fn mode_configuration_preserves_channel_users_rules_and_static_callouts() {
    let plan = build(&inputs()).unwrap();
    let mode = plan.description().mode_runtime.as_ref().unwrap();
    assert_eq!(mode.channel, "/Configuration/ComM/Config/Host");
    assert_eq!(mode.channel_handle, 0);
    assert_eq!(mode.main_symbol, "ComM_MainFunction_Host");
    assert_eq!((mode.period_ms, mode.minimum_full_ms), (1, 5));
    assert_eq!(mode.users.len(), 1);
    assert_eq!(mode.users[0].handle, 0);
    assert_eq!(mode.include, "Ecu_HostBusSM.h");
    assert_eq!(
        mode.rules.iter().map(|rule| rule.mode).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(mode.dcm_connections.len(), 1);
    assert!(mode.rules.iter().all(|rule| {
        rule.callout
            .starts_with("Ecu_HostBusSM_ApplyMode(0u, COMM_")
    }));
    let mut sources = inputs();
    replace_all(&mut sources, "/Config/Host/", "/Config/Vehicle/");
    change(
        &mut sources,
        "ecuc.arxml",
        "<SHORT-NAME>Host</SHORT-NAME>",
        "<SHORT-NAME>Vehicle</SHORT-NAME>",
    );
    replace_all(&mut sources, "/Config/Host<", "/Config/Vehicle<");
    replace_all(
        &mut sources,
        "ComM_MainFunction_Host",
        "ComM_MainFunction_Vehicle",
    );
    parameter(&mut sources, "Vehicle", "ComMChannelId", "7");
    parameter(&mut sources, "HostUser", "ComMUserIdentifier", "19");
    parameter(&mut sources, "General", "ComMEcuGroupClassification", "1");
    replace_all(
        &mut sources,
        "Ecu_HostBusSM_ApplyMode(0u,",
        "Ecu_HostBusSM_ApplyMode(7u,",
    );
    let renamed = build(&sources).unwrap();
    let mode = renamed.description().mode_runtime.as_ref().unwrap();
    assert_eq!(mode.channel, "/Configuration/ComM/Config/Vehicle");
    assert_eq!(mode.main_symbol, "ComM_MainFunction_Vehicle");
    assert_eq!(mode.channel_handle, 7);
    assert_eq!(mode.users[0].handle, 19);
    assert!(normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn selected_mode_configuration_rejects_unbound_features_and_dynamic_callouts() {
    for (owner, field, new_value, code) in [
        (
            "Host",
            "ComMBusType",
            "COMM_BUS_TYPE_CAN",
            "MODE_BUS_PROVIDER",
        ),
        (
            "Host",
            "ComMCDDBusPrefix",
            "OtherBusSM",
            "MODE_BUS_PROVIDER",
        ),
        (
            "NetworkManagement",
            "ComMNmVariant",
            "FULL",
            "MODE_NM_VARIANT",
        ),
        ("Host", "ComMMainFunctionPeriod", "0.002", "MODE_TIMEBASE"),
        (
            "CurrentMode",
            "BswMRequestProcessing",
            "BSWM_DEFERRED",
            "MODE_PROCESSING",
        ),
        (
            "Includes",
            "BswMUserIncludeFile",
            "../Ecu_HostBusSM.h",
            "MODE_INCLUDE",
        ),
        (
            "NoRule",
            "BswMNestedExecutionOnly",
            "true",
            "MODE_FEATURE_UNSUPPORTED",
        ),
        (
            "NoList",
            "BswMActionListExecution",
            "BSWM_TRIGGER",
            "MODE_RULE",
        ),
        (
            "FullAction",
            "BswMUserCalloutFunction",
            "Ecu_HostBusSM_ApplyMode(0u, currentMode)",
            "MODE_STATIC_CALLOUT",
        ),
        (
            "SilentAction",
            "BswMUserCalloutFunction",
            "Ecu_HostBusSM_ApplyMode(0u, COMM_FULL_COMMUNICATION)",
            "MODE_STATIC_CALLOUT",
        ),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, owner, field, new_value);
        rejects_in_both(&sources, code);
    }
}

/// Every enabled ComM or BswM feature outside the selection is refused at the container
/// that holds its switch, before any C is rendered.
#[test]
fn mode_features_outside_the_selection_are_refused_at_their_source() {
    let comm_general = "/Configuration/ComM/General";
    let channel = "/Configuration/ComM/Config/Host";
    let bswm_general = "/Configuration/BswM/General";
    for (owner, definition, object) in [
        // Rejects: ECUC_ComM_00555
        ("General", "ComMDevErrorDetect", comm_general),
        // Rejects: ECUC_ComM_00895
        (
            "General",
            "ComMDynamicPncToChannelMappingSupport",
            comm_general,
        ),
        // Rejects: ECUC_ComM_00560
        ("General", "ComMModeLimitationEnabled", comm_general),
        // Rejects: ECUC_ComM_00839
        ("General", "ComMPncSupport", comm_general),
        // Rejects: ECUC_ComM_00558
        ("General", "ComMResetAfterForcingNoComm", comm_general),
        // Rejects: ECUC_ComM_00695
        ("General", "ComMSynchronousWakeUp", comm_general),
        // Rejects: ECUC_ComM_00622
        ("General", "ComMVersionInfoApi", comm_general),
        // Rejects: ECUC_ComM_00559
        ("General", "ComMWakeupInhibitionEnabled", comm_general),
        // Rejects: ECUC_ComM_00787
        ("Host", "ComMFullCommRequestNotificationEnabled", channel),
        // Rejects: ECUC_ComM_00571
        ("Host", "ComMNoCom", channel),
        // Rejects: ECUC_ComM_00569
        ("Host", "ComMNoWakeup", channel),
        // Rejects: ECUC_ComM_00789
        ("Host", "ComMNoWakeUpInhibitionNvmStorage", channel),
        // Rejects: ECUC_BswM_00938
        ("General", "BswMCanSMEnabled", bswm_general),
        // Rejects: ECUC_BswM_00940
        ("General", "BswMDcmEnabled", bswm_general),
        // Rejects: ECUC_BswM_00811
        ("General", "BswMDevErrorDetect", bswm_general),
        // Rejects: ECUC_BswM_00941
        ("General", "BswMEcuMEnabled", bswm_general),
        // Rejects: ECUC_BswM_01072
        ("General", "BswMEthIfEnabled", bswm_general),
        // Rejects: ECUC_BswM_00942
        ("General", "BswMEthSMEnabled", bswm_general),
        // Rejects: ECUC_BswM_00943
        ("General", "BswMFrSMEnabled", bswm_general),
        // Rejects: ECUC_BswM_00949
        ("General", "BswMGenericRequestEnabled", bswm_general),
        // Rejects: ECUC_BswM_00987
        ("General", "BswMJ1939DcmEnabled", bswm_general),
        // Rejects: ECUC_BswM_00965
        ("General", "BswMJ1939NmEnabled", bswm_general),
        // Rejects: ECUC_BswM_00944
        ("General", "BswMLinSMEnabled", bswm_general),
        // Rejects: ECUC_BswM_00945
        ("General", "BswMLinTPEnabled", bswm_general),
        // Rejects: ECUC_BswM_01071
        ("General", "BswMNmEnabled", bswm_general),
        // Rejects: ECUC_BswM_00946
        ("General", "BswMNvMEnabled", bswm_general),
        // Rejects: ECUC_BswM_01093
        ("General", "BswMSdControlEnabled", bswm_general),
        // Rejects: ECUC_BswM_01047
        ("General", "BswMSdEnabled", bswm_general),
        // Rejects: ECUC_BswM_00812
        ("General", "BswMVersionInfoApi", bswm_general),
        // Rejects: ECUC_BswM_00935
        (
            "NoRule",
            "BswMNestedExecutionOnly",
            "/Configuration/BswM/Config/Arbitration/NoRule",
        ),
        // Rejects: ECUC_BswM_00902
        (
            "NoList",
            "BswMAbortOnFail",
            "/Configuration/BswM/Config/Control/NoList/Item",
        ),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, owner, definition, "true");
        rejected_at(&sources, "MODE_FEATURE_UNSUPPORTED", "ecuc.arxml", object);
    }
    let mut sources = inputs();
    // Rejects: ECUC_ComM_00568
    parameter(&mut sources, "NetworkManagement", "ComMNmVariant", "LIGHT");
    rejected_at(
        &sources,
        "MODE_NM_VARIANT",
        "ecuc.arxml",
        &format!("{channel}/NetworkManagement"),
    );
    let mut sources = inputs();
    // Rejects: ECUC_ComM_00567
    parameter(&mut sources, "Host", "ComMBusType", "COMM_BUS_TYPE_CAN");
    rejected_at(&sources, "MODE_BUS_PROVIDER", "ecuc.arxml", channel);
    let mut sources = inputs();
    // Rejects: ECUC_BswM_00822
    parameter(
        &mut sources,
        "CurrentMode",
        "BswMRequestProcessing",
        "BSWM_DEFERRED",
    );
    rejected_at(
        &sources,
        "MODE_PROCESSING",
        "ecuc.arxml",
        "/Configuration/BswM/Config/Arbitration/CurrentMode",
    );
}

/// Inserts a reference value into the container whose definition reference ends the
/// anchor, before that container's sub-containers.
fn insert_reference(sources: &mut [InputSource], anchor: &str, definition: &str, target: &str) {
    let source = sources
        .iter_mut()
        .find(|source| source.logical_path() == "ecuc.arxml")
        .unwrap();
    let text = std::str::from_utf8(source.bytes()).unwrap();
    let at = text.find(anchor).unwrap();
    let at = at + text[at..].find("<SUB-CONTAINERS>").unwrap();
    let reference = format!(
        "<REFERENCE-VALUES><ECUC-REFERENCE-VALUE><DEFINITION-REF DEST=\"ECUC-REFERENCE-DEF\">{definition}</DEFINITION-REF><VALUE-REF DEST=\"ECUC-CONTAINER-VALUE\">{target}</VALUE-REF></ECUC-REFERENCE-VALUE></REFERENCE-VALUES>"
    );
    let changed = format!("{}{reference}{}", &text[..at], &text[at..]);
    *source = InputSource::new("ecuc.arxml", changed.into_bytes()).unwrap();
}

/// BswM and ComM structures outside the selection are refused where they are configured:
/// other rule, list and condition values, other sources, actions and ports, and
/// parameters the runtime does not consume.
#[test]
fn mode_structures_outside_the_selection_are_refused_at_their_source() {
    let arbitration = "/Configuration/BswM/Config/Arbitration";
    let control = "/Configuration/BswM/Config/Control";
    for (owner, definition, value, code, object) in [
        // Rejects: ECUC_BswM_00815
        (
            "NoCondition",
            "BswMConditionType",
            "BSWM_EQUALS_NOT",
            "MODE_RULE",
            format!("{arbitration}/NoCondition"),
        ),
        // Rejects: ECUC_BswM_00932
        (
            "Initial",
            "BswMBswModeInitValue",
            "COMM_SILENT_COMMUNICATION",
            "MODE_INITIAL_VALUE",
            format!("{arbitration}/CurrentMode/Initial"),
        ),
        // Rejects: ECUC_BswM_00894
        (
            "NoList",
            "BswMActionListExecution",
            "BSWM_TRIGGER",
            "MODE_RULE",
            format!("{control}/NoList"),
        ),
        // Rejects: ECUC_BswM_00843
        (
            "FullAction",
            "BswMUserCalloutFunction",
            "Ecu_HostBusSM_ApplyMode(0u, currentMode)",
            "MODE_STATIC_CALLOUT",
            format!("{control}/FullAction/Available/Callout"),
        ),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, owner, definition, value);
        rejected_at(&sources, code, "ecuc.arxml", &object);
    }
    let root = "/AUTOSAR/EcucDefs/BswM/BswMConfig";
    for (owner, path, name, value, object) in [
        // Rejects: ECUC_BswM_01076
        (
            "NoList",
            format!("{root}/BswMModeControl/BswMActionList"),
            "BswMActionListPriority",
            "INTEGER:1",
            format!("{control}/NoList"),
        ),
        // Rejects: ECUC_BswM_00814
        (
            "NoExpression",
            format!("{root}/BswMArbitration/BswMLogicalExpression"),
            "BswMLogicalOperator",
            "ENUMERATION:BSWM_OR",
            format!("{arbitration}/NoExpression"),
        ),
        // Rejects: ECUC_BswM_00818
        (
            "NoRule",
            format!("{root}/BswMArbitration/BswMRule"),
            "BswMRuleFalseActionList",
            "REFERENCE:/Configuration/BswM/Config/Control/SilentList",
            format!("{arbitration}/NoRule"),
        ),
    ] {
        let mut sources = inputs();
        add_to_container(&mut sources, owner, &path, name, value, "");
        rejected_at(&sources, "MODE_RULE", "ecuc.arxml", &object);
    }
    let mut sources = inputs();
    // Rejects: ECUC_BswM_00813
    add_to_container(
        &mut sources,
        "General",
        "/AUTOSAR/EcucDefs/BswM/BswMGeneral",
        "BswMMainFunctionPeriod",
        "FLOAT:0.01",
        "",
    );
    rejected_at(
        &sources,
        "TARGET_PARAMETER_UNSUPPORTED",
        "ecuc.arxml",
        "/Configuration/BswM/General",
    );
    // A rule sharing another rule's action list gets that list's callout for another mode.
    let mut sources = inputs();
    // Rejects: ECUC_BswM_00817
    change(
        &mut sources,
        "ecuc.arxml",
        "<VALUE-REF DEST=\"ECUC-CONTAINER-VALUE\">/Configuration/BswM/Config/Control/NoList</VALUE-REF>",
        "<VALUE-REF DEST=\"ECUC-CONTAINER-VALUE\">/Configuration/BswM/Config/Control/SilentList</VALUE-REF>",
    );
    rejected_at(
        &sources,
        "MODE_STATIC_CALLOUT",
        "ecuc.arxml",
        &format!("{control}/SilentAction/Available/Callout"),
    );
    let mut sources = inputs();
    // Rejects: ECUC_BswM_00825
    change(
        &mut sources,
        "ecuc.arxml",
        "<VALUE-REF DEST=\"ECUC-CONTAINER-VALUE\">/Configuration/BswM/Config/Control/NoAction</VALUE-REF>",
        "<VALUE-REF DEST=\"ECUC-CONTAINER-VALUE\">/Configuration/BswM/Config/Control/SilentList</VALUE-REF>",
    );
    rejected_at(
        &sources,
        "MODE_REFERENCE",
        "ecuc.arxml",
        &format!("{control}/NoList/Item"),
    );
    // Definitions outside the built-in metadata are unknown where they appear.
    for (owner, path, name, value, object) in [
        // Rejects: ECUC_ComM_00786
        (
            "HostUser",
            "/AUTOSAR/EcucDefs/ComM/ComMConfigSet/ComMUser".to_owned(),
            "ComMUserEcucPartitionRef",
            "REFERENCE:/Configuration/EcuC/Partitions/OwnerPartition",
            "/Configuration/ComM/Config/HostUser".to_owned(),
        ),
        // Rejects: ECUC_BswM_01050
        (
            "Item",
            format!("{root}/BswMModeControl/BswMActionList/BswMActionListItem"),
            "BswMReportFailRuntimeErrorId",
            "INTEGER:128",
            format!("{control}/NoList/Item"),
        ),
        // Rejects: ECUC_BswM_00826
        (
            "Available",
            format!("{root}/BswMModeControl/BswMAction/BswMAvailableActions"),
            "BswMComMAllowCom",
            "",
            format!("{control}/NoAction/Available/BswMComMAllowCom"),
        ),
        // Rejects: ECUC_BswM_00856
        (
            "Source",
            format!("{root}/BswMArbitration/BswMModeRequestPort/BswMModeRequestSource"),
            "BswMSwcModeRequest",
            "",
            format!("{arbitration}/CurrentMode/Source/BswMSwcModeRequest"),
        ),
        // Rejects: ECUC_BswM_01052
        (
            "Arbitration",
            format!("{root}/BswMArbitration"),
            "BswMEventRequestPort",
            "",
            format!("{arbitration}/BswMEventRequestPort"),
        ),
        // Rejects: ECUC_BswM_00816
        (
            "Expected",
            format!("{root}/BswMArbitration/BswMModeCondition/BswMConditionValue"),
            "BswMModeDeclaration",
            "",
            format!("{arbitration}/NoCondition/Expected/BswMModeDeclaration"),
        ),
        // Rejects: ECUC_BswM_00802
        (
            "Control",
            format!("{root}/BswMModeControl"),
            "BswMSwitchPort",
            "",
            format!("{control}/BswMSwitchPort"),
        ),
    ] {
        let mut sources = inputs();
        add_to_container(&mut sources, owner, &path, name, value, "");
        rejected_with(
            &sources,
            ("REFERENCE_UNRESOLVED", "DEFINITION_UNKNOWN"),
            "ecuc.arxml",
            &object,
        );
    }
    for (anchor, definition, target, object) in [
        // Rejects: ECUC_BswM_00895
        (
            "/AUTOSAR/EcucDefs/BswM/BswMConfig</DEFINITION-REF>",
            "/AUTOSAR/EcucDefs/BswM/BswMConfig/BswMPartitionRef",
            "/Configuration/EcuC/Partitions/OwnerPartition",
            "/Configuration/BswM/Config",
        ),
        // Rejects: ECUC_ComM_00893
        (
            "/AUTOSAR/EcucDefs/ComM/ComMConfigSet/ComMChannel</DEFINITION-REF>",
            "/AUTOSAR/EcucDefs/ComM/ComMConfigSet/ComMChannel/ComMManageReference",
            "/Configuration/ComM/Config/Host",
            "/Configuration/ComM/Config/Host",
        ),
        // Rejects: ECUC_ComM_00894
        (
            "/AUTOSAR/EcucDefs/ComM/ComMConfigSet/ComMChannel</DEFINITION-REF>",
            "/AUTOSAR/EcucDefs/ComM/ComMConfigSet/ComMChannel/ComMChannelPartitionRef",
            "/Configuration/EcuC/Partitions/OwnerPartition",
            "/Configuration/ComM/Config/Host",
        ),
    ] {
        let mut sources = inputs();
        insert_reference(&mut sources, anchor, definition, target);
        rejected_with(
            &sources,
            ("REFERENCE_UNRESOLVED", "DEFINITION_UNKNOWN"),
            "ecuc.arxml",
            object,
        );
    }
}
