//! R24-11 ComM service semantics over the common typed C/S and mode plans.
use super::component::{name, one, referenced};
use super::composition::CompositionContract;
use super::graph::Graph;
use super::{DiagnosticCategory, PlanDiagnostic};
use std::collections::BTreeMap;

pub(super) fn callback_name(user: &str) -> String {
    format!(
        "Ecu_ComMMode_UM_{}",
        super::component::c_name(super::short_name(user))
    )
}

pub(super) fn channels_name(handle: u16) -> String {
    format!("comm_channels_{handle}")
}

fn reject(graph: &Graph, at: usize, code: &str) -> Vec<PlanDiagnostic> {
    vec![graph.diagnostic(
        at,
        DiagnosticCategory::Input,
        code,
        crate::product_message!("backend.integration.composition.contract_invalid", "code" => code),
        crate::product_message!("backend.integration.composition.repair_contract"),
    )]
}

pub(super) fn inspect(
    graph: &Graph,
    plan: &CompositionContract,
) -> Result<(), Vec<PlanDiagnostic>> {
    let services: Vec<_> = plan
        .components
        .iter()
        .filter(|component| {
            component
                .operations
                .iter()
                .any(|op| !op.read && super::short_name(&op.interface) == "ComM_UserRequest")
                || component
                    .mode_ports
                    .iter()
                    .any(|port| !port.read && super::short_name(&port.group) == "ComMMode")
        })
        .collect();
    let users: Vec<_> = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| super::schedule::definition_is(graph, *index, "ComMUser"))
        .collect();
    let at = graph.index(&plan.composition)?;
    let service = one(
        graph,
        at,
        services
            .iter()
            .map(|c| graph.index(&c.component))
            .collect::<Result<Vec<_>, _>>()?,
        "SERVICE_TYPE_CONFLICT",
    )?;
    let component = services[0];
    if graph.elements[service].tag != "SERVICE-SW-COMPONENT-TYPE"
        || component.operations.iter().filter(|op| !op.read).count() != users.len() * 5
        || component.mode_ports.len() != users.len()
    {
        return Err(reject(graph, service, "SERVICE_TYPE_CONFLICT"));
    }
    for user in users {
        let suffix = name(graph, user);
        let ports = graph.descendants(service, "P-PORT-PROTOTYPE");
        let port = one(
            graph,
            service,
            ports
                .iter()
                .copied()
                .filter(|port| name(graph, *port) == format!("UR_{suffix}"))
                .collect(),
            "SERVICE_TYPE_CONFLICT",
        )?;
        let port_path = &graph.elements[port].object;
        let args = component.port_arguments.get(port_path);
        let handle = super::schedule::value(graph, user, "ComMUserIdentifier", false)
            .and_then(super::rte_types::integer_constant);
        if args.is_none_or(|args| {
            args.len() != 1
                || args[0].native_type != "ComM_UserHandleType"
                || super::rte_types::integer_constant(args[0].value.trim_end_matches('U')) != handle
        }) {
            return Err(reject(graph, port, "SERVICE_TYPE_CONFLICT"));
        }
        let interface = referenced(
            graph,
            port,
            "PROVIDED-INTERFACE-TREF",
            "SERVICE_TYPE_CONFLICT",
        )?;
        let errors: BTreeMap<_, _> = graph
            .descendants(interface, "APPLICATION-ERROR")
            .into_iter()
            .map(|error| {
                (
                    name(graph, error),
                    graph
                        .text(error, "ERROR-CODE")
                        .and_then(super::rte_types::integer_constant),
                )
            })
            .collect();
        let all_errors = [
            "E_OK",
            "E_NOT_OK",
            "COMM_E_MODE_LIMITATION",
            "COMM_E_MULTIPLE_PNC_ASSIGNED",
            "COMM_E_NO_PNC_ASSIGNED",
        ];
        if graph.text(interface, "IS-SERVICE") != Some("true")
            || errors
                != all_errors
                    .iter()
                    .enumerate()
                    .map(|(value, name)| ((*name).to_owned(), Some(value as i128)))
                    .collect()
        {
            return Err(reject(graph, interface, "SERVICE_TYPE_CONFLICT"));
        }
        for operation in [
            "RequestComMode",
            "GetRequestedComMode",
            "GetCurrentComMode",
            "GetMaxComMode",
            "GetCurrentPNCComMode",
        ] {
            let source =
                graph.index(&format!("{}/{operation}", graph.elements[interface].object))?;
            let op = component
                .operations
                .iter()
                .find(|op| op.port == *port_path && op.operation == graph.elements[source].object)
                .ok_or_else(|| reject(graph, port, "SERVICE_TYPE_CONFLICT"))?;
            let expected = if operation == "RequestComMode" {
                &all_errors[..3]
            } else if operation == "GetCurrentPNCComMode" {
                &[
                    "E_OK",
                    "E_NOT_OK",
                    "COMM_E_MULTIPLE_PNC_ASSIGNED",
                    "COMM_E_NO_PNC_ASSIGNED",
                ]
            } else {
                &all_errors[..2]
            };
            let expected_errors: BTreeMap<_, _> = expected
                .iter()
                .filter_map(|error| {
                    all_errors
                        .iter()
                        .position(|name| name == error)
                        .map(|value| (format!("RTE_E_ComM_UserRequest_{error}"), value as u8))
                })
                .collect();
            let request = operation == "RequestComMode";
            if op.arguments.len() != 1
                || op.arguments[0].name != "ComMode"
                || op.arguments[0].native_type
                    != if request {
                        "ComM_ModeType"
                    } else {
                        "ComM_ModeType *"
                    }
                || op.arguments[0].direction != if request { "IN" } else { "OUT" }
                || op.application_errors != expected_errors
            {
                return Err(reject(graph, source, "SERVICE_TYPE_CONFLICT"));
            }
            let target = component
                .runnables
                .iter()
                .find(|run| run.path == op.runnable)
                .and_then(|run| run.bsw_target.as_ref());
            if target.is_none_or(|target| target.contract.symbol != format!("ComM_{operation}")) {
                return Err(reject(graph, source, "BSW_MAPPING_INVALID"));
            }
        }
        let port = one(
            graph,
            service,
            ports
                .iter()
                .copied()
                .filter(|port| name(graph, *port) == format!("UM_{suffix}"))
                .collect(),
            "SERVICE_TYPE_CONFLICT",
        )?;
        let interface = referenced(graph, port, "PROVIDED-INTERFACE-TREF", "MODE_TYPE_CONFLICT")?;
        let prototype = one(
            graph,
            interface,
            graph.descendants(interface, "MODE-GROUP"),
            "MODE_TYPE_CONFLICT",
        )?;
        let group = referenced(graph, prototype, "TYPE-TREF", "MODE_TYPE_CONFLICT")?;
        let modes = graph.descendants(group, "MODE-DECLARATION");
        if name(graph, interface) != "ComM_CurrentMode"
            || graph.text(interface, "IS-SERVICE") != Some("true")
            || name(graph, prototype) != "currentMode"
            || name(graph, group) != "ComMMode"
            || graph.text(group, "CATEGORY") != Some("ALPHABETIC_ORDER")
            || graph.text(group, "ON-TRANSITION-VALUE").is_some()
            || modes.len() != 3
            || modes
                .iter()
                .any(|mode| graph.text(*mode, "VALUE").is_some())
            || ![
                "COMM_NO_COMMUNICATION",
                "COMM_SILENT_COMMUNICATION",
                "COMM_FULL_COMMUNICATION",
            ]
            .iter()
            .all(|mode| modes.iter().any(|index| name(graph, *index) == *mode))
            || graph
                .target(group, "INITIAL-MODE-REF")
                .is_none_or(|initial| {
                    !modes.contains(&initial) || name(graph, initial) != "COMM_NO_COMMUNICATION"
                })
        {
            return Err(reject(graph, group, "MODE_TYPE_CONFLICT"));
        }
    }
    Ok(())
}

pub(super) fn inspect_bindings(
    graph: &Graph,
    plan: &CompositionContract,
) -> Result<(), Vec<PlanDiagnostic>> {
    for machine in &plan.mode_machines {
        if super::short_name(&machine.group) == "ComMMode"
            && (machine.module != "ComM"
                || !machine.bsw_managed
                || super::short_name(&machine.bsw_group)
                    != super::short_name(&machine.publisher.port))
        {
            return Err(reject(
                graph,
                graph.index(&machine.publisher.port)?,
                "MODE_MAPPING_INVALID",
            ));
        }
    }
    Ok(())
}
