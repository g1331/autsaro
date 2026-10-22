//! Generates the mainline projects that static analysis and CI check.
use autosar_config_core::Workspace;
use autosar_config_core::prepared::prepare_ecu_project_for_workspace;
use autosar_config_core::target::BuildTarget;
use std::fs;
use std::path::Path;

use super::packages;

/// Writes every analysis sample into the new directory `output`, one
/// subdirectory per sample, using `scratch` for the live projects.
pub(crate) fn generate(output: &Path, scratch: &super::workspace::Scratch) {
    fs::create_dir_all(output).unwrap();
    let target = if cfg!(windows) {
        BuildTarget::WindowsX64ControlledV1
    } else {
        BuildTarget::LinuxX64ControlledV1
    };
    for user in [false, true] {
        let case = if user {
            "user-application"
        } else {
            "standard-ecu"
        };
        let live = scratch.0.join(case);
        let preview =
            Workspace::preview_project_creation(&live, "Application", "standard-ecu").unwrap();
        let mut workspace = Workspace::create_project_previewed(&preview).unwrap();
        let initialization = workspace.preview_application_initialization().unwrap();
        workspace
            .initialize_application_previewed(&initialization)
            .unwrap();
        if user {
            let application = workspace.generation_snapshot().unwrap().applications[0].clone();
            let original = String::from_utf8(application.bytes).unwrap();
            let changed = original.replace(
                "Data[3] = (uint8)committed;",
                "Data[3] = (uint8)(committed + 1u);",
            );
            assert_ne!(original, changed);
            fs::write(application.disk_path, changed).unwrap();
            workspace = Workspace::open_project_manifest(
                &live.join("workbench-project.json"),
                &scratch.0.join("cache"),
            )
            .unwrap();
        }
        let plan = workspace.saved_integration_plan().unwrap();
        prepare_ecu_project_for_workspace(&workspace, &plan, target, true)
            .unwrap()
            .generate(&output.join(case))
            .unwrap();
    }
    let mut sources = super::mainline::inputs();
    // Exercise the supported non-default communication widths through the same
    // delivered programs and imported application sources as the normal pipeline.
    super::mainline::parameter(&mut sources, "Pdus", "PduIdTypeEnum", "UINT8");
    super::mainline::parameter(&mut sources, "Pdus", "PduLengthTypeEnum", "UINT32");
    let plan = super::mainline::build(&sources).unwrap();
    let applications = pipeline_applications(&plan);
    let live = scratch.0.join("author");
    fs::create_dir(&live).unwrap();
    for source in &sources {
        fs::write(live.join(source.logical_path()), source.bytes()).unwrap();
    }
    let manifest = autosar_config_core::arxml::ProjectManifest {
        format_version: 1,
        declared_release: "R24-11".into(),
        profile_hint: "singlecore-ecu-v1".into(),
        inputs: sources
            .iter()
            .map(|source| autosar_config_core::arxml::ProjectInput {
                path: source.logical_path().into(),
                role_hint: "standard".into(),
            })
            .collect(),
        application_inputs: Vec::new(),
        accepted_extension_definitions: Vec::new(),
    };
    let manifest_path = live.join("workbench-project.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let mut workspace =
        Workspace::open_project_manifest(&manifest_path, &scratch.0.join("cache")).unwrap();
    let preview = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&preview)
        .unwrap();
    for (slot, application) in plan
        .application_slot_descriptors()
        .unwrap()
        .iter()
        .zip(&applications)
    {
        fs::copy(&application.path, live.join(&slot.source_paths[0])).unwrap();
    }
    let workspace =
        Workspace::open_project_manifest(&manifest_path, &scratch.0.join("cache")).unwrap();
    let plan = workspace.saved_integration_plan().unwrap();
    let original = scratch.0.join("author-package");
    prepare_ecu_project_for_workspace(&workspace, &plan, target, true)
        .unwrap()
        .generate(&original)
        .unwrap();
    let package = scratch.0.join("moved-package");
    fs::rename(&original, &package).unwrap();
    fs::remove_dir_all(&live).unwrap();
    assert!(!original.exists() && !live.exists());
    let imported = autosar_config_core::generator::delivery::open_handoff(
        &package,
        &scratch.0.join("received"),
        &autosar_config_core::definitions::DefinitionCatalog::builtin().unwrap(),
    )
    .unwrap();
    let plan = imported.workspace.saved_integration_plan().unwrap();
    let regenerated = output.join("component-pipeline");
    prepare_ecu_project_for_workspace(&imported.workspace, &plan, target, true)
        .unwrap()
        .generate(&regenerated)
        .unwrap();
    assert_eq!(packages::payload(&package), packages::payload(&regenerated));
    // Without application DIDs or diagnostic routes the PduR, RTE and SchM
    // configuration takes its no-diagnostic branches, which no other sample reaches.
    let mut sources = super::mainline::sources_without_application_did(false);
    super::mainline::parameter(&mut sources, "Pdus", "PduIdTypeEnum", "UINT8");
    super::mainline::parameter(&mut sources, "Pdus", "PduLengthTypeEnum", "UINT8");
    let plan = super::mainline::build(&sources).unwrap();
    assert!(plan.description().diagnostic_transport.is_none());
    autosar_config_core::prepare_ecu_project_with_applications(
        &plan,
        target,
        &pipeline_applications(&plan),
    )
    .unwrap()
    .generate(&output.join("no-diagnostic"))
    .unwrap();
    assert_eq!(fs::read_dir(output).unwrap().count(), 4);
}

/// The fixture application sources for each component slot of `plan`.
fn pipeline_applications(
    plan: &autosar_config_core::integration::ValidatedIntegrationPlan,
) -> Vec<autosar_config_core::ApplicationSource> {
    plan.application_slot_descriptors()
        .unwrap()
        .iter()
        .map(|slot| autosar_config_core::ApplicationSource {
            component_instance: slot
                .producer_slot
                .strip_prefix("singlecore-ecu-v1:")
                .unwrap()
                .into(),
            path: Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/component-pipeline/applications")
                .join(slot.source_paths[0].rsplit('/').next().unwrap()),
        })
        .collect()
}
