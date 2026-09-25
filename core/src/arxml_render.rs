use crate::model::{DiagnosticView, Direction, FrameView, SignalView};
use std::fmt::Write;

fn ref_path(project: &str, name: &str) -> String {
    format!("/{project}/{name}")
}
fn global_pdu_path(project: &str, name: &str) -> String {
    format!("/{project}/EcuCCfg/EcucConfigSet/Pdus/{name}")
}


fn param(name: &str, kind: &str, value: impl std::fmt::Display, definition: &str) -> String {
    format!("<ECUC-{kind}-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-{definition}-PARAM-DEF\">{name}</DEFINITION-REF><VALUE>{value}</VALUE></ECUC-{kind}-PARAM-VALUE>")
}

fn number(path: &str, name: &str, value: impl std::fmt::Display) -> String {
    param(&format!("{path}/{name}"), "NUMERICAL", value, "INTEGER")
}

fn decimal(path: &str, name: &str, milliseconds: u32) -> String {
    param(&format!("{path}/{name}"), "NUMERICAL", format!("{}.{:03}", milliseconds / 1000, milliseconds % 1000), "FLOAT")
}

fn choice(path: &str, name: &str, value: &str) -> String {
    param(&format!("{path}/{name}"), "TEXTUAL", value, "ENUMERATION")
}

fn text(path: &str, name: &str, value: impl std::fmt::Display) -> String {
    param(&format!("{path}/{name}"), "TEXTUAL", value, "STRING")
}
fn boolean(path: &str, name: &str, value: bool) -> String {
    param(&format!("{path}/{name}"), "NUMERICAL", value, "BOOLEAN")
}

fn function(path: &str, name: &str, value: &str) -> String {
    format!("<ECUC-TEXTUAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-FUNCTION-NAME-DEF\">{path}/{name}</DEFINITION-REF><VALUE>{value}</VALUE></ECUC-TEXTUAL-PARAM-VALUE>")
}


fn reference(path: &str, name: &str, dest: &str, target: &str) -> String {
    format!("<ECUC-REFERENCE-VALUE><DEFINITION-REF DEST=\"ECUC-REFERENCE-DEF\">{path}/{name}</DEFINITION-REF><VALUE-REF DEST=\"{dest}\">{target}</VALUE-REF></ECUC-REFERENCE-VALUE>")
}

fn container(name: &str, definition: &str, params: &str, refs: &str, children: &str) -> String {
    let mut value = format!("<ECUC-CONTAINER-VALUE><SHORT-NAME>{name}</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">{definition}</DEFINITION-REF>");
    if !params.is_empty() {
        write!(value, "<PARAMETER-VALUES>{params}</PARAMETER-VALUES>").unwrap();
    }
    if !refs.is_empty() {
        write!(value, "<REFERENCE-VALUES>{refs}</REFERENCE-VALUES>").unwrap();
    }
    if !children.is_empty() {
        write!(value, "<SUB-CONTAINERS>{children}</SUB-CONTAINERS>").unwrap();
    }
    value.push_str("</ECUC-CONTAINER-VALUE>");
    value
}

fn module(name: &str, definition: &str, children: &str) -> String {
    format!("<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>{name}</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-MODULE-DEF\">/AUTOSAR/EcucDefs/{definition}</DEFINITION-REF><CONTAINERS>{children}</CONTAINERS></ECUC-MODULE-CONFIGURATION-VALUES>")
}

fn render_global_pdu(name: &str, system_path: &str, length: u32) -> String {
    let definition = "/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu";
    let value = container(name, definition, &number(definition, "PduLength", length), "", "");
    let metadata = format!("<ADMIN-DATA><SDGS><SDG GID=\"AutosarWorkbenchGlobalPduV1\"><SD GID=\"SystemPduRef\">{system_path}</SD></SDG></SDGS></ADMIN-DATA>");
    value.replacen("</SHORT-NAME>", &format!("</SHORT-NAME>{metadata}"), 1)
}

fn render_global_pdus(project: &str, frames: &[FrameView], diagnostic: Option<&DiagnosticView>) -> String {
    let mut pdus = String::new();
    for frame in frames {
        let name = format!("Pdu_{}", frame.name);
        pdus.push_str(&render_global_pdu(&name, &ref_path(project, &name), u32::from(frame.dlc)));
    }
    if diagnostic.is_some() {
        for name in ["NPdu_DiagRequest", "NPdu_DiagResponse"] {
            pdus.push_str(&render_global_pdu(name, &ref_path(project, name), 8));
        }
        for name in ["DcmPdu_DiagRequest", "DcmPdu_DiagResponse"] {
            pdus.push_str(&render_global_pdu(name, &ref_path(project, name), 256));
        }
    }
    let collection_path = "/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection";
    let collection_params = format!("{}{}", choice(collection_path, "PduIdTypeEnum", "UINT8"),
        choice(collection_path, "PduLengthTypeEnum", if diagnostic.is_some() { "UINT16" } else { "UINT8" }));
    let collection = container("Pdus", collection_path, &collection_params, "", &pdus);
    let config = container("EcucConfigSet", "/AUTOSAR/EcucDefs/EcuC/EcucConfigSet", "", "", &collection);
    let core_path = "/AUTOSAR/EcucDefs/EcuC/EcucHardware/EcucCoreDefinition";
    let core = container("Core0", core_path, &number(core_path, "EcucCoreId", 0), "", "");
    let hardware = container("Hardware", "/AUTOSAR/EcucDefs/EcuC/EcucHardware", "", "", &core);
    module("EcuCCfg", "EcuC", &(config + &hardware))
}

fn render_dtc(project: &str, diagnostic: &DiagnosticView, elements: &mut String) {
    let dtc = diagnostic.dtc.as_ref().unwrap();
    let base = "/AUTOSAR/EcucDefs/Dem/DemConfigSet";
    let attrs = format!("{base}/DemDTCAttributes");
    let dtc_def = format!("{base}/DemDTC");
    let event = format!("{base}/DemEventParameter");
    let general = "/AUTOSAR/EcucDefs/Dem/DemGeneral";
    let memory = format!("{general}/DemEventMemorySet");
    let primary = format!("{memory}/DemPrimaryMemory");
    let cycle = format!("{general}/DemOperationCycle");
    let client = format!("{general}/DemClient");
    let block_ref = format!("{general}/DemNvRamBlockId");
    let nv_block = ref_path(project, "NvMCfg/EventStatus");
    let primary_ref = ref_path(project, "DemCfg/DemGeneral/Memory/Primary");
    let memory_ref = ref_path(project, "DemCfg/DemGeneral/Memory");
    let cycle_ref = ref_path(project, "DemCfg/DemGeneral/OperationCycle");
    let attrs_ref = ref_path(project, "DemCfg/DemConfigSet/Attributes");
    let dtc_ref = ref_path(project, "DemCfg/DemConfigSet/DTC");
    let mut general_params = String::new();
    for (name, value) in [
        ("DemAgingRequiresNotFailedCycle", false), ("DemAgingRequiresTestedCycle", false),
        ("DemDebounceCounterBasedSupport", false), ("DemDebounceTimeBasedSupport", false),
        ("DemDevErrorDetect", false), ("DemGeneralInterfaceSupport", false),
        ("DemPTOSupport", false), ("DemResetConfirmedBitOnOverflow", false),
        ("DemResetPendingBitOnOverflow", false), ("DemStatusBitStorageTestFailed", true),
        ("DemTriggerFiMReports", false), ("DemTriggerMonitorInitBeforeClearOk", false),
        ("DemVersionInfoApi", false),
    ] {
        general_params.push_str(&boolean(general, name, value));
    }
    for (name, value) in [
        ("DemAvailabilitySupport", "DEM_NO_AVAILABILITY"),
        ("DemClearDTCBehavior", "DEM_CLRRESP_NONVOLATILE_FINISH"),
        ("DemClearDTCLimitation", "DEM_ONLY_CLEAR_ALL_DTCS"),
        ("DemDataElementDefaultEndianness", "BIG_ENDIAN"),
        ("DemEventCombinationSupport", "DEM_EVCOMB_DISABLED"),
        ("DemOBDSupport", "DEM_OBD_NO_OBD_SUPPORT"),
        ("DemStatusBitHandlingTestFailedSinceLastClear", "DEM_STATUS_BIT_NORMAL"),
        ("DemSuppressionSupport", "DEM_NO_SUPPRESSION"),
    ] {
        general_params.push_str(&choice(general, name, value));
    }
    general_params.push_str(&number(general, "DemMaxNumberPrestoredFF", 0));
    general_params.push_str(&decimal(general, "DemTaskTime", 1));
    let primary_params = number(&primary, "DemDtcStatusAvailabilityMask", 0x7f)
        + &choice(&primary, "DemEventDisplacementStrategy", "DEM_DISPLACEMENT_NONE")
        + &choice(&primary, "DemEventMemoryEntryStorageTrigger", "DEM_TRIGGER_ON_TEST_FAILED")
        + &number(&primary, "DemMaxNumberEventEntryPrimary", 1)
        + &choice(&primary, "DemOccurrenceCounterProcessing", "DEM_PROCESS_OCCCTR_TF")
        + &choice(&primary, "DemTypeOfFreezeFrameRecordNumeration", "DEM_FF_RECNUM_CALCULATED");
    let primary_value = container("Primary", &primary, &primary_params, "", "");
    let memory_params = number(&memory, "DemMaxNumberEventEntryPermanent", 0)
        + &choice(&memory, "DemTypeOfDTCSupported", "DEM_DTC_TRANSLATION_ISO14229_1");
    let memory_value = container("Memory", &memory, &memory_params, "", &primary_value);
    let cycle_value = container("OperationCycle", &cycle, &number(&cycle, "DemOperationCycleId", 0), "", "");
    let client_params = choice(&client, "DemClientFunctionality", "DEM_CLIENT_USES_FULL_FUNCTIONALITY")
        + &number(&client, "DemClientId", 0) + &boolean(&client, "DemClientUsesRte", false);
    let client_value = container("DcmClient", &client, &client_params,
        &reference(&client, "DemEventMemorySetRef", "ECUC-CONTAINER-VALUE", &memory_ref), "");
    let block_value = container("StatusBlock", &block_ref, "",
        &reference(&block_ref, "DemNvRamBlockIdRef", "ECUC-CONTAINER-VALUE", &nv_block), "");
    let general_value = container("DemGeneral", general, &general_params, "",
        &(client_value + &memory_value + &cycle_value + &block_value));
    let attrs_value = container("Attributes", &attrs, &number(&attrs, "DemDTCPriority", 1),
        &reference(&attrs, "DemMemoryDestinationRef", "ECUC-CONTAINER-VALUE", &primary_ref), "");
    let dtc_params = number(&dtc_def, "DemDtcValue", dtc.code)
        + &choice(&dtc_def, "DemNvStorageStrategy", "IMMEDIATE_AT_FIRST_OCCURRENCE");
    let dtc_value = container("DTC", &dtc_def, &dtc_params,
        &reference(&dtc_def, "DemDTCAttributesRef", "ECUC-CONTAINER-VALUE", &attrs_ref), "");
    let event_params = boolean(&event, "DemEventAvailable", true)
        + &number(&event, "DemEventConfirmationThreshold", 1)
        + &number(&event, "DemEventId", 1)
        + &choice(&event, "DemEventKind", "DEM_EVENT_KIND_BSW")
        + &choice(&event, "DemEventReportingType", "STANDARD_REPORTING")
        + &boolean(&event, "DemFFPrestorageSupported", false);
    let event_refs = reference(&event, "DemDTCRef", "ECUC-CONTAINER-VALUE", &dtc_ref)
        + &reference(&event, "DemOperationCycleRef", "ECUC-CONTAINER-VALUE", &cycle_ref);
    let debounce = format!("{event}/DemDebounceAlgorithmClass/DemDebounceMonitorInternal");
    let event_value = container("RxFrameTimeout", &event, &event_params, &event_refs,
        &container("Debounce", &debounce, "", "", ""));
    // The ECUC event has no standardized Rx CAN-frame reference. This one SDG
    // identifies the host-only monitor binding without claiming a ComM/MemIf link.
    let admin = format!("<ADMIN-DATA><SDGS><SDG GID=\"AutosarWorkbenchDtc\"><SD GID=\"MonitorFrameRef\">{}</SD></SDG></SDGS></ADMIN-DATA>", dtc.monitor_frame_path);
    let event_value = event_value.replacen("</SHORT-NAME>", &format!("</SHORT-NAME>{admin}"), 1);
    elements.push_str(&module("DemCfg", "Dem",
        &(container("DemConfigSet", base, "", "", &(attrs_value + &dtc_value + &event_value)) + &general_value)));

    let nv = "/AUTOSAR/EcucDefs/NvM/NvMBlockDescriptor";
    let mut nv_params = String::new();
    for (name, value) in [
        ("NvMBlockJobPriority", 1), ("NvMMaxNumOfReadRetries", 0),
        ("NvMMaxNumOfWriteRetries", 0), ("NvMNvBlockBaseNumber", 1),
        ("NvMNvBlockLength", 1), ("NvMNvBlockNum", 2),
        ("NvMNvramBlockIdentifier", 2), ("NvMNvramDeviceId", 0),
        ("NvMRomBlockNum", 0), ("NvMWriteVerificationDataSize", 1),
    ] { nv_params.push_str(&number(nv, name, value)); }
    for (name, value) in [
        ("NvMBlockUseAutoValidation", false), ("NvMBlockUseCompression", false),
        ("NvMBlockUseCrc", true), ("NvMBlockUseCRCCompMechanism", false),
        ("NvMBlockUsePort", false), ("NvMBlockUseSetRamBlockStatus", false),
        ("NvMBlockUseSyncMechanism", false), ("NvMBlockWriteProt", false),
        ("NvMBswMBlockStatusInformation", false), ("NvMResistantToChangedSw", false),
        ("NvMStaticBlockIDCheck", true), ("NvMWriteBlockOnce", false),
        ("NvMWriteVerification", true),
    ] { nv_params.push_str(&boolean(nv, name, value)); }
    nv_params.push_str(&choice(nv, "NvMBlockCrcType", "NVM_CRC32"));
    nv_params.push_str(&choice(nv, "NvMBlockManagementType", "NVM_BLOCK_REDUNDANT"));
    elements.push_str(&module("NvMCfg", "NvM", &container("EventStatus", nv, &nv_params, "", "")));
}

fn render_diagnostic(project: &str, diagnostic: &DiagnosticView, frame_count: usize, elements: &mut String, canif_children: &mut String) {
    let request_pdu = global_pdu_path(project, "NPdu_DiagRequest");
    let response_pdu = global_pdu_path(project, "NPdu_DiagResponse");
    let request_sdu = global_pdu_path(project, "DcmPdu_DiagRequest");
    let response_sdu = global_pdu_path(project, "DcmPdu_DiagResponse");
    for name in ["NPdu_DiagRequest", "NPdu_DiagResponse"] {
        write!(elements, "<N-PDU><SHORT-NAME>{name}</SHORT-NAME><LENGTH>8</LENGTH></N-PDU>").unwrap();
    }
    for name in ["DcmPdu_DiagRequest", "DcmPdu_DiagResponse"] {
        write!(elements, "<DCM-I-PDU><SHORT-NAME>{name}</SHORT-NAME><LENGTH>256</LENGTH></DCM-I-PDU>").unwrap();
    }
    for (name, id, is_tx) in [("DiagRequest", diagnostic.request_id, false), ("DiagResponse", diagnostic.response_id, true)] {
        let path = if is_tx { "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfTxPduCfg" } else { "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfRxPduCfg" };
        let params = if is_tx {
            format!("{}{}{}", number(path, "CanIfTxPduCanId", id), number(path, "CanIfTxPduId", frame_count), choice(path, "CanIfTxPduCanIdType", "STANDARD_CAN"))
        } else {
            format!("{}{}{}{}{}", number(path, "CanIfRxPduCanId", id), number(path, "CanIfRxPduId", frame_count), number(path, "CanIfRxPduDataLength", 8), choice(path, "CanIfRxPduCanIdType", "STANDARD_NO_FD_CAN"), boolean(path, "CanIfRxPduDataLengthCheck", false))
        };
        let refs = reference(path, if is_tx { "CanIfTxPduRef" } else { "CanIfRxPduRef" },
            "ECUC-CONTAINER-VALUE", if is_tx { &response_pdu } else { &request_pdu });
        canif_children.push_str(&container(&format!("Can_{name}"), path, &params, &refs, ""));
    }

    let rx = "/AUTOSAR/EcucDefs/CanTp/CanTpConfig/CanTpChannel/CanTpRxNSdu";
    let rx_pdu = format!("{rx}/CanTpRxNPdu");
    let tx_fc = format!("{rx}/CanTpTxFcNPdu");
    let rx_children = container("RequestNPdu", &rx_pdu, &number(&rx_pdu, "CanTpRxNPduId", 0), &reference(&rx_pdu, "CanTpRxNPduRef", "ECUC-CONTAINER-VALUE", &request_pdu), "")
        + &container("ResponseFc", &tx_fc, &number(&tx_fc, "CanTpTxFcNPduConfirmationPduId", 1), &reference(&tx_fc, "CanTpTxFcNPduRef", "ECUC-CONTAINER-VALUE", &response_pdu), "");
    let rx_params = format!("{}{}{}{}{}{}{}{}", number(rx, "CanTpRxNSduId", 0), choice(rx, "CanTpRxAddressingFormat", "CANTP_STANDARD"), choice(rx, "CanTpRxPaddingActivation", "CANTP_OFF"), choice(rx, "CanTpRxTaType", "CANTP_PHYSICAL"), number(rx, "CanTpBs", 0), decimal(rx, "CanTpSTmin", 0), number(rx, "CanTpRxWftMax", 0), decimal(rx, "CanTpNcr", diagnostic.n_cr_ms));
    let rx_sdu = container("Request", rx, &rx_params, &reference(rx, "CanTpRxNSduRef", "ECUC-CONTAINER-VALUE", &request_sdu), &rx_children);
    let tx = "/AUTOSAR/EcucDefs/CanTp/CanTpConfig/CanTpChannel/CanTpTxNSdu";
    let tx_pdu = format!("{tx}/CanTpTxNPdu");
    let rx_fc = format!("{tx}/CanTpRxFcNPdu");
    let tx_children = container("ResponseNPdu", &tx_pdu, &number(&tx_pdu, "CanTpTxNPduConfirmationPduId", 0), &reference(&tx_pdu, "CanTpTxNPduRef", "ECUC-CONTAINER-VALUE", &response_pdu), "")
        + &container("RequestFc", &rx_fc, &number(&rx_fc, "CanTpRxFcNPduId", 1), &reference(&rx_fc, "CanTpRxFcNPduRef", "ECUC-CONTAINER-VALUE", &request_pdu), "");
    let tx_params = format!("{}{}{}{}{}{}{}", number(tx, "CanTpTxNSduId", 0), choice(tx, "CanTpTxAddressingFormat", "CANTP_STANDARD"), choice(tx, "CanTpTxPaddingActivation", "CANTP_OFF"), choice(tx, "CanTpTxTaType", "CANTP_PHYSICAL"), boolean(tx, "CanTpTc", false), decimal(tx, "CanTpNas", diagnostic.n_bs_ms), decimal(tx, "CanTpNbs", diagnostic.n_bs_ms));
    let tx_sdu = container("Response", tx, &tx_params, &reference(tx, "CanTpTxNSduRef", "ECUC-CONTAINER-VALUE", &response_sdu), &tx_children);
    let channel = container("Channel", "/AUTOSAR/EcucDefs/CanTp/CanTpConfig/CanTpChannel", "", "", &(rx_sdu + &tx_sdu));
    let config_path = "/AUTOSAR/EcucDefs/CanTp/CanTpConfig";
    let config = container("CanTpConfig", config_path, &decimal(config_path, "CanTpMainFunctionPeriod", 1), "", &channel);
    let general_path = "/AUTOSAR/EcucDefs/CanTp/CanTpGeneral";
    let mut general_params = String::new();
    for name in ["CanTpChangeParameterApi", "CanTpDevErrorDetect", "CanTpEnableSecurityEventReporting", "CanTpReadParameterApi", "CanTpVersionInfoApi"] {
        general_params.push_str(&boolean(general_path, name, false));
    }
    general_params.push_str(&number(general_path, "CanTpPaddingByte", 0));
    let general = container("CanTpGeneral", general_path, &general_params, "", "");
    elements.push_str(&module("CanTpCfg", "CanTp", &(config + &general)));

    let base = "/AUTOSAR/EcucDefs/Dcm/DcmConfigSet";
    let dsd = format!("{base}/DcmDsd");
    let table = format!("{dsd}/DcmDsdServiceTable");
    let service_path = format!("{table}/DcmDsdService");
    let mut services = String::new();
    for (name, sid, subfunction) in [("SessionControl", 0x10, true), ("ReadDataByIdentifier", 0x22, false), ("TesterPresent", 0x3e, true)] {
        let params = format!("{}{}{}", number(&service_path, "DcmDsdSidTabServiceId", sid), boolean(&service_path, "DcmDsdServiceUsed", true), boolean(&service_path, "DcmDsdSidTabSubfuncAvail", subfunction));
        services.push_str(&container(name, &service_path, &params, "", ""));
    }
    if diagnostic.write_enabled {
        let params = format!("{}{}{}", number(&service_path, "DcmDsdSidTabServiceId", 0x2e),
            boolean(&service_path, "DcmDsdServiceUsed", true),
            boolean(&service_path, "DcmDsdSidTabSubfuncAvail", false));
        services.push_str(&container("WriteDataByIdentifier", &service_path, &params, "", ""));
    }
    if diagnostic.dtc.is_some() {
        for (name, sid, subfunction) in [("ClearDiagnosticInformation", 0x14, false), ("ReadDTCInformation", 0x19, true)] {
            let params = format!("{}{}{}", number(&service_path, "DcmDsdSidTabServiceId", sid),
                boolean(&service_path, "DcmDsdServiceUsed", true),
                boolean(&service_path, "DcmDsdSidTabSubfuncAvail", subfunction));
            let refs = if sid == 0x14 {
                reference(&service_path, "DcmDsdSidTabSessionLevelRef", "ECUC-CONTAINER-VALUE",
                    &ref_path(project, "DcmCfg/DcmConfigSet/DcmDsp/Sessions/Extended"))
            } else { String::new() };
            services.push_str(&container(name, &service_path, &params, &refs, ""));
        }
    }
    let table_value = container("Services", &table, &number(&table, "DcmDsdSidTabId", 0), "", &services);
    let dsd_value = container("DcmDsd", &dsd, "", "", &table_value);

    let dsl = format!("{base}/DcmDsl");
    let buffer = format!("{dsl}/DcmDslBuffer");
    let buffer_value = container("Buffer", &buffer, &number(&buffer, "DcmDslBufferSize", 256), "", "");
    let diag_resp = format!("{dsl}/DcmDslDiagResp");
    let diag_resp_value = container("DiagResp", &diag_resp, &boolean(&diag_resp, "DcmDslDiagRespOnSecondDeclinedRequest", true), "", "");
    let protocol = format!("{dsl}/DcmDslProtocol");
    let row = format!("{protocol}/DcmDslProtocolRow");
    let connection = format!("{row}/DcmDslConnection");
    let main = format!("{connection}/DcmDslMainConnection");
    let dcm_rx = format!("{main}/DcmDslProtocolRx");
    let dcm_tx = format!("{main}/DcmDslProtocolTx");
    let rx_value = container("Request", &dcm_rx, &(choice(&dcm_rx, "DcmDslProtocolRxAddrType", "DCM_PHYSICAL_TYPE") + &number(&dcm_rx, "DcmDslProtocolRxPduId", 0)), &reference(&dcm_rx, "DcmDslProtocolRxPduRef", "ECUC-CONTAINER-VALUE", &request_sdu), "");
    let tx_value = container("Response", &dcm_tx, &number(&dcm_tx, "DcmDslTxConfirmationPduId", 0), &reference(&dcm_tx, "DcmDslProtocolTxPduRef", "ECUC-CONTAINER-VALUE", &response_sdu), "");
    let main_value = container("Main", &main, &number(&main, "DcmDslProtocolRxConnectionId", 0), "", &(rx_value + &tx_value));
    let row_params = format!("{}{}{}{}{}{}", number(&row, "DcmDslProtocolPriority", 0), boolean(&row, "DcmDslProtocolRowUsed", true), choice(&row, "DcmDslProtocolType", "DCM_UDS_ON_CAN"), boolean(&row, "DcmSendRespPendOnRestart", false), decimal(&row, "DcmTimStrP2ServerAdjust", 0), decimal(&row, "DcmTimStrP2StarServerAdjust", 0));
    let buffer_ref = ref_path(project, "DcmCfg/DcmConfigSet/DcmDsl/Buffer");
    let table_ref = ref_path(project, "DcmCfg/DcmConfigSet/DcmDsd/Services");
    let row_refs = reference(&row, "DcmDslProtocolRxBufferRef", "ECUC-CONTAINER-VALUE", &buffer_ref)
        + &reference(&row, "DcmDslProtocolTxBufferRef", "ECUC-CONTAINER-VALUE", &buffer_ref)
        + &reference(&row, "DcmDslProtocolSIDTable", "ECUC-CONTAINER-VALUE", &table_ref);
    let row_value = container("UdsCan", &row, &row_params, &row_refs, &main_value);
    let protocol_value = container("Protocol", &protocol, "", "", &row_value);
    let dsl_value = container("DcmDsl", &dsl, "", "", &(buffer_value + &diag_resp_value + &protocol_value));

    let dsp = format!("{base}/DcmDsp");
    let did_path = format!("{dsp}/DcmDspDid");
    let data_path = format!("{dsp}/DcmDspData");
    let info_path = format!("{dsp}/DcmDspDidInfo");
    let did_signal_path = format!("{did_path}/DcmDspDidSignal");
    let mut data_values = String::new();
    let mut did_signals = String::new();
    for (index, signal_path) in diagnostic.signal_paths.iter().enumerate() {
        let data_name = format!("Data_{index}");
        let mut params = format!("{}{}{}{}", choice(&data_path, "DcmDspDataType", "UINT32"), choice(&data_path, "DcmDspDataUsePort", "USE_DATA_SYNCH_FNC"), choice(&data_path, "DcmDspDataEndianness", "BIG_ENDIAN"), function(&data_path, "DcmDspDataReadFnc", &format!("Ecu_DcmRead_{index}")));
        if diagnostic.write_enabled {
            params.push_str(&number(&data_path, "DcmDspDataByteSize", 4));
            params.push_str(&function(&data_path, "DcmDspDataWriteFnc", &format!("Ecu_DcmWrite_{index}")));
        }
        let data = container(&data_name, &data_path, &params, "", "");
        // DcmDspDidDataRef names DcmDspData, but ECUC has no direct ComSignalRef for it.
        let admin = format!("<ADMIN-DATA><SDGS><SDG GID=\"AutosarWorkbenchDiagnostic\"><SD GID=\"ComSignalRef\">{signal_path}</SD></SDG></SDGS></ADMIN-DATA>");
        data_values.push_str(&data.replacen("</SHORT-NAME>", &format!("</SHORT-NAME>{admin}"), 1));
        let signal_params = number(&did_signal_path, "DcmDspDidByteOffset", index * 4);
        let data_ref = ref_path(project, &format!("DcmCfg/DcmConfigSet/DcmDsp/{data_name}"));
        did_signals.push_str(&container(&format!("Signal_{index}"), &did_signal_path, &signal_params, &reference(&did_signal_path, "DcmDspDidDataRef", "ECUC-CONTAINER-VALUE", &data_ref), ""));
    }
    let info_ref = ref_path(project, "DcmCfg/DcmConfigSet/DcmDsp/DidInfo");
    let did_params = format!("{}{}{}{}", number(&did_path, "DcmDspDidIdentifier", diagnostic.did), number(&did_path, "DcmDspDidSize", diagnostic.signal_paths.len() * 4), boolean(&did_path, "DcmDspDidUsed", true), choice(&did_path, "DcmDspDidUsePort", "USE_DATA_ELEMENT_SPECIFIC_INTERFACES"));
    let mut did_value = container("Did", &did_path, &did_params, &reference(&did_path, "DcmDspDidInfoRef", "ECUC-CONTAINER-VALUE", &info_ref), &did_signals);
    let read_path = format!("{info_path}/DcmDspDidRead");
    let ext_ref = ref_path(project, "DcmCfg/DcmConfigSet/DcmDsp/Sessions/Extended");
    if let Some(rid) = diagnostic.reset_routine_id {
        let admin = format!("<ADMIN-DATA><SDGS><SDG GID=\"AutosarWorkbenchHostRestoreDidV1\"><SD GID=\"Rid\">{rid}</SD><SD GID=\"SessionRef\">{ext_ref}</SD></SDG></SDGS></ADMIN-DATA>");
        did_value = did_value.replacen("</SHORT-NAME>", &format!("</SHORT-NAME>{admin}"), 1);
    }
    let read_value = container("Read", &read_path, "", &reference(&read_path, "DcmDspDidReadSessionRef", "ECUC-CONTAINER-VALUE", &ext_ref), "");
    let mut info_children = read_value;
    if diagnostic.write_enabled {
        let write_path = format!("{info_path}/DcmDspDidWrite");
        info_children.push_str(&container("Write", &write_path, "",
            &reference(&write_path, "DcmDspDidWriteSessionRef", "ECUC-CONTAINER-VALUE", &ext_ref), ""));
    }
    let info_value = container("DidInfo", &info_path, &boolean(&info_path, "DcmDspDidDynamicallyDefined", false), "", &info_children);
    let sessions_path = format!("{dsp}/DcmDspSession");
    let session_path = format!("{sessions_path}/DcmDspSessionRow");
    let session = |name: &str, level: u8| {
        let params = format!("{}{}{}{}", number(&session_path, "DcmDspSessionLevel", level), choice(&session_path, "DcmDspSessionForBoot", "DCM_NO_BOOT"), decimal(&session_path, "DcmDspSessionP2ServerMax", 50), decimal(&session_path, "DcmDspSessionP2StarServerMax", 5000));
        container(name, &session_path, &params, "", "")
    };
    let sessions = container("Sessions", &sessions_path, "", "", &(session("Default", 1) + &session("Extended", 3)));
    let dsp_params = choice(&dsp, "DcmDspDataDefaultEndianness", "BIG_ENDIAN") + &boolean(&dsp, "DcmDspEnableObdMirror", false);
    let mut dsp_children = did_value + &info_value + &data_values + &sessions;
    if diagnostic.dtc.is_some() {
        dsp_children.push_str(&container("ClearDTC", &format!("{dsp}/DcmDspClearDTC"), "", "", ""));
        dsp_children.push_str(&container("ReadDTCInformation", &format!("{dsp}/DcmDspReadDTCInformation"), "", "", ""));
    }
    let dsp_value = container("DcmDsp", &dsp, &dsp_params, "", &dsp_children);
    let page_path = format!("{base}/DcmPageBufferCfg");
    let page_value = container("PageBuffer", &page_path, &boolean(&page_path, "DcmPagedBufferEnabled", false), "", "");
    let config = container("DcmConfigSet", base, "", "", &(dsd_value + &dsl_value + &dsp_value + &page_value));
    let general_path = "/AUTOSAR/EcucDefs/Dcm/DcmGeneral";
    let general_params = format!("{}{}{}{}{}{}", boolean(general_path, "DcmDevErrorDetect", false), boolean(general_path, "DcmEnableSecurityEventReporting", false), boolean(general_path, "DcmRespondAllRequest", false), boolean(general_path, "DcmVersionInfoApi", false), decimal(general_path, "DcmTaskTime", 1), decimal(general_path, "DcmS3ServerTimeoutOverwrite", diagnostic.s3_ms));
    let general = container("DcmGeneral", general_path, &general_params, "", "");
    elements.push_str(&module("DcmCfg", "Dcm", &(config + &general)));
    if diagnostic.dtc.is_some() {
        render_dtc(project, diagnostic, elements);
    }
}

pub fn render_profile(project: &str, frames: &[FrameView], signals: &[SignalView], diagnostic: Option<&DiagnosticView>) -> String {
    let mut elements = String::new();
    let mut com_children = String::new();
    let mut canif_children = String::new();
    for (frame_index, frame) in frames.iter().enumerate() {
        let frame_signals: Vec<_> = signals.iter().filter(|s| s.frame_path == frame.path).collect();
        let mut mappings = String::new();
        let mut com_refs = String::new();
        for signal in &frame_signals {
            write!(mappings, "<I-SIGNAL-TO-I-PDU-MAPPING><SHORT-NAME>Map_{}</SHORT-NAME><I-SIGNAL-REF DEST=\"I-SIGNAL\">{}</I-SIGNAL-REF><PACKING-BYTE-ORDER>MOST-SIGNIFICANT-BYTE-LAST</PACKING-BYTE-ORDER><START-POSITION>{}</START-POSITION></I-SIGNAL-TO-I-PDU-MAPPING>", signal.name, ref_path(project, &format!("ISignal_{}", signal.name)), signal.start_bit).unwrap();
            com_refs.push_str(&reference("/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu", "ComIPduSignalRef", "ECUC-CONTAINER-VALUE", &signal.path));
        }
        write!(elements, "<I-SIGNAL-I-PDU><SHORT-NAME>Pdu_{}</SHORT-NAME><LENGTH>{}</LENGTH>", frame.name, frame.dlc).unwrap();
        if !mappings.is_empty() {
            write!(elements, "<I-SIGNAL-TO-PDU-MAPPINGS>{mappings}</I-SIGNAL-TO-PDU-MAPPINGS>").unwrap();
        }
        elements.push_str("</I-SIGNAL-I-PDU>");
        let com_path = "/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu";
        let com_params = format!("{}{}{}{}", choice(com_path, "ComIPduDirection", match frame.direction { Direction::Tx => "SEND", Direction::Rx => "RECEIVE" }),
            number(com_path, "ComIPduHandleId", frame_index), choice(com_path, "ComIPduSignalProcessing", "IMMEDIATE"), choice(com_path, "ComIPduType", "NORMAL"));
        let com_pdu_ref = reference(com_path, "ComPduIdRef", "ECUC-CONTAINER-VALUE", &global_pdu_path(project, &format!("Pdu_{}", frame.name)));
        let tx_children = if let Some(period) = frame.period_ms {
            let tx_path = format!("{com_path}/ComTxIPdu");
            let mode_path = format!("{tx_path}/ComTxModeTrue/ComTxMode");
            let mode = container("Mode", &mode_path, &format!("{}{}", choice(&mode_path, "ComTxModeMode", "PERIODIC"), decimal(&mode_path, "ComTxModeTimePeriod", period)), "", "");
            let true_mode = container("TrueMode", &format!("{tx_path}/ComTxModeTrue"), "", "", &mode);
            container("Tx", &tx_path, &number(&tx_path, "ComTxIPduUnusedAreasDefault", 0), "", &true_mode)
        } else { String::new() };
        com_children.push_str(&container(&format!("Pdu_{}", frame.name), com_path, &com_params, &(com_pdu_ref + &com_refs), &tx_children));
        let (canif_def, canif_params, canif_ref) = match frame.direction {
            Direction::Tx => {
                let path = "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfTxPduCfg";
                (path, format!("{}{}{}", number(path, "CanIfTxPduCanId", frame.id), number(path, "CanIfTxPduId", frame_index), choice(path, "CanIfTxPduCanIdType", "STANDARD_CAN")), reference(path, "CanIfTxPduRef", "ECUC-CONTAINER-VALUE", &global_pdu_path(project, &format!("Pdu_{}", frame.name))))
            }
            Direction::Rx => {
                let path = "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfRxPduCfg";
                (path, format!("{}{}{}", number(path, "CanIfRxPduCanId", frame.id), number(path, "CanIfRxPduId", frame_index), number(path, "CanIfRxPduDataLength", frame.dlc)), reference(path, "CanIfRxPduRef", "ECUC-CONTAINER-VALUE", &global_pdu_path(project, &format!("Pdu_{}", frame.name))))
            }
        };
        canif_children.push_str(&container(&format!("Can_{}", frame.name), canif_def, &canif_params, &canif_ref, ""));
    }
    for (signal_index, signal) in signals.iter().enumerate() {
        write!(elements, "<I-SIGNAL><SHORT-NAME>ISignal_{}</SHORT-NAME><LENGTH>{}</LENGTH></I-SIGNAL>", signal.name, signal.length).unwrap();
        let path = "/AUTOSAR/EcucDefs/Com/ComConfig/ComSignal";
        let ty = if signal.length == 1 { "BOOLEAN" } else if signal.length <= 8 { "UINT8" } else if signal.length <= 16 { "UINT16" } else { "UINT32" };
        let mut params = format!("{}{}{}{}{}{}", number(path, "ComBitPosition", signal.start_bit), number(path, "ComBitSize", signal.length), number(path, "ComHandleId", signal_index), choice(path, "ComSignalEndianness", "LITTLE_ENDIAN"), text(path, "ComSignalInitValue", signal.initial_value), choice(path, "ComSignalType", ty));
        if let Some(frame) = frames.iter().find(|f| f.path == signal.frame_path) {
            if let Some(timeout) = frame.timeout_ms {
                params.push_str(&decimal(path, "ComTimeout", timeout));
            }
        }
        com_children.push_str(&container(&signal.name, path, &params, "", ""));
    }
    if let Some(diagnostic) = diagnostic {
        render_diagnostic(project, diagnostic, frames.len(), &mut elements, &mut canif_children);
    }
    elements.push_str(&render_global_pdus(project, frames, diagnostic));
    if !com_children.is_empty() {
        let config = container("ComConfig", "/AUTOSAR/EcucDefs/Com/ComConfig", "", "", &com_children);
        let general_path = "/AUTOSAR/EcucDefs/Com/ComGeneral";
        let general_params = format!("{}{}{}{}", boolean(general_path, "ComEnableSecurityEventReporting", false),
            boolean(general_path, "ComEnableSignalGroupArrayApi", false), number(general_path, "ComSupportedIPduGroups", 0),
            boolean(general_path, "ComVersionInfoApi", false));
        let general = container("ComGeneral", general_path, &general_params, "", "");
        elements.push_str(&module("ComCfg", "Com", &(config + &general)));
    }
    if !canif_children.is_empty() {
        let config = container("CanIfInitCfg", "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg", "", "", &canif_children);
        elements.push_str(&module("CanIfCfg", "CanIf", &config));
    }
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>{project}</SHORT-NAME><ELEMENTS>{elements}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>\n")
}
