use autosar_config_core::Workspace;
use autosar_config_core::integration::{IntegrationEdit, PlanDependencies, RuntimeCatalog};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub fn verify() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let scratch = super::Scratch::new();
    let mut originals = BTreeMap::new();
    let paths: Vec<_> = super::epic4_plan::inputs()
        .into_iter()
        .map(|source| {
            let path = scratch.0.join(source.logical_path());
            let text = std::str::from_utf8(source.bytes()).unwrap().to_owned();
            let text = if source.logical_path() == "application.arxml" {
                text.replace(
                    "<PERIOD>0.01</PERIOD>",
                    "<!-- retained before period -->\n                  <PERIOD>0.01</PERIOD>",
                )
            } else {
                text
            };
            fs::write(&path, &text).unwrap();
            originals.insert(source.logical_path().to_owned(), text);
            path
        })
        .collect();
    let mut workspace =
        Workspace::open_legacy(paths.clone(), dependencies.xsd_archive.clone()).unwrap();
    assert!(workspace.view().integration_candidate);
    let before = workspace
        .integration_plan_legacy(&runtime, dependencies.mod_archive.clone())
        .unwrap();
    let rx = before
        .description()
        .signals
        .iter()
        .find(|signal| signal.receive)
        .unwrap()
        .port
        .clone();
    let tx = before
        .description()
        .signals
        .iter()
        .find(|signal| !signal.receive)
        .unwrap()
        .port
        .clone();
    assert_eq!(before.description().sources.len(), 7);
    assert!(
        before
            .description()
            .sources
            .iter()
            .any(|source| source.roles.contains(&"retained".into()))
    );
    let edit = IntegrationEdit {
        can_ids: BTreeMap::from([(rx.clone(), 1100), (tx.clone(), 1101)]),
        application_period_ms: Some(20),
    };
    let report = workspace
        .edit_integration_legacy(&runtime, dependencies.mod_archive.clone(), edit)
        .unwrap();
    let description = report.description.unwrap();
    assert_eq!(description.component.as_ref().unwrap().period_ms, 20);
    assert!(
        description
            .signals
            .iter()
            .all(|signal| signal.can_id == if signal.receive { 1100 } else { 1101 })
    );
    assert_eq!(
        description
            .signals
            .iter()
            .find(|signal| !signal.receive)
            .unwrap()
            .transmit_period_ms,
        Some(20)
    );
    assert_eq!(
        description
            .schedule
            .entities
            .iter()
            .filter(|entity| entity.period_ms == 20)
            .count(),
        2
    );
    assert!(workspace.view().dirty);
    let preview = workspace
        .preview_integration_save_legacy(&runtime, dependencies.mod_archive.clone())
        .unwrap();
    assert_eq!(preview.files.iter().filter(|file| file.changed).count(), 4);
    for path in &paths {
        let name = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            originals[name],
            "Preview wrote {name}"
        );
    }
    for file in preview.files.iter().filter(|file| file.changed) {
        let name = Path::new(&file.path).file_name().unwrap().to_str().unwrap();
        let expected = match name {
            "application.arxml" | "bsw.arxml" => {
                originals[name].replace("<PERIOD>0.01</PERIOD>", "<PERIOD>0.020</PERIOD>")
            }
            "extract.arxml" => originals[name]
                .replace(
                    "<IDENTIFIER>800</IDENTIFIER>",
                    "<IDENTIFIER>1100</IDENTIFIER>",
                )
                .replace(
                    "<IDENTIFIER>801</IDENTIFIER>",
                    "<IDENTIFIER>1101</IDENTIFIER>",
                ),
            "ecuc.arxml" => originals[name]
                .replace("<VALUE>800</VALUE>", "<VALUE>1100</VALUE>")
                .replace("<VALUE>801</VALUE>", "<VALUE>1101</VALUE>")
                .replace("<VALUE>10</VALUE>", "<VALUE>20</VALUE>")
                .replace("<VALUE>0.01</VALUE>", "<VALUE>0.020</VALUE>"),
            _ => panic!("Unexpected changed source {name}"),
        };
        assert_eq!(
            file.after.as_deref(),
            Some(expected.as_str()),
            "Only supported text ranges may change in {name}"
        );
    }
    for edit in [
        IntegrationEdit {
            can_ids: BTreeMap::from([(rx.clone(), 1101)]),
            application_period_ms: None,
        },
        IntegrationEdit {
            can_ids: BTreeMap::from([(rx.clone(), 0x800)]),
            application_period_ms: None,
        },
        IntegrationEdit {
            can_ids: BTreeMap::from([("/Unrelated/Value".into(), 1102)]),
            application_period_ms: None,
        },
        IntegrationEdit {
            can_ids: BTreeMap::new(),
            application_period_ms: Some(0),
        },
        IntegrationEdit {
            can_ids: BTreeMap::new(),
            application_period_ms: Some(65536),
        },
    ] {
        let issues = workspace
            .edit_integration_legacy(&runtime, dependencies.mod_archive.clone(), edit)
            .err()
            .unwrap();
        assert!(
            issues
                .iter()
                .any(|issue| issue.object.is_some() && matches!(&issue.remedy, autosar_config_core::message::LocalizedText::Message(message) if message.key.starts_with("backend.")))
        );
        let unchanged = workspace
            .preview_integration_save_legacy(&runtime, dependencies.mod_archive.clone())
            .unwrap();
        assert_eq!(
            serde_json::to_vec(&preview).unwrap(),
            serde_json::to_vec(&unchanged).unwrap()
        );
    }
    assert!(
        serde_json::from_value::<IntegrationEdit>(serde_json::json!({"canIds":{},"valid":true}))
            .is_err()
    );
    let edited = workspace
        .edit_integration_legacy(
            &runtime,
            dependencies.mod_archive.clone(),
            IntegrationEdit {
                can_ids: BTreeMap::new(),
                application_period_ms: Some(25),
            },
        )
        .unwrap();
    assert_eq!(
        edited
            .description
            .unwrap()
            .component
            .as_ref()
            .unwrap()
            .period_ms,
        25
    );
    let issues = workspace
        .save_integration_previewed_legacy(
            &runtime,
            dependencies.mod_archive.clone(),
            &preview.revision,
        )
        .err()
        .unwrap();
    assert_eq!(issues[0].code, "SAVE_PREVIEW_STALE");
    let final_preview = workspace
        .preview_integration_save_legacy(&runtime, dependencies.mod_archive.clone())
        .unwrap();
    // Refuse an interrupted prior save's backup, and retain all original input.
    let changed_path = Path::new(
        &final_preview
            .files
            .iter()
            .find(|file| file.changed)
            .unwrap()
            .path,
    )
    .to_path_buf();
    let backup =
        changed_path.with_extension(format!("arxml.autosar-config-{}-0.bak", std::process::id()));
    fs::write(&backup, b"prior interrupted save").unwrap();
    let issues = workspace
        .save_integration_previewed_legacy(
            &runtime,
            dependencies.mod_archive.clone(),
            &final_preview.revision,
        )
        .err()
        .unwrap();
    assert_eq!(issues[0].code, "SAVE_FAILED");
    assert_eq!(fs::read(&backup).unwrap(), b"prior interrupted save");
    for path in &paths {
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            originals[path.file_name().unwrap().to_str().unwrap()]
        );
    }
    fs::remove_file(&backup).unwrap();
    let saved = workspace
        .save_integration_previewed_legacy(
            &runtime,
            dependencies.mod_archive.clone(),
            &final_preview.revision,
        )
        .unwrap();
    assert!(!workspace.view().dirty);
    let reopened = Workspace::open_legacy(paths.clone(), dependencies.xsd_archive.clone()).unwrap();
    let rechecked = reopened.inspect_integration_legacy(&runtime, dependencies.mod_archive.clone());
    assert_eq!(
        serde_json::to_vec(&saved).unwrap(),
        serde_json::to_vec(&rechecked).unwrap()
    );
    for name in ["services.arxml", "types.arxml", "unrelated.arxml"] {
        assert_eq!(
            fs::read_to_string(scratch.0.join(name)).unwrap(),
            originals[name]
        );
    }
    assert!(
        fs::read_to_string(scratch.0.join("application.arxml"))
            .unwrap()
            .contains("<!-- retained before period -->")
    );
    let old = workspace
        .preview_integration_save_legacy(&runtime, dependencies.mod_archive.clone())
        .unwrap();
    let external_path = scratch.0.join("unrelated.arxml");
    let external = originals["unrelated.arxml"].clone() + "\n<!-- external edit -->\n";
    fs::write(&external_path, &external).unwrap();
    let result = workspace.inspect_integration_legacy(&runtime, dependencies.mod_archive.clone());
    assert!(result.description.is_none());
    assert_eq!(result.diagnostics[0].code, "SOURCE_CHANGED");
    assert!(
        workspace
            .save_integration_previewed_legacy(
                &runtime,
                dependencies.mod_archive.clone(),
                &old.revision
            )
            .is_err()
    );
    assert_eq!(fs::read_to_string(&external_path).unwrap(), external);
    let dependency_workspace =
        Workspace::open_legacy(paths, dependencies.xsd_archive.clone()).unwrap();
    let missing_mod = dependency_workspace
        .inspect_integration_legacy(&runtime, scratch.0.join("missing-mod.zip"));
    assert!(missing_mod.description.is_none());
    assert!(
        missing_mod.diagnostics.iter().any(|issue| issue.category
            == autosar_config_core::integration::DiagnosticCategory::Dependency)
    );
    let partial = Workspace::open_legacy(
        vec![scratch.0.join("types.arxml")],
        dependencies.xsd_archive.clone(),
    )
    .unwrap();
    let report = partial.inspect_integration_legacy(&runtime, dependencies.mod_archive.clone());
    assert!(report.description.is_none());
    assert!(!report.diagnostics.is_empty());

    let mixed = super::Scratch::new();
    let mixed_paths: Vec<_> = super::epic4_plan::inputs()
        .into_iter()
        .map(|source| {
            let path = mixed.0.join(source.logical_path());
            let text = std::str::from_utf8(source.bytes()).unwrap().replace(
                "<IDENTIFIER>800</IDENTIFIER>",
                "<IDENTIFIER>800<!-- retained numeric comment --></IDENTIFIER>",
            );
            fs::write(&path, text).unwrap();
            path
        })
        .collect();
    let mut mixed_workspace =
        Workspace::open_legacy(mixed_paths.clone(), dependencies.xsd_archive.clone()).unwrap();
    let unchanged = mixed_workspace
        .preview_integration_save_legacy(&runtime, dependencies.mod_archive.clone())
        .unwrap();
    let issues = mixed_workspace
        .edit_integration_legacy(
            &runtime,
            dependencies.mod_archive.clone(),
            IntegrationEdit {
                can_ids: BTreeMap::from([(rx, 1100)]),
                application_period_ms: Some(20),
            },
        )
        .err()
        .unwrap();
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "EDIT_UNSAFE"
                && issue.file.as_deref() == Some("extract.arxml"))
    );
    assert_eq!(
        serde_json::to_vec(&unchanged).unwrap(),
        serde_json::to_vec(
            &mixed_workspace
                .preview_integration_save_legacy(&runtime, dependencies.mod_archive.clone())
                .unwrap()
        )
        .unwrap()
    );
    assert!(!mixed_workspace.view().dirty);

    // A foreign namespace cannot shadow a supported value: the mandatory
    // pinned XSD check rejects it before any editable plan can be obtained.
    for (name, original, shadow) in [
        (
            "extract.arxml",
            "<IDENTIFIER>800</IDENTIFIER>",
            "<ext:IDENTIFIER xmlns:ext=\"urn:test:extension\">900</ext:IDENTIFIER><IDENTIFIER>800</IDENTIFIER>",
        ),
        (
            "application.arxml",
            "<PERIOD>0.01</PERIOD>",
            "<ext:PERIOD xmlns:ext=\"urn:test:extension\">0.02</ext:PERIOD><PERIOD>0.01</PERIOD>",
        ),
        (
            "ecuc.arxml",
            "<VALUE>800</VALUE>",
            "<ext:VALUE xmlns:ext=\"urn:test:extension\">900</ext:VALUE><VALUE>800</VALUE>",
        ),
    ] {
        let source = super::epic4_plan::inputs()
            .into_iter()
            .find(|source| source.logical_path() == name)
            .unwrap();
        let text = std::str::from_utf8(source.bytes()).unwrap();
        assert!(text.contains(original));
        let text = text.replacen(original, shadow, 1);
        let issues = autosar_config_core::schema::validate_files(
            &dependencies.xsd_archive,
            &[(Path::new(name), text.as_str())],
        )
        .unwrap();
        assert!(
            issues.iter().any(|issue| issue.code == "R24_XSD"),
            "The foreign namespace shadow must be rejected: {name}"
        );
    }
}
