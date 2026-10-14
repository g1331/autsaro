//! Generated OS configuration built and run with the target toolchain.
use super::*;
use autosar_config_core::integration::InputSource;
use std::path::{Path, PathBuf};

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
fn checked(
    command: &mut std::process::Command,
    directory: &Path,
    name: &str,
) -> std::process::Output {
    let output = super::tooling::run_public_command(
        command,
        directory,
        name,
        std::time::Duration::from_secs(180),
    );
    assert!(
        output.status.success(),
        "{name}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// Builds `project` with the delivered `ecu-tool.py` and returns the binary.
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
fn build_ecu(project: &Path, output: &Path, mode: &str, control: Option<&Path>) -> PathBuf {
    checked(
        &mut super::tooling::ecu_build_command(project, output, mode, control),
        output.parent().unwrap(),
        &format!(
            "build-{}",
            output
                .file_name()
                .unwrap()
                .to_string_lossy()
                .replace(' ', "-")
        ),
    );
    super::tooling::native_binary(output, "ecu_probe")
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
fn run(binary: &Path, arguments: &[&str]) -> String {
    let mut command = std::process::Command::new(binary);
    command.args(arguments);
    let output = checked(
        &mut command,
        binary.parent().unwrap(),
        &format!("run-{}", arguments.join("-")),
    );
    String::from_utf8(output.stdout).unwrap()
}

/// SWS_Os_00393 conversion macros and the ECUC counter constants, checked by
/// the independent literal oracle `ecu_counter_time.c` and by header mutants
/// that the oracle must reject (exit 1: scale, 2: double evaluation, 3: constant).
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn counter_header_oracles() {
    let scratch = Scratch::new();
    let compile = |project: &Path, binary: &Path, renamed: bool, maximum: u32| {
        let mut command = super::tooling::c_compiler();
        command
            .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-I"])
            .arg(project.join("os/include"))
            .arg("-I")
            .arg(project.join("generated/include"))
            .arg("-I")
            .arg(project.join("bsw/include"))
            .arg("-I")
            .arg(project.join("integration/include"))
            .arg(format!("-DEXPECTED_COUNTER_MAX={maximum}u"));
        if renamed {
            command.arg("-DCOUNTER_TIME_RENAMED=1");
        }
        command
            .arg(fixture("ecu_counter_time.c"))
            .arg("-o")
            .arg(binary);
        checked(&mut command, &scratch.0, "counter-time-compile");
    };
    for (label, renamed, maximum) in [
        ("default", false, 65535u32),
        ("renamed", true, 65535),
        ("alternate-maximum", false, 4095),
    ] {
        let root = scratch.0.join(label);
        std::fs::create_dir(&root).unwrap();
        let (project, _) = project(
            &counter_variant(renamed, maximum),
            &root,
            super::tooling::native_target(),
        );
        let counter = if renamed {
            "RenamedCounter"
        } else {
            "SystemCounter"
        };
        let binary = super::tooling::native_binary(&root, "counter_time");
        compile(&project, &binary, renamed, maximum);
        let stdout = run(&binary, &[]);
        assert_eq!(
            stdout.trim(),
            "counter_time PASS: 24 literal unit values; 4 single evaluations; 4 expressions"
        );
        let header_path = project.join("generated/include/Os_Counter.h");
        let header = std::fs::read_to_string(&header_path).unwrap();
        // Macro values are aligned, so a mutant replaces the whole definition.
        let redefine = |macro_name: &str, value: &str| {
            let mut text = String::new();
            for line in header.lines() {
                let mut words = line.split_whitespace();
                if words.next() == Some("#define") && words.next() == Some(macro_name) {
                    text.push_str(&format!("#define {macro_name} {value}\n"));
                } else {
                    text.push_str(&format!("{line}\n"));
                }
            }
            text
        };
        for (name, mutant, expected_exit) in [
            (
                "wrong-nanosecond-scale",
                header.replace("* UINT64_C(1000000)", "* UINT64_C(2000000)"),
                1,
            ),
            (
                "duplicate-argument-evaluation",
                header.replace(
                    &format!("(Os_TicksToNs_{counter}((ticks)))"),
                    &format!("((void)(ticks), Os_TicksToNs_{counter}((ticks)))"),
                ),
                2,
            ),
            (
                "wrong-counter-maximum",
                redefine(
                    &format!("OSMAXALLOWEDVALUE_{counter}"),
                    &format!("UINT64_C({})", maximum - 1),
                ),
                3,
            ),
            (
                "wrong-counter-base",
                redefine(&format!("OSTICKSPERBASE_{counter}"), "UINT64_C(2)"),
                3,
            ),
            (
                "wrong-counter-minimum-cycle",
                redefine(&format!("OSMINCYCLE_{counter}"), "UINT64_C(2)"),
                3,
            ),
            (
                "wrong-system-tick-duration",
                redefine("OSTICKDURATION", "UINT64_C(2000000)"),
                3,
            ),
            (
                "wrong-counter-id-maximum-alias",
                redefine(
                    &format!("OSMAXALLOWEDVALUE_OS_COUNTER_ID_{counter}"),
                    &format!("OSTICKSPERBASE_{counter}"),
                ),
                3,
            ),
        ] {
            assert_ne!(header, mutant, "{name}");
            std::fs::write(&header_path, &mutant).unwrap();
            let mutated = super::tooling::native_binary(&root, name);
            compile(&project, &mutated, renamed, maximum);
            let result = super::tooling::run_public_command(
                &mut std::process::Command::new(&mutated),
                &root,
                name,
                std::time::Duration::from_secs(30),
            );
            assert_eq!(result.status.code(), Some(expected_exit), "{label} {name}");
        }
        std::fs::write(&header_path, &header).unwrap();
    }
}

/// A moved STANDARD delivery supplies its own OsStatus guard: a conflicting
/// consumer definition is refused at preprocessing, and the build runs the
/// independent `ecu_control.c` vectors.
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn standard_status_build() {
    let scratch = Scratch::new();
    let mut sources = single_component_sources();
    parameter(&mut sources, "OperatingSystem", "OsStatus", "STANDARD");
    let (original, _) = project(&sources, &scratch.0, super::tooling::native_target());
    let moved = scratch.0.join("moved standard target");
    std::fs::rename(&original, &moved).unwrap();
    let mut conflict = std::process::Command::new("gcc");
    conflict
        .args(["-std=c99", "-E", "-x", "c", "-DOS_STATUS_EXTENDED=1", "-I"])
        .arg(moved.join("os/include"))
        .arg("-I")
        .arg(moved.join("generated/include"))
        .arg("-I")
        .arg(moved.join("bsw/include"))
        .arg("-I")
        .arg(moved.join("integration/include"))
        .arg(fixture("ecu_standard_control.c"));
    let result = super::tooling::run_public_command(
        &mut conflict,
        &scratch.0,
        "standard-conflict",
        std::time::Duration::from_secs(60),
    );
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("OsStatus differs from the validated plan"),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let binary = build_ecu(
        &moved,
        &scratch.0.join("standard-build"),
        "probe",
        Some(&fixture("ecu_standard_control.c")),
    );
    let stdout = run(&binary, &[]);
    assert!(
        stdout.contains("independent_control signal=2 default_session_did=2 completed=20"),
        "{stdout}"
    );
}

/// RES_SCHEDULER (SWS_Os_00801): with OsUseResScheduler TRUE the generated
/// resource has ceiling of the task, task access only and GetResource returns
/// E_OK; FALSE configures no resource and GetResource is E_OS_ID.
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn scheduler_resource_runs() {
    let scratch = Scratch::new();
    for (value, resource, enabled) in [
        ("false", false, false),
        ("true", false, true),
        ("0", false, false),
        ("1", true, true),
    ] {
        let root = scratch.0.join(format!("scheduler {value}"));
        std::fs::create_dir(&root).unwrap();
        let (project, _) = project(
            &scheduler_variant(value, resource),
            &root,
            super::tooling::native_target(),
        );
        let binary = build_ecu(
            &project,
            &root.join("build"),
            "test",
            Some(&fixture("ecu_scheduler_control.c")),
        );
        let stdout = run(&binary, &[if enabled { "true" } else { "false" }]);
        assert!(
            stdout.contains(&format!(
                "scheduler_consumer enabled={} calls=2 reason=0",
                usize::from(enabled)
            )),
            "{value}: {stdout}"
        );
    }
}

/// The debugger expressions of an ARTI description as an independent C probe
/// (`arti_reader.py` writes the same probe after its official checks).
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
fn arti_expression_probe(project: &Path, header: &Path) {
    let xml =
        std::fs::read_to_string(project.join("generated/descriptions/Os_Arti.arxml")).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let expressions = doc
        .descendants()
        .filter(|node| {
            node.has_tag_name("ECUC-CONTAINER-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/ArtiExpression"))
                })
        })
        .map(|node| {
            node.descendants()
                .find(|child| child.has_tag_name("ECUC-TEXTUAL-PARAM-VALUE"))
                .and_then(|value| value.children().find(|child| child.has_tag_name("VALUE")))
                .and_then(|value| value.text())
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert!(!expressions.is_empty());
    let mut probe = String::from(
        "static void arti_expression_probe(void) {\n    volatile uintptr_t observed = 0u;\n",
    );
    for expression in expressions {
        assert!(
            !expression.contains('('),
            "call in debugger expression: {expression}"
        );
        probe.push_str(&format!("    observed = (uintptr_t)({expression});\n"));
    }
    probe.push_str("    (void)observed;\n}\n");
    std::fs::write(header, probe).unwrap();
}

/// ARTI R24-11 hooks observed at runtime by the independent consumer
/// `ecu_arti_control.c`: task/alarm/table/resource state agrees with the OS
/// services, every service error reaches the ErrorHook, no internal service
/// masquerades as application SetEvent, nothing is dropped.
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn arti_consumer() {
    let scratch = Scratch::new();
    for (label, os, sources) in [
        ("alarms", "Os", single_component_sources()),
        ("renamed", "OsAlt", renamed_os_variant()),
        ("tables", "Os", schedule_table_variant()),
    ] {
        let root = scratch.0.join(format!("arti {label}"));
        std::fs::create_dir(&root).unwrap();
        let (project, _) = project(&sources, &root, super::tooling::native_target());
        let consumer = root.join("consumer");
        std::fs::create_dir(&consumer).unwrap();
        for name in ["ecu_arti_control.c", "ecu_control.c"] {
            std::fs::copy(fixture(name), consumer.join(name)).unwrap();
        }
        arti_expression_probe(&project, &consumer.join("Arti_ExpressionProbe.h"));
        let binary = build_ecu(
            &project,
            &root.join("build"),
            "test",
            Some(&consumer.join("ecu_arti_control.c")),
        );
        let stdout = run(&binary, &[os]);
        assert!(
            stdout.contains(&format!(
                "arti_consumer os={os} observations=2 getters=2 errors=2 internal_services=0 dropped=0"
            )),
            "{label}: {stdout}"
        );
    }
}

/// (address, section VMA, size) of `symbol` and its section in a linked binary,
/// for ELF (absolute symbol values) and PE/COFF (section-relative values).
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
fn section_offset(binary: &Path, symbol: &str, section: &str) -> (u64, u64) {
    let objdump = super::tooling::execution_settings().objdump;
    let dump = |flag: &str| {
        let output = std::process::Command::new(&objdump)
            .arg(flag)
            .arg(binary)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    };
    let headers = dump("-h");
    let row = headers
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .find(|fields| fields.get(1) == Some(&section))
        .unwrap_or_else(|| panic!("{section} is not linked: {headers}"));
    let size = u64::from_str_radix(row[2], 16).unwrap();
    let vma = u64::from_str_radix(row[3], 16).unwrap();
    let symbols = dump("-t");
    let lines = symbols
        .lines()
        .filter(|line| line.split_whitespace().last() == Some(symbol))
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 1, "{symbol}: {symbols}");
    let offset = if let Some((_, value)) = lines[0].rsplit_once(" 0x") {
        u64::from_str_radix(value.split_whitespace().next().unwrap(), 16).unwrap()
    } else {
        u64::from_str_radix(lines[0].split_whitespace().next().unwrap(), 16).unwrap() - vma
    };
    (offset, size)
}

/// A renamed and renumbered single-application variant: task, ECU, CAN
/// identifiers, schedule positions and a 20 ms application/transmission period.
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
fn renumbered_variant() -> Vec<InputSource> {
    let mut sources = replaced(&single_component_sources(), "Task_Ecu", "OwnerTask");
    sources = replaced(&sources, "ReferenceEcu", "LocalEcu");
    for (old, new) in [(800, 1100), (801, 1101), (1792, 1793), (1800, 1801)] {
        sources = replaced(
            &sources,
            &format!("<IDENTIFIER>{old}</IDENTIFIER>"),
            &format!("<IDENTIFIER>{new}</IDENTIFIER>"),
        );
        sources = replaced(
            &sources,
            &format!("<VALUE>{old}</VALUE>"),
            &format!("<VALUE>{new}</VALUE>"),
        );
    }
    sources = replaced(&sources, "<PERIOD>0.01</PERIOD>", "<PERIOD>0.02</PERIOD>");
    parameter(&mut sources, "Alarm_App", "OsAlarmAlarmTime", "20");
    parameter(&mut sources, "Alarm_App", "OsAlarmCycleTime", "20");
    parameter(&mut sources, "TxValuePdu", "ComTxModeTimePeriod", "0.02");
    parameter(&mut sources, "Tx", "ComMainTxTimeBase", "0.02");
    for (owner, definition, handle) in [
        ("RxValue", "CanIfRxPduId", "1"),
        ("TxValue", "CanIfTxPduId", "1"),
        ("DiagRequest", "CanIfRxPduId", "0"),
        ("DiagResponse", "CanIfTxPduId", "0"),
    ] {
        parameter(&mut sources, owner, definition, handle);
    }
    // Spread every RTE/BSW position by 10 while keeping the order.
    let ecuc = sources
        .iter_mut()
        .find(|source| source.logical_path() == "ecuc.arxml")
        .unwrap();
    let mut text = std::str::from_utf8(ecuc.bytes()).unwrap().to_owned();
    let ranges = {
        let document = roxmltree::Document::parse(&text).unwrap();
        document
            .descendants()
            .filter(|node| {
                node.has_tag_name("VALUE")
                    && node.parent().is_some_and(|parent| {
                        parent.children().any(|child| {
                            child.has_tag_name("DEFINITION-REF")
                                && child
                                    .text()
                                    .is_some_and(|text| text.ends_with("PositionInTask"))
                        })
                    })
            })
            .map(|node| (node.range(), node.text().unwrap().parse::<u32>().unwrap()))
            .collect::<Vec<_>>()
    };
    assert_eq!(ranges.len(), 11);
    for (range, position) in ranges.into_iter().rev() {
        text.replace_range(range, &format!("<VALUE>{}</VALUE>", position * 10));
    }
    *ecuc = InputSource::new("ecuc.arxml", text.into_bytes()).unwrap();
    sources
}

/// The delivered public OS API compiles for an independent legacy consumer;
/// the vector table and OS entries are linked in their mapped sections; the
/// independent `ecu_control.c` vectors pass with alarms and schedule tables;
/// a renamed and renumbered variant runs with its own identities.
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn delivery_and_independent_control() {
    let scratch = Scratch::new();
    let (original, _) = project(
        &single_component_sources(),
        &scratch.0,
        super::tooling::native_target(),
    );
    let moved = scratch.0.join("moved delivery");
    std::fs::rename(&original, &moved).unwrap();
    // No repository header path and no OS object: only the moved delivery.
    let public = super::tooling::native_binary(&scratch.0, "public_compatibility");
    let mut compile = super::tooling::c_compiler();
    compile
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic", "-I"])
        .arg(moved.join("os/include"))
        .arg("-I")
        .arg(moved.join("generated/include"))
        .arg("-I")
        .arg(moved.join("bsw/include"))
        .arg("-I")
        .arg(moved.join("integration/include"))
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("runtime/os/tests/public_compatibility.c"),
        )
        .arg("-o")
        .arg(&public);
    checked(&mut compile, &scratch.0, "public-compile");
    assert_eq!(
        run(&public, &[]).replace("\r\n", "\n"),
        "public_compatibility declarations=16 evaluations=0 error_codes=23 unique=pass\n"
    );

    let probe = build_ecu(&moved, &scratch.0.join("probe build"), "test", None);
    // The 256-byte vector table starts its own section; OS entries and hooks
    // are mapped through Os_MemMap.h into the OS code section.
    assert_eq!(
        section_offset(&probe, "Os_InterruptVectorTable", ".os_vec"),
        (0, 256)
    );
    let sections = super::tooling::symbol_sections(
        Path::new(&super::tooling::execution_settings().objdump),
        &probe,
    );
    for name in [
        "Os_TaskEntry_OS_TASK_ID_Task_Ecu",
        "ErrorHook",
        "PreTaskHook",
        "PostTaskHook",
        "StartupHook",
        "ShutdownHook",
    ] {
        let found = sections
            .iter()
            .filter(|(symbol, _)| symbol == name)
            .map(|(_, section)| section.as_str())
            .collect::<Vec<_>>();
        assert_eq!(found, [".os_code"], "{name}");
    }
    let stdout = run(&probe, &[]);
    assert!(stdout.contains("ecu_probe completed=20"), "{stdout}");

    for (label, sources) in [
        ("alarms", single_component_sources()),
        ("tables", schedule_table_variant()),
    ] {
        let root = scratch.0.join(format!("control {label}"));
        std::fs::create_dir(&root).unwrap();
        let (project, _) = project(&sources, &root, super::tooling::native_target());
        let binary = build_ecu(
            &project,
            &root.join("build"),
            "probe",
            Some(&fixture("ecu_control.c")),
        );
        let stdout = run(&binary, &[]);
        assert!(
            stdout.contains("independent_control signal=2 default_session_did=2 completed=20"),
            "{label}: {stdout}"
        );
    }

    let root = scratch.0.join("renumbered");
    std::fs::create_dir(&root).unwrap();
    let sources = renumbered_variant();
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert!(
        plan.description()
            .schedule
            .entities
            .iter()
            .filter(|entity| entity.application)
            .all(|entity| entity.period_ms == 20)
    );
    let (project, files) = project(&sources, &root, super::tooling::native_target());
    let config = file(&files, "generated/src/Ecu_Config.c");
    assert!(config.contains("\"OwnerTask\""), "{config}");
    let binary = build_ecu(&project, &root.join("build"), "test", None);
    let stdout = run(&binary, &[]);
    // Com periodic transmission (SWS_Com_00495) at the 20 ms period with the
    // permuted CanIf handle 1 and CAN identifier 1101; nothing at 10 ms.
    let outputs = stdout
        .lines()
        .filter(|line| line.starts_with("ecu_output "))
        .collect::<Vec<_>>();
    assert_eq!(outputs.len(), 1, "{stdout}");
    assert!(
        outputs[0].starts_with("ecu_output epoch=20 ")
            && outputs[0].ends_with("pdu=1 id=1101 dlc=4"),
        "{stdout}"
    );
}

/// The delivered OS counter service links into the counter-service harness
/// (`tests/python/native/os_suites.py`) and rejects wrong bindings, units and
/// signatures.
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn counter_service_native() {
    let scratch = Scratch::new();
    for counter in ["SystemCounter", "AuxClock"] {
        let mut sources = single_component_sources();
        parameter(
            &mut sources,
            "SystemCounter",
            "OsCounterMaxAllowedValue",
            "15",
        );
        if counter != "SystemCounter" {
            sources = replaced(&sources, "SystemCounter", counter);
        }
        let root = scratch.0.join(counter);
        std::fs::create_dir(&root).unwrap();
        let (project, _) = project(&sources, &root, super::tooling::native_target());
        let stage = root.join("native");
        std::fs::create_dir(&stage).unwrap();
        let mut command = super::tooling::python_command();
        command
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/python/native/os_suites.py"))
            .arg("counter-service")
            .arg(&stage)
            .arg(&project)
            .arg(counter);
        checked(&mut command, &stage, "counter-service");
    }
}

/// A BSW initialization stage that fails closes the ECU through
/// `ShutdownOS(E_OS_STATE)` before any output, and retirement undoes only the
/// stages that began, newest first (#76). Det (2) and the routing tables (5)
/// have no deinitialization service; stage 7 is the diagnostic transport.
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn bootstrap_stage_failure_closes_without_output_and_retires_started_stages() {
    let scratch = Scratch::new();
    let (project, _) = project(
        &single_component_sources(),
        &scratch.0,
        super::tooling::native_target(),
    );
    let probe = build_ecu(&project, &scratch.0.join("build"), "test", None);
    let retired = |below: u32| {
        [8, 7, 6, 4, 3, 1]
            .into_iter()
            .filter(|stage| *stage < below)
            .map(|stage| format!("ecu_retire stage={stage}"))
            .collect::<Vec<_>>()
    };
    for stage in 0..=8u32 {
        let name = format!("stage-{stage}");
        let output = super::tooling::run_public_command(
            std::process::Command::new(&probe).arg(stage.to_string()),
            &scratch.0,
            &name,
            std::time::Duration::from_secs(30),
        );
        let stdout = String::from_utf8(output.stdout)
            .unwrap()
            .replace("\r\n", "\n");
        let lines = stdout
            .lines()
            .filter(|line| line.starts_with("ecu_retire "))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if stage == 0 {
            assert!(output.status.success(), "{name}: {stdout}");
            assert!(stdout.contains("ecu_probe completed=20"), "{stdout}");
            assert!(
                stdout.contains("ecu_shutdown reason=0 state=4 fail_stage=0"),
                "{stdout}"
            );
            assert_eq!(lines, retired(9), "{stdout}");
        } else {
            assert_eq!(output.status.code(), Some(7), "{name}: {stdout}");
            assert!(
                stdout.contains(&format!("ecu_shutdown reason=7 state=3 fail_stage={stage}")),
                "{stdout}"
            );
            assert!(!stdout.contains("ecu_output"), "{stdout}");
            assert!(!stdout.contains("ecu_probe completed"), "{stdout}");
            assert_eq!(lines, retired(stage), "{name}: {stdout}");
        }
    }
}
