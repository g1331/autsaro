use super::*;
use crate::generator::delivery::ApplicationSlotDescriptor;
use crate::project_model::ProjectProjection;
use serde::{Deserialize, Serialize};
use std::io::Write;

const MANIFEST: &str = "workbench-project.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationInitializationPreview {
    pub revision: String,
    pub slot: ApplicationSlotDescriptor,
    pub files: Vec<ProjectFilePreview>,
    pub manifest_before: String,
    pub manifest_after: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInitializationOutcome {
    pub projection: ProjectProjection,
    pub warnings: Vec<crate::message::LocalizedText>,
    pub retained_recovery_files: Vec<String>,
}

fn absent(path: &Path) -> Result<(), crate::message::LocalizedText> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{}: {error}", path.display()).into()),
        Ok(_) => Err(crate::product_message!(
            "backend.arxml.application.initialization_existing_path",
            "path" => path.display()
        )),
    }
}

fn target(root: &Path, logical: &str) -> Result<PathBuf, crate::message::LocalizedText> {
    let relative = super::project::relative_path(logical)?;
    super::project::safe_path(root, false)?;
    let mut path = root.to_owned();
    let components: Vec<_> = relative.components().collect();
    let mut missing = false;
    for (index, component) in components.iter().enumerate() {
        path.push(component.as_os_str());
        if missing {
            continue;
        }
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                super::project::safe_path(&path, false)?;
                if index + 1 == components.len() || !metadata.is_dir() {
                    return Err(crate::product_message!(
                        "backend.arxml.application.initialization_path_occupied",
                        "path" => path.display()
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => missing = true,
            Err(error) => return Err(format!("{}: {error}", path.display()).into()),
        }
    }
    Ok(path)
}

fn create_parents(
    root: &Path,
    logical: &str,
    created: &mut Vec<PathBuf>,
) -> Result<(), crate::message::LocalizedText> {
    let relative = super::project::relative_path(logical)?;
    let mut current = root.to_owned();
    for component in relative
        .parent()
        .ok_or_else(|| crate::product_message!("backend.arxml.application.member_parent_missing"))?
        .components()
    {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() => super::project::safe_path(&current, false)?,
            Ok(_) => {
                return Err(crate::product_message!(
                    "backend.arxml.application.parent_not_directory",
                    "path" => current.display()
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current)
                    .map_err(|error| format!("{}: {error}", current.display()))?;
                created.push(current.clone());
                super::project::safe_path(&current, false)?;
            }
            Err(error) => return Err(format!("{}: {error}", current.display()).into()),
        }
    }
    Ok(())
}

fn stage_file(
    path: &Path,
    bytes: &[u8],
    created: &mut Vec<PathBuf>,
) -> Result<(), crate::message::LocalizedText> {
    let mut handle = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    created.push(path.to_owned());
    handle
        .write_all(bytes)
        .and_then(|_| handle.sync_all())
        .map_err(|error| format!("{}: {error}", path.display()).into())
}

fn remove_owned(path: &Path, bytes: &[u8]) -> Result<(), crate::message::LocalizedText> {
    if super::project::read_bounded(path)? != bytes {
        return Err(crate::product_message!(
            "backend.arxml.application.externally_changed_bytes_retained",
            "path" => path.display()
        ));
    }
    super::project::safe_path(path, false)?;
    fs::remove_file(path).map_err(|error| format!("{}: {error}", path.display()).into())
}

fn cleanup_stage(stage: &Path, files: &[PathBuf]) -> Vec<crate::message::LocalizedText> {
    let mut errors = Vec::new();
    for path in files.iter().rev() {
        if fs::symlink_metadata(path)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
        {
            continue;
        }
        if let Err(error) = super::project::safe_path(path, false)
            .and_then(|_| fs::remove_file(path).map_err(|error| error.to_string().into()))
        {
            errors.push(crate::message::LocalizedText::messages([
                crate::product_message!(
                    "backend.arxml.application.staging_path_retained",
                    "path" => path.display()
                ),
                error,
            ]));
        }
    }
    if let Err(error) = fs::remove_dir(stage) {
        errors.push(crate::message::LocalizedText::messages([
            crate::product_message!(
                "backend.arxml.application.staging_directory_retained",
                "path" => stage.display()
            ),
            error.to_string().into(),
        ]));
    }
    errors
}

impl Workspace {
    /// Derives a real application slot and seed exclusively from the saved native plan.
    pub fn preview_application_initialization(
        &self,
    ) -> Result<ApplicationInitializationPreview, crate::message::LocalizedText> {
        if self.uses_legacy_validation() {
            return Err(crate::product_message!(
                "backend.arxml.application.builtin_validation_required"
            ));
        }
        crate::rules::rule_set_identity()?;
        if self.is_dirty() {
            return Err(crate::product_message!(
                "backend.arxml.application.save_before_initialization"
            ));
        }
        self.verify_saved_sources()?;
        let project = self.project.as_ref().ok_or_else(|| {
            crate::product_message!("backend.arxml.application.portable_project_required")
        })?;
        if !project.manifest.application_inputs.is_empty() {
            return Err(crate::product_message!(
                "backend.arxml.application.live_ownership_already_declared"
            ));
        }
        if project.manifest.accepted_extension_definitions != self.catalog.accepted_extensions() {
            return Err(crate::product_message!(
                "backend.arxml.application.catalog_acceptance_mismatch"
            ));
        }
        let root = project.path.parent().ok_or_else(|| {
            crate::product_message!("backend.arxml.application.project_root_missing")
        })?;
        let runtime =
            crate::integration::RuntimeCatalog::embedded().map_err(super::integration_errors)?;
        let plan = self
            .saved_integration_plan(&runtime)
            .map_err(super::integration_errors)?;
        let slot = plan
            .application_slot_descriptor()
            .map_err(super::integration_errors)?;
        let seeds = plan
            .application_seed_files()
            .map_err(super::integration_errors)?;
        if slot.producer_slot != "epic4-single-application-v1"
            || seeds.len() != 1
            || slot.source_paths.len() != 1
            || seeds[0].0 != slot.source_paths[0]
        {
            return Err(crate::product_message!(
                "backend.arxml.application.producer_slot_contract_mismatch"
            ));
        }
        let mut manifest = project.manifest.clone();
        let mut files = Vec::with_capacity(seeds.len());
        for (path, bytes) in seeds {
            target(root, &path)?;
            manifest.application_inputs.push(ApplicationInput {
                path: path.clone(),
                producer_slot: slot.producer_slot.clone(),
            });
            files.push(ProjectFilePreview {
                path,
                contents: String::from_utf8(bytes).map_err(|_| {
                    crate::product_message!("backend.arxml.application.source_not_utf_eight")
                })?,
            });
        }
        super::project::validate_manifest(&manifest)?;
        let manifest_after = super::project::render_manifest(&manifest)?;
        let mut digest = Sha256::new();
        digest.update(b"native-application-initialization-v1\0");
        for value in [
            self.input_fingerprint()?,
            self.definition_fingerprint()?,
            project.saved.clone(),
            manifest_after.clone(),
            serde_json::to_string(&slot).map_err(|error| error.to_string())?,
        ] {
            digest.update((value.len() as u64).to_le_bytes());
            digest.update(value.as_bytes());
        }
        for file in &files {
            for value in [&file.path, &file.contents] {
                digest.update((value.len() as u64).to_le_bytes());
                digest.update(value.as_bytes());
            }
        }
        Ok(ApplicationInitializationPreview {
            revision: format!("{:x}", digest.finalize()),
            slot,
            files,
            manifest_before: project.saved.clone(),
            manifest_after,
        })
    }

    /// Creates absent live sources and their manifest membership as one reviewed transaction.
    pub fn initialize_application_previewed(
        &mut self,
        preview: &ApplicationInitializationPreview,
    ) -> Result<ApplicationInitializationOutcome, crate::message::LocalizedText> {
        let expected = self.preview_application_initialization()?;
        if expected != *preview {
            return Err(crate::product_message!(
                "backend.arxml.application.initialization_preview_stale"
            ));
        }
        let project = self.project.as_ref().ok_or_else(|| {
            crate::product_message!("backend.arxml.application.project_membership_missing")
        })?;
        let manifest_path = project.path.clone();
        let root = manifest_path
            .parent()
            .ok_or_else(|| {
                crate::product_message!("backend.arxml.application.project_root_missing")
            })?
            .to_owned();
        let mut membership = (*project).clone();
        membership.manifest =
            serde_json::from_str(&expected.manifest_after).map_err(|error| error.to_string())?;
        membership.current = expected.manifest_after.clone();
        membership.saved = expected.manifest_after.clone();
        for file in &expected.files {
            membership
                .application_bytes
                .insert(file.path.clone(), file.contents.as_bytes().to_vec());
        }
        let next_revision = self.revision.checked_add(1).ok_or_else(|| {
            crate::product_message!("backend.arxml.application.workspace_revision_exhausted")
        })?;
        let mut projection = self.project_projection(
            &self.input_fingerprint_for_project(next_revision, Some(&membership))?,
        )?;
        projection.dirty = false;
        let backup = PathBuf::from(format!("{}.autosar.bak", manifest_path.display()));
        absent(&backup)?;
        let stage = root.join(format!(
            ".autosar-application-{}",
            super::projection::new_epoch()
        ));
        fs::create_dir(&stage).map_err(|error| format!("{}: {error}", stage.display()))?;
        let mut staged = Vec::new();
        let mut created_directories = Vec::new();
        let mut installed = Vec::new();
        let mut backup_created = false;
        let mut manifest_removed = false;
        let result = (|| -> Result<(), crate::message::LocalizedText> {
            for (index, file) in expected.files.iter().enumerate() {
                let path = stage.join(format!("{index}.application"));
                stage_file(&path, file.contents.as_bytes(), &mut staged)?;
            }
            let staged_manifest = stage.join(MANIFEST);
            stage_file(
                &staged_manifest,
                expected.manifest_after.as_bytes(),
                &mut staged,
            )?;
            self.verify_saved_sources()?;
            for (index, file) in expected.files.iter().enumerate() {
                create_parents(&root, &file.path, &mut created_directories)?;
                let path = target(&root, &file.path)?;
                fs::hard_link(&staged[index], &path)
                    .map_err(|error| format!("{}: {error}", path.display()))?;
                installed.push((path, file.contents.as_bytes()));
            }
            self.verify_saved_sources()?;
            for (path, bytes) in &installed {
                if super::project::read_bounded(path)? != *bytes {
                    return Err(crate::product_message!(
                        "backend.arxml.application.bytes_changed_before_publication",
                        "path" => path.display()
                    ));
                }
            }
            super::project::safe_path(&manifest_path, false)?;
            // Both backup and publication are no-clobber links. A raced path is
            // refused rather than overwritten by platform-specific rename semantics.
            fs::hard_link(&manifest_path, &backup)
                .map_err(|error| format!("{}: {error}", backup.display()))?;
            backup_created = true;
            if super::project::read_bounded(&backup)? != expected.manifest_before.as_bytes() {
                return Err(crate::product_message!(
                    "backend.arxml.application.manifest_changed_before_backup_confirmation"
                ));
            }
            self.verify_saved_sources()?;
            fs::remove_file(&manifest_path).map_err(|error| error.to_string())?;
            manifest_removed = true;
            fs::hard_link(&staged_manifest, &manifest_path)
                .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
            Ok(())
        })();
        if let Err(error) = result {
            let mut failures = Vec::new();
            let mut restored = !manifest_removed;
            if manifest_removed {
                match fs::hard_link(&backup, &manifest_path) {
                    Ok(()) => restored = true,
                    Err(restore) => failures.push(crate::message::LocalizedText::messages([
                        crate::product_message!(
                            "backend.arxml.application.original_manifest_backup_retained",
                            "path" => backup.display()
                        ),
                        restore.to_string().into(),
                    ])),
                }
            }
            for (path, bytes) in installed.iter().rev() {
                if restored {
                    if let Err(rollback) = remove_owned(path, bytes) {
                        failures.push(rollback);
                    }
                } else {
                    failures.push(crate::product_message!(
                        "backend.arxml.application.application_retained_pending_recovery",
                        "path" => path.display()
                    ));
                }
            }
            if backup_created && !manifest_removed {
                if let Err(cleanup) = remove_owned(&backup, expected.manifest_before.as_bytes()) {
                    failures.push(cleanup);
                }
            } else if backup_created
                && super::project::read_bounded(&manifest_path)
                    .is_ok_and(|bytes| bytes == expected.manifest_before.as_bytes())
            {
                if let Err(cleanup) = remove_owned(&backup, expected.manifest_before.as_bytes()) {
                    failures.push(cleanup);
                }
            }
            failures.extend(cleanup_stage(&stage, &staged));
            for path in created_directories.iter().rev() {
                if let Err(cleanup) = fs::remove_dir(path) {
                    failures.push(crate::message::LocalizedText::messages([
                        crate::product_message!(
                            "backend.arxml.application.new_directory_retained",
                            "path" => path.display()
                        ),
                        cleanup.to_string().into(),
                    ]));
                }
            }
            return Err(crate::message::LocalizedText::messages([
                crate::product_message!("backend.arxml.application.initialization_failed"),
                error,
                crate::product_message!("backend.arxml.application.recovery_details"),
                crate::message::LocalizedText::messages(failures),
            ]));
        }
        // Publication succeeded. Cleanup cannot undo live membership or turn it
        // into a false failure; retained recovery paths are returned explicitly.
        let mut warnings = Vec::new();
        let mut retained_recovery_files = Vec::new();
        if let Err(error) = remove_owned(&backup, expected.manifest_before.as_bytes()) {
            warnings.push(error);
            retained_recovery_files.push(backup.display().to_string());
        }
        let stage_warnings = cleanup_stage(&stage, &staged);
        if !stage_warnings.is_empty() {
            retained_recovery_files.push(stage.display().to_string());
            warnings.extend(stage_warnings);
        }
        self.project = Some(membership);
        self.revision = next_revision;
        Ok(ApplicationInitializationOutcome {
            projection,
            warnings,
            retained_recovery_files,
        })
    }
}
