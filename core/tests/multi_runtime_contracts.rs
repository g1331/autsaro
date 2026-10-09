//! Independently authored consumers of the separate standard multi runtime.
#![cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]

#[allow(dead_code)]
#[path = "support/tooling.rs"]
mod tooling;
#[allow(dead_code)]
#[path = "support/workspace.rs"]
mod workspace;

use std::path::{Path, PathBuf};
use std::process::Command;
use workspace::Scratch;

fn det_consumer(scratch: &Path, fixture: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    autosar_config_core::resources::AssetInventory::from_directory(root).unwrap();
    let executable = tooling::native_binary(scratch, "det-consumer");
    let settings = tooling::execution_settings();
    let mut command = Command::new(settings.compiler);
    if cfg!(target_os = "linux") {
        command.arg("-pthread");
    }
    let output = command
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-I")
        .arg(root.join("runtime/multi/include"))
        .arg("-I")
        .arg(root.join("runtime/multi/src"))
        .arg(root.join("runtime/multi/src/Det.c"))
        .arg(root.join("runtime/multi/src/Det_Host.c"))
        .arg(root.join("core/tests/fixtures/multi-runtime").join(fixture))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

#[test]
fn det_standard_consumer_executes_runtime_and_halt_contracts() {
    let scratch = Scratch::new();
    let executable = det_consumer(&scratch.0, "det_contract.c");
    for (mode, expected, halted) in [
        ("runtime", "runtime-ok\n", false),
        ("before-init", "", true),
        (
            "development",
            "development-first\ndevelopment-second\n",
            true,
        ),
        ("recursive-development", "development-first\n", true),
        ("invalid-config", "", true),
    ] {
        let output = Command::new(&executable)
            .arg(mode)
            .current_dir(&scratch.0)
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected, "{mode}");
        assert!(output.stderr.is_empty(), "{mode}: {:?}", output);
        if halted {
            // A failed assertion or ordinary rejection cannot satisfy halt.
            #[cfg(target_os = "linux")]
            {
                use std::os::unix::process::ExitStatusExt;
                assert_eq!(output.status.signal(), Some(6), "{mode}: {:?}", output);
            }
            #[cfg(windows)]
            assert_eq!(output.status.code(), Some(86), "{mode}: {:?}", output);
        } else {
            assert!(output.status.success(), "{mode}: {:?}", output);
        }
    }
}

#[test]
fn det_reports_concurrently_without_changing_callout_results() {
    let scratch = Scratch::new();
    let executable = det_consumer(&scratch.0, "det_parallel.c");
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{:?}",
        output
    );
}

#[test]
fn com_standard_consumer_executes_group_and_reception_monitoring() {
    let scratch = Scratch::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let executable = tooling::native_binary(&scratch.0, "com-consumer");
    let output = Command::new(tooling::execution_settings().compiler)
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-I")
        .arg(root.join("runtime/multi/include"))
        .arg("-I")
        .arg(root.join("runtime/multi/src"))
        .arg("-I")
        .arg(root.join("core/tests/fixtures/multi-runtime"))
        .arg(root.join("runtime/multi/src/Com.c"))
        .arg(root.join("core/tests/fixtures/multi-runtime/com_contract.c"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{:?}",
        output
    );
}

fn chain_consumer(scratch: &Path, fixture: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let inventory = autosar_config_core::resources::AssetInventory::from_directory(root).unwrap();
    // Materialize normal flat delivery headers, preserving the reused CAN assets' raw bytes.
    let include = scratch.join("include");
    std::fs::create_dir(&include).unwrap();
    for name in ["Can.h", "Can_GeneralTypes.h"] {
        let asset = inventory.get(&format!("runtime/include/{name}")).unwrap();
        std::fs::write(include.join(name), asset.bytes).unwrap();
        assert_eq!(
            std::fs::read(include.join(name)).unwrap(),
            std::fs::read(root.join(asset.relative_path)).unwrap()
        );
    }
    for entry in std::fs::read_dir(root.join("runtime/multi/include")).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), include.join(entry.file_name())).unwrap();
    }
    for name in [
        "ComStack_Cfg.h",
        "SchM_CanIf.h",
        "SchM_ComM.h",
        "SchM_CanTp.h",
    ] {
        std::fs::copy(
            root.join("core/tests/fixtures/multi-runtime").join(name),
            include.join(name),
        )
        .unwrap();
    }
    if fixture == "can_polling_contract.c" {
        std::fs::copy(
            root.join("core/tests/fixtures/multi-runtime/Ecu_ProfileLimits_Polling.h"),
            include.join("Ecu_ProfileLimits.h"),
        )
        .unwrap();
    }
    let executable = tooling::native_binary(scratch, "com-chain-consumer");
    let mut command = Command::new(tooling::execution_settings().compiler);
    if cfg!(target_os = "linux") {
        command.arg("-pthread");
    }
    if fixture == "mode_contract.c" {
        // Independent non-equal namespace oracle; final generation derives source IDs.
        command.args([
            "-DCAN_CONTROLLER_ID=7u",
            "-DCAN_CANIF_CONTROLLER_ID=9u",
            "-DCAN_TX_HOH=17u",
        ]);
    }
    command
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-I")
        .arg(include);
    for directory in [
        "runtime/multi/src",
        "runtime/include",
        "runtime/src",
        "runtime/host/include",
    ] {
        command.arg("-I").arg(root.join(directory));
    }
    for source in [
        "runtime/multi/src/Com.c",
        "runtime/multi/src/PduR.c",
        "runtime/multi/src/LSduR.c",
        "runtime/multi/src/CanIf.c",
        "runtime/multi/src/SchM_CanIf_Host.c",
        "runtime/multi/src/ComM.c",
        "runtime/multi/src/BswM.c",
        "runtime/multi/src/Ecu_HostBusSM.c",
        "runtime/multi/src/SchM_Mode_Host.c",
        "runtime/multi/src/Dcm.c",
        "runtime/multi/src/CanTp.c",
        "runtime/multi/src/SchM_Diagnostic_Host.c",
        "runtime/multi/src/Det.c",
        "runtime/multi/src/Det_Host.c",
        "runtime/src/Can.c",
        "runtime/src/Can_HostLock.c",
        "runtime/host/src/Can_Execution.c",
    ] {
        command.arg(root.join(source));
    }
    command.arg(root.join("core/tests/fixtures/multi-runtime").join(fixture));
    let output = command.arg("-o").arg(&executable).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

#[test]
fn com_periodic_transmission_executes_real_lower_modules_and_can_busy() {
    let scratch = Scratch::new();
    let executable = chain_consumer(&scratch.0, "com_chain_contract.c");
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{:?}",
        output
    );
}

#[test]
fn canif_different_pdus_and_actual_confirmation_stop_interleave_safely() {
    let scratch = Scratch::new();
    let executable = chain_consumer(&scratch.0, "canif_parallel.c");
    let logs = scratch.0.join("execution");
    std::fs::create_dir(&logs).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let spec = autosar_config_core::execution::ProcessSpec::for_duration(
        vec![executable.into_os_string()],
        scratch.0.clone(),
        vec![],
        std::time::Duration::from_secs(30),
        logs,
    )
    .unwrap();
    let result = tooling::execution_owner().run(spec).unwrap();
    assert!(result.success(), "{result:?}");
    assert!(
        std::fs::read(&result.stdout).unwrap().is_empty(),
        "{result:?}"
    );
    assert!(
        std::fs::read(&result.stderr).unwrap().is_empty(),
        "{result:?}"
    );
}

#[test]
fn comm_cdd_mode_demand_and_immediate_bswm_use_actual_lower_modes() {
    let scratch = Scratch::new();
    let executable = chain_consumer(&scratch.0, "mode_contract.c");
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{:?}",
        output
    );
}

#[test]
fn diagnostic_transport_owns_buffers_and_obeys_real_mode_and_confirmation() {
    let scratch = Scratch::new();
    let executable = chain_consumer(&scratch.0, "diagnostic_contract.c");
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{:?}",
        output
    );
}

#[test]
fn configured_can_polling_and_zero_dlc_preserve_real_admission_and_callbacks() {
    let scratch = Scratch::new();
    let executable = chain_consumer(&scratch.0, "can_polling_contract.c");
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{:?}",
        output
    );
}
