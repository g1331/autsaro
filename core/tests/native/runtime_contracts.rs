//! Independently authored consumers of the standard runtime modules.
use super::*;
use std::path::{Path, PathBuf};
use std::process::Command;

fn det_consumer(scratch: &Path, fixture: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    autosar_config_core::resources::AssetInventory::from_directory(root).unwrap();
    let executable = tooling::native_binary(scratch, "det-consumer");
    let mut command = tooling::c_compiler();
    if cfg!(target_os = "linux") {
        command.arg("-pthread");
    }
    let output = command
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-I")
        .arg(root.join("runtime/bsw/include"))
        .arg("-I")
        .arg(root.join("runtime/host/include"))
        .arg("-I")
        .arg(root.join("runtime/bsw/src"))
        .arg("-I")
        .arg(root.join("runtime/host/src"))
        .arg("-I")
        .arg(root.join("core/tests/fixtures/bsw-contracts"))
        .arg(root.join("runtime/host/src/SchM_CanIf_Host.c"))
        .arg(root.join("runtime/host/src/Can_HostLock.c"))
        .arg(root.join("runtime/bsw/src/Det.c"))
        .arg(root.join("runtime/host/src/Det_Host.c"))
        .arg(root.join("core/tests/fixtures/bsw-contracts").join(fixture))
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
        // Windows text-mode stdout writes CRLF; the contract is the line content.
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"),
            expected,
            "{mode}"
        );
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

/// Builds a Com consumer with the delivered host areas; `flags` add linker wraps.
fn com_consumer(scratch: &Path, fixture: &str, flags: &[&str]) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let executable = tooling::native_binary(scratch, "com-consumer");
    let mut command = tooling::c_compiler();
    if cfg!(target_os = "linux") {
        command.arg("-pthread");
    }
    let output = command
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .args(flags)
        .arg("-I")
        .arg(root.join("runtime/bsw/include"))
        .arg("-I")
        .arg(root.join("runtime/host/include"))
        .arg("-I")
        .arg(root.join("runtime/bsw/src"))
        .arg("-I")
        .arg(root.join("runtime/host/src"))
        .arg("-I")
        .arg(root.join("core/tests/fixtures/bsw-contracts"))
        .arg(root.join("runtime/host/src/SchM_CanIf_Host.c"))
        .arg(root.join("runtime/host/src/Can_HostLock.c"))
        .arg(root.join("runtime/bsw/src/Com.c"))
        .arg(root.join("core/tests/fixtures/bsw-contracts").join(fixture))
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
fn com_standard_consumer_executes_group_and_reception_monitoring() {
    let scratch = Scratch::new();
    let executable = com_consumer(&scratch.0, "com_contract.c", &[]);
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{:?}",
        output
    );
}

/// Com leaves COM_STATE before PduR and its users, and publishes in order (ADR 0018).
#[test]
fn com_notifications_and_transmissions_complete_once_while_another_waits_outside_the_area() {
    let scratch = Scratch::new();
    let executable = com_consumer(
        &scratch.0,
        "com_parallel.c",
        &["-Wl,--wrap=SchM_Exit_Com_COM_STATE"],
    );
    let logs = scratch.0.join("execution");
    std::fs::create_dir(&logs).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let result = run_owned(executable, &scratch.0, logs);
    assert!(result.success(), "{result:?}");
}

/// Headers a delivered mainline ECU generates for its BSW (`Ecu_Config.h`,
/// `Ecu_Execution.h`), taken from the built-in template's actual preparation.
fn mainline_generated_headers(scratch: &Path) -> Vec<(String, Vec<u8>)> {
    use autosar_config_core::Workspace;
    let live = scratch.join("mainline");
    let creation = Workspace::preview_project_creation(&live, "Headers", "standard-ecu").unwrap();
    let mut workspace = Workspace::create_project_previewed(&creation).unwrap();
    let initialization = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&initialization)
        .unwrap();
    let plan = workspace.saved_integration_plan().unwrap();
    let prepared = autosar_config_core::prepared::prepare_ecu_project_for_workspace(
        &workspace,
        &plan,
        tooling::native_target(),
        false,
    )
    .unwrap();
    [
        "generated/include/Ecu_Config.h",
        "generated/src/Ecu_Execution.h",
    ]
    .into_iter()
    .map(|path| {
        let file = prepared
            .files()
            .iter()
            .find(|file| file.path == path)
            .unwrap_or_else(|| panic!("{path} is not generated"));
        (path.to_owned(), file.bytes.to_vec())
    })
    .collect()
}

fn chain_consumer(scratch: &Path, fixture: &str, flags: &[&str]) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    // Materialize each family's delivery closure; no repository header fallback.
    let include = scratch.join("include");
    let sources = scratch.join("src");
    std::fs::create_dir(&include).unwrap();
    std::fs::create_dir(&sources).unwrap();
    for (path, bytes) in mainline_generated_headers(scratch) {
        std::fs::write(scratch.join(path.trim_start_matches("generated/")), bytes).unwrap();
    }
    for family in ["runtime/bsw/include", "runtime/host/include"] {
        for entry in std::fs::read_dir(root.join(family)).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                std::fs::copy(entry.path(), include.join(entry.file_name())).unwrap();
            }
        }
    }
    for name in [
        "ComStack_Cfg.h",
        "SchM_CanIf.h",
        "SchM_ComM.h",
        "SchM_CanTp.h",
        "SchM_Det.h",
        "SchM_Com.h",
        "SchM_PduR.h",
        "SchM_LSduR.h",
    ] {
        std::fs::copy(
            root.join("core/tests/fixtures/bsw-contracts").join(name),
            include.join(name),
        )
        .unwrap();
    }
    let executable = tooling::native_binary(scratch, "com-chain-consumer");
    let mut command = tooling::c_compiler();
    if cfg!(target_os = "linux") {
        command.arg("-pthread");
    }
    if matches!(
        fixture,
        "com_chain_contract.c" | "diagnostic_queued_contract.c"
    ) {
        command.arg("-DCAN_HOST_QUEUED_COMPLETION=1");
    }
    command.args(flags);
    if fixture == "can_polling_contract.c" {
        // Independent selected controller configuration; final generation derives ARXML values.
        command.args([
            "-DCAN_TX_HOH=1u",
            "-DCAN_RX_POLLING=1",
            "-DCAN_TX_POLLING=1",
            "-DCAN_BUSOFF_POLLING=1",
        ]);
    }
    if fixture == "can_callback_lifetime.c" {
        command.arg(if cfg!(windows) {
            "-Wl,--wrap=__imp_EnterCriticalSection"
        } else {
            "-Wl,--wrap=pthread_mutex_lock"
        });
        command.args([
            "-Wl,--wrap=SchM_Exit_Can_CAN_STATE",
            "-Wl,--wrap=Can_Write",
            "-Wl,--wrap=CanIf_ControllerModeIndication",
            "-Wl,--wrap=CanIf_ControllerBusOff",
            "-Wl,--wrap=CanIf_TxConfirmation",
            "-Wl,--wrap=CanIf_RxIndication",
        ]);
    }
    if fixture.starts_with("routing_") {
        command.arg("-Wl,--wrap=CanIf_Transmit");
    }
    if fixture == "routing_parallel.c" {
        command.args([
            "-Wl,--wrap=SchM_Exit_PduR_PDUR_STATE",
            "-Wl,--wrap=SchM_Exit_LSduR_LSDUR_STATE",
        ]);
    }
    if fixture == "can_contract.c" {
        // Distinct driver and CanIf controller IDs; CanIf callbacks only record.
        command.args([
            "-DCAN_CONTROLLER_ID=3u",
            "-DCAN_CANIF_CONTROLLER_ID=5u",
            "-DCAN_RX_HOH=0u",
            "-DCAN_TX_HOH=1u",
            "-DCAN_RX_POLLING=1",
            "-DCAN_TX_POLLING=1",
            "-DCAN_BUSOFF_POLLING=1",
            "-Wl,--wrap=CanIf_ControllerModeIndication",
            "-Wl,--wrap=CanIf_ControllerBusOff",
            "-Wl,--wrap=CanIf_TxConfirmation",
            "-Wl,--wrap=CanIf_RxIndication",
        ]);
    }
    if fixture == "mode_contract.c" {
        // Independent non-equal namespace oracle; final generation derives source IDs.
        command.args([
            "-DCAN_CONTROLLER_ID=7u",
            "-DCAN_CANIF_CONTROLLER_ID=9u",
            "-DCAN_RX_HOH=11u",
            "-DCAN_TX_HOH=17u",
        ]);
    }
    command
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-I")
        .arg(include);
    for family in ["runtime/bsw/src", "runtime/host/src"] {
        for entry in std::fs::read_dir(root.join(family)).unwrap() {
            let entry = entry.unwrap();
            if entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "h")
            {
                std::fs::copy(entry.path(), sources.join(entry.file_name())).unwrap();
            }
        }
    }
    command.arg("-I").arg(&sources);
    for source in [
        "runtime/bsw/src/Com.c",
        "runtime/bsw/src/PduR.c",
        "runtime/bsw/src/LSduR.c",
        "runtime/bsw/src/CanIf.c",
        "runtime/host/src/SchM_CanIf_Host.c",
        "runtime/bsw/src/ComM.c",
        "runtime/bsw/src/BswM.c",
        "runtime/host/src/Ecu_HostBusSM.c",
        "runtime/host/src/SchM_Mode_Host.c",
        "runtime/bsw/src/Dcm.c",
        "runtime/bsw/src/CanTp.c",
        "runtime/host/src/SchM_Diagnostic_Host.c",
        "runtime/bsw/src/Det.c",
        "runtime/host/src/Det_Host.c",
        "runtime/bsw/src/Can.c",
        "runtime/host/src/Can_Host.c",
        "runtime/host/src/Can_HostLock.c",
        "core/tests/fixtures/bsw-contracts/execution_sink.c",
        "core/tests/fixtures/bsw-contracts/schm_dcm_mode.c",
    ] {
        let delivered = sources.join(std::path::Path::new(source).file_name().unwrap());
        std::fs::copy(root.join(source), &delivered).unwrap();
        command.arg(delivered);
    }
    command.arg(root.join("core/tests/fixtures/bsw-contracts").join(fixture));
    let output = command.arg("-o").arg(&executable).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

/// Runs a threaded consumer under the owned-process limits.
fn run_owned(
    executable: PathBuf,
    scratch: &Path,
    logs: PathBuf,
) -> autosar_config_core::execution::ProcessResult {
    let spec = autosar_config_core::execution::ProcessSpec::for_duration(
        vec![executable.into_os_string()],
        scratch.to_path_buf(),
        vec![],
        std::time::Duration::from_secs(30),
        logs,
    )
    .unwrap();
    tooling::wait_success(
        tooling::execution_owner().spawn(spec, None).unwrap(),
        "consumer",
    )
}

#[test]
fn com_periodic_transmission_executes_real_lower_modules_and_can_busy() {
    let scratch = Scratch::new();
    let executable = chain_consumer(&scratch.0, "com_chain_contract.c", &[]);
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{:?}",
        output
    );
}

#[test]
fn canif_init_routing_pdu_modes_and_confirmations_follow_sws() {
    let scratch = Scratch::new();
    let executable = chain_consumer(&scratch.0, "canif_contract.c", &[]);
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
}

/// Shared CanIf state stays consistent under preemption (Verifies: SWS_CANIF_00064).
#[test]
fn canif_different_pdus_and_actual_confirmation_stop_interleave_safely() {
    let scratch = Scratch::new();
    let executable = chain_consumer(&scratch.0, "canif_parallel.c", &[]);
    let logs = scratch.0.join("execution");
    std::fs::create_dir(&logs).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let result = run_owned(executable, &scratch.0, logs);
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
fn routing_keeps_direction_and_module_identity_for_equal_handles() {
    let scratch = Scratch::new();
    let executable = chain_consumer(&scratch.0, "routing_contract.c", &[]);
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{:?}",
        output
    );
}

/// The routers leave their areas before calling another module (ADR 0018).
#[test]
fn routing_requests_complete_once_while_another_waits_outside_the_area() {
    let scratch = Scratch::new();
    let executable = chain_consumer(&scratch.0, "routing_parallel.c", &[]);
    let logs = scratch.0.join("execution");
    std::fs::create_dir(&logs).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let result = run_owned(executable, &scratch.0, logs);
    assert!(result.success(), "{result:?}");
}

#[test]
fn comm_cdd_mode_demand_and_immediate_bswm_use_actual_lower_modes() {
    let scratch = Scratch::new();
    let executable = chain_consumer(&scratch.0, "mode_contract.c", &[]);
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
    let executable = chain_consumer(&scratch.0, "diagnostic_contract.c", &[]);
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
    let executable = chain_consumer(&scratch.0, "can_polling_contract.c", &[]);
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{:?}",
        output
    );
}

#[test]
fn can_driver_follows_its_standard_contract() {
    let scratch = Scratch::new();
    let executable = chain_consumer(&scratch.0, "can_contract.c", &[]);
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{:?}",
        output
    );
}

#[test]
fn queued_diagnostic_cancelled_token_cannot_release_reused_transport_buffer() {
    let scratch = Scratch::new();
    let executable = chain_consumer(&scratch.0, "diagnostic_queued_contract.c", &[]);
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{:?}",
        output
    );
}

#[test]
fn can_callbacks_leave_driver_area_and_late_events_converge() {
    for polling in ["0", "1"] {
        let scratch = Scratch::new();
        let flags = [
            format!("-DCAN_TX_POLLING={polling}"),
            format!("-DCAN_RX_POLLING={polling}"),
            format!("-DCAN_BUSOFF_POLLING={polling}"),
        ];
        let flags: Vec<_> = flags.iter().map(String::as_str).collect();
        let executable = chain_consumer(&scratch.0, "can_callback_lifetime.c", &flags);
        let logs = scratch.0.join("execution");
        std::fs::create_dir(&logs).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let result = run_owned(executable, &scratch.0, logs);
        assert!(result.success(), "{result:?}");
    }
}

#[test]
fn cantp_n_as_timeout_aborts_once_and_ignores_late_confirmation() {
    let scratch = Scratch::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let executable = tooling::native_binary(&scratch.0, "cantp-consumer");
    let output = tooling::c_compiler()
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-I")
        .arg(root.join("runtime/bsw/include"))
        .arg("-I")
        .arg(root.join("runtime/host/include"))
        .arg("-I")
        .arg(root.join("runtime/bsw/src"))
        .arg("-I")
        .arg(root.join("runtime/host/src"))
        .arg("-I")
        .arg(root.join("core/tests/fixtures/bsw-contracts"))
        .arg(root.join("runtime/bsw/src/CanTp.c"))
        .arg(root.join("core/tests/fixtures/bsw-contracts/cantp_n_as_contract.c"))
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
}
