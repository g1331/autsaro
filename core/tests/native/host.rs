//! Host batch codec, process watchdog and CAN declaration views compiled natively.
use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(windows)]
use std::time::Duration;

#[test]
fn host_batch_codec_keeps_boundaries_numeric_sequence_and_recovery() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let scratch = Scratch::new();
    let binary = scratch.0.join("host-batch-codec.exe");
    let compiler = std::env::var_os("AUTOSAR_CC").unwrap_or_else(|| "gcc".into());
    let output = Command::new(compiler)
        .args(["-std=c99", "-O1", "-Wall", "-Wextra", "-Werror"])
        .arg("-I")
        .arg(root.join("runtime/ecu/include"))
        .arg("-I")
        .arg(root.join("runtime/os/include"))
        .arg("-I")
        .arg(root.join("runtime/bsw/include"))
        .arg("-I")
        .arg(root.join("runtime/host/include"))
        .arg(root.join("runtime/ecu/src/Ecu_HostBatch.c"))
        .arg(root.join("core/tests/fixtures/host_batch_codec.c"))
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let result = Command::new(binary).output().unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8(result.stdout).unwrap().contains(
            "host_batch_codec boundaries=256/257,1000/1001 numeric/sequence/recovery=pass"
        )
    );
}

#[cfg(windows)]
#[test]
fn public_command_watchdog_closes_the_whole_process_tree() {
    let scratch = Scratch::new();
    let pid_file = scratch.0.join("watchdog.pids");
    let mut command = tooling::python_command();
    command
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/python/process/probe.py"))
        .args(["descendant", "--kind", "hang", "--pid-file"])
        .arg(&pid_file);
    let started = std::time::Instant::now();
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tooling::run_public_command(
            &mut command,
            &scratch.0,
            "watchdog-probe",
            Duration::from_secs(2),
        )
    }));
    let returned = system_time();
    let failure = failure.expect_err("the deliberately stalled process must time out");
    let failure = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .unwrap_or_default();
    // A cleanup error also panics; only a confirmed timeout means the Job closed the tree.
    assert!(
        failure.contains("status: Timeout"),
        "watchdog must report a confirmed timeout: {failure}"
    );
    assert!(started.elapsed() < Duration::from_secs(8));
    let probes: Vec<(String, u32)> = fs::read_to_string(pid_file)
        .unwrap()
        .lines()
        .map(|line| {
            let mut fields = line.split_whitespace();
            let role = fields.next().unwrap().to_owned();
            (role, fields.next().unwrap().parse().unwrap())
        })
        .collect();
    assert_eq!(
        probes.len(),
        3,
        "parent, child and grandchild must actually start"
    );
    for (role, pid) in probes {
        assert_probe_closed(&role, pid, returned);
    }
}

/// Kernel32 calls used to observe probes in process: a PowerShell query can outlast its own
/// deadline on a loaded runner.
#[cfg(windows)]
mod kernel32 {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn GetSystemTimePreciseAsFileTime(time: *mut u64);
        pub fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        pub fn GetProcessTimes(
            handle: *mut c_void,
            creation: *mut u64,
            exit: *mut u64,
            kernel: *mut u64,
            user: *mut u64,
        ) -> i32;
        pub fn WaitForSingleObject(handle: *mut c_void, millis: u32) -> u32;
        pub fn CloseHandle(handle: *mut c_void) -> i32;
    }
}

/// Current time as FILETIME ticks, comparable with process creation times.
#[cfg(windows)]
fn system_time() -> u64 {
    let mut time = 0u64;
    // SAFETY: writes one FILETIME, which has the layout of a little-endian u64.
    unsafe { kernel32::GetSystemTimePreciseAsFileTime(&mut time) };
    time
}

/// The Job reports no active process slightly before each process object is signalled, and
/// Windows can hand a closed probe's PID to a later process. Waiting for the exit event covers
/// the first; creation times newer than the command's return rule out the second.
#[cfg(windows)]
fn assert_probe_closed(role: &str, pid: u32, returned: u64) {
    use kernel32::*;
    // Only the kernel's exit signalling remains once the Job is empty, so 5 s is generous on a
    // loaded runner, yet a probe that escaped the watchdog sleeps 30 s and still fails.
    const EXIT_SIGNAL_MILLIS: u32 = 5_000;
    // SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION
    // SAFETY: the returned handle is only queried, waited on and closed below.
    let handle = unsafe { OpenProcess(0x0010_1000, 0, pid) };
    if handle.is_null() {
        let error = std::io::Error::last_os_error();
        // ERROR_INVALID_PARAMETER: no process has this PID any more.
        if error.raw_os_error() == Some(87) {
            return;
        }
        panic!("Cannot inspect {role} probe PID {pid}: {error}");
    }
    let [mut created, mut exited, mut kernel, mut user] = [0u64; 4];
    // SAFETY: live handle and four writable FILETIME slots.
    let times =
        unsafe { GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) };
    let times_error = std::io::Error::last_os_error();
    let waiting = std::time::Instant::now();
    let outcome = if times != 0 && created <= returned {
        // SAFETY: live handle opened with SYNCHRONIZE.
        unsafe { WaitForSingleObject(handle, EXIT_SIGNAL_MILLIS) }
    } else {
        0
    };
    let waited = waiting.elapsed();
    let wait_error = std::io::Error::last_os_error();
    // SAFETY: closes the handle opened above exactly once.
    unsafe { CloseHandle(handle) };
    assert_ne!(
        times, 0,
        "Cannot read {role} probe PID {pid} times: {times_error}"
    );
    assert_ne!(
        outcome, 0xFFFF_FFFF,
        "Cannot wait for {role} probe PID {pid}: {wait_error}"
    );
    assert_eq!(
        outcome, 0,
        "watchdog must close {role} probe PID {pid} (created at FILETIME {created}, \
         command returned at {returned}); still running after {waited:?}"
    );
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn can_declaration_views_and_memmap_enforce_the_public_boundary() {
    let temp = Scratch::new();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let identity = tooling::execution_settings();
    let layout = r#"
#include <stddef.h>
#include <stdint.h>
typedef struct { PduIdType handle; uint8_t length; uint32_t id; uint8_t *data; } ExpectedPdu;
typedef struct { EcuStatus (*sink)(uint32_t, uint8_t, const uint8_t[8]); } ExpectedConfig;
typedef char Id32[(sizeof(Can_IdType) == 4u) ? 1 : -1];
typedef char Hoh16[(sizeof(Can_HwHandleType) == 2u) ? 1 : -1];
typedef char PduLayout[(sizeof(Can_PduType) == sizeof(ExpectedPdu) &&
 offsetof(Can_PduType, swPduHandle) == offsetof(ExpectedPdu, handle) &&
 offsetof(Can_PduType, length) == offsetof(ExpectedPdu, length) &&
 offsetof(Can_PduType, id) == offsetof(ExpectedPdu, id) &&
 offsetof(Can_PduType, sdu) == offsetof(ExpectedPdu, data)) ? 1 : -1];
typedef char ConfigLayout[(sizeof(Can_ConfigType) == sizeof(ExpectedConfig) &&
 offsetof(Can_ConfigType, sink) == 0u) ? 1 : -1];
void (*initialize)(const Can_ConfigType *) = Can_Init;
Std_ReturnType (*write_frame)(uint16_t, const Can_PduType *) = Can_Write;
"#;
    // Mainline consumers reach the standard view only as "standard/Can.h".
    for (index, (includes, allowed)) in [
        ("#include \"standard/Can.h\"\n", true),
        ("#include \"standard/Can.h\"\nCanMode leaked;\n", false),
        (
            "#include \"standard/Can.h\"\nvoid (*leaked)(CanMode) = Can_SetMode;\n",
            false,
        ),
        (
            "#include \"standard/Can.h\"\nvoid *leaked = (void *)&Can_HostFlush;\n",
            false,
        ),
        (
            "#include \"standard/Can.h\"\nvoid *leaked = (void *)&Can_InternalInject;\n",
            false,
        ),
        (
            "#include \"standard/Can.h\"\n#include \"Can_Host.h\"\n",
            true,
        ),
        (
            "#include \"Can_Host.h\"\n#include \"standard/Can.h\"\n",
            true,
        ),
        (
            "#define CAN_VERSION_INFO_API 1\n#include \"standard/Can.h\"\n",
            false,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let source = temp.0.join(format!("view-{index}.c"));
        fs::write(&source, format!("{includes}{layout}")).unwrap();
        let output = Command::new(&identity.compiler)
            .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic", "-c"])
            .arg("-I")
            .arg(root.join("runtime/bsw/include"))
            .arg("-I")
            .arg(root.join("runtime/host/include"))
            .arg("-I")
            .arg(root.join("core/tests/fixtures/bsw-contracts"))
            .arg(&source)
            .arg("-o")
            .arg(temp.0.join("view.o"))
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            allowed,
            "{index}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    for (index, (body, allowed)) in [
        ("#define CAN_START_SEC_CODE\n#include \"Can_MemMap.h\"\nCAN_CODE int mapped(void) { return 0; }\n#define CAN_STOP_SEC_CODE\n#include \"Can_MemMap.h\"\n#define CAN_START_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Can_MemMap.h\"\nint zero CAN_VAR_CLEARED;\n#define CAN_STOP_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Can_MemMap.h\"\n#define CAN_START_SEC_CODE\n#include \"Can_MemMap.h\"\nCAN_CODE int main(void) { return mapped() + zero; }\n#define CAN_STOP_SEC_CODE\n#include \"Can_MemMap.h\"\n", true),
        ("#include \"Can_MemMap.h\"\n", false),
        ("#define CAN_START_SEC_UNKNOWN\n#include \"Can_MemMap.h\"\n", false),
        ("#define CAN_STOP_SEC_CODE\n#include \"Can_MemMap.h\"\n", false),
        ("#define CAN_START_SEC_CODE\n#include \"Can_MemMap.h\"\n#define CAN_START_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Can_MemMap.h\"\n", false),
        ("#define CAN_START_SEC_CODE\n#include \"Can_MemMap.h\"\n#define CAN_STOP_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Can_MemMap.h\"\n", false),
        ("#define CAN_START_SEC_CODE\n#define CAN_STOP_SEC_CODE\n#include \"Can_MemMap.h\"\n", false),
    ].into_iter().enumerate() {
        let source = temp.0.join(format!("section-{index}.c"));
        fs::write(&source, body).unwrap();
        let binary = tooling::native_binary(&temp.0, "section");
        let mut command = Command::new(&identity.compiler);
        command.args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
            .arg("-I").arg(root.join("runtime/bsw/include"))
            .arg("-I").arg(root.join("runtime/host/include"))
            .arg("-I").arg(root.join("core/tests/fixtures/bsw-contracts")).arg(&source);
        if !allowed { command.arg("-c"); }
        let output = command.arg("-o").arg(&binary).output().unwrap();
        assert_eq!(output.status.success(), allowed, "section/{index}: {}", String::from_utf8_lossy(&output.stderr));
        if allowed {
            assert!(Command::new(&binary).status().unwrap().success());
            let sections = tooling::symbol_sections(&identity.objdump, &binary);
            for (symbol, section) in [("mapped", ".text"), ("zero", ".bss")] {
                assert!(sections.iter().any(|(name, actual)| name == symbol && actual == section), "{symbol}/{section}: {sections:?}");
            }
        }
    }
}
