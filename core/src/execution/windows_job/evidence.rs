//! Best-effort observations; never used to decide closure or retain process handles.
use super::*;
use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};

#[repr(C)]
struct ProcessIds {
    assigned: u32,
    listed: u32,
    ids: [usize; 128],
}

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
    fn IsProcessInJob(process: *mut c_void, job: *mut c_void, result: *mut i32) -> i32;
    fn WaitForSingleObject(handle: *mut c_void, millis: u32) -> u32;
    fn GetExitCodeProcess(process: *mut c_void, code: *mut u32) -> i32;
    fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
}

fn time(value: &FileTime) -> u64 {
    (u64::from(value.high) << 32) | u64::from(value.low)
}

fn error(result: i32) -> Option<i32> {
    (result == 0).then(|| io::Error::last_os_error().raw_os_error().unwrap_or(-1))
}

fn cpu_model() -> Option<String> {
    #[cfg(target_arch = "x86_64")]
    {
        // Query the maximum extended leaf before reading the x86-64 brand string.
        if std::arch::x86_64::__cpuid(0x8000_0000).eax >= 0x8000_0004 {
            let mut bytes = Vec::with_capacity(48);
            for leaf in 0x8000_0002..=0x8000_0004 {
                let value = std::arch::x86_64::__cpuid(leaf);
                for word in [value.eax, value.ebx, value.ecx, value.edx] {
                    bytes.extend_from_slice(&word.to_le_bytes());
                }
            }
            return Some(
                String::from_utf8_lossy(&bytes)
                    .trim_matches(['\0', ' '])
                    .to_owned(),
            );
        }
    }
    std::env::var("PROCESSOR_IDENTIFIER").ok()
}

fn process(handle: *mut c_void, pid: u32, job: *mut c_void) -> Value {
    let mut times: [FileTime; 4] = std::array::from_fn(|_| FileTime::default());
    let [created, exited, kernel, user] = &mut times;
    // SAFETY: the borrowed process/job handles are live; all outputs are writable.
    let times_error = error(unsafe { GetProcessTimes(handle, created, exited, kernel, user) });
    let mut member = 0;
    let membership_error = error(unsafe { IsProcessInJob(handle, job, &mut member) });
    let wait = unsafe { WaitForSingleObject(handle, 0) };
    let wait_error = (wait == u32::MAX).then(|| io::Error::last_os_error().raw_os_error());
    let mut code = 0;
    let exit_error = error(unsafe { GetExitCodeProcess(handle, &mut code) });
    let mut image = [0u16; 32768];
    let mut length = image.len() as u32;
    let image_error =
        error(unsafe { QueryFullProcessImageNameW(handle, 0, image.as_mut_ptr(), &mut length) });
    json!({
        "pid": pid,
        "created_100ns": times_error.is_none().then(|| time(&times[0])),
        // Exit time is undefined until the process object is signaled.
        "exited_100ns": (times_error.is_none() && wait == 0).then(|| time(&times[1])),
        "kernel_100ns": times_error.is_none().then(|| time(&times[2])),
        "user_100ns": times_error.is_none().then(|| time(&times[3])),
        "times_error": times_error,
        "in_job": membership_error.is_none().then_some(member != 0),
        "membership_error": membership_error,
        "wait": wait, "wait_error": wait_error,
        "exit_code": exit_error.is_none().then_some(code), "exit_error": exit_error,
        "image": image_error.is_none().then(|| String::from_utf16_lossy(&image[..length as usize])),
        "image_error": image_error,
    })
}

fn member(pid: u32, job: *mut c_void) -> Value {
    // SAFETY: query-only access plus SYNCHRONIZE; no inheritance or mutation.
    let raw = unsafe { OpenProcess(0x0010_1000, 0, pid) };
    if raw.is_null() {
        return json!({"pid": pid, "open_error": io::Error::last_os_error().raw_os_error()});
    }
    // SAFETY: OpenProcess returned a new handle. Drop it before the next closure query;
    // retained references could themselves change the accounting being investigated.
    let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
    process(handle.as_raw_handle(), pid, job)
}

pub(super) struct Evidence {
    path: PathBuf,
    started: Instant,
    identities: parking_lot::Mutex<std::collections::BTreeSet<(u32, u64)>>,
    before_stop: parking_lot::Mutex<Option<Value>>,
}

impl Evidence {
    pub(super) fn new(path: PathBuf) -> Self {
        Self {
            path,
            started: Instant::now(),
            identities: parking_lot::Mutex::new(Default::default()),
            before_stop: parking_lot::Mutex::new(None),
        }
    }

    fn append(&self, record: Value) {
        if record["phase"] == "before_stop" {
            *self.before_stop.lock() = Some(record.clone());
        }
        let result = (|| -> io::Result<()> {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            serde_json::to_writer(&mut file, &record)?;
            file.write_all(b"\n")
        })();
        if let Err(error) = result {
            let _ = writeln!(
                io::stderr().lock(),
                "Windows Job evidence write failed path={}: {error}",
                self.path.display()
            );
        }
    }

    pub(super) fn observe(&self, phase: &str, child: &Child, job: &OwnedHandle) {
        let job_handle = job.as_raw_handle();
        let mut accounting = Accounting::default();
        // SAFETY: correctly sized C records and live borrowed handles.
        let accounting_error = error(unsafe {
            QueryInformationJobObject(
                job_handle,
                1,
                &mut accounting as *mut _ as *mut c_void,
                size_of::<Accounting>() as u32,
                std::ptr::null_mut(),
            )
        });
        let mut ids = ProcessIds {
            assigned: 0,
            listed: 0,
            ids: [0; 128],
        };
        let list_error = error(unsafe {
            QueryInformationJobObject(
                job_handle,
                3,
                &mut ids as *mut _ as *mut c_void,
                size_of::<ProcessIds>() as u32,
                std::ptr::null_mut(),
            )
        });
        // A bounded diagnostic snapshot must label truncation/races, never infer an empty tree.
        let members: Vec<_> = if list_error.is_none() {
            ids.ids[..(ids.listed as usize).min(ids.ids.len())]
                .iter()
                .map(|&pid| member(pid as u32, job_handle))
                .collect()
        } else {
            Vec::new()
        };
        let mut identities = self.identities.lock();
        for member in &members {
            if let (Some(pid), Some(created)) =
                (member["pid"].as_u64(), member["created_100ns"].as_u64())
            {
                identities.insert((pid as u32, created));
            }
        }
        let previously_observed: Vec<_> = identities
            .iter()
            .map(|&(pid, created)| {
                let current = member(pid, job_handle);
                json!({"pid": pid, "expected_created_100ns": created,
                "same_identity": current["created_100ns"].as_u64().map(|value| value == created),
                "current": current})
            })
            .collect();
        let mut cpu: [FileTime; 3] = std::array::from_fn(|_| FileTime::default());
        let [idle, kernel, user] = &mut cpu;
        let cpu_error = error(unsafe { GetSystemTimes(idle, kernel, user) });
        self.append(json!({
            "phase": phase, "elapsed_us": self.started.elapsed().as_micros(),
            "owner_pid": std::process::id(), "root_pid": child.id(),
            "job_handle": format!("{job_handle:p}"),
            "root": process(child.as_raw_handle(), child.id(), job_handle),
            "accounting_error": accounting_error,
            "accounting": {"active": accounting.active, "total": accounting.total,
                "terminated_by_limit": accounting.terminated, "times_100ns": accounting.times},
            "assigned": ids.assigned, "listed": ids.listed, "list_error": list_error,
            "list_complete": list_error.is_none() && ids.assigned == ids.listed && ids.listed <= 128,
            "members": members,
            "previously_observed": previously_observed,
            "cpu": {"model": cpu_model(),
                "logical_processors": std::env::var("NUMBER_OF_PROCESSORS").ok(),
                "idle_kernel_user_100ns": cpu.map(|value| time(&value)), "error": cpu_error},
        }));
    }

    pub(super) fn result(&self, result: &super::super::ProcessResult) {
        self.append(
            json!({"phase": "result", "elapsed_us": self.started.elapsed().as_micros(),
            "scope": result.scope, "pid": result.pid, "pgid": result.pgid,
            "status": format!("{:?}", result.status), "exit_code": result.exit_code,
            "descendants_reclaimed": result.descendants_reclaimed,
            "stdout": result.stdout, "stderr": result.stderr}),
        );
        if !result.success() {
            // Existing failure tails carry the pre-cleanup identities as well as the full-log path.
            let _ = (|| -> io::Result<()> {
                let mut stderr = std::fs::OpenOptions::new()
                    .append(true)
                    .open(&result.stderr)?;
                writeln!(stderr, "\nWindows Job evidence: {}", self.path.display())?;
                if let Some(record) = &*self.before_stop.lock() {
                    writeln!(stderr, "Windows Job before cleanup: {record}")?;
                }
                Ok(())
            })();
        }
    }
}

pub(super) fn result_path(stderr: &Path) -> PathBuf {
    stderr.with_file_name("windows-job.jsonl")
}
