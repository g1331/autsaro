use super::catalog::{SymbolContract, c_identifier};
use super::component::c_name;
use super::configuration::{ConfigurationRecord, EventAssignment};
use super::diagnostic::DiagnosticContract;
use super::routing::PduRoute;
use super::schedule::ScheduleContract;
use super::{
    ContractArgument, DiagnosticCategory, FORMAT_VERSION, InputInspection, InputSource,
    ObjectIdentity, PlanDiagnostic, SignalChannel, SourceIdentity, catalog, configuration,
    diagnostic, routing, schedule,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandleAssignment {
    pub domain: String,
    pub path: String,
    pub handle: u32,
    pub c_name: String,
}

/// Serialized description of a W2 plan. Future definition owners remain
/// explicit: this description is not evidence of a W3 runtime link.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDescription {
    pub format_version: u32,
    pub profile: String,
    pub sources: Vec<SourceIdentity>,
    pub objects: Vec<ObjectIdentity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composition: Option<super::composition::CompositionContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub com_runtime: Option<super::com::ComRuntimeContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub communication_runtime: Option<super::can_runtime::CommunicationRuntimeContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode_runtime: Option<super::mode::ModeRuntimeContract>,
    pub schedule: ScheduleContract,
    pub signals: Vec<SignalChannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<DiagnosticContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic_transport: Option<super::diagnostic::DiagnosticTransportContract>,
    pub routes: Vec<PduRoute>,
    pub configuration: Vec<ConfigurationRecord>,
    pub events: Vec<EventAssignment>,
    pub handles: Vec<HandleAssignment>,
    pub symbols: Vec<SymbolContract>,
    pub runtime_sources: BTreeMap<String, String>,
    pub validation_dependencies: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_set_identity: Option<crate::project_model::RuleSetIdentity>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required_extension_definitions: Vec<crate::project_model::ExtensionDefinitionIdentity>,
}

/// All fields are private and construction only follows the checked path.
/// Consumers borrow its description and original source bytes; there is no
/// mutable accessor, unchecked constructor or Deserialize implementation.
pub struct ValidatedIntegrationPlan {
    description: PlanDescription,
    inputs: Vec<InputSource>,
}

impl ValidatedIntegrationPlan {
    pub fn description(&self) -> &PlanDescription {
        &self.description
    }
    pub fn sources(&self) -> &[InputSource] {
        &self.inputs
    }
}

// Both normal definition validation and plan construction consume this closure.
pub(super) struct CompositionInputs {
    com_runtime: Option<super::com::ComRuntimeContract>,
    communication_runtime: super::can_runtime::CommunicationRuntimeContract,
    mode_runtime: super::mode::ModeRuntimeContract,
    composition: super::composition::CompositionContract,
    schedule: ScheduleContract,
    signals: Vec<SignalChannel>,
    diagnostic: Option<DiagnosticContract>,
    diagnostic_transport: Option<super::diagnostic::DiagnosticTransportContract>,
    routes: Vec<PduRoute>,
    configuration: configuration::Configuration,
    handles: Vec<HandleAssignment>,
    symbols: Vec<SymbolContract>,
}

pub(super) fn inspect_composition(
    graph: &super::graph::Graph,
    definition_catalog: Option<&crate::definitions::DefinitionCatalog>,
) -> Result<CompositionInputs, Vec<PlanDiagnostic>> {
    let composition = super::composition::inspect(graph)?;
    let bindings: Vec<_> = composition
        .components
        .iter()
        .map(|component| (component.instance.as_str(), component.behavior.as_str()))
        .collect();
    let periodic: Vec<_> = composition
        .components
        .iter()
        .flat_map(|component| &component.runnables)
        .filter(|runnable| runnable.period_ms.is_some())
        .filter_map(|runnable| runnable.event.clone())
        .collect();
    let servers: Vec<_> = composition
        .components
        .iter()
        .flat_map(|component| &component.runnables)
        .filter(|runnable| runnable.period_ms.is_none() && runnable.event.is_some())
        .map(|runnable| runnable.path.clone())
        .collect();
    let mut schedule = schedule::inspect_events(graph, &bindings, &periodic, &servers, true)?;
    let signals: Vec<_> = composition
        .network_endpoints
        .iter()
        .map(|endpoint| endpoint.transport.clone())
        .collect();
    let diagnostic = if graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .iter()
        .any(|index| schedule::definition_is(graph, *index, "DcmDspData"))
    {
        let data = graph
            .of_kind("ECUC-CONTAINER-VALUE")
            .into_iter()
            .find(|index| schedule::definition_is(graph, *index, "DcmDspData"))
            .unwrap();
        let service_name = graph.text(data, "SHORT-NAME").unwrap_or("");
        let clients: Vec<_> = composition
            .connections
            .iter()
            .filter(|connection| {
                if !connection.service {
                    return false;
                }
                let requester = composition
                    .components
                    .iter()
                    .find(|component| component.instance == connection.requester.instance)
                    .unwrap();
                let operation = requester
                    .operations
                    .iter()
                    .find(|operation| {
                        operation.port == connection.requester.port
                            && operation.operation == connection.requester.member
                    })
                    .unwrap();
                let interface = *graph.objects.get(&operation.interface).unwrap();
                graph.elements[*graph.objects.get(&requester.component).unwrap()].tag
                    == "SERVICE-SW-COMPONENT-TYPE"
                    && matches!(graph.text(interface, "IS-SERVICE"), Some("true" | "1"))
                    && graph.text(interface, "SERVICE-KIND")
                        == Some("DIAGNOSTIC-COMMUNICATION-MANAGER")
                    && requester.runnables.iter().any(|runnable| {
                        runnable.event.is_none() && operation.callers.contains(&runnable.path)
                    })
                    && connection.requester.port.rsplit('/').next()
                        == Some(format!("DataServices_{service_name}").as_str())
                    && operation.interface.rsplit('/').next()
                        == Some(format!("DataServices_{service_name}").as_str())
                    && operation.operation.rsplit('/').next() == Some("ReadData")
            })
            .collect();
        if clients.len() != 1 {
            return Err(vec![graph.diagnostic(data, DiagnosticCategory::Input, "SERVICE_CLIENT_MISSING", crate::product_message!("backend.integration.composition.contract_invalid", "code" => "SERVICE_CLIENT_MISSING"), crate::product_message!("backend.integration.composition.repair_contract"))]);
        }
        let client = clients[0];
        let component = composition
            .components
            .iter()
            .find(|component| component.instance == client.provider.instance)
            .unwrap();
        let operation = component
            .operations
            .iter()
            .find(|operation| {
                operation.port == client.provider.port
                    && operation.operation == client.provider.member
            })
            .unwrap();
        if operation.arguments.len() != 1
            || operation.arguments[0].direction != "OUT"
            || !composition
                .array_types
                .values()
                .any(|native| native == &operation.arguments[0].native_type)
        {
            return Err(vec![graph.diagnostic(data, DiagnosticCategory::Input, "SERVICE_TYPE_CONFLICT", crate::product_message!("backend.integration.composition.contract_invalid", "code" => "SERVICE_TYPE_CONFLICT"), crate::product_message!("backend.integration.composition.repair_contract"))]);
        }
        Some(diagnostic::inspect_service(
            graph,
            &component.component,
            service_name,
            &client.requester.port,
        )?)
    } else {
        None
    };
    // Only the checked DCM requester may be called from BSW without an event.
    for component in &composition.components {
        for runnable in component
            .runnables
            .iter()
            .filter(|runnable| runnable.event.is_none())
        {
            if !diagnostic.as_ref().is_some_and(|diagnostic| {
                component.operations.iter().any(|operation| {
                    operation.read
                        && operation.port == diagnostic.client_port
                        && operation.callers == [runnable.path.clone()]
                })
            }) {
                return Err(vec![graph.diagnostic(*graph.objects.get(&runnable.path).unwrap(), DiagnosticCategory::Input, "SERVICE_CLIENT_MISSING", crate::product_message!("backend.integration.composition.contract_invalid", "code" => "SERVICE_CLIENT_MISSING"), crate::product_message!("backend.integration.composition.repair_contract"))]);
            }
        }
    }
    if let Some(diagnostic) = &diagnostic {
        let service = composition
            .components
            .iter()
            .find(|component| {
                component
                    .operations
                    .iter()
                    .any(|operation| operation.read && operation.port == diagnostic.client_port)
            })
            .unwrap();
        if service.diagnostic_session_port.is_none()
            || service.diagnostic_mode_ports.len() != composition.dcm_modes.len()
            || composition.components.iter().any(|component| {
                component.instance != service.instance
                    && component.diagnostic_session_port.is_some()
            })
        {
            return Err(vec![graph.diagnostic(*graph.objects.get(&service.component).unwrap(), DiagnosticCategory::Input, "SERVICE_TYPE_CONFLICT", crate::product_message!("backend.integration.composition.contract_invalid", "code" => "SERVICE_TYPE_CONFLICT"), crate::product_message!("backend.integration.composition.repair_contract"))]);
        }
    } else if composition
        .components
        .iter()
        .any(|component| component.diagnostic_session_port.is_some())
    {
        return Err(vec![graph.diagnostic(*graph.objects.get(&composition.composition).unwrap(), DiagnosticCategory::Input, "SERVICE_TYPE_CONFLICT", crate::product_message!("backend.integration.composition.contract_invalid", "code" => "SERVICE_TYPE_CONFLICT"), crate::product_message!("backend.integration.composition.repair_contract"))]);
    }
    let com_runtime = super::com::inspect(graph, &signals, &schedule)?;
    if com_runtime.is_none() {
        return Err(vec![graph.diagnostic(
            *graph.objects.get(&composition.composition).unwrap(),
            DiagnosticCategory::Input,
            "COM_CONFIGURATION",
            crate::product_message!("backend.integration.composition.contract_invalid", "code" => "COM_CONFIGURATION"),
            crate::product_message!("backend.integration.composition.repair_contract"),
        )]);
    }
    let diagnostic_transport = diagnostic::inspect_transport(
        graph,
        *graph.objects.get(&composition.composition).unwrap(),
        true,
    )?;
    let routed_transport = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .any(|index| {
            [
                ("PduRSrcPdu", "PduRSrcPduRef"),
                ("PduRDestPdu", "PduRDestPduRef"),
            ]
            .iter()
            .any(|(kind, reference)| {
                schedule::definition_is(graph, index, kind)
                    && schedule::value(graph, index, reference, true).is_some_and(|pdu| {
                        pdu == diagnostic_transport.rx_sdu || pdu == diagnostic_transport.tx_sdu
                    })
            })
        });
    let diagnostic_transport =
        (routed_transport || diagnostic.is_some()).then_some(diagnostic_transport);
    let routes = match &diagnostic_transport {
        Some(transport) => routing::inspect_transport(graph, &signals, transport)?,
        None => routing::inspect_optional(graph, &signals, None)?,
    };
    let communication_runtime = super::can_runtime::inspect(graph, &schedule, &composition)?;
    let mode_runtime = super::mode::inspect(graph, &schedule)?;
    let configuration = match definition_catalog {
        Some(catalog) => configuration::inspect_native(graph, catalog)?,
        None => configuration::inspect_native(
            graph,
            &crate::definitions::DefinitionCatalog::builtin().map_err(|message| {
                vec![PlanDiagnostic::dependency(
                    "BUILTIN_DEFINITIONS",
                    message,
                    crate::product_message!(
                        "backend.integration.mod.repair_installed_product_rule_inventory"
                    ),
                )]
            })?,
        )?,
    };
    let selected_runtime = catalog::bsw_catalog(com_runtime.as_ref().unwrap(), &mode_runtime);
    let mut symbols = catalog::inspect(graph, &selected_runtime)?;
    symbols.extend(super::composition::symbols(&composition));
    for symbol in ["Rte_COMCbk", "Rte_COMCbkRxTOut"] {
        symbols.push(SymbolContract {
            symbol: symbol.into(),
            return_type: "void".into(),
            arguments: vec![ContractArgument {
                name: "HandleId".into(),
                native_type: "CbkHandleIdType".into(),
                direction: "IN".into(),
            }],
            declaration_owner: "include/Rte_Com.h".into(),
            definition_owner: "src/Rte.c".into(),
            consumers: com_runtime
                .as_ref()
                .unwrap()
                .receptions
                .iter()
                .map(|reception| reception.user_signal.clone())
                .collect(),
        });
    }
    if diagnostic_transport.is_none() {
        // Preserve the raw source descriptors, but the effective selected plan
        // schedules only initialized producers with an actual routing path.
        schedule.entities.retain(|entity| {
            !matches!(
                entity.symbol.as_str(),
                "CanTp_MainFunction" | "Dcm_MainFunction"
            )
        });
    }
    let mut names = BTreeSet::new();
    for symbol in &symbols {
        if !c_identifier(&symbol.symbol) || !names.insert(symbol.symbol.clone()) {
            return Err(vec![graph.diagnostic(*graph.objects.get(&composition.composition).unwrap(), DiagnosticCategory::Input, "SYMBOL_PRODUCER_DUPLICATE", crate::product_message!("backend.integration.plan.external_c_symbol_invalid_or_multiple_producers", "value0" => symbol.symbol), crate::product_message!("backend.integration.plan.resolve_c_symbol_and_producer_conflicts"))]);
        }
    }
    let mut handles = Vec::new();
    for (domain, paths) in [
        ("os_task", vec![schedule.task.clone()]),
        ("os_counter", vec![schedule.counter.clone()]),
        (
            "os_event",
            configuration
                .events
                .iter()
                .map(|event| event.path.clone())
                .collect(),
        ),
        (
            "os_alarm",
            schedule
                .entities
                .iter()
                .filter(|entity| entity.expiry_point.is_none())
                .map(|entity| entity.alarm.clone())
                .collect(),
        ),
        (
            "os_schedule_table",
            schedule
                .entities
                .iter()
                .filter_map(|entity| entity.schedule_table.clone())
                .collect(),
        ),
        (
            "os_expiry_point",
            schedule
                .entities
                .iter()
                .filter_map(|entity| entity.expiry_point.clone())
                .collect(),
        ),
        (
            "com_signal",
            signals
                .iter()
                .map(|signal| signal.com_signal.clone())
                .collect(),
        ),
    ] {
        let paths: BTreeSet<String> = paths.into_iter().collect();
        for (handle, path) in paths.into_iter().enumerate() {
            let native = format!("{}_{}", domain, c_name(path.trim_start_matches('/')));
            if !names.insert(native.clone()) {
                return Err(vec![graph.diagnostic(
                    *graph.objects.get(&path).unwrap(),
                    DiagnosticCategory::Input,
                    "SYMBOL_NORMALIZATION_COLLISION",
                    crate::product_message!(
                        "backend.integration.plan.normalized_object_c_identifier_collision"
                    ),
                    crate::product_message!(
                        "backend.integration.plan.rename_object_to_avoid_c_normalization_collision"
                    ),
                )]);
            }
            handles.push(HandleAssignment {
                domain: domain.into(),
                path,
                c_name: native,
                handle: handle as u32,
            });
        }
    }
    let mut can_ids = BTreeSet::new();
    let mut can_handles = BTreeSet::new();
    for (object, id, handle, receive) in signals
        .iter()
        .map(|signal| {
            (
                signal.can_if_pdu.as_str(),
                signal.can_id,
                signal.can_if_handle,
                signal.receive,
            )
        })
        .chain(diagnostic_transport.iter().flat_map(|diagnostic| {
            [
                (
                    diagnostic.rx_sdu.as_str(),
                    diagnostic.request_can_id,
                    diagnostic.request_can_if_handle,
                    true,
                ),
                (
                    diagnostic.tx_sdu.as_str(),
                    diagnostic.response_can_id,
                    diagnostic.response_can_if_handle,
                    false,
                ),
            ]
        }))
    {
        // The delivered ComStack_Types.h defines PduIdType as uint8_t: a wider
        // handle is a target width limit, not an identifier collision.
        if handle > u16::from(u8::MAX) {
            return Err(vec![graph.diagnostic(
                *graph.objects.get(object).unwrap(),
                DiagnosticCategory::Unsupported,
                "CANIF_HANDLE_WIDTH",
                crate::product_message!("backend.integration.plan.canif_pdu_handle_out_of_range"),
                crate::product_message!("backend.integration.plan.use_uint8_canif_handles"),
            )]);
        }
        if !can_ids.insert(id) || !can_handles.insert((receive, handle)) {
            return Err(vec![graph.diagnostic(
                *graph.objects.get(object).unwrap(),
                DiagnosticCategory::Input,
                "CAN_ID_CONFLICT",
                crate::product_message!(
                    "backend.integration.plan.selected_channel_identifier_or_handle_collision"
                ),
                crate::product_message!(
                    "backend.integration.plan.assign_distinct_can_identifiers_and_pdu_handles"
                ),
            )]);
        }
    }
    symbols.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    Ok(CompositionInputs {
        com_runtime,
        communication_runtime,
        mode_runtime,
        composition,
        schedule,
        signals,
        diagnostic,
        diagnostic_transport,
        routes,
        configuration,
        handles,
        symbols,
    })
}

fn assemble_composition(
    inspection: InputInspection,
) -> Result<ValidatedIntegrationPlan, Vec<PlanDiagnostic>> {
    let CompositionInputs {
        com_runtime,
        communication_runtime,
        mode_runtime,
        composition,
        schedule,
        signals,
        diagnostic,
        diagnostic_transport,
        routes,
        configuration,
        handles,
        symbols,
    } = inspect_composition(&inspection.graph, inspection.definition_catalog.as_ref())?;
    let objects = inspection.objects();
    Ok(ValidatedIntegrationPlan {
        description: PlanDescription {
            format_version: FORMAT_VERSION,
            profile: super::composition::PROFILE.into(),
            sources: inspection.identities,
            objects,
            composition: Some(composition),
            com_runtime,
            communication_runtime: Some(communication_runtime),
            mode_runtime: Some(mode_runtime),
            schedule,
            signals,
            diagnostic,
            diagnostic_transport,
            routes,
            configuration: configuration.records,
            events: configuration.events,
            handles,
            symbols,
            runtime_sources: catalog::runtime_source_identities(),
            validation_dependencies: inspection.validation_dependencies,
            rule_set_identity: inspection.rule_set_identity,
            required_extension_definitions: inspection.required_extension_definitions,
        },
        inputs: inspection.sources,
    })
}

/// Construct the same finite target plan using product-native validation.
pub fn build_plan_native(
    sources: &[InputSource],
    catalog: &crate::definitions::DefinitionCatalog,
) -> Result<ValidatedIntegrationPlan, Vec<PlanDiagnostic>> {
    assemble_composition(super::inspect_inputs_native(sources, catalog)?)
}
