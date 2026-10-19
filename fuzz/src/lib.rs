//! Bodies of the Rust fuzz targets, shared with the regression tests that replay the committed
//! corpus and reproducers through the same code.
//!
//! A project input is a sequence of files, each a relative name and its bytes, all separated by
//! NUL bytes (`name\0bytes\0name\0bytes`); ARXML and JSON contain no NUL. Names keep to portable
//! components so that every file lands inside the scratch directory.

use autosar_config_core::Workspace;
use autosar_config_core::definitions::DefinitionCatalog;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const MANIFEST: &str = "workbench-project.json";

/// The named files of a project input, skipping any name that could leave the directory.
pub fn files(data: &[u8]) -> Vec<(&str, &[u8])> {
    let mut parts = data.split(|byte| *byte == 0);
    let mut files = Vec::new();
    while let (Some(name), Some(bytes)) = (parts.next(), parts.next()) {
        if let Ok(name) = std::str::from_utf8(name)
            && !name.is_empty()
            && name.len() <= 64
            && name.split('/').all(|part| {
                !matches!(part, "" | "." | "..")
                    && part
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-'))
            })
        {
            files.push((name, bytes));
        }
    }
    files
}

/// An empty directory owned by this process; libFuzzer workers are separate processes.
fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir()
        .join(format!("autosar-fuzz-{}", std::process::id()))
        .join(name);
    match fs::remove_dir_all(&directory) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            panic!("cannot clear {}: {error}", directory.display())
        }
        _ => {}
    }
    fs::create_dir_all(&directory).expect("scratch directory");
    directory
}

fn write(directory: &Path, name: &str, bytes: &[u8]) -> Option<PathBuf> {
    let path = directory.join(name);
    fs::create_dir_all(path.parent()?).ok()?;
    fs::write(&path, bytes).ok()?;
    Some(path)
}

/// Opens the files as the desktop does, an ARXML set or a project manifest when one is
/// present, and runs what the workbench reads right after opening.
pub fn project_import(data: &[u8]) {
    let directory = scratch("project");
    let mut documents = Vec::new();
    let mut manifest = None;
    for (name, bytes) in files(data) {
        let Some(path) = write(&directory, name, bytes) else {
            continue;
        };
        if name == MANIFEST {
            manifest = Some(path);
        } else if name.ends_with(".arxml") {
            documents.push(path);
        }
    }
    let opened = match manifest {
        Some(path) => Workspace::open_project_manifest(&path, &directory.join(".definitions")),
        None if !documents.is_empty() => Workspace::open(documents),
        None => return,
    };
    let Ok(mut workspace) = opened else {
        return;
    };
    workspace.view();
    if let Ok(fingerprint) = workspace.input_fingerprint() {
        let _ = workspace.project_projection(&fingerprint);
    }
    workspace.inspect_integration();
    let _ = workspace.validate();
}

/// Accepts the bytes as the one definition file of an extension catalog whose inventory
/// carries their actual digest, so the definitions themselves are parsed.
pub fn definition_extension(data: &[u8]) {
    let directory = scratch("extension");
    let source = directory.join("source");
    let inventory = format!(
        r#"{{"formatVersion":1,"catalogId":"fuzz","release":"R24-11","version":"1.0.0","files":[{{"path":"definitions.arxml","sha256":"{}"}}]}}"#,
        hex::encode(Sha256::digest(data))
    );
    let (Some(_), Some(catalog)) = (
        write(&source, "definitions.arxml", data),
        write(&source, "catalog.json", inventory.as_bytes()),
    ) else {
        return;
    };
    let mut definitions = DefinitionCatalog::builtin().expect("built-in definitions");
    let _ = definitions.accept_extension(&catalog, &directory.join("cache"));
}

// The C drivers in `c/`, which build.rs compiles only for `cargo fuzz`.
#[cfg(fuzzing)]
unsafe extern "C" {
    fn CanTpFuzz_TestOneInput(data: *const u8, size: usize) -> i32;
    fn DcmFuzz_TestOneInput(data: *const u8, size: usize) -> i32;
}

/// Drives the delivered CanTp with one input (`c/cantp_receive.c`).
#[cfg(fuzzing)]
pub fn cantp_receive(data: &[u8]) {
    // SAFETY: the driver only reads `data.len()` bytes from `data`.
    unsafe { CanTpFuzz_TestOneInput(data.as_ptr(), data.len()) };
}

/// Drives the delivered Dcm with one input (`c/dcm_request.c`).
#[cfg(fuzzing)]
pub fn dcm_request(data: &[u8]) {
    // SAFETY: the driver only reads `data.len()` bytes from `data`.
    unsafe { DcmFuzz_TestOneInput(data.as_ptr(), data.len()) };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every committed corpus entry and reproducer of `target` runs to completion.
    fn replay(target: &str, run: fn(&[u8])) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut count = 0;
        for kind in ["corpus", "regressions"] {
            let Ok(entries) = fs::read_dir(root.join(kind).join(target)) else {
                continue;
            };
            for entry in entries {
                let path = entry.unwrap().path();
                eprintln!("{}", path.display());
                run(&fs::read(&path).unwrap());
                count += 1;
            }
        }
        assert!(count > 0, "{target} has no committed inputs");
    }

    #[test]
    fn project_import_inputs_replay() {
        replay("project_import", project_import);
    }

    #[test]
    fn definition_extension_inputs_replay() {
        replay("definition_extension", definition_extension);
    }

    #[test]
    fn project_names_stay_inside_the_directory() {
        let data = b"a.arxml\0A\0../b.arxml\0B\0/c\0C\0d/./e\0D\0f/g.arxml\0F\0dangling";
        assert_eq!(
            files(data),
            [("a.arxml", &b"A"[..]), ("f/g.arxml", &b"F"[..])]
        );
    }
}
