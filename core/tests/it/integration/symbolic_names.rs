//! Independent standard names, values and collision boundaries of generated handles.
use super::*;

// Verifies[Can]: SWS_BSW_00200
// Verifies[ComM]: SWS_BSW_00200
// Verifies[BswM]: SWS_BSW_00200
// Verifies[Com]: SWS_BSW_00200
// Verifies[Rte]: SWS_BSW_00200
// Verifies[CanIf]: SWS_BSW_00200
// Verifies[CanTp]: SWS_BSW_00200
// Verifies[Dcm]: SWS_BSW_00200
// Verifies[PduR]: SWS_BSW_00200
// Verifies[LSduR]: SWS_BSW_00200
// Verifies: TPS_ECUC_02108
#[test]
fn symbolic_headers_and_consumers_preserve_provider_names_and_values() {
    let plan = build(&inputs()).unwrap();
    let files = prepared_pipeline_files(&plan);
    assert!(
        !files
            .iter()
            .any(|(path, _)| path == "generated/include/Det_Cfg.h")
    );
    let text = |path: &str| -> String {
        String::from_utf8(
            files
                .iter()
                .find(|(name, _)| name == path)
                .unwrap()
                .1
                .clone(),
        )
        .unwrap()
    };
    for (module, symbol, value) in [
        ("Com", "ComConf_ComIPduGroup_RxGroup", "0u"),
        ("Rte", "RteConf_ComUserSignal_RxValue", "17u"),
        ("CanIf", "CanIfConf_CanIfRxPduCfg_RxValue", "0u"),
        ("CanIf", "CanIfConf_CanIfTxPduCfg_TxValue", "0u"),
        ("PduR", "PduRConf_PduRSrcPdu_RxValueSource", "0u"),
        ("PduR", "PduRConf_PduRDestPdu_TxValueDestination", "1u"),
        ("LSduR", "LSduRConf_LSduRPathSourcePdu_RxValueSource", "0u"),
    ] {
        assert!(
            text(&format!("generated/include/{module}_Cfg.h"))
                .lines()
                .any(|line| line
                    .split_whitespace()
                    .take(3)
                    .eq(["#define", symbol, value])),
            "{symbol}"
        );
        assert_eq!(
            files
                .iter()
                .filter(|(_, bytes)| String::from_utf8_lossy(bytes)
                    .contains(&format!("#define {symbol} ")))
                .count(),
            1,
            "single producer for {symbol}"
        );
    }
    for (path, symbols) in [
        (
            "generated/src/PduR_PBcfg.c",
            &[
                "ComConf_ComIPdu_RxValuePdu",
                "LSduRConf_LSduRPathSourcePdu_TxValueSource",
            ][..],
        ),
        (
            "generated/src/LSduR_PBcfg.c",
            &[
                "PduRConf_PduRSrcPdu_RxValueSource",
                "CanIfConf_CanIfTxPduCfg_TxValue",
            ][..],
        ),
        (
            "generated/src/Ecu_RuntimeConfig.c",
            &["ComConf_ComIPduGroup_RxGroup", "CanTpConf_", "DcmConf_"][..],
        ),
    ] {
        let source = text(path);
        for symbol in symbols {
            assert!(source.contains(symbol), "{path}: {symbol}");
        }
    }
    let config = text("generated/src/Ecu_RuntimeConfig.c");
    assert!(config.contains("#include \"Rte_Cfg.h\""));
    assert!(config.contains("RteConf_ComUserSignal_RxValue"));
    let rte = text("generated/src/Rte.c");
    assert!(rte.contains("#include \"Rte_Cfg.h\""));
    assert_eq!(
        rte.matches("handle == RteConf_ComUserSignal_RxValue")
            .count(),
        2
    );
    assert!(config.contains("Ecu_HostBusSM_ApplyMode(ComMConf_ComMChannel_Host,"));
    assert!(config.contains("PduRConf_PduRSrcPdu_TxValueSource"));
    let profile = text("generated/include/Ecu_ProfileConfig.h");
    assert!(profile.contains("#include \"Can_Cfg.h\""));
    assert!(profile.contains("#include \"CanIf_Cfg.h\""));
}

#[test]
fn symbolic_names_reject_application_producer_collisions_before_generation() {
    for (module, kind, first) in [
        ("PduR", "PduRSrcPdu", "RxValueSource"),
        ("CanIf", "CanIfRxPduCfg", "RxValue"),
        ("LSduR", "LSduRPathSourcePdu", "RxValueSource"),
        ("Com", "ComIPduGroup", "RxGroup"),
        ("CanTp", "CanTpRxNPdu", "Frame"),
        ("Dcm", "DcmDslProtocolRx", "Receive"),
        ("Rte", "ComUserSignal", "RxValue"),
    ] {
        let mut sources = inputs();
        change(
            &mut sources,
            "ingress.arxml",
            "<SYMBOL>Ingress_Periodic</SYMBOL>",
            &format!("<SYMBOL>{module}Conf_{kind}_{first}</SYMBOL>"),
        );
        let errors = build(&sources).err().expect("name collision accepted");
        assert!(
            errors
                .iter()
                .any(|error| error.code == "SYMBOL_PRODUCER_DUPLICATE"
                    && error.object.as_deref() == Some("/Application/Ingress/Behavior/Periodic")),
            "{errors:?}"
        );
    }
}

// Verifies: SWS_Com_00401, SWS_Com_00402
#[test]
fn symbolic_com_names_share_the_standard_short_name_namespace() {
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "RxGroup"),
        |text| {
            text.replacen(
                "<SHORT-NAME>RxGroup</SHORT-NAME>",
                "<SHORT-NAME>RxValue</SHORT-NAME>",
                1,
            )
        },
    );
    replace_all(&mut sources, "/Com/Config/RxGroup", "/Com/Config/RxValue");
    let errors = build(&sources)
        .err()
        .expect("cross-kind Com collision accepted");
    assert!(
        errors.iter().any(|error| error.code == "OBJECT_DUPLICATE"
            && error.object.as_deref() == Some("/Configuration/Com/Config/RxValue")),
        "{errors:?}"
    );
}

// Verifies: SWS_Com_00402
#[test]
fn symbolic_com_names_are_c_identifiers_before_prefixing() {
    for (kind, original) in [
        ("ComSignal", "RxValue"),
        ("ComIPdu", "RxValuePdu"),
        ("ComIPduGroup", "RxGroup"),
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                named(node, "ECUC-CONTAINER-VALUE", original)
                    && node.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF")
                            && child
                                .text()
                                .is_some_and(|definition| definition.ends_with(&format!("/{kind}")))
                    })
            },
            |text| {
                text.replacen(
                    &format!("<SHORT-NAME>{original}</SHORT-NAME>"),
                    "<SHORT-NAME>if</SHORT-NAME>",
                    1,
                )
            },
        );
        replace_all(
            &mut sources,
            &format!("/Com/Config/{original}</"),
            "/Com/Config/if</",
        );
        let errors = build(&sources)
            .err()
            .expect("C keyword accepted as a COM identifier");
        assert!(
            errors
                .iter()
                .any(|error| error.code == "CONTRACT_NAME_COLLISION"
                    && error.object.as_deref() == Some("/Configuration/Com/Config/if")),
            "{errors:?}"
        );
    }
}

#[test]
fn symbolic_names_reserve_only_headers_with_actual_providers() {
    let mut sources = inputs();
    change(
        &mut sources,
        "ingress.arxml",
        "<SYMBOL>Ingress_Periodic</SYMBOL>",
        "<SYMBOL>DET_CFG_H</SYMBOL>",
    );
    assert!(
        build(&sources).is_ok(),
        "an absent configuration header cannot own this name"
    );
}
