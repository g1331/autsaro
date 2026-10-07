use crate::workbench::{AppState, OperationKind, Reply};
use crate::{
    SaveOutcome, background, commit_save, edit, integration_background, integration_failure,
};
use autosar_config_core::integration::{IntegrationEdit, IntegrationInspection, PlanDiagnostic};
use autosar_config_core::{DiagnosticSettings, Direction, SavePreview, Workspace, WorkspaceView};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub(super) async fn add_frame(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    name: String,
    id: u32,
    dlc: u8,
    direction: Direction,
    period_ms: Option<u32>,
    timeout_ms: Option<u32>,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        edit(&state, &fingerprint, |workspace| {
            workspace.add_frame(name, id, dlc, direction, period_ms, timeout_ms)
        })
    })
    .await
}
#[tauri::command]
pub(super) async fn add_signal(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    frame_path: String,
    name: String,
    start_bit: u8,
    length: u8,
    initial_value: u32,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        edit(&state, &fingerprint, |workspace| {
            workspace.add_signal(frame_path, name, start_bit, length, initial_value)
        })
    })
    .await
}
#[tauri::command]
pub(super) async fn update_frame(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    path: String,
    changes: serde_json::Value,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        edit(&state, &fingerprint, |workspace| {
            workspace.update_frame(&path, changes)
        })
    })
    .await
}
#[tauri::command]
pub(super) async fn update_signal(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    path: String,
    changes: serde_json::Value,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        edit(&state, &fingerprint, |workspace| {
            workspace.update_signal(&path, changes)
        })
    })
    .await
}
#[tauri::command]
pub(super) async fn configure_diagnostic(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    settings: DiagnosticSettings,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        edit(&state, &fingerprint, |workspace| {
            workspace.configure_diagnostic(settings)
        })
    })
    .await
}
#[tauri::command]
pub(super) async fn clear_diagnostic(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        edit(&state, &fingerprint, Workspace::clear_diagnostic)
    })
    .await
}
#[tauri::command]
pub(super) async fn configure_dtc(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    code: u32,
    monitor_frame_path: String,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        edit(&state, &fingerprint, |workspace| {
            workspace.configure_dtc(code, monitor_frame_path)
        })
    })
    .await
}
#[tauri::command]
pub(super) async fn clear_dtc(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        edit(&state, &fingerprint, Workspace::clear_dtc)
    })
    .await
}
#[tauri::command]
pub(super) async fn validate_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        let operation = state.begin(&fingerprint, "validate", OperationKind::Read)?;
        let mut workspace = operation.snapshot.workspace()?.clone();
        let value = workspace.validate()?;
        operation.publish(workspace, false, value)
    })
    .await
}
#[tauri::command]
pub(super) async fn inspect_integration(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<IntegrationInspection>, Vec<PlanDiagnostic>> {
    integration_background(state, move |state| {
        let operation = state
            .begin(&fingerprint, "inspect integration", OperationKind::Read)
            .map_err(integration_failure)?;
        let workspace = operation
            .snapshot
            .workspace()
            .map_err(integration_failure)?;
        let value = if workspace.uses_legacy_validation() {
            let dependencies = operation
                .snapshot
                .legacy_resources()
                .map_err(integration_failure)?;
            workspace.inspect_integration_legacy(&state.runtime, dependencies.mod_archive.clone())
        } else {
            workspace.inspect_integration(&state.runtime)
        };
        operation.commit(|_| Ok(value)).map_err(integration_failure)
    })
    .await
}
#[tauri::command]
pub(super) async fn edit_integration(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    changes: IntegrationEdit,
) -> Result<Reply<IntegrationInspection>, Vec<PlanDiagnostic>> {
    integration_background(state, move |state| {
        let operation = state
            .begin(&fingerprint, "edit integration", OperationKind::Edit)
            .map_err(integration_failure)?;
        let mut workspace = operation
            .snapshot
            .workspace()
            .map_err(integration_failure)?
            .clone();
        let value = if workspace.uses_legacy_validation() {
            let dependencies = operation
                .snapshot
                .legacy_resources()
                .map_err(integration_failure)?;
            workspace.edit_integration_legacy(
                &state.runtime,
                dependencies.mod_archive.clone(),
                changes,
            )?
        } else {
            workspace.edit_integration(&state.runtime, changes)?
        };
        operation
            .publish(workspace, true, value)
            .map_err(integration_failure)
    })
    .await
}
#[tauri::command]
pub(super) async fn preview_integration_save(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<SavePreview>, Vec<PlanDiagnostic>> {
    integration_background(state, move |state| {
        let operation = state
            .begin(
                &fingerprint,
                "preview integration save",
                OperationKind::Read,
            )
            .map_err(integration_failure)?;
        let workspace = operation
            .snapshot
            .workspace()
            .map_err(integration_failure)?
            .clone();
        let value = if workspace.uses_legacy_validation() {
            let dependencies = operation
                .snapshot
                .legacy_resources()
                .map_err(integration_failure)?;
            workspace
                .preview_integration_save_legacy(&state.runtime, dependencies.mod_archive.clone())?
        } else {
            workspace.preview_integration_save(&state.runtime)?
        };
        operation
            .publish(workspace, false, value)
            .map_err(integration_failure)
    })
    .await
}
#[tauri::command]
pub(super) async fn save_integration(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    revision: String,
) -> Result<Reply<SaveOutcome>, Vec<PlanDiagnostic>> {
    integration_background(state, move |state| {
        let operation = state
            .begin(&fingerprint, "save integration", OperationKind::Read)
            .map_err(integration_failure)?;
        let workspace = operation
            .snapshot
            .workspace()
            .map_err(integration_failure)?
            .clone();
        let prepared = if workspace.uses_legacy_validation() {
            let dependencies = operation
                .snapshot
                .legacy_resources()
                .map_err(integration_failure)?;
            workspace.prepare_integration_save_previewed_legacy(
                &state.runtime,
                dependencies.mod_archive.clone(),
                &revision,
            )?
        } else {
            workspace.prepare_integration_save_previewed(&state.runtime, &revision)?
        };
        operation
            .commit(|session| commit_save(session, prepared))
            .map_err(integration_failure)
    })
    .await
}
