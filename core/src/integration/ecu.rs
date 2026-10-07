use super::component::c_name;
use super::{DiagnosticCategory, PlanDiagnostic, ValidatedIntegrationPlan};
use crate::resources::AssetInventory;
use crate::target::BuildTarget;
use crate::{GenerationPreview, GenerationReport, generator};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

pub(crate) const APPLICATION_SOURCE_PATH: &str = "application/Application.c";

/// A complete deterministic source project from the same validated plan.
/// No unchecked constructor or caller-supplied source replacements exist.
pub struct EcuIntegrationFiles {
    files: Vec<(String, Vec<u8>)>,
}
impl EcuIntegrationFiles {
    pub fn files(&self) -> &[(String, Vec<u8>)] {
        &self.files
    }
    pub fn preview(&self, output: &Path) -> Result<GenerationPreview, String> {
        generator::output::preview_prepared(&self.files, output)
    }
    pub fn generate_previewed(
        &self,
        output: &Path,
        revision: &str,
    ) -> Result<GenerationReport, String> {
        generator::output::generate_prepared(self.files.clone(), output, Some(revision))
    }
}

fn reject(message: impl Into<String>) -> Vec<PlanDiagnostic> {
    vec![PlanDiagnostic {
        category: DiagnosticCategory::Tool,
        code: "ECU_SOURCE_CLOSURE".into(),
        file: None,
        object: None,
        message: message.into(),
        remedy: "Restore the matching compiled source/plan identities and resolve the located integration contract before generating.".into(),
    }]
}

impl ValidatedIntegrationPlan {
    /// The sole current native application producer. Paths are relative to the
    /// authoritative project root, not the generated immutable source package.
    pub fn application_slot_descriptor(
        &self,
    ) -> Result<generator::delivery::ApplicationSlotDescriptor, Vec<PlanDiagnostic>> {
        let component = &self.description().component;
        let contract = self.component_contract_files()?;
        let generated_headers = contract
            .files()
            .iter()
            .filter(|(path, _)| path.starts_with("include/") && path.ends_with(".h"))
            .map(|(path, _)| path.clone())
            .collect();
        Ok(generator::delivery::ApplicationSlotDescriptor {
            producer_slot: generator::delivery::APPLICATION_SLOT.into(),
            component_path: component.component.clone(),
            source_paths: vec![APPLICATION_SOURCE_PATH.into()],
            generated_headers,
            entry_symbols: vec![
                "Ecu_ApplicationInitialize".into(),
                component.periodic_symbol.clone(),
                "Ecu_ApplicationInspect".into(),
                component.service.runnable_symbol.clone(),
            ],
        })
    }

    /// Trusted create-only seed; never merges with or overwrites a user source.
    pub fn application_seed_files(&self) -> Result<Vec<(String, Vec<u8>)>, Vec<PlanDiagnostic>> {
        Ok(vec![(
            APPLICATION_SOURCE_PATH.into(),
            self.render_reference_application()?,
        )])
    }

    fn render_reference_application(&self) -> Result<Vec<u8>, Vec<PlanDiagnostic>> {
        let component = &self.description().component;
        let read = component
            .data_ports
            .iter()
            .find(|port| port.read)
            .ok_or_else(|| reject("The validated application read port is missing."))?;
        let write = component
            .data_ports
            .iter()
            .find(|port| !port.read)
            .ok_or_else(|| reject("The validated application write port is missing."))?;
        let substitutions = [
            (
                "APP_HEADER",
                format!(
                    "Rte_{}.h",
                    c_name(component.component.rsplit('/').next().unwrap())
                ),
            ),
            ("RX_INITIAL", read.initial_value.to_string()),
            ("TX_INITIAL", write.initial_value.to_string()),
            ("READ_API", read.api_symbol.clone()),
            ("WRITE_API", write.api_symbol.clone()),
            ("SERVER_API", component.service.runnable_symbol.clone()),
            ("PERIODIC_API", component.periodic_symbol.clone()),
            (
                "DATATYPE",
                c_name(component.service.array_type.rsplit('/').next().unwrap()),
            ),
        ];
        render_template("runtime/ecu/templates/Application.c.in", &substitutions)
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

    pub(crate) fn render_ecu_sources<'a>(
        &'a self,
        target: BuildTarget,
        application: Option<&'a [u8]>,
    ) -> Result<BTreeMap<String, Cow<'a, [u8]>>, Vec<PlanDiagnostic>> {
        let plan = self.description();
        let contract = self.component_contract_files()?;
        let mut files = BTreeMap::new();
        for asset in AssetInventory::embedded().selected(target, "ecu") {
            let delivered = crate::prepared::deliver_path(asset, "ecu").map_err(reject)?;
            if files
                .insert(delivered.clone(), Cow::Borrowed(asset.bytes))
                .is_some()
            {
                return Err(reject(format!(
                    "A trusted asset owner collides: {delivered}"
                )));
            }
        }
        let mut source_paths = BTreeMap::new();
        for (path, expected) in &plan.runtime_sources {
            let delivered = if path == "runtime/include/Os.h" {
                "bsw-origin/include/Os.h"
            } else {
                path.trim_start_matches("runtime/")
            };
            let bytes = files
                .get(delivered)
                .ok_or_else(|| reject(format!("The plan source is not delivered: {path}")))?;
            if format!("{:x}", Sha256::digest(bytes)) != *expected {
                return Err(reject(format!("The plan/source identity differs: {path}")));
            }
            source_paths.insert(path, delivered.to_owned());
        }
        let counter_name = c_name(plan.schedule.counter.rsplit('/').next().unwrap());
        let tick_ms = u64::from(plan.schedule.counter_tick_ms);
        let counter_header = format!(
            "/** @file Generated Counter time conversions (SWS_Os_00393).\n * Argument: a TickType value in0..UINT32_MAX; evaluated once.\n * Result: PhysicalTimeType. Integer seconds truncate toward zero.\n * The validated Counter resolution is {tick_ms} logical millisecond(s).\n * The largest nanosecond result fits uint64_t without overflow.\n */\n#ifndef AUTOSAR_GENERATED_OS_COUNTER_H\n#define AUTOSAR_GENERATED_OS_COUNTER_H\n#include \"Os_Types.h\"\nstatic inline PhysicalTimeType Os_TicksToNs_{counter_name}(TickType ticks) {{\n    return (PhysicalTimeType)ticks * UINT64_C({ns});\n}}\nstatic inline PhysicalTimeType Os_TicksToUs_{counter_name}(TickType ticks) {{\n    return (PhysicalTimeType)ticks * UINT64_C({us});\n}}\nstatic inline PhysicalTimeType Os_TicksToMs_{counter_name}(TickType ticks) {{\n    return (PhysicalTimeType)ticks * UINT64_C({tick_ms});\n}}\nstatic inline PhysicalTimeType Os_TicksToSec_{counter_name}(TickType ticks) {{\n    return ((PhysicalTimeType)ticks * UINT64_C({tick_ms})) / UINT64_C(1000);\n}}\n#define OS_TICKS2NS_{counter_name}(ticks) (Os_TicksToNs_{counter_name}((ticks)))\n#define OS_TICKS2US_{counter_name}(ticks) (Os_TicksToUs_{counter_name}((ticks)))\n#define OS_TICKS2MS_{counter_name}(ticks) (Os_TicksToMs_{counter_name}((ticks)))\n#define OS_TICKS2SEC_{counter_name}(ticks) (Os_TicksToSec_{counter_name}((ticks)))\n#endif\n",
            ns = tick_ms * 1_000_000,
            us = tick_ms * 1_000,
        );
        let counter_symbol = format!("OS_COUNTER_ID_{counter_name}");
        files.extend(
            super::os_service::files(&counter_name, &counter_symbol)
                .into_iter()
                .map(|(path, bytes)| (path, Cow::Owned(bytes))),
        );
        let legacy_constants = format!(
            "#define {counter_symbol} 0u\n#define OSMAXALLOWEDVALUE_{counter_name} UINT64_C({maximum})\n#define OSTICKSPERBASE_{counter_name} UINT64_C({base})\n#define OSMINCYCLE_{counter_name} UINT64_C({minimum})\n#define OSMAXALLOWEDVALUE_{counter_symbol} OSMAXALLOWEDVALUE_{counter_name}\n#define OSTICKSPERBASE_{counter_symbol} OSTICKSPERBASE_{counter_name}\n#define OSMINCYCLE_{counter_symbol} OSMINCYCLE_{counter_name}\n#define OSMAXALLOWEDVALUE OSMAXALLOWEDVALUE_{counter_name}\n#define OSTICKSPERBASE OSTICKSPERBASE_{counter_name}\n#define OSMINCYCLE OSMINCYCLE_{counter_name}\n#define OSTICKDURATION UINT64_C({nanoseconds})\n",
            maximum = plan.schedule.counter_maximum,
            base = plan.schedule.counter_ticks_per_base,
            minimum = plan.schedule.counter_minimum_cycle,
            nanoseconds = tick_ms * 1_000_000,
        );
        let mut counter_header = counter_header;
        let end = counter_header.rfind("#endif").unwrap();
        counter_header.insert_str(end, &legacy_constants);
        files.insert(
            "os/include/Os_Counter.h".into(),
            Cow::Owned(counter_header.into_bytes()),
        );
        let configuration = files.get_mut("os/include/Os_Cfg.h").unwrap();
        let mut configuration_text = std::str::from_utf8(configuration)
            .map_err(|error| reject(error.to_string()))?
            .to_owned();
        let extended_status = plan
            .configuration
            .iter()
            .find_map(|record| {
                record.parameters.iter().find_map(|(name, values)| {
                    name.ends_with("/OsStatus").then(|| values[0].as_str())
                })
            })
            .unwrap_or("EXTENDED")
            == "EXTENDED";
        configuration_text = configuration_text.replace(
            "#ifndef OS_STATUS_EXTENDED\n#define OS_STATUS_EXTENDED 1\n#endif",
            if extended_status {
                "#if defined(OS_STATUS_EXTENDED) && (OS_STATUS_EXTENDED != 1)\n#error OsStatus differs from the validated plan\n#endif\n#ifndef OS_STATUS_EXTENDED\n#define OS_STATUS_EXTENDED 1\n#endif"
            } else {
                "#if defined(OS_STATUS_EXTENDED) && (OS_STATUS_EXTENDED != 0)\n#error OsStatus differs from the validated plan\n#endif\n#ifndef OS_STATUS_EXTENDED\n#define OS_STATUS_EXTENDED 0\n#endif"
            },
        );
        let task_symbol = format!(
            "OS_TASK_ID_{}",
            plan.schedule.task.rsplit('/').next().unwrap()
        );
        let use_res_scheduler = plan
            .configuration
            .iter()
            .find(|record| record.definition == "/AUTOSAR/EcucDefs/Os/OsOS")
            .and_then(|record| {
                record
                    .parameters
                    .get("/AUTOSAR/EcucDefs/Os/OsOS/OsUseResScheduler")
            })
            .map(|values| matches!(values[0].as_str(), "true" | "1"))
            .ok_or_else(|| {
                reject("The validated OS scheduler resource configuration is missing.")
            })?;
        let end = configuration_text.rfind("#endif").unwrap();
        configuration_text.insert_str(
            end,
            &format!("#include \"Os_Counter.h\"\n#define {task_symbol} 0u\n"),
        );
        *configuration = Cow::Owned(configuration_text.into_bytes());
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
        .map_err(|error| reject(error.to_string()))?;
        timer_report.push(b'\n');
        files.insert("os-generation-timing.json".into(), Cow::Owned(timer_report));
        for (name, bytes) in contract.into_files() {
            if name.starts_with("include/") || name == "contract.json" {
                if let Some(previous) = files.get(&name) {
                    if previous.as_ref() != bytes {
                        return Err(reject(format!("A contract/runtime header differs: {name}")));
                    }
                } else {
                    files.insert(name, Cow::Owned(bytes));
                }
            }
        }
        let component = &plan.component;
        let app_header = format!(
            "Rte_{}.h",
            c_name(component.component.rsplit('/').next().unwrap())
        );
        let client_header = format!(
            "Rte_{}.h",
            c_name(
                component
                    .service
                    .client_instance
                    .rsplit('/')
                    .next()
                    .unwrap()
            )
        );
        if app_header == super::os_service::HEADER || client_header == super::os_service::HEADER {
            return Err(reject(
                "A generated component header collides with the OS service header",
            ));
        }
        let rx = plan.signals.iter().find(|signal| signal.receive).unwrap();
        let tx = plan.signals.iter().find(|signal| !signal.receive).unwrap();
        let signal_id = |path: &str| {
            plan.handles
                .iter()
                .find(|handle| handle.domain == "com_signal" && handle.path == path)
                .unwrap()
                .handle
        };
        let rx_id = signal_id(&rx.com_signal);
        let tx_id = signal_id(&tx.com_signal);
        let app = plan
            .schedule
            .entities
            .iter()
            .find(|entity| entity.application)
            .unwrap();
        let work = plan
            .schedule
            .entities
            .iter()
            .find(|entity| entity.period_ms == 1 && entity.os_event != app.os_event)
            .ok_or_else(|| reject("No distinct fixed owner work event exists."))?;
        let mask = |path: &str| {
            plan.events
                .iter()
                .find(|event| event.path == path)
                .unwrap()
                .mask
        };
        let io = plan
            .events
            .iter()
            .find(|event| event.path != app.os_event && event.path != work.os_event)
            .ok_or_else(|| reject("No distinct owner IO event exists."))?;
        let header = format!(
            "/** @file Generated checked target constants and OS configuration. */\n#ifndef ECU_TARGET_CONFIG_H\n#define ECU_TARGET_CONFIG_H\n#include \"Os_Target.h\"\n#include \"{app_header}\"\n#define ECU_TARGET_TASK 0u\n#define ECU_TARGET_EVENT_WORK {}u\n#define ECU_TARGET_EVENT_APP {}u\n#define ECU_TARGET_EVENT_IO {}u\n#define ECU_TARGET_RX_CAN_ID {}u\n#define ECU_TARGET_RX_DEADLINE_MS {}u\n#define ECU_TARGET_RUN_APPLICATION() {}()\n#define ECU_TARGET_TRANSMIT() Com_TriggerTransmit(1u)\nextern const Os_TargetConfig Ecu_OsConfig;\nvoid Ecu_ApplicationInitialize(void);\nStd_ReturnType Ecu_TargetReadDid(uint8_t *data);\n#ifdef ECU_TARGET_TESTS\nint Ecu_TargetTestFailStage(unsigned stage);\nvoid Ecu_TargetTestShutdown(StatusType reason);\n#endif\n#endif\n",
            mask(&work.os_event),
            mask(&app.os_event),
            io.mask,
            rx.can_id,
            rx.deadline_ms.unwrap(),
            component.periodic_symbol,
        );
        if plan
            .schedule
            .entities
            .iter()
            .filter(|entity| entity.period_ms == 1 && entity.os_event != app.os_event)
            .any(|entity| entity.trigger() != work.trigger() || entity.os_event != work.os_event)
        {
            return Err(reject(
                "The fixed owner work cycle does not have one alarm/event.",
            ));
        }
        files.insert(
            "include/Ecu_TargetConfig.h".into(),
            Cow::Owned(header.into_bytes()),
        );
        files.insert(
            "include/Ecu_ProfileLimits.h".into(),
            Cow::Owned(
                format!(
                    "/** @file Compile-time capacity of the validated integrated profile. */\n#ifndef ECU_PROFILE_LIMITS_H\n#define ECU_PROFILE_LIMITS_H\n#define ECU_MAX_PDU_PAYLOAD {}u\n#endif\n",
                    plan.diagnostic.buffer_bytes,
                )
                .into_bytes(),
            ),
        );
        let mut groups = BTreeMap::new();
        let mut table_groups = BTreeMap::new();
        for entity in &plan.schedule.entities {
            if let Some(table) = &entity.schedule_table {
                table_groups.insert(
                    table.clone(),
                    (
                        entity.period_ms,
                        mask(&entity.os_event),
                        entity.expiry_offset.unwrap(),
                        entity.table_start.unwrap(),
                    ),
                );
            } else {
                groups.insert(
                entity.alarm.clone(),
                (entity.period_ms, mask(&entity.os_event)),
                );
            }
        }
        let mut alarms = String::new();
        files.extend(
            super::arti::files(
                plan,
                use_res_scheduler,
                &groups.keys().cloned().collect::<Vec<_>>(),
                &table_groups.keys().cloned().collect::<Vec<_>>(),
            )
            .map_err(reject)?
            .into_iter()
            .map(|(path, bytes)| (path, Cow::Owned(bytes))),
        );
        for (id, (_, (period, event))) in groups.iter().enumerate() {
            writeln!(alarms, "    {{{id}u, 0u, OS_ALARM_EVENT, 0u, {event}u, NULL, 1u, 0u, {period}u, {period}u, 0u}},").unwrap();
        }
        let mut tables = String::new();
        let mut table_entries = String::new();
        for (id, (_, (period, event, offset, start))) in table_groups.iter().enumerate() {
            writeln!(tables, "static const Os_ExpiryAction table_actions_{id}[] = {{{{OS_ALARM_EVENT, 0u, {event}u}}}};\nstatic const Os_ExpiryPoint table_points_{id}[] = {{{{{offset}u, table_actions_{id}, 1u}}}};").unwrap();
            writeln!(table_entries, "    {{{id}u, 0u, {period}u, table_points_{id}, 1u, 1u, OS_SCHEDULE_SYNC_NONE, 1u, 0u, {start}u}},").unwrap();
        }
        if !table_groups.is_empty() {
            writeln!(
                tables,
                "static const Os_ScheduleTableConfig schedule_tables[] = {{\n{table_entries}}};"
            )
            .unwrap();
        }
        let alarm_declaration = if groups.is_empty() {
            String::new()
        } else {
            format!("static const Os_AlarmConfig alarms[] = {{\n{alarms}}};")
        };
        let read = component.data_ports.iter().find(|port| port.read).unwrap();
        let write = component.data_ports.iter().find(|port| !port.read).unwrap();
        let template = AssetInventory::embedded()
            .get("runtime/ecu/templates/Ecu_Config.c.in")
            .ok_or_else(|| reject("The trusted ECU configuration template is missing."))?;
        let mut config = std::str::from_utf8(template.bytes)
            .map_err(|error| reject(error.to_string()))?
            .to_owned();
        for (key, value) in [
            ("TASK_SYMBOL", task_symbol),
            (
                "TASK_NAME",
                serde_json::to_string(plan.schedule.task.rsplit('/').next().unwrap()).unwrap(),
            ),
            ("TASK_PRIORITY", plan.schedule.task_priority.to_string()),
            (
                "SCHEDULER_RESOURCE",
                if use_res_scheduler {
                    format!(
                        "static const Os_ResourceConfig scheduler_resource = {{RES_SCHEDULER, {}u, UINT16_C(1), UINT32_C(0)}};",
                        plan.schedule.task_priority
                    )
                } else {
                    String::new()
                },
            ),
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
            ("COUNTER_MAX", plan.schedule.counter_maximum.to_string()),
            ("COUNTER_SYMBOL", counter_symbol),
            (
                "COUNTER_BASE",
                plan.schedule.counter_ticks_per_base.to_string(),
            ),
            (
                "COUNTER_MIN_CYCLE",
                plan.schedule.counter_minimum_cycle.to_string(),
            ),
            ("ALARMS", alarm_declaration),
            (
                "ALARM_PTR",
                if groups.is_empty() { "NULL" } else { "alarms" }.to_owned(),
            ),
            ("ALARM_COUNT", groups.len().to_string()),
            ("SCHEDULE_TABLES", tables),
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
            ("RX_CAN_ID", rx.can_id.to_string()),
            ("TX_CAN_ID", tx.can_id.to_string()),
            ("RX_DEADLINE", rx.deadline_ms.unwrap().to_string()),
            ("APP_PERIOD", component.period_ms.to_string()),
            ("RX_ID", rx_id.to_string()),
            ("TX_ID", tx_id.to_string()),
            ("RX_INITIAL", read.initial_value.to_string()),
            ("TX_INITIAL", write.initial_value.to_string()),
            ("DIAG_RX_CAN_ID", plan.diagnostic.request_can_id.to_string()),
            (
                "DIAG_TX_CAN_ID",
                plan.diagnostic.response_can_id.to_string(),
            ),
            ("P2", plan.diagnostic.p2_ms.to_string()),
            ("P2_STAR", plan.diagnostic.p2_star_ms.to_string()),
            (
                "READ_DID_SESSIONS",
                plan.diagnostic
                    .sessions
                    .iter()
                    .fold(0u8, |mask, session| mask | (1u8 << session))
                    .to_string(),
            ),
            ("RX_CANIF_PDU", rx.can_if_handle.to_string()),
            ("TX_CANIF_PDU", tx.can_if_handle.to_string()),
            (
                "DIAG_RX_HANDLE",
                plan.diagnostic.request_can_if_handle.to_string(),
            ),
            ("S3", plan.diagnostic.s3_ms.to_string()),
            ("NAS", plan.diagnostic.n_as_ms.to_string()),
            ("NBS", plan.diagnostic.n_bs_ms.to_string()),
            ("NCR", plan.diagnostic.n_cr_ms.to_string()),
            ("DID", plan.diagnostic.did.to_string()),
            (
                "DIAG_TX_HANDLE",
                plan.diagnostic.response_can_if_handle.to_string(),
            ),
            (
                "ECU_NAME",
                serde_json::to_string(component.ecu.rsplit('/').next().unwrap()).unwrap(),
            ),
        ] {
            config = config.replace(&format!("@{key}@"), &value);
        }
        if config.contains('@') {
            return Err(reject(
                "The generated configuration has an unresolved placeholder.",
            ));
        }
        files.insert("src/Ecu_Config.c".into(), Cow::Owned(config.into_bytes()));
        let com = files.get_mut("include/Com.h").unwrap();
        let mut text = std::str::from_utf8(com)
            .map_err(|error| reject(error.to_string()))?
            .to_owned();
        let original_com = std::mem::replace(com, Cow::Owned(Vec::new()));
        let end = text.rfind("#endif").unwrap();
        text.insert_str(end, "typedef uint16_t Com_SignalIdType;\n#define COM_SERVICE_NOT_AVAILABLE 0x80u\nStd_ReturnType Com_SendSignal(Com_SignalIdType SignalId, const void *SignalDataPtr);\nStd_ReturnType Com_ReceiveSignal(Com_SignalIdType SignalId, void *SignalDataPtr);\n");
        *com = Cow::Owned(text.into_bytes());
        files.insert("bsw-origin/include/Com.h".into(), original_com);
        if let Some((_, delivered)) = source_paths
            .iter_mut()
            .find(|(path, _)| path.as_str() == "runtime/include/Com.h")
        {
            *delivered = "bsw-origin/include/Com.h".into();
        }
        let datatype = c_name(component.service.array_type.rsplit('/').next().unwrap());
        let call = format!(
            "Rte_Call_{}_{}",
            c_name(component.service.client_port.rsplit('/').next().unwrap()),
            c_name(&component.service.operation_name)
        );
        let substitutions = [
            ("APP_HEADER", app_header),
            ("CLIENT_HEADER", client_header),
            ("RX_ID", rx_id.to_string()),
            ("TX_ID", tx_id.to_string()),
            ("RX_INITIAL", read.initial_value.to_string()),
            ("TX_INITIAL", write.initial_value.to_string()),
            ("READ_API", read.api_symbol.clone()),
            ("WRITE_API", write.api_symbol.clone()),
            ("CALL_API", call),
            ("SERVER_API", component.service.runnable_symbol.clone()),
            ("CLIENT_API", component.service.client_symbol.clone()),
            ("PERIODIC_API", component.periodic_symbol.clone()),
            ("DATATYPE", datatype),
        ];
        files.insert(
            "src/Rte.c".into(),
            Cow::Owned(render_template(
                "runtime/ecu/templates/Rte.c.in",
                &substitutions,
            )?),
        );
        files.insert(
            "src/Application.c".into(),
            match application {
                Some(bytes) => Cow::Borrowed(bytes),
                None => Cow::Owned(self.render_reference_application()?),
            },
        );
        for source in self.sources() {
            files.insert(
                format!("inputs/{}", source.logical_path()),
                Cow::Borrowed(source.bytes()),
            );
        }
        let mut metadata = serde_json::to_vec_pretty(&serde_json::json!({
            "format": "autosar-ecu-integration-v1", "plan": plan,
            "originalBswSources": source_paths,
            "privateBswMappings": {
                "task": { "path": plan.schedule.task, "id": 0 },
                "counter": { "path": plan.schedule.counter, "id": 0 },
                "receiveFrame": { "comSignal": rx.com_signal, "index": 0, "canIfHandle": rx.can_if_handle },
                "transmitFrame": { "comSignal": tx.com_signal, "index": 1, "canIfHandle": tx.can_if_handle },
                "diagnosticTransmit": { "canIfHandle": plan.diagnostic.response_can_if_handle },
            },
            "target": target.spec().id,
            "owner": "one generated AUTOSTART extended Task_Ecu; StartupHook owns initialization",
            "time": "uint64 epoch, independent software Counter, uint32 FreeRTOS tick",
            "schm": "single BSW owner checks; native input/output and OS retain atomic protocols",
        })).map_err(|error| reject(error.to_string()))?;
        metadata.push(b'\n');
        files.insert("integration.json".into(), Cow::Owned(metadata));
        let readme = format!(
            "# ECU integration source project\n\nTarget: `{}`. This package contains the validated single-owner ECU, generated RTE/application, real BSW/OS, fixed FreeRTOS V11.3.1 original source and the target's selected controlled patches. Original MIT notices remain in `kernel/LICENSE.md`; Autsaro runtime, tool and template code is licensed under Apache-2.0 (see LICENSE and NOTICE). User-provided configurations and application code retain their own licensing. AUTOSAR XSD/MOD/PDF archives and compiler binaries are not redistributed.\n\n## Independent native build\n\nUse the pinned CPython 3.12.9 interpreter, GCC/compiler and binutils identities in `target.json`, and Git. Set `AUTOSAR_CC`, `AUTOSAR_OBJDUMP` and `AUTOSAR_GIT` to their absolute executable paths. No checkout, uv, Rust or Node is required. From this source directory run:\n\n```sh\n<CPython3.12.9> tools/ecu-tool.py build --project . --output <new-empty-external-directory> --mode host-batch\n<CPython3.12.9> tools/ecu-tool.py verify --project . --build-directory <another-new-empty-external-directory>\n```\n\nBuild modes: `host-batch` selects the actual production stdin/stdout entry; `probe` drives startup and 20 individual controlled ticks; `test` enables private test hooks. `--control-source <external-consumer.c>` is accepted only with `probe` or `test`, outside this sealed project. HostBatch cannot link TestMode or an external control source. The compiler runs against a private patched kernel copy; immutable source closure, actual PE/ELF sections, native TLS and original tool identity are checked. Source preparation and handoff reimport never run a compiler; explicit native preflight/build are separate operations.\n\n## Production protocol\n\nStage `BEGIN <epoch>`, at most256 `RX <CAN id> <DLC> <exact hex bytes>` records, then `COMMIT`. Epochs never decrease and a batch spans at most1000ms. Every intermediate tick completes individually; input precedes target-epoch processing. Equal epochs do not repeat periodic work. Real output write/flush precedes confirmation. `COMMIT_OK` requires actual Waiting with copied input, tick, output and confirmation drained; `COMMIT_ERROR` records executed work/BSW refusal; `REJECT` is admission failure. The5000ms host watchdog never advances automotive time; failed/blocked output or the257th pending output closes the ECU without claiming rollback.\n\nThe validated OsStatus is `{}`. Standard Status retains mandatory activation-limit/alarm warnings and host protection. `os-generation-timing.json` records Counter resolution, controlled tick and host deadlines; use the generated single-evaluation OS_TICKS2NS/US/MS/SEC macros. The independent verifier checks real CAN, one/two-DID reads, DID capacity refusal, N_Cr timeout/recovery and malformed admission; it does not establish full SC1, MCU, hard-real-time, ASIL or official certification.\n",
            target.spec().id,
            if extended_status {
                "EXTENDED"
            } else {
                "STANDARD"
            },
        );
        files.insert("README.md".into(), Cow::Owned(readme.into_bytes()));
        files.extend(
            super::artifacts::files(plan, &files)
                .map_err(reject)?
                .into_iter()
                .map(|(path, bytes)| (path, Cow::Owned(bytes))),
        );
        files.extend(
            super::handoff::verification_files(self)
                .map_err(reject)?
                .into_iter()
                .map(|(path, bytes)| (path, Cow::Owned(bytes))),
        );
        Ok(files)
    }
}

fn render_template(
    path: &str,
    substitutions: &[(&str, String)],
) -> Result<Vec<u8>, Vec<PlanDiagnostic>> {
    let asset = AssetInventory::embedded()
        .get(path)
        .ok_or_else(|| reject(format!("The trusted C template is missing: {path}")))?;
    let mut tail = std::str::from_utf8(asset.bytes).map_err(|error| reject(error.to_string()))?;
    let mut result = String::with_capacity(tail.len());
    while let Some((before, token)) = tail.split_once('@') {
        result.push_str(before);
        let (name, rest) = token
            .split_once('@')
            .ok_or_else(|| reject("A C template contains an unterminated placeholder."))?;
        let value = substitutions
            .iter()
            .find(|(key, _)| *key == name)
            .ok_or_else(|| {
                reject(format!(
                    "A C template contains an unknown placeholder: {name}"
                ))
            })?;
        result.push_str(&value.1);
        tail = rest;
    }
    result.push_str(tail);
    Ok(result.into_bytes())
}
