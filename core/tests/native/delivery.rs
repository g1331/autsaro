//! Built-in template deliveries built and verified with the target toolchain.
use super::*;
use crate::support::generated_packages::*;
use autosar_config_core::prepared::prepare_ecu_project_for_workspace;
use std::fs;
use std::path::Path;

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn copied_single_component_package_builds_offline_without_changing_source_bytes() {
    let scratch = Scratch::new();
    let ecu = standard_with_application(&scratch.0.join("ECU"));
    let original = scratch.0.join("original");
    prepare(&ecu, true).generate(&original).unwrap();
    let received = scratch.0.join("recipient/package");
    packages::copy_package(&original, &received);
    fs::remove_dir_all(original).unwrap();
    drop(ecu);
    fs::remove_dir_all(scratch.0.join("ECU")).unwrap();
    let before = packages::package_tree(&received);
    let output = received.parent().unwrap().join("build");
    let result = tooling::ecu_build_command(&received, &output, "host-batch", None)
        .current_dir(&received)
        .env_remove("PYTHONPATH")
        .env_remove("PYTHONHOME")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(tooling::native_binary(&output, "ecu_host_batch").is_file());
    assert_eq!(packages::package_tree(&received), before);
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn builtin_standard_template_generates_builds_and_passes_reference_verifier() {
    use autosar_config_core::integration::{build_ecu_project, verify_ecu_project};
    // The built-in template with its create-only seed is the reference echo
    // application that the delivered ecu-tool verifier checks.
    let scratch = Scratch::new();
    let ecu = standard_with_application(&scratch.0.join("ECU"));
    let plan = ecu.saved_integration_plan().unwrap();
    assert_eq!(
        plan.description().profile,
        autosar_config_core::integration::MULTI_PROFILE
    );
    let target = tooling::native_target();
    let project = scratch.0.join("package");
    prepare_ecu_project_for_workspace(&ecu, &plan, target, true)
        .unwrap()
        .generate(&project)
        .unwrap();
    let files = prepare_ecu_project_for_workspace(&ecu, &plan, target, true)
        .unwrap()
        .into_files();
    let settings = tooling::execution_settings();
    let owner = tooling::execution_owner();
    let output = scratch.0.join("build");
    fs::create_dir(&output).unwrap();
    let built = build_ecu_project(&files, &project, &output, &settings, &owner)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(Path::new(&built.binary_path).is_file(), "{}", built.log);
    let report = verify_ecu_project(&files, &project, &settings, &owner)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(report.passed, "{}", report.log);
    let log = report.log.to_string();
    for marker in [
        "protocol=echo PASS",
        "protocol=ncr-recovery PASS",
        "protocol=malformed PASS",
        "ECU_HANDOFF_VERIFY PASS",
    ] {
        assert!(log.contains(marker), "{marker}: {log}");
    }
}
