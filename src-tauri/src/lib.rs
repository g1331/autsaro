mod configuration;
mod verification;
mod workbench;

use autosar_config_core::generator::StagedBuild;
use autosar_config_core::integration::{
    DiagnosticCategory, IntegrationEdit, IntegrationInspection, PlanDiagnostic, RuntimeCatalog,
    ValidatedIntegrationPlan, build_ecu_project, verify_ecu_project,
};
use autosar_config_core::target::{BuildTarget, ExecutionSettings};
use autosar_config_core::{
    BuildReport, DiagnosticSettings, Direction, GenerationPreview, GenerationReport, RunReport,
    SavePreview, Workspace, WorkspaceView,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{Manager, State};
use workbench::{AppState, Capabilities, Operation, OperationKind, Reply, Session};

fn integration_failure(message: impl Into<String>) -> Vec<PlanDiagnostic> {
    vec![PlanDiagnostic {
        category: DiagnosticCategory::Tool,
        code: "WORKBENCH_CONTEXT".into(),
        file: None,
        object: None,
        message: message.into(),
        remedy:
            "Resolve the workspace, configuration or stale-operation conflict before continuing."
                .into(),
    }]
}

async fn background<T: Send + 'static>(
    state: State<'_, Arc<AppState>>,
    action: impl FnOnce(Arc<AppState>) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
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
    action: impl FnOnce(&mut Workspace) -> Result<T, String>,
) -> Result<Reply<T>, String> {
    let operation = state.begin(fingerprint, "edit", OperationKind::Edit)?;
    let mut workspace = operation.snapshot.workspace()?.clone();
    let value = action(&mut workspace)?;
    operation.publish(workspace, true, value)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveOutcome {
    workspace: WorkspaceView,
    error: Option<String>,
}

fn commit_save(
    session: &mut Session,
    prepared: autosar_config_core::arxml::PreparedSave,
) -> Result<SaveOutcome, String> {
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

#[tauri::command]
fn workbench_capabilities(state: State<'_, Arc<AppState>>) -> Result<Capabilities, String> {
    state.capabilities()
}
#[tauri::command]
async fn configure_validation_resources(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    xsd_archive: String,
    mod_archive: String,
) -> Result<Reply<()>, String> {
    background(state, move |state| {
        state.configure_resources(&fingerprint, xsd_archive.into(), mod_archive.into())
    })
    .await
}
#[tauri::command]
async fn configure_execution_tools(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    tools: ExecutionSettings,
) -> Result<Reply<()>, String> {
    background(state, move |state| {
        state.configure_tools(&fingerprint, tools)
    })
    .await
}
#[tauri::command]
async fn select_build_target(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    target: BuildTarget,
) -> Result<Reply<()>, String> {
    background(state, move |state| {
        state.select_target(&fingerprint, target)
    })
    .await
}
#[tauri::command]
async fn cancel_operation(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<()>, String> {
    background(state, move |state| state.cancel(&fingerprint)).await
}
#[tauri::command]
async fn close_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<()>, String> {
    background(state, move |state| state.close(&fingerprint)).await
}
#[tauri::command]
fn workspace_view(state: State<'_, Arc<AppState>>) -> Result<Reply<WorkspaceView>, String> {
    state.view()
}

#[tauri::command]
async fn create_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    directory: String,
    name: String,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        let operation = state.begin(&fingerprint, "create", OperationKind::Open)?;
        let workspace = Workspace::create(Path::new(&directory), &name)?;
        let view = workspace.view();
        operation.publish(workspace, true, view)
    })
    .await
}
#[tauri::command]
async fn open_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    paths: Vec<String>,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        let operation = state.begin(&fingerprint, "open", OperationKind::Open)?;
        let workspace = Workspace::open(paths.into_iter().map(PathBuf::from).collect())?;
        let view = workspace.view();
        operation.publish(workspace, true, view)
    })
    .await
}
#[tauri::command]
async fn open_handoff_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    directory: String,
    new_workspace_directory: Option<String>,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        let operation = state.begin(&fingerprint, "open handoff", OperationKind::Open)?;
        let root = Path::new(&directory);
        let metadata_path = root.join("handoff.json");
        let file = std::fs::symlink_metadata(&metadata_path).map_err(|error| error.to_string())?;
        if !file.is_file() || file.file_type().is_symlink() || file.len() > 50 * 1024 * 1024 {
            return Err("Handoff metadata must be a bounded regular file".into());
        }
        let metadata: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&metadata_path).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        let workspace = match metadata["format"].as_str() {
            Some("autosar-workbench-handoff-v2") => {
                let destination = new_workspace_directory
                    .as_deref()
                    .ok_or("Select a new empty workspace directory for a v2 snapshot import")?;
                let builtin = autosar_config_core::definitions::DefinitionCatalog::builtin()?;
                let catalog = operation
                    .snapshot
                    .workspace
                    .as_deref()
                    .map_or(&builtin, Workspace::definition_catalog);
                return operation.commit(|session| {
                    let imported = autosar_config_core::generator::delivery::open_handoff(
                        root,
                        Path::new(destination),
                        catalog,
                    )?;
                    let view = imported.workspace.view();
                    session.invalidate()?;
                    session.workspace = Some(Arc::new(imported.workspace));
                    Ok(view)
                });
            }
            Some("autosar-ecu-handoff-v1") => Workspace::open_ecu_handoff(
                root,
                operation.snapshot.legacy_resources()?,
                &state.runtime,
            )
            .map_err(|error| format!("{error:?}"))?,
            Some("autosar-host-handoff-v1") => autosar_config_core::generator::open_handoff(
                root,
                operation.snapshot.legacy_resources()?.xsd_archive.clone(),
            )?,
            _ => {
                return Err(
                    "Unsupported handoff format; the current workspace was retained.".into(),
                );
            }
        };
        let view = workspace.view();
        operation.publish(workspace, true, view)
    })
    .await
}

#[tauri::command]
async fn add_frame(
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
async fn add_signal(
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
async fn update_frame(
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
async fn update_signal(
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
async fn configure_diagnostic(
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
async fn clear_diagnostic(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        edit(&state, &fingerprint, Workspace::clear_diagnostic)
    })
    .await
}
#[tauri::command]
async fn configure_dtc(
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
async fn clear_dtc(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<WorkspaceView>, String> {
    background(state, move |state| {
        edit(&state, &fingerprint, Workspace::clear_dtc)
    })
    .await
}
#[tauri::command]
async fn validate_project(
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
async fn preview_save_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
) -> Result<Reply<SavePreview>, String> {
    background(state, move |state| {
        let operation = state.begin(&fingerprint, "preview save", OperationKind::Read)?;
        let mut workspace = operation.snapshot.workspace()?.clone();
        let value = workspace.preview_save()?;
        operation.publish(workspace, false, value)
    })
    .await
}
#[tauri::command]
async fn save_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    revision: String,
) -> Result<Reply<SaveOutcome>, String> {
    background(state, move |state| {
        let operation = state.begin(&fingerprint, "save", OperationKind::Read)?;
        let prepared = operation
            .snapshot
            .workspace()?
            .clone()
            .prepare_save_previewed(&revision)?;
        operation.commit(|session| commit_save(session, prepared))
    })
    .await
}

#[tauri::command]
async fn inspect_integration(
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
async fn edit_integration(
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
async fn preview_integration_save(
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
async fn save_integration(
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

fn ecu_plan(
    operation: &Operation,
    runtime: &RuntimeCatalog,
) -> Result<ValidatedIntegrationPlan, Vec<PlanDiagnostic>> {
    let workspace = operation
        .snapshot
        .workspace()
        .map_err(integration_failure)?;
    if workspace.uses_legacy_validation() {
        let dependencies = operation
            .snapshot
            .legacy_resources()
            .map_err(integration_failure)?;
        workspace.saved_integration_plan_legacy(runtime, dependencies.mod_archive.clone())
    } else {
        workspace.saved_integration_plan(runtime)
    }
}

fn prepare_ecu_for_operation<'a>(
    operation: &Operation,
    plan: &'a ValidatedIntegrationPlan,
    handoff: bool,
) -> Result<autosar_config_core::PreparedProject<'a>, Vec<PlanDiagnostic>> {
    let workspace = operation
        .snapshot
        .workspace()
        .map_err(integration_failure)?;
    if workspace.uses_legacy_validation() {
        autosar_config_core::prepare_ecu_project(plan, operation.snapshot.target, handoff)
    } else {
        autosar_config_core::prepared::prepare_ecu_project_for_workspace(
            workspace,
            plan,
            operation.snapshot.target,
            handoff,
        )
    }
}

#[tauri::command]
async fn preview_ecu_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
    handoff: bool,
) -> Result<Reply<GenerationPreview>, Vec<PlanDiagnostic>> {
    integration_background(state, move |state| {
        let operation = state
            .begin(&fingerprint, "preview ECU", OperationKind::Read)
            .map_err(integration_failure)?;
        let plan = ecu_plan(&operation, &state.runtime)?;
        let project = prepare_ecu_for_operation(&operation, &plan, handoff)?;
        let value = project
            .preview(Path::new(&output_directory))
            .map_err(integration_failure)?;
        operation
            .commit(|_| {
                operation.snapshot.workspace()?.verify_saved_sources()?;
                Ok(value)
            })
            .map_err(integration_failure)
    })
    .await
}
#[tauri::command]
async fn generate_ecu_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
    handoff: bool,
    revision: String,
) -> Result<Reply<GenerationReport>, Vec<PlanDiagnostic>> {
    integration_background(state, move |state| {
        let operation = state
            .begin(&fingerprint, "generate ECU", OperationKind::Read)
            .map_err(integration_failure)?;
        let plan = ecu_plan(&operation, &state.runtime)?;
        let project = prepare_ecu_for_operation(&operation, &plan, handoff)?;
        let staged = project
            .stage_previewed(Path::new(&output_directory), &revision)
            .map_err(integration_failure)?;
        operation
            .commit(|_| {
                operation.snapshot.workspace()?.verify_saved_sources()?;
                staged.commit()
            })
            .map_err(integration_failure)
    })
    .await
}
#[tauri::command]
async fn preflight_ecu(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    handoff: bool,
) -> Result<Reply<autosar_config_core::prepared::PreflightReport>, Vec<PlanDiagnostic>> {
    integration_background(state, move |state| {
        let native = state
            .capabilities()
            .map_err(integration_failure)?
            .native_execution;
        let operation = state
            .begin(
                &fingerprint,
                "preflight ECU",
                if native {
                    OperationKind::Native
                } else {
                    OperationKind::Read
                },
            )
            .map_err(integration_failure)?;
        let plan = ecu_plan(&operation, &state.runtime)?;
        let project = prepare_ecu_for_operation(&operation, &plan, handoff)?;
        let value = if native {
            project.native_preflight(
                operation
                    .snapshot
                    .tools
                    .as_deref()
                    .ok_or_else(|| integration_failure("尚未配置执行工具"))?,
                operation.native_owner().map_err(integration_failure)?,
            )
        } else {
            let mut report = project.preflight().clone();
            report.logs.push(format!(
                "Native preflight is not applicable to this host for {}",
                operation.snapshot.target.spec().id
            ));
            report
        };
        operation
            .commit(|_| {
                operation.snapshot.workspace()?.verify_saved_sources()?;
                Ok(value)
            })
            .map_err(integration_failure)
    })
    .await
}

fn check_target(output: &Path, target: BuildTarget) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_slice(
        &std::fs::read(output.join("target.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if value["target"] != serde_json::to_value(target).map_err(|error| error.to_string())? {
        return Err("生成目录不属于当前目标；请重新生成".into());
    }
    Ok(())
}
fn current_ecu(
    operation: &Operation,
    runtime: &RuntimeCatalog,
    output: &Path,
) -> Result<ValidatedIntegrationPlan, Vec<PlanDiagnostic>> {
    check_target(output, operation.snapshot.target).map_err(integration_failure)?;
    let plan = ecu_plan(operation, runtime)?;
    let data: serde_json::Value = serde_json::from_slice(
        &std::fs::read(output.join("integration.json"))
            .map_err(|error| integration_failure(error.to_string()))?,
    )
    .map_err(|error| integration_failure(error.to_string()))?;
    if data["plan"]
        != serde_json::to_value(plan.description())
            .map_err(|error| integration_failure(error.to_string()))?
    {
        return Err(integration_failure(
            "Generated source identities differ from the saved workspace. Regenerate before building or verifying.",
        ));
    }
    Ok(plan)
}

#[tauri::command]
async fn build_ecu(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
    build_directory: String,
) -> Result<Reply<BuildReport>, Vec<PlanDiagnostic>> {
    integration_background(state, move |state| {
        let operation = state
            .begin(&fingerprint, "build ECU", OperationKind::Native)
            .map_err(integration_failure)?;
        let source = Path::new(&output_directory);
        let plan = current_ecu(&operation, &state.runtime, source)?;
        let staged =
            StagedBuild::new(source, Path::new(&build_directory)).map_err(integration_failure)?;
        let result = build_ecu_project(
            &plan,
            source,
            staged.directory(),
            operation
                .snapshot
                .tools
                .as_deref()
                .ok_or_else(|| integration_failure("尚未配置执行工具"))?,
            operation.native_owner().map_err(integration_failure)?,
        )
        .map_err(integration_failure)?;
        current_ecu(&operation, &state.runtime, source)?;
        operation
            .commit(|_| {
                operation.snapshot.workspace()?.verify_saved_sources()?;
                let binary = Path::new(&result.binary_path)
                    .file_name()
                    .ok_or("构建未返回二进制名称")?;
                let destination = staged.commit()?;
                Ok(BuildReport {
                    binary_path: destination.join(binary).display().to_string(),
                    log: result.log,
                })
            })
            .map_err(integration_failure)
    })
    .await
}
#[tauri::command]
async fn verify_ecu(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
) -> Result<Reply<RunReport>, Vec<PlanDiagnostic>> {
    integration_background(state, move |state| {
        let operation = state
            .begin(&fingerprint, "verify ECU", OperationKind::Native)
            .map_err(integration_failure)?;
        let source = Path::new(&output_directory);
        let plan = current_ecu(&operation, &state.runtime, source)?;
        let value = verify_ecu_project(
            &plan,
            source,
            operation
                .snapshot
                .tools
                .as_deref()
                .ok_or_else(|| integration_failure("尚未配置执行工具"))?,
            operation.native_owner().map_err(integration_failure)?,
        )
        .map_err(integration_failure)?;
        current_ecu(&operation, &state.runtime, source)?;
        operation
            .commit(|_| {
                operation.snapshot.workspace()?.verify_saved_sources()?;
                Ok(value)
            })
            .map_err(integration_failure)
    })
    .await
}

fn preview_host(
    state: &Arc<AppState>,
    fingerprint: &str,
    output: &Path,
    handoff: bool,
) -> Result<Reply<GenerationPreview>, String> {
    let operation = state.begin(fingerprint, "preview host", OperationKind::Read)?;
    let mut workspace = operation.snapshot.workspace()?.clone();
    let value = autosar_config_core::prepare_host_project(
        &mut workspace,
        operation.snapshot.target,
        handoff,
    )?
    .preview(output)?;
    operation.commit(|_| {
        operation.snapshot.workspace()?.verify_saved_sources()?;
        Ok(value)
    })
}
fn generate_host(
    state: &Arc<AppState>,
    fingerprint: &str,
    output: &Path,
    revision: &str,
    handoff: bool,
) -> Result<Reply<GenerationReport>, String> {
    let operation = state.begin(fingerprint, "generate host", OperationKind::Read)?;
    let mut workspace = operation.snapshot.workspace()?.clone();
    let staged = autosar_config_core::prepare_host_project(
        &mut workspace,
        operation.snapshot.target,
        handoff,
    )?
    .stage_previewed(output, revision)?;
    operation.commit(|_| {
        operation.snapshot.workspace()?.verify_saved_sources()?;
        staged.commit()
    })
}
#[tauri::command]
async fn preview_generate_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
) -> Result<Reply<GenerationPreview>, String> {
    background(state, move |state| {
        preview_host(&state, &fingerprint, Path::new(&output_directory), false)
    })
    .await
}
#[tauri::command]
async fn preview_handoff_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
) -> Result<Reply<GenerationPreview>, String> {
    background(state, move |state| {
        preview_host(&state, &fingerprint, Path::new(&output_directory), true)
    })
    .await
}
#[tauri::command]
async fn generate_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
    revision: String,
) -> Result<Reply<GenerationReport>, String> {
    background(state, move |state| {
        generate_host(
            &state,
            &fingerprint,
            Path::new(&output_directory),
            &revision,
            false,
        )
    })
    .await
}
#[tauri::command]
async fn generate_handoff_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
    revision: String,
) -> Result<Reply<GenerationReport>, String> {
    background(state, move |state| {
        generate_host(
            &state,
            &fingerprint,
            Path::new(&output_directory),
            &revision,
            true,
        )
    })
    .await
}

#[tauri::command]
async fn build_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
    build_directory: String,
) -> Result<Reply<BuildReport>, String> {
    background(state, move |state| {
        let operation = state.begin(&fingerprint, "build host", OperationKind::Native)?;
        let source = Path::new(&output_directory);
        check_target(source, operation.snapshot.target)?;
        operation.snapshot.workspace()?.verify_saved_sources()?;
        let staged = StagedBuild::new(source, Path::new(&build_directory))?;
        let result = autosar_config_core::generator::build(
            source,
            staged.directory(),
            operation
                .snapshot
                .tools
                .as_deref()
                .ok_or("尚未配置执行工具")?,
            operation.native_owner()?,
        )?;
        operation.commit(|_| {
            operation.snapshot.workspace()?.verify_saved_sources()?;
            let binary = Path::new(&result.binary_path)
                .file_name()
                .ok_or("构建未返回二进制名称")?;
            let destination = staged.commit()?;
            Ok(BuildReport {
                binary_path: destination.join(binary).display().to_string(),
                log: result.log,
            })
        })
    })
    .await
}
#[tauri::command]
async fn run_virtual(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    first_output_directory: String,
    second_output_directory: String,
    first_binary_path: String,
    second_binary_path: String,
) -> Result<Reply<RunReport>, String> {
    background(state, move |state| {
        let operation = state.begin(&fingerprint, "run virtual", OperationKind::Native)?;
        check_target(
            Path::new(&first_output_directory),
            operation.snapshot.target,
        )?;
        check_target(
            Path::new(&second_output_directory),
            operation.snapshot.target,
        )?;
        operation.snapshot.workspace()?.verify_saved_sources()?;
        let value = autosar_config_core::host::run(
            Path::new(&first_output_directory),
            Path::new(&first_binary_path),
            Path::new(&second_output_directory),
            Path::new(&second_binary_path),
            operation.native_owner()?,
        )?;
        operation.commit(|_| {
            operation.snapshot.workspace()?.verify_saved_sources()?;
            Ok(value)
        })
    })
    .await
}
#[tauri::command]
async fn run_diagnostic(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
    binary_path: String,
) -> Result<Reply<RunReport>, String> {
    background(state, move |state| {
        let operation = state.begin(&fingerprint, "run diagnostic", OperationKind::Native)?;
        check_target(Path::new(&output_directory), operation.snapshot.target)?;
        operation.snapshot.workspace()?.verify_saved_sources()?;
        let value = autosar_config_core::host::run_diagnostic(
            Path::new(&output_directory),
            Path::new(&binary_path),
            operation.native_owner()?,
        )?;
        operation.commit(|_| {
            operation.snapshot.workspace()?.verify_saved_sources()?;
            Ok(value)
        })
    })
    .await
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
                            "AUTOSAR_CONFIG_DIR must be an absolute directory",
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
            configuration::verification_metrics,
            verification::verification_owned_failure,
            workbench_capabilities,
            configure_validation_resources,
            configure_execution_tools,
            select_build_target,
            cancel_operation,
            close_project,
            workspace_view,
            inspect_integration,
            edit_integration,
            preview_integration_save,
            save_integration,
            create_project,
            open_project,
            open_handoff_project,
            preview_ecu_project,
            generate_ecu_project,
            build_ecu,
            verify_ecu,
            preflight_ecu,
            add_frame,
            add_signal,
            update_frame,
            update_signal,
            configure_diagnostic,
            clear_diagnostic,
            configure_dtc,
            clear_dtc,
            preview_save_project,
            save_project,
            validate_project,
            preview_generate_project,
            generate_project,
            preview_handoff_project,
            generate_handoff_project,
            build_project,
            run_virtual,
            run_diagnostic
        ])
        .run(tauri::generate_context!())
        .expect("Tauri 桌面工作台无法启动");
}
