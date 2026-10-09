//! Source-derived contracts for the bounded flat, single-owner multi-SWC profile.
use super::component::{DataPort, c_name, implementation, milliseconds, name, one, referenced};
use super::graph::Graph;
use super::{ContractArgument, DiagnosticCategory, PlanDiagnostic, SymbolContract};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const PROFILE: &str = "singlecore-multi-swc-v1";

#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub instance: String,
    pub port: String,
    pub member: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub path: String,
    pub provider: Endpoint,
    pub requester: Endpoint,
    pub service: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub port: String,
    pub interface: String,
    pub operation: String,
    pub read: bool,
    pub api_symbol: String,
    pub implementation_symbol: String,
    pub arguments: Vec<ContractArgument>,
    pub argument_types: Vec<String>,
    pub runnable: String,
    pub callers: Vec<String>,
    pub call_points: Vec<String>,
    pub event: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Runnable {
    pub path: String,
    pub symbol: String,
    pub event: Option<String>,
    pub period_ms: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataAccess {
    pub path: String,
    pub endpoint: Endpoint,
    pub runnable: String,
    pub read: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    pub component: String,
    pub instance: String,
    pub ecu: String,
    pub ecu_mapping: String,
    pub rte_instance: String,
    pub behavior: String,
    pub header: String,
    pub interfaces: BTreeMap<String, String>,
    pub data_ports: Vec<DataPort>,
    pub data_accesses: Vec<DataAccess>,
    pub operations: Vec<Operation>,
    pub runnables: Vec<Runnable>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkEndpoint {
    pub endpoint: Endpoint,
    pub network: String,
    pub channel: String,
    pub transport: super::SignalChannel,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiComponentContract {
    pub system: String,
    pub composition: String,
    pub ecu: String,
    pub components: Vec<Component>,
    pub connections: Vec<Connection>,
    pub network_endpoints: Vec<NetworkEndpoint>,
    pub array_types: BTreeMap<String, String>,
}

fn fail(graph: &Graph, index: usize, code: &str) -> Vec<PlanDiagnostic> {
    vec![graph.diagnostic(
        index,
        DiagnosticCategory::Input,
        code,
        crate::product_message!("backend.integration.multi.contract_invalid", "code" => code),
        crate::product_message!("backend.integration.multi.repair_contract"),
    )]
}

pub(super) fn selected(graph: &Graph) -> bool {
    graph
        .of_kind("SYSTEM")
        .into_iter()
        .filter(|system| graph.text(*system, "CATEGORY") == Some("ECU_EXTRACT"))
        .flat_map(|system| graph.descendants(system, "ROOT-SW-COMPOSITION-PROTOTYPE"))
        .filter_map(|root| graph.target(root, "SOFTWARE-COMPOSITION-TREF"))
        .any(|composition| {
            graph
                .descendants(composition, "SW-COMPONENT-PROTOTYPE")
                .into_iter()
                .filter_map(|instance| graph.target(instance, "TYPE-TREF"))
                .filter(|component| {
                    graph.elements[*component].tag == "APPLICATION-SW-COMPONENT-TYPE"
                })
                .collect::<BTreeSet<_>>()
                .len()
                > 1
        })
}

fn path(graph: &Graph, index: usize) -> String {
    graph.elements[index].object.clone()
}

fn reference_descendant(
    graph: &Graph,
    index: usize,
    tag: &str,
) -> Result<usize, Vec<PlanDiagnostic>> {
    let reference = one(
        graph,
        index,
        graph.descendants(index, tag),
        "REFERENCE_UNRESOLVED",
    )?;
    graph
        .objects
        .get(&graph.elements[reference].text)
        .copied()
        .ok_or_else(|| fail(graph, reference, "REFERENCE_UNRESOLVED"))
}

pub(super) fn zero_seconds(value: &str) -> bool {
    let mantissa = match value.split_once(['e', 'E']) {
        Some((mantissa, exponent)) if exponent.parse::<i32>().is_ok() => mantissa,
        Some(_) => return false,
        None => value,
    };
    let mantissa = mantissa
        .strip_prefix('+')
        .or_else(|| mantissa.strip_prefix('-'))
        .unwrap_or(mantissa);
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    (!whole.is_empty() || !fraction.is_empty())
        && whole
            .bytes()
            .chain(fraction.bytes())
            .all(|byte| byte == b'0')
}

fn unsigned_encoding(graph: &Graph, implementation: usize) -> Result<(), Vec<PlanDiagnostic>> {
    let base = reference_descendant(graph, implementation, "BASE-TYPE-REF")?;
    if graph.text(base, "BASE-TYPE-ENCODING") != Some("NONE")
        || graph.text(base, "CATEGORY") != Some("FIXED_LENGTH")
    {
        return Err(fail(graph, base, "TYPE_CONFLICT"));
    }
    Ok(())
}

fn native_type(
    graph: &Graph,
    argument: usize,
    behavior: usize,
    arrays: &mut BTreeMap<String, String>,
) -> Result<(String, String), Vec<PlanDiagnostic>> {
    let datatype = referenced(graph, argument, "TYPE-TREF", "TYPE_MAP_MISSING")?;
    if graph.elements[datatype].tag == "APPLICATION-PRIMITIVE-DATA-TYPE" {
        if graph.text(datatype, "CATEGORY") != Some("VALUE") {
            return Err(fail(graph, datatype, "TYPE_CONFLICT"));
        }
        let mapped = implementation(graph, argument, behavior, datatype)?;
        unsigned_encoding(graph, mapped)?;
        return Ok((
            "uint32".into(),
            format!("{}:{}", path(graph, datatype), path(graph, mapped)),
        ));
    }
    if graph.elements[datatype].tag == "IMPLEMENTATION-DATA-TYPE"
        && graph.text(datatype, "CATEGORY") == Some("VALUE")
    {
        let base = reference_descendant(graph, datatype, "BASE-TYPE-REF")?;
        unsigned_encoding(graph, datatype)?;
        if graph.text(base, "NATIVE-DECLARATION") == Some("uint32")
            && graph.text(base, "BASE-TYPE-SIZE") == Some("32")
        {
            return Ok(("uint32".into(), path(graph, datatype)));
        }
    }
    if graph.elements[datatype].tag != "IMPLEMENTATION-DATA-TYPE"
        || graph.text(datatype, "CATEGORY") != Some("ARRAY")
    {
        return Err(fail(graph, argument, "SERVICE_TYPE_CONFLICT"));
    }
    let element = one(
        graph,
        datatype,
        graph.descendants(datatype, "IMPLEMENTATION-DATA-TYPE-ELEMENT"),
        "SERVICE_TYPE_CONFLICT",
    )?;
    let element_type = reference_descendant(graph, element, "IMPLEMENTATION-DATA-TYPE-REF")?;
    let base = reference_descendant(graph, element_type, "BASE-TYPE-REF")?;
    unsigned_encoding(graph, element_type)?;
    if graph.text(element, "CATEGORY") != Some("TYPE_REFERENCE")
        || graph.text(element, "ARRAY-SIZE") != Some("4")
        || graph.text(element, "ARRAY-SIZE-SEMANTICS") != Some("FIXED-SIZE")
        || graph.text(element_type, "CATEGORY") != Some("VALUE")
        || graph.text(base, "BASE-TYPE-SIZE") != Some("8")
        || graph.text(base, "NATIVE-DECLARATION") != Some("uint8")
        || graph.text(argument, "DIRECTION") != Some("OUT")
    {
        return Err(fail(graph, argument, "SERVICE_TYPE_CONFLICT"));
    }
    let native = c_name(&name(graph, datatype));
    arrays.insert(path(graph, datatype), native.clone());
    Ok((native, path(graph, datatype)))
}

pub(super) fn inspect(graph: &Graph) -> Result<MultiComponentContract, Vec<PlanDiagnostic>> {
    let context = *graph
        .objects
        .values()
        .next()
        .ok_or_else(|| Vec::<PlanDiagnostic>::new())?;
    let system = one(graph, context, graph.of_kind("SYSTEM"), "TARGET_NOT_UNIQUE")?;
    if graph.text(system, "CATEGORY") != Some("ECU_EXTRACT") {
        return Err(fail(graph, system, "TARGET_CATEGORY"));
    }
    let ecu = one(
        graph,
        system,
        graph.of_kind("ECU-INSTANCE"),
        "TARGET_NOT_UNIQUE",
    )?;
    let root = one(
        graph,
        system,
        graph.descendants(system, "ROOT-SW-COMPOSITION-PROTOTYPE"),
        "MULTIPLE_INSTANCES",
    )?;
    let composition = referenced(
        graph,
        root,
        "SOFTWARE-COMPOSITION-TREF",
        "MULTIPLE_INSTANCES",
    )?;
    if graph.elements[composition].tag != "COMPOSITION-SW-COMPONENT-TYPE"
        || !graph
            .descendants(composition, "DELEGATION-SW-CONNECTOR")
            .is_empty()
    {
        return Err(fail(graph, composition, "NESTED_COMPOSITION_UNSUPPORTED"));
    }
    let mut components = Vec::new();
    let mut types = BTreeSet::new();
    let mut arrays = BTreeMap::new();
    let mut common_period = None;
    for instance in graph.descendants(composition, "SW-COMPONENT-PROTOTYPE") {
        let component = referenced(graph, instance, "TYPE-TREF", "MULTIPLE_INSTANCES")?;
        if !matches!(
            graph.elements[component].tag.as_str(),
            "APPLICATION-SW-COMPONENT-TYPE" | "SERVICE-SW-COMPONENT-TYPE"
        ) || !types.insert(component)
        {
            return Err(fail(graph, instance, "MULTIPLE_INSTANCES"));
        }
        let behavior = one(
            graph,
            component,
            graph.descendants(component, "SWC-INTERNAL-BEHAVIOR"),
            "TYPE_CONFLICT",
        )?;
        if !matches!(
            graph.text(behavior, "SUPPORTS-MULTIPLE-INSTANTIATION"),
            Some("false" | "0")
        ) {
            return Err(fail(graph, behavior, "MULTIPLE_INSTANCES"));
        }
        for tag in [
            "QUEUED-RECEIVER-COM-SPEC",
            "QUEUED-SENDER-COM-SPEC",
            "ASYNCHRONOUS-SERVER-CALL-POINT",
            "MODE-ACCESS-POINTS",
            "DATA-READ-ACCESSS",
            "DATA-WRITE-ACCESSS",
            "VARIATION-POINT",
            "EXCLUSIVE-AREA",
            "EXCLUSIVE-AREA-REF",
            "CAN-ENTER-EXCLUSIVE-AREA-REF",
            "RUNS-INSIDE-EXCLUSIVE-AREA-REF",
            "EXCLUSIVE-AREA-NESTING-ORDER-REF",
        ] {
            if let Some(index) = graph.descendants(component, tag).first() {
                return Err(fail(graph, *index, "UNSUPPORTED_COMPONENT_FEATURE"));
            }
        }
        let mapping = one(
            graph,
            instance,
            graph
                .descendants(system, "SWC-TO-ECU-MAPPING")
                .into_iter()
                .filter(|mapping| {
                    graph
                        .descendants(*mapping, "TARGET-COMPONENT-REF")
                        .iter()
                        .any(|reference| graph.elements[*reference].text == path(graph, instance))
                })
                .collect(),
            "ECU_MAPPING",
        )?;
        if graph.target(mapping, "ECU-INSTANCE-REF") != Some(ecu) {
            return Err(fail(graph, mapping, "ECU_MAPPING"));
        }
        let rte_instance = one(
            graph,
            instance,
            graph
                .of_kind("ECUC-CONTAINER-VALUE")
                .into_iter()
                .filter(|index| {
                    super::schedule::definition_is(graph, *index, "RteSwComponentInstance")
                        && super::schedule::value(
                            graph,
                            *index,
                            "RteSoftwareComponentInstanceRef",
                            true,
                        ) == Some(graph.elements[instance].object.as_str())
                })
                .collect(),
            "INSTANCE_MAPPING",
        )?;
        for events in graph.children(behavior, "EVENTS") {
            for event in &graph.elements[events].children {
                if !matches!(
                    graph.elements[*event].tag.as_str(),
                    "TIMING-EVENT" | "OPERATION-INVOKED-EVENT"
                ) {
                    return Err(fail(graph, *event, "EVENT_UNSUPPORTED"));
                }
            }
        }
        let mut runnables = Vec::new();
        for runnable in graph.descendants(behavior, "RUNNABLE-ENTITY") {
            if graph
                .text(runnable, "MINIMUM-START-INTERVAL")
                .is_some_and(|value| !zero_seconds(value))
            {
                return Err(fail(graph, runnable, "MINIMUM_START_INTERVAL_UNSUPPORTED"));
            }
            if !matches!(
                graph.text(runnable, "CAN-BE-INVOKED-CONCURRENTLY"),
                Some("false" | "0")
            ) {
                return Err(fail(graph, runnable, "REENTRANCY_UNSUPPORTED"));
            }
            let events = graph
                .descendants(behavior, "TIMING-EVENT")
                .into_iter()
                .chain(graph.descendants(behavior, "OPERATION-INVOKED-EVENT"))
                .filter(|event| graph.target(*event, "START-ON-EVENT-REF") == Some(runnable))
                .collect::<Vec<_>>();
            // The existing Dcm client is invoked by BSW rather than an application event.
            if events.is_empty() && graph.elements[component].tag == "SERVICE-SW-COMPONENT-TYPE" {
                let symbol = graph.text(runnable, "SYMBOL").unwrap_or("").to_owned();
                if !super::catalog::c_identifier(&symbol) {
                    return Err(fail(graph, runnable, "SYMBOL_PRODUCER_DUPLICATE"));
                }
                runnables.push(Runnable {
                    path: path(graph, runnable),
                    symbol,
                    event: None,
                    period_ms: None,
                });
                continue;
            }
            let event = one(graph, runnable, events, "SCHEDULE_NOT_UNIQUE")?;
            let period = if graph.elements[event].tag == "TIMING-EVENT" {
                let period = graph
                    .text(event, "PERIOD")
                    .and_then(milliseconds)
                    .ok_or_else(|| fail(graph, event, "PERIOD_UNSUPPORTED"))?;
                if graph
                    .text(event, "OFFSET")
                    .is_some_and(|value| !zero_seconds(value))
                    || common_period.is_some_and(|value| value != period)
                {
                    return Err(fail(graph, event, "PERIOD_UNSUPPORTED"));
                }
                common_period = Some(period);
                Some(period)
            } else {
                None
            };
            let symbol = graph.text(runnable, "SYMBOL").unwrap_or("").to_owned();
            if !super::catalog::c_identifier(&symbol) {
                return Err(fail(graph, runnable, "SYMBOL_PRODUCER_DUPLICATE"));
            }
            runnables.push(Runnable {
                path: path(graph, runnable),
                symbol,
                event: Some(path(graph, event)),
                period_ms: period,
            });
        }
        // Every event must target a runnable owned by this behavior.
        for event in graph
            .descendants(behavior, "TIMING-EVENT")
            .into_iter()
            .chain(graph.descendants(behavior, "OPERATION-INVOKED-EVENT"))
        {
            if !runnables.iter().any(|runnable| {
                runnable.event.as_deref() == Some(graph.elements[event].object.as_str())
            }) {
                return Err(fail(graph, event, "EVENT_OWNERSHIP"));
            }
        }
        let mut data_ports = Vec::new();
        let mut interfaces = BTreeMap::new();
        let mut data_accesses = Vec::new();
        let mut operations = Vec::new();
        for port in graph
            .descendants(component, "P-PORT-PROTOTYPE")
            .into_iter()
            .chain(graph.descendants(component, "R-PORT-PROTOTYPE"))
        {
            let read = graph.elements[port].tag == "R-PORT-PROTOTYPE";
            let interface = referenced(
                graph,
                port,
                if read {
                    "REQUIRED-INTERFACE-TREF"
                } else {
                    "PROVIDED-INTERFACE-TREF"
                },
                "TYPE_CONFLICT",
            )?;
            interfaces.insert(path(graph, port), path(graph, interface));
            if graph.elements[interface].tag == "SENDER-RECEIVER-INTERFACE" {
                let element = one(
                    graph,
                    port,
                    graph.descendants(interface, "VARIABLE-DATA-PROTOTYPE"),
                    "TYPE_CONFLICT",
                )?;
                let application = referenced(graph, element, "TYPE-TREF", "TYPE_MAP_MISSING")?;
                if graph.elements[application].tag != "APPLICATION-PRIMITIVE-DATA-TYPE"
                    || graph.text(application, "CATEGORY") != Some("VALUE")
                {
                    return Err(fail(graph, element, "TYPE_CONFLICT"));
                }
                let implementation = implementation(graph, port, behavior, application)?;
                unsigned_encoding(graph, implementation)?;
                let specification = one(
                    graph,
                    port,
                    graph.descendants(
                        port,
                        if read {
                            "NONQUEUED-RECEIVER-COM-SPEC"
                        } else {
                            "NONQUEUED-SENDER-COM-SPEC"
                        },
                    ),
                    "TYPE_CONFLICT",
                )?;
                let initial = one(
                    graph,
                    specification,
                    graph.descendants(specification, "VALUE"),
                    "INIT_VALUE_REQUIRED",
                )?;
                let initial_value = graph.elements[initial]
                    .text
                    .parse::<u32>()
                    .map_err(|_| fail(graph, initial, "INIT_VALUE_REQUIRED"))?;
                if graph.target(specification, "DATA-ELEMENT-REF") != Some(element)
                    || !graph.descendants(interface, "INVALID-VALUE").is_empty()
                    || !graph.descendants(application, "INVALID-VALUE").is_empty()
                    || !graph
                        .descendants(implementation, "INVALID-VALUE")
                        .is_empty()
                {
                    return Err(fail(graph, specification, "TYPE_CONFLICT"));
                }
                let accesses = graph
                    .descendants(behavior, "PORT-PROTOTYPE-REF")
                    .into_iter()
                    .filter(|reference| graph.elements[*reference].text == path(graph, port))
                    .collect::<Vec<_>>();
                if accesses.is_empty() {
                    return Err(fail(graph, port, "PORT_ACCESS_MISSING"));
                }
                let mut owners = BTreeSet::new();
                for access in accesses {
                    let owner = graph
                        .ancestor(access, "RUNNABLE-ENTITY")
                        .ok_or_else(|| fail(graph, access, "PORT_ACCESS_OWNERSHIP"))?;
                    let variable = graph
                        .ancestor(access, "VARIABLE-ACCESS")
                        .ok_or_else(|| fail(graph, access, "PORT_ACCESS_OWNERSHIP"))?;
                    if reference_descendant(graph, variable, "TARGET-DATA-PROTOTYPE-REF")?
                        != element
                        || !runnables
                            .iter()
                            .any(|runnable| runnable.path == path(graph, owner))
                        || graph
                            .ancestor(
                                access,
                                if read {
                                    "DATA-RECEIVE-POINT-BY-ARGUMENTS"
                                } else {
                                    "DATA-SEND-POINTS"
                                },
                            )
                            .is_none()
                    {
                        return Err(fail(graph, access, "PORT_ACCESS_OWNERSHIP"));
                    }
                    data_accesses.push(DataAccess {
                        path: path(graph, variable),
                        endpoint: Endpoint {
                            instance: path(graph, instance),
                            port: path(graph, port),
                            member: path(graph, element),
                        },
                        runnable: path(graph, owner),
                        read,
                    });
                    owners.insert(path(graph, owner));
                }
                data_ports.push(DataPort {
                    path: path(graph, port),
                    name: name(graph, port),
                    element: path(graph, element),
                    element_name: name(graph, element),
                    application_type: path(graph, application),
                    implementation_type: path(graph, implementation),
                    c_type: "uint32".into(),
                    read,
                    api_symbol: format!(
                        "Rte_{}_{}_{}",
                        if read { "Read" } else { "Write" },
                        c_name(&name(graph, port)),
                        c_name(&name(graph, element))
                    ),
                    runnable: owners.into_iter().next().unwrap(),
                    initial_value,
                    alive_timeout_ms: if read {
                        let value = graph
                            .text(specification, "ALIVE-TIMEOUT")
                            .ok_or_else(|| fail(graph, specification, "FRESHNESS_REQUIRED"))?;
                        if zero_seconds(value) {
                            Some(0)
                        } else {
                            Some(
                                milliseconds(value).ok_or_else(|| {
                                    fail(graph, specification, "FRESHNESS_REQUIRED")
                                })?,
                            )
                        }
                    } else {
                        None
                    },
                });
            } else if graph.elements[interface].tag == "CLIENT-SERVER-INTERFACE" {
                let interface_operations = graph.descendants(interface, "CLIENT-SERVER-OPERATION");
                if interface_operations.is_empty() {
                    return Err(fail(graph, interface, "SERVICE_TYPE_CONFLICT"));
                }
                let specs = graph.descendants(
                    port,
                    if read {
                        "CLIENT-COM-SPEC"
                    } else {
                        "SERVER-COM-SPEC"
                    },
                );
                let mut specified = BTreeSet::new();
                for spec in &specs {
                    let operation = graph.target(*spec, "OPERATION-REF");
                    if !operation.is_some_and(|operation| {
                        interface_operations.contains(&operation) && specified.insert(operation)
                    }) {
                        return Err(fail(graph, *spec, "SERVICE_TYPE_CONFLICT"));
                    }
                }
                for operation in interface_operations {
                    if !graph
                        .descendants(operation, "POSSIBLE-ERROR-REF")
                        .is_empty()
                        || graph
                            .text(operation, "NO-RETURN-VALUE-PROVIDED")
                            .is_some_and(|value| !matches!(value, "false" | "0"))
                    {
                        return Err(fail(graph, operation, "APPLICATION_ERROR_UNSUPPORTED"));
                    }
                    let specs = graph
                        .descendants(
                            port,
                            if read {
                                "CLIENT-COM-SPEC"
                            } else {
                                "SERVER-COM-SPEC"
                            },
                        )
                        .into_iter()
                        .filter(|spec| graph.target(*spec, "OPERATION-REF") == Some(operation))
                        .collect();
                    let spec = one(graph, port, specs, "SERVICE_TYPE_CONFLICT")?;
                    if !read && graph.text(spec, "QUEUE-LENGTH") != Some("1") {
                        return Err(fail(graph, spec, "SERVICE_TYPE_CONFLICT"));
                    }
                    let mut arguments = Vec::new();
                    let mut argument_types = Vec::new();
                    for argument in graph.descendants(operation, "ARGUMENT-DATA-PROTOTYPE") {
                        if graph
                            .text(argument, "SERVER-ARGUMENT-IMPL-POLICY")
                            .is_some_and(|policy| policy != "USE-ARGUMENT-TYPE")
                        {
                            return Err(fail(graph, argument, "SERVICE_TYPE_CONFLICT"));
                        }
                        let direction = graph.text(argument, "DIRECTION").unwrap_or("");
                        if !matches!(direction, "IN" | "OUT" | "INOUT") {
                            return Err(fail(graph, argument, "ARGUMENT_DIRECTION"));
                        }
                        let (native, identity) =
                            native_type(graph, argument, behavior, &mut arrays)?;
                        arguments.push(ContractArgument {
                            name: c_name(&name(graph, argument)),
                            native_type: if native == "uint32" && direction != "IN" {
                                format!("{native} *")
                            } else {
                                native
                            },
                            direction: direction.into(),
                        });
                        argument_types.push(identity);
                    }
                    let mut callers = Vec::new();
                    let mut call_points = Vec::new();
                    let (runnable, event) = if read {
                        let calls = graph
                            .descendants(behavior, "SYNCHRONOUS-SERVER-CALL-POINT")
                            .into_iter()
                            .filter(|call| {
                                graph.descendants(*call, "CONTEXT-R-PORT-REF").iter().any(
                                    |reference| {
                                        graph.elements[*reference].text == path(graph, port)
                                    },
                                ) && graph
                                    .descendants(*call, "TARGET-REQUIRED-OPERATION-REF")
                                    .iter()
                                    .any(|reference| {
                                        graph.elements[*reference].text == path(graph, operation)
                                    })
                            })
                            .collect::<Vec<_>>();
                        if calls.is_empty() {
                            return Err(fail(graph, port, "SERVICE_CLIENT_MISSING"));
                        }
                        for call in &calls {
                            let owner = graph
                                .ancestor(*call, "RUNNABLE-ENTITY")
                                .ok_or_else(|| fail(graph, *call, "SERVICE_CLIENT_MISSING"))?;
                            if !graph.within(owner, behavior)
                                || (graph.elements[component].tag
                                    == "APPLICATION-SW-COMPONENT-TYPE"
                                    && !runnables
                                        .iter()
                                        .any(|runnable| runnable.path == path(graph, owner)))
                            {
                                return Err(fail(graph, *call, "CALL_POINT_OWNERSHIP"));
                            }
                            callers.push(path(graph, owner));
                            call_points.push(path(graph, *call));
                        }
                        (callers[0].clone(), None)
                    } else {
                        let events = graph
                            .descendants(behavior, "OPERATION-INVOKED-EVENT")
                            .into_iter()
                            .filter(|event| {
                                graph.descendants(*event, "CONTEXT-P-PORT-REF").iter().any(
                                    |reference| {
                                        graph.elements[*reference].text == path(graph, port)
                                    },
                                ) && graph
                                    .descendants(*event, "TARGET-PROVIDED-OPERATION-REF")
                                    .iter()
                                    .any(|reference| {
                                        graph.elements[*reference].text == path(graph, operation)
                                    })
                            })
                            .collect();
                        let event = one(graph, port, events, "SERVICE_CLIENT_MISSING")?;
                        let runnable =
                            referenced(graph, event, "START-ON-EVENT-REF", "EVENT_OWNERSHIP")?;
                        (path(graph, runnable), Some(path(graph, event)))
                    };
                    let api = format!(
                        "Rte_Call_{}_{}",
                        c_name(&name(graph, port)),
                        c_name(&name(graph, operation))
                    );
                    operations.push(Operation {
                        port: path(graph, port),
                        interface: path(graph, interface),
                        operation: path(graph, operation),
                        read,
                        api_symbol: api.clone(),
                        implementation_symbol: format!(
                            "Rte_{}_{}",
                            c_name(graph.elements[instance].object.trim_start_matches('/')),
                            api.trim_start_matches("Rte_")
                        ),
                        arguments,
                        argument_types,
                        runnable,
                        callers,
                        call_points,
                        event,
                    });
                }
            } else {
                return Err(fail(graph, port, "TYPE_CONFLICT"));
            }
        }
        for event in graph.descendants(behavior, "OPERATION-INVOKED-EVENT") {
            if operations
                .iter()
                .filter(|operation| {
                    !operation.read
                        && operation.event.as_deref() == Some(graph.elements[event].object.as_str())
                })
                .count()
                != 1
            {
                return Err(fail(graph, event, "EVENT_OWNERSHIP"));
            }
        }
        for access in graph.descendants(behavior, "VARIABLE-ACCESS") {
            if data_accesses
                .iter()
                .filter(|checked| checked.path == path(graph, access))
                .count()
                != 1
            {
                return Err(fail(graph, access, "PORT_ACCESS_OWNERSHIP"));
            }
        }
        for runnable in runnables.iter().filter(|runnable| runnable.event.is_none()) {
            if operations
                .iter()
                .filter(|operation| operation.read && operation.callers.contains(&runnable.path))
                .count()
                != 1
            {
                return Err(fail(
                    graph,
                    *graph.objects.get(&runnable.path).unwrap(),
                    "SERVICE_CLIENT_MISSING",
                ));
            }
        }
        for call in graph.descendants(behavior, "SYNCHRONOUS-SERVER-CALL-POINT") {
            if !operations
                .iter()
                .any(|operation| operation.call_points.contains(&path(graph, call)))
            {
                return Err(fail(graph, call, "CALL_POINT_OWNERSHIP"));
            }
        }
        components.push(Component {
            component: path(graph, component),
            instance: path(graph, instance),
            ecu: path(graph, ecu),
            ecu_mapping: path(graph, mapping),
            rte_instance: path(graph, rte_instance),
            behavior: path(graph, behavior),
            header: format!("include/Rte_{}.h", c_name(&name(graph, component))),
            interfaces,
            data_ports,
            data_accesses,
            operations,
            runnables,
        });
    }
    for rte_instance in graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| super::schedule::definition_is(graph, *index, "RteSwComponentInstance"))
    {
        if !components
            .iter()
            .any(|component| component.rte_instance == path(graph, rte_instance))
        {
            return Err(fail(graph, rte_instance, "INSTANCE_MAPPING"));
        }
    }
    for mapping in graph.descendants(system, "SWC-TO-ECU-MAPPING") {
        if graph.target(mapping, "ECU-INSTANCE-REF") != Some(ecu) {
            return Err(fail(graph, mapping, "ECU_MAPPING"));
        }
        let members = graph.descendants(mapping, "COMPONENT-IREF");
        if members.is_empty() {
            return Err(fail(graph, mapping, "ECU_MAPPING"));
        }
        let mut mapped_instances = BTreeSet::new();
        for iref in members {
            if graph.target(iref, "CONTEXT-COMPOSITION-REF") != Some(root)
                || !graph
                    .target(iref, "TARGET-COMPONENT-REF")
                    .is_some_and(|instance| {
                        mapped_instances.insert(instance)
                            && components
                                .iter()
                                .any(|component| component.instance == path(graph, instance))
                    })
            {
                return Err(fail(graph, iref, "ECU_MAPPING"));
            }
        }
    }
    let mut connections = Vec::new();
    let mut receivers = BTreeSet::new();
    for connector in graph.descendants(composition, "ASSEMBLY-SW-CONNECTOR") {
        let provider = one(
            graph,
            connector,
            graph.children(connector, "PROVIDER-IREF"),
            "CONNECTOR_OWNERSHIP",
        )?;
        let requester = one(
            graph,
            connector,
            graph.children(connector, "REQUESTER-IREF"),
            "CONNECTOR_OWNERSHIP",
        )?;
        let p_instance = referenced(
            graph,
            provider,
            "CONTEXT-COMPONENT-REF",
            "CONNECTOR_OWNERSHIP",
        )?;
        let r_instance = referenced(
            graph,
            requester,
            "CONTEXT-COMPONENT-REF",
            "CONNECTOR_OWNERSHIP",
        )?;
        let p_port = referenced(graph, provider, "TARGET-P-PORT-REF", "CONNECTOR_DIRECTION")?;
        let r_port = referenced(graph, requester, "TARGET-R-PORT-REF", "CONNECTOR_DIRECTION")?;
        let p_component = components
            .iter()
            .find(|component| component.instance == path(graph, p_instance))
            .ok_or_else(|| fail(graph, provider, "CONNECTOR_OWNERSHIP"))?;
        let r_component = components
            .iter()
            .find(|component| component.instance == path(graph, r_instance))
            .ok_or_else(|| fail(graph, requester, "CONNECTOR_OWNERSHIP"))?;
        if !graph.within(p_port, *graph.objects.get(&p_component.component).unwrap())
            || !graph.within(r_port, *graph.objects.get(&r_component.component).unwrap())
            || graph.elements[p_port].tag != "P-PORT-PROTOTYPE"
            || graph.elements[r_port].tag != "R-PORT-PROTOTYPE"
            || graph.target(p_port, "PROVIDED-INTERFACE-TREF")
                != graph.target(r_port, "REQUIRED-INTERFACE-TREF")
        {
            return Err(fail(graph, connector, "TYPE_CONFLICT"));
        }
        if let Some(p) = p_component
            .data_ports
            .iter()
            .find(|port| port.path == path(graph, p_port))
        {
            let r = r_component
                .data_ports
                .iter()
                .find(|port| port.path == path(graph, r_port))
                .ok_or_else(|| fail(graph, connector, "TYPE_CONFLICT"))?;
            if p.element != r.element
                || p.application_type != r.application_type
                || p.implementation_type != r.implementation_type
            {
                return Err(fail(graph, connector, "TYPE_CONFLICT"));
            }
            let endpoint = Endpoint {
                instance: r_component.instance.clone(),
                port: r.path.clone(),
                member: r.element.clone(),
            };
            if !receivers.insert(endpoint.clone()) {
                return Err(fail(graph, connector, "MULTIPLE_PRODUCERS"));
            }
            connections.push(Connection {
                path: path(graph, connector),
                provider: Endpoint {
                    instance: p_component.instance.clone(),
                    port: p.path.clone(),
                    member: p.element.clone(),
                },
                requester: endpoint,
                service: false,
            });
        } else {
            for r in r_component
                .operations
                .iter()
                .filter(|operation| operation.port == path(graph, r_port))
            {
                let p = p_component
                    .operations
                    .iter()
                    .find(|operation| {
                        operation.port == path(graph, p_port) && operation.operation == r.operation
                    })
                    .ok_or_else(|| fail(graph, connector, "SERVICE_TYPE_CONFLICT"))?;
                if p.argument_types != r.argument_types
                    || p.arguments
                        .iter()
                        .map(|argument| (&argument.native_type, &argument.direction))
                        .collect::<Vec<_>>()
                        != r.arguments
                            .iter()
                            .map(|argument| (&argument.native_type, &argument.direction))
                            .collect::<Vec<_>>()
                {
                    return Err(fail(graph, connector, "SERVICE_TYPE_CONFLICT"));
                }
                let endpoint = Endpoint {
                    instance: r_component.instance.clone(),
                    port: r.port.clone(),
                    member: r.operation.clone(),
                };
                if !receivers.insert(endpoint.clone()) {
                    return Err(fail(graph, connector, "MULTIPLE_PRODUCERS"));
                }
                connections.push(Connection {
                    path: path(graph, connector),
                    provider: Endpoint {
                        instance: p_component.instance.clone(),
                        port: p.port.clone(),
                        member: p.operation.clone(),
                    },
                    requester: endpoint,
                    service: true,
                });
            }
        }
    }
    let mut network_endpoints = Vec::new();
    for component in &components {
        let mut network_ports = Vec::new();
        for port in &component.data_ports {
            let endpoint = Endpoint {
                instance: component.instance.clone(),
                port: port.path.clone(),
                member: port.element.clone(),
            };
            let mappings = graph
                .descendants(system, "SENDER-RECEIVER-TO-SIGNAL-MAPPING")
                .into_iter()
                .filter(|mapping| {
                    graph
                        .descendants(*mapping, "CONTEXT-COMPONENT-REF")
                        .iter()
                        .any(|reference| graph.elements[*reference].text == component.instance)
                        && graph
                            .descendants(*mapping, "CONTEXT-PORT-REF")
                            .iter()
                            .any(|reference| graph.elements[*reference].text == port.path)
                })
                .collect::<Vec<_>>();
            let local = connections.iter().any(|connection| {
                connection.provider == endpoint || connection.requester == endpoint
            });
            if mappings.len() > 1
                || (!mappings.is_empty() && local)
                || (mappings.is_empty() && !local)
            {
                return Err(fail(
                    graph,
                    *graph.objects.get(&port.path).unwrap(),
                    "ENDPOINT_BINDING",
                ));
            }
            if mappings.is_empty() {
                let context = *graph.objects.get(&port.path).unwrap();
                if port.read {
                    let spec = one(
                        graph,
                        context,
                        graph.descendants(context, "NONQUEUED-RECEIVER-COM-SPEC"),
                        "TYPE_CONFLICT",
                    )?;
                    if !matches!(
                        graph.text(spec, "HANDLE-NEVER-RECEIVED"),
                        Some("false" | "0")
                    ) || !graph.text(spec, "ALIVE-TIMEOUT").is_some_and(zero_seconds)
                        || graph.text(spec, "HANDLE-TIMEOUT-TYPE") != Some("NONE")
                    {
                        return Err(fail(graph, spec, "LOCAL_FRESHNESS_UNSUPPORTED"));
                    }
                }
            } else {
                if port.read {
                    let context = *graph.objects.get(&port.path).unwrap();
                    let spec = one(
                        graph,
                        context,
                        graph.descendants(context, "NONQUEUED-RECEIVER-COM-SPEC"),
                        "TYPE_CONFLICT",
                    )?;
                    if !matches!(
                        graph.text(spec, "HANDLE-NEVER-RECEIVED"),
                        Some("true" | "1")
                    ) || graph.text(spec, "HANDLE-TIMEOUT-TYPE") != Some("NONE")
                    {
                        return Err(fail(graph, spec, "NETWORK_FRESHNESS_UNSUPPORTED"));
                    }
                }
                network_ports.push(port.clone());
            }
        }
        let signals = super::communication::inspect_ports(
            graph,
            &network_ports,
            &component.instance,
            common_period.ok_or_else(|| fail(graph, composition, "SCHEDULE_NOT_UNIQUE"))?,
        )?;
        for transport in signals {
            let channel = graph
                .ancestor(
                    *graph.objects.get(&transport.trigger).unwrap(),
                    "CAN-PHYSICAL-CHANNEL",
                )
                .ok_or_else(|| fail(graph, composition, "SIGNAL_MAPPING"))?;
            let network = graph
                .ancestor(channel, "CAN-CLUSTER")
                .ok_or_else(|| fail(graph, channel, "SIGNAL_MAPPING"))?;
            let port = component
                .data_ports
                .iter()
                .find(|port| port.path == transport.port)
                .unwrap();
            network_endpoints.push(NetworkEndpoint {
                endpoint: Endpoint {
                    instance: component.instance.clone(),
                    port: port.path.clone(),
                    member: port.element.clone(),
                },
                network: path(graph, network),
                channel: path(graph, channel),
                transport,
            });
        }
        for operation in component
            .operations
            .iter()
            .filter(|operation| operation.read)
        {
            if !connections.iter().any(|connection| {
                connection.requester
                    == (Endpoint {
                        instance: component.instance.clone(),
                        port: operation.port.clone(),
                        member: operation.operation.clone(),
                    })
            }) {
                return Err(fail(
                    graph,
                    *graph.objects.get(&operation.port).unwrap(),
                    "SERVICE_CLIENT_MISSING",
                ));
            }
        }
    }
    for mapping in graph.descendants(system, "SENDER-RECEIVER-TO-SIGNAL-MAPPING") {
        let iref = one(
            graph,
            mapping,
            graph.children(mapping, "DATA-ELEMENT-IREF"),
            "SIGNAL_MAPPING",
        )?;
        let instance = reference_descendant(graph, iref, "CONTEXT-COMPONENT-REF")?;
        let port = referenced(graph, iref, "CONTEXT-PORT-REF", "SIGNAL_MAPPING")?;
        let member = referenced(graph, iref, "TARGET-DATA-PROTOTYPE-REF", "SIGNAL_MAPPING")?;
        if graph.target(iref, "CONTEXT-COMPOSITION-REF") != Some(root)
            || !network_endpoints.iter().any(|checked| {
                checked.endpoint
                    == (Endpoint {
                        instance: path(graph, instance),
                        port: path(graph, port),
                        member: path(graph, member),
                    })
            })
        {
            return Err(fail(graph, mapping, "SIGNAL_MAPPING"));
        }
    }
    if !network_endpoints.is_empty() {
        let channel = super::component::selected_channel(graph, ecu)?;
        if network_endpoints
            .iter()
            .any(|endpoint| endpoint.channel != path(graph, channel))
        {
            return Err(fail(graph, channel, "SIGNAL_MAPPING"));
        }
    }
    // Traverse actual runnable call edges, including every synchronous call point.
    let mut calls: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for connection in connections.iter().filter(|connection| connection.service) {
        let provider = components
            .iter()
            .find(|component| component.instance == connection.provider.instance)
            .unwrap();
        let requester = components
            .iter()
            .find(|component| component.instance == connection.requester.instance)
            .unwrap();
        let server = provider
            .operations
            .iter()
            .find(|operation| {
                operation.port == connection.provider.port
                    && operation.operation == connection.provider.member
            })
            .unwrap();
        let client = requester
            .operations
            .iter()
            .find(|operation| {
                operation.port == connection.requester.port
                    && operation.operation == connection.requester.member
            })
            .unwrap();
        for caller in &client.callers {
            calls
                .entry(caller.clone())
                .or_default()
                .push(server.runnable.clone());
        }
    }
    for origin in calls.keys() {
        let mut pending = vec![origin.as_str()];
        let mut visited = BTreeSet::new();
        while let Some(runnable) = pending.pop() {
            if !visited.insert(runnable) {
                continue;
            }
            for target in calls.get(runnable).into_iter().flatten() {
                if target == origin {
                    return Err(fail(
                        graph,
                        *graph.objects.get(origin).unwrap(),
                        "SERVICE_RECURSION",
                    ));
                }
                pending.push(target);
            }
        }
    }
    let plan = MultiComponentContract {
        system: path(graph, system),
        composition: path(graph, composition),
        ecu: path(graph, ecu),
        components,
        connections,
        network_endpoints,
        array_types: arrays,
    };
    check_names(graph, &plan)?;
    Ok(plan)
}

pub(super) fn data_symbol(component: &Component, port: &DataPort) -> String {
    format!(
        "Rte_{}_{}",
        c_name(component.instance.trim_start_matches('/')),
        port.api_symbol.trim_start_matches("Rte_")
    )
}

fn check_names(graph: &Graph, plan: &MultiComponentContract) -> Result<(), Vec<PlanDiagnostic>> {
    let mut filenames = BTreeSet::from([
        "INCLUDE/RTE.H".to_owned(),
        "INCLUDE/RTE_TYPE.H".to_owned(),
        "INCLUDE/RTE_COM.H".to_owned(),
        "INCLUDE/STD_TYPES.H".to_owned(),
    ]);
    let mut names: BTreeSet<String> = "RTE_H RTE_TYPE_H RTE_COM_H Rte_COMCbk Rte_COMCbkRxTOut CbkHandleIdType STD_TYPES_H uint8 uint16 uint32 uint64 sint8 sint16 sint32 sint64 EcuStatus boolean Std_ReturnType Std_VersionInfoType TRUE FALSE E_OK E_NOT_OK RTE_E_COM_STOPPED RTE_E_NEVER_RECEIVED RTE_E_MAX_AGE_EXCEEDED intptr_t uintptr_t intmax_t uintmax_t INTMAX_MIN INTMAX_MAX UINTMAX_MAX INTPTR_MIN INTPTR_MAX UINTPTR_MAX PTRDIFF_MIN PTRDIFF_MAX SIZE_MAX SIG_ATOMIC_MIN SIG_ATOMIC_MAX WCHAR_MIN WCHAR_MAX WINT_MIN WINT_MAX INTMAX_C UINTMAX_C".split_whitespace().map(str::to_owned).collect();
    for width in [8, 16, 32, 64] {
        for signedness in ["int", "uint"] {
            for size in ["", "_least", "_fast"] {
                names.insert(format!("{signedness}{size}{width}_t"));
            }
        }
        for prefix in [
            "INT",
            "UINT",
            "INT_LEAST",
            "UINT_LEAST",
            "INT_FAST",
            "UINT_FAST",
        ] {
            names.insert(format!("{prefix}{width}_MAX"));
            names.insert(format!("{prefix}{width}_MIN"));
        }
        names.insert(format!("INT{width}_C"));
        names.insert(format!("UINT{width}_C"));
    }
    for (path, native) in &plan.array_types {
        if super::contracts::reserved_identifier(native) || !names.insert(native.clone()) {
            return Err(fail(
                graph,
                *graph.objects.get(path).unwrap(),
                "CONTRACT_NAME_COLLISION",
            ));
        }
    }
    let shared_names = names.clone();
    for component in &plan.components {
        let mut local_names = shared_names.clone();
        let index = *graph.objects.get(&component.component).unwrap();
        let type_name = component
            .header
            .trim_start_matches("include/Rte_")
            .trim_end_matches(".h");
        let guard = format!("RTE_{}_H", type_name.to_uppercase());
        if super::contracts::reserved_identifier(type_name)
            || !filenames.insert(component.header.to_uppercase())
            || !names.insert(guard.clone())
        {
            return Err(fail(graph, index, "CONTRACT_NAME_COLLISION"));
        }
        local_names.insert(guard);
        let mut aliases = BTreeSet::new();
        for port in &component.data_ports {
            if !aliases.insert(port.api_symbol.clone())
                || !names.insert(data_symbol(component, port))
            {
                return Err(fail(graph, index, "SYMBOL_NORMALIZATION_COLLISION"));
            }
        }
        for operation in component
            .operations
            .iter()
            .filter(|operation| operation.read)
        {
            if !aliases.insert(operation.api_symbol.clone())
                || !names.insert(operation.implementation_symbol.clone())
            {
                return Err(fail(graph, index, "SYMBOL_NORMALIZATION_COLLISION"));
            }
        }
        for runnable in &component.runnables {
            local_names.insert(runnable.symbol.clone());
            if super::contracts::reserved_identifier(&runnable.symbol)
                || !names.insert(runnable.symbol.clone())
            {
                return Err(fail(
                    graph,
                    *graph.objects.get(&runnable.path).unwrap(),
                    "SYMBOL_PRODUCER_DUPLICATE",
                ));
            }
        }
        for port in &component.data_ports {
            local_names.insert(data_symbol(component, port));
        }
        for operation in component
            .operations
            .iter()
            .filter(|operation| operation.read)
        {
            local_names.insert(operation.implementation_symbol.clone());
        }
        if aliases.iter().any(|alias| local_names.contains(alias)) {
            return Err(fail(graph, index, "CONTRACT_NAME_COLLISION"));
        }
        local_names.extend(aliases);
        for operation in &component.operations {
            let mut arguments = BTreeSet::new();
            if operation.arguments.iter().any(|argument| {
                super::contracts::reserved_identifier(&argument.name)
                    || local_names.contains(&argument.name)
                    || !arguments.insert(argument.name.clone())
            }) {
                return Err(fail(
                    graph,
                    *graph.objects.get(&operation.operation).unwrap(),
                    "CONTRACT_NAME_COLLISION",
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn symbols(plan: &MultiComponentContract) -> Vec<SymbolContract> {
    let mut symbols = Vec::new();
    for component in &plan.components {
        for port in &component.data_ports {
            symbols.push(SymbolContract {
                symbol: data_symbol(component, port),
                return_type: "Std_ReturnType".into(),
                arguments: vec![ContractArgument {
                    name: "data".into(),
                    native_type: if port.read { "uint32 *" } else { "uint32" }.into(),
                    direction: if port.read { "OUT" } else { "IN" }.into(),
                }],
                declaration_owner: component.header.clone(),
                definition_owner: "src/Rte.c".into(),
                consumers: component
                    .data_accesses
                    .iter()
                    .filter(|access| access.endpoint.port == port.path)
                    .map(|access| access.runnable.clone())
                    .collect(),
            });
        }
        for operation in component
            .operations
            .iter()
            .filter(|operation| operation.read)
        {
            symbols.push(SymbolContract {
                symbol: operation.implementation_symbol.clone(),
                return_type: "Std_ReturnType".into(),
                arguments: operation.arguments.clone(),
                declaration_owner: component.header.clone(),
                definition_owner: "src/Rte.c".into(),
                consumers: operation.callers.clone(),
            });
        }
        for runnable in &component.runnables {
            let operation = component.operations.iter().find(|operation| {
                operation.runnable == runnable.path && (!operation.read || runnable.event.is_none())
            });
            symbols.push(SymbolContract {
                symbol: runnable.symbol.clone(),
                return_type: if runnable.event.is_none() {
                    "Std_ReturnType"
                } else {
                    "void"
                }
                .into(),
                arguments: operation
                    .map(|operation| operation.arguments.clone())
                    .unwrap_or_default(),
                declaration_owner: component.header.clone(),
                definition_owner: if runnable.event.is_none() {
                    "src/Rte.c".into()
                } else {
                    format!(
                        "application/{}.c",
                        c_name(component.component.rsplit('/').next().unwrap())
                    )
                },
                consumers: if runnable.event.is_none() {
                    vec!["runtime/src/Dcm.c".into()]
                } else if let Some(operation) = operation {
                    plan.connections
                        .iter()
                        .filter(|connection| {
                            connection.provider.instance == component.instance
                                && connection.provider.port == operation.port
                                && connection.provider.member == operation.operation
                        })
                        .map(|connection| connection.requester.port.clone())
                        .collect()
                } else {
                    runnable.event.iter().cloned().collect()
                },
            });
        }
    }
    symbols
}
