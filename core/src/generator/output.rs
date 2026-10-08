use crate::model::{GenerationPreview, GenerationPreviewFile, GenerationReport, Issue};
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::fmt::Write;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub(super) fn file_digest(path: &Path) -> Result<String, crate::LocalizedText> {
    let mut file = fs::File::open(path)
        .map_err(|e| crate::product_message!("backend.generation.generated_file_missing", "path" => path.display(), "error" => e.to_string()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| crate::product_message!("backend.generation.generated_file_unreadable", "path" => path.display(), "error" => e.to_string()))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn integrity_record(dir: &Path, names: &[String]) -> Result<String, crate::LocalizedText> {
    let mut record = String::new();
    for name in names
        .iter()
        .map(String::as_str)
        .chain(std::iter::once("files.list"))
    {
        writeln!(record, "{}  {name}", file_digest(&dir.join(name))?).unwrap();
    }
    Ok(record)
}

fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

pub(crate) fn reserve_directory(
    parent: &Path,
    role: &str,
    output_name: &OsStr,
) -> Result<PathBuf, crate::LocalizedText> {
    #[cfg(unix)]
    let builder = {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = fs::DirBuilder::new();
        builder.mode(0o700);
        builder
    };
    #[cfg(not(unix))]
    let builder = fs::DirBuilder::new();
    for suffix in 0u64.. {
        let candidate = parent.join(format!(
            ".autosar-config-{role}-{}-{suffix}",
            std::process::id()
        ));
        if candidate.file_name() == Some(output_name) {
            continue;
        }
        match builder.create(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(
                    crate::product_message!("backend.generation.temporary_directory_failed", "path" => candidate.display(), "error" => error.to_string()),
                );
            }
        }
    }
    Err(crate::product_message!(
        "backend.generation.temporary_directory_exhausted"
    ))
}

fn check_entries(dir: &Path, root: &Path, names: &[String]) -> Result<(), crate::LocalizedText> {
    for entry in fs::read_dir(dir).map_err(|e| crate::LocalizedText::from(e.to_string()))? {
        let entry = entry.map_err(|e| crate::LocalizedText::from(e.to_string()))?;
        let path = entry.path();
        let name = path
            .strip_prefix(root)
            .map_err(|e| crate::LocalizedText::from(e.to_string()))?
            .to_str()
            .ok_or_else(|| crate::product_message!("backend.generation.output_name_not_utf8"))?
            .replace('\\', "/");
        let metadata =
            fs::symlink_metadata(&path).map_err(|e| crate::LocalizedText::from(e.to_string()))?;
        if is_reparse_point(&metadata) {
            return Err(
                crate::product_message!("backend.generation.output_link_refused", "path" => name),
            );
        }
        let kind = metadata.file_type();
        if kind.is_dir() {
            // A directory is generated only when it is an ancestor of a
            // declared file. This also covers nested kernel/OS/input trees;
            // arbitrary owner directories, even empty ones, still refuse.
            let prefix = format!("{name}/");
            if !names.iter().any(|item| item.starts_with(&prefix)) {
                return Err(
                    crate::product_message!("backend.generation.output_user_directory_refused", "path" => name),
                );
            }
            check_entries(&path, root, names)?;
        } else if !kind.is_file()
            || (name != "files.list"
                && name != "files.sha256"
                && names.binary_search(&name).is_err())
        {
            return Err(
                crate::product_message!("backend.generation.output_user_file_refused", "path" => name),
            );
        }
    }
    Ok(())
}

fn verify_generated_output(dir: &Path, names: &[String]) -> Result<(), crate::LocalizedText> {
    let metadata = match fs::symlink_metadata(dir) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string().into()),
    };
    if is_reparse_point(&metadata) || !metadata.file_type().is_dir() {
        return Err(
            crate::product_message!("backend.generation.output_not_plain_directory", "path" => dir.display()),
        );
    }
    if fs::read_dir(dir)
        .map_err(|e| crate::LocalizedText::from(e.to_string()))?
        .next()
        .is_none()
    {
        return Ok(());
    }
    let previous = fs::read_to_string(dir.join("files.list"))
        .map_err(|_| crate::product_message!("backend.generation.output_inventory_missing", "path" => dir.display()))?;
    if previous != format!("{}\n", names.join("\n")) {
        return Err(crate::product_message!(
            "backend.generation.output_inventory_mismatch"
        ));
    }
    check_entries(dir, dir, names)?;
    let recorded = fs::read_to_string(dir.join("files.sha256")).map_err(|_| {
        crate::product_message!("backend.generation.output_integrity_missing", "path" => dir.display())
    })?;
    if recorded != integrity_record(dir, names)? {
        return Err(crate::product_message!(
            "backend.generation.output_modified"
        ));
    }
    Ok(())
}

pub(crate) fn verify_build_input(output: &Path) -> Result<Vec<String>, crate::LocalizedText> {
    let list = fs::read_to_string(output.join("files.list"))
        .map_err(|e| crate::product_message!("backend.generation.build_inventory_missing", "error" => e.to_string()))?;
    let names: Vec<String> = list.lines().map(str::to_owned).collect();
    if names.is_empty()
        || list != format!("{}\n", names.join("\n"))
        || names.windows(2).any(|pair| pair[0] >= pair[1])
        || names.iter().any(|name| {
            name.contains('\\')
                || Path::new(name)
                    .components()
                    .any(|component| !matches!(component, std::path::Component::Normal(_)))
        })
    {
        return Err(crate::product_message!(
            "backend.generation.build_inventory_invalid"
        ));
    }
    verify_generated_output(output, &names).map_err(|error| {
        crate::LocalizedText::messages([
            crate::product_message!("backend.generation.build_integrity_failed"),
            error,
        ])
    })?;
    super::delivery::ownership::verify_directory(output, &names)?;
    Ok(names)
}

pub(crate) fn seal_files(mut files: Vec<(String, Vec<u8>)>) -> Vec<(String, Vec<u8>)> {
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let names: Vec<_> = files.iter().map(|(name, _)| name.clone()).collect();
    let list = names.join("\n") + "\n";
    let mut record = String::new();
    for (name, bytes) in &files {
        writeln!(record, "{:x}  {name}", Sha256::digest(bytes)).unwrap();
    }
    writeln!(record, "{:x}  files.list", Sha256::digest(list.as_bytes())).unwrap();
    files.push(("files.list".into(), list.into_bytes()));
    files.push(("files.sha256".into(), record.into_bytes()));
    files
}
pub(super) fn output_path(output: &Path) -> Result<PathBuf, crate::LocalizedText> {
    let output_name = output
        .file_name()
        .ok_or_else(|| crate::product_message!("backend.generation.output_name_required"))?;
    let parent = output
        .parent()
        .ok_or_else(|| crate::product_message!("backend.generation.output_parent_required"))?;
    if output_name == "." || output_name == ".." || parent.join(output_name) != output {
        return Err(crate::product_message!(
            "backend.generation.output_named_path_required"
        ));
    }
    let parent = if parent.is_absolute() {
        parent.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| crate::LocalizedText::from(e.to_string()))?
            .join(parent)
    };
    Ok(parent.join(output_name))
}

fn file_names(files: &[(String, Vec<u8>)]) -> Vec<String> {
    files
        .iter()
        .take(files.len() - 2)
        .map(|(name, _)| name.clone())
        .collect()
}

fn inspect_prepared(
    files: &[(String, Vec<u8>)],
    output: &Path,
    include_changes: bool,
    guard: Option<&super::delivery::NativeGuard>,
) -> Result<GenerationPreview, crate::LocalizedText> {
    if let Some(guard) = guard {
        guard.verify(Some(output))?;
    }
    let output = output_path(output)?;
    let names = file_names(files);
    verify_generated_output(&output, &names)?;
    let owners = if let Some((_, bytes)) = files
        .iter()
        .find(|(path, _)| path == super::delivery::OWNERSHIP_PATH)
    {
        let ledger: super::delivery::OwnershipLedger = serde_json::from_slice(bytes)
            .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
        ledger
            .files
            .into_iter()
            .map(|entry| (entry.path.clone(), entry))
            .collect::<std::collections::BTreeMap<_, _>>()
    } else {
        std::collections::BTreeMap::new()
    };
    if let Some(guard) = guard {
        if output.exists() && output.join(super::delivery::OWNERSHIP_PATH).exists() {
            super::delivery::reopen::verify_package(&output, &guard.catalog)?;
        }
    }
    let mut digest = Sha256::new();
    digest.update(output.to_string_lossy().as_bytes());
    if let Some(guard) = guard {
        digest.update(guard.revision_identity.as_bytes());
    }
    let mut changes = Vec::new();
    for (name, after) in files {
        let path = output.join(name);
        let before = match fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(
                    crate::product_message!("backend.generation.previous_file_unreadable", "path" => path.display(), "error" => error.to_string()),
                );
            }
        };
        digest.update((name.len() as u64).to_le_bytes());
        digest.update(name.as_bytes());
        if let Some(bytes) = &before {
            digest.update([1]);
            digest.update((bytes.len() as u64).to_le_bytes());
            digest.update(bytes);
        } else {
            digest.update([0]);
        }
        digest.update((after.len() as u64).to_le_bytes());
        digest.update(after);
        if !include_changes {
            continue;
        }
        let status = if before.is_none() {
            "new"
        } else if before.as_deref() == Some(after) {
            "unchanged"
        } else {
            "changed"
        };
        changes.push(GenerationPreviewFile {
            owner: owners
                .get(name)
                .map(|entry| entry.owner.as_str().to_owned())
                .or_else(|| {
                    (!owners.is_empty()
                        && matches!(
                            name.as_str(),
                            super::delivery::OWNERSHIP_PATH | "files.list" | "files.sha256"
                        ))
                    .then(|| "generated".into())
                }),
            producer_id: owners
                .get(name)
                .map(|entry| entry.producer_id.clone())
                .or_else(|| {
                    (!owners.is_empty()
                        && matches!(
                            name.as_str(),
                            super::delivery::OWNERSHIP_PATH | "files.list" | "files.sha256"
                        ))
                    .then(|| "autosar-generator".into())
                }),
            snapshot_of: owners.get(name).and_then(|entry| entry.snapshot_of.clone()),
            path: name.clone(),
            status: status.into(),
            before: if status == "changed" {
                Some(
                    String::from_utf8(before.unwrap())
                        .map_err(|_| crate::product_message!("backend.generation.previous_file_not_utf8", "path" => name))?,
                )
            } else {
                None
            },
            after: if status != "unchanged" {
                Some(
                    String::from_utf8(after.clone())
                        .map_err(|_| crate::product_message!("backend.generation.new_file_not_utf8", "path" => name))?,
                )
            } else {
                None
            },
        });
    }
    Ok(GenerationPreview {
        output_directory: output.display().to_string(),
        revision: format!("{:x}", digest.finalize()),
        files: changes,
    })
}
pub(crate) fn preview_prepared(
    files: &[(String, Vec<u8>)],
    output: &Path,
) -> Result<GenerationPreview, crate::LocalizedText> {
    inspect_prepared(files, output, true, None)
}

pub(crate) fn preview_prepared_checked(
    files: &[(String, Vec<u8>)],
    output: &Path,
    guard: Option<&super::delivery::NativeGuard>,
) -> Result<GenerationPreview, crate::LocalizedText> {
    inspect_prepared(files, output, true, guard)
}

/// An owned private source stage; dropping it never alters the destination.
pub struct StagedGeneration {
    files: Vec<(String, Vec<u8>)>,
    names: Vec<String>,
    output: PathBuf,
    stage: Option<PathBuf>,
    revision: String,
    guard: Option<super::delivery::NativeGuard>,
}

impl StagedGeneration {
    pub(crate) fn new(
        files: Vec<(String, Vec<u8>)>,
        output: &Path,
        expected_revision: Option<&str>,
    ) -> Result<Self, crate::LocalizedText> {
        Self::new_guarded(files, output, expected_revision, None)
    }

    pub(crate) fn new_guarded(
        files: Vec<(String, Vec<u8>)>,
        output: &Path,
        expected_revision: Option<&str>,
        guard: Option<super::delivery::NativeGuard>,
    ) -> Result<Self, crate::LocalizedText> {
        let output = output_path(output)?;
        let revision = inspect_prepared(&files, &output, false, guard.as_ref())?.revision;
        if expected_revision.is_some_and(|expected| expected != revision) {
            return Err(crate::product_message!("backend.generation.preview_stale"));
        }
        let names = file_names(&files);
        let parent = output
            .parent()
            .ok_or_else(|| crate::product_message!("backend.generation.output_parent_required"))?;
        let output_name = output
            .file_name()
            .ok_or_else(|| crate::product_message!("backend.generation.output_name_required"))?;
        fs::create_dir_all(parent)
            .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
        let stage = reserve_directory(parent, "stage", output_name)?;
        let staged = Self {
            files,
            names,
            output,
            stage: Some(stage),
            revision,
            guard,
        };
        for (name, contents) in &staged.files {
            let target = staged.stage.as_ref().unwrap().join(name);
            fs::create_dir_all(target.parent().unwrap())
                .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
            fs::write(target, contents)
                .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
        }
        Ok(staged)
    }

    /// Call while holding the workbench's fingerprint/operation commit lock.
    pub fn commit(mut self) -> Result<GenerationReport, crate::LocalizedText> {
        if inspect_prepared(&self.files, &self.output, false, self.guard.as_ref())?.revision
            != self.revision
        {
            return Err(crate::product_message!(
                "backend.generation.confirmation_stale"
            ));
        }
        let output = &self.output;
        let parent = output
            .parent()
            .ok_or_else(|| crate::product_message!("backend.generation.output_parent_required"))?;
        let output_name = output
            .file_name()
            .ok_or_else(|| crate::product_message!("backend.generation.output_name_required"))?;
        let existing = match fs::symlink_metadata(&output) {
            Ok(metadata) if is_reparse_point(&metadata) || !metadata.file_type().is_dir() => {
                return Err(
                    crate::product_message!("backend.generation.output_changed_type", "path" => output.display()),
                );
            }
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.to_string().into()),
        };
        // Producer/source validation can re-render a complete existing package.
        // Re-read live bytes again at the installation boundary, not just before
        // that work, so a late edit never authorizes replacing the old output.
        if let Some(guard) = &self.guard {
            guard.verify(Some(output))?;
        }
        let backup = if existing {
            let backup_root = reserve_directory(parent, "backup", output_name)?;
            let preserved = backup_root.join(output_name);
            fs::rename(&output, &preserved).map_err(|e| {
                crate::product_message!("backend.generation.previous_output_backup_failed", "path" => output.display(), "backup" => preserved.display(), "error" => e.to_string())
            })?;
            Some(preserved)
        } else {
            None
        };
        if let Err(error) = fs::rename(self.stage.as_ref().unwrap(), output) {
            return Err(if let Some(path) = &backup {
                crate::product_message!("backend.generation.output_install_failed_backup", "error" => error.to_string(), "path" => path.display())
            } else {
                crate::product_message!("backend.generation.output_install_failed", "error" => error.to_string())
            });
        }
        self.stage = None;
        Ok(GenerationReport {
            output_directory: output.display().to_string(),
            previous_output_directory: backup.map(|path| path.display().to_string()),
            files: std::mem::take(&mut self.names),
            issues: Vec::<Issue>::new(),
        })
    }
}

impl Drop for StagedGeneration {
    fn drop(&mut self) {
        if let Some(stage) = &self.stage {
            let _ = fs::remove_dir_all(stage);
        }
    }
}

pub(crate) fn generate_prepared(
    files: Vec<(String, Vec<u8>)>,
    output: &Path,
    expected_revision: Option<&str>,
) -> Result<GenerationReport, crate::LocalizedText> {
    StagedGeneration::new(files, output, expected_revision)?.commit()
}

/// Native build output remains private until a fingerprint-checked commit.
pub struct StagedBuild {
    root: PathBuf,
    directory: PathBuf,
    output: PathBuf,
}

impl StagedBuild {
    pub fn new(project: &Path, output: &Path) -> Result<Self, crate::LocalizedText> {
        let output = output_path(output)?;
        for path in output.ancestors().chain(project.ancestors()) {
            match fs::symlink_metadata(path) {
                Ok(metadata) if is_reparse_point(&metadata) => {
                    return Err(
                        crate::product_message!("backend.generation.build_link_refused", "path" => path.display()),
                    );
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string().into()),
            }
        }
        let project = project
            .canonicalize()
            .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
        let requested_parent = output
            .parent()
            .ok_or_else(|| crate::product_message!("backend.generation.build_parent_required"))?;
        let mut existing = requested_parent;
        while !existing
            .try_exists()
            .map_err(|error| crate::LocalizedText::from(error.to_string()))?
        {
            existing = existing.parent().ok_or_else(|| {
                crate::product_message!("backend.generation.build_existing_parent_required")
            })?;
        }
        let parent = existing
            .canonicalize()
            .map_err(|error| crate::LocalizedText::from(error.to_string()))?
            .join(
                requested_parent
                    .strip_prefix(existing)
                    .map_err(|error| crate::LocalizedText::from(error.to_string()))?,
            );
        let output =
            parent.join(output.file_name().ok_or_else(|| {
                crate::product_message!("backend.generation.build_name_required")
            })?);
        if output.starts_with(&project) || project.starts_with(&output) {
            return Err(crate::product_message!(
                "backend.generation.build_independent_output_required"
            ));
        }
        check_empty_build_output(&output)?;
        fs::create_dir_all(&parent)
            .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
        let root = reserve_directory(&parent, "build-stage", output.file_name().unwrap())?;
        let directory = root.join("build");
        Ok(Self {
            root,
            directory,
            output,
        })
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn commit(mut self) -> Result<PathBuf, crate::LocalizedText> {
        check_empty_build_output(&self.output)?;
        if self.output.exists() {
            fs::remove_dir(&self.output)
                .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
        }
        fs::rename(&self.directory, &self.output)
            .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
        Ok(std::mem::take(&mut self.output))
    }
}

impl Drop for StagedBuild {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn check_empty_build_output(output: &Path) -> Result<(), crate::LocalizedText> {
    match fs::symlink_metadata(output) {
        Ok(metadata) if is_reparse_point(&metadata) || !metadata.is_dir() => Err(
            crate::product_message!("backend.generation.build_plain_directory_required"),
        ),
        Ok(_) => {
            if fs::read_dir(output)
                .map_err(|error| crate::LocalizedText::from(error.to_string()))?
                .next()
                .is_some()
            {
                Err(crate::product_message!(
                    "backend.generation.build_empty_output_required"
                ))
            } else {
                Ok(())
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string().into()),
    }
}

#[cfg(test)]
mod localization_tests {
    use super::{output_path, reserve_directory};
    use crate::LocalizedText;
    use std::ffi::OsStr;
    use std::path::Path;

    #[test]
    fn unnamed_output_uses_structured_product_error() {
        let error = output_path(Path::new("")).unwrap_err();
        assert_eq!(
            serde_json::to_value(error).unwrap(),
            serde_json::json!({"key": "backend.generation.output_name_required", "params": {}})
        );
    }

    #[test]
    fn temporary_directory_failure_preserves_path_and_system_evidence() {
        let parent = Path::new("missing\0parent");
        let error = reserve_directory(parent, "stage", OsStr::new("output")).unwrap_err();
        match error {
            LocalizedText::Message(message) => {
                assert_eq!(message.key, "backend.generation.temporary_directory_failed");
                assert!(
                    message.params["path"]
                        .as_str()
                        .unwrap()
                        .contains("missing\0parent")
                );
                assert!(!message.params["error"].as_str().unwrap().is_empty());
            }
            other => panic!("Expected a product envelope, got {other:?}"),
        }
    }
}
