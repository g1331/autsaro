use autosar_config_core::integration::{PlanDependencies, RuntimeCatalog, build_plan};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

pub fn application_loop() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let plan = build_plan(&super::epic4_plan::inputs(), &dependencies, &runtime).unwrap();
    let project = plan.ecu_integration_files().unwrap();
    let scratch = super::Scratch::new();
    let source = scratch.0.join("application-source");
    let preview = project.preview(&source).unwrap();
    project
        .generate_previewed(&source, &preview.revision)
        .unwrap();
    let build = scratch.0.join("application-build");
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(source.join("build.ps1"))
        .arg("-OutputDirectory")
        .arg(&build)
        .arg("-ControlSource")
        .arg(root.join("core/tests/fixtures/application_loop.c"))
        .arg("-TestMode")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for mode in ["initial", "stale", "deadline", "write_fail", "extended"] {
        let stdout = scratch.0.join(format!("{mode}.stdout"));
        let stderr = scratch.0.join(format!("{mode}.stderr"));
        let mut child = Command::new(build.join("ecu_probe.exe"))
            .arg(mode)
            .stdout(std::fs::File::create(&stdout).unwrap())
            .stderr(std::fs::File::create(&stderr).unwrap())
            .spawn()
            .unwrap();
        let started = Instant::now();
        while child.try_wait().unwrap().is_none() {
            if started.elapsed() > Duration::from_secs(8) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("application {mode} exceeded host watchdog");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let status = child.wait().unwrap();
        let text = std::fs::read_to_string(&stdout).unwrap();
        let errors = std::fs::read_to_string(&stderr).unwrap();
        assert!(status.success(), "{mode}: {text}{errors}");
        assert!(text.contains("status_epoch_value_can_did=pass"), "{text}");
        match mode {
            "initial" => assert!(
                text.contains("read epoch=10 value=00000000 status=133"),
                "{text}"
            ),
            "stale" => {
                assert!(
                    text.contains("read epoch=30 value=12345678 status=64"),
                    "{text}"
                );
                assert!(
                    text.contains("snapshot at=30 value=00000000 committed=30 read=64 write=0"),
                    "{text}"
                );
            }
            "deadline" => assert!(
                text.contains("snapshot at=30 value=0a0b0c0d committed=30 read=0 write=0"),
                "{text}"
            ),
            "write_fail" => {
                assert!(
                    text.contains("snapshot at=20 value=12345678 committed=10 read=128 write=128"),
                    "{text}"
                );
                assert!(
                    text.contains("snapshot at=30 value=0a0b0c0d committed=30 read=0 write=0"),
                    "{text}"
                );
            }
            "extended" => assert!(text.contains("data=065003003201f400"), "{text}"),
            _ => unreachable!(),
        }
    }
}
