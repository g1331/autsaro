use super::component::{ComponentContract, milliseconds};
use super::graph::Graph;
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledEntity {
    pub mapping: String,
    pub event: String,
    pub entity: String,
    pub symbol: String,
    pub task: String,
    pub alarm: String,
    pub os_event: String,
    pub period_ms: u32,
    pub position: u32,
    pub application: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleContract {
    pub task: String,
    pub task_priority: u8,
    pub counter: String,
    pub counter_maximum: u32,
    pub entities: Vec<ScheduledEntity>,
    pub synchronous_event: String,
}

fn reject(graph: &Graph, index: usize, code: &str, message: &str) -> Vec<PlanDiagnostic> {
    vec![graph.diagnostic(index, DiagnosticCategory::Input, code, message,
        "Supply one explicit event/task/alarm mapping with matching logical period and a unique declared position in the ECU owner task.")]
}

pub(super) fn definition_is(graph: &Graph, index: usize, kind: &str) -> bool {
    graph
        .text(index, "DEFINITION-REF")
        .is_some_and(|definition| definition.ends_with(&format!("/{kind}")))
}

pub(super) fn values<'a>(
    graph: &'a Graph,
    container: usize,
    name: &str,
    reference: bool,
) -> Vec<&'a str> {
    graph
        .children(
            container,
            if reference {
                "REFERENCE-VALUES"
            } else {
                "PARAMETER-VALUES"
            },
        )
        .into_iter()
        .flat_map(|group| graph.elements[group].children.iter().copied())
        .filter(|index| definition_is(graph, *index, name))
        .filter_map(|index| graph.text(index, if reference { "VALUE-REF" } else { "VALUE" }))
        .collect()
}

pub(super) fn value<'a>(
    graph: &'a Graph,
    container: usize,
    name: &str,
    reference: bool,
) -> Option<&'a str> {
    let found = values(graph, container, name, reference);
    if found.len() == 1 {
        Some(found[0])
    } else {
        None
    }
}

fn enclosing(graph: &Graph, mut index: usize, kind: &str) -> Option<usize> {
    while let Some(parent) = graph.elements[index].parent {
        if definition_is(graph, parent, kind) {
            return Some(parent);
        }
        index = parent;
    }
    None
}

fn container(graph: &Graph, kind: &str, context: usize) -> Result<usize, Vec<PlanDiagnostic>> {
    let found: Vec<_> = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, kind))
        .collect();
    if found.len() == 1 {
        Ok(found[0])
    } else {
        Err(reject(
            graph,
            context,
            "SCHEDULE_NOT_UNIQUE",
            "A required ECUC scheduling object is missing or duplicated.",
        ))
    }
}

fn reference(graph: &Graph, mapping: usize, name: &str) -> Result<usize, Vec<PlanDiagnostic>> {
    value(graph, mapping, name, true)
        .and_then(|path| graph.objects.get(path).copied())
        .ok_or_else(|| {
            reject(
                graph,
                mapping,
                "SCHEDULE_NOT_UNIQUE",
                "The scheduling reference is missing or duplicated.",
            )
        })
}

pub(super) fn inspect(
    graph: &Graph,
    component: &ComponentContract,
) -> Result<ScheduleContract, Vec<PlanDiagnostic>> {
    let behavior = *graph.objects.get(&component.behavior).unwrap();
    let task = container(graph, "OsTask", behavior)?;
    let counter = container(graph, "OsCounter", behavior)?;
    let priority = value(graph, task, "OsTaskPriority", false)
        .and_then(|value| value.parse::<u8>().ok())
        .filter(|value| (1..=30).contains(value));
    if priority.is_none()
        || value(graph, task, "OsTaskActivation", false) != Some("1")
        || value(graph, task, "OsTaskSchedule", false) != Some("FULL")
        || values(graph, task, "OsTaskEventRef", true).len() != 3
        || graph
            .descendants(task, "ECUC-CONTAINER-VALUE")
            .into_iter()
            .filter(|index| definition_is(graph, *index, "OsTaskAutostart"))
            .count()
            != 1
    {
        return Err(reject(
            graph,
            task,
            "TASK_PROFILE",
            "The ECU owner must be one autostart FULL Extended Task with activation limit one and three explicit events.",
        ));
    }
    let maximum = value(graph, counter, "OsCounterMaxAllowedValue", false)
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value != 0);
    if maximum.is_none()
        || value(graph, counter, "OsCounterType", false) != Some("SOFTWARE")
        || value(graph, counter, "OsCounterTicksPerBase", false) != Some("1")
        || value(graph, counter, "OsCounterMinCycle", false) != Some("1")
        || value(graph, counter, "OsSecondsPerTick", false).and_then(milliseconds) != Some(1)
    {
        return Err(reject(
            graph,
            counter,
            "COUNTER_PROFILE",
            "SystemCounter must be a software counter with one logical millisecond per tick and unit base/minimum cycle.",
        ));
    }
    let task_events: BTreeSet<_> = values(graph, task, "OsTaskEventRef", true)
        .into_iter()
        .collect();
    let task_autostart = graph
        .descendants(task, "ECUC-CONTAINER-VALUE")
        .into_iter()
        .find(|index| definition_is(graph, *index, "OsTaskAutostart"))
        .unwrap();
    let modes: BTreeSet<_> = values(graph, task_autostart, "OsTaskAppModeRef", true)
        .into_iter()
        .collect();
    if modes.len() != 1
        || !modes.iter().all(|path| {
            graph
                .objects
                .get(*path)
                .is_some_and(|index| definition_is(graph, *index, "OsAppMode"))
        })
    {
        return Err(reject(
            graph,
            task_autostart,
            "TASK_PROFILE",
            "The owner task requires one explicit selected autostart application mode.",
        ));
    }
    if task_events.len() != 3 {
        return Err(reject(
            graph,
            task,
            "TASK_PROFILE",
            "The owner task's three event references must be distinct.",
        ));
    }
    let mut entities = Vec::new();
    let mut event_set = BTreeSet::new();
    let mut positions = BTreeSet::new();
    let mut synchronous = None;
    for mapping in graph.of_kind("ECUC-CONTAINER-VALUE") {
        let application = definition_is(graph, mapping, "RteEventToTaskMapping");
        if !application && !definition_is(graph, mapping, "RteBswEventToTaskMapping") {
            continue;
        }
        let prefix = if application { "Rte" } else { "RteBsw" };
        let event = reference(graph, mapping, &format!("{prefix}EventRef"))?;
        let instance_kind = if application {
            "RteSwComponentInstance"
        } else {
            "RteBswModuleInstance"
        };
        let instance = enclosing(graph, mapping, instance_kind).ok_or_else(|| {
            reject(
                graph,
                mapping,
                "INSTANCE_MAPPING",
                "The event mapping has no owning component/module instance.",
            )
        })?;
        if application {
            if value(graph, instance, "RteSoftwareComponentInstanceRef", true)
                != Some(component.instance.as_str())
            {
                return Err(reject(
                    graph,
                    mapping,
                    "INSTANCE_MAPPING",
                    "The application event mapping refers to a different component instance.",
                ));
            }
        } else {
            let implementation = reference(graph, instance, "RteBswImplementationRef")?;
            let implementation_behavior =
                graph
                    .target(implementation, "BEHAVIOR-REF")
                    .ok_or_else(|| {
                        reject(
                            graph,
                            instance,
                            "BSW_ENTRY_MISSING",
                            "The selected BSW implementation has no behavior description.",
                        )
                    })?;
            if !graph.within(event, implementation_behavior)
                || graph.text(implementation, "PROGRAMMING-LANGUAGE") != Some("C")
            {
                return Err(reject(
                    graph,
                    mapping,
                    "INSTANCE_MAPPING",
                    "The BSW event must belong to the selected C implementation's behavior.",
                ));
            }
        }
        if !event_set.insert(event) {
            return Err(reject(
                graph,
                mapping,
                "SCHEDULE_NOT_UNIQUE",
                "An event is mapped more than once.",
            ));
        }
        if graph.elements[event].tag == "OPERATION-INVOKED-EVENT" {
            if !application
                || synchronous.is_some()
                || !graph.within(event, behavior)
                || !values(graph, mapping, "RteMappedToTaskRef", true).is_empty()
                || !values(graph, mapping, "RteUsedOsAlarmRef", true).is_empty()
                || !values(graph, mapping, "RteUsedOsEventRef", true).is_empty()
                || !values(graph, mapping, "RtePositionInTask", false).is_empty()
            {
                return Err(reject(
                    graph,
                    mapping,
                    "SERVICE_ASYNC_MAPPING",
                    "The synchronous operation must not activate a separate task or periodic alarm.",
                ));
            }
            synchronous = Some(graph.elements[event].object.clone());
            continue;
        }
        let period = graph
            .text(event, "PERIOD")
            .and_then(milliseconds)
            .ok_or_else(|| {
                reject(
                    graph,
                    event,
                    "PERIOD_UNSUPPORTED",
                    "The mapped period must be a positive whole millisecond.",
                )
            })?;
        let mapped_task = reference(graph, mapping, &format!("{prefix}MappedToTaskRef"))?;
        let alarm = reference(graph, mapping, &format!("{prefix}UsedOsAlarmRef"))?;
        let os_event = reference(graph, mapping, &format!("{prefix}UsedOsEventRef"))?;
        let position = value(graph, mapping, &format!("{prefix}PositionInTask"), false)
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|value| *value != 0)
            .ok_or_else(|| {
                reject(
                    graph,
                    mapping,
                    "SCHEDULE_NOT_UNIQUE",
                    "Each periodic mapping requires a positive explicit task position.",
                )
            })?;
        if mapped_task != task
            || !definition_is(graph, alarm, "OsAlarm")
            || !definition_is(graph, os_event, "OsEvent")
            || !positions.insert(position)
            || !task_events.contains(graph.elements[os_event].object.as_str())
            || reference(graph, alarm, "OsAlarmCounterRef")? != counter
        {
            return Err(reject(
                graph,
                mapping,
                "SCHEDULE_NOT_UNIQUE",
                "Task, event, counter, alarm or task position is inconsistent.",
            ));
        }
        let autostarts: Vec<_> = graph
            .descendants(alarm, "ECUC-CONTAINER-VALUE")
            .into_iter()
            .filter(|index| definition_is(graph, *index, "OsAlarmAutostart"))
            .collect();
        let actions: Vec<_> = graph
            .descendants(alarm, "ECUC-CONTAINER-VALUE")
            .into_iter()
            .filter(|index| definition_is(graph, *index, "OsAlarmSetEvent"))
            .collect();
        if autostarts.len() != 1 || actions.len() != 1 {
            return Err(reject(
                graph,
                alarm,
                "PERIOD_ALARM_CONFLICT",
                "The periodic alarm needs one explicit autostart and SetEvent action.",
            ));
        }
        let autostart = autostarts[0];
        let alarm_modes: BTreeSet<_> = values(graph, autostart, "OsAlarmAppModeRef", true)
            .into_iter()
            .collect();
        if alarm_modes != modes {
            return Err(reject(
                graph,
                autostart,
                "PERIOD_ALARM_CONFLICT",
                "The periodic alarm and ECU owner task use different autostart modes.",
            ));
        }
        if value(graph, autostart, "OsAlarmAutostartType", false) != Some("RELATIVE")
            || value(graph, autostart, "OsAlarmAlarmTime", false)
                .and_then(|value| value.parse::<u32>().ok())
                != Some(period)
            || value(graph, autostart, "OsAlarmCycleTime", false)
                .and_then(|value| value.parse::<u32>().ok())
                != Some(period)
            || reference(graph, actions[0], "OsAlarmSetEventTaskRef")? != task
            || reference(graph, actions[0], "OsAlarmSetEventRef")? != os_event
        {
            return Err(reject(
                graph,
                event,
                "PERIOD_ALARM_CONFLICT",
                "The event period, relative alarm start/cycle and SetEvent action disagree.",
            ));
        }
        let entity = graph
            .target(
                event,
                if application {
                    "START-ON-EVENT-REF"
                } else {
                    "STARTS-ON-EVENT-REF"
                },
            )
            .ok_or_else(|| {
                reject(
                    graph,
                    event,
                    "SCHEDULE_NOT_UNIQUE",
                    "The scheduled event has no unique entity.",
                )
            })?;
        let symbol = if application {
            if graph.elements[event].object != component.timing_event
                || !graph.within(entity, behavior)
            {
                return Err(reject(
                    graph,
                    event,
                    "SCHEDULE_NOT_UNIQUE",
                    "The application timing mapping targets a different behavior.",
                ));
            }
            graph.text(entity, "SYMBOL").unwrap_or("").to_owned()
        } else {
            let entry = graph
                .target(entity, "IMPLEMENTED-ENTRY-REF")
                .ok_or_else(|| {
                    reject(
                        graph,
                        entity,
                        "BSW_ENTRY_MISSING",
                        "The schedulable BSW entity lacks its implementation entry.",
                    )
                })?;
            graph.text(entry, "SHORT-NAME").unwrap_or("").to_owned()
        };
        entities.push(ScheduledEntity {
            mapping: graph.elements[mapping].object.clone(),
            event: graph.elements[event].object.clone(),
            entity: graph.elements[entity].object.clone(),
            symbol,
            task: graph.elements[task].object.clone(),
            alarm: graph.elements[alarm].object.clone(),
            os_event: graph.elements[os_event].object.clone(),
            period_ms: period,
            position,
            application,
        });
    }
    if synchronous.is_none() || entities.iter().filter(|entity| entity.application).count() != 1 {
        return Err(reject(
            graph,
            behavior,
            "SCHEDULE_NOT_UNIQUE",
            "The selected application requires one periodic and one synchronous operation mapping.",
        ));
    }
    for event in graph.of_kind("BSW-TIMING-EVENT") {
        if !event_set.contains(&event) {
            return Err(reject(
                graph,
                event,
                "SCHEDULE_NOT_UNIQUE",
                "A declared BSW timing event has no ECU task mapping.",
            ));
        }
    }
    entities.sort_by_key(|entity| entity.position);
    let expected_order = [
        "Can_MainFunction_Wakeup",
        "CanTp_AdvanceTime",
        "Com_AdvanceTime",
        component.periodic_symbol.as_str(),
        "Com_TriggerTransmit",
        "Dcm_AdvanceTime",
    ];
    if entities
        .iter()
        .map(|entity| entity.symbol.as_str())
        .collect::<Vec<_>>()
        != expected_order
    {
        return Err(reject(
            graph,
            behavior,
            "SCHEDULE_ORDER",
            "Declared task positions do not match the target's required driver, transport, receive, application, transmit and diagnostic order.",
        ));
    }
    Ok(ScheduleContract {
        task: graph.elements[task].object.clone(),
        task_priority: priority.unwrap(),
        counter: graph.elements[counter].object.clone(),
        counter_maximum: maximum.unwrap(),
        entities,
        synchronous_event: synchronous.unwrap(),
    })
}
