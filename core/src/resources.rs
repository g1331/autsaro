use crate::target::BuildTarget;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;

/// Immutable, source-backed bytes whose identity was checked by build.rs.
#[derive(Clone, Copy, Debug)]
pub struct AssetEntry {
    pub relative_path: &'static str,
    pub sha256: &'static str,
    pub license: &'static str,
    pub role: &'static str,
    pub responsibility: &'static str,
    pub targets: &'static [&'static str],
    pub profiles: &'static [&'static str],
    pub bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/asset_index.rs"));

#[derive(Clone, Copy)]
pub struct AssetInventory {
    entries: &'static [AssetEntry],
}

impl AssetInventory {
    pub const fn embedded() -> Self {
        Self {
            entries: EMBEDDED_ASSETS,
        }
    }

    /// A caller-provided checkout is checked against the compiled identities,
    /// not against a manifest that can be edited alongside its source files.
    pub fn from_directory(root: &Path) -> Result<Self, crate::message::LocalizedText> {
        for entry in EMBEDDED_ASSETS {
            check_entry(root, entry)?;
        }
        Ok(Self::embedded())
    }

    pub fn verify_directory_asset(
        root: &Path,
        relative_path: &str,
    ) -> Result<(), crate::message::LocalizedText> {
        let entry = Self::embedded()
            .get(relative_path)
            .ok_or_else(|| crate::product_message!("backend.resources.not_in_inventory", "path" => relative_path))?;
        check_entry(root, entry)
    }

    pub fn entries(self) -> &'static [AssetEntry] {
        self.entries
    }

    pub fn get(self, path: &str) -> Option<&'static AssetEntry> {
        self.entries
            .iter()
            .find(|entry| entry.relative_path == path)
    }

    pub fn selected(
        self,
        target: BuildTarget,
        profile: &str,
    ) -> impl Iterator<Item = &'static AssetEntry> {
        self.entries.iter().filter(move |entry| {
            entry.targets.contains(&target.spec().id) && entry.profiles.contains(&profile)
        })
    }
}

fn check_entry(root: &Path, entry: &AssetEntry) -> Result<(), crate::message::LocalizedText> {
    let source = root.join(entry.relative_path);
    if !fs::symlink_metadata(&source)
        .map_err(|error| crate::product_message!("backend.resources.inspect_failed", "path" => source.display(), "error" => error))?
        .file_type()
        .is_file()
    {
        return Err(crate::product_message!("backend.resources.not_regular_file", "path" => source.display()));
    }
    let mut file = fs::File::open(&source).map_err(|error| error.to_string())?;
    let mut digest = Sha256::new();
    let mut chunk = [0u8; 8192];
    loop {
        let count = file.read(&mut chunk).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        digest.update(&chunk[..count]);
    }
    if format!("{:x}", digest.finalize()) != entry.sha256 {
        return Err(
            crate::product_message!("backend.resources.identity_differs", "path" => entry.relative_path),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_asset_preserves_path_as_message_parameter() {
        let path = "用户/unknown-asset.txt";
        let error = AssetInventory::verify_directory_asset(Path::new("."), path).unwrap_err();
        let wire = serde_json::to_value(&error).unwrap();
        assert_eq!(wire["key"], "backend.resources.not_in_inventory");
        assert_eq!(wire["params"]["path"], path);
    }
}
