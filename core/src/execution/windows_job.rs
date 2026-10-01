//! Own a hidden command and all descendants before its first instruction runs.
//! The job handle is private and non-inheritable; closing it also kills the tree.
use std::ffi::c_void;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, ExitStatus};
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Default)]
struct BasicLimits {
    process_time: i64,
    job_time: i64,
    flags: u32,
    minimum: usize,
    maximum: usize,
    active: u32,
    affinity: usize,
    priority: u32,
    scheduling: u32,
}
#[repr(C)]
#[derive(Default)]
struct ExtendedLimits {
    basic: BasicLimits,
    io: [u64; 6],
    process_memory: usize,
    job_memory: usize,
    peak_process: usize,
    peak_job: usize,
}
#[repr(C)]
#[derive(Default)]
struct Accounting {
    times: [i64; 4],
    faults: u32,
    total: u32,
    active: u32,
    terminated: u32,
}
#[repr(C)]
#[derive(Default)]
struct ThreadEntry {
    size: u32,
    usage: u32,
    thread: u32,
    process: u32,
    priority: i32,
    delta: i32,
    flags: u32,
}
#[repr(C)]
#[derive(Default)]
struct FileTime {
    low: u32,
    high: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> *mut c_void;
    fn SetInformationJobObject(job: *mut c_void, class: i32, data: *const c_void, size: u32)
    -> i32;
    fn AssignProcessToJobObject(job: *mut c_void, process: *mut c_void) -> i32;
    fn TerminateJobObject(job: *mut c_void, code: u32) -> i32;
    fn QueryInformationJobObject(
        job: *mut c_void,
        class: i32,
        data: *mut c_void,
        size: u32,
        returned: *mut u32,
    ) -> i32;
    fn CreateToolhelp32Snapshot(flags: u32, process: u32) -> *mut c_void;
    fn Thread32First(snapshot: *mut c_void, entry: *mut ThreadEntry) -> i32;
    fn Thread32Next(snapshot: *mut c_void, entry: *mut ThreadEntry) -> i32;
    fn OpenThread(access: u32, inherit: i32, id: u32) -> *mut c_void;
    fn GetThreadTimes(
        thread: *mut c_void,
        created: *mut FileTime,
        exited: *mut FileTime,
        kernel: *mut FileTime,
        user: *mut FileTime,
    ) -> i32;
    fn ResumeThread(thread: *mut c_void) -> u32;
}

fn checked(result: i32, operation: &str) -> Result<(), String> {
    if result == 0 {
        Err(format!("{operation}: {}", io::Error::last_os_error()))
    } else {
        Ok(())
    }
}
fn owned(raw: *mut c_void, operation: &str) -> Result<OwnedHandle, String> {
    if raw.is_null() || raw == (-1isize as *mut c_void) {
        return Err(format!("{operation}: {}", io::Error::last_os_error()));
    }
    // SAFETY: successful Win32 calls return a new, exclusively owned handle.
    Ok(unsafe { OwnedHandle::from_raw_handle(raw) })
}

fn resume_primary(process: u32) -> Result<(), String> {
    // ChildExt::main_thread_handle is nightly-only. Toolhelp plus creation time
    // finds the original (earliest) thread while the new process is suspended.
    // SAFETY: the snapshot and output records have the documented Win32 layout.
    let snapshot = owned(unsafe { CreateToolhelp32Snapshot(4, 0) }, "Thread snapshot")?;
    let mut entry = ThreadEntry {
        size: size_of::<ThreadEntry>() as u32,
        ..Default::default()
    };
    let mut primary: Option<(u64, OwnedHandle)> = None;
    // SAFETY: entry is writable and its size is set before enumeration.
    checked(
        unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) },
        "First thread",
    )?;
    loop {
        if entry.process == process {
            // SAFETY: only the newly created process's thread is opened;
            // query-time and suspend/resume rights do not grant tree-wide access.
            let thread = owned(
                unsafe { OpenThread(0x42, 0, entry.thread) },
                "Open primary thread",
            )?;
            let mut times: [FileTime; 4] = std::array::from_fn(|_| FileTime::default());
            let [created_time, exited, kernel, user] = &mut times;
            // SAFETY: four independent writable FILETIME records are supplied.
            checked(
                unsafe {
                    GetThreadTimes(thread.as_raw_handle(), created_time, exited, kernel, user)
                },
                "Thread creation time",
            )?;
            let created = (u64::from(times[0].high) << 32) | u64::from(times[0].low);
            if primary.as_ref().is_none_or(|(prior, _)| created < *prior) {
                primary = Some((created, thread));
            }
        }
        entry.size = size_of::<ThreadEntry>() as u32;
        // SAFETY: the snapshot is live and entry remains writable.
        if unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) } == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(18) {
                return Err(format!("Next thread: {error}"));
            }
            break;
        }
    }
    let (_, thread) = primary.ok_or("Suspended command has no primary thread")?;
    // SAFETY: the primary thread belongs to this live, suspended child.
    let prior = unsafe { ResumeThread(thread.as_raw_handle()) };
    if prior != 1 {
        return Err(format!(
            "Resume primary thread returned {prior}: {}",
            io::Error::last_os_error()
        ));
    }
    Ok(())
}

pub(crate) struct ProcessTree {
    child: Child,
    job: OwnedHandle,
}
impl ProcessTree {
    pub(crate) fn spawn(command: &mut Command) -> Result<Self, String> {
        // SAFETY: null attributes produce a private, non-inheritable job handle.
        let job = owned(
            unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) },
            "Create job",
        )?;
        let mut limits = ExtendedLimits::default();
        limits.basic.flags = 0x2000; // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        // SAFETY: limits has the documented extended-limit layout and size.
        checked(
            unsafe {
                SetInformationJobObject(
                    job.as_raw_handle(),
                    9,
                    &limits as *const _ as *const c_void,
                    size_of::<ExtendedLimits>() as u32,
                )
            },
            "Configure private job",
        )?;
        Self::spawn_in_job(command, job)
    }
    fn spawn_in_job(command: &mut Command, job: OwnedHandle) -> Result<Self, String> {
        let child = command
            .creation_flags(0x08000004)
            .spawn()
            .map_err(|e| e.to_string())?;
        let mut tree = Self { child, job };
        // SAFETY: both handles are live; the child's primary thread is suspended.
        if let Err(error) = checked(
            unsafe {
                AssignProcessToJobObject(tree.job.as_raw_handle(), tree.child.as_raw_handle())
            },
            "Assign suspended command",
        ) {
            tree.child.kill().map_err(|cleanup| {
                format!("{error}; suspended parent cleanup failed: {cleanup}")
            })?;
            tree.child
                .wait()
                .map_err(|cleanup| format!("{error}; suspended parent wait failed: {cleanup}"))?;
            return Err(error);
        }
        if let Err(error) = resume_primary(tree.child.id()) {
            tree.stop()
                .map_err(|cleanup| format!("{error}; {cleanup}"))?;
            return Err(error);
        }
        Ok(tree)
    }
    pub(crate) fn close_stdin(&mut self) {
        drop(self.child.stdin.take());
    }

    pub(crate) fn write_stdin(&mut self, bytes: &[u8], deadline_ns: u64) -> Result<(), String> {
        use std::io::Write;
        let remaining = deadline_ns.saturating_sub(super::monotonic_ns()?);
        if remaining == 0 {
            return Err("Interactive stdin deadline expired".into());
        }
        let mut pipe = self
            .child
            .stdin
            .take()
            .ok_or("Interactive stdin is not open")?;
        // A Windows anonymous-pipe write can block. The transferred buffer must
        // remain owned by the writer if OS cleanup cannot be confirmed.
        let bytes = bytes.to_vec();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let writer = std::thread::spawn(move || {
            let result = pipe.write_all(&bytes).and_then(|_| pipe.flush());
            let _ = sender.send((pipe, result));
        });
        match receiver.recv_timeout(std::time::Duration::from_nanos(remaining)) {
            Ok((pipe, result)) => {
                self.child.stdin = Some(pipe);
                writer
                    .join()
                    .map_err(|_| "Interactive stdin writer panicked")?;
                result.map_err(|error| error.to_string())
            }
            Err(error) => {
                self.stop()
                    .map_err(|cleanup| format!("Interactive stdin failed: {error}; {cleanup}"))?;
                writer
                    .join()
                    .map_err(|_| "Interactive stdin writer panicked during closure")?;
                Err(format!(
                    "Interactive stdin exceeded its absolute deadline: {error}"
                ))
            }
        }
    }
    pub(crate) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }
    pub(crate) fn id(&self) -> u32 {
        self.child.id()
    }
    pub(crate) fn active(&self) -> Result<u32, String> {
        let mut accounting = Accounting::default();
        // SAFETY: writable accounting record and live job handle.
        checked(
            unsafe {
                QueryInformationJobObject(
                    self.job.as_raw_handle(),
                    1,
                    &mut accounting as *mut _ as *mut c_void,
                    size_of::<Accounting>() as u32,
                    std::ptr::null_mut(),
                )
            },
            "Observe command tree shutdown",
        )?;
        Ok(accounting.active)
    }
    pub(crate) fn stop(&mut self) -> Result<(), String> {
        // SAFETY: the private job contains only this command and its descendants.
        checked(
            unsafe { TerminateJobObject(self.job.as_raw_handle(), 1) },
            "Terminate command tree",
        )?;
        let started = Instant::now();
        loop {
            if self.active()? == 0 {
                self.child
                    .wait()
                    .map_err(|e| format!("Wait for closed command: {e}"))?;
                return Ok(());
            }
            if started.elapsed() >= Duration::from_secs(5) {
                return Err("Command descendant shutdown could not be confirmed within 5s.".into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for ProcessTree {
    fn drop(&mut self) {
        // SAFETY: best-effort termination on exceptional paths; closing the
        // non-inheritable kill-on-close job is an additional kernel fallback.
        unsafe { TerminateJobObject(self.job.as_raw_handle(), 1) };
        let _ = self.child.kill();
        let _ = self.child.try_wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn DuplicateHandle(
            source: *mut c_void,
            handle: *mut c_void,
            target: *mut c_void,
            duplicate: *mut *mut c_void,
            access: u32,
            inherit: i32,
            options: u32,
        ) -> i32;
    }
    #[test]
    fn job_assignment_failure_closes_parent_before_command_execution() {
        let scratch = crate::generator::reserve_directory(
            &std::env::temp_dir(),
            "job-rejection",
            std::ffi::OsStr::new("private"),
        )
        .unwrap();
        let marker = scratch.join("must-not-run.txt");
        // A real query-only job handle makes assignment fail with access denied.
        // SAFETY: both job handles are exclusively owned; pseudo process handles
        // are borrowed and must not be closed.
        let job = owned(
            unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) },
            "Create test job",
        )
        .unwrap();
        let mut duplicate = std::ptr::null_mut();
        checked(
            unsafe {
                DuplicateHandle(
                    GetCurrentProcess(),
                    job.as_raw_handle(),
                    GetCurrentProcess(),
                    &mut duplicate,
                    4,
                    0,
                    0,
                )
            },
            "Query-only job",
        )
        .unwrap();
        let limited = owned(duplicate, "Duplicate test job").unwrap();
        let mut command = Command::new("powershell.exe");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "Set-Content -LiteralPath '{}' -Value executed",
                marker.display().to_string().replace('\'', "''")
            ),
        ]);
        let error = ProcessTree::spawn_in_job(&mut command, limited)
            .err()
            .unwrap();
        assert!(error.contains("Assign suspended command"), "{error}");
        assert!(
            !marker.exists(),
            "Rejected command ran before job assignment"
        );
        std::fs::remove_dir(scratch).unwrap();
    }
}
