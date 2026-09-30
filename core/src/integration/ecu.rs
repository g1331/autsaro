use super::component::c_name;
use super::{DiagnosticCategory, PlanDiagnostic, ValidatedIntegrationPlan, bsw_sources, offline};
use crate::{GenerationPreview, GenerationReport, generator};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write;
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
    pub fn preview(&self, output: &Path) -> Result<GenerationPreview, String> {
        generator::preview_prepared(&self.files, output)
    }
    pub fn generate_previewed(
        &self,
        output: &Path,
        revision: &str,
    ) -> Result<GenerationReport, String> {
        generator::generate_prepared(self.files.clone(), output, Some(revision))
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
    pub fn ecu_handoff_files(&self) -> Result<EcuIntegrationFiles, Vec<PlanDiagnostic>> {
        let project = self.ecu_integration_files()?;
        Ok(EcuIntegrationFiles {
            files: super::handoff::files(self, project.files).map_err(reject)?,
        })
    }
    pub fn ecu_integration_files(&self) -> Result<EcuIntegrationFiles, Vec<PlanDiagnostic>> {
        let project = self.ecu_source_files()?;
        super::link_check::verify(project.files()).map_err(reject)?;
        Ok(project)
    }
    pub(super) fn ecu_source_files(&self) -> Result<EcuIntegrationFiles, Vec<PlanDiagnostic>> {
        let plan = self.description();
        let contract = self.component_contract_files()?;
        let mut files: BTreeMap<String, Vec<u8>> = bsw_sources::sources().into_iter().collect();
        files.insert(
            "bsw-origin/include/Os.h".into(),
            include_bytes!("../../../runtime/include/Os.h").to_vec(),
        );
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
        files.extend(offline::sources().map_err(reject)?);
        let counter_name = c_name(plan.schedule.counter.rsplit('/').next().unwrap());
        let tick_ms = u64::from(plan.schedule.counter_tick_ms);
        let counter_header = format!(
            "/** @file Generated Counter time conversions (SWS_Os_00393).\n * Argument: a TickType value in0..UINT32_MAX; evaluated once.\n * Result: PhysicalTimeType. Integer seconds truncate toward zero.\n * The validated Counter resolution is {tick_ms} logical millisecond(s).\n * The largest nanosecond result fits uint64_t without overflow.\n */\n#ifndef AUTOSAR_GENERATED_OS_COUNTER_H\n#define AUTOSAR_GENERATED_OS_COUNTER_H\n#include \"Os_Types.h\"\nstatic inline PhysicalTimeType Os_TicksToNs_{counter_name}(TickType ticks) {{\n    return (PhysicalTimeType)ticks * UINT64_C({ns});\n}}\nstatic inline PhysicalTimeType Os_TicksToUs_{counter_name}(TickType ticks) {{\n    return (PhysicalTimeType)ticks * UINT64_C({us});\n}}\nstatic inline PhysicalTimeType Os_TicksToMs_{counter_name}(TickType ticks) {{\n    return (PhysicalTimeType)ticks * UINT64_C({tick_ms});\n}}\nstatic inline PhysicalTimeType Os_TicksToSec_{counter_name}(TickType ticks) {{\n    return ((PhysicalTimeType)ticks * UINT64_C({tick_ms})) / UINT64_C(1000);\n}}\n#define OS_TICKS2NS_{counter_name}(ticks) (Os_TicksToNs_{counter_name}((ticks)))\n#define OS_TICKS2US_{counter_name}(ticks) (Os_TicksToUs_{counter_name}((ticks)))\n#define OS_TICKS2MS_{counter_name}(ticks) (Os_TicksToMs_{counter_name}((ticks)))\n#define OS_TICKS2SEC_{counter_name}(ticks) (Os_TicksToSec_{counter_name}((ticks)))\n#endif\n",
            ns = tick_ms * 1_000_000,
            us = tick_ms * 1_000,
        );
        let counter_symbol = format!("OS_COUNTER_ID_{counter_name}");
        files.extend(super::os_service::files(&counter_name, &counter_symbol));
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
            counter_header.into_bytes(),
        );
        let configuration = files.get_mut("os/include/Os_Cfg.h").unwrap();
        let mut configuration_text =
            String::from_utf8(configuration.clone()).map_err(|error| reject(error.to_string()))?;
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
        *configuration = configuration_text.into_bytes();
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
                { "owner": "Os_TargetWaitTick", "clock": "Windows monotonic milliseconds",
                    "rangeMilliseconds": [1, 5000], "advancesAutomotiveTime": false },
                { "owner": "HostBatchV1", "clock": "Windows monotonic milliseconds",
                    "limitMilliseconds": 5000, "advancesAutomotiveTime": false }
            ],
            "scope": "Selected single-core controlled-logical-time target; no hardware timer claim."
        }))
        .map_err(|error| reject(error.to_string()))?;
        timer_report.push(b'\n');
        files.insert("os-generation-timing.json".into(), timer_report);
        for (name, bytes) in contract.files() {
            if name.starts_with("include/") || name == "contract.json" {
                if let Some(previous) = files.insert(name.clone(), bytes.clone()) {
                    if previous != *bytes {
                        return Err(reject(format!("A contract/runtime header differs: {name}")));
                    }
                }
            }
        }
        for (name, bytes) in [
            (
                "src/Ecu_HostBridge.c",
                include_bytes!("../../../runtime/ecu/src/Ecu_HostBridge.c").as_slice(),
            ),
            (
                "src/ecu_host_batch.c",
                include_bytes!("../../../runtime/ecu/src/ecu_host_batch.c").as_slice(),
            ),
            (
                "include/Ecu_HostBatch.h",
                include_bytes!("../../../runtime/ecu/include/Ecu_HostBatch.h").as_slice(),
            ),
            (
                "src/Ecu_HostBatch.c",
                include_bytes!("../../../runtime/ecu/src/Ecu_HostBatch.c").as_slice(),
            ),
            (
                "include/Ecu_Target.h",
                include_bytes!("../../../runtime/ecu/include/Ecu_Target.h").as_slice(),
            ),
            (
                "src/Ecu_Target.c",
                include_bytes!("../../../runtime/ecu/src/Ecu_Target.c").as_slice(),
            ),
            (
                "src/Ecu_OsHooks.c",
                include_bytes!("../../../runtime/ecu/src/Ecu_OsHooks.c").as_slice(),
            ),
            (
                "src/Ecu_SchM.c",
                include_bytes!("../../../runtime/ecu/src/Ecu_SchM.c").as_slice(),
            ),
            (
                "build.ps1",
                include_bytes!("../../../runtime/ecu/build.ps1").as_slice(),
            ),
            (
                "src/ecu_probe.c",
                include_bytes!("../../../runtime/ecu/src/ecu_probe.c").as_slice(),
            ),
        ] {
            if files.insert(name.into(), bytes.to_vec()).is_some() {
                return Err(reject(format!("An ECU source owner collides: {name}")));
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
            "/** @file Generated checked target constants and OS configuration. */\n#ifndef ECU_TARGET_CONFIG_H\n#define ECU_TARGET_CONFIG_H\n#include \"Os_Target.h\"\n#include \"{app_header}\"\n#define ECU_TARGET_TASK 0u\n#define ECU_TARGET_EVENT_WORK {}u\n#define ECU_TARGET_EVENT_APP {}u\n#define ECU_TARGET_EVENT_IO {}u\n#define ECU_TARGET_RX_CAN_ID {}u\n#define ECU_TARGET_RX_DEADLINE_MS {}u\n#define ECU_TARGET_TX_CANIF_PDU {}u\n#define ECU_TARGET_DIAG_TX_CANIF_PDU {}u\n#define ECU_TARGET_DCM_P2_MS {}u\n#define ECU_TARGET_DCM_P2_STAR_MS {}u\n#define ECU_TARGET_DCM_BUFFER_BYTES {}u\n#define ECU_TARGET_RUN_APPLICATION() {}()\n#define ECU_TARGET_TRANSMIT() Com_TriggerTransmit(1u)\nextern const Os_TargetConfig Ecu_OsConfig;\nvoid Ecu_ApplicationInitialize(void);\nStd_ReturnType Ecu_TargetReadDid(uint8_t *data);\n#ifdef ECU_TARGET_TESTS\nint Ecu_TargetTestFailStage(unsigned stage);\nvoid Ecu_TargetTestShutdown(StatusType reason);\n#endif\n#endif\n",
            mask(&work.os_event),
            mask(&app.os_event),
            io.mask,
            rx.can_id,
            rx.deadline_ms.unwrap(),
            tx.can_if_handle,
            plan.diagnostic.response_can_if_handle,
            plan.diagnostic.p2_ms,
            plan.diagnostic.p2_star_ms,
            plan.diagnostic.buffer_bytes,
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
            header.as_bytes().to_vec(),
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
            .map_err(reject)?,
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
        let mut config = include_str!("../../../runtime/ecu/templates/Ecu_Config.c.in").to_owned();
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
        files.insert("src/Ecu_Config.c".into(), config.into_bytes());
        let com = files.get_mut("include/Com.h").unwrap();
        let mut text = String::from_utf8(com.clone()).map_err(|error| reject(error.to_string()))?;
        let original_com = com.clone();
        let end = text.rfind("#endif").unwrap();
        text.insert_str(end, "typedef uint16_t Com_SignalIdType;\n#define COM_SERVICE_NOT_AVAILABLE 0x80u\nStd_ReturnType Com_SendSignal(Com_SignalIdType SignalId, const void *SignalDataPtr);\nStd_ReturnType Com_ReceiveSignal(Com_SignalIdType SignalId, void *SignalDataPtr);\n");
        *com = text.into_bytes();
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
        for (name, template) in [
            (
                "src/Rte.c",
                include_str!("../../../runtime/ecu/templates/Rte.c.in"),
            ),
            (
                "src/Application.c",
                include_str!("../../../runtime/ecu/templates/Application.c.in"),
            ),
        ] {
            let mut source = template.to_owned();
            for (placeholder, value) in &substitutions {
                source = source.replace(&format!("@{placeholder}@"), value);
            }
            if source.contains('@') {
                return Err(reject(
                    "A generated C template contains an unresolved placeholder.",
                ));
            }
            files.insert(name.into(), source.into_bytes());
        }
        for source in self.sources() {
            files.insert(
                format!("inputs/{}", source.logical_path()),
                source.bytes().to_vec(),
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
            "target": "Windows x64 GCC 16.1.0 controlled_logical_ms",
            "owner": "one generated AUTOSTART extended Task_Ecu; StartupHook owns initialization",
            "time": "uint64 epoch, independent software Counter, uint32 FreeRTOS tick",
            "schm": "single BSW owner checks; native input/output and OS retain atomic protocols",
        })).map_err(|error| reject(error.to_string()))?;
        metadata.push(b'\n');
        files.insert("integration.json".into(), metadata);
        files.insert(
            "toolchain.json".into(),
            serde_json::to_vec_pretty(&serde_json::json!({
                "identity": "gcc.exe (Rev5, Built by MSYS2 project) 16.1.0",
                "target": "x86_64-w64-mingw32",
                "sha256": "d38d4dd6bea387499487881383e644ab7c193ac8f8364dc4252d6e6cc09700e2",
            }))
            .unwrap(),
        );
        files.insert("README.md".into(), b"# ECU integration source project\n\nThis generated Windows x64 project consumes one validated standard input plan. The actual single Task_Ecu, generated RTE/reference application, BSW target variants, static OS configuration and fixed FreeRTOS sources are included. The native dispatcher uses the generated 32-slot writable .os_vec section. Standard OS Task and Hook declarations use Os_MemMap.h markers and execute from the read-only .os_code section; the selected profile uses default CODE without a SwAddrMethod location override. The original kernel and fourteen product patches are retained; build.ps1 applies the patches only to its separate build copy. MIT notices remain in kernel/LICENSE.md. Product source is included for authorized internal use, without a new public license grant. AUTOSAR XSD/MOD/PDF files and compiler binaries are external and are not redistributed.\n\nRun PowerShell build.ps1 -OutputDirectory <new-empty-directory> with Git and the pinned GCC 16.1.0 x64 toolchain on PATH. The script checks source manifests, compiler identity and native TLS, and builds a bounded startup probe. Source trees stay unchanged. An optional -ControlSource <external-consumer.c> links an independent native consumer instead of the bundled probe; it supplies main only and consumes the same delivered public headers and runtime. Generation itself requires this fixed compiler and Git for a complete compile/link preflight before installing any destination. Run the resulting ecu_probe.exe; it drives individual controlled ticks and consumes/confirms actual outputs outside the automotive task. Build with -HostBatch to select ecu_host_batch.exe, the production HostBatchV1 stdin/stdout entry. Stage BEGIN <epoch>, up to 256 RX <CAN id> <dlc> <exact hex bytes> lines, then COMMIT. Epochs never decrease and each batch spans at most 1000 ms. Each intermediate tick completes individually; inputs precede processing at their target epoch, and equal epochs do not repeat periodic work. OUT records are confirmed only after successful physical write/flush. COMMIT_OK requires real Waiting with copied inputs, ticks, outputs and acknowledgements drained. COMMIT_ERROR reports executed work and BSW input errors; REJECT is admission failure. Each COMMIT has one fixed 5000 ms host watchdog. Failed/blocked output or the 257th pending output closes the ECU without claiming rollback. Diagnostic trace retains a bounded prefix and reports trace_dropped separately. -HostBatch cannot be combined with -TestMode or -ControlSource. Include Os.h for the generated OS_TICKS2NS/US/MS/SEC_<Counter> macros. Pass a TickType value; each argument is evaluated once and integer seconds truncate toward zero. os-generation-timing.json lists the validated Counter resolution and the selected target internal timers and host watchdogs. These watchdogs never advance automotive time. The bundled probe checks bounded startup/control behavior. Application and network tests use the actual HostBatch interface or an independent ControlSource consumer.\n".to_vec());
        files.get_mut("README.md").unwrap().extend_from_slice(
            format!(
                "\nThe validated OsStatus is {}. Os_Cfg.h fixes this selection and rejects a conflicting compiler override. Both SC1 modes retain defensive host checks and report every detected nonzero service result to the configured ErrorHook. Standard Status retains the mandatory activation-limit and alarm warnings; it does not disable interrupt, stack or shutdown protection.\n",
                if extended_status { "EXTENDED" } else { "STANDARD" }
            ).as_bytes(),
        );
        files.insert(
            "include/Rte_MemMap.h".into(),
            include_bytes!("../../../runtime/ecu/include/Rte_MemMap.h").to_vec(),
        );
        files.extend(super::artifacts::files(plan, &files).map_err(reject)?);
        files.extend(super::handoff::verification_files(self).map_err(reject)?);
        let files = generator::seal_files(files.into_iter().collect());
        Ok(EcuIntegrationFiles { files })
    }
}
