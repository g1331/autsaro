//! Product-owned ECUC metadata and explicitly accepted, immutable extension catalogs.
//! Configuration bytes remain the authority; this catalog is a rebuildable projection.

mod builtin;
mod extension;
mod validation;

use crate::project_model::{
    ConfigurationDiagnostic, DefinitionDescriptor, ExtensionDefinitionIdentity, RuleSetIdentity,
    ScopeValidation, TypedValue, ValueKind,
};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, LazyLock};

pub(crate) const NS: &str = "http://autosar.org/schema/r4.0";
pub(crate) const RELEASE: &str = "R24-11";

#[derive(Clone, Debug)]
pub struct DefinitionCatalog {
    entries: Arc<BTreeMap<String, DefinitionDescriptor>>,
    targets: Arc<BTreeMap<String, Vec<String>>>,
    owners: BTreeMap<String, String>,
    extensions: BTreeMap<String, ExtensionDefinitionIdentity>,
    missing: BTreeSet<String>,
}

impl DefinitionCatalog {
    pub fn builtin() -> Result<Self, String> {
        static BUILTIN: LazyLock<Result<DefinitionCatalog, String>> =
            LazyLock::new(DefinitionCatalog::build_builtin);
        BUILTIN.clone()
    }

    fn build_builtin() -> Result<Self, String> {
        let mut catalog = Self {
            entries: Arc::new(BTreeMap::new()),
            targets: Arc::new(BTreeMap::new()),
            owners: BTreeMap::new(),
            extensions: BTreeMap::new(),
            missing: BTreeSet::new(),
        };
        builtin::populate(&mut catalog)?;
        catalog.check_metadata()?;
        Ok(catalog)
    }

    pub fn get(&self, definition_id: &str) -> Option<&DefinitionDescriptor> {
        self.entries.get(definition_id)
    }

    pub fn children(&self, definition_id: &str) -> Vec<&DefinitionDescriptor> {
        self.entries
            .range::<str, _>((
                std::ops::Bound::Excluded(definition_id),
                std::ops::Bound::Unbounded,
            ))
            .take_while(|(id, _)| {
                id.starts_with(definition_id)
                    && id.as_bytes().get(definition_id.len()) == Some(&b'/')
            })
            .filter_map(|(_, entry)| {
                (parent(&entry.definition_id) == Some(definition_id)).then_some(entry)
            })
            .collect()
    }

    pub fn definitions(&self) -> impl Iterator<Item = &DefinitionDescriptor> {
        self.entries.values()
    }

    /// Includes exact manifest requirements which could not be restored, so saving
    /// an unrelated builtin edit never silently drops a missing extension identity.
    pub fn accepted_extensions(&self) -> Vec<ExtensionDefinitionIdentity> {
        self.extensions.values().cloned().collect()
    }

    pub fn required_extensions(
        &self,
        definition_ids: &[String],
    ) -> Vec<ExtensionDefinitionIdentity> {
        let mut needed = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut pending: Vec<&str> = definition_ids.iter().map(String::as_str).collect();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            if let Some(owner) = self.owners.get(id) {
                needed.insert(owner.as_str());
            }
            if let Some(targets) = self.targets.get(id) {
                pending.extend(targets.iter().map(String::as_str));
            }
            if let Some(ancestor) = parent(id) {
                pending.push(ancestor);
            }
        }
        needed
            .into_iter()
            .filter_map(|id| self.extensions.get(id).cloned())
            .collect()
    }

    pub fn fingerprint(&self, identity: &RuleSetIdentity) -> String {
        let mut hash = Sha256::new();
        hash.update(b"autosar-definition-catalog-v1\0");
        hash.update(serde_json::to_vec(identity).expect("rule identity serialization"));
        for descriptor in self.entries.values() {
            hash.update(serde_json::to_vec(descriptor).expect("definition serialization"));
            hash.update(b"\0");
        }
        hash.update(
            serde_json::to_vec(self.targets.as_ref()).expect("reference target serialization"),
        );
        hash.update(
            serde_json::to_vec(&self.accepted_extensions()).expect("identity serialization"),
        );
        format!("{:x}", hash.finalize())
    }

    /// Deterministic metadata bytes for the trusted rule inventory, independent of
    /// filesystem locations and explicit project extension acceptance.
    pub fn builtin_metadata_sha256() -> Result<String, String> {
        let catalog = Self::builtin()?;
        let bytes = serde_json::to_vec(&(catalog.entries.as_ref(), catalog.targets.as_ref()))
            .map_err(|error| error.to_string())?;
        Ok(digest(&bytes))
    }

    pub fn validate_documents(&self, files: &[(&Path, &str)]) -> Result<ScopeValidation, String> {
        validation::documents(self, files)
    }

    pub fn accept_extension(
        &mut self,
        catalog_path: &Path,
        cache_root: &Path,
    ) -> Result<ExtensionDefinitionIdentity, String> {
        extension::accept(self, catalog_path, cache_root)
    }

    pub fn remove_extension(&mut self, catalog_id: &str) -> Result<(), String> {
        if self.extensions.remove(catalog_id).is_none() {
            return Err(format!("Extension catalog is not accepted: {catalog_id}"));
        }
        let ids: Vec<_> = self
            .owners
            .iter()
            .filter(|(_, owner)| owner.as_str() == catalog_id)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            self.entries_mut().remove(&id);
            self.targets_mut().remove(&id);
            self.owners.remove(&id);
        }
        self.missing.remove(catalog_id);
        Ok(())
    }

    pub fn restore_extensions(
        &mut self,
        identities: &[ExtensionDefinitionIdentity],
        cache_root: &Path,
    ) -> Result<Vec<ConfigurationDiagnostic>, String> {
        extension::restore(self, identities, cache_root)
    }

    /// Validates without numeric wire coercion or changing the original lexeme.
    pub fn validate_value(&self, definition_id: &str, value: &TypedValue) -> Result<(), String> {
        let descriptor = self
            .get(definition_id)
            .ok_or_else(|| format!("Definition is unavailable: {definition_id}"))?;
        if !descriptor.writable {
            return Err(descriptor
                .reason
                .clone()
                .unwrap_or_else(|| "Definition is readonly".into()));
        }
        validation::value(descriptor, value)
    }

    pub fn reference_target_definitions(&self, definition_id: &str) -> &[String] {
        self.targets
            .get(definition_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn entries_mut(&mut self) -> &mut BTreeMap<String, DefinitionDescriptor> {
        Arc::make_mut(&mut self.entries)
    }

    fn targets_mut(&mut self) -> &mut BTreeMap<String, Vec<String>> {
        Arc::make_mut(&mut self.targets)
    }

    fn insert(
        &mut self,
        descriptor: DefinitionDescriptor,
        targets: Vec<String>,
    ) -> Result<(), String> {
        let id = descriptor.definition_id.clone();
        if self.entries.contains_key(&id) {
            return Err(format!("Definition identity conflict: {id}"));
        }
        if !targets.is_empty() {
            self.targets_mut().insert(id.clone(), targets);
        }
        self.entries_mut().insert(id, descriptor);
        Ok(())
    }

    fn check_metadata(&self) -> Result<(), String> {
        for entry in self.entries.values() {
            if entry
                .upper_multiplicity
                .is_some_and(|upper| upper < entry.lower_multiplicity)
            {
                return Err(format!("Invalid multiplicity: {}", entry.definition_id));
            }
            if let Some(kind @ (ValueKind::Integer | ValueKind::Float)) = entry.kind {
                for bound in [&entry.minimum, &entry.maximum].into_iter().flatten() {
                    validation::value(
                        entry,
                        &TypedValue {
                            kind,
                            lexeme: bound.clone(),
                        },
                    )?;
                }
            }
            if let Some(default) = &entry.default_value {
                validation::value(entry, default)?;
                if entry.default_origin.is_none() {
                    return Err(format!("Missing default origin: {}", entry.definition_id));
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn parent(id: &str) -> Option<&str> {
    id.rsplit_once('/').map(|(parent, _)| parent)
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// XML comments and CDATA boundaries do not truncate a simple literal.
/// The common single text node stays borrowed from the original document.
pub(crate) fn xml_text<'a, 'input>(node: roxmltree::Node<'a, 'input>) -> Cow<'a, str> {
    let mut parts = node
        .children()
        .filter(|child| child.is_text())
        .filter_map(|child| child.text());
    let first = parts.next().unwrap_or("");
    let Some(second) = parts.next() else {
        return Cow::Borrowed(first);
    };
    let mut text = String::with_capacity(first.len() + second.len());
    text.push_str(first);
    text.push_str(second);
    for part in parts {
        text.push_str(part);
    }
    Cow::Owned(text)
}

pub(crate) fn xml_text_trimmed<'a, 'input>(node: roxmltree::Node<'a, 'input>) -> Cow<'a, str> {
    match xml_text(node) {
        Cow::Borrowed(text) => Cow::Borrowed(text.trim()),
        Cow::Owned(mut text) => {
            let length = text.trim().len();
            if length == 0 {
                text.clear();
            } else {
                let start = text.len() - text.trim_start().len();
                text.truncate(start + length);
                if start != 0 {
                    text.drain(..start);
                }
            }
            Cow::Owned(text)
        }
    }
}

pub(crate) fn descriptor(
    id: String,
    element_kind: &str,
    kind: Option<ValueKind>,
) -> DefinitionDescriptor {
    DefinitionDescriptor {
        definition_id: id,
        element_kind: element_kind.into(),
        kind,
        lower_multiplicity: 0,
        upper_multiplicity: Some(1),
        unit: None,
        minimum: None,
        maximum: None,
        enumeration: Vec::new(),
        default_value: None,
        default_origin: None,
        reference_destinations: Vec::new(),
        writable: true,
        reason: None,
    }
}
