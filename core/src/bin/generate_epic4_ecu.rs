use autosar_config_core::Workspace;
use autosar_config_core::integration::{PlanDependencies, RuntimeCatalog};
use std::path::PathBuf;

fn execute() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut repository = None;
    let mut output = None;
    let mut revision = None;
    let mut inputs = Vec::new();
    let mut write = false;
    while let Some(argument) = args.next() {
        if argument == "--write" {
            write = true;
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| format!("Missing value after {argument}"))?;
        match argument.as_str() {
            "--repository" => repository = Some(PathBuf::from(value)),
            "--output" => output = Some(PathBuf::from(value)),
            "--input" => inputs.push(PathBuf::from(value)),
            "--revision" => revision = Some(value),
            _ => return Err(format!("Unknown argument: {argument}")),
        }
    }
    let repository = repository.ok_or("Supply --repository <matching-source-checkout>.")?;
    let output = output.ok_or("Supply --output <generated-source-directory>.")?;
    if inputs.is_empty() {
        return Err("Supply --input <source.arxml> for each original source.".into());
    }
    if write && revision.is_none() {
        return Err(
            "Preview first; --write requires the exact --revision from that preview.".into(),
        );
    }
    let diagnostics =
        |issues| serde_json::to_string_pretty(&issues).unwrap_or_else(|error| error.to_string());
    let dependencies = PlanDependencies::from_repository(&repository);
    let runtime = RuntimeCatalog::from_repository(&repository).map_err(diagnostics)?;
    let workspace = Workspace::open(inputs, dependencies.xsd_archive)?;
    let plan = workspace
        .integration_plan(&runtime, dependencies.mod_archive)
        .map_err(diagnostics)?;
    let project = plan.ecu_integration_files().map_err(diagnostics)?;
    if write {
        let report = project.generate_previewed(&output, revision.as_deref().unwrap())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
        );
    } else {
        let preview = project.preview(&output)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&preview).map_err(|error| error.to_string())?
        );
    }
    Ok(())
}
fn main() {
    if let Err(error) = execute() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
