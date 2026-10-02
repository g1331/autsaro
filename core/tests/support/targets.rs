use super::{Scratch, archive, create_pair, epic4_handoff, epic4_plan, tooling};
use autosar_config_core::{DiagnosticSettings, Direction, Workspace, schema};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(any(windows, target_os = "linux"))]
pub(super) fn windows_and_linux_ecu_targets_execute_production_protocol() {
    use autosar_config_core::execution::{ProcessSpec, run_bounded};
    use autosar_config_core::integration::{
        PlanDependencies, RuntimeCatalog, build_plan, open_ecu_handoff,
    };
    use autosar_config_core::target::{BuildTarget, ExecutionSettings};
    use std::ffi::OsString;
    use std::time::Duration;

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let resources = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::embedded().unwrap();
    let plan = build_plan(&epic4_plan::inputs(), &resources, &runtime).unwrap();
    let target = if cfg!(windows) {
        BuildTarget::WindowsX64ControlledV1
    } else {
        BuildTarget::LinuxX64ControlledV1
    };
    let settings = ExecutionSettings::from_environment().unwrap();
    let prepared = autosar_config_core::prepare_ecu_project(&plan, target, true).unwrap();
    let preflight = prepared.native_preflight(&settings);
    assert_eq!(preflight.fingerprint, prepared.fingerprint());
    assert_eq!(
        preflight.status,
        autosar_config_core::prepared::PreflightStatus::Passed,
        "{preflight:?}"
    );
    let files = prepared.into_files();
    let retained = Scratch::new();
    let scratch = retained.0.clone();
    std::mem::forget(retained);
    println!(
        "production_target={} retained_artifacts={}",
        target.spec().id,
        scratch.display()
    );
    let original = scratch.join("original source");
    let install = |directory: &Path| {
        fs::create_dir(directory).unwrap();
        for (name, bytes) in &files {
            let path = directory.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
    };
    install(&original);
    let project = scratch.join("moved sealed source");
    fs::rename(original, &project).unwrap();
    let opened = open_ecu_handoff(&project, &resources, &runtime).unwrap();
    assert_eq!(opened.target(), target);
    assert_eq!(
        opened
            .plan()
            .ecu_handoff_files(opened.target())
            .unwrap()
            .files(),
        files.as_slice(),
        "Reimport must reconstruct the same complete target source package"
    );
    let logs = scratch.join("owner-logs");
    fs::create_dir(&logs).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&logs, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let run_tool = |source: &Path, arguments: Vec<OsString>| -> Result<String, String> {
        let mut argv = vec![
            settings.python.as_os_str().into(),
            source.join("tools/ecu-tool.py").into_os_string(),
        ];
        argv.extend(arguments);
        let spec = ProcessSpec::for_duration(
            argv,
            source.to_path_buf(),
            vec![
                ("AUTOSAR_CC".into(), settings.compiler.as_os_str().into()),
                (
                    "AUTOSAR_OBJDUMP".into(),
                    settings.objdump.as_os_str().into(),
                ),
                ("AUTOSAR_GIT".into(), settings.git.as_os_str().into()),
            ],
            Duration::from_secs(300),
            logs.clone(),
        )?;
        let result = run_bounded(spec)?;
        fs::read_to_string(result.stdout).map_err(|error| error.to_string())
    };
    let production = scratch.join("production build");
    println!(
        "{}",
        run_tool(
            &project,
            vec![
                "build".into(),
                "--project".into(),
                project.as_os_str().into(),
                "--output".into(),
                production.as_os_str().into(),
                "--mode".into(),
                "host-batch".into(),
            ]
        )
        .unwrap_or_else(|error| panic!("{error}; artifacts={}", scratch.display()))
    );
    let binary = tooling::native_binary(&production, "ecu_host_batch");
    let inspection = ProcessSpec::for_duration(
        vec![
            settings.objdump.as_os_str().into(),
            "-t".into(),
            binary.as_os_str().into(),
        ],
        production.clone(),
        vec![],
        Duration::from_secs(30),
        logs.clone(),
    )
    .unwrap();
    let inspected = run_bounded(inspection).unwrap();
    let symbols = fs::read_to_string(inspected.stdout).unwrap();
    for symbol in symbols
        .lines()
        .filter_map(|line| line.split_whitespace().last())
    {
        assert!(
            ![
                "Os_Test",
                "Os_StackTest",
                "Os_TimeTest",
                "Os_TargetTest",
                "Os_ArtiTest",
                "Ecu_TargetTest",
                "vTaskOsTest",
            ]
            .iter()
            .any(|prefix| symbol.starts_with(prefix)),
            "Private test symbol linked into production: {symbol}"
        );
    }
    let mixed = scratch.join("refused-production-control");
    assert!(
        run_tool(
            &project,
            vec![
                "build".into(),
                "--project".into(),
                project.as_os_str().into(),
                "--output".into(),
                mixed.as_os_str().into(),
                "--mode".into(),
                "host-batch".into(),
                "--control-source".into(),
                root.join("core/tests/fixtures/semantic_ecu.c")
                    .into_os_string(),
            ],
        )
        .is_err()
    );
    assert!(
        !mixed.exists(),
        "Rejected mixed build must not install an output"
    );
    let verification = scratch.join("independent verify");
    let verified = run_tool(
        &project,
        vec![
            "verify".into(),
            "--project".into(),
            project.as_os_str().into(),
            "--build-directory".into(),
            verification.as_os_str().into(),
        ],
    )
    .unwrap_or_else(|error| panic!("{error}; artifacts={}", scratch.display()));
    println!("{verified}");
    for case in ["missing", "tampered", "old-format", "extra"] {
        let bad = scratch.join(case);
        install(&bad);
        match case {
            "missing" => fs::remove_file(bad.join("src/Rte.c")).unwrap(),
            "tampered" => fs::write(bad.join("src/Rte.c"), b"altered source").unwrap(),
            "old-format" => {
                let mut metadata: serde_json::Value =
                    serde_json::from_slice(&fs::read(bad.join("target.json")).unwrap()).unwrap();
                metadata["format"] = "autosar-build-target-v0".into();
                fs::write(
                    bad.join("target.json"),
                    serde_json::to_vec(&metadata).unwrap(),
                )
                .unwrap();
                epic4_handoff::reseal(&bad);
            }
            "extra" => fs::write(bad.join("rogue.c"), b"unlisted source").unwrap(),
            _ => unreachable!(),
        }
        let before: Vec<_> = files
            .iter()
            .map(|(name, _)| (name, fs::read(bad.join(name)).ok()))
            .collect();
        let output = scratch.join(format!("refused-{case}"));
        assert!(
            run_tool(
                &bad,
                vec![
                    "build".into(),
                    "--project".into(),
                    bad.as_os_str().into(),
                    "--output".into(),
                    output.as_os_str().into(),
                    "--mode".into(),
                    "host-batch".into(),
                ]
            )
            .is_err(),
            "Invalid sealed input was admitted: {case}"
        );
        assert!(
            !output.exists(),
            "Invalid source changed its destination: {case}"
        );
        assert!(open_ecu_handoff(&bad, &resources, &runtime).is_err());
        for (name, bytes) in before {
            assert_eq!(fs::read(bad.join(name)).ok(), bytes);
        }
    }
    for (name, bytes) in files {
        assert_eq!(
            fs::read(project.join(name)).unwrap(),
            bytes,
            "Offline build/verification must not mutate the sealed package"
        );
    }
    fs::remove_dir_all(scratch).unwrap();
}
pub(super) fn source_generation_does_not_require_native_executor() {
    use autosar_config_core::integration::{PlanDependencies, RuntimeCatalog, build_plan};
    use autosar_config_core::prepared::PreflightStatus;
    use autosar_config_core::resources::AssetInventory;
    use autosar_config_core::target::{BuildTarget, ExecutionSettings};
    use autosar_config_core::{prepare_ecu_project, prepare_host_project};

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let resources = PlanDependencies::explicit(
        schema::schema_archive(root),
        root.join("docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_MOD_ECUConfigurationParameters.zip"),
    ).unwrap();
    let interpreter = std::env::var_os("AUTOSAR_PYTHON")
        .expect("Supply an absolute pinned interpreter for this development test");
    assert!(Path::new(&interpreter).is_absolute());
    let scratch = Scratch::new();
    let missing = scratch.0.join(if cfg!(windows) {
        "missing-gcc.exe"
    } else {
        "missing-gcc"
    });
    assert!(!missing.exists());
    let _execution = ExecutionSettings::new(
        missing.clone(),
        scratch.0.join("missing-objdump"),
        scratch.0.join("missing-git"),
        PathBuf::from(interpreter),
    )
    .unwrap();
    let inventory = AssetInventory::from_directory(root).unwrap();
    let runtime = RuntimeCatalog::embedded().unwrap();
    let plan = build_plan(&epic4_plan::inputs(), &resources, &runtime).unwrap();
    let mut targets = Vec::new();
    for target in BuildTarget::ALL {
        let first = prepare_ecu_project(&plan, target, false).unwrap();
        let repeated = prepare_ecu_project(&plan, target, false).unwrap();
        assert_eq!(first.fingerprint(), repeated.fingerprint());
        assert_eq!(first.preflight().status, PreflightStatus::NotRun);
        assert_eq!(first.preflight().fingerprint, first.fingerprint());
        assert!(first.preflight().logs.is_empty());
        let selected: Vec<_> = first
            .files()
            .iter()
            .filter_map(|file| {
                file.source
                    .filter(|asset| asset.role == "bsw")
                    .map(|asset| asset.relative_path)
            })
            .collect();
        assert!(selected.contains(&"runtime/src/Can.c"));
        assert!(first.files().iter().any(|file| file.path == "os/src/Os.c"));
        assert!(
            first
                .files()
                .iter()
                .any(|file| file.path == "kernel/tasks.c")
        );
        let (native_host, native_kernel) = match target {
            BuildTarget::WindowsX64ControlledV1 => (
                "os/src/host/windows/Os_HostWindows.c",
                "kernel/portable/MSVC-MingW/port.c",
            ),
            BuildTarget::LinuxX64ControlledV1 => (
                "os/src/host/linux/Os_HostLinux.c",
                "kernel/portable/ThirdParty/GCC/Posix/port.c",
            ),
        };
        assert!(first.files().iter().any(|file| file.path == native_host));
        assert!(first.files().iter().any(|file| file.path == native_kernel));
        assert!(
            first
                .files()
                .iter()
                .all(|file| file.source.is_none_or(|asset| {
                    inventory.get(asset.relative_path).is_some_and(|trusted| {
                        trusted.sha256 == asset.sha256
                            && format!("{:x}", Sha256::digest(&file.bytes)) == asset.sha256
                    })
                }))
        );
        let target_data: serde_json::Value = serde_json::from_slice(
            &first
                .files()
                .iter()
                .find(|file| file.path == "target.json")
                .unwrap()
                .bytes,
        )
        .unwrap();
        assert_eq!(target_data["format"], "autosar-build-target-v1");
        assert_eq!(target_data["target"], target.spec().id);
        assert_eq!(target_data["logicalClock"], target.spec().logical_clock);
        assert_eq!(target_data["objectFormat"], target.spec().object_format);
        assert_eq!(
            target_data["kernelPatches"].as_array().unwrap().len(),
            target.spec().kernel_patches.len(),
        );
        assert_eq!(
            serde_json::to_string(&target).unwrap(),
            format!("\"{}\"", target.spec().id)
        );
        assert!(serde_json::from_str::<BuildTarget>("\"windows-x64\"").is_err());
        println!(
            "source-only target={} bsw={} fingerprint={}",
            target.spec().id,
            selected.len(),
            first.fingerprint()
        );
        targets.push((target, selected, first.fingerprint().to_owned()));
    }
    assert_eq!(targets[0].1, targets[1].1);
    assert_ne!(targets[0].2, targets[1].2);
    assert_ne!(
        targets[0].0.spec().object_format,
        targets[1].0.spec().object_format
    );

    let (mut host, _) = create_pair(&scratch.0);
    let legacy =
        prepare_host_project(&mut host, BuildTarget::WindowsX64ControlledV1, true).unwrap();
    assert!(
        legacy
            .files()
            .iter()
            .any(|file| file.path == "Ecu_Config.c")
    );
    assert!(legacy.files().iter().any(|file| file.path == "src/Can.c"));
    assert!(
        legacy
            .files()
            .iter()
            .any(|file| file.path == "handoff.json")
    );
    assert_eq!(legacy.preflight().status, PreflightStatus::NotRun);
    let source_only =
        prepare_host_project(&mut host, BuildTarget::WindowsX64ControlledV1, false).unwrap();
    let host_source = PathBuf::from(&host.view().files[0].path);
    let mut changed_bytes = fs::read(&host_source).unwrap();
    changed_bytes.extend_from_slice(b"\n<!-- preserved source annotation -->\n");
    fs::write(&host_source, changed_bytes).unwrap();
    let mut changed_host = Workspace::open(vec![host_source], resources.xsd_archive).unwrap();
    let updated = prepare_host_project(
        &mut changed_host,
        BuildTarget::WindowsX64ControlledV1,
        false,
    )
    .unwrap();
    assert_eq!(
        source_only
            .files()
            .iter()
            .find(|file| file.path == "Ecu_Config.c")
            .unwrap()
            .bytes,
        updated
            .files()
            .iter()
            .find(|file| file.path == "Ecu_Config.c")
            .unwrap()
            .bytes,
    );
    assert_ne!(source_only.fingerprint(), updated.fingerprint());
}
pub(super) fn source_generation_rejects_modified_validation_and_asset_bytes() {
    use autosar_config_core::integration::{PlanDependencies, RuntimeCatalog, build_plan};
    use autosar_config_core::resources::AssetInventory;

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let standards = PlanDependencies::from_repository(root);
    let scratch = Scratch::new();
    let inputs = epic4_plan::inputs();
    let original: Vec<_> = inputs
        .iter()
        .map(|source| (source.logical_path().to_owned(), source.bytes().to_vec()))
        .collect();
    let runtime = RuntimeCatalog::embedded().unwrap();
    for (name, original_archive, xsd) in [
        ("XSD", &standards.xsd_archive, true),
        ("MOD", &standards.mod_archive, false),
    ] {
        let changed = scratch.0.join(format!("{name}.zip"));
        let mut bytes = fs::read(original_archive).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        fs::write(&changed, &bytes).unwrap();
        assert_eq!(
            fs::metadata(original_archive).unwrap().len(),
            fs::metadata(&changed).unwrap().len()
        );
        let altered = if xsd {
            PlanDependencies::explicit(changed, standards.mod_archive.clone()).unwrap()
        } else {
            PlanDependencies::explicit(standards.xsd_archive.clone(), changed).unwrap()
        };
        assert!(
            build_plan(&inputs, &altered, &runtime).is_err(),
            "{name} bytes were accepted"
        );
    }
    let bad_zip = scratch.0.join("broken.zip");
    fs::write(&bad_zip, b"not a ZIP archive").unwrap();
    assert!(schema::validate_files(&bad_zip, &[]).is_err());
    let wrong = PlanDependencies::explicit(
        standards.xsd_archive.clone(),
        scratch.0.join("missing-MOD.zip"),
    )
    .unwrap();
    assert!(build_plan(&inputs, &wrong, &runtime).is_err());
    let wrong_xsd = PlanDependencies::explicit(
        scratch.0.join("missing-XSD.zip"),
        standards.mod_archive.clone(),
    )
    .unwrap();
    assert!(build_plan(&inputs, &wrong_xsd, &runtime).is_err());
    let tampered = "runtime/src/Can.c";
    let destination = scratch.0.join(tampered);
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    let mut original_bytes = fs::read(root.join(tampered)).unwrap();
    original_bytes[0] ^= 1;
    fs::write(&destination, &original_bytes).unwrap();
    assert!(AssetInventory::verify_directory_asset(&scratch.0, tampered).is_err());
    assert_eq!(
        inputs
            .iter()
            .map(|source| (source.logical_path().to_owned(), source.bytes().to_vec()))
            .collect::<Vec<_>>(),
        original
    );
}
pub(super) fn linux_legacy_security_profile_is_rejected_during_preparation() {
    use autosar_config_core::prepare_host_project;
    use autosar_config_core::target::BuildTarget;

    let scratch = Scratch::new();
    let mut project = Workspace::create(&scratch.0.join("Secure"), "Secure", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(frame, "LiveValue".into(), 0, 32, 7)
        .unwrap()
        .signals[0]
        .path
        .clone();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: true,
            reset_routine_id: None,
            security_enabled: true,
        })
        .unwrap();
    project.save().unwrap();
    assert!(prepare_host_project(&mut project, BuildTarget::LinuxX64ControlledV1, false).is_err());
    let windows =
        prepare_host_project(&mut project, BuildTarget::WindowsX64ControlledV1, false).unwrap();
    assert!(
        windows
            .files()
            .iter()
            .any(|file| file.path == "src/Security.c")
    );
}
