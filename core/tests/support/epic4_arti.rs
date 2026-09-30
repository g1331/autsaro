use autosar_config_core::integration::{InputSource, PlanDependencies, RuntimeCatalog, build_plan};
use autosar_config_core::schema;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

pub fn verify() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let scratch = super::Scratch::new();
    for (label, os, scheduler, tables) in [
        ("alarms", "Os", false, false),
        ("renamed", "OsAlt", true, false),
        ("tables", "Os", false, true),
    ] {
        let inputs = super::epic4_plan::inputs()
            .into_iter()
            .map(|source| {
                let bytes = if tables && source.logical_path() == "ecuc.arxml" {
                    fs::read(root.join("core/tests/fixtures/epic4_timing/schedule_tables.arxml"))
                        .unwrap()
                } else {
                    source.bytes().to_vec()
                };
                let mut xml = String::from_utf8(bytes).unwrap();
                if os != "Os" {
                    xml = xml
                        .replace(
                            "<SHORT-NAME>Os</SHORT-NAME>",
                            &format!("<SHORT-NAME>{os}</SHORT-NAME>"),
                        )
                        .replace("/Configuration/Os/", &format!("/Configuration/{os}/"));
                }
                if scheduler {
                    xml = xml.replace(
                        "OsUseResScheduler</DEFINITION-REF><VALUE>false</VALUE>",
                        "OsUseResScheduler</DEFINITION-REF><VALUE>true</VALUE>",
                    );
                }
                InputSource::new(source.logical_path(), xml.into_bytes()).unwrap()
            })
            .collect::<Vec<_>>();
        let plan = build_plan(&inputs, &dependencies, &runtime)
            .unwrap_or_else(|error| panic!("{error:?}"));
        let files = plan
            .ecu_integration_files()
            .unwrap_or_else(|error| panic!("{error:?}"));
        let original = scratch.0.join(format!("source-{label}"));
        let preview = files.preview(&original).unwrap();
        files
            .generate_previewed(&original, &preview.revision)
            .unwrap();
        let project = scratch.0.join(format!("moved ARTI {label}"));
        fs::rename(&original, &project).unwrap();
        let path = project.join("os/Os_Arti.arxml");
        let xml = fs::read_to_string(&path).unwrap();
        let issues =
            schema::validate_files(&schema::schema_archive(root), &[(&path, &xml)]).unwrap();
        assert!(issues.is_empty(), "{issues:?}");
        let consumer_dir = scratch.0.join(format!("consumer-{label}"));
        fs::create_dir(&consumer_dir).unwrap();
        for name in ["ecu_arti_control.c", "ecu_control.c"] {
            fs::copy(
                root.join("core/tests/fixtures").join(name),
                consumer_dir.join(name),
            )
            .unwrap();
        }
        let mut reader = Command::new("python");
        reader
            .arg(root.join("core/tests/fixtures/arti_reader.py"))
            .arg(&project)
            .arg(root)
            .arg(os)
            .arg(consumer_dir.join("Arti_ExpressionProbe.h"));
        let result = super::epic4_ecu::run_public_command(
            &mut reader,
            &scratch.0,
            &format!("reader-{label}"),
            Duration::from_secs(30),
        );
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        if label == "alarms" {
            for (fault, changed) in [
                (
                    "broken reference",
                    xml.replace("/Configuration/Os/Task_Ecu", "/Configuration/Os/AbsentTask"),
                ),
                (
                    "missing/extra hook",
                    xml.replace("OsTask_Start", "OsTask_Unknown"),
                ),
            ] {
                assert_ne!(changed, xml);
                fs::write(&path, changed).unwrap();
                let mut reject = Command::new("python");
                reject
                    .arg(root.join("core/tests/fixtures/arti_reader.py"))
                    .arg(&project)
                    .arg(root)
                    .arg(os)
                    .arg(consumer_dir.join("rejected.h"));
                let result = super::epic4_ecu::run_public_command(
                    &mut reject,
                    &scratch.0,
                    &format!("reject-{}", fault.replace(['/', ' '], "-")),
                    Duration::from_secs(30),
                );
                assert!(
                    !result.status.success()
                        && String::from_utf8_lossy(&result.stderr).contains(fault),
                    "{}{}",
                    String::from_utf8_lossy(&result.stdout),
                    String::from_utf8_lossy(&result.stderr)
                );
            }
            fs::write(&path, &xml).unwrap();
            let binding_path = project.join("os/src/Os_Arti.c");
            let binding = fs::read_to_string(&binding_path).unwrap();
            let changed = binding.replace("0u, OsTask_Start,", "0u, OsTask_Absent,");
            assert_ne!(binding, changed);
            fs::write(&binding_path, changed).unwrap();
            let mut reject = Command::new("python");
            reject
                .arg(root.join("core/tests/fixtures/arti_reader.py"))
                .arg(&project)
                .arg(root)
                .arg(os)
                .arg(consumer_dir.join("rejected.h"));
            let result = super::epic4_ecu::run_public_command(
                &mut reject,
                &scratch.0,
                "reject-missing-binding",
                Duration::from_secs(30),
            );
            assert!(
                !result.status.success()
                    && String::from_utf8_lossy(&result.stderr).contains("missing literal hook")
            );
            fs::write(&binding_path, binding).unwrap();
        }
        let output = scratch.0.join(format!("build-{label}"));
        let mut compile = Command::new("powershell.exe");
        compile
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(project.join("build.ps1"))
            .arg("-OutputDirectory")
            .arg(&output)
            .arg("-TestMode")
            .arg("-ControlSource")
            .arg(consumer_dir.join("ecu_arti_control.c"));
        let result = super::epic4_ecu::run_public_command(
            &mut compile,
            &scratch.0,
            &format!("compile-{label}"),
            Duration::from_secs(180),
        );
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        let mut run = Command::new(output.join("ecu_probe.exe"));
        run.arg(os);
        let result = super::epic4_ecu::run_public_command(
            &mut run,
            &scratch.0,
            &format!("run-{label}"),
            Duration::from_secs(15),
        );
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stdout).contains(&format!(
            "arti_consumer os={os} observations=2 getters=2 errors=2 internal_services=0 dropped=0"
        )));
    }
    let mut native = Command::new("python");
    native.args(["-c", "import sys; from pathlib import Path; sys.path.insert(0, str(Path(sys.argv[1]) / 'scripts')); import epic4_os; epic4_os.check_arti_native(Path(sys.argv[2]))"])
        .arg(root).arg(scratch.0.join("native"));
    let result = super::epic4_ecu::run_public_command(
        &mut native,
        &scratch.0,
        "native-arti",
        Duration::from_secs(180),
    );
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
