//! Links, Windows reparse points and hard links at the workbench's source and
//! delivery boundaries. These run on every native platform, so the Windows
//! junction and symbolic link branches are exercised on Windows itself.
use super::*;
use autosar_config_core::LocalizedText;
use autosar_config_core::target::BuildTarget;
use std::fs;
use std::path::Path;

/// A directory symbolic link on POSIX, an NTFS junction on Windows.
fn link_directory(link: &Path, target: &Path) {
    #[cfg(windows)]
    {
        let linked = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .unwrap();
        assert!(
            linked.status.success(),
            "{}",
            String::from_utf8_lossy(&linked.stderr)
        );
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
}

fn link_file(link: &Path, target: &Path) {
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target, link).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
}

fn unlink(link: &Path) {
    // A junction is removed as a directory, a POSIX directory link as a file.
    if fs::remove_dir(link).is_err() {
        fs::remove_file(link).unwrap();
    }
}

fn refused(error: LocalizedText, key: &str, path: impl std::fmt::Display) {
    assert_eq!(
        serde_json::to_value(&error).unwrap(),
        serde_json::json!({"key": key, "params": {"path": path.to_string()}})
    );
}

#[test]
fn project_and_source_links_are_refused_at_the_link() {
    use autosar_config_core::Workspace;
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    live_pipeline_workspace(&root, &inputs());
    let alias = scratch.0.join("alias");
    link_directory(&alias, &root);
    refused(
        Workspace::open_project_manifest(
            &alias.join("workbench-project.json"),
            &scratch.0.join("cache"),
        )
        .err()
        .unwrap(),
        "backend.arxml.project.link_not_accepted",
        alias.display(),
    );
    refused(
        Workspace::open(vec![alias.join("ingress.arxml")])
            .err()
            .unwrap(),
        "backend.arxml.source_link_rejected",
        alias.display(),
    );
    // A single ARXML member replaced by a link to a file outside the project.
    let member = root.join("ingress.arxml");
    let external = scratch.0.join("external.arxml");
    fs::rename(&member, &external).unwrap();
    link_file(&member, &external);
    refused(
        Workspace::open_project_manifest(
            &root.join("workbench-project.json"),
            &scratch.0.join("cache"),
        )
        .err()
        .unwrap(),
        "backend.arxml.project.link_not_accepted",
        member.display(),
    );
    let source = inputs()
        .into_iter()
        .find(|source| source.logical_path() == "ingress.arxml")
        .unwrap();
    assert_eq!(fs::read(&external).unwrap(), source.bytes());
}

#[test]
fn application_member_link_refuses_initialization_reopen_and_generation() {
    use autosar_config_core::{Workspace, prepared::prepare_ecu_project_for_workspace};
    let scratch = Scratch::new();
    let root = scratch.0.join("live");
    let mut workspace = live_pipeline_workspace(&root, &inputs());
    let preview = workspace.preview_application_initialization().unwrap();
    let source = root.join(&preview.files[1].path);
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    let external = scratch.0.join("external.c");
    let bytes = b"/* external user source */\n";
    fs::write(&external, bytes).unwrap();
    link_file(&source, &external);
    refused(
        workspace
            .initialize_application_previewed(&preview)
            .err()
            .unwrap(),
        "backend.arxml.project.link_not_accepted",
        source.display(),
    );
    assert_eq!(fs::read(&external).unwrap(), bytes);
    assert!(
        preview
            .files
            .iter()
            .filter(|file| root.join(&file.path) != source)
            .all(|file| !root.join(&file.path).exists())
    );
    fs::remove_file(&source).unwrap();
    workspace
        .initialize_application_previewed(&preview)
        .unwrap();
    let plan = workspace.saved_integration_plan().unwrap();
    let prepared =
        prepare_ecu_project_for_workspace(&workspace, &plan, native_delivery_target(), true)
            .unwrap();
    fs::remove_file(&source).unwrap();
    link_file(&source, &external);
    refused(
        Workspace::open_project_manifest(&root.join("workbench-project.json"), &root.join("cache"))
            .err()
            .unwrap(),
        "backend.arxml.project.link_not_accepted",
        source.display(),
    );
    let output = scratch.0.join("sealed");
    refused(
        prepared.generate(&output).err().unwrap(),
        "backend.delivery.linked_path_refused",
        source.display(),
    );
    assert!(!output.exists());
    assert_eq!(fs::read(&external).unwrap(), bytes);
}

#[test]
fn generation_refuses_linked_outputs_and_entries_without_touching_their_targets() {
    let scratch = Scratch::new();
    let target = BuildTarget::LinuxX64ControlledV1;
    let sources = counter_variant(false, 65535);
    let prepare = || prepared(&sources, &scratch.0, target);
    let owner = scratch.0.join("OwnerData");
    fs::create_dir(&owner).unwrap();
    fs::write(owner.join("sentinel.txt"), b"owner content").unwrap();
    let untouched = || {
        let names: Vec<_> = fs::read_dir(&owner)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["sentinel.txt"]);
        assert_eq!(
            fs::read(owner.join("sentinel.txt")).unwrap(),
            b"owner content"
        );
    };
    // `OwnerData/.` names the owner's directory itself.
    assert_eq!(
        serde_json::to_value(prepare().generate(&owner.join(".")).unwrap_err()).unwrap()["key"],
        "backend.generation.output_inventory_missing"
    );
    untouched();

    // The output itself is a link: refused on preview, generation and, when the
    // link appears after staging, on confirmation.
    let output = scratch.0.join("Generated");
    link_directory(&output, &owner);
    for error in [
        prepare().preview(&output).unwrap_err(),
        prepare().generate(&output).unwrap_err(),
    ] {
        refused(
            error,
            "backend.delivery.linked_path_refused",
            output.display(),
        );
    }
    untouched();
    let raced = scratch.0.join("raced");
    let preview = prepare().preview(&raced).unwrap();
    let staged = prepare()
        .stage_previewed(&raced, &preview.revision)
        .unwrap();
    link_directory(&raced, &owner);
    refused(
        staged.commit().unwrap_err(),
        "backend.delivery.linked_path_refused",
        raced.display(),
    );
    untouched();

    // A directory or a file inside an existing output replaced by a link.
    let output = scratch.0.join("package");
    prepare().generate(&output).unwrap();
    let original = packages::payload(&output);
    let copy = scratch.0.join("generated-copy");
    fs::rename(output.join("generated"), &copy).unwrap();
    let before = packages::package_tree(&copy);
    link_directory(&output.join("generated"), &copy);
    refused(
        prepare().preview(&output).unwrap_err(),
        "backend.generation.output_link_refused",
        "generated",
    );
    refused(
        prepare().generate(&output).unwrap_err(),
        "backend.generation.output_link_refused",
        "generated",
    );
    assert_eq!(packages::package_tree(&copy), before);
    unlink(&output.join("generated"));
    fs::rename(&copy, output.join("generated")).unwrap();
    let file = output.join("bsw/src/Com.c");
    let moved = scratch.0.join("Com.c");
    fs::rename(&file, &moved).unwrap();
    link_file(&file, &moved);
    refused(
        prepare().generate(&output).unwrap_err(),
        "backend.generation.output_link_refused",
        "bsw/src/Com.c",
    );
    assert_eq!(fs::read(&moved).unwrap(), original["bsw/src/Com.c"]);
    fs::remove_file(&file).unwrap();
    fs::rename(&moved, &file).unwrap();
    assert_eq!(packages::payload(&output), original);
}

#[test]
fn contract_generation_refuses_a_linked_output_directory() {
    // Contract packages have no native guard; the output check itself refuses the link.
    let scratch = Scratch::new();
    let files = build(&inputs())
        .unwrap()
        .component_contract_files()
        .unwrap();
    let owner = scratch.0.join("owner");
    fs::create_dir(&owner).unwrap();
    let output = scratch.0.join("contracts");
    link_directory(&output, &owner);
    refused(
        files.preview(&output).unwrap_err(),
        "backend.generation.output_not_plain_directory",
        output.display(),
    );
    assert_eq!(fs::read_dir(&owner).unwrap().count(), 0);
}

#[test]
fn regeneration_does_not_write_through_a_hard_linked_output_file() {
    // Hard links are not refused: the previous output is moved aside as a whole,
    // so an external name for one of its files keeps the old bytes.
    let scratch = Scratch::new();
    let target = BuildTarget::LinuxX64ControlledV1;
    let output = scratch.0.join("package");
    prepared(&counter_variant(false, 65535), &scratch.0, target)
        .generate(&output)
        .unwrap();
    // The integrity record changes with every configuration change.
    let configuration = output.join("meta/files.sha256");
    let external = scratch.0.join("files.sha256");
    fs::hard_link(&configuration, &external).unwrap();
    let old = fs::read(&external).unwrap();
    prepared(&counter_variant(false, 4095), &scratch.0, target)
        .generate(&output)
        .unwrap();
    assert_ne!(fs::read(&configuration).unwrap(), old);
    assert_eq!(fs::read(&external).unwrap(), old);
}
