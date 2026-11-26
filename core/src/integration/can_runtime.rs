//! Source-backed communication types and selected CAN polling configuration.
use super::c_layout;
use super::component::milliseconds;
use super::graph::Graph;
use super::schedule::{ScheduleContract, definition_is, value, values};
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub enum CommunicationIntegerType {
    Uint8,
    Uint16,
    Uint32,
}
impl CommunicationIntegerType {
    fn maximum(self) -> u32 {
        match self {
            Self::Uint8 => u32::from(u8::MAX),
            Self::Uint16 => u32::from(u16::MAX),
            Self::Uint32 => u32::MAX,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct CanRuntimeContract {
    pub physical_channel: String,
    pub network: String,
    pub connector: String,
    pub can_if_controller: String,
    pub controller: String,
    pub controller_id: u8,
    pub can_if_controller_id: u8,
    pub receive_hardware: String,
    pub receive_id_type: String,
    pub receive_handle: u16,
    pub transmit_hardware: String,
    pub transmit_id_type: String,
    pub transmit_handle: u16,
    pub read_write_period: String,
    pub read_write_period_ms: u32,
    pub busoff_period_ms: u32,
    pub mode_period_ms: u32,
    pub receive_polling: bool,
    pub transmit_polling: bool,
    pub busoff_polling: bool,
    pub wakeup_polling: bool,
    pub nominal_bitrate: u32,
    pub data_bitrate: Option<u32>,
    pub brs: bool,
    pub fd_padding: u8,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct CanHardwareContract {
    pub path: String,
    pub handle: u16,
    pub controller: String,
    pub receive: bool,
    pub id_type: String,
    pub fd_padding: u8,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct PartitionRuntimeContract {
    pub path: String,
    pub name: String,
    pub id: u16,
    pub core: String,
    pub root_composition: String,
    pub instances: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct CommunicationRuntimeContract {
    pub pdu_collection: String,
    pub pdu_id_type: CommunicationIntegerType,
    pub pdu_length_type: CommunicationIntegerType,
    pub can: CanRuntimeContract,
    pub channels: Vec<CanRuntimeContract>,
    pub hardware: Vec<CanHardwareContract>,
    pub partition: PartitionRuntimeContract,
}

fn fail(graph: &Graph, index: usize, code: &str) -> Vec<PlanDiagnostic> {
    vec![graph.diagnostic(
        index,
        DiagnosticCategory::Input,
        code,
        crate::product_message!("backend.integration.composition.contract_invalid", "code" => code),
        crate::product_message!("backend.integration.composition.repair_contract"),
    )]
}
fn one(graph: &Graph, kind: &str) -> Result<usize, Vec<PlanDiagnostic>> {
    let found: Vec<_> = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, kind))
        .collect();
    if found.len() == 1 {
        Ok(found[0])
    } else {
        Err(fail(
            graph,
            found.first().copied().unwrap_or(0),
            "COMMUNICATION_CONFIGURATION",
        ))
    }
}

/// The controller and hardware object IDs are independent, dense namespaces.
/// A single driver owns the complete HRH/HTH namespace (ECUC_Can_00326).
fn numbered(
    graph: &Graph,
    kind: &str,
    name: &str,
    maximum: u16,
    code: &str,
) -> Result<BTreeMap<u16, usize>, Vec<PlanDiagnostic>> {
    let mut items = BTreeMap::new();
    for index in graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, kind))
    {
        let id = value(graph, index, name, false)
            .and_then(|text| text.parse::<u16>().ok())
            .filter(|id| *id <= maximum)
            .ok_or_else(|| fail(graph, index, code))?;
        if items.insert(id, index).is_some() {
            return Err(fail(graph, index, code));
        }
    }
    for (expected, (&id, &index)) in items.iter().enumerate() {
        if usize::from(id) != expected {
            return Err(fail(graph, index, code));
        }
    }
    Ok(items)
}
fn integer_type(
    graph: &Graph,
    collection: usize,
    name: &str,
) -> Result<CommunicationIntegerType, Vec<PlanDiagnostic>> {
    match value(graph, collection, name, false) {
        Some("UINT8") => Ok(CommunicationIntegerType::Uint8),
        Some("UINT16") => Ok(CommunicationIntegerType::Uint16),
        Some("UINT32") if name == "PduLengthTypeEnum" => Ok(CommunicationIntegerType::Uint32),
        _ => Err(fail(graph, collection, "COMMUNICATION_TYPE")),
    }
}
fn period(
    graph: &Graph,
    container: usize,
    name: &str,
    schedule: &ScheduleContract,
) -> Result<u32, Vec<PlanDiagnostic>> {
    value(graph, container, name, false)
        .and_then(milliseconds)
        .filter(|period| *period == schedule.counter_tick_ms)
        .ok_or_else(|| fail(graph, container, "CAN_POLLING_TIMEBASE"))
}

fn partition(
    graph: &Graph,
    composition: &super::composition::CompositionContract,
) -> Result<PartitionRuntimeContract, Vec<PlanDiagnostic>> {
    let index = one(graph, "EcucPartition")?;
    if super::composition::reserved_memory_scope(graph.text(index, "SHORT-NAME").unwrap_or("")) {
        return Err(fail(graph, index, "CONTRACT_NAME_COLLISION"));
    }

    let core = one(graph, "EcucCoreDefinition")?;
    if value(graph, index, "EcucPartitionId", false).and_then(|id| id.parse::<u16>().ok())
        != Some(0)
        || value(graph, core, "EcucCoreId", false).and_then(|id| id.parse::<u16>().ok()) != Some(0)
        || value(graph, index, "EcucPartitionCoreRef", true)
            != Some(graph.elements[core].object.as_str())
    {
        return Err(fail(graph, index, "RTE_PARTITION_CONFIGURATION"));
    }
    let roots = graph.of_kind("ROOT-SW-COMPOSITION-PROTOTYPE");
    let root = roots
        .iter()
        .copied()
        .filter(|root| {
            graph.text(*root, "SOFTWARE-COMPOSITION-TREF") == Some(composition.composition.as_str())
                && graph
                    .objects
                    .get(&composition.system)
                    .is_some_and(|system| graph.within(*root, *system))
        })
        .collect::<Vec<_>>();
    if root.len() != 1 {
        return Err(fail(graph, index, "RTE_PARTITION_CONFIGURATION"));
    }
    let root_path = graph.elements[root[0]].object.as_str();
    let mut instances = Vec::new();
    for group in graph.children(index, "REFERENCE-VALUES") {
        for reference in &graph.elements[group].children {
            if graph
                .text(*reference, "DEFINITION-REF")
                .is_some_and(|id| id.ends_with("/EcucPartitionSoftwareComponentInstanceRef"))
            {
                let irefs = graph.children(*reference, "VALUE-IREF");
                if irefs.len() != 1
                    || graph.elements[irefs[0]].children.len() != 2
                    || graph.text(irefs[0], "CONTEXT-ELEMENT-REF") != Some(root_path)
                {
                    return Err(fail(graph, *reference, "RTE_PARTITION_CONFIGURATION"));
                }
                let target = graph
                    .text(irefs[0], "TARGET-REF")
                    .ok_or_else(|| fail(graph, *reference, "RTE_PARTITION_CONFIGURATION"))?;
                instances.push(target.to_string());
            }
        }
    }
    instances.sort();
    let mut expected = composition
        .components
        .iter()
        .map(|component| component.instance.clone())
        .collect::<Vec<_>>();
    expected.sort();
    if instances != expected {
        return Err(fail(graph, index, "RTE_PARTITION_CONFIGURATION"));
    }
    for (kind, reference) in [
        ("RteComUser", "RteComUserEcucPartitionRef"),
        ("ComMainFunctionRx", "ComMainRxPartitionRef"),
        ("ComMainFunctionTx", "ComMainTxPartitionRef"),
    ] {
        let consumer = one(graph, kind)?;
        if values(graph, consumer, reference, true) != [graph.elements[index].object.as_str()] {
            return Err(fail(graph, consumer, "RTE_PARTITION_CONFIGURATION"));
        }
    }
    Ok(PartitionRuntimeContract {
        path: graph.elements[index].object.clone(),
        name: graph
            .text(index, "SHORT-NAME")
            .ok_or_else(|| fail(graph, index, "RTE_PARTITION_CONFIGURATION"))?
            .into(),
        id: 0,
        core: graph.elements[core].object.clone(),
        root_composition: root_path.into(),
        instances,
    })
}

fn bitrates(
    graph: &Graph,
    controller: usize,
) -> Result<(u32, Option<u32>, bool), Vec<PlanDiagnostic>> {
    let baud = value(graph, controller, "CanControllerDefaultBaudrate", true)
        .and_then(|path| graph.objects.get(path).copied())
        .filter(|index| {
            definition_is(graph, *index, "CanControllerBaudrateConfig")
                && graph.within(*index, controller)
        })
        .ok_or_else(|| fail(graph, controller, "CONTROLLER_MAPPING"))?;
    let nominal = value(graph, baud, "CanControllerBaudRate", false)
        .and_then(super::native_configuration::baudrate_kbit)
        .and_then(|value| value.checked_mul(1000))
        .ok_or_else(|| fail(graph, baud, "BAUDRATE_CONFLICT"))?;
    let fd: Vec<_> = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| {
            graph.within(*index, controller)
                && definition_is(graph, *index, "CanControllerFdBaudrateConfig")
        })
        .collect();
    let [config] = fd.as_slice() else {
        return if fd.is_empty() {
            Ok((nominal, None, false))
        } else {
            Err(fail(graph, fd[1], "CAN_FD_CONFIGURATION"))
        };
    };
    if graph.elements[*config]
        .parent
        .and_then(|parent| graph.elements[parent].parent)
        != Some(baud)
    {
        return Err(fail(graph, *config, "CAN_FD_CONFIGURATION"));
    }
    let data = value(graph, *config, "CanControllerFdBaudRate", false)
        .and_then(super::component::decimal_thousandths)
        .filter(|rate| (1..=16_000_000).contains(rate))
        .ok_or_else(|| fail(graph, *config, "CAN_FD_CONFIGURATION"))?;
    let brs = match value(graph, *config, "CanControllerTxBitRateSwitch", false) {
        Some("true" | "1") => true,
        Some("false" | "0") => false,
        _ => return Err(fail(graph, *config, "CAN_FD_CONFIGURATION")),
    };
    Ok((nominal, Some(data), brs))
}

fn reference(graph: &Graph, source: usize, name: &str) -> Result<usize, Vec<PlanDiagnostic>> {
    value(graph, source, name, true)
        .and_then(|path| graph.objects.get(path).copied())
        .ok_or_else(|| fail(graph, source, "CONTROLLER_MAPPING"))
}

fn route_controller(
    graph: &Graph,
    pdu: usize,
    receive: bool,
) -> Result<usize, Vec<PlanDiagnostic>> {
    let (hoh, binding, kind) = if receive {
        (
            reference(graph, pdu, "CanIfRxPduHrhIdRef")?,
            "CanIfHrhCanCtrlIdRef",
            "CanIfHrhCfg",
        )
    } else {
        let buffer = reference(graph, pdu, "CanIfTxPduBufferRef")?;
        (
            reference(graph, buffer, "CanIfBufferHthRef")?,
            "CanIfHthCanCtrlIdRef",
            "CanIfHthCfg",
        )
    };
    if !definition_is(graph, hoh, kind) {
        return Err(fail(graph, pdu, "CONTROLLER_MAPPING"));
    }
    let abstract_controller = reference(graph, hoh, binding)?;
    reference(graph, abstract_controller, "CanIfCtrlCanCtrlRef")
}

fn physical_bindings(
    graph: &Graph,
    composition: &super::composition::CompositionContract,
    diagnostic: Option<&super::diagnostic::DiagnosticTransportContract>,
) -> Result<BTreeMap<usize, usize>, Vec<PlanDiagnostic>> {
    let mut endpoints = composition
        .network_endpoints
        .iter()
        .map(|endpoint| {
            (
                endpoint.transport.can_if_pdu.as_str(),
                endpoint.channel.as_str(),
                endpoint.transport.receive,
            )
        })
        .collect::<Vec<_>>();
    if let Some(tp) = diagnostic {
        endpoints.push((&tp.can_if_rx_pdu, &tp.physical_channel, true));
        endpoints.push((&tp.can_if_tx_pdu, &tp.physical_channel, false));
    }
    let mut by_driver = BTreeMap::new();
    let mut by_channel = BTreeMap::new();
    for (pdu, channel, receive) in endpoints {
        let pdu = graph.index(pdu)?;
        let channel = graph.index(channel)?;
        let controller = route_controller(graph, pdu, receive)?;
        if by_driver
            .insert(controller, channel)
            .is_some_and(|prior| prior != channel)
            || by_channel
                .insert(channel, controller)
                .is_some_and(|prior| prior != controller)
        {
            return Err(fail(graph, pdu, "CONTROLLER_MAPPING"));
        }
    }
    Ok(by_driver)
}

pub(super) fn inspect(
    graph: &Graph,
    schedule: &ScheduleContract,
    composition: &super::composition::CompositionContract,
    diagnostic: Option<&super::diagnostic::DiagnosticTransportContract>,
) -> Result<CommunicationRuntimeContract, Vec<PlanDiagnostic>> {
    let partition = partition(graph, composition)?;
    let collection = one(graph, "EcucPduCollection")?;
    let pdu_id_type = integer_type(graph, collection, "PduIdTypeEnum")?;
    let pdu_length_type = integer_type(graph, collection, "PduLengthTypeEnum")?;
    // Every configured handle must fit its actual public communication type.
    let identifiers = [
        "CanIfRxPduId",
        "CanIfTxPduId",
        "CanTpRxNPduId",
        "CanTpRxNSduId",
        "CanTpTxFcNPduConfirmationPduId",
        "CanTpRxFcNPduId",
        "CanTpTxNPduConfirmationPduId",
        "CanTpTxNSduId",
        "DcmDslProtocolRxPduId",
        "DcmDslTxConfirmationPduId",
        "PduRSourcePduHandleId",
        "PduRDestPduHandleId",
    ];
    for container in graph.of_kind("ECUC-CONTAINER-VALUE") {
        for name in identifiers {
            for text in values(graph, container, name, false) {
                if text
                    .parse::<u32>()
                    .ok()
                    .filter(|id| *id <= pdu_id_type.maximum())
                    .is_none()
                {
                    return Err(fail(graph, container, "COMMUNICATION_HANDLE_RANGE"));
                }
            }
        }
        for text in values(graph, container, "PduLength", false) {
            if text
                .parse::<u32>()
                .ok()
                .filter(|length| *length <= pdu_length_type.maximum())
                .is_none()
            {
                return Err(fail(graph, container, "COMMUNICATION_LENGTH_RANGE"));
            }
        }
    }
    let driver_controllers = numbered(
        graph,
        "CanController",
        "CanControllerId",
        u16::from(u8::MAX),
        "CAN_CONTROLLER_ID",
    )?;
    let mut can_if_bindings = BTreeMap::new();
    for (id, abstract_controller) in numbered(
        graph,
        "CanIfCtrlCfg",
        "CanIfCtrlId",
        u16::from(u8::MAX),
        "CAN_CONTROLLER_ID",
    )? {
        let driver = reference(graph, abstract_controller, "CanIfCtrlCanCtrlRef")?;
        if !definition_is(graph, driver, "CanController")
            || can_if_bindings
                .insert(driver, (id, abstract_controller))
                .is_some()
        {
            return Err(fail(graph, abstract_controller, "CONTROLLER_MAPPING"));
        }
    }
    let general = one(graph, "CanGeneral")?;
    if !matches!(
        value(graph, general, "CanDevErrorDetect", false),
        Some("false" | "0")
    ) {
        return Err(fail(graph, general, "CAN_FEATURE_UNSUPPORTED"));
    }
    if !matches!(
        value(graph, general, "CanVersionInfoApi", false),
        None | Some("false" | "0")
    ) {
        return Err(fail(graph, general, "CAN_FEATURE_UNSUPPORTED"));
    }
    let rw = one(graph, "CanMainFunctionRWPeriods")?;
    let rw_path = graph.elements[rw].object.as_str();
    let mut hardware = Vec::new();
    for (handle, index) in numbered(
        graph,
        "CanHardwareObject",
        "CanObjectId",
        u16::MAX,
        "CAN_HARDWARE_HANDLES",
    )? {
        if value(graph, index, "CanMainFunctionRWPeriodRef", true) != Some(rw_path) {
            return Err(fail(graph, index, "CAN_POLLING_REFERENCE"));
        }
        if !matches!(
            value(graph, index, "CanTriggerTransmitEnable", false),
            None | Some("false" | "0")
        ) {
            return Err(fail(graph, index, "CAN_FEATURE_UNSUPPORTED"));
        }
        let receive = match value(graph, index, "CanObjectType", false) {
            Some("RECEIVE") => true,
            Some("TRANSMIT") => false,
            _ => return Err(fail(graph, index, "CAN_HARDWARE_HANDLES")),
        };
        let fd_padding = match value(graph, index, "CanFdPaddingValue", false) {
            None => 0,
            Some(value) if !receive => crate::definitions::unsigned_integer(value)
                .and_then(|padding| u8::try_from(padding).ok())
                .ok_or_else(|| fail(graph, index, "CAN_FD_PADDING"))?,
            Some(_) => return Err(fail(graph, index, "CAN_FD_PADDING")),
        };
        let id_type = match value(graph, index, "CanIdType", false) {
            Some(format @ ("STANDARD" | "EXTENDED" | "MIXED")) => format.to_owned(),
            _ => return Err(fail(graph, index, "CAN_ID_CONFLICT")),
        };
        let controller = reference(graph, index, "CanControllerRef")?;
        if !definition_is(graph, controller, "CanController") {
            return Err(fail(graph, index, "CONTROLLER_MAPPING"));
        }
        hardware.push(CanHardwareContract {
            path: graph.elements[index].object.clone(),
            handle,
            controller: graph.elements[controller].object.clone(),
            receive,
            id_type,
            fd_padding,
        });
    }
    for (symbol, configured) in [
        (
            "Can_MainFunction_Read",
            period(graph, rw, "CanMainFunctionPeriod", schedule)?,
        ),
        (
            "Can_MainFunction_Write",
            period(graph, rw, "CanMainFunctionPeriod", schedule)?,
        ),
        (
            "Can_MainFunction_Mode",
            period(graph, general, "CanMainFunctionModePeriod", schedule)?,
        ),
        (
            "Can_MainFunction_BusOff",
            period(graph, general, "CanMainFunctionBusoffPeriod", schedule)?,
        ),
    ] {
        if !schedule.entities.iter().any(|entity| {
            !entity.application && entity.symbol == symbol && entity.period_ms == configured
        }) {
            return Err(fail(graph, general, "CAN_POLLING_TIMEBASE"));
        }
    }
    let ecu = graph.of_kind("ECU-INSTANCE");
    let [ecu] = ecu.as_slice() else {
        return Err(fail(graph, general, "CONTROLLER_MAPPING"));
    };
    let selected = super::component::selected_channels(graph, *ecu);
    let physical = physical_bindings(graph, composition, diagnostic)?;
    let mut networks = BTreeSet::new();
    let mut channels = Vec::new();
    for (controller_id, controller) in driver_controllers {
        let (can_if_controller_id, can_if_controller) = can_if_bindings
            .get(&controller)
            .ok_or_else(|| fail(graph, controller, "CONTROLLER_MAPPING"))?;
        let physical_channel = physical
            .get(&controller)
            .copied()
            .filter(|channel| selected.contains(channel))
            .ok_or_else(|| fail(graph, controller, "CONTROLLER_MAPPING"))?;
        let network = graph
            .ancestor(physical_channel, "CAN-CLUSTER")
            .ok_or_else(|| fail(graph, physical_channel, "CONTROLLER_MAPPING"))?;
        if !networks.insert(network) {
            return Err(fail(graph, physical_channel, "CONTROLLER_MAPPING"));
        }
        let connectors = graph
            .descendants(physical_channel, "COMMUNICATION-CONNECTOR-REF")
            .into_iter()
            .filter_map(|reference| graph.objects.get(&graph.elements[reference].text).copied())
            .filter(|connector| graph.within(*connector, *ecu))
            .collect::<Vec<_>>();
        let [connector] = connectors.as_slice() else {
            return Err(fail(graph, physical_channel, "CONTROLLER_MAPPING"));
        };
        let (nominal_bitrate, data_bitrate, brs) = bitrates(graph, controller)?;
        let rates = graph.descendants(network, "BAUDRATE");
        if rates.len() != 1
            || graph.elements[rates[0]].text.parse::<u32>().ok() != Some(nominal_bitrate)
        {
            return Err(fail(graph, controller, "BAUDRATE_CONFLICT"));
        }
        for name in [
            "CanRxProcessing",
            "CanTxProcessing",
            "CanBusoffProcessing",
            "CanWakeupProcessing",
        ] {
            if value(graph, controller, name, false) != Some("POLLING") {
                return Err(fail(graph, controller, "CAN_PROCESSING"));
            }
        }
        let path = &graph.elements[controller].object;
        let receive = hardware
            .iter()
            .find(|hoh| &hoh.controller == path && hoh.receive)
            .ok_or_else(|| fail(graph, controller, "CAN_HARDWARE_HANDLES"))?;
        let transmit = hardware
            .iter()
            .find(|hoh| &hoh.controller == path && !hoh.receive)
            .ok_or_else(|| fail(graph, controller, "CAN_HARDWARE_HANDLES"))?;
        channels.push(CanRuntimeContract {
            physical_channel: graph.elements[physical_channel].object.clone(),
            network: graph.elements[network].object.clone(),
            connector: graph.elements[*connector].object.clone(),
            controller: path.clone(),
            controller_id: controller_id as u8,
            can_if_controller: graph.elements[*can_if_controller].object.clone(),
            can_if_controller_id: *can_if_controller_id as u8,
            receive_hardware: receive.path.clone(),
            receive_id_type: receive.id_type.clone(),
            receive_handle: receive.handle,
            transmit_hardware: transmit.path.clone(),
            transmit_id_type: transmit.id_type.clone(),
            transmit_handle: transmit.handle,
            read_write_period: rw_path.into(),
            read_write_period_ms: period(graph, rw, "CanMainFunctionPeriod", schedule)?,
            busoff_period_ms: period(graph, general, "CanMainFunctionBusoffPeriod", schedule)?,
            mode_period_ms: period(graph, general, "CanMainFunctionModePeriod", schedule)?,
            receive_polling: true,
            transmit_polling: true,
            busoff_polling: true,
            wakeup_polling: true,
            nominal_bitrate,
            data_bitrate,
            brs,
            fd_padding: transmit.fd_padding,
        });
    }
    if channels.len() != selected.len() || channels.is_empty() {
        return Err(fail(graph, general, "CONTROLLER_MAPPING"));
    }
    Ok(CommunicationRuntimeContract {
        partition,
        pdu_collection: graph.elements[collection].object.clone(),
        pdu_id_type,
        pdu_length_type,
        can: channels[0].clone(),
        channels,
        hardware,
    })
}

/// ECU-only CAN selection; component contracts do not consume driver configuration.
pub(super) fn profile_header(
    communication: &CommunicationRuntimeContract,
    comm_channel_count: usize,
    comm_user_count: usize,
    com_pdu_count: usize,
    can_if_counts: [usize; 3],
) -> (String, Vec<u8>) {
    let can = &communication.can;
    let profile = super::c_layout::commented_defines(
        &[
            (
                "CAN_CONTROLLER_CAPACITY",
                format!("{}u", communication.channels.len()),
            ),
            (
                "CAN_HOH_CAPACITY",
                format!("{}u", communication.hardware.len()),
            ),
            ("COM_MAX_PDUS", format!("{com_pdu_count}u")),
            ("COMM_USER_CAPACITY", format!("{comm_user_count}u")),
            ("COMM_CHANNEL_CAPACITY", format!("{comm_channel_count}u")),
            (
                "CANIF_CONTROLLER_CAPACITY",
                format!("{}u", can_if_counts[0]),
            ),
            ("CANIF_RX_CAPACITY", format!("{}u", can_if_counts[1])),
            ("CANIF_TX_CAPACITY", format!("{}u", can_if_counts[2].max(1))),
            ("CAN_HOST_QUEUED_COMPLETION", "STD_ON".to_owned()),
            ("CAN_RX_POLLING", "STD_ON".to_owned()),
            ("CAN_TX_POLLING", "STD_ON".to_owned()),
            ("CAN_BUSOFF_POLLING", "STD_ON".to_owned()),
            (
                "CAN_CONTROLLER_ID",
                super::symbolic_names::name("Can", "CanController", &can.controller),
            ),
            (
                "CAN_CANIF_CONTROLLER_ID",
                super::symbolic_names::name("CanIf", "CanIfCtrlCfg", &can.can_if_controller),
            ),
            (
                "CAN_RX_ID_TYPE",
                format!("CAN_HOH_ID_{}", can.receive_id_type),
            ),
            (
                "CAN_TX_ID_TYPE",
                format!("CAN_HOH_ID_{}", can.transmit_id_type),
            ),
            (
                "CAN_TX_HOH",
                super::symbolic_names::name("Can", "CanHardwareObject", &can.transmit_hardware),
            ),
            (
                "CAN_RX_HOH",
                super::symbolic_names::name("Can", "CanHardwareObject", &can.receive_hardware),
            ),
        ],
        "CAN profile.",
    );
    (
        "include/Ecu_ProfileConfig.h".into(),
        format!(
            r#"/** @file Generated communication selection. */
{guard_start}#include "Std_Types.h"
#include "Can_Cfg.h"
#include "CanIf_Cfg.h"
{checks}/** ECUC CanRxProcessing, CanTxProcessing and CanBusoffProcessing select POLLING.
 * The generated host target owns queued completion and the controller/HOH handles.
 */
{profile}#endif
"#,
            checks = [
                ("Std_Types.h", "STD_TYPES"),
                ("Can_Cfg.h", "CAN"),
                ("CanIf_Cfg.h", "CANIF")
            ]
            .into_iter()
            .map(|(h, p)| c_layout::release_check("Ecu_ProfileConfig.h", h, p))
            .collect::<String>(),
            guard_start = c_layout::header_start("ECU_PROFILE_CONFIG_H")
        )
        .into_bytes(),
    )
}

/// One producer for the selected standard communication widths.
pub(super) fn type_headers(communication: &CommunicationRuntimeContract) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    let integer = |kind: super::CommunicationIntegerType| match kind {
        super::CommunicationIntegerType::Uint8 => "uint8",
        super::CommunicationIntegerType::Uint16 => "uint16",
        super::CommunicationIntegerType::Uint32 => "uint32",
    };
    files.push((
        "include/ComStack_Cfg.h",
        format!(
            r#"/** @file EcuC-selected communication widths. */
{guard_start}#include "ComStack_Version.h"
#include "Platform_Types.h"
/** PDU handle width selected by EcuC PduIdTypeEnum. */
typedef {} PduIdType;
/** PDU byte-count width selected by EcuC PduLengthTypeEnum. */
typedef {} PduLengthType;
#endif
"#,
            integer(communication.pdu_id_type),
            integer(communication.pdu_length_type),
            guard_start = c_layout::header_start("COMSTACK_CFG_H")
        ),
    ));
    files
        .into_iter()
        .map(|(path, text)| (path.into(), text.into_bytes()))
        .collect()
}
