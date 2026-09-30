use super::{
    DiagnosticCategory, InputSource, PlanDependencies, PlanDiagnostic, RuntimeCatalog,
    ValidatedIntegrationPlan, build_plan,
};
use crate::generator;
use crate::{BuildReport, RunReport};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const FORMAT: &str = "autosar-ecu-handoff-v1";

fn issue(message: impl Into<String>) -> Vec<PlanDiagnostic> {
    vec![PlanDiagnostic {
        category: DiagnosticCategory::Input, code: "ECU_HANDOFF".into(), file: None, object: None,
        message: message.into(), remedy: "Preserve the old package. Restore its declared files and fixed dependencies, or recreate it from the matching saved inputs and workbench source version.".into(),
    }]
}
fn metadata(plan: &ValidatedIntegrationPlan) -> Value {
    let d = plan.description();
    json!({
        "format": FORMAT, "release": "CP/FO R24-11", "toolVersion": env!("CARGO_PKG_VERSION"),
        "profile": d.profile, "target": "Windows x64 GCC 16.1.0 controlled_logical_ms",
        "sources": d.sources, "runtimeSources": d.runtime_sources,
        "validationDependencies": d.validation_dependencies,
        "delivery": {
            "kernel": "kernel/source-manifest.json", "kernelLicense": "kernel/LICENSE.md",
            "compiler": "toolchain.json", "productRights": "owner-authorized internal use; no new public license grant",
            "externalInputs": "Legally obtain the pinned R24-11 XSD and MOD archives; not redistributed."
        }
    })
}

pub(super) fn files(
    plan: &ValidatedIntegrationPlan,
    project: Vec<(String, Vec<u8>)>,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut files: BTreeMap<_, _> = project
        .into_iter()
        .filter(|(p, _)| p != "files.list" && p != "files.sha256")
        .collect();
    let mut data = serde_json::to_vec_pretty(&metadata(plan)).map_err(|e| e.to_string())?;
    data.push(b'\n');
    files.insert("handoff.json".into(), data);
    files.extend(verification_files(plan)?);
    files.get_mut("README.md").ok_or("Missing project README")?.extend_from_slice(b"\n## Rebuildable ECU handoff\n\nThis package is explicitly autosar-ecu-handoff-v1, distinct from legacy autosar-host-handoff-v1. Saved ARXML logical paths and original bytes are under inputs/. handoff.json records their identities, fixed runtime producers and external validation dependencies. SHA-256 is an integrity check, not publisher authentication. The same source version of the workbench, plus legally acquired matching XSD/MOD archives, is required for validated reimport/regeneration; compiler binaries and official documents are not included. The complete fixed FreeRTOS source, fourteen patches and MIT notice are included. Product code is delivered for owner-authorized internal use only.\n\nMove the entire source directory; import this package in the workbench, validate and generate into another empty directory. Source identities and every regenerated product file are checked; edited or missing files fail without changing the package. Build with the pinned compiler into a separate empty output directory. Run verify.ps1 -BuildDirectory <another-empty-directory> to independently build and check actual CAN/DID echo, protocol timeout recovery and malformed-input rejection using the production HostBatch entry. Its endpoint input data does not contain a prior pass result. A failure or tool timeout is a failed check, never SC1/hardware approval. Keep local binaries and runtime state outside this sealed source tree.\n");
    Ok(generator::seal_files(files.into_iter().collect()))
}

pub(super) fn verification_files(
    plan: &ValidatedIntegrationPlan,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut files = BTreeMap::new();
    let d = plan.description();
    let read = d
        .component
        .data_ports
        .iter()
        .find(|p| p.read)
        .ok_or("Missing selected receive port")?;
    let write = d
        .component
        .data_ports
        .iter()
        .find(|p| !p.read)
        .ok_or("Missing selected transmit port")?;
    let rx = d
        .signals
        .iter()
        .find(|s| s.port == read.path)
        .ok_or("Missing receive channel")?;
    let tx = d
        .signals
        .iter()
        .find(|s| s.port == write.path)
        .ok_or("Missing transmit channel")?;
    let verification_inputs = json!({
        "format": "autosar-ecu-test-inputs-v1", "periodMs": d.component.period_ms,
        "receiveCanId": rx.can_id, "transmitCanId": tx.can_id,
        "requestCanId": d.diagnostic.request_can_id, "responseCanId": d.diagnostic.response_can_id,
        "did": d.diagnostic.did, "initialReceiveValue": read.initial_value,
        "receiveTimeoutMs": d.diagnostic.n_cr_ms,
        "scope": "Actual configured endpoints; independent fixed echo bytes and protocol assertions are in verify.ps1. This is test input, not a verification result."
    });
    files.insert(
        "verification/inputs.json".into(),
        serde_json::to_vec_pretty(&verification_inputs).map_err(|e| e.to_string())?,
    );
    files.insert(
        "verify.ps1".into(),
        include_bytes!("../../../runtime/ecu/verify.ps1").to_vec(),
    );
    Ok(files)
}

/// Verified source package. Its plan is reconstructed from actual input bytes.
pub struct EcuHandoff {
    root: PathBuf,
    plan: ValidatedIntegrationPlan,
}
impl EcuHandoff {
    pub fn plan(&self) -> &ValidatedIntegrationPlan {
        &self.plan
    }
    pub fn input_root(&self) -> PathBuf {
        self.root.join("inputs")
    }
    pub fn input_paths(&self) -> Vec<PathBuf> {
        self.plan
            .sources()
            .iter()
            .map(|s| self.input_root().join(s.logical_path()))
            .collect()
    }
}

pub fn open_ecu_handoff(
    output: &Path,
    dependencies: &PlanDependencies,
    runtime: &RuntimeCatalog,
) -> Result<EcuHandoff, Vec<PlanDiagnostic>> {
    let names = generator::verify_build_input(output).map_err(issue)?;
    let data: Value = serde_json::from_slice(
        &fs::read(output.join("handoff.json")).map_err(|e| issue(e.to_string()))?,
    )
    .map_err(|e| issue(e.to_string()))?;
    if data["format"] != FORMAT
        || data["release"] != "CP/FO R24-11"
        || data["toolVersion"] != env!("CARGO_PKG_VERSION")
    {
        return Err(issue("ECU handoff format, release or tool version differs"));
    }
    let declarations = data["sources"]
        .as_array()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| issue("ECU handoff has no input identities"))?;
    let mut sources = Vec::new();
    let mut paths = BTreeSet::new();
    for declaration in declarations {
        let logical = declaration["logicalPath"]
            .as_str()
            .ok_or_else(|| issue("Invalid logical input identity"))?;
        let input = InputSource::new(logical, Vec::new()).map_err(|e| vec![e])?;
        let path = format!("inputs/{}", input.logical_path());
        if !names.contains(&path) || !paths.insert(path.clone()) {
            return Err(issue(format!("Missing or repeated input: {path}")));
        }
        let bytes = fs::read(output.join(&path)).map_err(|e| issue(e.to_string()))?;
        if declaration["rawSha256"] != format!("{:x}", Sha256::digest(&bytes)) {
            return Err(issue(format!("Input identity differs: {path}")));
        }
        sources.push(InputSource::new(logical, bytes).map_err(|e| vec![e])?);
    }
    if names.iter().filter(|p| p.starts_with("inputs/")).count() != paths.len() {
        return Err(issue("Input file closure differs from handoff mapping"));
    }
    let plan = build_plan(&sources, dependencies, runtime)?;
    if metadata(&plan) != data {
        return Err(issue(
            "Revalidated input/runtime/dependency identities differ from handoff metadata",
        ));
    }
    let expected = plan.ecu_handoff_files()?;
    let expected_names: BTreeSet<_> = expected
        .files()
        .iter()
        .map(|(p, _)| p.as_str())
        .filter(|p| *p != "files.list" && *p != "files.sha256")
        .collect();
    if names.iter().map(String::as_str).collect::<BTreeSet<_>>() != expected_names {
        return Err(issue("Rebuilt ECU product file closure differs"));
    }
    for (path, bytes) in expected.files() {
        if fs::read(output.join(path)).map_err(|e| issue(e.to_string()))? != *bytes {
            return Err(issue(format!("Rebuilt ECU product bytes differ: {path}")));
        }
    }
    Ok(EcuHandoff {
        root: fs::canonicalize(output).map_err(|e| issue(e.to_string()))?,
        plan,
    })
}

fn checked_project(plan: &ValidatedIntegrationPlan, project: &Path) -> Result<(), String> {
    let names = generator::verify_build_input(project)?;
    let data: Value = serde_json::from_slice(
        &fs::read(project.join("integration.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if data["format"] != "autosar-ecu-integration-v1" || data["plan"]["profile"] != super::PROFILE {
        return Err("The source directory is not the selected ECU integration profile.".into());
    }
    let generated = plan.ecu_source_files().map_err(|e| format!("{e:?}"))?;
    let expected = if project.join("handoff.json").exists() {
        files(plan, generated.files().to_vec())?
    } else {
        generated.files().to_vec()
    };
    let expected_names: BTreeSet<_> = expected
        .iter()
        .map(|(p, _)| p.as_str())
        .filter(|p| *p != "files.list" && *p != "files.sha256")
        .collect();
    if names.iter().map(String::as_str).collect::<BTreeSet<_>>() != expected_names {
        return Err("Generated ECU file closure differs from the current validated plan.".into());
    }
    for (name, bytes) in expected {
        if fs::read(project.join(&name)).map_err(|e| e.to_string())? != bytes {
            return Err(format!(
                "Generated ECU source differs from the current validated plan: {name}"
            ));
        }
    }
    Ok(())
}
fn powershell(project: &Path, script: &str, output: &Path) -> Command {
    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(project.join(script));
    command
        .arg(if script == "build.ps1" {
            "-OutputDirectory"
        } else {
            "-BuildDirectory"
        })
        .arg(output);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW; no user desktop console.
    }
    command
}

struct PrivateDirectory {
    root: PathBuf,
    parent: PathBuf,
    reserved: PathBuf,
}
impl PrivateDirectory {
    fn new() -> Result<Self, String> {
        let temporary = std::env::temp_dir();
        let parent = fs::canonicalize(&temporary).map_err(|e| e.to_string())?;
        let root = generator::reserve_directory(
            &temporary,
            "ecu-command",
            std::ffi::OsStr::new("private"),
        )?;
        let reserved = fs::canonicalize(&root).map_err(|e| e.to_string())?;
        Ok(Self {
            root,
            parent,
            reserved,
        })
    }
    fn cleanup(&self) -> Result<(), String> {
        if !self.root.exists() {
            return Ok(());
        }
        let root = fs::canonicalize(&self.root).map_err(|e| e.to_string())?;
        if root != self.reserved || root.parent() != Some(self.parent.as_path()) {
            return Err("Private ECU cleanup path differs from its reserved directory.".into());
        }
        fs::remove_dir_all(root).map_err(|e| e.to_string())
    }
    fn finish(&self, result: Result<Output, String>) -> Result<Output, String> {
        let cleanup = self.cleanup();
        match (result, cleanup) {
            (Ok(mut output), Err(error)) => {
                output.stderr.extend_from_slice(
                    format!(
                        "\nTemporary ECU files retained at {}: {error}",
                        self.root.display()
                    )
                    .as_bytes(),
                );
                Ok(output)
            }
            (Err(error), Err(cleanup)) => Err(format!(
                "{error}\nTemporary files retained at {}: {cleanup}",
                self.root.display()
            )),
            (result, Ok(())) => result,
        }
    }
}
impl Drop for PrivateDirectory {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            eprintln!(
                "Temporary ECU cleanup failed at {}: {error}",
                self.root.display()
            );
        }
    }
}

fn run_bounded(
    command: &mut Command,
    directory: &Path,
    deadline: Duration,
) -> Result<Output, String> {
    let stdout = directory.join("command.stdout");
    let stderr = directory.join("command.stderr");
    let mut child = command
        .stdin(Stdio::null())
        .stdout(fs::File::create(&stdout).map_err(|e| e.to_string())?)
        .stderr(fs::File::create(&stderr).map_err(|e| e.to_string())?)
        .spawn()
        .map_err(|e| e.to_string())?;
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
            _ => {}
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let mut cleanup = Command::new("taskkill.exe")
                .args(["/PID", &child.id().to_string(), "/T", "/F"])
                .creation_flags(0x08000000)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| e.to_string())?;
            let limit = Instant::now();
            while cleanup.try_wait().map_err(|e| e.to_string())?.is_none()
                && limit.elapsed() < Duration::from_secs(3)
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            if cleanup.try_wait().map_err(|e| e.to_string())?.is_none() {
                cleanup.kill().map_err(|e| e.to_string())?;
            }
            let stopped = cleanup.wait().map_err(|e| e.to_string())?;
            if !stopped.success() && child.try_wait().map_err(|e| e.to_string())?.is_none() {
                child.kill().map_err(|e| e.to_string())?;
                child.wait().map_err(|e| e.to_string())?;
                return Err("ECU command timed out; parent closed but descendant cleanup could not be confirmed.".into());
            }
        }
        if child.try_wait().map_err(|e| e.to_string())?.is_none() {
            child.kill().map_err(|e| e.to_string())?;
        }
        child.wait().map_err(|e| e.to_string())?;
        return Err(format!(
            "ECU command exceeded its host watchdog or could not be observed; process tree closed.\n{}\n{}",
            fs::read_to_string(&stdout).unwrap_or_default(),
            fs::read_to_string(&stderr).unwrap_or_default()
        ));
    };
    Ok(Output {
        status,
        stdout: fs::read(stdout).map_err(|e| e.to_string())?,
        stderr: fs::read(stderr).map_err(|e| e.to_string())?,
    })
}
pub fn build_ecu_project(
    plan: &ValidatedIntegrationPlan,
    project: &Path,
    output: &Path,
) -> Result<BuildReport, String> {
    checked_project(plan, project)?;
    let capture = PrivateDirectory::new()?;
    let result = capture.finish(run_bounded(
        powershell(project, "build.ps1", output).arg("-HostBatch"),
        &capture.root,
        Duration::from_secs(180),
    ))?;
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    if !result.status.success() {
        return Err(format!("ECU build failed ({}): {log}", result.status));
    }
    checked_project(plan, project)?;
    let binary = output.join("ecu_host_batch.exe");
    if !binary.is_file() {
        return Err("Successful build did not produce its declared ECU binary.".into());
    }
    Ok(BuildReport {
        binary_path: binary.display().to_string(),
        log,
    })
}
pub fn verify_ecu_project(
    plan: &ValidatedIntegrationPlan,
    project: &Path,
) -> Result<RunReport, String> {
    checked_project(plan, project)?;
    let scratch = PrivateDirectory::new()?;
    let output = scratch.root.join("build");
    let result = scratch.finish(run_bounded(
        &mut powershell(project, "verify.ps1", &output),
        &scratch.root,
        Duration::from_secs(270),
    ))?;
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    checked_project(plan, project)?;
    let passed = result.status.success()
        && log
            .lines()
            .any(|s| s.starts_with("ECU_HANDOFF_VERIFY PASS:"));
    Ok(RunReport {
        passed,
        events: vec![
            "Independent production HostBatch CAN/DID, N_Cr recovery and malformed admission."
                .into(),
        ],
        log,
    })
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::os::windows::process::CommandExt;

    #[test]
    fn ecu_command_watchdog_closes_descendants_and_removes_private_files() {
        let scratch = PrivateDirectory::new().unwrap();
        let path = scratch.root.clone();
        let mut command = Command::new("python");
        command.args(["-c", "import subprocess,sys,time; p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(30)'],creationflags=0x08000000); print('child='+str(p.pid),flush=True); time.sleep(30)"]).creation_flags(0x08000000);
        let error = run_bounded(&mut command, &path, Duration::from_secs(2)).unwrap_err();
        assert!(error.contains("watchdog"), "{error}");
        let pid = fs::read_to_string(path.join("command.stdout"))
            .unwrap()
            .trim()
            .strip_prefix("child=")
            .unwrap()
            .parse::<u32>()
            .unwrap();
        let status = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ exit 1 }}"),
            ])
            .creation_flags(0x08000000)
            .status()
            .unwrap();
        assert!(
            status.success(),
            "watchdog left its compiler descendant running"
        );
        drop(scratch);
        assert!(!path.exists());
    }

    #[test]
    fn ecu_failed_command_and_spawn_failure_remove_private_files() {
        for missing in [false, true] {
            let scratch = PrivateDirectory::new().unwrap();
            let path = scratch.root.clone();
            let mut command = Command::new(if missing {
                "missing-ecu-test-tool.exe"
            } else {
                "powershell.exe"
            });
            command
                .args(["-NoProfile", "-NonInteractive", "-Command", "exit 7"])
                .creation_flags(0x08000000);
            let result = run_bounded(&mut command, &path, Duration::from_secs(5));
            if missing {
                assert!(result.is_err());
            } else {
                assert!(!result.unwrap().status.success());
            }
            drop(scratch);
            assert!(!path.exists());
        }
    }
}
