use crate::background;
use crate::workbench::{AppState, Capabilities, Reply};
use autosar_config_core::target::{BuildTarget, ExecutionSettings};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub(super) fn workbench_capabilities(
    state: State<'_, Arc<AppState>>,
) -> Result<Capabilities, autosar_config_core::LocalizedText> {
    #[cfg(feature = "native-webdriver")]
    let request = crate::verification::NativeRequest::begin("workbench_capabilities");
    let result = state.capabilities();
    #[cfg(feature = "native-webdriver")]
    request.finish(result.is_ok());
    result
}
#[tauri::command]
pub(super) async fn configure_execution_tools(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    tools: ExecutionSettings,
) -> Result<Reply<()>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        state.configure_tools(&fingerprint, tools)
    })
    .await
}
#[tauri::command]
pub(super) async fn select_build_target(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    target: BuildTarget,
) -> Result<Reply<()>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        state.select_target(&fingerprint, target)
    })
    .await
}
#[tauri::command]
pub(super) async fn cancel_operation(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<()>, autosar_config_core::LocalizedText> {
    background(state, move |state| state.cancel(&fingerprint)).await
}
