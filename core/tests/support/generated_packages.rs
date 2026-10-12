//! Prepared deliveries of the built-in templates.
use super::*;
use autosar_config_core::prepared::prepare_ecu_project_for_workspace;
use autosar_config_core::{PreparedProject, Workspace};
use std::path::Path;

pub(crate) fn prepare(workspace: &Workspace, handoff: bool) -> PreparedProject<'static> {
    let plan = workspace.saved_integration_plan().unwrap();
    prepare_ecu_project_for_workspace(workspace, &plan, tooling::native_target(), handoff).unwrap()
}

pub(crate) fn standard_with_application(root: &Path) -> Workspace {
    standard_named(root, "Application")
}

/// A standard ECU project with its create-only reference application.
pub(crate) fn standard_named(root: &Path, name: &str) -> Workspace {
    let creation = Workspace::preview_project_creation(root, name, "standard-ecu").unwrap();
    let mut workspace = Workspace::create_project_previewed(&creation).unwrap();
    let initialization = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&initialization)
        .unwrap();
    workspace
}
