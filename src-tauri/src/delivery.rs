use crate::workbench::{AppState, Operation, OperationKind, Reply};
use crate::{background, integration_background, integration_failure};
use autosar_config_core::generator::StagedBuild;
use autosar_config_core::integration::{
    PlanDiagnostic, RuntimeCatalog, ValidatedIntegrationPlan, build_ecu_project, verify_ecu_project,
};
use autosar_config_core::target::BuildTarget;
use autosar_config_core::{BuildReport, GenerationPreview, GenerationReport, RunReport};
use std::path::Path;
use std::sync::Arc;
use tauri::State;

pub(super) fn ecu_plan(
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

pub(super) fn prepare_ecu_for_operation<'a>(
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
pub(super) async fn preview_ecu_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
    handoff: bool,
) -> Result<Reply<GenerationPreview>, Vec<PlanDiagnostic>> {
    integration_background(state, move |state| {
        let operation = state
            .begin(
                &fingerprint,
                autosar_config_core::product_message!("backend.operation.preview_ecu"),
                OperationKind::Read,
            )
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
pub(super) async fn generate_ecu_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
    handoff: bool,
    revision: String,
) -> Result<Reply<GenerationReport>, Vec<PlanDiagnostic>> {
    integration_background(state, move |state| {
        let operation = state
            .begin(
                &fingerprint,
                autosar_config_core::product_message!("backend.operation.generate_ecu"),
                OperationKind::Read,
            )
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
pub(super) async fn preflight_ecu(
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
                autosar_config_core::product_message!("backend.operation.preflight_ecu"),
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
                    .ok_or_else(|| integration_failure(autosar_config_core::product_message!("backend.workbench.execution_tools_missing")))?,
                operation.native_owner().map_err(integration_failure)?,
            )
        } else {
            let mut report = project.preflight().clone();
            report.logs.push(autosar_config_core::product_message!("backend.delivery.native_preflight_not_applicable", "target" => operation.snapshot.target.spec().id));
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

pub(super) fn check_target(
    output: &Path,
    target: BuildTarget,
) -> Result<(), autosar_config_core::LocalizedText> {
    let value: serde_json::Value = serde_json::from_slice(
        &std::fs::read(output.join("target.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if value["target"] != serde_json::to_value(target).map_err(|error| error.to_string())? {
        return Err(
            autosar_config_core::product_message!("backend.delivery.target_mismatch").into(),
        );
    }
    Ok(())
}
pub(super) fn current_ecu(
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
        return Err(integration_failure(autosar_config_core::product_message!(
            "backend.delivery.saved_source_mismatch"
        )));
    }
    Ok(plan)
}

#[tauri::command]
pub(super) async fn build_ecu(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
    build_directory: String,
) -> Result<Reply<BuildReport>, Vec<PlanDiagnostic>> {
    integration_background(state, move |state| {
        let operation = state
            .begin(
                &fingerprint,
                autosar_config_core::product_message!("backend.operation.build_ecu"),
                OperationKind::Native,
            )
            .map_err(integration_failure)?;
        let source = Path::new(&output_directory);
        let plan = current_ecu(&operation, &state.runtime, source)?;
        let staged =
            StagedBuild::new(source, Path::new(&build_directory)).map_err(integration_failure)?;
        let result = build_ecu_project(
            &plan,
            source,
            staged.directory(),
            operation.snapshot.tools.as_deref().ok_or_else(|| {
                integration_failure(autosar_config_core::product_message!(
                    "backend.workbench.execution_tools_missing"
                ))
            })?,
            operation.native_owner().map_err(integration_failure)?,
        )
        .map_err(integration_failure)?;
        current_ecu(&operation, &state.runtime, source)?;
        operation
            .commit(|_| {
                operation.snapshot.workspace()?.verify_saved_sources()?;
                let binary = Path::new(&result.binary_path).file_name().ok_or(
                    autosar_config_core::product_message!("backend.delivery.binary_name_missing"),
                )?;
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
pub(super) async fn verify_ecu(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
) -> Result<Reply<RunReport>, Vec<PlanDiagnostic>> {
    integration_background(state, move |state| {
        let operation = state
            .begin(
                &fingerprint,
                autosar_config_core::product_message!("backend.operation.verify_ecu"),
                OperationKind::Native,
            )
            .map_err(integration_failure)?;
        let source = Path::new(&output_directory);
        let plan = current_ecu(&operation, &state.runtime, source)?;
        let value = verify_ecu_project(
            &plan,
            source,
            operation.snapshot.tools.as_deref().ok_or_else(|| {
                integration_failure(autosar_config_core::product_message!(
                    "backend.workbench.execution_tools_missing"
                ))
            })?,
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

pub(super) fn preview_host(
    state: &Arc<AppState>,
    fingerprint: &str,
    output: &Path,
    handoff: bool,
) -> Result<Reply<GenerationPreview>, autosar_config_core::LocalizedText> {
    let operation = state.begin(
        fingerprint,
        autosar_config_core::product_message!("backend.operation.preview_host"),
        OperationKind::Read,
    )?;
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
pub(super) fn generate_host(
    state: &Arc<AppState>,
    fingerprint: &str,
    output: &Path,
    revision: &str,
    handoff: bool,
) -> Result<Reply<GenerationReport>, autosar_config_core::LocalizedText> {
    let operation = state.begin(
        fingerprint,
        autosar_config_core::product_message!("backend.operation.generate_host"),
        OperationKind::Read,
    )?;
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
pub(super) async fn preview_generate_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
) -> Result<Reply<GenerationPreview>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        preview_host(&state, &fingerprint, Path::new(&output_directory), false)
    })
    .await
}
#[tauri::command]
pub(super) async fn preview_handoff_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
) -> Result<Reply<GenerationPreview>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        preview_host(&state, &fingerprint, Path::new(&output_directory), true)
    })
    .await
}
#[tauri::command]
pub(super) async fn generate_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
    revision: String,
) -> Result<Reply<GenerationReport>, autosar_config_core::LocalizedText> {
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
pub(super) async fn generate_handoff_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
    revision: String,
) -> Result<Reply<GenerationReport>, autosar_config_core::LocalizedText> {
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
pub(super) async fn build_project(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
    build_directory: String,
) -> Result<Reply<BuildReport>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.build_host"),
            OperationKind::Native,
        )?;
        let source = Path::new(&output_directory);
        check_target(source, operation.snapshot.target)?;
        operation.snapshot.workspace()?.verify_saved_sources()?;
        let staged = StagedBuild::new(source, Path::new(&build_directory))?;
        let result =
            autosar_config_core::generator::build(
                source,
                staged.directory(),
                operation.snapshot.tools.as_deref().ok_or(
                    autosar_config_core::product_message!(
                        "backend.workbench.execution_tools_missing"
                    ),
                )?,
                operation.native_owner()?,
            )?;
        operation.commit(|_| {
            operation.snapshot.workspace()?.verify_saved_sources()?;
            let binary = Path::new(&result.binary_path).file_name().ok_or(
                autosar_config_core::product_message!("backend.delivery.binary_name_missing"),
            )?;
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
pub(super) async fn run_virtual(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    first_output_directory: String,
    second_output_directory: String,
    first_binary_path: String,
    second_binary_path: String,
) -> Result<Reply<RunReport>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.run_virtual"),
            OperationKind::Native,
        )?;
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
pub(super) async fn run_diagnostic(
    state: State<'_, Arc<AppState>>,
    fingerprint: String,
    output_directory: String,
    binary_path: String,
) -> Result<Reply<RunReport>, autosar_config_core::LocalizedText> {
    background(state, move |state| {
        let operation = state.begin(
            &fingerprint,
            autosar_config_core::product_message!("backend.operation.run_diagnostic"),
            OperationKind::Native,
        )?;
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
