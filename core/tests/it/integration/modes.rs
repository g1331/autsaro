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

#[test]
fn synchronized_mode_plan_shares_fanout_and_separates_publishers() {
    // Verifies: SWS_Rte_07535, SWS_Rte_02628, SWS_Rte_02629
    let sources = mode_binding_inputs();
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:#?}"));
    let composition = plan.description().composition.as_ref().unwrap();
    let machines = &composition.mode_machines;
    let primary = machines
        .iter()
        .find(|machine| machine.switch_symbol == "SchM_Switch_ComM_Primary")
        .unwrap();
    let secondary = machines
        .iter()
        .find(|machine| machine.switch_symbol == "SchM_Switch_ComM_Secondary")
        .unwrap();
    assert_eq!(primary.initial, 3);
    assert_eq!(primary.values, [("Idle".into(), 3), ("Active".into(), 7)]);
    assert_eq!(primary.readers.len(), 2);
    assert_eq!(secondary.readers.len(), 1);
    assert_ne!(primary.variable, secondary.variable);
    assert!(normal_validation(&sources).diagnostics.is_empty());
    let files = prepared_pipeline_files(&plan);
    for (name, bytes) in &files {
        if name == "generated/src/SchM_Main.c" {
            let source = std::str::from_utf8(bytes).unwrap();
            assert!(source.contains("SchM_Switch_ComM_Primary(3u)"));
            assert!(source.contains("SchM_Switch_ComM_Secondary(3u)"));
        }
    }
}

#[test]
fn synchronized_modes_reject_invalid_group_context_and_competing_publishers_at_source() {
    // Verifies: SWS_Rte_07564, SWS_Rte_CONSTR_09014
    let mut wrong_type = mode_binding_inputs();
    change(
        &mut wrong_type,
        "bsw.arxml",
        "/Types/PowerState</TYPE-TREF>",
        "/AUTOSAR_Dcm/DcmDiagnosticSessionControl</TYPE-TREF>",
    );
    rejected_at(
        &wrong_type,
        "MODE_MAPPING_INVALID",
        "process.arxml",
        "/Application/PowerMapping",
    );

    let mut wrong_context = mode_binding_inputs();
    change(
        &mut wrong_context,
        "process.arxml",
        "/Application/Process/Behavior</SWC-BEHAVIOR-REF>",
        "/Application/Ingress/Behavior</SWC-BEHAVIOR-REF>",
    );
    rejected_at(
        &wrong_context,
        "MODE_MAPPING_INVALID",
        "process.arxml",
        "/Application/PowerMapping",
    );

    let mut unselected = mode_binding_inputs();
    change(
        &mut unselected,
        "bsw.arxml",
        "<BEHAVIOR-REF DEST=\"BSW-INTERNAL-BEHAVIOR\">/AUTOSAR_ComM/ComM/Behavior</BEHAVIOR-REF>",
        "<BEHAVIOR-REF DEST=\"BSW-INTERNAL-BEHAVIOR\">/AUTOSAR_ComM/ComM/Unselected</BEHAVIOR-REF>",
    );
    // Add another behavior in the same module, retaining the selected producer's entries.
    let source = unselected
        .iter_mut()
        .find(|source| source.logical_path() == "bsw.arxml")
        .unwrap();
    let original = std::str::from_utf8(source.bytes()).unwrap();
    let module = original.find("<SHORT-NAME>ComM</SHORT-NAME>").unwrap();
    let at = module + original[module..].find("</INTERNAL-BEHAVIORS>").unwrap();
    let changed = format!(
        "{}<BSW-INTERNAL-BEHAVIOR><SHORT-NAME>Unselected</SHORT-NAME></BSW-INTERNAL-BEHAVIOR>{}",
        &original[..at],
        &original[at..]
    );
    *source = InputSource::new("bsw.arxml", changed.into_bytes()).unwrap();
    rejected_at(
        &unselected,
        "BSW_MAPPING_INVALID",
        "process.arxml",
        "/Application/PowerMapping",
    );

    let mut renamed = mode_binding_inputs();
    replace_all(
        &mut renamed,
        "/AUTOSAR_ComM/ComM/",
        "/AUTOSAR_ComM/AlternateComM/",
    );
    change(
        &mut renamed,
        "bsw.arxml",
        "<SHORT-NAME>ComM</SHORT-NAME>",
        "<SHORT-NAME>AlternateComM</SHORT-NAME>",
    );
    rejected_at(
        &renamed,
        "BSW_EXECUTION_CONFLICT",
        "process.arxml",
        "/Application/PowerMapping",
    );

    let mut aliased = mode_binding_inputs();
    change(
        &mut aliased,
        "process.arxml",
        "/AUTOSAR_ComM/ComM/Secondary</BSW-MODE-GROUP-REF>",
        "/AUTOSAR_ComM/ComM/Primary</BSW-MODE-GROUP-REF>",
    );
    rejected_at(
        &aliased,
        "MODE_MULTIPLE_PUBLISHERS",
        "process.arxml",
        "/Application/Process/Secondary",
    );

    let mut dual = mode_binding_inputs();
    change(
        &mut dual,
        "process.arxml",
        "<SYMBOL>Process_Periodic</SYMBOL>",
        r#"<MODE-SWITCH-POINTS><MODE-SWITCH-POINT><SHORT-NAME>SwitchPrimary</SHORT-NAME><MODE-GROUP-IREF><CONTEXT-P-PORT-REF DEST="P-PORT-PROTOTYPE">/Application/Process/Primary</CONTEXT-P-PORT-REF><TARGET-MODE-GROUP-REF DEST="MODE-DECLARATION-GROUP-PROTOTYPE">/Types/PowerInterface/Mode</TARGET-MODE-GROUP-REF></MODE-GROUP-IREF></MODE-SWITCH-POINT></MODE-SWITCH-POINTS><SYMBOL>Process_Periodic</SYMBOL>"#,
    );
    rejected_at(
        &dual,
        "MODE_MULTIPLE_PUBLISHERS",
        "process.arxml",
        "/Application/Process/Behavior/Periodic/SwitchPrimary",
    );

    let mut collision = mode_binding_inputs();
    change(
        &mut collision,
        "types.arxml",
        "<SHORT-NAME>Input</SHORT-NAME>",
        "<SHORT-NAME>Rte_Mode_First_Mode</SHORT-NAME>",
    );
    rejected_at(
        &collision,
        "CONTRACT_NAME_COLLISION",
        "types.arxml",
        "/Types/ScalarService/Transform",
    );

    let mut malformed = swc_managed_mode_inputs();
    change(
        &mut malformed,
        "process.arxml",
        "<MODE-GROUP-IREF>",
        "<MODE-GROUP-IREF><P-MODE-GROUP-IN-ATOMIC-SWC-INSTANCE-REF>",
    );
    change(
        &mut malformed,
        "process.arxml",
        "</MODE-GROUP-IREF>",
        "</P-MODE-GROUP-IN-ATOMIC-SWC-INSTANCE-REF></MODE-GROUP-IREF>",
    );
    rejected_at(
        &malformed,
        "MODE_MAPPING_INVALID",
        "process.arxml",
        "/Application/Process/Behavior/Periodic/SwitchPrimary",
    );

    let mut enhanced = mode_binding_inputs();
    change(
        &mut enhanced,
        "ingress.arxml",
        "<ENHANCED-MODE-API>false</ENHANCED-MODE-API>",
        "<ENHANCED-MODE-API>true</ENHANCED-MODE-API>",
    );
    rejected_at(
        &enhanced,
        "MODE_API_UNSUPPORTED",
        "ingress.arxml",
        "/Application/Ingress/First",
    );
}

#[test]
fn rebinding_a_mode_reader_invalidates_the_previous_contract_preview() {
    let sources = mode_binding_inputs();
    let plan = build(&sources).unwrap();
    let files = plan.component_contract_files().unwrap();
    let scratch = Scratch::new();
    let output = scratch.0.join("contracts");
    let initial = files.preview(&output).unwrap();
    files
        .generate_previewed(&output, &initial.revision)
        .unwrap();
    let previous = std::fs::read(output.join("contract.json")).unwrap();
    let preview = files.preview(&output).unwrap();
    let mut rebound = sources;
    change(
        &mut rebound,
        "composition.arxml",
        "/Application/Process/Primary</TARGET-P-PORT-REF>",
        "/Application/Process/Secondary</TARGET-P-PORT-REF>",
    );
    let plan = build(&rebound).unwrap();
    let files = plan.component_contract_files().unwrap();
    assert_ne!(files.preview(&output).unwrap().revision, preview.revision);
    assert!(
        files
            .generate_previewed(&output, &preview.revision)
            .is_err()
    );
    assert_eq!(
        std::fs::read(output.join("contract.json")).unwrap(),
        previous
    );
}

#[test]
fn alphabetical_modes_use_declaration_values_without_native_comm_translation() {
    let mut sources = mode_binding_inputs();
    change(
        &mut sources,
        "types.arxml",
        "<CATEGORY>EXPLICIT_ORDER</CATEGORY>",
        "<CATEGORY>ALPHABETIC_ORDER</CATEGORY>",
    );
    let plan = build(&sources).unwrap();
    let machine = plan
        .description()
        .composition
        .as_ref()
        .unwrap()
        .mode_machines
        .iter()
        .find(|machine| machine.switch_symbol == "SchM_Switch_ComM_Primary")
        .unwrap();
    assert_eq!(machine.values, [("Active".into(), 0), ("Idle".into(), 1)]);
    assert_eq!(machine.initial, 1);
}

#[test]
fn explicit_dcm_synchronization_reuses_the_existing_backend_machine() {
    let mut sources = inputs();
    change(
        &mut sources,
        "services.arxml",
        "</ELEMENTS>",
        r#"
<SWC-BSW-MAPPING><SHORT-NAME>SessionMapping</SHORT-NAME><BSW-BEHAVIOR-REF DEST="BSW-INTERNAL-BEHAVIOR">/AUTOSAR_Dcm/Dcm/Behavior</BSW-BEHAVIOR-REF><SWC-BEHAVIOR-REF DEST="SWC-INTERNAL-BEHAVIOR">/Services/DcmService/Behavior</SWC-BEHAVIOR-REF><SYNCHRONIZED-MODE-GROUPS><SWC-BSW-SYNCHRONIZED-MODE-GROUP-PROTOTYPE><BSW-MODE-GROUP-REF DEST="MODE-DECLARATION-GROUP-PROTOTYPE">/AUTOSAR_Dcm/Dcm/DcmDiagnosticSessionControl</BSW-MODE-GROUP-REF><SWC-MODE-GROUP-IREF><CONTEXT-P-PORT-REF DEST="P-PORT-PROTOTYPE">/Services/DcmService/DiagnosticSessionControlModeSwitchInterface</CONTEXT-P-PORT-REF><TARGET-MODE-GROUP-REF DEST="MODE-DECLARATION-GROUP-PROTOTYPE">/Services/Dcm_DiagnosticSessionControlModeSwitchInterface/diagnosticSession</TARGET-MODE-GROUP-REF></SWC-MODE-GROUP-IREF></SWC-BSW-SYNCHRONIZED-MODE-GROUP-PROTOTYPE></SYNCHRONIZED-MODE-GROUPS></SWC-BSW-MAPPING>
</ELEMENTS>"#,
    );
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    let composition = plan.description().composition.as_ref().unwrap();
    let session = composition
        .mode_machines
        .iter()
        .find(|machine| machine.switch_symbol == "SchM_Switch_Dcm_DcmDiagnosticSessionControl")
        .unwrap();
    assert!(session.bsw_managed);
    assert_eq!(session.variable, "diagnostic_session_mode");
    assert_eq!(session.mapping.as_deref(), Some("/Services/SessionMapping"));
    let files = prepared_pipeline_files(&plan);
    let description = &files
        .iter()
        .find(|(name, _)| name == "generated/descriptions/Host_Implementation.arxml")
        .unwrap()
        .1;
    let document = roxmltree::Document::parse(std::str::from_utf8(description).unwrap()).unwrap();
    assert_eq!(
        document
            .descendants()
            .filter(|node| node.has_tag_name("BSW-MODE-GROUP-REF")
                && node.text() == Some("/AUTOSAR_Dcm/Dcm/DcmDiagnosticSessionControl"))
            .count(),
        1
    );
}
