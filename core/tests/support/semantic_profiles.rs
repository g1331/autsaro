use autosar_config_core::integration::{PlanDependencies, RuntimeCatalog, build_plan};
use autosar_config_core::{DiagnosticSettings, Direction, Workspace, generator, schema};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

fn run(command: &mut Command, directory: &Path, name: &str, seconds: u64) {
    let output =
        super::tooling::run_public_command(command, directory, name, Duration::from_secs(seconds));
    assert!(
        output.status.success(),
        "{name}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
}

pub(super) fn verify() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let scratch = super::Scratch::new();
    let target = super::tooling::native_target();
    let settings = super::tooling::execution_settings();
    let mut workspace = Workspace::create(
        &scratch.0.join("legacy-input"),
        "SemanticHost",
        schema::schema_archive(root),
    )
    .unwrap();
    let frame = workspace
        .add_frame(
            "DiagnosticData".into(),
            0x500,
            4,
            Direction::Tx,
            Some(1000),
            None,
        )
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = workspace
        .add_signal(frame, "DiagnosticValue".into(), 0, 32, 0x11223344)
        .unwrap()
        .signals[0]
        .path
        .clone();
    workspace
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    workspace.save().unwrap();
    let legacy = scratch.0.join("synchronous-source");
    generator::generate_handoff(&mut workspace, &legacy, target).unwrap();
    let description: Value =
        serde_json::from_slice(&fs::read(legacy.join("target.json")).unwrap()).unwrap();
    let host_binary = super::tooling::native_binary(&scratch.0, "semantic_host");
    let mut compile = Command::new(&settings.compiler);
    for flag in description["compilerFlags"].as_array().unwrap() {
        compile.arg(flag.as_str().unwrap());
    }
    for include in description["includePaths"].as_array().unwrap() {
        compile
            .arg("-I")
            .arg(legacy.join(include.as_str().unwrap()));
    }
    for source in description["sources"].as_array().unwrap() {
        let source = source.as_str().unwrap();
        if source != "src/ecu_host_main.c" {
            compile.arg(legacy.join(source));
        }
    }
    compile.arg(root.join("core/tests/fixtures/semantic_host.c"));
    for library in description["linkLibraries"].as_array().unwrap() {
        compile.arg(format!("-l{}", library.as_str().unwrap()));
    }
    compile.arg("-o").arg(&host_binary);
    run(&mut compile, &scratch.0, "semantic-host-compile", 180);
    run(
        &mut Command::new(&host_binary),
        &scratch.0,
        "semantic-host-run",
        30,
    );

    let resources = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::embedded().unwrap();
    let plan = build_plan(&super::epic4_plan::inputs(), &resources, &runtime).unwrap();
    let ecu = scratch.0.join("queued-source");
    let project = plan.ecu_integration_files(target).unwrap();
    let preview = project.preview(&ecu).unwrap();
    project.generate_previewed(&ecu, &preview.revision).unwrap();
    let output = scratch.0.join("queued-build");
    let control = root.join("core/tests/fixtures/semantic_ecu.c");
    let mut compile = super::tooling::ecu_build_command(&ecu, &output, "probe", Some(&control));
    run(&mut compile, &scratch.0, "semantic-ecu-compile", 180);
    let ecu_binary = super::tooling::native_binary(&output, "ecu_probe");
    run(
        &mut Command::new(&ecu_binary),
        &scratch.0,
        "semantic-ecu-run",
        30,
    );
}
