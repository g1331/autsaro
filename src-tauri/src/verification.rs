#[cfg(feature = "native-webdriver")]
use crate::background;
use crate::workbench::Reply;
#[cfg(feature = "native-webdriver")]
use crate::workbench::{AppState, OperationKind};
#[cfg(feature = "native-webdriver")]
use autosar_config_core::execution::ProcessSpec;
#[cfg(feature = "native-webdriver")]
use serde::Serialize;
#[cfg(feature = "native-webdriver")]
use std::ffi::OsString;
#[cfg(feature = "native-webdriver")]
use std::sync::Arc;
#[cfg(feature = "native-webdriver")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[cfg(feature = "native-webdriver")]
use tauri::State;

#[cfg(feature = "native-webdriver")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OwnedFailure {
    exit_code: Option<i32>,
    status: String,
    scope: String,
    stdout_path: String,
    stderr_path: String,
    descendants_reclaimed: bool,
    log: String,
}

#[cfg(feature = "native-webdriver")]
#[tauri::command]
pub(crate) async fn verification_owned_failure(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    delay_ms: u64,
) -> Result<Reply<OwnedFailure>, String> {
    if delay_ms > 5_000 {
        return Err("Owned verification delay exceeds 5000 ms".into());
    }
    background(state, move |state| {
        let operation = state.begin(&fingerprint, "owned verification failure", OperationKind::Native)?;
        let tools = operation.snapshot.tools.as_ref().ok_or("Configure the declared external execution tools first")?;
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| error.to_string())?.as_nanos();
        let directory = std::env::temp_dir().join(format!("autosar-owned-verification-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&directory).map_err(|error| error.to_string())?;
        let script = format!(
            "import sys,time\nfor i in range(20000):\n sys.stdout.write('Owned failure detail %06d: '%i+'x'*100+'\\n')\nsys.stdout.flush()\ntime.sleep({delay_ms}/1000)\nsys.stderr.write('Deliberate native acceptance failure\\n')\nsys.exit(23)\n"
        );
        let specification = ProcessSpec::for_duration(
            vec![tools.python.as_os_str().to_owned(), OsString::from("-c"), OsString::from(script)],
            directory.clone(),
            Vec::new(),
            Duration::from_secs(15),
            directory,
        )?;
        let result = operation.native_owner()?.spawn(specification, None)?.wait()?;
        let mut log = std::fs::read_to_string(&result.stdout).map_err(|error| error.to_string())?;
        let stderr = std::fs::read_to_string(&result.stderr).map_err(|error| error.to_string())?;
        log.push_str(&stderr);
        let value = OwnedFailure {
            exit_code: result.exit_code,
            status: format!("{:?}", result.status),
            scope: result.scope,
            stdout_path: result.stdout.display().to_string(),
            stderr_path: result.stderr.display().to_string(),
            descendants_reclaimed: result.descendants_reclaimed,
            log,
        };
        operation.commit(|_| Ok(value))
    })
    .await
}

#[cfg(not(feature = "native-webdriver"))]
#[tauri::command]
pub(crate) async fn verification_owned_failure() -> Result<Reply<()>, String> {
    Err("Owned verification requires the native-webdriver verification build".into())
}
