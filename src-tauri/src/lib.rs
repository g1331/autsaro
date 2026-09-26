use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::State;
use autosar_config_core::{schema, BuildReport, Direction, GenerationReport, RunReport, SavePreview, Workspace, WorkspaceView};

#[derive(Default)]
struct AppState {
    workspace: Mutex<Option<Workspace>>,
}

fn archive() -> PathBuf {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    schema::schema_archive(&repo)
}

fn with_workspace<T>(state: &AppState, operation: impl FnOnce(&mut Workspace) -> Result<T, String>) -> Result<T, String> {
    let mut guard = state.workspace.lock().map_err(|_| "工作区状态锁损坏".to_owned())?;
    operation(guard.as_mut().ok_or("请先创建或导入 ARXML 项目")?)
}

#[tauri::command]
fn create_project(state: State<'_, Arc<AppState>>, directory: String, name: String) -> Result<WorkspaceView, String> {
    let workspace = Workspace::create(Path::new(&directory), &name, archive())?;
    let view = workspace.view();
    *state.workspace.lock().map_err(|_| "工作区状态锁损坏")? = Some(workspace);
    Ok(view)
}

#[tauri::command]
fn open_project(state: State<'_, Arc<AppState>>, paths: Vec<String>) -> Result<WorkspaceView, String> {
    let workspace = Workspace::open(paths.into_iter().map(PathBuf::from).collect(), archive())?;
    let view = workspace.view();
    *state.workspace.lock().map_err(|_| "工作区状态锁损坏")? = Some(workspace);
    Ok(view)
}

#[tauri::command]
fn add_frame(state: State<'_, Arc<AppState>>, name: String, id: u32, dlc: u8, direction: Direction, period_ms: Option<u32>, timeout_ms: Option<u32>) -> Result<WorkspaceView, String> {
    with_workspace(&state, |w| w.add_frame(name, id, dlc, direction, period_ms, timeout_ms))
}

#[tauri::command]
fn add_signal(state: State<'_, Arc<AppState>>, frame_path: String, name: String, start_bit: u8, length: u8, initial_value: u32) -> Result<WorkspaceView, String> {
    with_workspace(&state, |w| w.add_signal(frame_path, name, start_bit, length, initial_value))
}

#[tauri::command]
fn update_frame(state: State<'_, Arc<AppState>>, path: String, changes: serde_json::Value) -> Result<WorkspaceView, String> {
    with_workspace(&state, |w| w.update_frame(&path, changes))
}

#[tauri::command]
fn update_signal(state: State<'_, Arc<AppState>>, path: String, changes: serde_json::Value) -> Result<WorkspaceView, String> {
    with_workspace(&state, |w| w.update_signal(&path, changes))
}

#[tauri::command]
fn configure_diagnostic(state: State<'_, Arc<AppState>>, request_id: u32, response_id: u32, s3_ms: u32, n_bs_ms: u32, n_cr_ms: u32, did: u16, signal_paths: Vec<String>, write_enabled: bool, reset_routine_id: Option<u16>, security_enabled: bool) -> Result<WorkspaceView, String> {
    with_workspace(&state, |w| w.configure_diagnostic(request_id, response_id, s3_ms, n_bs_ms, n_cr_ms, did, signal_paths, write_enabled, reset_routine_id, security_enabled))
}

#[tauri::command]
fn clear_diagnostic(state: State<'_, Arc<AppState>>) -> Result<WorkspaceView, String> {
    with_workspace(&state, Workspace::clear_diagnostic)
}

#[tauri::command]
fn configure_dtc(state: State<'_, Arc<AppState>>, code: u32, monitor_frame_path: String) -> Result<WorkspaceView, String> {
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
fn save_project(state: State<'_, Arc<AppState>>, revision: String) -> Result<WorkspaceView, String> {
    with_workspace(&state, |workspace| workspace.save_previewed(&revision))
}

#[tauri::command]
fn validate_project(state: State<'_, Arc<AppState>>) -> Result<WorkspaceView, String> {
    with_workspace(&state, Workspace::validate)
}

#[tauri::command]
fn generate_project(state: State<'_, Arc<AppState>>, output_directory: String) -> Result<GenerationReport, String> {
    with_workspace(&state, |workspace| {
        if workspace.view().dirty { return Err("请先保存 ARXML，再生成目标工程".into()); }
        autosar_config_core::generator::generate(workspace, Path::new(&output_directory))
    })
}

#[tauri::command]
async fn build_project(output_directory: String) -> Result<BuildReport, String> {
    tauri::async_runtime::spawn_blocking(move || autosar_config_core::generator::build(Path::new(&output_directory)))
        .await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn run_virtual(first_output_directory: String, second_output_directory: String) -> Result<RunReport, String> {
    tauri::async_runtime::spawn_blocking(move || autosar_config_core::host::run(Path::new(&first_output_directory), Path::new(&second_output_directory)))
        .await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn run_diagnostic(output_directory: String) -> Result<RunReport, String> {
    tauri::async_runtime::spawn_blocking(move || autosar_config_core::host::run_diagnostic(Path::new(&output_directory)))
        .await.map_err(|e| e.to_string())?
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(AppState::default()))
        .invoke_handler(tauri::generate_handler![create_project, open_project, add_frame, add_signal, update_frame, update_signal, configure_diagnostic, clear_diagnostic, configure_dtc, clear_dtc, preview_save_project, save_project, validate_project, generate_project, build_project, run_virtual, run_diagnostic])
        .run(tauri::generate_context!())
        .expect("Tauri 桌面工作台无法启动");
}
