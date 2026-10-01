use super::{ProcessOwner, ProcessSpec, ProcessStatus, run_bounded};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static FIXTURE_NONCE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "autosar-native-owner-{}-{}-{}",
            std::process::id(),
            super::monotonic_ns().expect("monotonic clock"),
            FIXTURE_NONCE.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir(&path).expect("private fixture directory");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
                .expect("private fixture permissions");
        }
        Self(path)
    }

    fn spec(&self, kind: &str, duration: Duration) -> ProcessSpec {
        let python = std::env::var_os("AUTOSAR_PYTHON")
            .expect("AUTOSAR_PYTHON must name the locked virtualenv interpreter");
        let argv: Vec<OsString> = [
            python,
            "-m".into(),
            "autosar_tooling".into(),
            "probe".into(),
            "descendant".into(),
            "--pid-file".into(),
            self.0.join("pids.txt").into_os_string(),
            "--kind".into(),
            kind.into(),
        ]
        .into();
        let scripts = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("scripts");
        ProcessSpec::for_duration(
            argv,
            scripts.clone(),
            vec![("PYTHONPATH".into(), scripts.into_os_string())],
            duration,
            self.0.clone(),
        )
        .expect("valid spec")
    }

    fn descendants(&self) -> Vec<u32> {
        std::fs::read_to_string(self.0.join("pids.txt"))
            .expect("probe registered descendants")
            .lines()
            .filter_map(|line| line.split_ascii_whitespace().nth(1))
            .map(|pid| pid.parse().expect("decimal PID"))
            .collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("fixture outputs closed");
    }
}

#[cfg(unix)]
fn alive(pid: u32) -> bool {
    // SAFETY: signal zero reads existence of a fixture-owned process.
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(windows)]
fn alive(pid: u32) -> bool {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn WaitForSingleObject(handle: *mut c_void, millis: u32) -> u32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    // SAFETY: only fixture PID is opened with SYNCHRONIZE; handle is closed.
    let handle = unsafe { OpenProcess(0x0010_0000, 0, pid) };
    if handle.is_null() {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(87) {
            return false;
        }
        panic!("Cannot inspect fixture PID {pid}: {error}");
    }
    let outcome = unsafe { WaitForSingleObject(handle, 0) };
    unsafe { CloseHandle(handle) };
    assert_ne!(outcome, 0xFFFF_FFFF, "Cannot wait for fixture PID {pid}");
    outcome == 0x102
}

fn assert_descendants_gone(fixture: &Fixture, count: usize) {
    let pids = fixture.descendants();
    assert!(
        pids.len() >= count,
        "expected {count} fixture PIDs, got {pids:?}"
    );
    let end = std::time::Instant::now() + Duration::from_secs(2);
    for pid in pids {
        while alive(pid) && std::time::Instant::now() < end {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!alive(pid), "fixture PID {pid} survived scope close");
    }
}

#[test]
fn normal_and_nonzero_preserve_logs_and_exit_code() {
    let fixture = Fixture::new();
    let result = run_bounded(fixture.spec("normal", Duration::from_secs(8))).unwrap();
    assert_eq!(result.status, ProcessStatus::Exited);
    assert_eq!(result.exit_code, Some(0));
    assert_descendants_gone(&fixture, 3);

    let result = run_bounded(fixture.spec("fail", Duration::from_secs(8))).unwrap_err();
    assert!(result.contains("exit_code=Some(7)"), "{result}");
    assert!(result.contains("stderr.log"), "{result}");
}

#[test]
fn parent_first_and_timeout_close_the_registered_tree() {
    for (kind, duration, expected) in [
        (
            "parent-first",
            Duration::from_secs(8),
            ProcessStatus::OrphanedMembers,
        ),
        ("hang", Duration::from_millis(500), ProcessStatus::Timeout),
    ] {
        let fixture = Fixture::new();
        let owner = ProcessOwner::new().unwrap();
        let mut process = owner.spawn(fixture.spec(kind, duration), None).unwrap();
        let result = process.wait().unwrap();
        assert_eq!(result.status, expected, "{kind}");
        assert_descendants_gone(&fixture, 3);
    }
}

#[test]
fn cancel_and_nested_scopes_close_without_stale_members() {
    let fixture = Fixture::new();
    let owner = ProcessOwner::new().unwrap();
    let mut process = owner
        .spawn(fixture.spec("hang", Duration::from_secs(8)), None)
        .unwrap();
    let sibling_fixture = Fixture::new();
    let mut sibling = owner
        .spawn(sibling_fixture.spec("normal", Duration::from_secs(8)), None)
        .unwrap();
    let end = std::time::Instant::now() + Duration::from_secs(5);
    while fixture.descendants_if_present().len() < 3 && std::time::Instant::now() < end {
        std::thread::sleep(Duration::from_millis(10));
    }
    let result = process.cancel().unwrap();
    assert_eq!(result.status, ProcessStatus::Cancelled);
    assert_descendants_gone(&fixture, 3);
    assert!(sibling.wait().unwrap().success());
    assert_descendants_gone(&sibling_fixture, 3);

    let nested = Fixture::new();
    let result = run_bounded(nested.spec("nested", Duration::from_secs(8))).unwrap();
    assert_eq!(result.status, ProcessStatus::Exited);
    assert_descendants_gone(&nested, 2);
}

#[test]
fn rejected_registration_never_executes_a_command() {
    let fixture = Fixture::new();
    let owner = ProcessOwner::new().unwrap();
    let mut spec = fixture.spec("normal", Duration::from_secs(8));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let public = fixture.0.join("public");
        std::fs::create_dir(&public).unwrap();
        std::fs::set_permissions(&public, std::fs::Permissions::from_mode(0o755)).unwrap();
        spec.log_directory = public;
    }
    #[cfg(windows)]
    {
        spec.argv[0] = fixture.0.join("missing-executable.exe").into_os_string();
    }
    assert!(owner.spawn(spec, None).is_err());
    assert!(!fixture.0.join("pids.txt").exists());
}

#[cfg(unix)]
#[test]
fn unexpected_supervisor_death_closes_guardian_mirror() {
    let fixture = Fixture::new();
    let owner = ProcessOwner::new().unwrap();
    let root = std::sync::Arc::clone(&owner.owner);
    let mut process = owner
        .spawn(fixture.spec("hang", Duration::from_secs(8)), None)
        .unwrap();
    let end = std::time::Instant::now() + Duration::from_secs(5);
    while fixture.descendants_if_present().len() < 3 && std::time::Instant::now() < end {
        std::thread::sleep(Duration::from_millis(10));
    }
    root.kill_supervisor_for_test().unwrap();
    let failure = process.wait().unwrap_err();
    assert!(failure.contains("supervisor_failed"), "{failure}");
    assert_descendants_gone(&fixture, 3);
    drop(process);
    drop(owner);
    root.remove_failed_test_logs();
}

#[cfg(unix)]
#[test]
fn unregistered_escape_never_reports_a_clean_tree() {
    let fixture = Fixture::new();
    let owner = ProcessOwner::new().unwrap();
    let mut process = owner
        .spawn(fixture.spec("escape", Duration::from_secs(8)), None)
        .unwrap();
    let result = process.wait();
    let records = std::fs::read_to_string(fixture.0.join("pids.txt")).unwrap();
    let escape_pid: i32 = records
        .lines()
        .find_map(|line| line.strip_prefix("escape "))
        .and_then(|record| record.split_ascii_whitespace().next())
        .unwrap()
        .parse()
        .unwrap();
    // SAFETY: the fixture records this deliberately non-cooperative process.
    unsafe { libc::kill(escape_pid, libc::SIGKILL) };
    assert_eq!(result.unwrap().status, ProcessStatus::CleanupUnconfirmed);
}

impl Fixture {
    fn descendants_if_present(&self) -> Vec<u32> {
        if self.0.join("pids.txt").exists() {
            self.descendants()
        } else {
            Vec::new()
        }
    }
}
