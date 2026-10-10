use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[path = "src/rules/grammar.rs"]
mod native_grammar;

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
        (path.starts_with("runtime/")
            || path.starts_with("third_party/freertos/")
            || path.starts_with("tools/python/src/ecu_tools/")
            || matches!(path.as_str(), "LICENSE" | "NOTICE"))
            && !path.contains(['\\', ':'])
            && Path::new(&path)
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        "uncontrolled asset path {path}"
    );
    assert!(matches!(license, "MIT" | "Apache-2.0"));
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
                .all(|profile| matches!(*profile, "ecu" | "ecu-multi" | "host"))
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
    let official_path = root.join("core/resources/official.json");
    let official = read_manifest(root, "core/resources/official.json");
    let mut official_constants = String::new();
    for (environment, constant) in [
        ("AUTOSAR_XSD_ARCHIVE", "SCHEMA_ZIP"),
        ("AUTOSAR_MOD_ARCHIVE", "MOD_ZIP"),
        ("AUTOSAR_SAMPLE_ARCHIVE", "SAMPLE_ZIP"),
    ] {
        let entry = official
            .as_array()
            .expect("official resources")
            .iter()
            .find(|entry| entry["environment"] == environment)
            .expect("official resource");
        official_constants.push_str(&format!(
            "pub const {constant}: &str = {:?};\n",
            entry["path"].as_str().expect("official path")
        ));
        if constant == "SCHEMA_ZIP" {
            official_constants.push_str(&format!(
                "pub const XSD_SHA256: &str = {:?};\n",
                entry["sha256"].as_str().expect("XSD identity")
            ));
        }
    }
    fs::write(
        PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR")).join("official_resources.rs"),
        official_constants,
    )
    .expect("official constants");
    println!("cargo:rerun-if-changed={}", official_path.display());
    let helper = root.join("tools/python/src/ecu_tools/workbench_v2.py");
    let helper_digest = format!(
        "{:x}",
        Sha256::digest(fs::read(&helper).expect("v2 helper"))
    );
    let helper_manifest =
        read_manifest(root, "tools/python/src/ecu_tools/workbench-v2-assets.json");
    assert_eq!(helper_manifest["files"][0]["path"], "workbench_v2.py");
    assert_eq!(
        helper_manifest["files"][0]["sha256"].as_str(),
        Some(helper_digest.as_str()),
        "v2 helper digest differs; review the source and update the asset inventory"
    );
    let helper_index = format!("const TRUSTED_HELPER_SHA256: &str = {helper_digest:?};\n");
    fs::write(
        PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR")).join("delivery_helper.rs"),
        helper_index,
    )
    .expect("helper identity");
    println!("cargo:rerun-if-changed={}", helper.display());
    println!(
        "cargo:rerun-if-changed={}",
        root.join("tools/python/src/ecu_tools/workbench-v2-assets.json")
            .display()
    );
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
        ("runtime/contracts/bsw-v1.json", "Apache-2.0", common),
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
        "Apache-2.0",
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
            "Apache-2.0",
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
    build_message_templates(&core);
    build_native_inventory(&core);
    println!(
        "cargo:rerun-if-changed={}",
        root.join("runtime/contracts/assets-v1.json").display()
    );
}

fn build_message_templates(core: &Path) {
    use std::collections::BTreeSet;

    fn placeholders(template: &str) -> BTreeSet<&str> {
        template
            .split("{{")
            .skip(1)
            .map(|part| {
                part.split_once("}}")
                    .expect("closed message interpolation")
                    .0
            })
            .collect()
    }

    let path = core.join("src/messages.json");
    println!("cargo:rerun-if-changed={}", path.display());
    let resources: BTreeMap<String, BTreeMap<String, String>> =
        serde_json::from_slice(&fs::read(path).expect("backend message catalog"))
            .expect("valid backend message catalog");
    let chinese = resources.get("zh-CN").expect("Chinese backend messages");
    let english = resources.get("en").expect("English backend messages");
    assert_eq!(resources.len(), 2, "unexpected backend catalog language");
    assert_eq!(
        chinese.keys().collect::<Vec<_>>(),
        english.keys().collect::<Vec<_>>(),
        "backend catalog language keys differ",
    );
    let mut generated = String::from("const ENGLISH_TEMPLATES: &[(&str, &str)] = &[\n");
    for (key, template) in english {
        assert!(
            key.starts_with("backend."),
            "backend message namespace: {key}"
        );
        assert!(
            !template.is_empty() && !chinese[key].is_empty(),
            "empty backend message: {key}"
        );
        assert_eq!(
            placeholders(&chinese[key]),
            placeholders(template),
            "backend message interpolation differs: {key}",
        );
        generated.push_str(&format!("({key:?}, {template:?}),\n"));
    }
    generated.push_str("];\n");
    let output = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    fs::write(output.join("message_templates.rs"), generated)
        .expect("write static backend message templates");
}

fn collect_native_sources(core: &Path, relative: &str, paths: &mut Vec<String>) {
    let path = core.join(relative);
    println!("cargo:rerun-if-changed={}", path.display());
    let metadata = fs::symlink_metadata(&path).expect("native rule source metadata");
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(
            metadata.file_attributes() & 0x400,
            0,
            "native rule sources cannot be reparse points"
        );
    }
    assert!(!metadata.file_type().is_symlink(), "linked native source");
    if metadata.is_dir() {
        for entry in fs::read_dir(&path).expect("native rule source directory") {
            let entry = entry.expect("native rule source entry");
            let name = entry.file_name().into_string().expect("UTF-8 source name");
            collect_native_sources(core, &format!("{relative}/{name}"), paths);
        }
    } else {
        assert!(
            metadata.is_file(),
            "native rule source is not a regular file"
        );
        paths.push(relative.to_owned());
    }
}

fn build_native_inventory(core: &Path) {
    let mut names = std::collections::BTreeSet::new();
    for (name, shape) in native_grammar::STRUCTURES {
        assert!(names.insert(*name), "duplicate native structural rule");
        assert!(
            shape.split_ascii_whitespace().count() <= 32,
            "native child group capacity"
        );
    }
    for (name, _) in native_grammar::SCALARS {
        assert!(names.insert(*name), "duplicate native scalar rule");
    }
    for name in native_grammar::REFERENCES {
        assert!(names.insert(*name), "duplicate native reference rule");
    }
    for name in native_grammar::OPEN_CHOICES {
        assert!(
            native_grammar::STRUCTURES.iter().any(|(parent, shape)| {
                parent == name && shape.split_ascii_whitespace().count() == 1
            }),
            "open native choice must be a single choice wrapper"
        );
    }
    // Missing definitions during parallel implementation fail the build rather
    // than publishing a different, apparently successful partial rule identity.
    let mut paths = Vec::new();
    for relative in [
        "build.rs",
        "src/rules.rs",
        "src/rules",
        "src/schema.rs",
        "src/definitions.rs",
        "src/model.rs",
        "src/project_model.rs",
        "src/arxml.rs",
        "src/arxml",
        "src/integration",
        "src/prepared.rs",
        "src/generator.rs",
        "src/generator",
        "src/verification.rs",
    ] {
        collect_native_sources(core, relative, &mut paths);
    }
    let definitions = core.join("src/definitions");
    println!("cargo:rerun-if-changed={}", definitions.display());
    if definitions.exists() {
        collect_native_sources(core, "src/definitions", &mut paths);
    }
    paths.sort();
    paths.dedup();
    let root = core.parent().expect("workspace root");
    let mut source_paths: Vec<_> = paths
        .into_iter()
        .map(|path| (format!("core/{path}"), core.join(path)))
        .collect();
    for relative in [
        "tools/python/src/ecu_tools/workbench_v2.py",
        "tools/python/src/ecu_tools/workbench-v2-assets.json",
    ] {
        let absolute = root.join(relative);
        let metadata = fs::symlink_metadata(&absolute).expect("native delivery source metadata");
        assert!(
            metadata.is_file() && !metadata.file_type().is_symlink(),
            "native delivery source must be a regular file"
        );
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            assert_eq!(
                metadata.file_attributes() & 0x400,
                0,
                "native delivery source cannot be a reparse point"
            );
        }
        println!("cargo:rerun-if-changed={}", absolute.display());
        source_paths.push((relative.to_owned(), absolute));
    }
    source_paths.sort_by(|left, right| left.0.cmp(&right.0));
    let mut sources = Vec::new();
    let mut generated = String::from("static INVENTORY_SOURCES: &[InventorySource] = &[\n");
    for (path, absolute) in source_paths {
        let bytes = fs::read(&absolute).expect("native implementation bytes");
        assert!(
            !bytes.is_empty(),
            "empty native implementation source: {path}"
        );
        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        sources.push(serde_json::json!({"path": path, "sha256": sha256}));
        generated.push_str(&format!(
            "InventorySource {{ path: {path:?}, sha256: {sha256:?}, bytes: include_bytes!({:?}) }},\n",
            absolute.to_str().expect("UTF-8 source path")
        ));
    }
    generated.push_str("];\n");
    let coverage: Vec<_> = native_grammar::coverage_rows()
        .into_iter()
        .map(|(scope, rule_id, subject)| {
            serde_json::json!({
                "ruleId": rule_id, "scope": scope, "subjects": [subject],
                "supported": true, "reason": null
            })
        })
        .chain(std::iter::once(serde_json::json!({
            "ruleId": "native.unsupported", "scope": "schema",
            "subjects": ["unlisted structure, attributes, conditions, variants and expression semantics"],
            "supported": false,
            "reason": "Only the listed product-authored native rules execute; this is not full XSD certification."
        })))
        .collect();
    let inventory = serde_json::json!({
        "format": "autosar-native-rule-inventory-v1",
        "release": native_grammar::RELEASE,
        "rulesVersion": native_grammar::RULES_VERSION,
        "sources": sources,
        "coverage": coverage
    });
    let bytes = serde_json::to_vec(&inventory).expect("deterministic rule inventory");
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    fs::write(out.join("native_rule_inventory.json"), &bytes).expect("native inventory");
    generated.push_str(&format!(
        "const TRUSTED_INVENTORY_SHA256: &str = {sha256:?};\n\
         static INVENTORY_BYTES: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/native_rule_inventory.json\"));\n"
    ));
    fs::write(out.join("native_rule_index.rs"), generated).expect("trusted native index");
}
