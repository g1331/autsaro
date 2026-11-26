use super::*;
use crate::generator::delivery::{FileOwner, OwnershipEntry, OwnershipLedger};
use crate::prepared::PreparedProject;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactChange {
    Added,
    Removed,
    Identity,
    Content,
    ApplicationSlot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactDifference {
    pub path: String,
    pub owner: FileOwner,
    pub producer: String,
    pub change: ArtifactChange,
    pub before: Option<String>,
    pub after: Option<String>,
}

struct Artifacts<'a> {
    files: BTreeMap<&'a str, &'a [u8]>,
    entries: BTreeMap<String, OwnershipEntry>,
}

impl<'a> Artifacts<'a> {
    fn read(project: &'a PreparedProject<'_>) -> Result<Self, LocalizedText> {
        let files: BTreeMap<_, _> = project
            .files()
            .iter()
            .map(|file| (file.path.as_str(), file.bytes.as_ref()))
            .collect();
        let ledger: OwnershipLedger = serde_json::from_slice(
            files
                .get(crate::generator::delivery::OWNERSHIP_PATH)
                .ok_or("A native ownership ledger is required.")?,
        )
        .map_err(|error| error.to_string())?;
        let entries = ledger
            .files
            .into_iter()
            .filter(|entry| entry.owner != FileOwner::Configuration)
            .map(|entry| {
                (
                    entry
                        .snapshot_of
                        .clone()
                        .unwrap_or_else(|| entry.path.clone()),
                    entry,
                )
            })
            .collect();
        Ok(Self { files, entries })
    }
}

fn remove_delivery_stamps(metadata: &mut serde_json::Map<String, serde_json::Value>) {
    for key in ["producerVersion", "resourceIdentities", "inputSnapshots"] {
        metadata.remove(key);
    }
}

fn remove_plan_stamps(plan: &mut serde_json::Map<String, serde_json::Value>) -> Option<()> {
    for key in [
        "validationDependencies",
        "ruleSetIdentity",
        "requiredExtensionDefinitions",
    ] {
        plan.remove(key);
    }
    for source in plan.get_mut("sources")?.as_array_mut()? {
        source.as_object_mut()?.remove("rawSha256");
    }
    Some(())
}

fn only_identity_stamps(path: &str, before: &[u8], after: &[u8]) -> bool {
    let normalize = |bytes: &[u8]| -> Option<serde_json::Value> {
        let mut value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
        let object = value.as_object_mut()?;
        match path {
            "target.json" => {
                object.remove("inputSha256");
                object.remove("definitionFingerprint");
                if let Some(delivery) = object
                    .get_mut("nativeDelivery")
                    .and_then(serde_json::Value::as_object_mut)
                {
                    remove_delivery_stamps(delivery);
                }
            }
            crate::generator::delivery::HANDOFF_PATH => remove_delivery_stamps(object),
            "meta/integration.json" => {
                remove_plan_stamps(object.get_mut("plan")?.as_object_mut()?)?
            }
            "meta/contract.json" => remove_plan_stamps(object)?,
            "meta/seal-policy.json" => {
                object.remove("payload");
                if let Some(delivery) = object
                    .get_mut("metadata")
                    .and_then(serde_json::Value::as_object_mut)
                {
                    remove_delivery_stamps(delivery);
                }
            }
            _ => return None,
        }
        Some(value)
    };
    normalize(before)
        .zip(normalize(after))
        .is_some_and(|(before, after)| before == after)
}

/// Compare immutable outputs by ownership and producer. Application content is
/// represented only by its digest and its live slot identity.
pub fn compare_project_artifacts(
    before: &PreparedProject<'_>,
    after: &PreparedProject<'_>,
) -> Result<Vec<ArtifactDifference>, LocalizedText> {
    let left = Artifacts::read(before)?;
    let right = Artifacts::read(after)?;
    let paths: BTreeSet<_> = left.entries.keys().chain(right.entries.keys()).collect();
    let mut differences = Vec::new();
    for path in paths {
        let old = left.entries.get(path);
        let new = right.entries.get(path);
        let entry = new.or(old).expect("A compared path has an entry");
        let change = match (old, new) {
            (Some(old), Some(new)) if old == new => continue,
            (Some(old), Some(new))
                if entry.owner == FileOwner::UserApplication
                    && (old.path != new.path || old.producer_id != new.producer_id) =>
            {
                ArtifactChange::ApplicationSlot
            }
            (Some(old), Some(new))
                if only_identity_stamps(
                    path,
                    left.files[old.path.as_str()],
                    right.files[new.path.as_str()],
                ) =>
            {
                ArtifactChange::Identity
            }
            (Some(_), Some(_)) => ArtifactChange::Content,
            (None, Some(_)) => ArtifactChange::Added,
            (Some(_), None) => ArtifactChange::Removed,
            (None, None) => continue,
        };
        differences.push(ArtifactDifference {
            path: path.clone(),
            owner: entry.owner,
            producer: entry.producer_id.clone(),
            change,
            before: old.map(|entry| entry.sha256.clone()),
            after: new.map(|entry| entry.sha256.clone()),
        });
    }
    Ok(differences)
}
