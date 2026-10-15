//! Source-local rejections for the selected Dcm configuration.
use super::single_component::{add_to_container, rejected_at, rejected_with};
use super::*;

/// The selected Dcm configuration must refuse optional behaviour before rendering C.
#[test]
fn dcm_features_outside_the_selection_are_refused_at_their_source() {
    let disabled = "TARGET_PARAMETER_UNSUPPORTED";
    for (owner, parameter_name, value, code, object) in [
        // Rejects: ECUC_Dcm_00823
        (
            "General",
            "DcmDevErrorDetect",
            "true",
            disabled,
            "/Configuration/Dcm/General",
        ),
        // Rejects: ECUC_Dcm_01187
        (
            "General",
            "DcmEnableSecurityEventReporting",
            "true",
            disabled,
            "/Configuration/Dcm/General",
        ),
        // Rejects: ECUC_Dcm_00821
        (
            "General",
            "DcmVersionInfoApi",
            "true",
            disabled,
            "/Configuration/Dcm/General",
        ),
        // Rejects: ECUC_Dcm_00600
        (
            "General",
            "DcmRespondAllRequest",
            "true",
            disabled,
            "/Configuration/Dcm/General",
        ),
        // Rejects: ECUC_Dcm_00776
        (
            "PageBuffer",
            "DcmPagedBufferEnabled",
            "true",
            disabled,
            "/Configuration/Dcm/Config/PageBuffer",
        ),
        // Rejects: ECUC_Dcm_00914
        (
            "ResponsePolicy",
            "DcmDslDiagRespOnSecondDeclinedRequest",
            "true",
            disabled,
            "/Configuration/Dcm/Config/Dsl/ResponsePolicy",
        ),
        // Rejects: ECUC_Dcm_01110
        (
            "UdsOnCan",
            "DcmDslProtocolType",
            "DCM_OBD_ON_CAN",
            "SERVICE_UNSUPPORTED",
            "/Configuration/Dcm/Config/Dsl/Protocols/UdsOnCan",
        ),
        // Rejects: ECUC_Dcm_00710
        (
            "Receive",
            "DcmDslProtocolRxAddrType",
            "DCM_FUNCTIONAL_TYPE",
            "SERVICE_UNSUPPORTED",
            "/Configuration/Dcm/Config/Dsl/Protocols/UdsOnCan",
        ),
        // Rejects: ECUC_Dcm_00713
        (
            "ApplicationValue",
            "DcmDspDataUsePort",
            "USE_DATA_ASYNCH_CLIENT_SERVER",
            "SERVICE_TYPE_CONFLICT",
            "/Configuration/Dcm/Config/Dsp/ApplicationValue",
        ),
        // Rejects: ECUC_Dcm_00985
        (
            "ApplicationValue",
            "DcmDspDataType",
            "UINT8_DYN",
            "SERVICE_TYPE_CONFLICT",
            "/Configuration/Dcm/Config/Dsp/ApplicationValue",
        ),
        // Rejects: ECUC_Dcm_00955
        (
            "ApplicationValue",
            "DcmDspDataConditionCheckReadFncUsed",
            "true",
            "SERVICE_TYPE_CONFLICT",
            "/Configuration/Dcm/Config/Dsp/ApplicationValue",
        ),
        // Rejects: ECUC_Dcm_01061
        (
            "Dsp",
            "DcmDspEnableObdMirror",
            "true",
            disabled,
            "/Configuration/Dcm/Config/Dsp",
        ),
        // Rejects: ECUC_Dcm_00805
        (
            "ApplicationDid",
            "DcmDspDidUsed",
            "false",
            disabled,
            "/Configuration/Dcm/Config/Dsp/ApplicationDid",
        ),
        // Rejects: ECUC_Dcm_00612
        (
            "ApplicationDidInfo",
            "DcmDspDidDynamicallyDefined",
            "true",
            "SERVICE_UNSUPPORTED",
            "/Configuration/Dcm/Config/Dsp/ApplicationDidInfo",
        ),
    ] {
        let mut sources = single_component_sources();
        parameter(&mut sources, owner, parameter_name, value);
        rejected_at(&sources, code, "ecuc.arxml", object);
    }
    let root = "/AUTOSAR/EcucDefs/Dcm";
    let owners = [
        (
            "General",
            format!("{root}/DcmGeneral"),
            "/Configuration/Dcm/General",
        ),
        (
            "Dsp",
            format!("{root}/DcmConfigSet/DcmDsp"),
            "/Configuration/Dcm/Config/Dsp",
        ),
        (
            "SessionControl",
            format!("{root}/DcmConfigSet/DcmDsd/DcmDsdServiceTable/DcmDsdService"),
            "/Configuration/Dcm/Config/Dsd/Services/SessionControl",
        ),
    ];
    for (owner, name, value) in [
        // Rejects: ECUC_Dcm_01240
        ("General", "DcmS3ServerTimeoutOverwrite", "FLOAT:6"),
        // Rejects: ECUC_Dcm_01151
        ("Dsp", "DcmDspAuthentication", ""),
        // Rejects: ECUC_Dcm_00937
        ("Dsp", "DcmDspDidRange", ""),
        // Rejects: ECUC_Dcm_00777
        (
            "SessionControl",
            "DcmDsdSidTabFnc",
            "FUNCTION-NAME:Custom_Service",
        ),
    ] {
        let (_, path, object) = owners.iter().find(|row| row.0 == owner).unwrap();
        let mut sources = single_component_sources();
        add_to_container(&mut sources, owner, path, name, value, "");
        if value.is_empty() || name == "DcmDsdSidTabFnc" {
            let object = if value.is_empty() {
                format!("{object}/{name}")
            } else {
                (*object).to_owned()
            };
            rejected_with(
                &sources,
                ("REFERENCE_UNRESOLVED", "DEFINITION_UNKNOWN"),
                "ecuc.arxml",
                &object,
            );
        } else {
            rejected_at(
                &sources,
                "TARGET_PARAMETER_UNSUPPORTED",
                "ecuc.arxml",
                object,
            );
        }
    }
}

/// Absent optional containers/parameters are rejected at their own definition, not ignored.
#[test]
fn dcm_optional_interfaces_require_an_explicitly_supported_configuration() {
    let root = "/AUTOSAR/EcucDefs/Dcm";
    for (owner, suffix, object, name, value) in [
        // Rejects: ECUC_Dcm_00932
        (
            "Config",
            "DcmConfigSet",
            "/Configuration/Dcm/Config",
            "DcmProcessingConditions",
            "",
        ),
        // Rejects: ECUC_Dcm_00681
        (
            "Dsd",
            "DcmConfigSet/DcmDsd",
            "/Configuration/Dcm/Config/Dsd",
            "DcmDsdServiceRequestManufacturerNotification",
            "",
        ),
        // Rejects: ECUC_Dcm_00816
        (
            "Dsd",
            "DcmConfigSet/DcmDsd",
            "/Configuration/Dcm/Config/Dsd",
            "DcmDsdServiceRequestSupplierNotification",
            "",
        ),
        // Rejects: ECUC_Dcm_00984
        (
            "General",
            "DcmGeneral",
            "/Configuration/Dcm/General",
            "DcmVinRef",
            "REFERENCE:/Configuration/Dcm/Config/Dsp/ApplicationDid",
        ),
        // Rejects: ECUC_Dcm_00971
        (
            "General",
            "DcmGeneral",
            "/Configuration/Dcm/General",
            "DcmDDDIDStorage",
            "BOOLEAN:true",
        ),
        // Rejects: ECUC_Dcm_01019
        (
            "General",
            "DcmGeneral",
            "/Configuration/Dcm/General",
            "DcmHeaderFileInclusion",
            "STRING:Custom.h",
        ),
        // Rejects: ECUC_Dcm_01245
        (
            "General",
            "DcmGeneral",
            "/Configuration/Dcm/General",
            "DcmSecureCoding",
            "",
        ),
        // Rejects: ECUC_Dcm_01188
        (
            "General",
            "DcmGeneral",
            "/Configuration/Dcm/General",
            "DcmSecurityEventRefs",
            "",
        ),
        // Rejects: ECUC_Dcm_00966
        (
            "Dsp",
            "DcmConfigSet/DcmDsp",
            "/Configuration/Dcm/Config/Dsp",
            "DcmDspDDDIDcheckPerSourceDID",
            "BOOLEAN:true",
        ),
        // Rejects: ECUC_Dcm_01081
        (
            "UdsOnCan",
            "DcmConfigSet/DcmDsl/DcmDslProtocol/DcmDslProtocolRow",
            "/Configuration/Dcm/Config/Dsl/Protocols/UdsOnCan",
            "DcmDspProtocolEcuAddr",
            "INTEGER:1",
        ),
        // Rejects: ECUC_Dcm_01099
        (
            "ApplicationDid",
            "DcmConfigSet/DcmDsp/DcmDspDid",
            "/Configuration/Dcm/Config/Dsp/ApplicationDid",
            "DcmDspDidSize",
            "INTEGER:4",
        ),
        // Rejects: ECUC_Dcm_01102
        (
            "ApplicationDid",
            "DcmConfigSet/DcmDsp/DcmDspDid",
            "/Configuration/Dcm/Config/Dsp/ApplicationDid",
            "DcmDspDidSupportInfo",
            "",
        ),
        // Rejects: ECUC_Dcm_01104
        (
            "ApplicationValue",
            "DcmConfigSet/DcmDsp/DcmDspData",
            "/Configuration/Dcm/Config/Dsp/ApplicationValue",
            "DcmDspDidDataSupportInfo",
            "",
        ),
        // Rejects: ECUC_Dcm_00900
        (
            "Dsp",
            "DcmConfigSet/DcmDsp",
            "/Configuration/Dcm/Config/Dsp",
            "DcmDspComControl",
            "",
        ),
        // Rejects: ECUC_Dcm_01025
        (
            "Dsp",
            "DcmConfigSet/DcmDsp",
            "/Configuration/Dcm/Config/Dsp",
            "DcmDspCommonAuthorization",
            "",
        ),
        // Rejects: ECUC_Dcm_01111
        (
            "Dsp",
            "DcmConfigSet/DcmDsp",
            "/Configuration/Dcm/Config/Dsp",
            "DcmDspEcuReset",
            "",
        ),
        // Rejects: ECUC_Dcm_00784
        (
            "Dsp",
            "DcmConfigSet/DcmDsp",
            "/Configuration/Dcm/Config/Dsp",
            "DcmDspMemory",
            "",
        ),
        // Rejects: ECUC_Dcm_01132
        (
            "Dsp",
            "DcmConfigSet/DcmDsp",
            "/Configuration/Dcm/Config/Dsp",
            "DcmDspMemoryTransfer",
            "",
        ),
        // Rejects: ECUC_Dcm_00626
        (
            "Dsp",
            "DcmConfigSet/DcmDsp",
            "/Configuration/Dcm/Config/Dsp",
            "DcmDspPid",
            "",
        ),
        // Rejects: ECUC_Dcm_00637
        (
            "Dsp",
            "DcmConfigSet/DcmDsp",
            "/Configuration/Dcm/Config/Dsp",
            "DcmDspRequestControl",
            "",
        ),
        // Rejects: ECUC_Dcm_01034
        (
            "Dsp",
            "DcmConfigSet/DcmDsp",
            "/Configuration/Dcm/Config/Dsp",
            "DcmDspRequestFileTransfer",
            "",
        ),
        // Rejects: ECUC_Dcm_00858
        (
            "Dsp",
            "DcmConfigSet/DcmDsp",
            "/Configuration/Dcm/Config/Dsp",
            "DcmDspRoe",
            "",
        ),
        // Rejects: ECUC_Dcm_00640
        (
            "Dsp",
            "DcmConfigSet/DcmDsp",
            "/Configuration/Dcm/Config/Dsp",
            "DcmDspRoutine",
            "",
        ),
        // Rejects: ECUC_Dcm_00630
        (
            "Dsp",
            "DcmConfigSet/DcmDsp",
            "/Configuration/Dcm/Config/Dsp",
            "DcmDspVehInfo",
            "",
        ),
        // Rejects: ECUC_Dcm_00957
        (
            "Dsp",
            "DcmConfigSet/DcmDsp",
            "/Configuration/Dcm/Config/Dsp",
            "DcmDspPeriodicTransmission",
            "",
        ),
    ] {
        let path = format!("{root}/{suffix}");
        let mut sources = single_component_sources();
        add_to_container(&mut sources, owner, &path, name, value, "");
        let location = if value.is_empty() {
            format!("{object}/{name}")
        } else {
            object.to_owned()
        };
        if name == "DcmDspDidSize" {
            rejected_at(
                &sources,
                "TARGET_PARAMETER_UNSUPPORTED",
                "ecuc.arxml",
                &location,
            );
        } else {
            rejected_with(
                &sources,
                ("REFERENCE_UNRESOLVED", "DEFINITION_UNKNOWN"),
                "ecuc.arxml",
                &location,
            );
        }
    }
}

#[test]
fn dcm_data_interfaces_keep_the_selected_synchronous_opaque_contract() {
    // Rejects: ECUC_Dcm_00713
    for port in [
        "USE_DATA_ASYNCH_CLIENT_SERVER",
        "USE_DATA_ASYNCH_CLIENT_SERVER_ERROR",
        "USE_DATA_ASYNCH_FNC",
        "USE_DATA_ASYNCH_FNC_ERROR",
        "USE_DATA_ASYNCH_FNC_PROXY",
        "USE_DATA_SENDER_RECEIVER",
        "USE_DATA_SENDER_RECEIVER_AS_SERVICE",
        "USE_DATA_SYNCH_FNC",
        "USE_DATA_SYNCH_FNC_PROXY",
        "USE_ECU_SIGNAL",
    ] {
        let mut sources = single_component_sources();
        parameter(&mut sources, "ApplicationValue", "DcmDspDataUsePort", port);
        rejected_at(
            &sources,
            "SERVICE_TYPE_CONFLICT",
            "ecuc.arxml",
            "/Configuration/Dcm/Config/Dsp/ApplicationValue",
        );
    }
    // Rejects: ECUC_Dcm_00985
    for kind in [
        "BOOLEAN",
        "FLOAT",
        "FLOAT_N",
        "SINT8",
        "SINT8_N",
        "SINT16",
        "SINT16_N",
        "SINT32",
        "SINT32_N",
        "UINT8",
        "UINT8_DYN",
        "UINT16",
        "UINT16_N",
        "UINT32",
        "UINT32_N",
    ] {
        let mut sources = single_component_sources();
        parameter(&mut sources, "ApplicationValue", "DcmDspDataType", kind);
        rejected_at(
            &sources,
            "SERVICE_TYPE_CONFLICT",
            "ecuc.arxml",
            "/Configuration/Dcm/Config/Dsp/ApplicationValue",
        );
    }
    // Rejects: ECUC_Dcm_01122
    for port in [
        "USE_ATOMIC_BNDM",
        "USE_ATOMIC_NV_DATA_INTERFACE",
        "USE_ATOMIC_SENDER_RECEIVER_INTERFACE",
        "USE_ATOMIC_SENDER_RECEIVER_INTERFACE_AS_SERVICE",
    ] {
        let mut sources = single_component_sources();
        parameter(&mut sources, "ApplicationDid", "DcmDspDidUsePort", port);
        rejected_at(
            &sources,
            "TARGET_PARAMETER_UNSUPPORTED",
            "ecuc.arxml",
            "/Configuration/Dcm/Config/Dsp/ApplicationDid",
        );
    }
    // Rejects: ECUC_Dcm_00986
    for endian in [Some("BIG_ENDIAN"), Some("LITTLE_ENDIAN"), None] {
        let mut sources = single_component_sources();
        if let Some(value) = endian {
            parameter(
                &mut sources,
                "ApplicationValue",
                "DcmDspDataEndianness",
                value,
            );
        } else {
            xml_edit(
                &mut sources,
                "ecuc.arxml",
                |node| {
                    node.has_tag_name("ECUC-TEXTUAL-PARAM-VALUE")
                        && node.children().any(|child| {
                            child.has_tag_name("DEFINITION-REF")
                                && child
                                    .text()
                                    .is_some_and(|text| text.ends_with("/DcmDspDataEndianness"))
                        })
                },
                |_| String::new(),
            );
        }
        rejected_at(
            &sources,
            "SERVICE_TYPE_CONFLICT",
            "ecuc.arxml",
            "/Configuration/Dcm/Config/Dsp/ApplicationValue",
        );
    }
    // Rejects: ECUC_Dcm_00611
    let mut sources = single_component_sources();
    parameter(
        &mut sources,
        "ApplicationValueInfo",
        "DcmDspDataScalingInfoSize",
        "1",
    );
    rejected_at(
        &sources,
        "TARGET_PARAMETER_UNSUPPORTED",
        "ecuc.arxml",
        "/Configuration/Dcm/Config/Dsp/ApplicationValueInfo",
    );
    // Rejects: ECUC_Dcm_01231
    let mut sources = single_component_sources();
    add_to_container(
        &mut sources,
        "Value",
        "/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsp/DcmDspDid/DcmDspDidSignal",
        "DcmDspDidSignalCompositePool",
        "",
        "",
    );
    rejected_with(
        &sources,
        ("REFERENCE_UNRESOLVED", "DEFINITION_UNKNOWN"),
        "ecuc.arxml",
        "/Configuration/Dcm/Config/Dsp/ApplicationDid/Value/DcmDspDidSignalCompositePool",
    );
}

#[test]
fn dcm_transport_resources_and_boot_selection_are_bounded_at_the_source() {
    for (name, kind, renamed, code, object) in [
        // Rejects: ECUC_Dcm_00695
        (
            "UdsOnCan",
            "DcmDslProtocolRow",
            "OtherProtocol",
            "DIAGNOSTIC_REFERENCE",
            "/Configuration/Dcm/Config/Dsl/Protocols/OtherProtocol/Connection/Physical",
        ),
        // Rejects: ECUC_Dcm_00705
        (
            "Connection",
            "DcmDslConnection",
            "OtherConnection",
            "DIAGNOSTIC_REFERENCE",
            "/Configuration/Dcm/Config/Dsl/Protocols/UdsOnCan/OtherConnection/Physical",
        ),
        // Rejects: ECUC_Dcm_00601
        (
            "ApplicationDid",
            "DcmDspDid",
            "OtherDid",
            "DIAGNOSTIC_NOT_UNIQUE",
            "/Configuration/Dcm/Config/Dsp/OtherDid",
        ),
    ] {
        let mut sources = single_component_sources();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                named(node, "ECUC-CONTAINER-VALUE", name)
                    && node.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF")
                            && child
                                .text()
                                .is_some_and(|text| text.ends_with(&format!("/{kind}")))
                    })
            },
            |text| {
                let copy = text.replacen(
                    &format!("<SHORT-NAME>{name}</SHORT-NAME>"),
                    &format!("<SHORT-NAME>{renamed}</SHORT-NAME>"),
                    1,
                );
                format!("{text}{copy}")
            },
        );
        rejected_at(&sources, code, "ecuc.arxml", object);
    }
    // Rejects: ECUC_Dcm_00711
    let mut sources = single_component_sources();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "Transmit")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/DcmDslProtocolTx"))
                })
        },
        |_| String::new(),
    );
    rejected_at(
        &sources,
        "DIAGNOSTIC_NOT_UNIQUE",
        "ecuc.arxml",
        "/Configuration/Dcm/Config/Dsl/Protocols/UdsOnCan",
    );
    // Rejects: ECUC_Dcm_00815
    let mut sources = single_component_sources();
    parameter(
        &mut sources,
        "DCM_EXTENDED_DIAGNOSTIC_SESSION",
        "DcmDspSessionForBoot",
        "DCM_OEM_BOOT",
    );
    rejected_at(
        &sources,
        "TARGET_PARAMETER_UNSUPPORTED",
        "ecuc.arxml",
        "/Configuration/Dcm/Config/Dsp/Sessions/DCM_EXTENDED_DIAGNOSTIC_SESSION",
    );
}

#[test]
fn dcm_access_filters_and_referenced_dids_are_refused_before_generation() {
    let root = "/AUTOSAR/EcucDefs/Dcm/DcmConfigSet";
    for (owner, suffix, object, parameter_name, destination) in [
        // Rejects: ECUC_Dcm_00734
        (
            "SessionControl",
            "DcmDsd/DcmDsdServiceTable/DcmDsdService",
            "/Configuration/Dcm/Config/Dsd/Services/SessionControl",
            "DcmDsdSidTabSessionLevelRef",
            "/Configuration/Dcm/Config/Dsp/Sessions/DCM_DEFAULT_SESSION",
        ),
        // Rejects: ECUC_Dcm_00733
        (
            "SessionControl",
            "DcmDsd/DcmDsdServiceTable/DcmDsdService",
            "/Configuration/Dcm/Config/Dsd/Services/SessionControl",
            "DcmDsdSidTabSecurityLevelRef",
            "/Configuration/Dcm/Config/Dsp/Sessions/DCM_DEFAULT_SESSION",
        ),
        // Rejects: ECUC_Dcm_00918
        (
            "SessionControl",
            "DcmDsd/DcmDsdServiceTable/DcmDsdService",
            "/Configuration/Dcm/Config/Dsd/Services/SessionControl",
            "DcmDsdSidTabModeRuleRef",
            "/Configuration/Dcm/Config/Dsp/Sessions/DCM_DEFAULT_SESSION",
        ),
        // Rejects: ECUC_Dcm_00804
        (
            "Default",
            "DcmDsd/DcmDsdServiceTable/DcmDsdService/DcmDsdSubService",
            "/Configuration/Dcm/Config/Dsd/Services/SessionControl/Default",
            "DcmDsdSubServiceSessionLevelRef",
            "/Configuration/Dcm/Config/Dsp/Sessions/DCM_DEFAULT_SESSION",
        ),
        // Rejects: ECUC_Dcm_00812
        (
            "Default",
            "DcmDsd/DcmDsdServiceTable/DcmDsdService/DcmDsdSubService",
            "/Configuration/Dcm/Config/Dsd/Services/SessionControl/Default",
            "DcmDsdSubServiceSecurityLevelRef",
            "/Configuration/Dcm/Config/Dsp/Sessions/DCM_DEFAULT_SESSION",
        ),
        // Rejects: ECUC_Dcm_00924
        (
            "Default",
            "DcmDsd/DcmDsdServiceTable/DcmDsdService/DcmDsdSubService",
            "/Configuration/Dcm/Config/Dsd/Services/SessionControl/Default",
            "DcmDsdSubServiceModeRuleRef",
            "/Configuration/Dcm/Config/Dsp/Sessions/DCM_DEFAULT_SESSION",
        ),
        // Rejects: ECUC_Dcm_00614
        (
            "Read",
            "DcmDsp/DcmDspDidInfo/DcmDspDidRead",
            "/Configuration/Dcm/Config/Dsp/ApplicationDidInfo/Read",
            "DcmDspDidReadSecurityLevelRef",
            "/Configuration/Dcm/Config/Dsp/Sessions/DCM_DEFAULT_SESSION",
        ),
        // Rejects: ECUC_Dcm_00917
        (
            "Read",
            "DcmDsp/DcmDspDidInfo/DcmDspDidRead",
            "/Configuration/Dcm/Config/Dsp/ApplicationDidInfo/Read",
            "DcmDspDidReadModeRuleRef",
            "/Configuration/Dcm/Config/Dsp/Sessions/DCM_DEFAULT_SESSION",
        ),
        // Rejects: ECUC_Dcm_00606
        (
            "ApplicationDid",
            "DcmDsp/DcmDspDid",
            "/Configuration/Dcm/Config/Dsp/ApplicationDid",
            "DcmDspDidRef",
            "/Configuration/Dcm/Config/Dsp/ApplicationDid",
        ),
    ] {
        let mut sources = single_component_sources();
        add_to_container(
            &mut sources,
            owner,
            &format!("{root}/{suffix}"),
            parameter_name,
            &format!("REFERENCE:{destination}"),
            "",
        );
        if matches!(
            parameter_name,
            "DcmDsdSidTabSessionLevelRef" | "DcmDsdSidTabSecurityLevelRef"
        ) {
            rejected_at(
                &sources,
                "TARGET_PARAMETER_UNSUPPORTED",
                "ecuc.arxml",
                object,
            );
        } else {
            rejected_with(
                &sources,
                ("REFERENCE_UNRESOLVED", "DEFINITION_UNKNOWN"),
                "ecuc.arxml",
                object,
            );
        }
    }
}

/// A structurally complete SecurityAccess row cannot enable the removed security stack.
/// Rejects: ECUC_Dcm_00759
#[test]
fn dcm_security_rows_with_all_mandatory_fields_are_rejected() {
    let path = "/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsp/DcmDspSecurity";
    let mut sources = single_component_sources();
    add_to_container(&mut sources, "Security", path, "DcmDspSecurityRow", "", "");
    for (name, value) in [
        ("DcmDspSecurityLevel", "INTEGER:1"),
        ("DcmDspSecuritySeedSize", "INTEGER:4"),
        ("DcmDspSecurityKeySize", "INTEGER:4"),
        ("DcmDspSecurityAttemptCounterEnabled", "BOOLEAN:false"),
        (
            "DcmDspSecurityUsePort",
            "ENUMERATION:USE_ASYNCH_CLIENT_SERVER",
        ),
        ("DcmDspSecurityDelayTime", "FLOAT:0"),
        ("DcmDspSecurityDelayTimeOnBoot", "FLOAT:0"),
    ] {
        add_to_container(
            &mut sources,
            "DcmDspSecurityRow",
            &format!("{path}/DcmDspSecurityRow"),
            name,
            value,
            "",
        );
    }
    rejected_at(
        &sources,
        "TARGET_PARAMETER_UNSUPPORTED",
        "ecuc.arxml",
        "/Configuration/Dcm/Config/Dsp/Security/DcmDspSecurityRow",
    );
}

/// The no-OBD interpretation of F6xx cannot be extended to an OBD-relevant ECU.
#[test]
fn dcm_refuses_obd_relevant_dem_declarations_at_their_source() {
    assert!(build(&single_component_sources()).is_ok());
    for support in [
        "DEM_OBD_MASTER_ECU",
        "DEM_OBD_PRIMARY_ECU",
        "DEM_OBD_DEP_SEC_ECU",
    ] {
        let mut sources = single_component_sources();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                named(node, "ECUC-CONTAINER-VALUE", "General")
                    && node.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF")
                            && child.text() == Some("/AUTOSAR/EcucDefs/Dem/DemGeneral")
                    })
            },
            |text| text.replace("DEM_OBD_NO_OBD_SUPPORT", support),
        );
        rejected_at(
            &sources,
            "SERVICE_UNSUPPORTED",
            "ecuc.arxml",
            "/Configuration/Dem/General",
        );
    }
}
