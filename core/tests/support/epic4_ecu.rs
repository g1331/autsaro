use autosar_config_core::integration::{InputSource, PlanDependencies, RuntimeCatalog, build_plan};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

fn changed_inputs(
    original: &[InputSource],
    edit: impl Fn(&str, String) -> String,
) -> Vec<InputSource> {
    original
        .iter()
        .map(|source| {
            let text = String::from_utf8(source.bytes().to_vec()).unwrap();
            InputSource::new(
                source.logical_path(),
                edit(source.logical_path(), text).into_bytes(),
            )
            .unwrap()
        })
        .collect()
}

fn parameter(text: &str, name: &str, old: u32, new: u32) -> String {
    let marker = format!("/{name}</DEFINITION-REF>");
    let mut result = String::new();
    let mut rest = text;
    while let Some(index) = rest.find(&marker) {
        let end = index + marker.len();
        result.push_str(&rest[..end]);
        rest = &rest[end..];
        let value_start = rest.find("<VALUE>").unwrap();
        let value_end = rest.find("</VALUE>").unwrap();
        result.push_str(&rest[..value_start]);
        let value = &rest[value_start + 7..value_end];
        result.push_str(&format!(
            "<VALUE>{}</VALUE>",
            if value == old.to_string() {
                new.to_string()
            } else {
                value.to_owned()
            }
        ));
        rest = &rest[value_end + 8..];
    }
    result.push_str(rest);
    result
}

fn compile(project: &Path, output: &Path, control: Option<&Path>) {
    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(project.join("build.ps1"))
        .arg("-OutputDirectory")
        .arg(output);
    if let Some(source) = control {
        command.arg("-ControlSource").arg(source);
    }
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

fn run_probe(binary: &Path, stage: Option<u32>) -> Output {
    let mut command = Command::new(binary);
    if let Some(stage) = stage {
        command.arg(stage.to_string());
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let started = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() >= Duration::from_secs(15) {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "ECU probe exceeded host watchdog: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    child.wait_with_output().unwrap()
}

pub fn verify() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let inputs = super::epic4_plan::inputs();
    let plan = build_plan(&inputs, &dependencies, &runtime).unwrap();
    let project = plan.ecu_integration_files().unwrap();
    let metadata: serde_json::Value = serde_json::from_slice(
        &project
            .files()
            .iter()
            .find(|(name, _)| name == "integration.json")
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(
        metadata["originalBswSources"].as_object().unwrap().len(),
        plan.description().runtime_sources.len()
    );
    for (original, expected) in &plan.description().runtime_sources {
        let delivered = metadata["originalBswSources"][original].as_str().unwrap();
        let bytes = &project
            .files()
            .iter()
            .find(|(name, _)| name == delivered)
            .unwrap()
            .1;
        assert_eq!(
            &format!("{:x}", Sha256::digest(bytes)),
            expected,
            "{original}"
        );
    }
    assert_eq!(
        project.files(),
        plan.ecu_integration_files().unwrap().files()
    );
    assert!(
        project
            .files()
            .iter()
            .all(|(_, bytes)| !String::from_utf8_lossy(bytes).contains("D:\\Codebase\\Autosar"))
    );
    let scratch = super::Scratch::new();
    let output = scratch.0.join("source-project");
    let preview = project.preview(&output).unwrap();
    project
        .generate_previewed(&output, &preview.revision)
        .unwrap();
    assert!(!output.join("os/src/Os_Advance.c").exists());
    assert!(!output.join("src/Os.c").exists());
    assert!(!output.join("src/Can_HostLock.c").exists());
    assert!(output.join("kernel-compat/include/StackMacros.h").is_file());
    assert!(output.join("kernel/include/stack_macros.h").is_file());
    let moved = scratch.0.join("moved-source-project");
    fs::rename(&output, &moved).unwrap();
    let build = scratch.0.join("new-independent-build");
    let compiled = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(moved.join("build.ps1"))
        .arg("-OutputDirectory")
        .arg(&build)
        .arg("-TestMode")
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}{}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
    let normal = run_probe(&build.join("ecu_probe.exe"), None);
    assert!(
        normal.status.success(),
        "{}{}",
        String::from_utf8_lossy(&normal.stdout),
        String::from_utf8_lossy(&normal.stderr)
    );
    let text = String::from_utf8(normal.stdout).unwrap();
    assert!(text.contains("ecu_probe completed=20"), "{text}");
    assert!(text.contains("trace=IdcpmgrsR"), "{text}");
    assert_eq!(text.matches("ecu_output epoch=").count(), 2, "{text}");
    assert_eq!(text.matches("wtcaxd").count(), 2, "{text}");
    assert_eq!(text.matches("wtcd").count(), 18, "{text}");
    for stage in 1..=8 {
        let failed = run_probe(&build.join("ecu_probe.exe"), Some(stage));
        assert!(!failed.status.success(), "stage {stage}");
        let text = String::from_utf8(failed.stdout).unwrap();
        assert!(
            text.contains(&format!("ecu_shutdown reason=7 state=3 fail_stage={stage}")),
            "{text}"
        );
        assert!(
            !text.contains("ecu_probe completed=") && !text.contains("ecu_output"),
            "{text}"
        );
        assert!(!text.contains("trace=IdcpmgrsR"), "{text}");
    }
    let independent_build = scratch.0.join("independent-consumer-build");
    compile(
        &moved,
        &independent_build,
        Some(&root.join("core/tests/fixtures/ecu_control.c")),
    );
    let independent = run_probe(&independent_build.join("ecu_probe.exe"), None);
    assert!(
        independent.status.success(),
        "{}{}",
        String::from_utf8_lossy(&independent.stdout),
        String::from_utf8_lossy(&independent.stderr)
    );
    assert!(
        String::from_utf8(independent.stdout)
            .unwrap()
            .contains("independent_control signal=2 default_session_did=2 completed=20")
    );

    let variant = changed_inputs(&inputs, |name, text| {
        let mut text = text
            .replace("EchoApplication", "LocalApplication")
            .replace("ReferenceEcu", "LocalEcu")
            .replace("Task_Ecu", "OwnerTask")
            .replace("RxValue", "ZReceive")
            .replace("TxValue", "ASend")
            .replace("Dcm_DataElement_ApplicationValueType", "SnapshotBytes")
            .replace("<PERIOD>0.01</PERIOD>", "<PERIOD>0.020</PERIOD>")
            .replace("<VALUE>0.01</VALUE>", "<VALUE>0.020</VALUE>")
            .replace("<VALUE>800</VALUE>", "<VALUE>1100</VALUE>")
            .replace("<VALUE>801</VALUE>", "<VALUE>1101</VALUE>")
            .replace("<VALUE>1792</VALUE>", "<VALUE>1793</VALUE>")
            .replace("<VALUE>1800</VALUE>", "<VALUE>1801</VALUE>");
        for (old, new) in [(800, 1100), (801, 1101), (1792, 1793), (1800, 1801)] {
            text = text.replace(
                &format!("<IDENTIFIER>{old}</IDENTIFIER>"),
                &format!("<IDENTIFIER>{new}</IDENTIFIER>"),
            );
        }
        if name == "ecuc.arxml" {
            for (parameter_name, old, new) in [
                ("OsAlarmAlarmTime", 10, 20),
                ("OsAlarmCycleTime", 10, 20),
                ("CanIfTxPduId", 1, 5),
                ("CanIfTxPduId", 3, 7),
                ("CanIfRxPduId", 0, 4),
                ("CanIfRxPduId", 2, 6),
            ] {
                text = parameter(&text, parameter_name, old, new);
            }
            for position in (1..=6).rev() {
                text = parameter(&text, "RtePositionInTask", position, position * 10);
                text = parameter(&text, "RteBswPositionInTask", position, position * 10);
            }
        }
        text
    });
    let variant_plan =
        build_plan(&variant, &dependencies, &runtime).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(variant_plan.description().component.period_ms, 20);
    let variant_project = variant_plan
        .ecu_integration_files()
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    let variant_directory = scratch.0.join("variant-source");
    let preview = variant_project.preview(&variant_directory).unwrap();
    variant_project
        .generate_previewed(&variant_directory, &preview.revision)
        .unwrap();
    let config = fs::read_to_string(variant_directory.join("src/Ecu_Config.c")).unwrap();
    assert!(
        config.contains("\"OwnerTask\"") && config.contains("\"LocalEcu\""),
        "{config}"
    );
    let variant_build = scratch.0.join("variant-build");
    compile(&variant_directory, &variant_build, None);
    let output = run_probe(&variant_build.join("ecu_probe.exe"), None);
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert_eq!(text.matches("ecu_output epoch=").count(), 1, "{text}");
    assert!(text.contains("pdu=5 id=1101 dlc=4"), "{text}");
    for symbol in [
        "main",
        "Ecu_TargetTask",
        "Ecu_TargetReadDid",
        "Ecu_ApplicationInitialize",
    ] {
        let changed = changed_inputs(&inputs, |_, text| {
            text.replace(
                "<SYMBOL>EchoApplication_Periodic</SYMBOL>",
                &format!("<SYMBOL>{symbol}</SYMBOL>"),
            )
        });
        let candidate = build_plan(&changed, &dependencies, &runtime)
            .unwrap_or_else(|issues| panic!("{symbol}: {issues:?}"));
        assert!(candidate.ecu_integration_files().is_err(), "{symbol}");
    }
    let allowed = changed_inputs(&inputs, |_, text| {
        text.replace(
            "<SYMBOL>EchoApplication_Periodic</SYMBOL>",
            "<SYMBOL>Ecu_UnownedApplication</SYMBOL>",
        )
    });
    let collision = changed_inputs(&inputs, |_, text| {
        text.replace("Dcm_DataElement_ApplicationValueType", "Os_TargetConfig")
    });
    assert!(
        build_plan(&collision, &dependencies, &runtime)
            .unwrap()
            .ecu_integration_files()
            .is_err()
    );
    assert!(
        build_plan(&allowed, &dependencies, &runtime)
            .unwrap()
            .ecu_integration_files()
            .is_ok()
    );
    let wide = changed_inputs(&inputs, |_, text| parameter(&text, "CanIfTxPduId", 1, 256));
    assert!(
        build_plan(&wide, &dependencies, &runtime)
            .err()
            .unwrap()
            .iter()
            .any(|issue| issue.code == "CANIF_HANDLE_WIDTH")
    );
    let reordered = changed_inputs(&inputs, |_, text| {
        let text = parameter(&text, "RteBswPositionInTask", 1, 8);
        parameter(&text, "RteBswPositionInTask", 6, 1)
    });
    assert!(
        build_plan(&reordered, &dependencies, &runtime)
            .err()
            .unwrap()
            .iter()
            .any(|issue| issue.code == "SCHEDULE_ORDER")
    );
    let snapshot: Vec<_> = project
        .files()
        .iter()
        .map(|(name, _)| (name.clone(), fs::read(moved.join(name)).unwrap()))
        .collect();
    let old_preview = project.preview(&moved).unwrap();
    assert!(
        variant_project
            .generate_previewed(&moved, &old_preview.revision)
            .is_err()
    );
    for (name, bytes) in &snapshot {
        assert_eq!(
            fs::read(moved.join(name)).unwrap(),
            *bytes,
            "stale variant preview: {name}"
        );
    }
    let cases: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("core/tests/fixtures/epic4/negative/cases.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(cases["cases"].as_array().unwrap().len(), 18);
    for case in cases["cases"].as_array().unwrap() {
        let mut changed = inputs.clone();
        for override_path in case["overrides"].as_array().unwrap() {
            let path = root
                .join("core/tests/fixtures/epic4")
                .join(override_path.as_str().unwrap());
            let name = path.file_name().unwrap().to_str().unwrap();
            *changed
                .iter_mut()
                .find(|source| source.logical_path() == name)
                .unwrap() = InputSource::new(name, fs::read(&path).unwrap()).unwrap();
        }
        assert!(
            build_plan(&changed, &dependencies, &runtime).is_err(),
            "{case}"
        );
        for (name, bytes) in &snapshot {
            assert_eq!(
                fs::read(moved.join(name)).unwrap(),
                *bytes,
                "{case}: {name}"
            );
        }
    }
    fs::create_dir(moved.join("kernel/owner-empty-directory")).unwrap();
    assert!(project.preview(&moved).is_err());
    fs::remove_dir(moved.join("kernel/owner-empty-directory")).unwrap();
    let untouched = project.preview(&moved).unwrap();
    fs::write(moved.join("include/Ecu_Target.h"), b"owner edited header").unwrap();
    assert!(
        project
            .generate_previewed(&moved, &untouched.revision)
            .is_err()
    );
    assert_eq!(
        fs::read(moved.join("include/Ecu_Target.h")).unwrap(),
        b"owner edited header"
    );
    let original = snapshot
        .iter()
        .find(|(name, _)| name == "include/Ecu_Target.h")
        .unwrap();
    fs::write(moved.join(&original.0), &original.1).unwrap();
    fs::write(moved.join("owner-note.txt"), b"retain my source note").unwrap();
    assert!(project.preview(&moved).is_err());
    assert_eq!(
        fs::read(moved.join("owner-note.txt")).unwrap(),
        b"retain my source note"
    );
}
