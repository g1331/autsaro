use super::c_layout::{self, Item};
use super::component::c_name;
use super::{DiagnosticCategory, PlanDiagnostic, ValidatedIntegrationPlan};
use crate::resources::AssetInventory;
use crate::target::BuildTarget;
use crate::{GenerationPreview, GenerationReport, generator};
use std::collections::BTreeMap;
use std::path::Path;

/// A complete deterministic source project from the same validated plan.
/// No unchecked constructor or caller-supplied source replacements exist.
pub struct EcuIntegrationFiles {
    files: Vec<(String, Vec<u8>)>,
}
impl EcuIntegrationFiles {
    pub fn files(&self) -> &[(String, Vec<u8>)] {
        &self.files
    }
    pub fn preview(
        &self,
        output: &Path,
    ) -> Result<GenerationPreview, crate::message::LocalizedText> {
        generator::output::preview_prepared(&self.files, output)
    }
    pub fn generate_previewed(
        &self,
        output: &Path,
        revision: &str,
    ) -> Result<GenerationReport, crate::message::LocalizedText> {
        generator::output::generate_prepared(self.files.clone(), output, Some(revision))
    }
}

pub(super) fn reject(message: crate::message::LocalizedText) -> Vec<PlanDiagnostic> {
    vec![PlanDiagnostic {
        category: DiagnosticCategory::Tool,
        code: "ECU_SOURCE_CLOSURE".into(),
        file: None,
        object: None,
        message,
        remedy: crate::product_message!(
            "backend.integration.ecu.generation_identity_contract_repair_required"
        ),
    }]
}

impl ValidatedIntegrationPlan {
    /// Every caller-owned producer is derived from the actual trusted symbol owner.
    /// The generated no-event Dcm service bridge is not a caller source slot.
    pub fn application_slot_descriptors(
        &self,
    ) -> Result<Vec<generator::delivery::ApplicationSlotDescriptor>, Vec<PlanDiagnostic>> {
        let composition = self.description().composition.as_ref().ok_or_else(|| {
            reject(crate::product_message!(
                "backend.integration.composition.consumer_unsupported"
            ))
        })?;
        Ok(application_slots(composition, &self.description().symbols))
    }

    /// Trusted create-only seed; never merges with or overwrites a user source.
    pub fn application_seed_files(&self) -> Result<Vec<(String, Vec<u8>)>, Vec<PlanDiagnostic>> {
        let composition = self.description().composition.as_ref().ok_or_else(|| {
            reject(crate::product_message!(
                "backend.integration.composition.consumer_unsupported"
            ))
        })?;
        // The reference shape gets the echo application the independent
        // verifier checks; any other shape gets neutral entry points.
        let reference = super::handoff::reference_shape(self);
        let mut files = Vec::new();
        for slot in self.application_slot_descriptors()? {
            let component = composition
                .components
                .iter()
                .find(|component| component.component == slot.component_path)
                .ok_or_else(|| PlanDiagnostic::plan_premise(&slot.component_path))?;
            let source_path = slot
                .source_paths
                .first()
                .ok_or_else(|| PlanDiagnostic::plan_premise(&slot.component_path))?;
            let scope = c_name(super::short_name(&component.component));
            let mut source = format!(
                "/** @file User-owned application entry points. */\n#include \"{}\"\n",
                component.header.trim_start_matches("include/")
            );
            if reference.is_some() {
                source.push_str(&format!("#define RTE_{scope}_START_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n/* Last value committed from the receive port; the DID reports it. */\nstatic uint32 committed RTE_{scope}_VAR_CLEARED_UNSPECIFIED;\n#define RTE_{scope}_STOP_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Rte_MemMap.h\"\n"));
            }
            source.push_str(&format!(
                "#define RTE_{scope}_START_SEC_CODE\n#include \"Rte_MemMap.h\"\n"
            ));
            for symbol in self
                .description()
                .symbols
                .iter()
                .filter(|symbol| symbol.definition_owner == *source_path)
            {
                let mut parameters: Vec<String> = symbol
                    .arguments
                    .iter()
                    .map(|argument| c_layout::parameter(&argument.native_type, &argument.name))
                    .collect();
                if parameters.is_empty() {
                    parameters.push("void".into());
                }
                let head = c_layout::signature(
                    &format!("RTE_{scope}_CODE {}", symbol.return_type),
                    &symbol.symbol,
                    &parameters,
                    "",
                    true,
                );
                if symbol.arguments.is_empty()
                    && reference
                        .as_ref()
                        .is_none_or(|shape| shape.periodic_symbol != symbol.symbol)
                {
                    source.push_str(&format!("{}\n", c_layout::short_function(&head, "")));
                    continue;
                }
                source.push_str(&format!("{head}\n{{\n"));
                if let Some(shape) = reference
                    .as_ref()
                    .filter(|shape| shape.periodic_symbol == symbol.symbol)
                {
                    // RTE_E_MAX_AGE_EXCEEDED still delivers the last received
                    // value (SWS_Rte_02703); RTE_E_NEVER_RECEIVED does not.
                    source.push_str(&format!("    uint32 value = committed;\n    const Std_ReturnType status = {}(&value);\n    if ((status == RTE_E_OK) || (status == RTE_E_MAX_AGE_EXCEEDED)) {{\n        committed = value;\n        (void){}(value);\n    }}\n}}\n",
                        shape.read.api_symbol, shape.write.api_symbol));
                    continue;
                }
                if let (Some(_), [argument]) = (&reference, symbol.arguments.as_slice())
                    && argument.direction == "OUT"
                    && argument.native_type != "uint32 *"
                {
                    // The application DID reports the committed value big-endian.
                    for (index, shift) in ["24u", "16u", "8u", ""].iter().enumerate() {
                        let value = if shift.is_empty() {
                            "committed".to_owned()
                        } else {
                            format!("(committed >> {shift})")
                        };
                        source.push_str(&format!(
                            "    {}[{index}] = (uint8){value};\n",
                            argument.name
                        ));
                    }
                    if symbol.return_type != "void" {
                        source.push_str("    return E_OK;\n");
                    }
                    source.push_str("}\n");
                    continue;
                }
                for argument in &symbol.arguments {
                    if argument.direction != "OUT" {
                        source.push_str(&format!("    (void){};\n", argument.name));
                    } else if argument.native_type == "uint32 *" {
                        source.push_str(&format!("    *{} = 0u;\n", argument.name));
                    } else {
                        // The checked contract supports only fixed uint8[4] OUT arrays.
                        for index in 0..4 {
                            source.push_str(&format!("    {}[{index}] = 0u;\n", argument.name));
                        }
                    }
                }
                if symbol.return_type != "void" {
                    source.push_str("    return E_OK;\n");
                }
                source.push_str("}\n");
            }
            source.push_str(&format!(
                "#define RTE_{scope}_STOP_SEC_CODE\n#include \"Rte_MemMap.h\"\n"
            ));
            files.push((source_path.clone(), source.into_bytes()));
        }
        Ok(files)
    }

    pub fn ecu_handoff_files(
        &self,
        target: BuildTarget,
    ) -> Result<EcuIntegrationFiles, Vec<PlanDiagnostic>> {
        let project = crate::prepare_ecu_project(self, target, true)?;
        Ok(EcuIntegrationFiles {
            files: project.into_files(),
        })
    }

    pub fn ecu_integration_files(
        &self,
        target: BuildTarget,
    ) -> Result<EcuIntegrationFiles, Vec<PlanDiagnostic>> {
        let project = crate::prepare_ecu_project(self, target, false)?;
        Ok(EcuIntegrationFiles {
            files: project.into_files(),
        })
    }

    pub(crate) fn render_ecu_sources(
        &self,
        target: BuildTarget,
    ) -> Result<(super::outputs::EcuFiles, String), Vec<PlanDiagnostic>> {
        super::ecu_config::files(self, target)
    }
}

/// The same delivery selection drives rendering, provenance and ledger checks:
/// the shared ECU driver/OS/kernel plus the mainline BSW, each delivered
/// verbatim. Template assets reach the package only through their emitter.
pub(crate) fn source_assets(
    target: BuildTarget,
) -> Result<BTreeMap<String, &'static crate::resources::AssetEntry>, crate::LocalizedText> {
    let inventory = AssetInventory::embedded();
    let mut files = BTreeMap::new();
    for asset in inventory
        .selected(target, "ecu")
        .filter(|asset| super::outputs::template_of(asset).is_none())
    {
        files.insert(crate::prepared::deliver_path(asset)?, asset);
    }
    Ok(files)
}

pub(super) fn application_slots(
    composition: &super::composition::CompositionContract,
    symbols: &[super::catalog::SymbolContract],
) -> Vec<generator::delivery::ApplicationSlotDescriptor> {
    let mut slots = Vec::new();
    for component in &composition.components {
        let source = format!(
            "application/{}.c",
            c_name(super::short_name(&component.component))
        );
        let entries: Vec<_> = symbols
            .iter()
            .filter(|symbol| symbol.definition_owner == source)
            .map(|symbol| symbol.symbol.clone())
            .collect();
        if !entries.is_empty() {
            slots.push(generator::delivery::ApplicationSlotDescriptor {
                producer_slot: format!("{}:{}", super::PROFILE, component.instance),
                component_path: component.component.clone(),
                source_paths: vec![source],
                generated_headers: vec![
                    component.header.clone(),
                    "include/Rte.h".into(),
                    "include/Rte_Type.h".into(),
                ],
                entry_symbols: entries,
            });
        }
    }
    slots
}

/// Structural ownership proof only. This neither authorizes extension catalogs
/// nor substitutes for native definition validation and regenerated-byte checks.
pub(crate) fn application_slots_from_sources(
    sources: &[super::InputSource],
) -> Result<Vec<generator::delivery::ApplicationSlotDescriptor>, Vec<PlanDiagnostic>> {
    let catalog = crate::definitions::DefinitionCatalog::builtin().map_err(reject)?;
    let graph = super::graph::Graph::from_catalog_target_scope(sources, &catalog)?;
    // Only the structural application/instance consumers participate here.
    // Unrelated ECUC modules and their extension definitions are not authorized
    // by an ownership proof; native preparation checks them separately.
    let rte_instances: Vec<_> = graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| super::schedule::definition_is(&graph, *index, "RteSwComponentInstance"))
        .collect();
    let mut pending = graph.of_kind("SYSTEM");
    let mut visited = std::collections::BTreeSet::new();
    let mut consumers = std::collections::BTreeSet::new();
    // The only ECUC reference used for application identity is the actual
    // RteSwComponentInstance -> composition prototype binding. Its definition
    // identity selects this field; no extension definition is accepted here.
    for root in &rte_instances {
        for value in graph.descendants(*root, "ECUC-REFERENCE-VALUE") {
            if graph.text(value, "DEFINITION-REF").is_some_and(|definition| {
                definition == "/AUTOSAR/EcucDefs/Rte/RteSwComponentInstance/RteSoftwareComponentInstanceRef"
            }) {
                consumers.extend(graph.descendants(value, "VALUE-REF"));
            }
        }
    }
    while let Some(index) = pending.pop() {
        if !visited.insert(index) {
            continue;
        }
        let element = &graph.elements[index];
        if element.tag.starts_with("ECUC-") {
            continue;
        }
        consumers.insert(index);
        pending.extend(element.children.iter().copied());
        if element.attributes.contains_key("DEST") {
            pending.extend(graph.objects.get(&element.text).copied());
        }
    }
    let references = graph.reference_diagnostics_for_indices(&consumers);
    if !references.is_empty() {
        return Err(references);
    }
    let composition = super::composition::inspect_ownership(&graph)?;
    Ok(application_slots(
        &composition,
        &super::composition::symbols(&composition),
    ))
}

/// Shared OS producer for both validated application profiles. Its inputs are
/// the checked counter/task/event contract, never caller-provided C fragments.
pub(super) struct OsGeneration {
    pub counter_symbol: String,
    pub task_symbol: String,
    pub use_res_scheduler: bool,
}

pub(super) fn configure_os(
    plan: &super::PlanDescription,
    outputs: &mut super::outputs::EcuOutputs,
) -> Result<OsGeneration, Vec<PlanDiagnostic>> {
    let counter_name = c_name(super::short_name(&plan.schedule.counter));
    let tick_ms = u64::from(plan.schedule.counter_tick_ms);
    let counter_symbol = format!("OS_COUNTER_ID_{counter_name}");
    let mut conversions = String::new();
    let mut macros = Vec::new();
    for (unit, function, result) in [
        (
            "NS",
            "Ns",
            format!(
                "(PhysicalTimeType)ticks * UINT64_C({})",
                tick_ms * 1_000_000
            ),
        ),
        (
            "US",
            "Us",
            format!("(PhysicalTimeType)ticks * UINT64_C({})", tick_ms * 1_000),
        ),
        (
            "MS",
            "Ms",
            format!("(PhysicalTimeType)ticks * UINT64_C({tick_ms})"),
        ),
        (
            "SEC",
            "Sec",
            format!("((PhysicalTimeType)ticks * UINT64_C({tick_ms})) / UINT64_C(1000)"),
        ),
    ] {
        let function = format!("Os_TicksTo{function}_{counter_name}");
        let head = c_layout::signature(
            "static inline PhysicalTimeType",
            &function,
            &["TickType ticks".to_owned()],
            "",
            false,
        );
        conversions.push_str(&format!("{head}\n{{\n    return {result};\n}}\n"));
        macros.push((
            format!("OS_TICKS2{unit}_{counter_name}(ticks)"),
            format!("({function}((ticks)))"),
        ));
    }
    macros.extend([
        (counter_symbol.clone(), "0u".to_owned()),
        (
            format!("OSMAXALLOWEDVALUE_{counter_name}"),
            format!("UINT64_C({})", plan.schedule.counter_maximum),
        ),
        (
            format!("OSTICKSPERBASE_{counter_name}"),
            format!("UINT64_C({})", plan.schedule.counter_ticks_per_base),
        ),
        (
            format!("OSMINCYCLE_{counter_name}"),
            format!("UINT64_C({})", plan.schedule.counter_minimum_cycle),
        ),
    ]);
    for constant in ["OSMAXALLOWEDVALUE", "OSTICKSPERBASE", "OSMINCYCLE"] {
        macros.push((
            format!("{constant}_{counter_symbol}"),
            format!("{constant}_{counter_name}"),
        ));
    }
    for constant in ["OSMAXALLOWEDVALUE", "OSTICKSPERBASE", "OSMINCYCLE"] {
        macros.push((constant.to_owned(), format!("{constant}_{counter_name}")));
    }
    macros.push((
        "OSTICKDURATION".to_owned(),
        format!("UINT64_C({})", tick_ms * 1_000_000),
    ));
    let macros = c_layout::defines(&macros);
    let counter_header = format!(
        "/** @file Generated Counter time conversions (SWS_Os_00393).\n * Argument: a TickType value in0..UINT32_MAX; evaluated once.\n * Result: PhysicalTimeType. Integer seconds truncate toward zero.\n * The validated Counter resolution is {tick_ms} logical millisecond(s).\n * The largest nanosecond result fits uint64_t without overflow.\n */\n#ifndef AUTOSAR_GENERATED_OS_COUNTER_H\n#define AUTOSAR_GENERATED_OS_COUNTER_H\n#include \"Os_Types.h\"\n{conversions}{macros}#endif\n",
    );
    outputs.emit_all(
        "os-service",
        super::os_service::files(&counter_name, &counter_symbol),
    )?;
    outputs.emit(super::outputs::OS, "include/Os_Counter.h", counter_header)?;
    let os_value = |parameter: &str| {
        plan.configuration.iter().find_map(|record| {
            record.parameters.iter().find_map(|(name, values)| {
                name.ends_with(parameter)
                    .then(|| values.first().map(String::as_str))
                    .flatten()
            })
        })
    };
    let extended_status = os_value("/OsStatus").unwrap_or("EXTENDED") == "EXTENDED";
    // SWS_Os_00439: the OIL-like switches select the OSError macros of the ErrorHook.
    let error_access = |macro_name: &str, parameter: &str| {
        let value = u8::from(matches!(os_value(parameter), Some("true" | "1")));
        (
            format!("#ifndef {macro_name}\n#define {macro_name} 1\n#endif"),
            format!(
                "#if defined({macro_name}) && ({macro_name} != {value})\n#error {} differs from the validated plan\n#endif\n#ifndef {macro_name}\n#define {macro_name} {value}\n#endif",
                &parameter[1..]
            ),
        )
    };
    let service_id = error_access("OS_USE_GET_SERVICE_ID", "/OsUseGetServiceId");
    let parameter_access = error_access("OS_USE_PARAMETER_ACCESS", "/OsUseParameterAccess");
    let status = if extended_status {
        "#if defined(OS_STATUS_EXTENDED) && (OS_STATUS_EXTENDED != 1)\n#error OsStatus differs from the validated plan\n#endif\n#ifndef OS_STATUS_EXTENDED\n#define OS_STATUS_EXTENDED 1\n#endif"
    } else {
        "#if defined(OS_STATUS_EXTENDED) && (OS_STATUS_EXTENDED != 0)\n#error OsStatus differs from the validated plan\n#endif\n#ifndef OS_STATUS_EXTENDED\n#define OS_STATUS_EXTENDED 0\n#endif"
    };
    let task_symbol = format!("OS_TASK_ID_{}", super::short_name(&plan.schedule.task));
    let use_res_scheduler = plan
        .configuration
        .iter()
        .find(|record| record.definition == "/AUTOSAR/EcucDefs/Os/OsOS")
        .and_then(|record| {
            record
                .parameters
                .get("/AUTOSAR/EcucDefs/Os/OsOS/OsUseResScheduler")
                .and_then(|values| values.first())
        })
        .map(|value| matches!(value.as_str(), "true" | "1"))
        .ok_or_else(|| {
            reject(crate::product_message!(
                "backend.integration.ecu.validated_os_scheduler_resource_config_missing"
            ))
        })?;
    let path = "include/Os_Cfg.h";
    let configuration = super::outputs::render(
        outputs.template(super::outputs::OS, path)?,
        path,
        &[
            super::outputs::Edit::Once(
                "#ifndef OS_STATUS_EXTENDED\n#define OS_STATUS_EXTENDED 1\n#endif",
                status,
            ),
            super::outputs::Edit::Once(&service_id.0, &service_id.1),
            super::outputs::Edit::Once(&parameter_access.0, &parameter_access.1),
            super::outputs::Edit::BeforeGuardEnd(&format!(
                "#include \"Os_Counter.h\"\n#define {task_symbol} 0u\n"
            )),
        ],
    )?;
    outputs.emit_rendered(super::outputs::OS, path, configuration)?;
    let mut timer_report = serde_json::to_vec_pretty(&serde_json::json!({
        "format": "autosar-os-generation-timing-v1",
        "requirement": "SWS_Os_00370",
        "counter": { "path": plan.schedule.counter, "name": counter_name,
            "tickNanoseconds": tick_ms * 1_000_000, "maximum": plan.schedule.counter_maximum },
        "internalPeriodicTimers": [],
        "kernelSoftwareTimers": { "enabled": false, "configUSE_TIMERS": 0 },
        "kernelTick": { "source": "explicit controlled interrupt1", "periodicHostThread": false,
            "logicalMillisecondsPerRequest": 1 },
        "hostTimeouts": [
            { "owner": "Os_TargetWaitTick", "clock": "native monotonic milliseconds",
                "rangeMilliseconds": [1, 5000], "advancesAutomotiveTime": false },
            { "owner": "HostBatchV1", "clock": "native monotonic milliseconds",
                "limitMilliseconds": 5000, "advancesAutomotiveTime": false }
        ],
        "scope": "Selected single-core controlled-logical-time target; no hardware timer claim."
    }))
    .map_err(|error| reject(error.to_string().into()))?;
    timer_report.push(b'\n');
    outputs.emit(
        super::outputs::OS,
        "meta/os-generation-timing.json",
        timer_report,
    )?;
    Ok(OsGeneration {
        counter_symbol,
        task_symbol,
        use_res_scheduler,
    })
}

pub(super) const ECU_CONFIG: &str = "src/Ecu_Config.c";

/// Emit the real OS task/counter/alarm/table configuration once for either
/// application profile. The remaining BSW portion is supplied by its producer.
pub(super) fn os_configuration(
    plan: &super::PlanDescription,
    outputs: &mut super::outputs::EcuOutputs,
    counter_symbol: &str,
    task_symbol: &str,
    use_res_scheduler: bool,
) -> Result<String, Vec<PlanDiagnostic>> {
    let mut groups = BTreeMap::new();
    let mut table_groups = BTreeMap::new();
    for entity in &plan.schedule.entities {
        if let Some(table) = &entity.schedule_table {
            table_groups.insert(
                table.clone(),
                (
                    entity.period_ms,
                    plan.event_mask(&entity.os_event)?,
                    entity
                        .expiry_offset
                        .ok_or_else(|| PlanDiagnostic::plan_premise(table))?,
                    entity
                        .table_start
                        .ok_or_else(|| PlanDiagnostic::plan_premise(table))?,
                ),
            );
        } else {
            groups.insert(
                entity.alarm.clone(),
                (entity.period_ms, plan.event_mask(&entity.os_event)?),
            );
        }
    }
    let mut alarms = Vec::new();
    let binding = "src/Os_Arti.c";
    let (description, binding_text) = super::arti::files(
        plan,
        use_res_scheduler,
        &groups.keys().cloned().collect::<Vec<_>>(),
        &table_groups.keys().cloned().collect::<Vec<_>>(),
        outputs.template(super::outputs::ARTI, binding)?,
    )?;
    outputs.emit(
        super::outputs::ARTI,
        "generated/descriptions/Os_Arti.arxml",
        description,
    )?;
    outputs.emit_rendered(super::outputs::ARTI, binding, binding_text)?;
    let row = |fields: String| Item::Row(fields.split(", ").map(str::to_owned).collect());
    for (id, (_, (period, event))) in groups.iter().enumerate() {
        alarms.push(row(format!(
            "{id}u, 0u, OS_ALARM_EVENT, 0u, {event}u, NULL, 1u, 0u, {period}u, {period}u, 0u"
        )));
    }
    // Alarms, schedule tables and RES_SCHEDULER share one template line.
    let mut objects = Vec::new();
    if !groups.is_empty() {
        objects.push(c_layout::initializer(
            "static const Os_AlarmConfig alarms[]",
            "",
            &alarms,
        ));
    }
    let mut table_entries = Vec::new();
    for (id, (_, (period, event, offset, start))) in table_groups.iter().enumerate() {
        objects.push(c_layout::initializer(
            &format!("static const Os_ExpiryAction table_actions_{id}[]"),
            "",
            &[row(format!("OS_ALARM_EVENT, 0u, {event}u"))],
        ));
        objects.push(c_layout::initializer(
            &format!("static const Os_ExpiryPoint table_points_{id}[]"),
            "",
            &[row(format!("{offset}u, table_actions_{id}, 1u"))],
        ));
        table_entries.push(row(format!("{id}u, 0u, {period}u, table_points_{id}, 1u, 1u, OS_SCHEDULE_SYNC_NONE, 1u, 0u, {start}u")));
    }
    if !table_groups.is_empty() {
        objects.push(c_layout::initializer(
            "static const Os_ScheduleTableConfig schedule_tables[]",
            "",
            &table_entries,
        ));
    }
    if use_res_scheduler {
        objects.push(c_layout::initializer(
            "static const Os_ResourceConfig scheduler_resource",
            "",
            &[
                Item::Value("RES_SCHEDULER".into()),
                Item::Value(format!("{}u", plan.schedule.task_priority)),
                Item::Value("UINT16_C(1)".into()),
                Item::Value("UINT32_C(0)".into()),
            ],
        ));
    }
    let template = outputs.template(super::outputs::OS, ECU_CONFIG)?;
    let values = [
        ("TASK_SYMBOL", task_symbol.to_owned()),
        (
            "TASK",
            c_layout::short_function(&format!("TASK({task_symbol})"), "Ecu_TargetTask();"),
        ),
        (
            "TASK_NAME",
            serde_json::Value::from(super::short_name(&plan.schedule.task)).to_string(),
        ),
        ("TASK_PRIORITY", plan.schedule.task_priority.to_string()),
        (
            "RESOURCE_PTR",
            if use_res_scheduler {
                "&scheduler_resource"
            } else {
                "NULL"
            }
            .to_owned(),
        ),
        ("RESOURCE_COUNT", usize::from(use_res_scheduler).to_string()),
        ("USE_RES_SCHEDULER", u8::from(use_res_scheduler).to_string()),
        ("COUNTER_MAX", plan.schedule.counter_maximum.to_string()),
        ("COUNTER_SYMBOL", counter_symbol.to_owned()),
        (
            "COUNTER_BASE",
            plan.schedule.counter_ticks_per_base.to_string(),
        ),
        (
            "COUNTER_MIN_CYCLE",
            plan.schedule.counter_minimum_cycle.to_string(),
        ),
        ("OS_OBJECTS", objects.join("\n")),
        (
            "ALARM_PTR",
            if groups.is_empty() { "NULL" } else { "alarms" }.to_owned(),
        ),
        ("ALARM_COUNT", groups.len().to_string()),
        (
            "SCHEDULE_PTR",
            if table_groups.is_empty() {
                "NULL"
            } else {
                "schedule_tables"
            }
            .to_owned(),
        ),
        ("SCHEDULE_COUNT", table_groups.len().to_string()),
    ]
    .map(|(key, value)| (format!("@{key}@"), value));
    let edits: Vec<_> = values
        .iter()
        .map(|(key, value)| super::outputs::Edit::Every(key, value))
        .collect();
    super::outputs::render(template, "src/Ecu_Config.c", &edits)
}

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "tests derive variant inputs from fixture ARXML"
)]
mod ownership_tests {
    use super::application_slots_from_sources;
    use crate::integration::InputSource;

    fn sources() -> Vec<InputSource> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/component-pipeline/arxml");
        let mut sources = std::fs::read_dir(root)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                InputSource::new(
                    entry.file_name().to_str().unwrap(),
                    std::fs::read(entry.path()).unwrap(),
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        sources.sort_by(|left, right| left.logical_path().cmp(right.logical_path()));
        sources
    }

    #[test]
    fn sealed_structural_ownership_uses_real_application_producers_and_typed_bindings() {
        let source = sources();
        let slots =
            application_slots_from_sources(&source).unwrap_or_else(|issues| panic!("{issues:?}"));
        assert_eq!(slots.len(), 3);
        assert_eq!(
            slots
                .iter()
                .map(|slot| slot.source_paths[0].as_str())
                .collect::<std::collections::BTreeSet<_>>(),
            std::collections::BTreeSet::from([
                "application/Ingress.c",
                "application/Observe.c",
                "application/Process.c"
            ])
        );
        assert!(
            !slots
                .iter()
                .any(|slot| slot.component_path.ends_with("DcmService"))
        );
        for (file, before, after) in [
            (
                "composition.arxml",
                "TYPE-TREF DEST=\"APPLICATION-SW-COMPONENT-TYPE\"",
                "TYPE-TREF DEST=\"SERVICE-SW-COMPONENT-TYPE\"",
            ),
            (
                "ecuc.arxml",
                "<VALUE-REF DEST=\"SW-COMPONENT-PROTOTYPE\">/Application/Pipeline/IngressInstance",
                "<VALUE-REF DEST=\"APPLICATION-SW-COMPONENT-TYPE\">/Application/Pipeline/IngressInstance",
            ),
        ] {
            let mut changed = source.clone();
            let input = changed
                .iter_mut()
                .find(|input| input.logical_path() == file)
                .unwrap();
            let text = std::str::from_utf8(input.bytes()).unwrap();
            assert!(text.contains(before));
            *input = InputSource::new(file, text.replacen(before, after, 1).into_bytes()).unwrap();
            assert!(
                application_slots_from_sources(&changed)
                    .unwrap_err()
                    .iter()
                    .any(|issue| issue.code == "REFERENCE_DEST")
            );
        }
        // Ownership does not authorize a catalog: unrelated ECUC definition
        // identities remain native validation's responsibility, outside this proof.
        let mut unrelated = source;
        let input = unrelated
            .iter_mut()
            .find(|input| input.logical_path() == "ecuc.arxml")
            .unwrap();
        let text = std::str::from_utf8(input.bytes()).unwrap();
        assert!(text.contains("/AUTOSAR/EcucDefs/Can/CanGeneral/CanDevErrorDetect"));
        *input = InputSource::new(
            "ecuc.arxml",
            text.replacen(
                "/AUTOSAR/EcucDefs/Can/CanGeneral/CanDevErrorDetect",
                "/Vendor/Extension/CanDevErrorDetect",
                1,
            )
            .into_bytes(),
        )
        .unwrap();
        assert_eq!(application_slots_from_sources(&unrelated).unwrap(), slots);
    }
}
