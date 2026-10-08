use super::{PlanDescription, configuration::ConfigurationRecord};
use std::fmt::Write;

const DEF: &str = "/AUTOSAR/EcucDefs/Arti";
const ROOT: &str = "/ArtiDescriptions/Arti";
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn parameter(definition: &str, value: &str, kind: &str) -> String {
    let tag = if matches!(kind, "ECUC-INTEGER-PARAM-DEF" | "ECUC-BOOLEAN-PARAM-DEF") {
        "ECUC-NUMERICAL-PARAM-VALUE"
    } else {
        "ECUC-TEXTUAL-PARAM-VALUE"
    };
    format!(
        "<{tag}><DEFINITION-REF DEST=\"{kind}\">{definition}</DEFINITION-REF><VALUE>{}</VALUE></{tag}>",
        escape(value)
    )
}
fn reference(definition: &str, target: &str) -> String {
    format!(
        "<ECUC-REFERENCE-VALUE><DEFINITION-REF DEST=\"ECUC-REFERENCE-DEF\">{definition}</DEFINITION-REF><VALUE-REF DEST=\"ECUC-CONTAINER-VALUE\">{}</VALUE-REF></ECUC-REFERENCE-VALUE>",
        escape(target)
    )
}
fn container(
    name: &str,
    definition: &str,
    parameters: &str,
    references: &str,
    children: &str,
) -> String {
    let mut xml = format!(
        "<ECUC-CONTAINER-VALUE><SHORT-NAME>{}</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">{definition}</DEFINITION-REF>",
        escape(name)
    );
    for (tag, contents) in [
        ("PARAMETER-VALUES", parameters),
        ("REFERENCE-VALUES", references),
        ("SUB-CONTAINERS", children),
    ] {
        if !contents.is_empty() {
            write!(xml, "<{tag}>{contents}</{tag}>").unwrap();
        }
    }
    xml.push_str("</ECUC-CONTAINER-VALUE>");
    xml
}
#[derive(Default)]
struct Description {
    values: String,
    generic: String,
    hardware: String,
    objects: String,
}
impl Description {
    fn property(&mut self, name: &str, expression: &str) -> String {
        let expr = format!("{name}Expression");
        self.values.push_str(&container(
            &expr,
            &format!("{DEF}/ArtiValues/ArtiExpression"),
            &parameter(
                &format!("{DEF}/ArtiValues/ArtiExpression/ArtiExpressionString"),
                expression,
                "ECUC-STRING-PARAM-DEF",
            ),
            "",
            "",
        ));
        self.values.push_str(&container(name, &format!("{DEF}/ArtiValues/ArtiObjectInstanceParameter"), "",
            &reference(&format!("{DEF}/ArtiValues/ArtiObjectInstanceParameter/ArtiObjectInstanceParameterExpressionRef"), &format!("{ROOT}/Values/{expr}")), ""));
        format!("{ROOT}/Values/{name}")
    }
    fn class_property(&mut self, name: &str, type_map: Option<&str>) -> String {
        let refs = type_map.map(|map| reference(&format!("{DEF}/ArtiValues/ArtiObjectClassParameter/ArtiObjectClassParameterTypeMapRef"), &format!("{ROOT}/Values/{map}"))).unwrap_or_default();
        self.values.push_str(&container(
            name,
            &format!("{DEF}/ArtiValues/ArtiObjectClassParameter"),
            "",
            &refs,
            "",
        ));
        format!("{ROOT}/Values/{name}")
    }
    fn class(&mut self, kind: &str, fields: &[(&str, Option<&str>)]) {
        let definition = format!("{DEF}/ArtiOs/ArtiOs{kind}Class");
        let mut refs = String::new();
        for (field, map) in fields {
            let value = self.class_property(&format!("{kind}Class{field}"), *map);
            refs.push_str(&reference(
                &format!("{definition}/ArtiOs{kind}Class{field}Ref"),
                &value,
            ));
        }
        self.objects.push_str(&container(
            &format!("{kind}Class"),
            &definition,
            "",
            &refs,
            "",
        ));
    }
    fn instance(
        &mut self,
        kind: &str,
        name: &str,
        ecuc: Option<&str>,
        id: Option<usize>,
        fields: &[(&str, String)],
        parameters: &[(&str, String, &str)],
    ) {
        let definition = format!("{DEF}/ArtiOs/ArtiOs{kind}Instance");
        let mut refs = String::new();
        if let Some(path) = ecuc {
            // Alarm/Resource use the published EcuC spelling; other objects use Ecuc.
            let suffix = if matches!(kind, "Alarm" | "Resource") {
                "EcuCRef"
            } else {
                "EcucRef"
            };
            refs.push_str(&reference(
                &format!("{definition}/ArtiOs{kind}Instance{suffix}"),
                path,
            ));
        }
        let mut params = String::new();
        if let Some(value) = id {
            params.push_str(&parameter(
                &format!("{definition}/ArtiOs{kind}InstanceId"),
                &value.to_string(),
                "ECUC-INTEGER-PARAM-DEF",
            ));
        }
        for (field, expression) in fields {
            let value = self.property(&format!("{name}{field}"), expression);
            refs.push_str(&reference(
                &format!("{definition}/ArtiOs{kind}Instance{field}Ref"),
                &value,
            ));
        }
        for (field, value, ty) in parameters {
            params.push_str(&parameter(
                &format!("{definition}/ArtiOs{kind}Instance{field}"),
                value,
                ty,
            ));
        }
        self.objects
            .push_str(&container(name, &definition, &params, &refs, ""));
    }
    fn pointer_map(&mut self, name: &str, expression: &str, target: &str) {
        let property = format!("{name}Address");
        self.property(&property, expression);
        let def = format!("{DEF}/ArtiValues/ArtiParameterTypeMap/ArtiParameterTypeMapPair");
        let refs = reference(
            &format!("{def}/ArtiParameterTypeMapPairInputExpressionRef"),
            &format!("{ROOT}/Values/{property}Expression"),
        ) + &reference(&format!("{def}/ArtiParameterTypeMapPairOutputRef"), target)
            .replace(
                "DEST=\"ECUC-REFERENCE-DEF\"",
                "DEST=\"ECUC-CHOICE-REFERENCE-DEF\"",
            );
        self.values.push_str(&container(
            name,
            &format!("{DEF}/ArtiValues/ArtiParameterTypeMap"),
            "",
            "",
            &container("Address", &def, "", &refs, ""),
        ));
    }
    fn type_map(&mut self, name: &str, entries: &[(usize, String)]) {
        let def = format!("{DEF}/ArtiValues/ArtiParameterTypeMap/ArtiParameterTypeMapPair");
        let mut pairs = String::new();
        for (input, target) in entries {
            let params = parameter(
                &format!("{def}/ArtiParameterTypeMapPairInput"),
                &input.to_string(),
                "ECUC-INTEGER-PARAM-DEF",
            );
            let refs = reference(&format!("{def}/ArtiParameterTypeMapPairOutputRef"), target)
                .replace(
                    "DEST=\"ECUC-REFERENCE-DEF\"",
                    "DEST=\"ECUC-CHOICE-REFERENCE-DEF\"",
                );
            pairs.push_str(&container(
                &format!("Value{input}"),
                &def,
                &params,
                &refs,
                "",
            ));
        }
        self.values.push_str(&container(
            name,
            &format!("{DEF}/ArtiValues/ArtiParameterTypeMap"),
            "",
            "",
            &pairs,
        ));
    }
}
fn selected<'a>(records: &'a [ConfigurationRecord], name: &str) -> Option<&'a ConfigurationRecord> {
    records
        .iter()
        .find(|record| record.definition == format!("/AUTOSAR/EcucDefs/Os/{name}"))
}
pub(super) fn files(
    plan: &PlanDescription,
    scheduler: bool,
    alarms: &[String],
    tables: &[String],
) -> Result<Vec<(String, Vec<u8>)>, crate::message::LocalizedText> {
    let os = selected(&plan.configuration, "OsOS").ok_or(crate::product_message!(
        "backend.integration.arti.arti_selected_os_configuration_required"
    ))?;
    let module = os
        .path
        .rsplit_once('/')
        .ok_or(crate::product_message!(
            "backend.integration.arti.arti_os_module_path_required"
        ))?
        .0;
    let name = module.rsplit('/').next().unwrap();
    if super::component::c_name(name) != name {
        return Err(crate::product_message!(
            "backend.integration.arti.arti_os_short_name_not_representable"
        )
        .into());
    }
    let mut d = Description::default();
    let mut states = String::new();
    for (kind, labels) in [
        ("Task", &["Running", "Waiting", "Ready", "Suspended"][..]),
        (
            "ScheduleTable",
            &[
                "Stopped",
                "Next",
                "Waiting",
                "Running",
                "RunningAndSynchronous",
            ][..],
        ),
    ] {
        let def = format!("{DEF}/ArtiValues/ArtiStates/ArtiStates{kind}State");
        let mut entries = Vec::new();
        for (value, label) in labels.iter().enumerate() {
            let object = format!("{kind}{label}");
            states.push_str(&container(
                &object,
                &def,
                &parameter(
                    &format!("{def}/ArtiStates{kind}StateEnum"),
                    &format!("Arti{kind}State{label}"),
                    "ECUC-ENUMERATION-PARAM-DEF",
                ),
                "",
                "",
            ));
            entries.push((value, format!("{ROOT}/Values/States/{object}")));
        }
        d.type_map(&format!("{kind}States"), &entries);
    }
    d.values.push_str(&container(
        "States",
        &format!("{DEF}/ArtiValues/ArtiStates"),
        &parameter(
            &format!("{DEF}/ArtiValues/ArtiStates/ArtiStatesTaskEnhanced"),
            "false",
            "ECUC-BOOLEAN-PARAM-DEF",
        ),
        "",
        &states,
    ));
    let mode = selected(&plan.configuration, "OsAppMode").ok_or(crate::product_message!(
        "backend.integration.arti.arti_selected_application_mode_required"
    ))?;
    d.type_map("AppModes", &[(1, mode.path.clone())]);
    d.class("", &[("AppMode", Some("AppModes"))]);
    d.instance(
        "",
        "OperatingSystem",
        Some(&os.path),
        None,
        &[
            ("AppMode", "*Os_ArtiAppMode".into()),
            ("Valid", "*Os_ArtiOsReady != 0".into()),
        ],
        &[],
    );
    d.pointer_map(
        "TaskStacks",
        "Os_ArtiTaskStacks[0]",
        &format!("{ROOT}/Os/Task0Stack"),
    );
    d.pointer_map(
        "TaskContexts",
        "Os_ArtiTaskContexts[0]",
        &format!("{ROOT}/Os/Task0Context"),
    );
    d.class(
        "Task",
        &[
            ("CurrentTaskState", Some("TaskStates")),
            ("Priority", None),
            ("Stack", Some("TaskStacks")),
            ("Context", Some("TaskContexts")),
        ],
    );
    d.instance(
        "Task",
        "Task0",
        Some(&plan.schedule.task),
        Some(0),
        &[
            ("CurrentTaskState", "Os_ArtiTasks[0].state".into()),
            ("Priority", "Os_ArtiTasks[0].priority".into()),
            ("CurrentActivations", "Os_ArtiTasks[0].activations".into()),
            ("Stack", "Os_ArtiTaskStacks[0]".into()),
            ("Context", "Os_ArtiTaskContexts[0]".into()),
            ("Valid", "*Os_ArtiOsReady != 0".into()),
        ],
        &[(
            "Function",
            format!(
                "Os_TaskEntry_OS_TASK_ID_{}",
                plan.schedule.task.rsplit('/').next().unwrap()
            ),
            "ECUC-FUNCTION-NAME-DEF",
        )],
    );
    d.class("Stack", &[]);
    d.instance("Stack", "Task0Stack", None, None, &[
        ("BaseAddress", "Os_ArtiTaskStacks[0] == 0 ? 0 : Os_ArtiTaskStacks[0]->high".into()),
        ("Size", "Os_ArtiTaskStacks[0] == 0 ? 0 : Os_ArtiTaskStacks[0]->high - Os_ArtiTaskStacks[0]->reserve_low".into()),
        ("Valid", "Os_ArtiTaskStacks[0] != 0".into())], &[("Direction", "DOWN".into(), "ECUC-STRING-PARAM-DEF")]);
    // Os_StackInit registers the native dispatcher before creating any actors.
    // Its record is the actual shared Cat2 execution stack, not a kernel buffer.
    d.instance(
        "Stack",
        "HostIsrStack",
        None,
        None,
        &[
            ("BaseAddress", "Os_ArtiStacks[0].high".into()),
            (
                "Size",
                "Os_ArtiStacks[0].high - Os_ArtiStacks[0].reserve_low".into(),
            ),
            ("Valid", "Os_ArtiStacks[0].thread_id != 0".into()),
        ],
        &[("Direction", "DOWN".into(), "ECUC-STRING-PARAM-DEF")],
    );
    d.class("Context", &[]);
    d.instance(
        "Context",
        "Task0Context",
        None,
        None,
        &[
            ("Address", "Os_ArtiTaskContexts[0]".into()),
            ("Size", "Os_ArtiNativeContextSize".into()),
            (
                "Valid",
                "Os_ArtiTaskStacks[0] != 0 && Os_ArtiTaskStacks[0]->context_valid != 0".into(),
            ),
        ],
        &[],
    );
    d.class("Isr", &[]);
    d.instance(
        "Isr",
        "HostInputIsr",
        None,
        Some(30),
        &[(
            "Valid",
            "Os_Config != 0 && Os_Config->input_event != 0".into(),
        )],
        &[(
            "Category",
            "CATEGORY_2".into(),
            "ECUC-ENUMERATION-PARAM-DEF",
        )],
    );
    if !alarms.is_empty() {
        d.class("Alarm", &[("State", None)]);
    }
    for (index, path) in alarms.iter().enumerate() {
        d.instance(
            "Alarm",
            &format!("Alarm{index}"),
            Some(path),
            None,
            &[
                ("State", format!("Os_ArtiAlarms[{index}].active")),
                ("AlarmTime", format!("Os_ArtiAlarms[{index}].remaining")),
                ("CycleTime", format!("Os_ArtiAlarms[{index}].cycle")),
                ("Valid", "*Os_ArtiOsReady != 0".into()),
            ],
            &[(
                "Counter",
                plan.schedule.counter.clone(),
                "ECUC-STRING-PARAM-DEF",
            )],
        );
    }
    if !tables.is_empty() {
        d.class(
            "ScheduleTable",
            &[("CurrentState", Some("ScheduleTableStates"))],
        );
    }
    for (index, path) in tables.iter().enumerate() {
        d.instance(
            "ScheduleTable",
            &format!("ScheduleTable{index}"),
            Some(path),
            None,
            &[
                (
                    "CurrentState",
                    format!("Os_ArtiScheduleTables[{index}].status"),
                ),
                ("CounterValue", "Os_ArtiCounters[0]".into()),
                (
                    "ExpiryTime",
                    format!("Os_ArtiScheduleTables[{index}].remaining"),
                ),
                (
                    "NextExpiryPoint",
                    format!("Os_ArtiScheduleTables[{index}].point"),
                ),
                (
                    "NextScheduleTable",
                    format!("Os_ArtiScheduleTables[{index}].next"),
                ),
            ],
            &[],
        );
    }
    if scheduler {
        d.type_map("ResourceOwners", &[(1, format!("{ROOT}/Os/Task0"))]);
        d.class(
            "Resource",
            &[("State", None), ("Locker", Some("ResourceOwners"))],
        );
        d.instance(
            "Resource",
            "RES_SCHEDULER",
            None,
            None,
            &[
                ("State", "Os_ArtiResourceOwners[0] != 0".into()),
                ("Locker", "Os_ArtiResourceOwners[0]".into()),
                ("Valid", "*Os_ArtiOsReady != 0".into()),
            ],
            &[(
                "Priority",
                plan.schedule.task_priority.to_string(),
                "ECUC-STRING-PARAM-DEF",
            )],
        );
    }
    d.type_map("TaskIds", &[(0, format!("{ROOT}/Os/Task0"))]);
    d.type_map("CoreIds", &[(0, format!("{ROOT}/Hardware/Core0"))]);
    d.type_map("IsrIds", &[(30, format!("{ROOT}/Os/HostInputIsr"))]);
    let hw_class = format!("{DEF}/ArtiHardware/ArtiHardwareCoreClass");
    let current_task_class = d.class_property("CoreCurrentTaskClass", Some("TaskIds"));
    let current_isr_class = d.class_property("CoreCurrentIsrClass", Some("IsrIds"));
    d.hardware.push_str(&container(
        "CoreClass",
        &hw_class,
        "",
        &(reference(
            &format!("{hw_class}/ArtiHardwareCoreClassCurrentTaskRef"),
            &current_task_class,
        ) + &reference(
            &format!("{hw_class}/ArtiHardwareCoreClassCurrentIsrRef"),
            &current_isr_class,
        )),
        "",
    ));
    let hw_instance = format!("{DEF}/ArtiHardware/ArtiHardwareCoreInstance");
    let mut hw_refs = reference(
        &format!("{hw_instance}/ArtiHardwareCoreInstanceEcucCoreRef"),
        "/ArtiDescriptions/HostEcuC/Hardware/LogicalCore0",
    );
    for (field, expression) in [
        ("CurrentTask", "Os_ArtiRunningTask"),
        (
            "CurrentIsr",
            "*Os_ArtiCurrentIsr <= 1 || *Os_ArtiCurrentIsr >= 32 ? 4294967295 : *Os_ArtiCurrentIsr",
        ),
        ("CurrentApplication", "4294967295"),
        ("Valid", "*Os_ArtiOsReady != 0"),
    ] {
        let value = d.property(&format!("Core0{field}"), expression);
        hw_refs.push_str(&reference(
            &format!("{hw_instance}/ArtiHardwareCoreInstance{field}Ref"),
            &value,
        ));
    }
    d.hardware.push_str(&container(
        "Core0",
        &hw_instance,
        &parameter(
            &format!("{hw_instance}/ArtiHardwareCoreInstanceCoreId"),
            "0",
            "ECUC-INTEGER-PARAM-DEF",
        ),
        &hw_refs,
        "",
    ));
    // Counter is an original generic ARTI object; OS has no ArtiOsCounter container.
    let generic_class = format!("{DEF}/ArtiGeneric/ArtiGenericComponentClass");
    let generic_parameter = format!("{generic_class}/ArtiGenericComponentClassParameter");
    d.generic.push_str(&container(
        "CounterClass",
        &generic_class,
        &parameter(
            &format!("{generic_class}/ArtiGenericComponentClassName"),
            "COUNTER",
            "ECUC-STRING-PARAM-DEF",
        ),
        "",
        &container(
            "Value",
            &generic_parameter,
            &parameter(
                &format!("{generic_parameter}/ArtiGenericComponentClassParameterName"),
                "Value",
                "ECUC-STRING-PARAM-DEF",
            ),
            "",
            "",
        ),
    ));
    d.property("Counter0Value", "Os_ArtiCounters[0]");
    let generic_instance = format!("{DEF}/ArtiGeneric/ArtiGenericComponentInstance");
    let instance_parameter = format!("{generic_instance}/ArtiGenericComponentInstanceParameter");
    let generic_refs = reference(
        &format!("{generic_instance}/ArtiGenericComponentInstanceClassRef"),
        &format!("{ROOT}/Generic/CounterClass"),
    );
    let value_refs = reference(
        &format!("{instance_parameter}/ArtiGenericComponentInstanceParameterClassParameterRef"),
        &format!("{ROOT}/Generic/CounterClass/Value"),
    ) + &reference(
        &format!("{instance_parameter}/ArtiGenericComponentInstanceParameterExpressionRef"),
        &format!("{ROOT}/Values/Counter0ValueExpression"),
    );
    d.generic.push_str(&container(
        "Counter0",
        &generic_instance,
        &parameter(
            &format!("{generic_instance}/ArtiGenericComponentInstanceName"),
            plan.schedule.counter.rsplit('/').next().unwrap(),
            "ECUC-STRING-PARAM-DEF",
        ),
        &generic_refs,
        &container("Value", &instance_parameter, "", &value_refs, ""),
    ));
    let mut hook_refs = String::new();
    let binding =
        include_str!("../../../runtime/os/src/Os_Arti.c").replace(", Os,", &format!(", {name},"));
    let mut hooks = std::collections::BTreeSet::new();
    for call in binding.split("ARTI_TRACE(").skip(1) {
        let (arguments, _) = call.split_once(';').ok_or(crate::product_message!(
            "backend.integration.arti.arti_trace_invocation_malformed"
        ))?;
        let arguments = arguments.split(',').collect::<Vec<_>>();
        if arguments.len() != 6 {
            return Err(crate::product_message!(
                "backend.integration.arti.arti_single_parameter_trace_invocation_malformed"
            )
            .into());
        }
        let context = arguments[0].trim();
        let class = arguments[1].trim();
        let event = arguments[4].trim();
        if !hooks.insert((context, class, event)) {
            continue;
        }
        let def = format!("{DEF}/ArtiValues/ArtiHook");
        let mut params = String::new();
        for (field, value) in [
            ("Context", context),
            ("Class", class),
            ("EventName", event),
            ("Instance", name),
            ("FileName", "Os_Arti.c"),
        ] {
            params.push_str(&parameter(
                &format!("{def}/ArtiHook{field}"),
                value,
                "ECUC-STRING-PARAM-DEF",
            ));
        }
        let mut refs = reference(
            &format!("{def}/ArtiHookInstanceParameterTypeRef"),
            &format!("{ROOT}/Values/CoreIds"),
        );
        if let Some(map) = match class {
            "AR_CP_OS_TASK" => Some("TaskIds"),
            "AR_CP_OS_CAT2ISR" => Some("IsrIds"),
            _ => None,
        } {
            refs.push_str(&reference(
                &format!("{def}/ArtiHookEventParameterTypeRef"),
                &format!("{ROOT}/Values/{map}"),
            ));
        }
        d.values
            .push_str(&container(event, &def, &params, &refs, ""));
        hook_refs.push_str(&reference(
            &format!("{DEF}/ArtiOs/ArtiOsInstance/ArtiOsInstanceHookRef"),
            &format!("{ROOT}/Values/{event}"),
        ));
    }
    // Attach hooks to the OS instance rather than introducing a parallel manifest.
    let instance_end = "</REFERENCE-VALUES></ECUC-CONTAINER-VALUE>";
    let start = d
        .objects
        .find("<SHORT-NAME>OperatingSystem</SHORT-NAME>")
        .ok_or(crate::product_message!(
            "backend.integration.arti.arti_generated_os_instance_missing"
        ))?;
    let at = start
        + d.objects[start..]
            .find(instance_end)
            .ok_or(crate::product_message!(
                "backend.integration.arti.arti_generated_os_instance_references_missing"
            ))?;
    d.objects.insert_str(at, &hook_refs);
    let children = container("Values", &format!("{DEF}/ArtiValues"), "", "", &d.values)
        + &container("Os", &format!("{DEF}/ArtiOs"), "", "", &d.objects)
        + &container("Generic", &format!("{DEF}/ArtiGeneric"), "", "", &d.generic)
        + &container(
            "Hardware",
            &format!("{DEF}/ArtiHardware"),
            "",
            "",
            &d.hardware,
        );
    let core_definition = "/AUTOSAR/EcucDefs/EcuC/EcucHardware/EcucCoreDefinition";
    let core = container(
        "LogicalCore0",
        core_definition,
        &parameter(
            &format!("{core_definition}/EcucCoreId"),
            "0",
            "ECUC-INTEGER-PARAM-DEF",
        ),
        "",
        "",
    );
    let hardware = container(
        "Hardware",
        "/AUTOSAR/EcucDefs/EcuC/EcucHardware",
        "",
        "",
        &core,
    );
    let host_ecuc = format!(
        "<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>HostEcuC</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-MODULE-DEF\">/AUTOSAR/EcucDefs/EcuC</DEFINITION-REF><IMPLEMENTATION-CONFIG-VARIANT>VARIANT-PRE-COMPILE</IMPLEMENTATION-CONFIG-VARIANT><POST-BUILD-VARIANT-USED>false</POST-BUILD-VARIANT-USED><CONTAINERS>{hardware}</CONTAINERS></ECUC-MODULE-CONFIGURATION-VALUES>"
    );
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>ArtiDescriptions</SHORT-NAME><ELEMENTS><ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>Arti</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-MODULE-DEF\">{DEF}</DEFINITION-REF><ECUC-DEF-EDITION>4.10.0</ECUC-DEF-EDITION><IMPLEMENTATION-CONFIG-VARIANT>VARIANT-PRE-COMPILE</IMPLEMENTATION-CONFIG-VARIANT><POST-BUILD-VARIANT-USED>false</POST-BUILD-VARIANT-USED><CONTAINERS>{children}</CONTAINERS></ECUC-MODULE-CONFIGURATION-VALUES>{host_ecuc}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>\n"
    );
    Ok(vec![
        ("os/Os_Arti.arxml".into(), xml.into_bytes()),
        ("os/src/Os_Arti.c".into(), binding.into_bytes()),
    ])
}
