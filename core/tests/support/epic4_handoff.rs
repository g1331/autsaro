use autosar_config_core::Workspace;
use autosar_config_core::integration::{
    InputSource, PlanDependencies, RuntimeCatalog, build_plan, open_ecu_handoff, verify_ecu_project,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

// Deliberately co-edit an attack copy's checksum record. The importer must still
// compare it with an independently rebuilt product, not trust these new hashes.
pub(super) fn reseal(project: &Path) {
    let list = fs::read(project.join("files.list")).unwrap();
    let mut hashes = String::new();
    for name in String::from_utf8(list.clone()).unwrap().lines() {
        hashes.push_str(&format!(
            "{:x}  {name}\n",
            Sha256::digest(fs::read(project.join(name)).unwrap())
        ));
    }
    hashes.push_str(&format!("{:x}  files.list\n", Sha256::digest(list)));
    fs::write(project.join("files.sha256"), hashes).unwrap();
}

pub fn verify() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let sources = super::epic4_plan::inputs()
        .into_iter()
        .map(|s| {
            InputSource::new(format!("Nested/{}", s.logical_path()), s.bytes().to_vec()).unwrap()
        })
        .collect::<Vec<_>>();
    let plan = build_plan(&sources, &dependencies, &runtime).unwrap();
    let scratch = super::Scratch::new();
    let original = scratch.0.join("handoff-original");
    let files = plan
        .ecu_handoff_files(super::tooling::native_target())
        .unwrap();
    let preview = files.preview(&original).unwrap();
    files
        .generate_previewed(&original, &preview.revision)
        .unwrap();
    let project = scratch.0.join("moved ECU handoff");
    fs::rename(original, &project).unwrap();
    let opened = open_ecu_handoff(&project, &dependencies, &runtime).unwrap();
    assert_eq!(
        serde_json::to_value(opened.plan().description()).unwrap(),
        serde_json::to_value(plan.description()).unwrap()
    );
    let workspace = Workspace::open_ecu_handoff(&project, &dependencies, &runtime).unwrap();
    let rebuilt = workspace
        .saved_integration_plan(&runtime, dependencies.mod_archive.clone())
        .unwrap();
    assert_eq!(
        rebuilt
            .sources()
            .iter()
            .map(InputSource::logical_path)
            .collect::<Vec<_>>(),
        sources
            .iter()
            .map(InputSource::logical_path)
            .collect::<Vec<_>>()
    );
    let regenerated = rebuilt.ecu_handoff_files(opened.target()).unwrap();
    assert_eq!(files.files(), regenerated.files());
    let output = scratch.0.join("fresh-source");
    let preview = regenerated.preview(&output).unwrap();
    regenerated
        .generate_previewed(&output, &preview.revision)
        .unwrap();
    let mut verify =
        super::tooling::ecu_verify_command(&output, &scratch.0.join("independent-verify-build"));
    let result = super::epic4_ecu::run_public_command(
        &mut verify,
        &scratch.0,
        "handoff-offline",
        Duration::from_secs(150),
    );
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let binary = fs::read(super::tooling::native_binary(
        &scratch.0.join("independent-verify-build"),
        "ecu_host_batch",
    ))
    .unwrap();
    let debug_paths = String::from_utf8_lossy(&binary)
        .replace('\\', "/")
        .to_lowercase();
    let author_path = scratch
        .0
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();
    assert!(
        !debug_paths.contains(&author_path),
        "Delivered binary retained its source/build machine path"
    );
    // An actual native producer emits a valid frame plus a spurious one. A
    // positive-only matcher would approve this deliberately altered package.
    let producer = output.join("src/ecu_host_batch.c");
    let saved_producer = fs::read(&producer).unwrap();
    let bad_producer = String::from_utf8(saved_producer.clone()).unwrap().replace(
        "    return fflush(stdout) == 0;",
        "    if (output != NULL) { (void)puts(\"OUT epoch=10 sequence=999 ticket=999 pdu=0 id=2047 dlc=1 data=ff\"); }\n    return fflush(stdout) == 0;",
    );
    assert_ne!(bad_producer.as_bytes(), saved_producer);
    fs::write(&producer, bad_producer).unwrap();
    reseal(&output);
    let mut rejected =
        super::tooling::ecu_verify_command(&output, &scratch.0.join("spurious-build"));
    let rejected = super::epic4_ecu::run_public_command(
        &mut rejected,
        &scratch.0,
        "spurious-output",
        Duration::from_secs(300),
    );
    assert!(!rejected.status.success());
    fs::write(&producer, saved_producer).unwrap();
    reseal(&output);

    // Exercise the delivered stdlib owner with real three-generation timeout
    // and failed-before-release/assignment processes; no shell cleanup exists.
    let mut owner_checks = Command::new(super::tooling::execution_settings().python);
    owner_checks.args([
        "-m", "unittest",
        "autosar_tooling.test_process.BoundedProcessTests.test_timeout_closes_registered_group_and_keeps_logs",
    ]);
    if cfg!(windows) {
        owner_checks.arg(
            "autosar_tooling.test_process.BoundedProcessTests.test_failed_job_assignment_closes_unstarted_process",
        );
    } else {
        owner_checks.args([
            "autosar_tooling.test_process.BoundedProcessTests.test_failed_registration_never_runs_a_command",
            "autosar_tooling.test_process.BoundedProcessTests.test_failed_release_closes_registered_but_unstarted_scope",
        ]);
    }
    owner_checks
        .env(
            "PYTHONPATH",
            std::env::join_paths([output.join("tools"), root.join("scripts")]).unwrap(),
        )
        .env("PATH", scratch.0.join("empty-path"));
    let result = super::epic4_ecu::run_public_command(
        &mut owner_checks,
        &scratch.0,
        "shipped-owner-closure",
        Duration::from_secs(30),
    );
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
    );
    let path = project.join("src/Rte.c");
    let original = fs::read(&path).unwrap();
    fs::write(&path, b"/* attacker changed the sealed implementation */\n").unwrap();
    assert!(open_ecu_handoff(&project, &dependencies, &runtime).is_err());
    assert!(files.preview(&project).is_err());
    assert_eq!(
        fs::read(&path).unwrap(),
        b"/* attacker changed the sealed implementation */\n"
    );
    reseal(&project);
    let executable_rejected =
        verify_ecu_project(&plan, &project, &super::tooling::execution_settings())
            .err()
            .unwrap();
    assert!(
        executable_rejected.contains("src/Rte.c"),
        "{executable_rejected}"
    );
    let rejected = open_ecu_handoff(&project, &dependencies, &runtime)
        .err()
        .unwrap();
    assert!(
        rejected.iter().any(|d| d.message.contains("src/Rte.c")),
        "{rejected:?}"
    );
    fs::write(&path, &original).unwrap();
    reseal(&project);
    let input = project.join("inputs/Nested/bsw.arxml");
    let saved = fs::read(&input).unwrap();
    fs::remove_file(&input).unwrap();
    assert!(open_ecu_handoff(&project, &dependencies, &runtime).is_err());
    fs::write(&input, saved).unwrap();
    fs::write(project.join("unlisted-owner.txt"), b"owner bytes").unwrap();
    assert!(open_ecu_handoff(&project, &dependencies, &runtime).is_err());
    assert_eq!(
        fs::read(project.join("unlisted-owner.txt")).unwrap(),
        b"owner bytes"
    );
    fs::remove_file(project.join("unlisted-owner.txt")).unwrap();
    let metadata = project.join("handoff.json");
    let old = fs::read(&metadata).unwrap();
    let mut changed: serde_json::Value = serde_json::from_slice(&old).unwrap();
    changed["format"] = "autosar-ecu-handoff-v99".into();
    fs::write(&metadata, serde_json::to_vec(&changed).unwrap()).unwrap();
    reseal(&project);
    assert!(open_ecu_handoff(&project, &dependencies, &runtime).is_err());
    fs::write(&metadata, &old).unwrap();
    reseal(&project);
    let external = project.join("inputs/Nested/application.arxml");
    let original = fs::read(&external).unwrap();
    fs::write(&external, b"external editor bytes").unwrap();
    let changed = workspace
        .saved_integration_plan(&runtime, dependencies.mod_archive.clone())
        .err()
        .unwrap();
    assert!(
        changed.iter().any(|d| d.code == "SOURCE_CHANGED"),
        "{changed:?}"
    );
    assert_eq!(fs::read(&external).unwrap(), b"external editor bytes");
    fs::write(&external, original).unwrap();
    let wrong_dependencies = PlanDependencies {
        mod_archive: scratch.0.join("missing licensed MOD.zip"),
        xsd_archive: dependencies.xsd_archive.clone(),
    };
    assert!(open_ecu_handoff(&project, &wrong_dependencies, &runtime).is_err());
    let mut workspace = Workspace::open_ecu_handoff(&project, &dependencies, &runtime).unwrap();
    workspace
        .edit_integration(
            &runtime,
            dependencies.mod_archive.clone(),
            autosar_config_core::integration::IntegrationEdit {
                application_period_ms: Some(15),
                can_ids: Default::default(),
            },
        )
        .unwrap();
    assert!(
        workspace
            .saved_integration_plan(&runtime, dependencies.mod_archive.clone())
            .is_err()
    );
}

pub fn verify_rapid() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let sources = super::epic4_plan::inputs();
    let scratch = super::Scratch::new();
    let rapid_inputs = scratch.0.join("rapid-inputs");
    let mut paths = Vec::new();
    for input in &sources {
        let path = rapid_inputs.join(input.logical_path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, input.bytes()).unwrap();
        paths.push(path);
    }
    let mut rapid = Workspace::open(paths, dependencies.xsd_archive.clone()).unwrap();
    rapid
        .edit_integration(
            &runtime,
            dependencies.mod_archive.clone(),
            autosar_config_core::integration::IntegrationEdit {
                application_period_ms: Some(1),
                can_ids: Default::default(),
            },
        )
        .unwrap();
    let save = rapid
        .preview_integration_save(&runtime, dependencies.mod_archive.clone())
        .unwrap();
    rapid
        .save_integration_previewed(&runtime, dependencies.mod_archive.clone(), &save.revision)
        .unwrap();
    let plan = rapid
        .saved_integration_plan(&runtime, dependencies.mod_archive.clone())
        .unwrap();
    let files = plan
        .ecu_handoff_files(super::tooling::native_target())
        .unwrap();
    let project = scratch.0.join("rapid-source");
    let preview = files.preview(&project).unwrap();
    files
        .generate_previewed(&project, &preview.revision)
        .unwrap();
    let report =
        verify_ecu_project(&plan, &project, &super::tooling::execution_settings()).unwrap();
    assert!(report.passed, "{}", report.log);
}
