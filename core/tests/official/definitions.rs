//! Built-in definitions compared with the official R24-11 ECUC definitions.
use autosar_config_core::definitions::DefinitionCatalog;
use autosar_config_core::project_model::ValueKind;
use sha2::{Digest, Sha256};
use std::fs;
#[cfg(feature = "official-oracles")]
use std::io::Read;
use std::path::Path;

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
