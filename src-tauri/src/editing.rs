use crate::workbench::{AppState, OperationKind, Reply};
use crate::{SaveOutcome, background, commit_save, integration_background, integration_failure};
use autosar_config_core::integration::{IntegrationEdit, IntegrationInspection, PlanDiagnostic};
use autosar_config_core::{SavePreview, WorkspaceView};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub(super) async fn validate_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<WorkspaceView>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.validate"),
            OperationKind::Read,
        )?;
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
            .begin(
                &fingerprint,
                autosar_config_core::product_message!("backend.operation.inspect_integration"),
                OperationKind::Read,
            )
            .map_err(integration_failure)?;
        let workspace = operation
            .snapshot
            .workspace()
            .map_err(integration_failure)?;
        let value = workspace.inspect_integration();
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
            .begin(
                &fingerprint,
                autosar_config_core::product_message!("backend.operation.edit_integration"),
                OperationKind::Edit,
            )
            .map_err(integration_failure)?;
        let mut workspace = operation
            .snapshot
            .workspace()
            .map_err(integration_failure)?
            .clone();
        let value = workspace.edit_integration(changes)?;
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
                autosar_config_core::product_message!("backend.operation.preview_integration_save"),
                OperationKind::Read,
            )
            .map_err(integration_failure)?;
        let workspace = operation
            .snapshot
            .workspace()
            .map_err(integration_failure)?
            .clone();
        let value = workspace.preview_integration_save()?;
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
            .begin(
                &fingerprint,
                autosar_config_core::product_message!("backend.operation.save_integration"),
                OperationKind::Read,
            )
            .map_err(integration_failure)?;
        let workspace = operation
            .snapshot
            .workspace()
            .map_err(integration_failure)?
            .clone();
        let prepared = workspace.prepare_integration_save_previewed(&revision)?;
        operation
            .commit(|session| commit_save(session, prepared))
            .map_err(integration_failure)
    })
    .await
}
