//! Test-only scheduling evidence, including failures while writing a large stdin.
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

type Result<T> = std::io::Result<T>;

pub(super) fn monotonic_ns() -> Result<u64> {
    autosar_config_core::execution::monotonic_ns()
        .map_err(|error| std::io::Error::other(format!("{error:?}")))
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .map_err(|error| std::io::Error::new(error.kind(), format!("{}: {error}", path.display())))
}

#[derive(Clone)]
struct ThreadSample {
    monotonic_ns: u64,
    counters: [u64; 3],
    switches: [u64; 2],
    cpu_ticks: [u64; 2],
    state: String,
    wchan: String,
}

impl ThreadSample {
    fn read(task: &Path) -> Result<Self> {
        let values = read(&task.join("schedstat"))?
            .split_whitespace()
            .map(str::parse)
            .collect::<std::result::Result<Vec<u64>, _>>()
            .map_err(std::io::Error::other)?;
        let counters = values.try_into().map_err(|_| {
            std::io::Error::other(format!(
                "{}: expected three schedstat counters",
                task.display()
            ))
        })?;
        let status = read(&task.join("status"))?;
        let field = |name: &str| -> Result<String> {
            status
                .lines()
                .find_map(|line| line.strip_prefix(name))
                .map(|value| value.trim().to_owned())
                .ok_or_else(|| std::io::Error::other(format!("{}: missing {name}", task.display())))
        };
        let switches = [
            field("voluntary_ctxt_switches:")?
                .parse()
                .map_err(std::io::Error::other)?,
            field("nonvoluntary_ctxt_switches:")?
                .parse()
                .map_err(std::io::Error::other)?,
        ];
        let stat = read(&task.join("stat"))?;
        let fields: Vec<_> = stat
            .rsplit_once(") ")
            .ok_or_else(|| std::io::Error::other("thread stat has no command suffix"))?
            .1
            .split_whitespace()
            .collect();
        let cpu_ticks = [11, 12].map(|index| {
            fields
                .get(index)
                .ok_or_else(|| std::io::Error::other("incomplete thread stat"))?
                .parse()
                .map_err(std::io::Error::other)
        });
        let [user, system] = cpu_ticks;
        Ok(Self {
            cpu_ticks: [user?, system?],
            monotonic_ns: monotonic_ns()?,
            counters,
            switches,
            state: field("State:")?,
            wchan: read(&task.join("wchan"))?.trim().into(),
        })
    }
}

struct Progress {
    source: fs::File,
    output: BufWriter<fs::File>,
    pending: Vec<u8>,
    committed_epoch: u64,
    last_output_epoch: u64,
    last_observed_ns: u64,
    receipts: Vec<String>,
}

impl Progress {
    fn new(directory: &Path) -> Result<Self> {
        Ok(Self {
            source: fs::File::open(directory.join("stdout"))?,
            output: BufWriter::new(fs::File::create(directory.join("batches.jsonl"))?),
            pending: Vec::new(),
            committed_epoch: 0,
            last_output_epoch: 0,
            last_observed_ns: 0,
            receipts: Vec::new(),
        })
    }

    fn observe(&mut self) -> Result<()> {
        self.source.read_to_end(&mut self.pending)?;
        let Some(end) = self.pending.iter().rposition(|byte| *byte == b'\n') else {
            return Ok(());
        };
        let rest = self.pending.split_off(end + 1);
        let complete = std::mem::replace(&mut self.pending, rest);
        let stamp = monotonic_ns()?;
        for raw in complete
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
        {
            let line = String::from_utf8_lossy(raw);
            if !["OUT ", "COMMIT_", "READY ", "lifecycle="]
                .iter()
                .any(|prefix| line.starts_with(*prefix))
            {
                continue;
            }
            let epoch = line
                .split_whitespace()
                .find_map(|part| part.strip_prefix("epoch="))
                .and_then(|value| value.parse::<u64>().ok());
            let mut event = serde_json::json!({"monotonic_ns": stamp, "line": line});
            if let Some(epoch) = epoch {
                if line.starts_with("COMMIT_") {
                    let ticks = epoch.saturating_sub(self.committed_epoch);
                    event["completed_ticks"] = ticks.into();
                    self.committed_epoch = epoch;
                    self.receipts.push(format!(
                        "batch_observed_ns={stamp} completed_ticks={ticks} {line}\n"
                    ));
                } else if line.starts_with("OUT ") {
                    self.last_output_epoch = self.last_output_epoch.max(epoch);
                }
            }
            writeln!(self.output, "{event}")?;
            self.last_observed_ns = stamp;
        }
        self.output.flush()
    }

    fn report(&self) -> String {
        format!(
            "{}progress_observed_ns={} committed_epoch={} last_output_epoch={} \
             incomplete_ticks_lower_bound={}\n",
            self.receipts.concat(),
            self.last_observed_ns,
            self.committed_epoch,
            self.last_output_epoch,
            self.last_output_epoch
                .saturating_sub(1)
                .saturating_sub(self.committed_epoch),
        )
    }
}

struct Capture {
    pid: u32,
    output: BufWriter<fs::File>,
    counters: BTreeMap<String, (ThreadSample, ThreadSample)>,
    errors: Vec<String>,
    progress: Progress,
    stopped: bool,
    ticks_per_second: i64,
}

impl Capture {
    fn new(pid: u32, directory: &Path) -> Result<Self> {
        Ok(Self {
            pid,
            output: BufWriter::new(fs::File::create(directory.join("schedstat.jsonl"))?),
            counters: BTreeMap::new(),
            errors: Vec::new(),
            progress: Progress::new(directory)?,
            stopped: false,
            ticks_per_second: unsafe { libc::sysconf(libc::_SC_CLK_TCK) },
        })
    }

    fn observe(&mut self) -> Result<()> {
        self.stopped = false;
        for task in fs::read_dir(format!("/proc/{}/task", self.pid))? {
            let sample = (|| -> Result<()> {
                let task = task?;
                let tid = task.file_name().to_string_lossy().into_owned();
                let value = ThreadSample::read(&task.path())?;
                self.stopped |= value.state.starts_with('T');
                writeln!(
                    self.output,
                    "{}",
                    serde_json::json!({
                        "monotonic_ns": value.monotonic_ns, "pid": self.pid, "tid": tid,
                        "run_ns": value.counters[0], "runqueue_wait_ns": value.counters[1],
                        "timeslices": value.counters[2],
                        "utime_ticks": value.cpu_ticks[0], "stime_ticks": value.cpu_ticks[1],
                        "clock_ticks_per_second": self.ticks_per_second,
                        "voluntary_ctxt_switches": value.switches[0],
                        "nonvoluntary_ctxt_switches": value.switches[1],
                        "state": value.state, "wchan": value.wchan,
                    })
                )?;
                self.counters
                    .entry(tid)
                    .and_modify(|pair| pair.1 = value.clone())
                    .or_insert_with(|| (value.clone(), value));
                Ok(())
            })();
            if let Err(error) = sample {
                self.errors.push(error.to_string());
            }
        }
        self.output.flush()?;
        self.progress.observe()
    }

    fn report(&mut self) -> String {
        if let Err(error) = self.progress.observe() {
            self.errors.push(error.to_string());
        }
        let mut report = format!("schedstat pid={}\n", self.pid);
        for (tid, (first, last)) in &self.counters {
            let elapsed = last.monotonic_ns.saturating_sub(first.monotonic_ns);
            let run = last.counters[0].saturating_sub(first.counters[0]);
            let queue = last.counters[1].saturating_sub(first.counters[1]);
            report.push_str(&format!(
                "tid={tid} first={:?} last={:?} sample_first_ns={} sample_last_ns={} \
                 run_delta_ns={run} runqueue_wait_delta_ns={queue} other_elapsed_ns={} \
                 voluntary_delta={} nonvoluntary_delta={} utime_delta_ticks={} stime_delta_ticks={} state={} wchan={}\n",
                first.counters,
                last.counters,
                first.monotonic_ns,
                last.monotonic_ns,
                elapsed.saturating_sub(run).saturating_sub(queue),
                last.switches[0].saturating_sub(first.switches[0]),
                last.switches[1].saturating_sub(first.switches[1]),
                last.cpu_ticks[0].saturating_sub(first.cpu_ticks[0]),
                last.cpu_ticks[1].saturating_sub(first.cpu_ticks[1]),
                last.state,
                last.wchan,
            ));
        }
        report.push_str(&self.progress.report());
        for error in &self.errors {
            report.push_str(&format!("capture_error={error}\n"));
        }
        report
    }
}

pub(super) struct Scheduling {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<String>>,
    failure_started: Arc<AtomicBool>,
    completed_report: Option<String>,
    capture_enabled: bool,
    lost_notice: bool,
}

impl Scheduling {
    pub(super) fn start(pid: u32, directory: &Path, binary: Option<&Path>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let failure_started = Arc::new(AtomicBool::new(false));
        let failing = Arc::clone(&failure_started);
        let capture_enabled = binary.is_some();
        let binary = binary.map(Path::to_owned);
        let directory = directory.to_owned();
        // Take the initial observation before writing stdin, including short-lived consumers.
        let capture = Capture::new(pid, &directory).map(|mut capture| {
            if let Err(error) = capture.observe() {
                capture.errors.push(error.to_string());
            }
            capture
        });
        let worker = thread::spawn(move || {
            let _sampler = binary
                .as_ref()
                .map(|_| super::sampler::Sampler::start(pid, &directory));
            let mut capture = match capture {
                Ok(capture) => capture,
                Err(error) => return format!("schedstat capture: {error}"),
            };
            while !stopping.load(Ordering::Acquire) {
                if let Err(error) = capture.observe() {
                    capture.errors.push(error.to_string());
                    break;
                }
                if capture.stopped && binary.is_some() {
                    failing.store(true, Ordering::Release);
                    break;
                }
                // Observation cadence only; never gates the ECU or adds a test grace period.
                thread::park_timeout(Duration::from_millis(50));
            }
            capture.report()
        });
        Self {
            stop,
            worker: Some(worker),
            failure_started,
            completed_report: None,
            capture_enabled,
            lost_notice: std::env::var("AUTOSAR_TEST_CAPTURE_LOST_NOTICE").as_deref() == Ok("1"),
        }
    }

    pub(super) fn failure_started(&self) -> bool {
        self.failure_started.load(Ordering::Acquire) && !self.lost_notice
    }

    pub(super) fn stopped_failure(&self, pid: u32) -> bool {
        self.capture_enabled
            && fs::read_to_string(format!("/proc/{pid}/status"))
                .is_ok_and(|status| status.lines().any(|line| line.starts_with("State:\tT ")))
    }

    pub(super) fn complete_failure(&mut self, pid: u32, binary: &Path, directory: &Path) {
        self.completed_report = self.finish();
        self.completed_report
            .get_or_insert_with(String::new)
            .push_str(&super::capture::freeze(pid, binary, directory));
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
        if let Some(report) = self.finish().or_else(|| self.completed_report.take())
            && thread::panicking()
        {
            eprintln!("{report}");
        }
    }
}

#[test]
fn records_live_thread_counters() {
    let scratch = super::super::Scratch::new();
    fs::write(scratch.0.join("stdout"), "").unwrap();
    let pid = std::process::id();
    let mut capture = Scheduling::start(pid, &scratch.0, None);
    let report = capture.finish().unwrap();
    assert!(report.contains(&format!("tid={pid} ")), "{report}");
    assert!(report.contains("runqueue_wait_delta_ns="), "{report}");
    for line in read(&scratch.0.join("schedstat.jsonl")).unwrap().lines() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(row["pid"], pid);
        for field in [
            "monotonic_ns",
            "runqueue_wait_ns",
            "voluntary_ctxt_switches",
            "nonvoluntary_ctxt_switches",
        ] {
            assert!(row[field].is_u64(), "{row}");
        }
        assert!(
            row["state"].is_string() && row["wchan"].is_string(),
            "{row}"
        );
    }
}

#[test]
fn run_failure_records_scene_and_diagnostics() {
    use std::os::unix::fs::PermissionsExt;
    const CHILD: &str = "AUTOSAR_TEST_PROTOCOL_FAILURE_DIRECTORY";
    const INPUT: &[u8] = b"BEGIN 3\nCOMMIT\nBEGIN 10\nCOMMIT\nBEGIN 15\nCOMMIT\n";
    if let Some(directory) = std::env::var_os(CHILD) {
        let directory = std::path::PathBuf::from(directory);
        super::run(&directory.join("consumer"), &directory.join("scene"), INPUT);
        panic!("failure consumer unexpectedly succeeded");
    }
    let scratch = super::super::Scratch::new();
    let binary = scratch.0.join("consumer");
    fs::write(
        &binary,
        concat!(
            "#!/bin/sh\ncat >/dev/null\n",
            "printf 'READY HostBatchV2\\nPROTOCOL_OK HostBatchV2\\nCOMMIT_OK batch=1 epoch=3\\n",
            "COMMIT_OK batch=2 epoch=10\\nOUT epoch=12\\n'\n",
            "printf 'injected consumer failure\\n' >&2\nexit 7\n",
        ),
    )
    .unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "integration::protocol::scheduling::run_failure_records_scene_and_diagnostics",
            "--nocapture",
        ])
        .env(CHILD, &scratch.0);
    let result = super::super::tooling::run_public_command(
        &mut command,
        &scratch.0,
        "protocol-failure",
        Duration::from_secs(30),
    );
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!result.status.success(), "{output}");
    for marker in [
        "injected consumer failure",
        "runqueue_wait_delta_ns=",
        "voluntary_delta=",
        "nonvoluntary_delta=",
        "state=",
        "wchan=",
        "batch_observed_ns=",
        "completed_ticks=3",
        "completed_ticks=7",
        "incomplete_ticks_lower_bound=1",
    ] {
        assert!(output.contains(marker), "missing {marker}: {output}");
    }
    let scene = scratch.0.join("scene");
    assert_eq!(
        fs::read(scene.join("stdin")).unwrap(),
        [b"PROTOCOL HostBatchV2\n".as_slice(), INPUT].concat(),
    );
    assert!(
        read(&scene.join("stdout"))
            .unwrap()
            .contains("OUT epoch=12")
    );
    assert_eq!(
        read(&scene.join("stderr")).unwrap(),
        "injected consumer failure\n"
    );
    assert!(!read(&scene.join("schedstat.jsonl")).unwrap().is_empty());
    let events: Vec<serde_json::Value> = read(&scene.join("batches.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        events
            .iter()
            .filter_map(|event| event["completed_ticks"].as_u64())
            .collect::<Vec<_>>(),
        [3, 7]
    );
    assert!(events.iter().all(|event| event["monotonic_ns"].is_u64()));
}
