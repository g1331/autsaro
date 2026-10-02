use autosar_config_core::integration::{PlanDependencies, RuntimeCatalog, build_plan};
use autosar_config_core::schema;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

fn reader(root: &Path, project: &Path, probe: &Path) -> Command {
    let mut command = super::tooling::python_command();
    command
        .arg(root.join("core/tests/fixtures/artifact_reader.py"))
        .arg(project)
        .arg(probe);
    command
}

pub fn verify() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let scratch = super::Scratch::new();
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let plan = build_plan(
        &super::epic4_plan::inputs(),
        &PlanDependencies::from_repository(root),
        &runtime,
    )
    .unwrap();
    let files = plan
        .ecu_integration_files(super::tooling::native_target())
        .unwrap();
    let original = scratch.0.join("generated");
    let preview = files.preview(&original).unwrap();
    files
        .generate_previewed(&original, &preview.revision)
        .unwrap();
    let project = scratch.0.join("moved module source");
    fs::rename(original, &project).unwrap();
    let path = project.join("descriptions/Host_Implementation.arxml");
    let xml = fs::read_to_string(&path).unwrap();
    let issues = schema::validate_files(&schema::schema_archive(root), &[(&path, &xml)]).unwrap();
    assert!(issues.is_empty(), "{issues:?}");
    let consumer = scratch.0.join("consumer");
    fs::create_dir(&consumer).unwrap();
    fs::copy(
        root.join("core/tests/fixtures/ecu_control.c"),
        consumer.join("ecu_control.c"),
    )
    .unwrap();
    let probe = consumer.join("Artifact_Probe.h");
    let result = super::tooling::run_public_command(
        &mut reader(root, &project, &probe),
        &scratch.0,
        "artifact-reader",
        Duration::from_secs(30),
    );
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let good_probe = fs::read_to_string(&probe).unwrap();
    fs::write(consumer.join("control.c"), "#define main behavior_main\n#include \"ecu_control.c\"\n#undef main\n#include \"Artifact_Probe.h\"\nint main(void) { check_artifact_entries(); return behavior_main(); }\n").unwrap();
    let binary_dir = scratch.0.join("independent-build");
    super::epic4_ecu::compile(&project, &binary_dir, Some(&consumer.join("control.c")));
    let binary = super::tooling::native_binary(&binary_dir, "ecu_probe");
    let result = super::epic4_ecu::run_probe(&binary, None);
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8_lossy(&result.stdout)
            .contains("independent_control signal=2 default_session_did=2 completed=20")
    );
    let mut sections = Command::new("objdump");
    sections.arg("-h").arg(&binary);
    let output = sections.output().unwrap();
    assert!(output.status.success());
    let sections = String::from_utf8(output.stdout).unwrap();
    let (i, row) = sections
        .lines()
        .enumerate()
        .find(|(_, line)| line.split_whitespace().nth(1) == Some(".rte_code"))
        .unwrap();
    let section = row
        .split_whitespace()
        .next()
        .unwrap()
        .parse::<u32>()
        .unwrap()
        + 1;
    let flags = sections.lines().nth(i + 1).unwrap();
    assert!(flags.contains("CODE") && flags.contains("READONLY") && flags.contains("ALLOC"));
    let output = Command::new("objdump")
        .arg("-t")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(output.status.success());
    let symbols = String::from_utf8(output.stdout).unwrap();
    for name in [
        "Rte_Read_RxValue_Value",
        "Rte_Write_TxValue_Value",
        "Rte_Call_ApplicationValue_ReadData",
        "Ecu_TargetInitializeRte",
        "Com_SendSignal",
        "Com_ReceiveSignal",
        "OsService_SystemCounter_GetCounterValue",
        "Rte_Call_OsService_GetElapsedValue",
    ] {
        let found: Vec<_> = symbols
            .lines()
            .filter(|line| line.ends_with(&format!(" {name}")))
            .collect();
        assert_eq!(found.len(), 1, "{name}: {symbols}");
        assert!(
            found[0].contains(&format!("(sec {section:2})")),
            "{name}: {}",
            found[0]
        );
    }
    let mut analysis = Command::new("cppcheck");
    analysis
        .args([
            "--std=c99",
            "--platform=win64",
            "--enable=warning,style,performance,portability",
            "--error-exitcode=1",
        ])
        .arg(format!("-I{}", project.join("include").display()))
        .arg(format!("-I{}", project.join("os/include").display()))
        .arg(project.join("src/Rte.c"))
        .arg(project.join("src/Rte_OsService.c"));
    let result = super::tooling::run_public_command(
        &mut analysis,
        &scratch.0,
        "rte-static-analysis",
        Duration::from_secs(60),
    );
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    for (label, changed, message) in [
        (
            "broken-ref",
            xml.replace(
                "/HostArtifacts/Rte/HostBehavior",
                "/HostArtifacts/Absent/HostBehavior",
            ),
            "broken reference",
        ),
        (
            "missing-source",
            xml.replace(
                "<SHORT-LABEL>Rte.c</SHORT-LABEL>",
                "<SHORT-LABEL>Absent.c</SHORT-LABEL>",
            ),
            "missing source dependency",
        ),
        (
            "substituted-valid-api",
            xml.replace("Can_MainFunction_Wakeup", "Can_DeInit"),
            "actual BSW entries Can",
        ),
    ] {
        assert_ne!(changed, xml);
        fs::write(&path, changed).unwrap();
        let result = super::tooling::run_public_command(
            &mut reader(root, &project, &consumer.join("rejected.h")),
            &scratch.0,
            label,
            Duration::from_secs(30),
        );
        assert!(
            !result.status.success() && String::from_utf8_lossy(&result.stderr).contains(message)
        );
        fs::write(&path, &xml).unwrap();
    }
    // An invented BSW API still forms valid XML; the independent native consumer must reject it.
    let changed = xml.replace("CanIf_Transmit", "CanIf_Absent");
    fs::write(&path, changed).unwrap();
    let result = super::tooling::run_public_command(
        &mut reader(root, &project, &probe),
        &scratch.0,
        "invented-api-reader",
        Duration::from_secs(30),
    );
    assert!(
        !result.status.success()
            && String::from_utf8_lossy(&result.stderr).contains("actual BSW entries CanIf")
    );
    fs::write(&path, &xml).unwrap();
    fs::write(
        &probe,
        good_probe.replace("&CanIf_Transmit", "&CanIf_Absent"),
    )
    .unwrap();
    let mut compile = Command::new("gcc");
    compile
        .args(["-std=c99", "-Werror", "-fsyntax-only"])
        .arg(format!("-I{}", project.join("include").display()))
        .arg(format!("-I{}", project.join("os/include").display()))
        .arg(format!("-I{}", project.join("os").display()))
        .arg(format!("-I{}", project.join("os/src").display()))
        .arg(consumer.join("control.c"));
    let result = compile.output().unwrap();
    assert!(
        !result.status.success()
            && String::from_utf8_lossy(&result.stderr).contains("CanIf_Absent"),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    for source in [
        "#define RTE_STOP_SEC_CODE\n#include \"Rte_MemMap.h\"\n",
        "#define RTE_START_SEC_CODE\n#include \"Rte_MemMap.h\"\n#define RTE_START_SEC_CODE\n#include \"Rte_MemMap.h\"\n",
    ] {
        let path = consumer.join("bad-memmap.c");
        fs::write(&path, source).unwrap();
        let output = Command::new("gcc")
            .args(["-std=c99", "-fsyntax-only"])
            .arg(format!("-I{}", project.join("include").display()))
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            !output.status.success()
                && String::from_utf8_lossy(&output.stderr).contains("RTE code section")
        );
    }
}
