//! ECU configuration and integration sources from the selected plan; applications remain caller owned.
use super::c_layout::{self, Item};
use super::{PlanDiagnostic, ValidatedIntegrationPlan};
use crate::target::BuildTarget;

fn header(name: &str, body: &str) -> String {
    format!(
        "/** @file Generated from the selected source contract. */\n#ifndef {name}\n#define {name}\n{body}\n#endif\n"
    )
}
/// Module interlink headers share the standard type import and C++ linkage.
fn schm_header(module: &str, body: &str) -> String {
    header(
        &format!("SCHM_{}_H", module.to_uppercase()),
        &format!(
            "#include \"SchM_{module}_Type.h\"\n#ifdef __cplusplus\nextern \"C\" {{\n#endif\n{body}\n#ifdef __cplusplus\n}}\n#endif\n"
        ),
    )
}

fn code_prototypes(module: &str, declarations: &str) -> String {
    let prefix = module.to_uppercase();
    format!(
        "\n#define {prefix}_START_SEC_CODE\n#include \"{module}_MemMap.h\"\n{declarations}\n#define {prefix}_STOP_SEC_CODE\n#include \"{module}_MemMap.h\"\n"
    )
}

fn failure() -> Vec<PlanDiagnostic> {
    vec![PlanDiagnostic::dependency(
        "ECU_SOURCE_CLOSURE",
        crate::product_message!("backend.integration.composition.consumer_unsupported"),
        crate::product_message!("backend.integration.composition.repair_contract"),
    )]
}

const ECU: &str = "ecu-config";

/// Writes one constant in its module's pre-build configuration data section.
fn config_data(output: &mut String, scope: &str, declaration: &str, items: &[Item]) {
    let definition = c_layout::initializer(
        declaration,
        &format!("RTE_{scope}_CONFIG_DATA_PREBUILD_UNSPECIFIED"),
        items,
    );
    output.push_str(&format!("#define RTE_{scope}_START_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n{definition}\n#define RTE_{scope}_STOP_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n"));
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
    let mut outputs = super::outputs::EcuOutputs::new(target)?;
    // The contract provenance is package metadata; its headers are generated C.
    for (path, bytes) in plan.contract_headers(composition)? {
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
            "/** Publish the selected connection authentication mode.\n * @param[in] mode 0 for deauthenticated, 1 for authenticated.\n * @return SCHM_E_OK after publishing the immediate mode.\n * Preconditions: initialized ECU.\n * Context: ECU task or callback; not inside an exclusive area.\n * Reentrancy: reentrant; the Dcm publisher orders updates.\n * Ownership: no pointer is retained.\n */\n{}\n/** Read the selected connection authentication mode.\n * @return Current mode.\n * Preconditions: initialized ECU.\n * Context: ECU task or callback; not inside an exclusive area.\n * Reentrancy: reentrant.\n * Ownership: None.\n */\n{}\n",
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
        "#ifdef __cplusplus\nextern \"C\" {{\n#endif\n{addition}#ifdef __cplusplus\n}}\n#endif\n"
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
    outputs.emit(
        ECU,
        "include/Ecu_Config.h",
        header(
            "ECU_CONFIG_H",
            "#include \"Std_Types.h\"\ntypedef enum { ECU_TX_SYNCHRONOUS = 0, ECU_TX_QUEUED = 1 } EcuTxConfirmation;\ntypedef struct {\n    EcuTxConfirmation tx_confirmation;\n} EcuPolicyConfig;\nextern const EcuPolicyConfig Ecu_Policy;",
        ),
    )?;
    outputs.emit_all(ECU, super::can_runtime::type_headers(communication))?;
    let symbols: Vec<_> = description
        .handles
        .iter()
        .filter(|handle| handle.domain == "com_signal")
        .map(|handle| {
            (
                super::com::symbolic_name(&handle.path),
                format!("{}U", handle.handle),
            )
        })
        .collect();
    outputs.emit(
        ECU,
        "include/Com_Cfg.h",
        header("COM_CFG_H", &c_layout::defines(&symbols)),
    )?;

    outputs.emit(
        ECU,
        "src/Ecu_Execution.h",
        header(
            "ECU_EXECUTION_H",
            "#include \"Ecu_Config.h\"\n#include \"Can_Host.h\"\nEcuStatus Ecu_ExecutionTransmit(\n    Can_TxSinkType sink, PduIdType handle, uint32_t id, uint8_t dlc, const uint8_t data[8]);\nuint64_t Ecu_ExecutionNow(void);",
        ),
    )?;
    outputs.emit(ECU, "src/Ecu_Execution.c", "#include \"Ecu_Execution.h\"\n#include \"Ecu_Target.h\"\nEcuStatus Ecu_ExecutionTransmit(\n    Can_TxSinkType sink, PduIdType handle, uint32_t id, uint8_t dlc, const uint8_t data[8])\n{\n    (void)sink;\n    return Ecu_TargetEnqueueTransmit(handle, id, dlc, data);\n}\nuint64_t Ecu_ExecutionNow(void) { return Ecu_TargetNow(); }\n")?;
    for (module, area) in [
        ("CanIf", "CANIF"),
        ("ComM", "COMM"),
        ("CanTp", "CANTP"),
        ("Det", "DET"),
        ("PduR", "PDUR"),
        ("LSduR", "LSDUR"),
    ] {
        let main = match module {
            "CanTp" => code_prototypes(module, "void CanTp_MainFunction(void);"),
            "ComM" => code_prototypes(module, &format!("void {}(void);", mode.main_symbol)),
            _ => String::new(),
        };
        outputs.emit(
            ECU,
            format!("include/SchM_{module}.h"),
            schm_header(
                module,
                &format!(
                    "void SchM_Enter_{module}_{area}_STATE(void);\nvoid SchM_Exit_{module}_{area}_STATE(void);{main}"
                ),
            ),
        )?;
    }
    outputs.emit(
        ECU,
        "include/SchM_Com.h",
        schm_header(
            "Com",
            &format!(
                "void SchM_Enter_Com_COM_STATE(void);\nvoid SchM_Exit_Com_COM_STATE(void);{}",
                code_prototypes(
                    "Com",
                    &format!(
                        "void {}(void);\nvoid {}(void);",
                        com.receive_main.symbol, com.transmit_main.symbol
                    )
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
                &format!("SCHM_{}_TYPE_H", module.to_uppercase()),
                &format!(
                    "#include \"Rte_Type.h\"\n{}",
                    if module == "Dcm" {
                        "#include \"Rte_Dcm_Type.h\"\n"
                    } else {
                        ""
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
    outputs.emit(
        ECU,
        "include/Ecu_TargetConfig.h",
        header(
            "ECU_TARGET_CONFIG_H",
            &format!(
                "#include \"Ecu_ProfileConfig.h\"\n#include \"Os_Target.h\"\n{}extern const Os_TargetConfig Ecu_OsConfig;\n/* Records in *reached each initialization stage (1 through 8) as it begins. */\nStd_ReturnType Ecu_StackBootstrap(uint8 *reached);\nStd_ReturnType Ecu_StackRunCycle(EventMaskType events);\n/* Retires the stages that began, newest first. */\nvoid Ecu_StackRetire(uint8 reached);\nint Ecu_StackFrameShape(uint32 id, uint8 dlc);\n#ifdef ECU_TARGET_TESTS\nint Ecu_TargetTestFailStage(unsigned stage);\nvoid Ecu_TargetTestRetire(unsigned stage);\nvoid Ecu_TargetTestShutdown(StatusType reason);\n#endif",
                c_layout::defines(&[
                    ("ECU_TARGET_TASK", "0u".to_owned()),
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
                ]),
            ),
        ),
    )?;
    let mut config = String::from(
        "/** @file Source-derived immutable communication configurations and owner dispatch. */\n#define RTE_CORE\n#include \"Ecu_TargetConfig.h\"\n#include \"Ecu_Target.h\"\n#include \"Ecu_Config.h\"\n#include \"Can_Host.h\"\n#include \"CanIf.h\"\n#include \"Det.h\"\n#include \"Det_Externals.h\"\n#include \"CanTp.h\"\n#include \"Com.h\"\n#include \"Com_Internal.h\"\n#include \"ComM.h\"\n#include \"ComM_BswM.h\"\n#include \"ComM_Internal.h\"\n#include \"BswM.h\"\n#include \"BswM_ComM.h\"\n#include \"Ecu_HostBusSM.h\"\n#include \"Dcm.h\"\n#include \"Dcm_ComM.h\"\n#include \"PduR.h\"\n#include \"PduR_Com.h\"\n#include \"PduR_CanIf.h\"\n#include \"LSduR.h\"\n#include \"LSduR_PduR.h\"\n#include \"Rte_Main.h\"\n#include \"Rte_Com.h\"\n#include \"SchM_Can.h\"\n#include \"SchM_CanTp.h\"\n#include \"SchM_Com.h\"\n#include \"SchM_ComM.h\"\n#include \"SchM_Dcm.h\"\n",
    );
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
    let routing_headers = "#include \"Com.h\"\n#include \"Dcm.h\"\n#include \"CanTp.h\"\n#include \"PduR.h\"\n#include \"PduR_Com.h\"\n#include \"PduR_CanIf.h\"\n#include \"LSduR.h\"\n#include \"LSduR_PduR.h\"\n";
    let mut pd_configuration = format!(
        "/** @file Source-derived selected PduR configuration. */\n#include \"PduR_Cfg.h\"\n{routing_headers}"
    );
    let mut ls_configuration = format!(
        "/** @file Source-derived selected LSduR configuration. */\n#include \"LSduR_Cfg.h\"\n{routing_headers}"
    );
    let mut can_if_configuration = "/** @file Source-derived selected CanIf configuration. */\n#include \"CanIf_Cfg.h\"\n#include \"Ecu_HostBusSM.h\"\n".to_owned();
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
        // No explicit ComIPduHandleId is selected. Assign its generated ID from
        // the corresponding unique route; public router IDs remain separate.
        let pdu = if signal.receive {
            route.destination_handle
        } else {
            route.source_handle
        };
        let signal_id = description
            .handles
            .iter()
            .find(|handle| handle.domain == "com_signal" && handle.path == signal.com_signal)
            .ok_or_else(|| PlanDiagnostic::plan_premise(&signal.com_signal))?
            .handle;
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
        pdus.push(row(&format!(
            "{pdu}u, {signal_id}u, {}, {initial}u, {}u, {}u",
            if signal.receive { "TRUE" } else { "FALSE" },
            reception_timeout,
            reception.map_or(0, |rx| u32::from(rx.callback_handle))
        )));
        if signal.receive {
            receive.push(row(&format!(
                "{}u, {}u, 4u",
                signal.can_id, signal.can_if_handle
            )));
            ls_rx.push(row(&format!(
                "{}u, {}u, PduR_CanIfRxIndication",
                signal.can_if_handle, route.source_handle
            )));
            pd_rx.push(row(&format!(
                "{}u, {pdu}u, Com_RxIndication",
                route.source_handle
            )));
        } else {
            transmit.push(row(&format!(
                "{}u, {}u, 4u",
                signal.can_id, signal.can_if_handle
            )));
            ls_tx.push(row(&format!(
                "LSDUR_UP_PDUR, {}u, {}u, PduR_CanIfTxConfirmation, PduR_CanIfTriggerTransmit",
                route.destination_handle, signal.can_if_handle
            )));
            pd_tx.push(row(&format!("PDUR_UP_COM, {}u, PDUR_LO_LSDUR, {}u, LSduR_PduRTransmit, Com_TxConfirmation, Com_TriggerTransmit", route.source_handle, route.destination_handle)));
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
            "com_pdus, {}u, {}u, Rte_COMCbk, Rte_COMCbkRxTOut, PduR_ComTransmit",
            description.signals.len(),
            com.receive_group.handle
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
        config.push_str("#include \"PduR_CanTp.h\"\n#include \"LSduR_CanTp.h\"\n");
        receive.push(row(&format!(
            "{}u, {}u, 8u",
            tp.request_can_id, tp.request_can_if_handle
        )));
        transmit.push(row(&format!(
            "{}u, {}u, 8u",
            tp.response_can_id, tp.response_can_if_handle
        )));
        ls_rx.push(row(&format!(
            "{}u, {}u, CanTp_RxIndication",
            tp.request_can_if_handle, tp.receive_npdu
        )));
        ls_tx.push(row(&format!(
            "LSDUR_UP_CANTP, {}u, {}u, CanTp_TxConfirmation, NULL_PTR",
            tp.transmit_npdu, tp.response_can_if_handle
        )));
        pd_tx.push(row(&format!(
            // CanTp confirms through tp_routes, under its own N-SDU namespace.
            "PDUR_UP_DCM, {}u, PDUR_LO_CANTP, {}u, CanTp_Transmit, NULL_PTR, NULL_PTR",
            tx_route.source_handle, tp.transmit_nsdu
        )));
        config_data(
            &mut pd_configuration,
            "PduR",
            "static const PduR_TpRouteType tp_routes[]",
            &[row(&format!(
                "{}u, {}u, {}u, {}u, Dcm_StartOfReception, Dcm_CopyRxData, Dcm_TpRxIndication, Dcm_CopyTxData, Dcm_TpTxConfirmation",
                rx_route.source_handle,
                tp.dcm_receive,
                tx_route.destination_handle,
                tp.dcm_transmit
            ))],
        );
        let period = description.schedule.counter_tick_ms;
        let ticks = |value| periods(value, period, &description.schedule.counter);
        config_data(
            &mut config,
            "CanTp",
            "static const CanTp_ConfigType tp_config",
            &values(&format!(
                "{}u, {}u, {}u, {}u, {}u, {}u, {}u, {}u, {}u, {}u, {}u, {period}u, 0u, 0u, {}u, {}u",
                tp.receive_nsdu,
                tp.transmit_nsdu,
                tp.receive_npdu,
                tp.transmit_npdu,
                tp.buffer_bytes,
                ticks(tp.n_as_ms)?,
                ticks(tp.n_ar_ms)?,
                ticks(tp.n_br_ms)?,
                ticks(tp.n_bs_ms)?,
                ticks(tp.n_cr_ms)?,
                ticks(tp.n_cs_ms)?,
                rx_route.source_handle,
                tx_route.destination_handle
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
                (did.did, bridge.symbol.clone())
            }
            None => (0, "NULL_PTR".to_string()),
        };
        config.push_str("static void Ecu_DcmAuthenticationMode(uint8 state);\n\n");
        config_data(
            &mut config,
            "Dcm",
            "static const Dcm_ConfigType dcm_config",
            &values(&format!(
                "{}u, {}u, {}u, {did}u, {read}, {}u, {}u, {}u, {}u, {}u, {}u, Ecu_DcmAuthenticationMode, {}u",
                tp.dcm_receive,
                tp.dcm_transmit,
                mode.channel_handle,
                ticks(tp.p2_ms)?,
                tp.p2_ms,
                tp.p2_star_ms,
                ticks(tp.s3_ms)?,
                tp.buffer_bytes,
                tx_route.source_handle,
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
            "can_rx, {rx_count}u, can_tx, {tx_count}u, Ecu_HostBusSM_ControllerModeIndication, Ecu_HostBusSM_ControllerBusOff, {}u, {}u, {}u, {}u",
            can.can_if_controller_id, can.controller_id, can.receive_handle, can.transmit_handle
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
                &format!("{}_CFG_H", module.to_uppercase()),
                &format!("#include \"{module}.h\"\nextern const {ty} {module}_Config;"),
            ),
        )?;
        outputs.emit(ECU, format!("src/{module}_PBcfg.c"), source.as_str())?;
        config.push_str(&format!("#include \"{module}_Cfg.h\"\n"));
    }

    config_data(
        &mut config,
        "Ecu_HostBusSM",
        "static const Ecu_HostBusSM_ConfigType bus_config",
        &values(&format!(
            "{}u, {}u",
            mode.channel_handle, can.can_if_controller_id
        )),
    );
    config_data(
        &mut config,
        "ComM",
        "static const ComM_UserHandleType users[]",
        &mode
            .users
            .iter()
            .map(|user| Item::Value(format!("{}u", user.handle)))
            .collect::<Vec<_>>(),
    );
    config_data(
        &mut config,
        "ComM",
        "static const ComM_ConfigType comm_config",
        &values(&format!(
            "{}u, users, {}u, {}u, Ecu_HostBusSM_RequestComMode, Ecu_HostBusSM_GetCurrentComMode, {}",
            mode.channel_handle,
            mode.users.len(),
            periods(mode.minimum_full_ms, mode.period_ms, &mode.main_symbol)?,
            if transport.is_some() {
                "Dcm_ComM_NoComModeEntered, Dcm_ComM_SilentComModeEntered, Dcm_ComM_FullComModeEntered"
            } else {
                "NULL_PTR, NULL_PTR, NULL_PTR"
            }
        )),
    );
    config_data(
        &mut config,
        "BswM",
        "static const BswM_ConfigType bswm_config",
        &values(&format!("{}u, Ecu_BswMAction", mode.channel_handle)),
    );
    for (symbol, scope, body) in [
        (&com.receive_main.symbol, "Com", "Ecu_ComMainFunctionRx();"),
        (&com.transmit_main.symbol, "Com", "Ecu_ComMainFunctionTx();"),
        (&mode.main_symbol, "ComM", "(void)ComM_RunChannel();"),
    ] {
        let function =
            c_layout::short_function(&format!("RTE_{scope}_CODE void {symbol}(void)"), body);
        config.push_str(&format!("#define RTE_{scope}_START_SEC_CODE\n#include \"Rte_MemMap.h\"\n{function}\n#define RTE_{scope}_STOP_SEC_CODE\n#include \"Rte_MemMap.h\"\n"));
    }
    // SWS_CanTp_00229 timeouts reach the HostBatch receipt through the
    // standard runtime callout (DetReportRuntimeErrorCallout), not a CanTp hook.
    config_data(
        &mut config,
        "Det",
        "static const Det_ErrorHookType det_runtime_callouts[1]",
        &values("Ecu_DetRuntimeCallout"),
    );
    config_data(
        &mut config,
        "Det",
        "static const Det_ConfigType det_config",
        &values("NULL_PTR, 0u, det_runtime_callouts, 1u"),
    );
    config.push_str("#define RTE_Ecu_START_SEC_CODE\n#include \"Rte_MemMap.h\"\n");
    // ECUC_BswM_00843: parameters are fixed by the configured complete call.
    config.push_str("static RTE_Ecu_CODE Std_ReturnType Ecu_BswMAction(NetworkHandleType channel,\n                                                  ComM_ModeType requested)\n{\n");
    config.push_str(&format!(
        "    if (channel != {}u) {{\n        return E_NOT_OK;\n    }}\n    switch (requested) {{\n",
        mode.channel_handle
    ));
    for rule in &mode.rules {
        config.push_str(&format!(
            "        case {}u:\n            (void){};\n            break;\n",
            rule.mode, rule.callout
        ));
    }
    config.push_str("        default:\n            return E_NOT_OK;\n    }\n    return E_OK;\n}\n");
    let callout_signature = |end| {
        c_layout::signature(
            "DET_CALLOUT_CODE Std_ReturnType",
            "Ecu_DetRuntimeCallout",
            &[
                "uint16 ModuleId".into(),
                "uint8 InstanceId".into(),
                "uint8 ApiId".into(),
                "uint8 ErrorId".into(),
            ],
            end,
            true,
        )
    };
    outputs.emit(
        ECU,
        "include/Det_Externals.h",
        header(
            "DET_EXTERNALS_H",
            &format!("#include \"Std_Types.h\"\n#define DET_START_SEC_ECU_DETRUNTIMECALLOUT_CODE\n#include \"Det_MemMap.h\"\n/** Forward the selected CanTp timeout to the ECU receipt.\n * @param[in] ModuleId Reporting module, 0..65535.\n * @param[in] InstanceId Reporting instance, 0..255.\n * @param[in] ApiId Reporting API, 0..255.\n * @param[in] ErrorId Runtime error, 0..255.\n * @return E_OK; other reports have no receipt side effect.\n * Preconditions: Selected CanTp timeouts are reported by the ECU owner.\n * Context: Called by Det outside DET_STATE.\n * Reentrancy: Filtering is stateless; receipt publication uses the ECU owner.\n * Ownership: No parameter is retained.\n */\n{}\n#define DET_STOP_SEC_ECU_DETRUNTIMECALLOUT_CODE\n#include \"Det_MemMap.h\"", callout_signature(";")),
        ),
    )?;
    outputs.emit(
        ECU,
        "src/Det_Callouts.c",
        format!("/** @file Configured Det runtime notification for controlled ECU receipts. */\n#include \"Det_Externals.h\"\n#include \"Ecu_Target.h\"\n#define DET_START_SEC_ECU_DETRUNTIMECALLOUT_CODE\n#include \"Det_MemMap.h\"\n{}\n{{\n    (void)InstanceId;\n    if ((ModuleId == 35u) && (ApiId == 0x06u) && ((ErrorId == 0xc0u) || (ErrorId == 0xd0u))) {{\n        Ecu_TargetReportTransportTimeout();\n    }}\n    return E_OK;\n}}\n#define DET_STOP_SEC_ECU_DETRUNTIMECALLOUT_CODE\n#include \"Det_MemMap.h\"\n", callout_signature("")),
    )?;
    // Eight initialization stages; a test build can fail before each one, and
    // retirement undoes only the stages that began, newest first.
    let stage = |number: u8| {
        format!(
            "#ifdef ECU_TARGET_TESTS\n    if (Ecu_TargetTestFailStage({number}u) != 0) {{\n        return E_NOT_OK;\n    }}\n#endif\n    *reached = {number}u;\n"
        )
    };
    config.push_str("RTE_Ecu_CODE Std_ReturnType Ecu_StackBootstrap(uint8 *reached)\n{\n    Can_ControllerStateType actual;\n    if ((reached == NULL_PTR) || (Ecu_TargetCheckLifecycleContext(TRUE) != E_OK)) {\n        return E_NOT_OK;\n    }\n    *reached = 0u;\n");
    config.push_str(&stage(1));
    config.push_str("    SchM_Init(&Ecu_SchMConfig);\n    SchM_Start();\n    if (Ecu_SchMReady() == FALSE) {\n        return E_NOT_OK;\n    }\n");
    config.push_str(&stage(2));
    config.push_str("    Det_Init(&det_config);\n    Det_Start();\n");
    config.push_str(&stage(3));
    config.push_str("    Can_Init(&driver);\n");
    config.push_str(&stage(4));
    config.push_str("    CanIf_Init(&CanIf_Config);\n    Ecu_HostBusSM_Init(&bus_config);\n");
    config.push_str(&stage(5));
    config.push_str("    LSduR_Init(&LSduR_Config);\n    PduR_Init(&PduR_Config);\n");
    config.push_str(&stage(6));
    config.push_str("    Com_Init(&com_config);\n    Com_IpduGroupStart(com_config.receive_group, TRUE);\n    Com_EnableReceptionDM(com_config.receive_group);\n    ComM_Init(&comm_config);\n    BswM_Init(&bswm_config);\n");
    // Stage 7 exists only with a diagnostic transport; the numbering stays fixed.
    if transport.is_some() {
        config.push_str(&stage(7));
        config.push_str("    CanTp_Init(&tp_config);\n    Dcm_Init(&dcm_config);\n");
    }
    let first_user = mode
        .users
        .first()
        .ok_or_else(|| PlanDiagnostic::plan_premise(&mode.channel))?;
    config.push_str(&stage(8));
    config.push_str(&format!("    if (Rte_Start() != E_OK) {{\n        return E_NOT_OK;\n    }}\n    ComM_CommunicationAllowed({}u, TRUE);\n    if (ComM_RequestComMode({}u, COMM_FULL_COMMUNICATION) != E_OK) {{\n        return E_NOT_OK;\n    }}\n    {}();\n    Can_MainFunction_Mode();\n    if ((CanIf_GetControllerMode({}u, &actual) != E_OK) || (actual != CAN_CS_STARTED)) {{\n        return E_NOT_OK;\n    }}\n    SchM_StartTiming();\n    return Ecu_SchMTimingActive() ? E_OK : E_NOT_OK;\n}}\n", mode.channel_handle, first_user.handle, mode.main_symbol, can.can_if_controller_id));
    config.push_str("RTE_Ecu_CODE Std_ReturnType Ecu_StackRunCycle(EventMaskType events)\n{\n    if ((Ecu_TargetIsOwner() == 0) || (Ecu_SchMTimingActive() == FALSE)) {\n        return E_NOT_OK;\n    }\n");
    for entity in &description.schedule.entities {
        if entity.application {
            config.push_str(&format!(
                "    if ((events & {}u) != 0u) {{\n        {}();\n    }}\n",
                description.event_mask(&entity.os_event)?,
                entity.symbol
            ));
        } else {
            let symbol = &entity.symbol;
            config.push_str(&format!(
                "    if ((events & {}u) != 0u) {{\n        {symbol}();\n    }}\n",
                description.event_mask(&entity.os_event)?
            ));
            if entity.symbol == com.transmit_main.symbol {
                config.push_str(
                    "    if (Can_HostFlush() != ECU_OK) {\n        return E_NOT_OK;\n    }\n",
                );
            }
        }
    }
    config.push_str("    return (Can_HostFlush() == ECU_OK) ? E_OK : E_NOT_OK;\n}\nRTE_Ecu_CODE void Ecu_StackRetire(uint8 reached)\n{\n");
    // Det (2) and the routing tables (5) have no deinitialization service.
    let mut retired = vec![(8, "(void)Rte_Stop();\n".to_owned())];
    if transport.is_some() {
        retired.push((7, "CanTp_Shutdown();\n".to_owned()));
    }
    retired.push((6, "BswM_Deinit();\n        Com_DeInit();\n".to_owned()));
    retired.push((
        4,
        "Ecu_HostBusSM_DeInit();\n        CanIf_DeInit();\n".to_owned(),
    ));
    retired.push((3, format!("(void)Can_SetControllerMode({}u, CAN_CS_STOPPED);\n        Can_MainFunction_Mode();\n        Can_MainFunction_BusOff();\n        Can_DeInit();\n", can.controller_id)));
    retired.push((1, "SchM_Deinit();\n".to_owned()));
    for (number, body) in retired {
        config.push_str(&format!("    if (reached >= {number}u) {{\n#ifdef ECU_TARGET_TESTS\n        Ecu_TargetTestRetire({number}u);\n#endif\n        {body}    }}\n"));
    }
    config.push_str("}\n");
    config.push_str("RTE_Ecu_CODE int Ecu_StackFrameShape(uint32 id, uint8 dlc)\n{\n    if ((id > 0x7ffu) || (dlc > 8u)) {\n        return 0;\n    }\n");
    for signal in &description.signals {
        config.push_str(&format!(
            "    if (id == {}u) {{\n        return {};\n    }}\n",
            signal.can_id,
            if signal.receive { "dlc == 4u" } else { "0" }
        ));
    }
    if let Some(tp) = transport {
        config.push_str(&format!(
            "    if (id == {}u) {{\n        return dlc == 8u;\n    }}\n    if (id == {}u) {{\n        return 0;\n    }}\n",
            tp.request_can_id, tp.response_can_id
        ));
    }
    config.push_str("    return 1;\n}\n");
    config.push_str("#define RTE_Ecu_STOP_SEC_CODE\n#include \"Rte_MemMap.h\"\n");
    if description.diagnostic_transport.is_some() {
        let function = c_layout::short_function(
            "static RTE_Dcm_CODE void Ecu_DcmAuthenticationMode(uint8 state)",
            &format!("(void)SchM_Switch_Dcm_{authentication_group}(state);"),
        );
        config.push_str(&format!("#define RTE_Dcm_START_SEC_CODE\n#include \"Rte_MemMap.h\"\n{function}\n#define RTE_Dcm_STOP_SEC_CODE\n#include \"Rte_MemMap.h\"\n"));
    }
    outputs.emit(ECU, "src/Ecu_RuntimeConfig.c", config)?;
    let mode_scope = super::memmap::dcm_scope(composition);
    outputs.emit(
        ECU,
        "src/SchM_Main.c",
        lifecycle(communication.partition.id, mode_scope, authentication_group),
    )?;
    // Only assets delivered byte-for-byte are original sources; templates an
    // emitter extended are generated outputs. The packaging tool is wrapped at delivery.
    let mut original_sources = outputs.original_assets();
    original_sources.remove("runtime/ecu-tool.py");
    outputs.emit(ECU, super::outputs::INTEGRATION_PATH, format!("{:#}\n", serde_json::json!({"format": "autosar-ecu-integration-v2", "plan": description, "target": target.spec().id, "owner": "single source-derived AUTOSTART extended task", "originalBswSources": original_sources })))?;
    let verification = super::handoff::verification_files(plan).map_err(|_| failure())?;
    let readme = format!(
        "# ECU integration source project\n\nTarget: `{}`. Caller application sources are sealed under `app/`; the source-derived component, communication and OS contracts are in `meta/integration.json`. Original MIT notices of the fixed FreeRTOS kernel remain in `kernel/LICENSE.md`; Autsaro runtime, tool and template code is licensed under Apache-2.0 (see LICENSE and NOTICE). User-provided configurations and application code retain their own licensing.\n\n## Independent native build\n\n{}\nFrom this directory run `python tools/ecu-tool.py build --project . --output <new-empty-external-directory> --mode host-batch`. The production BEGIN/RX/COMMIT protocol uses the single OS owner and controlled logical ticks. Physical output flush precedes driver polling confirmation. No compiler, AUTOSAR archive or application algorithm is generated or redistributed.\n\n`CMakeLists.txt` builds the same sources with the same flags, kernel patches and libraries from `target.json` for IDEs and CI: run `cmake -S . -B <outside-build-directory> -G Ninja -DCMAKE_C_COMPILER=<gcc>` and `cmake --build <outside-build-directory>`. CMake neither checks the pinned toolchain identity and target sections nor runs `verify`; `tools/ecu-tool.py` does.\n\n## Layout\n\n`bsw/` holds the static BSW, `integration/` the host adapters and both mains, `os/` and `kernel/` the OS adaptation, kernel patches and fixed kernel. `generated/` holds every configuration-derived file and is rewritten on each generation; `generated/templates/` holds component entry-point skeletons that are not compiled. `app/<component>/` is the sealed snapshot of each live application source, `inputs/` the configuration snapshot and `meta/` the package metadata and seal. Edit applications and configuration in the live project, never in this package.\n{}",
        target.spec().id,
        crate::generator::delivery::native_build_prerequisites(target),
        if verification.is_empty() {
            ""
        } else {
            "\n## Reference behaviour verification\n\nThis configuration has one 32-bit CAN receive signal, one 32-bit CAN transmit signal and an application DID, so `meta/verification-inputs.json` records their configured endpoints. `python tools/ecu-tool.py verify --project . --build-directory <another-new-empty-external-directory>` builds HostBatch and checks the reference echo application: the transmit signal repeats the last received value, the DID returns it big-endian, one/two-DID reads, DID capacity refusal, N_Cr timeout/recovery and malformed admission. Other application behaviour fails this check by design; it does not establish full SC1, MCU, hard-real-time, ASIL or official certification.\n"
        }
    );
    outputs.emit_all("handoff-verification", verification)?;
    let paths = outputs.paths().map(str::to_owned).collect();
    let artifacts = super::artifacts::files(description, &paths)?;
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
fn lifecycle(partition: u16, mode_scope: &str, authentication_group: &str) -> String {
    let lifecycle = format!(
        "#include \"Rte_Main.h\"\n#include \"Ecu_Target.h\"\n#define RTE_Rte_START_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n#define SCHM_LIFECYCLE_UNALLOCATED 0U\n#define SCHM_LIFECYCLE_INITIALIZED 1U\n#define SCHM_LIFECYCLE_STARTED     2U\n#define SCHM_LIFECYCLE_TIMING      3U\n#define SCHM_LIFECYCLE_RETIRED     4U\nstatic uint8 schm_lifecycle RTE_Rte_VAR_CLEARED_UNSPECIFIED;\n#define RTE_Rte_STOP_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n#define RTE_Rte_START_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED\n#include \"Rte_MemMap.h\"\nconst SchM_ConfigType Ecu_SchMConfig RTE_Rte_CONFIG_DATA_PREBUILD_UNSPECIFIED = {{{partition}u}};\n#define RTE_Rte_STOP_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n#define RTE_Rte_START_SEC_CODE\n#include \"Rte_MemMap.h\"\nRTE_Rte_CODE void SchM_Init(const SchM_ConfigType *ConfigPtr)\n{{\n    if ((Ecu_TargetCheckLifecycleContext(TRUE) != E_OK) ||\n        (schm_lifecycle != SCHM_LIFECYCLE_UNALLOCATED) || (ConfigPtr == NULL_PTR) ||\n        (ConfigPtr->partition != {partition}u)) {{\n        return;\n    }}\n    schm_lifecycle = SCHM_LIFECYCLE_INITIALIZED;\n}}\nRTE_Rte_CODE void SchM_Start(void)\n{{\n    if ((Ecu_TargetCheckLifecycleContext(TRUE) == E_OK) &&\n        (schm_lifecycle == SCHM_LIFECYCLE_INITIALIZED)) {{\n        schm_lifecycle = SCHM_LIFECYCLE_STARTED;\n    }}\n}}\nRTE_Rte_CODE void SchM_StartTiming(void)\n{{\n    if ((Ecu_TargetCheckLifecycleContext(TRUE) == E_OK) &&\n        (schm_lifecycle == SCHM_LIFECYCLE_STARTED)) {{\n        schm_lifecycle = SCHM_LIFECYCLE_TIMING;\n    }}\n}}\nRTE_Rte_CODE void SchM_Deinit(void)\n{{\n    if ((Ecu_TargetCheckLifecycleContext(FALSE) == E_OK) &&\n        (schm_lifecycle != SCHM_LIFECYCLE_UNALLOCATED)) {{\n        schm_lifecycle = SCHM_LIFECYCLE_RETIRED;\n    }}\n}}\nRTE_Rte_CODE boolean Ecu_SchMReady(void)\n{{\n    return (schm_lifecycle == SCHM_LIFECYCLE_STARTED) || (schm_lifecycle == SCHM_LIFECYCLE_TIMING);\n}}\nRTE_Rte_CODE boolean Ecu_SchMTimingActive(void) {{ return schm_lifecycle == SCHM_LIFECYCLE_TIMING; }}\n#define RTE_Rte_STOP_SEC_CODE\n#include \"Rte_MemMap.h\"\n"
    );
    let mut modes = format!(
        "\n#include \"SchM_Dcm.h\"\n#include \"Can_HostLock.h\"\n#define RTE_{mode_scope}_START_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n"
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
            modes.push_str(&format!("{declaration}\n    {attribute}\n"));
        }
    }
    modes.push_str(&format!(
        "#define RTE_{mode_scope}_STOP_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n#define RTE_{mode_scope}_START_SEC_CODE\n#include \"Rte_MemMap.h\"\n"
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
        modes.push_str(&format!("{switch}\n{{\n    Can_HostAreaEnter(CAN_HOST_AREA_SCHM_MODE);\n    {variable} = mode;\n    Can_HostAreaExit(CAN_HOST_AREA_SCHM_MODE);\n    return SCHM_E_OK;\n}}\n{mode}\n{{\n    Rte_ModeType_{group} mode;\n    Can_HostAreaEnter(CAN_HOST_AREA_SCHM_MODE);\n    mode = {variable};\n    Can_HostAreaExit(CAN_HOST_AREA_SCHM_MODE);\n    return mode;\n}}\n"));
    }
    modes.push_str(&format!(
        "#define RTE_{mode_scope}_STOP_SEC_CODE\n#include \"Rte_MemMap.h\"\n"
    ));
    lifecycle + &modes
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
