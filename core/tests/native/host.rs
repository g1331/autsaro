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
    assert!(
        failure.is_err(),
        "the deliberately stalled process must time out"
    );
    assert!(started.elapsed() < Duration::from_secs(8));
    let pids: Vec<u32> = fs::read_to_string(pid_file)
        .unwrap()
        .lines()
        .map(|line| line.split_whitespace().nth(1).unwrap().parse().unwrap())
        .collect();
    assert_eq!(
        pids.len(),
        3,
        "parent, child and grandchild must actually start"
    );
    for pid in pids {
        assert!(!alive(pid), "watchdog must close PID {pid}");
    }
}

/// Queried in process: a PowerShell query can outlast its own deadline on a loaded runner.
#[cfg(windows)]
fn alive(pid: u32) -> bool {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn WaitForSingleObject(handle: *mut c_void, millis: u32) -> u32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    // SAFETY: only a probe PID is opened with SYNCHRONIZE; the handle is closed.
    let handle = unsafe { OpenProcess(0x0010_0000, 0, pid) };
    if handle.is_null() {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(87) {
            return false;
        }
        panic!("Cannot inspect probe PID {pid}: {error}");
    }
    let outcome = unsafe { WaitForSingleObject(handle, 0) };
    unsafe { CloseHandle(handle) };
    assert_ne!(outcome, 0xFFFF_FFFF, "Cannot wait for probe PID {pid}");
    outcome == 0x102
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
