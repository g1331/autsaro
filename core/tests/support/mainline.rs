//! Mainline fixture inputs, XML edits and project helpers.
use super::packages;
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
use super::tooling;
use super::workspace::Scratch;
use autosar_config_core::{
    definitions::DefinitionCatalog,
    integration::{InputSource, ValidatedIntegrationPlan, build_plan_native},
};
use std::path::Path;

pub(crate) fn inputs() -> Vec<InputSource> {
    let directory =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/component-pipeline/arxml");
    let mut sources = std::fs::read_dir(directory)
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

pub(crate) fn build(
    sources: &[InputSource],
) -> Result<ValidatedIntegrationPlan, Vec<autosar_config_core::integration::PlanDiagnostic>> {
    build_plan_native(sources, &DefinitionCatalog::builtin().unwrap())
}

pub(crate) fn change(sources: &mut [InputSource], file: &str, before: &str, after: &str) {
    let source = sources
        .iter_mut()
        .find(|source| source.logical_path() == file)
        .unwrap();
    let text = std::str::from_utf8(source.bytes()).unwrap();
    assert!(text.contains(before), "{file}: {before}");
    *source = InputSource::new(file, text.replacen(before, after, 1).into_bytes()).unwrap();
}

/// Fixture inputs with two simultaneous network receivers: the existing `RxValue` path
/// (Com user handle 43) and a clone named `RxSecond` (handle 61, 50 ms timeout, CAN id 802).
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
pub(crate) fn two_receiver_inputs() -> Vec<InputSource> {
    // (file, element, anchor, definition the anchor's DEFINITION-REF must end with, edits)
    // Each edit sets the first <element> after `anchor` (empty anchor: block start) to `value`.
    type Edit = (&'static str, &'static str, &'static str);
    type Clone = (&'static str, &'static str, String, &'static str, Vec<Edit>);
    fn named(
        file: &'static str,
        tag: &'static str,
        short: &str,
        definition: &'static str,
        edits: &[Edit],
    ) -> Clone {
        let anchor = format!("<SHORT-NAME>{short}</SHORT-NAME>");
        (file, tag, anchor, definition, edits.to_vec())
    }
    let ecuc = |tag, short: &str, definition, edits: &[Edit]| {
        named("ecuc.arxml", tag, short, definition, edits)
    };
    let extract = |tag, short: &str, edits: &[Edit]| named("extract.arxml", tag, short, "", edits);
    let clones: Vec<Clone> = vec![
        extract("FRAME-PORT", "RxValueFramePort", &[]),
        extract("I-PDU-PORT", "RxValuePduPort", &[]),
        extract(
            "I-SIGNAL-PORT",
            "RxValueSignalPort",
            &[("", "TIMEOUT", "0.05")],
        ),
        extract(
            "CAN-FRAME-TRIGGERING",
            "RxValueTrigger",
            &[("", "IDENTIFIER", "802")],
        ),
        extract("I-SIGNAL-TRIGGERING", "RxValueSignalTrigger", &[]),
        extract("PDU-TRIGGERING", "RxValuePduTrigger", &[]),
        extract("CAN-FRAME", "RxValueFrame", &[]),
        extract("I-SIGNAL-I-PDU", "RxValuePdu", &[]),
        extract("I-SIGNAL", "RxValueSignal", &[]),
        extract("SYSTEM-SIGNAL", "RxValueSystemSignal", &[]),
        (
            "extract.arxml",
            "SENDER-RECEIVER-TO-SIGNAL-MAPPING",
            "/Application/Ingress/RxValue</CONTEXT-PORT-REF>".to_string(),
            "",
            vec![],
        ),
        ecuc(
            "ECUC-CONTAINER-VALUE",
            "RxValue",
            "/ComUserModuleCnf/ComUserSignal",
            &[("ComUserCbkHandleId", "VALUE", "61")],
        ),
        ecuc(
            "ECUC-CONTAINER-VALUE",
            "RxValuePdu",
            "/EcucPduCollection/Pdu",
            &[],
        ),
        ecuc(
            "ECUC-CONTAINER-VALUE",
            "RxValue",
            "/ComConfig/ComSignal",
            &[("ComTimeout", "VALUE", "0.05")],
        ),
        ecuc(
            "ECUC-CONTAINER-VALUE",
            "RxValuePdu",
            "/ComConfig/ComIPdu",
            &[],
        ),
        ecuc(
            "ECUC-CONTAINER-VALUE",
            "RxValue",
            "/CanIfInitCfg/CanIfRxPduCfg",
            &[
                ("CanIfRxPduCanId", "VALUE", "802"),
                ("CanIfRxPduId", "VALUE", "2"),
            ],
        ),
        ecuc(
            "ECUC-CONTAINER-VALUE",
            "RxValueSource",
            "/PduRRoutingPaths/PduRSrcPdu",
            &[("PduRSourcePduHandleId", "VALUE", "4")],
        ),
        ecuc(
            "ECUC-CONTAINER-VALUE",
            "RxValueDestination",
            "/PduRRoutingPaths/PduRDestPdu",
            &[("PduRDestPduHandleId", "VALUE", "4")],
        ),
        ecuc(
            "ECUC-CONTAINER-VALUE",
            "RxValue",
            "/PduRRoutingPaths/PduRRoutingPath",
            &[],
        ),
        (
            "ingress.arxml",
            "R-PORT-PROTOTYPE",
            "<SHORT-NAME>RxValue</SHORT-NAME>".to_string(),
            "",
            vec![("", "ALIVE-TIMEOUT", "0.05"), ("", "VALUE", "9")],
        ),
        (
            "ingress.arxml",
            "VARIABLE-ACCESS",
            "<SHORT-NAME>RRxValue</SHORT-NAME>".to_string(),
            "",
            vec![],
        ),
    ];
    let mut sources = inputs();
    for (file, tag, anchor, definition, edits) in clones {
        let source = sources
            .iter_mut()
            .find(|source| source.logical_path() == file)
            .unwrap();
        let mut text = std::str::from_utf8(source.bytes()).unwrap().to_owned();
        let start = text
            .match_indices(&anchor)
            .find(|(at, _)| {
                let rest = &text[*at..];
                definition.is_empty()
                    || rest[..rest.find("</DEFINITION-REF>").unwrap()].ends_with(definition)
            })
            .map(|(at, _)| text[..at].rfind(&format!("<{tag}>")).unwrap())
            .unwrap_or_else(|| panic!("{file}: no <{tag}> with {anchor} {definition}"));
        let end = start + text[start..].find(&format!("</{tag}>")).unwrap() + tag.len() + 3;
        let mut block = text[start..end].replace("RxValue", "RxSecond");
        for (anchor, element, value) in edits {
            let from = block.find(anchor).unwrap();
            let open = format!("<{element}>");
            let value_start = from + block[from..].find(&open).unwrap() + open.len();
            let value_end = value_start + block[value_start..].find('<').unwrap();
            block.replace_range(value_start..value_end, value);
        }
        // Keep the original's indentation for the copy that follows it.
        let line = text[..start].rfind('\n').map_or(0, |at| at + 1);
        let indent = text[line..start].to_owned();
        text.insert_str(end, &format!("\n{indent}{block}"));
        *source = InputSource::new(file, text.into_bytes()).unwrap();
    }
    change(
        &mut sources,
        "ecuc.arxml",
        "<VALUE>17</VALUE>",
        "<VALUE>43</VALUE>",
    );
    change(
        &mut sources,
        "ingress.arxml",
        "<VALUE>0</VALUE>",
        "<VALUE>7</VALUE>",
    );
    sources
}

pub(crate) fn live_pipeline_workspace(
    root: &Path,
    sources: &[InputSource],
) -> autosar_config_core::Workspace {
    std::fs::create_dir_all(root).unwrap();
    for source in sources {
        std::fs::write(root.join(source.logical_path()), source.bytes()).unwrap();
    }
    let manifest = autosar_config_core::arxml::ProjectManifest {
        format_version: 1,
        declared_release: "R24-11".into(),
        profile_hint: "singlecore-ecu-v1".into(),
        inputs: sources
            .iter()
            .map(|source| autosar_config_core::arxml::ProjectInput {
                path: source.logical_path().into(),
                role_hint: "standard".into(),
            })
            .collect(),
        application_inputs: Vec::new(),
        accepted_extension_definitions: Vec::new(),
    };
    let path = root.join("workbench-project.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    autosar_config_core::Workspace::open_project_manifest(&path, &root.join("cache")).unwrap()
}

pub(crate) fn initialized_pipeline_workspace(root: &Path) -> autosar_config_core::Workspace {
    let mut arxml = inputs();
    change(
        &mut arxml,
        "ingress.arxml",
        "<VALUE>0</VALUE>",
        "<VALUE>7</VALUE>",
    );
    let mut workspace = live_pipeline_workspace(root, &arxml);
    let initialization = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&initialization)
        .unwrap();
    for file in &initialization.files {
        let name = Path::new(&file.path).file_name().unwrap();
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/component-pipeline/applications")
                .join(name),
            root.join(&file.path),
        )
        .unwrap();
    }
    autosar_config_core::Workspace::open_project_manifest(
        &root.join("workbench-project.json"),
        &root.join("cache"),
    )
    .unwrap()
}

// Consume the normal complete producer rather than a public source-fragment API.
pub(crate) fn prepared_pipeline_files(plan: &ValidatedIntegrationPlan) -> Vec<(String, Vec<u8>)> {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/component-pipeline/applications");
    let scratch = Scratch::new();
    let renamed = plan
        .application_slot_descriptors()
        .unwrap()
        .iter()
        .any(|slot| slot.source_paths[0] == "application/Compute.c");
    let sources: Vec<_> = plan
        .application_slot_descriptors()
        .unwrap()
        .iter()
        .map(|slot| {
            let file = slot.source_paths[0].rsplit('/').next().unwrap();
            let mut text = std::fs::read_to_string(fixture.join(if file == "Compute.c" {
                "Process.c"
            } else {
                file
            }))
            .unwrap();
            if renamed {
                for (before, after) in [
                    ("Process", "Compute"),
                    ("ResultService", "Calculation"),
                    ("Transform", "Calculate"),
                ] {
                    text = text.replace(before, after);
                }
                if file == "Compute.c" {
                    text = text.replace("Rte_Read_Value_Value", "Rte_Read_InputValue_Value");
                }
            }
            let path = scratch.0.join(file);
            std::fs::write(&path, text).unwrap();
            autosar_config_core::ApplicationSource {
                component_instance: slot
                    .producer_slot
                    .strip_prefix("singlecore-ecu-v1:")
                    .unwrap()
                    .into(),
                path,
            }
        })
        .collect();
    autosar_config_core::prepare_ecu_project_with_applications(
        plan,
        autosar_config_core::target::BuildTarget::LinuxX64ControlledV1,
        &sources,
    )
    .unwrap()
    .into_files()
}

pub(crate) fn replace_all(sources: &mut [InputSource], before: &str, after: &str) {
    for source in sources {
        let text = std::str::from_utf8(source.bytes()).unwrap();
        *source = InputSource::new(
            source.logical_path(),
            text.replace(before, after).into_bytes(),
        )
        .unwrap();
    }
}

pub(crate) fn conflicting_schedule_sources() -> Vec<InputSource> {
    let mut sources = inputs();
    let original = sources
        .iter()
        .find(|source| source.logical_path() == "ecuc.arxml")
        .unwrap();
    let text = std::str::from_utf8(original.bytes()).unwrap().to_owned();
    let (task, app) = text
        .split_once("<SHORT-NAME>Alarm_App</SHORT-NAME>")
        .unwrap();
    let changed = format!(
        "{task}<SHORT-NAME>Alarm_App</SHORT-NAME>{}",
        app.replace(
            "/Configuration/Os/Ev_App</VALUE-REF>",
            "/Configuration/Os/Ev_Work</VALUE-REF>"
        )
    );
    change(&mut sources, "ecuc.arxml", &text, &changed);
    sources
}

pub(crate) fn assert_rejected_sources_preserved(sources: &[InputSource]) {
    let scratch = Scratch::new();
    let live = scratch.0.join("live");
    let workspace = live_pipeline_workspace(&live, sources);
    assert!(workspace.saved_integration_plan().is_err());
    assert!(workspace.preview_application_initialization().is_err());
    assert!(!live.join("application").exists());
    for source in sources {
        assert_eq!(
            std::fs::read(live.join(source.logical_path())).unwrap(),
            source.bytes()
        );
    }
}

pub(crate) fn equivalent_schedule_triggers(tables: bool) -> Vec<InputSource> {
    let mut sources = inputs();
    let alarm = subtree(&sources, "ecuc.arxml", "ECUC-CONTAINER-VALUE", "Alarm_App");
    let second_alarm = alarm.replace(
        "<SHORT-NAME>Alarm_App</SHORT-NAME>",
        "<SHORT-NAME>Alarm_Process</SHORT-NAME>",
    );
    change(
        &mut sources,
        "ecuc.arxml",
        &alarm,
        &format!("{alarm}{second_alarm}"),
    );
    let mapping = subtree(&sources, "ecuc.arxml", "ECUC-CONTAINER-VALUE", "Process");
    change(
        &mut sources,
        "ecuc.arxml",
        &mapping,
        &mapping.replace(
            "/Configuration/Os/Alarm_App</VALUE-REF>",
            "/Configuration/Os/Alarm_Process</VALUE-REF>",
        ),
    );
    if !tables {
        return sources;
    }
    // Process moves to its own table below; its alarm would stay unused.
    change(&mut sources, "ecuc.arxml", &second_alarm, "");
    let table_source = InputSource::new(
        "tables.arxml",
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/os-timing/schedule_tables.arxml"),
        )
        .unwrap(),
    )
    .unwrap();
    let table = subtree(
        &[table_source],
        "tables.arxml",
        "ECUC-CONTAINER-VALUE",
        "Table_App",
    );
    for component in ["Process", "Observe"] {
        let name = format!("Table_{component}");
        let cloned = table.replace(
            "<SHORT-NAME>Table_App</SHORT-NAME>",
            &format!("<SHORT-NAME>{name}</SHORT-NAME>"),
        );
        change(
            &mut sources,
            "ecuc.arxml",
            &alarm,
            &format!("{alarm}{cloned}"),
        );
        let mapping = subtree(&sources, "ecuc.arxml", "ECUC-CONTAINER-VALUE", component);
        let changed = mapping
            .replace("RteUsedOsAlarmRef", "RteUsedOsSchTblExpiryPointRef")
            .replace(
                if component == "Process" {
                    "/Configuration/Os/Alarm_Process</VALUE-REF>"
                } else {
                    "/Configuration/Os/Alarm_App</VALUE-REF>"
                },
                &format!("/Configuration/Os/{name}/Point</VALUE-REF>"),
            );
        change(&mut sources, "ecuc.arxml", &mapping, &changed);
    }
    parameter(
        &mut sources,
        "Table_Observe",
        "OsScheduleTableStartValue",
        "8",
    );
    parameter(
        &mut sources,
        "Table_Observe",
        "OsScheduleTblExpPointOffset",
        "2",
    );
    sources
}

pub(crate) fn subtree(sources: &[InputSource], file: &str, tag: &str, short_name: &str) -> String {
    let source = sources
        .iter()
        .find(|source| source.logical_path() == file)
        .unwrap();
    let text = std::str::from_utf8(source.bytes()).unwrap();
    let document = roxmltree::Document::parse(text).unwrap();
    let node = document
        .descendants()
        .find(|node| {
            node.tag_name().name() == tag
                && node.children().any(|child| {
                    child.tag_name().name() == "SHORT-NAME" && child.text() == Some(short_name)
                })
        })
        .unwrap();
    text[node.range()].to_owned()
}

pub(crate) fn normal_validation(
    sources: &[InputSource],
) -> autosar_config_core::project_model::ScopeValidation {
    let files = sources
        .iter()
        .map(|source| {
            (
                Path::new(source.logical_path()),
                std::str::from_utf8(source.bytes()).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    DefinitionCatalog::builtin()
        .unwrap()
        .validate_documents(&files)
        .unwrap()
}

pub(crate) fn xml_edit(
    sources: &mut [InputSource],
    file: &str,
    select: impl Fn(roxmltree::Node<'_, '_>) -> bool,
    replacement: impl Fn(&str) -> String,
) {
    let source = sources
        .iter_mut()
        .find(|source| source.logical_path() == file)
        .unwrap();
    let text = std::str::from_utf8(source.bytes()).unwrap();
    let document = roxmltree::Document::parse(text).unwrap();
    let range = document
        .descendants()
        .find(|node| select(*node))
        .unwrap()
        .range();
    let changed = format!(
        "{}{}{}",
        &text[..range.start],
        replacement(&text[range.clone()]),
        &text[range.end..]
    );
    *source = InputSource::new(file, changed.into_bytes()).unwrap();
}

pub(crate) fn named(node: roxmltree::Node<'_, '_>, tag: &str, name: &str) -> bool {
    node.tag_name().name() == tag
        && node
            .children()
            .any(|child| child.tag_name().name() == "SHORT-NAME" && child.text() == Some(name))
}

pub(crate) fn parameter(
    sources: &mut [InputSource],
    owner: &str,
    definition: &str,
    new_value: &str,
) {
    xml_edit(
        sources,
        "ecuc.arxml",
        |node| {
            node.tag_name().name() == "VALUE"
                && node.parent().is_some_and(|parent| {
                    parent.children().any(|child| {
                        child.tag_name().name() == "DEFINITION-REF"
                            && child
                                .text()
                                .is_some_and(|text| text.ends_with(&format!("/{definition}")))
                    })
                })
                && node
                    .ancestors()
                    .any(|ancestor| named(ancestor, "ECUC-CONTAINER-VALUE", owner))
        },
        |_| format!("<VALUE>{new_value}</VALUE>"),
    );
}

pub(crate) fn rejects_in_both(sources: &[InputSource], code: &str) {
    let issues = build(sources)
        .err()
        .unwrap_or_else(|| panic!("{code} accepted"));
    assert!(issues.iter().any(|issue| issue.code == code), "{issues:?}");
    let validation = normal_validation(sources);
    assert!(
        validation
            .diagnostics
            .iter()
            .any(|issue| issue.code == code && issue.file.is_some() && issue.path.is_some()),
        "{code}: {:?}",
        validation.diagnostics
    );
}

pub(crate) fn sources_without_application_did(retain_routes: bool) -> Vec<InputSource> {
    let mut sources = inputs();
    for name in [
        "ApplicationValue",
        "ApplicationDid",
        "DcmService",
        "DiagRequest",
        "DiagResponse",
        "DiagRequestSource",
        "DiagRequestDestination",
        "DiagResponseSource",
        "DiagResponseDestination",
    ]
    .into_iter()
    .filter(|name| !retain_routes || !name.starts_with("Diag"))
    {
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                named(node, "ECUC-CONTAINER-VALUE", name)
                    && (!name.starts_with("Diag")
                        || node.children().any(|child| {
                            child.tag_name().name() == "DEFINITION-REF"
                                && child.text().is_some_and(|text| text.contains("/PduR/"))
                        }))
            },
            |_| String::new(),
        );
    }
    xml_edit(
        &mut sources,
        "composition.arxml",
        |node| named(node, "SW-COMPONENT-PROTOTYPE", "DcmService"),
        |_| String::new(),
    );
    xml_edit(
        &mut sources,
        "composition.arxml",
        |node| named(node, "ASSEMBLY-SW-CONNECTOR", "Diagnostic"),
        |_| String::new(),
    );
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| {
            node.tag_name().name() == "COMPONENT-IREF"
                && node
                    .descendants()
                    .any(|child| child.text() == Some("/Application/Pipeline/DcmService"))
        },
        |_| String::new(),
    );
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.has_tag_name("ECUC-INSTANCE-REFERENCE-VALUE")
                && node.descendants().any(|child| {
                    child.has_tag_name("TARGET-REF")
                        && child.text() == Some("/Application/Pipeline/DcmService")
                })
        },
        |_| String::new(),
    );
    sources
}

/// The mainline reference project reduced to its single application
/// component `Ingress`: network endpoints and the DID server remain, every
/// peer component, local connector and instance reference is removed.
pub(crate) fn single_component_sources() -> Vec<InputSource> {
    let mut sources: Vec<_> = inputs()
        .into_iter()
        .filter(|source| !matches!(source.logical_path(), "process.arxml" | "observe.arxml"))
        .collect();
    for instance in ["ProcessInstance", "ObserveInstance"] {
        xml_edit(
            &mut sources,
            "composition.arxml",
            |node| named(node, "SW-COMPONENT-PROTOTYPE", instance),
            |_| String::new(),
        );
        let target = format!("/Application/Pipeline/{instance}");
        xml_edit(
            &mut sources,
            "extract.arxml",
            |node| {
                node.has_tag_name("COMPONENT-IREF")
                    && node
                        .descendants()
                        .any(|child| child.text() == Some(target.as_str()))
            },
            |_| String::new(),
        );
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                node.has_tag_name("ECUC-INSTANCE-REFERENCE-VALUE")
                    && node.descendants().any(|child| {
                        child.has_tag_name("TARGET-REF") && child.text() == Some(target.as_str())
                    })
            },
            |_| String::new(),
        );
    }
    for container in ["Process", "Observe"] {
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                named(node, "ECUC-CONTAINER-VALUE", container)
                    && node.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF")
                            && child
                                .text()
                                .is_some_and(|text| text.ends_with("/RteSwComponentInstance"))
                    })
            },
            |_| String::new(),
        );
    }
    for connector in [
        "IngressProcess",
        "IngressObserve",
        "ProcessObserve",
        "Scalar",
    ] {
        xml_edit(
            &mut sources,
            "composition.arxml",
            |node| named(node, "ASSEMBLY-SW-CONNECTOR", connector),
            |_| String::new(),
        );
    }
    xml_edit(
        &mut sources,
        "ingress.arxml",
        |node| named(node, "P-PORT-PROTOTYPE", "Value"),
        |_| String::new(),
    );
    xml_edit(
        &mut sources,
        "ingress.arxml",
        |node| named(node, "VARIABLE-ACCESS", "PValue"),
        |_| String::new(),
    );
    sources
}

/// Seals the single-component project with the reference echo application:
/// the composition fixture's `Ingress` without its removed local `Value` port.
/// Returns the project directory and its sealed closure.
pub(crate) fn single_component_project(
    root: &Path,
    target: autosar_config_core::target::BuildTarget,
) -> (std::path::PathBuf, Vec<(String, Vec<u8>)>) {
    let plan = build(&single_component_sources()).unwrap_or_else(|issues| panic!("{issues:?}"));
    single_component_project_from(root, target, &plan)
}

pub(crate) fn single_component_project_from(
    root: &Path,
    target: autosar_config_core::target::BuildTarget,
    plan: &ValidatedIntegrationPlan,
) -> (std::path::PathBuf, Vec<(String, Vec<u8>)>) {
    use autosar_config_core::{ApplicationSource, prepare_ecu_project_with_applications};
    let fixture = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/component-pipeline/applications/Ingress.c"),
    )
    .unwrap();
    let source = fixture.replace("        (void)Rte_Write_Value_Value(value);\n", "");
    assert_ne!(source, fixture);
    let application = root.join("Ingress.c");
    std::fs::write(&application, source).unwrap();
    let prepared = prepare_ecu_project_with_applications(
        plan,
        target,
        &[ApplicationSource {
            component_instance: "/Application/Pipeline/IngressInstance".into(),
            path: application,
        }],
    )
    .unwrap_or_else(|issues| panic!("{issues:?}"));
    let project = root.join("project");
    let files = prepared.into_files();
    write_files(&project, &files);
    (project, files)
}

pub(crate) fn write_files(project: &Path, files: &[(String, Vec<u8>)]) {
    for (path, bytes) in files {
        let path = project.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
}

pub(crate) fn pipeline_operation_inputs() -> Vec<InputSource> {
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "types.arxml",
        |node| named(node, "CLIENT-SERVER-OPERATION", "Transform"),
        |original| {
            format!(
                "{original}<CLIENT-SERVER-OPERATION><SHORT-NAME>Ping</SHORT-NAME></CLIENT-SERVER-OPERATION>"
            )
        },
    );
    for (file, tag) in [
        ("process.arxml", "SERVER-COM-SPEC"),
        ("observe.arxml", "CLIENT-COM-SPEC"),
    ] {
        xml_edit(
            &mut sources,
            file,
            |node| node.tag_name().name() == tag,
            |original| format!("{original}{}", original.replace("Transform", "Ping")),
        );
    }
    for (file, tag, name) in [
        (
            "process.arxml",
            "OPERATION-INVOKED-EVENT",
            "InvokeTransform",
        ),
        ("process.arxml", "RUNNABLE-ENTITY", "ServerTransform"),
        (
            "observe.arxml",
            "SYNCHRONOUS-SERVER-CALL-POINT",
            "CallTransform",
        ),
    ] {
        xml_edit(
            &mut sources,
            file,
            |node| named(node, tag, name),
            |original| format!("{original}{}", original.replace("Transform", "Ping")),
        );
    }
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "ReadApplicationValue")
                && node
                    .ancestors()
                    .any(|ancestor| named(ancestor, "ECUC-CONTAINER-VALUE", "Process"))
        },
        |original| {
            format!(
                "{original}{}",
                original
                    .replace("ReadApplicationValue", "InvokePing")
                    .replace("InvokeTransform", "InvokePing")
            )
        },
    );
    sources
}

pub(crate) fn template_inputs() -> Vec<InputSource> {
    let scratch = Scratch::new();
    let preview = autosar_config_core::Workspace::preview_project_creation(
        &scratch.0.join("template"),
        "Template",
        "standard-ecu",
    )
    .unwrap();
    let mut sources: Vec<_> = preview
        .files
        .into_iter()
        .filter(|file| file.path.ends_with(".arxml"))
        .map(|file| InputSource::new(file.path, file.contents.into_bytes()).unwrap())
        .collect();
    sources.sort_by(|left, right| left.logical_path().cmp(right.logical_path()));
    sources
}

pub(crate) fn template_execution_variants() -> Vec<Vec<InputSource>> {
    let mut result = Vec::new();
    let mut sources = template_inputs();
    change(
        &mut sources,
        "application.arxml",
        "<PERIOD>0.01</PERIOD>",
        "<OFFSET>0.001</OFFSET><PERIOD>0.01</PERIOD>",
    );
    result.push(sources);
    for file in ["application.arxml", "services.arxml"] {
        let source = template_inputs();
        let text = std::str::from_utf8(
            source
                .iter()
                .find(|source| source.logical_path() == file)
                .unwrap()
                .bytes(),
        )
        .unwrap();
        let names: Vec<_> = roxmltree::Document::parse(text)
            .unwrap()
            .descendants()
            .filter(|node| node.tag_name().name() == "RUNNABLE-ENTITY")
            .map(|node| {
                node.children()
                    .find(|child| child.tag_name().name() == "SHORT-NAME")
                    .unwrap()
                    .text()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        for name in names {
            let mut sources = template_inputs();
            xml_edit(
                &mut sources,
                file,
                |node| named(node, "RUNNABLE-ENTITY", &name),
                |original| {
                    original.replace("<CAN-BE-INVOKED-CONCURRENTLY>", "<MINIMUM-START-INTERVAL>0.001</MINIMUM-START-INTERVAL><CAN-BE-INVOKED-CONCURRENTLY>")
                },
            );
            result.push(sources);
        }
    }
    result
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
pub(crate) fn verify_production_owner(
    project: &Path,
    consumer: &Path,
    p2_star: Option<(&str, &str)>,
) {
    let output = consumer.join("pipeline-build");
    let binary = super::sealed_builds::host_batch(project, &output);
    verify_production_owner_binary(&binary, consumer, p2_star);
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
pub(crate) fn verify_production_owner_binary(
    binary: &Path,
    consumer: &Path,
    p2_star: Option<(&str, &str)>,
) {
    let logs = consumer.join("owner-logs");
    std::fs::create_dir(&logs).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let mut spec = autosar_config_core::execution::ProcessSpec::for_duration(
        vec![binary.as_os_str().to_owned()],
        consumer.to_path_buf(),
        vec![],
        std::time::Duration::from_secs(30),
        logs,
    )
    .unwrap();
    spec.stdin_stream = true;
    let owner = tooling::execution_owner();
    let mut process = owner.spawn(spec, None).unwrap();
    process.write_stdin(b"BEGIN 0\nRX 800 4 78563412\nRX 1792 8 0322123400000000\nCOMMIT\nBEGIN 1\nCOMMIT\nBEGIN 10\nRX 1792 8 0322123400000000\nCOMMIT\nBEGIN 10\nCOMMIT\nBEGIN 20\nRX 1792 8 0322123400000000\nCOMMIT\n").unwrap();
    if p2_star.is_some() {
        process
            .write_stdin(b"BEGIN 30\nRX 1792 8 0210030000000000\nCOMMIT\n")
            .unwrap();
    }
    process.close_stdin();
    let result = tooling::wait_success(process, "consumer");
    let text = std::fs::read_to_string(&result.stdout).unwrap();
    let records: Vec<_> = text
        .lines()
        .filter(|line| line.starts_with("OUT "))
        .map(|line| {
            let fields: std::collections::BTreeMap<_, _> = line
                .split_whitespace()
                .skip(1)
                .map(|field| field.split_once('=').unwrap())
                .collect();
            (
                fields["epoch"].parse::<u64>().unwrap(),
                fields["id"].parse::<u32>().unwrap(),
                fields["dlc"].parse::<u8>().unwrap(),
                fields["data"].to_string(),
            )
        })
        .collect();
    let mut expected = vec![
        (1, 1800, 8, "0762123400000000".into()),
        (10, 801, 4, "78563412".into()),
        (10, 1800, 8, "0762123412345678".into()),
        (20, 801, 4, "78563412".into()),
        (20, 1800, 8, "0762123412345678".into()),
    ];
    if let Some((_, wire)) = p2_star {
        expected.push((30, 801, 4, "78563412".into()));
        expected.push((30, 1800, 8, format!("0650030032{wire}00")));
    }
    assert_eq!(records, expected, "{text}");
    let receipts: Vec<_> = text
        .lines()
        .filter(|line| line.starts_with("COMMIT_OK "))
        .map(|line| {
            line.split_whitespace()
                .find_map(|field| field.strip_prefix("epoch="))
                .unwrap()
                .parse::<u64>()
                .unwrap()
        })
        .collect();
    let mut expected_receipts = vec![0, 1, 10, 10, 20];
    if p2_star.is_some() {
        expected_receipts.push(30);
    }
    assert_eq!(receipts, expected_receipts, "{text}");
    assert!(
        !text
            .lines()
            .any(|line| line.starts_with("REJECT ") || line.starts_with("COMMIT_ERROR ")),
        "{text}"
    );
    assert!(
        std::fs::read(&result.stderr).unwrap().is_empty(),
        "{result:?}"
    );
}

pub(crate) fn native_delivery_target() -> autosar_config_core::target::BuildTarget {
    if cfg!(windows) {
        autosar_config_core::target::BuildTarget::WindowsX64ControlledV1
    } else {
        autosar_config_core::target::BuildTarget::LinuxX64ControlledV1
    }
}

/// Handoff forgeries with the refusal reasons of the workbench importer and
/// of the delivered `ecu-tool.py`.
pub(crate) const HANDOFF_FORGERIES: [(&str, &str, &str); 9] = [
    (
        "application",
        "mapped_snapshot_changed",
        "Immutable native product/generated/input bytes changed",
    ),
    (
        "owner",
        "unknown variant",
        "Unknown native owner or payload path",
    ),
    (
        "producer",
        "ownership_write_authority_invalid",
        "Ownership cannot grant this path write authority",
    ),
    (
        "snapshot-path",
        "ownership_write_authority_invalid",
        "Ownership cannot grant this path write authority",
    ),
    (
        "payload-path",
        "portable_path_unsafe",
        "Unsafe native portable path",
    ),
    (
        "identity",
        "native_identity_unsupported",
        "Native input/resource identity differs",
    ),
    // A package sealed in the retired workbench handoff format.
    (
        "format",
        "retired_handoff_format",
        "Native input/resource identity differs",
    ),
    // An ownership ledger of an earlier format version.
    (
        "ledger-version",
        "ownership_identity_invalid",
        "Unknown ownership format version",
    ),
    // A member manifest of an earlier format version whose policy digest is forged too.
    (
        "manifest-version",
        "format_version_unsupported",
        "Unknown native member manifest version",
    ),
];

/// Generates `case` of [`HANDOFF_FORGERIES`] under `scratch` and reseals it.
pub(crate) fn forged_handoff(
    case: &str,
    workspace: &autosar_config_core::Workspace,
    plan: &ValidatedIntegrationPlan,
    scratch: &Path,
) -> std::path::PathBuf {
    use autosar_config_core::prepared::prepare_ecu_project_for_workspace;
    let package = scratch.join(case);
    let prepared =
        prepare_ecu_project_for_workspace(workspace, plan, native_delivery_target(), true).unwrap();
    prepared.generate(&package).unwrap();
    let ledger_path = package.join("meta/workbench-ownership.json");
    let mut ledger: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&ledger_path).unwrap()).unwrap();
    let row = ledger["files"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["path"] == "app/Process/Process.c")
        .unwrap();
    match case {
        "application" => {
            let mut bytes = std::fs::read(package.join("app/Process/Process.c")).unwrap();
            bytes.extend_from_slice(b"\n/* forged snapshot */\n");
            std::fs::write(package.join("app/Process/Process.c"), bytes).unwrap();
        }
        "owner" => row["owner"] = "unrecognized-authority".into(),
        "producer" => row["producerId"] = "unrecognized-producer".into(),
        "snapshot-path" => row["snapshotOf"] = "../outside.c".into(),
        "payload-path" => row["path"] = "../outside.c".into(),
        "ledger-version" => ledger["formatVersion"] = 2.into(),
        "manifest-version" => {
            use sha2::Digest;
            let path = package.join("inputs/workbench-project.json");
            let mut manifest: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            manifest["formatVersion"] = 2.into();
            let bytes = serde_json::to_vec_pretty(&manifest).unwrap();
            std::fs::write(&path, &bytes).unwrap();
            let policy_path = package.join("meta/seal-policy.json");
            let mut policy: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&policy_path).unwrap()).unwrap();
            policy["payload"]["inputs/workbench-project.json"] =
                hex::encode(sha2::Sha256::digest(&bytes)).into();
            std::fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
            let manifest_row = ledger["files"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|row| row["path"] == "inputs/workbench-project.json")
                .unwrap();
            manifest_row["sha256"] = hex::encode(sha2::Sha256::digest(&bytes)).into();
        }
        "identity" | "format" => {
            let mut target: serde_json::Value =
                serde_json::from_slice(&std::fs::read(package.join("target.json")).unwrap())
                    .unwrap();
            if case == "format" {
                target["nativeDelivery"]["format"] = "autosar-workbench-handoff-v2".into();
            } else {
                target["nativeDelivery"]["resourceIdentities"]["ruleSetIdentity"]["rulesVersion"] =
                    "unrecognized-rule-version".into();
            }
            std::fs::write(
                package.join("meta/handoff.json"),
                serde_json::to_vec_pretty(&target["nativeDelivery"]).unwrap(),
            )
            .unwrap();
            std::fs::write(
                package.join("target.json"),
                serde_json::to_vec_pretty(&target).unwrap(),
            )
            .unwrap();
        }
        _ => unreachable!(),
    }
    ledger["files"]
        .as_array_mut()
        .unwrap()
        .sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
    std::fs::write(&ledger_path, serde_json::to_vec_pretty(&ledger).unwrap()).unwrap();
    if case == "payload-path" {
        packages::seal(&package);
    } else {
        packages::reseal(&package);
    }
    package
}
