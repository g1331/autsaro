use super::graph::Graph;
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataPort {
    pub path: String,
    pub name: String,
    pub element: String,
    pub element_name: String,
    pub application_type: String,
    pub implementation_type: String,
    pub c_type: String,
    pub read: bool,
    pub api_symbol: String,
    pub runnable: String,
    pub initial_value: u32,
    pub alive_timeout_ms: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServicePort {
    pub path: String,
    pub name: String,
    pub operation: String,
    pub operation_name: String,
    pub array_type: String,
    pub array_length: u32,
    pub runnable: String,
    pub runnable_symbol: String,
    pub client_instance: String,
    pub client_port: String,
    pub client_runnable: String,
    pub client_symbol: String,
}

/// Checked component contract, without claiming that the ECU integration
/// graph or runtime link has already passed.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentContract {
    pub component: String,
    pub instance: String,
    pub ecu: String,
    pub behavior: String,
    pub data_ports: Vec<DataPort>,
    pub service: ServicePort,
    pub timing_event: String,
    pub periodic_runnable: String,
    pub periodic_symbol: String,
    pub period_ms: u32,
}

fn reject(graph: &Graph, index: usize, code: &str, message: &str) -> Vec<PlanDiagnostic> {
    vec![graph.diagnostic(index, DiagnosticCategory::Input, code, message,
        "Restore a unique, explicit relationship within the selected single-instance uint32 S/R and synchronous four-byte C/S profile.")]
}

fn one(
    graph: &Graph,
    context: usize,
    values: Vec<usize>,
    code: &str,
) -> Result<usize, Vec<PlanDiagnostic>> {
    if values.len() == 1 {
        Ok(values[0])
    } else {
        Err(reject(
            graph,
            context,
            code,
            "The selected relationship must identify exactly one object.",
        ))
    }
}

fn referenced(
    graph: &Graph,
    context: usize,
    tag: &str,
    code: &str,
) -> Result<usize, Vec<PlanDiagnostic>> {
    graph.target(context, tag).ok_or_else(|| {
        reject(
            graph,
            context,
            code,
            "A required typed reference is absent.",
        )
    })
}

fn name(graph: &Graph, index: usize) -> String {
    graph.text(index, "SHORT-NAME").unwrap_or("").into()
}

pub(super) fn c_name(value: &str) -> String {
    let mut result: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect();
    if result.starts_with(|character: char| character.is_ascii_digit()) {
        result.insert(0, '_');
    }
    result
}

fn false_value(value: Option<&str>) -> bool {
    matches!(value, Some("false" | "0"))
}

pub(super) fn milliseconds(value: &str) -> Option<u32> {
    let (mantissa, exponent) = match value.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i32>().ok()?),
        None => (value, 0),
    };
    let mantissa = mantissa.strip_prefix('+').unwrap_or(mantissa);
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    let digits = format!("{whole}{fraction}");
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let mut amount = digits.parse::<u128>().ok()?;
    let scale = exponent
        .checked_add(3)?
        .checked_sub(i32::try_from(fraction.len()).ok()?)?;
    if scale >= 0 {
        amount = amount.checked_mul(10u128.checked_pow(scale as u32)?)?;
    } else {
        let divisor = 10u128.checked_pow(scale.checked_neg()? as u32)?;
        if amount % divisor != 0 {
            return None;
        }
        amount /= divisor;
    }
    u32::try_from(amount).ok().filter(|amount| *amount != 0)
}

fn implementation(
    graph: &Graph,
    context: usize,
    behavior: usize,
    application: usize,
) -> Result<usize, Vec<PlanDiagnostic>> {
    let sets: Vec<_> = graph
        .descendants(behavior, "DATA-TYPE-MAPPING-REF")
        .iter()
        .filter_map(|index| graph.objects.get(&graph.elements[*index].text).copied())
        .collect();
    let mapping_context = sets.first().copied().unwrap_or(context);
    let maps: Vec<_> = sets
        .into_iter()
        .flat_map(|set| graph.descendants(set, "DATA-TYPE-MAP"))
        .filter(|map| graph.target(*map, "APPLICATION-DATA-TYPE-REF") == Some(application))
        .collect();
    let mapping = one(graph, mapping_context, maps, "TYPE_MAP_MISSING")?;
    let implementation = referenced(
        graph,
        mapping,
        "IMPLEMENTATION-DATA-TYPE-REF",
        "TYPE_MAP_MISSING",
    )?;
    let base = one(
        graph,
        implementation,
        graph.descendants(implementation, "BASE-TYPE-REF"),
        "TYPE_CONFLICT",
    )?;
    let base = *graph.objects.get(&graph.elements[base].text).unwrap();
    if graph.text(base, "BASE-TYPE-SIZE") != Some("32")
        || graph.text(base, "NATIVE-DECLARATION") != Some("uint32")
        || graph.text(implementation, "CATEGORY") != Some("VALUE")
    {
        return Err(reject(
            graph,
            context,
            "TYPE_CONFLICT",
            "S/R requires an explicitly mapped uint32 implementation type.",
        ));
    }
    Ok(implementation)
}

pub(super) fn selected_channel(graph: &Graph, ecu: usize) -> Result<usize, Vec<PlanDiagnostic>> {
    let channels = graph
        .of_kind("CAN-PHYSICAL-CHANNEL")
        .into_iter()
        .filter(|channel| {
            graph
                .descendants(*channel, "COMMUNICATION-CONNECTOR-REF")
                .into_iter()
                .any(|reference| {
                    graph
                        .objects
                        .get(&graph.elements[reference].text)
                        .is_some_and(|connector| graph.within(*connector, ecu))
                })
        })
        .collect();
    one(graph, ecu, channels, "TARGET_NOT_UNIQUE")
}

pub(super) fn inspect(graph: &Graph) -> Result<ComponentContract, Vec<PlanDiagnostic>> {
    let roots = graph
        .of_kind("SYSTEM")
        .into_iter()
        .filter(|index| graph.text(*index, "CATEGORY") == Some("ECU_EXTRACT"))
        .collect();
    let context = *graph.objects.values().next().ok_or_else(|| {
        vec![PlanDiagnostic::dependency(
            "INPUT_MISSING",
            "No AUTOSAR objects were supplied.",
            "Supply the selected ECU Extract and its dependencies.",
        )]
    })?;
    let system = one(graph, context, roots, "TARGET_NOT_UNIQUE")?;
    if graph.text(system, "CATEGORY") != Some("ECU_EXTRACT") {
        return Err(reject(
            graph,
            system,
            "TARGET_CATEGORY",
            "The target must be an already expanded ECU_EXTRACT.",
        ));
    }
    let ecu = one(
        graph,
        system,
        graph.of_kind("ECU-INSTANCE"),
        "TARGET_NOT_UNIQUE",
    )?;
    selected_channel(graph, ecu)?;
    let root_composition = one(
        graph,
        system,
        graph.descendants(system, "ROOT-SW-COMPOSITION-PROTOTYPE"),
        "MULTIPLE_INSTANCES",
    )?;
    let composition = referenced(
        graph,
        root_composition,
        "SOFTWARE-COMPOSITION-TREF",
        "MULTIPLE_INSTANCES",
    )?;
    let instance = one(
        graph,
        composition,
        graph
            .descendants(composition, "SW-COMPONENT-PROTOTYPE")
            .into_iter()
            .filter(|index| {
                graph.target(*index, "TYPE-TREF").is_some_and(|target| {
                    graph.elements[target].tag == "APPLICATION-SW-COMPONENT-TYPE"
                })
            })
            .collect(),
        "MULTIPLE_INSTANCES",
    )?;
    let component = referenced(graph, instance, "TYPE-TREF", "MULTIPLE_INSTANCES")?;
    let behavior = one(
        graph,
        component,
        graph.descendants(component, "SWC-INTERNAL-BEHAVIOR"),
        "TYPE_CONFLICT",
    )?;
    if !false_value(graph.text(behavior, "SUPPORTS-MULTIPLE-INSTANTIATION")) {
        return Err(reject(
            graph,
            behavior,
            "MULTIPLE_INSTANCES",
            "The application must declare single-instance behavior.",
        ));
    }
    for (root, tags) in [
        (
            component,
            vec![
                ("QUEUED-RECEIVER-COM-SPEC", "QUEUED_UNSUPPORTED"),
                ("QUEUED-SENDER-COM-SPEC", "QUEUED_UNSUPPORTED"),
                ("DATA-READ-ACCESSS", "IMPLICIT_UNSUPPORTED"),
                ("DATA-WRITE-ACCESSS", "IMPLICIT_UNSUPPORTED"),
                ("MODE-ACCESS-POINTS", "MODE_UNSUPPORTED"),
                ("ASYNCHRONOUS-SERVER-CALL-POINT", "ASYNC_UNSUPPORTED"),
                ("VARIATION-POINT", "VARIANT_UNSELECTED"),
            ],
        ),
        (
            system,
            vec![
                ("DATA-TRANSFORMATIONS", "TRANSFORMER_UNSUPPORTED"),
                ("VARIATION-POINT", "VARIANT_UNSELECTED"),
            ],
        ),
    ] {
        for (tag, code) in tags {
            if let Some(index) = graph.descendants(root, tag).first() {
                return Err(vec![graph.diagnostic(*index, DiagnosticCategory::Unsupported, code,
                    format!("{tag} is outside the selected supported profile."),
                    "Use explicit nonqueued S/R, a selected variant and synchronous local services without mode or transformer access.")]);
            }
        }
    }
    for runnable in graph.descendants(behavior, "RUNNABLE-ENTITY") {
        if !false_value(graph.text(runnable, "CAN-BE-INVOKED-CONCURRENTLY")) {
            return Err(reject(
                graph,
                runnable,
                "REENTRANCY_UNSUPPORTED",
                "Application runnables must be nonreentrant.",
            ));
        }
    }
    let event = one(
        graph,
        behavior,
        graph.descendants(behavior, "TIMING-EVENT"),
        "SCHEDULE_NOT_UNIQUE",
    )?;
    let runnable = referenced(graph, event, "START-ON-EVENT-REF", "SCHEDULE_NOT_UNIQUE")?;
    let period = graph
        .text(event, "PERIOD")
        .and_then(milliseconds)
        .ok_or_else(|| {
            reject(
                graph,
                event,
                "PERIOD_UNSUPPORTED",
                "The runnable period must be a positive whole logical millisecond.",
            )
        })?;
    let mut ports = Vec::new();
    for port in graph
        .descendants(component, "R-PORT-PROTOTYPE")
        .into_iter()
        .chain(graph.descendants(component, "P-PORT-PROTOTYPE"))
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
        if graph.elements[interface].tag == "CLIENT-SERVER-INTERFACE" {
            if read
                || !graph
                    .descendants(port, "NONQUEUED-RECEIVER-COM-SPEC")
                    .is_empty()
            {
                return Err(reject(
                    graph,
                    port,
                    "TYPE_CONFLICT",
                    "The explicit S/R port cannot refer to a C/S interface.",
                ));
            }
            continue;
        }
        if graph.elements[interface].tag != "SENDER-RECEIVER-INTERFACE" {
            return Err(reject(
                graph,
                port,
                "TYPE_CONFLICT",
                "Unsupported application port interface.",
            ));
        }
        let element = one(
            graph,
            port,
            graph.descendants(interface, "VARIABLE-DATA-PROTOTYPE"),
            "TYPE_CONFLICT",
        )?;
        let application = referenced(graph, element, "TYPE-TREF", "TYPE_MAP_MISSING")?;
        let implementation = implementation(graph, port, behavior, application)?;
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
            "TYPE_CONFLICT",
        )?;
        if graph.target(specification, "DATA-ELEMENT-REF") != Some(element)
            || graph.elements[initial].text.parse::<u32>().ok() != Some(0)
            || !graph.descendants(interface, "INVALID-VALUE").is_empty()
        {
            return Err(reject(
                graph,
                port,
                "TYPE_CONFLICT",
                "The nonqueued communication specification must bind the same element, initial zero and no invalid-value operation.",
            ));
        }
        let alive_timeout = if read {
            if !matches!(
                graph.text(specification, "HANDLE-NEVER-RECEIVED"),
                Some("true" | "1")
            ) || graph.text(specification, "HANDLE-TIMEOUT-TYPE") != Some("NONE")
            {
                return Err(reject(
                    graph,
                    port,
                    "TYPE_CONFLICT",
                    "Receive semantics require never-received handling and timeout type NONE.",
                ));
            }
            Some(
                graph
                    .text(specification, "ALIVE-TIMEOUT")
                    .and_then(milliseconds)
                    .ok_or_else(|| {
                        reject(
                            graph,
                            port,
                            "TYPE_CONFLICT",
                            "The receive deadline must be a positive whole logical millisecond.",
                        )
                    })?,
            )
        } else {
            None
        };
        let access_tag = if read {
            "DATA-RECEIVE-POINT-BY-ARGUMENTS"
        } else {
            "DATA-SEND-POINTS"
        };
        let access = one(
            graph,
            port,
            graph.descendants(runnable, access_tag),
            "TYPE_CONFLICT",
        )?;
        if graph.descendants(access, "PORT-PROTOTYPE-REF").len() != 1
            || graph.descendants(access, "TARGET-DATA-PROTOTYPE-REF").len() != 1
            || graph.elements[graph.descendants(access, "PORT-PROTOTYPE-REF")[0]].text
                != graph.elements[port].object
            || graph.elements[graph.descendants(access, "TARGET-DATA-PROTOTYPE-REF")[0]].text
                != graph.elements[element].object
        {
            return Err(reject(
                graph,
                port,
                "TYPE_CONFLICT",
                "Runnable access must identify the same port and data element.",
            ));
        }
        let port_name = name(graph, port);
        let element_name = name(graph, element);
        ports.push(DataPort {
            path: graph.elements[port].object.clone(),
            name: port_name.clone(),
            element: graph.elements[element].object.clone(),
            element_name: element_name.clone(),
            application_type: graph.elements[application].object.clone(),
            implementation_type: graph.elements[implementation].object.clone(),
            c_type: "uint32".into(),
            read,
            api_symbol: format!(
                "Rte_{}_{}_{}",
                if read { "Read" } else { "Write" },
                c_name(&port_name),
                c_name(&element_name)
            ),
            runnable: graph.elements[runnable].object.clone(),
            initial_value: 0,
            alive_timeout_ms: alive_timeout,
        });
    }
    if ports.len() != 2 || ports.iter().filter(|port| port.read).count() != 1 {
        return Err(reject(
            graph,
            component,
            "TYPE_CONFLICT",
            "The profile requires one explicit receive and one send data port.",
        ));
    }
    let operation_event = one(
        graph,
        behavior,
        graph.descendants(behavior, "OPERATION-INVOKED-EVENT"),
        "SERVICE_CLIENT_MISSING",
    )?;
    let service_runnable = referenced(
        graph,
        operation_event,
        "START-ON-EVENT-REF",
        "SERVICE_CLIENT_MISSING",
    )?;
    let service_port_ref = one(
        graph,
        operation_event,
        graph.descendants(operation_event, "CONTEXT-P-PORT-REF"),
        "SERVICE_CLIENT_MISSING",
    )?;
    let service_port = *graph
        .objects
        .get(&graph.elements[service_port_ref].text)
        .unwrap();
    let operation_ref = one(
        graph,
        operation_event,
        graph.descendants(operation_event, "TARGET-PROVIDED-OPERATION-REF"),
        "SERVICE_CLIENT_MISSING",
    )?;
    let operation = *graph
        .objects
        .get(&graph.elements[operation_ref].text)
        .unwrap();
    let server_specs = graph.descendants(service_port, "SERVER-COM-SPEC");
    let service_interface = referenced(
        graph,
        service_port,
        "PROVIDED-INTERFACE-TREF",
        "SERVICE_TYPE_CONFLICT",
    )?;
    if server_specs.len() != 1
        || graph.target(server_specs[0], "OPERATION-REF") != Some(operation)
        || graph
            .text(server_specs[0], "QUEUE-LENGTH")
            .and_then(|value| value.parse::<u32>().ok())
            != Some(1)
        || graph.descendants(service_interface, "CLIENT-SERVER-OPERATION") != [operation]
    {
        return Err(reject(
            graph,
            service_port,
            "SERVICE_TYPE_CONFLICT",
            "The synchronous service port must declare exactly the selected ReadData operation with queue length one.",
        ));
    }
    let arguments = graph.descendants(operation, "ARGUMENT-DATA-PROTOTYPE");
    let argument = one(graph, operation, arguments, "SERVICE_TYPE_CONFLICT")?;
    let array = referenced(graph, argument, "TYPE-TREF", "SERVICE_TYPE_CONFLICT")?;
    let byte = one(
        graph,
        array,
        graph.descendants(array, "IMPLEMENTATION-DATA-TYPE-ELEMENT"),
        "SERVICE_TYPE_CONFLICT",
    )?;
    let byte_type_ref = one(
        graph,
        byte,
        graph.descendants(byte, "IMPLEMENTATION-DATA-TYPE-REF"),
        "SERVICE_TYPE_CONFLICT",
    )?;
    let byte_type = *graph
        .objects
        .get(&graph.elements[byte_type_ref].text)
        .unwrap();
    let base_ref = one(
        graph,
        byte_type,
        graph.descendants(byte_type, "BASE-TYPE-REF"),
        "SERVICE_TYPE_CONFLICT",
    )?;
    let base = *graph.objects.get(&graph.elements[base_ref].text).unwrap();
    if graph.text(argument, "DIRECTION") != Some("OUT")
        || graph.text(array, "CATEGORY") != Some("ARRAY")
        || graph.text(byte, "ARRAY-SIZE") != Some("4")
        || graph.text(byte, "ARRAY-SIZE-SEMANTICS") != Some("FIXED-SIZE")
        || graph.text(base, "NATIVE-DECLARATION") != Some("uint8")
        || graph.text(base, "BASE-TYPE-SIZE") != Some("8")
    {
        return Err(reject(
            graph,
            argument,
            "SERVICE_TYPE_CONFLICT",
            "ReadData requires one fixed uint8[4] OUT argument.",
        ));
    }
    let service_interface = referenced(
        graph,
        service_port,
        "PROVIDED-INTERFACE-TREF",
        "SERVICE_TYPE_CONFLICT",
    )?;
    if !graph.within(service_port, component)
        || !graph.within(operation, service_interface)
        || !graph.within(runnable, behavior)
        || !graph.within(service_runnable, behavior)
        || !matches!(
            graph.text(service_interface, "IS-SERVICE"),
            Some("true" | "1")
        )
        || name(graph, operation) != "ReadData"
    {
        return Err(reject(
            graph,
            operation_event,
            "SERVICE_TYPE_CONFLICT",
            "The local service operation and runnable must belong to the selected application and service interface.",
        ));
    }
    let connector = one(
        graph,
        composition,
        graph
            .descendants(composition, "ASSEMBLY-SW-CONNECTOR")
            .into_iter()
            .filter(|connector| {
                graph
                    .descendants(*connector, "TARGET-P-PORT-REF")
                    .into_iter()
                    .any(|reference| {
                        graph.elements[reference].text == graph.elements[service_port].object
                    })
            })
            .collect(),
        "SERVICE_CLIENT_MISSING",
    )?;
    let provider = one(
        graph,
        connector,
        graph.children(connector, "PROVIDER-IREF"),
        "SERVICE_CLIENT_MISSING",
    )?;
    let requester = one(
        graph,
        connector,
        graph.children(connector, "REQUESTER-IREF"),
        "SERVICE_CLIENT_MISSING",
    )?;
    let client_instance = referenced(
        graph,
        requester,
        "CONTEXT-COMPONENT-REF",
        "SERVICE_CLIENT_MISSING",
    )?;
    let client_port = referenced(
        graph,
        requester,
        "TARGET-R-PORT-REF",
        "SERVICE_CLIENT_MISSING",
    )?;
    let client_component = referenced(
        graph,
        client_instance,
        "TYPE-TREF",
        "SERVICE_CLIENT_MISSING",
    )?;
    let client_specs = graph.descendants(client_port, "CLIENT-COM-SPEC");
    if client_specs.len() != 1 || graph.target(client_specs[0], "OPERATION-REF") != Some(operation)
    {
        return Err(reject(
            graph,
            client_port,
            "SERVICE_CLIENT_MISSING",
            "The client communication specification must declare the same connected operation.",
        ));
    }
    if graph.target(provider, "CONTEXT-COMPONENT-REF") != Some(instance)
        || !graph.within(client_port, client_component)
        || graph.target(client_port, "REQUIRED-INTERFACE-TREF") != Some(service_interface)
        || graph.elements[client_component].tag != "SERVICE-SW-COMPONENT-TYPE"
    {
        return Err(reject(
            graph,
            connector,
            "SERVICE_CLIENT_MISSING",
            "The connector must bind the same application instance to its declared local service client.",
        ));
    }
    if !graph
        .descendants(client_component, "ASYNCHRONOUS-SERVER-CALL-POINT")
        .is_empty()
    {
        return Err(vec![graph.diagnostic(
            client_component,
            DiagnosticCategory::Unsupported,
            "ASYNC_UNSUPPORTED",
            "The service client requests asynchronous execution.",
            "Use a synchronous server call in the same ECU owner task.",
        )]);
    }
    let client_call = one(
        graph,
        client_component,
        graph.descendants(client_component, "SYNCHRONOUS-SERVER-CALL-POINT"),
        "SERVICE_CLIENT_MISSING",
    )?;
    let operation_iref = one(
        graph,
        client_call,
        graph.children(client_call, "OPERATION-IREF"),
        "SERVICE_CLIENT_MISSING",
    )?;
    if graph.target(operation_iref, "CONTEXT-R-PORT-REF") != Some(client_port)
        || graph.target(operation_iref, "TARGET-REQUIRED-OPERATION-REF") != Some(operation)
    {
        return Err(reject(
            graph,
            client_call,
            "SERVICE_CLIENT_MISSING",
            "The synchronous client call must reference the same connected service operation.",
        ));
    }
    let client_runnable = one(
        graph,
        client_component,
        graph
            .descendants(client_component, "RUNNABLE-ENTITY")
            .into_iter()
            .filter(|candidate| graph.within(client_call, *candidate))
            .collect(),
        "SERVICE_CLIENT_MISSING",
    )?;
    for bound_instance in [instance, client_instance] {
        let binding = one(
            graph,
            system,
            graph
                .descendants(system, "SWC-TO-ECU-MAPPING")
                .into_iter()
                .filter(|mapping| {
                    graph
                        .descendants(*mapping, "TARGET-COMPONENT-REF")
                        .into_iter()
                        .any(|reference| {
                            graph.elements[reference].text == graph.elements[bound_instance].object
                        })
                })
                .collect(),
            "SERVICE_ECU_CONFLICT",
        )?;
        if graph.target(binding, "ECU-INSTANCE-REF") != Some(ecu) {
            return Err(reject(
                graph,
                binding,
                "SERVICE_ECU_CONFLICT",
                "Application and service client must map to the same selected ECU.",
            ));
        }
    }
    ports.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(ComponentContract {
        component: graph.elements[component].object.clone(),
        instance: graph.elements[instance].object.clone(),
        ecu: graph.elements[ecu].object.clone(),
        behavior: graph.elements[behavior].object.clone(),
        data_ports: ports,
        service: ServicePort {
            path: graph.elements[service_port].object.clone(),
            name: name(graph, service_port),
            operation: graph.elements[operation].object.clone(),
            operation_name: name(graph, operation),
            array_type: graph.elements[array].object.clone(),
            array_length: 4,
            runnable: graph.elements[service_runnable].object.clone(),
            runnable_symbol: graph.text(service_runnable, "SYMBOL").unwrap_or("").into(),
            client_instance: graph.elements[client_instance].object.clone(),
            client_port: graph.elements[client_port].object.clone(),
            client_runnable: graph.elements[client_runnable].object.clone(),
            client_symbol: graph.text(client_runnable, "SYMBOL").unwrap_or("").into(),
        },
        timing_event: graph.elements[event].object.clone(),
        periodic_runnable: graph.elements[runnable].object.clone(),
        periodic_symbol: graph.text(runnable, "SYMBOL").unwrap_or("").into(),
        period_ms: period,
    })
}
