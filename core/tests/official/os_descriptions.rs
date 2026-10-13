//! Generated OS descriptions validated against the official schema and MOD.
use super::*;
use autosar_config_core::target::BuildTarget;
use std::path::Path;

/// Os_Arti.arxml and Os_Service.arxml validate against the official R24-11
/// schema, and the independent ARTI reader resolves every ARTI reference
/// against the official ECUC MOD definitions and the delivered inputs.
#[cfg(feature = "official-oracles")]
#[cfg(feature = "official-oracles")]
#[test]
fn official_descriptions() {
    use autosar_config_core::schema;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let archive = schema::schema_archive(root);
    let python = std::env::var_os("AUTOSAR_PYTHON")
        .expect("Set AUTOSAR_PYTHON to the absolute Python interpreter");
    let scratch = Scratch::new();
    let reader = |project: &Path, os: &str, header: &Path| {
        std::process::Command::new(&python)
            .arg(fixture("arti_reader.py"))
            .arg(project)
            .arg(root)
            .arg(os)
            .arg(header)
            .output()
            .unwrap()
    };
    for (label, os, sources) in [
        ("alarms", "Os", single_component_sources()),
        ("renamed", "OsAlt", renamed_os_variant()),
        ("tables", "Os", schedule_table_variant()),
    ] {
        let directory = scratch.0.join(label);
        std::fs::create_dir(&directory).unwrap();
        let (project, _) = project(&sources, &directory, BuildTarget::LinuxX64ControlledV1);
        for name in ["os/Os_Arti.arxml", "os/Os_Service.arxml"] {
            let path = project.join(name);
            let xml = std::fs::read_to_string(&path).unwrap();
            let issues = schema::validate_files(&archive, &[(&path, &xml)]).unwrap();
            assert!(issues.is_empty(), "{label} {name}: {issues:?}");
        }
        let header = directory.join("Arti_ExpressionProbe.h");
        let output = reader(&project, os, &header);
        assert!(
            output.status.success(),
            "{label}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(header.is_file());
        if label != "alarms" {
            continue;
        }
        let path = project.join("os/Os_Arti.arxml");
        let xml = std::fs::read_to_string(&path).unwrap();
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
            assert_ne!(changed, xml, "{fault}");
            std::fs::write(&path, changed).unwrap();
            let output = reader(&project, os, &directory.join("rejected.h"));
            assert!(
                !output.status.success() && String::from_utf8_lossy(&output.stderr).contains(fault),
                "{fault}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::fs::write(&path, &xml).unwrap();
        let binding_path = project.join("os/src/Os_Arti.c");
        let binding = std::fs::read_to_string(&binding_path).unwrap();
        let changed = binding.replace("0u, OsTask_Start,", "0u, OsTask_Absent,");
        assert_ne!(binding, changed);
        std::fs::write(&binding_path, changed).unwrap();
        let output = reader(&project, os, &directory.join("rejected.h"));
        assert!(
            !output.status.success()
                && String::from_utf8_lossy(&output.stderr).contains("missing literal hook"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
