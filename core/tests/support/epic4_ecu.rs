use autosar_config_core::integration::{InputSource, PlanDependencies, RuntimeCatalog, build_plan};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::Duration;

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

pub(super) fn compile(project: &Path, output: &Path, control: Option<&Path>) {
    let mut command = super::tooling::ecu_build_command(project, output, "probe", control);
    let result = run_public_command(
        &mut command,
        output.parent().unwrap(),
        "ecu-build",
        Duration::from_secs(180),
    );
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

pub(super) fn run_probe(binary: &Path, stage: Option<u32>) -> Output {
    let mut command = Command::new(binary);
    if let Some(stage) = stage {
        command.arg(stage.to_string());
    }
    run_public_command(
        &mut command,
        binary.parent().unwrap(),
        &format!("ecu-probe-{}", stage.unwrap_or(0)),
        Duration::from_secs(15),
    )
}

fn check_vector_section(binary: &Path) {
    let output = Command::new("objdump")
        .arg("-h")
        .arg(binary)
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let rows: Vec<_> = text
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .filter(|fields| fields.get(1) == Some(&".os_vec"))
        .collect();
    assert_eq!(rows.len(), 1, "{text}");
    assert_eq!(u64::from_str_radix(rows[0][2], 16).unwrap(), 256);
    let symbol_section = rows[0][0].parse::<u32>().unwrap() + 1;
    let output = Command::new("objdump")
        .arg("-t")
        .arg(binary)
        .output()
        .unwrap();
    assert!(output.status.success());
    let symbols = String::from_utf8(output.stdout).unwrap();
    let tables: Vec<_> = symbols
        .lines()
        .filter(|line| line.ends_with(" Os_InterruptVectorTable"))
        .collect();
    assert_eq!(tables.len(), 1, "{symbols}");
    assert!(tables[0].contains(&format!("(sec {symbol_section:2})")));
    assert!(tables[0].contains("0x0000000000000000 Os_InterruptVectorTable"));
}

fn check_entry_sections(binary: &Path) {
    let output = Command::new("objdump")
        .arg("-h")
        .arg(binary)
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let row = text
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .find(|fields| fields.get(1) == Some(&".os_code"))
        .expect("generated OS entry code has its actual linked section");
    let section = row[0].parse::<u32>().unwrap() + 1;
    let output = Command::new("objdump")
        .arg("-t")
        .arg(binary)
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for name in [
        "Os_TaskEntry_OS_TASK_ID_Task_Ecu",
        "ErrorHook",
        "PreTaskHook",
        "PostTaskHook",
        "StartupHook",
        "ShutdownHook",
    ] {
        let suffix = format!(" {name}");
        let rows: Vec<_> = text
            .lines()
            .filter(|line| line.ends_with(&suffix))
            .collect();
        assert_eq!(rows.len(), 1, "{name}: {text}");
        assert!(
            rows[0].contains(&format!("(sec {section:2})")),
            "{name}: {}",
            rows[0]
        );
    }
}

pub(super) fn run_public_command(
    command: &mut Command,
    directory: &Path,
    name: &str,
    watchdog: Duration,
) -> Output {
    use autosar_config_core::execution::{ProcessOwner, ProcessSpec, ProcessStatus};
    let settings = super::tooling::execution_settings();
    let program = match command.get_program().to_str() {
        Some("gcc") => settings.compiler,
        Some("objdump") => settings.objdump,
        Some("python") => settings.python,
        _ => {
            let path = std::path::PathBuf::from(command.get_program());
            if path.is_absolute() {
                path
            } else {
                std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                    .map(|root| root.join(&path))
                    .find(|path| path.is_file())
                    .unwrap_or_else(|| {
                        panic!("Test command executable is unavailable: {}", path.display())
                    })
            }
        }
    };
    let logs = directory.join(format!("{name}.owned-logs"));
    fs::create_dir_all(&logs).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&logs, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let mut argv = vec![program.as_os_str().to_owned()];
    argv.extend(command.get_args().map(std::ffi::OsStr::to_owned));
    let environment = command
        .get_envs()
        .map(|(name, value)| {
            (
                name.to_owned(),
                value
                    .expect("Test commands do not remove environment variables")
                    .to_owned(),
            )
        })
        .collect();
    let spec = ProcessSpec::for_duration(
        argv,
        command.get_current_dir().unwrap_or(directory).to_path_buf(),
        environment,
        watchdog,
        logs,
    )
    .unwrap();
    let owner = ProcessOwner::new().unwrap();
    let result = owner.spawn(spec, None).unwrap().wait().unwrap();
    let stdout = fs::read(&result.stdout).unwrap();
    let stderr = fs::read(&result.stderr).unwrap();
    fs::write(directory.join(format!("{name}.stdout")), &stdout).unwrap();
    fs::write(directory.join(format!("{name}.stderr")), &stderr).unwrap();
    assert_eq!(
        result.status,
        ProcessStatus::Exited,
        "{name} exceeded host watchdog or scope closure failed: {result:?}; {}{}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr),
    );
    let exit_code = result
        .exit_code
        .expect("A normally exited root has an observed exit code");
    #[cfg(windows)]
    let status = {
        use std::os::windows::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(exit_code as u32)
    };
    #[cfg(unix)]
    let status = {
        use std::os::unix::process::ExitStatusExt;
        assert!(
            exit_code >= 0,
            "Native test command terminated by signal: {exit_code}"
        );
        std::process::ExitStatus::from_raw(exit_code << 8)
    };
    Output {
        status,
        stdout,
        stderr,
    }
}

#[cfg(windows)]
pub fn verify_public_watchdog() {
    let scratch = super::Scratch::new();
    let mut command = Command::new("python");
    command.args([
        "-c",
        "import subprocess,sys,time; child=subprocess.Popen([sys.executable,'-c','import time; time.sleep(30)'],creationflags=0x08000000); print('watchdog_child='+str(child.pid),flush=True); time.sleep(30)",
    ]);
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_public_command(
            &mut command,
            &scratch.0,
            "watchdog-probe",
            Duration::from_secs(2),
        )
    }));
    let panic = failure.expect_err("the deliberately stalled process must time out");
    let message = panic.downcast_ref::<String>().unwrap();
    assert!(message.contains("watchdog-probe exceeded host watchdog"));
    let output = fs::read_to_string(scratch.0.join("watchdog-probe.stdout")).unwrap();
    let pid = output
        .lines()
        .find_map(|line| line.strip_prefix("watchdog_child="))
        .unwrap()
        .parse::<u32>()
        .unwrap();
    let mut query = Command::new("powershell.exe");
    query.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        &format!("if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ exit 1 }}"),
    ]);
    let gone = run_public_command(
        &mut query,
        &scratch.0,
        "watchdog-descendant-query",
        Duration::from_secs(5),
    );
    assert!(gone.status.success(), "watchdog must close its descendant");
}

pub fn verify() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let inputs = super::epic4_plan::inputs();
    let plan = build_plan(&inputs, &dependencies, &runtime).unwrap();
    let project = plan
        .ecu_integration_files(super::tooling::native_target())
        .unwrap();
    for header in ["Os.h", "Os_Types.h", "Rte_Os_Type.h"] {
        let delivered = &project
            .files()
            .iter()
            .find(|(name, _)| name == &format!("os/include/{header}"))
            .unwrap()
            .1;
        assert_eq!(
            delivered,
            &fs::read(root.join("runtime/os/include").join(header)).unwrap(),
            "public OS/RTE header must be delivered unchanged: {header}"
        );
    }
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
        plan.ecu_integration_files(super::tooling::native_target())
            .unwrap()
            .files()
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
    // Compile the same independent legacy source against the moved delivery,
    // with no repository header search path or OS object linked in.
    let public_binary = scratch.0.join("delivered-public-compatibility.exe");
    let mut public_command = Command::new("gcc");
    public_command
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-I")
        .arg(moved.join("os/include"))
        .arg("-I")
        .arg(moved.join("include"))
        .arg(root.join("runtime/os/tests/public_compatibility.c"))
        .arg("-o")
        .arg(&public_binary);
    let public_compile = run_public_command(
        &mut public_command,
        &scratch.0,
        "public-compile",
        Duration::from_secs(60),
    );
    assert!(
        public_compile.status.success(),
        "{}{}",
        String::from_utf8_lossy(&public_compile.stdout),
        String::from_utf8_lossy(&public_compile.stderr)
    );
    let public_run = run_public_command(
        &mut Command::new(&public_binary),
        &scratch.0,
        "public-run",
        Duration::from_secs(5),
    );
    assert!(public_run.status.success());
    assert!(public_run.stderr.is_empty());
    assert_eq!(
        String::from_utf8(public_run.stdout)
            .unwrap()
            .replace("\r\n", "\n"),
        "public_compatibility declarations=16 evaluations=0 error_codes=23 unique=pass\n"
    );
    let build = scratch.0.join("new-independent-build");
    let compiled = run_public_command(
        &mut super::tooling::ecu_build_command(&moved, &build, "test", None),
        &scratch.0,
        "independent-build",
        Duration::from_secs(180),
    );
    assert!(
        compiled.status.success(),
        "{}{}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
    check_vector_section(&super::tooling::native_binary(&build, "ecu_probe"));
    check_entry_sections(&super::tooling::native_binary(&build, "ecu_probe"));
    let normal = run_probe(&super::tooling::native_binary(&build, "ecu_probe"), None);
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
    // Twenty completed ticks plus two physically confirmed outputs produce
    // real Pre/Post transitions, including each confirmation-only wake.
    // Diagnostic trace saves a bounded prefix; omitted markers are explicit.
    let mut expected_trace = String::from("IdcpmgrsRpq");
    for _ in 0..2 {
        expected_trace.push_str(&"pwtcdq".repeat(9));
        expected_trace.push_str("pwtcaxdqpq");
    }
    assert_eq!(expected_trace.len(), 139);
    let lifecycle = text
        .lines()
        .find(|line| line.starts_with("lifecycle=Closed"))
        .unwrap();
    let actual_trace = lifecycle
        .split_whitespace()
        .find_map(|field| field.strip_prefix("trace="))
        .unwrap();
    assert_eq!(actual_trace, &expected_trace[..127], "{text}");
    assert!(lifecycle.ends_with("trace_dropped=12"), "{text}");
    for stage in 1..=8 {
        let failed = run_probe(
            &super::tooling::native_binary(&build, "ecu_probe"),
            Some(stage),
        );
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
    let independent = run_probe(
        &super::tooling::native_binary(&independent_build, "ecu_probe"),
        None,
    );
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
        .ecu_integration_files(super::tooling::native_target())
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
    let output = run_probe(
        &super::tooling::native_binary(&variant_build, "ecu_probe"),
        None,
    );
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
        let prepared = autosar_config_core::prepared::prepare_ecu_project(
            &candidate,
            super::tooling::native_target(),
            false,
        )
        .unwrap();
        let report = prepared.native_preflight(&super::tooling::execution_settings());
        assert_eq!(
            report.status,
            autosar_config_core::prepared::PreflightStatus::Failed,
            "{symbol}: {:?}",
            report.logs,
        );
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
    let collision = build_plan(&collision, &dependencies, &runtime).unwrap();
    let prepared = autosar_config_core::prepared::prepare_ecu_project(
        &collision,
        super::tooling::native_target(),
        false,
    )
    .unwrap();
    let report = prepared.native_preflight(&super::tooling::execution_settings());
    assert_eq!(
        report.status,
        autosar_config_core::prepared::PreflightStatus::Failed,
        "{:?}",
        report.logs,
    );
    assert!(
        build_plan(&allowed, &dependencies, &runtime)
            .unwrap()
            .ecu_integration_files(super::tooling::native_target())
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
