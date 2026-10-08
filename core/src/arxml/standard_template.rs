//! Explicit source policy for the product-original virtual ECU template.
//! This does not materialize catalog defaults into opened user projects.
use super::*;
use crate::project_model::{DefinitionDescriptor, TypedValue, ValueKind};

const ROOT: &str = "/AUTOSAR/EcucDefs/";

fn value(id: &str) -> Result<&'static str, crate::message::LocalizedText> {
    let name = id.rsplit('/').next().ok_or_else(|| {
        crate::product_message!("backend.arxml.standard_template.definition_name_missing")
    })?;
    Ok(match name {
        "EcucCoreId" => "0",
        "CanControllerBaseAddress"
        | "CanControllerBaudRateConfigID"
        | "CanIndex"
        | "ComTxIPduUnusedAreasDefault"
        | "ComSupportedIPduGroups"
        | "McuRamSectors"
        | "McuClockSettingId"
        | "McuMode"
        | "McuResetReason" => "0",
        "CanControllerPropSeg" => "6",
        "CanControllerSeg1" => "7",
        "CanControllerSeg2" => "2",
        "CanControllerSyncJumpWidth" | "McuNumberOfMcuModes" => "1",
        "McuClockReferencePointFrequency" => "80000000",
        "CanMainFunctionModePeriod" | "CanTimeoutDuration" | "DcmTaskTime" => "0.001",
        "CanObjectPayloadLength" => "CAN_OBJECT_PL_8",
        "CanIfDispatchUserCtrlBusOffUL" | "CanIfDispatchUserCtrlModeIndicationUL" => "CDD",
        "CanIfInitCfgSet" => "VirtualCanConfiguration",
        "CanIfPrivateSoftwareFilterType" => "LINEAR",
        "CanIfPublicHandleTypeEnum" => "UINT16",
        "ComIPduSignalProcessing" => "DEFERRED",
        "ComIPduType" => "NORMAL",
        "DcmDspDataDefaultEndianness" => "LITTLE_ENDIAN",
        "DcmDspDidUsePort" => "USE_DATA_ELEMENT_SPECIFIC_INTERFACES",
        "McuClockSrcFailureNotification" => "DISABLED",
        "CanIfPrivateDataLengthCheck" | "DcmDspDidUsed" | "McuInitClock" | "McuNoPll" => "true",
        "CanDevErrorDetect"
        | "CanEnableSecurityEventReporting"
        | "CanGlobalTimeSupport"
        | "CanMultiplexedTransmission"
        | "CanVersionInfoApi"
        | "CanIfCtrlWakeupSupport"
        | "CanIfFixedBuffer"
        | "CanIfPublicTxBuffering"
        | "DcmRespondAllRequest"
        | "CanIfSupportTTCAN"
        | "CanIfBusMirroringSupport"
        | "CanIfDevErrorDetect"
        | "CanIfEnableSecurityEventReporting"
        | "CanIfGlobalTimeSupport"
        | "CanIfPublicCtrlPnEnable"
        | "CanIfPublicMultipleDrvSupport"
        | "CanIfPublicPnSupport"
        | "CanIfPublicReadRxPduDataApi"
        | "CanIfPublicReadRxPduNotifyStatusApi"
        | "CanIfPublicReadTxPduNotifyStatusApi"
        | "CanIfPublicSetDynamicTxIdApi"
        | "CanIfPublicTrcvPnEnable"
        | "CanIfPublicTxConfirmPollingSupport"
        | "CanIfPublicWakeupCheckValidSupport"
        | "CanIfTriggerTransmitSupport"
        | "CanIfTxOfflineActiveSupport"
        | "CanIfVersionInfoApi"
        | "CanIfWakeupSupport"
        | "CanTpChangeParameterApi"
        | "CanTpDevErrorDetect"
        | "CanTpEnableSecurityEventReporting"
        | "CanTpReadParameterApi"
        | "CanTpVersionInfoApi"
        | "ComEnableSecurityEventReporting"
        | "ComEnableSignalGroupArrayApi"
        | "ComVersionInfoApi"
        | "DcmDevErrorDetect"
        | "DcmEnableSecurityEventReporting"
        | "DcmVersionInfoApi"
        | "DcmDspEnableObdMirror"
        | "McuDevErrorDetect"
        | "McuGetRamStateApi"
        | "McuPerformResetApi"
        | "McuVersionInfoApi" => "false",
        "CanCpuClockRef" => "/Configuration/Mcu/Module/Clock/ClockReference",
        "CanSupportTTCANRef" => "/Configuration/CanIf/Private",
        "CanIfCtrlDrvInitHohConfigRef" => "/Configuration/CanIf/Config/Handles",
        "CanIfCtrlDrvNameRef" => "/Configuration/Can/General",
        "PduIdTypeEnum" | "PduLengthTypeEnum" => "UINT16",
        "CanTpTc"
        | "DcmPagedBufferEnabled"
        | "DcmDslDiagRespOnSecondDeclinedRequest"
        | "DemAgingRequiresNotFailedCycle"
        | "DemAgingRequiresTestedCycle"
        | "DemDebounceCounterBasedSupport"
        | "DemDebounceTimeBasedSupport"
        | "DemDevErrorDetect"
        | "DemGeneralInterfaceSupport"
        | "DemPTOSupport"
        | "DemResetConfirmedBitOnOverflow"
        | "DemResetPendingBitOnOverflow"
        | "DemStatusBitStorageTestFailed"
        | "DemTriggerFiMReports"
        | "DemTriggerMonitorInitBeforeClearOk"
        | "DemVersionInfoApi"
        | "DemClientUsesRte"
        | "DemEventAvailable"
        | "DemFFPrestorageSupported" => "false",
        "DemAvailabilitySupport" => "DEM_EVENT_AVAILABILITY",
        "DemClearDTCBehavior" => "DEM_CLRRESP_VOLATILE",
        "DemClearDTCLimitation" => "DEM_ALL_SUPPORTED_DTCS",
        "DemDataElementDefaultEndianness" => "LITTLE_ENDIAN",
        "DemEventCombinationSupport" => "DEM_EVCOMB_DISABLED",
        "DemOBDSupport" => "DEM_OBD_NO_OBD_SUPPORT",
        "DemStatusBitHandlingTestFailedSinceLastClear" => "DEM_STATUS_BIT_NORMAL",
        "DemSuppressionSupport" => "DEM_NO_SUPPRESSION",
        "DemMaxNumberPrestoredFF"
        | "DemMaxNumberEventEntryPermanent"
        | "DemOperationCycleId"
        | "DemClientId" => "0",
        "DemTaskTime" => "0.001",
        "DemTypeOfDTCSupported" => "DEM_DTC_TRANSLATION_ISO14229_1",
        "DemDtcStatusAvailabilityMask" => "255",
        "DemMaxNumberEventEntryPrimary" | "DemEventConfirmationThreshold" | "DemEventId" => "1",
        "DemEventDisplacementStrategy" => "DEM_DISPLACEMENT_NONE",
        "DemEventMemoryEntryStorageTrigger" => "DEM_TRIGGER_ON_CONFIRMED",
        "DemOccurrenceCounterProcessing" => "DEM_PROCESS_OCCCTR_CDTC",
        "DemTypeOfFreezeFrameRecordNumeration" => "DEM_FF_RECNUM_CONFIGURED",
        "DemClientFunctionality" => "DEM_CLIENT_USES_FULL_FUNCTIONALITY",
        "DemEventKind" => "DEM_EVENT_KIND_BSW",
        "DemEventReportingType" => "STANDARD_REPORTING",
        "DemEventMemorySetRef" => "/Configuration/Dem/General/MemorySet",
        "DemOperationCycleRef" => "/Configuration/Dem/General/OperationCycle",
        "DcmDemClientRef" => "/Configuration/Dem/General/Client",
        _ => {
            return Err(crate::product_message!(
                "backend.arxml.standard_template.required_field_source_policy_missing",
                "id" => id
            ));
        }
    })
}

fn instance_name(id: &str) -> Result<&'static str, crate::message::LocalizedText> {
    Ok(
        match id.strip_prefix(ROOT).ok_or_else(|| {
            crate::product_message!("backend.arxml.standard_template.definition_not_builtin")
        })? {
            "EcuC/EcucHardware" => "Hardware",
            "EcuC/EcucHardware/EcucCoreDefinition" => "VirtualCore",
            "Can/CanGeneral"
            | "CanTp/CanTpGeneral"
            | "Com/ComGeneral"
            | "Mcu/McuGeneralConfiguration" => "General",
            "CanIf/CanIfDispatchCfg" => "Dispatch",
            "CanIf/CanIfPrivateCfg" => "Private",
            "CanIf/CanIfPublicCfg" => "Public",
            "Mcu" => "Mcu",
            "Mcu/McuModuleConfiguration" => "Module",
            "Mcu/McuModuleConfiguration/McuClockSettingConfig" => "Clock",
            "Mcu/McuModuleConfiguration/McuClockSettingConfig/McuClockReferencePoint" => {
                "ClockReference"
            }
            "Mcu/McuModuleConfiguration/McuModeSettingConf" => "Mode",
            "Mcu/McuPublishedInformation" => "Published",
            "Mcu/McuPublishedInformation/McuResetReasonConf" => "PowerOnReset",
            "Dcm/DcmConfigSet/DcmPageBufferCfg" => "PageBuffer",
            "Dcm/DcmConfigSet/DcmDsl/DcmDslDiagResp" => "ResponsePolicy",
            "Dcm/DcmConfigSet/DcmDsp/DcmDspSecurity" => "Security",
            "Dem" => "Dem",
            "Dem/DemGeneral" => "General",
            "Dem/DemGeneral/DemEventMemorySet" => "MemorySet",
            "Dem/DemGeneral/DemEventMemorySet/DemPrimaryMemory" => "Primary",
            "Dem/DemGeneral/DemOperationCycle" => "OperationCycle",
            "Dem/DemGeneral/DemClient" => "Client",
            "Dem/DemConfigSet" => "Config",
            "Dem/DemConfigSet/DemEventParameter" => "UnavailableControllerEvent",
            "Dem/DemConfigSet/DemEventParameter/DemDebounceAlgorithmClass" => "Debounce",
            "Dem/DemConfigSet/DemEventParameter/DemDebounceAlgorithmClass/DemDebounceMonitorInternal" => {
                "MonitorInternal"
            }
            _ => {
                return Err(crate::product_message!(
                    "backend.arxml.standard_template.required_instance_source_policy_missing",
                    "id" => id
                ));
            }
        },
    )
}

fn field_xml(
    catalog: &crate::definitions::DefinitionCatalog,
    field: &DefinitionDescriptor,
) -> Result<String, crate::message::LocalizedText> {
    let kind = field.kind.ok_or_else(|| {
        crate::product_message!("backend.arxml.standard_template.field_is_instance_definition")
    })?;
    let lexeme = value(&field.definition_id)?;
    let leaf = if kind == ValueKind::Reference {
        format!("<VALUE-REF DEST=\"ECUC-CONTAINER-VALUE\">{lexeme}</VALUE-REF>")
    } else {
        catalog.validate_value(
            &field.definition_id,
            &TypedValue {
                kind,
                lexeme: lexeme.into(),
            },
        )?;
        format!("<VALUE>{lexeme}</VALUE>")
    };
    let element = super::projection::field_element(kind);
    Ok(format!(
        "<{element}><DEFINITION-REF DEST=\"{}\">{}</DEFINITION-REF>{leaf}</{element}>",
        field.element_kind, field.definition_id
    ))
}

fn instance_xml(
    catalog: &crate::definitions::DefinitionCatalog,
    definition: &DefinitionDescriptor,
) -> Result<String, crate::message::LocalizedText> {
    let module = definition.element_kind == "ECUC-MODULE-DEF";
    let element = if module {
        "ECUC-MODULE-CONFIGURATION-VALUES"
    } else {
        "ECUC-CONTAINER-VALUE"
    };
    let name = instance_name(&definition.definition_id)?;
    let mut xml = format!(
        "<{element}><SHORT-NAME>{name}</SHORT-NAME><DEFINITION-REF DEST=\"{}\">{}</DEFINITION-REF>",
        definition.element_kind, definition.definition_id
    );
    if module {
        xml.push_str("<ECUC-DEF-EDITION>4.10.0</ECUC-DEF-EDITION><IMPLEMENTATION-CONFIG-VARIANT>VARIANT-PRE-COMPILE</IMPLEMENTATION-CONFIG-VARIANT><POST-BUILD-VARIANT-USED>false</POST-BUILD-VARIANT-USED>");
    }
    let children = catalog.children(&definition.definition_id);
    for reference in [false, true] {
        let fields = children
            .iter()
            .filter(|field| {
                field.lower_multiplicity > 0
                    && field.kind.is_some()
                    && (field.kind == Some(ValueKind::Reference)) == reference
            })
            .map(|field| field_xml(catalog, field))
            .collect::<Result<Vec<_>, _>>()?
            .concat();
        if !fields.is_empty() {
            let group = if reference {
                "REFERENCE-VALUES"
            } else {
                "PARAMETER-VALUES"
            };
            xml.push_str(&format!("<{group}>{fields}</{group}>"));
        }
    }
    let nested = if definition.definition_id
        == "/AUTOSAR/EcucDefs/Dem/DemConfigSet/DemEventParameter/DemDebounceAlgorithmClass"
    {
        let branch = catalog.get("/AUTOSAR/EcucDefs/Dem/DemConfigSet/DemEventParameter/DemDebounceAlgorithmClass/DemDebounceMonitorInternal")
            .ok_or_else(|| crate::product_message!(
                "backend.arxml.standard_template.monitor_internal_debounce_metadata_missing"
            ))?;
        instance_xml(catalog, branch)?
    } else {
        children
            .into_iter()
            .filter(|child| child.kind.is_none() && child.lower_multiplicity > 0)
            .map(|child| instance_xml(catalog, child))
            .collect::<Result<Vec<_>, _>>()?
            .concat()
    };
    if !nested.is_empty() {
        let group = if module {
            "CONTAINERS"
        } else {
            "SUB-CONTAINERS"
        };
        xml.push_str(&format!("<{group}>{nested}</{group}>"));
    }
    xml.push_str(&format!("</{element}>"));
    Ok(xml)
}

pub(super) fn complete(contents: &mut String) -> Result<(), crate::message::LocalizedText> {
    let catalog = crate::definitions::DefinitionCatalog::builtin()?;
    let document = Document::parse(contents).map_err(|error| error.to_string())?;
    let mut patches = Vec::new();
    let package = document
        .descendants()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "AR-PACKAGE"
                && path_of(*node) == "/Configuration"
        })
        .ok_or_else(|| {
            crate::product_message!(
                "backend.arxml.standard_template.original_configuration_package_missing"
            )
        })?;
    let mut supporting_modules = String::new();
    for id in ["/AUTOSAR/EcucDefs/Mcu", "/AUTOSAR/EcucDefs/Dem"] {
        let definition = catalog.get(id).ok_or_else(|| {
            crate::product_message!(
                "backend.arxml.standard_template.supporting_module_metadata_missing"
            )
        })?;
        supporting_modules.push_str(&instance_xml(&catalog, definition)?);
    }
    super::changes::group_insert(
        contents,
        package.range(),
        "ELEMENTS",
        &supporting_modules,
        &mut patches,
    )?;
    for node in document.descendants().filter(|node| {
        node.is_element()
            && matches!(
                node.tag_name().name(),
                "ECUC-MODULE-CONFIGURATION-VALUES" | "ECUC-CONTAINER-VALUE"
            )
    }) {
        let Some(id) = definition(node) else {
            continue;
        };
        let children = catalog.children(&id);
        let present: BTreeSet<_> = node
            .children()
            .filter(|node| {
                node.is_element()
                    && matches!(
                        node.tag_name().name(),
                        "CONTAINERS" | "SUB-CONTAINERS" | "PARAMETER-VALUES" | "REFERENCE-VALUES"
                    )
            })
            .flat_map(|group| group.children().filter(|node| node.is_element()))
            .filter_map(definition)
            .collect();
        let missing: Vec<_> = children
            .into_iter()
            .filter(|child| child.lower_multiplicity > 0 && !present.contains(&child.definition_id))
            .collect();
        let nested = missing
            .iter()
            .filter(|child| child.kind.is_none())
            .map(|child| instance_xml(&catalog, child))
            .collect::<Result<Vec<_>, _>>()?
            .concat();
        if !nested.is_empty() {
            let group = if node.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES" {
                "CONTAINERS"
            } else {
                "SUB-CONTAINERS"
            };
            super::changes::group_insert(contents, node.range(), group, &nested, &mut patches)?;
        }
        // Reverse group insertion order keeps simultaneous zero-width patches
        // in the native PARAMETER, REFERENCE, SUB-CONTAINER sequence.
        for reference in [true, false] {
            let fields = missing
                .iter()
                .filter(|field| {
                    field.kind.is_some() && (field.kind == Some(ValueKind::Reference)) == reference
                })
                .map(|field| field_xml(&catalog, field))
                .collect::<Result<Vec<_>, _>>()?
                .concat();
            if !fields.is_empty() {
                let group = if reference {
                    "REFERENCE-VALUES"
                } else {
                    "PARAMETER-VALUES"
                };
                super::changes::group_insert(contents, node.range(), group, &fields, &mut patches)?;
            }
        }
    }
    apply_patches(contents, &mut patches)
}
