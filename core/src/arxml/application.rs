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
    pub slots: Vec<ApplicationSlotDescriptor>,
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

fn remove_owned(
    path: &Path,
    bytes: &[u8],
    recovery: &Path,
    publish: impl Fn(&Path, &Path) -> std::io::Result<()>,
) -> Result<(), crate::message::LocalizedText> {
    // Capture before checking bytes so a replacement at the live path is never unlinked.
    let reservation = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(recovery)
        .map_err(|error| format!("{}: {error}", recovery.display()))?;
    drop(reservation);
    if let Err(error) = super::persistence::capture_file(path, recovery) {
        fs::remove_file(recovery)
            .map_err(|cleanup| format!("{}: {cleanup}", recovery.display()))?;
        return Err(format!("{}: {error}", path.display()).into());
    }
    if super::project::read_bounded(recovery).is_ok_and(|actual| actual == bytes) {
        return fs::remove_file(recovery)
            .map_err(|error| format!("{}: {error}", recovery.display()).into());
    }
    let retained = crate::product_message!(
        "backend.arxml.application.externally_changed_bytes_retained",
        "path" => path.display()
    );
    match publish(recovery, path) {
        Ok(()) => {
            fs::remove_file(recovery)
                .map_err(|error| format!("{}: {error}", recovery.display()))?;
            Err(retained)
        }
        Err(error) => Err(crate::message::LocalizedText::messages([
            retained,
            format!("{} -> {}: {error}", recovery.display(), path.display()).into(),
        ])),
    }
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
        let slots = plan
            .application_slot_descriptors()
            .map_err(super::integration_errors)?;
        let seeds = plan
            .application_seed_files()
            .map_err(super::integration_errors)?;
        if slots.is_empty()
            || slots
                .iter()
                .map(|slot| slot.source_paths.len())
                .sum::<usize>()
                != seeds.len()
            || slots
                .iter()
                .flat_map(|slot| &slot.source_paths)
                .any(|path| {
                    seeds
                        .iter()
                        .filter(|(seed_path, _)| seed_path == path)
                        .count()
                        != 1
                })
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
                producer_slot: slots
                    .iter()
                    .find(|slot| slot.source_paths.contains(&path))
                    .unwrap()
                    .producer_slot
                    .clone(),
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
            serde_json::to_string(&slots).map_err(|error| error.to_string())?,
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
            slots,
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
        self.initialize_application_with_publish(preview, |from, to| fs::hard_link(from, to))
    }

    fn initialize_application_with_publish(
        &mut self,
        preview: &ApplicationInitializationPreview,
        publish: impl Fn(&Path, &Path) -> std::io::Result<()>,
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
        let manifest_published = std::cell::Cell::new(false);
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
                publish(&staged[index], &path)
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
            let manifest = SourceFile {
                path: manifest_path.clone(),
                saved: expected.manifest_before.clone(),
                text: expected.manifest_after.clone(),
                original_name: None,
            };
            let verify_members = || -> Result<(), crate::message::LocalizedText> {
                for file in &self.files {
                    if super::project::read_bounded(&file.path)? != file.saved.as_bytes() {
                        return Err(crate::product_message!(
                            "backend.arxml.application.bytes_changed_before_publication",
                            "path" => file.path.display()
                        ));
                    }
                }
                for (path, bytes) in &installed {
                    if super::project::read_bounded(path)? != *bytes {
                        return Err(crate::product_message!(
                            "backend.arxml.application.bytes_changed_before_publication",
                            "path" => path.display()
                        ));
                    }
                }
                Ok(())
            };
            super::persistence::install_staged_with_publish(
                &manifest,
                &staged_manifest,
                &backup,
                |from, to| {
                    if to == manifest_path {
                        verify_members()
                            .map_err(|error| std::io::Error::other(error.to_string()))?;
                    }
                    publish(from, to)?;
                    if to == manifest_path {
                        manifest_published.set(true);
                        verify_members()
                            .map_err(|error| std::io::Error::other(error.to_string()))?;
                        if super::project::read_bounded(to)
                            .map_err(|error| std::io::Error::other(error.to_string()))?
                            != expected.manifest_after.as_bytes()
                        {
                            return Err(std::io::Error::other(
                                crate::product_message!(
                                    "backend.arxml.application.bytes_changed_before_publication",
                                    "path" => manifest_path.display()
                                )
                                .to_string(),
                            ));
                        }
                    }
                    Ok(())
                },
            )?;
            Ok(())
        })();
        if let Err(error) = result {
            let mut failures = Vec::new();
            let manifest_restored = !manifest_published.get()
                || super::project::read_bounded(&manifest_path)
                    .is_ok_and(|bytes| bytes == expected.manifest_before.as_bytes());
            for (index, (path, bytes)) in installed.iter().enumerate().rev() {
                if manifest_restored {
                    if let Err(rollback) = remove_owned(
                        path,
                        bytes,
                        &stage.join(format!("{index}.rollback")),
                        &publish,
                    ) {
                        failures.push(rollback);
                    }
                } else {
                    failures.push(crate::product_message!(
                        "backend.arxml.application.application_retained_pending_recovery",
                        "path" => path.display()
                    ));
                }
            }
            if !manifest_restored && fs::symlink_metadata(&backup).is_ok() {
                failures.push(crate::product_message!(
                    "backend.arxml.application.original_manifest_backup_retained",
                    "path" => backup.display()
                ));
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
        let captured_backup = stage.join("manifest.rollback");
        if let Err(error) = remove_owned(
            &backup,
            expected.manifest_before.as_bytes(),
            &captured_backup,
            &publish,
        ) {
            warnings.push(error);
            for path in [&backup, &captured_backup] {
                if fs::symlink_metadata(path).is_ok() {
                    retained_recovery_files.push(path.display().to_string());
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "autosar-application-{}-{}",
                std::process::id(),
                super::super::projection::new_epoch()
            ));
            fs::create_dir(&root).unwrap();
            Self(root)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn multi_workspace(root: &Path) -> Workspace {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi-component");
        let mut inputs = Vec::new();
        for entry in fs::read_dir(fixtures).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_str().unwrap().to_owned();
            fs::copy(entry.path(), root.join(&name)).unwrap();
            inputs.push(ProjectInput {
                path: name,
                role_hint: "standard".into(),
            });
        }
        inputs.sort_by(|left, right| left.path.cmp(&right.path));
        let manifest = ProjectManifest {
            format_version: 1,
            declared_release: "R24-11".into(),
            profile_hint: crate::integration::MULTI_PROFILE.into(),
            inputs,
            application_inputs: Vec::new(),
            accepted_extension_definitions: Vec::new(),
        };
        let path = root.join(MANIFEST);
        fs::write(
            &path,
            super::super::project::render_manifest(&manifest).unwrap(),
        )
        .unwrap();
        Workspace::open_project_manifest(&path, &root.join("cache")).unwrap()
    }

    #[test]
    fn later_application_collision_rolls_back_earlier_members() {
        let scratch = Scratch::new();
        let mut workspace = multi_workspace(&scratch.0);
        let preview = workspace.preview_application_initialization().unwrap();
        assert_eq!(preview.slots.len(), 3);
        let before = workspace.input_fingerprint().unwrap();
        let collision = scratch.0.join(&preview.files[1].path);
        let error = workspace
            .initialize_application_with_publish(&preview, |from, to| {
                if to == collision {
                    fs::write(to, "external source at publication")?;
                }
                fs::hard_link(from, to)
            })
            .unwrap_err();
        assert!(error.to_string().contains(&collision.display().to_string()));
        assert_eq!(
            fs::read_to_string(&collision).unwrap(),
            "external source at publication"
        );
        assert!(!scratch.0.join(&preview.files[0].path).exists());
        assert!(!scratch.0.join(&preview.files[2].path).exists());
        assert_eq!(
            fs::read_to_string(scratch.0.join(MANIFEST)).unwrap(),
            preview.manifest_before
        );
        assert_eq!(workspace.input_fingerprint().unwrap(), before);
        assert!(
            workspace
                .project_manifest()
                .unwrap()
                .application_inputs
                .is_empty()
        );
    }

    #[test]
    fn manifest_replacements_preserve_external_bytes_and_original_recovery() {
        for before_capture in [true, false] {
            let scratch = Scratch::new();
            let mut workspace = multi_workspace(&scratch.0);
            let preview = workspace.preview_application_initialization().unwrap();
            let before = workspace.input_fingerprint().unwrap();
            let manifest = scratch.0.join(MANIFEST);
            let backup = PathBuf::from(format!("{}.autosar.bak", manifest.display()));
            let error = workspace
                .initialize_application_with_publish(&preview, |from, to| {
                    if (before_capture && to == backup) || (!before_capture && to == manifest) {
                        fs::remove_file(&manifest).or_else(|error| {
                            if error.kind() == std::io::ErrorKind::NotFound {
                                Ok(())
                            } else {
                                Err(error)
                            }
                        })?;
                        fs::write(&manifest, "external manifest replacement")?;
                    }
                    fs::hard_link(from, to)
                })
                .unwrap_err();
            assert_eq!(
                fs::read_to_string(&manifest).unwrap(),
                "external manifest replacement"
            );
            for file in &preview.files {
                assert!(!scratch.0.join(&file.path).exists());
            }
            assert_eq!(workspace.input_fingerprint().unwrap(), before);
            assert_eq!(
                workspace.project.as_ref().unwrap().saved,
                preview.manifest_before
            );
            if before_capture {
                assert!(!backup.exists());
            } else {
                assert_eq!(
                    fs::read_to_string(&backup).unwrap(),
                    preview.manifest_before
                );
                assert!(error.to_string().contains(&backup.display().to_string()));
            }
        }
    }

    #[test]
    fn source_replacements_before_or_during_manifest_publication_abort_membership() {
        for (after_publication, in_place) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let scratch = Scratch::new();
            let mut workspace = multi_workspace(&scratch.0);
            let preview = workspace.preview_application_initialization().unwrap();
            let baseline = workspace.input_fingerprint().unwrap();
            let source = scratch.0.join(&preview.files[1].path);
            let manifest = scratch.0.join(MANIFEST);
            let backup = PathBuf::from(format!("{}.autosar.bak", manifest.display()));
            workspace
                .initialize_application_with_publish(&preview, |from, to| {
                    fs::hard_link(from, to)?;
                    if (!after_publication && to == backup) || (after_publication && to == manifest)
                    {
                        if !in_place {
                            fs::remove_file(&source)?;
                        }
                        fs::write(&source, "external source replacement")?;
                    }
                    Ok(())
                })
                .unwrap_err();
            assert_eq!(
                fs::read_to_string(&source).unwrap(),
                "external source replacement"
            );
            assert_eq!(
                fs::read_to_string(&manifest).unwrap(),
                preview.manifest_before
            );
            for file in preview
                .files
                .iter()
                .filter(|file| scratch.0.join(&file.path) != source)
            {
                assert!(!scratch.0.join(&file.path).exists());
            }
            assert_eq!(workspace.input_fingerprint().unwrap(), baseline);
            assert_eq!(
                workspace.project.as_ref().unwrap().saved,
                preview.manifest_before
            );
            assert!(
                workspace
                    .project_manifest()
                    .unwrap()
                    .application_inputs
                    .is_empty()
            );
        }
    }

    #[test]
    fn failed_manifest_restoration_retains_every_referenced_source_and_original_baseline() {
        let scratch = Scratch::new();
        let mut workspace = multi_workspace(&scratch.0);
        let preview = workspace.preview_application_initialization().unwrap();
        let baseline = workspace.input_fingerprint().unwrap();
        let manifest = scratch.0.join(MANIFEST);
        let backup = PathBuf::from(format!("{}.autosar.bak", manifest.display()));
        let recovery = backup.with_extension("rollback");
        let source = scratch.0.join(&preview.files[1].path);
        let error = workspace
            .initialize_application_with_publish(&preview, |from, to| {
                fs::hard_link(from, to)?;
                if to == manifest {
                    fs::create_dir(&recovery)?;
                    fs::write(recovery.join("external"), "external recovery evidence")?;
                    fs::write(&source, "external in-place source edit")?;
                }
                Ok(())
            })
            .unwrap_err();
        assert_eq!(
            fs::read_to_string(&manifest).unwrap(),
            preview.manifest_after
        );
        let published: ProjectManifest =
            serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
        assert_eq!(published.application_inputs.len(), 3);
        for member in &published.application_inputs {
            let path = scratch.0.join(&member.path);
            assert!(path.is_file());
            assert!(error.to_string().contains(&path.display().to_string()));
            if path != source {
                let expected = preview
                    .files
                    .iter()
                    .find(|file| file.path == member.path)
                    .unwrap();
                assert_eq!(fs::read_to_string(path).unwrap(), expected.contents);
            }
        }
        assert_eq!(
            fs::read_to_string(source).unwrap(),
            "external in-place source edit"
        );
        assert_eq!(
            fs::read_to_string(&backup).unwrap(),
            preview.manifest_before
        );
        assert_eq!(
            fs::read_to_string(recovery.join("external")).unwrap(),
            "external recovery evidence"
        );
        assert!(error.to_string().contains(&backup.display().to_string()));
        assert!(error.to_string().contains(&recovery.display().to_string()));
        assert_eq!(workspace.input_fingerprint().unwrap(), baseline);
        assert_eq!(
            workspace.project.as_ref().unwrap().saved,
            preview.manifest_before
        );
        assert!(
            workspace
                .project_manifest()
                .unwrap()
                .application_inputs
                .is_empty()
        );
    }

    #[test]
    fn successful_publication_reports_actual_captured_backup_recovery_path() {
        use std::cell::Cell;
        let scratch = Scratch::new();
        let mut workspace = multi_workspace(&scratch.0);
        let preview = workspace.preview_application_initialization().unwrap();
        let manifest = scratch.0.join(MANIFEST);
        let backup = PathBuf::from(format!("{}.autosar.bak", manifest.display()));
        let backup_calls = Cell::new(0);
        let outcome = workspace
            .initialize_application_with_publish(&preview, |from, to| {
                if to == backup {
                    backup_calls.set(backup_calls.get() + 1);
                    if backup_calls.get() == 2 {
                        fs::write(to, "external recovery collision")?;
                    }
                }
                fs::hard_link(from, to)?;
                if to == manifest {
                    fs::remove_file(&backup)?;
                    fs::write(&backup, "external backup replacement")?;
                }
                Ok(())
            })
            .unwrap();
        let captured = outcome
            .retained_recovery_files
            .iter()
            .find(|path| path.ends_with("manifest.rollback"))
            .unwrap();
        assert_eq!(
            fs::read_to_string(captured).unwrap(),
            "external backup replacement"
        );
        assert_eq!(
            fs::read_to_string(&backup).unwrap(),
            "external recovery collision"
        );
        assert!(
            outcome
                .retained_recovery_files
                .iter()
                .all(|path| fs::symlink_metadata(path).is_ok())
        );
        assert!(!outcome.warnings.is_empty());
        assert!(!outcome.projection.dirty);
        assert_eq!(
            fs::read_to_string(&manifest).unwrap(),
            preview.manifest_after
        );
        workspace.ensure_sources_current().unwrap();
        assert_eq!(
            workspace.project.as_ref().unwrap().saved,
            preview.manifest_after
        );
    }

    #[test]
    fn rollback_preserves_a_directory_replacing_a_source() {
        let scratch = Scratch::new();
        let source = scratch.0.join("Application.c");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("external"), "external directory data").unwrap();
        assert!(
            remove_owned(&source, b"ours", &scratch.0.join("rollback"), |from, to| {
                fs::hard_link(from, to)
            })
            .is_err()
        );
        assert_eq!(
            fs::read_to_string(source.join("external")).unwrap(),
            "external directory data"
        );
        assert!(!scratch.0.join("rollback").exists());
    }
}
