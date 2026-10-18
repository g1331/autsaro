//! Built-in definitions compared with the fixed official R24-11 sources that define
//! them: ECUC definitions with the MOD, SWC/System template vocabulary with the XSD.
use autosar_config_core::definitions::DefinitionCatalog;
use autosar_config_core::project_model::{DefinitionDescriptor, ValueKind};
use sha2::{Digest, Sha256};
use std::fs;
#[cfg(feature = "official-oracles")]
use std::io::Read;
use std::path::Path;

/// ECUC definitions are absolute definition paths; template vocabulary is keyed by
/// metaclass tag (`KIND` or `KIND#FIELD`) and is not part of the ECUC MOD.
fn is_ecuc(definition_id: &str) -> bool {
    definition_id.starts_with('/')
}

#[cfg(feature = "official-oracles")]
#[test]
fn fixed_r24_11_mod_agrees_on_ecuc_integer_precision_enum_and_default() {
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
        hex::encode(Sha256::digest(xml.as_bytes())),
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
    let mut defaults = 0;
    let mut without_defaults = 0;
    for expected in catalog
        .definitions()
        .filter(|entry| is_ecuc(&entry.definition_id))
    {
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
        if expected.kind.is_some() {
            if text("DEFAULT-VALUE").is_some() {
                defaults += 1;
                compare(
                    &mut failures,
                    id,
                    "default origin",
                    Some("builtin:product-ecuc-r24-11"),
                    expected.default_origin.as_deref(),
                );
                compare(
                    &mut failures,
                    id,
                    "default kind",
                    expected.kind,
                    expected.default_value.as_ref().map(|value| value.kind),
                );
            } else {
                without_defaults += 1;
                compare(
                    &mut failures,
                    id,
                    "absent default origin",
                    None,
                    expected.default_origin.as_deref(),
                );
            }
        }
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
    assert_eq!(
        (defaults, without_defaults),
        (140, 360),
        "all 500 delivered parameters must be compared, including absent defaults"
    );
    eprintln!(
        "R24-11 default comparison: {defaults} declared, {without_defaults} absent; all 500 parameters compared"
    );
    assert!(
        failures.is_empty(),
        "Fixed metadata oracle mismatches:\n{}",
        failures.join("\n")
    );
}

#[cfg(feature = "official-oracles")]
type XsdNode<'a, 'input> = roxmltree::Node<'a, 'input>;

/// Named declarations of the fixed R24-11 XSD. AUTOSAR expresses inheritance with
/// `xsd:group` references only, so content models are resolved through groups.
#[cfg(feature = "official-oracles")]
struct Xsd<'a, 'input> {
    types: std::collections::HashMap<&'a str, XsdNode<'a, 'input>>,
    groups: std::collections::HashMap<&'a str, XsdNode<'a, 'input>>,
    simple: std::collections::HashMap<&'a str, XsdNode<'a, 'input>>,
}

#[cfg(feature = "official-oracles")]
impl<'a, 'input> Xsd<'a, 'input> {
    fn new(document: &'a roxmltree::Document<'input>) -> Self {
        let named = |tag| {
            document
                .root_element()
                .children()
                .filter(move |node| node.has_tag_name(tag))
                .filter_map(|node| Some((node.attribute("name")?, node)))
                .collect()
        };
        Self {
            types: named("complexType"),
            groups: named("group"),
            simple: named("simpleType"),
        }
    }

    /// Content model of an element: its named or anonymous complex type.
    fn content(&self, element: XsdNode<'a, 'input>) -> Option<XsdNode<'a, 'input>> {
        match element.attribute("type") {
            Some(name) => self.types.get(local(name)).copied(),
            None => element
                .children()
                .find(|child| child.has_tag_name("complexType")),
        }
    }

    /// Element declarations directly allowed by a content model.
    fn elements(&self, content: XsdNode<'a, 'input>) -> Vec<XsdNode<'a, 'input>> {
        let mut found = Vec::new();
        let mut pending = vec![content];
        while let Some(node) = pending.pop() {
            for child in node.children().filter(XsdNode::is_element) {
                match (child.tag_name().name(), child.attribute("ref")) {
                    ("element", _) => found.push(child),
                    ("group", Some(name)) => pending.push(self.groups[local(name)]),
                    ("sequence" | "choice" | "all", _) => pending.push(child),
                    _ => {}
                }
            }
        }
        found
    }

    fn child(&self, content: XsdNode<'a, 'input>, name: &str) -> Option<XsdNode<'a, 'input>> {
        self.elements(content)
            .into_iter()
            .find(|element| element.attribute("name") == Some(name))
    }

    /// Every `tag` declaration below `owner` that is not inside another identifiable
    /// object, mirroring how the projection assigns undecorated fields to their owner.
    fn descendants(&self, owner: XsdNode<'a, 'input>, tag: &str) -> Vec<XsdNode<'a, 'input>> {
        let mut found = Vec::new();
        let mut visited = std::collections::HashSet::from([owner.id()]);
        let mut pending = vec![owner];
        while let Some(content) = pending.pop() {
            for element in self.elements(content) {
                if element.attribute("name") == Some(tag) {
                    found.push(element);
                } else if let Some(next) = self.content(element)
                    && self.child(next, "SHORT-NAME").is_none()
                    && visited.insert(next.id())
                {
                    pending.push(next);
                }
            }
        }
        found
    }

    /// Enumeration literals of a simple type, or of a complex type's simple content.
    fn literals(&self, content: XsdNode<'a, 'input>) -> Vec<&'a str> {
        let base = content
            .descendants()
            .find(|node| node.has_tag_name("extension"))
            .and_then(|extension| extension.attribute("base"))
            .and_then(|name| self.simple.get(local(name)).copied())
            .unwrap_or(content);
        base.descendants()
            .filter(|node| node.has_tag_name("enumeration"))
            .filter_map(|node| node.attribute("value"))
            .collect()
    }

    /// Whether one XSD leaf declaration can carry the authored field: the product may
    /// restrict an official type (an integer inside a numerical value) but not widen it.
    fn admits(&self, leaf: XsdNode<'a, 'input>, expected: &DefinitionDescriptor) -> bool {
        let content = self.content(leaf);
        let literals = |content: Option<XsdNode<'a, 'input>>| {
            content
                .map(|content| self.literals(content))
                .unwrap_or_default()
        };
        let subset = |authored: &[String], official: &[&str]| {
            !official.is_empty()
                && authored
                    .iter()
                    .all(|item| official.contains(&item.as_str()))
        };
        let type_name = leaf.attribute("type").map(local);
        match expected.kind.expect("template field kind") {
            ValueKind::Reference => {
                let destinations = content
                    .and_then(|content| {
                        content.descendants().find(|node| {
                            node.has_tag_name("attribute") && node.attribute("name") == Some("DEST")
                        })
                    })
                    .and_then(|attribute| attribute.attribute("type"))
                    .and_then(|name| self.simple.get(local(name)).copied());
                subset(&expected.reference_destinations, &literals(destinations))
            }
            // Category values are defined by the template specifications; the XSD only
            // fixes their lexical form, so it can confirm the field but not the literals.
            ValueKind::Enumeration if type_name == Some("CATEGORY-STRING") => true,
            ValueKind::Enumeration => subset(&expected.enumeration, &literals(content)),
            ValueKind::Boolean => type_name == Some("BOOLEAN"),
            ValueKind::Float => type_name == Some("TIME-VALUE"),
            ValueKind::Integer => matches!(
                type_name,
                Some(
                    "POSITIVE-INTEGER"
                        | "INTEGER-VALUE-VARIATION-POINT"
                        | "NUMERICAL-VALUE-VARIATION-POINT"
                )
            ),
            ValueKind::FunctionName => type_name == Some("C-IDENTIFIER"),
            ValueKind::String => type_name == Some("NMTOKEN-STRING"),
        }
    }
}

#[cfg(feature = "official-oracles")]
fn local(name: &str) -> &str {
    name.strip_prefix("AR:").unwrap_or(name)
}

#[cfg(feature = "official-oracles")]
#[test]
fn fixed_r24_11_xsd_declares_template_elements_fields_and_references() {
    let archive_path = autosar_config_core::schema::schema_archive(
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap(),
    );
    let mut archive =
        zip::ZipArchive::new(fs::File::open(&archive_path).expect("fixed local R24-11 XSD oracle"))
            .unwrap();
    let mut xml = String::new();
    archive
        .by_name("AUTOSAR_00053.xsd")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(xml.as_bytes())),
        "43161400d3eaa55db2ab416289c933aba7977e61d3da8678fad1761caaf61457"
    );
    let document = roxmltree::Document::parse(&xml).unwrap();
    let xsd = Xsd::new(&document);
    let catalog = DefinitionCatalog::builtin().unwrap();
    let mut failures = Vec::new();
    let mut compared = 0;
    for expected in catalog
        .definitions()
        .filter(|entry| !is_ecuc(&entry.definition_id))
    {
        compared += 1;
        let id = &expected.definition_id;
        let Some((owner, _)) = id.split_once('#') else {
            // A metaclass is a concrete XSD type, declared as an element of that type
            // inside a wrapper of each parent the catalog nests it under.
            if xsd
                .types
                .get(id.as_str())
                .is_none_or(|content| content.attribute("abstract") == Some("true"))
            {
                failures.push(format!("{id}: no concrete XSD complex type"));
            }
            let parents = std::iter::once("AR-PACKAGE")
                .chain(
                    catalog
                        .definitions()
                        .map(|entry| entry.definition_id.as_str()),
                )
                .filter(|parent| {
                    !is_ecuc(parent)
                        && catalog
                            .children(parent)
                            .iter()
                            .any(|child| &child.definition_id == id)
                });
            for parent in parents {
                let held = xsd.types.get(parent).is_some_and(|content| {
                    xsd.elements(*content).into_iter().any(|wrapper| {
                        xsd.content(wrapper)
                            .and_then(|wrapped| xsd.child(wrapped, id))
                            .is_some_and(|element| element.attribute("type").map(local) == Some(id))
                    })
                });
                if !held {
                    failures.push(format!("{id}: XSD {parent} holds no {id} element"));
                }
            }
            continue;
        };
        let Some(content) = xsd.types.get(owner).copied() else {
            failures.push(format!("{id}: no XSD complex type {owner}"));
            continue;
        };
        // A decorated field names its exact path; an undecorated one is any descendant.
        let leaves = match expected.element_kind.split_once('/') {
            None => xsd.descendants(content, &expected.element_kind),
            Some(_) => {
                let mut leaf = None;
                let mut scope = Some(content);
                for name in expected.element_kind.split('/') {
                    leaf = scope.and_then(|scope| xsd.child(scope, name));
                    scope = leaf.and_then(|leaf| xsd.content(leaf));
                }
                leaf.into_iter().collect()
            }
        };
        if leaves.is_empty() {
            failures.push(format!(
                "{id}: XSD {owner} has no {}",
                expected.element_kind
            ));
        } else if !leaves.iter().any(|leaf| xsd.admits(*leaf, expected)) {
            failures.push(format!(
                "{id}: no XSD declaration admits {:?} {:?}{:?}",
                expected.kind, expected.enumeration, expected.reference_destinations
            ));
        }
    }
    assert!(compared > 0, "no template definitions were compared");
    assert!(
        failures.is_empty(),
        "Fixed XSD oracle mismatches:\n{}",
        failures.join("\n")
    );
}
