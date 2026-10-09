use autosar_config_core::{
    definitions::DefinitionCatalog,
    integration::{InputSource, RuntimeCatalog, ValidatedIntegrationPlan, build_plan_native},
};
use std::path::Path;

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[allow(dead_code)]
#[path = "support/tooling.rs"]
mod tooling;
#[allow(dead_code)]
#[path = "support/workspace.rs"]
mod workspace;
use workspace::Scratch;

fn inputs() -> Vec<InputSource> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi-component");
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
fn build(
    sources: &[InputSource],
) -> Result<ValidatedIntegrationPlan, Vec<autosar_config_core::integration::PlanDiagnostic>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    build_plan_native(
        sources,
        &DefinitionCatalog::builtin().unwrap(),
        &RuntimeCatalog::from_repository(root).unwrap(),
    )
}
fn change(sources: &mut [InputSource], file: &str, before: &str, after: &str) {
    let source = sources
        .iter_mut()
        .find(|source| source.logical_path() == file)
        .unwrap();
    let text = std::str::from_utf8(source.bytes()).unwrap();
    assert!(text.contains(before), "{file}: {before}");
    *source = InputSource::new(file, text.replacen(before, after, 1).into_bytes()).unwrap();
}

fn live_multi_workspace(root: &Path, sources: &[InputSource]) -> autosar_config_core::Workspace {
    std::fs::create_dir_all(root).unwrap();
    for source in sources {
        std::fs::write(root.join(source.logical_path()), source.bytes()).unwrap();
    }
    let manifest = autosar_config_core::arxml::ProjectManifest {
        format_version: 1,
        declared_release: "R24-11".into(),
        profile_hint: "singlecore-multi-swc-v1".into(),
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

#[test]
fn multi_workspace_initialization_reopen_and_regeneration_preserve_every_user_source() {
    use autosar_config_core::{
        Workspace, prepared::prepare_ecu_project_for_workspace, target::BuildTarget,
    };
    for renamed in [false, true] {
        let scratch = Scratch::new();
        let root = scratch.0.join("live");
        let mut arxml = inputs();
        if renamed {
            for (before, after) in [
                ("Process</", "Compute</"),
                ("Process/", "Compute/"),
                ("Process_", "Compute_"),
                ("ProcessInstance", "ComputeInstance"),
                ("ResultService", "Calculation"),
                ("Transform", "Calculate"),
            ] {
                replace_all(&mut arxml, before, after);
            }
        }
        let mut workspace = live_multi_workspace(&root, &arxml);
        let preview = workspace.preview_application_initialization().unwrap();
        assert_eq!(preview.slots.len(), 3);
        assert_eq!(preview.files.len(), 3);
        let mut stale = preview.clone();
        stale.slots[1].component_path.push_str("/untrusted");
        assert!(workspace.initialize_application_previewed(&stale).is_err());
        assert!(
            preview
                .files
                .iter()
                .all(|file| !root.join(&file.path).exists())
        );
        for path in [
            root.join("workbench-project.json"),
            root.join(arxml[0].logical_path()),
        ] {
            let before = std::fs::read(&path).unwrap();
            let mut edited = before.clone();
            edited.push(b'\n');
            std::fs::write(&path, &edited).unwrap();
            assert!(
                workspace
                    .initialize_application_previewed(&preview)
                    .is_err()
            );
            assert_eq!(std::fs::read(&path).unwrap(), edited);
            assert!(
                preview
                    .files
                    .iter()
                    .all(|file| !root.join(&file.path).exists())
            );
            std::fs::write(&path, before).unwrap();
        }
        let outcome = workspace
            .initialize_application_previewed(&preview)
            .unwrap();
        assert!(outcome.warnings.is_empty());
        assert!(outcome.retained_recovery_files.is_empty());
        let mut user_sources = std::collections::BTreeMap::new();
        for (index, file) in preview.files.iter().enumerate() {
            let mut bytes = file.contents.as_bytes().to_vec();
            bytes.extend_from_slice(
                format!("\r\n/* User edit {index}: 用户源码. */\r\n").as_bytes(),
            );
            std::fs::write(root.join(&file.path), &bytes).unwrap();
            user_sources.insert(file.path.clone(), bytes);
        }
        assert!(workspace.generation_snapshot().is_err());
        let workspace = Workspace::open_project_manifest(
            &root.join("workbench-project.json"),
            &root.join("cache"),
        )
        .unwrap();
        let runtime = RuntimeCatalog::embedded().unwrap();
        let plan = workspace.saved_integration_plan(&runtime).unwrap();
        let output = scratch.0.join("sealed");
        let prepared = || {
            prepare_ecu_project_for_workspace(
                &workspace,
                &plan,
                BuildTarget::LinuxX64ControlledV1,
                true,
            )
            .unwrap()
        };
        let generation = prepared().preview(&output).unwrap();
        prepared()
            .generate_previewed(&output, &generation.revision)
            .unwrap();
        for (path, bytes) in &user_sources {
            assert_eq!(&std::fs::read(root.join(path)).unwrap(), bytes);
            assert_eq!(
                &std::fs::read(output.join(path.replace("application/", "src/"))).unwrap(),
                bytes
            );
        }
        for source in &arxml {
            assert_eq!(
                std::fs::read(root.join(source.logical_path())).unwrap(),
                source.bytes()
            );
        }
        let original_output = std::fs::read(output.join("files.sha256")).unwrap();
        let old_preview = prepared().preview(&output).unwrap();
        let mut guarded = user_sources
            .iter()
            .map(|(path, bytes)| (root.join(path), bytes.clone()))
            .collect::<Vec<_>>();
        guarded.push((
            root.join("workbench-project.json"),
            std::fs::read(root.join("workbench-project.json")).unwrap(),
        ));
        guarded.push((
            root.join(arxml[0].logical_path()),
            arxml[0].bytes().to_vec(),
        ));
        for (path, bytes) in guarded {
            let pending = prepared();
            let mut changed = bytes.clone();
            changed.push(b'\n');
            std::fs::write(&path, &changed).unwrap();
            assert!(
                pending
                    .generate_previewed(&output, &old_preview.revision)
                    .is_err()
            );
            assert_eq!(std::fs::read(&path).unwrap(), changed);
            assert_eq!(
                std::fs::read(output.join("files.sha256")).unwrap(),
                original_output
            );
            for (source, bytes) in &user_sources {
                assert_eq!(
                    &std::fs::read(output.join(source.replace("application/", "src/"))).unwrap(),
                    bytes
                );
            }
            std::fs::write(&path, bytes).unwrap();
        }
    }
}

#[test]
fn multi_workspace_reopen_rejects_incomplete_unknown_and_wrong_slot_membership() {
    use autosar_config_core::Workspace;
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut workspace = live_multi_workspace(&root, &inputs());
    let preview = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&preview)
        .unwrap();
    let original = workspace.project_manifest().unwrap().clone();
    for mutation in 0..5 {
        let mut manifest = original.clone();
        match mutation {
            0 => {
                manifest.application_inputs.pop();
            }
            1 => manifest.application_inputs[0]
                .producer_slot
                .push_str("/unknown"),
            2 => manifest.application_inputs.swap(0, 1),
            3 => manifest
                .application_inputs
                .push(manifest.application_inputs[0].clone()),
            4 => manifest.application_inputs[0].path = "application/Unknown.c".into(),
            _ => unreachable!(),
        }
        if mutation == 2 {
            let slot = manifest.application_inputs[0].producer_slot.clone();
            manifest.application_inputs[0].producer_slot =
                manifest.application_inputs[1].producer_slot.clone();
            manifest.application_inputs[1].producer_slot = slot;
        }
        if mutation == 4 {
            std::fs::write(root.join("application/Unknown.c"), "/* unknown */").unwrap();
        }
        let path = root.join("workbench-project.json");
        std::fs::write(&path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        assert!(
            Workspace::open_project_manifest(&path, &root.join("cache")).is_err(),
            "mutation {mutation}"
        );
    }
    std::fs::write(
        root.join("workbench-project.json"),
        serde_json::to_vec_pretty(&original).unwrap(),
    )
    .unwrap();
    // Ownership remains structurally valid while an unrelated target restriction blocks generation.
    let ecuc_path = root.join("ecuc.arxml");
    let ecuc = std::fs::read_to_string(&ecuc_path).unwrap();
    let changed = ecuc.replacen("<VALUE>IMMEDIATE</VALUE>", "<VALUE>DEFERRED</VALUE>", 1);
    assert_ne!(ecuc, changed);
    std::fs::write(&ecuc_path, changed).unwrap();
    let reopened =
        Workspace::open_project_manifest(&root.join("workbench-project.json"), &root.join("cache"))
            .unwrap();
    assert!(
        reopened
            .saved_integration_plan(&RuntimeCatalog::embedded().unwrap())
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn multi_workspace_initialization_and_reopen_refuse_symlink_members() {
    use autosar_config_core::{Workspace, prepared::prepare_ecu_project_for_workspace};
    use std::os::unix::fs::symlink;
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut workspace = live_multi_workspace(&root, &inputs());
    let preview = workspace.preview_application_initialization().unwrap();
    let source = root.join(&preview.files[1].path);
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    let external = scratch.0.join("external.c");
    let bytes = b"/* external user source */\n";
    std::fs::write(&external, bytes).unwrap();
    symlink(&external, &source).unwrap();
    assert!(
        workspace
            .initialize_application_previewed(&preview)
            .is_err()
    );
    assert_eq!(std::fs::read(&external).unwrap(), bytes);
    assert!(
        preview
            .files
            .iter()
            .filter(|file| root.join(&file.path) != source)
            .all(|file| !root.join(&file.path).exists())
    );
    std::fs::remove_file(&source).unwrap();
    workspace
        .initialize_application_previewed(&preview)
        .unwrap();
    let plan = workspace
        .saved_integration_plan(&RuntimeCatalog::embedded().unwrap())
        .unwrap();
    let prepared = prepare_ecu_project_for_workspace(
        &workspace,
        &plan,
        autosar_config_core::target::BuildTarget::LinuxX64ControlledV1,
        true,
    )
    .unwrap();
    std::fs::remove_file(&source).unwrap();
    symlink(&external, &source).unwrap();
    assert!(
        Workspace::open_project_manifest(&root.join("workbench-project.json"), &root.join("cache"))
            .is_err()
    );
    let output = scratch.0.join("sealed");
    assert!(prepared.generate(&output).is_err());
    assert!(!output.exists());
    assert_eq!(std::fs::read(&external).unwrap(), bytes);
}

#[test]
fn multi_workspace_missing_or_directory_source_refuses_reopen_and_prepared_generation() {
    use autosar_config_core::{
        Workspace, prepared::prepare_ecu_project_for_workspace, target::BuildTarget,
    };
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut workspace = live_multi_workspace(&root, &inputs());
    let initialization = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&initialization)
        .unwrap();
    let plan = workspace
        .saved_integration_plan(&RuntimeCatalog::embedded().unwrap())
        .unwrap();
    let prepare = || {
        prepare_ecu_project_for_workspace(
            &workspace,
            &plan,
            BuildTarget::LinuxX64ControlledV1,
            true,
        )
        .unwrap()
    };
    let output = scratch.0.join("sealed");
    let report = prepare().generate(&output).unwrap();
    let sealed_before = report
        .files
        .iter()
        .map(|file| (file.clone(), std::fs::read(output.join(file)).unwrap()))
        .collect::<Vec<_>>();
    let user_before = initialization
        .files
        .iter()
        .map(|file| {
            (
                file.path.clone(),
                std::fs::read(root.join(&file.path)).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let manifest = root.join("workbench-project.json");
    let manifest_before = std::fs::read(&manifest).unwrap();
    let source = root.join(&user_before[1].0);
    for directory in [false, true] {
        let pending = prepare();
        std::fs::remove_file(&source).unwrap();
        if directory {
            std::fs::create_dir(&source).unwrap();
            std::fs::write(source.join("external"), b"external directory contents").unwrap();
        }
        assert!(Workspace::open_project_manifest(&manifest, &root.join("cache")).is_err());
        assert!(pending.generate(&output).is_err());
        assert_eq!(std::fs::read(&manifest).unwrap(), manifest_before);
        for (path, bytes) in user_before
            .iter()
            .filter(|(path, _)| root.join(path) != source)
        {
            assert_eq!(&std::fs::read(root.join(path)).unwrap(), bytes);
        }
        for (path, bytes) in &sealed_before {
            assert_eq!(&std::fs::read(output.join(path)).unwrap(), bytes);
        }
        if directory {
            assert_eq!(
                std::fs::read(source.join("external")).unwrap(),
                b"external directory contents"
            );
            std::fs::remove_file(source.join("external")).unwrap();
            std::fs::remove_dir(&source).unwrap();
        } else {
            assert!(!source.exists());
        }
        std::fs::write(&source, &user_before[1].1).unwrap();
    }
}

#[test]
fn multi_workspace_ownership_rejects_existing_foreign_signal_mapping_member() {
    use autosar_config_core::Workspace;
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut arxml = inputs();
    change(
        &mut arxml,
        "types.arxml",
        "</SENDER-RECEIVER-INTERFACE>",
        r#"</SENDER-RECEIVER-INTERFACE>
        <SENDER-RECEIVER-INTERFACE>
          <SHORT-NAME>ForeignInterface</SHORT-NAME>
          <IS-SERVICE>false</IS-SERVICE>
          <DATA-ELEMENTS>
            <VARIABLE-DATA-PROTOTYPE>
              <SHORT-NAME>ForeignValue</SHORT-NAME>
              <TYPE-TREF DEST="APPLICATION-PRIMITIVE-DATA-TYPE">/Types/ApplicationUint32</TYPE-TREF>
            </VARIABLE-DATA-PROTOTYPE>
          </DATA-ELEMENTS>
        </SENDER-RECEIVER-INTERFACE>"#,
    );
    let mut workspace = live_multi_workspace(&root, &arxml);
    let initialization = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&initialization)
        .unwrap();
    let extract = root.join("extract.arxml");
    let before = std::fs::read_to_string(&extract).unwrap();
    let member = r#"<TARGET-DATA-PROTOTYPE-REF DEST="VARIABLE-DATA-PROTOTYPE">/Types/ValueInterface/Value</TARGET-DATA-PROTOTYPE-REF>"#;
    assert!(before.contains(member));
    std::fs::write(&extract, before.replacen(member,
        r#"<TARGET-DATA-PROTOTYPE-REF DEST="VARIABLE-DATA-PROTOTYPE">/Types/ForeignInterface/ForeignValue</TARGET-DATA-PROTOTYPE-REF>"#, 1)).unwrap();
    let paths = arxml
        .iter()
        .map(|source| root.join(source.logical_path()))
        .chain(
            initialization
                .files
                .iter()
                .map(|file| root.join(&file.path)),
        )
        .chain(std::iter::once(root.join("workbench-project.json")));
    let bytes_before = paths
        .map(|path| {
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect::<Vec<_>>();
    let error =
        Workspace::open_project_manifest(&root.join("workbench-project.json"), &root.join("cache"))
            .err()
            .unwrap();
    assert!(error.to_string().contains("SIGNAL_MAPPING"), "{error}");
    assert!(
        !error.to_string().contains("REFERENCE_UNRESOLVED"),
        "{error}"
    );
    for (path, bytes) in bytes_before {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}

// Consume the normal complete producer rather than a public source-fragment API.
fn prepared_multi_files(plan: &ValidatedIntegrationPlan) -> Vec<(String, Vec<u8>)> {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi-application");
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
                    .strip_prefix("singlecore-multi-swc-v1:")
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

#[test]
fn source_derived_multi_contract_is_deterministic_and_keeps_local_identity() {
    let sources = inputs();
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    let description = plan.description();
    let multi = description.multi.as_ref().unwrap();
    assert_eq!(description.profile, "singlecore-multi-swc-v1");
    assert!(description.component.is_none());
    assert_eq!(multi.components.len(), 4);
    assert_eq!(multi.connections.len(), 5);
    assert_eq!(multi.network_endpoints.len(), 2);
    assert_eq!(
        description
            .schedule
            .entities
            .iter()
            .filter(|entity| entity.application)
            .map(|entity| entity.symbol.as_str())
            .collect::<Vec<_>>(),
        ["Ingress_Periodic", "Process_Periodic", "Observe_Periodic"]
    );
    for component in &multi.components {
        assert_ne!(component.component, component.instance);
        assert!(component.rte_instance.starts_with("/Configuration/Rte/"));
        if component.component.ends_with("Process") || component.component.ends_with("Observe") {
            assert!(
                !multi
                    .network_endpoints
                    .iter()
                    .any(|endpoint| endpoint.endpoint.instance == component.instance)
            );
        }
    }
    let read_values = multi
        .components
        .iter()
        .flat_map(|component| &component.data_ports)
        .filter(|port| port.name == "Value" && port.read)
        .map(|port| port.initial_value)
        .collect::<Vec<_>>();
    assert_eq!(read_values, [7, 9]);
    let server = description
        .symbols
        .iter()
        .find(|symbol| symbol.symbol == "Process_Transform")
        .unwrap();
    assert_eq!(server.return_type, "void");
    assert_eq!(
        server
            .arguments
            .iter()
            .map(|argument| (argument.native_type.as_str(), argument.direction.as_str()))
            .collect::<Vec<_>>(),
        [("uint32", "IN"), ("uint32 *", "OUT"), ("uint32 *", "INOUT")]
    );
    let files = plan.component_contract_files().unwrap();
    let mut reversed = sources.clone();
    reversed.reverse();
    assert_eq!(
        files.files(),
        build(&reversed)
            .unwrap()
            .component_contract_files()
            .unwrap()
            .files()
    );
    assert!(description.legacy_component().is_err());
    assert!(
        plan.ecu_integration_files(autosar_config_core::target::BuildTarget::LinuxX64ControlledV1)
            .is_err()
    );
    if let Ok(directory) = std::env::var("AUTOSAR_MULTI_CONTRACT_OUTPUT") {
        let output = Path::new(&directory);
        let preview = files.preview(output).unwrap();
        files.generate_previewed(output, &preview.revision).unwrap();
    }
}

#[test]
fn multi_com_plan_keeps_real_group_timebase_and_notification_identity() {
    let plan = build(&inputs()).unwrap();
    let com = plan.description().com_runtime.as_ref().unwrap();
    assert_eq!(com.callback_header, "Rte_Com.h");
    assert_eq!(com.receive_group.path, "/Configuration/Com/Config/RxGroup");
    assert_eq!(com.receive_group.handle, 0);
    assert_eq!(
        com.receive_group.members,
        ["/Configuration/Com/Config/RxValuePdu"]
    );
    assert_eq!(com.receive_main.symbol, "Com_MainFunctionRx_Rx");
    assert_eq!(com.receive_main.period_ms, 1);
    assert_eq!(com.transmit_main.symbol, "Com_MainFunctionTx_Tx");
    assert_eq!(com.transmit_main.period_ms, 10);
    let reception = &com.receptions[0];
    assert_eq!(reception.signal, "/Configuration/Com/Config/RxValue");
    assert_eq!(
        reception.user_signal,
        "/Configuration/Rte/ComUser/Callbacks/RxValue"
    );
    assert_eq!(reception.callback_handle, 17);
    assert_eq!(reception.first_timeout_ms, 0);
    assert_eq!(reception.timeout_ms, 30);
    assert_eq!(reception.receive_callback, "Rte_COMCbk");
    assert_eq!(reception.timeout_callback, "Rte_COMCbkRxTOut");
    let mut zero = inputs();
    xml_edit(
        &mut zero,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|text| text.ends_with("/ComFirstTimeout"))
            })
        },
        |text| text.replace("<VALUE>0</VALUE>", "<VALUE>0.0e3</VALUE>"),
    );
    assert_eq!(
        build(&zero)
            .unwrap()
            .description()
            .com_runtime
            .as_ref()
            .unwrap()
            .receptions[0]
            .first_timeout_ms,
        0
    );
    let mut renamed = inputs();
    replace_all(
        &mut renamed,
        "/Com/Config/RxGroup",
        "/Com/Config/ReceptionGroup",
    );
    xml_edit(
        &mut renamed,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "RxGroup"),
        |text| {
            text.replace(
                "<SHORT-NAME>RxGroup</SHORT-NAME>",
                "<SHORT-NAME>ReceptionGroup</SHORT-NAME>",
            )
        },
    );
    assert_eq!(
        build(&renamed)
            .unwrap()
            .description()
            .com_runtime
            .as_ref()
            .unwrap()
            .receive_group
            .path,
        "/Configuration/Com/Config/ReceptionGroup"
    );
}

#[test]
fn multi_com_rejects_tx_main_period_different_from_periodic_pdu() {
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "Transmit10ms"),
        |text| {
            text.replace("Alarm_App", "Alarm_Work")
                .replace("Ev_App", "Ev_Work")
        },
    );
    xml_edit(
        &mut sources,
        "bsw.arxml",
        |node| named(node, "BSW-TIMING-EVENT", "Com_MainFunctionTx_Tx_10ms"),
        |text| text.replace("<PERIOD>0.01</PERIOD>", "<PERIOD>0.001</PERIOD>"),
    );
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|value| value.ends_with("/ComMainTxTimeBase"))
            })
        },
        |text| text.replace("<VALUE>0.01</VALUE>", "<VALUE>0.001</VALUE>"),
    );
    rejects_in_both(&sources, "COM_TIMEBASE");
}

#[test]
fn multi_com_control_configuration_rejects_inconsistent_group_timebase_and_callbacks() {
    for (field, original, replacement, code) in [
        ("ComSupportedIPduGroups", "1", "0", "COM_RX_GROUP"),
        ("ComIPduGroupHandleId", "0", "1", "COM_RX_GROUP"),
        ("ComMainRxTimeBase", "0.001", "0.002", "COM_TIMEBASE"),
        ("ComMainTxTimeBase", "0.01", "0.001", "COM_TIMEBASE"),
        (
            "ComUserHeaderInclude",
            "Rte_Com.h",
            "Wrong.h",
            "COM_CALLBACK",
        ),
        ("ComUserCallbackName", "Rte_COMCbk", "Wrong", "COM_CALLBACK"),
        (
            "ComUserCallbackType",
            "COM_RX_ACK",
            "COM_TX_ACK",
            "COM_CALLBACK",
        ),
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with(&format!("/{field}")))
                })
            },
            |text| {
                text.replace(
                    &format!("<VALUE>{original}</VALUE>"),
                    &format!("<VALUE>{replacement}</VALUE>"),
                )
            },
        );
        rejects_in_both(&sources, code);
    }
    for (field, code) in [
        ("ComIPduGroupRef", "COM_RX_GROUP"),
        ("ComIPduMainFunctionRef", "COM_RX_GROUP"),
        ("ComUserCallbackRef", "COM_CALLBACK"),
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with(&format!("/{field}")))
                })
            },
            |_| String::new(),
        );
        rejects_in_both(&sources, code);
    }
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.has_tag_name("PARAMETER-VALUES")
                && node.parent().is_some_and(|parent| {
                    parent.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF")
                            && child
                                .text()
                                .is_some_and(|text| text.ends_with("/ComUserSignal"))
                    })
                })
        },
        |_| String::new(),
    );
    rejects_in_both(&sources, "COM_CALLBACK");
    // A live target exists, so rejection proves direction/ownership rather than a dangling ref.
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|text| text.ends_with("/ComIPduGroupRef"))
            })
        },
        |text| {
            text.replace(
                "/Configuration/Com/Config/RxGroup",
                "/Configuration/Com/Config/TxValuePdu",
            )
        },
    );
    rejects_in_both(&sources, "COM_RX_GROUP");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|text| text.ends_with("/ComUserCallbackRef"))
            })
        },
        |text| format!("{text}{text}"),
    );
    rejects_in_both(&sources, "COM_CALLBACK");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "RxGroup"),
        |text| {
            format!(
                "{text}{}",
                text.replace(
                    "<SHORT-NAME>RxGroup</SHORT-NAME>",
                    "<SHORT-NAME>DuplicateGroup</SHORT-NAME>"
                )
            )
        },
    );
    rejects_in_both(&sources, "COM_RX_GROUP");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|text| text.ends_with("/ComIPduGroupRef"))
            })
        },
        |text| format!("{text}{text}"),
    );
    rejects_in_both(&sources, "COM_RX_GROUP");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "TxValuePdu")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text().is_some_and(|text| text.ends_with("/ComIPdu"))
                })
        },
        |text| {
            text.replacen("<REFERENCE-VALUES>", r#"<REFERENCE-VALUES><ECUC-REFERENCE-VALUE><DEFINITION-REF DEST="ECUC-REFERENCE-DEF">/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu/ComIPduGroupRef</DEFINITION-REF><VALUE-REF DEST="ECUC-CONTAINER-VALUE">/Configuration/Com/Config/RxGroup</VALUE-REF></ECUC-REFERENCE-VALUE>"#, 1)
        },
    );
    rejects_in_both(&sources, "COM_RX_GROUP");
}

#[test]
fn malformed_multi_contracts_are_rejected_at_real_source_objects() {
    let vectors = [
        (
            "composition.arxml",
            "/Application/Process/Value</TARGET-R-PORT-REF>",
            "/Application/Process/Result</TARGET-R-PORT-REF>",
            "direction",
        ),
        (
            "composition.arxml",
            "/Application/Process/Value</TARGET-R-PORT-REF>",
            "/Application/Observe/Value</TARGET-R-PORT-REF>",
            "ownership",
        ),
        (
            "composition.arxml",
            "/Application/Process/Value</TARGET-R-PORT-REF>",
            "/Application/Missing/Value</TARGET-R-PORT-REF>",
            "dangling",
        ),
        (
            "composition.arxml",
            "<SHORT-NAME>ProcessInstance</SHORT-NAME>",
            "<SHORT-NAME>IngressInstance</SHORT-NAME>",
            "duplicate instance",
        ),
        (
            "process.arxml",
            "<SHORT-NAME>Process</SHORT-NAME>",
            "<SHORT-NAME>ingress</SHORT-NAME>",
            "case collision",
        ),
        (
            "process.arxml",
            "/Types/ApplicationTypes</DATA-TYPE-MAPPING-REF>",
            "/Types/MissingMapping</DATA-TYPE-MAPPING-REF>",
            "mapping",
        ),
        (
            "types.arxml",
            "<DIRECTION>INOUT</DIRECTION>",
            "<DIRECTION>INVALID</DIRECTION>",
            "parameter direction",
        ),
        (
            "types.arxml",
            "<ARRAY-SIZE>4</ARRAY-SIZE>",
            "<ARRAY-SIZE>5</ARRAY-SIZE>",
            "array length",
        ),
        (
            "process.arxml",
            "<ALIVE-TIMEOUT>0</ALIVE-TIMEOUT>",
            "<ALIVE-TIMEOUT>1e-999</ALIVE-TIMEOUT>",
            "timeout underflow",
        ),
        (
            "process.arxml",
            "<ALIVE-TIMEOUT>0</ALIVE-TIMEOUT>",
            "<ALIVE-TIMEOUT>invalid</ALIVE-TIMEOUT>",
            "malformed timeout",
        ),
        (
            "process.arxml",
            "<HANDLE-NEVER-RECEIVED>false</HANDLE-NEVER-RECEIVED>",
            "<HANDLE-NEVER-RECEIVED>true</HANDLE-NEVER-RECEIVED>",
            "local freshness",
        ),
        (
            "process.arxml",
            "<VALUE>7</VALUE>",
            "<VALUE>-1</VALUE>",
            "initial value",
        ),
        (
            "process.arxml",
            "<PERIOD>0.01</PERIOD>",
            "<PERIOD>0.02</PERIOD>",
            "period mismatch",
        ),
        (
            "ecuc.arxml",
            "<VALUE>5</VALUE>",
            "<VALUE>4</VALUE>",
            "position",
        ),
        (
            "ecuc.arxml",
            "RteEventIsMappedToTask</DEFINITION-REF>\n                      <VALUE>false</VALUE>",
            "RteEventIsMappedToTask</DEFINITION-REF>\n                      <VALUE>true</VALUE>",
            "server task mapping",
        ),
    ];
    for (file, before, after, label) in vectors {
        let mut sources = inputs();
        change(&mut sources, file, before, after);
        let original = sources
            .iter()
            .map(|source| source.bytes().to_vec())
            .collect::<Vec<_>>();
        let issues = build(&sources)
            .err()
            .unwrap_or_else(|| panic!("{label} accepted"));
        assert!(
            issues
                .iter()
                .any(|issue| issue.file.is_some() && issue.object.is_some()),
            "{label}: {issues:?}"
        );
        assert_eq!(
            original,
            sources
                .iter()
                .map(|source| source.bytes().to_vec())
                .collect::<Vec<_>>()
        );
    }
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn generated_headers_compile_and_link_independent_scalar_and_void_server_signatures() {
    for renamed in [false, true] {
        let mut inputs = multi_operation_inputs();
        if renamed {
            for (before, after) in [
                ("/Application/Process", "/Application/Compute"),
                ("/ProcessInstance", "/ComputeInstance"),
                (">ProcessInstance<", ">ComputeInstance<"),
                (
                    "<SHORT-NAME>Process</SHORT-NAME>",
                    "<SHORT-NAME>Compute</SHORT-NAME>",
                ),
                ("Process_", "Compute_"),
                ("ResultService", "Calculation"),
                ("Transform", "Calculate"),
            ] {
                replace_all(&mut inputs, before, after);
            }
            replace_all(
                &mut inputs,
                "/Application/Compute/Value",
                "/Application/Compute/InputValue",
            );
            xml_edit(
                &mut inputs,
                "process.arxml",
                |node| named(node, "R-PORT-PROTOTYPE", "Value"),
                |original| {
                    original.replace(
                        "<SHORT-NAME>Value</SHORT-NAME>",
                        "<SHORT-NAME>InputValue</SHORT-NAME>",
                    )
                },
            );
        }
        let plan = build(&inputs).unwrap_or_else(|issues| panic!("{issues:?}"));
        let scratch = Scratch::new();
        let directory = scratch.0.join("multi-typed");
        std::fs::create_dir_all(directory.join("include")).unwrap();
        for (file, bytes) in plan.component_contract_files().unwrap().files() {
            std::fs::write(directory.join(file), bytes).unwrap();
        }
        // These definitions and function pointer types are independent of generated metadata.
        let sources = [
            (
                "process.c",
                r#"#include "Rte_Process.h"
static uint32 written;
static uint32 pinged;
void Process_Ping(void) { pinged++; }
void (*const empty_server)(void) = Process_Ping;
void Process_Periodic(void) {}
void Process_Transform(uint32 Input, uint32 *Output, uint32 *State) { *Output=Input; *State+=Input; }
void (*const scalar_server)(uint32, uint32 *, uint32 *) = Process_Transform;
Std_ReturnType Rte_Application_Pipeline_ProcessInstance_Read_Value_Value(uint32 *data) { *data=7U; return E_OK; }
Std_ReturnType Rte_Application_Pipeline_ProcessInstance_Write_Result_Value(uint32 data) { written=data; return E_OK; }
int test_process(void);
int test_process(void) {
    uint32 value=0U, result=0U, state=1U;
    Std_ReturnType (*read_value)(uint32 *)=Rte_Read_Value_Value;
    Std_ReturnType (*write_result)(uint32)=Rte_Write_Result_Value;
    scalar_server(42U,&result,&state);
    empty_server();
    return read_value(&value)==E_OK && value==7U && write_result(43U)==E_OK && written==43U && result==42U && state==43U && pinged==1U ? 0 : 1;
}
"#,
            ),
            (
                "ingress.c",
                r#"#include "Rte_Ingress.h"
static uint32 tx_value, local_value;
void Ingress_Periodic(void) {}
void Ingress_ReadData(Dcm_DataElement_ApplicationValueType Data) { Data[0]=1U; Data[1]=2U; Data[2]=3U; Data[3]=4U; }
void (*const byte_server)(uint8 *) = Ingress_ReadData;
Std_ReturnType Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(uint32 *data) { *data=11U; return E_OK; }
Std_ReturnType Rte_Application_Pipeline_IngressInstance_Write_TxValue_Value(uint32 data) { tx_value=data; return E_OK; }
Std_ReturnType Rte_Application_Pipeline_IngressInstance_Write_Value_Value(uint32 data) { local_value=data; return E_OK; }
int test_ingress(void);
int test_ingress(void) {
    uint32 value=0U;
    uint8 bytes[4]={0U,0U,0U,0U};
    byte_server(bytes);
    return Rte_Read_RxValue_Value(&value)==E_OK && value==11U && Rte_Write_TxValue_Value(21U)==E_OK && Rte_Write_Value_Value(31U)==E_OK && tx_value==21U && local_value==31U && bytes[0]==1U && bytes[3]==4U ? 0 : 1;
}
"#,
            ),
            (
                "observe.c",
                r#"#include "Rte_Observe.h"
void Observe_Periodic(void) {}
Std_ReturnType Rte_Application_Pipeline_ObserveInstance_Call_ResultService_Ping(void) { return E_OK; }
Std_ReturnType (*const empty_client)(void) = Rte_Call_ResultService_Ping;
Std_ReturnType Rte_Application_Pipeline_ObserveInstance_Read_Value_Value(uint32 *data) { *data=9U; return E_OK; }
Std_ReturnType Rte_Application_Pipeline_ObserveInstance_Read_Result_Value(uint32 *data) { *data=19U; return E_OK; }
Std_ReturnType Rte_Application_Pipeline_ObserveInstance_Call_ResultService_Transform(uint32 Input, uint32 *Output, uint32 *State) { *Output=Input; *State+=Input; return E_OK; }
Std_ReturnType (*const scalar_client)(uint32,uint32 *,uint32 *) = Rte_Call_ResultService_Transform;
int test_observe(void);
int test_observe(void) {
    uint32 result=0U, state=1U, value=0U, read_result=0U;
    return empty_client()==E_OK && scalar_client(42U,&result,&state)==E_OK && result==42U && state==43U && Rte_Read_Value_Value(&value)==E_OK && value==9U && Rte_Read_Result_Value(&read_result)==E_OK && read_result==19U ? 0 : 1;
}
"#,
            ),
            (
                "dcm.c",
                r#"#include "Rte_DcmService.h"
Std_ReturnType Rte_Application_Pipeline_DcmService_Call_DataServices_ApplicationValue_ReadData(Dcm_DataElement_ApplicationValueType Data) { Data[0]=1U; Data[1]=2U; Data[2]=3U; Data[3]=4U; return E_OK; }
Std_ReturnType DcmService_ReadData(Dcm_DataElement_ApplicationValueType Data) { return Rte_Call_DataServices_ApplicationValue_ReadData(Data); }
Std_ReturnType (*const byte_client)(uint8 *)=Rte_Call_DataServices_ApplicationValue_ReadData;
Std_ReturnType (*const dcm_bridge)(uint8 *)=DcmService_ReadData;
int test_dcm(void);
int test_dcm(void) {
    uint8 bytes[4]={0U,0U,0U,0U};
    return byte_client(bytes)==E_OK && bytes[0]==1U && bytes[1]==2U && bytes[2]==3U && bytes[3]==4U && dcm_bridge(bytes)==E_OK ? 0 : 1;
}
"#,
            ),
            (
                "main.c",
                r#"int test_process(void);
int test_ingress(void);
int test_observe(void);
int test_dcm(void);
int main(void) { return test_process() || test_ingress() || test_observe() || test_dcm(); }
"#,
            ),
        ];
        for (file, bytes) in sources {
            let (mut file, mut bytes) = (file.to_owned(), bytes.to_owned());
            if renamed {
                for (before, after) in [
                    ("Process", "Compute"),
                    ("ResultService", "Calculation"),
                    ("Transform", "Calculate"),
                ] {
                    file = file.replace(before, after);
                    bytes = bytes.replace(before, after);
                }
                if file == "process.c" {
                    bytes = bytes
                        .replace("Rte_Read_Value_Value", "Rte_Read_InputValue_Value")
                        .replace(
                            "ComputeInstance_Read_Value_Value",
                            "ComputeInstance_Read_InputValue_Value",
                        );
                }
            }
            std::fs::write(directory.join(file), bytes).unwrap();
        }
        let executable = tooling::native_binary(&directory, "typed");
        let mut compiler = std::process::Command::new("gcc");
        compiler
            .current_dir(&directory)
            .args([
                "-std=c99",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-Iinclude",
                "process.c",
                "ingress.c",
                "observe.c",
                "dcm.c",
                "main.c",
                "-o",
            ])
            .arg(&executable);
        let output = tooling::run_public_command(
            &mut compiler,
            &directory,
            "typed-compile",
            std::time::Duration::from_secs(60),
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = tooling::run_public_command(
            &mut std::process::Command::new(executable),
            &directory,
            "typed-run",
            std::time::Duration::from_secs(30),
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        // Compile the actual generated producer too: the consumer definitions
        // above validate independent signatures, not this implementation's bodies.
        std::fs::create_dir_all(directory.join("src")).unwrap();
        let produced = prepared_multi_files(&plan);
        let metadata = std::str::from_utf8(
            &produced
                .iter()
                .find(|(file, _)| file == "descriptions/Host_Implementation.arxml")
                .unwrap()
                .1,
        )
        .unwrap();
        assert!(metadata.contains(if renamed {
            "<SHORT-NAME>Compute_CODE</SHORT-NAME>"
        } else {
            "<SHORT-NAME>Process_CODE</SHORT-NAME>"
        }));
        let partition = plan
            .description()
            .communication_runtime
            .as_ref()
            .unwrap()
            .partition
            .path
            .rsplit('/')
            .next()
            .unwrap();
        assert!(metadata.contains(&format!(
            "<SHORT-NAME>{partition}_CALLOUT_CODE</SHORT-NAME>"
        )));
        for (file, bytes) in produced {
            if file.starts_with("include/") || file == "src/Rte.c" {
                std::fs::write(directory.join(file), bytes).unwrap();
            }
        }
        let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let settings = tooling::execution_settings();
        let mut compiler = std::process::Command::new(&settings.compiler);
        compiler.current_dir(&directory).args([
            "-std=c99",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
            "-Iinclude",
        ]);
        for include in [
            "runtime/ecu/include",
            "runtime/os/include",
            "runtime/include",
        ] {
            compiler.arg("-I").arg(repository.join(include));
        }
        compiler.args(["-c", "src/Rte.c", "-o", "Rte.o"]);
        let output = tooling::run_public_command(
            &mut compiler,
            &directory,
            "rte-producer-compile",
            std::time::Duration::from_secs(60),
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = tooling::run_public_command(
            std::process::Command::new(&settings.objdump)
                .current_dir(&directory)
                .args(["-h", "Rte.o"]),
            &directory,
            "rte-producer-sections",
            std::time::Duration::from_secs(30),
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let sections = String::from_utf8_lossy(&output.stdout);
        assert!(sections.contains(".rte_code"), "{sections}");
        assert!(
            sections.contains(".bss.rte.Observe.VAR_CLEARED_UNSPECIFIED"),
            "{sections}"
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}

fn replace_all(sources: &mut [InputSource], before: &str, after: &str) {
    for source in sources {
        let text = std::str::from_utf8(source.bytes()).unwrap();
        *source = InputSource::new(
            source.logical_path(),
            text.replace(before, after).into_bytes(),
        )
        .unwrap();
    }
}

#[test]
fn collisions_are_rejected_with_all_references_still_valid() {
    let vectors = [
        ("Dcm_DataElement_ApplicationValueType", "boolean"),
        (
            "Dcm_DataElement_ApplicationValueType",
            "Std_VersionInfoType",
        ),
        ("Dcm_DataElement_ApplicationValueType", "UINT32_MAX"),
        ("Dcm_DataElement_ApplicationValueType", "uint_fast32_t"),
        ("Process", "ingress"),
        ("Process_Periodic", "Rte_Read_Value_Value"),
        ("Process_Periodic", "Rte_COMCbk"),
        ("Process_Periodic", "Rte_COMCbkRxTOut"),
        ("Process_Periodic", "Com_MainFunctionRx_Rx"),
        ("Process_Periodic", "Com_MainFunctionTx_Tx"),
        (
            "<SHORT-NAME>Input</SHORT-NAME>",
            "<SHORT-NAME>uint32</SHORT-NAME>",
        ),
        (
            "<SHORT-NAME>Input</SHORT-NAME>",
            "<SHORT-NAME>Std_ReturnType</SHORT-NAME>",
        ),
        (
            "<SHORT-NAME>Input</SHORT-NAME>",
            "<SHORT-NAME>E_OK</SHORT-NAME>",
        ),
    ];
    for (before, after) in vectors {
        let mut sources = inputs();
        if before == "Process" {
            replace_all(&mut sources, "/Application/Process", "/Application/ingress");
            replace_all(
                &mut sources,
                "<SHORT-NAME>Process</SHORT-NAME>",
                "<SHORT-NAME>ingress</SHORT-NAME>",
            );
        } else {
            replace_all(&mut sources, before, after);
        }
        let issues = build(&sources)
            .err()
            .unwrap_or_else(|| panic!("{after} accepted"));
        assert!(
            issues.iter().any(|issue| matches!(
                issue.code.as_str(),
                "CONTRACT_NAME_COLLISION"
                    | "SYMBOL_NORMALIZATION_COLLISION"
                    | "SYMBOL_PRODUCER_DUPLICATE"
            )),
            "{after}: {issues:?}"
        );
        assert!(
            !issues
                .iter()
                .any(|issue| issue.code == "REFERENCE_UNRESOLVED"),
            "{after}: dangling references mask collision"
        );
    }
}

#[test]
fn missing_and_duplicate_relations_are_rejected_before_emission() {
    let cases = [
        ("process.arxml", "INIT-VALUE", None, false),
        ("process.arxml", "DATA-TYPE-MAPPING-REFS", None, false),
        ("process.arxml", "TIMING-EVENT", Some("Periodic10ms"), false),
        ("process.arxml", "TIMING-EVENT", Some("Periodic10ms"), true),
        ("ecuc.arxml", "ECUC-CONTAINER-VALUE", Some("Process"), false),
        (
            "ecuc.arxml",
            "ECUC-CONTAINER-VALUE",
            Some("ReadApplicationValue"),
            false,
        ),
        (
            "composition.arxml",
            "ASSEMBLY-SW-CONNECTOR",
            Some("IngressProcess"),
            false,
        ),
        (
            "composition.arxml",
            "ASSEMBLY-SW-CONNECTOR",
            Some("IngressProcess"),
            true,
        ),
    ];
    for (file, tag, short_name, duplicate) in cases {
        let mut sources = inputs();
        let source = sources
            .iter_mut()
            .find(|source| source.logical_path() == file)
            .unwrap();
        let text = std::str::from_utf8(source.bytes()).unwrap();
        let document = roxmltree::Document::parse(text).unwrap();
        let node = document
            .descendants()
            .find(|node| {
                node.tag_name().name() == tag
                    && short_name.is_none_or(|name| {
                        node.children().any(|child| {
                            child.tag_name().name() == "SHORT-NAME" && child.text() == Some(name)
                        })
                    })
            })
            .unwrap();
        let range = node.range();
        let replacement = if duplicate {
            format!(
                "{}{}",
                &text[range.clone()],
                text[range.clone()].replacen(
                    &format!("<SHORT-NAME>{}</SHORT-NAME>", short_name.unwrap()),
                    "<SHORT-NAME>Duplicate</SHORT-NAME>",
                    1
                )
            )
        } else {
            String::new()
        };
        let mutated = format!(
            "{}{}{}",
            &text[..range.start],
            replacement,
            &text[range.end..]
        );
        *source = InputSource::new(file, mutated.into_bytes()).unwrap();
        let issues = build(&sources)
            .err()
            .unwrap_or_else(|| panic!("{file}/{tag} duplicate={duplicate} accepted"));
        assert!(
            issues
                .iter()
                .any(|issue| issue.file.is_some() && issue.object.is_some()),
            "{issues:?}"
        );
    }
}

#[test]
fn renamed_component_and_port_contracts_follow_full_source_identity() {
    let mut sources = inputs();
    replace_all(&mut sources, "Ingress", "Gateway");
    replace_all(
        &mut sources,
        "/Application/Process/Value",
        "/Application/Process/InputValue",
    );
    change(
        &mut sources,
        "process.arxml",
        "<SHORT-NAME>Value</SHORT-NAME>",
        "<SHORT-NAME>InputValue</SHORT-NAME>",
    );
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    let multi = plan.description().multi.as_ref().unwrap();
    let gateway = multi
        .components
        .iter()
        .find(|component| component.component == "/Application/Gateway")
        .unwrap();
    assert_eq!(gateway.instance, "/Application/Pipeline/GatewayInstance");
    assert_eq!(gateway.header, "include/Rte_Gateway.h");
    let process = multi
        .components
        .iter()
        .find(|component| component.component == "/Application/Process")
        .unwrap();
    assert_eq!(
        process
            .data_ports
            .iter()
            .find(|port| port.read)
            .unwrap()
            .api_symbol,
        "Rte_Read_InputValue_Value"
    );
    assert!(
        multi
            .connections
            .iter()
            .any(|connection| connection.requester.port == "/Application/Process/InputValue")
    );
    assert!(
        plan.component_contract_files()
            .unwrap()
            .files()
            .iter()
            .any(|(file, _)| file == "include/Rte_Gateway.h")
    );
}

#[cfg(feature = "official-oracles")]
#[test]
fn official_and_native_entry_points_produce_the_same_multi_headers() {
    use autosar_config_core::integration::{PlanDependencies, build_plan};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let plan = build_plan(
        &inputs(),
        &PlanDependencies::from_repository(root),
        &RuntimeCatalog::from_repository(root).unwrap(),
    )
    .unwrap_or_else(|issues| panic!("{issues:?}"));
    let native = build(&inputs()).unwrap();
    let official = plan.component_contract_files().unwrap();
    let native = native.component_contract_files().unwrap();
    assert_eq!(
        official
            .files()
            .iter()
            .filter(|(file, _)| file.ends_with(".h"))
            .collect::<Vec<_>>(),
        native
            .files()
            .iter()
            .filter(|(file, _)| file.ends_with(".h"))
            .collect::<Vec<_>>()
    );
}

fn subtree(sources: &[InputSource], file: &str, tag: &str, short_name: &str) -> String {
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

#[test]
fn every_call_point_and_actual_server_recursion_are_checked() {
    let mut sources = inputs();
    let call = subtree(
        &sources,
        "observe.arxml",
        "SYNCHRONOUS-SERVER-CALL-POINT",
        "CallTransform",
    );
    let foreign = call
        .replace("CallTransform", "ForeignCall")
        .replace(
            "/Application/Observe/ResultService",
            "/Services/DcmService/DataServices_ApplicationValue",
        )
        .replace(
            "/Types/ScalarService/Transform",
            "/Types/DataServices_ApplicationValue/ReadData",
        );
    change(
        &mut sources,
        "observe.arxml",
        "</SERVER-CALL-POINTS>",
        &format!("{foreign}</SERVER-CALL-POINTS>"),
    );
    let issues = build(&sources)
        .err()
        .expect("Foreign second call point accepted");
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "CALL_POINT_OWNERSHIP"),
        "{issues:?}"
    );

    let mut sources = inputs();
    let port = subtree(
        &sources,
        "observe.arxml",
        "R-PORT-PROTOTYPE",
        "ResultService",
    )
    .replace(
        "<SHORT-NAME>ResultService</SHORT-NAME>",
        "<SHORT-NAME>LoopClient</SHORT-NAME>",
    );
    change(
        &mut sources,
        "process.arxml",
        "</PORTS>",
        &format!("{port}</PORTS>"),
    );
    let recursive_call = call.replace("CallTransform", "RecursiveCall").replace(
        "/Application/Observe/ResultService",
        "/Application/Process/LoopClient",
    );
    change(
        &mut sources,
        "process.arxml",
        "<SYMBOL>Process_Transform</SYMBOL>",
        &format!(
            "<SERVER-CALL-POINTS>{recursive_call}</SERVER-CALL-POINTS><SYMBOL>Process_Transform</SYMBOL>"
        ),
    );
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "Scalar",
    )
    .replace(
        "<SHORT-NAME>Scalar</SHORT-NAME>",
        "<SHORT-NAME>Recursive</SHORT-NAME>",
    )
    .replace(
        "/Application/Pipeline/ObserveInstance",
        "/Application/Pipeline/ProcessInstance",
    )
    .replace(
        "/Application/Observe/ResultService",
        "/Application/Process/LoopClient",
    );
    change(
        &mut sources,
        "composition.arxml",
        "</CONNECTORS>",
        &format!("{connector}</CONNECTORS>"),
    );
    let issues = build(&sources)
        .err()
        .expect("Recursive server call accepted");
    assert!(
        issues.iter().any(|issue| issue.code == "SERVICE_RECURSION"),
        "{issues:?}"
    );
    assert!(
        normal_validation(&sources)
            .diagnostics
            .iter()
            .any(|issue| issue.code == "SERVICE_RECURSION")
    );
}

#[test]
fn equal_width_implementation_types_and_network_local_double_binding_are_rejected() {
    let mut sources = inputs();
    let implementation = subtree(
        &sources,
        "types.arxml",
        "IMPLEMENTATION-DATA-TYPE",
        "uint32",
    )
    .replace(
        "<SHORT-NAME>uint32</SHORT-NAME>",
        "<SHORT-NAME>OtherUint32</SHORT-NAME>",
    );
    let mapping = subtree(
        &sources,
        "types.arxml",
        "DATA-TYPE-MAPPING-SET",
        "ApplicationTypes",
    )
    .replace(
        "<SHORT-NAME>ApplicationTypes</SHORT-NAME>",
        "<SHORT-NAME>OtherTypes</SHORT-NAME>",
    )
    .replace(
        "/Types/uint32</IMPLEMENTATION-DATA-TYPE-REF>",
        "/Types/OtherUint32</IMPLEMENTATION-DATA-TYPE-REF>",
    );
    change(
        &mut sources,
        "types.arxml",
        "</ELEMENTS>",
        &format!("{implementation}{mapping}</ELEMENTS>"),
    );
    change(
        &mut sources,
        "process.arxml",
        "/Types/ApplicationTypes</DATA-TYPE-MAPPING-REF>",
        "/Types/OtherTypes</DATA-TYPE-MAPPING-REF>",
    );
    let issues = build(&sources)
        .err()
        .expect("Equal-width distinct implementation types accepted");
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "TYPE_CONFLICT" || issue.code == "SERVICE_TYPE_CONFLICT"),
        "{issues:?}"
    );
    assert!(
        !issues
            .iter()
            .any(|issue| issue.code == "REFERENCE_UNRESOLVED")
    );

    let mut sources = inputs();
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "IngressProcess",
    )
    .replace(
        "/Application/Pipeline/ProcessInstance",
        "/Application/Pipeline/IngressInstance",
    )
    .replace("/Application/Process/Value", "/Application/Ingress/RxValue");
    let original = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "IngressProcess",
    );
    change(&mut sources, "composition.arxml", &original, &connector);
    let issues = build(&sources)
        .err()
        .expect("Local and network producer accepted");
    assert!(
        issues.iter().any(|issue| issue.code == "ENDPOINT_BINDING"),
        "{issues:?}"
    );
    assert!(
        normal_validation(&sources)
            .diagnostics
            .iter()
            .any(|issue| issue.code == "ENDPOINT_BINDING")
    );
}

#[test]
fn unsigned_contract_rejects_conflicting_base_encodings_with_valid_references() {
    for native in ["uint8", "uint32"] {
        for encoding in ["2C", "IEEE754"] {
            let mut sources = inputs();
            change(
                &mut sources,
                "types.arxml",
                &format!(
                    "<BASE-TYPE-ENCODING>NONE</BASE-TYPE-ENCODING>\n          <NATIVE-DECLARATION>{native}</NATIVE-DECLARATION>"
                ),
                &format!(
                    "<BASE-TYPE-ENCODING>{encoding}</BASE-TYPE-ENCODING>\n          <NATIVE-DECLARATION>{native}</NATIVE-DECLARATION>"
                ),
            );
            let issues = build(&sources)
                .err()
                .expect("conflicting encoding rejected");
            assert!(
                issues.iter().any(|issue| issue.code == "TYPE_CONFLICT"),
                "{issues:?}"
            );
            assert!(
                !issues.iter().any(|issue| issue.code.contains("UNRESOLVED")),
                "{issues:?}"
            );
        }
    }
}

fn normal_validation(
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

fn xml_edit(
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

fn named(node: roxmltree::Node<'_, '_>, tag: &str, name: &str) -> bool {
    node.tag_name().name() == tag
        && node
            .children()
            .any(|child| child.tag_name().name() == "SHORT-NAME" && child.text() == Some(name))
}

fn parameter(sources: &mut [InputSource], owner: &str, definition: &str, new_value: &str) {
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

fn rejects_in_both(sources: &[InputSource], code: &str) {
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

#[test]
fn normal_definition_validation_closes_multi_schedule_routes_types_and_handles() {
    let validation = normal_validation(&inputs());
    assert!(
        validation.diagnostics.is_empty(),
        "{:?}",
        validation.diagnostics
    );
    for (owner, definition, value, code) in [
        ("Observe", "RtePositionInTask", "5", "SCHEDULE_NOT_UNIQUE"),
        (
            "ReadApplicationValue",
            "RteEventIsMappedToTask",
            "true",
            "SERVICE_ASYNC_MAPPING",
        ),
        ("RxValue", "CanIfRxPduId", "2", "CAN_ID_CONFLICT"),
        (
            "RxValueSource",
            "PduRSrcPduUpTxConf",
            "false",
            "PDU_ROUTE_CONFIRMATION",
        ),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, owner, definition, value);
        rejects_in_both(&sources, code);
    }
    let mut sources = inputs();
    change(
        &mut sources,
        "composition.arxml",
        "/Application/Process/Value</TARGET-R-PORT-REF>",
        "/Application/Process/Result</TARGET-R-PORT-REF>",
    );
    rejects_in_both(&sources, "REFERENCE_DEST");
    let mut sources = inputs();
    change(
        &mut sources,
        "types.arxml",
        "<BASE-TYPE-ENCODING>NONE</BASE-TYPE-ENCODING>",
        "<BASE-TYPE-ENCODING>2C</BASE-TYPE-ENCODING>",
    );
    rejects_in_both(&sources, "TYPE_CONFLICT");
    let mut sources = inputs();
    parameter(&mut sources, "TxValue", "CanIfTxPduId", "0");
    assert!(
        build(&sources).is_ok(),
        "same handle in different directions must be allowed"
    );
    assert!(normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn selected_network_profile_rejects_all_local_endpoints_without_panicking() {
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| node.has_tag_name("DATA-MAPPINGS"),
        |_| String::new(),
    );
    xml_edit(
        &mut sources,
        "ingress.arxml",
        |node| named(node, "R-PORT-PROTOTYPE", "RxValue"),
        |text| {
            text.replace(
                "<HANDLE-NEVER-RECEIVED>true</HANDLE-NEVER-RECEIVED>",
                "<HANDLE-NEVER-RECEIVED>false</HANDLE-NEVER-RECEIVED>",
            )
            .replace(
                "<ALIVE-TIMEOUT>0.03</ALIVE-TIMEOUT>",
                "<ALIVE-TIMEOUT>0</ALIVE-TIMEOUT>",
            )
        },
    );
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "IngressProcess",
    )
    .replace("IngressProcess", "LocalIngress")
    .replace("/Application/Ingress/Value", "/Application/Ingress/TxValue")
    .replace(
        "/Application/Pipeline/ProcessInstance",
        "/Application/Pipeline/IngressInstance",
    )
    .replace("/Application/Process/Value", "/Application/Ingress/RxValue");
    change(
        &mut sources,
        "composition.arxml",
        "</CONNECTORS>",
        &format!("{connector}</CONNECTORS>"),
    );
    rejects_in_both(&sources, "COM_CONFIGURATION");
}

#[test]
fn network_freshness_and_runnable_execution_constraints_are_explicit() {
    for (before, after) in [
        (
            "<HANDLE-NEVER-RECEIVED>true</HANDLE-NEVER-RECEIVED>",
            "<HANDLE-NEVER-RECEIVED>false</HANDLE-NEVER-RECEIVED>",
        ),
        (
            "<HANDLE-TIMEOUT-TYPE>NONE</HANDLE-TIMEOUT-TYPE>",
            "<HANDLE-TIMEOUT-TYPE>REPLACE</HANDLE-TIMEOUT-TYPE>",
        ),
    ] {
        let mut sources = inputs();
        change(&mut sources, "ingress.arxml", before, after);
        rejects_in_both(&sources, "NETWORK_FRESHNESS_UNSUPPORTED");
    }
    let mut sources = inputs();
    change(
        &mut sources,
        "process.arxml",
        "<CAN-BE-INVOKED-CONCURRENTLY>false",
        "<MINIMUM-START-INTERVAL>0</MINIMUM-START-INTERVAL><CAN-BE-INVOKED-CONCURRENTLY>false",
    );
    assert!(build(&sources).is_ok());
    change(
        &mut sources,
        "process.arxml",
        "<MINIMUM-START-INTERVAL>0",
        "<MINIMUM-START-INTERVAL>0.001",
    );
    rejects_in_both(&sources, "MINIMUM_START_INTERVAL_UNSUPPORTED");
    let mut sources = inputs();
    change(
        &mut sources,
        "process.arxml",
        "<EVENTS>",
        "<EXCLUSIVE-AREAS><EXCLUSIVE-AREA><SHORT-NAME>Lock</SHORT-NAME></EXCLUSIVE-AREA></EXCLUSIVE-AREAS><EVENTS>",
    );
    change(
        &mut sources,
        "process.arxml",
        "<CAN-BE-INVOKED-CONCURRENTLY>false",
        "<CAN-ENTER-EXCLUSIVE-AREA-REFS><CAN-ENTER-EXCLUSIVE-AREA-REF DEST=\"EXCLUSIVE-AREA\">/Application/Process/Behavior/Lock</CAN-ENTER-EXCLUSIVE-AREA-REF></CAN-ENTER-EXCLUSIVE-AREA-REFS><CAN-BE-INVOKED-CONCURRENTLY>false",
    );
    let issues = build(&sources).err().unwrap();
    assert!(
        issues
            .iter()
            .any(|issue| issue.file.as_deref() == Some("process.arxml")),
        "{issues:?}"
    );
    // The bounded normal grammar rejects this official declaration before semantic planning.
    assert!(!normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn dcm_bridge_requires_actual_service_identity_and_follows_renames() {
    for (before, after) in [
        (
            "<SERVICE-KIND>DIAGNOSTIC-COMMUNICATION-MANAGER</SERVICE-KIND>",
            "<SERVICE-KIND>COM-MANAGER</SERVICE-KIND>",
        ),
        (
            "<IS-SERVICE>true</IS-SERVICE>",
            "<IS-SERVICE>false</IS-SERVICE>",
        ),
    ] {
        let mut sources = inputs();
        change(&mut sources, "types.arxml", before, after);
        rejects_in_both(&sources, "SERVICE_CLIENT_MISSING");
    }
    let mut sources = inputs();
    replace_all(
        &mut sources,
        "/Services/DcmService/DataServices_ApplicationValue",
        "/Services/DcmService/IncorrectPort",
    );
    change(
        &mut sources,
        "services.arxml",
        "<SHORT-NAME>DataServices_ApplicationValue</SHORT-NAME>",
        "<SHORT-NAME>IncorrectPort</SHORT-NAME>",
    );
    rejects_in_both(&sources, "SERVICE_CLIENT_MISSING");
    let mut sources = inputs();
    replace_all(&mut sources, "ApplicationValue", "VehicleValue");
    replace_all(&mut sources, "DcmService", "DiagnosticClient");
    // The application provider port is identified by its connector, not by the DID name.
    replace_all(
        &mut sources,
        "/Application/Ingress/VehicleValue",
        "/Application/Ingress/SnapshotRead",
    );
    change(
        &mut sources,
        "ingress.arxml",
        "<SHORT-NAME>VehicleValue</SHORT-NAME>",
        "<SHORT-NAME>SnapshotRead</SHORT-NAME>",
    );
    let plan = build(&sources).unwrap();
    assert_eq!(
        plan.description().diagnostic.as_ref().unwrap().client_port,
        "/Services/DiagnosticClient/DataServices_VehicleValue"
    );
    assert!(normal_validation(&sources).diagnostics.is_empty());
}

fn sources_without_application_did(retain_routes: bool) -> Vec<InputSource> {
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

#[test]
fn network_routing_is_checked_without_optional_did() {
    let mut sources = sources_without_application_did(false);
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert!(plan.description().diagnostic.is_none());
    assert!(plan.description().diagnostic_transport.is_none());
    assert!(
        !plan
            .description()
            .schedule
            .entities
            .iter()
            .any(|entity| matches!(
                entity.symbol.as_str(),
                "CanTp_MainFunction" | "Dcm_MainFunction"
            ))
    );
    assert_eq!(plan.description().routes.len(), 2);
    assert!(normal_validation(&sources).diagnostics.is_empty());
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "RxValue")
                && node.children().any(|child| {
                    child
                        .text()
                        .is_some_and(|text| text.ends_with("/PduRRoutingPath"))
                })
        },
        |_| String::new(),
    );
    rejects_in_both(&sources, "PDU_ROUTE_NOT_UNIQUE");
}

#[test]
fn ordinary_application_client_sharing_did_server_is_not_the_dcm_bridge() {
    let mut sources = inputs();
    let port = subtree(
        &sources,
        "services.arxml",
        "R-PORT-PROTOTYPE",
        "DataServices_ApplicationValue",
    );
    let call = subtree(
        &sources,
        "services.arxml",
        "SYNCHRONOUS-SERVER-CALL-POINT",
        "ReadApplicationValue",
    )
    .replace("/Services/DcmService", "/Application/Observe");
    change(
        &mut sources,
        "observe.arxml",
        "</PORTS>",
        &format!("{port}</PORTS>"),
    );
    change(
        &mut sources,
        "observe.arxml",
        "</SERVER-CALL-POINTS>",
        &format!("{call}</SERVER-CALL-POINTS>"),
    );
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "Diagnostic",
    )
    .replace(
        "<SHORT-NAME>Diagnostic</SHORT-NAME>",
        "<SHORT-NAME>ApplicationDiagnostic</SHORT-NAME>",
    )
    .replace(
        "/Application/Pipeline/DcmService",
        "/Application/Pipeline/ObserveInstance",
    )
    .replace("/Services/DcmService", "/Application/Observe");
    change(
        &mut sources,
        "composition.arxml",
        "</CONNECTORS>",
        &format!("{connector}</CONNECTORS>"),
    );
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(
        plan.description().diagnostic.as_ref().unwrap().client_port,
        "/Services/DcmService/DataServices_ApplicationValue"
    );
    assert_eq!(
        plan.description()
            .multi
            .as_ref()
            .unwrap()
            .connections
            .iter()
            .filter(|connection| connection.service
                && connection.provider.port == "/Application/Ingress/ApplicationValue")
            .count(),
        2
    );
    assert!(normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn two_real_dcm_requesters_are_rejected_without_dangling_references() {
    let mut sources = inputs();
    let component = subtree(
        &sources,
        "services.arxml",
        "SERVICE-SW-COMPONENT-TYPE",
        "DcmService",
    )
    .replace("DcmService", "DcmSecond");
    change(
        &mut sources,
        "services.arxml",
        "</ELEMENTS>",
        &format!("{component}</ELEMENTS>"),
    );
    let prototype = subtree(
        &sources,
        "composition.arxml",
        "SW-COMPONENT-PROTOTYPE",
        "DcmService",
    )
    .replace("DcmService", "DcmSecond");
    change(
        &mut sources,
        "composition.arxml",
        "</COMPONENTS>",
        &format!("{prototype}</COMPONENTS>"),
    );
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "Diagnostic",
    )
    .replace(
        "<SHORT-NAME>Diagnostic</SHORT-NAME>",
        "<SHORT-NAME>SecondDiagnostic</SHORT-NAME>",
    )
    .replace("DcmService", "DcmSecond");
    change(
        &mut sources,
        "composition.arxml",
        "</CONNECTORS>",
        &format!("{connector}</CONNECTORS>"),
    );
    let mapping = "<COMPONENT-IREF><CONTEXT-COMPOSITION-REF DEST=\"ROOT-SW-COMPOSITION-PROTOTYPE\">/Extract/ReferenceExtract/RootComposition</CONTEXT-COMPOSITION-REF><TARGET-COMPONENT-REF DEST=\"SW-COMPONENT-PROTOTYPE\">/Application/Pipeline/DcmSecond</TARGET-COMPONENT-REF></COMPONENT-IREF>";
    change(
        &mut sources,
        "extract.arxml",
        "</COMPONENT-IREFS>",
        &format!("{mapping}</COMPONENT-IREFS>"),
    );
    let rte_instance = subtree(&sources, "ecuc.arxml", "ECUC-CONTAINER-VALUE", "DcmService");
    change(
        &mut sources,
        "ecuc.arxml",
        &rte_instance,
        &format!(
            "{}{}",
            rte_instance,
            rte_instance.replace("DcmService", "DcmSecond")
        ),
    );
    rejects_in_both(&sources, "SERVICE_CLIENT_MISSING");
}

#[cfg(feature = "official-oracles")]
#[test]
fn execution_constraint_rejections_use_officially_valid_arxml() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for exclusive_area in [false, true] {
        let mut sources = inputs();
        if exclusive_area {
            change(
                &mut sources,
                "process.arxml",
                "<EVENTS>",
                "<EXCLUSIVE-AREAS><EXCLUSIVE-AREA><SHORT-NAME>Lock</SHORT-NAME></EXCLUSIVE-AREA></EXCLUSIVE-AREAS><EVENTS>",
            );
            change(
                &mut sources,
                "process.arxml",
                "<CAN-BE-INVOKED-CONCURRENTLY>false",
                "<CAN-ENTER-EXCLUSIVE-AREA-REFS><CAN-ENTER-EXCLUSIVE-AREA-REF DEST=\"EXCLUSIVE-AREA\">/Application/Process/Behavior/Lock</CAN-ENTER-EXCLUSIVE-AREA-REF></CAN-ENTER-EXCLUSIVE-AREA-REFS><CAN-BE-INVOKED-CONCURRENTLY>false",
            );
        } else {
            change(
                &mut sources,
                "process.arxml",
                "<CAN-BE-INVOKED-CONCURRENTLY>false",
                "<MINIMUM-START-INTERVAL>0.001</MINIMUM-START-INTERVAL><CAN-BE-INVOKED-CONCURRENTLY>false",
            );
        }
        let files = sources
            .iter()
            .map(|source| {
                (
                    Path::new(source.logical_path()),
                    std::str::from_utf8(source.bytes()).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let issues = autosar_config_core::schema::validate_files(
            &autosar_config_core::schema::schema_archive(root),
            &files,
        )
        .unwrap();
        assert!(issues.is_empty(), "{issues:?}");
        assert!(build(&sources).is_err());
        assert!(!normal_validation(&sources).diagnostics.is_empty());
    }
}

#[test]
fn normal_definition_validation_keeps_duplicate_producer_diagnostics() {
    let mut sources = inputs();
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "IngressProcess",
    )
    .replace(
        "<SHORT-NAME>IngressProcess</SHORT-NAME>",
        "<SHORT-NAME>SecondProducer</SHORT-NAME>",
    );
    change(
        &mut sources,
        "composition.arxml",
        "</CONNECTORS>",
        &format!("{connector}</CONNECTORS>"),
    );
    rejects_in_both(&sources, "MULTIPLE_PRODUCERS");
}

fn multi_operation_inputs() -> Vec<InputSource> {
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

#[test]
fn client_server_sets_close_in_both_directions_and_empty_interfaces_are_rejected() {
    for file in ["process.arxml", "observe.arxml"] {
        for foreign in [false, true] {
            let mut sources = inputs();
            let tag = if file == "process.arxml" {
                "SERVER-COM-SPEC"
            } else {
                "CLIENT-COM-SPEC"
            };
            xml_edit(
                &mut sources,
                file,
                |node| node.tag_name().name() == tag,
                |original| {
                    format!(
                        "{original}{}",
                        if foreign {
                            original.replace(
                                "/Types/ScalarService/Transform",
                                "/Types/DataServices_ApplicationValue/ReadData",
                            )
                        } else {
                            original.into()
                        }
                    )
                },
            );
            rejects_in_both(&sources, "SERVICE_TYPE_CONFLICT");
        }
    }
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "process.arxml",
        |node| named(node, "OPERATION-INVOKED-EVENT", "InvokeTransform"),
        |original| {
            format!(
                "{original}<OPERATION-INVOKED-EVENT><SHORT-NAME>InvokeUnbound</SHORT-NAME><START-ON-EVENT-REF DEST=\"RUNNABLE-ENTITY\">/Application/Process/Behavior/Unbound</START-ON-EVENT-REF><OPERATION-IREF><CONTEXT-P-PORT-REF DEST=\"P-PORT-PROTOTYPE\">/Application/Process/ResultService</CONTEXT-P-PORT-REF><TARGET-PROVIDED-OPERATION-REF DEST=\"CLIENT-SERVER-OPERATION\">/Types/DataServices_ApplicationValue/ReadData</TARGET-PROVIDED-OPERATION-REF></OPERATION-IREF></OPERATION-INVOKED-EVENT>"
            )
        },
    );
    change(
        &mut sources,
        "process.arxml",
        "</RUNNABLES>",
        "<RUNNABLE-ENTITY><SHORT-NAME>Unbound</SHORT-NAME><CAN-BE-INVOKED-CONCURRENTLY>false</CAN-BE-INVOKED-CONCURRENTLY><SYMBOL>Process_Unbound</SYMBOL></RUNNABLE-ENTITY></RUNNABLES>",
    );
    rejects_in_both(&sources, "EVENT_OWNERSHIP");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "types.arxml",
        |node| named(node, "CLIENT-SERVER-INTERFACE", "ScalarService"),
        |_| {
            "<CLIENT-SERVER-INTERFACE><SHORT-NAME>ScalarService</SHORT-NAME><IS-SERVICE>false</IS-SERVICE></CLIENT-SERVER-INTERFACE>".into()
        },
    );
    for (file, tag) in [
        ("process.arxml", "PROVIDED-COM-SPECS"),
        ("observe.arxml", "REQUIRED-COM-SPECS"),
        ("process.arxml", "OPERATION-IREF"),
        ("observe.arxml", "OPERATION-IREF"),
    ] {
        xml_edit(
            &mut sources,
            file,
            |node| {
                node.tag_name().name() == tag
                    && (tag == "OPERATION-IREF"
                        || node.ancestors().any(|ancestor| {
                            named(ancestor, "P-PORT-PROTOTYPE", "ResultService")
                                || named(ancestor, "R-PORT-PROTOTYPE", "ResultService")
                        }))
            },
            |_| String::new(),
        );
    }
    rejects_in_both(&sources, "SERVICE_TYPE_CONFLICT");
}

#[test]
fn every_operation_has_its_own_spec_call_and_event_and_void_is_valid() {
    let sources = multi_operation_inputs();
    assert!(normal_validation(&sources).diagnostics.is_empty());
    let plan = build(&sources).unwrap();
    let files = plan.component_contract_files().unwrap();
    assert!(
        std::str::from_utf8(
            &files
                .files()
                .iter()
                .find(|(path, _)| path == "include/Rte_Process.h")
                .unwrap()
                .1
        )
        .unwrap()
        .contains("void Process_Ping(void);")
    );
    for (file, tag, code) in [
        ("process.arxml", "SERVER-COM-SPEC", "SERVICE_TYPE_CONFLICT"),
        ("observe.arxml", "CLIENT-COM-SPEC", "SERVICE_TYPE_CONFLICT"),
        (
            "observe.arxml",
            "SYNCHRONOUS-SERVER-CALL-POINT",
            "SERVICE_CLIENT_MISSING",
        ),
        (
            "process.arxml",
            "OPERATION-INVOKED-EVENT",
            "SERVICE_CLIENT_MISSING",
        ),
    ] {
        let mut changed = sources.clone();
        xml_edit(
            &mut changed,
            file,
            |node| {
                node.tag_name().name() == tag
                    && node.descendants().any(|child| {
                        child
                            .text()
                            .is_some_and(|text| text.ends_with("/Ping") || text == "InvokePing")
                    })
            },
            |_| String::new(),
        );
        if tag == "OPERATION-INVOKED-EVENT" {
            xml_edit(
                &mut changed,
                "ecuc.arxml",
                |node| named(node, "ECUC-CONTAINER-VALUE", "InvokePing"),
                |_| String::new(),
            );
            xml_edit(
                &mut changed,
                "process.arxml",
                |node| named(node, "RUNNABLE-ENTITY", "ServerPing"),
                |_| String::new(),
            );
        }
        rejects_in_both(&changed, code);
    }
}

#[test]
fn argument_policy_explicit_task_flags_offset_and_concurrency_are_checked() {
    let mut sources = inputs();
    change(&mut sources, "types.arxml", "USE-ARGUMENT-TYPE", "USE-VOID");
    rejects_in_both(&sources, "SERVICE_TYPE_CONFLICT");
    for prefix in ["Rte", "RteBsw"] {
        for remove in [false, true] {
            let mut sources = inputs();
            xml_edit(
                &mut sources,
                "ecuc.arxml",
                |node| {
                    node.tag_name().name() == "ECUC-NUMERICAL-PARAM-VALUE"
                        && node.children().any(|child| {
                            child.text().is_some_and(|text| {
                                text.ends_with(&format!("/{prefix}EventIsMappedToTask"))
                            })
                        })
                        && node.children().any(|child| child.text() == Some("true"))
                },
                |original| {
                    if remove {
                        String::new()
                    } else {
                        original.replace("<VALUE>true</VALUE>", "<VALUE>false</VALUE>")
                    }
                },
            );
            rejects_in_both(&sources, "INSTANCE_MAPPING");
        }
    }
    for name in ["Periodic", "ServerTransform"] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "process.arxml",
            |node| named(node, "RUNNABLE-ENTITY", name),
            |original| {
                original.replace(
                    "<CAN-BE-INVOKED-CONCURRENTLY>false",
                    "<CAN-BE-INVOKED-CONCURRENTLY>true",
                )
            },
        );
        rejects_in_both(&sources, "REENTRANCY_UNSUPPORTED");
    }
    let mut sources = inputs();
    change(
        &mut sources,
        "process.arxml",
        "<PERIOD>0.01</PERIOD>",
        "<OFFSET>0</OFFSET><PERIOD>0.01</PERIOD>",
    );
    assert!(build(&sources).is_ok());
    assert!(normal_validation(&sources).diagnostics.is_empty());
    change(
        &mut sources,
        "process.arxml",
        "<OFFSET>0</OFFSET>",
        "<OFFSET>0.001</OFFSET>",
    );
    rejects_in_both(&sources, "PERIOD_UNSUPPORTED");
}

#[test]
fn contract_provenance_preserves_complete_rule_and_extension_identities() {
    let plan = build(&inputs()).unwrap();
    let files = plan.component_contract_files().unwrap();
    let json: serde_json::Value = serde_json::from_slice(
        &files
            .files()
            .iter()
            .find(|(path, _)| path == "contract.json")
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(
        json["ruleSetIdentity"],
        serde_json::to_value(&plan.description().rule_set_identity).unwrap()
    );
    assert_eq!(
        json["requiredExtensionDefinitions"],
        serde_json::to_value(&plan.description().required_extension_definitions).unwrap()
    );
    assert_eq!(
        json["validationDependencies"],
        serde_json::to_value(&plan.description().validation_dependencies).unwrap()
    );
}

#[cfg(feature = "official-oracles")]
#[test]
fn zero_offset_and_multiple_operations_are_accepted_by_official_and_native_inputs() {
    use autosar_config_core::integration::{PlanDependencies, build_plan};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut sources = multi_operation_inputs();
    change(
        &mut sources,
        "process.arxml",
        "<PERIOD>0.01</PERIOD>",
        "<OFFSET>0</OFFSET><PERIOD>0.01</PERIOD>",
    );
    let official = build_plan(
        &sources,
        &PlanDependencies::from_repository(root),
        &RuntimeCatalog::from_repository(root).unwrap(),
    )
    .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert!(normal_validation(&sources).diagnostics.is_empty());
    let native = build(&sources).unwrap();
    let official_files = official.component_contract_files().unwrap();
    let native_files = native.component_contract_files().unwrap();
    assert_eq!(
        official_files
            .files()
            .iter()
            .filter(|(path, _)| path.ends_with(".h"))
            .collect::<Vec<_>>(),
        native_files
            .files()
            .iter()
            .filter(|(path, _)| path.ends_with(".h"))
            .collect::<Vec<_>>()
    );
    change(
        &mut sources,
        "process.arxml",
        "<OFFSET>0</OFFSET>",
        "<OFFSET>0.001</OFFSET>",
    );
    let errors = build_plan(
        &sources,
        &PlanDependencies::from_repository(root),
        &RuntimeCatalog::from_repository(root).unwrap(),
    )
    .err()
    .unwrap();
    assert!(
        errors
            .iter()
            .any(|issue| issue.code == "PERIOD_UNSUPPORTED"),
        "{errors:?}"
    );
}

#[test]
fn unsupported_edit_and_handoff_preserve_live_inputs_and_existing_output() {
    use autosar_config_core::{Workspace, integration::IntegrationEdit, target::BuildTarget};
    let scratch = Scratch::new();
    let source_directory = scratch.0.join("source");
    std::fs::create_dir(&source_directory).unwrap();
    let inputs = inputs();
    let paths: Vec<_> = inputs
        .iter()
        .map(|source| {
            let path = source_directory.join(source.logical_path());
            std::fs::write(&path, source.bytes()).unwrap();
            path
        })
        .collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let mut workspace = Workspace::open(paths.clone()).unwrap();
    let before = serde_json::to_value(&workspace.project_projection("same").unwrap()).unwrap();
    let errors = workspace
        .edit_integration(
            &runtime,
            IntegrationEdit {
                application_period_ms: Some(20),
                ..IntegrationEdit::default()
            },
        )
        .err()
        .unwrap();
    assert!(errors.iter().any(|issue| issue.code == "EDIT_UNSUPPORTED"));
    assert_eq!(
        before,
        serde_json::to_value(&workspace.project_projection("same").unwrap()).unwrap()
    );
    let plan = build(&inputs).unwrap();
    let contract = plan.component_contract_files().unwrap();
    let output = scratch.0.join("contract");
    let preview = contract.preview(&output).unwrap();
    contract
        .generate_previewed(&output, &preview.revision)
        .unwrap();
    assert!(
        plan.ecu_handoff_files(BuildTarget::LinuxX64ControlledV1)
            .is_err()
    );
    assert!(
        plan.ecu_integration_files(BuildTarget::LinuxX64ControlledV1)
            .is_err()
    );
    for (file, bytes) in contract.files() {
        assert_eq!(&std::fs::read(output.join(file)).unwrap(), bytes);
    }
    for (source, path) in inputs.iter().zip(paths) {
        assert_eq!(std::fs::read(path).unwrap(), source.bytes());
    }
}

#[test]
fn type_categories_are_required_with_valid_references_in_both_entrances() {
    for category in [None, Some("VALUE")] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "types.arxml",
            |node| node.tag_name().name() == "IMPLEMENTATION-DATA-TYPE-ELEMENT",
            |original| {
                original.replace(
                    "<CATEGORY>TYPE_REFERENCE</CATEGORY>",
                    &category
                        .map(|value| format!("<CATEGORY>{value}</CATEGORY>"))
                        .unwrap_or_default(),
                )
            },
        );
        rejects_in_both(&sources, "SERVICE_TYPE_CONFLICT");
    }
    for base in ["Base_uint8", "Base_uint32"] {
        for category in [None, Some("VARIABLE_LENGTH")] {
            let mut sources = inputs();
            xml_edit(
                &mut sources,
                "types.arxml",
                |node| named(node, "SW-BASE-TYPE", base),
                |original| {
                    original.replace(
                        "<CATEGORY>FIXED_LENGTH</CATEGORY>",
                        &category
                            .map(|value| format!("<CATEGORY>{value}</CATEGORY>"))
                            .unwrap_or_default(),
                    )
                },
            );
            rejects_in_both(&sources, "TYPE_CONFLICT");
        }
    }
}

#[test]
fn flat_signal_context_and_ecu_members_are_unique_and_nonempty() {
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| node.tag_name().name() == "DATA-ELEMENT-IREF",
        |original| {
            original.replace("</CONTEXT-COMPONENT-REF>", "</CONTEXT-COMPONENT-REF><CONTEXT-COMPONENT-REF DEST=\"SW-COMPONENT-PROTOTYPE\">/Application/Pipeline/ProcessInstance</CONTEXT-COMPONENT-REF>")
        },
    );
    rejects_in_both(&sources, "REFERENCE_UNRESOLVED");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| node.tag_name().name() == "COMPONENT-IREF",
        |original| format!("{original}{original}"),
    );
    rejects_in_both(&sources, "ECU_MAPPING");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| node.tag_name().name() == "SWC-TO-ECU-MAPPING",
        |original| {
            format!(
                "{original}<SWC-TO-ECU-MAPPING><SHORT-NAME>Empty</SHORT-NAME><ECU-INSTANCE-REF DEST=\"ECU-INSTANCE\">/Extract/ReferenceEcu</ECU-INSTANCE-REF></SWC-TO-ECU-MAPPING>"
            )
        },
    );
    rejects_in_both(&sources, "ECU_MAPPING");
}

#[test]
fn multi_network_zero_timeout_matches_without_changing_runtime_scope() {
    let mut sources = inputs();
    change(
        &mut sources,
        "ingress.arxml",
        "<ALIVE-TIMEOUT>0.03</ALIVE-TIMEOUT>",
        "<ALIVE-TIMEOUT>0</ALIVE-TIMEOUT>",
    );
    parameter(&mut sources, "RxValue", "ComTimeout", "0");
    assert!(normal_validation(&sources).diagnostics.is_empty());
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(
        plan.description()
            .signals
            .iter()
            .find(|signal| signal.receive)
            .unwrap()
            .deadline_ms,
        Some(0)
    );
    parameter(&mut sources, "RxValue", "ComTimeout", "0.02");
    rejects_in_both(&sources, "TIMEOUT_CONFLICT");
}

fn legacy_inputs() -> Vec<InputSource> {
    let scratch = Scratch::new();
    let preview = autosar_config_core::Workspace::preview_project_creation(
        &scratch.0.join("legacy"),
        "Legacy",
        "standard-ecu-v1",
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

fn legacy_execution_variants() -> Vec<Vec<InputSource>> {
    let mut result = Vec::new();
    let mut sources = legacy_inputs();
    change(
        &mut sources,
        "application.arxml",
        "<PERIOD>0.01</PERIOD>",
        "<OFFSET>0.001</OFFSET><PERIOD>0.01</PERIOD>",
    );
    result.push(sources);
    for file in ["application.arxml", "services.arxml"] {
        let source = legacy_inputs();
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
            let mut sources = legacy_inputs();
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

#[test]
fn legacy_native_rejects_nonzero_execution_constraints_and_keeps_exact_zero() {
    for (index, sources) in legacy_execution_variants().into_iter().enumerate() {
        let expected = if index == 0 {
            "OFFSET_UNSUPPORTED"
        } else {
            "MINIMUM_START_INTERVAL_UNSUPPORTED"
        };
        let issues = build(&sources)
            .err()
            .expect("legacy execution constraint accepted");
        assert!(
            issues.iter().any(|issue| issue.code == expected),
            "{issues:?}"
        );
        let mut zero = sources;
        replace_all(&mut zero, "<OFFSET>0.001</OFFSET>", "<OFFSET>0e1</OFFSET>");
        replace_all(
            &mut zero,
            "<MINIMUM-START-INTERVAL>0.001</MINIMUM-START-INTERVAL>",
            "<MINIMUM-START-INTERVAL>0.000</MINIMUM-START-INTERVAL>",
        );
        assert!(build(&zero).is_ok());
        assert!(normal_validation(&zero).diagnostics.is_empty());
    }
}

#[cfg(feature = "official-oracles")]
#[test]
fn legacy_official_legal_inputs_reject_nonzero_execution_constraints() {
    use autosar_config_core::integration::{PlanDependencies, build_plan};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    for (index, sources) in legacy_execution_variants().into_iter().enumerate() {
        let expected = if index == 0 {
            "OFFSET_UNSUPPORTED"
        } else {
            "MINIMUM_START_INTERVAL_UNSUPPORTED"
        };
        let issues = build_plan(&sources, &dependencies, &runtime)
            .err()
            .expect("legacy constraint accepted");
        assert!(
            issues.iter().any(|issue| issue.code == expected),
            "{issues:?}"
        );
    }
}

#[test]
fn serialized_symbol_consumers_include_all_runnable_accesses_and_client_callers() {
    let mut sources = inputs();
    let accesses = format!(
        "<DATA-RECEIVE-POINT-BY-ARGUMENTS>{}</DATA-RECEIVE-POINT-BY-ARGUMENTS>",
        subtree(&sources, "process.arxml", "VARIABLE-ACCESS", "RValue")
    );
    xml_edit(
        &mut sources,
        "process.arxml",
        |node| named(node, "RUNNABLE-ENTITY", "ServerTransform"),
        |original| original.replace("<SYMBOL>", &format!("{accesses}<SYMBOL>")),
    );
    xml_edit(
        &mut sources,
        "observe.arxml",
        |node| named(node, "RUNNABLE-ENTITY", "Periodic"),
        |original| {
            format!(
                "{original}{}",
                original
                    .replace(
                        "<SHORT-NAME>Periodic</SHORT-NAME>",
                        "<SHORT-NAME>ExtraPeriodic</SHORT-NAME>"
                    )
                    .replace(
                        "<SYMBOL>Observe_Periodic</SYMBOL>",
                        "<SYMBOL>Observe_ExtraPeriodic</SYMBOL>"
                    )
            )
        },
    );
    xml_edit(
        &mut sources,
        "observe.arxml",
        |node| named(node, "TIMING-EVENT", "Periodic10ms"),
        |original| {
            format!(
                "{original}{}",
                original
                    .replace("Periodic10ms", "Extra10ms")
                    .replace("/Behavior/Periodic", "/Behavior/ExtraPeriodic")
            )
        },
    );
    parameter(&mut sources, "Transmit10ms", "RteBswPositionInTask", "8");
    parameter(&mut sources, "Dcm", "RteBswPositionInTask", "9");
    for (name, position) in [
        ("Can_MainFunction_Read", "10"),
        ("Can_MainFunction_Write", "11"),
        ("Can_MainFunction_Mode", "12"),
        ("Can_MainFunction_BusOff", "13"),
    ] {
        parameter(&mut sources, name, "RteBswPositionInTask", position);
    }
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "Periodic10ms")
                && node
                    .ancestors()
                    .any(|ancestor| named(ancestor, "ECUC-CONTAINER-VALUE", "Observe"))
        },
        |original| {
            format!(
                "{original}{}",
                original
                    .replace(
                        "<SHORT-NAME>Periodic10ms</SHORT-NAME>",
                        "<SHORT-NAME>Extra10ms</SHORT-NAME>"
                    )
                    .replace(
                        "/Observe/Behavior/Periodic10ms",
                        "/Observe/Behavior/Extra10ms"
                    )
                    .replace("<VALUE>6</VALUE>", "<VALUE>7</VALUE>")
            )
        },
    );
    assert!(normal_validation(&sources).diagnostics.is_empty());
    let files = build(&sources).unwrap().component_contract_files().unwrap();
    let json: serde_json::Value = serde_json::from_slice(
        &files
            .files()
            .iter()
            .find(|(file, _)| file == "contract.json")
            .unwrap()
            .1,
    )
    .unwrap();
    for (symbol, expected) in [
        (
            "Rte_Application_Pipeline_ProcessInstance_Read_Value_Value",
            vec![
                "/Application/Process/Behavior/Periodic",
                "/Application/Process/Behavior/ServerTransform",
            ],
        ),
        (
            "Rte_Application_Pipeline_ObserveInstance_Call_ResultService_Transform",
            vec![
                "/Application/Observe/Behavior/ExtraPeriodic",
                "/Application/Observe/Behavior/Periodic",
            ],
        ),
    ] {
        let actual = json["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["symbol"] == symbol)
            .unwrap()["consumers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry.as_str().unwrap())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual, expected.into_iter().collect());
    }
}

#[test]
fn normal_legacy_definition_validation_rejects_only_new_nonzero_execution_constraints() {
    assert!(normal_validation(&legacy_inputs()).diagnostics.is_empty());
    let expected = [
        (
            "OFFSET_UNSUPPORTED",
            "application.arxml",
            "/Application/EchoApplication/Behavior/Periodic10ms",
        ),
        (
            "MINIMUM_START_INTERVAL_UNSUPPORTED",
            "application.arxml",
            "/Application/EchoApplication/Behavior/PeriodicRunnable",
        ),
        (
            "MINIMUM_START_INTERVAL_UNSUPPORTED",
            "application.arxml",
            "/Application/EchoApplication/Behavior/ReadDataRunnable",
        ),
        (
            "MINIMUM_START_INTERVAL_UNSUPPORTED",
            "services.arxml",
            "/Services/DcmService/Behavior/DcmReadData",
        ),
    ];
    let cases = legacy_execution_variants();
    assert_eq!(cases.len(), expected.len());
    for (mut sources, (code, file, path)) in cases.into_iter().zip(expected) {
        let validation = normal_validation(&sources);
        assert!(
            validation.diagnostics.iter().any(|issue| issue.code == code
                && issue.file.as_deref() == Some(file)
                && issue.path.as_deref() == Some(path)),
            "{validation:?}"
        );
        let issues = build(&sources).err().unwrap();
        assert!(issues.iter().any(|issue| issue.code == code), "{issues:?}");
        replace_all(&mut sources, "<OFFSET>0.001</OFFSET>", "<OFFSET>0</OFFSET>");
        replace_all(
            &mut sources,
            "<MINIMUM-START-INTERVAL>0.001</MINIMUM-START-INTERVAL>",
            "<MINIMUM-START-INTERVAL>0</MINIMUM-START-INTERVAL>",
        );
        assert!(normal_validation(&sources).diagnostics.is_empty());
        assert!(build(&sources).is_ok());
    }
    // A capability flag does not change the actual historical graph.
    // Definition editing stays valid; native generation keeps its own refusal.
    let mut unrelated = legacy_inputs();
    change(
        &mut unrelated,
        "application.arxml",
        "<SUPPORTS-MULTIPLE-INSTANTIATION>false",
        "<SUPPORTS-MULTIPLE-INSTANTIATION>true",
    );
    let validation = normal_validation(&unrelated);
    assert!(validation.diagnostics.is_empty());
    assert_eq!(
        validation.status,
        autosar_config_core::project_model::ValidationStatus::Unsupported
    );
    assert!(validation.coverage.iter().any(|rule| rule.rule_id
        == "native.definition.legacy-dcm-mode-dependency"
        && !rule.supported));
    assert!(
        build(&unrelated)
            .err()
            .unwrap()
            .iter()
            .any(|issue| issue.code == "MULTIPLE_INSTANCES")
    );
}

#[test]
fn communication_types_and_polling_periods_are_source_derived() {
    let plan = build(&inputs()).unwrap_or_else(|issues| panic!("{issues:?}"));
    let runtime = plan.description().communication_runtime.as_ref().unwrap();
    let json = serde_json::to_value(runtime).unwrap();
    assert_eq!(json["pduIdType"], "UINT16");
    assert_eq!(json["pduLengthType"], "UINT16");
    assert_eq!(
        runtime.can.read_write_period,
        "/Configuration/Can/General/Polling"
    );
    assert_eq!(runtime.can.read_write_period_ms, 1);
    assert_eq!(runtime.can.controller_id, 0);
    assert_eq!(runtime.can.can_if_controller_id, 0);
    assert_eq!(runtime.can.receive_handle, 0);
    assert_eq!(runtime.can.transmit_handle, 1);
    assert_eq!(runtime.can.busoff_period_ms, 1);
    assert_eq!(runtime.can.mode_period_ms, 1);
    assert!(runtime.can.receive_polling && runtime.can.transmit_polling);
    assert!(runtime.can.busoff_polling && runtime.can.wakeup_polling);
    let mut sources = inputs();
    parameter(&mut sources, "Pdus", "PduIdTypeEnum", "UINT8");
    parameter(&mut sources, "Pdus", "PduLengthTypeEnum", "UINT32");
    replace_all(&mut sources, "/General/Polling", "/General/PollCycle");
    change(
        &mut sources,
        "ecuc.arxml",
        "<SHORT-NAME>Polling</SHORT-NAME>",
        "<SHORT-NAME>PollCycle</SHORT-NAME>",
    );
    parameter(&mut sources, "Receive", "CanObjectId", "1");
    parameter(&mut sources, "Transmit", "CanObjectId", "0");
    let renamed = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    let runtime = renamed
        .description()
        .communication_runtime
        .as_ref()
        .unwrap();
    let json = serde_json::to_value(runtime).unwrap();
    assert_eq!(json["pduIdType"], "UINT8");
    assert_eq!(json["pduLengthType"], "UINT32");
    assert!(runtime.can.read_write_period.ends_with("/PollCycle"));
    assert_eq!(runtime.can.controller_id, 0);
    assert_eq!(runtime.can.can_if_controller_id, 0);
    assert_eq!(runtime.can.receive_handle, 1);
    assert_eq!(runtime.can.transmit_handle, 0);
    assert!(normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn multi_communication_rejects_truncation_and_unbound_polling_in_both_entries() {
    for (owner, field, new_value, code) in [
        (
            "Polling",
            "CanMainFunctionPeriod",
            "0.002",
            "CAN_POLLING_TIMEBASE",
        ),
        (
            "General",
            "CanMainFunctionBusoffPeriod",
            "0.002",
            "CAN_POLLING_TIMEBASE",
        ),
        (
            "General",
            "CanMainFunctionModePeriod",
            "0.002",
            "CAN_POLLING_TIMEBASE",
        ),
        (
            "Controller",
            "CanBusoffProcessing",
            "INTERRUPT",
            "CAN_PROCESSING",
        ),
        (
            "General",
            "CanDevErrorDetect",
            "true",
            "CAN_FEATURE_UNSUPPORTED",
        ),
        ("Transmit", "CanObjectId", "0", "CAN_HARDWARE_HANDLES"),
        ("Transmit", "CanObjectId", "17", "CAN_HARDWARE_HANDLES"),
        ("Receive", "CanObjectId", "17", "CAN_HARDWARE_HANDLES"),
        ("Controller", "CanControllerId", "7", "CAN_CONTROLLER_ID"),
        ("Controller", "CanIfCtrlId", "9", "CAN_CONTROLLER_ID"),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, owner, field, new_value);
        rejects_in_both(&sources, code);
    }
    let mut sources = inputs();
    parameter(&mut sources, "Pdus", "PduIdTypeEnum", "UINT8");
    parameter(&mut sources, "RxValue", "CanIfRxPduId", "256");
    rejects_in_both(&sources, "COMMUNICATION_HANDLE_RANGE");
    let mut sources = inputs();
    parameter(&mut sources, "Pdus", "PduLengthTypeEnum", "UINT8");
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "RxValuePdu"),
        |text| {
            format!(
                "{}\n{}",
                text,
                text.replace(
                    "<SHORT-NAME>RxValuePdu</SHORT-NAME>",
                    "<SHORT-NAME>AdditionalPdu</SHORT-NAME>"
                )
                .replace("<VALUE>4</VALUE>", "<VALUE>256</VALUE>")
            )
        },
    );
    rejects_in_both(&sources, "COMMUNICATION_LENGTH_RANGE");
    let mut sources = inputs();
    change(
        &mut sources,
        "ecuc.arxml",
        "/Configuration/Can/General/Polling</VALUE-REF>",
        "/Configuration/Can/General</VALUE-REF>",
    );
    rejects_in_both(&sources, "CAN_POLLING_REFERENCE");
}

#[test]
fn mode_configuration_preserves_channel_users_rules_and_static_callouts() {
    let plan = build(&inputs()).unwrap();
    let mode = plan.description().mode_runtime.as_ref().unwrap();
    assert_eq!(mode.channel, "/Configuration/ComM/Config/Host");
    assert_eq!(mode.channel_handle, 0);
    assert_eq!(mode.main_symbol, "ComM_MainFunction_Host");
    assert_eq!((mode.period_ms, mode.minimum_full_ms), (1, 5));
    assert_eq!(mode.users.len(), 1);
    assert_eq!(mode.ecu_group_classification, 3);
    assert_eq!(mode.users[0].handle, 0);
    assert_eq!(mode.include, "Ecu_HostBusSM.h");
    assert_eq!(mode.initial_mode, 0);
    assert_eq!(
        mode.rules.iter().map(|rule| rule.mode).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(mode.dcm_connections.len(), 1);
    assert!(mode.rules.iter().all(|rule| {
        rule.callout
            .starts_with("Ecu_HostBusSM_ApplyMode(0u, COMM_")
    }));
    let mut sources = inputs();
    replace_all(&mut sources, "/Config/Host/", "/Config/Vehicle/");
    change(
        &mut sources,
        "ecuc.arxml",
        "<SHORT-NAME>Host</SHORT-NAME>",
        "<SHORT-NAME>Vehicle</SHORT-NAME>",
    );
    replace_all(&mut sources, "/Config/Host<", "/Config/Vehicle<");
    replace_all(
        &mut sources,
        "ComM_MainFunction_Host",
        "ComM_MainFunction_Vehicle",
    );
    parameter(&mut sources, "Vehicle", "ComMChannelId", "7");
    parameter(&mut sources, "HostUser", "ComMUserIdentifier", "19");
    parameter(&mut sources, "General", "ComMEcuGroupClassification", "1");
    replace_all(
        &mut sources,
        "Ecu_HostBusSM_ApplyMode(0u,",
        "Ecu_HostBusSM_ApplyMode(7u,",
    );
    let renamed = build(&sources).unwrap();
    let mode = renamed.description().mode_runtime.as_ref().unwrap();
    assert_eq!(mode.channel, "/Configuration/ComM/Config/Vehicle");
    assert_eq!(mode.main_symbol, "ComM_MainFunction_Vehicle");
    assert_eq!(mode.channel_handle, 7);
    assert_eq!(mode.users[0].handle, 19);
    assert_eq!(mode.ecu_group_classification, 1);
    assert!(normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn selected_mode_configuration_rejects_unbound_features_and_dynamic_callouts() {
    for (owner, field, new_value, code) in [
        (
            "Host",
            "ComMBusType",
            "COMM_BUS_TYPE_CAN",
            "MODE_BUS_PROVIDER",
        ),
        (
            "Host",
            "ComMCDDBusPrefix",
            "OtherBusSM",
            "MODE_BUS_PROVIDER",
        ),
        (
            "NetworkManagement",
            "ComMNmVariant",
            "FULL",
            "MODE_NM_VARIANT",
        ),
        ("Host", "ComMMainFunctionPeriod", "0.002", "MODE_TIMEBASE"),
        (
            "CurrentMode",
            "BswMRequestProcessing",
            "BSWM_DEFERRED",
            "MODE_PROCESSING",
        ),
        (
            "Includes",
            "BswMUserIncludeFile",
            "../Ecu_HostBusSM.h",
            "MODE_INCLUDE",
        ),
        (
            "NoRule",
            "BswMNestedExecutionOnly",
            "true",
            "MODE_FEATURE_UNSUPPORTED",
        ),
        (
            "NoList",
            "BswMActionListExecution",
            "BSWM_TRIGGER",
            "MODE_RULE",
        ),
        (
            "FullAction",
            "BswMUserCalloutFunction",
            "Ecu_HostBusSM_ApplyMode(0u, currentMode)",
            "MODE_STATIC_CALLOUT",
        ),
        (
            "SilentAction",
            "BswMUserCalloutFunction",
            "Ecu_HostBusSM_ApplyMode(0u, COMM_FULL_COMMUNICATION)",
            "MODE_STATIC_CALLOUT",
        ),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, owner, field, new_value);
        rejects_in_both(&sources, code);
    }
}

#[test]
fn required_dcm_channel_reference_has_only_verified_historical_compatibility() {
    use autosar_config_core::project_model::ValidationStatus;
    let legacy = legacy_inputs();
    let validation = normal_validation(&legacy);
    assert!(validation.diagnostics.is_empty());
    assert_eq!(validation.status, ValidationStatus::Unsupported);
    assert!(validation.coverage.iter().any(|rule| rule.rule_id
        == "native.definition.legacy-dcm-mode-dependency"
        && !rule.supported));
    let plan = build(&legacy).unwrap();
    assert_eq!(
        plan.description().profile,
        autosar_config_core::integration::PROFILE
    );
    assert!(plan.description().mode_runtime.is_none());
    assert!(
        plan.ecu_integration_files(autosar_config_core::target::BuildTarget::LinuxX64ControlledV1)
            .is_ok()
    );
    let mut multi = inputs();
    xml_edit(
        &mut multi,
        "ecuc.arxml",
        |node| {
            node.tag_name().name() == "ECUC-REFERENCE-VALUE"
                && node.children().any(|child| {
                    child.tag_name().name() == "DEFINITION-REF"
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/DcmDslProtocolComMChannelRef"))
                })
        },
        |_| String::new(),
    );
    assert!(
        normal_validation(&multi)
            .diagnostics
            .iter()
            .any(|issue| issue.code == "MULTIPLICITY")
    );
    assert!(build(&multi).is_err());
    // An explicit new mode module is a new configuration, even when the
    // remaining files retain the historical single-component shape.
    for module in ["ComM", "BswM"] {
        let mut sources = legacy.clone();
        change(
            &mut sources,
            "ecuc.arxml",
            "</ELEMENTS>",
            &format!(
                "<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>{module}</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-MODULE-DEF\">/AUTOSAR/EcucDefs/{module}</DEFINITION-REF></ECUC-MODULE-CONFIGURATION-VALUES></ELEMENTS>"
            ),
        );
        assert!(
            normal_validation(&sources)
                .diagnostics
                .iter()
                .any(|issue| issue.code == "MULTIPLICITY"
                    && issue.witness.as_ref().is_some_and(|witness| format!(
                        "{:?}",
                        witness.constraint
                    )
                    .contains("DcmDslProtocolComMChannelRef")))
        );
    }
    let source = legacy
        .iter()
        .find(|source| source.logical_path() == "ecuc.arxml")
        .unwrap();
    let text = std::str::from_utf8(source.bytes()).unwrap();
    let document = roxmltree::Document::parse(text).unwrap();
    let dcm = document
        .descendants()
        .find(|node| named(*node, "ECUC-MODULE-CONFIGURATION-VALUES", "Dcm"))
        .unwrap();
    let standalone = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Configuration</SHORT-NAME><ELEMENTS>{}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>",
        &text[dcm.range()]
    );
    let standalone = [InputSource::new("dcm.arxml", standalone.into_bytes()).unwrap()];
    let validation = normal_validation(&standalone);
    assert!(
        validation
            .diagnostics
            .iter()
            .any(|issue| issue.code == "MULTIPLICITY"
                && issue.witness.as_ref().is_some_and(|witness| format!(
                    "{:?}",
                    witness.constraint
                )
                .contains("DcmDslProtocolComMChannelRef"))),
        "{:?}",
        validation.diagnostics
    );
}

#[test]
fn historical_profile_identity_uses_bindings_instead_of_runnable_capabilities() {
    let mut concurrent = legacy_inputs();
    change(
        &mut concurrent,
        "application.arxml",
        "<CAN-BE-INVOKED-CONCURRENTLY>false",
        "<CAN-BE-INVOKED-CONCURRENTLY>true",
    );
    let validation = normal_validation(&concurrent);
    assert!(validation.diagnostics.is_empty());
    assert_eq!(
        validation.status,
        autosar_config_core::project_model::ValidationStatus::Unsupported
    );
    let issues = build(&concurrent).err().unwrap();
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "REENTRANCY_UNSUPPORTED"),
        "{issues:?}"
    );
    assert!(!issues.iter().any(|issue| issue.code == "MULTIPLICITY"));
    let mut wrong_rte = legacy_inputs();
    xml_edit(
        &mut wrong_rte,
        "ecuc.arxml",
        |node| {
            node.tag_name().name() == "VALUE-REF"
                && node.parent().is_some_and(|parent| {
                    parent.children().any(|child| {
                        child.tag_name().name() == "DEFINITION-REF"
                            && child.text().is_some_and(|text| {
                                text.ends_with("/RteSoftwareComponentInstanceRef")
                            })
                    })
                })
        },
        |_| {
            "<VALUE-REF DEST=\"SW-COMPONENT-PROTOTYPE\">/Application/ReferenceComposition/DcmService</VALUE-REF>".into()
        },
    );
    let mut unmapped = legacy_inputs();
    xml_edit(
        &mut unmapped,
        "extract.arxml",
        |node| {
            node.tag_name().name() == "COMPONENT-IREF"
                && node.children().any(|child| {
                    child.tag_name().name() == "TARGET-COMPONENT-REF"
                        && child.text() == Some("/Application/ReferenceComposition/EchoApplication")
                })
        },
        |_| String::new(),
    );
    for sources in [wrong_rte, unmapped] {
        let validation = normal_validation(&sources);
        assert!(
            validation
                .diagnostics
                .iter()
                .any(|issue| issue.code == "MULTIPLICITY"
                    && issue.witness.as_ref().is_some_and(|witness| format!(
                        "{:?}",
                        witness.constraint
                    )
                    .contains("DcmDslProtocolComMChannelRef"))),
            "{:?}",
            validation.diagnostics
        );
        assert!(
            !validation
                .coverage
                .iter()
                .any(|rule| rule.rule_id == "native.definition.legacy-dcm-mode-dependency")
        );
    }
}

#[test]
fn multi_internal_active_session_did_cannot_be_replaced_by_application_source() {
    let mut sources = inputs();
    parameter(
        &mut sources,
        "ApplicationDid",
        "DcmDspDidIdentifier",
        "61830",
    );
    rejects_in_both(&sources, "DIAGNOSTIC_IDENTIFIER");
    let issues = build(&sources).err().expect("reserved DID refused");
    let message = issues
        .iter()
        .find(|issue| issue.code == "DIAGNOSTIC_IDENTIFIER")
        .unwrap()
        .message
        .to_string();
    assert!(
        message.contains("0xF186") && message.contains("reserved") && message.contains("internal"),
        "{message}"
    );
    let validation = normal_validation(&sources);
    let message = validation
        .diagnostics
        .iter()
        .find(|issue| issue.code == "DIAGNOSTIC_IDENTIFIER")
        .unwrap()
        .message
        .to_string();
    assert!(
        message.contains("0xF186") && message.contains("reserved") && message.contains("internal"),
        "{message}"
    );
    let catalog: serde_json::Value =
        serde_json::from_str(include_str!("../src/messages.json")).unwrap();
    assert!(
        catalog["zh-CN"]["backend.integration.diagnostic.did_identifier_reserved"]
            .as_str()
            .unwrap()
            .contains("保留")
    );
}

#[test]
fn selected_dcm_subfunction_availability_matches_actual_service_dispatch() {
    for (service, available) in [
        ("SessionControl", "false"),
        ("TesterPresent", "false"),
        ("ReadDataByIdentifier", "true"),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, service, "DcmDsdSidTabSubfuncAvail", available);
        rejects_in_both(&sources, "SERVICE_UNSUPPORTED");
    }
}

#[test]
fn selected_com_manual_trigger_requires_zero_minimum_delay_and_no_callout() {
    let mut sources = inputs();
    parameter(&mut sources, "Transmit", "ComMinimumDelayTime", "0.001");
    rejects_in_both(&sources, "COM_FEATURE_UNSUPPORTED");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "TxValuePdu")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text().is_some_and(|text| text.ends_with("/ComIPdu"))
                })
        },
        |text| {
            text.replacen("<PARAMETER-VALUES>", r#"<PARAMETER-VALUES><ECUC-TEXTUAL-PARAM-VALUE><DEFINITION-REF DEST="ECUC-FUNCTION-NAME-DEF">/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu/ComIPduCallout</DEFINITION-REF><VALUE>ApplicationCallout</VALUE></ECUC-TEXTUAL-PARAM-VALUE>"#, 1)
        },
    );
    rejects_in_both(&sources, "COM_FEATURE_UNSUPPORTED");
}

#[test]
fn actual_runtime_source_owners_derive_all_application_slots_and_keep_service_generated() {
    let plan = build(&inputs()).unwrap();
    let slots = plan.application_slot_descriptors().unwrap();
    assert_eq!(slots.len(), 3);
    assert_eq!(
        slots
            .iter()
            .map(|slot| slot.source_paths[0].as_str())
            .collect::<Vec<_>>(),
        [
            "application/Ingress.c",
            "application/Process.c",
            "application/Observe.c"
        ]
    );
    for slot in &slots {
        assert!(
            slot.producer_slot
                .starts_with("singlecore-multi-swc-v1:/Application/Pipeline/")
        );
        assert!(!slot.entry_symbols.is_empty());
        assert!(
            slot.entry_symbols.iter().all(|entry| plan
                .description()
                .symbols
                .iter()
                .any(|symbol| symbol.symbol == *entry
                    && symbol.definition_owner == slot.source_paths[0]))
        );
    }
    assert!(
        plan.description()
            .symbols
            .iter()
            .any(|symbol| symbol.symbol == "DcmService_ReadData"
                && symbol.definition_owner == "src/Rte.c")
    );
    let files = prepared_multi_files(&plan);
    assert!(files.iter().any(|(path, _)| path == "src/Rte.c"));
}

#[test]
fn component_memory_scopes_cannot_alias_selected_runtime_producers() {
    for name in [
        "Rte",
        "Ecu",
        "Com",
        "ComM",
        "Can",
        "CanIf",
        "CanTp",
        "Dcm",
        "PduR",
        "LSduR",
        "BswM",
        "Ecu_HostBusSM",
    ] {
        let mut sources = inputs();
        replace_all(
            &mut sources,
            "/Application/Process",
            &format!("/Application/{name}"),
        );
        change(
            &mut sources,
            "process.arxml",
            "<SHORT-NAME>Process</SHORT-NAME>",
            &format!("<SHORT-NAME>{name}</SHORT-NAME>"),
        );
        rejects_in_both(&sources, "CONTRACT_NAME_COLLISION");
        let mut partition_sources = inputs();
        replace_all(&mut partition_sources, "OwnerPartition", name);
        rejects_in_both(&partition_sources, "CONTRACT_NAME_COLLISION");
    }
}

#[test]
fn shared_rte_lifecycle_os_target_kernel_and_c_runtime_producers_cannot_be_shadowed() {
    for symbol in [
        "Rte_Start",
        "Rte_Stop",
        "SchM_Init",
        "SchM_StartTiming",
        "Dcm_GetSesCtrlType",
        "CanTp_CancelReceive",
        "StartOS",
        "ShutdownOS",
        "Ecu_TargetTask",
        "xTaskCreate",
        "Os_TargetPrepare",
        "Arti_Trace",
        "main",
        "memcpy",
        "printf",
        "abort",
        "TaskType",
        "SCHEDULETABLE_STOPPED",
    ] {
        let mut sources = inputs();
        change(
            &mut sources,
            "ingress.arxml",
            "<SYMBOL>Ingress_Periodic</SYMBOL>",
            &format!("<SYMBOL>{symbol}</SYMBOL>"),
        );
        rejects_in_both(&sources, "SYMBOL_PRODUCER_DUPLICATE");
    }
}

#[test]
fn multi_signal_processing_matches_actual_immediate_notification_contract() {
    for pdu in ["RxValuePdu", "TxValuePdu"] {
        let mut sources = inputs();
        parameter(&mut sources, pdu, "ComIPduSignalProcessing", "DEFERRED");
        rejects_in_both(&sources, "COM_FEATURE_UNSUPPORTED");
    }
}

#[test]
fn actual_partition_owns_all_instances_and_all_com_producers() {
    let plan = build(&inputs()).unwrap();
    let partition = &plan
        .description()
        .communication_runtime
        .as_ref()
        .unwrap()
        .partition;
    assert_eq!(partition.name, "OwnerPartition");
    assert_eq!(partition.id, 0);
    assert_eq!(partition.instances.len(), 4);
    assert!(
        partition
            .instances
            .iter()
            .any(|instance| instance.ends_with("/DcmService"))
    );
    let files = prepared_multi_files(&plan);
    let source = String::from_utf8(
        files
            .iter()
            .find(|(name, _)| name == "src/Rte.c")
            .unwrap()
            .1
            .clone(),
    )
    .unwrap();
    assert!(source.contains("RTE_Ingress_START_SEC_CODE"));
    assert!(source.contains("RTE_Observe_START_SEC_VAR_CLEARED_UNSPECIFIED"));
    assert!(source.contains("RTE_OwnerPartition_START_SEC_CALLOUT_CODE"));
    let mut sources = inputs();
    parameter(&mut sources, "OwnerPartition", "EcucPartitionId", "1");
    rejects_in_both(&sources, "RTE_PARTITION_CONFIGURATION");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.has_tag_name("REFERENCE-VALUES")
                && node.descendants().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/RteComUserEcucPartitionRef"))
                })
        },
        |_| String::new(),
    );
    rejects_in_both(&sources, "RTE_PARTITION_CONFIGURATION");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.has_tag_name("ECUC-INSTANCE-REFERENCE-VALUE")
                && node.descendants().any(|child| {
                    child.has_tag_name("TARGET-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/DcmService"))
                })
        },
        |_| String::new(),
    );
    rejects_in_both(&sources, "RTE_PARTITION_CONFIGURATION");
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn initialized_multi_scaffolds_compile_link_and_initialize_only_out_arguments() {
    use autosar_config_core::prepared::prepare_ecu_project_for_workspace;
    let scratch = Scratch::new();
    let live = scratch.0.join("live");
    let mut workspace = live_multi_workspace(&live, &inputs());
    let preview = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&preview)
        .unwrap();
    let plan = workspace
        .saved_integration_plan(&RuntimeCatalog::embedded().unwrap())
        .unwrap();
    let project = scratch.0.join("sealed");
    prepare_ecu_project_for_workspace(&workspace, &plan, tooling::native_target(), true)
        .unwrap()
        .generate(&project)
        .unwrap();
    let build = scratch.0.join("build");
    let compiled = tooling::ecu_build_command(&project, &build, "host-batch", None)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
    let harness = scratch.0.join("scaffold_contract.c");
    std::fs::write(
        &harness,
        r#"#define RTE_CORE
#include "Rte_Process.h"
#include "Rte_Ingress.h"
#include <assert.h>
int main(void) {
    uint32 output = 99u;
    uint32 state = 17u;
    Dcm_DataElement_ApplicationValueType data = {1u, 2u, 3u, 4u};
    void (*scalar_server)(uint32, uint32 *, uint32 *) = &Process_Transform;
    void (*array_server)(Dcm_DataElement_ApplicationValueType) = &Ingress_ReadData;
    scalar_server(23u, &output, &state);
    array_server(data);
    assert(output == 0u);
    assert(state == 17u);
    assert((data[0] == 0u) && (data[1] == 0u) && (data[2] == 0u) && (data[3] == 0u));
    return 0;
}
"#,
    )
    .unwrap();
    let binary = tooling::native_binary(&scratch.0, "scaffold_contract");
    let settings = tooling::execution_settings();
    let compiled = std::process::Command::new(settings.compiler)
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic", "-I"])
        .arg(project.join("include"))
        .arg(project.join("src/Process.c"))
        .arg(project.join("src/Ingress.c"))
        .arg(project.join("src/Observe.c"))
        .arg(&harness)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    assert!(
        std::process::Command::new(binary)
            .status()
            .unwrap()
            .success()
    );
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn sealed_multi_project_builds_and_runs_production_owner() {
    use autosar_config_core::{Workspace, prepared::prepare_ecu_project_for_workspace};
    for (renamed, swapped, p2_star) in [
        (false, false, None),
        (true, false, None),
        (false, true, Some(("100", "2710"))),
        (false, false, Some(("65.54", "199a"))),
    ] {
        let scratch = Scratch::new();
        let mut arxml = inputs();
        if swapped {
            parameter(&mut arxml, "Receive", "CanObjectId", "1");
            parameter(&mut arxml, "Transmit", "CanObjectId", "0");
        }
        if let Some((seconds, _)) = p2_star {
            for session in ["DCM_DEFAULT_SESSION", "DCM_EXTENDED_DIAGNOSTIC_SESSION"] {
                parameter(&mut arxml, session, "DcmDspSessionP2StarServerMax", seconds);
            }
        }
        if renamed {
            for (before, after) in [
                ("Ingress", "Gateway"),
                ("Process</", "Compute</"),
                ("Process/", "Compute/"),
                ("Process_", "Compute_"),
                ("ProcessInstance", "ComputeInstance"),
                ("ResultService", "Calculation"),
                ("Transform", "Calculate"),
            ] {
                replace_all(&mut arxml, before, after);
            }
            arxml.reverse();
        }
        let live = scratch.0.join("live");
        let mut workspace = live_multi_workspace(&live, &arxml);
        let initialization = workspace.preview_application_initialization().unwrap();
        workspace
            .initialize_application_previewed(&initialization)
            .unwrap();
        let plan = build(&arxml).unwrap();
        for slot in plan.application_slot_descriptors().unwrap().iter() {
            let file = slot.source_paths[0].rsplit('/').next().unwrap();
            let original = file
                .replace("Gateway", "Ingress")
                .replace("Compute", "Process");
            let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/multi-application")
                .join(original);
            let mut text = std::fs::read_to_string(fixture).unwrap();
            if renamed {
                for (before, after) in [
                    ("Ingress", "Gateway"),
                    ("Process", "Compute"),
                    ("ResultService", "Calculation"),
                    ("Transform", "Calculate"),
                ] {
                    text = text.replace(before, after);
                }
            }
            std::fs::write(live.join(&slot.source_paths[0]), text).unwrap();
        }
        let workspace = Workspace::open_project_manifest(
            &live.join("workbench-project.json"),
            &scratch.0.join("cache"),
        )
        .unwrap();
        let plan = workspace
            .saved_integration_plan(&RuntimeCatalog::embedded().unwrap())
            .unwrap();
        let prepared =
            prepare_ecu_project_for_workspace(&workspace, &plan, tooling::native_target(), true)
                .unwrap_or_else(|issues| panic!("{issues:?}"));
        assert_eq!(prepared.application_slots().len(), 3);
        let project = scratch.0.join("multi-project");
        prepared.generate(&project).unwrap();
        let output = scratch.0.join("multi-build");
        let result = tooling::ecu_build_command(&project, &output, "host-batch", None)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        let binary = tooling::native_binary(&output, "ecu_host_batch");
        let logs = scratch.0.join("owner-logs");
        std::fs::create_dir(&logs).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut spec = autosar_config_core::execution::ProcessSpec::for_duration(
            vec![binary.into_os_string()],
            scratch.0.clone(),
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
        let result = process.wait().unwrap();
        assert!(result.success(), "{result:?}");
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
}

#[test]
fn multi_selected_catalog_binds_standard_signatures_and_actual_periodic_producers() {
    let plan = build(&inputs()).unwrap();
    let description = plan.description();
    assert_eq!(
        description
            .schedule
            .entities
            .iter()
            .map(|entity| (entity.position, entity.symbol.as_str()))
            .collect::<Vec<_>>(),
        [
            (0, "ComM_MainFunction_Host"),
            (1, "Can_MainFunction_Wakeup"),
            (2, "CanTp_MainFunction"),
            (3, "Com_MainFunctionRx_Rx"),
            (4, "Ingress_Periodic"),
            (5, "Process_Periodic"),
            (6, "Observe_Periodic"),
            (7, "Com_MainFunctionTx_Tx"),
            (8, "Dcm_MainFunction"),
            (9, "Can_MainFunction_Read"),
            (10, "Can_MainFunction_Write"),
            (11, "Can_MainFunction_Mode"),
            (12, "Can_MainFunction_BusOff"),
        ]
    );
    assert!(
        !description
            .runtime_sources
            .contains_key("runtime/src/Com.c")
    );
    assert!(
        description
            .runtime_sources
            .contains_key("runtime/multi/src/Com.c")
    );
    assert!(
        description
            .runtime_sources
            .contains_key("core/src/integration/multi_ecu.rs")
    );
    for name in [
        "Com_MainFunctionRx_Rx",
        "Com_MainFunctionTx_Tx",
        "ComM_MainFunction_Host",
    ] {
        let symbol = description
            .symbols
            .iter()
            .find(|symbol| symbol.symbol == name)
            .unwrap();
        assert_eq!(symbol.return_type, "void");
        assert!(symbol.arguments.is_empty());
        assert_eq!(symbol.definition_owner, "src/Ecu_RuntimeConfig.c");
    }
    let transmit = description
        .symbols
        .iter()
        .find(|symbol| symbol.symbol == "CanIf_Transmit")
        .unwrap();
    assert_eq!(transmit.return_type, "Std_ReturnType");
    assert_eq!(
        transmit
            .arguments
            .iter()
            .map(|argument| argument.native_type.as_str())
            .collect::<Vec<_>>(),
        ["PduIdType", "const PduInfoType *"]
    );
    for (name, field, old, new) in [
        ("Com_SendSignal", "IS-SYNCHRONOUS", "false", "true"),
        ("Com_ReceiveSignal", "IS-REENTRANT", "true", "false"),
        ("CanIf_Transmit", "IS-REENTRANT", "true", "false"),
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "bsw.arxml",
            |node| named(node, "BSW-MODULE-ENTRY", name),
            |text| {
                text.replace(
                    &format!("<{field}>{old}</{field}>"),
                    &format!("<{field}>{new}</{field}>"),
                )
            },
        );
        rejects_in_both(&sources, "BSW_SIGNATURE_CONFLICT");
    }
    for (field, value) in [
        ("IS-REENTRANT", "false"),
        ("IS-SYNCHRONOUS", "true"),
        ("CALL-TYPE", "REGULAR"),
        ("EXECUTION-CONTEXT", "UNSPECIFIED"),
        ("SW-SERVICE-IMPL-POLICY", "STANDARD"),
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "bsw.arxml",
            |node| named(node, "BSW-MODULE-ENTRY", "Can_Init"),
            |text| text.replace(&format!("<{field}>{value}</{field}>"), ""),
        );
        rejects_in_both(&sources, "BSW_SIGNATURE_CONFLICT");
    }
    for (field, old, new) in [
        ("CALL-TYPE", "REGULAR", "CALLBACK"),
        ("SW-SERVICE-IMPL-POLICY", "STANDARD", "MACRO"),
        ("BSW-ENTRY-KIND", "CONCRETE", "ABSTRACT"),
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "bsw.arxml",
            |node| named(node, "BSW-MODULE-ENTRY", "Can_Init"),
            |text| {
                text.replace(
                    &format!("<{field}>{old}</{field}>"),
                    &format!("<{field}>{new}</{field}>"),
                )
            },
        );
        rejects_in_both(&sources, "BSW_SIGNATURE_CONFLICT");
    }
    for (old, new) in [
        ("<MODULE-ID>80</MODULE-ID>", "<MODULE-ID>60</MODULE-ID>"),
        ("<CATEGORY>BSW_MODULE</CATEGORY>", "<CATEGORY></CATEGORY>"),
    ] {
        let mut sources = inputs();
        change(&mut sources, "bsw.arxml", old, new);
        rejects_in_both(&sources, "BSW_ENTRY_IDENTITY");
    }
    let pdu_router = description
        .symbols
        .iter()
        .find(|symbol| symbol.symbol == "PduR_ComTransmit")
        .unwrap();
    assert_eq!(
        pdu_router.declaration_owner,
        "runtime/multi/include/PduR_Com.h"
    );
    for path in [
        "core/src/integration/ecu.rs",
        "core/src/integration/contracts.rs",
    ] {
        assert!(description.runtime_sources.contains_key(path));
    }
    let mut sources = inputs();
    replace_all(
        &mut sources,
        "AUTOSAR_Com</SHORT-NAME>",
        "Vendor_Com</SHORT-NAME>",
    );
    replace_all(&mut sources, "/AUTOSAR_Com/", "/Vendor_Com/");
    rejects_in_both(&sources, "BSW_ENTRY_IDENTITY");
}

#[test]
fn dcm_service_mode_source_requires_the_fixed_standard_contract() {
    let plan = build(&inputs()).unwrap();
    let service = plan
        .description()
        .multi
        .as_ref()
        .unwrap()
        .components
        .iter()
        .find(|component| component.component == "/Services/DcmService")
        .unwrap();
    let mode = service.diagnostic_session_port.as_ref().unwrap();
    assert_eq!(
        mode.port,
        "/Services/DcmService/DiagnosticSessionControlModeSwitchInterface"
    );
    assert_eq!(
        mode.prototype,
        "/Services/Dcm_DiagnosticSessionControlModeSwitchInterface/diagnosticSession"
    );
    assert_eq!(mode.mode_group, "/AUTOSAR_Dcm/DcmDiagnosticSessionControl");
    for (file, tag, name, before, after) in [
        (
            "services.arxml",
            "MODE-SWITCH-INTERFACE",
            "Dcm_DiagnosticSessionControlModeSwitchInterface",
            "<IS-SERVICE>true</IS-SERVICE>",
            "<IS-SERVICE>false</IS-SERVICE>",
        ),
        (
            "services.arxml",
            "P-PORT-PROTOTYPE",
            "DiagnosticSessionControlModeSwitchInterface",
            "<ENHANCED-MODE-API>false</ENHANCED-MODE-API>",
            "<ENHANCED-MODE-API>true</ENHANCED-MODE-API>",
        ),
        (
            "bsw.arxml",
            "MODE-DECLARATION-GROUP",
            "DcmDiagnosticSessionControl",
            "<ON-TRANSITION-VALUE>255</ON-TRANSITION-VALUE>",
            "<ON-TRANSITION-VALUE>254</ON-TRANSITION-VALUE>",
        ),
        (
            "bsw.arxml",
            "MODE-DECLARATION-GROUP",
            "DcmDiagnosticSessionControl",
            "<CATEGORY>EXPLICIT_ORDER</CATEGORY>",
            "<CATEGORY>ALPHABETIC_ORDER</CATEGORY>",
        ),
        (
            "bsw.arxml",
            "MODE-DECLARATION",
            "DCM_PROGRAMMING_SESSION",
            "<VALUE>1</VALUE>",
            "<VALUE>4</VALUE>",
        ),
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            file,
            |node| named(node, tag, name),
            |text| {
                assert!(text.contains(before));
                text.replace(before, after)
            },
        );
        rejects_in_both(&sources, "TYPE_CONFLICT");
    }
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "bsw.arxml",
        |node| named(node, "AR-PACKAGE", "AUTOSAR_Dcm"),
        |text| {
            text.replacen("</ELEMENTS>", "<MODE-DECLARATION-GROUP><SHORT-NAME>ForeignModes</SHORT-NAME><CATEGORY>EXPLICIT_ORDER</CATEGORY><INITIAL-MODE-REF DEST=\"MODE-DECLARATION\">/AUTOSAR_Dcm/ForeignModes/DCM_DEFAULT_SESSION</INITIAL-MODE-REF><MODE-DECLARATIONS><MODE-DECLARATION><SHORT-NAME>DCM_DEFAULT_SESSION</SHORT-NAME><VALUE>0</VALUE></MODE-DECLARATION></MODE-DECLARATIONS><ON-TRANSITION-VALUE>255</ON-TRANSITION-VALUE></MODE-DECLARATION-GROUP></ELEMENTS>", 1)
        },
    );
    xml_edit(
        &mut sources,
        "bsw.arxml",
        |node| {
            named(
                node,
                "MODE-DECLARATION-GROUP",
                "DcmDiagnosticSessionControl",
            )
        },
        |text| {
            text.replace(
                "/AUTOSAR_Dcm/DcmDiagnosticSessionControl/DCM_DEFAULT_SESSION</INITIAL-MODE-REF>",
                "/AUTOSAR_Dcm/ForeignModes/DCM_DEFAULT_SESSION</INITIAL-MODE-REF>",
            )
        },
    );
    rejects_in_both(&sources, "TYPE_CONFLICT");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "services.arxml",
        |node| {
            named(
                node,
                "P-PORT-PROTOTYPE",
                "SecurityAccessModeSwitchInterface",
            )
        },
        |_| String::new(),
    );
    rejects_in_both(&sources, "SERVICE_TYPE_CONFLICT");
    for original in ["DCM_DEFAULT_SESSION", "DCM_EXTENDED_DIAGNOSTIC_SESSION"] {
        let mut sources = inputs();
        let source = sources
            .iter_mut()
            .find(|source| source.logical_path() == "ecuc.arxml")
            .unwrap();
        // Preserve all reference identities while editing only the ECUC row.
        replace_all(
            std::slice::from_mut(source),
            original,
            &format!("{original}_Changed"),
        );
        rejects_in_both(&sources, "DIAGNOSTIC_SESSION");
    }
}

#[test]
fn plural_application_preparation_freezes_every_real_producer_and_refuses_bad_members() {
    use autosar_config_core::{ApplicationSource, prepare_ecu_project_with_applications};
    let scratch = Scratch::new();
    let plan = build(&inputs()).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi-application");
    let sources: Vec<_> = plan
        .application_slot_descriptors()
        .unwrap()
        .iter()
        .map(|slot| {
            let file = slot.source_paths[0].rsplit('/').next().unwrap();
            let path = scratch.0.join(file);
            std::fs::copy(fixture.join(file), &path).unwrap();
            ApplicationSource {
                component_instance: slot
                    .producer_slot
                    .strip_prefix("singlecore-multi-swc-v1:")
                    .unwrap()
                    .into(),
                path,
            }
        })
        .collect();
    let prepare = |members: &[ApplicationSource]| {
        prepare_ecu_project_with_applications(
            &plan,
            autosar_config_core::target::BuildTarget::LinuxX64ControlledV1,
            members,
        )
    };
    assert!(prepare(&sources[..2]).is_err());
    let mut wrong = sources.clone();
    wrong[1].component_instance = wrong[0].component_instance.clone();
    assert!(prepare(&wrong).is_err());
    wrong = sources.clone();
    wrong[1].path = wrong[0].path.clone();
    assert!(prepare(&wrong).is_err());
    wrong = sources.clone();
    wrong[0].component_instance = "/Application/Pipeline/Unexpected".into();
    assert!(prepare(&wrong).is_err());
    wrong = sources.clone();
    wrong.push(sources[0].clone());
    assert!(prepare(&wrong).is_err());
    let first: std::collections::BTreeMap<_, _> = prepare(&sources)
        .unwrap()
        .into_files()
        .into_iter()
        .collect();
    let mut reversed = sources.clone();
    reversed.reverse();
    assert_eq!(
        first,
        prepare(&reversed)
            .unwrap()
            .into_files()
            .into_iter()
            .collect()
    );
    for source in &sources {
        let file = source.path.file_name().unwrap().to_str().unwrap();
        assert_eq!(
            first[&format!("src/{file}")],
            std::fs::read(&source.path).unwrap()
        );
        assert!(!first.contains_key(&format!("application/{file}")));
    }
    // Each independent caller source is checked again at the normal staging
    // boundary; a stale later member must not publish a partial project.
    for (index, source) in sources.iter().enumerate() {
        let prepared = prepare(&sources).unwrap();
        let original = std::fs::read(&source.path).unwrap();
        let mut changed = original.clone();
        changed.extend_from_slice(b"\n/* changed after preparation */\n");
        std::fs::write(&source.path, changed).unwrap();
        let output = scratch.0.join(format!("stale-{index}"));
        assert!(prepared.generate(&output).is_err());
        assert!(!output.exists());
        std::fs::write(&source.path, original).unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let linked = scratch.0.join("linked.c");
        symlink(&sources[0].path, &linked).unwrap();
        wrong = sources.clone();
        wrong[1].path = linked;
        assert!(prepare(&wrong).is_err());
    }
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn sealed_multi_owner_preserves_network_init_and_deadline_phase() {
    use autosar_config_core::{ApplicationSource, prepare_ecu_project_with_applications};
    let scratch = Scratch::new();
    let mut source = inputs();
    change(
        &mut source,
        "ingress.arxml",
        "<VALUE>0</VALUE>",
        "<VALUE>7</VALUE>",
    );
    let plan = build(&source).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi-application");
    let applications: Vec<_> = plan
        .application_slot_descriptors()
        .unwrap()
        .iter()
        .map(|slot| ApplicationSource {
            component_instance: slot
                .producer_slot
                .strip_prefix("singlecore-multi-swc-v1:")
                .unwrap()
                .into(),
            path: fixture.join(slot.source_paths[0].rsplit('/').next().unwrap()),
        })
        .collect();
    let prepared =
        prepare_ecu_project_with_applications(&plan, tooling::native_target(), &applications)
            .unwrap();
    let project = scratch.0.join("project");
    for (path, bytes) in prepared.into_files() {
        let path = project.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    let output = scratch.0.join("build");
    let built = tooling::ecu_build_command(
        &project,
        &output,
        "test",
        Some(&fixture.join("owner_probe.c")),
    )
    .output()
    .unwrap();
    assert!(
        built.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
    for phase in ["before", "after", "canonical", "controls", "epoch0", "late"] {
        let logs = scratch.0.join(phase);
        std::fs::create_dir(&logs).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let spec = autosar_config_core::execution::ProcessSpec::for_duration(
            vec![
                tooling::native_binary(&output, "ecu_probe").into_os_string(),
                phase.into(),
            ],
            scratch.0.clone(),
            vec![],
            std::time::Duration::from_secs(30),
            logs,
        )
        .unwrap();
        let result = tooling::execution_owner()
            .spawn(spec, None)
            .unwrap()
            .wait()
            .unwrap();
        let text = std::fs::read_to_string(&result.stdout).unwrap();
        assert!(
            result.success(),
            "{phase}: {result:?}\n{text}\n{}",
            std::fs::read_to_string(&result.stderr).unwrap()
        );
        let observations: Vec<_> = text
            .lines()
            .filter_map(|line| {
                let fields: Vec<_> = line.split_whitespace().collect();
                (fields.first() == Some(&"OBS")).then(|| {
                    (
                        fields[1].to_string(),
                        fields[2].parse::<u64>().unwrap(),
                        fields[3].parse::<u8>().unwrap(),
                        fields[4].parse::<u32>().unwrap(),
                    )
                })
            })
            .collect();
        let expected: Vec<_> = if phase == "late" {
            assert_eq!(
                text.lines()
                    .filter(|line| line.starts_with("LATE "))
                    .collect::<Vec<_>>(),
                [
                    "LATE 1 1800 0762123400000000",
                    "LATE 2 1800 0762123400000000"
                ]
            );
            vec![("rx".into(), 1, 0, 21), ("rx".into(), 2, 0, 21)]
        } else if phase == "epoch0" {
            std::iter::once(("rx".to_string(), 0, 0, 21))
                .chain((1..=30u64).map(|epoch| {
                    (
                        "main".to_string(),
                        epoch,
                        if epoch == 30 { 64 } else { 0 },
                        21,
                    )
                }))
                .collect()
        } else if phase == "controls" {
            (1..=120u64)
                .flat_map(|epoch| {
                    let (status, value) = match epoch {
                        1 => (133, 7),
                        2..=20 => (0, 21),
                        21..=42 => (0, 42),
                        43..=71 => (0, 43),
                        72 | 74 => (64, 43),
                        73 => (128, 43),
                        75 => (0, 0),
                        76..=104 => (0, 45),
                        105..=109 => (64, 45),
                        _ => (128, 45),
                    };
                    let mut rows = vec![("main".to_string(), epoch, status, value)];
                    match epoch {
                        1 => rows.push(("rx".into(), epoch, 0, 21)),
                        20 => rows.push(("rx".into(), epoch, 0, 42)),
                        42 => rows.push(("rx".into(), epoch, 0, 43)),
                        73 => rows.push(("rx".into(), epoch, 128, 43)),
                        74 => rows.push(("rx".into(), epoch, 0, 44)),
                        75 => rows.push(("rx".into(), epoch, 0, 45)),
                        _ => {}
                    }
                    rows
                })
                .collect()
        } else {
            (1..=if phase == "canonical" { 20u64 } else { 61u64 })
                .flat_map(|epoch| {
                    let value = if epoch == 1 {
                        7
                    } else if phase == "canonical" {
                        0x12345678
                    } else if epoch < 31 || (epoch == 31 && phase == "after") {
                        21
                    } else {
                        42
                    };
                    let status = if epoch == 1 {
                        133
                    } else if epoch == 61 || (epoch == 31 && phase == "after") {
                        64
                    } else {
                        0
                    };
                    let mut rows = vec![("main".to_string(), epoch, status, value)];
                    if epoch == 1 {
                        rows.push((
                            "rx".into(),
                            epoch,
                            0,
                            if phase == "canonical" { 0x12345678 } else { 21 },
                        ));
                    }
                    if epoch == 31 && phase == "after" {
                        rows.push(("rx".into(), epoch, 0, 42));
                        rows.push(("rx".into(), epoch, 0, 42));
                    }
                    if epoch == 30 && phase == "before" {
                        rows.push(("rx".into(), 31, 0, 42));
                        rows.push(("rx".into(), 31, 0, 42));
                    }
                    rows
                })
                .collect()
        };
        assert_eq!(observations, expected, "{phase}: {text}");
        assert!(std::fs::read(&result.stderr).unwrap().is_empty());
    }
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn sealed_multi_optional_did_and_network_only_routes_use_real_producers() {
    use autosar_config_core::{ApplicationSource, prepare_ecu_project_with_applications};
    for diagnostic in [true, false] {
        let scratch = Scratch::new();
        let plan = build(&sources_without_application_did(diagnostic)).unwrap();
        assert!(plan.description().diagnostic.is_none());
        assert_eq!(
            plan.description().diagnostic_transport.is_some(),
            diagnostic
        );
        let fixtures =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi-application");
        let applications: Vec<_> = plan
            .application_slot_descriptors()
            .unwrap()
            .iter()
            .map(|slot| ApplicationSource {
                component_instance: slot
                    .producer_slot
                    .strip_prefix("singlecore-multi-swc-v1:")
                    .unwrap()
                    .into(),
                path: fixtures.join(slot.source_paths[0].rsplit('/').next().unwrap()),
            })
            .collect();
        let prepared =
            prepare_ecu_project_with_applications(&plan, tooling::native_target(), &applications)
                .unwrap();
        let project = scratch.0.join("project");
        for (path, bytes) in prepared.into_files() {
            let path = project.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        let output = scratch.0.join("build");
        let built = tooling::ecu_build_command(&project, &output, "host-batch", None)
            .output()
            .unwrap();
        assert!(
            built.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
        let logs = scratch.0.join("logs");
        std::fs::create_dir(&logs).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut spec = autosar_config_core::execution::ProcessSpec::for_duration(
            vec![tooling::native_binary(&output, "ecu_host_batch").into_os_string()],
            scratch.0.clone(),
            vec![],
            std::time::Duration::from_secs(30),
            logs,
        )
        .unwrap();
        spec.stdin_stream = true;
        let owner = tooling::execution_owner();
        let mut child = owner.spawn(spec, None).unwrap();
        let input_result = child.write_stdin(b"BEGIN 0\nRX 800 4 2A000000\nRX 1792 8 0322F18600000000\nCOMMIT\nBEGIN 1\nCOMMIT\nBEGIN 1\nRX 1792 8 0322123400000000\nCOMMIT\nBEGIN 2\nCOMMIT\nBEGIN 10\nCOMMIT\n");
        child.close_stdin();
        let result = child.wait().unwrap();
        let text = std::fs::read_to_string(&result.stdout).unwrap();
        assert!(
            result.success(),
            "{diagnostic}: {result:?} input={input_result:?}\n{text}\n{}",
            std::fs::read_to_string(&result.stderr).unwrap()
        );
        let records: Vec<_> = text
            .lines()
            .filter(|line| line.starts_with("OUT "))
            .map(|line| {
                let fields: std::collections::BTreeMap<_, _> = line
                    .split_whitespace()
                    .skip(1)
                    .map(|item| item.split_once('=').unwrap())
                    .collect();
                (
                    fields["epoch"].parse::<u64>().unwrap(),
                    fields["id"].parse::<u32>().unwrap(),
                    fields["dlc"].parse::<u8>().unwrap(),
                    fields["data"].to_string(),
                )
            })
            .collect();
        let expected = if diagnostic {
            vec![
                (1, 1800, 8, "0462f18601000000".to_string()),
                (2, 1800, 8, "037f223100000000".into()),
                (10, 801, 4, "2a000000".into()),
            ]
        } else {
            vec![(10, 801, 4, "2a000000".into())]
        };
        assert_eq!(records, expected, "{diagnostic}: {text}");
        assert!(
            !text
                .lines()
                .any(|line| line.starts_with("REJECT ") || line.starts_with("COMMIT_ERROR ")),
            "{text}"
        );
        assert!(std::fs::read(&result.stderr).unwrap().is_empty());
        let config = std::fs::read_to_string(project.join("src/Ecu_RuntimeConfig.c")).unwrap();
        let metadata =
            std::fs::read_to_string(project.join("descriptions/Host_Implementation.arxml"))
                .unwrap();
        if !diagnostic {
            assert!(!config.contains("CanTp_MainFunction();"));
            assert!(!config.contains("Dcm_MainFunction();"));
            let document = roxmltree::Document::parse(&metadata).unwrap();
            assert!(
                !document
                    .descendants()
                    .filter(|node| node.has_tag_name("BSW-TIMING-EVENT"))
                    .any(|node| node.descendants().any(|child| child
                        .text()
                        .is_some_and(|text| text.contains("CanTp_MainFunction")
                            || text.contains("Dcm_MainFunction"))))
            );
        }
    }
}

#[test]
fn prepared_multi_bsw_entries_describe_actual_call_mechanisms_and_implementation_policy() {
    let plan = build(&inputs()).unwrap();
    let files = prepared_multi_files(&plan);
    let bytes = &files
        .iter()
        .find(|(path, _)| path == "descriptions/Host_Implementation.arxml")
        .unwrap()
        .1;
    let document = roxmltree::Document::parse(std::str::from_utf8(bytes).unwrap()).unwrap();
    let imported_refs: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name("EXPECTED-ENTRYS"))
        .flat_map(|node| {
            node.descendants()
                .filter(|child| child.has_tag_name("BSW-MODULE-ENTRY-REF"))
        })
        .map(|node| node.text().unwrap())
        .collect();
    for required in [
        "/AUTOSAR_Can/BswModuleEntrys/Can_Write",
        "/AUTOSAR_CanIf/BswModuleEntrys/CanIf_RxIndication",
        "/AUTOSAR_CanTp/BswModuleEntrys/CanTp_Transmit",
        "/AUTOSAR_Dcm/BswModuleEntrys/Dcm_StartOfReception",
        "/AUTOSAR_ComM/BswModuleEntrys/ComM_DCM_ActiveDiagnostic",
        "/AUTOSAR_PduR/BswModuleEntrys/PduR_CanTpCopyTxData",
        "/AUTOSAR_LSduR/BswModuleEntrys/LSduR_CanTpTransmit",
        "/AUTOSAR_Os/BswModuleEntrys/GetElapsedValue",
    ] {
        assert!(imported_refs.contains(&required), "{required}");
    }
    let actual_entries: std::collections::BTreeMap<_, _> = document
        .descendants()
        .filter(|node| node.has_tag_name("BSW-MODULE-ENTRY"))
        .map(|node| {
            let mut packages: Vec<_> = node
                .ancestors()
                .filter(|ancestor| ancestor.has_tag_name("AR-PACKAGE"))
                .map(|package| {
                    package
                        .children()
                        .find(|child| child.has_tag_name("SHORT-NAME"))
                        .unwrap()
                        .text()
                        .unwrap()
                })
                .collect();
            packages.reverse();
            let name = node
                .children()
                .find(|child| child.has_tag_name("SHORT-NAME"))
                .unwrap()
                .text()
                .unwrap();
            (format!("/{}/{name}", packages.join("/")), node)
        })
        .collect();
    for reference in document
        .descendants()
        .filter(|node| node.has_tag_name("BSW-MODULE-ENTRY-REF"))
    {
        assert_eq!(reference.attribute("DEST"), Some("BSW-MODULE-ENTRY"));
        assert!(
            actual_entries.contains_key(reference.text().unwrap()),
            "{}",
            reference.text().unwrap()
        );
    }
    for (module, name, reentrant, synchronous, call_type) in [
        ("Can", "Can_Write", "true", "true", "REGULAR"),
        ("Can", "Can_SetControllerMode", "false", "false", "REGULAR"),
        ("Can", "Can_GetControllerMode", "false", "true", "REGULAR"),
        (
            "Can",
            "Can_GetControllerErrorState",
            "true",
            "true",
            "REGULAR",
        ),
        (
            "Can",
            "Can_GetControllerRxErrorCounter",
            "true",
            "true",
            "REGULAR",
        ),
        (
            "Can",
            "Can_GetControllerTxErrorCounter",
            "true",
            "true",
            "REGULAR",
        ),
        (
            "CanIf",
            "CanIf_SetControllerMode",
            "true",
            "false",
            "REGULAR",
        ),
        (
            "CanIf",
            "CanIf_GetControllerMode",
            "false",
            "true",
            "REGULAR",
        ),
        (
            "CanIf",
            "CanIf_GetControllerErrorState",
            "true",
            "true",
            "REGULAR",
        ),
        ("CanIf", "CanIf_GetPduMode", "true", "true", "REGULAR"),
        ("CanIf", "CanIf_SetPduMode", "false", "true", "REGULAR"),
        ("CanIf", "CanIf_RxIndication", "true", "true", "CALLBACK"),
        ("CanIf", "CanIf_TxConfirmation", "true", "true", "CALLBACK"),
        (
            "CanIf",
            "CanIf_ControllerModeIndication",
            "true",
            "true",
            "CALLBACK",
        ),
        (
            "CanIf",
            "CanIf_ControllerBusOff",
            "true",
            "true",
            "CALLBACK",
        ),
        ("CanTp", "CanTp_Transmit", "true", "true", "REGULAR"),
        ("CanTp", "CanTp_RxIndication", "true", "true", "CALLBACK"),
        ("CanTp", "CanTp_TxConfirmation", "true", "true", "CALLBACK"),
        ("Dcm", "Dcm_CopyTxData", "true", "true", "CALLBACK"),
        ("PduR", "PduR_CanTpCopyTxData", "true", "true", "CALLBACK"),
        ("PduR", "PduR_CanIfRxIndication", "true", "true", "CALLBACK"),
        ("PduR", "PduR_DcmTransmit", "true", "true", "REGULAR"),
        ("LSduR", "LSduR_CanTpTransmit", "true", "true", "REGULAR"),
        (
            "LSduR",
            "LSduR_CanIfTxConfirmation",
            "true",
            "true",
            "CALLBACK",
        ),
        ("Com", "Com_SendSignal", "true", "false", "REGULAR"),
        ("Com", "Com_TriggerTransmit", "true", "true", "CALLBACK"),
        (
            "ComM",
            "ComM_DCM_ActiveDiagnostic",
            "true",
            "true",
            "CALLBACK",
        ),
        (
            "ComM",
            "ComM_BusSM_ModeIndication",
            "true",
            "false",
            "CALLBACK",
        ),
        ("BswM", "BswM_ComM_CurrentMode", "true", "true", "CALLBACK"),
        ("Rte", "Rte_COMCbk", "false", "true", "CALLBACK"),
    ] {
        let path = format!("/AUTOSAR_{module}/BswModuleEntrys/{name}");
        let entry = actual_entries[&path];
        for (tag, expected) in [
            ("IS-REENTRANT", reentrant),
            ("IS-SYNCHRONOUS", synchronous),
            ("CALL-TYPE", call_type),
        ] {
            assert_eq!(
                entry
                    .children()
                    .find(|child| child.has_tag_name(tag))
                    .unwrap()
                    .text(),
                Some(expected),
                "{path}: {tag}"
            );
        }
    }
    for entry in document
        .descendants()
        .filter(|node| node.has_tag_name("BSW-MODULE-ENTRY"))
    {
        let field = |tag| {
            entry
                .children()
                .find(|child| child.has_tag_name(tag))
                .and_then(|child| child.text())
                .unwrap()
        };
        let name = field("SHORT-NAME");
        assert_eq!(field("SW-SERVICE-IMPL-POLICY"), "STANDARD", "{name}");
        assert_eq!(field("BSW-ENTRY-KIND"), "CONCRETE", "{name}");
        for tag in [
            "IS-REENTRANT",
            "IS-SYNCHRONOUS",
            "CALL-TYPE",
            "EXECUTION-CONTEXT",
        ] {
            assert!(!field(tag).is_empty(), "{name}: {tag}");
        }
        if matches!(
            name,
            "Rte_COMCbk"
                | "Rte_COMCbkRxTOut"
                | "Dcm_TpTxConfirmation"
                | "Dcm_ComM_NoComModeEntered"
                | "Dcm_ComM_SilentComModeEntered"
                | "Dcm_ComM_FullComModeEntered"
        ) {
            assert_eq!(field("CALL-TYPE"), "CALLBACK", "{name}");
        }
        if matches!(
            name,
            "Dcm_ResetToDefaultSession" | "GetCounterValue" | "GetElapsedValue"
        ) {
            assert_eq!(field("IS-REENTRANT"), "true", "{name}");
            assert_eq!(field("IS-SYNCHRONOUS"), "true", "{name}");
            assert_eq!(field("CALL-TYPE"), "REGULAR", "{name}");
        }
        if name == "Dcm_MainFunction" {
            assert_eq!(field("CALL-TYPE"), "SCHEDULED");
        }
    }
}

#[test]
fn authentication_mode_source_follows_actual_connection_and_rejects_invalid_standard_state() {
    let mut source = inputs();
    replace_all(&mut source, "Physical", "Link");
    let plan = build(&source).unwrap();
    assert!(
        plan.description()
            .multi
            .as_ref()
            .unwrap()
            .dcm_modes
            .iter()
            .any(|mode| mode.group == "DcmAuthenticationState_Link")
    );
    let files: std::collections::BTreeMap<_, _> = prepared_multi_files(&plan).into_iter().collect();
    for (path, symbol) in [
        (
            "src/Ecu_RuntimeConfig.c",
            "SchM_Switch_Dcm_DcmAuthenticationState_Link",
        ),
        ("src/SchM.c", "SchM_Mode_Dcm_DcmAuthenticationState_Link"),
        (
            "include/SchM_Dcm.h",
            "SchM_Switch_Dcm_DcmAuthenticationState_Link",
        ),
    ] {
        let text = std::str::from_utf8(&files[path]).unwrap();
        assert!(text.contains(symbol), "{path}");
        assert!(!text.contains("DcmAuthenticationState_Physical"), "{path}");
    }
    let mut source = inputs();
    xml_edit(
        &mut source,
        "bsw.arxml",
        |node| named(node, "MODE-DECLARATION", "DCM_AUTHENTICATED"),
        |text| text.replace("<VALUE>1</VALUE>", "<VALUE>2</VALUE>"),
    );
    rejects_in_both(&source, "TYPE_CONFLICT");
    let mut source = inputs();
    xml_edit(
        &mut source,
        "services.arxml",
        |node| {
            named(
                node,
                "P-PORT-PROTOTYPE",
                "AuthenticationStateModeSwitchInterface_Physical",
            )
        },
        |_| String::new(),
    );
    rejects_in_both(&source, "SERVICE_TYPE_CONFLICT");
}

#[test]
fn destination_only_diagnostic_routes_and_explicit_lsdu_policy_cannot_be_ignored() {
    let mut source = sources_without_application_did(true);
    xml_edit(
        &mut source,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "DiagRequestSource"),
        |_| String::new(),
    );
    xml_edit(
        &mut source,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "DiagRequest")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/PduRRoutingPath"))
                })
        },
        |_| String::new(),
    );
    rejects_in_both(&source, "PDU_ROUTE_NOT_UNIQUE");
    let mut source = inputs();
    xml_edit(
        &mut source,
        "ecuc.arxml",
        |node| named(node, "AR-PACKAGE", "Configuration"),
        |text| {
            text.replacen("</ELEMENTS>", "<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>LSduR</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-MODULE-DEF\">/AUTOSAR/EcucDefs/LSduR</DEFINITION-REF></ECUC-MODULE-CONFIGURATION-VALUES></ELEMENTS>", 1)
        },
    );
    rejects_in_both(&source, "MODULE_UNSUPPORTED");
}
