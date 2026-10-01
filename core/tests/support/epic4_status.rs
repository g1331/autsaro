use autosar_config_core::integration::{InputSource, PlanDependencies, RuntimeCatalog, build_plan};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn verify_generated_standard() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let original = super::epic4_plan::inputs();
    let replaced = |status: &str| {
        original
            .iter()
            .map(|source| {
                let mut text = String::from_utf8(source.bytes().to_vec()).unwrap();
                if source.logical_path() == "ecuc.arxml" {
                    let marker = "OsStatus</DEFINITION-REF>";
                    let parameter = text.find(marker).unwrap();
                    let start = parameter + text[parameter..].find("<VALUE>").unwrap() + 7;
                    let end = start + text[start..].find("</VALUE>").unwrap();
                    text.replace_range(start..end, status);
                }
                InputSource::new(source.logical_path(), text.into_bytes()).unwrap()
            })
            .collect::<Vec<_>>()
    };
    for status in ["STANDARD", "EXTENDED"] {
        let inputs = replaced(status);
        let plan = build_plan(&inputs, &dependencies, &runtime).unwrap();
        let files = plan
            .ecu_integration_files(super::tooling::native_target())
            .unwrap();
        let header = std::str::from_utf8(
            &files
                .files()
                .iter()
                .find(|(name, _)| name == "os/include/Os_Cfg.h")
                .unwrap()
                .1,
        )
        .unwrap();
        assert!(header.contains(if status == "STANDARD" {
            "#define OS_STATUS_EXTENDED 0"
        } else {
            "#define OS_STATUS_EXTENDED 1"
        }));
        if status == "STANDARD" {
            let scratch = super::Scratch::new();
            let project = scratch.0.join("original");
            let preview = files.preview(&project).unwrap();
            files
                .generate_previewed(&project, &preview.revision)
                .unwrap();
            let moved = scratch.0.join("moved standard target");
            fs::rename(&project, &moved).unwrap();
            let conflict = Command::new("gcc")
                .args(["-std=c99", "-E", "-x", "c", "-DOS_STATUS_EXTENDED=1", "-I"])
                .arg(moved.join("os/include"))
                .arg("-I")
                .arg(moved.join("include"))
                .arg(root.join("core/tests/fixtures/ecu_standard_control.c"))
                .output()
                .unwrap();
            assert!(!conflict.status.success());
            assert!(
                String::from_utf8_lossy(&conflict.stderr)
                    .contains("OsStatus differs from the validated plan")
            );
            let output = scratch.0.join("build");
            super::epic4_ecu::compile(
                &moved,
                &output,
                Some(&root.join("core/tests/fixtures/ecu_standard_control.c")),
            );
            let binary = super::tooling::native_binary(&output, "ecu_probe");
            let result = super::epic4_ecu::run_probe(&binary, None);
            assert!(
                result.status.success(),
                "{}{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }
    for status in ["UNKNOWN", "standard"] {
        let issues = match build_plan(&replaced(status), &dependencies, &runtime) {
            Ok(_) => panic!("Unsupported OsStatus accepted: {status}"),
            Err(issues) => issues,
        };
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "TARGET_PARAMETER_UNSUPPORTED"),
            "{issues:?}"
        );
    }
}
