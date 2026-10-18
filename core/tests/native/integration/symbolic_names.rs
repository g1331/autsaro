//! Real generated consumers with renamed containers and independent handle values.
use super::*;
use autosar_config_core::{ApplicationSource, prepare_ecu_project_with_applications};
use std::path::Path;
use std::time::Duration;

// Verifies[Can]: SWS_BSW_00200
// Verifies[ComM]: SWS_BSW_00200
// Verifies[BswM]: SWS_BSW_00200
// Verifies[Com]: SWS_BSW_00200
// Verifies[CanIf]: SWS_BSW_00200
// Verifies[CanTp]: SWS_BSW_00200
// Verifies[Dcm]: SWS_BSW_00200
// Verifies[PduR]: SWS_BSW_00200
// Verifies[LSduR]: SWS_BSW_00200
// Verifies: TPS_ECUC_02108
#[test]
fn symbolic_handles_compile_link_and_run_renamed_nondefault_mapping() {
    let mut sources = inputs();
    for (owner, parameter_name, value) in [
        ("Receive", "CanObjectId", "1"),
        ("Transmit", "CanObjectId", "0"),
        ("HostUser", "ComMUserIdentifier", "7"),
        ("RxValueSource", "PduRSourcePduHandleId", "011"),
        ("RxValueDestination", "PduRDestPduHandleId", "12"),
        ("TxValueSource", "PduRSourcePduHandleId", "13"),
        ("TxValueDestination", "PduRDestPduHandleId", "14"),
        ("DiagRequestSource", "PduRSourcePduHandleId", "21"),
        ("DiagRequestDestination", "PduRDestPduHandleId", "22"),
        ("DiagResponseSource", "PduRSourcePduHandleId", "23"),
        ("DiagResponseDestination", "PduRDestPduHandleId", "24"),
        ("DiagRequest", "CanTpRxNSduId", "31"),
        ("DiagResponse", "CanTpTxNSduId", "32"),
        ("Frame", "CanTpRxNPduId", "33"),
        ("FlowControl", "CanTpRxFcNPduId", "33"),
        ("Frame", "CanTpTxNPduConfirmationPduId", "34"),
        ("FlowControl", "CanTpTxFcNPduConfirmationPduId", "34"),
        ("Receive", "DcmDslProtocolRxPduId", "41"),
        ("Transmit", "DcmDslTxConfirmationPduId", "42"),
        ("RxValue", "CanIfRxPduId", "1"),
        ("DiagRequest", "CanIfRxPduId", "0"),
        ("TxValue", "CanIfTxPduId", "1"),
        ("DiagResponse", "CanIfTxPduId", "0"),
    ] {
        parameter(&mut sources, owner, parameter_name, value);
    }
    // These names belong only to ECUC containers, so application source is unchanged.
    for before in [
        "RxValueSource",
        "TxValueSource",
        "DiagRequestSource",
        "DiagResponseSource",
        "RxValueDestination",
        "TxValueDestination",
        "DiagRequestDestination",
        "DiagResponseDestination",
        "Frame",
        "FlowControl",
        "Receive",
        "Transmit",
        "RxGroup",
    ] {
        replace_all(
            &mut sources,
            &format!("<SHORT-NAME>{before}</SHORT-NAME>"),
            &format!("<SHORT-NAME>{before}Alt</SHORT-NAME>"),
        );
        replace_all(
            &mut sources,
            &format!("/{before}</"),
            &format!("/{before}Alt</"),
        );
    }
    let plan = build(&sources).unwrap_or_else(|errors| panic!("{errors:?}"));
    let scratch = Scratch::new();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/component-pipeline/applications");
    let applications: Vec<_> = plan
        .application_slot_descriptors()
        .unwrap()
        .iter()
        .map(|slot| ApplicationSource {
            component_instance: slot
                .producer_slot
                .strip_prefix("singlecore-ecu-v1:")
                .unwrap()
                .into(),
            path: fixture.join(slot.source_paths[0].rsplit('/').next().unwrap()),
        })
        .collect();
    let project = scratch.0.join("project");
    prepare_ecu_project_with_applications(&plan, tooling::native_target(), &applications)
        .unwrap()
        .generate(&project)
        .unwrap();
    let consumer = scratch.0.join("consumer.c");
    std::fs::write(
        &consumer,
        r#"#include "Can_Cfg.h"
#include "ComM_Cfg.h"
#include "CanIf_Cfg.h"
#include "CanTp_Cfg.h"
#include "Com_Cfg.h"
#include "Dcm_Cfg.h"
#include "PduR_Cfg.h"
#include "LSduR_Cfg.h"
#if defined(PduRConf_PduRSrcPdu_RxValueSource) || defined(ComConf_ComIPduGroup_RxGroup)
#error Renamed containers must not retain their former symbolic names
#endif
int main(void)
{
    return !(CanConf_CanController_Controller == 0u &&
             CanConf_CanHardwareObject_ReceiveAlt == 1u &&
             CanConf_CanHardwareObject_TransmitAlt == 0u &&
             ComMConf_ComMChannel_Host == 0u &&
             ComMConf_ComMUser_HostUser == 7u &&
             CanIfConf_CanIfCtrlCfg_Controller == 0u &&
             DcmConf_DcmDslProtocolRow_UdsOnCan == 3u &&
             PduRConf_PduRSrcPdu_RxValueSourceAlt == 11u &&
             PduRConf_PduRDestPdu_TxValueDestinationAlt == 14u &&
             CanTpConf_CanTpRxNSdu_DiagRequest == 31u &&
             CanTpConf_CanTpTxNSdu_DiagResponse == 32u &&
             CanTpConf_CanTpRxNPdu_FrameAlt == 33u &&
             CanTpConf_CanTpTxNPdu_FrameAlt == 34u &&
             CanTpConf_CanTpRxFcNPdu_FlowControlAlt == 33u &&
             CanTpConf_CanTpTxFcNPdu_FlowControlAlt == 34u &&
             DcmConf_DcmDslProtocolRx_ReceiveAlt == 41u &&
             DcmConf_DcmDslProtocolTx_TransmitAlt == 42u &&
             CanIfConf_CanIfRxPduCfg_RxValue == 1u &&
             CanIfConf_CanIfTxPduCfg_TxValue == 1u &&
             ComConf_ComIPdu_RxValuePdu == 12u &&
             ComConf_ComIPdu_TxValuePdu == 13u &&
             ComConf_ComIPduGroup_RxGroupAlt == 0u &&
             LSduRConf_LSduRPathSourcePdu_TxValueSourceAlt == 14u &&
             LSduRConf_LSduRPathDestinationPdu_TxValueDestinationAlt == 1u);
}
"#,
    )
    .unwrap();
    let binary = tooling::native_binary(&scratch.0, "consumer");
    let mut compiler = tooling::c_compiler();
    compiler.args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"]);
    for include in ["generated/include", "bsw/include", "os/include"] {
        compiler.arg("-I").arg(project.join(include));
    }
    compiler.arg(&consumer).arg("-o").arg(&binary);
    let result = tooling::run_public_command(
        &mut compiler,
        &scratch.0,
        "symbolic-consumer",
        Duration::from_secs(30),
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = tooling::run_public_command(
        &mut std::process::Command::new(binary),
        &scratch.0,
        "symbolic-values",
        Duration::from_secs(30),
    );
    assert!(result.status.success());
    // Actual sealed HostBatch build/link and existing independent CAN/UDS assertions.
    verify_production_owner(&project, &scratch.0, None);
}
