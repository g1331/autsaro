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

/// Builds a consumer of one delivered module (`runtime/bsw/src/<module>.c`) with the
/// delivered host areas; `flags` add linker wraps.
fn strict_consumer(scratch: &Path, module: &str, fixture: &str, flags: &[&str]) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let executable = tooling::native_binary(scratch, "strict-consumer");
    let mut command = tooling::c_compiler();
    if cfg!(target_os = "linux") {
        command.arg("-pthread");
    }
    if module == "Dcm" {
        for (path, bytes) in mainline_generated_headers(scratch) {
            if matches!(
                path.as_str(),
                "generated/include/Rte_Dcm_Type.h" | "generated/include/Rte_Type.h"
            ) {
                std::fs::write(scratch.join(Path::new(&path).file_name().unwrap()), bytes).unwrap();
            }
        }
        command.arg("-I").arg(scratch);
        command.arg(root.join("runtime/host/src/SchM_Diagnostic_Host.c"));
    }
    if matches!(module, "ComM" | "BswM") {
        command.arg(root.join("runtime/host/src/SchM_Mode_Host.c"));
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
        .arg(root.join(format!("runtime/bsw/src/{module}.c")))
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
    let executable = strict_consumer(&scratch.0, "Com", "com_contract.c", &[]);
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
    let executable = strict_consumer(
        &scratch.0,
        "Com",
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

/// Runs a strict consumer's window oracle under the owned-execution harness.
fn run_window(module: &str, fixture: &str, wrap: &str) {
    let scratch = Scratch::new();
    let executable = strict_consumer(&scratch.0, module, fixture, &[wrap]);
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

/// ComM leaves COMM_STATE before the bus state manager, BswM and Dcm (ADR 0018).
#[test]
fn comm_bus_requests_and_mode_publications_stay_ordered_while_another_waits_outside_the_area() {
    run_window(
        "ComM",
        "comm_parallel.c",
        "-Wl,--wrap=SchM_Exit_ComM_COMM_STATE",
    );
}

/// BswM runs its action outside BSWM_STATE and postpones concurrent input (ADR 0018).
#[test]
fn bswm_actions_run_once_in_order_while_another_waits_outside_the_area() {
    run_window(
        "BswM",
        "bswm_parallel.c",
        "-Wl,--wrap=SchM_Exit_BswM_BSWM_STATE",
    );
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
        "generated/include/Rte_Dcm_Type.h",
        "generated/include/Rte_Type.h",
        "generated/src/Ecu_Execution.h",
        "generated/include/Rte_Main.h",
        "generated/include/Rte_MemMap.h",
        "generated/src/SchM.c",
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

/// The host bus state manager calls CanIf and ComM outside its own area (ADR 0018).
#[test]
fn bus_state_manager_registers_transitions_and_publishes_in_order_outside_its_area() {
    let scratch = Scratch::new();
    let executable = chain_consumer(
        &scratch.0,
        "bussm_parallel.c",
        &[
            "-Wl,--wrap=Can_HostAreaExit",
            "-Wl,--wrap=CanIf_SetControllerMode",
            "-Wl,--wrap=CanIf_SetPduMode",
        ],
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
fn cantp_standard_consumer_executes_full_duplex_transport_outside_its_area() {
    let scratch = Scratch::new();
    let executable = strict_consumer(&scratch.0, "CanTp", "cantp_contract.c", &[]);
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{:?}",
        output
    );
}

/// CanTp leaves CANTP_STATE before PduR and LSduR, and discards overtaken results (ADR 0018).
#[test]
fn cantp_events_complete_once_while_another_context_waits_outside_the_area() {
    let scratch = Scratch::new();
    let executable = strict_consumer(
        &scratch.0,
        "CanTp",
        "cantp_parallel.c",
        &["-Wl,--wrap=SchM_Exit_CanTp_CANTP_STATE"],
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

#[test]
fn dcm_standard_consumer_executes_sessions_keepalive_and_response_failures() {
    let scratch = Scratch::new();
    let executable = strict_consumer(&scratch.0, "Dcm", "dcm_contract.c", &[]);
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{output:?}"
    );
}

#[test]
fn dcm_application_calls_and_publications_survive_concurrent_restarts() {
    let scratch = Scratch::new();
    let executable = strict_consumer(
        &scratch.0,
        "Dcm",
        "dcm_parallel.c",
        &["-Wl,--wrap=SchM_Exit_Dcm_DCM_STATE"],
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

/// The actual generated immediate SchM switch must work outside the ECU owner.
/// Verifies: SWS_Dcm_00520, SWS_Dcm_01062, SWS_Dcm_00311
#[test]
fn dcm_generated_mode_switch_publishes_from_a_non_owner_context() {
    let scratch = Scratch::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for (path, bytes) in mainline_generated_headers(&scratch.0) {
        std::fs::write(scratch.0.join(Path::new(&path).file_name().unwrap()), bytes).unwrap();
    }
    // Resolve the SchM consumer's quoted include beside the generated type header.
    std::fs::copy(
        root.join("runtime/bsw/include/SchM_Dcm.h"),
        scratch.0.join("SchM_Dcm.h"),
    )
    .unwrap();
    std::fs::write(scratch.0.join("Ecu_Target.h"), "#include \"Std_Types.h\"\nint Ecu_TargetIsOwner(void);\nStd_ReturnType Ecu_TargetCheckLifecycleContext(boolean starting);\n").unwrap();
    let harness = scratch.0.join("mode-context.c");
    std::fs::write(&harness, r#"#include "Rte_Dcm_Type.h"
#include "Dcm.h"
#include "SchM_Dcm.h"
#include "Dcm_ComM.h"
#include "PduR_Dcm.h"
#include "ComM_Dcm.h"
#include <assert.h>
int Ecu_TargetIsOwner(void) { return 0; }
Std_ReturnType Ecu_TargetCheckLifecycleContext(boolean starting) { (void)starting; return E_OK; }
void ComM_DCM_ActiveDiagnostic(NetworkHandleType channel) { (void)channel; }
void ComM_DCM_InactiveDiagnostic(NetworkHandleType channel) { (void)channel; }
Std_ReturnType PduR_DcmTransmit(PduIdType id, const PduInfoType *info) { (void)id; (void)info; assert(0); return E_NOT_OK; }
static void authentication(uint8 value) { assert(value == 0u); }
int main(void) {
    const Dcm_ConfigType config = {17u, 19u, 7u, 0u, NULL_PTR, 4u, 50u, 5000u, 5u, 64u, 41u, authentication, 0u};
    uint8 bytes[2] = {0x10u, 0x83u};
    PduInfoType info = {bytes, NULL_PTR, 2u};
    PduLengthType available;
    Dcm_SesCtrlType session;
    Dcm_Init(&config);
    assert(SchM_Mode_Dcm_DcmDiagnosticSessionControl() == 0u);
    Dcm_ComM_FullComModeEntered(7u);
    assert(Dcm_StartOfReception(17u, &info, 2u, &available) == BUFREQ_OK);
    assert(Dcm_CopyRxData(17u, &info, &available) == BUFREQ_OK);
    Dcm_TpRxIndication(17u, E_OK);
    Dcm_MainFunction();
    assert(Dcm_GetSesCtrlType(&session) == E_OK && session == 3u);
    assert(SchM_Mode_Dcm_DcmDiagnosticSessionControl() == 2u);
    assert(Dcm_ResetToDefaultSession() == E_OK);
    assert(Dcm_GetSesCtrlType(&session) == E_OK && session == 1u);
    assert(SchM_Mode_Dcm_DcmDiagnosticSessionControl() == 0u);
    return 0;
}
"#).unwrap();
    let binary = tooling::native_binary(&scratch.0, "mode-context");
    let mut compiler = tooling::c_compiler();
    if cfg!(target_os = "linux") {
        compiler.arg("-pthread");
    }
    let output = compiler
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-I")
        .arg(&scratch.0)
        .args([
            "-Iruntime/bsw/include",
            "-Iruntime/bsw/src",
            "-Iruntime/host/include",
            "-Iruntime/host/src",
            "-Icore/tests/fixtures/bsw-contracts",
        ])
        .current_dir(root)
        .args([
            "runtime/bsw/src/Dcm.c",
            "runtime/host/src/Can_HostLock.c",
            "runtime/host/src/SchM_Diagnostic_Host.c",
        ])
        .arg(scratch.0.join("SchM.c"))
        .arg(harness)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new(binary).output().unwrap();
    assert!(output.status.success(), "{output:?}");
}
