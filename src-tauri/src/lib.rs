use autosar_config_core::integration::{
    DiagnosticCategory, IntegrationEdit, IntegrationInspection, PlanDependencies, PlanDiagnostic,
    RuntimeCatalog, build_ecu_project, verify_ecu_project,
};
use autosar_config_core::{
    BuildReport, DiagnosticSettings, Direction, GenerationPreview, GenerationReport, RunReport,
    SavePreview, Workspace, WorkspaceView, schema,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Default)]
struct AppState {
    workspace: Mutex<Option<Workspace>>,
}

fn archive() -> PathBuf {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    schema::schema_archive(&repo)
}

fn with_workspace<T>(
    state: &AppState,
    operation: impl FnOnce(&mut Workspace) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = state
        .workspace
        .lock()
        .map_err(|_| "工作区状态锁损坏".to_owned())?;
    operation(guard.as_mut().ok_or("请先创建或导入 ARXML 项目")?)
}

fn integration_failure(message: impl Into<String>) -> Vec<PlanDiagnostic> {
    vec![PlanDiagnostic {
        category: DiagnosticCategory::Tool,
        code: "WORKSPACE_UNAVAILABLE".into(),
        file: None,
        object: None,
        message: message.into(),
        remedy:
            "Import the standard ARXML input set and resolve the workspace error before continuing."
                .into(),
    }]
}

fn with_integration<T>(
    state: &AppState,
    operation: impl FnOnce(&mut Workspace, &RuntimeCatalog, PathBuf) -> Result<T, Vec<PlanDiagnostic>>,
) -> Result<T, Vec<PlanDiagnostic>> {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let runtime = RuntimeCatalog::from_repository(&repo)?;
    let dependencies = PlanDependencies::from_repository(&repo);
    let mut guard = state
        .workspace
        .lock()
        .map_err(|_| integration_failure("工作区状态锁损坏"))?;
    operation(
        guard
            .as_mut()
            .ok_or_else(|| integration_failure("请先导入标准 ARXML 输入"))?,
        &runtime,
        dependencies.mod_archive,
    )
}

#[tauri::command]
async fn inspect_integration(
    state: State<'_, Arc<AppState>>,
) -> Result<IntegrationInspection, Vec<PlanDiagnostic>> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        with_integration(&state, |workspace, runtime, archive| {
            Ok(workspace.inspect_integration(runtime, archive))
        })
    })
    .await
    .map_err(|error| integration_failure(error.to_string()))?
}

#[tauri::command]
async fn edit_integration(
    state: State<'_, Arc<AppState>>,
    changes: IntegrationEdit,
) -> Result<IntegrationInspection, Vec<PlanDiagnostic>> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        with_integration(&state, |workspace, runtime, archive| {
            workspace.edit_integration(runtime, archive, changes)
        })
    })
    .await
    .map_err(|error| integration_failure(error.to_string()))?
}

#[tauri::command]
async fn preview_integration_save(
    state: State<'_, Arc<AppState>>,
) -> Result<SavePreview, Vec<PlanDiagnostic>> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        with_integration(&state, |workspace, runtime, archive| {
            workspace.preview_integration_save(runtime, archive)
        })
    })
    .await
    .map_err(|error| integration_failure(error.to_string()))?
}

#[tauri::command]
async fn save_integration(
    state: State<'_, Arc<AppState>>,
    revision: String,
) -> Result<IntegrationInspection, Vec<PlanDiagnostic>> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        with_integration(&state, |workspace, runtime, archive| {
            workspace.save_integration_previewed(runtime, archive, &revision)
        })
    })
    .await
    .map_err(|error| integration_failure(error.to_string()))?
}

#[tauri::command]
fn create_project(
    state: State<'_, Arc<AppState>>,
    directory: String,
    name: String,
) -> Result<WorkspaceView, String> {
    let workspace = Workspace::create(Path::new(&directory), &name, archive())?;
    let view = workspace.view();
    *state.workspace.lock().map_err(|_| "工作区状态锁损坏")? = Some(workspace);
    Ok(view)
}

#[tauri::command]
fn workspace_view(state: State<'_, Arc<AppState>>) -> Result<WorkspaceView, String> {
    with_workspace(&state, |workspace| Ok(workspace.view()))
}

#[tauri::command]
fn open_project(
    state: State<'_, Arc<AppState>>,
    paths: Vec<String>,
) -> Result<WorkspaceView, String> {
    let workspace = Workspace::open(paths.into_iter().map(PathBuf::from).collect(), archive())?;
    let view = workspace.view();
    *state.workspace.lock().map_err(|_| "工作区状态锁损坏")? = Some(workspace);
    Ok(view)
}

#[tauri::command]
async fn open_handoff_project(
    state: State<'_, Arc<AppState>>,
    directory: String,
) -> Result<WorkspaceView, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let root = Path::new(&directory);
        let metadata: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("handoff.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let workspace = match metadata["format"].as_str() {
            Some("autosar-ecu-handoff-v1") => {
                let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
                let runtime =
                    RuntimeCatalog::from_repository(&repo).map_err(|e| format!("{e:?}"))?;
                Workspace::open_ecu_handoff(
                    root,
                    &PlanDependencies::from_repository(&repo),
                    &runtime,
                )
                .map_err(|e| format!("{e:?}"))?
            }
            Some("autosar-host-handoff-v1") => {
                autosar_config_core::generator::open_handoff(root, archive())?
            }
            _ => {
                return Err(
                    "Unsupported handoff format; the current workspace was retained.".into(),
                );
            }
        };
        let view = workspace.view();
        *state.workspace.lock().map_err(|_| "工作区状态锁损坏")? = Some(workspace);
        Ok(view)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn preview_ecu_project(
    state: State<'_, Arc<AppState>>,
    output_directory: String,
    handoff: bool,
) -> Result<GenerationPreview, Vec<PlanDiagnostic>> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        with_integration(&state, |workspace, runtime, archive| {
            let plan = workspace.saved_integration_plan(runtime, archive.clone())?;
            let files = if handoff {
                plan.ecu_handoff_files()?
            } else {
                plan.ecu_integration_files()?
            };
            workspace.saved_integration_plan(runtime, archive.clone())?;
            files
                .preview(Path::new(&output_directory))
                .map_err(integration_failure)
        })
    })
    .await
    .map_err(|e| integration_failure(e.to_string()))?
}

#[tauri::command]
async fn generate_ecu_project(
    state: State<'_, Arc<AppState>>,
    output_directory: String,
    handoff: bool,
    revision: String,
) -> Result<GenerationReport, Vec<PlanDiagnostic>> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        with_integration(&state, |workspace, runtime, archive| {
            let plan = workspace.saved_integration_plan(runtime, archive.clone())?;
            let files = if handoff {
                plan.ecu_handoff_files()?
            } else {
                plan.ecu_integration_files()?
            };
            workspace.saved_integration_plan(runtime, archive.clone())?;
            files
                .generate_previewed(Path::new(&output_directory), &revision)
                .map_err(integration_failure)
        })
    })
    .await
    .map_err(|e| integration_failure(e.to_string()))?
}

#[tauri::command]
async fn build_ecu(
    state: State<'_, Arc<AppState>>,
    output_directory: String,
    build_directory: String,
) -> Result<BuildReport, Vec<PlanDiagnostic>> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        with_integration(&state, |workspace, runtime, archive| {
            let plan = current_ecu(workspace, runtime, &archive, Path::new(&output_directory))?;
            let result = build_ecu_project(
                &plan,
                Path::new(&output_directory),
                Path::new(&build_directory),
            )
            .map_err(integration_failure)?;
            current_ecu(workspace, runtime, &archive, Path::new(&output_directory))?;
            Ok(result)
        })
    })
    .await
    .map_err(|e| integration_failure(e.to_string()))?
}

#[tauri::command]
async fn verify_ecu(
    state: State<'_, Arc<AppState>>,
    output_directory: String,
) -> Result<RunReport, Vec<PlanDiagnostic>> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        with_integration(&state, |workspace, runtime, archive| {
            let plan = current_ecu(workspace, runtime, &archive, Path::new(&output_directory))?;
            let result = verify_ecu_project(&plan, Path::new(&output_directory))
                .map_err(integration_failure)?;
            current_ecu(workspace, runtime, &archive, Path::new(&output_directory))?;
            Ok(result)
        })
    })
    .await
    .map_err(|e| integration_failure(e.to_string()))?
}

fn current_ecu(
    workspace: &Workspace,
    runtime: &RuntimeCatalog,
    archive: &Path,
    output: &Path,
) -> Result<autosar_config_core::integration::ValidatedIntegrationPlan, Vec<PlanDiagnostic>> {
    let plan = workspace.saved_integration_plan(runtime, archive.to_path_buf())?;
    let bytes = std::fs::read(output.join("integration.json"))
        .map_err(|e| integration_failure(e.to_string()))?;
    let data: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| integration_failure(e.to_string()))?;
    if data["plan"]
        != serde_json::to_value(plan.description())
            .map_err(|e| integration_failure(e.to_string()))?
    {
        return Err(integration_failure(
            "Generated source identities differ from the saved workspace. Regenerate before building or verifying.",
        ));
    }
    Ok(plan)
}

#[tauri::command]
fn add_frame(
    state: State<'_, Arc<AppState>>,
    name: String,
    id: u32,
    dlc: u8,
    direction: Direction,
    period_ms: Option<u32>,
    timeout_ms: Option<u32>,
) -> Result<WorkspaceView, String> {
    with_workspace(&state, |w| {
        w.add_frame(name, id, dlc, direction, period_ms, timeout_ms)
    })
}

#[tauri::command]
fn add_signal(
    state: State<'_, Arc<AppState>>,
    frame_path: String,
    name: String,
    start_bit: u8,
    length: u8,
    initial_value: u32,
) -> Result<WorkspaceView, String> {
    with_workspace(&state, |w| {
        w.add_signal(frame_path, name, start_bit, length, initial_value)
    })
}

#[tauri::command]
fn update_frame(
    state: State<'_, Arc<AppState>>,
    path: String,
    changes: serde_json::Value,
) -> Result<WorkspaceView, String> {
    with_workspace(&state, |w| w.update_frame(&path, changes))
}

#[tauri::command]
fn update_signal(
    state: State<'_, Arc<AppState>>,
    path: String,
    changes: serde_json::Value,
) -> Result<WorkspaceView, String> {
    with_workspace(&state, |w| w.update_signal(&path, changes))
}

#[tauri::command]
fn configure_diagnostic(
    state: State<'_, Arc<AppState>>,
    settings: DiagnosticSettings,
) -> Result<WorkspaceView, String> {
    with_workspace(&state, |w| w.configure_diagnostic(settings))
}

#[tauri::command]
fn clear_diagnostic(state: State<'_, Arc<AppState>>) -> Result<WorkspaceView, String> {
    with_workspace(&state, Workspace::clear_diagnostic)
}

#[tauri::command]
fn configure_dtc(
    state: State<'_, Arc<AppState>>,
    code: u32,
    monitor_frame_path: String,
) -> Result<WorkspaceView, String> {
    with_workspace(&state, |w| w.configure_dtc(code, monitor_frame_path))
}

#[tauri::command]
fn clear_dtc(state: State<'_, Arc<AppState>>) -> Result<WorkspaceView, String> {
    with_workspace(&state, Workspace::clear_dtc)
}

#[tauri::command]
fn preview_save_project(state: State<'_, Arc<AppState>>) -> Result<SavePreview, String> {
    with_workspace(&state, Workspace::preview_save)
}
#[tauri::command]
fn save_project(
    state: State<'_, Arc<AppState>>,
    revision: String,
) -> Result<WorkspaceView, String> {
    with_workspace(&state, |workspace| workspace.save_previewed(&revision))
}

#[tauri::command]
fn validate_project(state: State<'_, Arc<AppState>>) -> Result<WorkspaceView, String> {
    with_workspace(&state, Workspace::validate)
}

#[tauri::command]
fn preview_generate_project(
    state: State<'_, Arc<AppState>>,
    output_directory: String,
) -> Result<GenerationPreview, String> {
    with_workspace(&state, |workspace| {
        if workspace.view().dirty {
            return Err("请先保存 ARXML，再预览目标工程".into());
        }
        autosar_config_core::generator::preview_generate(workspace, Path::new(&output_directory))
    })
}

#[tauri::command]
fn preview_handoff_project(
    state: State<'_, Arc<AppState>>,
    output_directory: String,
) -> Result<GenerationPreview, String> {
    with_workspace(&state, |workspace| {
        autosar_config_core::generator::preview_handoff(workspace, Path::new(&output_directory))
    })
}

#[tauri::command]
fn generate_project(
    state: State<'_, Arc<AppState>>,
    output_directory: String,
    revision: String,
) -> Result<GenerationReport, String> {
    with_workspace(&state, |workspace| {
        if workspace.view().dirty {
            return Err("请先保存 ARXML，再生成目标工程".into());
        }
        autosar_config_core::generator::generate_previewed(
            workspace,
            Path::new(&output_directory),
            &revision,
        )
    })
}

#[tauri::command]
fn generate_handoff_project(
    state: State<'_, Arc<AppState>>,
    output_directory: String,
    revision: String,
) -> Result<GenerationReport, String> {
    with_workspace(&state, |workspace| {
        autosar_config_core::generator::generate_handoff_previewed(
            workspace,
            Path::new(&output_directory),
            &revision,
        )
    })
}

#[tauri::command]
async fn build_project(output_directory: String) -> Result<BuildReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        autosar_config_core::generator::build(Path::new(&output_directory))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn run_virtual(
    first_output_directory: String,
    second_output_directory: String,
) -> Result<RunReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        autosar_config_core::host::run(
            Path::new(&first_output_directory),
            Path::new(&second_output_directory),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn run_diagnostic(output_directory: String) -> Result<RunReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        autosar_config_core::host::run_diagnostic(Path::new(&output_directory))
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(AppState::default()))
        .invoke_handler(tauri::generate_handler![
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
