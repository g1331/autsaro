//! ECU configuration and integration sources from the selected plan; applications remain caller owned.
use super::{PlanDiagnostic, ValidatedIntegrationPlan};
use crate::target::BuildTarget;

fn header(name: &str, body: &str) -> String {
    format!(
        "/** @file Generated from the selected source contract. */\n#ifndef {name}\n#define {name}\n{body}\n#endif\n"
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

/// Writes one constant in its module's pre-build configuration data section;
/// a multi-line `initializer` ends with a newline.
fn config_data(output: &mut String, scope: &str, declaration: &str, initializer: &str) {
    output.push_str(&format!("#define RTE_{scope}_START_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED\n#include \"Ecu_MemMap.h\"\n{declaration} RTE_{scope}_CONFIG_DATA_PREBUILD_UNSPECIFIED = {initializer};\n#define RTE_{scope}_STOP_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED\n#include \"Ecu_MemMap.h\"\n"));
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
    outputs.emit_all(
        "contracts",
        plan.contract_headers(composition, true)?
            .into_iter()
            .map(|(path, bytes)| match path.as_str() {
                "contract.json" => ("meta/contract.json".into(), bytes),
                _ => (path, bytes),
            }),
    )?;
    outputs.emit_all(super::outputs::RTE, super::rte::files(plan)?)?;
    outputs.emit_verbatim(super::outputs::RTE)?;
    outputs.emit_verbatim(super::outputs::MEMMAP)?;
    // The selected authentication group's mode type and SchM declarations
    // complete the trusted Dcm headers.
    for (path, addition) in [
        (
            "include/Rte_Dcm_Type.h",
            format!(
                "typedef uint8 Rte_ModeType_{authentication_group};\n#define RTE_MODE_{authentication_group}_DCM_DEAUTHENTICATED 0u\n#define RTE_MODE_{authentication_group}_DCM_AUTHENTICATED 1u\n#define RTE_TRANSITION_{authentication_group} 255u\n"
            ),
        ),
        (
            "include/SchM_Dcm.h",
            format!(
                "Std_ReturnType SchM_Switch_Dcm_{authentication_group}(Rte_ModeType_{authentication_group} mode);\nRte_ModeType_{authentication_group} SchM_Mode_Dcm_{authentication_group}(void);\n"
            ),
        ),
    ] {
        let text = super::outputs::render(
            outputs.template(super::outputs::DCM_MODES, path)?,
            path,
            &[super::outputs::Edit::BeforeGuardEnd(&addition)],
        )?;
        outputs.emit_rendered(super::outputs::DCM_MODES, path, text)?;
    }
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
            "#include \"Std_Types.h\"\ntypedef enum { ECU_TX_SYNCHRONOUS = 0, ECU_TX_QUEUED = 1 } EcuTxConfirmation;\ntypedef struct { EcuTxConfirmation tx_confirmation; } EcuPolicyConfig;\nextern const EcuPolicyConfig Ecu_Policy;",
        ),
    )?;
    let profile = format!(
        "#define CAN_HOST_QUEUED_COMPLETION 1\n#define CAN_ZERO_LENGTH_SUPPORTED 1\n#define CAN_RX_POLLING 1\n#define CAN_TX_POLLING 1\n#define CAN_BUSOFF_POLLING 1\n#define CAN_CONTROLLER_ID {}u\n#define CAN_CANIF_CONTROLLER_ID {}u\n#define CAN_TX_HOH {}u\n#define CAN_RX_HOH {}u",
        can.controller_id, can.can_if_controller_id, can.transmit_handle, can.receive_handle
    );
    outputs.emit(
        ECU,
        "include/Ecu_ProfileConfig.h",
        header("ECU_PROFILE_CONFIG_H", &profile),
    )?;
    let integer = |kind: super::CommunicationIntegerType| match kind {
        super::CommunicationIntegerType::Uint8 => "uint8",
        super::CommunicationIntegerType::Uint16 => "uint16",
        super::CommunicationIntegerType::Uint32 => "uint32",
    };
    outputs.emit(ECU, "include/ComStack_Cfg.h", format!("/** @file EcuC-selected communication widths. */\n#ifndef COMSTACK_CFG_H\n#define COMSTACK_CFG_H\n#include \"Platform_Types.h\"\n#include \"Ecu_ProfileConfig.h\"\ntypedef {} PduIdType;\ntypedef {} PduLengthType;\n#endif\n", integer(communication.pdu_id_type), integer(communication.pdu_length_type)))?;
    outputs.emit(
        ECU,
        "src/Ecu_Execution.h",
        header(
            "ECU_EXECUTION_H",
            "#include \"Ecu_Config.h\"\n#include \"Can_Host.h\"\nEcuStatus Ecu_ExecutionTransmit(CanTxSink sink, PduIdType handle, uint32_t id, uint8_t dlc, const uint8_t data[8]);\nuint64_t Ecu_ExecutionNow(void);",
        ),
    )?;
    outputs.emit(ECU, "src/Ecu_Execution.c", "#include \"Ecu_Execution.h\"\n#include \"Ecu_Target.h\"\nEcuStatus Ecu_ExecutionTransmit(CanTxSink sink, PduIdType handle, uint32_t id, uint8_t dlc, const uint8_t data[8]) {\n    (void)sink;\n    return Ecu_TargetEnqueueTransmit(handle, id, dlc, data);\n}\nuint64_t Ecu_ExecutionNow(void) { return Ecu_TargetNow(); }\n")?;
    for (module, area) in [("CanIf", "CANIF"), ("ComM", "COMM"), ("CanTp", "CANTP")] {
        let main = match module {
            "CanTp" => "\nvoid CanTp_MainFunction(void);".to_owned(),
            "ComM" => format!("\nvoid {}(void);", mode.main_symbol),
            _ => String::new(),
        };
        outputs.emit(
            ECU,
            format!("include/SchM_{module}.h"),
            header(
                &format!("SCHM_{}_H", module.to_uppercase()),
                &format!(
                    "void SchM_Enter_{module}_{area}_STATE(void);\nvoid SchM_Exit_{module}_{area}_STATE(void);{main}"
                ),
            ),
        )?;
    }
    outputs.emit(
        ECU,
        "include/SchM_Com.h",
        header(
            "SCHM_COM_H",
            &format!(
                "void {}(void);\nvoid {}(void);",
                com.receive_main.symbol, com.transmit_main.symbol
            ),
        ),
    )?;
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
                "#include \"Ecu_ProfileConfig.h\"\n#include \"Os_Target.h\"\n#define ECU_TARGET_TASK 0u\n#define ECU_MAX_PDU_PAYLOAD {}u\n#define ECU_TARGET_DIAGNOSTIC_RX_CAN_ID {}u\n#define ECU_TARGET_EVENT_WORK {}u\n#define ECU_TARGET_EVENT_APP {}u\n#define ECU_TARGET_EVENT_IO {}u\nextern const Os_TargetConfig Ecu_OsConfig;\n/* Records in *reached each initialization stage (1 through 8) as it begins. */\nStd_ReturnType Ecu_StackBootstrap(uint8 *reached);\nStd_ReturnType Ecu_StackRunCycle(EventMaskType events);\n/* Retires the stages that began, newest first. */\nvoid Ecu_StackRetire(uint8 reached);\nint Ecu_StackFrameShape(uint32 id, uint8 dlc);\n#ifdef ECU_TARGET_TESTS\nint Ecu_TargetTestFailStage(unsigned stage);\nvoid Ecu_TargetTestRetire(unsigned stage);\nvoid Ecu_TargetTestShutdown(StatusType reason);\n#endif",
                description
                    .diagnostic_transport
                    .as_ref()
                    .map_or(4, |transport| transport.buffer_bytes),
                description
                    .diagnostic_transport
                    .as_ref()
                    .map_or(u32::MAX, |transport| transport.request_can_id),
                description.event_mask(&work.os_event)?,
                description.event_mask(&app.os_event)?,
                io.mask,
            ),
        ),
    )?;
    let mut config = String::from(
        "/** @file Source-derived immutable communication configurations and owner dispatch. */\n#define RTE_CORE\n#include \"Ecu_TargetConfig.h\"\n#include \"Ecu_Target.h\"\n#include \"Ecu_Config.h\"\n#include \"Can_Host.h\"\n#include \"CanIf.h\"\n#include \"Det.h\"\n#include \"CanTp.h\"\n#include \"Com.h\"\n#include \"Com_Internal.h\"\n#include \"ComM.h\"\n#include \"ComM_BswM.h\"\n#include \"ComM_Internal.h\"\n#include \"BswM.h\"\n#include \"BswM_ComM.h\"\n#include \"Ecu_HostBusSM.h\"\n#include \"Dcm.h\"\n#include \"Dcm_ComM.h\"\n#include \"PduR.h\"\n#include \"PduR_Com.h\"\n#include \"PduR_CanIf.h\"\n#include \"LSduR.h\"\n#include \"LSduR_PduR.h\"\n#include \"Rte_Main.h\"\n#include \"Rte_Com.h\"\n#include \"SchM_Can.h\"\n#include \"SchM_CanTp.h\"\n#include \"SchM_Com.h\"\n#include \"SchM_ComM.h\"\n#include \"SchM_Dcm.h\"\n",
    );
    for component in &composition.components {
        config.push_str(&format!(
            "#include \"{}\"\n",
            component.header.trim_start_matches("include/")
        ));
    }
    config.push_str("static Std_ReturnType Ecu_BswMAction(NetworkHandleType channel, ComM_ModeType requested);\nstatic Std_ReturnType Ecu_DetRuntimeCallout(uint16_t ModuleId, uint8_t InstanceId, uint8_t ApiId, uint8_t ErrorId);\nstatic void Ecu_ComModeChanged(NetworkHandleType channel, ComM_ModeType mode);\n");
    config_data(
        &mut config,
        "Ecu",
        "const EcuPolicyConfig Ecu_Policy",
        "{ECU_TX_QUEUED}",
    );
    config_data(
        &mut config,
        "Can",
        "static const Can_ConfigType driver",
        "{NULL_PTR}",
    );
    let mut receive = String::new();
    let mut transmit = String::new();
    let routing_headers = "#include \"Com.h\"\n#include \"Dcm.h\"\n#include \"CanTp.h\"\n#include \"PduR.h\"\n#include \"PduR_Com.h\"\n#include \"PduR_CanIf.h\"\n#include \"LSduR.h\"\n#include \"LSduR_PduR.h\"\n";
    let mut pd_configuration = format!(
        "/** @file Source-derived selected PduR configuration. */\n#include \"PduR_Cfg.h\"\n{routing_headers}"
    );
    let mut ls_configuration = format!(
        "/** @file Source-derived selected LSduR configuration. */\n#include \"LSduR_Cfg.h\"\n{routing_headers}"
    );
    let mut can_if_configuration = "/** @file Source-derived selected CanIf configuration. */\n#include \"CanIf_Cfg.h\"\n#include \"Ecu_HostBusSM.h\"\n".to_owned();
    let mut ls_rx = String::new();
    let mut ls_tx = String::new();
    let mut pd_rx = String::new();
    let mut pd_tx = String::new();
    let mut pdus = String::new();
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
        pdus.push_str(&format!(
            "    {{{pdu}u, {signal_id}u, {}, {initial}u, {}u, {}u}},\n",
            if signal.receive { "TRUE" } else { "FALSE" },
            reception_timeout,
            reception.map_or(0, |rx| u32::from(rx.callback_handle))
        ));
        if signal.receive {
            receive.push_str(&format!(
                "    {{{}u, {}u, 4u}},\n",
                signal.can_id, signal.can_if_handle
            ));
            ls_rx.push_str(&format!(
                "    {{{}u, {}u, PduR_CanIfRxIndication}},\n",
                signal.can_if_handle, route.source_handle
            ));
            pd_rx.push_str(&format!(
                "    {{{}u, {pdu}u, Com_RxIndication}},\n",
                route.source_handle
            ));
        } else {
            transmit.push_str(&format!(
                "    {{{}u, {}u, 4u}},\n",
                signal.can_id, signal.can_if_handle
            ));
            ls_tx.push_str(&format!("    {{LSDUR_UP_PDUR, {}u, {}u, PduR_CanIfTxConfirmation, PduR_CanIfTriggerTransmit}},\n", route.destination_handle, signal.can_if_handle));
            pd_tx.push_str(&format!("    {{PDUR_UP_COM, {}u, {}u, LSduR_PduRTransmit, Com_TxConfirmation, Com_TriggerTransmit}},\n", route.source_handle, route.destination_handle));
        }
    }
    config_data(
        &mut config,
        "Com",
        "static const Com_PduConfigType com_pdus[]",
        &format!("{{\n{pdus}}}"),
    );
    config_data(
        &mut config,
        "Com",
        "static const Com_ConfigType com_config",
        &format!(
            "{{com_pdus, {}u, {}u, Rte_COMCbk, Rte_COMCbkRxTOut, PduR_ComTransmit}}",
            description.signals.len(),
            com.receive_group.handle
        ),
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
        receive.push_str(&format!(
            "    {{{}u, {}u, 8u}},\n",
            tp.request_can_id, tp.request_can_if_handle
        ));
        transmit.push_str(&format!(
            "    {{{}u, {}u, 8u}},\n",
            tp.response_can_id, tp.response_can_if_handle
        ));
        ls_rx.push_str(&format!(
            "    {{{}u, {}u, CanTp_RxIndication}},\n",
            tp.request_can_if_handle, tp.receive_npdu
        ));
        ls_tx.push_str(&format!(
            "    {{LSDUR_UP_CANTP, {}u, {}u, CanTp_TxConfirmation, NULL_PTR}},\n",
            tp.transmit_npdu, tp.response_can_if_handle
        ));
        pd_tx.push_str(&format!(
            "    {{PDUR_UP_DCM, {}u, {}u, CanTp_Transmit, Dcm_TpTxConfirmation, NULL_PTR}},\n",
            tx_route.source_handle, tp.transmit_nsdu
        ));
        config_data(
            &mut pd_configuration,
            "PduR",
            "static const PduR_TpRouteType tp_routes[]",
            &format!(
                "{{\n    {{{}u, {}u, {}u, {}u, Dcm_StartOfReception, Dcm_CopyRxData, Dcm_TpRxIndication, Dcm_CopyTxData, Dcm_TpTxConfirmation}}\n}}",
                rx_route.source_handle,
                tp.dcm_receive,
                tx_route.destination_handle,
                tp.dcm_transmit
            ),
        );
        let period = description.schedule.counter_tick_ms;
        let ticks = |value| periods(value, period, &description.schedule.counter);
        config_data(
            &mut config,
            "CanTp",
            "static const CanTp_ConfigType tp_config",
            &format!(
                "{{{}u, {}u, {}u, {}u, {}u, {}u, {}u, {}u, {}u, {}u, {period}u, 0u, 0u, {}u, {}u}}",
                tp.receive_nsdu,
                tp.transmit_nsdu,
                tp.receive_npdu,
                tp.transmit_npdu,
                tp.buffer_bytes,
                ticks(tp.n_as_ms)?,
                ticks(tp.n_ar_ms)?,
                ticks(tp.n_bs_ms)?,
                ticks(tp.n_cr_ms)?,
                ticks(tp.n_cs_ms)?,
                rx_route.source_handle,
                tx_route.destination_handle
            ),
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
            &format!(
                "{{{}u, {}u, {}u, {did}u, {read}, {}u, {}u, {}u, {}u, {}u, {}u, Ecu_DcmAuthenticationMode, {}u}}",
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
            ),
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
            &format!("{{\n{entries}}}"),
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
        &format!(
            "{{can_rx, {rx_count}u, can_tx, {tx_count}u, Ecu_HostBusSM_ControllerModeIndication, Ecu_HostBusSM_ControllerBusOff, {}u, {}u, {}u, {}u}}",
            can.can_if_controller_id, can.controller_id, can.receive_handle, can.transmit_handle
        ),
    );
    config_data(
        &mut ls_configuration,
        "LSduR",
        "const LSduR_PBConfigType LSduR_Config",
        &format!("{{0u, ls_rx, {rx_count}u, ls_tx, {tx_count}u}}"),
    );
    config_data(
        &mut pd_configuration,
        "PduR",
        "const PduR_PBConfigType PduR_Config",
        &format!(
            "{{0u, pd_rx, {}u, pd_tx, {tx_count}u, {}, {}u}}",
            rx_signals,
            if transport.is_some() {
                "tp_routes"
            } else {
                "NULL_PTR"
            },
            usize::from(transport.is_some())
        ),
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
        &format!(
            "{{{}u, {}u}}",
            mode.channel_handle, can.can_if_controller_id
        ),
    );
    config_data(
        &mut config,
        "ComM",
        "static const ComM_UserHandleType users[]",
        &format!(
            "{{{}}}",
            mode.users
                .iter()
                .map(|user| format!("{}u", user.handle))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    config_data(
        &mut config,
        "ComM",
        "static const ComM_ConfigType comm_config",
        &format!(
            "{{{}u, users, {}u, {}u, Ecu_HostBusSM_RequestComMode, Ecu_HostBusSM_GetCurrentComMode, Ecu_ComModeChanged, {}u}}",
            mode.channel_handle,
            mode.users.len(),
            periods(mode.minimum_full_ms, mode.period_ms, &mode.main_symbol)?,
            mode.ecu_group_classification
        ),
    );
    config_data(
        &mut config,
        "BswM",
        "static const BswM_ConfigType bswm_config",
        &format!(
            "{{{}u, {}u, Ecu_BswMAction}}",
            mode.channel_handle, mode.initial_mode
        ),
    );
    for (symbol, scope, body) in [
        (&com.receive_main.symbol, "Com", "Ecu_ComMainFunctionRx();"),
        (&com.transmit_main.symbol, "Com", "Ecu_ComMainFunctionTx();"),
        (&mode.main_symbol, "ComM", "(void)ComM_RunChannel();"),
    ] {
        config.push_str(&format!("#define RTE_{scope}_START_SEC_CODE\n#include \"Ecu_MemMap.h\"\nRTE_{scope}_CODE void {symbol}(void) {{ {body} }}\n#define RTE_{scope}_STOP_SEC_CODE\n#include \"Ecu_MemMap.h\"\n"));
    }
    // SWS_CanTp_00229 timeouts reach the HostBatch receipt through the
    // standard runtime callout (DetReportRuntimeErrorCallout), not a CanTp hook.
    config_data(
        &mut config,
        "Det",
        "static const Det_ErrorHookType det_runtime_callouts[1]",
        "{Ecu_DetRuntimeCallout}",
    );
    config_data(
        &mut config,
        "Det",
        "static const Det_ConfigType det_config",
        "{NULL_PTR, 0u, det_runtime_callouts, 1u}",
    );
    config.push_str("#define RTE_Ecu_START_SEC_CODE\n#include \"Ecu_MemMap.h\"\n");
    config.push_str("static RTE_Ecu_CODE void Ecu_ComModeChanged(NetworkHandleType channel, ComM_ModeType current) {\n    BswM_ComM_CurrentMode(channel, current);\n");
    if transport.is_some() {
        config.push_str("    if (current == COMM_FULL_COMMUNICATION) { Dcm_ComM_FullComModeEntered(channel); }\n    else if (current == COMM_SILENT_COMMUNICATION) { Dcm_ComM_SilentComModeEntered(channel); }\n    else { Dcm_ComM_NoComModeEntered(channel); }\n");
    }
    config.push_str("}\n");
    // ECUC_BswM_00843: parameters are fixed by the configured complete call.
    config.push_str("static RTE_Ecu_CODE Std_ReturnType Ecu_BswMAction(NetworkHandleType channel, ComM_ModeType requested) {\n");
    config.push_str(&format!(
        "    if (channel != {}u) {{ return E_NOT_OK; }}\n    switch (requested) {{\n",
        mode.channel_handle
    ));
    for rule in &mode.rules {
        config.push_str(&format!(
            "    case {}u: (void){}; break;\n",
            rule.mode, rule.callout
        ));
    }
    config.push_str("    default: return E_NOT_OK;\n    }\n    return E_OK;\n}\n");
    config.push_str("static RTE_Ecu_CODE Std_ReturnType Ecu_DetRuntimeCallout(uint16_t ModuleId, uint8_t InstanceId, uint8_t ApiId, uint8_t ErrorId) {\n    (void)InstanceId;\n    if ((ModuleId == 35u) && (ApiId == 0x06u) && ((ErrorId == 0xc0u) || (ErrorId == 0xd0u))) {\n        Ecu_TargetReportTransportTimeout();\n    }\n    return E_OK;\n}\n");
    // Eight initialization stages; a test build can fail before each one, and
    // retirement undoes only the stages that began, newest first.
    let stage = |number: u8| {
        format!(
            "#ifdef ECU_TARGET_TESTS\n    if (Ecu_TargetTestFailStage({number}u) != 0) {{ return E_NOT_OK; }}\n#endif\n    *reached = {number}u;\n"
        )
    };
    config.push_str("RTE_Ecu_CODE Std_ReturnType Ecu_StackBootstrap(uint8 *reached) {\n    Can_ControllerStateType actual;\n    if ((reached == NULL_PTR) || (Ecu_TargetCheckLifecycleContext(TRUE) != E_OK)) { return E_NOT_OK; }\n    *reached = 0u;\n");
    config.push_str(&stage(1));
    config.push_str("    SchM_Init(&Ecu_SchMConfig);\n    SchM_Start();\n    if (Ecu_SchMReady() == FALSE) { return E_NOT_OK; }\n");
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
    config.push_str(&format!("    if (Rte_Start() != E_OK) {{ return E_NOT_OK; }}\n    ComM_CommunicationAllowed({}u, TRUE);\n    if (ComM_RequestComMode({}u, COMM_FULL_COMMUNICATION) != E_OK) {{ return E_NOT_OK; }}\n    {}();\n    Can_MainFunction_Mode();\n    if ((CanIf_GetControllerMode({}u, &actual) != E_OK) || (actual != CAN_CS_STARTED)) {{ return E_NOT_OK; }}\n    SchM_StartTiming();\n    return Ecu_SchMTimingActive() ? E_OK : E_NOT_OK;\n}}\n", mode.channel_handle, first_user.handle, mode.main_symbol, can.can_if_controller_id));
    config.push_str("RTE_Ecu_CODE Std_ReturnType Ecu_StackRunCycle(EventMaskType events) {\n    if ((Ecu_TargetIsOwner() == 0) || (Ecu_SchMTimingActive() == FALSE)) { return E_NOT_OK; }\n");
    for entity in &description.schedule.entities {
        if entity.application {
            config.push_str(&format!(
                "    if ((events & {}u) != 0u) {{ {}(); }}\n",
                description.event_mask(&entity.os_event)?,
                entity.symbol
            ));
        } else {
            let symbol = &entity.symbol;
            config.push_str(&format!(
                "    if ((events & {}u) != 0u) {{ {symbol}(); }}\n",
                description.event_mask(&entity.os_event)?
            ));
            if entity.symbol == com.transmit_main.symbol {
                config.push_str("    if (Can_HostFlush() != ECU_OK) { return E_NOT_OK; }\n");
            }
        }
    }
    config.push_str("    return (Can_HostFlush() == ECU_OK) ? E_OK : E_NOT_OK;\n}\nRTE_Ecu_CODE void Ecu_StackRetire(uint8 reached) {\n");
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
    config.push_str("RTE_Ecu_CODE int Ecu_StackFrameShape(uint32 id, uint8 dlc) {\n    if ((id > 0x7ffu) || (dlc > 8u)) { return 0; }\n");
    for signal in &description.signals {
        config.push_str(&format!(
            "    if (id == {}u) {{ return {}; }}\n",
            signal.can_id,
            if signal.receive { "dlc == 4u" } else { "0" }
        ));
    }
    if let Some(tp) = transport {
        config.push_str(&format!(
            "    if (id == {}u) {{ return dlc == 8u; }}\n    if (id == {}u) {{ return 0; }}\n",
            tp.request_can_id, tp.response_can_id
        ));
    }
    config.push_str("    return 1;\n}\n");
    config.push_str("#define RTE_Ecu_STOP_SEC_CODE\n#include \"Ecu_MemMap.h\"\n");
    if description.diagnostic_transport.is_some() {
        config.push_str(&format!("#define RTE_Dcm_START_SEC_CODE\n#include \"Ecu_MemMap.h\"\nstatic RTE_Dcm_CODE void Ecu_DcmAuthenticationMode(uint8 state) {{ (void)SchM_Switch_Dcm_{authentication_group}(state); }}\n#define RTE_Dcm_STOP_SEC_CODE\n#include \"Ecu_MemMap.h\"\n"));
    }
    outputs.emit(ECU, "src/Ecu_RuntimeConfig.c", config)?;
    let config_scopes: Vec<_> = [
        "Can",
        "CanIf",
        "Com",
        "CanTp",
        "Dcm",
        "PduR",
        "LSduR",
        "ComM",
        "BswM",
        "Det",
        "Ecu_HostBusSM",
        "Ecu",
    ]
    .into_iter()
    .flat_map(|scope| {
        [
            (scope.into(), "CONFIG_DATA_PREBUILD_UNSPECIFIED"),
            (scope.into(), "CODE"),
        ]
    })
    .collect();
    outputs.emit(
        ECU,
        "include/Ecu_MemMap.h",
        super::rte::memory_map(&config_scopes, None),
    )?;
    let mode_scope = composition
        .components
        .iter()
        .find(|component| component.diagnostic_session_port.is_some())
        .map(|component| super::short_name(&component.component))
        .unwrap_or("Dcm");
    outputs.emit(
        ECU,
        "src/SchM.c",
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
        "#include \"Rte_Main.h\"\n#include \"Ecu_Target.h\"\n#define RTE_Rte_START_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\nstatic boolean initialized RTE_VAR_CLEARED;\nstatic boolean started RTE_VAR_CLEARED;\nstatic boolean timing RTE_VAR_CLEARED;\nstatic boolean allocated RTE_VAR_CLEARED;\n#define RTE_Rte_STOP_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n#define RTE_Rte_START_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED\n#include \"Rte_MemMap.h\"\nconst SchM_ConfigType Ecu_SchMConfig RTE_CONFIG_DATA_PREBUILD = {{{partition}u}};\n#define RTE_Rte_STOP_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n#define RTE_Rte_START_SEC_CODE\n#include \"Rte_MemMap.h\"\nRTE_CODE void SchM_Init(const SchM_ConfigType *config) {{\n    if ((Ecu_TargetCheckLifecycleContext(TRUE) != E_OK) || (allocated == TRUE) || (config == NULL_PTR) || (config->partition != {partition}u)) {{ return; }}\n    allocated = TRUE;\n    initialized = TRUE;\n}}\nRTE_CODE void SchM_Start(void) {{\n    if ((Ecu_TargetCheckLifecycleContext(TRUE) == E_OK) && (initialized == TRUE)) {{ started = TRUE; }}\n}}\nRTE_CODE void SchM_StartTiming(void) {{\n    if ((Ecu_TargetCheckLifecycleContext(TRUE) == E_OK) && (started == TRUE)) {{ timing = TRUE; }}\n}}\nRTE_CODE void SchM_Deinit(void) {{\n    if (Ecu_TargetCheckLifecycleContext(FALSE) == E_OK) {{ timing = FALSE; started = FALSE; initialized = FALSE; }}\n}}\nRTE_CODE boolean Ecu_SchMReady(void) {{ return started; }}\nRTE_CODE boolean Ecu_SchMTimingActive(void) {{ return timing; }}\n#define RTE_Rte_STOP_SEC_CODE\n#include \"Rte_MemMap.h\"\n"
    );
    let modes = format!(
        "\n#include \"SchM_Dcm.h\"\n#include \"SchM_CanIf.h\"\n#define RTE_{mode_scope}_START_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\nstatic Rte_ModeType_DcmDiagnosticSessionControl diagnostic_session_mode RTE_VAR_CLEARED;\nstatic Rte_ModeType_{authentication_group} authentication_mode RTE_VAR_CLEARED;\n#define RTE_{mode_scope}_STOP_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n#define RTE_{mode_scope}_START_SEC_CODE\n#include \"Rte_MemMap.h\"\nRTE_CODE Std_ReturnType SchM_Switch_Dcm_{authentication_group}(Rte_ModeType_{authentication_group} mode) {{\n    if (Ecu_TargetIsOwner() == 0) {{ return SCHM_E_LIMIT; }}\n    SchM_Enter_CanIf_CANIF_STATE();\n    authentication_mode = mode;\n    SchM_Exit_CanIf_CANIF_STATE();\n    return SCHM_E_OK;\n}}\nRTE_CODE Rte_ModeType_{authentication_group} SchM_Mode_Dcm_{authentication_group}(void) {{\n    Rte_ModeType_{authentication_group} mode;\n    if (Ecu_TargetIsOwner() == 0) {{ return RTE_TRANSITION_{authentication_group}; }}\n    SchM_Enter_CanIf_CANIF_STATE();\n    mode = authentication_mode;\n    SchM_Exit_CanIf_CANIF_STATE();\n    return mode;\n}}\nRTE_CODE Std_ReturnType SchM_Switch_Dcm_DcmDiagnosticSessionControl(Rte_ModeType_DcmDiagnosticSessionControl mode) {{\n    if (Ecu_TargetIsOwner() == 0) {{ return SCHM_E_LIMIT; }}\n    SchM_Enter_CanIf_CANIF_STATE();\n    diagnostic_session_mode = mode;\n    SchM_Exit_CanIf_CANIF_STATE();\n    return SCHM_E_OK;\n}}\nRTE_CODE Rte_ModeType_DcmDiagnosticSessionControl SchM_Mode_Dcm_DcmDiagnosticSessionControl(void) {{\n    Rte_ModeType_DcmDiagnosticSessionControl mode;\n    if (Ecu_TargetIsOwner() == 0) {{ return RTE_TRANSITION_DcmDiagnosticSessionControl; }}\n    SchM_Enter_CanIf_CANIF_STATE();\n    mode = diagnostic_session_mode;\n    SchM_Exit_CanIf_CANIF_STATE();\n    return mode;\n}}\n#define RTE_{mode_scope}_STOP_SEC_CODE\n#include \"Rte_MemMap.h\"\n"
    );
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
