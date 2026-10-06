use autosar_config_core::Workspace;
use autosar_config_core::integration::{
    InputSource, IntegrationEdit, PlanDependencies, RuntimeCatalog, build_plan,
};
use std::fs;
use std::path::Path;

pub fn generated_tables() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let inputs = super::epic4_plan::inputs()
        .into_iter()
        .map(|source| {
            if source.logical_path() == "ecuc.arxml" {
                InputSource::new(
                    "ecuc.arxml",
                    fs::read(root.join("core/tests/fixtures/epic4_timing/schedule_tables.arxml"))
                        .unwrap(),
                )
                .unwrap()
            } else {
                source
            }
        })
        .collect::<Vec<_>>();
    let plan =
        build_plan(&inputs, &dependencies, &runtime).unwrap_or_else(|error| panic!("{error:?}"));
    assert!(
        plan.description()
            .schedule
            .entities
            .iter()
            .all(|entity| entity.alarm.is_empty() && entity.expiry_point.is_some())
    );
    for (name, change) in [
        (
            "not-repeating",
            (
                "OsScheduleTableRepeating</DEFINITION-REF><VALUE>true</VALUE>",
                "OsScheduleTableRepeating</DEFINITION-REF><VALUE>false</VALUE>",
            ),
        ),
        (
            "wrong-strategy",
            ("<VALUE>NONE</VALUE>", "<VALUE>IMPLICIT</VALUE>"),
        ),
        (
            "wrong-first-offset",
            (
                "OsScheduleTblExpPointOffset</DEFINITION-REF><VALUE>0</VALUE>",
                "OsScheduleTblExpPointOffset</DEFINITION-REF><VALUE>1</VALUE>",
            ),
        ),
        (
            "wrong-owner",
            (
                "/Configuration/Os/Task_Ecu</VALUE-REF>",
                "/Configuration/Os/SystemCounter</VALUE-REF>",
            ),
        ),
        (
            "hardware-counter-unselected",
            ("<VALUE>SOFTWARE</VALUE>", "<VALUE>HARDWARE</VALUE>"),
        ),
    ] {
        let changed = inputs
            .iter()
            .map(|source| {
                let mut bytes = source.bytes().to_vec();
                if source.logical_path() == "ecuc.arxml" {
                    let original = String::from_utf8(bytes).unwrap();
                    let text = original.replace(change.0, change.1);
                    assert_ne!(
                        original, text,
                        "{name}: mutation did not reach the real input"
                    );
                    bytes = text.into_bytes();
                }
                InputSource::new(source.logical_path(), bytes).unwrap()
            })
            .collect::<Vec<_>>();
        assert!(
            build_plan(&changed, &dependencies, &runtime).is_err(),
            "accepted {name}"
        );
    }
    let project = plan
        .ecu_integration_files(super::tooling::native_target())
        .unwrap();
    let scratch = super::Scratch::new();
    let source = scratch.0.join("tables-source");
    let preview = project.preview(&source).unwrap();
    project
        .generate_previewed(&source, &preview.revision)
        .unwrap();
    let build = scratch.0.join("tables-build");
    super::epic4_ecu::compile(&source, &build, None);
    let output =
        super::epic4_ecu::run_probe(&super::tooling::native_binary(&build, "ecu_probe"), None);
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let consumer_build = scratch.0.join("tables-consumer-build");
    super::epic4_ecu::compile(
        &source,
        &consumer_build,
        Some(&root.join("core/tests/fixtures/ecu_control.c")),
    );
    let output = super::epic4_ecu::run_probe(
        &super::tooling::native_binary(&consumer_build, "ecu_probe"),
        None,
    );
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("independent_control signal=2 default_session_did=2 completed=20")
    );
    for offset in [0, 2] {
        let originals = scratch.0.join(format!("table-inputs-{offset}"));
        fs::create_dir(&originals).unwrap();
        let paths = inputs
            .iter()
            .map(|input| {
                let path = originals.join(input.logical_path());
                let mut bytes = input.bytes().to_vec();
                if input.logical_path() == "ecuc.arxml" && offset == 2 {
                    let text = String::from_utf8(bytes).unwrap();
                    let (prefix, app) = text
                        .split_once("<SHORT-NAME>Table_App</SHORT-NAME>")
                        .unwrap();
                    let app = app
                        .replacen(
                            "OsScheduleTableStartValue</DEFINITION-REF><VALUE>10</VALUE>",
                            "OsScheduleTableStartValue</DEFINITION-REF><VALUE>8</VALUE>",
                            1,
                        )
                        .replacen(
                            "OsScheduleTblExpPointOffset</DEFINITION-REF><VALUE>0</VALUE>",
                            "OsScheduleTblExpPointOffset</DEFINITION-REF><VALUE>2</VALUE>",
                            1,
                        );
                    bytes = format!("{prefix}<SHORT-NAME>Table_App</SHORT-NAME>{app}").into_bytes();
                }
                fs::write(&path, bytes).unwrap();
                path
            })
            .collect();
        let mut workspace =
            Workspace::open_legacy(paths, dependencies.xsd_archive.clone()).unwrap();
        let initial = workspace
            .integration_plan_legacy(&runtime, dependencies.mod_archive.clone())
            .unwrap();
        let app = initial
            .description()
            .schedule
            .entities
            .iter()
            .find(|entity| entity.application)
            .unwrap();
        assert_eq!(app.table_start, Some(10 - offset));
        assert_eq!(app.expiry_offset, Some(offset));
        workspace
            .edit_integration_legacy(
                &runtime,
                dependencies.mod_archive.clone(),
                IntegrationEdit {
                    application_period_ms: Some(20),
                    ..Default::default()
                },
            )
            .unwrap();
        let edited = workspace
            .integration_plan_legacy(&runtime, dependencies.mod_archive.clone())
            .unwrap();
        assert_eq!(edited.description().component.period_ms, 20);
        let app = edited
            .description()
            .schedule
            .entities
            .iter()
            .find(|entity| entity.application)
            .unwrap();
        assert_eq!(app.table_start, Some(20 - offset));
        assert_eq!(app.expiry_offset, Some(offset));
        let save = workspace
            .preview_integration_save_legacy(&runtime, dependencies.mod_archive.clone())
            .unwrap();
        workspace
            .save_integration_previewed_legacy(
                &runtime,
                dependencies.mod_archive.clone(),
                &save.revision,
            )
            .unwrap();
        let project = workspace
            .integration_plan_legacy(&runtime, dependencies.mod_archive.clone())
            .unwrap()
            .ecu_integration_files(super::tooling::native_target())
            .unwrap();
        let edited_source = scratch.0.join(format!("edited-table-source-{offset}"));
        let preview = project.preview(&edited_source).unwrap();
        project
            .generate_previewed(&edited_source, &preview.revision)
            .unwrap();
        let edited_build = scratch.0.join(format!("edited-table-build-{offset}"));
        let output = super::tooling::run_public_command(
            &mut super::tooling::ecu_build_command(
                &edited_source,
                &edited_build,
                "host-batch",
                None,
            ),
            &scratch.0,
            &format!("edited-build-{offset}"),
            std::time::Duration::from_secs(180),
        );
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        // Refresh at tick 30: the unchanged Com RX timeout is 30 ms. There
        // must be no periodic output at 30; the next edited period is tick 40.
        let text = super::epic4_batch::run_text(&super::tooling::native_binary(&edited_build, "ecu_host_batch"), &scratch.0.join(format!("edited-observation-{offset}")), b"BEGIN 0\nRX 800 4 78563412\nCOMMIT\nBEGIN 10\nRX 1792 8 0322123400000000\nCOMMIT\nBEGIN 20\nRX 1792 8 0322123400000000\nCOMMIT\nBEGIN 30\nRX 800 4 78563412\nCOMMIT\nBEGIN 40\nRX 1792 8 0322123400000000\nCOMMIT\n");
        let frames: Vec<_> = text
            .lines()
            .filter(|line| line.starts_with("OUT "))
            .collect();
        assert_eq!(frames.len(), 5, "offset={offset}: {text}");
        assert!(
            frames[0].starts_with("OUT epoch=10 ")
                && frames[0].ends_with("id=1800 dlc=8 data=0762123400000000"),
            "{text}"
        );
        assert!(
            frames[1].starts_with("OUT epoch=20 ")
                && frames[1].ends_with("id=801 dlc=4 data=78563412"),
            "{text}"
        );
        assert!(
            frames[2].starts_with("OUT epoch=20 ")
                && frames[2].ends_with("id=1800 dlc=8 data=0762123412345678"),
            "{text}"
        );
        assert!(
            frames[3].starts_with("OUT epoch=40 ")
                && frames[3].ends_with("id=801 dlc=4 data=78563412"),
            "{text}"
        );
        assert!(
            frames[4].starts_with("OUT epoch=40 ")
                && frames[4].ends_with("id=1800 dlc=8 data=0762123412345678"),
            "{text}"
        );
        assert_eq!(text.matches("COMMIT_OK ").count(), 5, "{text}");
    }
}
