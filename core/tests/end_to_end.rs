// Shared support modules serve different test layers; only registered tests run.
#![allow(dead_code)]

use std::fs;
use std::io::Read;
use std::path::Path;
use workspace::Scratch;

#[path = "support/workspace.rs"]
mod workspace;

#[path = "support/epic4_reference.rs"]
mod epic4_reference;

#[path = "support/tooling.rs"]
mod tooling;

#[test]
fn epic4_reference_input_baseline() {
    epic4_reference::verify();
}

#[test]
fn epic4_independent_oracle_contracts() {
    tooling::verify_protocol_oracles();
}

#[test]
fn official_r24_sample_imports_as_one_split_package_without_rewriting_sources() {
    let temp = Scratch::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let zip_path = autosar_config_core::schema::reference_archive(
        root,
        "AUTOSAR_SAMPLE_ARCHIVE",
        autosar_config_core::schema::SAMPLE_ZIP,
    );
    let mut zip = zip::ZipArchive::new(fs::File::open(zip_path).unwrap()).unwrap();
    let prefix =
        "AUTOSAR_CP_EXP_ModelingShowCases/30_MeasurementCalibration/10_Introductory/model/";
    let mut paths = Vec::new();
    let mut originals = Vec::new();
    for index in 0..zip.len() {
        let mut member = zip.by_index(index).unwrap();
        if !member.name().starts_with(prefix) || !member.name().ends_with(".arxml") {
            continue;
        }
        let path = temp.0.join(Path::new(member.name()).file_name().unwrap());
        let mut content = Vec::new();
        member.read_to_end(&mut content).unwrap();
        fs::write(&path, &content).unwrap();
        originals.push((path.clone(), content));
        paths.push(path);
    }
    assert_eq!(
        paths.len(),
        15,
        "the official R24 example must have all model files"
    );
    let mut project = autosar_config_core::Workspace::open(paths).unwrap();
    let view = project.validate().unwrap();
    // Built-in rules do not cover every official showcase template; only cross-file
    // path resolution proves the split packages merged.
    assert!(
        !view
            .issues
            .iter()
            .any(|issue| issue.code == "UNRESOLVED_REF"),
        "split AR-PACKAGE paths may merge across files: {:?}",
        view.issues
    );
    assert!(!project.view().dirty);
    // Uncovered showcase templates block saving; either way sources must stay byte-identical.
    if let Err(error) = project.save() {
        assert!(
            format!("{error:?}").contains("backend.arxml.persistence.blocking_diagnostic"),
            "{error:?}"
        );
    }
    for (path, content) in originals {
        assert_eq!(content, fs::read(path).unwrap());
    }
    assert!(
        project.integration_plan().is_err(),
        "an unconfigured ECU must not produce a misleading mainline plan"
    );
}
