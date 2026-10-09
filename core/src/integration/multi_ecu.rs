//! Complete selected multi-component ECU producer; applications remain caller owned.
use super::{PlanDiagnostic, ValidatedIntegrationPlan};
use crate::target::BuildTarget;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Write;

fn put(files: &mut BTreeMap<String, Cow<'_, [u8]>>, path: &str, text: String) {
    files.insert(path.into(), Cow::Owned(text.into_bytes()));
}
fn header(name: &str, body: &str) -> String {
    format!(
        "/** @file Generated from the selected source contract. */\n#ifndef {name}\n#define {name}\n{body}\n#endif\n"
    )
}
fn failure() -> Vec<PlanDiagnostic> {
    vec![PlanDiagnostic::dependency(
        "ECU_SOURCE_CLOSURE",
        crate::product_message!("backend.integration.multi.consumer_unsupported"),
        crate::product_message!("backend.integration.multi.repair_contract"),
    )]
}

// Every declaration below is emitted by this producer. Apply the chosen
// pre-build configuration class to complete declarations, including arrays;
// no caller source is parsed or rewritten.
fn mapped_configuration(source: &str) -> String {
    let mut mapped = String::new();
    let mut active: Option<&str> = None;
    for line in source.lines() {
        let declaration = line
            .strip_prefix("static const ")
            .or_else(|| line.strip_prefix("const "));
        if let Some(declaration) = declaration {
            let ty = declaration.split_whitespace().next().unwrap();
            let scope = ty.split('_').next().unwrap();
            let scope = if scope == "EcuPolicyConfig" {
                "Ecu"
            } else if scope == "Ecu" {
                "Ecu_HostBusSM"
            } else {
                scope
            };
            assert!(active.is_none());
            writeln!(mapped, "#define RTE_{scope}_START_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED\n#include \"Ecu_MemMap.h\"").unwrap();
            let (left, right) = line.split_once(" = ").unwrap();
            writeln!(
                mapped,
                "{left} RTE_{scope}_CONFIG_DATA_PREBUILD_UNSPECIFIED = {right}"
            )
            .unwrap();
            active = Some(scope);
        } else {
            writeln!(mapped, "{line}").unwrap();
        }
        if line.ends_with(';') {
            if let Some(scope) = active.take() {
                writeln!(mapped, "#define RTE_{scope}_STOP_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED\n#include \"Ecu_MemMap.h\"").unwrap();
            }
        }
    }
    assert!(active.is_none());
    mapped
}

pub(super) fn files(
    plan: &ValidatedIntegrationPlan,
    target: BuildTarget,
) -> Result<BTreeMap<String, Cow<'static, [u8]>>, Vec<PlanDiagnostic>> {
    let description = plan.description();
    let multi = description.multi.as_ref().ok_or_else(failure)?;
    let com = description.com_runtime.as_ref().ok_or_else(failure)?;
    let authentication = multi
        .dcm_modes
        .iter()
        .find(|definition| definition.group.starts_with("DcmAuthenticationState_"))
        .unwrap();
    let authentication_group = authentication.group.as_ref();
    let communication = description.communication_runtime.as_ref().unwrap();
    let can = &communication.can;
    let mode = description.mode_runtime.as_ref().unwrap();
    let mut files = BTreeMap::new();
    for (path, asset) in super::ecu::source_assets(target, true).map_err(|_| failure())? {
        files.insert(path, Cow::Borrowed(asset.bytes));
    }
    for (path, bytes) in plan.rte_runtime_files()?.into_files() {
        if path.starts_with("include/") || path == "src/Rte.c" || path == "contract.json" {
            files.insert(path, Cow::Owned(bytes));
        }
    }
    // Target mapping retains source-scoped AUTOSAR keywords while placing code
    // in the delivered target's real executable RTE section.
    let mapping = files.get_mut("include/Rte_MemMap.h").unwrap();
    let mut text = String::from_utf8(mapping.to_vec()).unwrap();
    for scope in std::iter::once("Rte").chain(
        multi
            .components
            .iter()
            .map(|component| component.component.rsplit('/').next().unwrap()),
    ) {
        text = text.replace(&format!(".text.rte.{scope}.CODE"), ".rte_code");
    }
    if !multi
        .components
        .iter()
        .any(|component| component.diagnostic_session_port.is_some())
    {
        text = text.replace(".text.rte.Dcm.CODE", ".rte_code");
    }
    text = text.replace(
        &format!(".text.rte.{}.CALLOUT_CODE", communication.partition.name),
        ".rte_code",
    );
    *mapping = Cow::Owned(text.into_bytes());
    let types = files.get_mut("include/Rte_Dcm_Type.h").unwrap();
    let addition = format!(
        "typedef uint8 Rte_ModeType_{authentication_group};\n#define RTE_MODE_{authentication_group}_DCM_DEAUTHENTICATED 0u\n#define RTE_MODE_{authentication_group}_DCM_AUTHENTICATED 1u\n#define RTE_TRANSITION_{authentication_group} 255u\n"
    );
    *types = Cow::Owned(
        String::from_utf8(types.to_vec())
            .unwrap()
            .replace("#endif", &(addition + "#endif"))
            .into_bytes(),
    );
    let scheduler = files.get_mut("include/SchM_Dcm.h").unwrap();
    let declarations = format!(
        "Std_ReturnType SchM_Switch_Dcm_{authentication_group}(Rte_ModeType_{authentication_group} mode);\nRte_ModeType_{authentication_group} SchM_Mode_Dcm_{authentication_group}(void);\n"
    );
    *scheduler = Cow::Owned(
        String::from_utf8(scheduler.to_vec())
            .unwrap()
            .replace("#endif", &(declarations + "#endif"))
            .into_bytes(),
    );
    let os = super::ecu::configure_os(description, &mut files)?;
    let os_service = files.get_mut("src/Rte_OsService.c").unwrap();
    *os_service = Cow::Owned(
        String::from_utf8(os_service.to_vec())
            .unwrap()
            .replace("RTE_START_SEC_CODE", "RTE_Rte_START_SEC_CODE")
            .replace("RTE_STOP_SEC_CODE", "RTE_Rte_STOP_SEC_CODE")
            .into_bytes(),
    );
    let os_config = super::ecu::os_configuration(
        description,
        &mut files,
        &os.counter_symbol,
        &os.task_symbol,
        os.use_res_scheduler,
    )?;
    put(
        &mut files,
        "src/Ecu_Config.c",
        os_config
            .split("static const EcuFrameConfig frames[]")
            .next()
            .unwrap()
            .to_string(),
    );
    put(
        &mut files,
        "include/Ecu_Config.h",
        header(
            "ECU_CONFIG_H",
            "#include \"Std_Types.h\"\ntypedef enum { ECU_TX_SYNCHRONOUS = 0, ECU_TX_QUEUED = 1 } EcuTxConfirmation;\ntypedef struct { EcuTxConfirmation tx_confirmation; } EcuPolicyConfig;\nextern const EcuPolicyConfig Ecu_Policy;",
        ),
    );
    let profile = format!(
        "#define ECU_MULTI_COMPONENT 1\n#define CAN_HOST_QUEUED_COMPLETION 1\n#define CAN_ZERO_LENGTH_SUPPORTED 1\n#define CAN_RX_POLLING 1\n#define CAN_TX_POLLING 1\n#define CAN_BUSOFF_POLLING 1\n#define CAN_CONTROLLER_ID {}u\n#define CAN_CANIF_CONTROLLER_ID {}u\n#define CAN_TX_HOH {}u",
        can.controller_id, can.can_if_controller_id, can.transmit_handle
    );
    put(
        &mut files,
        "include/Ecu_ProfileConfig.h",
        header("ECU_PROFILE_CONFIG_H", &profile),
    );
    let cfg = files.get_mut("include/ComStack_Cfg.h").unwrap();
    *cfg = Cow::Owned(
        String::from_utf8(cfg.to_vec())
            .unwrap()
            .replace(
                "#include \"Platform_Types.h\"",
                "#include \"Platform_Types.h\"\n#include \"Ecu_ProfileConfig.h\"",
            )
            .into_bytes(),
    );
    put(
        &mut files,
        "src/Ecu_Execution.h",
        header(
            "ECU_EXECUTION_H",
            "#include \"Ecu_Config.h\"\n#include \"Can.h\"\nEcuStatus Ecu_ExecutionTransmit(CanTxSink sink, PduIdType handle, uint32_t id, uint8_t dlc, const uint8_t data[8]);\nuint64_t Ecu_ExecutionNow(void);",
        ),
    );
    put(&mut files, "src/Ecu_Execution.c", "#include \"Ecu_Execution.h\"\n#include \"Ecu_Target.h\"\nEcuStatus Ecu_ExecutionTransmit(CanTxSink sink, PduIdType handle, uint32_t id, uint8_t dlc, const uint8_t data[8]) {\n    (void)sink;\n    return Ecu_TargetEnqueueTransmit(handle, id, dlc, data);\n}\nuint64_t Ecu_ExecutionNow(void) { return Ecu_TargetNow(); }\n".into());
    for (module, area) in [("CanIf", "CANIF"), ("ComM", "COMM"), ("CanTp", "CANTP")] {
        put(
            &mut files,
            &format!("include/SchM_{module}.h"),
            header(
                &format!("SCHM_{}_H", module.to_uppercase()),
                &format!(
                    "void SchM_Enter_{module}_{area}_STATE(void);\nvoid SchM_Exit_{module}_{area}_STATE(void);{}",
                    if module == "CanTp" {
                        "\nvoid CanTp_MainFunction(void);"
                    } else {
                        ""
                    }
                ),
            ),
        );
    }
    put(
        &mut files,
        "include/SchM_Com.h",
        header(
            "SCHM_COM_H",
            &format!(
                "void {}(void);\nvoid {}(void);",
                com.receive_main.symbol, com.transmit_main.symbol
            ),
        ),
    );
    let comm_header = files.get_mut("include/SchM_ComM.h").unwrap();
    *comm_header = Cow::Owned(
        String::from_utf8(comm_header.to_vec())
            .unwrap()
            .replace(
                "#endif",
                &format!("void {}(void);\n#endif", mode.main_symbol),
            )
            .into_bytes(),
    );
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
    let mask = |path: &str| {
        description
            .events
            .iter()
            .find(|event| event.path == path)
            .unwrap()
            .mask
    };
    let io = description
        .events
        .iter()
        .find(|event| event.path != app.os_event && event.path != work.os_event)
        .ok_or_else(failure)?;
    let rx = description
        .signals
        .iter()
        .find(|signal| signal.receive)
        .ok_or_else(failure)?;
    put(
        &mut files,
        "include/Ecu_TargetConfig.h",
        header(
            "ECU_TARGET_CONFIG_H",
            &format!(
                "#include \"Ecu_ProfileConfig.h\"\n#include \"Os_Target.h\"\n#define ECU_TARGET_TASK 0u\n#define ECU_MAX_PDU_PAYLOAD {}u\n#define ECU_TARGET_DIAGNOSTIC_RX_CAN_ID {}u\n#define ECU_TARGET_EVENT_WORK {}u\n#define ECU_TARGET_EVENT_APP {}u\n#define ECU_TARGET_EVENT_IO {}u\n#define ECU_TARGET_RX_CAN_ID {}u\n#define ECU_TARGET_RX_DEADLINE_MS {}u\nextern const Os_TargetConfig Ecu_OsConfig;\nStd_ReturnType Ecu_MultiBootstrap(void);\nStd_ReturnType Ecu_MultiRunCycle(EventMaskType events);\nvoid Ecu_MultiRetire(void);\nint Ecu_MultiFrameShape(uint32 id, uint8 dlc);\n#ifdef ECU_TARGET_TESTS\nint Ecu_TargetTestFailStage(unsigned stage);\nvoid Ecu_TargetTestShutdown(StatusType reason);\n#endif",
                description
                    .diagnostic_transport
                    .as_ref()
                    .map_or(4, |transport| transport.buffer_bytes),
                description
                    .diagnostic_transport
                    .as_ref()
                    .map_or(u32::MAX, |transport| transport.request_can_id),
                mask(&work.os_event),
                mask(&app.os_event),
                io.mask,
                rx.can_id,
                rx.deadline_ms.unwrap()
            ),
        ),
    );
    let mut config = String::from(
        "/** @file Source-derived immutable communication configurations and owner dispatch. */\n#define RTE_CORE\n#include \"Ecu_TargetConfig.h\"\n#include \"Ecu_Target.h\"\n#include \"Ecu_Config.h\"\n#include \"Can.h\"\n#include \"CanIf.h\"\n#include \"CanTp.h\"\n#include \"Com.h\"\n#include \"Com_Internal.h\"\n#include \"ComM.h\"\n#include \"ComM_BswM.h\"\n#include \"ComM_Internal.h\"\n#include \"BswM.h\"\n#include \"BswM_ComM.h\"\n#include \"Ecu_HostBusSM.h\"\n#include \"Dcm.h\"\n#include \"Dcm_ComM.h\"\n#include \"PduR.h\"\n#include \"PduR_Com.h\"\n#include \"PduR_CanIf.h\"\n#include \"LSduR.h\"\n#include \"LSduR_PduR.h\"\n#include \"Rte_Main.h\"\n#include \"Rte_Com.h\"\n#include \"SchM_Can.h\"\n#include \"SchM_CanTp.h\"\n#include \"SchM_Com.h\"\n#include \"SchM_ComM.h\"\n#include \"SchM_Dcm.h\"\n",
    );
    for component in &multi.components {
        writeln!(
            config,
            "#include \"{}\"",
            component.header.trim_start_matches("include/")
        )
        .unwrap();
    }
    config.push_str("static Std_ReturnType Ecu_BswMAction(NetworkHandleType channel, ComM_ModeType requested);\nstatic void Ecu_ComModeChanged(NetworkHandleType channel, ComM_ModeType mode);\nconst EcuPolicyConfig Ecu_Policy = {ECU_TX_QUEUED};\nstatic const Can_ConfigType driver = {NULL_PTR};\n");
    let mut receive = String::new();
    let mut transmit = String::new();
    let routing_headers = "#include \"Com.h\"\n#include \"Dcm.h\"\n#include \"CanTp.h\"\n#include \"PduR.h\"\n#include \"PduR_Com.h\"\n#include \"PduR_CanIf.h\"\n#include \"LSduR.h\"\n#include \"LSduR_PduR.h\"\n";
    let mut pd_configuration = format!(
        "/** @file Source-derived selected PduR configuration. */\n#include \"PduR_Cfg.h\"\n{routing_headers}"
    );
    let mut ls_configuration = format!(
        "/** @file Source-derived selected LSduR configuration. */\n#include \"LSduR_Cfg.h\"\n{routing_headers}"
    );
    let mut ls_rx = String::new();
    let mut ls_tx = String::new();
    let mut pd_rx = String::new();
    let mut pd_tx = String::new();
    config.push_str("static const Com_PduConfigType com_pdus[] = {\n");
    for signal in &description.signals {
        let route = description
            .routes
            .iter()
            .find(|route| route.pdu == signal.global_pdu)
            .unwrap();
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
            .unwrap()
            .handle;
        // communication::inspect requires source ComSignalInitValue=0.
        // Receiver ComSpec init is independent RTE state; it must not replace
        // the actual COM initialization or group Start(TRUE) value.
        let initial = 0;
        let reception = com
            .receptions
            .iter()
            .find(|reception| reception.signal == signal.com_signal);
        writeln!(
            config,
            "    {{{pdu}u, {signal_id}u, {}, {initial}u, {}u, {}u}},",
            if signal.receive { "TRUE" } else { "FALSE" },
            reception.map_or(0, |rx| rx.timeout_ms / com.receive_main.period_ms),
            reception.map_or(0, |rx| u32::from(rx.callback_handle))
        )
        .unwrap();
        if signal.receive {
            writeln!(
                receive,
                "    {{{}u, {}u, 4u}},",
                signal.can_id, signal.can_if_handle
            )
            .unwrap();
            writeln!(
                ls_rx,
                "    {{{}u, {}u, PduR_CanIfRxIndication}},",
                signal.can_if_handle, route.source_handle
            )
            .unwrap();
            writeln!(
                pd_rx,
                "    {{{}u, {pdu}u, Com_RxIndication}},",
                route.source_handle
            )
            .unwrap();
        } else {
            writeln!(
                transmit,
                "    {{{}u, {}u, 4u}},",
                signal.can_id, signal.can_if_handle
            )
            .unwrap();
            writeln!(ls_tx, "    {{LSDUR_UP_PDUR, {}u, {}u, PduR_CanIfTxConfirmation, PduR_CanIfTriggerTransmit}},", route.destination_handle, signal.can_if_handle).unwrap();
            writeln!(pd_tx, "    {{PDUR_UP_COM, {}u, {}u, LSduR_PduRTransmit, Com_TxConfirmation, Com_TriggerTransmit}},", route.source_handle, route.destination_handle).unwrap();
        }
    }
    config.push_str("};\n");
    writeln!(config, "static const Com_ConfigType com_config = {{com_pdus, {}u, {}u, Rte_COMCbk, Rte_COMCbkRxTOut, PduR_ComTransmit}};", description.signals.len(), com.receive_group.handle).unwrap();
    let transport = &description.diagnostic_transport;
    if let Some(tp) = transport {
        let rx_route = description
            .routes
            .iter()
            .find(|route| route.pdu == tp.rx_sdu)
            .unwrap();
        let tx_route = description
            .routes
            .iter()
            .find(|route| route.pdu == tp.tx_sdu)
            .unwrap();
        config.push_str("#include \"PduR_CanTp.h\"\n#include \"LSduR_CanTp.h\"\n");
        writeln!(
            receive,
            "    {{{}u, {}u, 8u}},",
            tp.request_can_id, tp.request_can_if_handle
        )
        .unwrap();
        writeln!(
            transmit,
            "    {{{}u, {}u, 8u}},",
            tp.response_can_id, tp.response_can_if_handle
        )
        .unwrap();
        writeln!(
            ls_rx,
            "    {{{}u, {}u, CanTp_RxIndication}},",
            tp.request_can_if_handle, tp.receive_npdu
        )
        .unwrap();
        writeln!(
            ls_tx,
            "    {{LSDUR_UP_CANTP, {}u, {}u, CanTp_TxConfirmation, NULL_PTR}},",
            tp.transmit_npdu, tp.response_can_if_handle
        )
        .unwrap();
        writeln!(
            pd_tx,
            "    {{PDUR_UP_DCM, {}u, {}u, CanTp_Transmit, Dcm_TpTxConfirmation, NULL_PTR}},",
            tx_route.source_handle, tp.transmit_nsdu
        )
        .unwrap();
        writeln!(pd_configuration, "static const PduR_TpRouteType tp_routes[] = {{\n    {{{}u, {}u, {}u, {}u, Dcm_StartOfReception, Dcm_CopyRxData, Dcm_TpRxIndication, Dcm_CopyTxData, Dcm_TpTxConfirmation}}\n}};", rx_route.source_handle, tp.dcm_receive, tx_route.destination_handle, tp.dcm_transmit).unwrap();
        let period = description.schedule.counter_tick_ms;
        writeln!(config, "static const CanTp_ConfigType tp_config = {{{}u, {}u, {}u, {}u, {}u, {}u, {}u, {}u, {}u, {}u, {period}u, 0u, 0u, {}u, {}u}};", tp.receive_nsdu, tp.transmit_nsdu, tp.receive_npdu, tp.transmit_npdu, tp.buffer_bytes, tp.n_as_ms / period, tp.n_ar_ms / period, tp.n_bs_ms / period, tp.n_cr_ms / period, tp.n_cs_ms / period, rx_route.source_handle, tx_route.destination_handle).unwrap();
        let (did, read) =
            description
                .diagnostic
                .as_ref()
                .map_or((0, "NULL_PTR".to_string()), |did| {
                    let bridge = multi
                        .components
                        .iter()
                        .flat_map(|component| &component.runnables)
                        .find(|runnable| runnable.event.is_none())
                        .unwrap();
                    (did.did, bridge.symbol.clone())
                });
        writeln!(
            config,
            "static void Ecu_DcmAuthenticationMode(uint8 state);\n"
        )
        .unwrap();
        writeln!(config, "static const Dcm_ConfigType dcm_config = {{{}u, {}u, {}u, {did}u, {read}, {}u, {}u, {}u, {}u, {}u, {}u, Ecu_DcmAuthenticationMode}};", tp.dcm_receive, tp.dcm_transmit, mode.channel_handle, tp.p2_ms / period, tp.p2_ms, tp.p2_star_ms, tp.s3_ms / period, tp.buffer_bytes, tx_route.source_handle).unwrap();
    }
    for (name, ty, entries) in [
        ("can_rx", "CanIf_RxPduConfigType", &receive),
        ("can_tx", "CanIf_TxPduConfigType", &transmit),
        ("ls_rx", "LSduR_RxRouteType", &ls_rx),
        ("ls_tx", "LSduR_TxRouteType", &ls_tx),
        ("pd_rx", "PduR_RxRouteType", &pd_rx),
        ("pd_tx", "PduR_TxRouteType", &pd_tx),
    ] {
        let owner = if name.starts_with("ls_") {
            &mut ls_configuration
        } else if name.starts_with("pd_") {
            &mut pd_configuration
        } else {
            &mut config
        };
        writeln!(owner, "static const {ty} {name}[] = {{\n{entries}}};").unwrap();
    }
    let rx_count = description
        .signals
        .iter()
        .filter(|signal| signal.receive)
        .count()
        + usize::from(transport.is_some());
    let tx_count = description
        .signals
        .iter()
        .filter(|signal| !signal.receive)
        .count()
        + usize::from(transport.is_some());
    writeln!(config, "static const CanIf_ConfigType can_if_config = {{can_rx, {rx_count}u, can_tx, {tx_count}u, Ecu_HostBusSM_ControllerModeIndication, Ecu_HostBusSM_ControllerBusOff, {}u, {}u, {}u, {}u}};", can.can_if_controller_id, can.controller_id, can.receive_handle, can.transmit_handle).unwrap();
    writeln!(
        ls_configuration,
        "const LSduR_PBConfigType LSduR_Config = {{0u, ls_rx, {rx_count}u, ls_tx, {tx_count}u}};"
    )
    .unwrap();
    writeln!(
        pd_configuration,
        "const PduR_PBConfigType PduR_Config = {{0u, pd_rx, {}u, pd_tx, {tx_count}u, {}, {}u}};",
        rx_count - usize::from(transport.is_some()),
        if transport.is_some() {
            "tp_routes"
        } else {
            "NULL_PTR"
        },
        usize::from(transport.is_some())
    )
    .unwrap();
    for (module, source) in [("PduR", &pd_configuration), ("LSduR", &ls_configuration)] {
        put(
            &mut files,
            &format!("include/{module}_Cfg.h"),
            header(
                &format!("{}_CFG_H", module.to_uppercase()),
                &format!(
                    "#include \"{module}.h\"\nextern const {module}_PBConfigType {module}_Config;"
                ),
            ),
        );
        put(
            &mut files,
            &format!("src/{module}_PBcfg.c"),
            mapped_configuration(source),
        );
        writeln!(config, "#include \"{module}_Cfg.h\"").unwrap();
    }

    writeln!(config, "static const Ecu_HostBusSM_ConfigType bus_config = {{{}u, {}u}};\nstatic const ComM_UserHandleType users[] = {{{}}};\nstatic const ComM_ConfigType comm_config = {{{}u, users, {}u, {}u, Ecu_HostBusSM_RequestComMode, Ecu_HostBusSM_GetCurrentComMode, Ecu_ComModeChanged, {}u}};\nstatic const BswM_ConfigType bswm_config = {{{}u, {}u, Ecu_BswMAction}};", mode.channel_handle, can.can_if_controller_id, mode.users.iter().map(|user| format!("{}u", user.handle)).collect::<Vec<_>>().join(", "), mode.channel_handle, mode.users.len(), mode.minimum_full_ms / mode.period_ms, mode.ecu_group_classification, mode.channel_handle, mode.initial_mode).unwrap();
    for (symbol, scope, body) in [
        (&com.receive_main.symbol, "Com", "Ecu_ComMainFunctionRx();"),
        (&com.transmit_main.symbol, "Com", "Ecu_ComMainFunctionTx();"),
        (&mode.main_symbol, "ComM", "(void)ComM_RunChannel();"),
    ] {
        writeln!(config, "#define RTE_{scope}_START_SEC_CODE\n#include \"Ecu_MemMap.h\"\nRTE_{scope}_CODE void {symbol}(void) {{ {body} }}\n#define RTE_{scope}_STOP_SEC_CODE\n#include \"Ecu_MemMap.h\"").unwrap();
    }
    config.push_str("#define RTE_Ecu_START_SEC_CODE\n#include \"Ecu_MemMap.h\"\n");
    config.push_str("static void Ecu_ComModeChanged(NetworkHandleType channel, ComM_ModeType current) {\n    BswM_ComM_CurrentMode(channel, current);\n");
    if transport.is_some() {
        config.push_str("    if (current == COMM_FULL_COMMUNICATION) { Dcm_ComM_FullComModeEntered(channel); }\n    else if (current == COMM_SILENT_COMMUNICATION) { Dcm_ComM_SilentComModeEntered(channel); }\n    else { Dcm_ComM_NoComModeEntered(channel); }\n");
    }
    config.push_str("}\n");
    // ECUC_BswM_00843: parameters are fixed by the configured complete call.
    config.push_str("static Std_ReturnType Ecu_BswMAction(NetworkHandleType channel, ComM_ModeType requested) {\n");
    writeln!(
        config,
        "    if (channel != {}u) {{ return E_NOT_OK; }}\n    switch (requested) {{",
        mode.channel_handle
    )
    .unwrap();
    for rule in &mode.rules {
        writeln!(
            config,
            "    case {}u: (void){}; break;",
            rule.mode, rule.callout
        )
        .unwrap();
    }
    config.push_str("    default: return E_NOT_OK;\n    }\n    return E_OK;\n}\n");
    config.push_str("Std_ReturnType Ecu_MultiBootstrap(void) {\n    Can_ControllerStateType actual;\n    if (Ecu_TargetCheckLifecycleContext(TRUE) != E_OK) { return E_NOT_OK; }\n    SchM_Init(&Ecu_SchMConfig);\n    SchM_Start();\n    if (Ecu_SchMReady() == FALSE) { return E_NOT_OK; }\n    Can_Init(&driver);\n    CanIf_Init(&can_if_config);\n    Ecu_HostBusSM_Init(&bus_config);\n    LSduR_Init(&LSduR_Config);\n    PduR_Init(&PduR_Config);\n    Com_Init(&com_config);\n    Com_IpduGroupStart(com_config.receive_group, TRUE);\n    Com_EnableReceptionDM(com_config.receive_group);\n    ComM_Init(&comm_config);\n    BswM_Init(&bswm_config);\n");
    if transport.is_some() {
        config.push_str("    CanTp_Init(&tp_config);\n    Dcm_Init(&dcm_config);\n");
    }
    writeln!(config, "    if (Rte_Start() != E_OK) {{ return E_NOT_OK; }}\n    ComM_CommunicationAllowed({}u, TRUE);\n    if (ComM_RequestComMode({}u, COMM_FULL_COMMUNICATION) != E_OK) {{ return E_NOT_OK; }}\n    {}();\n    Can_MainFunction_Mode();\n    if ((CanIf_GetControllerMode({}u, &actual) != E_OK) || (actual != CAN_CS_STARTED)) {{ return E_NOT_OK; }}\n    SchM_StartTiming();\n    return Ecu_SchMTimingActive() ? E_OK : E_NOT_OK;\n}}", mode.channel_handle, mode.users[0].handle, mode.main_symbol, can.can_if_controller_id).unwrap();
    config.push_str("Std_ReturnType Ecu_MultiRunCycle(EventMaskType events) {\n    if ((Ecu_TargetIsOwner() == 0) || (Ecu_SchMTimingActive() == FALSE)) { return E_NOT_OK; }\n");
    for entity in &description.schedule.entities {
        if entity.application {
            writeln!(
                config,
                "    if ((events & {}u) != 0u) {{ {}(); }}",
                mask(&entity.os_event),
                entity.symbol
            )
            .unwrap();
        } else {
            let symbol = &entity.symbol;
            writeln!(
                config,
                "    if ((events & {}u) != 0u) {{ {symbol}(); }}",
                mask(&entity.os_event)
            )
            .unwrap();
            if entity.symbol == com.transmit_main.symbol {
                config.push_str("    if (Can_HostFlush() != ECU_OK) { return E_NOT_OK; }\n");
            }
        }
    }
    config.push_str("    return (Can_HostFlush() == ECU_OK) ? E_OK : E_NOT_OK;\n}\nvoid Ecu_MultiRetire(void) {\n    (void)Rte_Stop();\n");
    if transport.is_some() {
        config.push_str("    CanTp_Shutdown();\n");
    }
    config.push_str(
        "    Com_DeInit();\n    BswM_Deinit();\n    Ecu_HostBusSM_DeInit();\n    CanIf_DeInit();\n",
    );
    writeln!(config, "    (void)Can_SetControllerMode({}u, CAN_CS_STOPPED);\n    Can_MainFunction_Mode();\n    Can_MainFunction_BusOff();\n    Can_DeInit();\n    SchM_Deinit();\n}}", can.controller_id).unwrap();
    config.push_str("int Ecu_MultiFrameShape(uint32 id, uint8 dlc) {\n    if ((id > 0x7ffu) || (dlc > 8u)) { return 0; }\n");
    for signal in &description.signals {
        writeln!(
            config,
            "    if (id == {}u) {{ return {}; }}",
            signal.can_id,
            if signal.receive { "dlc == 4u" } else { "0" }
        )
        .unwrap();
    }
    if let Some(tp) = transport {
        writeln!(
            config,
            "    if (id == {}u) {{ return dlc == 8u; }}\n    if (id == {}u) {{ return 0; }}",
            tp.request_can_id, tp.response_can_id
        )
        .unwrap();
    }
    config.push_str("    return 1;\n}\n");
    config.push_str("#define RTE_Ecu_STOP_SEC_CODE\n#include \"Ecu_MemMap.h\"\n");
    for (signature, mapped) in [
        (
            "static void Ecu_ComModeChanged(NetworkHandleType channel, ComM_ModeType current) {",
            "static RTE_Ecu_CODE void Ecu_ComModeChanged(NetworkHandleType channel, ComM_ModeType current) {",
        ),
        (
            "static Std_ReturnType Ecu_BswMAction(NetworkHandleType channel, ComM_ModeType requested) {",
            "static RTE_Ecu_CODE Std_ReturnType Ecu_BswMAction(NetworkHandleType channel, ComM_ModeType requested) {",
        ),
        (
            "Std_ReturnType Ecu_MultiBootstrap(void) {",
            "RTE_Ecu_CODE Std_ReturnType Ecu_MultiBootstrap(void) {",
        ),
        (
            "Std_ReturnType Ecu_MultiRunCycle(EventMaskType events) {",
            "RTE_Ecu_CODE Std_ReturnType Ecu_MultiRunCycle(EventMaskType events) {",
        ),
        (
            "void Ecu_MultiRetire(void) {",
            "RTE_Ecu_CODE void Ecu_MultiRetire(void) {",
        ),
        (
            "int Ecu_MultiFrameShape(uint32 id, uint8 dlc) {",
            "RTE_Ecu_CODE int Ecu_MultiFrameShape(uint32 id, uint8 dlc) {",
        ),
    ] {
        config = config.replace(signature, mapped);
    }
    if description.diagnostic_transport.is_some() {
        writeln!(config, "#define RTE_Dcm_START_SEC_CODE\n#include \"Ecu_MemMap.h\"\nstatic RTE_Dcm_CODE void Ecu_DcmAuthenticationMode(uint8 state) {{ (void)SchM_Switch_Dcm_{authentication_group}(state); }}\n#define RTE_Dcm_STOP_SEC_CODE\n#include \"Ecu_MemMap.h\"").unwrap();
    }
    put(
        &mut files,
        "src/Ecu_RuntimeConfig.c",
        mapped_configuration(&config),
    );
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
    put(
        &mut files,
        "include/Ecu_MemMap.h",
        super::multi_rte::memory_map(&config_scopes),
    );
    let mode_scope = multi
        .components
        .iter()
        .find(|component| component.diagnostic_session_port.is_some())
        .map(|component| component.component.rsplit('/').next().unwrap())
        .unwrap_or("Dcm");
    lifecycle(
        &mut files,
        communication.partition.id,
        mode_scope,
        authentication_group,
    );
    for source in plan.sources() {
        files.insert(
            format!("inputs/{}", source.logical_path()),
            Cow::Owned(source.bytes().to_vec()),
        );
    }
    put(&mut files, "integration.json", format!("{}\n", serde_json::to_string_pretty(&serde_json::json!({"format": "autosar-ecu-integration-v1", "plan": description, "target": target.spec().id, "owner": "single source-derived AUTOSTART extended task", "originalBswSources": super::ecu::source_assets(target, true).map_err(|_| failure())?.iter().map(|(path, asset)| (asset.relative_path, path)).collect::<BTreeMap<_, _>>() })).unwrap()));
    put(
        &mut files,
        "README.md",
        format!(
            "# ECU integration source project\n\nTarget: `{}`. Caller application sources are sealed under `src/`; the source-derived component, communication and OS contracts are in `integration.json`.\n\nUse Python 3.11 or newer and the fixed GCC/binutils/Git identities in `target.json`. From this directory run `python tools/ecu-tool.py build --project . --output <new-empty-external-directory> --mode host-batch`. The production BEGIN/RX/COMMIT protocol uses the single OS owner and controlled logical ticks. Physical output flush precedes driver polling confirmation. No compiler, AUTOSAR archive or application algorithm is generated or redistributed.\n",
            target.spec().id
        ),
    );
    files.extend(
        super::artifacts::files(description, &files)
            .map_err(|_| failure())?
            .into_iter()
            .map(|(path, bytes)| (path, Cow::Owned(bytes))),
    );
    Ok(files)
}

fn lifecycle(
    files: &mut BTreeMap<String, Cow<'_, [u8]>>,
    partition: u16,
    mode_scope: &str,
    authentication_group: &str,
) {
    let main = files.get_mut("include/Rte_Main.h").unwrap();
    *main = Cow::Owned(String::from_utf8(main.to_vec()).unwrap().replace("#endif", "typedef struct { uint16 partition; } SchM_ConfigType;\nvoid SchM_Init(const SchM_ConfigType *config);\nvoid SchM_Start(void);\nvoid SchM_StartTiming(void);\nvoid SchM_Deinit(void);\nextern const SchM_ConfigType Ecu_SchMConfig;\nboolean Ecu_SchMReady(void);\nboolean Ecu_SchMTimingActive(void);\n#endif").into_bytes());
    put(
        files,
        "src/SchM.c",
        format!(
            "#include \"Rte_Main.h\"\n#include \"Ecu_Target.h\"\n#define RTE_Rte_START_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\nstatic boolean initialized RTE_VAR_CLEARED;\nstatic boolean started RTE_VAR_CLEARED;\nstatic boolean timing RTE_VAR_CLEARED;\nstatic boolean allocated RTE_VAR_CLEARED;\n#define RTE_Rte_STOP_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n#define RTE_Rte_START_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED\n#include \"Rte_MemMap.h\"\nconst SchM_ConfigType Ecu_SchMConfig RTE_CONFIG_DATA_PREBUILD = {{{partition}u}};\n#define RTE_Rte_STOP_SEC_CONFIG_DATA_PREBUILD_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n#define RTE_Rte_START_SEC_CODE\n#include \"Rte_MemMap.h\"\nRTE_CODE void SchM_Init(const SchM_ConfigType *config) {{\n    if ((Ecu_TargetCheckLifecycleContext(TRUE) != E_OK) || (allocated == TRUE) || (config == NULL_PTR) || (config->partition != {partition}u)) {{ return; }}\n    allocated = TRUE;\n    initialized = TRUE;\n}}\nRTE_CODE void SchM_Start(void) {{\n    if ((Ecu_TargetCheckLifecycleContext(TRUE) == E_OK) && (initialized == TRUE)) {{ started = TRUE; }}\n}}\nRTE_CODE void SchM_StartTiming(void) {{\n    if ((Ecu_TargetCheckLifecycleContext(TRUE) == E_OK) && (started == TRUE)) {{ timing = TRUE; }}\n}}\nRTE_CODE void SchM_Deinit(void) {{\n    if (Ecu_TargetCheckLifecycleContext(FALSE) == E_OK) {{ timing = FALSE; started = FALSE; initialized = FALSE; }}\n}}\nRTE_CODE boolean Ecu_SchMReady(void) {{ return started; }}\nRTE_CODE boolean Ecu_SchMTimingActive(void) {{ return timing; }}\n#define RTE_Rte_STOP_SEC_CODE\n#include \"Rte_MemMap.h\"\n"
        ),
    );
    let modes = format!(
        "\n#include \"SchM_Dcm.h\"\n#include \"Can_HostLock.h\"\n#define RTE_{mode_scope}_START_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\nstatic Rte_ModeType_DcmDiagnosticSessionControl diagnostic_session_mode RTE_VAR_CLEARED;\nstatic Rte_ModeType_{authentication_group} authentication_mode RTE_VAR_CLEARED;\n#define RTE_{mode_scope}_STOP_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n#define RTE_{mode_scope}_START_SEC_CODE\n#include \"Rte_MemMap.h\"\nRTE_CODE Std_ReturnType SchM_Switch_Dcm_{authentication_group}(Rte_ModeType_{authentication_group} mode) {{\n    if (Ecu_TargetIsOwner() == 0) {{ return SCHM_E_LIMIT; }}\n    Can_Lock();\n    authentication_mode = mode;\n    Can_Unlock();\n    return SCHM_E_OK;\n}}\nRTE_CODE Rte_ModeType_{authentication_group} SchM_Mode_Dcm_{authentication_group}(void) {{\n    Rte_ModeType_{authentication_group} mode;\n    if (Ecu_TargetIsOwner() == 0) {{ return RTE_TRANSITION_{authentication_group}; }}\n    Can_Lock();\n    mode = authentication_mode;\n    Can_Unlock();\n    return mode;\n}}\nRTE_CODE Std_ReturnType SchM_Switch_Dcm_DcmDiagnosticSessionControl(Rte_ModeType_DcmDiagnosticSessionControl mode) {{\n    if (Ecu_TargetIsOwner() == 0) {{ return SCHM_E_LIMIT; }}\n    Can_Lock();\n    diagnostic_session_mode = mode;\n    Can_Unlock();\n    return SCHM_E_OK;\n}}\nRTE_CODE Rte_ModeType_DcmDiagnosticSessionControl SchM_Mode_Dcm_DcmDiagnosticSessionControl(void) {{\n    Rte_ModeType_DcmDiagnosticSessionControl mode;\n    if (Ecu_TargetIsOwner() == 0) {{ return RTE_TRANSITION_DcmDiagnosticSessionControl; }}\n    Can_Lock();\n    mode = diagnostic_session_mode;\n    Can_Unlock();\n    return mode;\n}}\n#define RTE_{mode_scope}_STOP_SEC_CODE\n#include \"Rte_MemMap.h\"\n"
    );
    let lifecycle = files.get_mut("src/SchM.c").unwrap();
    lifecycle.to_mut().extend_from_slice(modes.as_bytes());
}
