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

fn check_batch_token(
    fingerprint: &str,
    changes: &ChangeSet,
) -> Result<(), autosar_config_core::LocalizedText> {
    if changes.input_fingerprint != fingerprint {
        return Err(autosar_config_core::product_message!(
            "backend.configuration.change_identity_mismatch"
        )
        .into());
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn project_projection(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<ProjectProjection>, autosar_config_core::LocalizedText> {
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
) -> Result<Reply<String>, autosar_config_core::LocalizedText> {
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
) -> Result<Reply<ChangePreview>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        check_batch_token(&fingerprint, &change_set)?;
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.preview_configuration_change"),
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
) -> Result<Reply<ChangeOutcome>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        check_batch_token(&fingerprint, &change_set)?;
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.apply_configuration_change"),
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
) -> Result<Reply<ProjectProjection>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.accept_definition_catalog"),
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
) -> Result<Reply<ProjectProjection>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.remove_definition_catalog"),
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
) -> Result<Reply<WorkspaceView>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.open_member_project"),
            OperationKind::Open,
        )?;
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
) -> Result<Reply<ProjectCreationPreview>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.preview_project_creation"),
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
) -> Result<Reply<WorkspaceView>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.create_previewed_project"),
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
) -> Result<Reply<ProjectCreationPreview>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.preview_save_as_project"),
            OperationKind::Edit,
        )?;
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
) -> Result<Reply<WorkspaceView>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.save_as_previewed_project"),
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
) -> Result<Reply<()>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        state.configure_appearance(&fingerprint, appearance)
    })
    .await
}

#[tauri::command]
pub(crate) async fn configure_language(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    language: crate::workbench::Language,
) -> Result<Reply<()>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        state.configure_language(&fingerprint, language)
    })
    .await
}

#[cfg(feature = "native-webdriver")]
#[tauri::command]
pub(crate) async fn verification_metrics(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<autosar_config_core::verification::Metrics>, autosar_config_core::LocalizedText> {
    background(state, move |state| state.verification_metrics(&fingerprint)).await
}

#[cfg(not(feature = "native-webdriver"))]
#[tauri::command]
pub(crate) async fn verification_metrics() -> Result<Reply<()>, autosar_config_core::LocalizedText>
{
    Err(autosar_config_core::product_message!("backend.verification.metrics_build_required").into())
}

#[tauri::command]
pub(crate) async fn preview_application_initialization(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<
    Reply<autosar_config_core::arxml::ApplicationInitializationPreview>,
    autosar_config_core::LocalizedText,
> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!(
                "backend.operation.preview_application_initialization"
            ),
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
) -> Result<
    Reply<autosar_config_core::arxml::ApplicationInitializationOutcome>,
    autosar_config_core::LocalizedText,
> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!(
                "backend.operation.initialize_application_sources"
            ),
            OperationKind::Edit,
        )?;
        let workspace = operation.snapshot.workspace()?.clone();
        operation.initialize_application(workspace, &preview)
    })
    .await
}
