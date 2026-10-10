use super::component::{ComponentContract, milliseconds};
use super::graph::Graph;
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledEntity {
    pub mapping: String,
    pub event: String,
    pub entity: String,
    pub symbol: String,
    pub task: String,
    pub alarm: String,
    pub expiry_point: Option<String>,
    pub schedule_table: Option<String>,
    pub expiry_offset: Option<u32>,
    pub table_start: Option<u32>,
    pub os_event: String,
    pub period_ms: u32,
    pub position: u32,
    pub application: bool,
}

impl ScheduledEntity {
    pub fn trigger(&self) -> &str {
        self.expiry_point.as_deref().unwrap_or(&self.alarm)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleContract {
    pub task: String,
    pub task_priority: u8,
    pub counter: String,
    pub counter_maximum: u32,
    pub counter_ticks_per_base: u32,
    pub counter_minimum_cycle: u32,
    pub counter_tick_ms: u32,
    pub entities: Vec<ScheduledEntity>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub synchronous_event: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub synchronous_events: Vec<String>,
}

fn reject(
    graph: &Graph,
    index: usize,
    code: &str,
    message: crate::message::LocalizedText,
) -> Vec<PlanDiagnostic> {
    vec![graph.diagnostic(
        index,
        DiagnosticCategory::Input,
        code,
        message,
        crate::product_message!(
            "backend.integration.schedule.explicit_event_task_alarm_mapping_required"
        ),
    )]
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
            crate::product_message!(
                "backend.integration.schedule.ecuc_scheduling_object_missing_or_duplicate"
            ),
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
                crate::product_message!(
                    "backend.integration.schedule.scheduling_reference_missing_or_duplicate"
                ),
            )
        })
}

pub(super) fn inspect(
    graph: &Graph,
    component: &ComponentContract,
) -> Result<ScheduleContract, Vec<PlanDiagnostic>> {
    inspect_events(
        graph,
        &[(component.instance.as_str(), component.behavior.as_str())],
        &[component.timing_event.clone()],
        &[component.service.runnable.clone()],
        false,
    )
}

pub(super) fn inspect_events(
    graph: &Graph,
    components: &[(&str, &str)],
    periodic_events: &[String],
    server_runnables: &[String],
    multi: bool,
) -> Result<ScheduleContract, Vec<PlanDiagnostic>> {
    let behavior = *graph.objects.get(components[0].1).unwrap();
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
            crate::product_message!(
                "backend.integration.schedule.ecu_owner_task_configuration_required"
            ),
        ));
    }
    let maximum = value(graph, counter, "OsCounterMaxAllowedValue", false)
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value <= 65535 && (multi || *value != 0));
    let counter_tick_ms = value(graph, counter, "OsSecondsPerTick", false).and_then(milliseconds);
    let counter_ticks_per_base = value(graph, counter, "OsCounterTicksPerBase", false)
        .and_then(|text| text.parse::<u32>().ok());
    let counter_minimum_cycle =
        value(graph, counter, "OsCounterMinCycle", false).and_then(|text| text.parse::<u32>().ok());
    if maximum.is_none()
        || value(graph, counter, "OsCounterType", false) != Some("SOFTWARE")
        || counter_ticks_per_base != Some(1)
        || counter_minimum_cycle != Some(1)
        || value(graph, counter, "OsCounterTicksPerBase", false) != Some("1")
        || value(graph, counter, "OsCounterMinCycle", false) != Some("1")
        || counter_tick_ms != Some(1)
    {
        return Err(reject(
            graph,
            counter,
            "COUNTER_PROFILE",
            crate::product_message!(
                "backend.integration.schedule.system_counter_configuration_required"
            ),
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
            crate::product_message!(
                "backend.integration.schedule.owner_task_autostart_application_mode_required"
            ),
        ));
    }
    if task_events.len() != 3 {
        return Err(reject(
            graph,
            task,
            "TASK_PROFILE",
            crate::product_message!(
                "backend.integration.schedule.owner_task_event_references_must_be_distinct"
            ),
        ));
    }
    let mut entities = Vec::new();
    let mut event_set = BTreeSet::new();
    let mut positions = BTreeSet::new();
    let mut synchronous = Vec::new();
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
                crate::product_message!(
                    "backend.integration.schedule.event_mapping_owner_instance_missing"
                ),
            )
        })?;
        if application {
            let owner = value(graph, instance, "RteSoftwareComponentInstanceRef", true);
            if !components
                .iter()
                .any(|(component_instance, component_behavior)| {
                    owner == Some(*component_instance)
                        && graph
                            .objects
                            .get(*component_behavior)
                            .is_some_and(|behavior| graph.within(event, *behavior))
                })
            {
                return Err(reject(
                    graph,
                    mapping,
                    "INSTANCE_MAPPING",
                    crate::product_message!(
                        "backend.integration.schedule.application_event_mapping_component_mismatch"
                    ),
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
                            crate::product_message!(
                                "backend.integration.schedule.bsw_implementation_behavior_missing"
                            ),
                        )
                    })?;
            if !graph.within(event, implementation_behavior)
                || graph.text(implementation, "PROGRAMMING-LANGUAGE") != Some("C")
            {
                return Err(reject(
                    graph,
                    mapping,
                    "INSTANCE_MAPPING",
                    crate::product_message!(
                        "backend.integration.schedule.bsw_event_implementation_behavior_mismatch"
                    ),
                ));
            }
        }
        if !event_set.insert(event) {
            return Err(reject(
                graph,
                mapping,
                "SCHEDULE_NOT_UNIQUE",
                crate::product_message!("backend.integration.schedule.event_mapped_multiple_times"),
            ));
        }
        if graph.elements[event].tag == "OPERATION-INVOKED-EVENT" {
            if !application
                || (!multi && !synchronous.is_empty())
                || !graph
                    .target(event, "START-ON-EVENT-REF")
                    .is_some_and(|runnable| {
                        server_runnables.contains(&graph.elements[runnable].object)
                    })
                || (multi
                    && !matches!(
                        value(graph, mapping, "RteEventIsMappedToTask", false),
                        Some("false" | "0")
                    ))
                || !values(graph, mapping, "RteMappedToTaskRef", true).is_empty()
                || !values(graph, mapping, "RteUsedOsAlarmRef", true).is_empty()
                || !values(graph, mapping, "RteUsedOsSchTblExpiryPointRef", true).is_empty()
                || !values(graph, mapping, "RteUsedOsEventRef", true).is_empty()
                || !values(graph, mapping, "RtePositionInTask", false).is_empty()
            {
                return Err(reject(
                    graph,
                    mapping,
                    "SERVICE_ASYNC_MAPPING",
                    crate::product_message!(
                        "backend.integration.schedule.synchronous_operation_separate_activation_forbidden"
                    ),
                ));
            }
            synchronous.push(graph.elements[event].object.clone());
            continue;
        }
        if multi
            && !matches!(
                value(
                    graph,
                    mapping,
                    &format!("{prefix}EventIsMappedToTask"),
                    false
                ),
                Some("true" | "1")
            )
        {
            return Err(reject(
                graph,
                mapping,
                "INSTANCE_MAPPING",
                crate::product_message!(
                    "backend.integration.schedule.scheduling_mapping_inconsistent"
                ),
            ));
        }
        let period = graph
            .text(event, "PERIOD")
            .and_then(milliseconds)
            .ok_or_else(|| {
                reject(
                    graph,
                    event,
                    "PERIOD_UNSUPPORTED",
                    crate::product_message!("backend.integration.schedule.mapped_period_must_be_positive_whole_millisecond"),
                )
            })?;
        if multi && period > maximum.unwrap() {
            return Err(reject(
                graph,
                event,
                "PERIOD_ALARM_CONFLICT",
                crate::product_message!(
                    "backend.integration.schedule.event_period_alarm_and_set_event_mismatch"
                ),
            ));
        }
        let mapped_task = reference(graph, mapping, &format!("{prefix}MappedToTaskRef"))?;
        let alarm_refs = values(graph, mapping, &format!("{prefix}UsedOsAlarmRef"), true);
        let expiry_refs = values(
            graph,
            mapping,
            &format!("{prefix}UsedOsSchTblExpiryPointRef"),
            true,
        );
        if alarm_refs.len() + expiry_refs.len() != 1 {
            return Err(reject(
                graph,
                mapping,
                "SCHEDULE_NOT_UNIQUE",
                crate::product_message!(
                    "backend.integration.schedule.single_alarm_or_expiry_point_timing_source_required"
                ),
            ));
        }
        let using_table = !expiry_refs.is_empty();
        let alarm = reference(
            graph,
            mapping,
            &format!(
                "{prefix}UsedOs{}Ref",
                if using_table {
                    "SchTblExpiryPoint"
                } else {
                    "Alarm"
                }
            ),
        )?;
        let table = if using_table {
            enclosing(graph, alarm, "OsScheduleTable")
        } else {
            None
        };
        let os_event = reference(graph, mapping, &format!("{prefix}UsedOsEventRef"))?;
        let position = value(graph, mapping, &format!("{prefix}PositionInTask"), false)
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|value| *value <= 65535 && (multi || *value != 0))
            .ok_or_else(|| {
                reject(
                    graph,
                    mapping,
                    "SCHEDULE_NOT_UNIQUE",
                    crate::product_message!("backend.integration.schedule.periodic_mapping_positive_task_position_required"),
                )
            })?;
        if mapped_task != task
            || (!using_table && !definition_is(graph, alarm, "OsAlarm"))
            || (using_table
                && (!definition_is(graph, alarm, "OsScheduleTableExpiryPoint") || table.is_none()))
            || !definition_is(graph, os_event, "OsEvent")
            || !positions.insert(position)
            || !task_events.contains(graph.elements[os_event].object.as_str())
            || reference(
                graph,
                table.unwrap_or(alarm),
                if using_table {
                    "OsScheduleTableCounterRef"
                } else {
                    "OsAlarmCounterRef"
                },
            )? != counter
        {
            return Err(reject(
                graph,
                mapping,
                "SCHEDULE_NOT_UNIQUE",
                crate::product_message!(
                    "backend.integration.schedule.scheduling_mapping_inconsistent"
                ),
            ));
        }
        let autostarts: Vec<_> = graph
            .descendants(table.unwrap_or(alarm), "ECUC-CONTAINER-VALUE")
            .into_iter()
            .filter(|index| {
                definition_is(
                    graph,
                    *index,
                    if using_table {
                        "OsScheduleTableAutostart"
                    } else {
                        "OsAlarmAutostart"
                    },
                )
            })
            .collect();
        let actions: Vec<_> = graph
            .descendants(alarm, "ECUC-CONTAINER-VALUE")
            .into_iter()
            .filter(|index| {
                definition_is(
                    graph,
                    *index,
                    if using_table {
                        "OsScheduleTableEventSetting"
                    } else {
                        "OsAlarmSetEvent"
                    },
                )
            })
            .collect();
        if autostarts.len() != 1 || actions.len() != 1 {
            return Err(reject(
                graph,
                alarm,
                "PERIOD_ALARM_CONFLICT",
                crate::product_message!(
                    "backend.integration.schedule.periodic_alarm_autostart_and_set_event_required"
                ),
            ));
        }
        let autostart = autostarts[0];
        let alarm_modes: BTreeSet<_> = values(
            graph,
            autostart,
            if using_table {
                "OsScheduleTableAppModeRef"
            } else {
                "OsAlarmAppModeRef"
            },
            true,
        )
        .into_iter()
        .collect();
        if alarm_modes != modes {
            return Err(reject(
                graph,
                autostart,
                "PERIOD_ALARM_CONFLICT",
                crate::product_message!(
                    "backend.integration.schedule.periodic_alarm_owner_task_autostart_mode_mismatch"
                ),
            ));
        }
        let expiry_offset = using_table
            .then(|| {
                value(graph, alarm, "OsScheduleTblExpPointOffset", false)
                    .and_then(|text| text.parse::<u32>().ok())
            })
            .flatten();
        let table_start = using_table
            .then(|| {
                value(graph, autostart, "OsScheduleTableStartValue", false)
                    .and_then(|text| text.parse::<u32>().ok())
            })
            .flatten();
        if using_table {
            let table = table.unwrap();
            let sync: Vec<_> = graph
                .descendants(table, "ECUC-CONTAINER-VALUE")
                .into_iter()
                .filter(|index| definition_is(graph, *index, "OsScheduleTableSync"))
                .collect();
            let points: Vec<_> = graph
                .descendants(table, "ECUC-CONTAINER-VALUE")
                .into_iter()
                .filter(|index| definition_is(graph, *index, "OsScheduleTableExpiryPoint"))
                .collect();
            if sync.len() != 1
                || value(graph, sync[0], "OsScheduleTblSyncStrategy", false) != Some("NONE")
                || points != vec![alarm]
                || graph
                    .descendants(alarm, "ECUC-CONTAINER-VALUE")
                    .into_iter()
                    .filter(|index| definition_is(graph, *index, "OsScheduleTableTaskActivation"))
                    .count()
                    != 0
                || !matches!(
                    value(graph, table, "OsScheduleTableRepeating", false),
                    Some("true" | "1")
                )
                || value(graph, table, "OsScheduleTableDuration", false)
                    .and_then(|value| value.parse::<u32>().ok())
                    != Some(period)
                || value(graph, autostart, "OsScheduleTableAutostartType", false)
                    != Some("RELATIVE")
                || expiry_offset.is_none_or(|offset| offset >= period)
                || table_start.is_none_or(|start| {
                    start == 0
                        || start.checked_add(expiry_offset.unwrap_or(u32::MAX)) != Some(period)
                })
                || reference(graph, actions[0], "OsScheduleTableSetEventTaskRef")? != task
                || reference(graph, actions[0], "OsScheduleTableSetEventRef")? != os_event
            {
                return Err(reject(
                    graph,
                    mapping,
                    "PERIOD_SCHEDULE_TABLE_CONFLICT",
                    crate::product_message!(
                        "backend.integration.schedule.periodic_rte_group_expiry_point_configuration_required"
                    ),
                ));
            }
        } else if value(graph, autostart, "OsAlarmAutostartType", false) != Some("RELATIVE")
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
                crate::product_message!(
                    "backend.integration.schedule.event_period_alarm_and_set_event_mismatch"
                ),
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
                    crate::product_message!(
                        "backend.integration.schedule.scheduled_event_unique_entity_missing"
                    ),
                )
            })?;
        let symbol = if application {
            if !periodic_events.contains(&graph.elements[event].object)
                || !components.iter().any(|(_, owner)| {
                    graph
                        .objects
                        .get(*owner)
                        .is_some_and(|behavior| graph.within(entity, *behavior))
                })
            {
                return Err(reject(
                    graph,
                    event,
                    "SCHEDULE_NOT_UNIQUE",
                    crate::product_message!(
                        "backend.integration.schedule.application_timing_mapping_behavior_mismatch"
                    ),
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
                        crate::product_message!("backend.integration.schedule.schedulable_bsw_entity_implementation_entry_missing"),
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
            alarm: if using_table {
                String::new()
            } else {
                graph.elements[alarm].object.clone()
            },
            expiry_point: using_table.then(|| graph.elements[alarm].object.clone()),
            schedule_table: table.map(|index| graph.elements[index].object.clone()),
            expiry_offset,
            table_start,
            os_event: graph.elements[os_event].object.clone(),
            period_ms: period,
            position,
            application,
        });
    }
    if synchronous.len() != server_runnables.len()
        || entities.iter().filter(|entity| entity.application).count() != periodic_events.len()
        || (!multi && synchronous.is_empty())
    {
        return Err(reject(
            graph,
            behavior,
            "SCHEDULE_NOT_UNIQUE",
            crate::product_message!(
                "backend.integration.schedule.application_periodic_and_synchronous_mappings_required"
            ),
        ));
    }
    for event in graph.of_kind("BSW-TIMING-EVENT") {
        if !event_set.contains(&event) {
            return Err(reject(
                graph,
                event,
                "SCHEDULE_NOT_UNIQUE",
                crate::product_message!(
                    "backend.integration.schedule.bsw_timing_event_ecu_task_mapping_missing"
                ),
            ));
        }
    }
    // Each admitted alarm/table first fires at period_ms and repeats at that
    // period. A task event cannot distinguish different cadences or triggers.
    let mut event_periods = BTreeMap::new();
    for entity in &entities {
        if event_periods
            .insert((&entity.task, &entity.os_event), entity.period_ms)
            .is_some_and(|period| period != entity.period_ms)
        {
            return Err(reject(
                graph,
                graph.objects[&entity.mapping],
                "SCHEDULE_NOT_UNIQUE",
                crate::product_message!(
                    "backend.integration.schedule.shared_event_period_mismatch"
                ),
            ));
        }
    }
    entities.sort_by_key(|entity| entity.position);
    let mapped_tables: BTreeSet<_> = entities
        .iter()
        .filter_map(|entity| entity.schedule_table.as_deref())
        .collect();
    let declared_tables: BTreeSet<_> = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, "OsScheduleTable"))
        .map(|index| graph.elements[index].object.as_str())
        .collect();
    if mapped_tables != declared_tables {
        return Err(reject(
            graph,
            behavior,
            "SCHEDULE_TABLE_UNBOUND",
            crate::product_message!(
                "backend.integration.schedule.reference_ecu_tables_require_periodic_rte_groups"
            ),
        ));
    }
    let configured_symbol = |kind: &str, prefix: &str| {
        graph
            .of_kind("ECUC-CONTAINER-VALUE")
            .into_iter()
            .find(|index| definition_is(graph, *index, kind))
            .map(|index| format!("{prefix}_{}", graph.text(index, "SHORT-NAME").unwrap_or("")))
            .or_else(|| {
                entities
                    .iter()
                    .find(|entity| entity.symbol.starts_with(&format!("{prefix}_")))
                    .map(|entity| entity.symbol.clone())
            })
            .unwrap_or_default()
    };
    let mut expected_order: Vec<String> = if multi {
        vec![
            configured_symbol("ComMChannel", "ComM_MainFunction"),
            "Can_MainFunction_Wakeup".into(),
            "CanTp_MainFunction".into(),
            configured_symbol("ComMainFunctionRx", "Com_MainFunctionRx"),
        ]
    } else {
        vec![
            "Can_MainFunction_Wakeup".into(),
            "CanTp_AdvanceTime".into(),
            "Com_AdvanceTime".into(),
        ]
    };
    expected_order.extend(
        entities
            .iter()
            .filter(|entity| entity.application)
            .map(|entity| entity.symbol.clone()),
    );
    if multi
        && entities
            .iter()
            .filter(|entity| entity.application)
            .map(|entity| entity.period_ms)
            .collect::<BTreeSet<_>>()
            .len()
            != 1
    {
        return Err(reject(
            graph,
            behavior,
            "PERIOD_UNSUPPORTED",
            crate::product_message!(
                "backend.integration.schedule.declared_task_positions_order_mismatch"
            ),
        ));
    }
    if multi {
        expected_order.extend([
            configured_symbol("ComMainFunctionTx", "Com_MainFunctionTx"),
            "Dcm_MainFunction".into(),
            "Can_MainFunction_Read".into(),
            "Can_MainFunction_Write".into(),
            "Can_MainFunction_Mode".into(),
            "Can_MainFunction_BusOff".into(),
        ]);
    } else {
        expected_order.extend(["Com_TriggerTransmit".into(), "Dcm_AdvanceTime".into()]);
    }
    if entities
        .iter()
        .map(|entity| entity.symbol.as_str())
        .collect::<Vec<_>>()
        != expected_order
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    {
        return Err(reject(
            graph,
            behavior,
            "SCHEDULE_ORDER",
            crate::product_message!(
                "backend.integration.schedule.declared_task_positions_order_mismatch"
            ),
        ));
    }
    Ok(ScheduleContract {
        task: graph.elements[task].object.clone(),
        task_priority: priority.unwrap(),
        counter: graph.elements[counter].object.clone(),
        counter_maximum: maximum.unwrap(),
        counter_ticks_per_base: counter_ticks_per_base.unwrap(),
        counter_minimum_cycle: counter_minimum_cycle.unwrap(),
        counter_tick_ms: counter_tick_ms.unwrap(),
        entities,
        synchronous_event: if multi {
            String::new()
        } else {
            synchronous.first().cloned().unwrap_or_default()
        },
        synchronous_events: if multi { synchronous } else { Vec::new() },
    })
}
