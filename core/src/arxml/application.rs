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
    pub warnings: Vec<String>,
    pub retained_recovery_files: Vec<String>,
}

fn absent(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{}: {error}", path.display())),
        Ok(_) => Err(format!(
            "Initialization never overwrites an existing path: {}",
            path.display()
        )),
    }
}

fn target(root: &Path, logical: &str) -> Result<PathBuf, String> {
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
                    return Err(format!(
                        "Initialization path is occupied: {}",
                        path.display()
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => missing = true,
            Err(error) => return Err(format!("{}: {error}", path.display())),
        }
    }
    Ok(path)
}

fn create_parents(root: &Path, logical: &str, created: &mut Vec<PathBuf>) -> Result<(), String> {
    let relative = super::project::relative_path(logical)?;
    let mut current = root.to_owned();
    for component in relative
        .parent()
        .ok_or("Application member has no parent.")?
        .components()
    {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() => super::project::safe_path(&current, false)?,
            Ok(_) => {
                return Err(format!(
                    "Application parent is not a directory: {}",
                    current.display()
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current)
                    .map_err(|error| format!("{}: {error}", current.display()))?;
                created.push(current.clone());
                super::project::safe_path(&current, false)?;
            }
            Err(error) => return Err(format!("{}: {error}", current.display())),
        }
    }
    Ok(())
}

fn stage_file(path: &Path, bytes: &[u8], created: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut handle = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    created.push(path.to_owned());
    handle
        .write_all(bytes)
        .and_then(|_| handle.sync_all())
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn remove_owned(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if super::project::read_bounded(path)? != bytes {
        return Err(format!(
            "Externally changed bytes are retained: {}",
            path.display()
        ));
    }
    super::project::safe_path(path, false)?;
    fs::remove_file(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn cleanup_stage(stage: &Path, files: &[PathBuf]) -> Vec<String> {
    let mut errors = Vec::new();
    for path in files.iter().rev() {
        if fs::symlink_metadata(path)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
        {
            continue;
        }
        if let Err(error) = super::project::safe_path(path, false)
            .and_then(|_| fs::remove_file(path).map_err(|error| error.to_string()))
        {
            errors.push(format!(
                "Private staging path retained at {}: {error}",
                path.display()
            ));
        }
    }
    if let Err(error) = fs::remove_dir(stage) {
        errors.push(format!(
            "Private staging directory retained at {}: {error}",
            stage.display()
        ));
    }
    errors
}

impl Workspace {
    /// Derives a real application slot and seed exclusively from the saved native plan.
    pub fn preview_application_initialization(
        &self,
    ) -> Result<ApplicationInitializationPreview, String> {
        if self.uses_legacy_validation() {
            return Err("Native application initialization requires builtin validation.".into());
        }
        crate::rules::rule_set_identity()?;
        if self.is_dirty() {
            return Err("Save the complete project before application initialization.".into());
        }
        self.verify_saved_sources()?;
        let project = self
            .project
            .as_ref()
            .ok_or("Save direct ARXML as a portable project before initializing an application.")?;
        if !project.manifest.application_inputs.is_empty() {
            return Err("The project already declares live application ownership; initialization will not overwrite it.".into());
        }
        if project.manifest.accepted_extension_definitions != self.catalog.accepted_extensions() {
            return Err("Current catalog acceptance differs from saved project membership.".into());
        }
        let root = project.path.parent().ok_or("Project root is missing.")?;
        let runtime = crate::integration::RuntimeCatalog::embedded()
            .map_err(|issues| format!("{issues:?}"))?;
        let plan = self
            .saved_integration_plan(&runtime)
            .map_err(|issues| format!("{issues:?}"))?;
        let slot = plan
            .application_slot_descriptor()
            .map_err(|issues| format!("{issues:?}"))?;
        let seeds = plan
            .application_seed_files()
            .map_err(|issues| format!("{issues:?}"))?;
        if slot.producer_slot != "epic4-single-application-v1"
            || seeds.len() != 1
            || slot.source_paths.len() != 1
            || seeds[0].0 != slot.source_paths[0]
        {
            return Err(
                "Trusted application producer does not match the supported v1 slot contract."
                    .into(),
            );
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
                contents: String::from_utf8(bytes)
                    .map_err(|_| "Trusted application source is not UTF-8.")?,
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
    ) -> Result<ApplicationInitializationOutcome, String> {
        let expected = self.preview_application_initialization()?;
        if expected != *preview {
            return Err(
                "Application initialization preview is stale or modified; preview again.".into(),
            );
        }
        let project = self
            .project
            .as_ref()
            .ok_or("Project membership is missing.")?;
        let manifest_path = project.path.clone();
        let root = manifest_path
            .parent()
            .ok_or("Project root is missing.")?
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
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or("Workspace revision is exhausted.")?;
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
        let result = (|| -> Result<(), String> {
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
                    return Err(format!(
                        "New application bytes changed before manifest publication: {}",
                        path.display()
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
                return Err("Project manifest changed before backup confirmation.".into());
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
                    Err(restore) => failures.push(format!(
                        "Original manifest backup retained at {}: {restore}",
                        backup.display()
                    )),
                }
            }
            for (path, bytes) in installed.iter().rev() {
                if restored {
                    if let Err(rollback) = remove_owned(path, bytes) {
                        failures.push(rollback);
                    }
                } else {
                    failures.push(format!(
                        "New application retained until manifest recovery: {}",
                        path.display()
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
                    failures.push(format!(
                        "New directory retained at {}: {cleanup}",
                        path.display()
                    ));
                }
            }
            return Err(format!(
                "Application initialization failed: {error}; recovery: {}",
                failures.join("; ")
            ));
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
