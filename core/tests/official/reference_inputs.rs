//! Reference inputs and the official R24-11 showcase against the licensed archives.
use super::*;
use autosar_config_core::schema;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::Path;

#[test]
fn reference_input_inventory_and_schema_expectations() {
    verify_reference_inputs();
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
        let name = member.name().unwrap().into_owned();
        if !name.starts_with(prefix) || !name.ends_with(".arxml") {
            continue;
        }
        let path = temp.0.join(Path::new(&name).file_name().unwrap());
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

fn verify_reference_inputs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let fixture = root.join("core/tests/fixtures/echo-application");
    let manifest: Value =
        serde_json::from_slice(&fs::read(fixture.join("manifest.json")).unwrap()).unwrap();
    // Locate the archive like every official test; the manifest pins its identity.
    let schema_path = schema::schema_archive(root);
    assert_eq!(
        hex::encode(Sha256::digest(
            fs::read(&schema_path).expect("not_run: required local R24-11 XSD archive is missing")
        )),
        manifest["external_xsd"]["sha256"].as_str().unwrap(),
        "R24-11 XSD archive identity; missing dependency is a failure, never a skip"
    );
    let mut positive = Vec::new();
    for entry in manifest["files"].as_array().unwrap() {
        let relative = entry["path"].as_str().unwrap();
        let path = fixture.join(relative);
        let bytes = fs::read(&path).unwrap();
        assert_eq!(
            hex::encode(Sha256::digest(&bytes)),
            entry["sha256"].as_str().unwrap(),
            "raw fixture bytes: {relative}"
        );
        if relative.starts_with("positive/") && relative.ends_with(".arxml") {
            positive.push((path, String::from_utf8(bytes).unwrap()));
        }
    }
    assert_eq!(positive.len(), 7, "all declared positive file boundaries");
    let declared: BTreeSet<_> = positive
        .iter()
        .map(|(path, _)| path.file_name().unwrap().to_owned())
        .collect();
    let discovered: BTreeSet<_> = fs::read_dir(fixture.join("positive"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "arxml"))
        .map(|path| path.file_name().unwrap().to_owned())
        .collect();
    assert_eq!(
        declared, discovered,
        "every positive input must enter XSD validation"
    );
    let borrowed: Vec<_> = positive
        .iter()
        .map(|(p, t)| (p.as_path(), t.as_str()))
        .collect();
    let issues = schema::validate_files(&schema_path, &borrowed).unwrap();
    assert!(issues.is_empty(), "positive R24-11 XSD: {issues:?}");
    let cases: Value =
        serde_json::from_slice(&fs::read(fixture.join("negative/cases.json")).unwrap()).unwrap();
    assert_eq!(cases["cases"].as_array().unwrap().len(), 18);
    let mut rows = Vec::new();
    for case in cases["cases"].as_array().unwrap() {
        let mut inputs = positive.clone();
        for relative in case["overrides"].as_array().unwrap() {
            let override_path = fixture.join(relative.as_str().unwrap());
            let slot = inputs
                .iter_mut()
                .find(|(path, _)| path.file_name() == override_path.file_name())
                .expect("negative copy overrides a declared positive input");
            *slot = (
                override_path.clone(),
                fs::read_to_string(override_path).unwrap(),
            );
        }
        let borrowed: Vec<_> = inputs
            .iter()
            .map(|(p, t)| (p.as_path(), t.as_str()))
            .collect();
        let issues = schema::validate_files(&schema_path, &borrowed).unwrap();
        let expected = case["expected_xsd"].as_str().unwrap();
        assert_eq!(issues.is_empty(), expected == "pass", "{case}: {issues:?}");
        if expected == "fail" {
            assert!(issues.iter().all(|issue| issue.code == "R24_XSD"));
        }
        rows.push(json!({"id": case["id"], "xsd": expected, "diagnostics": issues}));
    }
    println!(
        "REFERENCE_INPUT_EVIDENCE {}",
        json!({"profile": manifest["profile"], "status": "pass", "positive_xsd_files": 7,
            "negative": rows, "input_manifest": manifest,
            "scope": "original fixture baseline; product plan/SC1/integration not established"})
    );
}
