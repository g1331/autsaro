use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Clone)]
struct Source {
    path: String,
    expected: String,
    license: String,
    role: String,
    responsibility: String,
    targets: Vec<String>,
    profiles: Vec<String>,
}

fn read_manifest(root: &Path, name: &str) -> serde_json::Value {
    serde_json::from_slice(&fs::read(root.join(name)).expect(name)).expect(name)
}

fn add(
    rows: &mut BTreeMap<String, Source>,
    path: String,
    hash: String,
    license: &str,
    role: &str,
    responsibility: &str,
    targets: &[&str],
    profiles: &[&str],
) {
    assert!(
        (path.starts_with("runtime/") || path.starts_with("third_party/freertos/"))
            && !path.contains(['\\', ':'])
            && Path::new(&path)
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        "uncontrolled asset path {path}"
    );
    assert!(matches!(license, "MIT" | "Project-owned"));
    assert!(matches!(
        role,
        "bsw" | "os" | "kernel" | "patch" | "target" | "delivery" | "provenance"
    ));
    assert!(!responsibility.is_empty());
    assert!(
        !targets.is_empty()
            && targets.iter().all(|id| {
                matches!(*id, "windows-x64-controlled-v1" | "linux-x64-controlled-v1")
            })
    );
    assert!(
        !profiles.is_empty()
            && profiles
                .iter()
                .all(|profile| matches!(*profile, "ecu" | "host"))
    );
    assert!(hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert!(
        rows.insert(
            path.clone(),
            Source {
                path,
                expected: hash,
                license: license.into(),
                role: role.into(),
                responsibility: responsibility.into(),
                targets: targets.iter().map(|id| id.to_string()).collect(),
                profiles: profiles.iter().map(|profile| profile.to_string()).collect(),
            }
        )
        .is_none(),
        "duplicate asset identity"
    );
}

fn main() {
    let core = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let root = core.parent().expect("workspace root");
    let common: &[&str] = &["windows-x64-controlled-v1", "linux-x64-controlled-v1"];
    let windows: &[&str] = &["windows-x64-controlled-v1"];
    let linux: &[&str] = &["linux-x64-controlled-v1"];
    let manifest = read_manifest(root, "runtime/contracts/assets-v1.json");
    assert_eq!(manifest["format"], "autosar-asset-inventory-v1");
    let mut rows = BTreeMap::new();
    for entry in manifest["assets"].as_array().expect("assets array") {
        let path = entry["path"].as_str().expect("asset path").to_owned();
        let targets: Vec<_> = entry["targets"]
            .as_array()
            .expect("asset targets")
            .iter()
            .map(|value| value.as_str().expect("target name"))
            .collect();
        let profiles: Vec<_> = entry["profiles"]
            .as_array()
            .expect("asset profiles")
            .iter()
            .map(|value| value.as_str().expect("profile name"))
            .collect();
        add(
            &mut rows,
            path,
            entry["sha256"].as_str().expect("asset SHA-256").into(),
            entry["license"].as_str().expect("asset license"),
            entry["role"].as_str().expect("asset role"),
            entry["responsibility"].as_str().expect("asset owner"),
            &targets,
            &profiles,
        );
    }
    let provenance = [
        ("runtime/contracts/bsw-v1.json", "Project-owned", common),
        ("third_party/freertos/source-manifest.json", "MIT", common),
        (
            "third_party/freertos/posix-source-manifest.json",
            "MIT",
            linux,
        ),
    ];
    let listed: Vec<_> = manifest["externalIdentitySources"]
        .as_array()
        .expect("provenance")
        .iter()
        .map(|value| value.as_str().expect("provenance path"))
        .collect();
    assert_eq!(
        listed,
        provenance.iter().map(|row| row.0).collect::<Vec<_>>()
    );
    for (path, license, targets) in provenance {
        let bytes = fs::read(root.join(path)).expect("provenance file");
        add(
            &mut rows,
            path.into(),
            format!("{:x}", Sha256::digest(bytes)),
            license,
            "provenance",
            "upstream inventory",
            targets,
            &["ecu"],
        );
    }
    let own_manifest = "runtime/contracts/assets-v1.json";
    let own_bytes = fs::read(root.join(own_manifest)).expect("asset inventory");
    add(
        &mut rows,
        own_manifest.into(),
        format!("{:x}", Sha256::digest(own_bytes)),
        "Project-owned",
        "provenance",
        "workbench",
        common,
        &["ecu", "host"],
    );
    let bsw = read_manifest(root, "runtime/contracts/bsw-v1.json");
    assert_eq!(bsw["formatVersion"], 1);
    for (path, hash) in bsw["sources"].as_object().expect("BSW source map") {
        add(
            &mut rows,
            path.clone(),
            hash.as_str().expect("BSW SHA-256").into(),
            "Project-owned",
            "bsw",
            "workbench",
            common,
            &["ecu", "host"],
        );
    }
    for (manifest_path, targets) in [
        ("third_party/freertos/source-manifest.json", windows),
        ("third_party/freertos/posix-source-manifest.json", linux),
    ] {
        let kernel = read_manifest(root, manifest_path);
        assert_eq!(kernel["tag"], "V11.3.1");
        assert_eq!(kernel["commit"], "054e14f3397023aa83813a65aa065fc4597d481b");
        assert_eq!(kernel["license"], "MIT");
        for (file, hash) in kernel["files"].as_object().expect("kernel source map") {
            let path = format!("third_party/freertos/{file}");
            let applicable = if file.starts_with("portable/MSVC-MingW/") {
                windows
            } else if file.starts_with("portable/ThirdParty/GCC/Posix/") {
                linux
            } else {
                common
            };
            assert!(targets.iter().any(|id| applicable.contains(id)));
            if let Some(existing) = rows.get_mut(&path) {
                assert_eq!(existing.expected, hash.as_str().expect("kernel SHA-256"));
                existing.targets = common.iter().map(|id| id.to_string()).collect();
            } else {
                add(
                    &mut rows,
                    path,
                    hash.as_str().expect("kernel SHA-256").into(),
                    "MIT",
                    "kernel",
                    "FreeRTOS upstream",
                    applicable,
                    &["ecu"],
                );
            }
        }
    }
    let mut output = String::from("pub static EMBEDDED_ASSETS: &[AssetEntry] = &[\n");
    for item in rows.values() {
        let path = root.join(&item.path);
        let metadata = fs::symlink_metadata(&path).expect("asset file");
        assert!(
            metadata.file_type().is_file(),
            "asset is not a regular file: {}",
            item.path
        );
        let contents = fs::read(&path).expect("asset bytes");
        assert_eq!(
            format!("{:x}", Sha256::digest(&contents)),
            item.expected,
            "asset digest differs: {}",
            item.path
        );
        println!("cargo:rerun-if-changed={}", path.display());
        output.push_str(&format!(
            "    AssetEntry {{ relative_path: {:?}, sha256: {:?}, license: {:?}, role: {:?}, responsibility: {:?}, targets: &[{}], profiles: &[{}], bytes: include_bytes!({:?}) }},\n",
            item.path, item.expected, item.license, item.role, item.responsibility,
            item.targets.iter().map(|id| format!("{id:?}")).collect::<Vec<_>>().join(", "),
            item.profiles.iter().map(|name| format!("{name:?}")).collect::<Vec<_>>().join(", "),
            path.to_str().expect("UTF-8 asset path"),
        ));
    }
    output.push_str("];\n");
    fs::write(
        PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR")).join("asset_index.rs"),
        output,
    )
    .expect("write embedded asset index");
    println!(
        "cargo:rerun-if-changed={}",
        root.join("runtime/contracts/assets-v1.json").display()
    );
}
