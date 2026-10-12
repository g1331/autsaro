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
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi-component");
    let mut sources: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            autosar_config_core::integration::InputSource::new(
                entry.file_name().to_str().unwrap(),
                fs::read(entry.path()).unwrap(),
            )
            .unwrap()
        })
        .collect();
    sources.sort_by(|left, right| left.logical_path().cmp(right.logical_path()));
    let plan = autosar_config_core::integration::build_plan_native(
        &sources,
        &autosar_config_core::definitions::DefinitionCatalog::builtin().unwrap(),
    )
    .unwrap();
    let applications: Vec<_> = plan
        .application_slot_descriptors()
        .unwrap()
        .iter()
        .map(|slot| autosar_config_core::ApplicationSource {
            component_instance: slot
                .producer_slot
                .strip_prefix("singlecore-ecu-v1:")
                .unwrap()
                .into(),
            path: Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/multi-application")
                .join(slot.source_paths[0].rsplit('/').next().unwrap()),
        })
        .collect();
    let live = scratch.0.join("multi-author");
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
    let original = scratch.0.join("multi-author-package");
    prepare_ecu_project_for_workspace(&workspace, &plan, target, true)
        .unwrap()
        .generate(&original)
        .unwrap();
    let package = scratch.0.join("moved-multi-package");
    fs::rename(&original, &package).unwrap();
    fs::remove_dir_all(&live).unwrap();
    assert!(!original.exists() && !live.exists());
    let imported = autosar_config_core::generator::delivery::open_handoff(
        &package,
        &scratch.0.join("received-multi"),
        &autosar_config_core::definitions::DefinitionCatalog::builtin().unwrap(),
    )
    .unwrap();
    let plan = imported.workspace.saved_integration_plan().unwrap();
    let regenerated = output.join("multi-component");
    prepare_ecu_project_for_workspace(&imported.workspace, &plan, target, true)
        .unwrap()
        .generate(&regenerated)
        .unwrap();
    assert_eq!(packages::payload(&package), packages::payload(&regenerated));
    assert_eq!(fs::read_dir(output).unwrap().count(), 3);
}
