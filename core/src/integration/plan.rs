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
    pub component: ComponentContract,
    pub schedule: ScheduleContract,
    pub signals: Vec<SignalChannel>,
    pub diagnostic: DiagnosticContract,
    pub routes: Vec<PduRoute>,
    pub configuration: Vec<ConfigurationRecord>,
    pub events: Vec<EventAssignment>,
    pub handles: Vec<HandleAssignment>,
    pub symbols: Vec<SymbolContract>,
    pub runtime_sources: BTreeMap<String, String>,
    pub validation_dependencies: BTreeMap<String, String>,
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
    let component = component::inspect(graph)?;
    let schedule = schedule::inspect(graph, &component)?;
    let signals = communication::inspect(graph, &component)?;
    let diagnostic = diagnostic::inspect(graph, &component)?;
    let routes = routing::inspect(graph, &signals, &diagnostic)?;
    let configuration = configuration::inspect(graph)?;
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
        return Err(vec![graph.diagnostic(index, DiagnosticCategory::Input, "SYMBOL_NORMALIZATION_COLLISION",
            "The generated service array type collides with a native C type, keyword or existing scalar contract.",
            "Give the service implementation type a distinct C name without changing its four-byte OUT shape.")]);
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
            "story-4.11:Rte.h",
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
    let mut names = BTreeSet::new();
    for symbol in &symbols {
        if !c_identifier(&symbol.symbol) || !names.insert(symbol.symbol.clone()) {
            return Err(vec![graph.diagnostic(context, DiagnosticCategory::Input, "SYMBOL_PRODUCER_DUPLICATE",
                format!("External C symbol {} is invalid or has multiple producers.", symbol.symbol),
                "Use distinct valid declared C symbols and resolve the competing source/stage responsibility before generating.")]);
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
                .map(|entity| entity.alarm.clone())
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
                return Err(vec![graph.diagnostic(index, DiagnosticCategory::Input, "SYMBOL_NORMALIZATION_COLLISION",
                    "Two distinct complete object paths normalize to the same generated C identifier.",
                    "Rename one affected object or parent so both complete identities remain distinct after C normalization.")]);
            }
        }
    }
    let rx = signals.iter().find(|signal| signal.receive).unwrap();
    let tx = signals.iter().find(|signal| !signal.receive).unwrap();
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
        return Err(vec![graph.diagnostic(context, DiagnosticCategory::Input, "CAN_ID_CONFLICT",
            "Selected channels share a physical identifier or a handle within the same CanIf direction domain.",
            "Assign distinct physical CAN identifiers and distinct PDU handles within each Rx/Tx namespace.")]);
    }
    symbols.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    let objects = inspection.objects();
    let description = PlanDescription {
        format_version: FORMAT_VERSION,
        profile: PROFILE.into(),
        sources: inspection.identities,
        objects,
        component,
        schedule,
        signals,
        diagnostic,
        routes,
        configuration: configuration.records,
        events: configuration.events,
        handles,
        symbols,
        runtime_sources: runtime.source_identities().clone(),
        validation_dependencies: BTreeMap::from([
            ("R24-11 XSD".into(), super::XSD_SHA256.into()),
            ("R24-11 MOD".into(), super::MOD_SHA256.into()),
        ]),
    };
    Ok(ValidatedIntegrationPlan {
        description,
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
