//! Test-only timing ELF construction and first-close debugger entry.
use super::*;

const FLAGS: &[&str] = &[
    "-DOS_HOST_CAPTURE_TESTS=1",
    "-g",
    "-Wl,--wrap=Os_BackendShutdown",
    "-Wl,--wrap=Os_BackendRequestShutdown",
    "-Wl,--wrap=Ecu_TargetAbortNative",
    "-Wl,--wrap=WaitForMultipleObjects",
    "-Wl,--wrap=SuspendThread",
    "-Wl,--wrap=ResumeThread",
    "-Wl,--wrap=Ecu_TargetTakeOutput",
    "-Wl,--wrap=Os_TargetAdvanceOneTick",
];

pub(super) fn control(project: &Path, scratch: &Path, extra: &str) -> PathBuf {
    let fixtures = root().join("core/tests/fixtures/host-batch");
    let mut text = fs::read_to_string(project.join("integration/src/ecu_host_batch.c")).unwrap();
    if std::env::var_os("AUTOSAR_TEST_TIMING_CAPTURE_FAILURE").is_some() {
        text.push_str("\n#define AUTOSAR_CAPTURE_INJECT 1\n");
    }
    for source in ["linux_watchdog_capture.c", "linux_watchdog_budget.c"] {
        text.push('\n');
        text.push_str(&fs::read_to_string(fixtures.join(source)).unwrap());
    }
    text.push_str(extra);
    let path = scratch.join("timing-control.c");
    fs::write(&path, text).unwrap();
    path
}

pub(super) fn configure(command: &mut Command) {
    command.args(FLAGS);
}

pub(super) fn mark(binary: &Path) {
    fs::write(
        binary.with_extension("capture"),
        "first-close-and-budget-v1\n",
    )
    .unwrap();
}

pub(super) fn project() -> Built {
    let scratch = super::super::Scratch::new();
    let (project, _) =
        super::super::single_component_project(&scratch.0, super::super::tooling::native_target());
    let control = control(&project, &scratch.0, "");
    let output = scratch.0.join("timing-build");
    let kinds = std::env::var_os("AUTOSAR_SANITIZE").unwrap_or_default();
    let mut command = super::super::tooling::ecu_sanitizer_build_command(
        &project,
        &output,
        "probe",
        Some(&control),
        &kinds,
    );
    configure(&mut command);
    require_success(&super::super::tooling::run_public_command(
        &mut command,
        &scratch.0,
        "timing-build",
        Duration::from_secs(180),
    ));
    let directory = if kinds.is_empty() {
        output.join("production")
    } else {
        output
    };
    let binary = super::super::tooling::native_binary(&directory, "ecu_probe");
    mark(&binary);
    Built {
        scratch,
        project,
        binary,
    }
}

pub(super) fn freeze(pid: u32, binary: &Path, directory: &Path) -> String {
    let mut command = Command::new("gdb");
    command
        .args([
            "-q",
            "-nx",
            "-batch",
            "-ex",
            "set pagination off",
            "-ex",
            "set confirm off",
            "-ex",
            "set debuginfod enabled off",
            "-ex",
            "set print elements 32",
        ])
        .arg(binary)
        .arg("-p")
        .arg(pid.to_string())
        .arg("-x")
        .arg(root().join("core/tests/fixtures/host-batch/linux_watchdog_state.py"))
        .args(["-ex", "kill"])
        .env("AUTOSAR_WATCHDOG_OUTPUT", directory)
        .env("AUTOSAR_WATCHDOG_MODE", "automatic");
    // An inherited ProcessOwner may launch siblings of this test process. Yama's
    // parent grant requires GDB to remain our descendant, so own this bounded child directly.
    struct Debugger(std::process::Child);
    impl Drop for Debugger {
        fn drop(&mut self) {
            if self.0.try_wait().ok().flatten().is_none() {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
    }
    let output_path = directory.join("pre-close.stdout");
    let error_path = directory.join("pre-close.stderr");
    let result = (|| -> std::io::Result<String> {
        command
            .stdin(Stdio::null())
            .stdout(fs::File::create(&output_path)?)
            .stderr(fs::File::create(&error_path)?);
        let mut debugger = Debugger(command.spawn()?);
        let started = std::time::Instant::now();
        let status = loop {
            if let Some(status) = debugger.0.try_wait()? {
                break status;
            }
            if started.elapsed() >= Duration::from_secs(15) {
                debugger.0.kill()?;
                break debugger.0.wait()?;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        Ok(format!(
            "debugger={status}\n{}{}",
            fs::read_to_string(&output_path)?,
            fs::read_to_string(&error_path)?
        ))
    })();
    let valid = fs::read(directory.join("automatic.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .is_some_and(|snapshot| snapshot["before_close"] == true);
    format!(
        "pre_close_capture pid={pid} reason=7 valid={valid}\n{}",
        result.unwrap_or_else(|error| format!("capture_error={error}"))
    )
}

#[test]
fn timing_failure_automatically_preserves_budget_and_first_close() {
    fn find(directory: &Path, name: &str, output: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                find(&path, name, output);
            } else if path.file_name().unwrap() == name {
                output.push(path);
            }
        }
    }
    let scratch = super::super::Scratch::new();
    for (test, lost_notice) in [
        (
            "single_component_meets_independent_protocol_and_timing_vectors",
            false,
        ),
        ("s3_output_publication_wakes_armed_native_event_wait", false),
        (
            "single_component_meets_independent_protocol_and_timing_vectors",
            true,
        ),
    ] {
        let directory = scratch.0.join(format!("{test}-{lost_notice}"));
        fs::create_dir(&directory).unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                &format!("integration::protocol::{test}"),
                "--nocapture",
            ])
            .env("AUTOSAR_TEST_TIMING_CAPTURE_FAILURE", "1")
            .env(
                "AUTOSAR_TEST_CAPTURE_LOST_NOTICE",
                if lost_notice { "1" } else { "0" },
            )
            .env("TMPDIR", &directory)
            .env("RUST_TEST_THREADS", "1");
        let result = super::super::tooling::run_public_command(
            &mut command,
            &directory,
            "injected-timing-failure",
            Duration::from_secs(180),
        );
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            !result.status.success(),
            "injection did not reach {test}: {text}"
        );
        assert!(
            text.contains("pre_close_capture") && text.contains("valid=true"),
            "{text}"
        );
        let mut snapshots = Vec::new();
        find(&directory, "automatic.json", &mut snapshots);
        assert_eq!(snapshots.len(), 1, "{snapshots:?}\n{text}");
        let scene = snapshots[0].parent().unwrap();
        let snapshot: Value = serde_json::from_slice(&fs::read(&snapshots[0]).unwrap()).unwrap();
        assert_eq!(snapshot["before_close"], true, "{snapshot}");
        assert_eq!(snapshot["captured_reason"], 7);
        let frozen = snapshot["freeze_monotonic_ns"].as_u64().unwrap();
        assert!(frozen > 0);
        let age = snapshot["monotonic_ns"].as_u64().unwrap() - frozen;
        if lost_notice {
            assert!(
                (20_000_000_000..46_000_000_000).contains(&age),
                "deadline recovery was not exercised: {age}"
            );
        } else {
            assert!(
                age < 16_000_000_000,
                "capture was deferred until the execution deadline: {age}"
            );
        }
        assert_eq!(snapshot["globals"]["Os_Closing"], "0");
        assert!(
            matches!(snapshot["sched_schedstats"].as_str(), Some("0" | "1"))
                || snapshot["sched_schedstats"]["error"]
                    .as_str()
                    .is_some_and(|error| !error.is_empty())
        );
        for field in ["mutex_owner", "mutex_depth", "mutex_generation"] {
            assert!(
                snapshot["gate"][field].is_u64(),
                "missing {field}: {snapshot}"
            );
        }
        let stacks = fs::read_to_string(scene.join("all-stacks.txt")).unwrap();
        assert!(
            stacks.contains("Os_BackendAssert") && stacks.contains("Os_HostCaptureBeforeClose"),
            "{stacks}"
        );
        let samples: Vec<Value> = fs::read_to_string(scene.join("schedstat.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        for role in ["H", "S"] {
            let tid = snapshot["roles"][role].as_u64().unwrap();
            let budget = &snapshot["budget"][role];
            assert_eq!(budget["tid"], tid);
            assert_eq!(budget["clock_errors"], 0);
            let attributed = budget["phases"]
                .as_array()
                .unwrap()
                .iter()
                .map(|phase| phase["exclusive_cpu_ns"].as_u64().unwrap())
                .sum::<u64>()
                + budget["outside_cpu_ns"].as_u64().unwrap();
            if budget["revision"].as_u64().unwrap().is_multiple_of(2) {
                assert_eq!(attributed, budget["accounted_cpu_ns"].as_u64().unwrap());
            }
            for sample in budget["samples"].as_array().unwrap() {
                let total = sample["phase_exclusive_cpu_ns"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|value| value.as_u64().unwrap())
                    .sum::<u64>()
                    + sample["outside_cpu_ns"].as_u64().unwrap();
                assert_eq!(total, sample["cpu_ns"].as_u64().unwrap());
            }

            assert!(
                !budget["samples"].as_array().unwrap().is_empty(),
                "{budget}"
            );
            assert!(budget["samples"][0]["wall_ns"].as_u64().unwrap() > 0);
            assert!(budget["samples"][0]["cpu_ns"].as_u64().unwrap() > 0);
            assert!(
                budget["phases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|phase| phase["cpu_ns"].as_u64().unwrap() > 0),
                "{budget}"
            );
            let handle = snapshot["handles"]
                .as_array()
                .unwrap()
                .iter()
                .find(|handle| handle["thread_id"].as_str() == Some(&tid.to_string()))
                .unwrap();
            for field in ["requested", "acknowledged", "resumed"] {
                assert!(
                    handle[field].as_str().unwrap().parse::<u64>().is_ok(),
                    "{handle}"
                );
            }
            assert!(
                samples
                    .iter()
                    .any(|sample| sample["tid"].as_str() == Some(&tid.to_string())
                        && sample["utime_ticks"].is_u64()
                        && sample["stime_ticks"].is_u64()
                        && sample["clock_ticks_per_second"]
                            .as_u64()
                            .is_some_and(|ticks| ticks > 0)),
                "missing user/system CPU for {role}"
            );
        }
        assert_eq!(snapshot["budget"]["H"]["phases"][3]["calls"], 100);
        for phase in [1, 2] {
            assert!(
                snapshot["budget"]["S"]["phases"][phase]["calls"]
                    .as_u64()
                    .unwrap()
                    > 0
            );
        }
        assert!(
            snapshot["threads"]
                .as_array()
                .unwrap()
                .iter()
                .all(|thread| thread["backtrace"]
                    .as_str()
                    .is_some_and(|trace| trace.contains("#0")))
        );
        let sampling: Value =
            serde_json::from_slice(&fs::read(scene.join("sampling.json")).unwrap()).unwrap();
        assert_eq!(sampling["pid"], snapshot["pid"]);
        assert!(sampling["started_ns"].is_u64() && sampling["finished_ns"].is_u64());
        if sampling["mode"] == "boundary-accounting" {
            assert!(
                sampling["unavailable"]
                    .as_str()
                    .is_some_and(|error| !error.is_empty())
            );
        } else {
            assert!(fs::metadata(scene.join("perf.data")).unwrap().len() > 0);
        }
        let mut receipts = Vec::new();
        find(&directory, "compile-commands.json", &mut receipts);
        assert!(
            receipts.iter().any(|path| {
                let receipt: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
                receipt["production"]
                    .as_array()
                    .is_some_and(|argv| argv.iter().any(|arg| arg == "-DOS_HOST_CAPTURE_TESTS=1"))
            }),
            "missing diagnostic compile receipt"
        );
        assert!(!fs::read(scene.join("stdin")).unwrap().is_empty());
        assert!(!fs::read(scene.join("batches.jsonl")).unwrap().is_empty());
    }
}
