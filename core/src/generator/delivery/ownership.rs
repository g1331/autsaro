use super::{
    APPLICATION_OUTPUT, APPLICATION_SLOT, FORMAT, HOST_PROFILE, HandoffMetadata, InputKind,
    OWNERSHIP_PATH, PROJECT_PATH, digest, json_bytes, safe_relative,
};
use crate::prepared::PreparedFile;
use crate::resources::AssetInventory;
use crate::target::BuildTarget;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const CONFIG_PRODUCER: &str = "autosar-workbench-config-v1";
const PRODUCT_PRODUCER: &str = "autosar-product";
const GENERATED_PRODUCER: &str = "autosar-generator";
const BUILD_PRODUCER: &str = "autosar-workbench-build-v2";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileOwner {
    Configuration,
    UserApplication,
    Product,
    Generated,
    Target,
    Build,
}

impl FileOwner {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::UserApplication => "user-application",
            Self::Product => "product",
            Self::Generated => "generated",
            Self::Target => "target",
            Self::Build => "build",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnershipEntry {
    pub path: String,
    pub owner: FileOwner,
    pub producer_id: String,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_of: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnershipLedger {
    pub format_version: u32,
    pub producer_version: String,
    pub profile_id: String,
    pub files: Vec<OwnershipEntry>,
}

fn classification(
    path: &str,
    metadata: &HandoffMetadata,
    source: Option<&crate::resources::AssetEntry>,
) -> Result<(FileOwner, String, Option<String>), String> {
    if path == PROJECT_PATH
        || metadata
            .input_snapshots
            .iter()
            .any(|input| input.kind == InputKind::Arxml && input.package_path == path)
    {
        return Ok((FileOwner::Configuration, CONFIG_PRODUCER.into(), None));
    }
    if let Some(input) = metadata
        .input_snapshots
        .iter()
        .find(|input| input.kind == InputKind::Application && input.package_path == path)
    {
        if path != APPLICATION_OUTPUT || input.producer_slot.as_deref() != Some(APPLICATION_SLOT) {
            return Err("Unrecognized application snapshot producer or source location.".into());
        }
        return Ok((
            FileOwner::UserApplication,
            APPLICATION_SLOT.into(),
            Some(input.logical_path.clone()),
        ));
    }
    if path.starts_with("tools/") {
        return Ok((FileOwner::Build, BUILD_PRODUCER.into(), None));
    }
    if path == "target.json" || source.is_some_and(|asset| asset.role == "target") {
        return Ok((FileOwner::Target, metadata.target_id.spec().id.into(), None));
    }
    if source.is_some() {
        Ok((FileOwner::Product, PRODUCT_PRODUCER.into(), None))
    } else {
        Ok((FileOwner::Generated, GENERATED_PRODUCER.into(), None))
    }
}

pub(crate) fn ledger<'a>(
    files: &BTreeMap<String, PreparedFile<'a>>,
    metadata: &HandoffMetadata,
) -> Result<OwnershipLedger, String> {
    let mut entries = Vec::with_capacity(files.len());
    for (path, file) in files {
        if matches!(
            path.as_str(),
            OWNERSHIP_PATH | "files.list" | "files.sha256"
        ) {
            return Err("An ownership ledger cannot enumerate itself or seal metadata.".into());
        }
        safe_relative(path)?;
        let (owner, producer_id, snapshot_of) = classification(path, metadata, file.source)?;
        entries.push(OwnershipEntry {
            path: path.clone(),
            owner,
            producer_id,
            sha256: digest(&file.bytes),
            snapshot_of,
        });
    }
    Ok(OwnershipLedger {
        format_version: 1,
        producer_version: env!("CARGO_PKG_VERSION").into(),
        profile_id: metadata.profile_id.clone(),
        files: entries,
    })
}

pub(crate) fn add_ledger<'a>(
    files: &mut BTreeMap<String, PreparedFile<'a>>,
    metadata: &HandoffMetadata,
) -> Result<(), String> {
    let ledger = ledger(files, metadata)?;
    super::insert_file(
        files,
        OWNERSHIP_PATH.into(),
        Cow::Owned(json_bytes(&ledger)?),
    )
}

pub(crate) fn metadata_from_target(bytes: &[u8]) -> Result<Option<HandoffMetadata>, String> {
    let target: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let Some(native) = target.get("nativeDelivery") else {
        return Ok(None);
    };
    let metadata: HandoffMetadata =
        serde_json::from_value(native.clone()).map_err(|error| error.to_string())?;
    validate_metadata(&metadata)?;
    let recorded_target: BuildTarget =
        serde_json::from_value(target["target"].clone()).map_err(|error| error.to_string())?;
    if recorded_target != metadata.target_id
        || target["format"] != "autosar-build-target-v1"
        || target["profile"]
            != if metadata.profile_id == HOST_PROFILE {
                "host"
            } else {
                "ecu"
            }
        || target["definitionFingerprint"]
            .as_str()
            .is_none_or(|value| value.len() != 64)
    {
        return Err("Native delivery does not agree with its actual build profile/target/definition identity.".into());
    }
    Ok(Some(metadata))
}

pub(crate) fn validate_metadata(metadata: &HandoffMetadata) -> Result<(), String> {
    if metadata.format != FORMAT
        || metadata.producer_version != env!("CARGO_PKG_VERSION")
        || !matches!(
            metadata.profile_id.as_str(),
            HOST_PROFILE | crate::integration::PROFILE
        )
        || metadata.project_path != PROJECT_PATH
        || metadata.ownership_path != OWNERSHIP_PATH
        || metadata.resource_identities.rule_set_identity != crate::rules::rule_set_identity()?
        || metadata.input_snapshots.is_empty()
    {
        return Err(
            "Unsupported v2 producer/profile/target or incompatible trusted native rule identity."
                .into(),
        );
    }
    let mut logical = BTreeSet::new();
    let mut package = BTreeSet::new();
    for input in &metadata.input_snapshots {
        safe_relative(&input.logical_path)?;
        safe_relative(&input.package_path)?;
        if input.sha256.len() != 64
            || !input.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            || !logical.insert(input.logical_path.to_ascii_lowercase())
            || !package.insert(input.package_path.to_ascii_lowercase())
            || input.role.is_empty()
        {
            return Err("Invalid or duplicate native input snapshot identity.".into());
        }
        match input.kind {
            InputKind::Arxml => {
                if input.package_path != format!("inputs/{}", input.logical_path)
                    || !Path::new(&input.logical_path)
                        .extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("arxml"))
                    || input.producer_slot.is_some()
                {
                    return Err("Invalid native configuration source mapping.".into());
                }
            }
            InputKind::Application => {
                if metadata.profile_id != crate::integration::PROFILE
                    || input.package_path != APPLICATION_OUTPUT
                    || input.producer_slot.as_deref() != Some(APPLICATION_SLOT)
                    || input.role != "user-application"
                {
                    return Err(
                        "Unsupported native application producer slot or source boundary.".into(),
                    );
                }
            }
        }
    }
    let required = &metadata.resource_identities.required_extension_definitions;
    if required
        .windows(2)
        .any(|pair| pair[0].catalog_id >= pair[1].catalog_id)
        || required.iter().any(|identity| {
            identity.release != "R24-11"
                || identity.version.is_empty()
                || identity.sha256.len() != 64
                || !identity.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    {
        return Err("Invalid required extension identity closure.".into());
    }
    Ok(())
}

pub(crate) fn verify_ledger_bytes(
    files: &BTreeMap<String, &[u8]>,
    metadata: &HandoffMetadata,
) -> Result<OwnershipLedger, String> {
    validate_metadata(metadata)?;
    let ledger: OwnershipLedger = serde_json::from_slice(
        files
            .get(OWNERSHIP_PATH)
            .ok_or("Native package is missing its ownership ledger.")?,
    )
    .map_err(|error| error.to_string())?;
    if ledger.format_version != 1
        || ledger.producer_version != env!("CARGO_PKG_VERSION")
        || ledger.profile_id != metadata.profile_id
        || ledger
            .files
            .windows(2)
            .any(|pair| pair[0].path >= pair[1].path)
    {
        return Err(
            "Unknown ownership version/producer/profile or noncanonical owner order.".into(),
        );
    }
    let profile = if metadata.profile_id == HOST_PROFILE {
        "host"
    } else {
        "ecu"
    };
    let mut trusted_assets = BTreeMap::new();
    for asset in AssetInventory::embedded().selected(metadata.target_id, profile) {
        trusted_assets.insert(crate::prepared::deliver_path(asset, profile)?, asset);
    }
    if profile == "ecu" {
        let origin = AssetInventory::embedded()
            .get("runtime/include/Com.h")
            .ok_or("The trusted Com adaptation origin is missing.")?;
        trusted_assets.insert("bsw-origin/include/Com.h".into(), origin);
    }
    let mut owned = BTreeSet::new();
    let mut portable = BTreeSet::new();
    for entry in &ledger.files {
        safe_relative(&entry.path)?;
        if matches!(
            entry.path.as_str(),
            OWNERSHIP_PATH | "files.list" | "files.sha256"
        ) || !owned.insert(entry.path.as_str())
            || !portable.insert(entry.path.to_ascii_lowercase())
        {
            return Err("The ownership closure contains duplicate/reserved paths.".into());
        }
        let bytes = files
            .get(&entry.path)
            .ok_or_else(|| format!("Owned payload is missing: {}", entry.path))?;
        if digest(bytes) != entry.sha256 {
            return Err(format!("Owned payload changed: {}", entry.path));
        }
        let source = trusted_assets
            .get(&entry.path)
            .copied()
            .filter(|asset| asset.bytes == *bytes);
        let (owner, producer_id, snapshot_of) = classification(&entry.path, metadata, source)?;
        if owner != entry.owner
            || producer_id != entry.producer_id
            || snapshot_of != entry.snapshot_of
        {
            return Err(format!(
                "Ownership cannot grant write authority to this producer/path: {}",
                entry.path
            ));
        }
    }
    let payload: BTreeSet<_> = files
        .keys()
        .map(String::as_str)
        .filter(|path| !matches!(*path, OWNERSHIP_PATH | "files.list" | "files.sha256"))
        .collect();
    if owned != payload {
        return Err("Native ownership must describe the entire sealed payload, without extra or missing files.".into());
    }
    for input in &metadata.input_snapshots {
        let bytes = files
            .get(&input.package_path)
            .ok_or("Mapped native input is not in the owned payload.")?;
        if digest(bytes) != input.sha256 {
            return Err(format!(
                "Mapped input snapshot identity changed: {}",
                input.logical_path
            ));
        }
    }
    if let Some(handoff) = files.get("handoff.json") {
        let declared: HandoffMetadata =
            serde_json::from_slice(handoff).map_err(|error| error.to_string())?;
        if declared != *metadata {
            return Err(
                "Handoff and build metadata do not bind the same native input/resource snapshot."
                    .into(),
            );
        }
    }
    Ok(ledger)
}

pub(crate) fn verify_directory(
    directory: &Path,
    names: &[String],
) -> Result<Option<HandoffMetadata>, String> {
    if !names.iter().any(|path| path == OWNERSHIP_PATH) {
        if fs::read(directory.join("target.json"))
            .ok()
            .map(|bytes| metadata_from_target(&bytes))
            .transpose()?
            .flatten()
            .is_some()
        {
            return Err("Native build metadata cannot omit its ownership ledger.".into());
        }
        return Ok(None);
    }
    let target = super::read_source(&directory.join("target.json"))?;
    let metadata = metadata_from_target(&target)?
        .ok_or("An ownership ledger requires actual native delivery metadata.")?;
    let mut owned_bytes = BTreeMap::new();
    for path in names {
        owned_bytes.insert(path.clone(), super::read_source(&directory.join(path))?);
    }
    let borrowed: BTreeMap<_, _> = owned_bytes
        .iter()
        .map(|(path, bytes)| (path.clone(), bytes.as_slice()))
        .collect();
    verify_ledger_bytes(&borrowed, &metadata)?;
    Ok(Some(metadata))
}
