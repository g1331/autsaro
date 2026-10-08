mod configuration;
mod delivery;
mod editing;
mod project;
mod settings;
mod verification;
mod workbench;

use autosar_config_core::integration::{DiagnosticCategory, PlanDiagnostic};
use autosar_config_core::{Workspace, WorkspaceView};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Manager, State};
use workbench::{AppState, OperationKind, Reply, Session};

fn integration_failure(
    message: impl Into<autosar_config_core::LocalizedText>,
) -> Vec<PlanDiagnostic> {
    vec![PlanDiagnostic {
        category: DiagnosticCategory::Tool,
        code: "WORKBENCH_CONTEXT".into(),
        file: None,
        object: None,
        message: message.into(),
        remedy: autosar_config_core::product_message!("backend.workbench.resolve_context"),
    }]
}

async fn background<T: Send + 'static>(
    state: State<'_, Arc<AppState>>,
    action: impl FnOnce(Arc<AppState>) -> Result<T, autosar_config_core::LocalizedText> + Send + 'static,
) -> Result<T, autosar_config_core::LocalizedText> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || action(state))
        .await
        .map_err(|error| error.to_string())?
}

async fn integration_background<T: Send + 'static>(
    state: State<'_, Arc<AppState>>,
    action: impl FnOnce(Arc<AppState>) -> Result<T, Vec<PlanDiagnostic>> + Send + 'static,
) -> Result<T, Vec<PlanDiagnostic>> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || action(state))
        .await
        .map_err(|error| integration_failure(error.to_string()))?
}

fn edit<T>(
    state: &Arc<AppState>,
    fingerprint: &str,
    action: impl FnOnce(&mut Workspace) -> Result<T, autosar_config_core::LocalizedText>,
) -> Result<Reply<T>, autosar_config_core::LocalizedText> {
    let operation = state.begin(
        fingerprint,
        autosar_config_core::product_message!("backend.operation.edit"),
        OperationKind::Edit,
    )?;
    let mut workspace = operation.snapshot.workspace()?.clone();
    let value = action(&mut workspace)?;
    operation.publish(workspace, true, value)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveOutcome {
    workspace: WorkspaceView,
    error: Option<autosar_config_core::LocalizedText>,
}

fn commit_save(
    session: &mut Session,
    prepared: autosar_config_core::arxml::PreparedSave,
) -> Result<SaveOutcome, autosar_config_core::LocalizedText> {
    session.invalidate()?;
    let (workspace, error) = match prepared.commit() {
        Ok(workspace) => (workspace, None),
        Err(failure) => {
            let (workspace, error) = failure.into_parts();
            (workspace, Some(error))
        }
    };
    let value = SaveOutcome {
        workspace: workspace.view(),
        error,
    };
    session.workspace = Some(Arc::new(workspace));
    Ok(value)
}

pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    #[cfg(all(feature = "native-webdriver", target_os = "macos"))]
    let builder = builder.plugin(tauri_plugin_wdio_webdriver::init());
    builder
        .setup(|app| {
            let config_dir = match std::env::var_os("AUTOSAR_CONFIG_DIR") {
                Some(value) => {
                    let path = PathBuf::from(value);
                    if !path.is_absolute() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            autosar_config_core::product_message!(
                                "backend.workbench.config_dir_absolute"
                            ),
                        )
                        .into());
                    }
                    path
                }
                None => app.path().app_config_dir()?,
            };
            let state =
                AppState::new(config_dir.join("settings.json")).map_err(std::io::Error::other)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            configuration::project_projection,
            configuration::read_project_source,
            configuration::prepare_configuration_change,
            configuration::apply_configuration_change,
            configuration::import_definition_catalog,
            configuration::remove_definition_catalog,
            configuration::open_member_project,
            configuration::preview_project_creation,
            configuration::create_project_previewed,
            configuration::preview_save_as_project,
            configuration::save_as_project_previewed,
            configuration::preview_application_initialization,
            configuration::initialize_application_previewed,
            configuration::configure_appearance,
            configuration::configure_language,
            configuration::verification_metrics,
            verification::verification_owned_failure,
            settings::workbench_capabilities,
            settings::configure_validation_resources,
            settings::configure_execution_tools,
            settings::select_build_target,
            settings::cancel_operation,
            project::close_project,
            project::workspace_view,
            editing::inspect_integration,
            editing::edit_integration,
            editing::preview_integration_save,
            editing::save_integration,
            project::create_project,
            project::open_project,
            project::open_handoff_project,
            delivery::preview_ecu_project,
            delivery::generate_ecu_project,
            delivery::build_ecu,
            delivery::verify_ecu,
            delivery::preflight_ecu,
            editing::add_frame,
            editing::add_signal,
            editing::update_frame,
            editing::update_signal,
            editing::configure_diagnostic,
            editing::clear_diagnostic,
            editing::configure_dtc,
            editing::clear_dtc,
            project::preview_save_project,
            project::save_project,
            editing::validate_project,
            delivery::preview_generate_project,
            delivery::generate_project,
            delivery::preview_handoff_project,
            delivery::generate_handoff_project,
            delivery::build_project,
            delivery::run_virtual,
            delivery::run_diagnostic
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| panic!("{error}"));
}
