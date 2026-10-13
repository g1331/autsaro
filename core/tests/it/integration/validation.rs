//! Definition validation of mainline projects.
use super::*;

#[test]
fn normal_definition_validation_closes_schedule_routes_types_and_handles() {
    let validation = normal_validation(&inputs());
    assert!(
        validation.diagnostics.is_empty(),
        "{:?}",
        validation.diagnostics
    );
    for (owner, definition, value, code) in [
        ("Observe", "RtePositionInTask", "5", "SCHEDULE_NOT_UNIQUE"),
        (
            "ReadApplicationValue",
            "RteEventIsMappedToTask",
            "true",
            "SERVICE_ASYNC_MAPPING",
        ),
        ("RxValue", "CanIfRxPduId", "2", "CAN_ID_CONFLICT"),
        (
            "RxValueSource",
            "PduRSrcPduUpTxConf",
            "false",
            "PDU_ROUTE_CONFIRMATION",
        ),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, owner, definition, value);
        rejects_in_both(&sources, code);
    }
    let mut sources = inputs();
    change(
        &mut sources,
        "composition.arxml",
        "/Application/Process/Value</TARGET-R-PORT-REF>",
        "/Application/Process/Result</TARGET-R-PORT-REF>",
    );
    rejects_in_both(&sources, "REFERENCE_DEST");
    let mut sources = inputs();
    change(
        &mut sources,
        "types.arxml",
        "<BASE-TYPE-ENCODING>NONE</BASE-TYPE-ENCODING>",
        "<BASE-TYPE-ENCODING>2C</BASE-TYPE-ENCODING>",
    );
    rejects_in_both(&sources, "TYPE_CONFLICT");
    let mut sources = inputs();
    parameter(&mut sources, "TxValue", "CanIfTxPduId", "0");
    assert!(
        build(&sources).is_ok(),
        "same handle in different directions must be allowed"
    );
    assert!(normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn normal_definition_validation_keeps_duplicate_producer_diagnostics() {
    let mut sources = inputs();
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "IngressProcess",
    )
    .replace(
        "<SHORT-NAME>IngressProcess</SHORT-NAME>",
        "<SHORT-NAME>SecondProducer</SHORT-NAME>",
    );
    change(
        &mut sources,
        "composition.arxml",
        "</CONNECTORS>",
        &format!("{connector}</CONNECTORS>"),
    );
    rejects_in_both(&sources, "MULTIPLE_PRODUCERS");
}

#[test]
fn template_definition_validation_locates_nonzero_execution_constraints() {
    assert!(normal_validation(&template_inputs()).diagnostics.is_empty());
    let expected = [
        (
            "PERIOD_UNSUPPORTED",
            "application.arxml",
            "/Application/EchoApplication/Behavior/Periodic10ms",
        ),
        (
            "MINIMUM_START_INTERVAL_UNSUPPORTED",
            "application.arxml",
            "/Application/EchoApplication/Behavior/Periodic",
        ),
        (
            "MINIMUM_START_INTERVAL_UNSUPPORTED",
            "application.arxml",
            "/Application/EchoApplication/Behavior/ServerReadData",
        ),
        (
            "MINIMUM_START_INTERVAL_UNSUPPORTED",
            "services.arxml",
            "/Services/DcmService/Behavior/DcmReadData",
        ),
    ];
    let cases = template_execution_variants();
    assert_eq!(cases.len(), expected.len());
    for (mut sources, (code, file, path)) in cases.into_iter().zip(expected) {
        let validation = normal_validation(&sources);
        assert!(
            validation.diagnostics.iter().any(|issue| issue.code == code
                && issue.file.as_deref() == Some(file)
                && issue.path.as_deref() == Some(path)),
            "{validation:?}"
        );
        let issues = build(&sources).err().unwrap();
        assert!(issues.iter().any(|issue| issue.code == code), "{issues:?}");
        replace_all(&mut sources, "<OFFSET>0.001</OFFSET>", "<OFFSET>0</OFFSET>");
        replace_all(
            &mut sources,
            "<MINIMUM-START-INTERVAL>0.001</MINIMUM-START-INTERVAL>",
            "<MINIMUM-START-INTERVAL>0</MINIMUM-START-INTERVAL>",
        );
        assert!(normal_validation(&sources).diagnostics.is_empty());
        assert!(build(&sources).is_ok());
    }
}
