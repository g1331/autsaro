use super::{ProcessResult, ProcessSpec, monotonic_ns};
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NONCE: AtomicU64 = AtomicU64::new(0);

fn group_alive(pgid: i32) -> bool {
    // SAFETY: negative PID addresses only the registered cooperative group.
    unsafe { libc::kill(-pgid, 0) == 0 }
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

pub(super) struct UnixOwner {
    socket: PathBuf,
    token: String,
    supervisor: Mutex<Option<Child>>,
    directory: Option<PathBuf>,
    groups: Mutex<HashMap<String, i32>>,
}

impl UnixOwner {
    pub(super) fn start() -> Result<Arc<Self>, String> {
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
            }));
        }
        let python = PathBuf::from(
            std::env::var_os("AUTOSAR_PYTHON")
                .ok_or("Set AUTOSAR_PYTHON to the absolute locked CPython executable")?,
        );
        if !python.is_absolute() || !python.is_file() {
            return Err("AUTOSAR_PYTHON must name an existing absolute executable".into());
        }
        let directory = private_directory()?;
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
        let mut child = Command::new(python)
            .args(["-m", "ecu_tools.owner", "--socket"])
            .arg(&socket)
            .arg("--guardian-pid")
            .arg(std::process::id().to_string())
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
        }))
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
        if response["op"] == "closed" {
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
        let inherited_parent = std::env::var("ECU_OWNER_SCOPE").ok();
        let response = self.request(json!({
            "op": "reserve", "parent": parent.or(inherited_parent.as_deref()),
            "deadline_ns": spec.deadline_ns, "argv": argv, "cwd": spec.cwd,
            "env": env, "log_directory": spec.log_directory,
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
        let process = UnixProcess {
            owner: Arc::clone(self),
            scope,
            pid,
            pgid,
            stdout: PathBuf::from(response["stdout"].as_str().ok_or("Missing stdout path")?),
            stderr: PathBuf::from(response["stderr"].as_str().ok_or("Missing stderr path")?),
            finished: false,
        };
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
            let _ = fs::remove_file(&self.socket);
            let _ = fs::remove_file(directory.join("supervisor.log"));
            let _ = fs::remove_dir(directory);
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
                    let _ = fs::remove_file(&self.socket);
                    let _ = fs::remove_file(directory.join("supervisor.log"));
                    let _ = fs::remove_dir(directory);
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
    finished: bool,
}

impl UnixProcess {
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
