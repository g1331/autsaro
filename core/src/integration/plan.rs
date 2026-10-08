use super::catalog::{SymbolContract, c_identifier};
use super::component::{ComponentContract, c_name};
use super::configuration::{ConfigurationRecord, EventAssignment};
use super::diagnostic::DiagnosticContract;
use super::routing::PduRoute;
use super::schedule::ScheduleContract;
use super::{
    ContractArgument, DiagnosticCategory, FORMAT_VERSION, InputInspection, InputSource,
    ObjectIdentity, PROFILE, PlanDependencies, PlanDiagnostic, RuntimeCatalog, SignalChannel,
    SourceIdentity, catalog, communication, component, configuration, diagnostic, inspect_inputs,
    routing, schedule,
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
    pub component: Option<ComponentContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multi: Option<super::multi::MultiComponentContract>,
    pub schedule: ScheduleContract,
    pub signals: Vec<SignalChannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<DiagnosticContract>,
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

impl PlanDescription {
    pub fn legacy_component(&self) -> Result<&ComponentContract, crate::message::LocalizedText> {
        self.component.as_ref().ok_or_else(|| {
            crate::product_message!("backend.integration.multi.consumer_unsupported").into()
        })
    }
    pub fn legacy_diagnostic(&self) -> Result<&DiagnosticContract, crate::message::LocalizedText> {
        self.diagnostic.as_ref().ok_or_else(|| {
            crate::product_message!("backend.integration.multi.consumer_unsupported").into()
        })
    }
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

fn staged(
    symbol: &str,
    return_type: &str,
    arguments: Vec<ContractArgument>,
    declaration: &str,
    definition: &str,
    consumer: &str,
) -> SymbolContract {
    SymbolContract {
        symbol: symbol.into(),
        return_type: return_type.into(),
        arguments,
        declaration_owner: declaration.into(),
        definition_owner: definition.into(),
        consumers: vec![consumer.into()],
    }
}

fn argument(name: &str, native_type: &str, direction: &str) -> ContractArgument {
    ContractArgument {
        name: name.into(),
        native_type: native_type.into(),
        direction: direction.into(),
    }
}

fn assemble(
    inspection: InputInspection,
    runtime: &RuntimeCatalog,
) -> Result<ValidatedIntegrationPlan, Vec<PlanDiagnostic>> {
    let graph = &inspection.graph;
    if super::multi::selected(graph) {
        return assemble_multi(inspection, runtime);
    }
    let component = component::inspect(graph)?;
    let schedule = schedule::inspect(graph, &component)?;
    let signals = communication::inspect(graph, &component)?;
    let diagnostic = diagnostic::inspect(graph, &component)?;
    let routes = routing::inspect(graph, &signals, &diagnostic)?;
    let configuration = match &inspection.definition_catalog {
        Some(catalog) => configuration::inspect_native(graph, catalog)?,
        None => configuration::inspect(graph)?,
    };
    let mut symbols = catalog::inspect(graph, runtime)?;
    let context = *graph.objects.get(&component.component).unwrap();
    let datatype = component.service.array_type.rsplit('/').next().unwrap();
    if [
        "uint8",
        "uint16",
        "uint32",
        "uint64",
        "sint8",
        "sint16",
        "sint32",
        "sint64",
        "uint8_t",
        "uint16_t",
        "uint32_t",
        "uint64_t",
        "Std_ReturnType",
        "EcuStatus",
        "void",
        "char",
        "short",
        "int",
        "long",
        "float",
        "double",
        "struct",
        "union",
        "enum",
    ]
    .contains(&c_name(datatype).as_str())
    {
        let index = *graph.objects.get(&component.service.array_type).unwrap();
        return Err(vec![graph.diagnostic(
            index,
            DiagnosticCategory::Input,
            "SYMBOL_NORMALIZATION_COLLISION",
            crate::product_message!("backend.integration.plan.service_array_type_c_name_collision"),
            crate::product_message!(
                "backend.integration.plan.rename_service_type_preserving_out_shape"
            ),
        )]);
    }
    for port in &component.data_ports {
        let argument_type = if port.read { "uint32 *" } else { "uint32" };
        symbols.push(staged(
            &port.api_symbol,
            "Std_ReturnType",
            vec![argument(
                "data",
                argument_type,
                if port.read { "OUT" } else { "IN" },
            )],
            "story-4.11:component contract header",
            "story-4.13:generated Rte.c",
            &port.path,
        ));
    }
    symbols.push(staged(
        &component.periodic_symbol,
        "void",
        Vec::new(),
        "story-4.11:component contract header",
        "story-4.15:application implementation",
        &component.periodic_runnable,
    ));
    symbols.push(staged(
        &component.service.runnable_symbol,
        "Std_ReturnType",
        vec![argument("Data", datatype, "OUT")],
        "story-4.11:component contract header",
        "story-4.15:application implementation",
        &component.service.runnable,
    ));
    symbols.push(staged(
        &component.service.client_symbol,
        "Std_ReturnType",
        vec![argument("Data", datatype, "OUT")],
        "story-4.11:service contract header",
        "story-4.13:generated Dcm service binding",
        &component.service.client_runnable,
    ));
    let client_port = component.service.client_port.rsplit('/').next().unwrap();
    let call = format!(
        "Rte_Call_{}_{}",
        c_name(client_port),
        c_name(&component.service.operation_name)
    );
    symbols.push(staged(
        &call,
        "Std_ReturnType",
        vec![argument("Data", datatype, "OUT")],
        "story-4.11:service contract header",
        "story-4.13:generated Rte.c",
        &component.service.client_port,
    ));
    for (name, direction, native_type) in [
        ("Com_SendSignal", "IN", "const void *"),
        ("Com_ReceiveSignal", "OUT", "void *"),
    ] {
        symbols.push(staged(
            name,
            "Std_ReturnType",
            vec![
                argument("SignalId", "Com_SignalIdType", "IN"),
                argument("SignalDataPtr", native_type, direction),
            ],
            "story-4.13:Com.h",
            "story-4.13:BSW signature adapter",
            &component.component,
        ));
    }
    let counter_name = c_name(schedule.counter.rsplit('/').next().unwrap());
    let mut names = BTreeSet::new();
    for operation in ["GetCounterValue", "GetElapsedValue"] {
        names.insert(format!("OsService_{counter_name}_{operation}"));
        names.insert(format!("Rte_Call_OsService_{operation}"));
    }
    for symbol in &symbols {
        if !c_identifier(&symbol.symbol) || !names.insert(symbol.symbol.clone()) {
            return Err(vec![graph.diagnostic(context, DiagnosticCategory::Input, "SYMBOL_PRODUCER_DUPLICATE",
                crate::product_message!("backend.integration.plan.external_c_symbol_invalid_or_multiple_producers", "value0" => symbol.symbol),
                crate::product_message!("backend.integration.plan.resolve_c_symbol_and_producer_conflicts"))]);
        }
    }
    let mut handles = Vec::new();
    for (domain, paths) in [
        ("os_task", vec![schedule.task.clone()]),
        ("os_counter", vec![schedule.counter.clone()]),
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
        let ordered: BTreeSet<String> = paths.into_iter().collect();
        for (handle, path) in ordered.into_iter().enumerate() {
            handles.push(HandleAssignment {
                domain: domain.into(),
                c_name: format!("{}_{}", domain, c_name(path.trim_start_matches('/'))),
                path,
                handle: handle as u32,
            });
        }
    }
    let mut handle_names = BTreeMap::new();
    for handle in &handles {
        if let Some(previous) = handle_names.insert(&handle.c_name, &handle.path) {
            if previous != &handle.path {
                let index = *graph.objects.get(&handle.path).unwrap();
                return Err(vec![graph.diagnostic(
                    index,
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
        }
    }
    let rx = signals.iter().find(|signal| signal.receive).unwrap();
    let tx = signals.iter().find(|signal| !signal.receive).unwrap();
    if [
        rx.can_if_handle,
        tx.can_if_handle,
        diagnostic.request_can_if_handle,
        diagnostic.response_can_if_handle,
    ]
    .into_iter()
    .any(|handle| handle > u16::from(u8::MAX))
    {
        return Err(vec![graph.diagnostic(
            context,
            DiagnosticCategory::Unsupported,
            "CANIF_HANDLE_WIDTH",
            crate::product_message!("backend.integration.plan.canif_pdu_handle_out_of_range"),
            crate::product_message!("backend.integration.plan.use_uint8_canif_handles"),
        )]);
    }
    if rx.can_if_handle == diagnostic.request_can_if_handle
        || tx.can_if_handle == diagnostic.response_can_if_handle
        || [
            rx.can_id,
            tx.can_id,
            diagnostic.request_can_id,
            diagnostic.response_can_id,
        ]
        .into_iter()
        .collect::<BTreeSet<_>>()
        .len()
            != 4
    {
        return Err(vec![graph.diagnostic(
            context,
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
    symbols.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    let objects = inspection.objects();
    let description = PlanDescription {
        format_version: FORMAT_VERSION,
        profile: PROFILE.into(),
        sources: inspection.identities,
        objects,
        component: Some(component),
        multi: None,
        schedule,
        signals,
        diagnostic: Some(diagnostic),
        routes,
        configuration: configuration.records,
        events: configuration.events,
        handles,
        symbols,
        runtime_sources: runtime.source_identities().clone(),
        validation_dependencies: inspection.validation_dependencies,
        rule_set_identity: inspection.rule_set_identity,
        required_extension_definitions: inspection.required_extension_definitions,
    };
    Ok(ValidatedIntegrationPlan {
        description,
        inputs: inspection.sources,
    })
}

// Both normal definition validation and plan construction consume this closure.
pub(super) struct MultiPlanInputs {
    multi: super::multi::MultiComponentContract,
    schedule: ScheduleContract,
    signals: Vec<SignalChannel>,
    diagnostic: Option<DiagnosticContract>,
    routes: Vec<PduRoute>,
    configuration: configuration::Configuration,
    handles: Vec<HandleAssignment>,
    symbols: Vec<SymbolContract>,
}

pub(super) fn inspect_multi(
    graph: &super::graph::Graph,
    definition_catalog: Option<&crate::definitions::DefinitionCatalog>,
    runtime: &RuntimeCatalog,
) -> Result<MultiPlanInputs, Vec<PlanDiagnostic>> {
    let multi = super::multi::inspect(graph)?;
    let bindings: Vec<_> = multi
        .components
        .iter()
        .map(|component| (component.instance.as_str(), component.behavior.as_str()))
        .collect();
    let periodic: Vec<_> = multi
        .components
        .iter()
        .flat_map(|component| &component.runnables)
        .filter(|runnable| runnable.period_ms.is_some())
        .filter_map(|runnable| runnable.event.clone())
        .collect();
    let servers: Vec<_> = multi
        .components
        .iter()
        .flat_map(|component| &component.runnables)
        .filter(|runnable| runnable.period_ms.is_none() && runnable.event.is_some())
        .map(|runnable| runnable.path.clone())
        .collect();
    let schedule = schedule::inspect_events(graph, &bindings, &periodic, &servers, true)?;
    let signals: Vec<_> = multi
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
        let clients: Vec<_> = multi
            .connections
            .iter()
            .filter(|connection| {
                if !connection.service {
                    return false;
                }
                let requester = multi
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
            return Err(vec![graph.diagnostic(data, DiagnosticCategory::Input, "SERVICE_CLIENT_MISSING", crate::product_message!("backend.integration.multi.contract_invalid", "code" => "SERVICE_CLIENT_MISSING"), crate::product_message!("backend.integration.multi.repair_contract"))]);
        }
        let client = clients[0];
        let component = multi
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
            || !multi
                .array_types
                .values()
                .any(|native| native == &operation.arguments[0].native_type)
        {
            return Err(vec![graph.diagnostic(data, DiagnosticCategory::Input, "SERVICE_TYPE_CONFLICT", crate::product_message!("backend.integration.multi.contract_invalid", "code" => "SERVICE_TYPE_CONFLICT"), crate::product_message!("backend.integration.multi.repair_contract"))]);
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
    for component in &multi.components {
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
                return Err(vec![graph.diagnostic(*graph.objects.get(&runnable.path).unwrap(), DiagnosticCategory::Input, "SERVICE_CLIENT_MISSING", crate::product_message!("backend.integration.multi.contract_invalid", "code" => "SERVICE_CLIENT_MISSING"), crate::product_message!("backend.integration.multi.repair_contract"))]);
            }
        }
    }
    let routes = routing::inspect_optional(graph, &signals, diagnostic.as_ref())?;
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
    let mut symbols = catalog::inspect(graph, runtime)?;
    symbols.extend(super::multi::symbols(&multi));
    let mut names = BTreeSet::new();
    for symbol in &symbols {
        if !c_identifier(&symbol.symbol) || !names.insert(symbol.symbol.clone()) {
            return Err(vec![graph.diagnostic(*graph.objects.get(&multi.composition).unwrap(), DiagnosticCategory::Input, "SYMBOL_PRODUCER_DUPLICATE", crate::product_message!("backend.integration.plan.external_c_symbol_invalid_or_multiple_producers", "value0" => symbol.symbol), crate::product_message!("backend.integration.plan.resolve_c_symbol_and_producer_conflicts"))]);
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
        .chain(diagnostic.iter().flat_map(|diagnostic| {
            [
                (
                    diagnostic.data.as_str(),
                    diagnostic.request_can_id,
                    diagnostic.request_can_if_handle,
                    true,
                ),
                (
                    diagnostic.data.as_str(),
                    diagnostic.response_can_id,
                    diagnostic.response_can_if_handle,
                    false,
                ),
            ]
        }))
    {
        if !can_ids.insert(id)
            || !can_handles.insert((receive, handle))
            || handle > u16::from(u8::MAX)
        {
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
    Ok(MultiPlanInputs {
        multi,
        schedule,
        signals,
        diagnostic,
        routes,
        configuration,
        handles,
        symbols,
    })
}

fn assemble_multi(
    inspection: InputInspection,
    runtime: &RuntimeCatalog,
) -> Result<ValidatedIntegrationPlan, Vec<PlanDiagnostic>> {
    let MultiPlanInputs {
        multi,
        schedule,
        signals,
        diagnostic,
        routes,
        configuration,
        handles,
        symbols,
    } = inspect_multi(
        &inspection.graph,
        inspection.definition_catalog.as_ref(),
        runtime,
    )?;
    let objects = inspection.objects();
    Ok(ValidatedIntegrationPlan {
        description: PlanDescription {
            format_version: FORMAT_VERSION,
            profile: super::multi::PROFILE.into(),
            sources: inspection.identities,
            objects,
            component: None,
            multi: Some(multi),
            schedule,
            signals,
            diagnostic,
            routes,
            configuration: configuration.records,
            events: configuration.events,
            handles,
            symbols,
            runtime_sources: runtime.source_identities().clone(),
            validation_dependencies: inspection.validation_dependencies,
            rule_set_identity: inspection.rule_set_identity,
            required_extension_definitions: inspection.required_extension_definitions,
        },
        inputs: inspection.sources,
    })
}

pub fn build_plan(
    sources: &[InputSource],
    dependencies: &PlanDependencies,
    runtime: &RuntimeCatalog,
) -> Result<ValidatedIntegrationPlan, Vec<PlanDiagnostic>> {
    assemble(inspect_inputs(sources, dependencies)?, runtime)
}

/// Construct the same finite target plan using product-native validation.
pub fn build_plan_native(
    sources: &[InputSource],
    catalog: &crate::definitions::DefinitionCatalog,
    runtime: &RuntimeCatalog,
) -> Result<ValidatedIntegrationPlan, Vec<PlanDiagnostic>> {
    assemble(super::inspect_inputs_native(sources, catalog)?, runtime)
}
