//! The selected Det configuration is explicit and never drops a requested service.
use super::single_component::{add_to_container, rejected_at};
use super::*;

#[test]
fn det_configuration_selects_the_delivered_runtime_notification() {
    // Verifies: ECUC_Det_00001, ECUC_Det_00002, ECUC_Det_00003, ECUC_Det_00004
    // Verifies: ECUC_Det_00010
    let sources = inputs();
    assert!(normal_validation(&sources).diagnostics.is_empty());
    let plan = build(&sources).unwrap();
    let records = &plan.description().configuration;
    assert!(
        records
            .iter()
            .any(|record| record.definition == "/AUTOSAR/EcucDefs/Det/DetGeneral")
    );
    for literal in ["false", "0"] {
        let mut selected = sources.clone();
        parameter(&mut selected, "General", "DetVersionInfoApi", literal);
        add_to_container(
            &mut selected,
            "General",
            "/AUTOSAR/EcucDefs/Det/DetGeneral",
            "DetForwardToDlt",
            &format!("BOOLEAN:{literal}"),
            "",
        );
        assert!(build(&selected).is_ok());
        assert!(normal_validation(&selected).diagnostics.is_empty());
    }
    let files = prepared_pipeline_files(&plan);
    let text = |path| {
        std::str::from_utf8(&files.iter().find(|(name, _)| name == path).unwrap().1).unwrap()
    };
    assert!(text("generated/include/Det_Externals.h").contains("Ecu_DetRuntimeCallout("));
    assert!(text("generated/src/Det_Callouts.c").contains("Ecu_TargetReportTransportTimeout();"));
    assert!(text("generated/src/Ecu_RuntimeConfig.c").contains("#include \"Det_Externals.h\""));
    assert!(
        !text("generated/src/Ecu_RuntimeConfig.c")
            .contains("Std_ReturnType Ecu_DetRuntimeCallout(")
    );
}

#[test]
fn det_optional_features_are_refused_at_their_configuration_object() {
    // Rejects: ECUC_Det_00003
    let mut sources = inputs();
    parameter(&mut sources, "General", "DetVersionInfoApi", "true");
    rejected_at(
        &sources,
        "DET_CONFIGURATION",
        "ecuc.arxml",
        "/Configuration/Det/General",
    );
    // Rejects: ECUC_Det_00006
    let mut sources = inputs();
    add_to_container(
        &mut sources,
        "General",
        "/AUTOSAR/EcucDefs/Det/DetGeneral",
        "DetForwardToDlt",
        "BOOLEAN:true",
        "",
    );
    rejected_at(
        &sources,
        "DET_CONFIGURATION",
        "ecuc.arxml",
        "/Configuration/Det/General",
    );
    // Rejects: ECUC_Det_00005
    let mut sources = inputs();
    add_to_container(
        &mut sources,
        "Notification",
        "/AUTOSAR/EcucDefs/Det/DetNotification",
        "DetErrorHook",
        "FUNCTION-NAME:Application_Error",
        "",
    );
    rejected_at(
        &sources,
        "DET_CONFIGURATION",
        "ecuc.arxml",
        "/Configuration/Det/Notification",
    );
    for name in ["OtherCallout", ""] {
        let mut sources = inputs();
        parameter(
            &mut sources,
            "Notification",
            "DetReportRuntimeErrorCallout",
            name,
        );
        rejected_at(
            &sources,
            "DET_CONFIGURATION",
            "ecuc.arxml",
            "/Configuration/Det/Notification",
        );
    }
    let mut sources = inputs();
    add_to_container(
        &mut sources,
        "Notification",
        "/AUTOSAR/EcucDefs/Det/DetNotification",
        "DetReportRuntimeErrorCallout",
        "FUNCTION-NAME:Ecu_DetRuntimeCallout",
        "",
    );
    rejected_at(
        &sources,
        "DET_CONFIGURATION",
        "ecuc.arxml",
        "/Configuration/Det/Notification",
    );
}

#[test]
fn det_service_instances_are_refused_instead_of_losing_their_rte_binding() {
    // Rejects: ECUC_Det_00008
    // Verifies: ECUC_Det_00007
    let mut sources = inputs();
    let configuration = "<ECUC-CONTAINER-VALUE><SHORT-NAME>Config</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">/AUTOSAR/EcucDefs/Det/DetConfigSet</DEFINITION-REF></ECUC-CONTAINER-VALUE>";
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-MODULE-CONFIGURATION-VALUES", "Det"),
        |text| {
            let end = text.rfind("</CONTAINERS>").unwrap();
            format!("{}{configuration}{}", &text[..end], &text[end..])
        },
    );
    assert!(build(&sources).is_ok());
    assert!(normal_validation(&sources).diagnostics.is_empty());
    add_to_container(
        &mut sources,
        "Config",
        "/AUTOSAR/EcucDefs/Det/DetConfigSet",
        "DetModule",
        "",
        "<PARAMETER-VALUES><ECUC-NUMERICAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-INTEGER-PARAM-DEF\">/AUTOSAR/EcucDefs/Det/DetConfigSet/DetModule/DetModuleId</DEFINITION-REF><VALUE>4096</VALUE></ECUC-NUMERICAL-PARAM-VALUE></PARAMETER-VALUES><SUB-CONTAINERS><ECUC-CONTAINER-VALUE><SHORT-NAME>Instance</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">/AUTOSAR/EcucDefs/Det/DetConfigSet/DetModule/DetModuleInstance</DEFINITION-REF><PARAMETER-VALUES><ECUC-NUMERICAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-INTEGER-PARAM-DEF\">/AUTOSAR/EcucDefs/Det/DetConfigSet/DetModule/DetModuleInstance/DetInstanceId</DEFINITION-REF><VALUE>0</VALUE></ECUC-NUMERICAL-PARAM-VALUE></PARAMETER-VALUES></ECUC-CONTAINER-VALUE></SUB-CONTAINERS>",
    );
    rejected_at(
        &sources,
        "DET_CONFIGURATION",
        "ecuc.arxml",
        "/Configuration/Det/Config/DetModule",
    );
}

#[test]
fn det_parameters_cannot_be_hidden_on_another_container() {
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "General")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text() == Some("/AUTOSAR/EcucDefs/Det/DetGeneral")
                })
        },
        |text| {
            text.replacen("<PARAMETER-VALUES>", "<PARAMETER-VALUES><ECUC-TEXTUAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-FUNCTION-NAME-DEF\">/AUTOSAR/EcucDefs/Det/DetNotification/DetReportRuntimeErrorCallout</DEFINITION-REF><VALUE>OtherCallout</VALUE></ECUC-TEXTUAL-PARAM-VALUE>", 1)
        },
    );
    rejected_at(
        &sources,
        "DEFINITION_PARENT",
        "ecuc.arxml",
        "/Configuration/Det/General",
    );
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "Notification"),
        |text| {
            text.replace(
                "/Det/DetNotification</DEFINITION-REF>",
                "/Det/DetGeneral</DEFINITION-REF>",
            )
        },
    );
    rejected_at(
        &sources,
        "DET_CONFIGURATION",
        "ecuc.arxml",
        "/Configuration/Det/Notification",
    );
}
