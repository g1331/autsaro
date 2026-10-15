//! Mode management configuration of the mainline.
use super::*;

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
