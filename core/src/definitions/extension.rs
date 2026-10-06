use super::{DefinitionCatalog, NS, RELEASE, descriptor, digest};
use crate::model::Severity;
use crate::project_model::{
    ConfigurationDiagnostic, ExtensionDefinitionIdentity, TypedValue, ValueKind,
};
use roxmltree::{Document, Node};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Inventory {
    format_version: u32,
    catalog_id: String,
    release: String,
    version: String,
    files: Vec<Member>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Member {
    path: String,
    sha256: String,
}

struct Loaded {
    identity: ExtensionDefinitionIdentity,
    metadata: DefinitionCatalog,
    bytes: BTreeMap<String, Vec<u8>>,
}

fn segment(value: &str) -> bool {
    if value.is_empty() || value.len() > 128 || value.ends_with('.') {
        return false;
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"_-.".contains(&byte))
    {
        return false;
    }
    let base = value.split('.').next().unwrap_or("").to_ascii_uppercase();
    !matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$")
        && !(base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
        && value != "."
        && value != ".."
}

fn relative(value: &str) -> bool {
    value.len() <= 4096
        && value.split('/').count() <= 64
        && !value.contains('\\')
        && !value.contains(':')
        && !Path::new(value).is_absolute()
        && value.split('/').all(segment)
        && Path::new(value)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

fn sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn no_link(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors().filter(|path| !path.as_os_str().is_empty()) {
        let metadata = fs::symlink_metadata(ancestor)
            .map_err(|error| format!("{}: {error}", ancestor.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "Links are forbidden in definition catalogs: {}",
                ancestor.display()
            ));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(format!(
                    "Reparse points are forbidden: {}",
                    ancestor.display()
                ));
            }
        }
    }
    Ok(())
}

fn bounded_read(path: &Path, maximum: u64) -> Result<Vec<u8>, String> {
    no_link(path)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    let file = options.open(path).map_err(|error| error.to_string())?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > maximum {
        return Err(format!(
            "Catalog member is not a bounded regular file: {}",
            path.display()
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    #[cfg(feature = "verification-metrics")]
    crate::verification::before(crate::verification::Phase::DefinitionRead);
    file.take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > maximum {
        return Err(format!(
            "Catalog member exceeded its size limit: {}",
            path.display()
        ));
    }
    no_link(path)?;
    Ok(bytes)
}

fn collect(
    root: &Path,
    directory: &Path,
    expected: &BTreeSet<String>,
    files: &mut BTreeSet<String>,
) -> Result<(), String> {
    no_link(directory)?;
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        no_link(&path)?;
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata.is_dir() {
            let relative = path.strip_prefix(root).map_err(|error| error.to_string())?;
            if relative.components().count() > 64 {
                return Err("Catalog directory nesting exceeds the safety limit".into());
            }
            let mut prefix = relative
                .to_str()
                .ok_or("Catalog paths must be UTF-8")?
                .replace('\\', "/");
            prefix.push('/');
            let member = expected
                .range::<str, _>((
                    std::ops::Bound::Included(prefix.as_str()),
                    std::ops::Bound::Unbounded,
                ))
                .next();
            if !member.is_some_and(|member| member.starts_with(&prefix)) {
                return Err("Catalog contains a directory outside its complete inventory".into());
            }
            collect(root, &path, expected, files)?;
        } else if metadata.is_file() {
            let name = path
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .to_str()
                .ok_or("Catalog paths must be UTF-8")?
                .replace('\\', "/");
            if !relative(&name) || !files.insert(name) || files.len() > 1025 {
                return Err("Unsafe, duplicate, or excessive catalog membership".into());
            }
        } else {
            return Err("Catalog contains a nonregular payload".into());
        }
    }
    Ok(())
}

fn load(path: &Path) -> Result<Loaded, String> {
    if path.file_name().and_then(|name| name.to_str()) != Some("catalog.json") {
        return Err("Select catalog.json, not an individual definition member".into());
    }
    let root = path.parent().ok_or("Catalog has no parent directory")?;
    let raw = bounded_read(path, 1024 * 1024)?;
    let inventory: Inventory =
        serde_json::from_slice(&raw).map_err(|error| format!("Invalid catalog.json: {error}"))?;
    if inventory.format_version != 1
        || inventory.release != RELEASE
        || !segment(&inventory.catalog_id)
        || inventory.version.is_empty()
        || inventory.version.len() > 128
        || inventory.files.is_empty()
        || inventory.files.len() > 1024
    {
        return Err("Catalog requires formatVersion 1, CP R24-11, a portable catalogId, version and members".into());
    }
    let mut expected = BTreeSet::from(["catalog.json".to_string()]);
    let mut folded = BTreeSet::from(["catalog.json".to_string()]);
    let mut previous: Option<&str> = None;
    for member in &inventory.files {
        if !relative(&member.path)
            || !sha(&member.sha256)
            || !member.path.ends_with(".arxml")
            || !expected.insert(member.path.clone())
            || !folded.insert(member.path.to_ascii_lowercase())
            || previous.is_some_and(|path| path >= member.path.as_str())
        {
            return Err(
                "Inventory members must be unique sorted safe ARXML paths with lowercase SHA-256"
                    .into(),
            );
        }
        previous = Some(&member.path);
    }
    let mut actual = BTreeSet::new();
    collect(root, root, &expected, &mut actual)?;
    if actual != expected {
        return Err("Catalog payload differs from its complete inventory".into());
    }
    let identity = ExtensionDefinitionIdentity {
        catalog_id: inventory.catalog_id,
        release: inventory.release,
        version: inventory.version,
        sha256: digest(&raw),
    };
    let mut metadata = DefinitionCatalog {
        entries: Arc::new(BTreeMap::new()),
        targets: Arc::new(BTreeMap::new()),
        owners: BTreeMap::new(),
        extensions: BTreeMap::new(),
        missing: BTreeSet::new(),
    };
    let mut bytes = BTreeMap::from([("catalog.json".into(), raw)]);
    let mut total = 0usize;
    for member in inventory.files {
        let raw = bounded_read(&root.join(&member.path), 50 * 1024 * 1024)?;
        total = total
            .checked_add(raw.len())
            .ok_or("Catalog size overflow")?;
        if total > 512 * 1024 * 1024 || digest(&raw) != member.sha256 {
            return Err(format!(
                "Definition member digest or size mismatch: {}",
                member.path
            ));
        }
        let text = std::str::from_utf8(&raw).map_err(|error| error.to_string())?;
        parse(&mut metadata, text, &identity)?;
        bytes.insert(member.path, raw);
    }
    if !metadata
        .entries
        .values()
        .any(|entry| entry.element_kind == "ECUC-MODULE-DEF")
    {
        return Err("Catalog does not contain a native module definition".into());
    }
    metadata.check_metadata()?;
    // Every native reference destination must belong to this catalog or the
    // trusted builtin catalog; it must never acquire meaning from another project.
    let builtin = DefinitionCatalog::builtin()?;
    for (id, targets) in metadata.targets.iter() {
        if !metadata.get(id).is_some_and(|entry| entry.writable) {
            continue;
        }
        for target in targets {
            let entry = metadata
                .get(target)
                .or_else(|| builtin.get(target))
                .ok_or_else(|| format!("Definition target is unavailable: {target}"))?;
            if entry.kind.is_some()
                || !matches!(
                    entry.element_kind.as_str(),
                    "ECUC-PARAM-CONF-CONTAINER-DEF" | "ECUC-CHOICE-CONTAINER-DEF"
                )
            {
                return Err(format!(
                    "Ordinary reference destination is not a container definition: {target}"
                ));
            }
        }
    }
    Ok(Loaded {
        identity,
        metadata,
        bytes,
    })
}

fn install(catalog: &mut DefinitionCatalog, loaded: &Loaded) -> Result<(), String> {
    if let Some(old) = catalog.extensions.get(&loaded.identity.catalog_id) {
        if old != &loaded.identity {
            return Err(
                "An accepted catalogId cannot be silently replaced by another identity".into(),
            );
        }
    }
    let builtin = DefinitionCatalog::builtin()?;
    for id in loaded.metadata.entries.keys() {
        if builtin
            .definitions()
            .filter(|definition| definition.element_kind == "ECUC-MODULE-DEF")
            .any(|module| {
                id == &module.definition_id
                    || id
                        .strip_prefix(&module.definition_id)
                        .is_some_and(|tail| tail.starts_with('/'))
            })
        {
            return Err(format!(
                "Extensions cannot replace a builtin module namespace: {id}"
            ));
        }
        if catalog.entries.contains_key(id)
            && catalog.owners.get(id) != Some(&loaded.identity.catalog_id)
        {
            return Err(format!(
                "Extension conflicts with an existing definition: {id}"
            ));
        }
    }
    for (id, entry) in loaded.metadata.entries.iter() {
        catalog.entries_mut().insert(id.clone(), entry.clone());
        catalog
            .owners
            .insert(id.clone(), loaded.identity.catalog_id.clone());
    }
    for (id, targets) in loaded.metadata.targets.iter() {
        catalog.targets_mut().insert(id.clone(), targets.clone());
    }
    catalog
        .extensions
        .insert(loaded.identity.catalog_id.clone(), loaded.identity.clone());
    catalog.missing.remove(&loaded.identity.catalog_id);
    Ok(())
}

struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn accept(
    catalog: &mut DefinitionCatalog,
    path: &Path,
    root: &Path,
) -> Result<ExtensionDefinitionIdentity, String> {
    let loaded = load(path)?;
    let mut prospective = catalog.clone();
    install(&mut prospective, &loaded)?;
    if let Some(existing) = root
        .ancestors()
        .find(|path| !path.as_os_str().is_empty() && fs::symlink_metadata(path).is_ok())
    {
        no_link(existing)?;
    }
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    no_link(root)?;
    let destination = root.join(&loaded.identity.sha256);
    if fs::symlink_metadata(&destination).is_ok() {
        let cached = load(&destination.join("catalog.json"))?;
        if cached.identity != loaded.identity {
            return Err("Existing cache does not match the accepted exact identity".into());
        }
    } else {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let stage = root.join(format!(
            ".staging-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&stage).map_err(|error| error.to_string())?;
        let guard = Staging(stage.clone());
        for (relative, bytes) in &loaded.bytes {
            let path = stage.join(relative);
            fs::create_dir_all(path.parent().ok_or("Invalid staged member")?)
                .map_err(|error| error.to_string())?;
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&path)
                .map_err(|error| error.to_string())?;
            file.write_all(bytes)
                .and_then(|_| file.sync_all())
                .map_err(|error| error.to_string())?;
        }
        let verified = load(&stage.join("catalog.json"))?;
        if verified.identity != loaded.identity {
            return Err("Staged cache identity changed".into());
        }
        // No overwrite: a competing publisher must produce the same verified cache.
        if let Err(error) = fs::rename(&stage, &destination) {
            if !destination.exists()
                || load(&destination.join("catalog.json"))?.identity != loaded.identity
            {
                return Err(format!(
                    "Cannot publish immutable definition cache: {error}"
                ));
            }
        }
        drop(guard);
    }
    *catalog = prospective;
    Ok(loaded.identity)
}

pub(super) fn restore(
    catalog: &mut DefinitionCatalog,
    identities: &[ExtensionDefinitionIdentity],
    root: &Path,
) -> Result<Vec<ConfigurationDiagnostic>, String> {
    let mut prospective = DefinitionCatalog::builtin()?;
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();
    for identity in identities {
        if identity.release != RELEASE
            || !sha(&identity.sha256)
            || !segment(&identity.catalog_id)
            || identity.version.is_empty()
            || !seen.insert(&identity.catalog_id)
        {
            return Err("Invalid or duplicate required extension identity".into());
        }
        let result = load(&root.join(&identity.sha256).join("catalog.json")).and_then(|loaded| {
            if loaded.identity != *identity {
                return Err("Cached identity differs from the exact project requirement".into());
            }
            install(&mut prospective, &loaded)
        });
        if let Err(error) = result {
            prospective
                .extensions
                .insert(identity.catalog_id.clone(), identity.clone());
            prospective.missing.insert(identity.catalog_id.clone());
            // Preserve known consumers when restoring an existing session. A fresh
            // session still reports each actual unknown definition as unsupported.
            for (id, owner) in &catalog.owners {
                if owner == &identity.catalog_id {
                    prospective.owners.insert(id.clone(), owner.clone());
                }
            }
            diagnostics.push(super::validation::diagnostic(
                None,
                None,
                "EXTENSION_MISSING",
                Severity::Warning,
                format!(
                    "Exact extension {} is unavailable: {error}",
                    identity.catalog_id
                ),
                "Explicitly import the exact catalog and all inventory members.",
                &identity.catalog_id,
                &identity.sha256,
            ));
        }
    }
    *catalog = prospective;
    Ok(diagnostics)
}

fn child<'a, 'input>(node: Node<'a, 'input>, tag: &str) -> Option<std::borrow::Cow<'a, str>> {
    node.children()
        .find(|n| {
            n.is_element() && n.tag_name().namespace() == Some(NS) && n.tag_name().name() == tag
        })
        .map(super::xml_text_trimmed)
}

fn parse(
    catalog: &mut DefinitionCatalog,
    text: &str,
    identity: &ExtensionDefinitionIdentity,
) -> Result<(), String> {
    if text.contains("<!DOCTYPE") || text.contains("<!ENTITY") {
        return Err("DTD and entity declarations are forbidden".into());
    }
    let document = Document::parse(text).map_err(|error| error.to_string())?;
    if document.root_element().tag_name().name() != "AUTOSAR"
        || document.root_element().tag_name().namespace() != Some(NS)
    {
        return Err("Definition document must be AUTOSAR CP XML".into());
    }
    if document.root_element().attributes().any(|attribute| {
        attribute.name() == "schemaLocation"
            && attribute
                .value()
                .split_whitespace()
                .any(|value| value.contains("AUTOSAR_") && !value.ends_with("AUTOSAR_00053.xsd"))
    }) {
        return Err("Extension XML declares a different AUTOSAR schema release".into());
    }
    for node in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().namespace() == Some(NS))
    {
        let tag = node.tag_name().name();
        let kind = match tag {
            "ECUC-INTEGER-PARAM-DEF" => Some(ValueKind::Integer),
            "ECUC-FLOAT-PARAM-DEF" => Some(ValueKind::Float),
            "ECUC-BOOLEAN-PARAM-DEF" => Some(ValueKind::Boolean),
            "ECUC-ENUMERATION-PARAM-DEF" => Some(ValueKind::Enumeration),
            "ECUC-STRING-PARAM-DEF" => Some(ValueKind::String),
            "ECUC-FUNCTION-NAME-DEF" => Some(ValueKind::FunctionName),
            "ECUC-REFERENCE-DEF"
            | "ECUC-CHOICE-REFERENCE-DEF"
            | "ECUC-FOREIGN-REFERENCE-DEF"
            | "ECUC-INSTANCE-REFERENCE-DEF" => Some(ValueKind::Reference),
            "ECUC-MODULE-DEF" | "ECUC-PARAM-CONF-CONTAINER-DEF" | "ECUC-CHOICE-CONTAINER-DEF" => {
                None
            }
            _ => continue,
        };
        if tag != "ECUC-MODULE-DEF"
            && !node
                .ancestors()
                .skip(1)
                .any(|owner| owner.tag_name().name() == "ECUC-MODULE-DEF")
        {
            return Err("Native definitions must belong to an explicit module definition".into());
        }
        let mut names: Vec<_> = node
            .ancestors()
            .filter_map(|node| child(node, "SHORT-NAME"))
            .collect();
        if names.is_empty()
            || names
                .iter()
                .any(|name| !super::validation::identifier(name))
        {
            return Err("Definition SHORT-NAME must be a portable native identifier".into());
        }
        names.reverse();
        let id = format!("/{}", names.join("/"));
        let mut entry = descriptor(id, tag, kind);
        entry.lower_multiplicity = child(node, "LOWER-MULTIPLICITY")
            .as_deref()
            .unwrap_or("0")
            .parse()
            .map_err(|_| "Invalid lower multiplicity")?;
        entry.upper_multiplicity =
            if child(node, "UPPER-MULTIPLICITY-INFINITE").as_deref() == Some("true") {
                None
            } else {
                Some(
                    child(node, "UPPER-MULTIPLICITY")
                        .as_deref()
                        .unwrap_or("1")
                        .parse()
                        .map_err(|_| "Invalid upper multiplicity")?,
                )
            };
        entry.minimum = child(node, "MIN").map(std::borrow::Cow::into_owned);
        entry.maximum = child(node, "MAX")
            .filter(|value| value.as_ref() != "INF")
            .map(std::borrow::Cow::into_owned);
        if tag == "ECUC-ENUMERATION-PARAM-DEF" {
            entry.enumeration = node
                .descendants()
                .filter(|n| n.tag_name().name() == "ECUC-ENUMERATION-LITERAL-DEF")
                .filter_map(|n| child(n, "SHORT-NAME").map(std::borrow::Cow::into_owned))
                .collect();
            if entry.enumeration.is_empty()
                || entry.enumeration.iter().collect::<BTreeSet<_>>().len()
                    != entry.enumeration.len()
            {
                return Err("Enumeration requires unique literals".into());
            }
        }
        entry.unit = child(node, "UNIT-REF").map(std::borrow::Cow::into_owned);
        if let Some(default) = node.children().find(|child| {
            child.is_element()
                && child.tag_name().namespace() == Some(NS)
                && child.tag_name().name() == "DEFAULT-VALUE"
        }) {
            if default.children().any(|child| child.is_element()) {
                entry.writable = false;
                entry.reason =
                    Some("Structured default expression semantics are unsupported".into());
            } else {
                entry.default_value = Some(TypedValue {
                    kind: kind.ok_or("A container cannot have a parameter default")?,
                    lexeme: super::xml_text(default).into_owned(),
                });
                entry.default_origin = Some(format!(
                    "extension:{}:{}:{}",
                    identity.catalog_id, identity.version, identity.sha256
                ));
            }
        }
        let mut targets = Vec::new();
        if kind == Some(ValueKind::Reference) {
            targets = node
                .descendants()
                .filter(|n| n.tag_name().name() == "DESTINATION-REF")
                .map(|node| super::xml_text_trimmed(node).into_owned())
                .collect();
            entry.reference_destinations = if tag == "ECUC-FOREIGN-REFERENCE-DEF" {
                child(node, "DESTINATION-TYPE")
                    .map(|kind| vec![kind.into_owned()])
                    .unwrap_or_default()
            } else {
                vec!["ECUC-CONTAINER-VALUE".into()]
            };
            if entry.reference_destinations.is_empty()
                || (tag != "ECUC-FOREIGN-REFERENCE-DEF" && targets.is_empty())
            {
                entry.writable = false;
                entry.reason = Some("Reference destination semantics are unavailable".into());
            }
        }
        let opaque = node.ancestors().any(|ancestor| {
            ancestor.children().any(|n| {
                matches!(
                    n.tag_name().name(),
                    "VARIATION-POINT"
                        | "LOWER-MULTIPLICITY-CONDITION"
                        | "UPPER-MULTIPLICITY-CONDITION"
                )
            })
        });
        if opaque || tag == "ECUC-INSTANCE-REFERENCE-DEF" {
            entry.writable = false;
            entry.reason = Some(
                if tag == "ECUC-INSTANCE-REFERENCE-DEF" {
                    "Instance-reference context semantics are unsupported"
                } else {
                    "Conditional or variation-dependent definition is unsupported"
                }
                .into(),
            );
        } else if node.children().any(|child| {
            matches!(
                child.tag_name().name(),
                "REGULAR-EXPRESSION" | "MIN-LENGTH" | "MAX-LENGTH" | "DEFAULT-VALUE-EXPR"
            )
        }) {
            entry.writable = false;
            entry.reason =
                Some("Additional expression or textual constraints are unsupported".into());
        }
        catalog.insert(entry, targets)?;
    }
    Ok(())
}
