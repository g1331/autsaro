//! Dcm services, DIDs and diagnostic routes of the mainline.
use super::*;
use autosar_config_core::integration::InputSource;
use std::path::Path;

#[test]
fn dcm_bridge_requires_actual_service_identity_and_follows_renames() {
    for (before, after) in [
        (
            "<SERVICE-KIND>DIAGNOSTIC-COMMUNICATION-MANAGER</SERVICE-KIND>",
            "<SERVICE-KIND>COM-MANAGER</SERVICE-KIND>",
        ),
        (
            "<IS-SERVICE>true</IS-SERVICE>",
            "<IS-SERVICE>false</IS-SERVICE>",
        ),
    ] {
        let mut sources = inputs();
        change(&mut sources, "types.arxml", before, after);
        rejects_in_both(&sources, "SERVICE_CLIENT_MISSING");
    }
    let mut sources = inputs();
    replace_all(
        &mut sources,
        "/Services/DcmService/DataServices_ApplicationValue",
        "/Services/DcmService/IncorrectPort",
    );
    change(
        &mut sources,
        "services.arxml",
        "<SHORT-NAME>DataServices_ApplicationValue</SHORT-NAME>",
        "<SHORT-NAME>IncorrectPort</SHORT-NAME>",
    );
    rejects_in_both(&sources, "SERVICE_CLIENT_MISSING");
    let mut sources = inputs();
    replace_all(&mut sources, "ApplicationValue", "VehicleValue");
    replace_all(&mut sources, "DcmService", "DiagnosticClient");
    // The application provider port is identified by its connector, not by the DID name.
    replace_all(
        &mut sources,
        "/Application/Ingress/VehicleValue",
        "/Application/Ingress/SnapshotRead",
    );
    change(
        &mut sources,
        "ingress.arxml",
        "<SHORT-NAME>VehicleValue</SHORT-NAME>",
        "<SHORT-NAME>SnapshotRead</SHORT-NAME>",
    );
    let plan = build(&sources).unwrap();
    assert_eq!(
        plan.description().diagnostic.as_ref().unwrap().client_port,
        "/Services/DiagnosticClient/DataServices_VehicleValue"
    );
    assert!(normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn network_routing_is_checked_without_optional_did() {
    let mut sources = sources_without_application_did(false);
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert!(plan.description().diagnostic.is_none());
    assert!(plan.description().diagnostic_transport.is_none());
    assert!(
        !plan
            .description()
            .schedule
            .entities
            .iter()
            .any(|entity| matches!(
                entity.symbol.as_str(),
                "CanTp_MainFunction" | "Dcm_MainFunction"
            ))
    );
    assert_eq!(plan.description().routes.len(), 2);
    assert!(normal_validation(&sources).diagnostics.is_empty());
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "RxValue")
                && node.children().any(|child| {
                    child
                        .text()
                        .is_some_and(|text| text.ends_with("/PduRRoutingPath"))
                })
        },
        |_| String::new(),
    );
    rejects_in_both(&sources, "PDU_ROUTE_NOT_UNIQUE");
}

#[test]
fn ordinary_application_client_sharing_did_server_is_not_the_dcm_bridge() {
    let mut sources = inputs();
    let port = subtree(
        &sources,
        "services.arxml",
        "R-PORT-PROTOTYPE",
        "DataServices_ApplicationValue",
    );
    let call = subtree(
        &sources,
        "services.arxml",
        "SYNCHRONOUS-SERVER-CALL-POINT",
        "ReadApplicationValue",
    )
    .replace("/Services/DcmService", "/Application/Observe");
    change(
        &mut sources,
        "observe.arxml",
        "</PORTS>",
        &format!("{port}</PORTS>"),
    );
    change(
        &mut sources,
        "observe.arxml",
        "</SERVER-CALL-POINTS>",
        &format!("{call}</SERVER-CALL-POINTS>"),
    );
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "Diagnostic",
    )
    .replace(
        "<SHORT-NAME>Diagnostic</SHORT-NAME>",
        "<SHORT-NAME>ApplicationDiagnostic</SHORT-NAME>",
    )
    .replace(
        "/Application/Pipeline/DcmService",
        "/Application/Pipeline/ObserveInstance",
    )
    .replace("/Services/DcmService", "/Application/Observe");
    change(
        &mut sources,
        "composition.arxml",
        "</CONNECTORS>",
        &format!("{connector}</CONNECTORS>"),
    );
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(
        plan.description().diagnostic.as_ref().unwrap().client_port,
        "/Services/DcmService/DataServices_ApplicationValue"
    );
    assert_eq!(
        plan.description()
            .composition
            .as_ref()
            .unwrap()
            .connections
            .iter()
            .filter(|connection| connection.service
                && connection.provider.port == "/Application/Ingress/ApplicationValue")
            .count(),
        2
    );
    assert!(normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn two_real_dcm_requesters_are_rejected_without_dangling_references() {
    let mut sources = inputs();
    let component = subtree(
        &sources,
        "services.arxml",
        "SERVICE-SW-COMPONENT-TYPE",
        "DcmService",
    )
    .replace("DcmService", "DcmSecond");
    change(
        &mut sources,
        "services.arxml",
        "</ELEMENTS>",
        &format!("{component}</ELEMENTS>"),
    );
    let prototype = subtree(
        &sources,
        "composition.arxml",
        "SW-COMPONENT-PROTOTYPE",
        "DcmService",
    )
    .replace("DcmService", "DcmSecond");
    change(
        &mut sources,
        "composition.arxml",
        "</COMPONENTS>",
        &format!("{prototype}</COMPONENTS>"),
    );
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "Diagnostic",
    )
    .replace(
        "<SHORT-NAME>Diagnostic</SHORT-NAME>",
        "<SHORT-NAME>SecondDiagnostic</SHORT-NAME>",
    )
    .replace("DcmService", "DcmSecond");
    change(
        &mut sources,
        "composition.arxml",
        "</CONNECTORS>",
        &format!("{connector}</CONNECTORS>"),
    );
    let mapping = "<COMPONENT-IREF><CONTEXT-COMPOSITION-REF DEST=\"ROOT-SW-COMPOSITION-PROTOTYPE\">/Extract/ReferenceExtract/RootComposition</CONTEXT-COMPOSITION-REF><TARGET-COMPONENT-REF DEST=\"SW-COMPONENT-PROTOTYPE\">/Application/Pipeline/DcmSecond</TARGET-COMPONENT-REF></COMPONENT-IREF>";
    change(
        &mut sources,
        "extract.arxml",
        "</COMPONENT-IREFS>",
        &format!("{mapping}</COMPONENT-IREFS>"),
    );
    let rte_instance = subtree(&sources, "ecuc.arxml", "ECUC-CONTAINER-VALUE", "DcmService");
    change(
        &mut sources,
        "ecuc.arxml",
        &rte_instance,
        &format!(
            "{}{}",
            rte_instance,
            rte_instance.replace("DcmService", "DcmSecond")
        ),
    );
    rejects_in_both(&sources, "SERVICE_CLIENT_MISSING");
}

#[test]
fn required_dcm_channel_reference_is_strict_for_every_builtin_project() {
    use autosar_config_core::project_model::ValidationStatus;
    let template = template_inputs();
    let validation = normal_validation(&template);
    assert!(validation.diagnostics.is_empty());
    assert_eq!(validation.status, ValidationStatus::Passed);
    let plan = build(&template).unwrap();
    assert_eq!(
        plan.description().profile,
        autosar_config_core::integration::PROFILE
    );
    assert!(plan.description().mode_runtime.is_some());
    assert!(plan.component_contract_files().is_ok());
    // Historical single-component sources configure neither ComM nor BswM.
    // Built-in validation no longer relaxes their Dcm channel dependency.
    let fixtures =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/echo-application/positive");
    let historical: Vec<_> = [
        "application.arxml",
        "bsw.arxml",
        "ecuc.arxml",
        "extract.arxml",
        "services.arxml",
        "types.arxml",
    ]
    .into_iter()
    .map(|file| InputSource::new(file, std::fs::read(fixtures.join(file)).unwrap()).unwrap())
    .collect();
    let validation = normal_validation(&historical);
    assert!(
        validation
            .diagnostics
            .iter()
            .any(|issue| issue.code == "MULTIPLICITY"
                && issue.witness.as_ref().is_some_and(|witness| format!(
                    "{:?}",
                    witness.constraint
                )
                .contains("DcmDslProtocolComMChannelRef"))),
        "{:?}",
        validation.diagnostics
    );
    assert!(
        !validation
            .coverage
            .iter()
            .any(|rule| rule.rule_id == "native.definition.legacy-dcm-mode-dependency")
    );
    assert!(build(&historical).is_err());
    let mut composition = inputs();
    xml_edit(
        &mut composition,
        "ecuc.arxml",
        |node| {
            node.tag_name().name() == "ECUC-REFERENCE-VALUE"
                && node.children().any(|child| {
                    child.tag_name().name() == "DEFINITION-REF"
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/DcmDslProtocolComMChannelRef"))
                })
        },
        |_| String::new(),
    );
    assert!(
        normal_validation(&composition)
            .diagnostics
            .iter()
            .any(|issue| issue.code == "MULTIPLICITY")
    );
    assert!(build(&composition).is_err());
    let source = historical
        .iter()
        .find(|source| source.logical_path() == "ecuc.arxml")
        .unwrap();
    let text = std::str::from_utf8(source.bytes()).unwrap();
    let document = roxmltree::Document::parse(text).unwrap();
    let dcm = document
        .descendants()
        .find(|node| named(*node, "ECUC-MODULE-CONFIGURATION-VALUES", "Dcm"))
        .unwrap();
    let standalone = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Configuration</SHORT-NAME><ELEMENTS>{}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>",
        &text[dcm.range()]
    );
    let standalone = [InputSource::new("dcm.arxml", standalone.into_bytes()).unwrap()];
    let validation = normal_validation(&standalone);
    assert!(
        validation
            .diagnostics
            .iter()
            .any(|issue| issue.code == "MULTIPLICITY"
                && issue.witness.as_ref().is_some_and(|witness| format!(
                    "{:?}",
                    witness.constraint
                )
                .contains("DcmDslProtocolComMChannelRef"))),
        "{:?}",
        validation.diagnostics
    );
}

#[test]
fn internal_active_session_did_cannot_be_replaced_by_application_source() {
    let mut sources = inputs();
    parameter(
        &mut sources,
        "ApplicationDid",
        "DcmDspDidIdentifier",
        "61830",
    );
    rejects_in_both(&sources, "DIAGNOSTIC_IDENTIFIER");
    let issues = build(&sources).err().expect("reserved DID refused");
    let message = issues
        .iter()
        .find(|issue| issue.code == "DIAGNOSTIC_IDENTIFIER")
        .unwrap()
        .message
        .to_string();
    assert!(
        message.contains("0xF186") && message.contains("reserved") && message.contains("internal"),
        "{message}"
    );
    let validation = normal_validation(&sources);
    let message = validation
        .diagnostics
        .iter()
        .find(|issue| issue.code == "DIAGNOSTIC_IDENTIFIER")
        .unwrap()
        .message
        .to_string();
    assert!(
        message.contains("0xF186") && message.contains("reserved") && message.contains("internal"),
        "{message}"
    );
    let catalog: serde_json::Value =
        serde_json::from_str(include_str!("../../../src/messages.json")).unwrap();
    assert!(
        catalog["zh-CN"]["backend.integration.diagnostic.did_identifier_reserved"]
            .as_str()
            .unwrap()
            .contains("保留")
    );
}

#[test]
fn selected_dcm_subfunction_availability_matches_actual_service_dispatch() {
    for (service, available) in [
        ("SessionControl", "false"),
        ("TesterPresent", "false"),
        ("ReadDataByIdentifier", "true"),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, service, "DcmDsdSidTabSubfuncAvail", available);
        rejects_in_both(&sources, "SERVICE_UNSUPPORTED");
    }
}

#[test]
fn dcm_service_table_offers_exactly_session_read_and_tester_present() {
    let services = "/Configuration/Dcm/Config/Dsd/Services";
    let tester = subtree(
        &inputs(),
        "ecuc.arxml",
        "ECUC-CONTAINER-VALUE",
        "TesterPresent",
    );
    // SecurityAccess (0x27) and RoutineControl (0x31) have no mainline handler,
    // so an added service is refused at its own container instead of generated.
    for (name, id) in [("SecurityAccess", "39"), ("RoutineControl", "49")] {
        let added = tester
            .replacen(
                "<SHORT-NAME>TesterPresent</SHORT-NAME>",
                &format!("<SHORT-NAME>{name}</SHORT-NAME>"),
                1,
            )
            .replacen("<VALUE>62</VALUE>", &format!("<VALUE>{id}</VALUE>"), 1);
        let mut sources = inputs();
        change(
            &mut sources,
            "ecuc.arxml",
            &tester,
            &format!("{tester}{added}"),
        );
        service_refused_at(&sources, &format!("{services}/{name}"));
    }
    let mut sources = inputs();
    change(&mut sources, "ecuc.arxml", &tester, "");
    service_refused_at(&sources, services);
}

fn service_refused_at(sources: &[InputSource], object: &str) {
    let issues = build(sources)
        .err()
        .unwrap_or_else(|| panic!("{object} accepted"));
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "SERVICE_UNSUPPORTED"
                && issue.object.as_deref() == Some(object)),
        "{issues:?}"
    );
    let validation = normal_validation(sources);
    assert!(
        validation
            .diagnostics
            .iter()
            .any(|issue| issue.code == "SERVICE_UNSUPPORTED"
                && issue.file.as_deref() == Some("ecuc.arxml")
                && issue.path.as_deref() == Some(object)),
        "{:?}",
        validation.diagnostics
    );
}

#[test]
fn dcm_service_mode_source_requires_the_fixed_standard_contract() {
    let plan = build(&inputs()).unwrap();
    let service = plan
        .description()
        .composition
        .as_ref()
        .unwrap()
        .components
        .iter()
        .find(|component| component.component == "/Services/DcmService")
        .unwrap();
    let mode = service.diagnostic_session_port.as_ref().unwrap();
    assert_eq!(
        mode.port,
        "/Services/DcmService/DiagnosticSessionControlModeSwitchInterface"
    );
    assert_eq!(
        mode.prototype,
        "/Services/Dcm_DiagnosticSessionControlModeSwitchInterface/diagnosticSession"
    );
    assert_eq!(mode.mode_group, "/AUTOSAR_Dcm/DcmDiagnosticSessionControl");
    for (file, tag, name, before, after) in [
        (
            "services.arxml",
            "MODE-SWITCH-INTERFACE",
            "Dcm_DiagnosticSessionControlModeSwitchInterface",
            "<IS-SERVICE>true</IS-SERVICE>",
            "<IS-SERVICE>false</IS-SERVICE>",
        ),
        (
            "services.arxml",
            "P-PORT-PROTOTYPE",
            "DiagnosticSessionControlModeSwitchInterface",
            "<ENHANCED-MODE-API>false</ENHANCED-MODE-API>",
            "<ENHANCED-MODE-API>true</ENHANCED-MODE-API>",
        ),
        (
            "bsw.arxml",
            "MODE-DECLARATION-GROUP",
            "DcmDiagnosticSessionControl",
            "<ON-TRANSITION-VALUE>255</ON-TRANSITION-VALUE>",
            "<ON-TRANSITION-VALUE>254</ON-TRANSITION-VALUE>",
        ),
        (
            "bsw.arxml",
            "MODE-DECLARATION-GROUP",
            "DcmDiagnosticSessionControl",
            "<CATEGORY>EXPLICIT_ORDER</CATEGORY>",
            "<CATEGORY>ALPHABETIC_ORDER</CATEGORY>",
        ),
        (
            "bsw.arxml",
            "MODE-DECLARATION",
            "DCM_PROGRAMMING_SESSION",
            "<VALUE>1</VALUE>",
            "<VALUE>4</VALUE>",
        ),
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            file,
            |node| named(node, tag, name),
            |text| {
                assert!(text.contains(before));
                text.replace(before, after)
            },
        );
        rejects_in_both(&sources, "TYPE_CONFLICT");
    }
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "bsw.arxml",
        |node| named(node, "AR-PACKAGE", "AUTOSAR_Dcm"),
        |text| {
            text.replacen("</ELEMENTS>", "<MODE-DECLARATION-GROUP><SHORT-NAME>ForeignModes</SHORT-NAME><CATEGORY>EXPLICIT_ORDER</CATEGORY><INITIAL-MODE-REF DEST=\"MODE-DECLARATION\">/AUTOSAR_Dcm/ForeignModes/DCM_DEFAULT_SESSION</INITIAL-MODE-REF><MODE-DECLARATIONS><MODE-DECLARATION><SHORT-NAME>DCM_DEFAULT_SESSION</SHORT-NAME><VALUE>0</VALUE></MODE-DECLARATION></MODE-DECLARATIONS><ON-TRANSITION-VALUE>255</ON-TRANSITION-VALUE></MODE-DECLARATION-GROUP></ELEMENTS>", 1)
        },
    );
    xml_edit(
        &mut sources,
        "bsw.arxml",
        |node| {
            named(
                node,
                "MODE-DECLARATION-GROUP",
                "DcmDiagnosticSessionControl",
            )
        },
        |text| {
            text.replace(
                "/AUTOSAR_Dcm/DcmDiagnosticSessionControl/DCM_DEFAULT_SESSION</INITIAL-MODE-REF>",
                "/AUTOSAR_Dcm/ForeignModes/DCM_DEFAULT_SESSION</INITIAL-MODE-REF>",
            )
        },
    );
    rejects_in_both(&sources, "TYPE_CONFLICT");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "services.arxml",
        |node| {
            named(
                node,
                "P-PORT-PROTOTYPE",
                "SecurityAccessModeSwitchInterface",
            )
        },
        |_| String::new(),
    );
    rejects_in_both(&sources, "SERVICE_TYPE_CONFLICT");
    for original in ["DCM_DEFAULT_SESSION", "DCM_EXTENDED_DIAGNOSTIC_SESSION"] {
        let mut sources = inputs();
        let source = sources
            .iter_mut()
            .find(|source| source.logical_path() == "ecuc.arxml")
            .unwrap();
        // Preserve all reference identities while editing only the ECUC row.
        replace_all(
            std::slice::from_mut(source),
            original,
            &format!("{original}_Changed"),
        );
        rejects_in_both(&sources, "DIAGNOSTIC_SESSION");
    }
}

#[test]
fn authentication_mode_source_follows_actual_connection_and_rejects_invalid_standard_state() {
    let mut source = inputs();
    replace_all(&mut source, "Physical", "Link");
    let plan = build(&source).unwrap();
    assert!(
        plan.description()
            .composition
            .as_ref()
            .unwrap()
            .dcm_modes
            .iter()
            .any(|mode| mode.group == "DcmAuthenticationState_Link")
    );
    let files: std::collections::BTreeMap<_, _> =
        prepared_pipeline_files(&plan).into_iter().collect();
    for (path, symbol) in [
        (
            "src/Ecu_RuntimeConfig.c",
            "SchM_Switch_Dcm_DcmAuthenticationState_Link",
        ),
        ("src/SchM.c", "SchM_Mode_Dcm_DcmAuthenticationState_Link"),
        (
            "include/SchM_Dcm.h",
            "SchM_Switch_Dcm_DcmAuthenticationState_Link",
        ),
    ] {
        let text = std::str::from_utf8(&files[path]).unwrap();
        assert!(text.contains(symbol), "{path}");
        assert!(!text.contains("DcmAuthenticationState_Physical"), "{path}");
    }
    let mut source = inputs();
    xml_edit(
        &mut source,
        "bsw.arxml",
        |node| named(node, "MODE-DECLARATION", "DCM_AUTHENTICATED"),
        |text| text.replace("<VALUE>1</VALUE>", "<VALUE>2</VALUE>"),
    );
    rejects_in_both(&source, "TYPE_CONFLICT");
    let mut source = inputs();
    xml_edit(
        &mut source,
        "services.arxml",
        |node| {
            named(
                node,
                "P-PORT-PROTOTYPE",
                "AuthenticationStateModeSwitchInterface_Physical",
            )
        },
        |_| String::new(),
    );
    rejects_in_both(&source, "SERVICE_TYPE_CONFLICT");
}

#[test]
fn destination_only_diagnostic_routes_and_explicit_lsdu_policy_cannot_be_ignored() {
    let mut source = sources_without_application_did(true);
    xml_edit(
        &mut source,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "DiagRequestSource"),
        |_| String::new(),
    );
    xml_edit(
        &mut source,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "DiagRequest")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/PduRRoutingPath"))
                })
        },
        |_| String::new(),
    );
    rejects_in_both(&source, "PDU_ROUTE_NOT_UNIQUE");
    let mut source = inputs();
    xml_edit(
        &mut source,
        "ecuc.arxml",
        |node| named(node, "AR-PACKAGE", "Configuration"),
        |text| {
            text.replacen("</ELEMENTS>", "<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>LSduR</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-MODULE-DEF\">/AUTOSAR/EcucDefs/LSduR</DEFINITION-REF></ECUC-MODULE-CONFIGURATION-VALUES></ELEMENTS>", 1)
        },
    );
    rejects_in_both(&source, "MODULE_UNSUPPORTED");
}
