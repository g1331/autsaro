use autosar_config_core::{DiagnosticSettings, Direction, Workspace, generator, host, schema};
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn standard_can_host_entry_points_reject_invalid_requests_and_send_valid_frame() {
    let temp = Scratch::new();
    let harness = temp.0.join("can_standard_api.c");
    let binary = temp.0.join(if cfg!(windows) {
        "can_standard_api.exe"
    } else {
        "can_standard_api"
    });
    fs::write(
        &harness,
r#"#include "Can.h"
#include "CanIf.h"
#ifdef _WIN32
#include <windows.h>
#else
#include <pthread.h>
#endif
static unsigned sent;
static unsigned received;
static unsigned fail_output;
static int valid_result;
static int invalid_result;
static EcuStatus emit(uint32_t id, uint8_t dlc, const uint8_t data[8]) {
    Can_ControllerStateType state = CAN_CS_UNINIT;
    if (Can_GetControllerMode(0u, &state) != E_OK || state != CAN_CS_STARTED) return ECU_ERR_CONTROLLER;
    if (id != 0x321u || dlc != 2u || data[0] != 0x12u || data[1] != 0x34u) return ECU_ERR_IO;
    if (fail_output != 0u) return ECU_ERR_IO;
    ++sent;
    return ECU_OK;
}
EcuStatus CanIf_RxIndication(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    if (id != 0x321u || dlc != 2u || data[0] != 0x12u || now_ms != 10u) return ECU_ERR_IO;
    ++received;
    return ECU_OK;
}
static void send_valid_frames(void) {
    uint8_t bytes[8] = {0x12u, 0x34u};
    unsigned i;
    for (i = 0u; i < 100u; ++i) {
        if (Can_Transmit(0x321u, 2u, bytes) != ECU_OK) valid_result = 1;
    }
}
static void reject_invalid_frames(void) {
    uint8_t bytes[8] = {0x12u, 0x34u};
    unsigned i;
    for (i = 0u; i < 100u; ++i) {
        if (Can_Transmit(0x800u, 2u, bytes) != ECU_ERR_FRAME_ID) invalid_result = 1;
    }
}
#ifdef _WIN32
static DWORD WINAPI valid_thread(LPVOID unused) { (void)unused; send_valid_frames(); return 0u; }
static DWORD WINAPI invalid_thread(LPVOID unused) { (void)unused; reject_invalid_frames(); return 0u; }
#else
static void *valid_thread(void *unused) { (void)unused; send_valid_frames(); return NULL; }
static void *invalid_thread(void *unused) { (void)unused; reject_invalid_frames(); return NULL; }
#endif
int main(void) {
    uint8_t bytes[8] = {0x12u, 0x34u};
    Can_ConfigType config = {emit};
    Can_PduType pdu = {0u, 2u, 0x321u, bytes};
    Can_ControllerStateType state = CAN_CS_UNINIT;
    Can_Init(NULL);
    if (Can_GetControllerMode(0u, &state) != E_NOT_OK) return 1;
    Can_Init(&config);
    if (Can_GetControllerMode(0u, &state) != E_OK || state != CAN_CS_STOPPED) return 2;
    if (Can_Write(0u, &pdu) != E_NOT_OK) return 3;
    if (Can_SetControllerMode(0u, CAN_CS_STOPPED) != E_NOT_OK) return 13;
    if (Can_SetControllerMode(1u, CAN_CS_STARTED) != E_NOT_OK) return 4;
    if (Can_SetControllerMode(0u, CAN_CS_STARTED) != E_OK) return 5;
    if (Can_SetControllerMode(0u, CAN_CS_STARTED) != E_NOT_OK) return 14;
    if (Can_Write(1u, &pdu) != E_NOT_OK || Can_Write(0u, NULL) != E_NOT_OK) return 6;
    pdu.id = 0x800u;
    if (Can_Write(0u, &pdu) != E_NOT_OK) return 7;
    pdu.id = 0x321u;
    pdu.length = 0u;
    if (Can_Write(0u, &pdu) != E_NOT_OK) return 8;
    pdu.length = 2u;
    if (Can_Write(0u, &pdu) != E_OK || sent != 1u) return 9;
    if (Can_Inject(0x321u, 2u, bytes, 10u) != ECU_OK || received != 1u) return 10;
    if (Can_Inject(0x321u, 2u, NULL, 10u) != ECU_ERR_CONFIG || received != 1u) return 12;
    fail_output = 1u;
    if (Can_Transmit(0x321u, 2u, bytes) != ECU_ERR_IO || sent != 1u) return 20;
    fail_output = 0u;
    Can_SetMode(CAN_BUS_OFF);
    if (Can_GetControllerMode(0u, &state) != E_OK || state != CAN_CS_STOPPED) return 15;
    if (Can_GetMode() != CAN_BUS_OFF) return 21;
    if (Can_Transmit(0x321u, 2u, bytes) != ECU_ERR_CONTROLLER) return 11;
    if (Can_SetControllerMode(0u, CAN_CS_STARTED) != E_OK) return 16;
    if (Can_GetMode() != CAN_STARTED) return 17;
    if (Can_SetControllerMode(0u, CAN_CS_STOPPED) != E_OK) return 18;
    if (Can_SetControllerMode(0u, CAN_CS_STOPPED) != E_NOT_OK) return 19;
    if (Can_SetControllerMode(0u, CAN_CS_STARTED) != E_OK) return 22;
#ifdef _WIN32
    {
        HANDLE valid = CreateThread(NULL, 0, valid_thread, NULL, 0, NULL);
        HANDLE invalid = CreateThread(NULL, 0, invalid_thread, NULL, 0, NULL);
        if (valid == NULL || invalid == NULL) return 23;
        if (WaitForSingleObject(valid, INFINITE) != WAIT_OBJECT_0) return 24;
        if (WaitForSingleObject(invalid, INFINITE) != WAIT_OBJECT_0) return 25;
        CloseHandle(valid);
        CloseHandle(invalid);
    }
#else
    {
        pthread_t valid;
        pthread_t invalid;
        if (pthread_create(&valid, NULL, valid_thread, NULL) != 0) return 23;
        if (pthread_create(&invalid, NULL, invalid_thread, NULL) != 0) return 24;
        if (pthread_join(valid, NULL) != 0 || pthread_join(invalid, NULL) != 0) return 25;
    }
#endif
    if (valid_result != 0 || invalid_result != 0 || sent != 101u) return 26;
    return 0;
}
"#,
    )
    .unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let build = Command::new("gcc")
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-pthread")
        .arg(format!("-I{}", root.join("runtime/include").display()))
        .arg(root.join("runtime/src/Can.c"))
        .arg(&harness)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&binary).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("autosar-config-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn archive() -> PathBuf {
    schema::schema_archive(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
}
fn create_pair(root: &Path) -> (Workspace, Workspace) {
    let zip = archive();
    let mut a = Workspace::create(&root.join("Alpha"), "Alpha", zip.clone()).unwrap();
    let tx = a
        .add_frame("Command".into(), 0x321, 2, Direction::Tx, Some(10), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    a.add_signal(tx, "SendCount".into(), 3, 8, 5).unwrap();
    let rx = a
        .add_frame("Reply".into(), 0x456, 2, Direction::Rx, None, Some(50))
        .unwrap()
        .frames
        .into_iter()
        .find(|f| f.name == "Reply")
        .unwrap()
        .path;
    a.add_signal(rx, "RecvStatus".into(), 0, 8, 0).unwrap();
    assert!(a.validate().unwrap().issues.is_empty());
    a.save().unwrap();
    let mut b = Workspace::create(&root.join("Beta"), "Beta", zip).unwrap();
    let rx = b
        .add_frame("Command".into(), 0x321, 2, Direction::Rx, None, Some(40))
        .unwrap()
        .frames[0]
        .path
        .clone();
    b.add_signal(rx, "RecvCount".into(), 3, 8, 0).unwrap();
    let tx = b
        .add_frame("Reply".into(), 0x456, 2, Direction::Tx, Some(20), None)
        .unwrap()
        .frames
        .into_iter()
        .find(|f| f.name == "Reply")
        .unwrap()
        .path;
    b.add_signal(tx, "SendStatus".into(), 0, 8, 7).unwrap();
    assert!(b.validate().unwrap().issues.is_empty());
    b.save().unwrap();
    (a, b)
}

#[test]
fn regeneration_preserves_user_edits_to_generated_files() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    generator::generate(&mut ecu, &output).unwrap();
    for name in [
        "src/Com.c",
        "Ecu_Config.c",
        "README.md",
        "build.ps1",
        "files.list",
        "files.sha256",
    ] {
        let changed = b"user edited generated output";
        let file = output.join(name);
        let original = fs::read(&file).unwrap();
        fs::write(&file, changed).unwrap();
        assert!(generator::generate(&mut ecu, &output).is_err(), "{name}");
        assert_eq!(fs::read(&file).unwrap(), changed, "{name}");
        fs::write(file, original).unwrap();
    }
}

#[cfg(windows)]
#[test]
fn generated_handoff_builds_and_runs_after_moving_without_the_workbench() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    generator::generate(&mut ecu, &output).unwrap();
    let delivered = temp.0.join("Delivered ECU with spaces");
    fs::rename(&output, &delivered).unwrap();
    let readme = fs::read_to_string(delivered.join("README.md")).unwrap();
    assert!(readme.contains(".\\ecu_host.exe"));
    assert!(!readme.contains("--nvm"));
    assert!(!readme.contains("{{RUN_COMMAND}}"));
    let listed = fs::read_to_string(delivered.join("files.list")).unwrap();
    assert!(listed.lines().any(|name| name == "README.md"));
    assert!(listed.lines().any(|name| name == "build.ps1"));

    let documented_command = readme
        .lines()
        .find(|line| line.contains("powershell -NoProfile -ExecutionPolicy Bypass -File"))
        .unwrap()
        .split('`')
        .nth(1)
        .unwrap()
        .replace("<generated-directory>", delivered.to_str().unwrap());
    let build = Command::new("powershell")
        .args(["-NoProfile", "-Command", &documented_command])
        .current_dir(&temp.0)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let binary = delivered.join("ecu_host.exe");
    let mut process = Command::new(&binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    process.stdin.take().unwrap().write_all(b"T 10\n").unwrap();
    let result = process.wait_with_output().unwrap();
    assert!(result.status.success());
    assert_eq!(
        String::from_utf8(result.stdout).unwrap().trim(),
        "X 801 2 2800"
    );

    fs::write(&binary, b"owner binary").unwrap();
    let repeated = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(delivered.join("build.ps1"))
        .current_dir(&temp.0)
        .output()
        .unwrap();
    assert!(!repeated.status.success());
    assert_eq!(fs::read(&binary).unwrap(), b"owner binary");
}

#[test]
fn regeneration_rejects_missing_proof_and_unlisted_user_content() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    generator::generate(&mut ecu, &output).unwrap();
    let config = fs::read(output.join("Ecu_Config.c")).unwrap();

    let proof = output.join("files.sha256");
    let proof_contents = fs::read(&proof).unwrap();
    fs::remove_file(&proof).unwrap();
    assert!(
        generator::generate(&mut ecu, &output)
            .unwrap_err()
            .contains("完整性记录")
    );
    assert_eq!(fs::read(output.join("Ecu_Config.c")).unwrap(), config);

    fs::write(&proof, b"invalid proof\n").unwrap();
    assert!(generator::generate(&mut ecu, &output).is_err());
    fs::write(&proof, proof_contents).unwrap();
    let manifest = output.join("files.list");
    let manifest_contents = fs::read(&manifest).unwrap();
    fs::remove_file(&manifest).unwrap();
    assert!(
        generator::generate(&mut ecu, &output)
            .unwrap_err()
            .contains("文件清单")
    );
    assert_eq!(fs::read(output.join("Ecu_Config.c")).unwrap(), config);
    fs::write(&manifest, manifest_contents).unwrap();
    let extra = output.join("notes.txt");
    fs::write(&extra, b"user content").unwrap();
    assert!(generator::generate(&mut ecu, &output).is_err());
    assert_eq!(fs::read(&extra).unwrap(), b"user content");
    fs::remove_file(&extra).unwrap();
    let extra_dir = output.join("user-data");
    fs::create_dir(&extra_dir).unwrap();
    assert!(generator::generate(&mut ecu, &output).is_err());
    assert!(extra_dir.is_dir());
    assert_eq!(fs::read(output.join("Ecu_Config.c")).unwrap(), config);
}

#[test]
fn regeneration_rejects_a_missing_generated_file() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    generator::generate(&mut ecu, &output).unwrap();
    let missing = output.join("include/Can.h");
    fs::remove_file(&missing).unwrap();
    let manifest = fs::read(output.join("files.list")).unwrap();

    assert!(generator::generate(&mut ecu, &output).is_err());
    assert!(!missing.exists());
    assert_eq!(fs::read(output.join("files.list")).unwrap(), manifest);
}

#[cfg(windows)]
#[test]
fn generation_rejects_junction_output_without_touching_its_target() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let owner = temp.0.join("OwnerData");
    fs::create_dir(&owner).unwrap();
    fs::write(owner.join("sentinel.txt"), b"owner content").unwrap();
    let output = temp.0.join("Generated");
    let junction = Command::new("cmd")
        .arg("/C")
        .arg("mklink")
        .arg("/J")
        .arg(&output)
        .arg(&owner)
        .output()
        .unwrap();
    assert!(
        junction.status.success(),
        "{}",
        String::from_utf8_lossy(&junction.stderr)
    );

    assert!(
        generator::generate(&mut ecu, &output)
            .unwrap_err()
            .contains("重解析点")
    );
    assert_eq!(
        fs::read(owner.join("sentinel.txt")).unwrap(),
        b"owner content"
    );
    assert!(!owner.join("Ecu_Config.c").exists());
    fs::remove_dir(&output).unwrap();
    assert_eq!(
        fs::read(owner.join("sentinel.txt")).unwrap(),
        b"owner content"
    );
}

#[test]
fn generation_rejects_trailing_dot_alias_without_touching_owner_directory() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let owner = temp.0.join("OwnerData");
    fs::create_dir(&owner).unwrap();
    fs::write(owner.join("sentinel.txt"), b"owner content").unwrap();

    assert!(generator::generate(&mut ecu, &owner.join(".")).is_err());
    assert_eq!(
        fs::read(owner.join("sentinel.txt")).unwrap(),
        b"owner content"
    );
    assert!(!owner.join("OwnerData").exists());
}

#[test]
fn regeneration_keeps_previous_output_tree() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    let stage_collision = temp
        .0
        .join(format!(".autosar-config-stage-{}-0", std::process::id()));
    fs::create_dir(&stage_collision).unwrap();
    fs::write(
        stage_collision.join("owner.txt"),
        b"keep staged owner content",
    )
    .unwrap();
    let initial = generator::generate(&mut ecu, &output).unwrap();
    assert!(initial.previous_output_directory.is_none());
    assert_eq!(
        fs::read(stage_collision.join("owner.txt")).unwrap(),
        b"keep staged owner content"
    );
    let old_config = fs::read(output.join("Ecu_Config.c")).unwrap();
    let old_manifest = fs::read(output.join("files.list")).unwrap();
    let collision = temp
        .0
        .join(format!(".autosar-config-backup-{}-0", std::process::id()));
    fs::create_dir(&collision).unwrap();
    fs::write(collision.join("owner.txt"), b"do not replace").unwrap();
    let frame = ecu
        .view()
        .frames
        .iter()
        .find(|frame| frame.name == "Command")
        .unwrap()
        .path
        .clone();
    ecu.update_frame(&frame, serde_json::json!({"periodMs": 15}))
        .unwrap();
    ecu.save().unwrap();
    let regenerated = generator::generate(&mut ecu, &output).unwrap();
    let previous = PathBuf::from(regenerated.previous_output_directory.unwrap());
    assert!(previous.is_absolute() && !previous.starts_with(&collision));
    assert_eq!(fs::read(previous.join("Ecu_Config.c")).unwrap(), old_config);
    assert_eq!(fs::read(previous.join("files.list")).unwrap(), old_manifest);
    assert_eq!(
        fs::read(collision.join("owner.txt")).unwrap(),
        b"do not replace"
    );
    assert_ne!(fs::read(output.join("Ecu_Config.c")).unwrap(), old_config);

    let next = generator::generate(&mut ecu, &output).unwrap();
    assert_ne!(
        next.previous_output_directory.as_deref(),
        Some(previous.to_str().unwrap())
    );
    assert_eq!(fs::read(previous.join("Ecu_Config.c")).unwrap(), old_config);
}

#[cfg(windows)]
#[test]
fn regeneration_preserves_built_binary_until_owner_moves_it() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    generator::generate(&mut ecu, &output).unwrap();
    let binary = generator::build(&output).unwrap().binary_path;
    let original = fs::read(&binary).unwrap();
    let config = fs::read(output.join("Ecu_Config.c")).unwrap();
    let manifest = fs::read(output.join("files.list")).unwrap();
    let proof = fs::read(output.join("files.sha256")).unwrap();

    let error = generator::generate(&mut ecu, &output).unwrap_err();
    assert!(error.contains("请选择新的空输出目录"), "{error}");
    assert_eq!(fs::read(&binary).unwrap(), original);
    assert_eq!(fs::read(output.join("Ecu_Config.c")).unwrap(), config);
    assert_eq!(fs::read(output.join("files.list")).unwrap(), manifest);
    assert_eq!(fs::read(output.join("files.sha256")).unwrap(), proof);
    let archived = temp.0.join("PreservedBuild.exe");
    fs::rename(&binary, &archived).unwrap();
    generator::generate(&mut ecu, &output).unwrap();
    assert_eq!(fs::read(&archived).unwrap(), original);
}

#[cfg(windows)]
#[test]
fn rebuild_rejects_existing_binary_without_overwriting_owner_bytes() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    let generated = generator::generate(&mut ecu, &output).unwrap();
    let output = PathBuf::from(generated.output_directory);
    let binary = generator::build(&output).unwrap().binary_path;
    fs::write(&binary, b"owner modified binary").unwrap();

    assert!(generator::build(&output).is_err());
    assert_eq!(fs::read(&binary).unwrap(), b"owner modified binary");
    let archived = temp.0.join("OwnerBinary.exe");
    fs::rename(&binary, &archived).unwrap();
    let rebuilt = generator::build(&output).unwrap().binary_path;
    let mut process = Command::new(rebuilt)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    process
        .stdin
        .take()
        .unwrap()
        .write_all(b"S 0 54\nT 10\n")
        .unwrap();
    let result = process.wait_with_output().unwrap();
    assert!(result.status.success());
    assert_eq!(
        String::from_utf8(result.stdout).unwrap().trim(),
        "X 801 2 B001"
    );
    assert_eq!(fs::read(archived).unwrap(), b"owner modified binary");
}

#[test]
fn build_rejects_changed_generated_inputs_before_compiling() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    for (case, path, change) in [
        ("changed source", "Ecu_Config.c", "append"),
        ("changed manifest", "files.list", "append"),
        ("extra source", "src/Owner.c", "create"),
        ("missing header", "include/Can.h", "remove"),
    ] {
        let output = temp.0.join(case);
        generator::generate(&mut ecu, &output).unwrap();
        let target = output.join(path);
        match change {
            "append" => {
                let mut content = fs::read(&target).unwrap();
                content.extend_from_slice(b"\n/* external edit */\n");
                fs::write(&target, content).unwrap();
            }
            "create" => fs::write(&target, b"int owner(void) { return 1; }\n").unwrap(),
            "remove" => fs::remove_file(&target).unwrap(),
            _ => unreachable!(),
        }
        let before = if target.exists() {
            Some(fs::read(&target).unwrap())
        } else {
            None
        };
        let error = generator::build(&output).unwrap_err();
        assert!(error.contains("拒绝构建"), "{case}: {error}");
        assert!(
            !output
                .join(if cfg!(windows) {
                    "ecu_host.exe"
                } else {
                    "ecu_host"
                })
                .exists(),
            "{case}"
        );
        assert_eq!(
            target.exists().then(|| fs::read(&target).unwrap()),
            before,
            "{case}"
        );
    }
}

#[cfg(windows)]
#[test]
fn build_rejects_source_changed_during_compilation_before_installing_binary() {
    if let Some(output) = std::env::var_os("AUTOSAR_BUILD_MUTATION_CHILD") {
        let error = generator::build(Path::new(&output)).unwrap_err();
        assert!(
            error.contains("编译期间") && error.contains("完整性检查失败"),
            "{error}"
        );
        assert!(!Path::new(&output).join("ecu_host.exe").exists());
        return;
    }

    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    generator::generate(&mut ecu, &output).unwrap();
    let target = output.join("Ecu_Config.c");
    let compiler_source = temp.0.join("mutating_compiler.c");
    let compiler = temp.0.join("mutating_compiler.exe");
    fs::write(&compiler_source, r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(int argc, char **argv) {
    const char *target = getenv("AUTOSAR_MUTATION_TARGET");
    FILE *changed = target ? fopen(target, "ab") : NULL;
    if (changed == NULL || fputs("\n/* changed during build */\n", changed) < 0 || fclose(changed) != 0) return 1;
    for (int i = 1; i + 1 < argc; ++i) {
        if (strcmp(argv[i], "-o") == 0) {
            FILE *binary = fopen(argv[i + 1], "wb");
            if (binary == NULL || fputs("staged binary", binary) < 0 || fclose(binary) != 0) return 2;
            return 0;
        }
    }
    return 3;
}
"#).unwrap();
    let compiled = Command::new("gcc")
        .arg(&compiler_source)
        .arg("-o")
        .arg(&compiler)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let child = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("build_rejects_source_changed_during_compilation_before_installing_binary")
        .arg("--nocapture")
        .env("AUTOSAR_BUILD_MUTATION_CHILD", &output)
        .env("AUTOSAR_MUTATION_TARGET", &target)
        .env("AUTOSAR_CC", &compiler)
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    assert!(!output.join("ecu_host.exe").exists());
    assert!(
        fs::read_to_string(&target)
            .unwrap()
            .contains("changed during build")
    );
    let stages: Vec<_> = fs::read_dir(&temp.0)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".autosar-config-build-")
        })
        .collect();
    assert_eq!(stages.len(), 1);
    assert_eq!(
        fs::read(stages[0].join("ecu_host.exe")).unwrap(),
        b"staged binary"
    );
}

#[cfg(windows)]
#[test]
fn untouched_output_regenerates_changed_config_and_runs_the_new_schedule() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    generator::generate(&mut ecu, &output).unwrap();
    let original = fs::read(output.join("Ecu_Config.c")).unwrap();
    let frame = ecu
        .view()
        .frames
        .iter()
        .find(|frame| frame.name == "Command")
        .unwrap()
        .path
        .clone();
    ecu.update_frame(&frame, serde_json::json!({"periodMs": 15}))
        .unwrap();
    ecu.save().unwrap();
    generator::generate(&mut ecu, &output).unwrap();
    assert_ne!(fs::read(output.join("Ecu_Config.c")).unwrap(), original);

    let binary = generator::build(&output).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin
        .take()
        .unwrap()
        .write_all(b"S 0 54\nT 10\nT 15\n")
        .unwrap();
    let result = ecu.wait_with_output().unwrap();
    assert!(result.status.success());
    assert_eq!(
        String::from_utf8(result.stdout).unwrap().trim(),
        "X 801 2 B001"
    );
}

#[test]
fn generation_preview_is_read_only_and_confirmed_files_match() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("PreviewOutput");
    fs::create_dir(&output).unwrap();
    let preview = generator::preview_generate(&mut ecu, &output).unwrap();
    assert!(fs::read_dir(&output).unwrap().next().is_none());
    assert!(preview.files.iter().all(|file| file.status == "new"));
    assert!(preview.files.iter().any(|file| file.path == "Ecu_Config.c"
        && file.after.as_ref().unwrap().contains("const EcuConfig")));
    generator::generate_previewed(&mut ecu, &output, &preview.revision).unwrap();
    for file in &preview.files {
        assert_eq!(
            fs::read_to_string(output.join(&file.path)).unwrap(),
            file.after.as_deref().unwrap()
        );
    }

    let first = fs::read(output.join("Ecu_Config.c")).unwrap();
    let frame = ecu
        .view()
        .frames
        .iter()
        .find(|frame| frame.name == "Command")
        .unwrap()
        .path
        .clone();
    ecu.update_frame(&frame, serde_json::json!({"periodMs": 15}))
        .unwrap();
    ecu.save().unwrap();
    let changed = generator::preview_generate(&mut ecu, &output).unwrap();
    assert_eq!(
        changed
            .files
            .iter()
            .find(|file| file.path == "Ecu_Config.c")
            .unwrap()
            .status,
        "changed"
    );
    assert_eq!(fs::read(output.join("Ecu_Config.c")).unwrap(), first);
    assert!(
        generator::generate_previewed(&mut ecu, &output, &preview.revision)
            .unwrap_err()
            .contains("预览已失效")
    );
    generator::generate_previewed(&mut ecu, &output, &changed.revision).unwrap();
    assert_ne!(fs::read(output.join("Ecu_Config.c")).unwrap(), first);
}

#[test]
fn generation_preview_rejects_changed_existing_output() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("PreviewOutput");
    generator::generate(&mut ecu, &output).unwrap();
    let preview = generator::preview_generate(&mut ecu, &output).unwrap();
    let original = fs::read(output.join("Ecu_Config.c")).unwrap();
    fs::write(output.join("Ecu_Config.c"), b"owner change").unwrap();
    assert!(generator::generate_previewed(&mut ecu, &output, &preview.revision).is_err());
    assert_eq!(
        fs::read(output.join("Ecu_Config.c")).unwrap(),
        b"owner change"
    );
    assert_ne!(fs::read(output.join("Ecu_Config.c")).unwrap(), original);
}

#[test]
fn generation_preview_rejects_user_files_binaries_and_bad_proofs_before_writing() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let user_output = temp.0.join("UserOutput");
    fs::create_dir(&user_output).unwrap();
    fs::write(user_output.join("owner.txt"), b"keep me").unwrap();
    assert!(
        generator::preview_generate(&mut ecu, &user_output)
            .unwrap_err()
            .contains("拒绝覆盖")
    );
    assert_eq!(fs::read(user_output.join("owner.txt")).unwrap(), b"keep me");

    let built_output = temp.0.join("BuiltOutput");
    generator::generate(&mut ecu, &built_output).unwrap();
    let binary = built_output.join(if cfg!(windows) {
        "ecu_host.exe"
    } else {
        "ecu_host"
    });
    fs::write(&binary, b"owner binary").unwrap();
    assert!(
        generator::preview_generate(&mut ecu, &built_output)
            .unwrap_err()
            .contains("二进制文件")
    );
    assert_eq!(fs::read(&binary).unwrap(), b"owner binary");

    let changed_output = temp.0.join("ChangedOutput");
    generator::generate(&mut ecu, &changed_output).unwrap();
    fs::write(changed_output.join("Ecu_Config.c"), b"owner edit").unwrap();
    assert!(
        generator::preview_generate(&mut ecu, &changed_output)
            .unwrap_err()
            .contains("完整性记录已被修改")
    );
    assert_eq!(
        fs::read(changed_output.join("Ecu_Config.c")).unwrap(),
        b"owner edit"
    );
}

#[cfg(windows)]
#[test]
fn generated_c99_ecus_exchange_golden_vectors_and_recover_from_faults() {
    let temp = Scratch::new();
    let (mut a, mut b) = create_pair(&temp.0);
    let out_a = temp.0.join("GeneratedAlpha");
    let out_b = temp.0.join("GeneratedBeta");
    fs::create_dir(&out_a).unwrap();
    fs::create_dir(&out_b).unwrap();
    generator::generate(&mut a, &out_a).unwrap();
    generator::generate(&mut b, &out_b).unwrap();
    let mut names: Vec<String> = fs::read_to_string(out_a.join("files.list"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    names.extend(["files.list".into(), "files.sha256".into()]);
    let original: Vec<_> = names
        .iter()
        .map(|name| fs::read(out_a.join(name)).unwrap())
        .collect();
    generator::generate(&mut a, &out_a).unwrap();
    for (name, expected) in names.iter().zip(&original) {
        assert_eq!(
            &fs::read(out_a.join(name)).unwrap(),
            expected,
            "identical ARXML changed generated file {name}"
        );
    }
    let exe_a = generator::build(&out_a).unwrap().binary_path;
    let exe_b = generator::build(&out_b).unwrap().binary_path;
    let mut tx = Command::new(exe_a)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    tx.stdin
        .take()
        .unwrap()
        .write_all(b"S 0 54\nT 10\n")
        .unwrap();
    let actual = String::from_utf8(tx.wait_with_output().unwrap().stdout).unwrap();
    assert!(
        actual
            .lines()
            .any(|line| line.trim_end_matches('\r') == "X 801 2 B001"),
        "golden LSB0 bit packing: {actual}"
    );
    let mut rx = Command::new(exe_b)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    rx.stdin
        .take()
        .unwrap()
        .write_all(b"T 10\nR 801 2 B001\nG 0\n")
        .unwrap();
    let received = String::from_utf8(rx.wait_with_output().unwrap().stdout).unwrap();
    assert!(
        received
            .lines()
            .any(|line| line.trim_end_matches('\r') == "V 0 54 1"),
        "independent received value: {received}"
    );
    let result = host::run(&out_a, &out_b).unwrap();
    assert!(result.passed, "host bus failed: {}", result.log);
    assert!(result.events.iter().any(|e| e.contains("BUS_OFF")));
    assert!(result.events.iter().any(|e| e.contains("DLC")));
}

#[cfg(windows)]
#[test]
fn global_ecuc_pdu_binding_roundtrips_and_rejects_wrong_com_reference_type() {
    let temp = Scratch::new();
    let source = temp.0.join("Closure/Closure.arxml");
    let mut project = Workspace::create(source.parent().unwrap(), "Closure", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 1, Direction::Tx, Some(10), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    project
        .add_signal(frame.clone(), "Value".into(), 0, 8, 7)
        .unwrap();
    project.save().unwrap();
    let saved = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&saved).unwrap();
    let global = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-CONTAINER-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("SHORT-NAME") && child.text() == Some("Pdu_Live")
                })
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text()
                            == Some("/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu")
                })
        })
        .unwrap();
    assert!(
        global
            .descendants()
            .any(|node| node.has_tag_name("VALUE") && node.text() == Some("1"))
    );
    assert!(
        !global
            .descendants()
            .any(|node| node.has_tag_name("DEFINITION-REF")
                && node
                    .text()
                    .is_some_and(|value| value.ends_with("/DynamicLength"))),
        "System Template constr_3448 excludes DynamicLength for system I-PDU bindings"
    );
    assert!(global.descendants().any(|node| node.has_tag_name("SDG")
        && node.attribute("GID") == Some("AutosarWorkbenchGlobalPduV1")
        && node.descendants().any(|child| child.has_tag_name("SD")
            && child.attribute("GID") == Some("SystemPduRef")
            && child.text() == Some(frame.as_str()))));
    let com_ref = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-REFERENCE-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|value| value.ends_with("/ComPduIdRef"))
                })
        })
        .unwrap();
    let reference = com_ref
        .children()
        .find(|node| node.has_tag_name("VALUE-REF"))
        .unwrap();
    assert_eq!(reference.attribute("DEST"), Some("ECUC-CONTAINER-VALUE"));
    assert_eq!(
        reference.text(),
        Some("/Closure/EcuCCfg/EcucConfigSet/Pdus/Pdu_Live")
    );
    let mut reopened = Workspace::open(vec![source.clone()], archive()).unwrap();
    assert!(reopened.validate().unwrap().issues.is_empty());
    let generated = temp.0.join("Generated");
    generator::generate(&mut reopened, &generated).unwrap();
    let binary = generator::build(&generated).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin.take().unwrap().write_all(b"T 10\n").unwrap();
    assert_eq!(
        String::from_utf8(ecu.wait_with_output().unwrap().stdout)
            .unwrap()
            .trim(),
        "X 801 1 07"
    );

    let wrong = saved.replacen(
        &saved[reference.range()],
        &saved[reference.range()].replace("ECUC-CONTAINER-VALUE", "I-SIGNAL-I-PDU"),
        1,
    );
    fs::write(&source, &wrong).unwrap();
    let mut unsupported = Workspace::open(vec![source.clone()], archive()).unwrap();
    let issue = unsupported
        .validate()
        .unwrap()
        .issues
        .into_iter()
        .find(|issue| issue.code == "PDU_UNSUPPORTED")
        .expect("wrong ECUC destination must block use");
    assert!(
        issue
            .file
            .as_deref()
            .is_some_and(|file| file.contains("Closure.arxml"))
    );
    assert!(
        issue
            .path
            .as_deref()
            .is_some_and(|path| path.contains("Pdu_Live"))
    );
    assert!(unsupported.save().unwrap_err().contains("PDU_UNSUPPORTED"));
    assert!(
        generator::generate(&mut unsupported, &temp.0.join("Unsafe"))
            .unwrap_err()
            .contains("PDU_UNSUPPORTED")
    );
    assert_eq!(fs::read_to_string(source).unwrap(), wrong);
}

#[test]
fn host_can_ecuc_closes_required_mod_fields_and_rejects_broken_links() {
    use roxmltree::Node;
    use std::collections::BTreeMap;

    fn field(node: Node<'_, '_>, name: &str) -> String {
        node.children()
            .find(|child| child.is_element() && child.tag_name().name() == name)
            .and_then(|child| child.text())
            .unwrap_or("")
            .to_owned()
    }
    fn path(node: Node<'_, '_>) -> String {
        let mut names: Vec<_> = node
            .ancestors()
            .filter_map(|parent| {
                let name = field(parent, "SHORT-NAME");
                if name.is_empty() { None } else { Some(name) }
            })
            .collect();
        names.reverse();
        format!("/{}", names.join("/"))
    }
    fn named<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
        node.children()
            .find(|child| child.is_element() && child.tag_name().name() == name)
    }

    let temp = Scratch::new();
    let mut project =
        Workspace::create(&temp.0.join("CanClosure"), "CanClosure", archive()).unwrap();
    let tx = project
        .add_frame("Transmit".into(), 0x321, 4, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(tx, "LiveValue".into(), 0, 32, 7)
        .unwrap()
        .signals[0]
        .path
        .clone();
    let rx = project
        .add_frame("Receive".into(), 0x456, 1, Direction::Rx, None, Some(50))
        .unwrap()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Receive")
        .unwrap()
        .path;
    project
        .add_signal(rx, "ReceivedValue".into(), 0, 8, 0)
        .unwrap();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("CanClosure/CanClosure.arxml");
    let xml = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();

    let mod_zip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_MOD_ECUConfigurationParameters.zip");
    let mut mod_archive = zip::ZipArchive::new(
        fs::File::open(mod_zip).expect("R24-11 ECUC MOD archive is required for this test"),
    )
    .unwrap();
    let mut mod_xml = String::new();
    mod_archive
        .by_name("AUTOSAR_CP_MOD_ECUConfigurationParameters.arxml")
        .unwrap()
        .read_to_string(&mut mod_xml)
        .unwrap();
    let mod_doc = roxmltree::Document::parse(&mod_xml).unwrap();
    let definitions: BTreeMap<_, _> = mod_doc
        .descendants()
        .filter(|node| {
            node.is_element()
                && (node.tag_name().name().ends_with("-PARAM-DEF")
                    || node.tag_name().name().ends_with("-REFERENCE-DEF")
                    || node.tag_name().name().ends_with("-CONTAINER-DEF")
                    || node.tag_name().name() == "ECUC-MODULE-DEF")
        })
        .map(|node| (path(node), node))
        .collect();
    let containers: BTreeMap<_, _> = doc
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "ECUC-CONTAINER-VALUE")
        .map(|node| (path(node), node))
        .collect();
    let modules: Vec<_> = doc
        .descendants()
        .filter(|node| {
            node.is_element()
                && node.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
                && matches!(
                    field(*node, "SHORT-NAME").as_str(),
                    "McuCfg" | "CanCfg" | "CanIfCfg"
                )
        })
        .collect();
    assert_eq!(
        modules.len(),
        3,
        "fixed host CAN needs Mcu, Can and CanIf configuration"
    );
    for module in modules {
        for value in module.descendants().filter(|node| {
            node.is_element()
                && matches!(
                    node.tag_name().name(),
                    "ECUC-MODULE-CONFIGURATION-VALUES" | "ECUC-CONTAINER-VALUE"
                )
        }) {
            let definition = field(value, "DEFINITION-REF");
            let spec = definitions
                .get(&definition)
                .unwrap_or_else(|| panic!("missing MOD definition {definition}"));
            for (spec_group, value_group) in [
                ("CONTAINERS", "CONTAINERS"),
                ("SUB-CONTAINERS", "SUB-CONTAINERS"),
                ("PARAMETERS", "PARAMETER-VALUES"),
                ("REFERENCES", "REFERENCE-VALUES"),
            ] {
                if let Some(group) = named(*spec, spec_group) {
                    for required in group.children().filter(|child| {
                        child.is_element() && field(*child, "LOWER-MULTIPLICITY") == "1"
                    }) {
                        let required_path = path(required);
                        let count = named(value, value_group)
                            .into_iter()
                            .flat_map(|group| group.children())
                            .filter(|child| {
                                child.is_element()
                                    && field(*child, "DEFINITION-REF") == required_path
                            })
                            .count();
                        assert_eq!(
                            count,
                            1,
                            "{} requires exactly one {}",
                            path(value),
                            required_path
                        );
                    }
                }
            }
        }
        for reference in module
            .descendants()
            .filter(|node| node.is_element() && node.tag_name().name() == "ECUC-REFERENCE-VALUE")
        {
            let definition = field(reference, "DEFINITION-REF");
            let spec = definitions
                .get(&definition)
                .unwrap_or_else(|| panic!("missing MOD reference {definition}"));
            let target = named(reference, "VALUE-REF").unwrap();
            assert_eq!(
                target.attribute("DEST"),
                Some("ECUC-CONTAINER-VALUE"),
                "{definition}"
            );
            let target_path = target.text().unwrap();
            let resolved = containers
                .get(target_path)
                .unwrap_or_else(|| panic!("unresolved {definition} -> {target_path}"));
            let target_def = field(*resolved, "DEFINITION-REF");
            let allowed = spec
                .descendants()
                .filter(|node| node.is_element() && node.tag_name().name() == "DESTINATION-REF")
                .any(|node| node.text() == Some(target_def.as_str()));
            assert!(allowed, "{definition} cannot point to {target_def}");
        }
        for parameter in module.descendants().filter(|node| {
            node.is_element()
                && matches!(
                    node.tag_name().name(),
                    "ECUC-NUMERICAL-PARAM-VALUE" | "ECUC-TEXTUAL-PARAM-VALUE"
                )
        }) {
            let definition = field(parameter, "DEFINITION-REF");
            let spec = definitions
                .get(&definition)
                .unwrap_or_else(|| panic!("missing MOD parameter {definition}"));
            let value = field(parameter, "VALUE");
            if let Some(literals) = named(*spec, "LITERALS") {
                assert!(
                    literals.children().any(
                        |literal| literal.is_element() && field(literal, "SHORT-NAME") == value
                    ),
                    "{definition} has unsupported value {value}"
                );
            }
            if spec.tag_name().name() == "ECUC-BOOLEAN-PARAM-DEF" {
                assert!(
                    matches!(value.as_str(), "true" | "false"),
                    "{definition} has invalid boolean {value}"
                );
            }
            if let Ok(number) = value.parse::<f64>() {
                if let Ok(minimum) = field(*spec, "MIN").parse::<f64>() {
                    assert!(number >= minimum, "{definition} is below the MOD minimum");
                }
                if let Ok(maximum) = field(*spec, "MAX").parse::<f64>() {
                    assert!(number <= maximum, "{definition} is above the MOD maximum");
                }
            }
        }
    }

    let mcu_module = doc
        .descendants()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
                && field(*node, "SHORT-NAME") == "McuCfg"
        })
        .unwrap();
    let split_source = temp.0.join("CanClosure/Clock.arxml");
    let split_first = format!(
        "{}{}",
        &xml[..mcu_module.range().start],
        &xml[mcu_module.range().end..]
    );
    let split_second = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>CanClosure</SHORT-NAME><ELEMENTS>{}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>",
        &xml[mcu_module.range()]
    );
    fs::write(&source, &split_first).unwrap();
    fs::write(&split_source, &split_second).unwrap();
    let mut split = Workspace::open(vec![source.clone(), split_source.clone()], archive()).unwrap();
    assert!(
        split.validate().unwrap().issues.is_empty(),
        "split Mcu clock must resolve across files"
    );
    generator::generate(&mut split, &temp.0.join("GeneratedSplit")).unwrap();
    split.save().unwrap();
    assert_eq!(fs::read_to_string(&source).unwrap(), split_first);
    assert_eq!(fs::read_to_string(&split_source).unwrap(), split_second);
    fs::write(&source, &xml).unwrap();

    let private = containers
        .get("/CanClosure/CanIfCfg/CanIfPrivateCfg")
        .unwrap();
    let removed_private = format!(
        "{}{}",
        &xml[..private.range().start],
        &xml[private.range().end..]
    );
    let broken_clock = xml.replacen(
        "/CanClosure/McuCfg/McuModuleConfiguration/HostClockSetting/HostClock</VALUE-REF>",
        "/CanClosure/McuCfg/McuModuleConfiguration/HostClockSetting/MissingClock</VALUE-REF>",
        1,
    );
    let buffer_ref = doc
        .descendants()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "ECUC-REFERENCE-VALUE"
                && field(*node, "DEFINITION-REF").ends_with("/CanIfTxPduBufferRef")
        })
        .unwrap();
    let removed_buffer_ref = format!(
        "{}{}",
        &xml[..buffer_ref.range().start],
        &xml[buffer_ref.range().end..]
    );
    let wrong_hoh = xml.replacen(
        "/CanClosure/CanCfg/CanConfigSet/HostRxObject</VALUE-REF>",
        "/CanClosure/CanCfg/CanConfigSet/HostTxObject</VALUE-REF>",
        1,
    );
    for (name, modified) in [
        ("missing-private", removed_private),
        ("clock-ref", broken_clock),
        ("missing-buffer-ref", removed_buffer_ref),
        ("wrong-hoh", wrong_hoh),
    ] {
        assert_ne!(modified, xml, "{name} must change the fixture");
        fs::write(&source, &modified).unwrap();
        let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
        assert!(
            imported
                .validate()
                .unwrap()
                .issues
                .iter()
                .any(|issue| issue.code == "PDU_UNSUPPORTED"),
            "{name}"
        );
        assert!(imported.view().files[0].readonly, "{name}");
        assert!(imported.save().is_err(), "{name}");
        assert!(
            generator::generate(&mut imported, &temp.0.join(format!("Unsafe{name}"))).is_err(),
            "{name}"
        );
        assert_eq!(fs::read_to_string(&source).unwrap(), modified, "{name}");
    }
}

#[test]
fn missing_required_com_or_ecuc_root_is_read_only_and_cannot_generate() {
    let temp = Scratch::new();
    let source = temp.0.join("Closure/Closure.arxml");
    let mut project = Workspace::create(source.parent().unwrap(), "Closure", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 1, Direction::Tx, Some(10), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    project.add_signal(frame, "Value".into(), 0, 8, 7).unwrap();
    project.save().unwrap();
    let xml = fs::read_to_string(&source).unwrap();
    for name in ["ComGeneral", "Hardware"] {
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let required = doc
            .descendants()
            .find(|node| {
                node.has_tag_name("ECUC-CONTAINER-VALUE")
                    && node
                        .children()
                        .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some(name))
            })
            .unwrap();
        let modified = format!(
            "{}{}",
            &xml[..required.range().start],
            &xml[required.range().end..]
        );
        fs::write(&source, &modified).unwrap();
        let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
        let issue = imported
            .validate()
            .unwrap()
            .issues
            .into_iter()
            .find(|issue| issue.code == "PDU_UNSUPPORTED")
            .unwrap_or_else(|| panic!("missing {name} must block generation"));
        assert!(
            issue
                .file
                .as_deref()
                .is_some_and(|file| file.contains("Closure.arxml"))
        );
        assert!(imported.save().unwrap_err().contains("PDU_UNSUPPORTED"));
        assert!(
            generator::generate(&mut imported, &temp.0.join(format!("Unsafe{name}")))
                .unwrap_err()
                .contains("PDU_UNSUPPORTED")
        );
        assert_eq!(fs::read_to_string(&source).unwrap(), modified);
    }
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let module = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-MODULE-CONFIGURATION-VALUES")
                && node.children().any(|child| {
                    child.has_tag_name("SHORT-NAME") && child.text() == Some("EcuCCfg")
                })
        })
        .unwrap();
    let insertion = module.range().end - "</CONTAINERS></ECUC-MODULE-CONFIGURATION-VALUES>".len();
    let ecuc_extra = format!(
        "{}<ECUC-CONTAINER-VALUE><SHORT-NAME>Partitions</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">/AUTOSAR/EcucDefs/EcuC/EcucPartitionCollection</DEFINITION-REF></ECUC-CONTAINER-VALUE>{}",
        &xml[..insertion],
        &xml[insertion..]
    );
    let config = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-CONTAINER-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text() == Some("/AUTOSAR/EcucDefs/Com/ComConfig")
                })
        })
        .unwrap();
    let insertion = config.range().end - "</SUB-CONTAINERS></ECUC-CONTAINER-VALUE>".len();
    let com_extra = format!(
        "{}<ECUC-CONTAINER-VALUE><SHORT-NAME>Groups</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">/AUTOSAR/EcucDefs/Com/ComConfig/ComIPduGroup</DEFINITION-REF></ECUC-CONTAINER-VALUE>{}",
        &xml[..insertion],
        &xml[insertion..]
    );
    for (name, modified) in [("Partitions", ecuc_extra), ("Groups", com_extra)] {
        fs::write(&source, &modified).unwrap();
        let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
        let issue = imported
            .validate()
            .unwrap()
            .issues
            .into_iter()
            .find(|issue| issue.code == "PDU_UNSUPPORTED")
            .unwrap_or_else(|| panic!("unknown {name} must be read-only"));
        assert!(
            issue
                .file
                .as_deref()
                .is_some_and(|file| file.contains("Closure.arxml"))
        );
        assert!(imported.save().unwrap_err().contains("PDU_UNSUPPORTED"));
        assert!(
            generator::generate(&mut imported, &temp.0.join(format!("Unsafe{name}")))
                .unwrap_err()
                .contains("PDU_UNSUPPORTED")
        );
        assert_eq!(fs::read_to_string(&source).unwrap(), modified);
    }
    for name in ["ComCfg", "EcuCCfg"] {
        let module = doc
            .descendants()
            .find(|node| {
                node.has_tag_name("ECUC-MODULE-CONFIGURATION-VALUES")
                    && node
                        .children()
                        .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some(name))
            })
            .unwrap();
        let definition = module
            .children()
            .find(|child| child.has_tag_name("DEFINITION-REF"))
            .unwrap();
        let modified = format!(
            "{}<DEFINITION-REF DEST=\"ECUC-MODULE-DEF\">/AUTOSAR/EcucDefs/CanIf</DEFINITION-REF>{}",
            &xml[..definition.range().start],
            &xml[definition.range().end..]
        );
        fs::write(&source, &modified).unwrap();
        let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
        let issue = imported
            .validate()
            .unwrap()
            .issues
            .into_iter()
            .find(|issue| issue.code == "PDU_UNSUPPORTED")
            .unwrap_or_else(|| panic!("{name} parent definition must block use"));
        assert_eq!(
            issue.path.as_deref(),
            Some(format!("/Closure/{name}").as_str())
        );
        assert!(
            issue
                .file
                .as_deref()
                .is_some_and(|file| file.contains("Closure.arxml"))
        );
        assert!(imported.save().unwrap_err().contains("PDU_UNSUPPORTED"));
        assert!(
            generator::generate(&mut imported, &temp.0.join(format!("Unsafe{name}")))
                .unwrap_err()
                .contains("PDU_UNSUPPORTED")
        );
        assert_eq!(fs::read_to_string(&source).unwrap(), modified);
    }
    let renamed = xml
        .replacen(
            "<SHORT-NAME>ComCfg</SHORT-NAME>",
            "<SHORT-NAME>AltCom</SHORT-NAME>",
            1,
        )
        .replace("/Closure/ComCfg/", "/Closure/AltCom/");
    let renamed_doc = roxmltree::Document::parse(&renamed).unwrap();
    let general = renamed_doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-CONTAINER-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("SHORT-NAME") && child.text() == Some("ComGeneral")
                })
        })
        .unwrap();
    let missing_general = format!(
        "{}{}",
        &renamed[..general.range().start],
        &renamed[general.range().end..]
    );
    let module = renamed_doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-MODULE-CONFIGURATION-VALUES")
                && node
                    .children()
                    .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some("AltCom"))
        })
        .unwrap();
    let definition = module
        .children()
        .find(|child| child.has_tag_name("DEFINITION-REF"))
        .unwrap();
    let foreign_parent = format!(
        "{}<DEFINITION-REF DEST=\"ECUC-MODULE-DEF\">/AUTOSAR/EcucDefs/CanIf</DEFINITION-REF>{}",
        &renamed[..definition.range().start],
        &renamed[definition.range().end..]
    );
    for (name, modified) in [
        ("Renamed", renamed),
        ("MissingGeneral", missing_general),
        ("ForeignParent", foreign_parent),
    ] {
        fs::write(&source, &modified).unwrap();
        let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
        assert_eq!(
            imported.view().frames.len(),
            1,
            "the altered ComIPdu remains consumed"
        );
        let issue = imported
            .validate()
            .unwrap()
            .issues
            .into_iter()
            .find(|issue| issue.code == "PDU_UNSUPPORTED")
            .unwrap_or_else(|| panic!("{name} Com owner must be rejected"));
        assert_eq!(issue.path.as_deref(), Some("/Closure/AltCom"));
        assert!(
            issue
                .file
                .as_deref()
                .is_some_and(|file| file.contains("Closure.arxml"))
        );
        assert!(imported.save().unwrap_err().contains("PDU_UNSUPPORTED"));
        assert!(
            generator::generate(&mut imported, &temp.0.join(format!("Unsafe{name}")))
                .unwrap_err()
                .contains("PDU_UNSUPPORTED")
        );
        assert_eq!(fs::read_to_string(&source).unwrap(), modified);
    }
}

#[test]
fn multiple_consumed_com_modules_cannot_generate() {
    let temp = Scratch::new();
    let source = temp.0.join("Closure/Closure.arxml");
    let mut project = Workspace::create(source.parent().unwrap(), "Closure", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 1, Direction::Tx, Some(10), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    project.add_signal(frame, "Value".into(), 0, 8, 7).unwrap();
    project.save().unwrap();
    let xml = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let module = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-MODULE-CONFIGURATION-VALUES")
                && node
                    .children()
                    .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some("ComCfg"))
        })
        .unwrap();
    let duplicate = xml[module.range()]
        .replacen(
            "<SHORT-NAME>ComCfg</SHORT-NAME>",
            "<SHORT-NAME>AltCom</SHORT-NAME>",
            1,
        )
        .replace("/Closure/ComCfg/", "/Closure/AltCom/");
    let insertion = xml.find("</ELEMENTS>").unwrap();
    let modified = format!("{}{}{}", &xml[..insertion], duplicate, &xml[insertion..]);
    fs::write(&source, &modified).unwrap();
    let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
    let issue = imported
        .validate()
        .unwrap()
        .issues
        .into_iter()
        .find(|issue| issue.code == "PDU_UNSUPPORTED" && issue.message.contains("唯一"))
        .expect("two consumed Com modules must not produce a host profile");
    assert_eq!(issue.path.as_deref(), Some("/Closure/AltCom"));
    assert!(
        issue
            .file
            .as_deref()
            .is_some_and(|file| file.contains("Closure.arxml"))
    );
    assert!(imported.save().is_err());
    let output = temp.0.join("UnsafeDuplicateComOwner");
    assert!(generator::generate(&mut imported, &output).is_err());
    assert!(!output.exists());
    assert_eq!(fs::read_to_string(source).unwrap(), modified);
}

#[test]
fn imported_global_pdu_cannot_duplicate_system_binding_or_misstate_diagnostic_length() {
    let temp = Scratch::new();
    let source = temp.0.join("Diag/Diag.arxml");
    let mut project = Workspace::create(source.parent().unwrap(), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 4, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(frame, "Value".into(), 0, 32, 7)
        .unwrap()
        .signals[0]
        .path
        .clone();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: true,
            reset_routine_id: Some(0xf001),
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let xml = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let find_pdu = |name| {
        doc.descendants()
            .find(|node| {
                node.has_tag_name("ECUC-CONTAINER-VALUE")
                    && node
                        .children()
                        .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some(name))
                    && node.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF")
                            && child.text()
                                == Some(
                                    "/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu",
                                )
                    })
            })
            .unwrap()
    };
    let live = find_pdu("Pdu_Live");
    let shadow = xml[live.range()].replacen(
        "<SHORT-NAME>Pdu_Live</SHORT-NAME>",
        "<SHORT-NAME>Pdu_Shadow</SHORT-NAME>",
        1,
    );
    let duplicate = format!(
        "{}{}{}",
        &xml[..live.range().end],
        shadow,
        &xml[live.range().end..]
    );
    let diagnostic = find_pdu("DcmPdu_DiagRequest");
    let short = &xml[diagnostic.range()];
    let wrong_length = format!(
        "{}{}{}",
        &xml[..diagnostic.range().start],
        short.replacen("<VALUE>256</VALUE>", "<VALUE>8</VALUE>", 1),
        &xml[diagnostic.range().end..]
    );
    let request = find_pdu("NPdu_DiagRequest");
    let missing = format!(
        "{}{}",
        &xml[..request.range().start],
        &xml[request.range().end..]
    );
    let insertion = live.range().end - "</ECUC-CONTAINER-VALUE>".len();
    let dependent = format!(
        "{}<REFERENCE-VALUES><ECUC-REFERENCE-VALUE><DEFINITION-REF DEST=\"ECUC-REFERENCE-DEF\">/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu/PduTriggeredByRef</DEFINITION-REF><VALUE-REF DEST=\"PDU-TRIGGERING\">/Diag/UnknownTrigger</VALUE-REF></ECUC-REFERENCE-VALUE></REFERENCE-VALUES>{}",
        &xml[..insertion],
        &xml[insertion..]
    );
    let insertion = live.range().start + xml[live.range()].find("</PARAMETER-VALUES>").unwrap();
    let dynamic = format!(
        "{}<ECUC-NUMERICAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-BOOLEAN-PARAM-DEF\">/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu/DynamicLength</DEFINITION-REF><VALUE>false</VALUE></ECUC-NUMERICAL-PARAM-VALUE>{}",
        &xml[..insertion],
        &xml[insertion..]
    );
    for (name, modified) in [
        ("duplicate", duplicate),
        ("length", wrong_length),
        ("missing", missing),
        ("dependent", dependent),
        ("dynamic", dynamic),
    ] {
        fs::write(&source, &modified).unwrap();
        let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
        let issue = imported
            .validate()
            .unwrap()
            .issues
            .into_iter()
            .find(|issue| issue.code == "PDU_UNSUPPORTED")
            .unwrap_or_else(|| {
                panic!("{name} must not generate an ECU from ambiguous PDU bindings")
            });
        assert!(
            issue
                .file
                .as_deref()
                .is_some_and(|file| file.contains("Diag.arxml"))
        );
        assert!(imported.save().unwrap_err().contains("PDU_UNSUPPORTED"));
        assert!(
            generator::generate(&mut imported, &temp.0.join(format!("Unsafe{name}")))
                .unwrap_err()
                .contains("PDU_UNSUPPORTED")
        );
        assert_eq!(fs::read_to_string(&source).unwrap(), modified);
    }
}

#[test]
fn diagnostic_ecuc_refs_reject_old_system_destinations_and_dynamic_npdu() {
    let temp = Scratch::new();
    let source = temp.0.join("Diag/Diag.arxml");
    let mut project = Workspace::create(source.parent().unwrap(), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 4, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(frame, "Value".into(), 0, 32, 7)
        .unwrap()
        .signals[0]
        .path
        .clone();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let xml = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    for (name, system, old_dest) in [
        ("CanTpTxNPduRef", "/Diag/NPdu_DiagResponse", "N-PDU"),
        (
            "DcmDslProtocolRxPduRef",
            "/Diag/DcmPdu_DiagRequest",
            "DCM-I-PDU",
        ),
    ] {
        let reference = doc
            .descendants()
            .find(|node| {
                node.has_tag_name("ECUC-REFERENCE-VALUE")
                    && node.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF")
                            && child
                                .text()
                                .is_some_and(|value| value.ends_with(&format!("/{name}")))
                    })
            })
            .unwrap();
        let value = reference
            .children()
            .find(|child| child.has_tag_name("VALUE-REF"))
            .unwrap();
        assert_eq!(value.attribute("DEST"), Some("ECUC-CONTAINER-VALUE"));
        let modified = format!(
            "{}<VALUE-REF DEST=\"{old_dest}\">{system}</VALUE-REF>{}",
            &xml[..value.range().start],
            &xml[value.range().end..]
        );
        fs::write(&source, &modified).unwrap();
        let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
        assert!(
            imported
                .validate()
                .unwrap()
                .issues
                .iter()
                .any(|issue| issue.code == "DIAG_UNSUPPORTED"),
            "{name} must not silently bind a system PDU"
        );
        assert!(imported.save().is_err());
        assert!(generator::generate(&mut imported, &temp.0.join(format!("Unsafe{name}"))).is_err());
        assert_eq!(fs::read_to_string(&source).unwrap(), modified);
    }
    let n_pdu = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("N-PDU")
                && node.children().any(|child| {
                    child.has_tag_name("SHORT-NAME") && child.text() == Some("NPdu_DiagRequest")
                })
        })
        .unwrap();
    let insertion = n_pdu.range().start
        + xml[n_pdu.range()].find("</SHORT-NAME>").unwrap()
        + "</SHORT-NAME>".len();
    let modified = format!(
        "{}<HAS-DYNAMIC-LENGTH>true</HAS-DYNAMIC-LENGTH>{}",
        &xml[..insertion],
        &xml[insertion..]
    );
    fs::write(&source, &modified).unwrap();
    let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
    assert!(
        imported
            .validate()
            .unwrap()
            .issues
            .iter()
            .any(|issue| issue.code == "DIAG_UNSUPPORTED")
    );
    assert!(imported.save().is_err());
    assert!(generator::generate(&mut imported, &temp.0.join("UnsafeDynamicNPdu")).is_err());
    assert_eq!(fs::read_to_string(source).unwrap(), modified);
}

#[cfg(windows)]
#[test]
fn host_rejects_id_matched_wrong_dlc_even_when_other_frames_exchange() {
    let temp = Scratch::new();
    let (mut a, mut b) = create_pair(&temp.0);
    let tx = a
        .add_frame("Mismatch".into(), 0x600, 2, Direction::Tx, Some(10), None)
        .unwrap()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Mismatch")
        .unwrap()
        .path;
    a.add_signal(tx, "ExtraTx".into(), 0, 8, 1).unwrap();
    let rx = b
        .add_frame("Mismatch".into(), 0x600, 1, Direction::Rx, None, Some(50))
        .unwrap()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Mismatch")
        .unwrap()
        .path;
    b.add_signal(rx, "ExtraRx".into(), 0, 8, 0).unwrap();
    a.save().unwrap();
    b.save().unwrap();
    let out_a = temp.0.join("GeneratedAlpha");
    let out_b = temp.0.join("GeneratedBeta");
    generator::generate(&mut a, &out_a).unwrap();
    generator::generate(&mut b, &out_b).unwrap();
    generator::build(&out_a).unwrap();
    generator::build(&out_b).unwrap();
    let result = host::run(&out_a, &out_b).unwrap();
    assert!(
        !result.passed && result.log.contains("FRAME_DLC"),
        "ID-matched DLC mismatch was ignored: {}",
        result.log
    );
}

#[test]
fn imported_unknown_content_survives_supported_edit_without_rewriting_other_file() {
    let temp = Scratch::new();
    let (a, _) = create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    let retained =
        "<I-SIGNAL><SHORT-NAME>RetainedUnknown</SHORT-NAME><LENGTH>1</LENGTH></I-SIGNAL>";
    fs::write(
        &source,
        text.replacen("</ELEMENTS>", &format!("{retained}</ELEMENTS>"), 1),
    )
    .unwrap();
    let other = temp.0.join("Unrelated.arxml");
    let unrelated = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Other</SHORT-NAME><ELEMENTS><I-SIGNAL><SHORT-NAME>Extra</SHORT-NAME><LENGTH>1</LENGTH></I-SIGNAL></ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>";
    fs::write(&other, unrelated).unwrap();
    let mut imported = Workspace::open(vec![source.clone(), other.clone()], archive()).unwrap();
    assert_eq!(imported.view().files.len(), 2);
    let frame = imported
        .view()
        .frames
        .into_iter()
        .find(|f| f.name == "Command")
        .unwrap();
    imported
        .update_frame(&frame.path, serde_json::json!({"id": 802, "dlc": 3}))
        .unwrap();
    let original_source = fs::read_to_string(&source).unwrap();
    let preview = imported.preview_save().unwrap();
    assert_eq!(preview.files.len(), 2);
    let changed_path = fs::canonicalize(&source).unwrap().display().to_string();
    let changed = preview
        .files
        .iter()
        .find(|file| file.path == changed_path)
        .unwrap();
    assert!(changed.changed);
    assert_eq!(changed.before.as_deref(), Some(original_source.as_str()));
    assert!(changed.after.as_deref().unwrap().contains(retained));
    assert_eq!(
        fs::read_to_string(&source).unwrap(),
        original_source,
        "preview must not write ARXML"
    );
    let unchanged_path = fs::canonicalize(&other).unwrap().display().to_string();
    let unchanged = preview
        .files
        .iter()
        .find(|file| file.path == unchanged_path)
        .unwrap();
    assert!(!unchanged.changed);
    assert!(unchanged.before.is_none() && unchanged.after.is_none());
    imported.save_previewed(&preview.revision).unwrap();
    assert_eq!(
        fs::read_to_string(&source).unwrap(),
        changed.after.as_deref().unwrap()
    );
    let reopened = Workspace::open(vec![source.clone()], archive()).unwrap();
    let updated = reopened
        .view()
        .frames
        .into_iter()
        .find(|item| item.path == frame.path)
        .unwrap();
    assert_eq!(
        (updated.id, updated.dlc),
        (802, 3),
        "imported frame must retain ID and global PDU length"
    );
    assert!(fs::read_to_string(&source).unwrap().contains(retained));
    assert_eq!(
        unrelated,
        fs::read_to_string(other).unwrap(),
        "unmodified ARXML must stay byte-identical"
    );
    assert_eq!(a.view().frames.len(), 2);
}

#[test]
fn save_does_not_overwrite_external_changes_to_managed_arxml() {
    let temp = Scratch::new();
    let (mut project, _) = create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let frame = project
        .view()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Command")
        .unwrap();
    project
        .update_frame(&frame.path, serde_json::json!({"id": 802}))
        .unwrap();
    let original = fs::read_to_string(&source).unwrap();
    let external = original.replacen("<VALUE>801</VALUE>", "<VALUE>803</VALUE>", 1);
    assert_ne!(original, external);
    fs::write(&source, &external).unwrap();
    assert!(project.save().unwrap_err().contains("外部修改"));
    assert_eq!(fs::read_to_string(&source).unwrap(), external);
    assert!(project.view().dirty);
    assert_eq!(
        fs::read_dir(source.parent().unwrap()).unwrap().count(),
        1,
        "failed save left staging files"
    );
}

#[test]
fn save_preview_rejects_edits_and_external_changes_after_preview() {
    let temp = Scratch::new();
    let (mut project, _) = create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let frame = project
        .view()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Command")
        .unwrap();
    project
        .update_frame(&frame.path, serde_json::json!({"id": 802}))
        .unwrap();
    let first = project.preview_save().unwrap();
    project
        .update_frame(&frame.path, serde_json::json!({"id": 804}))
        .unwrap();
    assert!(
        project
            .save_previewed(&first.revision)
            .unwrap_err()
            .contains("重新查看")
    );
    assert!(
        fs::read_to_string(&source)
            .unwrap()
            .contains("<VALUE>801</VALUE>")
    );

    let second = project.preview_save().unwrap();
    let external = fs::read_to_string(&source).unwrap().replacen(
        "<VALUE>801</VALUE>",
        "<VALUE>803</VALUE>",
        1,
    );
    fs::write(&source, &external).unwrap();
    assert!(
        project
            .save_previewed(&second.revision)
            .unwrap_err()
            .contains("外部修改")
    );
    assert!(project.preview_save().unwrap_err().contains("外部修改"));
    assert_eq!(fs::read_to_string(&source).unwrap(), external);
}

#[test]
fn split_package_save_preserves_sources_and_rejects_stale_reference_file() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let other = temp.0.join("Alpha/Signals.arxml");
    let original = fs::read_to_string(&source).unwrap();
    let start = original
        .find("<I-SIGNAL><SHORT-NAME>ISignal_SendCount</SHORT-NAME>")
        .unwrap();
    let end = start + original[start..].find("</I-SIGNAL>").unwrap() + "</I-SIGNAL>".len();
    let signal = &original[start..end];
    fs::write(&source, original.replacen(signal, "", 1)).unwrap();
    fs::write(&other, format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Alpha</SHORT-NAME><ELEMENTS>{signal}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>\n")).unwrap();

    let mut project = Workspace::open(vec![source.clone(), other.clone()], archive()).unwrap();
    assert!(project.validate().unwrap().issues.is_empty());
    let frame = project
        .view()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Command")
        .unwrap();
    let untouched = fs::read(&other).unwrap();
    project
        .update_frame(&frame.path, serde_json::json!({"id": 802}))
        .unwrap();
    project.save().unwrap();
    assert_eq!(
        fs::read(&other).unwrap(),
        untouched,
        "supported edit must leave the other file byte-identical"
    );
    let mut project = Workspace::open(vec![source.clone(), other.clone()], archive()).unwrap();
    assert!(project.validate().unwrap().issues.is_empty());
    assert_eq!(
        project
            .view()
            .frames
            .iter()
            .find(|item| item.name == "Command")
            .unwrap()
            .id,
        802
    );
    assert!(!project.view().dirty);
    let external =
        fs::read_to_string(&other)
            .unwrap()
            .replacen("<LENGTH>8</LENGTH>", "<LENGTH>7</LENGTH>", 1);
    fs::write(&other, &external).unwrap();
    assert!(project.validate().unwrap_err().contains("外部修改"));
    let unsafe_output = temp.0.join("StaleGeneration");
    assert!(
        generator::generate(&mut project, &unsafe_output)
            .unwrap_err()
            .contains("外部修改")
    );
    assert!(
        !unsafe_output.exists(),
        "stale sources must not materialize C99 output"
    );
    project
        .update_frame(&frame.path, serde_json::json!({"id": 803}))
        .unwrap();
    let before_save = fs::read(&source).unwrap();

    assert!(project.save().unwrap_err().contains("外部修改"));
    assert_eq!(
        fs::read(&source).unwrap(),
        before_save,
        "dirty source must not be partly committed"
    );
    assert_eq!(
        fs::read_to_string(&other).unwrap(),
        external,
        "external source must be preserved"
    );
    assert!(project.view().dirty);
}

#[test]
fn three_file_host_can_edit_preserves_retained_and_untouched_sources() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let clock = temp.0.join("Alpha/Clock.arxml");
    let signals = temp.0.join("Alpha/Signals.arxml");
    let original = fs::read_to_string(&source).unwrap();
    let signal_start = original
        .find("<I-SIGNAL><SHORT-NAME>ISignal_SendCount</SHORT-NAME>")
        .unwrap();
    let signal_end =
        signal_start + original[signal_start..].find("</I-SIGNAL>").unwrap() + "</I-SIGNAL>".len();
    let system_signal = &original[signal_start..signal_end];
    let without_signal = original.replacen(system_signal, "", 1);
    let clock_start = without_signal
        .find("<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>McuCfg</SHORT-NAME>")
        .unwrap();
    let clock_end = clock_start
        + without_signal[clock_start..]
            .find("</ECUC-MODULE-CONFIGURATION-VALUES>")
            .unwrap()
        + "</ECUC-MODULE-CONFIGURATION-VALUES>".len();
    let mcu = &without_signal[clock_start..clock_end];
    fs::write(&source, without_signal.replacen(mcu, "", 1)).unwrap();
    let wrap = |content: &str| {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Alpha</SHORT-NAME><ELEMENTS>{content}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>\n"
        )
    };
    fs::write(&clock, wrap(mcu)).unwrap();
    let retained =
        "<I-SIGNAL><SHORT-NAME>RetainedUnknown</SHORT-NAME><LENGTH>1</LENGTH></I-SIGNAL>";
    fs::write(&signals, wrap(&(system_signal.to_owned() + retained))).unwrap();

    let mut project = Workspace::open(
        vec![source.clone(), clock.clone(), signals.clone()],
        archive(),
    )
    .unwrap();
    assert!(project.validate().unwrap().issues.is_empty());
    assert_eq!(
        project
            .view()
            .files
            .iter()
            .map(|file| file.retained_count)
            .sum::<usize>(),
        1
    );
    let unchanged_clock = fs::read(&clock).unwrap();
    let signal = project
        .view()
        .signals
        .into_iter()
        .find(|signal| signal.name == "SendCount")
        .unwrap();
    project
        .update_signal(&signal.path, serde_json::json!({"length": 7}))
        .unwrap();
    project.save().unwrap();
    assert_eq!(fs::read(&clock).unwrap(), unchanged_clock);
    assert!(fs::read_to_string(&signals).unwrap().contains(retained));
    let mut reopened = Workspace::open(vec![source, clock, signals], archive()).unwrap();
    assert!(reopened.validate().unwrap().issues.is_empty());
    assert_eq!(
        reopened
            .view()
            .signals
            .into_iter()
            .find(|item| item.name == "SendCount")
            .unwrap()
            .length,
        7
    );
}

#[test]
fn official_r24_sample_imports_as_one_split_package_without_rewriting_sources() {
    let temp = Scratch::new();
    let zip_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_EXP_ModelingShowCases.zip",
    );
    let mut zip = zip::ZipArchive::new(fs::File::open(zip_path).unwrap()).unwrap();
    let prefix =
        "AUTOSAR_CP_EXP_ModelingShowCases/30_MeasurementCalibration/10_Introductory/model/";
    let mut paths = Vec::new();
    let mut originals = Vec::new();
    for index in 0..zip.len() {
        let mut member = zip.by_index(index).unwrap();
        if !member.name().starts_with(prefix) || !member.name().ends_with(".arxml") {
            continue;
        }
        let name = Path::new(member.name()).file_name().unwrap();
        let path = temp.0.join(name);
        let mut content = Vec::new();
        member.read_to_end(&mut content).unwrap();
        fs::write(&path, &content).unwrap();
        originals.push((path.clone(), content));
        paths.push(path);
    }
    assert_eq!(
        paths.len(),
        15,
        "the official R24 example must have all model files"
    );
    let mut project = Workspace::open(paths, archive()).unwrap();
    assert!(
        project.validate().unwrap().issues.is_empty(),
        "split AR-PACKAGE paths may merge across files"
    );
    assert!(!project.view().dirty);
    project.save().unwrap();
    for (path, content) in originals {
        assert_eq!(content, fs::read(path).unwrap());
    }
    assert!(
        generator::generate(&mut project, &temp.0.join("Generated")).is_err(),
        "unconfigured CAN profile must not produce a misleading ECU"
    );
}

#[test]
fn unresolved_r24_variant_is_preserved_but_blocks_generation() {
    let temp = Scratch::new();
    let (a, _) = create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    let anchor = "</START-POSITION></I-SIGNAL-TO-I-PDU-MAPPING>";
    let variant = "<VARIATION-POINT><SHORT-LABEL>UnboundVariant</SHORT-LABEL></VARIATION-POINT>";
    assert!(text.contains(anchor));
    fs::write(
        &source,
        text.replacen(
            anchor,
            &format!("</START-POSITION>{variant}</I-SIGNAL-TO-I-PDU-MAPPING>"),
            1,
        ),
    )
    .unwrap();
    let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
    let checked = imported.validate().unwrap();
    assert!(
        checked
            .issues
            .iter()
            .any(|issue| issue.code == "VARIANT_DEPENDENCY")
    );
    imported.save().unwrap();
    assert!(fs::read_to_string(source).unwrap().contains(variant));
    let generated = temp.0.join("UnsafeOutput");
    assert!(generator::generate(&mut imported, &generated).is_err());
    assert!(
        !generated.exists(),
        "variant-dependent C99 output must not be materialized"
    );
    assert_eq!(a.view().frames.len(), 2);
}

#[test]
fn package_variant_affecting_profile_blocks_generation() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    let variant = "<VARIATION-POINT><SHORT-LABEL>UnboundPackage</SHORT-LABEL></VARIATION-POINT>";
    fs::write(
        &source,
        text.replacen("</AR-PACKAGE>", &format!("{variant}</AR-PACKAGE>"), 1),
    )
    .unwrap();
    let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
    let checked = imported.validate().unwrap();
    assert!(
        checked
            .issues
            .iter()
            .any(|issue| issue.code == "VARIANT_DEPENDENCY"),
        "{:?}",
        checked.issues
    );
    let generated = temp.0.join("UnsafePackageOutput");
    assert!(generator::generate(&mut imported, &generated).is_err());
    assert!(!generated.exists());
}

#[test]
fn conflicting_canif_entries_for_one_pdu_block_generation() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    let start = text
        .find("<ECUC-CONTAINER-VALUE><SHORT-NAME>Can_Command</SHORT-NAME>")
        .unwrap();
    let end = start
        + text[start..].find("</ECUC-CONTAINER-VALUE>").unwrap()
        + "</ECUC-CONTAINER-VALUE>".len();
    let duplicate = text[start..end]
        .replace(
            "<SHORT-NAME>Can_Command</SHORT-NAME>",
            "<SHORT-NAME>Can_Conflict</SHORT-NAME>",
        )
        .replace("<VALUE>801</VALUE>", "<VALUE>1536</VALUE>");
    assert!(duplicate.contains("<VALUE>1536</VALUE>"));
    fs::write(
        &source,
        format!("{}{}{}", &text[..end], duplicate, &text[end..]),
    )
    .unwrap();
    let mut imported = Workspace::open(vec![source], archive()).unwrap();
    let checked = imported.validate().unwrap();
    assert!(
        checked
            .issues
            .iter()
            .any(|issue| issue.code == "CANIF_PDU_DUPLICATE"),
        "{:?}",
        checked.issues
    );
    let generated = temp.0.join("UnsafeCanIfOutput");
    assert!(generator::generate(&mut imported, &generated).is_err());
    assert!(!generated.exists());
}

#[test]
fn ipdu_mapping_disagreement_with_com_blocks_generation() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    for (field, original, changed) in [
        (
            "byte order",
            "<PACKING-BYTE-ORDER>MOST-SIGNIFICANT-BYTE-LAST</PACKING-BYTE-ORDER>",
            "<PACKING-BYTE-ORDER>MOST-SIGNIFICANT-BYTE-FIRST</PACKING-BYTE-ORDER>",
        ),
        (
            "start position",
            "<START-POSITION>3</START-POSITION>",
            "<START-POSITION>4</START-POSITION>",
        ),
    ] {
        assert!(text.contains(original), "missing {field} fixture");
        fs::write(&source, text.replacen(original, changed, 1)).unwrap();
        let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
        let checked = imported.validate().unwrap();
        assert!(
            checked
                .issues
                .iter()
                .any(|issue| issue.code == "PDU_MAPPING"),
            "{field}: {:?}",
            checked.issues
        );
        let generated = temp.0.join(format!("Unsafe{field}Output"));
        assert!(
            generator::generate(&mut imported, &generated).is_err(),
            "{field} generated incompatible C99"
        );
        assert!(!generated.exists());
    }
}

#[test]
fn linked_can_frame_must_match_canif_and_pdu_layout() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    let topology = r#"
<CAN-FRAME><SHORT-NAME>CommandFrame</SHORT-NAME><FRAME-LENGTH>2</FRAME-LENGTH>
  <PDU-TO-FRAME-MAPPINGS><PDU-TO-FRAME-MAPPING><SHORT-NAME>CommandMapping</SHORT-NAME>
    <PACKING-BYTE-ORDER>MOST-SIGNIFICANT-BYTE-LAST</PACKING-BYTE-ORDER>
    <PDU-REF DEST="I-SIGNAL-I-PDU">/Alpha/Pdu_Command</PDU-REF><START-POSITION>0</START-POSITION>
  </PDU-TO-FRAME-MAPPING></PDU-TO-FRAME-MAPPINGS></CAN-FRAME>
<CAN-CLUSTER><SHORT-NAME>Network</SHORT-NAME><CAN-CLUSTER-VARIANTS><CAN-CLUSTER-CONDITIONAL>
  <PHYSICAL-CHANNELS><CAN-PHYSICAL-CHANNEL><SHORT-NAME>Bus</SHORT-NAME>
    <FRAME-TRIGGERINGS><CAN-FRAME-TRIGGERING><SHORT-NAME>CommandOnBus</SHORT-NAME>
      <FRAME-REF DEST="CAN-FRAME">/Alpha/CommandFrame</FRAME-REF><IDENTIFIER>801</IDENTIFIER>
    </CAN-FRAME-TRIGGERING></FRAME-TRIGGERINGS>
  </CAN-PHYSICAL-CHANNEL></PHYSICAL-CHANNELS>
</CAN-CLUSTER-CONDITIONAL></CAN-CLUSTER-VARIANTS></CAN-CLUSTER>"#;
    let compatible = text.replacen("</ELEMENTS>", &format!("{topology}</ELEMENTS>"), 1);
    fs::write(&source, &compatible).unwrap();
    let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
    assert!(imported.validate().unwrap().issues.is_empty());
    generator::generate(&mut imported, &temp.0.join("CompatibleNetworkOutput")).unwrap();
    for (original, changed) in [
        (
            "<START-POSITION>0</START-POSITION>",
            "<START-POSITION>8</START-POSITION>",
        ),
        (
            "<FRAME-LENGTH>2</FRAME-LENGTH>",
            "<FRAME-LENGTH>3</FRAME-LENGTH>",
        ),
    ] {
        let altered_topology = topology.replacen(original, changed, 1);
        fs::write(
            &source,
            text.replacen("</ELEMENTS>", &format!("{altered_topology}</ELEMENTS>"), 1),
        )
        .unwrap();
        let mut mismatched = Workspace::open(vec![source.clone()], archive()).unwrap();
        let checked = mismatched.validate().unwrap();
        assert!(
            checked
                .issues
                .iter()
                .any(|issue| issue.code == "CAN_FRAME_MAPPING"),
            "{:?}",
            checked.issues
        );
        assert!(generator::generate(&mut mismatched, &temp.0.join("UnsafeFrameOutput")).is_err());
    }
    fs::write(
        &source,
        compatible.replacen(
            "<IDENTIFIER>801</IDENTIFIER>",
            "<IDENTIFIER>802</IDENTIFIER>",
            1,
        ),
    )
    .unwrap();
    imported = Workspace::open(vec![source], archive()).unwrap();
    let checked = imported.validate().unwrap();
    assert!(
        checked
            .issues
            .iter()
            .any(|issue| issue.code == "CAN_ID_MISMATCH"),
        "{:?}",
        checked.issues
    );
    let generated = temp.0.join("UnsafeNetworkOutput");
    assert!(generator::generate(&mut imported, &generated).is_err());
    assert!(!generated.exists());
}

#[cfg(windows)]
#[test]
fn configured_diagnostic_ecu_roundtrips_arxml_and_exchanges_live_multiframe_did() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    project
        .add_signal(frame.clone(), "LiveA".into(), 0, 32, 0)
        .unwrap();
    let view = project
        .add_signal(frame.clone(), "LiveB".into(), 32, 32, 0)
        .unwrap();
    let sources: Vec<_> = view
        .signals
        .iter()
        .filter(|signal| signal.frame_path == frame)
        .map(|signal| signal.path.clone())
        .collect();
    assert!(
        project
            .configure_diagnostic(DiagnosticSettings {
                request_id: 0x321,
                response_id: 0x708,
                s3_ms: 5000,
                n_bs_ms: 200,
                n_cr_ms: 200,
                did: 0x1234,
                signal_paths: sources.clone(),
                write_enabled: false,
                reset_routine_id: None,
                security_enabled: false
            })
            .is_err()
    );
    assert!(project.view().diagnostic.is_none());
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: sources,
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    let validation = project.validate().unwrap();
    assert!(validation.issues.is_empty(), "{:?}", validation.issues);
    project.save().unwrap();

    let source = temp.0.join("Diag/Diag.arxml");
    let original = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&original).unwrap();
    let global: Vec<_> = doc
        .descendants()
        .filter(|node| {
            node.has_tag_name("ECUC-CONTAINER-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text()
                            == Some("/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu")
                })
        })
        .collect();
    assert_eq!(
        global.len(),
        5,
        "Com, CanTp and Dcm system PDUs need distinct global handles"
    );
    assert!(
        global.iter().all(|pdu| !pdu
            .descendants()
            .any(|node| node.has_tag_name("DEFINITION-REF")
                && node
                    .text()
                    .is_some_and(|value| value.ends_with("/DynamicLength")))),
        "constr_3448 excludes DynamicLength for I-SIGNAL-I-PDU, N-PDU and DCM-I-PDU"
    );
    let n_pdus: Vec<_> = doc
        .descendants()
        .filter(|node| {
            node.has_tag_name("N-PDU")
                && node.children().any(|child| {
                    child.has_tag_name("SHORT-NAME")
                        && child
                            .text()
                            .is_some_and(|name| name.starts_with("NPdu_Diag"))
                })
        })
        .collect();
    assert_eq!(n_pdus.len(), 2);
    assert!(
        n_pdus.iter().all(|pdu| !pdu
            .children()
            .any(|child| child.has_tag_name("HAS-DYNAMIC-LENGTH"))),
        "System Template constr_3448 excludes hasDynamicLength on N-PDU"
    );
    assert!(doc.descendants().any(|node| {
        node.has_tag_name("ECUC-TEXTUAL-PARAM-VALUE")
            && node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|value| value.ends_with("/PduLengthTypeEnum"))
            })
            && node
                .children()
                .any(|child| child.has_tag_name("VALUE") && child.text() == Some("UINT16"))
    }));
    for (name, target) in [
        ("CanIfRxPduRef", "NPdu_DiagRequest"),
        ("CanIfTxPduRef", "NPdu_DiagResponse"),
        ("CanTpRxNPduRef", "NPdu_DiagRequest"),
        ("CanTpTxFcNPduRef", "NPdu_DiagResponse"),
        ("CanTpTxNPduRef", "NPdu_DiagResponse"),
        ("CanTpRxFcNPduRef", "NPdu_DiagRequest"),
        ("CanTpRxNSduRef", "DcmPdu_DiagRequest"),
        ("CanTpTxNSduRef", "DcmPdu_DiagResponse"),
        ("DcmDslProtocolRxPduRef", "DcmPdu_DiagRequest"),
        ("DcmDslProtocolTxPduRef", "DcmPdu_DiagResponse"),
    ] {
        let target = format!("/Diag/EcuCCfg/EcucConfigSet/Pdus/{target}");
        assert!(
            doc.descendants()
                .any(|node| node.has_tag_name("ECUC-REFERENCE-VALUE")
                    && node
                        .children()
                        .any(|child| child.has_tag_name("DEFINITION-REF")
                            && child
                                .text()
                                .is_some_and(|value| value.ends_with(&format!("/{name}"))))
                    && node.children().any(|child| child.has_tag_name("VALUE-REF")
                        && child.attribute("DEST") == Some("ECUC-CONTAINER-VALUE")
                        && child.text() == Some(target.as_str()))),
            "{name} must target {target}"
        );
    }
    fs::write(
        &source,
        original.replace("</ELEMENTS>", "<!-- user annotation --></ELEMENTS>"),
    )
    .unwrap();
    let mut reopened = Workspace::open(vec![source.clone()], archive()).unwrap();
    let diagnostic = reopened.view().diagnostic.unwrap();
    assert_eq!(
        (
            diagnostic.request_id,
            diagnostic.response_id,
            diagnostic.did
        ),
        (0x700, 0x708, 0x1234)
    );
    let output = temp.0.join("GeneratedDiag");
    generator::generate(&mut reopened, &output).unwrap();
    let mut names: Vec<String> = fs::read_to_string(output.join("files.list"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    names.extend(["files.list".into(), "files.sha256".into()]);
    let original_files: Vec<_> = names
        .iter()
        .map(|name| fs::read(output.join(name)).unwrap())
        .collect();
    generator::generate(&mut reopened, &output).unwrap();
    for (name, expected) in names.iter().zip(&original_files) {
        assert_eq!(
            &fs::read(output.join(name)).unwrap(),
            expected,
            "identical diagnostic ARXML changed {name}"
        );
    }
    let binary = generator::build(&output).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin.take().unwrap().write_all(
        b"S 0 287454020\nS 1 1432778632\nR 1792 4 03221234\nR 1792 3 021003\nR 1792 4 03221234\nR 1792 3 300000\nT 5001\nR 1792 4 03221234\n"
    ).unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert!(output.status.success());
    let log = String::from_utf8(output.stdout).unwrap();
    let frames: Vec<_> = log
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        frames,
        [
            "X 1800 4 037F2231",
            "X 1800 7 06500300320032",
            "X 1800 8 100B621234112233",
            "X 1800 6 214455667788",
            "X 1800 4 037F2231",
        ],
        "{log}"
    );
    let report = host::run_diagnostic(&temp.0.join("GeneratedDiag")).unwrap();
    assert!(
        report.passed,
        "independent host diagnostic tester failed: {}",
        report.log
    );
    assert!(report.events.iter().any(|event| event.contains("多帧")));
    reopened.clear_diagnostic().unwrap();
    reopened.save().unwrap();
    assert!(
        fs::read_to_string(&source)
            .unwrap()
            .contains("<!-- user annotation -->")
    );
    let mut signal_only = Workspace::open(vec![temp.0.join("Diag/Diag.arxml")], archive()).unwrap();
    assert!(signal_only.view().diagnostic.is_none());
    let signal_output = temp.0.join("GeneratedSignalsOnly");
    generator::generate(&mut signal_only, &signal_output).unwrap();
    generator::build(&signal_output).unwrap();
    assert!(host::run_diagnostic(&signal_output).is_err());
}

#[cfg(windows)]
#[test]
fn active_session_did_reports_session_transitions_and_rejects_invalid_reads() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(frame, "LiveValue".into(), 0, 32, 42)
        .unwrap()
        .signals[0]
        .path
        .clone();
    assert!(
        project
            .configure_diagnostic(DiagnosticSettings {
                request_id: 0x700,
                response_id: 0x708,
                s3_ms: 5000,
                n_bs_ms: 200,
                n_cr_ms: 200,
                did: 0xF186,
                signal_paths: vec![signal.clone()],
                write_enabled: false,
                reset_routine_id: None,
                security_enabled: false
            })
            .is_err()
    );
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("Diag/Diag.arxml");
    let saved = fs::read(&source).unwrap();
    let mut reopened = Workspace::open(vec![source.clone()], archive()).unwrap();
    assert!(reopened.validate().unwrap().issues.is_empty());
    let generated = temp.0.join("GeneratedDiag");
    generator::generate(&mut reopened, &generated).unwrap();
    assert_eq!(fs::read(&source).unwrap(), saved);
    let binary = generator::build(&generated).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin.take().unwrap().write_all(
        b"R 1792 4 0322F186\nR 1792 4 0322F187\nR 1792 3 0222F1\nR 1792 3 021003\nR 1792 4 0322F186\nR 1792 3 021001\nR 1792 4 0322F186\nR 1792 3 021003\nT 5001\nR 1792 4 0322F186\n"
    ).unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let log = String::from_utf8(output.stdout).unwrap();
    let diagnostic: Vec<_> = log
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        diagnostic,
        [
            "X 1800 5 0462F18601",
            "X 1800 4 037F2231",
            "X 1800 4 037F2213",
            "X 1800 7 06500300320032",
            "X 1800 5 0462F18603",
            "X 1800 7 06500100320032",
            "X 1800 5 0462F18601",
            "X 1800 7 06500300320032",
            "X 1800 5 0462F18601",
        ],
        "{log}"
    );
    let report = host::run_diagnostic(&generated).unwrap();
    assert!(report.passed, "{report:?}");
    assert!(report.events.iter().any(|event| event.contains("0xF186")));

    let signal = reopened.view().diagnostic.unwrap().signal_paths[0].clone();
    reopened
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0xF187,
            signal_paths: vec![signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    reopened.save().unwrap();
    let generated = temp.0.join("GeneratedF187");
    generator::generate(&mut reopened, &generated).unwrap();
    generator::build(&generated).unwrap();
    let report = host::run_diagnostic(&generated).unwrap();
    assert!(report.passed, "0xF187 is a configurable DID: {report:?}");
}

#[cfg(windows)]
#[test]
fn multiple_dids_keep_request_order_and_skip_unavailable_values() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(frame, "LiveValue".into(), 0, 32, 42)
        .unwrap()
        .signals[0]
        .path
        .clone();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("Diag/Diag.arxml");
    let saved = fs::read(&source).unwrap();
    let mut reopened = Workspace::open(vec![source.clone()], archive()).unwrap();
    assert!(reopened.validate().unwrap().issues.is_empty());
    let generated = temp.0.join("GeneratedDiag");
    generator::generate(&mut reopened, &generated).unwrap();
    assert_eq!(fs::read(&source).unwrap(), saved);
    let binary = generator::build(&generated).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut commands = b"R 1792 6 05221234F186\nR 1792 3 021003\nR 1792 6 05221234F186\nR 1792 3 300000\nR 1792 6 0522F1861234\nR 1792 3 300000\nR 1792 6 0522F187F186\nR 1792 6 0522F187F188\nR 1792 5 0422F18612\nR 1792 6 0522F186F186\nR 1792 4 03221234\nR 1792 3 300000\n".to_vec();
    let mut oversized = vec![0x22];
    for _ in 0..86 {
        oversized.extend_from_slice(&[0xF1, 0x86]);
    }
    assert_eq!(oversized.len(), 173);
    let mut first = vec![0x10, oversized.len() as u8];
    first.extend_from_slice(&oversized[..6]);
    let mut request_frames = vec![first];
    for (index, chunk) in oversized[6..].chunks(7).enumerate() {
        let mut frame = vec![0x20 | ((index as u8 + 1) & 0x0F)];
        frame.extend_from_slice(chunk);
        request_frames.push(frame);
    }
    for frame in request_frames {
        let hex = frame
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join("");
        commands.extend_from_slice(format!("R 1792 {} {hex}\n", frame.len()).as_bytes());
    }
    commands.extend_from_slice(b"R 1792 4 0322F186\n");
    ecu.stdin.take().unwrap().write_all(&commands).unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let log = String::from_utf8(output.stdout).unwrap();
    let frames: Vec<_> = log
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        frames,
        [
            "X 1800 5 0462F18601",
            "X 1800 7 06500300320032",
            "X 1800 8 100A621234000000",
            "X 1800 5 212AF18603",
            "X 1800 8 100A62F186031234",
            "X 1800 5 210000002A",
            "X 1800 5 0462F18603",
            "X 1800 4 037F2231",
            "X 1800 4 037F2213",
            "X 1800 8 0762F18603F18603",
            "X 1800 8 076212340000002A",
            "X 1800 3 300000",
            "X 1800 4 037F2214",
            "X 1800 5 0462F18603",
        ],
        "{log}"
    );
    let report = host::run_diagnostic(&generated).unwrap();
    assert!(report.passed, "{report:?}");
    assert!(report.events.iter().any(|event| event.contains("多 DID")));
}

#[cfg(windows)]
#[test]
fn diagnostic_transport_discards_bad_or_timed_out_multiframe_requests_and_recovers() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    project
        .add_signal(frame.clone(), "LiveValue".into(), 0, 32, 42)
        .unwrap();
    let view = project
        .add_signal(frame, "LiveOther".into(), 32, 32, 43)
        .unwrap();
    let sources = view
        .signals
        .iter()
        .map(|signal| signal.path.clone())
        .collect();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: sources,
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let generated = temp.0.join("GeneratedDiag");
    generator::generate(&mut project, &generated).unwrap();
    let binary = generator::build(&generated).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin.take().unwrap().write_all(
        b"R 1792 8 100A221234123412\nR 1792 4 22341234\nR 1792 8 100A221234123412\nR 1792 5 2134123412\nR 1792 8 100A221234123412\nT 201\nR 1792 3 023E00\nR 1792 3 021003\nR 1792 4 03221234\nT 402\nR 1792 3 023E00\n"
    ).unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert!(output.status.success());
    let log = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = log
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect();
    let flow_controls = lines
        .iter()
        .filter(|line| **line == "X 1800 3 300000")
        .count();
    assert_eq!(flow_controls, 3, "{log}");
    assert!(lines.contains(&"E TP_SEQUENCE"), "{log}");
    assert!(lines.contains(&"X 1800 4 037F2213"), "{log}");
    assert_eq!(
        lines.iter().filter(|line| **line == "E TP_TIMEOUT").count(),
        2,
        "{log}"
    );
    assert!(lines.contains(&"X 1800 3 027E00"), "{log}");
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("X 1800 8 100B621234")),
        "{log}"
    );
}

#[cfg(windows)]
#[test]
fn diagnostic_tester_present_keeps_session_and_fc_block_size_paces_response() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    for (name, id) in [("LiveA", 0x321), ("LiveB", 0x322)] {
        let frame = project
            .add_frame(name.into(), id, 8, Direction::Tx, Some(1000), None)
            .unwrap()
            .frames
            .into_iter()
            .find(|item| item.name == name)
            .unwrap()
            .path;
        project
            .add_signal(frame.clone(), format!("{name}Low"), 0, 32, 0)
            .unwrap();
        project
            .add_signal(frame, format!("{name}High"), 32, 32, 0)
            .unwrap();
    }
    let sources = project
        .view()
        .signals
        .iter()
        .map(|signal| signal.path.clone())
        .collect();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: sources,
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let generated = temp.0.join("GeneratedDiag");
    generator::generate(&mut project, &generated).unwrap();
    let binary = generator::build(&generated).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin.take().unwrap().write_all(
        b"S 0 16909060\nS 1 84281096\nS 2 151653132\nS 3 219025168\nR 1792 3 021003\nT 4000\nR 1792 3 023E80\nT 8000\nR 1792 4 03221234\nR 1792 3 300105\nG 0\nT 8005\nG 0\nR 1792 3 300105\nT 13006\nR 1792 4 03221234\n"
    ).unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert!(output.status.success());
    let log = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = log
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect();
    let diagnostic: Vec<_> = lines
        .iter()
        .filter(|line| line.starts_with("X 1800 "))
        .copied()
        .collect();
    assert_eq!(
        diagnostic,
        [
            "X 1800 7 06500300320032",
            "X 1800 8 1013621234010203",
            "X 1800 8 210405060708090A",
            "X 1800 7 220B0C0D0E0F10",
            "X 1800 4 037F2231",
        ],
        "{log}"
    );
    let fence_positions: Vec<_> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("V 0 "))
        .map(|(index, _)| index)
        .collect();
    let second_cf = lines
        .iter()
        .position(|line| *line == "X 1800 7 220B0C0D0E0F10")
        .unwrap();
    assert_eq!(fence_positions.len(), 2, "{log}");
    assert!(
        fence_positions[1] < second_cf,
        "FC block size was ignored: {log}"
    );
}

#[cfg(windows)]
#[test]
fn unsupported_imported_transport_padding_blocks_diagnostic_generation() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(frame, "LiveA".into(), 0, 32, 0)
        .unwrap()
        .signals[0]
        .path
        .clone();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("Diag/Diag.arxml");
    let original = fs::read_to_string(&source).unwrap();
    fs::write(&source, original.replacen(">CANTP_OFF<", ">CANTP_ON<", 1)).unwrap();
    let mut imported = Workspace::open(vec![source], archive()).unwrap();
    assert!(
        imported
            .view()
            .issues
            .iter()
            .any(|issue| issue.code == "DIAG_UNSUPPORTED")
    );
    let generated = temp.0.join("RejectedDiag");
    assert!(generator::generate(&mut imported, &generated).is_err());
    assert!(!generated.exists());
}

#[cfg(windows)]
#[test]
fn supported_dtcs_include_zero_status_and_follow_configured_lifecycle() {
    let temp = Scratch::new();
    for (name, code, encoded) in [
        ("SupportedA", 0x123456, "123456"),
        ("SupportedB", 0xABCDEF, "ABCDEF"),
    ] {
        let directory = temp.0.join(name);
        let mut project = Workspace::create(&directory, name, archive()).unwrap();
        let tx = project
            .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
            .unwrap()
            .frames[0]
            .path
            .clone();
        let signal = project
            .add_signal(tx, "LiveValue".into(), 0, 32, 7)
            .unwrap()
            .signals[0]
            .path
            .clone();
        let rx = project
            .add_frame("Heartbeat".into(), 0x456, 2, Direction::Rx, None, Some(50))
            .unwrap()
            .frames
            .into_iter()
            .find(|frame| frame.name == "Heartbeat")
            .unwrap()
            .path;
        project
            .add_signal(rx.clone(), "HeartbeatValue".into(), 0, 8, 0)
            .unwrap();
        project
            .configure_diagnostic(DiagnosticSettings {
                request_id: 0x700,
                response_id: 0x708,
                s3_ms: 5000,
                n_bs_ms: 200,
                n_cr_ms: 200,
                did: 0x1234,
                signal_paths: vec![signal],
                write_enabled: false,
                reset_routine_id: None,
                security_enabled: false,
            })
            .unwrap();
        project.configure_dtc(code, rx).unwrap();
        project.save().unwrap();
        let source = directory.join(format!("{name}.arxml"));
        let saved = fs::read(&source).unwrap();
        let mut reopened = Workspace::open(vec![source.clone()], archive()).unwrap();
        assert!(reopened.validate().unwrap().issues.is_empty());
        assert_eq!(reopened.view().diagnostic.unwrap().dtc.unwrap().code, code);
        let generated = directory.join("generated");
        generator::generate(&mut reopened, &generated).unwrap();
        assert_eq!(fs::read(&source).unwrap(), saved);
        let binary = generator::build(&generated).unwrap().binary_path;
        let storage = directory.join("dtc.nvm");
        let run = |input: &str| {
            let mut ecu = Command::new(&binary)
                .arg("--nvm")
                .arg(&storage)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            ecu.stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
            let output = ecu.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
            let lines: Vec<String> = String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .map(|line| line.trim_end_matches('\r').to_owned())
                .collect();
            assert!(
                !lines.iter().any(|line| line.starts_with("E ")),
                "{lines:?}"
            );
            lines
                .into_iter()
                .filter(|line| line.starts_with("X 1800 "))
                .collect::<Vec<_>>()
        };
        let supported = |status| format!("X 1800 8 07590A7F{encoded}{status:02X}");
        // Initial reads, including rejected requests, must not mutate persistent state.
        assert!(run("").is_empty());
        let before = fs::read(&storage).unwrap();
        assert_eq!(
            run(
                "R 1792 3 02190A\nR 1792 4 03190A00\nR 1792 2 0119\nR 1792 3 021903\nR 1792 3 02198A\nR 1792 3 02190A\n"
            ),
            vec![
                supported(0x50),
                "X 1800 4 037F1913".into(),
                "X 1800 4 037F1913".into(),
                "X 1800 4 037F1912".into(),
                "X 1800 4 037F1912".into(),
                supported(0x50),
            ]
        );
        assert_eq!(fs::read(&storage).unwrap(), before);
        // Zero status is still supported; a zero status mask remains empty for 01/02.
        assert_eq!(
            run(
                "R 1110 2 0100\nR 1792 3 02190A\nR 1792 4 03190100\nR 1792 4 03190200\nR 1792 4 0319027F\nR 1792 3 021003\nR 1792 3 028502\nT 51\nR 1792 3 02190A\nR 1792 3 028501\nR 1110 2 0100\nT 102\nR 1792 3 02190A\n"
            ),
            vec![
                supported(0x00),
                "X 1800 7 0659017F010000".into(),
                "X 1800 4 0359027F".into(),
                "X 1800 4 0359027F".into(),
                "X 1800 7 06500300320032".into(),
                "X 1800 3 02C502".into(),
                supported(0x00),
                "X 1800 3 02C501".into(),
                supported(0x2F),
            ]
        );
        assert_eq!(
            run(
                "R 1792 3 02190A\nR 1110 2 0100\nR 1792 3 02190A\nR 1792 3 021003\nR 1792 5 0414FFFFFF\nR 1792 3 02190A\n"
            ),
            vec![
                supported(0x6D),
                supported(0x2C),
                "X 1800 7 06500300320032".into(),
                "X 1800 2 0154".into(),
                supported(0x50),
            ]
        );
        assert_eq!(run("R 1792 3 02190A\n"), vec![supported(0x50)]);
        let report = host::run_diagnostic(&generated).unwrap();
        assert!(report.passed, "{report:?}");
        assert!(
            report
                .events
                .iter()
                .any(|event| event.contains("0x19/0x0A"))
        );

        reopened.clear_dtc().unwrap();
        reopened.save().unwrap();
        let mut cleared = Workspace::open(vec![source], archive()).unwrap();
        assert!(cleared.view().diagnostic.unwrap().dtc.is_none());
        let without_dtc = directory.join("without-dtc");
        generator::generate(&mut cleared, &without_dtc).unwrap();
        let binary = generator::build(&without_dtc).unwrap().binary_path;
        let mut ecu = Command::new(binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        ecu.stdin
            .take()
            .unwrap()
            .write_all(b"R 1792 3 02190A\n")
            .unwrap();
        let output = ecu.wait_with_output().unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "X 1800 4 037F1911"
        );
    }
}

#[cfg(windows)]
#[test]
fn rx_timeout_dtc_is_reported_cleared_and_persists_across_ecu_restarts() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let tx = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let did_signal = project
        .add_signal(tx, "LiveValue".into(), 0, 32, 7)
        .unwrap()
        .signals[0]
        .path
        .clone();
    let rx = project
        .add_frame("Heartbeat".into(), 0x456, 2, Direction::Rx, None, Some(50))
        .unwrap()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Heartbeat")
        .unwrap()
        .path;
    project
        .add_signal(rx.clone(), "HeartbeatValue".into(), 0, 8, 0)
        .unwrap();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![did_signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.configure_dtc(0x123456, rx.clone()).unwrap();
    project.save().unwrap();
    let source = temp.0.join("Diag/Diag.arxml");
    let original = fs::read_to_string(&source).unwrap();
    let document = roxmltree::Document::parse(&original).unwrap();
    let dcm_service = document.descendants().find(|node| node.has_tag_name("ECUC-CONTAINER-VALUE") &&
        node.children().any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some("ControlDTCSetting")) &&
        node.children().any(|child| child.has_tag_name("DEFINITION-REF") &&
            child.text() == Some("/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsd/DcmDsdServiceTable/DcmDsdService"))).unwrap();
    assert!(dcm_service.descendants().any(|node| {
        node.has_tag_name("ECUC-NUMERICAL-PARAM-VALUE")
            && node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|value| value.ends_with("/DcmDsdSidTabServiceId"))
            })
            && node
                .children()
                .any(|child| child.has_tag_name("VALUE") && child.text() == Some("133"))
    }));
    assert!(
        dcm_service
            .descendants()
            .any(|node| node.has_tag_name("VALUE-REF")
                && node.attribute("DEST") == Some("ECUC-CONTAINER-VALUE")
                && node.text() == Some("/Diag/DcmCfg/DcmConfigSet/DcmDsp/Sessions/Extended"))
    );
    assert!(
        document
            .descendants()
            .any(|node| node.has_tag_name("ECUC-REFERENCE-VALUE")
                && node
                    .children()
                    .any(|child| child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|value| value.ends_with("/DcmDemClientRef")))
                && node.children().any(|child| child.has_tag_name("VALUE-REF")
                    && child.attribute("DEST") == Some("ECUC-CONTAINER-VALUE")
                    && child.text() == Some("/Diag/DemCfg/DemGeneral/DcmClient")))
    );
    let option = document
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-NUMERICAL-PARAM-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text().is_some_and(|value| {
                            value.ends_with("/DcmSupportDTCSettingControlOptionRecord")
                        })
                })
        })
        .unwrap();
    let option_value = option
        .children()
        .find(|node| node.has_tag_name("VALUE"))
        .unwrap();
    assert_eq!(option_value.text(), Some("false"));
    let mutated = format!(
        "{}<VALUE>true</VALUE>{}",
        &original[..option_value.range().start],
        &original[option_value.range().end..]
    );
    let unsupported_path = temp.0.join("Unsupported.arxml");
    fs::write(&unsupported_path, &mutated).unwrap();
    let mut unsupported = Workspace::open(vec![unsupported_path.clone()], archive()).unwrap();
    assert!(
        unsupported
            .view()
            .issues
            .iter()
            .any(|issue| issue.code == "DIAG_UNSUPPORTED")
    );
    assert!(generator::generate(&mut unsupported, &temp.0.join("UnsupportedOutput")).is_err());
    assert_eq!(fs::read_to_string(&unsupported_path).unwrap(), mutated);
    fs::write(
        &source,
        original.replace("</ELEMENTS>", "<!-- retained by owner --></ELEMENTS>"),
    )
    .unwrap();
    let mut reopened = Workspace::open(vec![source.clone()], archive()).unwrap();
    let dtc = reopened.view().diagnostic.unwrap().dtc.unwrap();
    assert_eq!((dtc.code, dtc.monitor_frame_path), (0x123456, rx));
    let generated = temp.0.join("GeneratedDiag");
    generator::generate(&mut reopened, &generated).unwrap();
    let handoff = fs::read_to_string(generated.join("README.md")).unwrap();
    assert!(handoff.contains(".\\ecu_host.exe --nvm .\\ecu.nvm"));
    assert!(!handoff.contains("--security-key"));
    let binary = generator::build(&generated).unwrap().binary_path;
    let without_storage = Command::new(&binary).output().unwrap();
    assert!(!without_storage.status.success());
    assert!(String::from_utf8_lossy(&without_storage.stdout).contains("E CONFIG"));
    let storage = temp.0.join("dtc.nvm");
    let run = |storage: &Path, input: &[u8]| {
        let mut ecu = Command::new(&binary)
            .arg("--nvm")
            .arg(storage)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        ecu.stdin.take().unwrap().write_all(input).unwrap();
        let result = ecu.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
        String::from_utf8(result.stdout).unwrap()
    };
    let first = run(&storage, b"R 1110 2 0100\nR 1792 4 03190208\nR 1792 4 03190108\nR 1792 4 03190100\nR 1792 3 021901\nR 1792 4 03190308\nT 51\nR 1792 4 03190208\nR 1792 4 03190108\nR 1792 4 03190110\nR 1792 4 03190200\n");
    let first: Vec<_> = first
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        first,
        [
            "X 1800 4 0359027F",
            "X 1800 7 0659017F010000",
            "X 1800 7 0659017F010000",
            "X 1800 4 037F1913",
            "X 1800 4 037F1912",
            "X 1800 8 0759027F1234562F",
            "X 1800 7 0659017F010001",
            "X 1800 7 0659017F010000",
            "X 1800 4 0359027F",
        ],
        "{first:?}"
    );
    assert!(storage.is_file());

    let second = run(&storage, b"R 1792 4 03190208\nR 1792 4 03190108\nR 1792 4 03190110\nR 1110 2 0200\nR 1792 4 03190208\nR 1792 4 03190108\nR 1792 5 0414FFFFFF\nR 1792 3 021003\nR 1792 5 0414000001\nR 1792 5 0414FFFFFF\nR 1792 4 03190208\nR 1792 4 03190108\n");
    let second: Vec<_> = second
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        second,
        [
            "X 1800 8 0759027F1234566D",
            "X 1800 7 0659017F010001",
            "X 1800 7 0659017F010000",
            "X 1800 8 0759027F1234562C",
            "X 1800 7 0659017F010001",
            "X 1800 4 037F147F",
            "X 1800 7 06500300320032",
            "X 1800 4 037F1431",
            "X 1800 2 0154",
            "X 1800 4 0359027F",
            "X 1800 7 0659017F010000",
        ],
        "{second:?}"
    );

    let third = run(&storage, b"R 1792 4 03190208\nR 1792 4 03190108\n");
    assert_eq!(
        third
            .lines()
            .map(|line| line.trim_end_matches('\r'))
            .collect::<Vec<_>>(),
        ["X 1800 4 0359027F", "X 1800 7 0659017F010000"]
    );
    let control_storage = temp.0.join("dtc-control.nvm");
    let controlled = run(&control_storage, b"R 1792 3 028502\nR 1792 3 021003\nR 1792 2 0185\nR 1792 6 058502FFFFFF\nR 1792 3 028503\nR 1792 3 028582\nR 1792 3 028502\nR 1110 2 0100\nT 51\nR 1792 4 03190208\nR 1792 4 03190108\nR 1792 3 028501\nR 1110 2 0100\nT 102\nR 1792 4 03190208\nR 1792 3 028502\nR 1110 2 0100\nT 153\nR 1792 4 03190208\nR 1792 5 0414FFFFFF\nR 1792 4 03190208\nR 1792 3 028502\nR 1792 3 021001\nR 1792 3 028502\nR 1110 2 0100\nT 204\nR 1792 4 03190208\n");
    let controlled: Vec<_> = controlled
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        controlled,
        [
            "X 1800 4 037F857F",
            "X 1800 7 06500300320032",
            "X 1800 4 037F8513",
            "X 1800 4 037F8513",
            "X 1800 4 037F8512",
            "X 1800 4 037F8512",
            "X 1800 3 02C502",
            "X 1800 4 0359027F",
            "X 1800 7 0659017F010000",
            "X 1800 3 02C501",
            "X 1800 8 0759027F1234562F",
            "X 1800 3 02C502",
            "X 1800 8 0759027F1234562F",
            "X 1800 2 0154",
            "X 1800 4 0359027F",
            "X 1800 3 02C502",
            "X 1800 7 06500100320032",
            "X 1800 4 037F857F",
            "X 1800 8 0759027F1234562F",
        ],
        "{controlled:?}"
    );
    let s3_storage = temp.0.join("dtc-s3.nvm");
    let s3 = run(&s3_storage, b"R 1792 3 021003\nR 1792 3 028502\nR 1110 2 0100\nT 51\nT 5100\nR 1792 3 028502\nR 1110 2 0100\nT 5151\nR 1792 4 03190208\n");
    let s3: Vec<_> = s3
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        s3,
        [
            "X 1800 7 06500300320032",
            "X 1800 3 02C502",
            "X 1800 4 037F857F",
            "X 1800 8 0759027F1234562F",
        ],
        "{s3:?}"
    );
    let restart_storage = temp.0.join("dtc-restart.nvm");
    let disabled = run(
        &restart_storage,
        b"R 1792 3 021003\nR 1792 3 028502\nR 1110 2 0100\nT 51\nR 1792 4 03190208\n",
    );
    assert!(
        disabled
            .lines()
            .any(|line| line.trim_end_matches('\r') == "X 1800 4 0359027F")
    );
    let enabled_after_restart = run(
        &restart_storage,
        b"R 1110 2 0100\nT 51\nR 1792 4 03190208\n",
    );
    assert!(
        enabled_after_restart
            .lines()
            .any(|line| line.trim_end_matches('\r') == "X 1800 8 0759027F1234562F")
    );
    let mut slots = fs::read(&storage).unwrap();
    assert_eq!(slots.len(), 64);
    let first_sequence = u64::from_le_bytes(slots[4..12].try_into().unwrap());
    let second_sequence = u64::from_le_bytes(slots[36..44].try_into().unwrap());
    let newest = if first_sequence > second_sequence {
        0
    } else {
        1
    };
    slots[newest * 32 + 16] ^= 1;
    fs::write(&storage, slots).unwrap();
    let torn = Command::new(&binary)
        .arg("--nvm")
        .arg(&storage)
        .output()
        .unwrap();
    assert!(!torn.status.success());
    assert!(String::from_utf8_lossy(&torn.stdout).contains("E NVM"));
    fs::write(&storage, [0u8; 64]).unwrap();
    let failed = Command::new(&binary)
        .arg("--nvm")
        .arg(&storage)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stdout).contains("E NVM"));
    reopened.clear_dtc().unwrap();
    reopened.save().unwrap();
    assert!(
        fs::read_to_string(&source)
            .unwrap()
            .contains("<!-- retained by owner -->")
    );
    let saved_without_dtc = fs::read_to_string(&source).unwrap();
    let stripped = roxmltree::Document::parse(&saved_without_dtc).unwrap();
    assert!(!stripped.descendants().any(|node| {
        node.has_tag_name("ECUC-CONTAINER-VALUE")
            && node.children().any(|child| {
                child.has_tag_name("SHORT-NAME") && child.text() == Some("ControlDTCSetting")
            })
    }));
    assert!(
        !stripped
            .descendants()
            .any(|node| node.has_tag_name("ECUC-REFERENCE-VALUE")
                && node
                    .children()
                    .any(|child| child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|value| value.ends_with("/DcmDemClientRef"))))
    );
    let mut without_dtc = Workspace::open(vec![source], archive()).unwrap();
    let diagnostic = without_dtc.view().diagnostic.unwrap();
    assert!(diagnostic.dtc.is_none());
    assert_eq!(diagnostic.did, 0x1234);
    let simple = temp.0.join("GeneratedWithoutDtc");
    generator::generate(&mut without_dtc, &simple).unwrap();
    let binary = generator::build(&simple).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin
        .take()
        .unwrap()
        .write_all(b"R 1792 4 03190108\nR 1792 3 028502\n")
        .unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| line.trim_end_matches('\r'))
            .collect::<Vec<_>>(),
        ["X 1800 4 037F1911", "X 1800 4 037F8511"]
    );
    assert!(host::run_diagnostic(&simple).unwrap().passed);
}

#[cfg(windows)]
#[test]
fn extended_session_write_did_changes_live_can_but_not_restart_state() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    project
        .add_signal(frame.clone(), "LiveA".into(), 0, 32, 1)
        .unwrap();
    let view = project
        .add_signal(frame.clone(), "LiveB".into(), 32, 32, 2)
        .unwrap();
    let signals = view
        .signals
        .iter()
        .filter(|signal| signal.frame_path == frame)
        .map(|signal| signal.path.clone())
        .collect();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: signals,
            write_enabled: true,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("Diag/Diag.arxml");
    let original = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&original).unwrap();
    assert!(!doc.descendants().any(|node| {
        node.has_tag_name("SDG")
            && node
                .attribute("GID")
                .is_some_and(|gid| gid.starts_with("AutosarWorkbenchHostRestoreDid"))
    }));
    fs::write(
        &source,
        original.replace("</ELEMENTS>", "<!-- owner note --></ELEMENTS>"),
    )
    .unwrap();
    let mut reopened = Workspace::open(vec![source.clone()], archive()).unwrap();
    assert!(reopened.view().diagnostic.unwrap().write_enabled);
    let generated = temp.0.join("GeneratedWrite");
    generator::generate(&mut reopened, &generated).unwrap();
    let binary = generator::build(&generated).unwrap().binary_path;
    let run = |commands: &[u8]| {
        let mut ecu = Command::new(&binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        ecu.stdin.take().unwrap().write_all(commands).unwrap();
        let result = ecu.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
        String::from_utf8(result.stdout).unwrap()
    };
    let first = run(b"R 1792 4 032E1234\nR 1792 3 021003\nR 1792 5 043101F001\nR 1792 4 032E4321\nR 1792 4 032E1234\nR 1792 4 03221234\nR 1792 3 300000\nR 1792 8 100B2E1234112233\nR 1792 6 214455667788\nR 1792 4 03221234\nR 1792 3 300000\nT 100\nT 5101\nR 1792 4 032E1234\n");
    let responses: Vec<_> = first
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        responses,
        [
            "X 1800 4 037F2E31",
            "X 1800 7 06500300320032",
            "X 1800 4 037F3111",
            "X 1800 4 037F2E31",
            "X 1800 4 037F2E13",
            "X 1800 8 100B621234000000",
            "X 1800 6 210100000002",
            "X 1800 3 300000",
            "X 1800 4 036E1234",
            "X 1800 8 100B621234112233",
            "X 1800 6 214455667788",
            "X 1800 4 037F2E31",
        ],
        "{first}"
    );
    assert!(
        first
            .lines()
            .any(|line| line.trim_end_matches('\r') == "X 801 8 4433221188776655"),
        "{first}"
    );
    let after_restart = run(b"R 1792 3 021003\nR 1792 4 03221234\nR 1792 3 300000\n");
    assert!(
        after_restart.contains("X 1800 8 100B621234000000"),
        "{after_restart}"
    );
    assert!(
        after_restart.contains("X 1800 6 210100000002"),
        "{after_restart}"
    );
    let report = host::run_diagnostic(&generated).unwrap();
    assert!(report.passed, "{}", report.log);
    assert!(report.events.iter().any(|event| event.contains("写入")));
    reopened.clear_diagnostic().unwrap();
    reopened.save().unwrap();
    assert!(
        fs::read_to_string(&source)
            .unwrap()
            .contains("<!-- owner note -->")
    );
}

#[cfg(windows)]
#[test]
fn security_access_roundtrips_and_gates_host_writes() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Secure"), "Secure", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(frame, "LiveValue".into(), 0, 32, 7)
        .unwrap()
        .signals[0]
        .path
        .clone();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: true,
            reset_routine_id: Some(0xf001),
            security_enabled: true,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("Secure/Secure.arxml");
    let xml = fs::read_to_string(&source).unwrap();
    assert!(xml.contains("DcmDspSecurityRow"));
    assert!(!xml.contains("5A5A5A5A"));
    let mut reopened = Workspace::open(vec![source], archive()).unwrap();
    assert!(reopened.view().diagnostic.unwrap().security_enabled);
    let generated = temp.0.join("GeneratedSecure");
    generator::generate(&mut reopened, &generated).unwrap();
    let handoff = fs::read_to_string(generated.join("README.md")).unwrap();
    assert!(
        handoff
            .contains(".\\ecu_host.exe --security-key .\\ecu.key --security-state .\\ecu.security")
    );
    assert!(!handoff.contains("--nvm"));
    let configuration = fs::read_to_string(generated.join("Ecu_Config.c")).unwrap();
    assert!(!configuration.contains("5A5A5A5A"));
    let binary = generator::build(&generated).unwrap().binary_path;
    let without_key = Command::new(&binary).output().unwrap();
    assert!(!without_key.status.success());
    assert!(String::from_utf8_lossy(&without_key.stdout).contains("E CONFIG"));
    let report = host::run_diagnostic(&generated).unwrap();
    assert!(report.passed, "{}", report.log);
    assert!(report.events.iter().any(|event| event.contains("0x27")));
}

#[cfg(windows)]
#[test]
fn security_access_gates_dtc_mutations_without_a_writable_did() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("SecureDtc"), "SecureDtc", archive()).unwrap();
    let tx = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(tx, "LiveValue".into(), 0, 32, 7)
        .unwrap()
        .signals[0]
        .path
        .clone();
    let rx = project
        .add_frame("Heartbeat".into(), 0x456, 2, Direction::Rx, None, Some(50))
        .unwrap()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Heartbeat")
        .unwrap()
        .path;
    project
        .add_signal(rx.clone(), "HeartbeatValue".into(), 0, 8, 0)
        .unwrap();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.configure_dtc(0x123456, rx).unwrap();
    let current = project.view().diagnostic.unwrap();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: current.signal_paths,
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: true,
        })
        .unwrap();
    project.save().unwrap();
    let mut reopened =
        Workspace::open(vec![temp.0.join("SecureDtc/SecureDtc.arxml")], archive()).unwrap();
    assert!(reopened.view().diagnostic.unwrap().security_enabled);
    assert!(
        reopened.clear_dtc().is_err(),
        "sole protected operation cannot be removed silently"
    );
    let generated = temp.0.join("GeneratedSecureDtc");
    generator::generate(&mut reopened, &generated).unwrap();
    let handoff = fs::read_to_string(generated.join("README.md")).unwrap();
    assert!(handoff.contains(".\\ecu_host.exe --nvm .\\ecu.nvm --security-key .\\ecu.key --security-state .\\ecu.security"));
    generator::build(&generated).unwrap();
    let report = host::run_diagnostic(&generated).unwrap();
    assert!(report.passed, "{}", report.log);
}

#[cfg(windows)]
#[test]
fn start_routine_restores_written_did_signals_and_respects_session() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    project
        .add_signal(frame.clone(), "LiveA".into(), 0, 32, 1)
        .unwrap();
    let view = project
        .add_signal(frame.clone(), "LiveB".into(), 32, 32, 2)
        .unwrap();
    let signals: Vec<_> = view
        .signals
        .iter()
        .filter(|signal| signal.frame_path == frame)
        .map(|signal| signal.path.clone())
        .collect();
    assert!(
        project
            .configure_diagnostic(DiagnosticSettings {
                request_id: 0x700,
                response_id: 0x708,
                s3_ms: 5000,
                n_bs_ms: 200,
                n_cr_ms: 200,
                did: 0x1234,
                signal_paths: signals.clone(),
                write_enabled: false,
                reset_routine_id: Some(0xF001),
                security_enabled: false
            })
            .is_err()
    );
    assert!(project.view().diagnostic.is_none());
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: signals,
            write_enabled: true,
            reset_routine_id: Some(0xF001),
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("Diag/Diag.arxml");
    let original = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&original).unwrap();
    let did = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-CONTAINER-VALUE")
                && node
                    .children()
                    .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some("Did"))
        })
        .unwrap();
    let routine: Vec<_> = did
        .descendants()
        .filter(|node| {
            node.has_tag_name("SDG")
                && node.attribute("GID") == Some("AutosarWorkbenchHostRestoreDidV1")
        })
        .collect();
    assert_eq!(routine.len(), 1, "host routine metadata must be unique");
    let field = |name: &str| {
        routine[0]
            .children()
            .find(|node| node.has_tag_name("SD") && node.attribute("GID") == Some(name))
            .and_then(|node| node.text())
    };
    assert_eq!(field("Rid"), Some("61441"));
    assert_eq!(
        field("SessionRef"),
        Some("/Diag/DcmCfg/DcmConfigSet/DcmDsp/Sessions/Extended")
    );
    assert!(!doc.descendants().any(|node| {
        node.has_tag_name("DEFINITION-REF")
            && node.text().is_some_and(|path| {
                path.ends_with("/DcmDspRoutine")
                    || path.ends_with("/DcmDspCommonAuthorization")
                    || path.ends_with("/DcmDspStartRoutine")
            })
    }));
    assert!(
        !doc.descendants()
            .any(|node| node.has_tag_name("ECUC-CONTAINER-VALUE")
                && node.children().any(|child| child.has_tag_name("SHORT-NAME")
                    && child.text() == Some("RoutineControl")))
    );
    fs::write(
        &source,
        original.replace("</ELEMENTS>", "<!-- retained annotation --></ELEMENTS>"),
    )
    .unwrap();
    let mut reopened = Workspace::open(vec![source.clone()], archive()).unwrap();
    assert_eq!(
        reopened.view().diagnostic.unwrap().reset_routine_id,
        Some(0xF001)
    );
    let generated = temp.0.join("GeneratedRoutine");
    generator::generate(&mut reopened, &generated).unwrap();
    let binary = generator::build(&generated).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin.take().unwrap().write_all(
        b"R 1792 5 043101F001\nR 1792 3 021003\nR 1792 5 043101F002\nR 1792 5 043102F001\nR 1792 6 053101F00100\nR 1792 8 100B2E1234112233\nR 1792 6 214455667788\nR 1792 4 03221234\nR 1792 3 300000\nR 1792 5 043101F001\nR 1792 4 03221234\nR 1792 3 300000\nT 100\nT 5101\nR 1792 5 043101F001\n"
    ).unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert!(output.status.success());
    let log = String::from_utf8(output.stdout).unwrap();
    let responses: Vec<_> = log
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        responses,
        [
            "X 1800 4 037F3131",
            "X 1800 7 06500300320032",
            "X 1800 4 037F3131",
            "X 1800 4 037F3112",
            "X 1800 4 037F3113",
            "X 1800 3 300000",
            "X 1800 4 036E1234",
            "X 1800 8 100B621234112233",
            "X 1800 6 214455667788",
            "X 1800 5 047101F001",
            "X 1800 8 100B621234000000",
            "X 1800 6 210100000002",
            "X 1800 4 037F3131",
        ],
        "{log}"
    );
    assert!(
        log.lines()
            .any(|line| line.trim_end_matches('\r') == "X 801 8 0100000002000000"),
        "{log}"
    );
    let report = host::run_diagnostic(&generated).unwrap();
    assert!(report.passed, "{}", report.log);
    assert!(report.events.iter().any(|event| event.contains("例程")));
    reopened.clear_diagnostic().unwrap();
    reopened.save().unwrap();
    assert!(
        fs::read_to_string(&source)
            .unwrap()
            .contains("<!-- retained annotation -->")
    );
}

#[test]
fn noncanonical_pdu_input_is_not_rewritten_or_generated() {
    let temp = Scratch::new();
    let source = temp.0.join("Unsupported.arxml");
    let original = include_str!("fixtures/legacy-signal.arxml");
    fs::write(&source, original).unwrap();
    let mut project = Workspace::open(vec![source.clone()], archive()).unwrap();
    let view = project.validate().unwrap();
    assert!(!view.dirty);
    assert!(view.files.iter().all(|file| file.readonly));
    assert!(view.issues.iter().any(|issue| {
        issue.code == "PDU_UNSUPPORTED"
            && issue
                .path
                .as_deref()
                .is_some_and(|path| path.contains("ComCfg"))
    }));
    assert!(
        project
            .preview_save()
            .unwrap_err()
            .contains("PDU_UNSUPPORTED")
    );
    assert!(project.save().is_err());
    let output = temp.0.join("Rejected");
    assert!(generator::generate(&mut project, &output).is_err());
    assert!(!output.exists());
    assert_eq!(fs::read_to_string(source).unwrap(), original);
}

#[test]
fn host_routine_metadata_rejects_unknown_version_and_wrong_session() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 4, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(frame, "Value".into(), 0, 32, 1)
        .unwrap()
        .signals[0]
        .path
        .clone();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: true,
            reset_routine_id: Some(0xf001),
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("Diag/Diag.arxml");
    let saved = fs::read_to_string(&source).unwrap();
    let start = saved
        .find("<SDG GID=\"AutosarWorkbenchHostRestoreDidV1\">")
        .unwrap();
    let end = start + saved[start..].find("</SDG>").unwrap() + "</SDG>".len();
    let group = &saved[start..end];
    for (altered, reason) in [
        (
            saved.replace(
                "AutosarWorkbenchHostRestoreDidV1",
                "AutosarWorkbenchHostRestoreDidV2",
            ),
            "版本",
        ),
        (
            saved.replace(
                "<SD GID=\"SessionRef\">/Diag/DcmCfg/DcmConfigSet/DcmDsp/Sessions/Extended</SD>",
                "<SD GID=\"SessionRef\">/Diag/DcmCfg/DcmConfigSet/DcmDsp/Sessions/Default</SD>",
            ),
            "SessionRef",
        ),
        (saved.replace("<SD GID=\"Rid\">61441</SD>", ""), "Rid"),
        (saved.replacen(group, &format!("{group}{group}"), 1), "重复"),
    ] {
        assert_ne!(saved, altered);
        fs::write(&source, &altered).unwrap();
        let error = Workspace::open(vec![source.clone()], archive())
            .err()
            .expect("invalid host metadata must not be accepted");
        assert!(
            error.contains(reason) && error.contains("Diag.arxml"),
            "{error}"
        );
        assert_eq!(fs::read_to_string(&source).unwrap(), altered);
    }
}

#[cfg(windows)]
#[test]
fn generated_dcm_callbacks_link_for_independent_consumer_and_update_live_signals() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let first = project
        .add_signal(frame.clone(), "First".into(), 0, 32, 0x01020304)
        .unwrap();
    let second = project
        .add_signal(frame.clone(), "Second".into(), 32, 32, 0xaabbccdd)
        .unwrap();
    let paths = [
        first.signals.last().unwrap().path.clone(),
        second.signals.last().unwrap().path.clone(),
    ];
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: paths.to_vec(),
            write_enabled: true,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source_path = temp.0.join("Diag/Diag.arxml");
    let mut reopened = Workspace::open(vec![source_path.clone()], archive()).unwrap();
    reopened
        .update_frame(&frame, serde_json::json!({"periodMs": 110}))
        .unwrap();
    reopened.save().unwrap();
    let source = fs::read_to_string(&source_path).unwrap();
    let doc = roxmltree::Document::parse(&source).unwrap();
    let callbacks = |parameter: &str| -> Vec<String> {
        doc.descendants()
            .filter(|node| node.has_tag_name("ECUC-TEXTUAL-PARAM-VALUE"))
            .filter(|node| {
                node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text().is_some_and(|text| text.ends_with(parameter))
                })
            })
            .filter_map(|node| {
                node.children()
                    .find(|child| child.has_tag_name("VALUE"))
                    .and_then(|value| value.text())
                    .map(str::to_owned)
            })
            .collect()
    };
    let reads = callbacks("/DcmDspDataReadFnc");
    let writes = callbacks("/DcmDspDataWriteFnc");
    assert_eq!((reads.len(), writes.len()), (2, 2));
    let output = temp.0.join("GeneratedDiag");
    generator::generate(&mut reopened, &output).unwrap();
    generator::build(&output).unwrap();
    let report = host::run_diagnostic(&output).unwrap();
    assert!(
        report.passed,
        "generated ECU diagnostic smoke: {}",
        report.log
    );

    let consumer = temp.0.join("consumer.c");
    fs::write(&consumer, format!(r#"
#include <stdint.h>
#include <string.h>
#include "Dcm_Externals.h"
#include "Ecu_Runtime.h"
#include "Rte.h"

static EcuStatus sink(uint32_t id, uint8_t dlc, const uint8_t data[8])
{{
    (void)id;
    (void)dlc;
    (void)data;
    return ECU_OK;
}}

int main(void)
{{
    uint8_t data[4];
    uint8_t valid;
    uint32_t value;
    Dcm_NegativeResponseCodeType error_code = 0u;
    const uint8_t first_initial[4] = {{ 0x01u, 0x02u, 0x03u, 0x04u }};
    const uint8_t second_initial[4] = {{ 0xaau, 0xbbu, 0xccu, 0xddu }};
    const uint8_t written[4] = {{ 0x11u, 0x22u, 0x33u, 0x44u }};
    if (Ecu_Init(&Ecu_Config, sink, NULL, NULL, NULL) != ECU_OK) return 1;
    if ({read_first}(data) != E_OK || memcmp(data, first_initial, 4u) != 0) return 2;
    if ({read_second}(data) != E_OK || memcmp(data, second_initial, 4u) != 0) return 3;
    if ({write_first}(written, &error_code) != E_OK) return 4;
    if ({read_first}(data) != E_OK || memcmp(data, written, 4u) != 0) return 5;
    if (Rte_ReadSignal(0u, &value, &valid) != ECU_OK || value != 0x11223344u || valid != 1u) return 6;
    if (Rte_ReadSignal(1u, &value, &valid) != ECU_OK || value != 0xaabbccddu || valid != 1u) return 7;
    if ({write_second}(first_initial, &error_code) != E_OK) return 8;
    if (Rte_ReadSignal(1u, &value, &valid) != ECU_OK || value != 0x01020304u || valid != 1u) return 9;
    if ({write_first}(NULL, &error_code) != E_NOT_OK || error_code != 0x72u) return 10;
    return 0;
}}
"#, read_first = reads[0], read_second = reads[1], write_first = writes[0], write_second = writes[1])).unwrap();
    let cc = std::env::var_os("AUTOSAR_CC")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("gcc"));
    let mut sources: Vec<_> = fs::read_dir(output.join("src"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "c")
                && path.file_name().unwrap() != "ecu_host_main.c"
        })
        .collect();
    sources.sort();
    sources.push(output.join("Ecu_Config.c"));
    sources.push(consumer);
    let executable = temp.0.join("consumer.exe");
    let built = Command::new(cc)
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-I")
        .arg(output.join("include"))
        .arg("-I")
        .arg(&output)
        .args(sources)
        .arg("-o")
        .arg(&executable)
        .arg("-lbcrypt")
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "independent consumer link: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let observed = Command::new(executable).output().unwrap();
    assert!(
        observed.status.success(),
        "independent consumer failed at check {:?}: {}",
        observed.status.code(),
        String::from_utf8_lossy(&observed.stderr)
    );
}
