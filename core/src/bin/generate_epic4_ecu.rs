use autosar_config_core::LocalizedText;
use autosar_config_core::Workspace;
use autosar_config_core::integration::{PlanDependencies, RuntimeCatalog};
use autosar_config_core::prepare_ecu_project;
use autosar_config_core::target::{BuildTarget, ExecutionSettings};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn execute() -> Result<(), LocalizedText> {
    let mut args = std::env::args().skip(1);
    let mut target = None;
    let mut output = None;
    let mut revision = None;
    let mut inputs = Vec::new();
    let mut xsd_archive = None;
    let mut mod_archive = None;
    let mut write = false;
    let mut handoff = false;
    let mut preflight_requested = false;
    let mut seen = BTreeSet::new();
    while let Some(argument) = args.next() {
        if argument != "--input" && !seen.insert(argument.clone()) {
            return Err(
                autosar_config_core::product_message!("backend.cli.repeated_argument", "argument" => argument),
            );
        }
        match argument.as_str() {
            "--write" => {
                write = true;
                continue;
            }
            "--handoff" => {
                handoff = true;
                continue;
            }
            "--preflight" => {
                preflight_requested = true;
                continue;
            }
            _ => {}
        }
        let value = args
            .next()
            .ok_or_else(|| autosar_config_core::product_message!("backend.cli.argument_value_missing", "argument" => &argument))?;
        match argument.as_str() {
            "--target" => {
                target = Some(
                    serde_json::from_value::<BuildTarget>(serde_json::Value::String(value))
                        .map_err(|error| error.to_string())?,
                )
            }
            "--output" => output = Some(PathBuf::from(value)),
            "--input" => inputs.push(PathBuf::from(value)),
            "--xsd-archive" => xsd_archive = Some(PathBuf::from(value)),
            "--mod-archive" => mod_archive = Some(PathBuf::from(value)),
            "--revision" => revision = Some(value),
            _ => {
                return Err(
                    autosar_config_core::product_message!("backend.cli.unknown_argument", "argument" => argument),
                );
            }
        }
    }
    let target = target
        .ok_or_else(|| autosar_config_core::product_message!("backend.cli.target_required"))?;
    let output = output
        .ok_or_else(|| autosar_config_core::product_message!("backend.cli.output_required"))?;
    if inputs.is_empty() {
        return Err(autosar_config_core::product_message!(
            "backend.cli.inputs_required"
        ));
    }
    if write != revision.is_some() {
        return Err(autosar_config_core::product_message!(
            "backend.cli.preview_revision_required"
        ));
    }
    let xsd_archive = xsd_archive
        .or_else(|| std::env::var_os("AUTOSAR_XSD_ARCHIVE").map(PathBuf::from))
        .ok_or_else(|| autosar_config_core::product_message!("backend.cli.xsd_required"))?;
    let mod_archive = mod_archive
        .or_else(|| std::env::var_os("AUTOSAR_MOD_ARCHIVE").map(PathBuf::from))
        .ok_or_else(|| autosar_config_core::product_message!("backend.cli.mod_required"))?;
    let dependencies = PlanDependencies::explicit(xsd_archive, mod_archive)?;
    let diagnostics = |issues: Vec<autosar_config_core::integration::PlanDiagnostic>| {
        LocalizedText::messages(issues.into_iter().map(|issue| issue.message))
    };
    let runtime = RuntimeCatalog::embedded().map_err(diagnostics)?;
    let workspace = Workspace::open_legacy(inputs, dependencies.xsd_archive)?;
    let plan = workspace
        .integration_plan_legacy(&runtime, dependencies.mod_archive)
        .map_err(diagnostics)?;
    let project = prepare_ecu_project(&plan, target, handoff).map_err(diagnostics)?;
    let preflight = if preflight_requested && target.is_native() {
        let settings = ExecutionSettings::from_environment()?;
        let owner = autosar_config_core::execution::ProcessOwner::with_python(&settings.python)?;
        project.native_preflight(&settings, &owner)
    } else {
        project.preflight().clone()
    };
    if preflight.status == autosar_config_core::prepared::PreflightStatus::Failed {
        let report = serde_json::json!({ "preflight": preflight });
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
        );
        return Err(autosar_config_core::product_message!(
            "backend.cli.preflight_failed"
        ));
    }
    let mut report = if write {
        serde_json::to_value(project.generate_previewed(&output, revision.as_deref().unwrap())?)
            .map_err(|error| error.to_string())?
    } else {
        serde_json::to_value(project.preview(&output)?).map_err(|error| error.to_string())?
    };
    report
        .as_object_mut()
        .ok_or_else(|| autosar_config_core::product_message!("backend.cli.invalid_report"))?
        .insert(
            "preflight".into(),
            serde_json::to_value(preflight).map_err(|error| error.to_string())?,
        );
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn main() {
    if let Err(error) = execute() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
