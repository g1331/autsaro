//! The specifically retained generated interfaces and address observation.
use super::*;
use std::path::Path;
use std::time::Duration;

#[test]
fn exact_generated_retention_contracts() {
    let scratch = Scratch::new();
    let samples = match std::env::var_os("AUTOSAR_C_ANALYSIS_SAMPLES") {
        Some(path) => std::path::absolute(std::path::PathBuf::from(path)).unwrap(),
        None => {
            let output = scratch.0.join("samples");
            analysis_samples::generate(&output, &scratch);
            output
        }
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let output = tooling::run_public_command(
        tooling::python_command()
            .current_dir(root)
            .env("AUTOSAR_C_ANALYSIS_SAMPLES", samples)
            .env("PYTHONPATH", root.join("tools/python/src"))
            .args([
                "-B",
                "-m",
                "unittest",
                "discover",
                "-s",
                "tests/python/native",
                "-p",
                "test_generated_retention.py",
            ]),
        &scratch.0,
        "retained-interfaces",
        Duration::from_secs(180),
    );
    assert!(output.status.success(), "{output:?}");
}

#[test]
fn retained_arti_address_rejects_high_bit_truncation() {
    let scratch = Scratch::new();
    let (project, _) = project(
        &single_component_sources(),
        &scratch.0,
        tooling::native_target(),
    );
    let consumer = scratch.0.join("consumer");
    std::fs::create_dir(&consumer).unwrap();
    for name in ["ecu_arti_control.c", "ecu_control.c"] {
        let source = std::fs::read_to_string(fixture(&format!("os/{name}"))).unwrap();
        let source = if name == "ecu_arti_control.c" {
            format!("#define ARTI_REQUIRE_FULL_ADDRESS\n{source}")
        } else {
            source
        };
        std::fs::write(consumer.join(name), source).unwrap();
    }
    super::os::arti_expression_probe(&project, &consumer.join("Arti_ExpressionProbe.h"));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let output = tooling::run_public_command(
        tooling::python_command()
            .current_dir(root)
            .env("PYTHONPATH", format!("{}{}{}", root.join("tools/python/src").display(), if cfg!(windows) { ";" } else { ":" }, root.join("tests/python/native").display()))
            .args(["-B", "-c", "from pathlib import Path; import sys; from test_generated_retention import check_arti_address; check_arti_address(*(Path(p) for p in sys.argv[1:]))"])
            .arg(&project).arg(&scratch.0).arg(consumer.join("ecu_arti_control.c")),
        &scratch.0,
        "retained-address",
        Duration::from_secs(180),
    );
    assert!(output.status.success(), "{output:?}");
}
