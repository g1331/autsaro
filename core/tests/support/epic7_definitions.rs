use autosar_config_core::definitions::DefinitionCatalog;
use autosar_config_core::project_model::{
    RuleSetIdentity, TypedValue, ValidationStatus, ValueKind,
};
use sha2::{Digest, Sha256};
use std::fs;
#[cfg(feature = "official-oracles")]
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const ROOT: &str = "/AUTOSAR/EcucDefs/";

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "autosar-definitions-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn identity() -> RuleSetIdentity {
    RuleSetIdentity {
        release: "R24-11".into(),
        rules_version: "product-rules-v1".into(),
        sha256: "a".repeat(64),
    }
}

fn document(elements: &str) -> String {
    format!(
        "<AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Project</SHORT-NAME><ELEMENTS>{elements}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>"
    )
}

fn field(catalog: &DefinitionCatalog, id: &str, lexeme: &str) -> String {
    let definition = catalog.get(id).unwrap();
    let tag = match definition.kind.unwrap() {
        ValueKind::Integer | ValueKind::Float | ValueKind::Boolean => "ECUC-NUMERICAL-PARAM-VALUE",
        _ => "ECUC-TEXTUAL-PARAM-VALUE",
    };
    format!(
        "<{tag}><DEFINITION-REF DEST=\"{}\">{id}</DEFINITION-REF><VALUE>{lexeme}</VALUE></{tag}>",
        definition.element_kind
    )
}

fn container(id: &str, name: &str, fields: &str, children: &str) -> String {
    format!(
        "<ECUC-CONTAINER-VALUE><SHORT-NAME>{name}</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">{id}</DEFINITION-REF><PARAMETER-VALUES>{fields}</PARAMETER-VALUES><SUB-CONTAINERS>{children}</SUB-CONTAINERS></ECUC-CONTAINER-VALUE>"
    )
}

fn module(id: &str, name: &str, children: &str) -> String {
    format!(
        "<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>{name}</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-MODULE-DEF\">{id}</DEFINITION-REF><CONTAINERS>{children}</CONTAINERS></ECUC-MODULE-CONFIGURATION-VALUES>"
    )
}

fn extension(directory: &Path, id: &str, package: &str) -> PathBuf {
    fs::create_dir_all(directory).unwrap();
    let xml = document(&format!(
        "<ECUC-MODULE-DEF><SHORT-NAME>{package}</SHORT-NAME><LOWER-MULTIPLICITY>0</LOWER-MULTIPLICITY><UPPER-MULTIPLICITY>1</UPPER-MULTIPLICITY><CONTAINERS><ECUC-PARAM-CONF-CONTAINER-DEF><SHORT-NAME>Settings</SHORT-NAME><LOWER-MULTIPLICITY>0</LOWER-MULTIPLICITY><UPPER-MULTIPLICITY>1</UPPER-MULTIPLICITY><PARAMETERS><ECUC-INTEGER-PARAM-DEF><SHORT-NAME>Limit</SHORT-NAME><LOWER-MULTIPLICITY>0</LOWER-MULTIPLICITY><UPPER-MULTIPLICITY>1</UPPER-MULTIPLICITY><MIN>0</MIN><MAX>18446744073709551615</MAX><DEFAULT-VALUE>9007199254740993</DEFAULT-VALUE></ECUC-INTEGER-PARAM-DEF></PARAMETERS></ECUC-PARAM-CONF-CONTAINER-DEF></CONTAINERS></ECUC-MODULE-DEF>"
    ));
    fs::write(directory.join("definitions.arxml"), &xml).unwrap();
    let inventory = serde_json::json!({"formatVersion":1,"catalogId":id,"release":"R24-11","version":"1.0.0","files":[{"path":"definitions.arxml","sha256":format!("{:x}",Sha256::digest(xml.as_bytes()))}]});
    let path = directory.join("catalog.json");
    fs::write(&path, serde_json::to_vec_pretty(&inventory).unwrap()).unwrap();
    path
}

#[test]
fn builtin_values_preserve_integer_precision_and_reject_wrong_kind_and_nonfinite() {
    let catalog = DefinitionCatalog::builtin().unwrap();
    let event = format!("{ROOT}Os/OsEvent/OsEventMask");
    for raw in [
        "9007199254740993",
        "18446744073709551615",
        "0xffffffffffffffff",
    ] {
        catalog
            .validate_value(
                &event,
                &TypedValue {
                    kind: ValueKind::Integer,
                    lexeme: raw.into(),
                },
            )
            .unwrap();
    }
    for raw in ["18446744073709551616", "-1", "9007199254740993.0"] {
        assert!(
            catalog
                .validate_value(
                    &event,
                    &TypedValue {
                        kind: ValueKind::Integer,
                        lexeme: raw.into()
                    }
                )
                .is_err()
        );
    }
    assert!(
        catalog
            .validate_value(
                &event,
                &TypedValue {
                    kind: ValueKind::Float,
                    lexeme: "1".into()
                }
            )
            .is_err()
    );
    let timeout = format!("{ROOT}Com/ComConfig/ComSignal/ComTimeout");
    for raw in ["NaN", "inf", "-0.1", "3600.001"] {
        assert!(
            catalog
                .validate_value(
                    &timeout,
                    &TypedValue {
                        kind: ValueKind::Float,
                        lexeme: raw.into()
                    }
                )
                .is_err()
        );
    }
    catalog
        .validate_value(
            &timeout,
            &TypedValue {
                kind: ValueKind::Float,
                lexeme: "0.0010".into(),
            },
        )
        .unwrap();
}

#[test]
fn builtin_all_seven_value_kinds_apply_authored_constraints() {
    let catalog = DefinitionCatalog::builtin().unwrap();
    for (id, kind, valid, invalid) in [
        ("Os/OsTask/OsTaskPriority", ValueKind::Integer, "5", "-1"),
        (
            "Com/ComConfig/ComSignal/ComTimeout",
            ValueKind::Float,
            "0.020",
            "NaN",
        ),
        (
            "Can/CanGeneral/CanDevErrorDetect",
            ValueKind::Boolean,
            "true",
            "yes",
        ),
        (
            "Os/OsTask/OsTaskSchedule",
            ValueKind::Enumeration,
            "FULL",
            "PARTIAL",
        ),
        (
            "CanIf/CanIfInitCfg/CanIfInitCfgSet",
            ValueKind::String,
            "HostInit",
            "",
        ),
        (
            "Dcm/DcmConfigSet/DcmDsp/DcmDspData/DcmDspDataReadFnc",
            ValueKind::FunctionName,
            "Read_Value",
            "Read_Value()",
        ),
        (
            "Os/OsTask/OsTaskEventRef",
            ValueKind::Reference,
            "/Project/Os/Event",
            "../Event",
        ),
    ] {
        catalog
            .validate_value(
                &format!("{ROOT}{id}"),
                &TypedValue {
                    kind,
                    lexeme: valid.into(),
                },
            )
            .unwrap();
        if kind != ValueKind::String {
            assert!(
                catalog
                    .validate_value(
                        &format!("{ROOT}{id}"),
                        &TypedValue {
                            kind,
                            lexeme: invalid.into()
                        }
                    )
                    .is_err()
            );
        } else {
            catalog
                .validate_value(
                    &format!("{ROOT}{id}"),
                    &TypedValue {
                        kind,
                        lexeme: invalid.into(),
                    },
                )
                .unwrap();
        }
    }
}

#[test]
fn builtin_default_is_metadata_not_a_synthesized_explicit_entry() {
    let catalog = DefinitionCatalog::builtin().unwrap();
    let id = format!("{ROOT}Can/CanGeneral/CanDevErrorDetect");
    let definition = catalog.get(&id).unwrap();
    assert_eq!(definition.default_value.as_ref().unwrap().lexeme, "false");
    assert!(
        definition
            .default_origin
            .as_ref()
            .unwrap()
            .starts_with("builtin:")
    );
    let general = container(&format!("{ROOT}Can/CanGeneral"), "General", "", "");
    let xml = document(&module(&format!("{ROOT}Can"), "Can", &general));
    let before = xml.clone();
    let result = catalog
        .validate_documents(&[(Path::new("can.arxml"), &xml)])
        .unwrap();
    assert!(result.diagnostics.iter().any(|issue| {
        issue.code == "MULTIPLICITY"
            && matches!(
                &issue.witness.as_ref().unwrap().constraint,
                autosar_config_core::LocalizedText::Raw(metadata) if metadata.contains("CanDevErrorDetect")
            )
    }));
    assert_eq!(xml, before);
    let explicit = field(&catalog, &id, "true");
    let xml = document(&module(
        &format!("{ROOT}Can"),
        "Can",
        &container(&format!("{ROOT}Can/CanGeneral"), "General", &explicit, ""),
    ));
    let result = catalog
        .validate_documents(&[(Path::new("can.arxml"), &xml)])
        .unwrap();
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|issue| issue.code == "VALUE_RANGE")
    );
    assert!(!result.diagnostics.iter().any(|issue| {
        issue.code == "MULTIPLICITY"
            && matches!(
                &issue.witness.as_ref().unwrap().constraint,
                autosar_config_core::LocalizedText::Raw(metadata) if metadata.contains("CanDevErrorDetect")
            )
    }));
}

#[test]
fn builtin_validation_detects_duplicate_entries_and_wrong_definition_destination() {
    let catalog = DefinitionCatalog::builtin().unwrap();
    let id = format!("{ROOT}Os/OsEvent/OsEventMask");
    let value = field(&catalog, &id, "9007199254740993");
    let xml = document(&module(
        &format!("{ROOT}Os"),
        "Os",
        &container(
            &format!("{ROOT}Os/OsEvent"),
            "Event",
            &(value.clone() + &value),
            "",
        ),
    ));
    let result = catalog
        .validate_documents(&[(Path::new("os.arxml"), &xml)])
        .unwrap();
    assert_eq!(result.status, ValidationStatus::Failed);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|issue| issue.code == "MULTIPLICITY"
                && issue.path.as_deref() == Some("/Project/Os/Event")
                && issue.file.as_deref() == Some("os.arxml"))
    );
    let wrong = xml.replace(
        "DEST=\"ECUC-INTEGER-PARAM-DEF\"",
        "DEST=\"ECUC-FLOAT-PARAM-DEF\"",
    );
    let result = catalog
        .validate_documents(&[(Path::new("os.arxml"), &wrong)])
        .unwrap();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|issue| issue.code == "DEFINITION_DEST")
    );
}

#[test]
fn builtin_reference_rejects_right_dest_with_wrong_container_definition() {
    let catalog = DefinitionCatalog::builtin().unwrap();
    let reference = format!(
        "<ECUC-REFERENCE-VALUE><DEFINITION-REF DEST=\"ECUC-REFERENCE-DEF\">{ROOT}Os/OsTask/OsTaskEventRef</DEFINITION-REF><VALUE-REF DEST=\"ECUC-CONTAINER-VALUE\">/Project/Os/Counter</VALUE-REF></ECUC-REFERENCE-VALUE>"
    );
    let task = container(&format!("{ROOT}Os/OsTask"), "Task", "", "").replace(
        "</PARAMETER-VALUES>",
        &format!("</PARAMETER-VALUES><REFERENCE-VALUES>{reference}</REFERENCE-VALUES>"),
    );
    let counter = container(&format!("{ROOT}Os/OsCounter"), "Counter", "", "");
    let xml = document(&module(&format!("{ROOT}Os"), "Os", &(task + &counter)));
    let result = catalog
        .validate_documents(&[(Path::new("os.arxml"), &xml)])
        .unwrap();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|issue| issue.code == "REFERENCE_TARGET")
    );
    let event = xml.replace(
        &format!("{ROOT}Os/OsCounter</DEFINITION-REF>"),
        &format!("{ROOT}Os/OsEvent</DEFINITION-REF>"),
    );
    let result = catalog
        .validate_documents(&[(Path::new("os.arxml"), &event)])
        .unwrap();
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|issue| issue.code.starts_with("REFERENCE_"))
    );
    let unresolved = event.replace(
        "/Project/Os/Counter</VALUE-REF>",
        "/Project/Os/Missing</VALUE-REF>",
    );
    let result = catalog
        .validate_documents(&[(Path::new("os.arxml"), &unresolved)])
        .unwrap();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|issue| issue.code == "REFERENCE_UNRESOLVED")
    );
}

#[test]
fn builtin_unknown_and_instance_reference_semantics_are_unsupported_not_passed() {
    let catalog = DefinitionCatalog::builtin().unwrap();
    let unknown = container("/Vendor/Unaccepted/Settings", "Settings", "", "");
    let xml = document(&unknown);
    let result = catalog
        .validate_documents(&[(Path::new("vendor.arxml"), &xml)])
        .unwrap();
    assert_eq!(result.status, ValidationStatus::Unsupported);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|issue| issue.code == "DEFINITION_UNKNOWN")
    );
    let parameter = field(&catalog, &format!("{ROOT}Os/OsEvent/OsEventMask"), "1").replace(
        "<VALUE>1</VALUE>",
        "<VALUE-EXPR><SYSC-REF DEST=\"SYSTEM-CONSTANT\">/Constants/Mask</SYSC-REF></VALUE-EXPR>",
    );
    let xml = document(&container(
        &format!("{ROOT}Os/OsEvent"),
        "Event",
        &parameter,
        "",
    ));
    let result = catalog
        .validate_documents(&[(Path::new("expression.arxml"), &xml)])
        .unwrap();
    assert!(result.coverage.iter().any(|coverage| !coverage.supported));
    assert!(
        result
            .diagnostics
            .iter()
            .any(|issue| issue.code == "SEMANTICS_UNSUPPORTED")
    );
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|issue| issue.code == "VALUE_RANGE")
    );
}

#[test]
fn extension_acceptance_is_explicit_and_restore_requires_exact_complete_cache() {
    let temp = Directory::new();
    let path = extension(&temp.0.join("source"), "vendor-one", "VendorOne");
    let cache = temp.0.join("cache");
    let mut catalog = DefinitionCatalog::builtin().unwrap();
    let original = catalog.fingerprint(&identity());
    let accepted = catalog.accept_extension(&path, &cache).unwrap();
    assert_eq!(
        accepted.sha256,
        format!("{:x}", Sha256::digest(fs::read(&path).unwrap()))
    );
    let id = "/Project/VendorOne/Settings/Limit";
    catalog
        .validate_value(
            id,
            &TypedValue {
                kind: ValueKind::Integer,
                lexeme: "18446744073709551615".into(),
            },
        )
        .unwrap();
    assert_eq!(
        catalog.required_extensions(&[id.into()]),
        vec![accepted.clone()]
    );
    assert!(
        catalog
            .required_extensions(&[format!("{ROOT}Os/OsEvent/OsEventMask")])
            .is_empty()
    );
    assert_ne!(original, catalog.fingerprint(&identity()));
    let mut fresh = DefinitionCatalog::builtin().unwrap();
    assert!(fresh.get(id).is_none());
    assert!(
        fresh
            .restore_extensions(&[accepted.clone()], &cache)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        fresh.fingerprint(&identity()),
        catalog.fingerprint(&identity())
    );
    fresh.remove_extension("vendor-one").unwrap();
    assert!(fresh.get(id).is_none());
    assert!(
        cache
            .join(&accepted.sha256)
            .join("definitions.arxml")
            .exists()
    );
    assert!(path.exists());
    fs::write(
        cache.join(&accepted.sha256).join("definitions.arxml"),
        b"corrupt",
    )
    .unwrap();
    let old = catalog.fingerprint(&identity());
    assert!(catalog.accept_extension(&path, &cache).is_err());
    assert_eq!(catalog.fingerprint(&identity()), old);
    let issues = fresh
        .restore_extensions(&[accepted.clone()], &cache)
        .unwrap();
    assert!(issues.iter().any(|issue| issue.code == "EXTENSION_MISSING"));
    assert_eq!(fresh.accepted_extensions(), vec![accepted]);
    assert!(
        fresh
            .get(&format!("{ROOT}Os/OsEvent/OsEventMask"))
            .is_some()
    );
    assert!(fresh.get(id).is_none());
}

#[test]
fn extension_rejects_wrong_release_digest_extra_payload_and_identity_conflict_atomically() {
    let temp = Directory::new();
    let path = extension(&temp.0.join("source"), "vendor", "Vendor");
    let cache = temp.0.join("cache");
    let mut catalog = DefinitionCatalog::builtin().unwrap();
    let accepted = catalog.accept_extension(&path, &cache).unwrap();
    let old = catalog.fingerprint(&identity());
    let raw = fs::read(&path).unwrap();
    let mut inventory: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    inventory["release"] = "R23-11".into();
    fs::write(&path, serde_json::to_vec(&inventory).unwrap()).unwrap();
    assert!(catalog.accept_extension(&path, &cache).is_err());
    fs::write(&path, &raw).unwrap();
    fs::write(path.parent().unwrap().join("plugin.exe"), b"not executable").unwrap();
    assert!(catalog.accept_extension(&path, &cache).is_err());
    fs::remove_file(path.parent().unwrap().join("plugin.exe")).unwrap();
    fs::write(
        path.parent().unwrap().join("definitions.arxml"),
        b"broken digest",
    )
    .unwrap();
    assert!(catalog.accept_extension(&path, &cache).is_err());
    let replacement = extension(&temp.0.join("replacement"), "vendor", "DifferentModule");
    assert!(catalog.accept_extension(&replacement, &cache).is_err());
    assert_eq!(catalog.accepted_extensions(), vec![accepted]);
    assert_eq!(catalog.fingerprint(&identity()), old);
}

#[test]
fn extension_fingerprint_is_sorted_and_independent_of_source_and_cache_paths() {
    let temp = Directory::new();
    let one = extension(&temp.0.join("one"), "one", "One");
    let two = extension(&temp.0.join("two"), "two", "Two");
    let mut forward = DefinitionCatalog::builtin().unwrap();
    let mut reverse = DefinitionCatalog::builtin().unwrap();
    forward
        .accept_extension(&one, &temp.0.join("cache-one"))
        .unwrap();
    forward
        .accept_extension(&two, &temp.0.join("cache-one"))
        .unwrap();
    reverse
        .accept_extension(&two, &temp.0.join("cache-two"))
        .unwrap();
    reverse
        .accept_extension(&one, &temp.0.join("cache-two"))
        .unwrap();
    assert_eq!(
        forward.fingerprint(&identity()),
        reverse.fingerprint(&identity())
    );
    assert_eq!(
        forward
            .required_extensions(&["/Project/One/Settings/Limit".into()])
            .iter()
            .map(|id| id.catalog_id.as_str())
            .collect::<Vec<_>>(),
        vec!["one"]
    );
}

#[cfg(feature = "official-oracles")]
#[test]
fn fixed_r24_11_oracle_agrees_on_integer_precision_enum_and_default() {
    let archive_path = autosar_config_core::schema::reference_archive(
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap(),
        "AUTOSAR_MOD_ARCHIVE",
        autosar_config_core::schema::MOD_ZIP,
    );
    let mut archive =
        zip::ZipArchive::new(fs::File::open(&archive_path).expect("fixed local R24-11 MOD oracle"))
            .unwrap();
    let mut xml = String::new();
    archive
        .by_name("AUTOSAR_CP_MOD_ECUConfigurationParameters.arxml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(xml.as_bytes())),
        "55ad8924b9aaccf600effc06e779c2d9c3300d182749e1bc7c079a55e7a2c68f"
    );
    let document = roxmltree::Document::parse(&xml).unwrap();
    let catalog = DefinitionCatalog::builtin().unwrap();
    let mut oracle = std::collections::BTreeMap::new();
    for node in document.descendants().filter(|node| {
        node.is_element()
            && node
                .children()
                .any(|child| child.has_tag_name("SHORT-NAME"))
    }) {
        let mut names: Vec<_> = node
            .ancestors()
            .filter_map(|owner| {
                owner
                    .children()
                    .find(|child| child.has_tag_name("SHORT-NAME"))
                    .and_then(|child| child.text())
            })
            .collect();
        names.reverse();
        oracle.insert(format!("/{}", names.join("/")), node);
    }
    fn compare<T: std::fmt::Debug + PartialEq>(
        failures: &mut Vec<String>,
        id: &str,
        constraint: &str,
        oracle: T,
        authored: T,
    ) {
        if oracle != authored {
            failures.push(format!(
                "{id} {constraint}: oracle={oracle:?}, authored={authored:?}"
            ));
        }
    }
    let mut failures = Vec::new();
    for expected in catalog.definitions() {
        let id = &expected.definition_id;
        let Some(node) = oracle.get(id) else {
            failures.push(format!(
                "Authored definition {id} is absent from fixed oracle"
            ));
            continue;
        };
        compare(
            &mut failures,
            id,
            "kind",
            node.tag_name().name(),
            expected.element_kind.as_str(),
        );
        let text = |tag| {
            node.children()
                .find(|child| child.has_tag_name(tag))
                .and_then(|child| child.text())
        };
        compare(
            &mut failures,
            id,
            "minimum",
            text("MIN"),
            expected.minimum.as_deref(),
        );
        compare(
            &mut failures,
            id,
            "maximum",
            text("MAX").filter(|maximum| *maximum != "INF"),
            expected.maximum.as_deref(),
        );
        compare(
            &mut failures,
            id,
            "lower",
            text("LOWER-MULTIPLICITY").unwrap().parse::<u32>().unwrap(),
            expected.lower_multiplicity,
        );
        let upper = if text("UPPER-MULTIPLICITY-INFINITE") == Some("true") {
            None
        } else {
            Some(text("UPPER-MULTIPLICITY").unwrap().parse::<u32>().unwrap())
        };
        compare(
            &mut failures,
            id,
            "upper",
            upper,
            expected.upper_multiplicity,
        );
        compare(
            &mut failures,
            id,
            "default",
            text("DEFAULT-VALUE"),
            expected
                .default_value
                .as_ref()
                .map(|value| value.lexeme.as_str()),
        );
        if expected.kind == Some(ValueKind::Enumeration) {
            let mut actual: Vec<_> = node
                .descendants()
                .filter(|node| node.has_tag_name("ECUC-ENUMERATION-LITERAL-DEF"))
                .filter_map(|node| {
                    node.children()
                        .find(|child| child.has_tag_name("SHORT-NAME"))
                        .and_then(|child| child.text())
                })
                .collect();
            let mut authored: Vec<_> = expected.enumeration.iter().map(String::as_str).collect();
            actual.sort();
            authored.sort();
            compare(&mut failures, id, "literals", actual, authored);
        }
    }
    assert!(
        failures.is_empty(),
        "Fixed metadata oracle mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn builtin_standard_ecu_consumers_have_real_types_and_boundaries() {
    let catalog = DefinitionCatalog::builtin().unwrap();
    let xml = include_str!("../fixtures/epic4/positive/ecuc.arxml");
    let baseline = catalog
        .validate_documents(&[(Path::new("ecuc.arxml"), xml)])
        .unwrap();
    assert!(
        !baseline.diagnostics.iter().any(|issue| matches!(
            issue.code.as_str(),
            "DEFINITION_UNKNOWN"
                | "DEFINITION_DEST"
                | "DEFINITION_KIND"
                | "VALUE_RANGE"
                | "REFERENCE_TARGET"
        )),
        "{:?}",
        baseline.diagnostics
    );
    let document = roxmltree::Document::parse(xml).unwrap();
    let field = document
        .descendants()
        .find(|node| {
            node.children().any(|child| {
                child.tag_name().name() == "DEFINITION-REF"
                    && child.text()
                        == Some("/AUTOSAR/EcucDefs/Can/CanConfigSet/CanController/CanControllerId")
            })
        })
        .unwrap();
    let value = field
        .children()
        .find(|node| node.tag_name().name() == "VALUE")
        .unwrap();
    let mutated = format!(
        "{}<VALUE>256</VALUE>{}",
        &xml[..value.range().start],
        &xml[value.range().end..]
    );
    let result = catalog
        .validate_documents(&[(Path::new("ecuc.arxml"), &mutated)])
        .unwrap();
    assert!(result.diagnostics.iter().any(|issue| {
        issue.code == "VALUE_RANGE"
            && matches!(
                &issue.witness.as_ref().unwrap().constraint,
                autosar_config_core::LocalizedText::Raw(metadata) if metadata.contains("CanControllerId")
            )
    }));
}

#[test]
fn extension_rejects_traversal_reserved_names_and_extra_inventory_members() {
    let temp = Directory::new();
    let path = extension(&temp.0.join("source"), "vendor", "Vendor");
    let raw = fs::read(&path).unwrap();
    let mut catalog = DefinitionCatalog::builtin().unwrap();
    let original = catalog.fingerprint(&identity());
    for unsafe_path in [
        "../definitions.arxml",
        "/definitions.arxml",
        "C:/definitions.arxml",
        "nested\\\\definitions.arxml",
        "CON.arxml",
        "catalog.json",
        "definitions.arxml:payload",
    ] {
        let mut inventory: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        inventory["files"][0]["path"] = unsafe_path.into();
        fs::write(&path, serde_json::to_vec(&inventory).unwrap()).unwrap();
        assert!(
            catalog
                .accept_extension(&path, &temp.0.join("cache"))
                .is_err(),
            "{unsafe_path}"
        );
        assert_eq!(catalog.fingerprint(&identity()), original);
    }
    let mut inventory: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    let duplicate = inventory["files"][0].clone();
    inventory["files"].as_array_mut().unwrap().push(duplicate);
    fs::write(&path, serde_json::to_vec(&inventory).unwrap()).unwrap();
    assert!(
        catalog
            .accept_extension(&path, &temp.0.join("cache"))
            .is_err()
    );
}

#[test]
fn extension_cannot_shadow_builtin_definition_namespace() {
    let temp = Directory::new();
    let path = extension(&temp.0.join("source"), "vendor", "Os");
    let member = path.parent().unwrap().join("definitions.arxml");
    let raw = fs::read_to_string(&member).unwrap();
    let raw = raw.replace("<SHORT-NAME>Project</SHORT-NAME><ELEMENTS>",
        "<SHORT-NAME>AUTOSAR</SHORT-NAME><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>EcucDefs</SHORT-NAME><ELEMENTS>")
        .replace("</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>",
            "</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AR-PACKAGE></AR-PACKAGES></AUTOSAR>");
    fs::write(&member, &raw).unwrap();
    let mut inventory: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    inventory["files"][0]["sha256"] = format!("{:x}", Sha256::digest(raw.as_bytes())).into();
    fs::write(&path, serde_json::to_vec(&inventory).unwrap()).unwrap();
    let mut catalog = DefinitionCatalog::builtin().unwrap();
    let old = catalog.fingerprint(&identity());
    let error = catalog
        .accept_extension(&path, &temp.0.join("cache"))
        .unwrap_err();
    assert_eq!(
        serde_json::to_value(&error).unwrap()["key"],
        "backend.definitions.extension.builtin_namespace_replacement_forbidden"
    );
    assert_eq!(old, catalog.fingerprint(&identity()));
    assert_eq!(
        catalog
            .get(&format!("{ROOT}Os/OsEvent/OsEventMask"))
            .unwrap()
            .maximum
            .as_deref(),
        Some("18446744073709551615")
    );
}

#[cfg(unix)]
#[test]
fn extension_rejects_linked_source_members() {
    let temp = Directory::new();
    let path = extension(&temp.0.join("source"), "vendor", "Vendor");
    let member = path.parent().unwrap().join("definitions.arxml");
    let outside = temp.0.join("outside.arxml");
    fs::rename(&member, &outside).unwrap();
    std::os::unix::fs::symlink(&outside, &member).unwrap();
    let mut catalog = DefinitionCatalog::builtin().unwrap();
    let error = catalog
        .accept_extension(&path, &temp.0.join("cache"))
        .unwrap_err();
    assert_eq!(
        serde_json::to_value(&error).unwrap()["key"],
        "backend.definitions.extension.catalog_links_forbidden"
    );
    assert!(catalog.accepted_extensions().is_empty());
}

#[test]
fn builtin_foreign_reference_resolves_across_sources_and_checks_actual_destination() {
    let catalog = DefinitionCatalog::builtin().unwrap();
    let signal = format!("{ROOT}Com/ComConfig/ComSignal");
    let reference = format!(
        "<ECUC-REFERENCE-VALUE><DEFINITION-REF DEST=\"ECUC-FOREIGN-REFERENCE-DEF\">{signal}/ComSystemTemplateSystemSignalRef</DEFINITION-REF><VALUE-REF DEST=\"I-SIGNAL-TO-I-PDU-MAPPING\">/Project/Mapping</VALUE-REF></ECUC-REFERENCE-VALUE>"
    );
    let parameters = field(&catalog, &format!("{signal}/ComBitPosition"), "0")
        + &field(&catalog, &format!("{signal}/ComSignalType"), "UINT32")
        + &field(
            &catalog,
            &format!("{signal}/ComSignalEndianness"),
            "LITTLE_ENDIAN",
        );
    let signal = container(&signal, "Signal", &parameters, "").replace(
        "</PARAMETER-VALUES>",
        &format!("</PARAMETER-VALUES><REFERENCE-VALUES>{reference}</REFERENCE-VALUES>"),
    );
    let config = container(&format!("{ROOT}Com/ComConfig"), "Config", "", &signal);
    let values = document(&module(&format!("{ROOT}Com"), "Com", &config));
    let extract = document(
        "<I-SIGNAL-TO-I-PDU-MAPPING><SHORT-NAME>Mapping</SHORT-NAME></I-SIGNAL-TO-I-PDU-MAPPING>",
    );
    let files = [
        (Path::new("values.arxml"), values.as_str()),
        (Path::new("extract.arxml"), extract.as_str()),
    ];
    let result = catalog.validate_documents(&files).unwrap();
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|issue| issue.code.starts_with("REFERENCE_"))
    );
    let wrong = extract.replace("I-SIGNAL-TO-I-PDU-MAPPING", "I-SIGNAL");
    let result = catalog
        .validate_documents(&[
            (Path::new("values.arxml"), &values),
            (Path::new("extract.arxml"), &wrong),
        ])
        .unwrap();
    assert!(result.diagnostics.iter().any(
        |issue| issue.code == "REFERENCE_DEST" && issue.file.as_deref() == Some("values.arxml")
    ));
}

#[test]
fn builtin_can_signal_overlap_uses_endianness_and_stable_value_witnesses() {
    let catalog = DefinitionCatalog::builtin().unwrap();
    let signal = format!("{ROOT}Com/ComConfig/ComSignal");
    let make_signal = |name, start: &str, size: &str, endianness: &str| {
        let fields = field(&catalog, &format!("{signal}/ComBitPosition"), start)
            + &field(&catalog, &format!("{signal}/ComBitSize"), size)
            + &field(&catalog, &format!("{signal}/ComSignalType"), "UINT8")
            + &field(
                &catalog,
                &format!("{signal}/ComSignalEndianness"),
                endianness,
            );
        container(&signal, name, &fields, "")
    };
    let pdu = format!("{ROOT}Com/ComConfig/ComIPdu");
    let references: String = ["Big", "Little"].iter().map(|name| format!(
        "<ECUC-REFERENCE-VALUE><DEFINITION-REF DEST=\"ECUC-REFERENCE-DEF\">{pdu}/ComIPduSignalRef</DEFINITION-REF><VALUE-REF DEST=\"ECUC-CONTAINER-VALUE\">/Project/Com/Config/{name}</VALUE-REF></ECUC-REFERENCE-VALUE>")).collect();
    let pdu_value = container(&pdu, "Pdu", "", "").replace(
        "</PARAMETER-VALUES>",
        &format!("</PARAMETER-VALUES><REFERENCE-VALUES>{references}</REFERENCE-VALUES>"),
    );
    let create = |little_start| {
        let children = make_signal("Big", "7", "8", "BIG_ENDIAN")
            + &make_signal("Little", little_start, "8", "LITTLE_ENDIAN")
            + &pdu_value;
        document(&module(
            &format!("{ROOT}Com"),
            "Com",
            &container(&format!("{ROOT}Com/ComConfig"), "Config", "", &children),
        ))
    };
    let disjoint = create("8");
    let result = catalog
        .validate_documents(&[(Path::new("com.arxml"), &disjoint)])
        .unwrap();
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|issue| issue.code == "CAN_SIGNAL_OVERLAP")
    );
    let overlap = create("0");
    let result = catalog
        .validate_documents(&[(Path::new("com.arxml"), &overlap)])
        .unwrap();
    let issue = result
        .diagnostics
        .iter()
        .find(|issue| issue.code == "CAN_SIGNAL_OVERLAP")
        .unwrap();
    let witness = issue.witness.as_ref().unwrap();
    let autosar_config_core::LocalizedText::Raw(counterexample) = &witness.counterexample else {
        panic!("Signal layout evidence must remain raw machine data");
    };
    assert!(!counterexample.contains("/Project"));
    assert!(counterexample.contains("7:8:0:8"));
}

#[test]
fn builtin_unselected_variants_do_not_become_false_duplicate_errors() {
    let catalog = DefinitionCatalog::builtin().unwrap();
    let os = format!("{ROOT}Os/OsOS");
    let hooks = format!("{os}/OsHooks");
    let hook_fields: String = [
        "OsErrorHook",
        "OsPostTaskHook",
        "OsPreTaskHook",
        "OsShutdownHook",
        "OsStartupHook",
    ]
    .iter()
    .map(|name| field(&catalog, &format!("{hooks}/{name}"), "false"))
    .collect();
    let os_fields = field(&catalog, &format!("{os}/OsStatus"), "EXTENDED")
        + &field(&catalog, &format!("{os}/OsUseGetServiceId"), "false")
        + &field(&catalog, &format!("{os}/OsUseParameterAccess"), "false")
        + &field(&catalog, &format!("{os}/OsUseResScheduler"), "false");
    let operating_system = container(
        &os,
        "OperatingSystem",
        &os_fields,
        &container(&hooks, "Hooks", &hook_fields, ""),
    );
    let mask = field(&catalog, &format!("{ROOT}Os/OsEvent/OsEventMask"), "1").replace(
        "</ECUC-NUMERICAL-PARAM-VALUE>",
        "<VARIATION-POINT/></ECUC-NUMERICAL-PARAM-VALUE>",
    );
    let event = container(
        &format!("{ROOT}Os/OsEvent"),
        "Event",
        &(mask.clone() + &mask),
        "",
    );
    let mode = container(&format!("{ROOT}Os/OsAppMode"), "Default", "", "");
    let xml = document(&module(
        &format!("{ROOT}Os"),
        "Os",
        &(operating_system + &event + &mode),
    ));
    let result = catalog
        .validate_documents(&[(Path::new("os.arxml"), &xml)])
        .unwrap();
    assert_eq!(
        result.status,
        ValidationStatus::Unsupported,
        "{:?}",
        result.diagnostics
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|issue| issue.code == "SEMANTICS_UNSUPPORTED")
    );
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|issue| issue.code == "MULTIPLICITY")
    );
}

#[test]
fn builtin_overlap_reports_every_pair_so_removing_one_bad_signal_preserves_other_witnesses() {
    let catalog = DefinitionCatalog::builtin().unwrap();
    let signal = format!("{ROOT}Com/ComConfig/ComSignal");
    let pdu = format!("{ROOT}Com/ComConfig/ComIPdu");
    let create = |names: &[&str]| {
        let signals: String = names
            .iter()
            .map(|name| {
                let values = field(&catalog, &format!("{signal}/ComBitPosition"), "0")
                    + &field(&catalog, &format!("{signal}/ComBitSize"), "8")
                    + &field(&catalog, &format!("{signal}/ComSignalType"), "UINT8")
                    + &field(
                        &catalog,
                        &format!("{signal}/ComSignalEndianness"),
                        "LITTLE_ENDIAN",
                    );
                container(&signal, name, &values, "")
            })
            .collect();
        let refs: String = names.iter().map(|name| format!(
            "<ECUC-REFERENCE-VALUE><DEFINITION-REF DEST=\"ECUC-REFERENCE-DEF\">{pdu}/ComIPduSignalRef</DEFINITION-REF><VALUE-REF DEST=\"ECUC-CONTAINER-VALUE\">/Project/Com/Config/{name}</VALUE-REF></ECUC-REFERENCE-VALUE>")).collect();
        let pdu_value = container(&pdu, "Pdu", "", "").replace(
            "</PARAMETER-VALUES>",
            &format!("</PARAMETER-VALUES><REFERENCE-VALUES>{refs}</REFERENCE-VALUES>"),
        );
        document(&module(
            &format!("{ROOT}Com"),
            "Com",
            &container(
                &format!("{ROOT}Com/ComConfig"),
                "Config",
                "",
                &(signals + &pdu_value),
            ),
        ))
    };
    let before = create(&["A", "B", "C"]);
    let before = catalog
        .validate_documents(&[(Path::new("com.arxml"), &before)])
        .unwrap();
    let before: Vec<_> = before
        .diagnostics
        .iter()
        .filter(|issue| issue.code == "CAN_SIGNAL_OVERLAP")
        .collect();
    assert_eq!(before.len(), 3);
    let after = create(&["A", "C"]);
    let after = catalog
        .validate_documents(&[(Path::new("com.arxml"), &after)])
        .unwrap();
    let after: Vec<_> = after
        .diagnostics
        .iter()
        .filter(|issue| issue.code == "CAN_SIGNAL_OVERLAP")
        .collect();
    assert_eq!(after.len(), 1);
    let witness = after[0].witness.as_ref().unwrap();
    let previous = before
        .iter()
        .find(|issue| {
            let subjects = &issue.witness.as_ref().unwrap().subjects;
            subjects.contains(&"/Project/Com/Config/A".to_string())
                && subjects.contains(&"/Project/Com/Config/C".to_string())
        })
        .unwrap()
        .witness
        .as_ref()
        .unwrap();
    assert_eq!(witness.counterexample, previous.counterexample);
    assert!(matches!(&witness.counterexample,
        autosar_config_core::LocalizedText::Raw(evidence) if evidence.contains("[0, 1, 2, 3, 4, 5, 6, 7]")
    ));
}

#[test]
fn split_simple_literals_use_complete_text_and_keep_source_witness_ranges() {
    let catalog = DefinitionCatalog::builtin().unwrap();
    let signal = format!("{ROOT}Com/ComConfig/ComSignal");
    let pdu = format!("{ROOT}Com/ComConfig/ComIPdu");
    let fields = field(&catalog, &format!("{signal}/ComBitPosition"), "0")
        + &field(
            &catalog,
            &format!("{signal}/ComBitSize"),
            "6<!--do not truncate-->5",
        )
        + &field(
            &catalog,
            &format!("{signal}/ComSignalEndianness"),
            "LITTLE_ENDIAN",
        );
    let value = container(&signal, "A<!--identity-->B", &fields, "");
    let reference = format!(
        "<ECUC-REFERENCE-VALUE><DEFINITION-REF DEST=\"ECUC-REFERENCE-DEF\">{pdu}/ComIPduSignalRef</DEFINITION-REF><VALUE-REF DEST=\"ECUC-CONTAINER-VALUE\">/Project/Com/Config/A<!--identity-->B</VALUE-REF></ECUC-REFERENCE-VALUE>"
    );
    let pdu_value = container(&pdu, "Pdu", "", "").replace(
        "</PARAMETER-VALUES>",
        &format!("</PARAMETER-VALUES><REFERENCE-VALUES>{reference}</REFERENCE-VALUES>"),
    );
    let source = document(&module(
        &format!("{ROOT}Com"),
        "Com",
        &container(
            &format!("{ROOT}Com/ComConfig"),
            "Config",
            "",
            &(value + &pdu_value),
        ),
    ));
    let result = catalog
        .validate_documents(&[(Path::new("split.arxml"), &source)])
        .unwrap();
    let issue = result
        .diagnostics
        .iter()
        .find(|issue| {
            issue.code == "VALUE_RANGE"
                && matches!(
                    &issue.witness.as_ref().unwrap().constraint,
                    autosar_config_core::LocalizedText::Raw(metadata) if metadata.contains("ComBitSize")
                )
        })
        .unwrap();
    assert_eq!(issue.path.as_deref(), Some("/Project/Com/Config/AB"));
    let parsed = roxmltree::Document::parse(&source).unwrap();
    let entry = parsed
        .descendants()
        .find(|node| {
            node.children().any(|child| {
                child.has_tag_name("VALUE")
                    && child
                        .children()
                        .any(|text| text.is_text() && text.text() == Some("6"))
            })
        })
        .unwrap();
    let range = entry.range();
    assert!(
        issue
            .witness
            .as_ref()
            .unwrap()
            .subjects
            .contains(&format!("entry-range:{}:{}", range.start, range.end))
    );
    assert_eq!(
        issue.witness.as_ref().unwrap().counterexample,
        autosar_config_core::LocalizedText::Raw("65".into())
    );
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|issue| issue.code.starts_with("REFERENCE_"))
    );
}

#[test]
fn extension_split_name_bound_and_default_are_not_truncated() {
    let temp = Directory::new();
    let path = extension(&temp.0.join("source"), "split-vendor", "Vendor");
    let member = path.parent().unwrap().join("definitions.arxml");
    let original = fs::read_to_string(&member).unwrap();
    let updated = original
        .replace(
            "<SHORT-NAME>Limit</SHORT-NAME>",
            "<SHORT-NAME>Li<!--name-->mit</SHORT-NAME>",
        )
        .replace(
            "<MAX>18446744073709551615</MAX>",
            "<MAX>1<!--bound-->0</MAX>",
        )
        .replace(
            "<DEFAULT-VALUE>9007199254740993</DEFAULT-VALUE>",
            "<DEFAULT-VALUE>1<!--default-->0</DEFAULT-VALUE>",
        );
    fs::write(&member, &updated).unwrap();
    let mut inventory: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    inventory["files"][0]["sha256"] = format!("{:x}", Sha256::digest(updated.as_bytes())).into();
    fs::write(&path, serde_json::to_vec(&inventory).unwrap()).unwrap();
    let mut catalog = DefinitionCatalog::builtin().unwrap();
    catalog
        .accept_extension(&path, &temp.0.join("cache"))
        .unwrap();
    let definition = catalog.get("/Project/Vendor/Settings/Limit").unwrap();
    assert_eq!(definition.maximum.as_deref(), Some("10"));
    assert_eq!(definition.default_value.as_ref().unwrap().lexeme, "10");
    assert!(
        catalog
            .validate_value(
                &definition.definition_id,
                &TypedValue {
                    kind: ValueKind::Integer,
                    lexeme: "11".into()
                }
            )
            .is_err()
    );
}

#[test]
fn standard_documents_accept_definition_valid_sc2_and_float_baud_without_target_policy_leak() {
    let temp = Directory::new();
    let preview = autosar_config_core::Workspace::preview_project_creation(
        &temp.0.join("project"),
        "DefinitionPolicy",
        "standard-ecu-v1",
    )
    .unwrap();
    let catalog = DefinitionCatalog::builtin().unwrap();
    let validate = |files: &[autosar_config_core::arxml::ProjectFilePreview]| {
        let documents: Vec<_> = files
            .iter()
            .filter(|file| file.path.ends_with(".arxml"))
            .map(|file| (Path::new(&file.path), file.contents.as_str()))
            .collect();
        catalog.validate_documents(&documents).unwrap()
    };
    let baseline = validate(&preview.files);
    assert_eq!(
        baseline.status,
        ValidationStatus::Unsupported,
        "{:?}",
        baseline.diagnostics
    );
    for (parameter, lexeme) in [
        ("OsScalabilityClass", "SC2"),
        ("CanControllerBaudRate", "500.0"),
    ] {
        let mut files = preview.files.clone();
        let source = files
            .iter_mut()
            .find(|file| file.path == "ecuc.arxml")
            .unwrap();
        let document = roxmltree::Document::parse(&source.contents).unwrap();
        let entry = document
            .descendants()
            .find(|node| {
                node.is_element()
                    && node.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF")
                            && child.text().is_some_and(|definition| {
                                definition.rsplit('/').next() == Some(parameter)
                            })
                    })
            })
            .unwrap();
        let value = entry
            .children()
            .find(|child| child.has_tag_name("VALUE"))
            .unwrap();
        let range = value.range();
        source
            .contents
            .replace_range(range, &format!("<VALUE>{lexeme}</VALUE>"));
        let changed = validate(&files);
        assert!(
            changed.diagnostics.is_empty(),
            "{parameter}: {:?}",
            changed.diagnostics
        );
        assert_eq!(changed.status, ValidationStatus::Unsupported);
        assert!(changed.coverage.iter().any(|rule| rule.rule_id
            == "native.definition.legacy-dcm-mode-dependency"
            && !rule.supported));
    }
}
