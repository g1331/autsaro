use crate::execution::{OwnedProcess, ProcessOwner, ProcessSpec};
use crate::message::LocalizedText;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) struct TempNvm {
    pub(super) path: PathBuf,
}
impl TempNvm {
    pub(super) fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "autosar-dtc-{}-{stamp}-{}.nvm",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        Self { path }
    }
}
impl Drop for TempNvm {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub(super) struct SecurityFiles {
    key: TempNvm,
    pub(super) state: TempNvm,
}
impl SecurityFiles {
    pub(super) fn new() -> Result<Self, LocalizedText> {
        let files = Self {
            key: TempNvm::new(),
            state: TempNvm::new(),
        };
        fs::write(&files.key.path, [0x5au8; 32])
            .map_err(|e| crate::product_message!("backend.host.prepare_key", "error" => e))?;
        Ok(files)
    }
}

pub(super) struct EcuProcess {
    child: OwnedProcess,
    reader: Option<BufReader<fs::File>>,
    pending: String,
    input: Vec<u8>,
    logs: PathBuf,
    closed: bool,
}
impl EcuProcess {
    pub(super) fn start(
        path: &Path,
        nvm_path: Option<&Path>,
        security: Option<&SecurityFiles>,
        owner: &ProcessOwner,
    ) -> Result<Self, LocalizedText> {
        let path = path.canonicalize().map_err(|error| error.to_string())?;
        let logs = crate::generator::output::reserve_directory(
            &std::env::temp_dir(),
            "legacy-process",
            std::ffi::OsStr::new("private"),
        )?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&logs, fs::Permissions::from_mode(0o700))
                .map_err(|error| error.to_string())?;
        }
        let mut argv = vec![path.as_os_str().into()];
        if let Some(storage) = nvm_path {
            argv.extend(["--nvm".into(), storage.as_os_str().into()]);
        }
        if let Some(files) = security {
            argv.extend([
                "--security-key".into(),
                files.key.path.as_os_str().into(),
                "--security-state".into(),
                files.state.path.as_os_str().into(),
            ]);
        }
        let mut spec = ProcessSpec::for_duration(
            argv,
            path.parent()
                .ok_or_else(|| crate::product_message!("backend.host.binary_parent_missing"))?
                .to_path_buf(),
            Vec::new(),
            Duration::from_secs(120),
            logs.clone(),
        )?;
        spec.stdin_stream = true;
        let child = owner.spawn(spec, None)?;
        let reader =
            BufReader::new(fs::File::open(child.stdout_path()).map_err(|error| error.to_string())?);
        Ok(Self {
            child,
            reader: Some(reader),
            pending: String::new(),
            input: Vec::new(),
            logs,
            closed: false,
        })
    }

    pub(super) fn command(&mut self, command: &str) -> Result<(), LocalizedText> {
        self.input.clear();
        self.input.extend_from_slice(command.as_bytes());
        self.input.push(b'\n');
        self.child.write_stdin(&self.input)
    }

    pub(super) fn next_line(&mut self) -> Result<String, LocalizedText> {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let read = self
                .reader
                .as_mut()
                .ok_or_else(|| crate::product_message!("backend.host.stdout_closed"))?
                .read_line(&mut self.pending)
                .map_err(|error| error.to_string())?;
            if self.pending.ends_with('\n') {
                let mut line = std::mem::take(&mut self.pending);
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
                return Ok(line);
            }
            if std::time::Instant::now() >= deadline {
                return Err(
                    crate::product_message!("backend.host.response_timeout", "logs" => self.logs.display()),
                );
            }
            if read == 0 {
                thread::sleep(Duration::from_millis(2));
            }
        }
    }

    pub(super) fn finish(&mut self) -> Result<(), LocalizedText> {
        self.child.close_stdin();
        let result = self.child.wait()?;
        self.closed = true;
        drop(self.reader.take());
        if !result.success() {
            return Err(
                crate::product_message!("backend.host.close_failed", "result" => format!("{result:?}"), "stderr" => fs::read_to_string(&result.stderr).unwrap_or_default()),
            );
        }
        fs::remove_dir_all(&self.logs).map_err(|error| error.to_string().into())
    }

    pub(super) fn expect_corrupt_state_refusal(
        path: &Path,
        nvm: Option<&Path>,
        security: Option<&SecurityFiles>,
        owner: &ProcessOwner,
    ) -> Result<(), LocalizedText> {
        let mut actor = Self::start(path, nvm, security, owner)?;
        actor.child.close_stdin();
        let result = actor.child.wait()?;
        actor.closed = true;
        drop(actor.reader.take());
        let output = fs::read_to_string(&result.stdout).map_err(|error| error.to_string())?;
        if result.status != crate::execution::ProcessStatus::Exited
            || result.exit_code == Some(0)
            || !output
                .lines()
                .any(|line| line.trim_end_matches('\r') == "E NVM")
        {
            return Err(
                crate::product_message!("backend.host.corrupt_state_not_refused", "result" => format!("{result:?}"), "output" => output),
            );
        }
        fs::remove_dir_all(&actor.logs).map_err(|error| error.to_string().into())
    }
    pub(super) fn query(
        &mut self,
        commands: &[String],
        fence: u16,
    ) -> Result<(Vec<String>, String), LocalizedText> {
        for command in commands {
            self.command(command)?;
        }
        self.command(&format!("G {fence}"))?;
        let mut events = Vec::new();
        loop {
            let line = self.next_line()?;
            if line.starts_with("V ") {
                return Ok((events, line));
            }
            if line.starts_with("E ") {
                return Err(
                    crate::product_message!("backend.host.command_refused", "line" => line),
                );
            }
            if !line.starts_with("X ") {
                return Err(
                    crate::product_message!("backend.host.unknown_protocol", "line" => line),
                );
            }
            events.push(line);
        }
    }
}
impl Drop for EcuProcess {
    fn drop(&mut self) {
        drop(self.reader.take());
        if !self.closed {
            if let Err(error) = self.child.cancel() {
                eprintln!(
                    "{}",
                    crate::product_message!("backend.host.ownership_close_failed", "error" => error, "logs" => self.logs.display())
                );
            }
        }
    }
}
