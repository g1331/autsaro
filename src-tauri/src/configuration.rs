use crate::background;
use crate::workbench::{AppState, Appearance, OperationKind, Reply};
use autosar_config_core::arxml::ProjectCreationPreview;
use autosar_config_core::project_model::{
    ChangeOutcome, ChangePreview, ChangeSet, ProjectProjection,
};
use autosar_config_core::{Workspace, WorkspaceView};
use std::path::Path;
use std::sync::Arc;
use tauri::State;

fn check_batch_token(fingerprint: &str, changes: &ChangeSet) -> Result<(), String> {
    if changes.input_fingerprint != fingerprint {
        return Err(
            "STALE_DELIVERY: ChangeSet input identity differs from the current request".into(),
        );
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn project_projection(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<ProjectProjection>, String> {
    background(state, move |state| {
        state.read_workspace(&fingerprint, |workspace| {
            workspace.project_projection(&fingerprint)
        })
    })
    .await
}

#[tauri::command]
pub(crate) async fn read_project_source(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    source_id: String,
) -> Result<Reply<String>, String> {
    background(state, move |state| {
        state.read_workspace(&fingerprint, |workspace| workspace.source_text(&source_id))
    })
    .await
}

#[tauri::command]
pub(crate) async fn prepare_configuration_change(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    change_set: ChangeSet,
) -> Result<Reply<ChangePreview>, String> {
    background(state, move |state| {
        check_batch_token(&fingerprint, &change_set)?;
        let operation = state.begin(
            &fingerprint,
            "preview configuration change",
            OperationKind::Edit,
        )?;
        let preview = operation
            .snapshot
            .workspace()?
            .prepare_change(&change_set)?;
        operation.commit(|_| Ok(preview))
    })
    .await
}

#[tauri::command]
pub(crate) async fn apply_configuration_change(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    change_set: ChangeSet,
    change_revision: String,
) -> Result<Reply<ChangeOutcome>, String> {
    background(state, move |state| {
        check_batch_token(&fingerprint, &change_set)?;
        let operation = state.begin(
            &fingerprint,
            "apply configuration change",
            OperationKind::Edit,
        )?;
        let mut workspace = operation.snapshot.workspace()?.clone();
        let outcome = workspace.apply_change(&change_set, &change_revision)?;
        operation.publish_configuration(workspace, outcome)
    })
    .await
}

#[tauri::command]
pub(crate) async fn import_definition_catalog(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    catalog_path: String,
) -> Result<Reply<ProjectProjection>, String> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            "accept definition catalog",
            OperationKind::Edit,
        )?;
        let mut workspace = operation.snapshot.workspace()?.clone();
        workspace
            .accept_definition_catalog(Path::new(&catalog_path), &state.definition_cache_root()?)?;
        let projection = workspace.project_projection(&fingerprint)?;
        operation.publish_projection(workspace, projection)
    })
    .await
}

#[tauri::command]
pub(crate) async fn remove_definition_catalog(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    catalog_id: String,
) -> Result<Reply<ProjectProjection>, String> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            "remove definition catalog",
            OperationKind::Edit,
        )?;
        let mut workspace = operation.snapshot.workspace()?.clone();
        workspace.remove_definition_catalog(&catalog_id)?;
        let projection = workspace.project_projection(&fingerprint)?;
        operation.publish_projection(workspace, projection)
    })
    .await
}

#[tauri::command]
pub(crate) async fn open_member_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    path: String,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        let operation = state.begin(&fingerprint, "open member project", OperationKind::Open)?;
        let workspace =
            Workspace::open_project_manifest(Path::new(&path), &state.definition_cache_root()?)?;
        let view = workspace.view();
        operation.publish(workspace, true, view)
    })
    .await
}

#[tauri::command]
pub(crate) async fn preview_project_creation(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    directory: String,
    name: String,
    template_id: String,
) -> Result<Reply<ProjectCreationPreview>, String> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            "preview project creation",
            OperationKind::Open,
        )?;
        let preview =
            Workspace::preview_project_creation(Path::new(&directory), &name, &template_id)?;
        operation.commit(|_| Ok(preview))
    })
    .await
}

#[tauri::command]
pub(crate) async fn create_project_previewed(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    preview: ProjectCreationPreview,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            "create previewed project",
            OperationKind::Open,
        )?;
        operation.commit(move |session| {
            let workspace = Workspace::create_project_previewed(&preview)?;
            let view = workspace.view();
            session.invalidate()?;
            session.workspace = Some(Arc::new(workspace));
            Ok(view)
        })
    })
    .await
}

#[tauri::command]
pub(crate) async fn preview_save_as_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    directory: String,
    name: String,
) -> Result<Reply<ProjectCreationPreview>, String> {
    background(state, move |state| {
        let operation =
            state.begin(&fingerprint, "preview save as project", OperationKind::Edit)?;
        let preview = operation
            .snapshot
            .workspace()?
            .preview_save_as_project(Path::new(&directory), &name)?;
        operation.commit(|_| Ok(preview))
    })
    .await
}

#[tauri::command]
pub(crate) async fn save_as_project_previewed(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    preview: ProjectCreationPreview,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            "save as previewed project",
            OperationKind::Edit,
        )?;
        operation.commit(|session| {
            let workspace = operation
                .snapshot
                .workspace()?
                .save_as_project_previewed(&preview)?;
            let view = workspace.view();
            session.invalidate()?;
            session.workspace = Some(Arc::new(workspace));
            Ok(view)
        })
    })
    .await
}

#[tauri::command]
pub(crate) async fn configure_appearance(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    appearance: Appearance,
) -> Result<Reply<()>, String> {
    background(state, move |state| {
        state.configure_appearance(&fingerprint, appearance)
    })
    .await
}

#[cfg(feature = "native-webdriver")]
#[tauri::command]
pub(crate) async fn verification_metrics(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<autosar_config_core::verification::Metrics>, String> {
    background(state, move |state| state.verification_metrics(&fingerprint)).await
}

#[cfg(not(feature = "native-webdriver"))]
#[tauri::command]
pub(crate) async fn verification_metrics() -> Result<Reply<()>, String> {
    Err("Verification metrics require the native-webdriver verification build".into())
}

#[tauri::command]
pub(crate) async fn preview_application_initialization(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<autosar_config_core::arxml::ApplicationInitializationPreview>, String> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            "preview application initialization",
            OperationKind::Edit,
        )?;
        let preview = operation
            .snapshot
            .workspace()?
            .preview_application_initialization()?;
        operation.commit(|_| Ok(preview))
    })
    .await
}

#[tauri::command]
pub(crate) async fn initialize_application_previewed(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    preview: autosar_config_core::arxml::ApplicationInitializationPreview,
) -> Result<Reply<autosar_config_core::arxml::ApplicationInitializationOutcome>, String> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            "initialize application sources",
            OperationKind::Edit,
        )?;
        let workspace = operation.snapshot.workspace()?.clone();
        operation.initialize_application(workspace, &preview)
    })
    .await
}
