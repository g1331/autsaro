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
    fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
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

/// Path of the system console host, `None` when the system directory cannot be read.
fn system_host() -> Option<String> {
    let mut directory = [0u16; 260];
    // SAFETY: the buffer is writable for the stated number of UTF-16 units.
    let length = unsafe { GetSystemDirectoryW(directory.as_mut_ptr(), directory.len() as u32) };
    let length = length as usize;
    (0 < length && length < directory.len()).then(|| {
        format!(
            "{}\\conhost.exe",
            String::from_utf16_lossy(&directory[..length])
        )
    })
}

/// An unreadable system directory (`None`) matches no image.
fn is_system_host(image: &str, system_host: Option<&str>) -> bool {
    system_host.is_some_and(|host| host.eq_ignore_ascii_case(image))
}

/// Identities of the Job members whose image is the system console host.
///
/// Reading the image of a process in teardown soon fails with access denied, so
/// callers record identities early and decide with them later.
pub(super) fn hosts(job: &OwnedHandle) -> BTreeSet<Identity> {
    let system_host = system_host();
    job_pids(job)
        .into_iter()
        .flatten()
        .filter_map(|pid| {
            let member = inspect(pid).ok()?;
            let image = member.image?;
            is_system_host(&image, system_host.as_deref()).then_some((pid, member.created?))
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

/// Wait until the active count reaches zero while only exited console hosts
/// remain. Every observation is judged against `limit_ns`, so a Job first seen
/// empty after it still fails; `Ok(false)` reports a real leftover or the limit.
pub(super) fn wait_until_finished(
    mut active: impl FnMut() -> Result<u32, LocalizedText>,
    mut only_exited_hosts: impl FnMut() -> bool,
    mut now_ns: impl FnMut() -> u64,
    mut pause: impl FnMut(Duration),
    limit_ns: u64,
) -> Result<bool, LocalizedText> {
    loop {
        let active = active()?;
        if now_ns() >= limit_ns {
            return Ok(false);
        }
        if active == 0 {
            return Ok(true);
        }
        if !only_exited_hosts() {
            return Ok(false);
        }
        let remaining = limit_ns.saturating_sub(now_ns());
        if remaining == 0 {
            return Ok(false);
        }
        pause(Duration::from_nanos(remaining).min(Duration::from_millis(10)));
    }
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
    fn only_the_system_console_host_path_is_a_host_image() {
        let system = Some("C:\\Windows\\System32\\conhost.exe");
        assert!(is_system_host("C:\\Windows\\System32\\conhost.exe", system));
        assert!(is_system_host("c:\\WINDOWS\\system32\\CONHOST.EXE", system));
        assert!(!is_system_host("C:\\Temp\\conhost.exe", system));
        assert!(!is_system_host("C:\\Windows\\System32\\git.exe", system));
        // An unreadable system directory matches nothing.
        assert!(!is_system_host("C:\\Windows\\System32\\conhost.exe", None));
    }

    #[test]
    fn the_system_directory_comes_from_the_api_not_the_environment() {
        let root = std::env::var("SystemRoot").unwrap();
        let host = system_host().expect("system directory");
        assert!(host.eq_ignore_ascii_case(&format!("{root}\\System32\\conhost.exe")));
    }

    /// A simulated clock that only moves when the waiter pauses or a probe costs time.
    struct Clock(std::cell::Cell<u64>);

    thread_local!(static PAUSES: std::cell::Cell<u32> = const { std::cell::Cell::new(0) });

    fn run(
        clock: &Clock,
        gone_at_ns: u64,
        only_exited_hosts: bool,
        probe_cost_ns: u64,
        stall_ns: u64,
        limit_ns: u64,
    ) -> bool {
        wait_until_finished(
            || Ok(u32::from(clock.0.get() < gone_at_ns)),
            || {
                clock.0.set(clock.0.get() + probe_cost_ns);
                only_exited_hosts
            },
            || clock.0.get(),
            |pause| {
                PAUSES.with(|pauses| pauses.set(pauses.get() + 1));
                assert!(
                    PAUSES.with(|pauses| pauses.get()) < 5_000,
                    "wait did not terminate"
                );
                clock
                    .0
                    .set(clock.0.get() + pause.as_nanos() as u64 + stall_ns)
            },
            limit_ns,
        )
        .unwrap()
    }

    const MS: u64 = 1_000_000;

    #[test]
    fn teardown_lag_inside_the_bound_succeeds_and_outside_it_fails() {
        assert!(run(&Clock(0.into()), 404 * MS, true, 0, 0, 5_000 * MS));
        assert!(!run(&Clock(0.into()), 6_000 * MS, true, 0, 0, 5_000 * MS));
    }

    #[test]
    fn a_real_leftover_fails_at_once() {
        let clock = Clock(0.into());
        assert!(!run(&clock, u64::MAX, false, 0, 0, 5_000 * MS));
        assert_eq!(clock.0.get(), 0);
    }

    #[test]
    fn a_job_first_seen_empty_after_the_limit_still_fails() {
        // Limit 30 ms, the Job empties at 31 ms.
        assert!(!run(&Clock(0.into()), 31 * MS, true, 0, 0, 30 * MS));
        // A probe that takes 40 ms and a pause that overshoots by 1 s.
        assert!(!run(&Clock(0.into()), 30 * MS, true, 40 * MS, 0, 20 * MS));
        assert!(!run(
            &Clock(0.into()),
            404 * MS,
            true,
            0,
            1_000 * MS,
            300 * MS
        ));
        // The same lag inside the limit succeeds.
        assert!(run(&Clock(0.into()), 30 * MS, true, 0, 0, 300 * MS));
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
