use autosar_config_core::schema;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub fn verify() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let fixture = root.join("core/tests/fixtures/epic4");
    let manifest: Value =
        serde_json::from_slice(&fs::read(fixture.join("manifest.json")).unwrap()).unwrap();
    let schema_path = root.join(manifest["external_xsd"]["path"].as_str().unwrap());
    assert_eq!(
        format!(
            "{:x}",
            Sha256::digest(
                fs::read(&schema_path)
                    .expect("not_run: required local R24-11 XSD archive is missing")
            )
        ),
        manifest["external_xsd"]["sha256"].as_str().unwrap(),
        "R24-11 XSD archive identity; missing dependency is a failure, never a skip"
    );
    let mut positive = Vec::new();
    for entry in manifest["files"].as_array().unwrap() {
        let relative = entry["path"].as_str().unwrap();
        let path = fixture.join(relative);
        let bytes = fs::read(&path).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
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
    let scratch = super::Scratch::new();
    let result = super::tooling::run_public_command(
        super::tooling::python_command()
            .args(["-m", "autosar_tooling", "input-oracles"])
            .current_dir(root),
        &scratch.0,
        "input-oracles",
        std::time::Duration::from_secs(30),
    );
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let semantic: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(semantic["positive"], "pass");
    println!(
        "EPIC4_INPUT_EVIDENCE {}",
        json!({"profile": manifest["profile"], "status": "pass", "positive_xsd_files": 7,
            "negative": rows, "semantic": semantic, "input_manifest": manifest,
            "scope": "original fixture baseline; product plan/SC1/integration not established"})
    );
}
