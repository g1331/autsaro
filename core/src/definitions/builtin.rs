//! Authored product configuration vocabulary. This is deliberately not an XML
//! conversion or a runtime dependency on the official MOD archive. Numeric bounds
//! remain text so the unsigned 64-bit OS domain survives every consumer unchanged.
use super::{DefinitionCatalog, descriptor};
use crate::project_model::{TypedValue, ValueKind};

const ROOT: &str = "/AUTOSAR/EcucDefs/";
const U8: &str = "255";
const U16: &str = "65535";
const U32: &str = "4294967295";
const U64: &str = "18446744073709551615";
const PDU: &str = "EcuC/EcucConfigSet/EcucPduCollection/Pdu";

struct Author<'a>(&'a mut DefinitionCatalog);
impl Author<'_> {
    fn container(
        &mut self,
        path: &str,
        lower: u32,
        upper: Option<u32>,
    ) -> Result<(), crate::message::LocalizedText> {
        let id = format!("{ROOT}{path}");
        if let Some((parent, _)) = path.rsplit_once('/') {
            if self.0.get(&format!("{ROOT}{parent}")).is_none() {
                self.container(parent, 0, None)?;
            }
        }
        let element = if path.contains('/') {
            "ECUC-PARAM-CONF-CONTAINER-DEF"
        } else {
            "ECUC-MODULE-DEF"
        };
        let entry = self
            .0
            .entries_mut()
            .entry(id.clone())
            .or_insert_with(|| descriptor(id, element, None));
        entry.lower_multiplicity = lower;
        entry.upper_multiplicity = upper;
        Ok(())
    }

    fn choice(
        &mut self,
        path: &str,
        lower: u32,
        upper: Option<u32>,
    ) -> Result<(), crate::message::LocalizedText> {
        self.container(path, lower, upper)?;
        self.0
            .entries_mut()
            .get_mut(&format!("{ROOT}{path}"))
            .unwrap()
            .element_kind = "ECUC-CHOICE-CONTAINER-DEF".into();
        Ok(())
    }

    fn field(
        &mut self,
        path: &str,
        name: &str,
        kind: ValueKind,
        lower: u32,
    ) -> Result<(), crate::message::LocalizedText> {
        let tag = match kind {
            ValueKind::Integer => "ECUC-INTEGER-PARAM-DEF",
            ValueKind::Float => "ECUC-FLOAT-PARAM-DEF",
            ValueKind::Boolean => "ECUC-BOOLEAN-PARAM-DEF",
            ValueKind::Enumeration => "ECUC-ENUMERATION-PARAM-DEF",
            ValueKind::String => "ECUC-STRING-PARAM-DEF",
            ValueKind::FunctionName => "ECUC-FUNCTION-NAME-DEF",
            ValueKind::Reference => "ECUC-REFERENCE-DEF",
        };
        if self.0.get(&format!("{ROOT}{path}")).is_none() {
            self.container(path, 0, None)?;
        }
        let mut entry = descriptor(format!("{ROOT}{path}/{name}"), tag, Some(kind));
        entry.lower_multiplicity = lower;
        self.0.insert(entry, Vec::new())
    }

    fn booleans(
        &mut self,
        path: &str,
        names: &str,
        lower: u32,
    ) -> Result<(), crate::message::LocalizedText> {
        for name in names.split_whitespace() {
            self.field(path, name, ValueKind::Boolean, lower)?;
        }
        Ok(())
    }

    fn integers(
        &mut self,
        path: &str,
        names: &str,
        minimum: &str,
        maximum: &str,
        lower: u32,
    ) -> Result<(), crate::message::LocalizedText> {
        for name in names.split_whitespace() {
            self.field(path, name, ValueKind::Integer, lower)?;
            let entry = self
                .0
                .entries_mut()
                .get_mut(&format!("{ROOT}{path}/{name}"))
                .unwrap();
            entry.minimum = Some(minimum.into());
            entry.maximum = Some(maximum.into());
        }
        Ok(())
    }

    fn floats(
        &mut self,
        path: &str,
        names: &str,
        maximum: Option<&str>,
        lower: u32,
    ) -> Result<(), crate::message::LocalizedText> {
        for name in names.split_whitespace() {
            self.field(path, name, ValueKind::Float, lower)?;
            let entry = self
                .0
                .entries_mut()
                .get_mut(&format!("{ROOT}{path}/{name}"))
                .unwrap();
            entry.minimum = Some("0".into());
            entry.maximum = maximum.map(str::to_string);
            entry.unit = Some("s".into());
        }
        Ok(())
    }

    fn enumeration(
        &mut self,
        path: &str,
        names: &str,
        literals: &str,
        lower: u32,
    ) -> Result<(), crate::message::LocalizedText> {
        for name in names.split_whitespace() {
            self.field(path, name, ValueKind::Enumeration, lower)?;
            self.0
                .entries_mut()
                .get_mut(&format!("{ROOT}{path}/{name}"))
                .unwrap()
                .enumeration = literals.split_whitespace().map(str::to_string).collect();
        }
        Ok(())
    }

    fn reference(
        &mut self,
        path: &str,
        name: &str,
        target: &str,
        lower: u32,
        upper: Option<u32>,
    ) -> Result<(), crate::message::LocalizedText> {
        self.field(path, name, ValueKind::Reference, lower)?;
        let id = format!("{ROOT}{path}/{name}");
        let entry = self.0.entries_mut().get_mut(&id).unwrap();
        entry.upper_multiplicity = upper;
        entry.reference_destinations = vec!["ECUC-CONTAINER-VALUE".into()];
        self.0.targets_mut().insert(
            id,
            target
                .split_whitespace()
                .map(|path| format!("{ROOT}{path}"))
                .collect(),
        );
        Ok(())
    }

    fn foreign(
        &mut self,
        path: &str,
        name: &str,
        destinations: &str,
        lower: u32,
        upper: Option<u32>,
    ) -> Result<(), crate::message::LocalizedText> {
        self.field(path, name, ValueKind::Reference, lower)?;
        let entry = self
            .0
            .entries_mut()
            .get_mut(&format!("{ROOT}{path}/{name}"))
            .unwrap();
        entry.element_kind = "ECUC-FOREIGN-REFERENCE-DEF".into();
        entry.upper_multiplicity = upper;
        entry.reference_destinations = destinations
            .split_whitespace()
            .map(str::to_string)
            .collect();
        Ok(())
    }

    fn default(&mut self, path: &str, name: &str, lexeme: &str) {
        let entry = self
            .0
            .entries_mut()
            .get_mut(&format!("{ROOT}{path}/{name}"))
            .unwrap();
        entry.default_value = Some(TypedValue {
            kind: entry.kind.unwrap(),
            lexeme: lexeme.into(),
        });
        entry.default_origin = Some("builtin:product-ecuc-r24-11-v1".into());
    }
}

pub(super) fn populate(
    catalog: &mut DefinitionCatalog,
) -> Result<(), crate::message::LocalizedText> {
    let mut a = Author(catalog);
    for module in [
        "Can", "CanIf", "CanTp", "Com", "Dcm", "EcuC", "Os", "PduR", "Rte", "Mcu", "Dem", "NvM",
    ] {
        a.container(module, 0, if module == "Can" { None } else { Some(1) })?;
    }
    communication(&mut a)?;
    diagnostic(&mut a)?;
    operating_system(&mut a)?;
    persistence(&mut a)?;
    defaults(&mut a);
    Ok(())
}

fn communication(a: &mut Author<'_>) -> Result<(), crate::message::LocalizedText> {
    a.container("EcuC/EcucConfigSet", 0, Some(1))?;
    a.container("EcuC/EcucConfigSet/EcucPduCollection", 0, Some(1))?;
    a.container(PDU, 0, None)?;
    a.integers(PDU, "PduLength", "0", U32, 1)?;
    a.booleans(PDU, "DynamicLength", 0)?;
    let collection = "EcuC/EcucConfigSet/EcucPduCollection";
    a.enumeration(collection, "PduIdTypeEnum", "UINT8 UINT16", 1)?;
    a.enumeration(collection, "PduLengthTypeEnum", "UINT8 UINT16 UINT32", 1)?;
    a.container("EcuC/EcucHardware", 1, Some(1))?;
    a.container("EcuC/EcucHardware/EcucCoreDefinition", 1, None)?;
    a.integers(
        "EcuC/EcucHardware/EcucCoreDefinition",
        "EcucCoreId",
        "0",
        U16,
        1,
    )?;
    a.container("Can/CanConfigSet", 1, Some(1))?;
    let controller = "Can/CanConfigSet/CanController";
    a.container(controller, 1, None)?;
    a.booleans(
        controller,
        "CanControllerActivation CanHwPnSupport CanWakeupSupport",
        1,
    )?;
    a.integers(controller, "CanControllerId", "0", U8, 1)?;
    a.integers(controller, "CanControllerBaseAddress", "0", U32, 1)?;
    a.enumeration(
        controller,
        "CanBusoffProcessing CanWakeupProcessing",
        "INTERRUPT POLLING",
        1,
    )?;
    a.enumeration(
        controller,
        "CanRxProcessing CanTxProcessing",
        "INTERRUPT MIXED POLLING",
        1,
    )?;
    let baud = "Can/CanConfigSet/CanController/CanControllerBaudrateConfig";
    a.container(baud, 1, None)?;
    a.floats(baud, "CanControllerBaudRate", Some("2000"), 1)?;
    a.0.entries_mut()
        .get_mut(&format!("{ROOT}{baud}/CanControllerBaudRate"))
        .unwrap()
        .unit = Some("kbit/s".into());
    a.integers(baud, "CanControllerBaudRateConfigID", "0", U16, 1)?;
    a.integers(baud, "CanControllerPropSeg", "0", "384", 1)?;
    a.integers(
        baud,
        "CanControllerSeg1 CanControllerSeg2 CanControllerSyncJumpWidth",
        "0",
        U8,
        1,
    )?;
    a.reference(controller, "CanControllerDefaultBaudrate", baud, 1, Some(1))?;
    a.reference(
        controller,
        "CanCpuClockRef",
        "Mcu/McuModuleConfiguration/McuClockSettingConfig/McuClockReferencePoint",
        1,
        Some(1),
    )?;
    let hardware = "Can/CanConfigSet/CanHardwareObject";
    a.container(hardware, 0, None)?;
    a.integers(hardware, "CanObjectId", "0", U16, 1)?;
    a.integers(hardware, "CanHwObjectCount", "1", U16, 1)?;
    a.enumeration(hardware, "CanHandleType", "BASIC FULL", 1)?;
    a.enumeration(hardware, "CanIdType", "EXTENDED MIXED STANDARD", 1)?;
    a.enumeration(hardware, "CanObjectType", "RECEIVE TRANSMIT", 1)?;
    a.enumeration(hardware, "CanObjectPayloadLength", "CAN_OBJECT_PL_8 CAN_OBJECT_PL_12 CAN_OBJECT_PL_16 CAN_OBJECT_PL_20 CAN_OBJECT_PL_24 CAN_OBJECT_PL_32 CAN_OBJECT_PL_48 CAN_OBJECT_PL_64", 0)?;
    a.reference(hardware, "CanControllerRef", controller, 1, Some(1))?;
    a.booleans(hardware, "CanTriggerTransmitEnable", 0)?;
    a.default(hardware, "CanTriggerTransmitEnable", "false");
    a.reference(
        hardware,
        "CanMainFunctionRWPeriodRef",
        "Can/CanGeneral/CanMainFunctionRWPeriods",
        0,
        Some(1),
    )?;
    let general = "Can/CanGeneral";
    a.container(general, 1, Some(1))?;
    a.booleans(general, "CanDevErrorDetect CanEnableSecurityEventReporting CanGlobalTimeSupport CanMultiplexedTransmission CanVersionInfoApi", 1)?;
    a.integers(general, "CanIndex", "0", U8, 1)?;
    a.floats(general, "CanMainFunctionModePeriod", None, 1)?;
    a.floats(general, "CanMainFunctionBusoffPeriod", None, 0)?;
    let rw = "Can/CanGeneral/CanMainFunctionRWPeriods";
    a.container(rw, 0, None)?;
    a.floats(rw, "CanMainFunctionPeriod", None, 1)?;
    a.floats(general, "CanTimeoutDuration", Some("65.535"), 1)?;
    a.0.entries_mut()
        .get_mut(&format!("{ROOT}{general}/CanTimeoutDuration"))
        .unwrap()
        .minimum = Some("1E-6".into());
    a.reference(
        general,
        "CanSupportTTCANRef",
        "CanIf/CanIfPrivateCfg",
        1,
        Some(1),
    )?;
    a.default(general, "CanDevErrorDetect", "false");

    let ctrl = "CanIf/CanIfCtrlDrvCfg/CanIfCtrlCfg";
    a.container("CanIf/CanIfCtrlDrvCfg", 1, None)?;
    a.container(ctrl, 1, None)?;
    a.integers(ctrl, "CanIfCtrlId", "0", U8, 1)?;
    a.booleans(ctrl, "CanIfCtrlWakeupSupport", 1)?;
    a.reference(ctrl, "CanIfCtrlCanCtrlRef", controller, 1, Some(1))?;
    a.reference(
        "CanIf/CanIfCtrlDrvCfg",
        "CanIfCtrlDrvInitHohConfigRef",
        "CanIf/CanIfInitCfg/CanIfInitHohCfg",
        1,
        Some(1),
    )?;
    a.reference(
        "CanIf/CanIfCtrlDrvCfg",
        "CanIfCtrlDrvNameRef",
        general,
        1,
        Some(1),
    )?;
    a.container("CanIf/CanIfDispatchCfg", 1, Some(1))?;
    a.enumeration(
        "CanIf/CanIfDispatchCfg",
        "CanIfDispatchUserCtrlBusOffUL CanIfDispatchUserCtrlModeIndicationUL",
        "CAN_SM CDD",
        1,
    )?;
    let init = "CanIf/CanIfInitCfg";
    a.container(init, 1, Some(1))?;
    a.field(init, "CanIfInitCfgSet", ValueKind::String, 1)?;
    let hoh = "CanIf/CanIfInitCfg/CanIfInitHohCfg";
    a.container(hoh, 0, None)?;
    let hrh = "CanIf/CanIfInitCfg/CanIfInitHohCfg/CanIfHrhCfg";
    let hth = "CanIf/CanIfInitCfg/CanIfInitHohCfg/CanIfHthCfg";
    for (path, prefix) in [(hrh, "CanIfHrh"), (hth, "CanIfHth")] {
        a.container(path, 0, None)?;
        a.reference(path, &format!("{prefix}CanCtrlIdRef"), ctrl, 1, Some(1))?;
        a.reference(path, &format!("{prefix}IdSymRef"), hardware, 1, Some(1))?;
        a.0.entries_mut()
            .get_mut(&format!("{ROOT}{path}/{prefix}IdSymRef"))
            .unwrap()
            .element_kind = "ECUC-CHOICE-REFERENCE-DEF".into();
    }
    a.booleans(hrh, "CanIfHrhSoftwareFilter", 1)?;
    let buffer = "CanIf/CanIfInitCfg/CanIfBufferCfg";
    a.container(buffer, 0, None)?;
    a.integers(buffer, "CanIfBufferSize", "0", U8, 1)?;
    a.reference(buffer, "CanIfBufferHthRef", hth, 1, Some(1))?;
    let rx = "CanIf/CanIfInitCfg/CanIfRxPduCfg";
    let tx = "CanIf/CanIfInitCfg/CanIfTxPduCfg";
    for (path, prefix) in [(rx, "CanIfRxPdu"), (tx, "CanIfTxPdu")] {
        a.container(path, 0, None)?;
        a.integers(path, &format!("{prefix}CanId"), "0", "536870911", 0)?;
        a.integers(path, &format!("{prefix}Id"), "0", U32, 1)?;
        a.reference(path, &format!("{prefix}Ref"), PDU, 1, Some(1))?;
    }
    a.enumeration(rx, "CanIfRxPduCanIdType", "EXTENDED_CAN EXTENDED_FD_CAN EXTENDED_NO_FD_CAN STANDARD_CAN STANDARD_FD_CAN STANDARD_NO_FD_CAN", 0)?;
    a.enumeration(
        tx,
        "CanIfTxPduCanIdType",
        "EXTENDED_CAN EXTENDED_FD_CAN STANDARD_CAN STANDARD_FD_CAN",
        0,
    )?;
    a.integers(rx, "CanIfRxPduDataLength", "0", "2048", 1)?;
    a.booleans(
        rx,
        "CanIfRxPduDataLengthCheck CanIfRxPduReadData CanIfRxPduReadNotifyStatus",
        1,
    )?;
    a.booleans(tx, "CanIfTxPduReadNotifyStatus CanIfTxPduTruncation", 1)?;
    a.enumeration(tx, "CanIfTxPduType", "DYNAMIC STATIC", 1)?;
    a.reference(rx, "CanIfRxPduHrhIdRef", hrh, 1, Some(1))?;
    a.reference(tx, "CanIfTxPduBufferRef", buffer, 1, Some(1))?;
    let private = "CanIf/CanIfPrivateCfg";
    a.container(private, 1, Some(1))?;
    a.booleans(
        private,
        "CanIfFixedBuffer CanIfPrivateDataLengthCheck CanIfSupportTTCAN",
        1,
    )?;
    a.enumeration(
        private,
        "CanIfPrivateSoftwareFilterType",
        "BINARY INDEX LINEAR TABLE",
        1,
    )?;
    let public = "CanIf/CanIfPublicCfg";
    a.container(public, 1, Some(1))?;
    a.booleans(public, "CanIfBusMirroringSupport CanIfDevErrorDetect CanIfEnableSecurityEventReporting CanIfGlobalTimeSupport CanIfPublicCtrlPnEnable CanIfPublicMultipleDrvSupport CanIfPublicPnSupport CanIfPublicReadRxPduDataApi CanIfPublicReadRxPduNotifyStatusApi CanIfPublicReadTxPduNotifyStatusApi CanIfPublicSetDynamicTxIdApi CanIfPublicTrcvPnEnable CanIfPublicTxBuffering CanIfPublicTxConfirmPollingSupport CanIfPublicWakeupCheckValidSupport CanIfTriggerTransmitSupport CanIfTxOfflineActiveSupport CanIfVersionInfoApi CanIfWakeupSupport", 1)?;
    a.enumeration(public, "CanIfPublicHandleTypeEnum", "UINT8 UINT16", 1)?;

    a.container("CanTp/CanTpConfig", 1, Some(1))?;
    a.floats("CanTp/CanTpConfig", "CanTpMainFunctionPeriod", None, 1)?;
    a.container("CanTp/CanTpConfig/CanTpChannel", 1, None)?;
    for (path, prefix) in [
        ("CanTp/CanTpConfig/CanTpChannel/CanTpRxNSdu", "CanTpRx"),
        ("CanTp/CanTpConfig/CanTpChannel/CanTpTxNSdu", "CanTpTx"),
    ] {
        a.container(path, 0, None)?;
        a.integers(path, &format!("{prefix}NSduId"), "0", U16, 1)?;
        a.enumeration(
            path,
            &format!("{prefix}AddressingFormat"),
            "CANTP_EXTENDED CANTP_MIXED CANTP_MIXED29BIT CANTP_NORMALFIXED CANTP_STANDARD",
            1,
        )?;
        a.enumeration(
            path,
            &format!("{prefix}PaddingActivation"),
            "CANTP_OFF CANTP_ON",
            1,
        )?;
        a.enumeration(
            path,
            &format!("{prefix}TaType"),
            "CANTP_FUNCTIONAL CANTP_PHYSICAL",
            1,
        )?;
        a.reference(path, &format!("{prefix}NSduRef"), PDU, 1, Some(1))?;
    }
    let rx = "CanTp/CanTpConfig/CanTpChannel/CanTpRxNSdu";
    let tx = "CanTp/CanTpConfig/CanTpChannel/CanTpTxNSdu";
    a.integers(rx, "CanTpBs", "0", U8, 0)?;
    a.integers(rx, "CanTpRxWftMax", "0", U16, 0)?;
    a.floats(rx, "CanTpNar CanTpNbr CanTpNcr CanTpSTmin", None, 0)?;
    a.floats(tx, "CanTpNas", None, 1)?;
    a.floats(tx, "CanTpNbs CanTpNcs", None, 0)?;
    a.booleans(tx, "CanTpTc", 1)?;
    for (path, name, handle, lower) in [
        (
            format!("{rx}/CanTpRxNPdu"),
            "CanTpRxNPduRef",
            "CanTpRxNPduId",
            1,
        ),
        (
            format!("{rx}/CanTpTxFcNPdu"),
            "CanTpTxFcNPduRef",
            "CanTpTxFcNPduConfirmationPduId",
            0,
        ),
        (
            format!("{tx}/CanTpTxNPdu"),
            "CanTpTxNPduRef",
            "CanTpTxNPduConfirmationPduId",
            1,
        ),
        (
            format!("{tx}/CanTpRxFcNPdu"),
            "CanTpRxFcNPduRef",
            "CanTpRxFcNPduId",
            0,
        ),
    ] {
        a.container(&path, lower, Some(1))?;
        a.integers(&path, handle, "0", U16, 1)?;
        a.reference(&path, name, PDU, 1, Some(1))?;
    }
    let general = "CanTp/CanTpGeneral";
    a.container(general, 1, Some(1))?;
    a.booleans(general, "CanTpChangeParameterApi CanTpDevErrorDetect CanTpEnableSecurityEventReporting CanTpReadParameterApi CanTpVersionInfoApi", 1)?;
    a.integers(general, "CanTpPaddingByte", "0", U8, 1)?;

    a.container("Com/ComConfig", 1, Some(1))?;
    let signal = "Com/ComConfig/ComSignal";
    a.container(signal, 0, None)?;
    a.integers(signal, "ComBitPosition", "0", U32, 1)?;
    a.integers(signal, "ComBitSize", "0", "64", 0)?;
    a.integers(signal, "ComHandleId", "0", U16, 0)?;
    a.enumeration(signal, "ComSignalType", "BOOLEAN FLOAT32 FLOAT64 SINT8 SINT16 SINT32 SINT64 UINT8 UINT16 UINT32 UINT64 UINT8_DYN UINT8_N", 1)?;
    a.enumeration(
        signal,
        "ComSignalEndianness",
        "BIG_ENDIAN LITTLE_ENDIAN OPAQUE",
        1,
    )?;
    a.enumeration(signal, "ComTransferProperty", "PENDING TRIGGERED TRIGGERED_ON_CHANGE TRIGGERED_ON_CHANGE_WITHOUT_REPETITION TRIGGERED_WITHOUT_REPETITION", 0)?;
    a.enumeration(
        signal,
        "ComRxDataTimeoutAction",
        "NONE REPLACE SUBSTITUTE",
        0,
    )?;
    a.field(signal, "ComSignalInitValue", ValueKind::String, 0)?;
    a.floats(signal, "ComTimeout ComFirstTimeout", Some("3600"), 0)?;
    a.foreign(
        signal,
        "ComSystemTemplateSystemSignalRef",
        "I-SIGNAL-TO-I-PDU-MAPPING",
        0,
        Some(1),
    )?;
    let pdu = "Com/ComConfig/ComIPdu";
    a.container(pdu, 0, None)?;
    a.enumeration(pdu, "ComIPduDirection", "RECEIVE SEND", 1)?;
    a.enumeration(pdu, "ComIPduSignalProcessing", "DEFERRED IMMEDIATE", 1)?;
    a.enumeration(pdu, "ComIPduType", "NORMAL TP", 1)?;
    a.integers(pdu, "ComIPduHandleId", "0", U16, 0)?;
    a.reference(pdu, "ComIPduSignalRef", signal, 0, None)?;
    let group = "Com/ComConfig/ComIPduGroup";
    a.container(group, 0, None)?;
    a.integers(group, "ComIPduGroupHandleId", "0", U16, 1)?;
    a.reference(group, "ComIPduGroupGroupRef", group, 0, None)?;
    a.reference(pdu, "ComIPduGroupRef", group, 0, None)?;
    for (kind, timebase) in [("Rx", "ComMainRxTimeBase"), ("Tx", "ComMainTxTimeBase")] {
        let path = format!("Com/ComConfig/ComMainFunction{kind}");
        a.container(&path, 0, None)?;
        a.floats(&path, timebase, None, 1)?;
    }
    a.reference(
        pdu,
        "ComIPduMainFunctionRef",
        "Com/ComConfig/ComMainFunctionRx Com/ComConfig/ComMainFunctionTx",
        0,
        Some(1),
    )?;
    a.0.entries_mut()
        .get_mut(&format!("{ROOT}{pdu}/ComIPduMainFunctionRef"))
        .unwrap()
        .element_kind = "ECUC-CHOICE-REFERENCE-DEF".into();
    a.reference(pdu, "ComPduIdRef", PDU, 1, Some(1))?;
    let tx = "Com/ComConfig/ComIPdu/ComTxIPdu";
    a.container(tx, 0, Some(1))?;
    a.integers(tx, "ComTxIPduUnusedAreasDefault", "0", U8, 1)?;
    for branch in ["ComTxModeTrue", "ComTxModeFalse"] {
        let path = format!("{tx}/{branch}");
        a.container(&path, 0, Some(1))?;
        let mode = format!("{path}/ComTxMode");
        a.container(&mode, 1, Some(1))?;
        a.enumeration(&mode, "ComTxModeMode", "DIRECT MIXED NONE PERIODIC", 1)?;
        a.floats(&mode, "ComTxModeTimePeriod", Some("3600"), 0)?;
    }
    let general = "Com/ComGeneral";
    a.container(general, 1, Some(1))?;
    a.booleans(
        general,
        "ComEnableSecurityEventReporting ComEnableSignalGroupArrayApi ComVersionInfoApi",
        1,
    )?;
    a.integers(general, "ComSupportedIPduGroups", "0", U16, 1)?;
    Ok(())
}

fn diagnostic(a: &mut Author<'_>) -> Result<(), crate::message::LocalizedText> {
    let base = "Dcm/DcmConfigSet";
    a.container(base, 1, Some(1))?;
    let dsd = format!("{base}/DcmDsd");
    a.container(&dsd, 1, Some(1))?;
    let table = format!("{dsd}/DcmDsdServiceTable");
    a.container(&table, 1, Some(256))?;
    a.integers(&table, "DcmDsdSidTabId", "0", U8, 1)?;
    let service = format!("{table}/DcmDsdService");
    a.container(&service, 1, None)?;
    a.integers(&service, "DcmDsdSidTabServiceId", "0", U8, 1)?;
    a.booleans(&service, "DcmDsdServiceUsed DcmDsdSidTabSubfuncAvail", 1)?;
    let session = format!("{base}/DcmDsp/DcmDspSession/DcmDspSessionRow");
    let security = format!("{base}/DcmDsp/DcmDspSecurity/DcmDspSecurityRow");
    a.reference(&service, "DcmDsdSidTabSessionLevelRef", &session, 0, None)?;
    a.reference(&service, "DcmDsdSidTabSecurityLevelRef", &security, 0, None)?;
    let subservice = format!("{service}/DcmDsdSubService");
    a.container(&subservice, 0, None)?;
    a.integers(&subservice, "DcmDsdSubServiceId", "0", "127", 1)?;
    a.booleans(&subservice, "DcmDsdSubServiceUsed", 1)?;
    let dsl = format!("{base}/DcmDsl");
    a.container(&dsl, 1, Some(1))?;
    let buffer = format!("{dsl}/DcmDslBuffer");
    a.container(&buffer, 1, Some(256))?;
    a.integers(&buffer, "DcmDslBufferSize", "8", "4294967294", 1)?;
    let diag_resp = format!("{dsl}/DcmDslDiagResp");
    a.container(&diag_resp, 1, Some(1))?;
    a.booleans(&diag_resp, "DcmDslDiagRespOnSecondDeclinedRequest", 1)?;
    let protocol = format!("{dsl}/DcmDslProtocol");
    a.container(&protocol, 1, Some(1))?;
    let row = format!("{protocol}/DcmDslProtocolRow");
    a.container(&row, 1, None)?;
    a.integers(&row, "DcmDslProtocolPriority", "0", U8, 1)?;
    a.booleans(&row, "DcmDslProtocolRowUsed DcmSendRespPendOnRestart", 1)?;
    a.enumeration(&row, "DcmDslProtocolType", "DCM_OBD_ON_CAN DCM_OBD_ON_FLEXRAY DCM_OBD_ON_IP DCM_PERIODICTRANS_ON_CAN DCM_PERIODICTRANS_ON_FLEXRAY DCM_PERIODICTRANS_ON_IP DCM_ROE_ON_CAN DCM_ROE_ON_FLEXRAY DCM_ROE_ON_IP DCM_UDS_ON_CAN DCM_UDS_ON_FLEXRAY DCM_UDS_ON_IP DCM_UDS_ON_LIN DCM_SUPPLIER_1 DCM_SUPPLIER_2 DCM_SUPPLIER_3 DCM_SUPPLIER_4 DCM_SUPPLIER_5 DCM_SUPPLIER_6 DCM_SUPPLIER_7 DCM_SUPPLIER_8 DCM_SUPPLIER_9 DCM_SUPPLIER_10 DCM_SUPPLIER_11 DCM_SUPPLIER_12 DCM_SUPPLIER_13 DCM_SUPPLIER_14 DCM_SUPPLIER_15", 1)?;
    a.floats(&row, "DcmTimStrP2ServerAdjust", Some("1"), 1)?;
    a.floats(&row, "DcmTimStrP2StarServerAdjust", Some("5"), 1)?;
    a.reference(&row, "DcmDslProtocolRxBufferRef", &buffer, 1, Some(1))?;
    a.reference(&row, "DcmDslProtocolTxBufferRef", &buffer, 1, Some(1))?;
    a.reference(&row, "DcmDslProtocolSIDTable", &table, 1, Some(1))?;
    a.reference(
        &row,
        "DcmDemClientRef",
        "Dem/DemGeneral/DemClient",
        1,
        Some(1),
    )?;
    let connection = format!("{row}/DcmDslConnection");
    a.choice(&connection, 1, None)?;
    let main = format!("{connection}/DcmDslMainConnection");
    a.container(&main, 0, Some(1))?;
    a.integers(&main, "DcmDslProtocolRxConnectionId", "0", U16, 1)?;
    let rx = format!("{main}/DcmDslProtocolRx");
    a.container(&rx, 1, None)?;
    a.enumeration(
        &rx,
        "DcmDslProtocolRxAddrType",
        "DCM_FUNCTIONAL_TYPE DCM_PHYSICAL_TYPE",
        1,
    )?;
    a.integers(&rx, "DcmDslProtocolRxPduId", "0", U16, 1)?;
    a.reference(&rx, "DcmDslProtocolRxPduRef", PDU, 1, Some(1))?;
    let tx = format!("{main}/DcmDslProtocolTx");
    a.container(&tx, 0, Some(1))?;
    a.integers(&tx, "DcmDslTxConfirmationPduId", "0", U16, 1)?;
    a.reference(&tx, "DcmDslProtocolTxPduRef", PDU, 1, Some(1))?;
    let dsp = format!("{base}/DcmDsp");
    a.container(&dsp, 0, Some(1))?;
    a.enumeration(
        &dsp,
        "DcmDspDataDefaultEndianness",
        "BIG_ENDIAN LITTLE_ENDIAN OPAQUE",
        1,
    )?;
    a.booleans(&dsp, "DcmDspEnableObdMirror", 1)?;
    a.integers(&dsp, "DcmDspMaxDidToRead", "1", U16, 0)?;
    let data = format!("{dsp}/DcmDspData");
    a.container(&data, 0, None)?;
    a.integers(&data, "DcmDspDataByteSize", "0", U16, 0)?;
    a.booleans(&data, "DcmDspDataConditionCheckReadFncUsed", 0)?;
    a.enumeration(&data, "DcmDspDataType", "BOOLEAN FLOAT FLOAT_N SINT8 SINT8_N SINT16 SINT16_N SINT32 SINT32_N UINT8 UINT8_N UINT8_DYN UINT16 UINT16_N UINT32 UINT32_N", 1)?;
    a.enumeration(&data, "DcmDspDataUsePort", "USE_DATA_ASYNCH_CLIENT_SERVER USE_DATA_ASYNCH_CLIENT_SERVER_ERROR USE_DATA_ASYNCH_FNC USE_DATA_ASYNCH_FNC_ERROR USE_DATA_ASYNCH_FNC_PROXY USE_DATA_SENDER_RECEIVER USE_DATA_SENDER_RECEIVER_AS_SERVICE USE_DATA_SYNCH_CLIENT_SERVER USE_DATA_SYNCH_FNC USE_DATA_SYNCH_FNC_PROXY USE_ECU_SIGNAL", 1)?;
    a.enumeration(
        &data,
        "DcmDspDataEndianness",
        "BIG_ENDIAN LITTLE_ENDIAN OPAQUE",
        0,
    )?;
    for function in ["DcmDspDataReadFnc", "DcmDspDataWriteFnc"] {
        a.field(&data, function, ValueKind::FunctionName, 0)?;
    }
    let data_info = format!("{dsp}/DcmDspDataInfo");
    a.container(&data_info, 0, None)?;
    a.integers(&data_info, "DcmDspDataScalingInfoSize", "0", U32, 0)?;
    a.reference(&data, "DcmDspDataInfoRef", &data_info, 0, Some(1))?;
    let did = format!("{dsp}/DcmDspDid");
    a.container(&did, 0, None)?;
    a.integers(&did, "DcmDspDidIdentifier", "0", U16, 1)?;
    a.integers(&did, "DcmDspDidSize", "0", U16, 0)?;
    a.booleans(&did, "DcmDspDidUsed", 1)?;
    a.enumeration(&did, "DcmDspDidUsePort", "USE_ATOMIC_BNDM USE_ATOMIC_NV_DATA_INTERFACE USE_ATOMIC_SENDER_RECEIVER_INTERFACE USE_ATOMIC_SENDER_RECEIVER_INTERFACE_AS_SERVICE USE_DATA_ELEMENT_SPECIFIC_INTERFACES", 1)?;
    let info = format!("{dsp}/DcmDspDidInfo");
    a.container(&info, 0, None)?;
    a.booleans(&info, "DcmDspDidDynamicallyDefined", 1)?;
    a.reference(&did, "DcmDspDidInfoRef", &info, 1, Some(1))?;
    let signal = format!("{did}/DcmDspDidSignal");
    a.container(&signal, 0, None)?;
    a.integers(&signal, "DcmDspDidByteOffset", "0", U16, 1)?;
    a.reference(&signal, "DcmDspDidDataRef", &data, 0, Some(1))?;
    for (name, prefix) in [
        ("DcmDspDidRead", "DcmDspDidRead"),
        ("DcmDspDidWrite", "DcmDspDidWrite"),
    ] {
        let path = format!("{info}/{name}");
        a.container(&path, 0, Some(1))?;
        a.reference(&path, &format!("{prefix}SessionRef"), &session, 0, None)?;
        if name == "DcmDspDidWrite" {
            a.reference(&path, "DcmDspDidWriteSecurityLevelRef", &security, 0, None)?;
        }
    }
    a.container(&format!("{dsp}/DcmDspSession"), 1, Some(1))?;
    a.container(&session, 0, Some(31))?;
    a.integers(&session, "DcmDspSessionLevel", "1", "126", 1)?;
    a.enumeration(
        &session,
        "DcmDspSessionForBoot",
        "DCM_NO_BOOT DCM_OEM_BOOT DCM_OEM_BOOT_RESPAPP DCM_SYS_BOOT DCM_SYS_BOOT_RESPAPP",
        1,
    )?;
    a.floats(&session, "DcmDspSessionP2ServerMax", Some("1"), 1)?;
    a.floats(&session, "DcmDspSessionP2StarServerMax", Some("100"), 1)?;
    let security_root = format!("{dsp}/DcmDspSecurity");
    a.container(&security_root, 1, Some(1))?;
    a.booleans(
        &security_root,
        "DcmDspSecurityResetAttemptCounterOnTimeout",
        0,
    )?;
    a.container(&security, 0, Some(31))?;
    a.integers(&security, "DcmDspSecurityLevel", "1", "63", 1)?;
    a.integers(
        &security,
        "DcmDspSecuritySeedSize DcmDspSecurityKeySize",
        "1",
        U32,
        1,
    )?;
    a.integers(&security, "DcmDspSecurityNumAttDelay", "1", U8, 0)?;
    a.booleans(&security, "DcmDspSecurityAttemptCounterEnabled", 1)?;
    a.enumeration(
        &security,
        "DcmDspSecurityUsePort",
        "USE_ASYNCH_CLIENT_SERVER USE_ASYNCH_FNC",
        1,
    )?;
    a.floats(
        &security,
        "DcmDspSecurityDelayTime DcmDspSecurityDelayTimeOnBoot",
        Some("65535"),
        1,
    )?;
    for name in [
        "DcmDspSecurityGetSeedFnc",
        "DcmDspSecurityCompareKeyFnc",
        "DcmDspSecurityGetAttemptCounterFnc",
        "DcmDspSecuritySetAttemptCounterFnc",
    ] {
        a.field(&security, name, ValueKind::FunctionName, 0)?;
    }
    for name in [
        "DcmDspClearDTC",
        "DcmDspReadDTCInformation",
        "DcmDspControlDTCSetting",
    ] {
        a.container(&format!("{dsp}/{name}"), 0, Some(1))?;
    }
    a.booleans(
        &format!("{dsp}/DcmDspControlDTCSetting"),
        "DcmSupportDTCSettingControlOptionRecord",
        0,
    )?;
    let page = format!("{base}/DcmPageBufferCfg");
    a.container(&page, 1, Some(1))?;
    a.booleans(&page, "DcmPagedBufferEnabled", 1)?;
    let general = "Dcm/DcmGeneral";
    a.container(general, 1, Some(1))?;
    a.booleans(
        general,
        "DcmDevErrorDetect DcmEnableSecurityEventReporting DcmRespondAllRequest DcmVersionInfoApi",
        1,
    )?;
    a.floats(general, "DcmTaskTime", None, 1)?;
    a.floats(general, "DcmS3ServerTimeoutOverwrite", None, 0)?;
    a.0.entries_mut()
        .get_mut(&format!("{ROOT}{general}/DcmS3ServerTimeoutOverwrite"))
        .unwrap()
        .minimum = Some("5".into());
    Ok(())
}

fn operating_system(a: &mut Author<'_>) -> Result<(), crate::message::LocalizedText> {
    a.container("Os/OsAppMode", 1, None)?;
    let os = "Os/OsOS";
    a.container(os, 1, Some(1))?;
    a.enumeration(os, "OsStatus", "EXTENDED STANDARD", 1)?;
    a.enumeration(os, "OsScalabilityClass", "SC1 SC2 SC3 SC4", 0)?;
    a.booleans(
        os,
        "OsUseGetServiceId OsUseParameterAccess OsUseResScheduler",
        1,
    )?;
    let hooks = "Os/OsOS/OsHooks";
    a.container(hooks, 1, Some(1))?;
    a.booleans(
        hooks,
        "OsErrorHook OsPostTaskHook OsPreTaskHook OsShutdownHook OsStartupHook",
        1,
    )?;
    a.booleans(hooks, "OsProtectionHook", 0)?;
    a.container("Os/OsResource", 0, None)?;
    a.enumeration(
        "Os/OsResource",
        "OsResourceProperty",
        "INTERNAL LINKED STANDARD",
        1,
    )?;
    let task = "Os/OsTask";
    a.container(task, 0, None)?;
    a.integers(task, "OsTaskActivation", "1", U32, 1)?;
    a.integers(task, "OsTaskPriority", "0", U32, 1)?;
    a.enumeration(task, "OsTaskSchedule", "FULL NON", 1)?;
    a.reference(task, "OsTaskEventRef", "Os/OsEvent", 0, None)?;
    let autostart = "Os/OsTask/OsTaskAutostart";
    a.container(autostart, 0, Some(1))?;
    a.reference(autostart, "OsTaskAppModeRef", "Os/OsAppMode", 1, None)?;
    a.container("Os/OsEvent", 0, None)?;
    a.integers("Os/OsEvent", "OsEventMask", "0", U64, 0)?;
    let counter = "Os/OsCounter";
    a.container(counter, 0, None)?;
    a.integers(
        counter,
        "OsCounterMaxAllowedValue OsCounterMinCycle",
        "1",
        U64,
        1,
    )?;
    a.integers(counter, "OsCounterTicksPerBase", "1", U32, 1)?;
    a.enumeration(counter, "OsCounterType", "HARDWARE SOFTWARE", 1)?;
    a.floats(counter, "OsSecondsPerTick", None, 0)?;
    let alarm = "Os/OsAlarm";
    a.container(alarm, 0, None)?;
    a.reference(alarm, "OsAlarmCounterRef", counter, 1, Some(1))?;
    let autostart = "Os/OsAlarm/OsAlarmAutostart";
    a.container(autostart, 0, Some(1))?;
    a.integers(autostart, "OsAlarmAlarmTime OsAlarmCycleTime", "0", U64, 1)?;
    a.enumeration(autostart, "OsAlarmAutostartType", "ABSOLUTE RELATIVE", 1)?;
    a.reference(autostart, "OsAlarmAppModeRef", "Os/OsAppMode", 1, None)?;
    a.choice("Os/OsAlarm/OsAlarmAction", 1, Some(1))?;
    let action = "Os/OsAlarm/OsAlarmAction/OsAlarmSetEvent";
    a.container(action, 0, Some(1))?;
    a.reference(action, "OsAlarmSetEventTaskRef", task, 1, Some(1))?;
    a.reference(action, "OsAlarmSetEventRef", "Os/OsEvent", 1, Some(1))?;
    let schedule = "Os/OsScheduleTable";
    a.container(schedule, 0, None)?;
    a.integers(schedule, "OsScheduleTableDuration", "0", U64, 1)?;
    a.booleans(schedule, "OsScheduleTableRepeating", 1)?;
    a.reference(schedule, "OsScheduleTableCounterRef", counter, 1, Some(1))?;
    let autostart = "Os/OsScheduleTable/OsScheduleTableAutostart";
    a.container(autostart, 0, Some(1))?;
    a.enumeration(
        autostart,
        "OsScheduleTableAutostartType",
        "ABSOLUTE RELATIVE SYNCHRON",
        1,
    )?;
    a.integers(autostart, "OsScheduleTableStartValue", "0", U64, 0)?;
    a.reference(
        autostart,
        "OsScheduleTableAppModeRef",
        "Os/OsAppMode",
        1,
        None,
    )?;
    let expiry = "Os/OsScheduleTable/OsScheduleTableExpiryPoint";
    a.container(expiry, 1, None)?;
    a.integers(expiry, "OsScheduleTblExpPointOffset", "0", U64, 1)?;
    let event = "Os/OsScheduleTable/OsScheduleTableExpiryPoint/OsScheduleTableTaskActivation";
    a.container(event, 0, None)?;
    a.reference(event, "OsScheduleTableActivateTaskRef", task, 1, Some(1))?;
    let event = "Os/OsScheduleTable/OsScheduleTableExpiryPoint/OsScheduleTableEventSetting";
    a.container(event, 0, None)?;
    a.reference(event, "OsScheduleTableSetEventTaskRef", task, 1, Some(1))?;
    a.reference(
        event,
        "OsScheduleTableSetEventRef",
        "Os/OsEvent",
        1,
        Some(1),
    )?;
    let sync = "Os/OsScheduleTable/OsScheduleTableSync";
    a.container(sync, 0, Some(1))?;
    a.enumeration(
        sync,
        "OsScheduleTblSyncStrategy",
        "EXPLICIT IMPLICIT NONE",
        1,
    )?;
    a.default(sync, "OsScheduleTblSyncStrategy", "NONE");

    let routes = "PduR/PduRRoutingPaths";
    a.container(routes, 1, Some(1))?;
    let source = "PduR/PduRRoutingPaths/PduRSrcPdu";
    let destination = "PduR/PduRRoutingPaths/PduRDestPdu";
    let route = "PduR/PduRRoutingPaths/PduRRoutingPath";
    for path in [source, destination, route] {
        a.container(path, 0, None)?;
    }
    a.integers(source, "PduRSourcePduHandleId", "0", U16, 1)?;
    a.booleans(source, "PduRSrcPduUpTxConf", 1)?;
    a.reference(source, "PduRSrcPduRef", PDU, 1, Some(1))?;
    a.integers(destination, "PduRDestPduHandleId", "0", U16, 0)?;
    a.booleans(destination, "PduRTransmissionConfirmation", 1)?;
    a.reference(destination, "PduRDestPduRef", PDU, 1, Some(1))?;
    a.reference(route, "PduRDestPduRRef", destination, 1, Some(1))?;
    a.reference(route, "PduRSrcPduRRef", source, 1, Some(1))?;

    for (path, mapping, prefix) in [
        ("Rte/RteSwComponentInstance", "RteEventToTaskMapping", "Rte"),
        (
            "Rte/RteBswModuleInstance",
            "RteBswEventToTaskMapping",
            "RteBsw",
        ),
    ] {
        a.container(path, 0, None)?;
        let map = format!("{path}/{mapping}");
        a.container(&map, 0, None)?;
        a.integers(&map, &format!("{prefix}PositionInTask"), "0", U16, 0)?;
        a.reference(&map, &format!("{prefix}MappedToTaskRef"), task, 0, Some(1))?;
        a.reference(&map, &format!("{prefix}UsedOsAlarmRef"), alarm, 0, Some(1))?;
        a.reference(
            &map,
            &format!("{prefix}UsedOsEventRef"),
            "Os/OsEvent",
            0,
            Some(1),
        )?;
        a.reference(
            &map,
            &format!("{prefix}UsedOsSchTblExpiryPointRef"),
            expiry,
            0,
            Some(1),
        )?;
        a.booleans(&map, &format!("{prefix}EventIsMappedToTask"), 0)?;
        a.default(&map, &format!("{prefix}EventIsMappedToTask"), "false");
        if prefix == "Rte" {
            a.foreign(
                path,
                "RteSoftwareComponentInstanceRef",
                "SW-COMPONENT-PROTOTYPE",
                0,
                Some(1),
            )?;
            a.foreign(
                &map,
                "RteEventRef",
                "TIMING-EVENT OPERATION-INVOKED-EVENT DATA-RECEIVED-EVENT INIT-EVENT",
                1,
                None,
            )?;
        } else {
            a.foreign(
                path,
                "RteBswImplementationRef",
                "BSW-IMPLEMENTATION",
                1,
                Some(1),
            )?;
            a.foreign(
                &map,
                "RteBswEventRef",
                "BSW-TIMING-EVENT BSW-MODE-SWITCH-EVENT BSW-INTERNAL-TRIGGER-OCCURRED-EVENT",
                1,
                None,
            )?;
        }
    }
    let user = "Rte/RteComUser";
    a.container(user, 0, None)?;
    let config = "Rte/RteComUser/ComUserModuleCnf";
    a.container(config, 0, Some(1))?;
    a.field(config, "ComUserHeaderInclude", ValueKind::String, 0)?;
    let callback = "Rte/RteComUser/ComUserModuleCnf/ComUserCallback";
    a.container(callback, 0, None)?;
    a.field(callback, "ComUserCallbackName", ValueKind::FunctionName, 1)?;
    a.enumeration(
        callback,
        "ComUserCallbackType",
        "COM_RX_ACK COM_RX_INV COM_RX_TOUT COM_TX_ACK COM_TX_ERR COM_TX_TOUT",
        1,
    )?;
    let signal = "Rte/RteComUser/ComUserModuleCnf/ComUserSignal";
    a.container(signal, 0, None)?;
    a.integers(signal, "ComUserCbkHandleId", "0", U16, 0)?;
    a.reference(signal, "ComUserCallbackRef", callback, 0, None)?;
    a.foreign(
        signal,
        "ComUserSystemTemplateSystemSignalRef",
        "I-SIGNAL-TO-I-PDU-MAPPING",
        0,
        Some(1),
    )?;
    Ok(())
}

fn persistence(a: &mut Author<'_>) -> Result<(), crate::message::LocalizedText> {
    let general = "Mcu/McuGeneralConfiguration";
    a.container(general, 1, Some(1))?;
    a.booleans(general, "McuDevErrorDetect McuGetRamStateApi McuInitClock McuNoPll McuPerformResetApi McuVersionInfoApi", 1)?;
    let config = "Mcu/McuModuleConfiguration";
    a.container(config, 1, Some(1))?;
    a.enumeration(
        config,
        "McuClockSrcFailureNotification",
        "DISABLED ENABLED",
        1,
    )?;
    a.integers(config, "McuNumberOfMcuModes", "1", U8, 1)?;
    a.integers(config, "McuRamSectors", "0", U32, 1)?;
    let clock = "Mcu/McuModuleConfiguration/McuClockSettingConfig";
    a.container(clock, 1, None)?;
    a.integers(clock, "McuClockSettingId", "0", U8, 1)?;
    let point = "Mcu/McuModuleConfiguration/McuClockSettingConfig/McuClockReferencePoint";
    a.container(point, 1, None)?;
    a.floats(point, "McuClockReferencePointFrequency", None, 1)?;
    a.0.entries_mut()
        .get_mut(&format!("{ROOT}{point}/McuClockReferencePointFrequency"))
        .unwrap()
        .unit = Some("Hz".into());
    let mode = "Mcu/McuModuleConfiguration/McuModeSettingConf";
    a.container(mode, 1, None)?;
    a.integers(mode, "McuMode", "0", U8, 1)?;
    a.container("Mcu/McuPublishedInformation", 1, Some(1))?;
    a.container("Mcu/McuPublishedInformation/McuResetReasonConf", 1, None)?;
    a.integers(
        "Mcu/McuPublishedInformation/McuResetReasonConf",
        "McuResetReason",
        "0",
        U8,
        1,
    )?;

    let general = "Dem/DemGeneral";
    a.container(general, 1, Some(1))?;
    a.booleans(general, "DemAgingRequiresNotFailedCycle DemAgingRequiresTestedCycle DemDebounceCounterBasedSupport DemDebounceTimeBasedSupport DemDevErrorDetect DemGeneralInterfaceSupport DemPTOSupport DemResetConfirmedBitOnOverflow DemResetPendingBitOnOverflow DemStatusBitStorageTestFailed DemTriggerFiMReports DemTriggerMonitorInitBeforeClearOk DemVersionInfoApi", 1)?;
    a.enumeration(
        general,
        "DemAvailabilitySupport",
        "DEM_NO_AVAILABILITY DEM_EVENT_AVAILABILITY",
        1,
    )?;
    a.enumeration(
        general,
        "DemClearDTCBehavior",
        "DEM_CLRRESP_NONVOLATILE_FINISH DEM_CLRRESP_NONVOLATILE_TRIGGER DEM_CLRRESP_VOLATILE",
        1,
    )?;
    a.enumeration(
        general,
        "DemClearDTCLimitation",
        "DEM_ALL_SUPPORTED_DTCS DEM_ONLY_CLEAR_ALL_DTCS",
        1,
    )?;
    a.enumeration(
        general,
        "DemDataElementDefaultEndianness",
        "BIG_ENDIAN LITTLE_ENDIAN OPAQUE",
        1,
    )?;
    a.enumeration(
        general,
        "DemEventCombinationSupport",
        "DEM_EVCOMB_DISABLED DEM_EVCOMB_ONRETRIEVAL DEM_EVCOMB_ONSTORAGE",
        1,
    )?;
    a.enumeration(
        general,
        "DemOBDSupport",
        "DEM_OBD_DEP_SEC_ECU DEM_OBD_MASTER_ECU DEM_OBD_NO_OBD_SUPPORT DEM_OBD_PRIMARY_ECU",
        1,
    )?;
    a.enumeration(
        general,
        "DemStatusBitHandlingTestFailedSinceLastClear",
        "DEM_STATUS_BIT_AGING_AND_DISPLACEMENT DEM_STATUS_BIT_NORMAL",
        1,
    )?;
    a.enumeration(
        general,
        "DemSuppressionSupport",
        "DEM_DTC_SUPPRESSION DEM_NO_SUPPRESSION",
        1,
    )?;
    a.integers(general, "DemMaxNumberPrestoredFF", "0", U8, 1)?;
    a.floats(general, "DemTaskTime", None, 1)?;
    let memory = "Dem/DemGeneral/DemEventMemorySet";
    a.container(memory, 1, Some(255))?;
    a.integers(memory, "DemMaxNumberEventEntryPermanent", "0", U8, 1)?;
    a.enumeration(memory, "DemTypeOfDTCSupported", "DEM_DTC_TRANSLATION_ISO11992_4 DEM_DTC_TRANSLATION_ISO14229_1 DEM_DTC_TRANSLATION_ISO15031_6 DEM_DTC_TRANSLATION_SAEJ1939_73 DEM_DTC_TRANSLATION_SAE_J2012_DA_DTCFORMAT_04", 1)?;
    let primary = "Dem/DemGeneral/DemEventMemorySet/DemPrimaryMemory";
    a.container(primary, 1, Some(1))?;
    a.integers(primary, "DemDtcStatusAvailabilityMask", "0", U8, 1)?;
    a.integers(primary, "DemMaxNumberEventEntryPrimary", "1", U8, 1)?;
    a.enumeration(
        primary,
        "DemEventDisplacementStrategy",
        "DEM_DISPLACEMENT_FULL DEM_DISPLACEMENT_NONE DEM_DISPLACEMENT_PRIO_OCC",
        1,
    )?;
    a.enumeration(
        primary,
        "DemEventMemoryEntryStorageTrigger",
        "DEM_TRIGGER_ON_CONFIRMED DEM_TRIGGER_ON_FDC_THRESHOLD DEM_TRIGGER_ON_TEST_FAILED",
        1,
    )?;
    a.enumeration(
        primary,
        "DemOccurrenceCounterProcessing",
        "DEM_PROCESS_OCCCTR_CDTC DEM_PROCESS_OCCCTR_TF",
        1,
    )?;
    a.enumeration(
        primary,
        "DemTypeOfFreezeFrameRecordNumeration",
        "DEM_FF_RECNUM_CALCULATED DEM_FF_RECNUM_CONFIGURED",
        1,
    )?;
    let cycle = "Dem/DemGeneral/DemOperationCycle";
    a.container(cycle, 1, Some(256))?;
    a.integers(cycle, "DemOperationCycleId", "0", U8, 1)?;
    let client = "Dem/DemGeneral/DemClient";
    a.container(client, 1, Some(255))?;
    a.integers(client, "DemClientId", "0", U8, 1)?;
    a.booleans(client, "DemClientUsesRte", 1)?;
    a.enumeration(
        client,
        "DemClientFunctionality",
        "DEM_CLIENT_USES_FULL_FUNCTIONALITY DEM_CLIENT_ONLY_USES_EVENTOVERFLOW_INTERFACE",
        1,
    )?;
    a.reference(client, "DemEventMemorySetRef", memory, 1, Some(1))?;
    let block = "Dem/DemGeneral/DemNvRamBlockId";
    a.container(block, 0, None)?;
    a.reference(
        block,
        "DemNvRamBlockIdRef",
        "NvM/NvMBlockDescriptor",
        1,
        Some(1),
    )?;
    a.container("Dem/DemConfigSet", 1, Some(1))?;
    let attrs = "Dem/DemConfigSet/DemDTCAttributes";
    a.container(attrs, 0, Some(65535))?;
    a.integers(attrs, "DemDTCPriority", "1", U8, 1)?;
    a.reference(attrs, "DemMemoryDestinationRef", primary, 1, Some(1))?;
    a.0.entries_mut()
        .get_mut(&format!("{ROOT}{attrs}/DemMemoryDestinationRef"))
        .unwrap()
        .element_kind = "ECUC-CHOICE-REFERENCE-DEF".into();
    let dtc = "Dem/DemConfigSet/DemDTC";
    a.container(dtc, 0, Some(65535))?;
    a.integers(dtc, "DemDtcValue", "256", "16777214", 0)?;
    a.enumeration(
        dtc,
        "DemNvStorageStrategy",
        "DURING_SHUTDOWN IMMEDIATE_AT_FIRST_OCCURRENCE",
        0,
    )?;
    a.reference(dtc, "DemDTCAttributesRef", attrs, 1, Some(1))?;
    let event = "Dem/DemConfigSet/DemEventParameter";
    a.container(event, 1, Some(65535))?;
    a.booleans(event, "DemEventAvailable DemFFPrestorageSupported", 1)?;
    a.integers(event, "DemEventConfirmationThreshold", "1", U8, 1)?;
    a.integers(event, "DemEventId", "1", U16, 1)?;
    a.enumeration(
        event,
        "DemEventKind",
        "DEM_EVENT_KIND_BSW DEM_EVENT_KIND_SWC",
        1,
    )?;
    a.enumeration(
        event,
        "DemEventReportingType",
        "STANDARD_REPORTING STANDARD_REPORTING_WITH_MONITOR_DATA",
        1,
    )?;
    a.reference(event, "DemDTCRef", dtc, 0, Some(1))?;
    a.reference(event, "DemOperationCycleRef", cycle, 1, Some(1))?;
    a.choice(
        "Dem/DemConfigSet/DemEventParameter/DemDebounceAlgorithmClass",
        1,
        Some(1),
    )?;
    a.container(
        "Dem/DemConfigSet/DemEventParameter/DemDebounceAlgorithmClass/DemDebounceMonitorInternal",
        0,
        Some(1),
    )?;
    let nv = "NvM/NvMBlockDescriptor";
    a.container(nv, 1, Some(65536))?;
    a.integers(nv, "NvMBlockJobPriority", "0", U8, 1)?;
    a.integers(
        nv,
        "NvMMaxNumOfReadRetries NvMMaxNumOfWriteRetries",
        "0",
        "7",
        1,
    )?;
    a.integers(nv, "NvMNvramDeviceId", "0", "1", 1)?;
    a.integers(nv, "NvMNvBlockBaseNumber", "1", "65534", 1)?;
    a.integers(
        nv,
        "NvMNvBlockLength NvMWriteVerificationDataSize",
        "1",
        U16,
        1,
    )?;
    a.integers(nv, "NvMNvramBlockIdentifier", "2", U16, 1)?;
    a.integers(nv, "NvMNvBlockNum", "1", U8, 1)?;
    a.integers(nv, "NvMRomBlockNum", "0", "254", 1)?;
    a.booleans(nv, "NvMBlockUseAutoValidation NvMBlockUseCompression NvMBlockUseCrc NvMBlockUseCRCCompMechanism NvMBlockUsePort NvMBlockUseSetRamBlockStatus NvMBlockUseSyncMechanism NvMBlockWriteProt NvMBswMBlockStatusInformation NvMResistantToChangedSw NvMStaticBlockIDCheck NvMWriteBlockOnce NvMWriteVerification", 1)?;
    a.enumeration(nv, "NvMBlockCrcType", "NVM_CRC8 NVM_CRC16 NVM_CRC32", 0)?;
    a.enumeration(
        nv,
        "NvMBlockManagementType",
        "NVM_BLOCK_NATIVE NVM_BLOCK_REDUNDANT NVM_BLOCK_DATASET",
        1,
    )?;
    Ok(())
}

fn defaults(a: &mut Author<'_>) {
    for (path, lexeme, names) in [
        (
            "Can/CanGeneral",
            "false",
            "CanEnableSecurityEventReporting CanVersionInfoApi",
        ),
        ("Can/CanConfigSet/CanController", "false", "CanHwPnSupport"),
        (
            "Can/CanConfigSet/CanController/CanControllerBaudrateConfig",
            "0",
            "CanControllerBaudRateConfigID",
        ),
        (
            "Can/CanConfigSet/CanHardwareObject",
            "1",
            "CanHwObjectCount",
        ),
        (
            "CanIf/CanIfCtrlDrvCfg/CanIfCtrlCfg",
            "false",
            "CanIfCtrlWakeupSupport",
        ),
        ("CanIf/CanIfInitCfg/CanIfBufferCfg", "0", "CanIfBufferSize"),
        (
            "CanIf/CanIfInitCfg/CanIfInitHohCfg/CanIfHrhCfg",
            "true",
            "CanIfHrhSoftwareFilter",
        ),
        (
            "CanIf/CanIfInitCfg/CanIfRxPduCfg",
            "true",
            "CanIfRxPduDataLengthCheck",
        ),
        (
            "CanIf/CanIfInitCfg/CanIfRxPduCfg",
            "false",
            "CanIfRxPduReadData CanIfRxPduReadNotifyStatus",
        ),
        (
            "CanIf/CanIfInitCfg/CanIfTxPduCfg",
            "false",
            "CanIfTxPduReadNotifyStatus",
        ),
        (
            "CanIf/CanIfInitCfg/CanIfTxPduCfg",
            "true",
            "CanIfTxPduTruncation",
        ),
        (
            "CanIf/CanIfPrivateCfg",
            "false",
            "CanIfFixedBuffer CanIfSupportTTCAN",
        ),
        (
            "CanIf/CanIfPrivateCfg",
            "true",
            "CanIfPrivateDataLengthCheck",
        ),
        (
            "CanIf/CanIfPublicCfg",
            "false",
            "CanIfBusMirroringSupport CanIfDevErrorDetect CanIfEnableSecurityEventReporting CanIfPublicCtrlPnEnable CanIfPublicPnSupport CanIfPublicReadRxPduDataApi CanIfPublicReadRxPduNotifyStatusApi CanIfPublicReadTxPduNotifyStatusApi CanIfPublicSetDynamicTxIdApi CanIfPublicTrcvPnEnable CanIfPublicTxBuffering CanIfPublicWakeupCheckValidSupport CanIfTxOfflineActiveSupport CanIfVersionInfoApi",
        ),
        (
            "CanIf/CanIfPublicCfg",
            "true",
            "CanIfPublicMultipleDrvSupport CanIfTriggerTransmitSupport CanIfWakeupSupport",
        ),
        (
            "CanTp/CanTpGeneral",
            "false",
            "CanTpDevErrorDetect CanTpEnableSecurityEventReporting CanTpVersionInfoApi",
        ),
        (
            "Com/ComGeneral",
            "false",
            "ComEnableSecurityEventReporting ComEnableSignalGroupArrayApi ComVersionInfoApi",
        ),
        (
            "Dcm/DcmGeneral",
            "false",
            "DcmDevErrorDetect DcmEnableSecurityEventReporting DcmVersionInfoApi",
        ),
        (
            "Dcm/DcmConfigSet/DcmDsd/DcmDsdServiceTable/DcmDsdService",
            "true",
            "DcmDsdServiceUsed",
        ),
        (
            "Dcm/DcmConfigSet/DcmDsd/DcmDsdServiceTable/DcmDsdService/DcmDsdSubService",
            "true",
            "DcmDsdSubServiceUsed",
        ),
        (
            "Dcm/DcmConfigSet/DcmDsl/DcmDslProtocol/DcmDslProtocolRow",
            "true",
            "DcmDslProtocolRowUsed DcmSendRespPendOnRestart",
        ),
        (
            "Dcm/DcmConfigSet/DcmDsp/DcmDspDid",
            "USE_DATA_ELEMENT_SPECIFIC_INTERFACES",
            "DcmDspDidUsePort",
        ),
        ("Dcm/DcmConfigSet/DcmDsp", "false", "DcmDspEnableObdMirror"),
        (
            "Dcm/DcmConfigSet/DcmDsp/DcmDspControlDTCSetting",
            "false",
            "DcmSupportDTCSettingControlOptionRecord",
        ),
        (
            "Dcm/DcmConfigSet/DcmDsp/DcmDspSecurity",
            "false",
            "DcmDspSecurityResetAttemptCounterOnTimeout",
        ),
        (
            "Dcm/DcmConfigSet/DcmDsp/DcmDspSession/DcmDspSessionRow",
            "DCM_NO_BOOT",
            "DcmDspSessionForBoot",
        ),
        (
            "Mcu/McuGeneralConfiguration",
            "false",
            "McuDevErrorDetect McuVersionInfoApi",
        ),
        (
            "Mcu/McuGeneralConfiguration",
            "true",
            "McuInitClock McuNoPll",
        ),
        (
            "Dem/DemGeneral",
            "false",
            "DemAgingRequiresNotFailedCycle DemDevErrorDetect DemVersionInfoApi",
        ),
        (
            "Dem/DemGeneral",
            "true",
            "DemResetConfirmedBitOnOverflow DemResetPendingBitOnOverflow",
        ),
        (
            "Dem/DemGeneral",
            "DEM_ALL_SUPPORTED_DTCS",
            "DemClearDTCLimitation",
        ),
        (
            "Dem/DemGeneral",
            "DEM_STATUS_BIT_NORMAL",
            "DemStatusBitHandlingTestFailedSinceLastClear",
        ),
        (
            "Dem/DemGeneral/DemClient",
            "DEM_CLIENT_USES_FULL_FUNCTIONALITY",
            "DemClientFunctionality",
        ),
        ("Dem/DemGeneral/DemClient", "false", "DemClientUsesRte"),
        (
            "Dem/DemGeneral/DemEventMemorySet/DemPrimaryMemory",
            "DEM_TRIGGER_ON_TEST_FAILED",
            "DemEventMemoryEntryStorageTrigger",
        ),
        (
            "Dem/DemConfigSet/DemDTC",
            "DURING_SHUTDOWN",
            "DemNvStorageStrategy",
        ),
        (
            "Dem/DemConfigSet/DemEventParameter",
            "1",
            "DemEventConfirmationThreshold",
        ),
        (
            "Dem/DemConfigSet/DemEventParameter",
            "DEM_EVENT_KIND_SWC",
            "DemEventKind",
        ),
        (
            "Dem/DemConfigSet/DemEventParameter",
            "STANDARD_REPORTING",
            "DemEventReportingType",
        ),
        (
            "Dem/DemConfigSet/DemEventParameter",
            "false",
            "DemFFPrestorageSupported",
        ),
        ("NvM/NvMBlockDescriptor", "0", "NvMMaxNumOfReadRetries"),
        (
            "NvM/NvMBlockDescriptor",
            "false",
            "NvMBlockUseAutoValidation NvMBlockUseCompression NvMBlockUseCRCCompMechanism NvMBlockUseSyncMechanism NvMBswMBlockStatusInformation NvMStaticBlockIDCheck NvMWriteVerification",
        ),
        ("Os/OsOS", "true", "OsUseResScheduler"),
        (
            "PduR/PduRRoutingPaths/PduRSrcPdu",
            "true",
            "PduRSrcPduUpTxConf",
        ),
    ] {
        for name in names.split_whitespace() {
            a.default(path, name, lexeme);
        }
    }
}
