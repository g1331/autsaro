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

#[cfg(all(test, feature = "native-tests"))]
mod tests;

/// A command's root exit and its owned-tree cleanup are separate contracts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionPolicy {
    /// Business tasks must not leave live workers after their root exits.
    #[default]
    RequireTreeExit,
    /// One-shot tools may leave helpers; close them before publishing the result.
    CloseTreeOnExit,
}

/// The monotonic deadline is absolute; child operations cannot reset it.
pub struct ProcessSpec {
    pub argv: Vec<OsString>,
    pub cwd: PathBuf,
    pub env: Vec<(OsString, OsString)>,
    pub deadline_ns: u64,
    pub log_directory: PathBuf,
    /// Open a bounded interactive stdin channel; output remains captured in logs.
    pub stdin_stream: bool,
    pub completion: CompletionPolicy,
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
            stdin_stream: false,
            completion: CompletionPolicy::RequireTreeExit,
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
    /// True only after live descendants were reclaimed and closure confirmed.
    pub descendants_reclaimed: bool,
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
    #[cfg(windows)]
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    #[cfg(windows)]
    jobs: parking_lot::Mutex<Vec<std::sync::Arc<std::os::windows::io::OwnedHandle>>>,
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
            Ok(Self {
                cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                jobs: parking_lot::Mutex::new(Vec::new()),
            })
        }
    }

    pub fn with_python(python: &std::path::Path) -> Result<Self, String> {
        if !python.is_absolute() || !python.is_file() {
            return Err("The configured CPython must name an existing absolute executable".into());
        }
        #[cfg(unix)]
        {
            Ok(Self {
                owner: unix::UnixOwner::start_with_python(python)?,
            })
        }
        #[cfg(windows)]
        {
            Self::new()
        }
    }

    pub fn run(&self, spec: ProcessSpec) -> Result<ProcessResult, String> {
        let result = self.spawn(spec, None)?.wait()?;
        if !result.success() {
            return Err(format!(
                "Owned command failed: status={:?} exit_code={:?} scope={} pid={} stdout={} stderr={}",
                result.status,
                result.exit_code,
                result.scope,
                result.pid,
                result.stdout.display(),
                result.stderr.display(),
            ));
        }
        Ok(result)
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
            let mut jobs = self.jobs.lock();
            if self.cancelled.load(std::sync::atomic::Ordering::Acquire) {
                return Err("Process owner is cancelled; no new command can start".into());
            }
            let capture = private_capture(&spec.log_directory)?;
            let stdout = capture.join("stdout.log");
            let stderr = capture.join("stderr.log");
            let mut command = std::process::Command::new(&spec.argv[0]);
            command
                .args(&spec.argv[1..])
                .current_dir(&spec.cwd)
                .envs(spec.env.iter().cloned());
            command.stdin(if spec.stdin_stream {
                std::process::Stdio::piped()
            } else {
                std::process::Stdio::null()
            });
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
            jobs.push(tree.job());
            let pid = tree.id();
            Ok(OwnedProcess {
                spec,
                backend: Backend::Windows {
                    tree,
                    stdout,
                    stderr,
                    pid,
                    finished: false,
                    cancelled: std::sync::Arc::clone(&self.cancelled),
                },
            })
        }
    }

    pub fn cancel(&self) -> Result<(), String> {
        #[cfg(unix)]
        {
            self.owner.cancel()
        }
        #[cfg(windows)]
        {
            self.cancelled
                .store(true, std::sync::atomic::Ordering::Release);
            windows_job::stop_jobs(&self.jobs.lock())
        }
    }
}

impl Drop for ProcessOwner {
    fn drop(&mut self) {
        let _ = self.cancel();
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
        cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    },
}

pub struct OwnedProcess {
    #[cfg(windows)]
    spec: ProcessSpec,
    backend: Backend,
}

impl OwnedProcess {
    pub fn stdout_path(&self) -> &std::path::Path {
        match &self.backend {
            #[cfg(unix)]
            Backend::Unix(process) => process.stdout_path(),
            #[cfg(windows)]
            Backend::Windows { stdout, .. } => stdout,
        }
    }

    pub fn write_stdin(&mut self, bytes: &[u8]) -> Result<(), String> {
        match &mut self.backend {
            #[cfg(unix)]
            Backend::Unix(process) => process.write_stdin(bytes),
            #[cfg(windows)]
            Backend::Windows { tree, finished, .. } => {
                if *finished {
                    return Err("Owned process already completed".into());
                }
                tree.write_stdin(bytes, self.spec.deadline_ns)
            }
        }
    }

    pub fn close_stdin(&mut self) {
        match &mut self.backend {
            #[cfg(unix)]
            Backend::Unix(process) => process.close_stdin(),
            #[cfg(windows)]
            Backend::Windows { tree, .. } => tree.close_stdin(),
        }
    }
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
                cancelled,
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
                        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                            ProcessStatus::Cancelled
                        } else if orphaned
                            && self.spec.completion == CompletionPolicy::RequireTreeExit
                        {
                            ProcessStatus::OrphanedMembers
                        } else {
                            ProcessStatus::Exited
                        },
                        status.code(),
                        orphaned,
                    );
                }
                if monotonic_ns()? >= self.spec.deadline_ns {
                    tree.stop()?;
                    break (ProcessStatus::Timeout, None, false);
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
                descendants_reclaimed: status.2,
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
                ..
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
                descendants_reclaimed: false,
            })
        }
    }
}

pub fn run_bounded(spec: ProcessSpec) -> Result<ProcessResult, String> {
    ProcessOwner::new()?.run(spec)
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
