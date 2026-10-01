//! One bounded argument-vector execution seam for native commands.
//!
//! All descendants within a registered cooperative scope are closed before a
//! result is published. POSIX uses a short-lived supervisor; Windows reuses
//! the suspended-create, non-breakaway, kill-on-close Job implementation.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows_job;

#[cfg(windows)]
pub(crate) use windows_job::ProcessTree;

#[cfg(test)]
mod tests;

/// The monotonic deadline is absolute; child operations cannot reset it.
pub struct ProcessSpec {
    pub argv: Vec<OsString>,
    pub cwd: PathBuf,
    pub env: Vec<(OsString, OsString)>,
    pub deadline_ns: u64,
    pub log_directory: PathBuf,
}

impl ProcessSpec {
    pub fn for_duration(
        argv: Vec<OsString>,
        cwd: PathBuf,
        env: Vec<(OsString, OsString)>,
        duration: Duration,
        log_directory: PathBuf,
    ) -> Result<Self, String> {
        let deadline_ns = monotonic_ns()?
            .checked_add(
                u64::try_from(duration.as_nanos())
                    .map_err(|_| "Process deadline duration exceeds u64 nanoseconds")?,
            )
            .ok_or("Process deadline overflow")?;
        Ok(Self {
            argv,
            cwd,
            env,
            deadline_ns,
            log_directory,
        })
    }

    fn validate(&self) -> Result<(), String> {
        if self.argv.is_empty() || !std::path::Path::new(&self.argv[0]).is_absolute() {
            return Err("ProcessSpec requires an absolute executable argv".into());
        }
        if self.deadline_ns <= monotonic_ns()? {
            return Err("Process deadline expired before launch".into());
        }
        if !self.cwd.is_dir() || !self.log_directory.is_dir() {
            return Err("Process working/log directory is missing".into());
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProcessStatus {
    Exited,
    Timeout,
    Cancelled,
    OrphanedMembers,
    CleanupUnconfirmed,
}

impl ProcessStatus {
    #[cfg(unix)]
    fn from_owner(value: &str) -> Result<Self, String> {
        match value {
            "exited" => Ok(Self::Exited),
            "timeout" => Ok(Self::Timeout),
            "cancelled" => Ok(Self::Cancelled),
            "orphaned_members" => Ok(Self::OrphanedMembers),
            "cleanup_unconfirmed" => Ok(Self::CleanupUnconfirmed),
            _ => Err(format!("Unknown supervisor closure status: {value}")),
        }
    }
}

#[derive(Debug)]
pub struct ProcessResult {
    pub scope: String,
    pub pid: u32,
    pub pgid: Option<i32>,
    pub exit_code: Option<i32>,
    pub status: ProcessStatus,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
}

impl ProcessResult {
    pub fn success(&self) -> bool {
        self.status == ProcessStatus::Exited && self.exit_code == Some(0)
    }
}

/// A root owner may launch independent sibling or nested scopes.
pub struct ProcessOwner {
    #[cfg(unix)]
    owner: std::sync::Arc<unix::UnixOwner>,
}

impl ProcessOwner {
    pub fn new() -> Result<Self, String> {
        #[cfg(unix)]
        {
            return Ok(Self {
                owner: unix::UnixOwner::start()?,
            });
        }
        #[cfg(windows)]
        {
            Ok(Self {})
        }
    }

    pub fn spawn(&self, spec: ProcessSpec, parent: Option<&str>) -> Result<OwnedProcess, String> {
        spec.validate()?;
        #[cfg(unix)]
        {
            let process = self.owner.spawn(&spec, parent)?;
            Ok(OwnedProcess {
                backend: Backend::Unix(process),
            })
        }
        #[cfg(windows)]
        {
            let _ = parent; // Nested Windows Jobs remain contained by the root Job.
            let capture = private_capture(&spec.log_directory)?;
            let stdout = capture.join("stdout.log");
            let stderr = capture.join("stderr.log");
            let mut command = std::process::Command::new(&spec.argv[0]);
            command
                .args(&spec.argv[1..])
                .current_dir(&spec.cwd)
                .envs(spec.env.iter().cloned());
            command.stdin(std::process::Stdio::null());
            command.stdout(std::process::Stdio::from(
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&stdout)
                    .map_err(|error| error.to_string())?,
            ));
            command.stderr(std::process::Stdio::from(
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&stderr)
                    .map_err(|error| error.to_string())?,
            ));
            let tree = ProcessTree::spawn(&mut command)?;
            let pid = tree.id();
            Ok(OwnedProcess {
                spec,
                backend: Backend::Windows {
                    tree,
                    stdout,
                    stderr,
                    pid,
                    finished: false,
                },
            })
        }
    }
}

#[cfg(windows)]
fn private_capture(directory: &std::path::Path) -> Result<PathBuf, String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NONCE: AtomicU64 = AtomicU64::new(0);
    for _ in 0..100 {
        let path = directory.join(format!(
            "owned-command-{}-{}",
            monotonic_ns()?,
            NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("Cannot reserve private process log directory".into())
}

enum Backend {
    #[cfg(unix)]
    Unix(unix::UnixProcess),
    #[cfg(windows)]
    Windows {
        tree: ProcessTree,
        stdout: PathBuf,
        stderr: PathBuf,
        pid: u32,
        finished: bool,
    },
}

pub struct OwnedProcess {
    #[cfg(windows)]
    spec: ProcessSpec,
    backend: Backend,
}

impl OwnedProcess {
    pub fn wait(&mut self) -> Result<ProcessResult, String> {
        #[cfg(unix)]
        {
            let Backend::Unix(process) = &mut self.backend;
            process.wait()
        }
        #[cfg(windows)]
        {
            let Backend::Windows {
                tree,
                stdout,
                stderr,
                pid,
                finished,
            } = &mut self.backend;
            if *finished {
                return Err("Owned process already completed".into());
            }
            let status = loop {
                if let Some(status) = tree.try_wait().map_err(|error| error.to_string())? {
                    // The Job's active count may lag a just-reaped cooperative exit.
                    let grace = std::time::Instant::now() + Duration::from_millis(200);
                    while tree.active()? != 0 && std::time::Instant::now() < grace {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    let orphaned = tree.active()? != 0;
                    tree.stop()?;
                    break (
                        if orphaned {
                            ProcessStatus::OrphanedMembers
                        } else {
                            ProcessStatus::Exited
                        },
                        status.code(),
                    );
                }
                if monotonic_ns()? >= self.spec.deadline_ns {
                    tree.stop()?;
                    break (ProcessStatus::Timeout, None);
                }
                std::thread::sleep(Duration::from_millis(20));
            };
            *finished = true;
            Ok(ProcessResult {
                scope: format!("windows-job-{pid}"),
                pid: *pid,
                pgid: None,
                exit_code: status.1,
                status: status.0,
                stdout: stdout.clone(),
                stderr: stderr.clone(),
            })
        }
    }

    pub fn cancel(&mut self) -> Result<ProcessResult, String> {
        #[cfg(unix)]
        {
            let Backend::Unix(process) = &mut self.backend;
            process.cancel()
        }
        #[cfg(windows)]
        {
            let Backend::Windows {
                tree,
                stdout,
                stderr,
                pid,
                finished,
            } = &mut self.backend;
            if *finished {
                return Err("Owned process already completed".into());
            }
            tree.stop()?;
            *finished = true;
            Ok(ProcessResult {
                scope: format!("windows-job-{pid}"),
                pid: *pid,
                pgid: None,
                exit_code: tree
                    .try_wait()
                    .map_err(|error| error.to_string())?
                    .and_then(|status| status.code()),
                status: ProcessStatus::Cancelled,
                stdout: stdout.clone(),
                stderr: stderr.clone(),
            })
        }
    }
}

pub fn run_bounded(spec: ProcessSpec) -> Result<ProcessResult, String> {
    let owner = ProcessOwner::new()?;
    let mut process = owner.spawn(spec, None)?;
    let result = process.wait()?;
    if !result.success() {
        return Err(format!(
            "Owned command failed: status={:?} exit_code={:?} scope={} pid={} stdout={} stderr={}",
            result.status,
            result.exit_code,
            result.scope,
            result.pid,
            result.stdout.display(),
            result.stderr.display()
        ));
    }
    Ok(result)
}

/// A common OS monotonic clock, not the ECU's logical epoch.
#[cfg(target_os = "linux")]
pub fn monotonic_ns() -> Result<u64, String> {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: a writable timespec is passed to CLOCK_MONOTONIC.
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut value) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let nanos = (value.tv_sec as u128) * 1_000_000_000 + (value.tv_nsec as u128);
    nanos
        .try_into()
        .map_err(|_| "Monotonic clock overflow".into())
}

#[cfg(target_os = "macos")]
pub fn monotonic_ns() -> Result<u64, String> {
    use std::sync::LazyLock;
    #[repr(C)]
    struct Timebase {
        numer: u32,
        denom: u32,
    }
    unsafe extern "C" {
        fn mach_absolute_time() -> u64;
        fn mach_timebase_info(info: *mut Timebase) -> i32;
    }
    static TIMEBASE: LazyLock<Result<(u32, u32), String>> = LazyLock::new(|| {
        let mut info = Timebase { numer: 0, denom: 0 };
        // SAFETY: the kernel writes the timebase into a valid record.
        let status = unsafe { mach_timebase_info(&mut info) };
        if status != 0 || info.denom == 0 {
            Err(format!("mach_timebase_info failed: {status}"))
        } else {
            Ok((info.numer, info.denom))
        }
    });
    let (numer, denom) = *TIMEBASE.as_ref().map_err(Clone::clone)?;
    // SAFETY: mach_absolute_time has no pointer arguments and is monotonic.
    let value = unsafe { mach_absolute_time() };
    ((value as u128) * (numer as u128) / (denom as u128))
        .try_into()
        .map_err(|_| "Monotonic clock overflow".into())
}

#[cfg(windows)]
pub fn monotonic_ns() -> Result<u64, String> {
    use std::sync::LazyLock;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn QueryPerformanceCounter(value: *mut i64) -> i32;
        fn QueryPerformanceFrequency(value: *mut i64) -> i32;
    }
    static FREQUENCY: LazyLock<Result<i64, String>> = LazyLock::new(|| {
        let mut value = 0;
        // SAFETY: the kernel writes a frequency into a valid i64.
        let status = unsafe { QueryPerformanceFrequency(&mut value) };
        if status == 0 || value <= 0 {
            Err(format!(
                "QueryPerformanceFrequency failed: {}",
                std::io::Error::last_os_error()
            ))
        } else {
            Ok(value)
        }
    });
    let frequency = *FREQUENCY.as_ref().map_err(Clone::clone)?;
    let mut value = 0;
    // SAFETY: the kernel writes the current counter into a valid i64.
    if unsafe { QueryPerformanceCounter(&mut value) } == 0 || value < 0 {
        return Err(format!(
            "QueryPerformanceCounter failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    ((value as u128) * 1_000_000_000 / (frequency as u128))
        .try_into()
        .map_err(|_| "Monotonic clock overflow".into())
}
