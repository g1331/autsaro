//! Test-only scheduling evidence, including failures while writing a large stdin.
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) struct Scheduling {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<String>>,
}

impl Scheduling {
    pub(super) fn start(pid: u32, directory: &Path) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let path = directory.join("schedstat.jsonl");
        let worker = thread::spawn(move || {
            let mut file = match fs::File::create(&path) {
                Ok(file) => file,
                Err(error) => return format!("schedstat capture {}: {error}", path.display()),
            };
            let mut counters = BTreeMap::<String, ([u64; 3], [u64; 3])>::new();
            let mut errors = Vec::new();
            while !stopping.load(Ordering::Acquire) {
                let tasks = match fs::read_dir(format!("/proc/{pid}/task")) {
                    Ok(tasks) => tasks,
                    Err(error) => {
                        errors.push(format!("/proc/{pid}/task: {error}"));
                        break;
                    }
                };
                for task in tasks {
                    let sample = (|| -> std::io::Result<()> {
                        let task = task?;
                        let tid = task.file_name().to_string_lossy().into_owned();
                        let source = task.path().join("schedstat");
                        let raw = fs::read_to_string(&source).map_err(|error| {
                            std::io::Error::new(
                                error.kind(),
                                format!("{}: {error}", source.display()),
                            )
                        })?;
                        let values: Vec<u64> = raw
                            .split_whitespace()
                            .map(str::parse)
                            .collect::<Result<_, _>>()
                            .map_err(std::io::Error::other)?;
                        let values: [u64; 3] = values.try_into().map_err(|_| {
                            std::io::Error::other("schedstat requires three counters")
                        })?;
                        let wall_ns = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_nanos();
                        writeln!(
                            file,
                            "{}",
                            serde_json::json!({
                                "wall_ns": wall_ns.to_string(), "pid": pid, "tid": tid,
                                "run_ns": values[0], "runqueue_wait_ns": values[1],
                                "timeslices": values[2],
                            })
                        )?;
                        counters
                            .entry(tid)
                            .and_modify(|value| value.1 = values)
                            .or_insert((values, values));
                        Ok(())
                    })();
                    if let Err(error) = sample {
                        errors.push(error.to_string());
                    }
                }
                // Observation cadence only; never gates the ECU or adds a test grace period.
                thread::park_timeout(Duration::from_millis(50));
            }
            let mut report = format!("schedstat pid={pid} file={}\n", path.display());
            for (tid, (first, last)) in counters {
                report.push_str(&format!(
                    "tid={tid} first={first:?} last={last:?} run_delta_ns={} runqueue_wait_delta_ns={}\n",
                    last[0].saturating_sub(first[0]), last[1].saturating_sub(first[1]),
                ));
            }
            for error in errors {
                report.push_str(&format!("capture_error={error}\n"));
            }
            report
        });
        Self {
            stop,
            worker: Some(worker),
        }
    }

    fn finish(&mut self) -> Option<String> {
        self.stop.store(true, Ordering::Release);
        self.worker.take().map(|worker| {
            worker.thread().unpark();
            worker
                .join()
                .unwrap_or_else(|_| "schedstat worker panicked".into())
        })
    }
}

impl Drop for Scheduling {
    fn drop(&mut self) {
        if let Some(report) = self.finish()
            && thread::panicking()
        {
            eprintln!("{report}");
        }
    }
}

#[test]
fn records_live_thread_counters() {
    let scratch = super::super::Scratch::new();
    let pid = std::process::id();
    let mut capture = Scheduling::start(pid, &scratch.0);
    let path = scratch.0.join("schedstat.jsonl");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while fs::metadata(&path).map_or(true, |value| value.len() == 0) {
        assert!(
            std::time::Instant::now() < deadline,
            "no scheduling observation"
        );
        thread::yield_now();
    }
    let report = capture.finish().unwrap();
    assert!(report.contains(&format!("tid={pid} ")), "{report}");
    assert!(report.contains("runqueue_wait_delta_ns="), "{report}");
    let data = fs::read_to_string(path).unwrap();
    for line in data.lines() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(row["pid"], pid);
        assert!(row["runqueue_wait_ns"].is_u64());
    }
}

#[test]
fn prints_counters_when_replay_panics() {
    const CHILD: &str = "AUTOSAR_TEST_SCHEDSTAT_PANIC_DIRECTORY";
    if let Some(directory) = std::env::var_os(CHILD) {
        let directory = std::path::PathBuf::from(directory);
        let _capture = Scheduling::start(std::process::id(), &directory);
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while fs::metadata(directory.join("schedstat.jsonl")).map_or(true, |value| value.len() == 0)
        {
            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        }
        panic!("injected replay failure");
    }
    let scratch = super::super::Scratch::new();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "integration::protocol::scheduling::prints_counters_when_replay_panics",
            "--nocapture",
        ])
        .env(CHILD, &scratch.0);
    let result = super::super::tooling::run_public_command(
        &mut command,
        &scratch.0,
        "schedstat-panic",
        Duration::from_secs(30),
    );
    assert!(!result.status.success());
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.contains("injected replay failure"), "{output}");
    assert!(output.contains("runqueue_wait_delta_ns="), "{output}");
}
