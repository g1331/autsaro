use super::{ProcessResult, ProcessSpec, monotonic_ns};
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NONCE: AtomicU64 = AtomicU64::new(0);

fn group_alive(pgid: i32) -> bool {
    // SAFETY: negative PID addresses only the registered cooperative group.
    if unsafe { libc::kill(-pgid, 0) != 0 } {
        return false;
    }
    #[cfg(target_os = "linux")]
    {
        // An outer subreaper may retain stopped members as zombies after a
        // nested supervisor dies. killpg(0) still sees them, but they cannot
        // execute. Unknown /proc state remains unconfirmed, never successful.
        let Ok(entries) = fs::read_dir("/proc") else {
            return true;
        };
        let mut observed = false;
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().parse::<u32>().is_err() {
                continue;
            }
            let Ok(stat) = fs::read_to_string(entry.path().join("stat")) else {
                continue;
            };
            let Some((_, fields)) = stat.rsplit_once(") ") else {
                continue;
            };
            let mut fields = fields.split_ascii_whitespace();
            let (Some(state), Some(_parent), Some(group)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            if group.parse::<i32>() == Ok(pgid) {
                observed = true;
                if state != "Z" && state != "X" {
                    return true;
                }
            }
        }
        return !observed;
    }
    #[cfg(not(target_os = "linux"))]
    true
}

fn signal_group(pgid: i32, signal: i32) {
    // SAFETY: the guardian mirrors only groups returned by its own supervisor.
    unsafe { libc::kill(-pgid, signal) };
}

fn private_directory() -> Result<PathBuf, String> {
    for _ in 0..100 {
        let name = format!(
            "ecu-owner-{}-{}-{}",
            std::process::id(),
            monotonic_ns()?,
            NONCE.fetch_add(1, Ordering::Relaxed)
        );
        let path = std::env::temp_dir().join(name);
        let mut builder = DirBuilder::new();
        builder.mode(0o700);
        match builder.create(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("Unable to reserve a private owner directory".into())
}

struct UnixInput {
    directory: PathBuf,
    path: PathBuf,
    file: Option<File>,
}

impl UnixInput {
    fn new() -> Result<Self, String> {
        use std::os::unix::ffi::OsStrExt;
        let directory = private_directory()?;
        let path = directory.join("stdin.pipe");
        let mut input = Self {
            directory,
            path,
            file: None,
        };
        let name = std::ffi::CString::new(input.path.as_os_str().as_bytes())
            .map_err(|error| error.to_string())?;
        // SAFETY: a terminated private path and owner-only FIFO mode.
        if unsafe { libc::mkfifo(name.as_ptr(), 0o600) } != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        input.file = Some(
            OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&input.path)
                .map_err(|error| error.to_string())?,
        );
        Ok(input)
    }

    fn registered_writer(&mut self) -> Result<(), String> {
        // The registered, still-gated child already owns the read descriptor.
        // Remove our temporary reader so child exit produces EPIPE, not a FIFO
        // held alive by the sender itself.
        let writer = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&self.path)
            .map_err(|error| error.to_string())?;
        self.file = Some(writer);
        Ok(())
    }
}

impl Drop for UnixInput {
    fn drop(&mut self) {
        drop(self.file.take());
        for result in [fs::remove_file(&self.path), fs::remove_dir(&self.directory)] {
            if let Err(error) = result {
                if error.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("Private interactive stdin cleanup failed: {error}");
                }
            }
        }
    }
}

pub(super) struct UnixOwner {
    socket: PathBuf,
    token: String,
    supervisor: Mutex<Option<Child>>,
    directory: Option<PathBuf>,
    groups: Mutex<HashMap<String, i32>>,
    cancelled: AtomicBool,
    registration: Mutex<()>,
}

impl UnixOwner {
    pub(super) fn start() -> Result<Arc<Self>, String> {
        Self::start_with_inheritance(true, None)
    }

    pub(super) fn start_with_python(python: &Path) -> Result<Arc<Self>, String> {
        Self::start_with_inheritance(true, Some(python))
    }

    #[cfg(test)]
    pub(super) fn start_fresh_for_test() -> Result<Arc<Self>, String> {
        Self::start_with_inheritance(false, None)
    }

    fn start_with_inheritance(
        inherit: bool,
        executable: Option<&Path>,
    ) -> Result<Arc<Self>, String> {
        if inherit {
            if let (Some(socket), Some(token)) = (
                std::env::var_os("ECU_OWNER_SOCKET"),
                std::env::var_os("ECU_OWNER_TOKEN"),
            ) {
                return Ok(Arc::new(Self {
                    socket: PathBuf::from(socket),
                    token: token.to_string_lossy().into_owned(),
                    supervisor: Mutex::new(None),
                    directory: None,
                    groups: Mutex::new(HashMap::new()),
                    cancelled: AtomicBool::new(false),
                    registration: Mutex::new(()),
                }));
            }
        }
        let python = match executable {
            Some(path) => std::borrow::Cow::Borrowed(path),
            None => std::borrow::Cow::Owned(PathBuf::from(
                std::env::var_os("AUTOSAR_PYTHON")
                    .ok_or("Set AUTOSAR_PYTHON to the absolute locked CPython executable")?,
            )),
        };
        if !python.is_absolute() || !python.is_file() {
            return Err("AUTOSAR_PYTHON must name an existing absolute executable".into());
        }
        let directory = private_directory()?;
        for asset in crate::resources::AssetInventory::embedded().entries() {
            if let Some(relative) = asset.relative_path.strip_prefix("scripts/")
                && relative.starts_with("ecu_tools/")
            {
                let path = directory.join(relative);
                fs::create_dir_all(path.parent().unwrap()).map_err(|error| error.to_string())?;
                fs::write(path, asset.bytes).map_err(|error| error.to_string())?;
            }
        }
        let socket = directory.join("owner.sock");
        let mut entropy = [0u8; 32];
        File::open("/dev/urandom")
            .and_then(|mut source| source.read_exact(&mut entropy))
            .map_err(|error| format!("Cannot obtain owner capability: {error}"))?;
        let mut token = String::with_capacity(64);
        for byte in entropy {
            write!(&mut token, "{byte:02x}").map_err(|error| error.to_string())?;
        }
        let log_path = directory.join("supervisor.log");
        let log = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&log_path)
            .map_err(|error| error.to_string())?;
        let mut child = Command::new(python.as_ref())
            .args(["-S", "-m", "ecu_tools.owner", "--socket"])
            .arg(&socket)
            .arg("--guardian-pid")
            .arg(std::process::id().to_string())
            .env("PYTHONPATH", &directory)
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::from(
                log.try_clone().map_err(|error| error.to_string())?,
            ))
            .stderr(Stdio::from(log))
            .spawn()
            .map_err(|error| format!("Cannot start supervisor: {error}"))?;
        let handshake = child
            .stdin
            .take()
            .ok_or_else(|| "Supervisor stdin was unavailable".to_owned())
            .and_then(|mut input| {
                input
                    .write_all(format!("{token}\n").as_bytes())
                    .map_err(|error| error.to_string())
            });
        if let Err(error) = handshake {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("supervisor_failed: capability transfer: {error}"));
        }
        let started = Instant::now();
        while !socket.exists() {
            if child
                .try_wait()
                .map_err(|error| error.to_string())?
                .is_some()
            {
                return Err(format!("supervisor_failed: {}", log_path.display()));
            }
            if started.elapsed() >= Duration::from_secs(30) {
                child.kill().map_err(|error| error.to_string())?;
                child.wait().map_err(|error| error.to_string())?;
                return Err(format!(
                    "supervisor_failed: socket not ready at {}",
                    socket.display()
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(Arc::new(Self {
            socket,
            token,
            supervisor: Mutex::new(Some(child)),
            directory: Some(directory),
            groups: Mutex::new(HashMap::new()),
            cancelled: AtomicBool::new(false),
            registration: Mutex::new(()),
        }))
    }

    pub(super) fn cancel(&self) -> Result<(), String> {
        self.cancelled.store(true, Ordering::Release);
        let _registration = self.registration.lock();
        let scopes: Vec<_> = self.groups.lock().keys().cloned().collect();
        let mut failure = None;
        for scope in scopes {
            match self.request(json!({"op": "close", "scope": scope, "reason": "cancelled"})) {
                Ok(result) if result["status"] != "cleanup_unconfirmed" => {}
                Ok(result) => failure = Some(format!("cleanup_unconfirmed: {result}")),
                Err(error) => failure = Some(error),
            }
        }
        failure.map_or(Ok(()), Err)
    }

    fn cleanup_mirror(&self) -> bool {
        let groups = self.groups.lock();
        for &pgid in groups.values() {
            signal_group(pgid, libc::SIGTERM);
        }
        let end = Instant::now() + Duration::from_secs(2);
        while Instant::now() < end && groups.values().any(|&pgid| group_alive(pgid)) {
            std::thread::sleep(Duration::from_millis(10));
        }
        for &pgid in groups.values() {
            if group_alive(pgid) {
                signal_group(pgid, libc::SIGKILL);
            }
        }
        !groups.values().any(|&pgid| group_alive(pgid))
    }

    fn exchange(&self, mut request: Value) -> Result<Value, String> {
        request["token"] = Value::String(self.token.clone());
        let mut stream = UnixStream::connect(&self.socket).map_err(|error| error.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|error| error.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(30)))
            .map_err(|error| error.to_string())?;
        let mut bytes = serde_json::to_vec(&request).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        stream
            .write_all(&bytes)
            .map_err(|error| error.to_string())?;
        let mut response = Vec::new();
        stream
            .take(1_048_577)
            .read_to_end(&mut response)
            .map_err(|error| error.to_string())?;
        if response.len() > 1_048_576 || response.last() != Some(&b'\n') {
            return Err("Supervisor response incomplete or too large".into());
        }
        serde_json::from_slice(&response).map_err(|error| error.to_string())
    }

    pub(super) fn request(&self, request: Value) -> Result<Value, String> {
        let response = self.exchange(request).map_err(|error| {
            let status = if self.cleanup_mirror() {
                "supervisor_failed"
            } else {
                "cleanup_unconfirmed"
            };
            format!("{status}: {error}")
        })?;
        if response["op"] == "error" {
            return Err(response["message"]
                .as_str()
                .unwrap_or("Unknown supervisor error")
                .to_owned());
        }
        if response["op"] == "closed" && response["status"] != "cleanup_unconfirmed" {
            if let Some(scope) = response["scope"].as_str() {
                self.groups.lock().remove(scope);
            }
        }
        Ok(response)
    }

    pub(super) fn spawn(
        self: &Arc<Self>,
        spec: &ProcessSpec,
        parent: Option<&str>,
    ) -> Result<UnixProcess, String> {
        let _registration = self.registration.lock();
        if self.cancelled.load(Ordering::Acquire) {
            return Err("Process owner is cancelled; no new command can start".into());
        }
        let argv: Vec<_> = spec
            .argv
            .iter()
            .map(|item| {
                item.to_str()
                    .ok_or("Command argv must be UTF-8 for the supervisor protocol")
            })
            .collect::<Result<_, _>>()?;
        let mut env: HashMap<String, String> = std::env::vars_os()
            .map(|(key, value)| {
                Ok((
                    key.into_string()
                        .map_err(|_| "Non-UTF-8 environment variable name")?,
                    value
                        .into_string()
                        .map_err(|_| "Non-UTF-8 environment variable value")?,
                ))
            })
            .collect::<Result<_, &str>>()?;
        for (key, value) in &spec.env {
            env.insert(
                key.to_str()
                    .ok_or("Non-UTF-8 environment variable name")?
                    .to_owned(),
                value
                    .to_str()
                    .ok_or("Non-UTF-8 environment variable value")?
                    .to_owned(),
            );
        }
        let inherited_parent = if self.supervisor.lock().is_none() {
            std::env::var("ECU_OWNER_SCOPE").ok()
        } else {
            None
        };
        let input = if spec.stdin_stream {
            Some(UnixInput::new()?)
        } else {
            None
        };
        let response = self.request(json!({
            "op": "reserve", "parent": parent.or(inherited_parent.as_deref()),
            "deadline_ns": spec.deadline_ns, "argv": argv, "cwd": spec.cwd,
            "env": env, "log_directory": spec.log_directory,
            "stdin_file": input.as_ref().map(|input| &input.path),
            "stdin_fifo": spec.stdin_stream,
            "completion": spec.completion,
        }))?;
        if response["op"] != "registered" {
            return Err(format!("Expected registered scope, got {response}"));
        }
        let scope = response["scope"]
            .as_str()
            .ok_or("Missing registered scope")?
            .to_owned();
        let pid = response["pid"].as_u64().ok_or("Missing registered PID")? as u32;
        let pgid = response["pgid"].as_i64().ok_or("Missing registered PGID")? as i32;
        self.groups.lock().insert(scope.clone(), pgid);
        if self.cancelled.load(Ordering::Acquire) {
            self.request(json!({"op": "close", "scope": scope, "reason": "cancelled"}))?;
            return Err("Process owner was cancelled before command release".into());
        }
        let mut process = UnixProcess {
            owner: Arc::clone(self),
            scope,
            pid,
            pgid,
            stdout: PathBuf::from(response["stdout"].as_str().ok_or("Missing stdout path")?),
            stderr: PathBuf::from(response["stderr"].as_str().ok_or("Missing stderr path")?),
            input,
            deadline_ns: response["deadline_ns"]
                .as_u64()
                .ok_or("Missing effective owner deadline")?,
            finished: false,
        };
        if let Some(input) = &mut process.input {
            input.registered_writer()?;
        }
        if let Err(error) = self.request(json!({"op": "release", "scope": process.scope})) {
            let _ =
                self.request(json!({"op": "close", "scope": process.scope, "reason": "cancelled"}));
            return Err(error);
        }
        Ok(process)
    }
    #[cfg(test)]
    pub(super) fn kill_supervisor_for_test(&self) -> Result<(), String> {
        let mut supervisor = self.supervisor.lock();
        let child = supervisor.as_mut().ok_or("Missing test supervisor")?;
        child.kill().map_err(|error| error.to_string())?;
        child.wait().map_err(|error| error.to_string())?;
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn remove_failed_test_logs(&self) {
        if let Some(directory) = &self.directory {
            let _ = fs::remove_dir_all(directory);
        }
    }
}

impl Drop for UnixOwner {
    fn drop(&mut self) {
        if let Some(mut child) = self.supervisor.lock().take() {
            let closed = self
                .request(json!({"op": "shutdown"}))
                .is_ok_and(|response| response["status"] == "closed");
            let end = Instant::now() + Duration::from_secs(3);
            while child.try_wait().ok().flatten().is_none() && Instant::now() < end {
                std::thread::sleep(Duration::from_millis(10));
            }
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
                let _ = child.wait();
            }
            if closed {
                if let Some(directory) = &self.directory {
                    let _ = fs::remove_dir_all(directory);
                }
            }
        }
    }
}

pub(super) struct UnixProcess {
    owner: Arc<UnixOwner>,
    scope: String,
    pid: u32,
    pgid: i32,
    stdout: PathBuf,
    stderr: PathBuf,
    input: Option<UnixInput>,
    deadline_ns: u64,
    finished: bool,
}

impl UnixProcess {
    pub(super) fn stdout_path(&self) -> &std::path::Path {
        &self.stdout
    }

    pub(super) fn close_stdin(&mut self) {
        if let Some(input) = &mut self.input {
            drop(input.file.take());
        }
    }

    pub(super) fn write_stdin(&mut self, bytes: &[u8]) -> Result<(), String> {
        if self.finished {
            return Err("Owned process already completed".into());
        }
        let mut offset = 0;
        while offset < bytes.len() {
            if monotonic_ns()? >= self.deadline_ns {
                self.cancel()?;
                return Err("Interactive stdin exceeded its absolute deadline".into());
            }
            let result = self
                .input
                .as_mut()
                .and_then(|input| input.file.as_mut())
                .ok_or("Interactive stdin is not open")?
                .write(&bytes[offset..]);
            match result {
                Ok(0) => return Err("Interactive stdin closed during write".into()),
                Ok(written) => offset += written,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        Ok(())
    }

    fn result(&self, response: &Value) -> Result<ProcessResult, String> {
        Ok(ProcessResult {
            scope: self.scope.clone(),
            pid: self.pid,
            pgid: Some(self.pgid),
            status: super::ProcessStatus::from_owner(
                response["status"]
                    .as_str()
                    .ok_or("Missing closure status")?,
            )?,
            exit_code: response["exit_code"].as_i64().map(|value| value as i32),
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
            descendants_reclaimed: response["descendants_reclaimed"]
                .as_bool()
                .ok_or("Missing confirmed descendant cleanup result")?,
        })
    }

    pub(super) fn wait(&mut self) -> Result<ProcessResult, String> {
        loop {
            let response = self
                .owner
                .request(json!({"op": "closed", "scope": self.scope}))?;
            match response["op"].as_str() {
                Some("closed") => {
                    self.finished = true;
                    return self.result(&response);
                }
                Some("running") => std::thread::sleep(Duration::from_millis(20)),
                _ => return Err(format!("Unexpected owner status: {response}")),
            }
        }
    }

    pub(super) fn cancel(&mut self) -> Result<ProcessResult, String> {
        let response = self.owner.request(json!({
            "op": "close", "scope": self.scope, "reason": "cancelled"
        }))?;
        self.finished = true;
        self.result(&response)
    }
}

impl Drop for UnixProcess {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.cancel();
        }
    }
}
