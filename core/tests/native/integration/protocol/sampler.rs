//! Optional unprivileged software stacks; boundary accounting is always available.
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub(super) struct Sampler {
    process: Option<Child>,
    directory: PathBuf,
    report: serde_json::Value,
}

impl Sampler {
    pub(super) fn start(pid: u32, directory: &Path) -> Self {
        let mut sampler = Self {
            process: None,
            directory: directory.to_owned(),
            report: serde_json::json!({
                "mode": "boundary-accounting", "pid": pid,
                "event": "task-clock:u", "frequency_hz": 19,
                "started_ns": super::scheduling::monotonic_ns().ok(),
            }),
        };
        // Linux perf_event_attr: SOFTWARE / TASK_CLOCK, disabled, user space only.
        let mut attributes = [0u64; 16];
        attributes[0] = 1 | (128 << 32);
        attributes[1] = 1;
        attributes[5] = 1 | (1 << 5) | (1 << 6);
        let descriptor =
            unsafe { libc::syscall(libc::SYS_perf_event_open, attributes.as_ptr(), 0, -1, -1, 0) };
        if descriptor < 0 {
            sampler.report["unavailable"] = std::io::Error::last_os_error().to_string().into();
        } else {
            unsafe { libc::close(descriptor as libc::c_int) };
            match fs::File::create(directory.join("perf.stderr")).and_then(|errors| {
                Command::new("perf")
                    .args([
                        "record",
                        "--quiet",
                        "-N",
                        "-e",
                        "task-clock:u",
                        "-F",
                        "19",
                        "--call-graph",
                        "dwarf,4096",
                        "--clockid",
                        "mono",
                        "-p",
                    ])
                    .arg(pid.to_string())
                    .arg("-o")
                    .arg(directory.join("perf.data"))
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(errors)
                    .spawn()
            }) {
                Ok(process) => {
                    sampler.report["sampler_pid"] = process.id().into();
                    sampler.report["mode"] = "software-stacks-and-boundary-accounting".into();
                    sampler.process = Some(process);
                }
                Err(error) => sampler.report["unavailable"] = error.to_string().into(),
            }
        }
        sampler
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        let mut recorded = false;
        if let Some(process) = &mut self.process {
            let started = Instant::now();
            match process.try_wait() {
                Ok(None) => {
                    // Child is still owned and unreaped; its PID cannot have been reused.
                    unsafe { libc::kill(process.id() as libc::pid_t, libc::SIGINT) };
                    loop {
                        match process.try_wait() {
                            Ok(Some(status)) => {
                                recorded =
                                    status.success() || status.signal() == Some(libc::SIGINT);
                                self.report["status"] = status.to_string().into();
                                break;
                            }
                            Ok(None) if started.elapsed() < Duration::from_secs(5) => {
                                std::thread::sleep(Duration::from_millis(10));
                            }
                            result => {
                                self.report["stop_error"] = format!("{result:?}").into();
                                let _ = process.kill();
                                let _ = process.wait();
                                break;
                            }
                        }
                    }
                }
                Ok(Some(status)) => {
                    recorded = status.success() || status.signal() == Some(libc::SIGINT);
                    self.report["status"] = status.to_string().into();
                }
                Err(error) => {
                    self.report["unavailable"] = error.to_string().into();
                    let _ = process.kill();
                    let _ = process.wait();
                }
            }
        }
        if self.process.is_some()
            && (!recorded
                || !self
                    .directory
                    .join("perf.data")
                    .metadata()
                    .is_ok_and(|metadata| metadata.len() > 0))
        {
            self.report["mode"] = "boundary-accounting".into();
            self.report["unavailable"] = format!(
                "perf recording unavailable: {}",
                fs::read_to_string(self.directory.join("perf.stderr"))
                    .unwrap_or_else(|error| error.to_string())
            )
            .into();
        }
        self.report["finished_ns"] = super::scheduling::monotonic_ns().ok().into();
        if let Err(error) = fs::write(
            self.directory.join("sampling.json"),
            format!("{}\n", self.report),
        ) {
            eprintln!("capture_error=sampling report: {error}");
        }
    }
}
