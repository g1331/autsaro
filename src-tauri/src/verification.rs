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
pub(crate) struct NativeRequest {
    id: u64,
    command: &'static str,
}

#[cfg(feature = "native-webdriver")]
impl NativeRequest {
    pub(crate) fn begin(command: &'static str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let request = Self {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            command,
        };
        request.record("entered", None);
        request
    }

    pub(crate) fn finish(self, ok: bool) {
        self.record("returned", Some(ok));
    }

    fn record(&self, stage: &str, ok: Option<bool>) {
        use std::io::Write;
        // Keep native command receipt separate from WebView/driver delivery.
        let _ = writeln!(
            std::io::stderr(),
            "AUTOSAR_NATIVE_REQUEST {}",
            serde_json::json!({
                "id": self.id,
                "command": self.command,
                "stage": stage,
                "ok": ok,
                "at": SystemTime::now().duration_since(UNIX_EPOCH).map(|at| at.as_millis()).ok(),
                "pid": std::process::id(),
            })
        );
    }
}

#[cfg(feature = "native-webdriver")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(specta::Type))]
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
#[cfg_attr(test, specta::specta)]
pub(crate) async fn verification_owned_failure(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    delay_ms: u64,
) -> Result<Reply<OwnedFailure>, autosar_config_core::LocalizedText> {
    if delay_ms > 5_000 {
        return Err(autosar_config_core::product_message!(
            "backend.verification.delay_limit"
        ));
    }
    background(state, move |state| {
        let operation = state.begin(&fingerprint, autosar_config_core::product_message!("backend.operation.owned_verification_failure"), OperationKind::Native)?;
        let tools = operation.snapshot.tools.as_ref().ok_or(autosar_config_core::product_message!("backend.verification.tools_required"))?;
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| error.to_string())?.as_nanos();
        let directory = std::env::temp_dir().join(format!("autosar-owned-verification-{}-{stamp}", std::process::id()));
        #[cfg(unix)]
        let created = {
            use std::os::unix::fs::DirBuilderExt;
            std::fs::DirBuilder::new().mode(0o700).create(&directory)
        };
        #[cfg(not(unix))]
        let created = std::fs::create_dir(&directory);
        created.map_err(|error| error.to_string())?;
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
#[cfg_attr(test, specta::specta)]
pub(crate) async fn verification_owned_failure()
-> Result<Reply<()>, autosar_config_core::LocalizedText> {
    Err(autosar_config_core::product_message!(
        "backend.verification.build_required"
    ))
}

/// WebView2 150+ ignores `WEBVIEW2_*` variables in elevated hosts such as the
/// GitHub Windows runners, so msedgedriver's debugging port and profile reach
/// the verification build only through the WebView2 environment API.
#[cfg(all(windows, feature = "native-webdriver"))]
pub(crate) fn hand_webview2_to_driver(config: &mut tauri::Config) {
    let arguments = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| driver_browser_arguments(&value));
    let profile = std::env::var_os("WEBVIEW2_USER_DATA_FOLDER").map(std::path::PathBuf::from);
    for window in &mut config.app.windows {
        if arguments.is_some() {
            window.additional_browser_args = arguments.clone();
        }
        if profile.is_some() {
            window.data_directory = profile.clone();
        }
    }
}

/// Explicit arguments replace wry's defaults (wry 0.55 `src/webview2/mod.rs`),
/// and Chromium keeps only the last `--disable-features`, so both are merged.
// shortcut: splits on whitespace; msedgedriver passes no quoted values, revisit if it does.
#[cfg(all(any(windows, test), feature = "native-webdriver"))]
fn driver_browser_arguments(driver: &str) -> String {
    let mut disabled = vec!["msWebOOUI", "msPdfOOUI", "msSmartScreenProtection"];
    let mut others = vec!["--autoplay-policy=no-user-gesture-required"];
    for argument in driver.split_whitespace() {
        match argument.strip_prefix("--disable-features=") {
            Some(features) => disabled.extend(features.split(',')),
            None => others.push(argument),
        }
    }
    format!(
        "--disable-features={} {}",
        disabled.join(","),
        others.join(" ")
    )
}

#[cfg(all(test, feature = "native-webdriver"))]
mod tests {
    #[test]
    fn driver_arguments_keep_wry_defaults_in_one_feature_switch() {
        assert_eq!(
            super::driver_browser_arguments(
                "--disable-features=IgnoreDuplicateNavs,Prewarm --remote-debugging-port=0"
            ),
            "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,IgnoreDuplicateNavs,Prewarm \
             --autoplay-policy=no-user-gesture-required --remote-debugging-port=0"
        );
    }
}
