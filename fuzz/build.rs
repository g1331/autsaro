//! Compiles the CanTp and Dcm libFuzzer drivers (`c/`) with the delivered runtime sources.
//! Only `cargo fuzz` builds them (`--cfg fuzzing`); `cargo test` and Clippy of this crate need
//! no C toolchain. The C code is instrumented like the Rust code: libFuzzer coverage plus
//! AddressSanitizer and UndefinedBehaviorSanitizer, whose handlers come with the Rust ASan
//! runtime that `cargo fuzz` links.

use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=c");
    println!("cargo:rerun-if-changed=../runtime/bsw");
    println!("cargo:rerun-if-changed=../runtime/host");
    println!("cargo:rerun-if-changed=../core/tests/fixtures/bsw-contracts");
    println!("cargo:rerun-if-env-changed=CC");
    if std::env::var_os("CARGO_CFG_FUZZING").is_none() {
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let generated = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("include");
    generated_headers(&generated);
    let sanitize = std::env::var("CARGO_CFG_SANITIZE").unwrap_or_default();
    let build = |name: &str, include: &[PathBuf], files: &[&str]| {
        let mut build = cc::Build::new();
        if std::env::var_os("CC").is_none() {
            build.compiler("clang");
        }
        build
            .std("c99")
            .warnings_into_errors(true)
            .flag("-pedantic")
            .flag("-pthread")
            .flag("-fno-omit-frame-pointer")
            .flag("-fsanitize-coverage=inline-8bit-counters,pc-table,trace-cmp");
        if sanitize.split(',').any(|kind| kind == "address") {
            build
                .flag("-fsanitize=address,undefined")
                .flag("-fno-sanitize-recover=all");
        }
        for directory in include {
            build.include(directory);
        }
        for directory in [
            "runtime/bsw/include",
            "runtime/host/include",
            "runtime/bsw/src",
            "runtime/host/src",
            "core/tests/fixtures/bsw-contracts",
        ] {
            build.include(root.join(directory));
        }
        for file in files {
            build.file(root.join(file));
        }
        build.compile(name);
    };
    build(
        "cantp_receive",
        &[],
        &["fuzz/c/cantp_receive.c", "runtime/bsw/src/CanTp.c"],
    );
    build(
        "dcm_request",
        std::slice::from_ref(&generated),
        &[
            "fuzz/c/dcm_request.c",
            "runtime/bsw/src/Dcm.c",
            "runtime/host/src/SchM_Diagnostic_Host.c",
        ],
    );
    build(
        "host_areas",
        &[],
        &[
            "runtime/host/src/SchM_CanIf_Host.c",
            "runtime/host/src/Can_HostLock.c",
        ],
    );
}

/// The RTE type headers a delivered mainline ECU generates for Dcm, from the built-in template's
/// actual preparation, as the native Dcm consumers use them.
fn generated_headers(directory: &Path) {
    use autosar_config_core::Workspace;
    let live = directory.with_file_name("mainline");
    let _ = std::fs::remove_dir_all(&live);
    let creation = Workspace::preview_project_creation(&live, "Headers", "standard-ecu").unwrap();
    let mut workspace = Workspace::create_project_previewed(&creation).unwrap();
    let initialization = workspace.preview_application_initialization().unwrap();
    workspace
        .initialize_application_previewed(&initialization)
        .unwrap();
    let plan = workspace.saved_integration_plan().unwrap();
    let prepared = autosar_config_core::prepared::prepare_ecu_project_for_workspace(
        &workspace,
        &plan,
        autosar_config_core::target::BuildTarget::LinuxX64ControlledV1,
        false,
    )
    .unwrap();
    std::fs::create_dir_all(directory).unwrap();
    for path in [
        "generated/include/Rte_Dcm_Type.h",
        "generated/include/Rte_Type.h",
    ] {
        let file = prepared
            .files()
            .iter()
            .find(|file| file.path == path)
            .unwrap_or_else(|| panic!("{path} is not generated"));
        std::fs::write(
            directory.join(Path::new(path).file_name().unwrap()),
            &file.bytes,
        )
        .unwrap();
    }
}
