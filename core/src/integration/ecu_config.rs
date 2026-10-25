//! ECU configuration and integration sources from the selected plan; applications remain caller owned.
use super::c_layout::{self, Item};
use super::outputs::ECU;
use super::symbolic_names::{self, name};
use super::{PlanDiagnostic, ValidatedIntegrationPlan};
use crate::target::BuildTarget;

fn header(owner: &str, name: &str, body: &str) -> String {
    format!(
        "/** @file Generated from the selected source contract. */\n{}",
        c_layout::header(name, &format!("#include \"{owner}_Version.h\"\n{body}"))
    )
}
/// Module interlink headers share the standard type import and C++ linkage.
fn schm_header(module: &str, body: &str) -> String {
    header(
        "Rte",
        &format!("SCHM_{}_H", module.to_uppercase()),
        &format!(
            r#"#include "SchM_{module}_Type.h"
{linkage_begin}{body}{memmap_check}
{linkage_end}"#,
            memmap_check = if body.contains("_MemMap.h") {
                c_layout::release_check(
                    &format!("SchM_{module}.h"),
                    &format!("{module}_MemMap.h"),
                    "MEMMAP",
                )
            } else {
                String::new()
            },
            linkage_begin = c_layout::linkage(true),
            linkage_end = c_layout::linkage(false)
        ),
    )
}

/// Public void declarations use the same four contract fields as runtime headers.
fn void_prototype(symbol: &str, purpose: &str, conditions: [&str; 4]) -> String {
    let details = [
        c_layout::ContractLine::Detail("@par Return"),
        c_layout::ContractLine::Detail("None."),
    ];
    let fields = c_layout::CONTRACT_FIELDS
        .into_iter()
        .zip(conditions)
        .map(|(field, value)| c_layout::ContractLine::Condition(field, value));
    let comment = c_layout::contract_comment(purpose, details.into_iter().chain(fields), false);
    format!("{comment}void {symbol}(void);\n")
}

fn area_prototypes(module: &str, area: &str) -> String {
    let enter = void_prototype(
        &format!("SchM_Enter_{module}_{area}_STATE"),
        &format!("Enter {area}_STATE (SWS_Rte_07250); this module's area nests with none."),
        [
            "Target lock provider available; no exclusive area is held by this context.",
            "task or callback, including interrupt context.",
            "not reentrant within one context; concurrent contexts serialize.",
            "this context holds the area until the matching Exit.",
        ],
    );
    let exit = void_prototype(
        &format!("SchM_Exit_{module}_{area}_STATE"),
        &format!("Leave {area}_STATE (SWS_Rte_07253)."),
        [
            "this context holds the matching area.",
            "the same context as the matching Enter.",
            "not reentrant within one context.",
            "releases this context's area.",
        ],
    );
    format!("{enter}{exit}")
}

fn main_prototype(symbol: &str, purpose: &str) -> String {
    void_prototype(
        symbol,
        purpose,
        [
            "module initialized; otherwise no scheduled work.",
            "scheduled ECU owner task, outside exclusive areas.",
            "not reentrant for the same module instance.",
            "callbacks borrow call-local storage and execute outside the module area.",
        ],
    )
}

fn code_prototypes(module: &str, declarations: &str) -> String {
    let prefix = module.to_uppercase();
    format!(
        r#"
{start_code}{declarations}
{stop_code}"#,
        start_code = c_layout::section(&prefix, module, "CODE", true),
        stop_code = c_layout::section(&prefix, module, "CODE", false)
    )
}

fn failure() -> Vec<PlanDiagnostic> {
    vec![PlanDiagnostic::dependency(
        "ECU_SOURCE_CLOSURE",
        crate::product_message!("backend.integration.composition.consumer_unsupported"),
        crate::product_message!("backend.integration.composition.repair_contract"),
    )]
}

/// Writes one constant in its module's pre-build configuration data section.
fn config_data(output: &mut String, scope: &str, declaration: &str, items: &[Item]) {
    let definition = c_layout::initializer(
        declaration,
        &format!("RTE_{scope}_CONFIG_DATA_PREBUILD_UNSPECIFIED"),
        items,
    );
    output.push_str(&format!(
        r#"{start_config_data_prebuild_unspecified}{definition}
{stop_config_data_prebuild_unspecified}"#,
        start_config_data_prebuild_unspecified = c_layout::section(
            &format!("RTE_{scope}"),
            "Rte",
            "CONFIG_DATA_PREBUILD_UNSPECIFIED",
            true
        ),
        stop_config_data_prebuild_unspecified = c_layout::section(
            &format!("RTE_{scope}"),
            "Rte",
            "CONFIG_DATA_PREBUILD_UNSPECIFIED",
            false
        )
    ));
}

/// The items of a comma-separated list of plain values.
fn values(list: &str) -> Vec<Item> {
    list.split(", ")
        .map(|value| Item::Value(value.into()))
        .collect()
}

/// A table row of plain values.
fn row(list: &str) -> Item {
    Item::Row(list.split(", ").map(str::to_owned).collect())
}

/// Whole periods of `period_ms` in `value_ms`; a zero period cannot be
/// scheduled and is reported at the object that defines it.
fn periods(value_ms: u32, period_ms: u32, object: &str) -> Result<u32, Vec<PlanDiagnostic>> {
    value_ms
        .checked_div(period_ms)
        .ok_or_else(|| PlanDiagnostic::plan_premise(object))
}

pub(super) fn files(
    plan: &ValidatedIntegrationPlan,
    target: BuildTarget,
) -> Result<(super::outputs::EcuFiles, String), Vec<PlanDiagnostic>> {
    let description = plan.description();
    let composition = description.ecu_composition()?;
    let com = description.ecu_com()?;
    let authentication_group = composition.authentication_modes()?.group.as_ref();
    let communication = description.ecu_communication()?;
    let can = &communication.can;
    let mode = description.ecu_mode()?;
    let symbolic = symbolic_names::Names::new(description)?;
    let mut outputs = super::outputs::EcuOutputs::new(target)?;
    // The contract provenance is package metadata; its headers are generated C.
    for (path, bytes) in plan.contract_headers(composition, &symbolic)? {
        if matches!(
            path.as_str(),
            "include/Rte_Type.h"
                | "include/Rte_Main.h"
                | "include/Rte.h"
                | "include/Rte_Cfg.h"
                | "include/Rte_DataHandleType.h"
        ) {
            let text =
                String::from_utf8(bytes).map_err(|_| PlanDiagnostic::output_missing(&path))?;
            outputs.emit_rendered(super::outputs::RTE, &path, text)?;
            continue;
        }
        let producer = if path.ends_with("_MemMap.h") {
            super::outputs::MEMMAP
        } else {
            "contracts"
        };
        let path = if path == "contract.json" {
            "meta/contract.json".into()
        } else {
            path
        };
        outputs.emit(producer, path, bytes)?;
    }
    outputs.emit_all(super::outputs::RTE, super::rte::files(plan)?)?;
    outputs.emit_verbatim(super::outputs::RTE)?;
    outputs.emit_verbatim(super::outputs::MEMMAP)?;
    // The selected authentication group's mode type and SchM declarations
    // complete the trusted Dcm headers.
    let text = plan.dcm_type_header(composition)?;
    outputs.emit_rendered(super::outputs::DCM_MODES, "include/Rte_Dcm_Type.h", text)?;
    let (path, addition) = (
        "include/SchM_Dcm.h",
        format!(
            r#"/** Publish the selected connection authentication mode.
 * @param[in] mode 0 for deauthenticated, 1 for authenticated.
 * @return SCHM_E_OK after publishing the immediate mode.
 * Preconditions: initialized ECU.
 * Context: ECU task or callback; not inside an exclusive area.
 * Reentrancy: reentrant; the Dcm publisher orders updates.
 * Ownership: no pointer is retained.
 */
{}
/** Read the selected connection authentication mode.
 * @return Current mode.
 * Preconditions: initialized ECU.
 * Context: ECU task or callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
{}
"#,
            c_layout::signature(
                "Std_ReturnType",
                &format!("SchM_Switch_Dcm_{authentication_group}"),
                &[format!("Rte_ModeType_{authentication_group} mode")],
                ";",
                false,
            ),
            c_layout::signature(
                &format!("Rte_ModeType_{authentication_group}"),
                &format!("SchM_Mode_Dcm_{authentication_group}"),
                &["void".to_owned()],
                ";",
                false,
            )
        ),
    );
    let addition = format!(
        r#"{linkage_begin}{addition}{linkage_end}"#,
        linkage_begin = c_layout::linkage(true),
        linkage_end = c_layout::linkage(false)
    );
    let text = super::outputs::render(
        outputs.template(super::outputs::DCM_MODES, path)?,
        path,
        &[super::outputs::Edit::BeforeGuardEnd(&addition)],
    )?;
    outputs.emit_rendered(super::outputs::DCM_MODES, path, text)?;
    let os = super::ecu::configure_os(description, &mut outputs)?;
    let os_config = super::ecu::os_configuration(
        description,
        &mut outputs,
        &os.counter_symbol,
        &os.task_symbol,
        os.use_res_scheduler,
    )?;
    outputs.emit_rendered(super::outputs::OS, super::ecu::ECU_CONFIG, os_config)?;
    outputs.emit_all(ECU, super::can_runtime::type_headers(communication))?;
    let (path, bytes) =
        super::can_runtime::profile_header(communication, symbolic.parameter("CanIfCtrlId")?);
    outputs.emit(ECU, path, bytes)?;
    for (module, definitions) in &symbolic.headers {
        if matches!(module.as_str(), "CanIf" | "PduR" | "LSduR" | "Rte") {
            continue;
        }
        outputs.emit(
            ECU,
            format!("include/{module}_Cfg.h"),
            header(
                module,
                &format!("{}_CFG_H", module.to_uppercase()),
                &c_layout::commented_defines(definitions, "Generated handle."),
            ),
        )?;
    }

    for (module, area) in [
        ("CanIf", "CANIF"),
        ("ComM", "COMM"),
        ("CanTp", "CANTP"),
        ("Det", "DET"),
        ("PduR", "PDUR"),
        ("LSduR", "LSDUR"),
    ] {
        let main = match module {
            "CanTp" => code_prototypes(
                module,
                &main_prototype(
                    "CanTp_MainFunction",
                    "Advance transport timers and pending segmented transfers.",
                ),
            ),
            "ComM" => code_prototypes(
                module,
                &main_prototype(
                    &mode.main_symbol,
                    "Advance the selected channel hold timer and retry pending bus-mode requests.",
                ),
            ),
            _ => String::new(),
        };
        outputs.emit(
            ECU,
            format!("include/SchM_{module}.h"),
            schm_header(module, &format!("{}{main}", area_prototypes(module, area))),
        )?;
    }
    outputs.emit(
        ECU,
        "include/SchM_Com.h",
        schm_header(
            "Com",
            &format!(
                "{}{}",
                area_prototypes("Com", "COM"),
                code_prototypes(
                    "Com",
                    &format!("{}{}", main_prototype(&com.receive_main.symbol, "Advance Rx deadlines and publish pending reception/timeout notifications."), main_prototype(&com.transmit_main.symbol, "Snapshot each enabled periodic Tx PDU and request its transmission once."))
                )
            ),
        ),
    )?;
    for module in [
        "Can", "CanIf", "CanTp", "Com", "ComM", "BswM", "Dcm", "Det", "PduR", "LSduR",
    ] {
        outputs.emit(
            ECU,
            format!("include/SchM_{module}_Type.h"),
            header(
                "Rte",
                &format!("SCHM_{}_TYPE_H", module.to_uppercase()),
                &format!(
                    "#include \"Rte_Type.h\"\n{}",
                    if module == "Dcm" {
                        format!(
                            "#include \"Rte_Dcm_Type.h\"\n{}",
                            c_layout::release_check("SchM_Dcm_Type.h", "Rte_Dcm_Type.h", "DCM")
                        )
                    } else {
                        String::new()
                    }
                ),
            ),
        )?;
    }
    let app = description
        .schedule
        .entities
        .iter()
        .find(|entity| entity.application)
        .ok_or_else(failure)?;
    let work = description
        .schedule
        .entities
        .iter()
        .find(|entity| {
            !entity.application && entity.period_ms == description.schedule.counter_tick_ms
        })
        .ok_or_else(failure)?;
    let io = description
        .events
        .iter()
        .find(|event| event.path != app.os_event && event.path != work.os_event)
        .ok_or_else(failure)?;
    let path = "include/Ecu_TargetConfig.h";
    let defines = c_layout::commented_defines(
        &[
            ("ECU_TARGET_TASK", os.task_symbol.clone()),
            (
                "ECU_MAX_PDU_PAYLOAD",
                format!(
                    "{}u",
                    description
                        .diagnostic_transport
                        .as_ref()
                        .map_or(4, |transport| transport.buffer_bytes)
                ),
            ),
            (
                "ECU_TARGET_DIAGNOSTIC_RX_CAN_ID",
                format!(
                    "{}u",
                    description
                        .diagnostic_transport
                        .as_ref()
                        .map_or(u32::MAX, |transport| transport.request_can_id)
                ),
            ),
            (
                "ECU_TARGET_EVENT_WORK",
                format!("{}u", description.event_mask(&work.os_event)?),
            ),
            (
                "ECU_TARGET_EVENT_APP",
                format!("{}u", description.event_mask(&app.os_event)?),
            ),
            ("ECU_TARGET_EVENT_IO", format!("{}u", io.mask)),
        ],
        "Validated target handle or limit.",
    );
    let text = super::outputs::render(
        outputs.template(ECU, path)?,
        path,
        &[super::outputs::Edit::Once(
            "@DEFINES@",
            &format!(
                "#include \"Os_Cfg.h\"\n{}{defines}",
                c_layout::release_check("Ecu_TargetConfig.h", "Os_Cfg.h", "OS")
            ),
        )],
    )?;
    outputs.emit_rendered(ECU, path, text)?;
    let mut config = String::from(
        r#"/** @file Source-derived immutable communication configurations and owner dispatch. */
#define RTE_CORE
#include "Ecu_TargetConfig.h"
#include "Ecu_Target.h"
#include "Ecu_Config.h"
#include "Can_Host.h"
#include "Can_Cfg.h"
#include "CanIf.h"
#include "CanIf_Cfg.h"
#include "PduR_Cfg.h"
#include "LSduR_Cfg.h"
#include "Det.h"
#include "Det_Externals.h"
#include "CanTp.h"
#include "Com.h"
#include "Com_Internal.h"
#include "Com_Cfg.h"
#include "Rte_Cfg.h"
#include "ComM_Cfg.h"
#include "CanTp_Cfg.h"
#include "Dcm_Cfg.h"
#include "ComM.h"
#include "ComM_BswM.h"
#include "ComM_Internal.h"
#include "BswM.h"
#include "BswM_ComM.h"
#include "Ecu_HostBusSM.h"
#include "Dcm.h"
#include "Dcm_ComM.h"
#include "PduR.h"
#include "PduR_Com.h"
#include "PduR_CanIf.h"
#include "LSduR.h"
#include "LSduR_PduR.h"
#include "Rte_Main.h"
#include "Rte_Com.h"
#include "SchM_Can.h"
#include "SchM_CanTp.h"
#include "SchM_Com.h"
#include "SchM_ComM.h"
#include "SchM_Dcm.h"
"#,
    );
    for (header, prefix) in [
        ("Can_Cfg.h", "CAN"),
        ("CanIf.h", "CANIF"),
        ("CanIf_Cfg.h", "CANIF"),
        ("PduR_Cfg.h", "PDUR"),
        ("LSduR_Cfg.h", "LSDUR"),
        ("Det.h", "DET"),
        ("Det_Externals.h", "DET"),
        ("CanTp.h", "CANTP"),
        ("Com.h", "COM"),
        ("Com_Internal.h", "COM"),
        ("Com_Cfg.h", "COM"),
        ("ComM_Cfg.h", "COMM"),
        ("CanTp_Cfg.h", "CANTP"),
        ("Dcm_Cfg.h", "DCM"),
        ("ComM.h", "COMM"),
        ("ComM_BswM.h", "COMM"),
        ("ComM_Internal.h", "COMM"),
        ("BswM.h", "BSWM"),
        ("BswM_ComM.h", "BSWM"),
        ("Dcm.h", "DCM"),
        ("Dcm_ComM.h", "DCM"),
        ("PduR.h", "PDUR"),
        ("PduR_Com.h", "PDUR"),
        ("PduR_CanIf.h", "PDUR"),
        ("LSduR.h", "LSDUR"),
        ("LSduR_PduR.h", "LSDUR"),
        ("Rte_Cfg.h", "RTE"),
        ("Rte_Main.h", "RTE"),
        ("Rte_Com.h", "RTE"),
        ("SchM_Can.h", "RTE"),
        ("SchM_CanTp.h", "RTE"),
        ("SchM_Com.h", "RTE"),
        ("SchM_ComM.h", "RTE"),
        ("SchM_Dcm.h", "RTE"),
    ] {
        config.push_str(&c_layout::release_check(
            "Ecu_RuntimeConfig.c",
            header,
            prefix,
        ));
    }
    if description.diagnostic_transport.is_some() {
        config.push_str(
            r#"#include "PduR_CanTp.h"
#include "LSduR_CanTp.h"
"#,
        );
        config.push_str(&c_layout::release_check(
            "Ecu_RuntimeConfig.c",
            "PduR_CanTp.h",
            "PDUR",
        ));
        config.push_str(&c_layout::release_check(
            "Ecu_RuntimeConfig.c",
            "LSduR_CanTp.h",
            "LSDUR",
        ));
    }
    for component in &composition.components {
        config.push_str(&format!(
            "#include \"{}\"\n",
            component.header.trim_start_matches("include/")
        ));
    }
    config.push_str("static Std_ReturnType Ecu_BswMAction(NetworkHandleType channel, ComM_ModeType requested);\n");
    config_data(
        &mut config,
        "Ecu",
        "const EcuPolicyConfig Ecu_Policy",
        &values("ECU_TX_QUEUED"),
    );
    config_data(
        &mut config,
        "Can",
        "static const Can_ConfigType driver",
        &values("NULL_PTR"),
    );
    let routing_headers = r#"#include "Com_Cfg.h"
#include "CanIf_Cfg.h"
#include "CanTp_Cfg.h"
#include "Dcm_Cfg.h"
#include "LSduR_Cfg.h"
#include "PduR_Cfg.h"
#include "Com.h"
#include "Dcm.h"
#include "CanTp.h"
#include "PduR.h"
#include "PduR_Com.h"
#include "PduR_CanIf.h"
#include "LSduR.h"
#include "LSduR_PduR.h"
"#;
    let mut pd_configuration = format!(
        r#"/** @file Source-derived selected PduR configuration. */
#include "PduR_Cfg.h"
{routing_headers}"#
    );
    let mut ls_configuration = format!(
        r#"/** @file Source-derived selected LSduR configuration. */
#include "LSduR_Cfg.h"
{routing_headers}"#
    );
    let mut can_if_configuration = r#"/** @file Source-derived selected CanIf configuration. */
#include "Can_Cfg.h"
#include "CanIf_Cfg.h"
#include "Ecu_HostBusSM.h"
"#
    .to_owned();
    can_if_configuration.push_str(&c_layout::release_check(
        "CanIf_PBcfg.c",
        "Can_Cfg.h",
        "CAN",
    ));
    for (header, prefix) in [
        ("Com_Cfg.h", "COM"),
        ("CanIf_Cfg.h", "CANIF"),
        ("CanTp_Cfg.h", "CANTP"),
        ("Dcm_Cfg.h", "DCM"),
        ("LSduR_Cfg.h", "LSDUR"),
        ("Com.h", "COM"),
        ("Dcm.h", "DCM"),
        ("CanTp.h", "CANTP"),
        ("LSduR.h", "LSDUR"),
        ("LSduR_PduR.h", "LSDUR"),
    ] {
        pd_configuration.push_str(&c_layout::release_check("PduR_PBcfg.c", header, prefix));
    }
    let mut receive = Vec::new();
    let mut transmit = Vec::new();
    let mut ls_rx = Vec::new();
    let mut ls_tx = Vec::new();
    let mut pd_rx = Vec::new();
    let mut pd_tx = Vec::new();
    let mut pdus = Vec::new();
    for signal in &description.signals {
        let route = description
            .routes
            .iter()
            .find(|route| route.pdu == signal.global_pdu)
            .ok_or_else(|| PlanDiagnostic::plan_premise(&signal.global_pdu))?;
        // Tx IDs are passed to PduR_ComTransmit; Rx IDs are provided by COM.
        let pdu = if signal.receive {
            name("Com", "ComIPdu", &signal.com_pdu)
        } else {
            name("PduR", "PduRSrcPdu", &route.source)
        };
        let signal_id = name("Com", "ComSignal", &signal.com_signal);
        let can_if = name(
            "CanIf",
            if signal.receive {
                "CanIfRxPduCfg"
            } else {
                "CanIfTxPduCfg"
            },
            &signal.can_if_pdu,
        );
        let source = name("PduR", "PduRSrcPdu", &route.source);
        let ls_source = name("LSduR", "LSduRPathSourcePdu", &route.source);
        // signals::inspect requires source ComSignalInitValue=0.
        // Receiver ComSpec init is independent RTE state; it must not replace
        // the actual COM initialization or group Start(TRUE) value.
        let initial = 0;
        let reception = com
            .receptions
            .iter()
            .find(|reception| reception.signal == signal.com_signal);
        let reception_timeout = match reception {
            Some(rx) => periods(
                rx.timeout_ms,
                com.receive_main.period_ms,
                &com.receive_main.path,
            )?,
            None => 0,
        };
        let callback = reception.map_or_else(
            || "0u".into(),
            |rx| name("Rte", "ComUserSignal", &rx.user_signal),
        );
        pdus.push(row(&format!(
            "{pdu}, {signal_id}, {}, {initial}u, {}u, {callback}",
            if signal.receive { "TRUE" } else { "FALSE" },
            reception_timeout
        )));
        if signal.receive {
            receive.push(row(&format!("{}u, {can_if}, 4u", signal.can_id)));
            ls_rx.push(row(&format!(
                "{ls_source}, {source}, &PduR_CanIfRxIndication"
            )));
            pd_rx.push(row(&format!("{source}, {pdu}, &Com_RxIndication")));
        } else {
            transmit.push(row(&format!("{}u, {can_if}, 4u", signal.can_id)));
            ls_tx.push(row(&format!(
                "LSDUR_UP_PDUR, {ls_source}, {can_if}, &PduR_CanIfTxConfirmation, &PduR_CanIfTriggerTransmit"
            )));
            pd_tx.push(row(&format!("PDUR_UP_COM, {source}, PDUR_LO_LSDUR, {ls_source}, &LSduR_PduRTransmit, &Com_TxConfirmation, &Com_TriggerTransmit")));
        }
    }
    config_data(
        &mut config,
        "Com",
        "static const Com_PduConfigType com_pdus[]",
        &pdus,
    );
    config_data(
        &mut config,
        "Com",
        "static const Com_ConfigType com_config",
        &values(&format!(
            "com_pdus, {}u, {}, &Rte_COMCbk, &Rte_COMCbkRxTOut, &PduR_ComTransmit",
            description.signals.len(),
            name("Com", "ComIPduGroup", &com.receive_group.path)
        )),
    );
    let transport = &description.diagnostic_transport;
    if let Some(tp) = transport {
        let rx_route = description
            .routes
            .iter()
            .find(|route| route.pdu == tp.rx_sdu)
            .ok_or_else(|| PlanDiagnostic::plan_premise(&tp.rx_sdu))?;
        let tx_route = description
            .routes
            .iter()
            .find(|route| route.pdu == tp.tx_sdu)
            .ok_or_else(|| PlanDiagnostic::plan_premise(&tp.tx_sdu))?;
        let can_rx = symbolic.reference("CanIfRxPduRef", &tp.rx_npdu)?;
        let can_tx = symbolic.reference("CanIfTxPduRef", &tp.tx_npdu)?;
        let tp_rx = symbolic.parameter("CanTpRxNPduId")?;
        let tp_rx_sdu = symbolic.parameter("CanTpRxNSduId")?;
        let tp_tx_sdu = symbolic.parameter("CanTpTxNSduId")?;
        let dcm_rx = symbolic.parameter("DcmDslProtocolRxPduId")?;
        let dcm_tx = symbolic.parameter("DcmDslTxConfirmationPduId")?;
        let pd_rx_source = name("PduR", "PduRSrcPdu", &rx_route.source);
        let pd_tx_source = name("PduR", "PduRSrcPdu", &tx_route.source);
        let pd_tx_destination = name("PduR", "PduRDestPdu", &tx_route.destination);
        let ls_rx_source = name("LSduR", "LSduRPathSourcePdu", &rx_route.source);
        let ls_tx_source = name("LSduR", "LSduRPathSourcePdu", &tx_route.source);
        receive.push(row(&format!("{}u, {can_rx}, 8u", tp.request_can_id)));
        transmit.push(row(&format!("{}u, {can_tx}, 8u", tp.response_can_id)));
        ls_rx.push(row(&format!(
            "{ls_rx_source}, {tp_rx}, &CanTp_RxIndication"
        )));
        ls_tx.push(row(&format!(
            "LSDUR_UP_CANTP, {ls_tx_source}, {can_tx}, &CanTp_TxConfirmation, NULL_PTR"
        )));
        // CanTp confirms through tp_routes, under its own N-SDU namespace.
        pd_tx.push(row(&format!("PDUR_UP_DCM, {pd_tx_source}, PDUR_LO_CANTP, {tp_tx_sdu}, &CanTp_Transmit, NULL_PTR, NULL_PTR")));
        config_data(
            &mut pd_configuration,
            "PduR",
            "static const PduR_TpRouteType tp_routes[]",
            &[row(&format!(
                "{pd_rx_source}, {dcm_rx}, {pd_tx_destination}, {dcm_tx}, &Dcm_StartOfReception, &Dcm_CopyRxData, &Dcm_TpRxIndication, &Dcm_CopyTxData, &Dcm_TpTxConfirmation"
            ))],
        );
        let period = description.schedule.counter_tick_ms;
        let ticks = |value| periods(value, period, &description.schedule.counter);
        config_data(
            &mut config,
            "CanTp",
            "static const CanTp_ConfigType tp_config",
            &values(&format!(
                "{tp_rx_sdu}, {tp_tx_sdu}, {tp_rx}, {ls_tx_source}, {}u, {}u, {}u, {}u, {}u, {}u, {}u, {period}u, 0u, 0u, {pd_rx_source}, {pd_tx_destination}",
                tp.buffer_bytes,
                ticks(tp.n_as_ms)?,
                ticks(tp.n_ar_ms)?,
                ticks(tp.n_br_ms)?,
                ticks(tp.n_bs_ms)?,
                ticks(tp.n_cr_ms)?,
                ticks(tp.n_cs_ms)?
            )),
        );
        let (did, read) = match &description.diagnostic {
            Some(did) => {
                let bridge = composition
                    .components
                    .iter()
                    .flat_map(|component| &component.runnables)
                    .find(|runnable| runnable.event.is_none())
                    .ok_or_else(|| PlanDiagnostic::plan_premise(&did.client_port))?;
                (did.did, format!("&{}", bridge.symbol))
            }
            None => (0, "NULL_PTR".to_string()),
        };
        config.push_str(
            r#"static void Ecu_DcmAuthenticationMode(uint8 state);

"#,
        );
        config_data(
            &mut config,
            "Dcm",
            "static const Dcm_ConfigType dcm_config",
            &values(&format!(
                "{dcm_rx}, {dcm_tx}, {}, {did}u, {read}, {}u, {}u, {}u, {}u, {}u, {pd_tx_source}, &Ecu_DcmAuthenticationMode, {}u",
                name("ComM", "ComMChannel", &mode.channel),
                ticks(tp.p2_ms)?,
                tp.p2_ms,
                tp.p2_star_ms,
                ticks(tp.s3_ms)?,
                tp.buffer_bytes,
                tp.max_did_to_read.unwrap_or(0)
            )),
        );
    }
    for (scope, name, ty, entries) in [
        ("CanIf", "can_rx", "CanIf_RxPduConfigType", &receive),
        ("CanIf", "can_tx", "CanIf_TxPduConfigType", &transmit),
        ("LSduR", "ls_rx", "LSduR_RxRouteType", &ls_rx),
        ("LSduR", "ls_tx", "LSduR_TxRouteType", &ls_tx),
        ("PduR", "pd_rx", "PduR_RxRouteType", &pd_rx),
        ("PduR", "pd_tx", "PduR_TxRouteType", &pd_tx),
    ] {
        let owner = match scope {
            "LSduR" => &mut ls_configuration,
            "PduR" => &mut pd_configuration,
            _ => &mut can_if_configuration,
        };
        config_data(
            owner,
            scope,
            &format!("static const {ty} {name}[]"),
            entries,
        );
    }
    let rx_signals = description
        .signals
        .iter()
        .filter(|signal| signal.receive)
        .count();
    let rx_count = rx_signals + usize::from(transport.is_some());
    let tx_count = description
        .signals
        .iter()
        .filter(|signal| !signal.receive)
        .count()
        + usize::from(transport.is_some());
    config_data(
        &mut can_if_configuration,
        "CanIf",
        "const CanIf_ConfigType CanIf_Config",
        &values(&format!(
            "can_rx, {rx_count}u, can_tx, {tx_count}u, &Ecu_HostBusSM_ControllerModeIndication, &Ecu_HostBusSM_ControllerBusOff, {}, {}, {}, {}",
            symbolic.parameter("CanIfCtrlId")?,
            name("Can", "CanController", &can.controller),
            name("Can", "CanHardwareObject", &can.receive_hardware),
            name("Can", "CanHardwareObject", &can.transmit_hardware)
        )),
    );
    config_data(
        &mut ls_configuration,
        "LSduR",
        "const LSduR_PBConfigType LSduR_Config",
        &values(&format!("0u, ls_rx, {rx_count}u, ls_tx, {tx_count}u")),
    );
    let configuration_id = "/AUTOSAR/EcucDefs/PduR/PduRRoutingPaths/PduRConfigurationId";
    let configuration_id = description
        .configuration
        .iter()
        .find_map(|record| record.parameters.get(configuration_id))
        .and_then(|values| values.first())
        .ok_or_else(|| PlanDiagnostic::plan_premise(configuration_id))?;
    config_data(
        &mut pd_configuration,
        "PduR",
        "const PduR_PBConfigType PduR_Config",
        &values(&format!(
            "{configuration_id}u, pd_rx, {}u, pd_tx, {tx_count}u, {}, {}u",
            rx_signals,
            if transport.is_some() {
                "tp_routes"
            } else {
                "NULL_PTR"
            },
            usize::from(transport.is_some())
        )),
    );
    // Post-build sets live in <Mip>_PBcfg.c (SWS_BSW_00015).
    for (module, ty, source) in [
        ("CanIf", "CanIf_ConfigType", &can_if_configuration),
        ("PduR", "PduR_PBConfigType", &pd_configuration),
        ("LSduR", "LSduR_PBConfigType", &ls_configuration),
    ] {
        outputs.emit(
            ECU,
            format!("include/{module}_Cfg.h"),
            header(
                module,
                &format!("{}_CFG_H", module.to_uppercase()),
                &format!(
                    "#include \"{module}.h\"\n{}extern const {ty} {module}_Config;",
                    c_layout::commented_defines(
                        symbolic
                            .headers
                            .get(module)
                            .map(Vec::as_slice)
                            .unwrap_or_default(),
                        "Generated handle."
                    )
                ),
            ),
        )?;
        let source = if module == "LSduR" {
            source.to_owned()
        } else {
            format!(
                "{source}{}",
                c_layout::release_check(&format!("{module}_PBcfg.c"), "Rte_MemMap.h", "MEMMAP")
            )
        };
        outputs.emit(ECU, format!("src/{module}_PBcfg.c"), source)?;
    }

    config_data(
        &mut config,
        "Ecu_HostBusSM",
        "static const Ecu_HostBusSM_ConfigType bus_config",
        &values(&format!(
            "{}, {}",
            name("ComM", "ComMChannel", &mode.channel),
            symbolic.parameter("CanIfCtrlId")?
        )),
    );
    config_data(
        &mut config,
        "ComM",
        "static const ComM_UserHandleType users[]",
        &mode
            .users
            .iter()
            .map(|user| Item::Value(name("ComM", "ComMUser", &user.path)))
            .collect::<Vec<_>>(),
    );
    config_data(
        &mut config,
        "ComM",
        "static const ComM_ConfigType comm_config",
        &values(&format!(
            "{}, users, {}u, {}u, &Ecu_HostBusSM_RequestComMode, &Ecu_HostBusSM_GetCurrentComMode, {}",
            name("ComM", "ComMChannel", &mode.channel),
            mode.users.len(),
            periods(mode.minimum_full_ms, mode.period_ms, &mode.main_symbol)?,
            if transport.is_some() {
                "&Dcm_ComM_NoComModeEntered, &Dcm_ComM_SilentComModeEntered, &Dcm_ComM_FullComModeEntered"
            } else {
                "NULL_PTR, NULL_PTR, NULL_PTR"
            }
        )),
    );
    config_data(
        &mut config,
        "BswM",
        "static const BswM_ConfigType bswm_config",
        &values(&format!(
            "{}, &Ecu_BswMAction",
            name("ComM", "ComMChannel", &mode.channel)
        )),
    );
    for (symbol, scope, body) in [
        (&com.receive_main.symbol, "Com", "Ecu_ComMainFunctionRx();"),
        (&com.transmit_main.symbol, "Com", "Ecu_ComMainFunctionTx();"),
        (&mode.main_symbol, "ComM", "(void)ComM_RunChannel();"),
    ] {
        let function =
            c_layout::short_function(&format!("RTE_{scope}_CODE void {symbol}(void)"), body);
        config.push_str(&format!(
            r#"{start_code}{function}
{stop_code}"#,
            start_code = c_layout::section(&format!("RTE_{scope}"), "Rte", "CODE", true),
            stop_code = c_layout::section(&format!("RTE_{scope}"), "Rte", "CODE", false)
        ));
    }
    // SWS_CanTp_00229 timeouts reach the HostBatch receipt through the
    // standard runtime callout (DetReportRuntimeErrorCallout), not a CanTp hook.
    config_data(
        &mut config,
        "Det",
        "static const Det_ErrorHookType det_runtime_callouts[1]",
        &values("&Ecu_DetRuntimeCallout"),
    );
    config_data(
        &mut config,
        "Det",
        "static const Det_ConfigType det_config",
        &values("NULL_PTR, 0u, det_runtime_callouts, 1u"),
    );
    config.push_str(&c_layout::section("RTE_Ecu", "Rte", "CODE", true));
    // ECUC_BswM_00843: parameters are fixed by the configured complete call.
    config.push_str(
        r#"static RTE_Ecu_CODE Std_ReturnType Ecu_BswMAction(NetworkHandleType channel,
                                                  ComM_ModeType requested)
{
    Std_ReturnType result = E_OK;
"#,
    );
    config.push_str(&format!(
        r#"    if (channel != {}) {{
        return E_NOT_OK;
    }}
    switch (requested) {{
"#,
        name("ComM", "ComMChannel", &mode.channel)
    ));
    for rule in &mode.rules {
        let requested = [
            "COMM_NO_COMMUNICATION",
            "COMM_SILENT_COMMUNICATION",
            "COMM_FULL_COMMUNICATION",
        ][usize::from(rule.mode)];
        let channel = name("ComM", "ComMChannel", &mode.channel);
        config.push_str(&format!(
            r#"        case {}u:
            (void)Ecu_HostBusSM_ApplyMode({channel}, {requested});
            break;
"#,
            rule.mode
        ));
    }
    config.push_str(
        r#"        default:
            result = E_NOT_OK;
            break;
    }
    return result;
}
"#,
    );
    // Eight initialization stages; a test build can fail before each one, and
    // retirement undoes only the stages that began, newest first.
    let stage = |number: u8| {
        format!(
            r#"#if defined(ECU_TARGET_TESTS) && (ECU_TARGET_TESTS == STD_ON)
    if (Ecu_TargetTestFailStage({number}u) != 0) {{
        return E_NOT_OK;
    }}
#endif
    *reached = {number}u;
"#
        )
    };
    config.push_str(
        r#"RTE_Ecu_CODE Std_ReturnType Ecu_StackBootstrap(uint8 *reached)
{
    Can_ControllerStateType actual;
    if ((reached == NULL_PTR) || (Ecu_TargetCheckLifecycleContext(TRUE) != E_OK)) {
        return E_NOT_OK;
    }
    *reached = 0u;
"#,
    );
    config.push_str(&stage(1));
    config.push_str(
        r#"    SchM_Init(&Ecu_SchMConfig);
    SchM_Start();
    if (Ecu_SchMReady() == FALSE) {
        return E_NOT_OK;
    }
"#,
    );
    config.push_str(&stage(2));
    config.push_str(
        r#"    Det_Init(&det_config);
    Det_Start();
"#,
    );
    config.push_str(&stage(3));
    config.push_str("    Can_Init(&driver);\n");
    config.push_str(&stage(4));
    config.push_str(
        r#"    CanIf_Init(&CanIf_Config);
    Ecu_HostBusSM_Init(&bus_config);
"#,
    );
    config.push_str(&stage(5));
    config.push_str(
        r#"    LSduR_Init(&LSduR_Config);
    PduR_Init(&PduR_Config);
"#,
    );
    config.push_str(&stage(6));
    config.push_str(
        r#"    Com_Init(&com_config);
    Com_IpduGroupStart(com_config.receive_group, TRUE);
    Com_EnableReceptionDM(com_config.receive_group);
    ComM_Init(&comm_config);
    BswM_Init(&bswm_config);
"#,
    );
    // Stage 7 exists only with a diagnostic transport; the numbering stays fixed.
    if transport.is_some() {
        config.push_str(&stage(7));
        config.push_str(
            r#"    CanTp_Init(&tp_config);
    Dcm_Init(&dcm_config);
"#,
        );
    }
    let first_user = mode
        .users
        .first()
        .ok_or_else(|| PlanDiagnostic::plan_premise(&mode.channel))?;
    config.push_str(&stage(8));
    config.push_str(&format!(
        r#"    if (Rte_Start() != E_OK) {{
        return E_NOT_OK;
    }}
    ComM_CommunicationAllowed({}, TRUE);
    if (ComM_RequestComMode({}, COMM_FULL_COMMUNICATION) != E_OK) {{
        return E_NOT_OK;
    }}
    {}();
    Can_MainFunction_Mode();
    if ((CanIf_GetControllerMode({}, &actual) != E_OK) ||
        (actual != CAN_CS_STARTED)) {{
        return E_NOT_OK;
    }}
    SchM_StartTiming();
    return Ecu_SchMTimingActive() ? E_OK : E_NOT_OK;
}}
"#,
        name("ComM", "ComMChannel", &mode.channel),
        name("ComM", "ComMUser", &first_user.path),
        mode.main_symbol,
        symbolic.parameter("CanIfCtrlId")?
    ));
    config.push_str(
        r#"RTE_Ecu_CODE Std_ReturnType Ecu_StackRunCycle(EventMaskType events)
{
    if ((Ecu_TargetIsOwner() == 0) || (Ecu_SchMTimingActive() == FALSE)) {
        return E_NOT_OK;
    }
"#,
    );
    for entity in &description.schedule.entities {
        if entity.application {
            config.push_str(&format!(
                r#"    if ((events & {}u) != 0u) {{
        {}();
    }}
"#,
                description.event_mask(&entity.os_event)?,
                entity.symbol
            ));
        } else {
            let symbol = &entity.symbol;
            config.push_str(&format!(
                r#"    if ((events & {}u) != 0u) {{
        {symbol}();
    }}
"#,
                description.event_mask(&entity.os_event)?
            ));
            if entity.symbol == com.transmit_main.symbol {
                config.push_str(
                    r#"    if (Can_HostFlush() != ECU_OK) {
        return E_NOT_OK;
    }
"#,
                );
            }
        }
    }
    config.push_str(
        r#"    return (Can_HostFlush() == ECU_OK) ? E_OK : E_NOT_OK;
}
RTE_Ecu_CODE void Ecu_StackRetire(uint8 reached)
{
"#,
    );
    // Det (2) and the routing tables (5) have no deinitialization service.
    let mut retired = vec![(8, "(void)Rte_Stop();\n".to_owned())];
    if transport.is_some() {
        retired.push((7, "CanTp_Shutdown();\n".to_owned()));
    }
    retired.push((
        6,
        r#"BswM_Deinit();
        Com_DeInit();
"#
        .to_owned(),
    ));
    retired.push((
        4,
        r#"Ecu_HostBusSM_DeInit();
        CanIf_DeInit();
"#
        .to_owned(),
    ));
    retired.push((
        3,
        format!(
            r#"(void)Can_SetControllerMode({}, CAN_CS_STOPPED);
        Can_MainFunction_Mode();
        Can_MainFunction_BusOff();
        Can_DeInit();
"#,
            name("Can", "CanController", &can.controller)
        ),
    ));
    retired.push((1, "SchM_Deinit();\n".to_owned()));
    for (number, body) in retired {
        config.push_str(&format!(
            r#"    if (reached >= {number}u) {{
#if defined(ECU_TARGET_TESTS) && (ECU_TARGET_TESTS == STD_ON)
        Ecu_TargetTestRetire({number}u);
#endif
        {body}    }}
"#
        ));
    }
    config.push_str("}\n");
    config.push_str(
        r#"RTE_Ecu_CODE int Ecu_StackFrameShape(uint32 id, uint8 dlc)
{
    if ((id > 0x7ffu) || (dlc > 8u)) {
        return 0;
    }
"#,
    );
    for signal in &description.signals {
        config.push_str(&format!(
            r#"    if (id == {}u) {{
        return {};
    }}
"#,
            signal.can_id,
            if signal.receive { "dlc == 4u" } else { "0" }
        ));
    }
    if let Some(tp) = transport {
        config.push_str(&format!(
            r#"    if (id == {}u) {{
        return dlc == 8u;
    }}
    if (id == {}u) {{
        return 0;
    }}
"#,
            tp.request_can_id, tp.response_can_id
        ));
    }
    config.push_str(
        r#"    return 1;
}
"#,
    );
    config.push_str(&c_layout::section("RTE_Ecu", "Rte", "CODE", false));
    if description.diagnostic_transport.is_some() {
        let function = c_layout::short_function(
            "static RTE_Dcm_CODE void Ecu_DcmAuthenticationMode(uint8 state)",
            &format!("(void)SchM_Switch_Dcm_{authentication_group}(state);"),
        );
        config.push_str(&format!(
            r#"{start_code}{function}
{stop_code}"#,
            start_code = c_layout::section("RTE_Dcm", "Rte", "CODE", true),
            stop_code = c_layout::section("RTE_Dcm", "Rte", "CODE", false)
        ));
    }
    config.push_str(&c_layout::release_check(
        "Ecu_RuntimeConfig.c",
        "Rte_MemMap.h",
        "MEMMAP",
    ));
    outputs.emit(ECU, "src/Ecu_RuntimeConfig.c", config)?;
    let mode_scope = super::memmap::dcm_scope(composition);
    let path = "src/SchM_Main.c";
    let lifecycle = super::outputs::render(
        outputs.template(ECU, path)?,
        path,
        &[
            super::outputs::Edit::Every("@PARTITION@", &communication.partition.id.to_string()),
            super::outputs::Edit::Once("@MODES@", &mode_switches(mode_scope, authentication_group)),
        ],
    )?;
    outputs.emit_rendered(ECU, path, lifecycle)?;
    outputs.emit_verbatim(ECU)?;
    // Only assets delivered byte-for-byte are original sources; templates an
    // emitter extended are generated outputs. The packaging tool is wrapped at delivery.
    let mut original_sources = outputs.original_assets();
    original_sources.remove("runtime/ecu-tool.py");
    outputs.emit(ECU, super::outputs::INTEGRATION_PATH, format!("{:#}\n", serde_json::json!({"format": "autosar-ecu-integration-v2", "plan": description, "target": target.spec().id, "owner": "single source-derived AUTOSTART extended task", "originalBswSources": original_sources })))?;
    let verification = super::handoff::verification_files(plan).map_err(|_| failure())?;
    let readme = format!(
        r#"# ECU integration source project

Target: `{}`. Caller application sources are sealed under `app/`; the source-derived component, communication and OS contracts are in `meta/integration.json`. Original MIT notices of the fixed FreeRTOS kernel remain in `kernel/LICENSE.md`; Autsaro runtime, tool and template code is licensed under Apache-2.0 (see LICENSE and NOTICE). User-provided configurations and application code retain their own licensing.

## Independent native build

{}
From this directory run `python tools/ecu-tool.py build --project . --output <new-empty-external-directory> --mode host-batch`. The production BEGIN/RX/COMMIT protocol uses the single OS owner and controlled logical ticks. Physical output flush precedes driver polling confirmation. No compiler, AUTOSAR archive or application algorithm is generated or redistributed.

`CMakeLists.txt` builds the same sources with the same flags, kernel patches and libraries from `target.json` for IDEs and CI: run `cmake -S . -B <outside-build-directory> -G Ninja -DCMAKE_C_COMPILER=<gcc>` and `cmake --build <outside-build-directory>`. CMake neither checks the pinned toolchain identity and target sections nor runs `verify`; `tools/ecu-tool.py` does.

## Layout

The [module manuals](generated/descriptions/MODULES.md) describe delivered BSW interfaces, exclusive areas, scheduling, selected configuration and integration limits. The selected implementation description is [Host_Implementation.arxml](generated/descriptions/Host_Implementation.arxml); input BSW descriptions remain source provenance.

`bsw/` holds the static BSW, `integration/` the host adapters and both mains, `os/` and `kernel/` the OS adaptation, kernel patches and fixed kernel. `generated/` holds every configuration-derived file and is rewritten on each generation; `generated/templates/` holds component entry-point skeletons that are not compiled. `app/<component>/` is the sealed snapshot of each live application source, `inputs/` the configuration snapshot and `meta/` the package metadata and seal. Edit applications and configuration in the live project, never in this package.
{}"#,
        target.spec().id,
        crate::generator::delivery::native_build_prerequisites(target),
        if verification.is_empty() {
            ""
        } else {
            r#"
## Reference behaviour verification

This configuration has one 32-bit CAN receive signal, one 32-bit CAN transmit signal and an application DID, so `meta/verification-inputs.json` records their configured endpoints. `python tools/ecu-tool.py verify --project . --build-directory <another-new-empty-external-directory>` builds HostBatch and checks the reference echo application: the transmit signal repeats the last received value, the DID returns it big-endian, one/two-DID reads, DID capacity refusal, N_Cr timeout/recovery and malformed admission. Other application behaviour fails this check by design; it does not establish full SC1, MCU, hard-real-time, ASIL or official certification.
"#
        }
    );
    outputs.emit_all("handoff-verification", verification)?;
    let artifacts = super::artifacts::files(description, &outputs.contents())?;
    outputs.emit_all("artifacts", artifacts)?;
    // Not compiled: the create-only live seed, to compare entry points after a configuration change.
    outputs.emit_all(
        "templates",
        plan.application_seed_files()?
            .into_iter()
            .map(|(path, bytes)| {
                let name = path.trim_start_matches("application/");
                (format!("generated/templates/{name}"), bytes)
            }),
    )?;
    Ok((outputs.finish()?, readme))
}

/// SchM lifecycle and the Dcm mode switches over the selected mode scope.
fn mode_switches(mode_scope: &str, authentication_group: &str) -> String {
    let mut modes = format!(
        r#"
{start_var_cleared_unspecified}"#,
        start_var_cleared_unspecified = c_layout::section(
            &format!("RTE_{mode_scope}"),
            "Rte",
            "VAR_CLEARED_UNSPECIFIED",
            true
        )
    );
    for (group, variable) in [
        ("DcmDiagnosticSessionControl", "diagnostic_session_mode"),
        (authentication_group, "authentication_mode"),
    ] {
        let declaration = format!("static Rte_ModeType_{group} {variable}");
        let attribute = format!("RTE_{mode_scope}_VAR_CLEARED_UNSPECIFIED;");
        if declaration.len() + attribute.len() < 100 {
            modes.push_str(&format!("{declaration} {attribute}\n"));
        } else {
            modes.push_str(&format!(
                r#"{declaration}
    {attribute}
"#
            ));
        }
    }
    modes.push_str(&format!(
        r#"{stop_var_cleared_unspecified}{start_code}"#,
        stop_var_cleared_unspecified = c_layout::section(
            &format!("RTE_{mode_scope}"),
            "Rte",
            "VAR_CLEARED_UNSPECIFIED",
            false
        ),
        start_code = c_layout::section(&format!("RTE_{mode_scope}"), "Rte", "CODE", true)
    ));
    for (group, variable) in [
        (authentication_group, "authentication_mode"),
        ("DcmDiagnosticSessionControl", "diagnostic_session_mode"),
    ] {
        let switch = c_layout::signature(
            "RTE_CODE Std_ReturnType",
            &format!("SchM_Switch_Dcm_{group}"),
            &[format!("Rte_ModeType_{group} mode")],
            "",
            true,
        );
        let mode = c_layout::signature(
            &format!("RTE_CODE Rte_ModeType_{group}"),
            &format!("SchM_Mode_Dcm_{group}"),
            &["void".to_owned()],
            "",
            true,
        );
        modes.push_str(&format!(
            r#"{switch}
{{
    Can_HostAreaEnter(CAN_HOST_AREA_SCHM_MODE);
    {variable} = mode;
    Can_HostAreaExit(CAN_HOST_AREA_SCHM_MODE);
    return SCHM_E_OK;
}}
{mode}
{{
    Rte_ModeType_{group} mode;
    Can_HostAreaEnter(CAN_HOST_AREA_SCHM_MODE);
    mode = {variable};
    Can_HostAreaExit(CAN_HOST_AREA_SCHM_MODE);
    return mode;
}}
"#
        ));
    }
    modes.push_str(&c_layout::section(
        &format!("RTE_{mode_scope}"),
        "Rte",
        "CODE",
        false,
    ));
    modes
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_zero_period_is_a_located_plan_premise() {
        assert_eq!(super::periods(1500, 10, "/Os/Counter").unwrap(), 150);
        let zero = super::periods(1500, 0, "/Os/Counter").unwrap_err();
        assert_eq!(zero[0].code, "PLAN_PREMISE");
        assert_eq!(zero[0].object.as_deref(), Some("/Os/Counter"));
    }
}
