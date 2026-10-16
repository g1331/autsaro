use crate::workbench::{AppState, OperationKind, Reply};
use crate::{SaveOutcome, background, commit_save};
use autosar_config_core::{SavePreview, Workspace, WorkspaceView};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub(super) async fn close_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<()>, autosar_config_core::LocalizedText> {
    background(state, move |state| state.close(&fingerprint)).await
}
#[tauri::command]
pub(super) fn workspace_view(
    state: State<'_, Arc<AppState>>,
) -> Result<Reply<WorkspaceView>, autosar_config_core::LocalizedText> {
    state.view()
}

#[tauri::command]
pub(super) async fn create_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    directory: String,
    name: String,
) -> Result<Reply<WorkspaceView>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.create"),
            OperationKind::Open,
        )?;
        let workspace = Workspace::create(Path::new(&directory), &name)?;
        let view = workspace.view();
        operation.publish(workspace, true, view)
    })
    .await
}
#[tauri::command]
pub(super) async fn open_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    paths: Vec<String>,
) -> Result<Reply<WorkspaceView>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.open"),
            OperationKind::Open,
        )?;
        let workspace = Workspace::open(paths.into_iter().map(PathBuf::from).collect())?;
        let view = workspace.view();
        operation.publish(workspace, true, view)
    })
    .await
}
#[tauri::command]
pub(super) async fn open_handoff_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    directory: String,
    new_workspace_directory: Option<String>,
) -> Result<Reply<WorkspaceView>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.open_handoff"),
            OperationKind::Open,
        )?;
        let root = Path::new(&directory);
        autosar_config_core::generator::delivery::require_current_format(root)?;
        let destination =
            new_workspace_directory
                .as_deref()
                .ok_or(autosar_config_core::product_message!(
                    "backend.project.handoff_destination_required"
                ))?;
        let builtin = autosar_config_core::definitions::DefinitionCatalog::builtin()?;
        let catalog = operation
            .snapshot
            .workspace
            .as_deref()
            .map_or(&builtin, Workspace::definition_catalog);
        operation.commit(|session| {
            let imported = autosar_config_core::generator::delivery::open_handoff(
                root,
                Path::new(destination),
                catalog,
            )?;
            let view = imported.workspace.view();
            session.invalidate()?;
            session.workspace = Some(Arc::new(imported.workspace));
            Ok(view)
        })
    })
    .await
}

#[tauri::command]
pub(super) async fn preview_save_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<SavePreview>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.preview_save"),
            OperationKind::Read,
        )?;
        let mut workspace = operation.snapshot.workspace()?.clone();
        let value = workspace.preview_save()?;
        operation.publish(workspace, false, value)
    })
    .await
}
#[tauri::command]
pub(super) async fn save_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    revision: String,
) -> Result<Reply<SaveOutcome>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.save"),
            OperationKind::Read,
        )?;
        let prepared = operation
            .snapshot
            .workspace()?
            .clone()
            .prepare_save_previewed(&revision)?;
        operation.commit(|session| commit_save(session, prepared))
    })
    .await
}
