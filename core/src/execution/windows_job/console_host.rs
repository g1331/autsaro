//! Console hosts that have exited but are still finishing their process teardown.
//!
//! The Job's active count drops only after Windows completes a member's teardown,
//! and a console host (`conhost.exe`, created for every `CREATE_NO_WINDOW` command)
//! can take far longer than its client: one CI run held four of them for 404 ms
//! after their clients exited. Unlike `evidence`, results here decide closure.
use super::*;
use std::collections::BTreeSet;

/// PID and creation time; the creation time excludes PID reuse.
pub(super) type Identity = (u32, u64);

/// The five-second budget `stop` already allows for members to leave after
/// `TerminateJobObject`; the command's own deadline still caps the wait.
pub(super) const TEARDOWN_LIMIT: Duration = Duration::from_secs(5);

const STILL_ACTIVE: u32 = 259;
const ERROR_INVALID_PARAMETER: i32 = 87;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
    fn GetProcessTimes(
        process: *mut c_void,
        created: *mut FileTime,
        exited: *mut FileTime,
        kernel: *mut FileTime,
        user: *mut FileTime,
    ) -> i32;
    fn QueryFullProcessImageNameW(
        process: *mut c_void,
        flags: u32,
        name: *mut u16,
        size: *mut u32,
    ) -> i32;
    fn GetExitCodeProcess(process: *mut c_void, code: *mut u32) -> i32;
}

/// What one member currently reports; `None` fields could not be read.
struct Member {
    created: Option<u64>,
    image: Option<String>,
    exit_code: Option<u32>,
}

/// Member PIDs, or `None` when the list is unavailable or truncated.
fn job_pids(job: &OwnedHandle) -> Option<Vec<u32>> {
    let mut ids = ProcessIds {
        assigned: 0,
        listed: 0,
        ids: [0; 128],
    };
    // SAFETY: correctly sized C record and live borrowed job handle.
    let listed = unsafe {
        QueryInformationJobObject(
            job.as_raw_handle(),
            3,
            &mut ids as *mut _ as *mut c_void,
            size_of::<ProcessIds>() as u32,
            std::ptr::null_mut(),
        )
    };
    (listed != 0 && ids.assigned == ids.listed && ids.listed as usize <= ids.ids.len()).then(|| {
        ids.ids[..ids.listed as usize]
            .iter()
            .map(|&pid| pid as u32)
            .collect()
    })
}

/// `Err` carries the OS error when the member cannot be opened.
fn inspect(pid: u32) -> Result<Member, i32> {
    // SAFETY: query-only access; no inheritance or mutation.
    let raw = unsafe { OpenProcess(0x0000_1000, 0, pid) };
    if raw.is_null() {
        return Err(io::Error::last_os_error().raw_os_error().unwrap_or(-1));
    }
    // SAFETY: OpenProcess returned a new handle; dropping it releases the only reference.
    let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
    let raw = handle.as_raw_handle();
    let mut times: [FileTime; 4] = std::array::from_fn(|_| FileTime::default());
    let [created, exited, kernel, user] = &mut times;
    // SAFETY: the process handle is live; all outputs are writable.
    let times_read = unsafe { GetProcessTimes(raw, created, exited, kernel, user) } != 0;
    let mut image = [0u16; 32768];
    let mut length = image.len() as u32;
    let image_read =
        unsafe { QueryFullProcessImageNameW(raw, 0, image.as_mut_ptr(), &mut length) } != 0;
    let mut code = 0;
    let code_read = unsafe { GetExitCodeProcess(raw, &mut code) } != 0;
    Ok(Member {
        created: times_read.then(|| (u64::from(times[0].high) << 32) | u64::from(times[0].low)),
        image: image_read.then(|| String::from_utf16_lossy(&image[..length as usize])),
        exit_code: code_read.then_some(code),
    })
}

fn is_system_host(image: &str) -> bool {
    std::env::var_os("SystemRoot").is_some_and(|root| {
        let host = std::path::Path::new(&root)
            .join("System32")
            .join("conhost.exe");
        host.to_string_lossy().eq_ignore_ascii_case(image)
    })
}

/// Identities of the Job members whose image is the system console host.
///
/// Reading the image of a process in teardown soon fails with access denied, so
/// callers record identities early and decide with them later.
pub(super) fn hosts(job: &OwnedHandle) -> BTreeSet<Identity> {
    job_pids(job)
        .into_iter()
        .flatten()
        .filter_map(|pid| {
            let member = inspect(pid).ok()?;
            let image = member.image?;
            is_system_host(&image).then_some((pid, member.created?))
        })
        .collect()
}

/// A console host exits when its last client disconnects, so an exited host
/// means every client in the Job is gone.
fn is_exited_host(pid: u32, member: &Result<Member, i32>, hosts: &BTreeSet<Identity>) -> bool {
    match member {
        // Left the Job between listing and opening.
        Err(ERROR_INVALID_PARAMETER) => true,
        Err(_) => false,
        Ok(member) => {
            member
                .created
                .is_some_and(|created| hosts.contains(&(pid, created)))
                && member.exit_code.is_some_and(|code| code != STILL_ACTIVE)
        }
    }
}

/// True when every remaining member is a known console host that has exited;
/// any other member, a running host or an unreadable list is a real leftover.
pub(super) fn only_exited_hosts(job: &OwnedHandle, hosts: &BTreeSet<Identity>) -> bool {
    job_pids(job).is_some_and(|pids| {
        pids.into_iter()
            .all(|pid| is_exited_host(pid, &inspect(pid), hosts))
    })
}

/// Wait up to `within` for the active count to reach zero while only exited
/// console hosts remain. `Ok(false)` reports a real leftover or the expired bound.
pub(super) fn wait_until_finished(
    mut active: impl FnMut() -> Result<u32, LocalizedText>,
    mut only_exited_hosts: impl FnMut() -> bool,
    within: Duration,
) -> Result<bool, LocalizedText> {
    let limit = Instant::now() + within;
    while active()? != 0 {
        if Instant::now() >= limit || !only_exited_hosts() {
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(created: Option<u64>, exit_code: Option<u32>) -> Result<Member, i32> {
        Ok(Member {
            created,
            image: None,
            exit_code,
        })
    }

    #[test]
    fn only_a_known_exited_host_of_the_same_identity_is_excused() {
        let hosts = BTreeSet::from([(632, 100)]);
        assert!(is_exited_host(632, &member(Some(100), Some(0)), &hosts));
        // Still running: its clients may be alive.
        assert!(!is_exited_host(
            632,
            &member(Some(100), Some(STILL_ACTIVE)),
            &hosts
        ));
        assert!(!is_exited_host(632, &member(Some(100), None), &hosts));
        // Not recorded as a console host, or the PID was reused.
        assert!(!is_exited_host(700, &member(Some(100), Some(0)), &hosts));
        assert!(!is_exited_host(632, &member(Some(101), Some(0)), &hosts));
        assert!(!is_exited_host(632, &member(None, Some(0)), &hosts));
        // Unreadable member; a member that vanished is not a leftover.
        assert!(!is_exited_host(632, &Err(5), &hosts));
        assert!(is_exited_host(632, &Err(ERROR_INVALID_PARAMETER), &hosts));
    }

    #[test]
    fn the_system_console_host_path_is_the_only_host_image() {
        let root = std::env::var("SystemRoot").unwrap();
        assert!(is_system_host(&format!("{root}\\System32\\conhost.exe")));
        assert!(is_system_host(&format!(
            "{}\\SYSTEM32\\CONHOST.EXE",
            root.to_lowercase()
        )));
        assert!(!is_system_host("C:\\Temp\\conhost.exe"));
        assert!(!is_system_host(&format!("{root}\\System32\\git.exe")));
    }

    #[test]
    fn teardown_lag_inside_the_bound_succeeds_and_outside_it_fails() {
        let started = Instant::now();
        let lagging = |lag: Duration| move || Ok(u32::from(started.elapsed() < lag));
        let lag = Duration::from_millis(100);
        assert!(wait_until_finished(lagging(lag), || true, Duration::from_secs(2)).unwrap());
        assert!(
            !wait_until_finished(lagging(Duration::from_secs(60)), || true, lag).unwrap(),
            "a host that never leaves must still fail at the bound"
        );
    }

    #[test]
    fn a_real_leftover_fails_without_waiting_for_the_bound() {
        let started = Instant::now();
        assert!(!wait_until_finished(|| Ok(1), || false, Duration::from_secs(30)).unwrap());
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_real_console_host_is_recognised_while_its_client_runs() {
        let scratch = crate::generator::output::reserve_directory(
            &std::env::temp_dir(),
            "console-host",
            std::ffi::OsStr::new("recognised"),
        )
        .unwrap();
        let mut command = Command::new(std::env::var_os("AUTOSAR_PYTHON").unwrap());
        command
            .args(["-I", "-S", "-c", "import sys; sys.stdin.buffer.read()"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let mut tree = ProcessTree::spawn(&mut command, &scratch.join("stderr.log")).unwrap();
        let end = Instant::now() + Duration::from_secs(8);
        let mut found = hosts(&tree.job);
        while found.is_empty() {
            assert!(Instant::now() < end, "no console host appeared in the Job");
            std::thread::sleep(Duration::from_millis(10));
            found = hosts(&tree.job);
        }
        // The Python client still runs, so the Job holds more than exited hosts.
        assert!(!only_exited_hosts(&tree.job, &found));
        tree.close_stdin();
        tree.stop().unwrap();
        drop(tree);
        std::fs::remove_dir_all(scratch).unwrap();
    }
}
