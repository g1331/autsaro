use autosar_config_core::integration::{InputSource, PlanDependencies, RuntimeCatalog, build_plan};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::process::Command;

fn compile(root: &Path, project: &Path, binary: &Path, renamed: bool, maximum: u32) {
    let mut command = Command::new("gcc");
    command.args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-I"]);
    command.arg(project.join("os/include"));
    command.arg("-I").arg(project.join("include"));
    command.arg(format!("-DEXPECTED_COUNTER_MAX={maximum}u"));
    if renamed {
        command.arg("-DCOUNTER_TIME_RENAMED=1");
    }
    let output = command
        .arg(root.join("core/tests/fixtures/ecu_counter_time.c"))
        .arg("-o")
        .arg(binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn verify() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let original = super::epic4_plan::inputs();
    let scratch = super::Scratch::new();
    let mut observations = Vec::new();
    for (renamed, maximum) in [(false, 65535u32), (true, 65535u32), (false, 4095u32)] {
        let inputs = original
            .iter()
            .map(|source| {
                let mut text = String::from_utf8(source.bytes().to_vec()).unwrap();
                if maximum == 4095 && source.logical_path() == "ecuc.arxml" {
                    let parameter = text
                        .find("OsCounterMaxAllowedValue</DEFINITION-REF>")
                        .unwrap();
                    let start = parameter + text[parameter..].find("<VALUE>").unwrap() + 7;
                    let end = start + text[start..].find("</VALUE>").unwrap();
                    text.replace_range(start..end, "4095");
                }
                InputSource::new(
                    source.logical_path(),
                    if renamed {
                        text.replace("SystemCounter", "RenamedCounter")
                    } else {
                        text
                    }
                    .into_bytes(),
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let plan = build_plan(&inputs, &dependencies, &runtime).unwrap();
        assert_eq!(plan.description().schedule.counter_tick_ms, 1);
        assert_eq!(plan.description().schedule.counter_ticks_per_base, 1);
        assert_eq!(plan.description().schedule.counter_minimum_cycle, 1);
        let files = plan.ecu_integration_files().unwrap();
        let project = scratch.0.join(if maximum == 4095 {
            "alternate-maximum"
        } else if renamed {
            "renamed"
        } else {
            "default"
        });
        let preview = files.preview(&project).unwrap();
        files
            .generate_previewed(&project, &preview.revision)
            .unwrap();
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(project.join("os-generation-timing.json")).unwrap())
                .unwrap();
        let counter = if renamed {
            "RenamedCounter"
        } else {
            "SystemCounter"
        };
        assert_eq!(report["counter"]["name"], counter);
        assert_eq!(report["counter"]["tickNanoseconds"], 1_000_000);
        assert_eq!(report["counter"]["maximum"], maximum);
        assert_eq!(report["internalPeriodicTimers"], serde_json::json!([]));
        assert_eq!(report["kernelSoftwareTimers"]["enabled"], false);
        assert_eq!(report["kernelSoftwareTimers"]["configUSE_TIMERS"], 0);
        assert_eq!(report["kernelTick"]["periodicHostThread"], false);
        assert_eq!(report["kernelTick"]["logicalMillisecondsPerRequest"], 1);
        assert_eq!(
            report["hostTimeouts"][0]["rangeMilliseconds"],
            serde_json::json!([1, 5000])
        );
        assert_eq!(report["hostTimeouts"][0]["advancesAutomotiveTime"], false);
        assert_eq!(report["hostTimeouts"][1]["limitMilliseconds"], 5000);
        assert_eq!(report["hostTimeouts"][1]["advancesAutomotiveTime"], false);
        let header_path = project.join("os/include/Os_Counter.h");
        let header = fs::read_to_string(&header_path).unwrap();
        let raw_headers = root
            .join(".scratch/epic4/story418-generated-time-headers")
            .join(if maximum == 4095 {
                "SystemCounter-4095"
            } else {
                counter
            });
        for (name, bytes) in files.files() {
            if name.starts_with("os/include/") || name.starts_with("include/") {
                let destination = raw_headers.join(name);
                fs::create_dir_all(destination.parent().unwrap()).unwrap();
                fs::write(destination, bytes).unwrap();
            }
        }
        if renamed {
            assert!(!header.contains("SystemCounter"));
        }
        let binary = scratch.0.join(format!("{counter}.exe"));
        compile(root, &project, &binary, renamed, maximum);
        let output = Command::new(&binary).output().unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "counter_time PASS: 24 literal unit values; 4 single evaluations; 4 expressions"
        );
        let mut mutations = Vec::new();
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
                header.replace(
                    &format!("OSMAXALLOWEDVALUE_{counter} UINT64_C({maximum})"),
                    &format!("OSMAXALLOWEDVALUE_{counter} UINT64_C({})", maximum - 1),
                ),
                3,
            ),
            (
                "wrong-counter-base",
                header.replace(
                    &format!("OSTICKSPERBASE_{counter} UINT64_C(1)"),
                    &format!("OSTICKSPERBASE_{counter} UINT64_C(2)"),
                ),
                3,
            ),
            (
                "wrong-counter-minimum-cycle",
                header.replace(
                    &format!("OSMINCYCLE_{counter} UINT64_C(1)"),
                    &format!("OSMINCYCLE_{counter} UINT64_C(2)"),
                ),
                3,
            ),
            (
                "wrong-system-tick-duration",
                header.replace(
                    "OSTICKDURATION UINT64_C(1000000)",
                    "OSTICKDURATION UINT64_C(2000000)",
                ),
                3,
            ),
            (
                "wrong-counter-id-maximum-alias",
                header.replace(
                    &format!(
                        "OSMAXALLOWEDVALUE_OS_COUNTER_ID_{counter} OSMAXALLOWEDVALUE_{counter}"
                    ),
                    &format!("OSMAXALLOWEDVALUE_OS_COUNTER_ID_{counter} OSTICKSPERBASE_{counter}"),
                ),
                3,
            ),
        ] {
            assert_ne!(header, mutant);
            fs::write(&header_path, &mutant).unwrap();
            let mutated_binary = scratch.0.join(format!("{counter}-{name}.exe"));
            compile(root, &project, &mutated_binary, renamed, maximum);
            let result = Command::new(&mutated_binary).output().unwrap();
            assert_eq!(result.status.code(), Some(expected_exit));
            mutations.push(serde_json::json!({
                "name": name, "detected": true, "exitCode": expected_exit,
                "mutatedHeaderSha256": format!("{:x}", Sha256::digest(mutant.as_bytes())),
                "binarySha256": format!("{:x}", Sha256::digest(fs::read(mutated_binary).unwrap()))
            }));
        }
        fs::write(&header_path, &header).unwrap();
        observations.push(serde_json::json!({
            "counter": counter, "status": "pass", "literalUnitValues": 24,
            "counterMaximum": maximum,
            "singleEvaluations": 4, "expressionVectors": 4, "timerReport": report,
            "legacyCounterConstantChecks": 11,
            "headerSha256": format!("{:x}", Sha256::digest(header.as_bytes())),
            "binarySha256": format!("{:x}", Sha256::digest(fs::read(binary).unwrap())),
            "compiledMutations": mutations
        }));
    }
    for resolution in ["0.002", "0.0005", "0"] {
        let inputs = original
            .iter()
            .map(|source| {
                let text = String::from_utf8(source.bytes().to_vec()).unwrap();
                let mut changed = text.clone();
                if source.logical_path() == "ecuc.arxml" {
                    let parameter = changed.find("OsSecondsPerTick</DEFINITION-REF>").unwrap();
                    let start = parameter + changed[parameter..].find("<VALUE>").unwrap() + 7;
                    let end = start + changed[start..].find("</VALUE>").unwrap();
                    assert_eq!(&changed[start..end], "0.001");
                    changed.replace_range(start..end, resolution);
                    assert_ne!(changed, text);
                }
                InputSource::new(source.logical_path(), changed.into_bytes()).unwrap()
            })
            .collect::<Vec<_>>();
        let error = build_plan(&inputs, &dependencies, &runtime).err().unwrap();
        assert!(error.iter().any(|issue| issue.code == "COUNTER_PROFILE"));
    }
    let sources = [
        "core/src/integration/ecu.rs",
        "core/src/integration/schedule.rs",
        "core/tests/fixtures/ecu_counter_time.c",
        "core/tests/support/epic4_counter_time.rs",
        "runtime/os/include/Os_Types.h",
        "runtime/os/include/Os_Cfg.h",
        "runtime/os/FreeRTOSConfig.h",
        "runtime/os/patches/0001-controlled-host-lifecycle.patch",
    ]
    .into_iter()
    .map(|name| {
        (
            name,
            format!("{:x}", Sha256::digest(fs::read(root.join(name)).unwrap())),
        )
    })
    .collect::<std::collections::BTreeMap<_, _>>();
    let compiler = Command::new("gcc").arg("--version").output().unwrap();
    assert!(compiler.status.success());
    let evidence = serde_json::json!({
        "status": "pass for selected generated Counter time contracts; complete SC1/C221 remains open",
        "test": "epic4_generated_counter_timing_contracts",
        "compiler": String::from_utf8_lossy(&compiler.stdout).lines().next().unwrap(),
        "language": "C99", "product_sources": sources, "observations": observations,
        "rejectedResolutionsSeconds": ["0.002", "0.0005", "0"],
        "scope": "Two generated and compiled Counter names,64 independent values/evaluation/expression checks, four compiled negative oracles, internal timing reports and three profile rejections. No RTE service-port or full MISRA claim."
    });
    let evidence_path = root.join("docs/assurance/evidence/epic4/generated-counter-time-4-18.json");
    let mut bytes = serde_json::to_vec_pretty(&evidence).unwrap();
    bytes.push(b'\n');
    fs::write(evidence_path, bytes).unwrap();
}
