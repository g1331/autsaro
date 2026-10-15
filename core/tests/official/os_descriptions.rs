//! Generated OS descriptions validated against the official schema and MOD.
use super::*;
use autosar_config_core::target::BuildTarget;
use std::path::Path;

/// Os_Arti.arxml and Os_Service.arxml validate against the official R24-11
/// schema, and the independent ARTI reader resolves every ARTI reference
/// against the official ECUC MOD definitions and the delivered inputs.
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
    // Every emitted ArtiHardware/ArtiOs container, parameter and reference of the
    // four variants, checked against the official MOD multiplicities and targets;
    // each verified definition is emitted by at least one variant.
    // Verifies: ECUC_Arti_00055, 00056, 00057, 00058, 00059, 00060, 00061, 00062, 00063, 00068,
    // 00069, 00071, 00072, 00073, 00074, 00075, 00079, 00080, 00081, 00086, 00087, 00088,
    // 00089, 00090, 00091, 00092, 00093, 00096, 00099, 00100, 00101, 00102, 00103, 00104,
    // 00105, 00106, 00107, 00108, 00109, 00111, 00113, 00114, 00115, 00117, 00118, 00119,
    // 00120, 00122, 00124, 00125, 00136, 00137, 00139, 00140, 00141, 00144, 00145, 00146,
    // 00147, 00148, 00150, 00151, 00154, 00155, 00156, 00157, 00174, 00182, 00184, 00185,
    // 00187, 00188, 00189, 00190, 00192, 00193
    let mut emitted = std::collections::BTreeSet::new();
    for (label, os, sources) in [
        ("alarms", "Os", single_component_sources()),
        ("renamed", "OsAlt", renamed_os_variant()),
        ("tables", "Os", schedule_table_variant()),
        ("scheduler", "Os", scheduler_variant("true", false)),
    ] {
        let directory = scratch.0.join(label);
        std::fs::create_dir(&directory).unwrap();
        let (project, _) = project(&sources, &directory, BuildTarget::LinuxX64ControlledV1);
        for name in ["Os_Arti.arxml", "Os_Service.arxml"] {
            let path = delivered(&project, name);
            let xml = std::fs::read_to_string(&path).unwrap();
            let issues = schema::validate_files(&archive, &[(&path, &xml)]).unwrap();
            assert!(issues.is_empty(), "{label} {name}: {issues:?}");
        }
        let arti = std::fs::read_to_string(delivered(&project, "Os_Arti.arxml")).unwrap();
        emitted.extend(arti.split("</DEFINITION-REF>").filter_map(|part| {
            let path = part.rsplit('>').next()?;
            path.strip_prefix("/AUTOSAR/EcucDefs/Arti/")
                .map(|path| path.rsplit('/').next().unwrap().to_owned())
        }));
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
        let path = delivered(&project, "Os_Arti.arxml");
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
        let binding_path = delivered(&project, "Os_Arti.c");
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
    let missing: Vec<_> = [
        "ArtiHardwareCoreInstanceCurrentApplicationRef",
        "ArtiHardwareCoreClassCurrentIsrRef",
        "ArtiHardwareCoreInstanceCurrentIsrRef",
        "ArtiHardwareCoreClassCurrentTaskRef",
        "ArtiHardwareCoreInstanceCurrentTaskRef",
        "ArtiHardwareCoreInstanceEcucCoreRef",
        "ArtiHardware",
        "ArtiHardwareCoreClass",
        "ArtiHardwareCoreInstance",
        "ArtiOsTaskClassCurrentTaskStateRef",
        "ArtiOsTaskInstanceCurrentTaskStateRef",
        "ArtiOs",
        "ArtiOsClassAppModeRef",
        "ArtiOsInstanceAppModeRef",
        "ArtiOsClass",
        "ArtiOsInstanceEcucRef",
        "ArtiOsInstanceHookRef",
        "ArtiOsInstance",
        "ArtiOsIsrClass",
        "ArtiOsIsrInstance",
        "ArtiOsTaskClass",
        "ArtiOsTaskInstanceEcucRef",
        "ArtiOsTaskInstanceFunction",
        "ArtiOsTaskInstance",
        "ArtiHardwareCoreInstanceCoreId",
        "ArtiOsTaskInstanceId",
        "ArtiOsIsrInstanceId",
        "ArtiHardwareCoreInstanceValidRef",
        "ArtiOsInstanceValidRef",
        "ArtiOsTaskClassContextRef",
        "ArtiOsTaskClassPriorityRef",
        "ArtiOsTaskClassStackRef",
        "ArtiOsTaskInstanceValidRef",
        "ArtiOsTaskInstanceContextRef",
        "ArtiOsTaskInstanceCurrentActivationsRef",
        "ArtiOsTaskInstancePriorityRef",
        "ArtiOsTaskInstanceStackRef",
        "ArtiOsAlarmClass",
        "ArtiOsAlarmInstance",
        "ArtiOsAlarmClassStateRef",
        "ArtiOsAlarmInstanceCounter",
        "ArtiOsAlarmInstanceCycleTimeRef",
        "ArtiOsAlarmInstanceEcuCRef",
        "ArtiOsAlarmInstanceStateRef",
        "ArtiOsAlarmInstanceValidRef",
        "ArtiOsContextClass",
        "ArtiOsContextInstance",
        "ArtiOsContextInstanceAddressRef",
        "ArtiOsContextInstanceSizeRef",
        "ArtiOsContextInstanceValidRef",
        "ArtiOsResourceClass",
        "ArtiOsResourceInstance",
        "ArtiOsResourceClassLockerRef",
        "ArtiOsResourceClassStateRef",
        "ArtiOsResourceInstancePriority",
        "ArtiOsResourceInstanceStateRef",
        "ArtiOsResourceInstanceLockerRef",
        "ArtiOsResourceInstanceValidRef",
        "ArtiOsStackClass",
        "ArtiOsStackInstance",
        "ArtiOsStackInstanceDirection",
        "ArtiOsStackInstanceBaseAddressRef",
        "ArtiOsStackInstanceSizeRef",
        "ArtiOsStackInstanceValidRef",
        "ArtiOsAlarmInstanceAlarmTimeRef",
        "ArtiOsIsrInstanceValidRef",
        "ArtiOsIsrInstanceCategory",
        "ArtiOsScheduleTableClass",
        "ArtiOsScheduleTableClassCurrentStateRef",
        "ArtiOsScheduleTableInstance",
        "ArtiOsScheduleTableInstanceCounterValueRef",
        "ArtiOsScheduleTableInstanceCurrentStateRef",
        "ArtiOsScheduleTableInstanceEcucRef",
        "ArtiOsScheduleTableInstanceExpiryTimeRef",
        "ArtiOsScheduleTableInstanceNextExpiryPointRef",
        "ArtiOsScheduleTableInstanceNextScheduleTableRef",
    ]
    .into_iter()
    .filter(|name| !emitted.contains(*name))
    .collect();
    assert!(
        missing.is_empty(),
        "verified ARTI definitions not emitted: {missing:?}"
    );
}

/// The single file named `name` in the delivered project's file list, so the
/// test follows the delivery layout instead of hardcoding its directories.
#[cfg(feature = "official-oracles")]
fn delivered(project: &Path, name: &str) -> std::path::PathBuf {
    let list = std::fs::read_to_string(project.join("meta/files.list")).unwrap();
    let matches: Vec<_> = list
        .lines()
        .filter(|line| line.rsplit('/').next() == Some(name))
        .collect();
    assert_eq!(matches.len(), 1, "{name} in meta/files.list: {matches:?}");
    project.join(matches[0])
}
