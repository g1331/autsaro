use super::*;
use crate::project_model::*;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::Arc;

const MANIFEST: &str = "workbench-project.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectInput {
    pub path: String,
    pub role_hint: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationInput {
    pub path: String,
    pub producer_slot: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectManifest {
    pub format_version: u32,
    pub declared_release: String,
    pub profile_hint: String,
    pub inputs: Vec<ProjectInput>,
    pub application_inputs: Vec<ApplicationInput>,
    pub accepted_extension_definitions: Vec<ExtensionDefinitionIdentity>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerationInputSnapshot {
    pub logical_path: String,
    pub disk_path: PathBuf,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerationSnapshot {
    pub manifest: ProjectManifest,
    pub manifest_bytes: Vec<u8>,
    pub manifest_path: Option<PathBuf>,
    pub project_root: Option<PathBuf>,
    pub inputs: Vec<GenerationInputSnapshot>,
    pub applications: Vec<GenerationInputSnapshot>,
}

#[derive(Clone)]
pub(super) struct ProjectMembership {
    pub path: PathBuf,
    pub manifest: ProjectManifest,
    pub saved: String,
    pub current: String,
    pub application_bytes: BTreeMap<String, Vec<u8>>,
    pub extension_diagnostics: Vec<ConfigurationDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectFilePreview {
    pub path: String,
    pub contents: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectCreationPreview {
    pub revision: String,
    pub template_id: String,
    pub directory: String,
    pub name: String,
    pub files: Vec<ProjectFilePreview>,
    pub accepted_extension_definitions: Vec<ExtensionDefinitionIdentity>,
}

pub(super) fn safe_path(
    path: &Path,
    allow_missing_leaf: bool,
) -> Result<(), crate::message::LocalizedText> {
    if path
        .components()
        .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(crate::product_message!(
            "backend.arxml.project.parent_traversal_forbidden"
        ));
    }
    for ancestor in path.ancestors() {
        let metadata = match fs::symlink_metadata(ancestor) {
            Ok(metadata) => metadata,
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && allow_missing_leaf
                    && ancestor == path =>
            {
                continue;
            }
            Err(error) => return Err(format!("{}: {error}", ancestor.display()).into()),
        };
        #[cfg(windows)]
        let linked = {
            use std::os::windows::fs::MetadataExt;
            metadata.file_attributes() & 0x400 != 0
        };
        #[cfg(not(windows))]
        let linked = metadata.file_type().is_symlink();
        if linked {
            return Err(crate::product_message!(
                "backend.arxml.project.link_not_accepted",
                "path" => ancestor.display()
            ));
        }
    }
    Ok(())
}

pub(super) fn read_bounded(path: &Path) -> Result<Vec<u8>, crate::message::LocalizedText> {
    use std::io::Read;
    safe_path(path, false)?;
    #[cfg(feature = "verification-metrics")]
    crate::verification::before(crate::verification::Phase::SourceRead);
    let file = fs::File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > 50 * 1024 * 1024 {
        return Err(crate::product_message!(
            "backend.arxml.project.input_file_size_limit",
            "path" => path.display()
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(50 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > 50 * 1024 * 1024 {
        return Err(crate::product_message!(
            "backend.arxml.project.input_grew_beyond_limit",
            "path" => path.display()
        ));
    }
    Ok(bytes)
}

pub(super) fn relative_path(value: &str) -> Result<&Path, crate::message::LocalizedText> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || value.contains(':')
        || path.is_absolute()
        || value.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || part.ends_with(['.', ' '])
                || part
                    .chars()
                    .any(|ch| ch.is_control() || matches!(ch, '<' | '>' | '"' | '|' | '?' | '*'))
        })
        || !path
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
    {
        return Err(crate::product_message!(
            "backend.arxml.project.member_path_must_be_safe",
            "path" => value
        ));
    }
    Ok(path)
}

fn destination(directory: &Path) -> Result<PathBuf, crate::message::LocalizedText> {
    let selected = if directory.is_absolute() {
        directory.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(directory)
    };
    safe_path(&selected, true)?;
    if selected.exists() {
        if !selected.is_dir()
            || fs::read_dir(&selected)
                .map_err(|error| error.to_string())?
                .next()
                .is_some()
        {
            return Err(crate::product_message!(
                "backend.arxml.project.destination_must_be_new_or_empty"
            ));
        }
        fs::canonicalize(selected).map_err(|error| error.to_string().into())
    } else {
        let parent = selected.parent().ok_or_else(|| {
            crate::product_message!("backend.arxml.project.destination_parent_missing")
        })?;
        let name = selected.file_name().ok_or_else(|| {
            crate::product_message!("backend.arxml.project.destination_name_missing")
        })?;
        Ok(fs::canonicalize(parent)
            .map_err(|error| error.to_string())?
            .join(name))
    }
}

pub(super) fn render_manifest(
    manifest: &ProjectManifest,
) -> Result<String, crate::message::LocalizedText> {
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(manifest).map_err(|error| error.to_string())?
    ))
}

pub(super) fn validate_manifest(
    manifest: &ProjectManifest,
) -> Result<(), crate::message::LocalizedText> {
    if manifest.format_version != 1
        || manifest.declared_release != "R24-11"
        || manifest.inputs.is_empty()
    {
        return Err(crate::product_message!(
            "backend.arxml.project.manifest_version_release_or_inputs_invalid"
        ));
    }
    let mut paths = BTreeSet::new();
    for input in &manifest.inputs {
        let path = relative_path(&input.path)?;
        if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("arxml"))
            || !paths.insert(input.path.to_ascii_lowercase())
        {
            return Err(crate::product_message!(
                "backend.arxml.project.arxml_paths_must_be_unique"
            ));
        }
    }
    let mut slots = BTreeSet::new();
    for input in &manifest.application_inputs {
        relative_path(&input.path)?;
        if input.producer_slot.is_empty()
            || !slots.insert(&input.producer_slot)
            || !paths.insert(input.path.to_ascii_lowercase())
        {
            return Err(crate::product_message!(
                "backend.arxml.project.application_slot_or_path_invalid"
            ));
        }
    }
    let mut catalogs = BTreeSet::new();
    let mut previous = None;
    for identity in &manifest.accepted_extension_definitions {
        if identity.release != "R24-11"
            || identity.version.is_empty()
            || identity.catalog_id.is_empty()
            || identity.sha256.len() != 64
            || !identity.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            || !catalogs.insert(&identity.catalog_id)
            || previous.is_some_and(|id: &String| id >= &identity.catalog_id)
        {
            return Err(crate::product_message!(
                "backend.arxml.project.accepted_catalog_identities_invalid"
            ));
        }
        previous = Some(&identity.catalog_id);
    }
    Ok(())
}

fn template_files(
    name: &str,
    template: &str,
) -> Result<(String, Vec<ProjectFilePreview>, Vec<ProjectInput>), crate::message::LocalizedText> {
    if !valid_name(name) {
        return Err(crate::product_message!(
            "backend.arxml.project.project_name_constraints"
        ));
    }
    let mut files = Vec::new();
    let profile;
    match template {
        "can-empty-v1" | "can-signals-v1" => {
            profile = "host-can-v1".to_owned();
            let (frames, signals) = if template == "can-signals-v1" {
                let frame_path = format!("/{name}/Pdu_Command");
                (
                    vec![FrameView {
                        path: frame_path.clone(),
                        name: "Command".into(),
                        id: 0x321,
                        dlc: 4,
                        direction: Direction::Tx,
                        period_ms: Some(10),
                        timeout_ms: None,
                    }],
                    vec![SignalView {
                        path: format!("/{name}/ComCfg/ComConfig/Value"),
                        name: "Value".into(),
                        frame_path,
                        start_bit: 0,
                        length: 32,
                        initial_value: 0,
                    }],
                )
            } else {
                (Vec::new(), Vec::new())
            };
            files.push(ProjectFilePreview {
                path: format!("{name}.arxml"),
                contents: render_profile(name, &frames, &signals, None),
            });
        }
        "standard-ecu-v1" => {
            profile = crate::integration::PROFILE.into();
            // These seven fixtures are product-original source, not official archive payloads.
            let originals = [
                (
                    "extract.arxml",
                    include_str!("../../tests/fixtures/epic4/positive/extract.arxml"),
                ),
                (
                    "types.arxml",
                    include_str!("../../tests/fixtures/epic4/positive/types.arxml"),
                ),
                (
                    "application.arxml",
                    include_str!("../../tests/fixtures/epic4/positive/application.arxml"),
                ),
                (
                    "bsw.arxml",
                    include_str!("../../tests/fixtures/epic4/positive/bsw.arxml"),
                ),
                (
                    "ecuc.arxml",
                    include_str!("../../tests/fixtures/epic4/positive/ecuc.arxml"),
                ),
                (
                    "services.arxml",
                    include_str!("../../tests/fixtures/epic4/positive/services.arxml"),
                ),
                (
                    "unrelated.arxml",
                    include_str!("../../tests/fixtures/epic4/positive/unrelated.arxml"),
                ),
            ];
            for (path, original) in originals {
                let document = Document::parse(original).map_err(|error| error.to_string())?;
                let mut patches = Vec::new();
                for node in document.descendants().filter(|node| node.is_element()) {
                    if node.tag_name().name() == "ECU-INSTANCE"
                        && child_text(node, "SHORT-NAME").as_deref() == Some("ReferenceEcu")
                    {
                        patch_child(node, "SHORT-NAME", name.into(), &mut patches)?;
                    } else if node.tag_name().name().ends_with("-REF") {
                        if let Some(path) = node
                            .text()
                            .and_then(|value| value.strip_prefix("/Extract/ReferenceEcu"))
                        {
                            if path.is_empty() || path.starts_with('/') {
                                patches.push(Patch {
                                    range: super::projection::simple_text_range(node)?,
                                    value: format!("/Extract/{name}{path}"),
                                });
                            }
                        }
                    }
                }
                let mut contents = original.to_owned();
                apply_patches(&mut contents, &mut patches)?;
                if path == "ecuc.arxml" {
                    super::standard_template::complete(&mut contents)?;
                }
                files.push(ProjectFilePreview {
                    path: path.into(),
                    contents,
                });
            }
        }
        _ => {
            return Err(crate::product_message!(
                "backend.arxml.project.unknown_template_id"
            ));
        }
    }
    let inputs = files
        .iter()
        .map(|file| ProjectInput {
            path: file.path.clone(),
            role_hint: "source".into(),
        })
        .collect();
    Ok((profile, files, inputs))
}

fn preview_revision(
    preview: &ProjectCreationPreview,
    input: &str,
    catalog: &crate::definitions::DefinitionCatalog,
) -> Result<String, crate::message::LocalizedText> {
    let mut content = preview.clone();
    content.revision.clear();
    let mut digest = Sha256::new();
    digest.update(serde_json::to_vec(&content).map_err(|error| error.to_string())?);
    digest.update(input.as_bytes());
    let identity = crate::rules::rule_set_identity()?;
    digest.update(serde_json::to_vec(&identity).map_err(|error| error.to_string())?);
    digest.update(catalog.fingerprint(&identity).as_bytes());
    Ok(format!("{:x}", digest.finalize()))
}

fn from_preview(
    preview: &ProjectCreationPreview,
    catalog: Arc<crate::definitions::DefinitionCatalog>,
    require_definition: bool,
) -> Result<Workspace, crate::message::LocalizedText> {
    let root = PathBuf::from(&preview.directory);
    let manifest_file = preview
        .files
        .iter()
        .find(|file| file.path == MANIFEST)
        .ok_or_else(|| crate::product_message!("backend.arxml.project.preview_manifest_missing"))?;
    let manifest: ProjectManifest =
        serde_json::from_str(&manifest_file.contents).map_err(|error| error.to_string())?;
    validate_manifest(&manifest)?;
    let mut files = Vec::new();
    for input in &manifest.inputs {
        let file = preview
            .files
            .iter()
            .find(|file| file.path == input.path)
            .ok_or_else(|| {
                crate::product_message!("backend.arxml.project.preview_arxml_member_missing")
            })?;
        if file.contents.len() > 50 * 1024 * 1024
            || file.contents.contains("<!DOCTYPE")
            || file.contents.contains("<!ENTITY")
        {
            return Err(crate::product_message!(
                "backend.arxml.project.unsafe_source"
            ));
        }
        let document = Document::parse(&file.contents).map_err(|error| error.to_string())?;
        if document.root_element().tag_name().namespace() != Some(NS)
            || document.root_element().tag_name().name() != "AUTOSAR"
        {
            return Err(crate::product_message!(
                "backend.arxml.project.source_not_autosar_xml"
            ));
        }
        files.push(SourceFile {
            path: root.join(&input.path),
            text: file.contents.clone(),
            saved: file.contents.clone(),
            original_name: None,
        });
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let name = files
        .iter()
        .filter_map(|file| Document::parse(&file.text).ok())
        .find_map(|document| {
            document
                .descendants()
                .find(|node| node.is_element() && node.tag_name().name() == "AR-PACKAGE")
                .and_then(|node| child_text(node, "SHORT-NAME"))
        })
        .unwrap_or_else(|| "ImportedEcu".into());
    let application_bytes = manifest
        .application_inputs
        .iter()
        .map(|input| {
            preview
                .files
                .iter()
                .find(|file| file.path == input.path)
                .map(|file| (input.path.clone(), file.contents.as_bytes().to_vec()))
                .ok_or_else(|| {
                    crate::product_message!(
                        "backend.arxml.project.preview_application_member_missing"
                    )
                })
        })
        .collect::<Result<_, _>>()?;
    let mut workspace = Workspace {
        name,
        files,
        frames: Vec::new(),
        signals: Vec::new(),
        diagnostic: None,
        issues: Vec::new(),
        schema_zip: None,
        integration_input_root: Some(root.clone()),
        catalog,
        snapshot: Arc::default(),
        epoch: super::projection::new_epoch(),
        next_identity: 0,
        revision: 0,
        project: Some(ProjectMembership {
            path: root.join(MANIFEST),
            manifest,
            saved: manifest_file.contents.clone(),
            current: manifest_file.contents.clone(),
            application_bytes,
            extension_diagnostics: Vec::new(),
        }),
    };
    workspace.refresh()?;
    for scope in &workspace.snapshot.validation {
        if scope.scope == ValidationScope::Schema
            || (require_definition && scope.scope == ValidationScope::Definition)
        {
            if let Some(issue) = scope
                .diagnostics
                .iter()
                .find(|issue| matches!(issue.severity, Severity::Error))
            {
                return Err(crate::message::LocalizedText::messages([
                    crate::product_message!(
                        "backend.arxml.project.preview_validation_failed",
                        "code" => &issue.code
                    ),
                    issue.message.clone(),
                ]));
            }
        }
    }
    Ok(workspace)
}

fn publish(
    preview: &ProjectCreationPreview,
    workspace: Workspace,
) -> Result<Workspace, crate::message::LocalizedText> {
    let target = destination(Path::new(&preview.directory))?;
    let parent = target
        .parent()
        .ok_or_else(|| crate::product_message!("backend.arxml.project.directory_parent_missing"))?;
    let stage = parent.join(format!(
        ".autosar-project-{}",
        super::projection::new_epoch()
    ));
    fs::create_dir(&stage).map_err(|error| error.to_string())?;
    let result = (|| -> Result<(), crate::message::LocalizedText> {
        for file in &preview.files {
            let relative = relative_path(&file.path)?;
            let path = stage.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let mut handle = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|error| error.to_string())?;
            handle
                .write_all(file.contents.as_bytes())
                .and_then(|_| handle.sync_all())
                .map_err(|error| error.to_string())?;
        }
        // Recheck immediately before atomic publication. An existing empty selected directory is removed only if still empty.
        destination(&target)?;
        if target.exists() {
            fs::remove_dir(&target).map_err(|error| error.to_string())?;
        }
        fs::rename(&stage, &target).map_err(|error| error.to_string())?;
        Ok(())
    })();
    if let Err(error) = result {
        // Stage is privately owned and never accepted as a finished project.
        let cleanup = fs::remove_dir_all(&stage);
        return Err(match cleanup {
            Ok(()) => error,
            Err(cleanup) => crate::message::LocalizedText::messages([
                error,
                crate::product_message!("backend.arxml.project.private_staging_cleanup_failed"),
                cleanup.to_string().into(),
            ]),
        });
    }
    Ok(workspace)
}

impl Workspace {
    /// Captures only saved, owned members after checking the live source boundary.
    pub fn generation_snapshot(&self) -> Result<GenerationSnapshot, crate::message::LocalizedText> {
        if self.uses_legacy_validation() {
            return Err(crate::product_message!(
                "backend.arxml.project.generation_requires_builtin_validation"
            ));
        }
        crate::rules::rule_set_identity()?;
        if self.is_dirty() {
            return Err(crate::product_message!(
                "backend.arxml.project.save_before_native_delivery"
            ));
        }
        self.verify_saved_sources()?;
        let accepted = self.catalog.accepted_extensions();
        let (manifest, manifest_bytes, manifest_path, root) = if let Some(project) = &self.project {
            validate_manifest(&project.manifest)?;
            if project.manifest.accepted_extension_definitions != accepted {
                return Err(crate::product_message!(
                    "backend.arxml.project.saved_catalog_acceptance_mismatch"
                ));
            }
            (
                project.manifest.clone(),
                project.saved.as_bytes().to_vec(),
                Some(project.path.clone()),
                Some(
                    project
                        .path
                        .parent()
                        .ok_or_else(|| {
                            crate::product_message!("backend.arxml.project.project_root_missing")
                        })?
                        .to_owned(),
                ),
            )
        } else {
            let mut names = BTreeSet::new();
            let mut inputs = Vec::with_capacity(self.files.len());
            for file in &self.files {
                let name = file
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| {
                        crate::product_message!("backend.arxml.project.source_name_not_utf_eight")
                    })?;
                relative_path(name)?;
                if !names.insert(name.to_ascii_lowercase()) {
                    return Err(crate::product_message!(
                        "backend.arxml.project.direct_source_names_collide"
                    ));
                }
                inputs.push(ProjectInput {
                    path: name.into(),
                    role_hint: "source".into(),
                });
            }
            let manifest = ProjectManifest {
                format_version: 1,
                declared_release: "R24-11".into(),
                profile_hint: self.snapshot.profile.clone(),
                inputs,
                application_inputs: Vec::new(),
                accepted_extension_definitions: accepted,
            };
            let bytes = render_manifest(&manifest)?.into_bytes();
            (manifest, bytes, None, None)
        };
        if manifest.inputs.len() != self.files.len() {
            return Err(crate::product_message!(
                "backend.arxml.project.source_membership_not_exact"
            ));
        }
        let mut inputs = Vec::with_capacity(manifest.inputs.len());
        let mut owned = BTreeSet::new();
        for member in &manifest.inputs {
            let file = self
                .files
                .iter()
                .find(|file| match &root {
                    Some(root) => file.path == root.join(&member.path),
                    None => {
                        file.path.file_name().and_then(|name| name.to_str())
                            == Some(member.path.as_str())
                    }
                })
                .ok_or_else(|| {
                    crate::product_message!("backend.arxml.project.declared_source_not_owned")
                })?;
            if !owned.insert(&file.path) {
                return Err(crate::product_message!(
                    "backend.arxml.project.duplicate_source_identity"
                ));
            }
            inputs.push(GenerationInputSnapshot {
                logical_path: member.path.clone(),
                disk_path: file.path.clone(),
                bytes: file.saved.as_bytes().to_vec(),
            });
        }
        let mut applications = Vec::with_capacity(manifest.application_inputs.len());
        if let Some(project) = &self.project {
            if project.application_bytes.len() != manifest.application_inputs.len() {
                return Err(crate::product_message!(
                    "backend.arxml.project.application_byte_ownership_mismatch"
                ));
            }
            for member in &manifest.application_inputs {
                applications.push(GenerationInputSnapshot {
                    logical_path: member.path.clone(),
                    disk_path: root
                        .as_ref()
                        .ok_or_else(|| {
                            crate::product_message!(
                                "backend.arxml.project.application_root_missing"
                            )
                        })?
                        .join(&member.path),
                    bytes: project
                        .application_bytes
                        .get(&member.path)
                        .ok_or_else(|| {
                            crate::product_message!(
                                "backend.arxml.project.declared_application_bytes_missing"
                            )
                        })?
                        .clone(),
                });
            }
        }
        inputs.sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
        applications.sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
        Ok(GenerationSnapshot {
            manifest,
            manifest_bytes,
            manifest_path,
            project_root: root,
            inputs,
            applications,
        })
    }

    pub fn preview_project_creation(
        directory: &Path,
        name: &str,
        template_id: &str,
    ) -> Result<ProjectCreationPreview, crate::message::LocalizedText> {
        let directory = destination(directory)?;
        let (profile, mut files, inputs) = template_files(name, template_id)?;
        let manifest = ProjectManifest {
            format_version: 1,
            declared_release: "R24-11".into(),
            profile_hint: profile,
            inputs,
            application_inputs: Vec::new(),
            accepted_extension_definitions: Vec::new(),
        };
        files.push(ProjectFilePreview {
            path: MANIFEST.into(),
            contents: render_manifest(&manifest)?,
        });
        let catalog = Arc::new(crate::definitions::DefinitionCatalog::builtin()?);
        let mut preview = ProjectCreationPreview {
            revision: String::new(),
            template_id: template_id.into(),
            directory: directory.display().to_string(),
            name: name.into(),
            files,
            accepted_extension_definitions: Vec::new(),
        };
        from_preview(&preview, catalog.clone(), true)?;
        preview.revision = preview_revision(&preview, "", &catalog)?;
        Ok(preview)
    }

    pub fn create_project_previewed(
        preview: &ProjectCreationPreview,
    ) -> Result<Self, crate::message::LocalizedText> {
        let expected = Self::preview_project_creation(
            Path::new(&preview.directory),
            &preview.name,
            &preview.template_id,
        )?;
        if *preview != expected {
            return Err(crate::product_message!(
                "backend.arxml.project.creation_preview_changed"
            ));
        }
        let workspace = from_preview(
            &expected,
            Arc::new(crate::definitions::DefinitionCatalog::builtin()?),
            true,
        )?;
        publish(&expected, workspace)
    }

    pub fn preview_save_as_project(
        &self,
        directory: &Path,
        name: &str,
    ) -> Result<ProjectCreationPreview, crate::message::LocalizedText> {
        if !valid_name(name) {
            return Err(crate::product_message!(
                "backend.arxml.project.invalid_project_name"
            ));
        }
        self.ensure_sources_current()?;
        self.check_configuration_transition(self)?;
        let directory = destination(directory)?;
        let mut paths = BTreeSet::new();
        let mut files = Vec::new();
        let mut inputs = Vec::new();
        for file in &self.files {
            let path = if let Some(project) = &self.project {
                file.path
                    .strip_prefix(project.path.parent().unwrap())
                    .map_err(|error| error.to_string())?
                    .to_str()
                    .ok_or_else(|| {
                        crate::product_message!("backend.arxml.project.member_path_not_utf_eight")
                    })?
                    .replace('\\', "/")
            } else {
                file.path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| {
                        crate::product_message!("backend.arxml.project.source_name_not_utf_eight")
                    })?
                    .to_owned()
            };
            relative_path(&path)?;
            if !paths.insert(path.to_ascii_lowercase()) {
                return Err(crate::product_message!(
                    "backend.arxml.project.save_as_source_names_conflict"
                ));
            }
            inputs.push(ProjectInput {
                path: path.clone(),
                role_hint: "source".into(),
            });
            files.push(ProjectFilePreview {
                path,
                contents: file.text.clone(),
            });
        }
        let application_inputs = self
            .project
            .as_ref()
            .map(|project| project.manifest.application_inputs.clone())
            .unwrap_or_default();
        if let Some(project) = &self.project {
            for input in &application_inputs {
                let bytes = project.application_bytes.get(&input.path).ok_or_else(|| {
                    crate::product_message!("backend.arxml.project.application_snapshot_missing")
                })?;
                files.push(ProjectFilePreview {
                    path: input.path.clone(),
                    contents: String::from_utf8(bytes.clone()).map_err(|_| {
                        crate::product_message!(
                            "backend.arxml.project.application_source_must_be_utf_eight"
                        )
                    })?,
                });
            }
        }
        let accepted = self.catalog.accepted_extensions();
        let manifest = ProjectManifest {
            format_version: 1,
            declared_release: "R24-11".into(),
            profile_hint: self.snapshot.profile.clone(),
            inputs,
            application_inputs,
            accepted_extension_definitions: accepted.clone(),
        };
        validate_manifest(&manifest)?;
        files.push(ProjectFilePreview {
            path: MANIFEST.into(),
            contents: render_manifest(&manifest)?,
        });
        let mut preview = ProjectCreationPreview {
            revision: String::new(),
            template_id: "save-as-project-v1".into(),
            directory: directory.display().to_string(),
            name: name.into(),
            files,
            accepted_extension_definitions: accepted,
        };
        preview.revision = preview_revision(&preview, &self.input_fingerprint()?, &self.catalog)?;
        Ok(preview)
    }

    pub fn save_as_project_previewed(
        &self,
        preview: &ProjectCreationPreview,
    ) -> Result<Self, crate::message::LocalizedText> {
        let expected =
            self.preview_save_as_project(Path::new(&preview.directory), &preview.name)?;
        if *preview != expected {
            return Err(crate::product_message!(
                "backend.arxml.project.save_as_preview_stale"
            ));
        }
        let workspace = from_preview(&expected, self.catalog.clone(), false)?;
        publish(&expected, workspace)
    }

    pub fn open_project_manifest(
        path: &Path,
        cache_root: &Path,
    ) -> Result<Self, crate::message::LocalizedText> {
        Self::open_project_using(path, |manifest| {
            let mut catalog = crate::definitions::DefinitionCatalog::builtin()?;
            let diagnostics =
                catalog.restore_extensions(&manifest.accepted_extension_definitions, cache_root)?;
            Ok((catalog, diagnostics))
        })
    }

    /// Opens a sealed project using only the caller's explicitly accepted catalogs.
    pub fn open_project_with_catalog(
        path: &Path,
        accepted: &crate::definitions::DefinitionCatalog,
    ) -> Result<Self, crate::message::LocalizedText> {
        Self::open_project_using(path, |manifest| {
            let identities = accepted.accepted_extensions();
            for required in &manifest.accepted_extension_definitions {
                if !identities.contains(required) {
                    return Err(crate::product_message!(
                        "backend.arxml.project.exact_catalog_not_accepted",
                        "catalog_id" => &required.catalog_id
                    ));
                }
            }
            let mut catalog = accepted.clone();
            for identity in identities {
                if !manifest.accepted_extension_definitions.contains(&identity) {
                    catalog.remove_extension(&identity.catalog_id)?;
                }
            }
            Ok((catalog, Vec::new()))
        })
    }

    fn open_project_using(
        path: &Path,
        restore: impl FnOnce(
            &ProjectManifest,
        ) -> Result<
            (
                crate::definitions::DefinitionCatalog,
                Vec<ConfigurationDiagnostic>,
            ),
            crate::message::LocalizedText,
        >,
    ) -> Result<Self, crate::message::LocalizedText> {
        let selected = if path.is_absolute() {
            path.to_owned()
        } else {
            std::env::current_dir()
                .map_err(|error| error.to_string())?
                .join(path)
        };
        safe_path(&selected, false)?;
        if selected.file_name().and_then(|name| name.to_str()) != Some(MANIFEST)
            || fs::metadata(&selected)
                .map_err(|error| error.to_string())?
                .len()
                > 50 * 1024 * 1024
        {
            return Err(crate::product_message!(
                "backend.arxml.project.invalid_manifest_file"
            ));
        }
        let path = fs::canonicalize(selected).map_err(|error| error.to_string())?;
        let root = path.parent().ok_or_else(|| {
            crate::product_message!("backend.arxml.project.manifest_root_missing")
        })?;
        let saved = String::from_utf8(read_bounded(&path)?).map_err(|error| error.to_string())?;
        let manifest: ProjectManifest =
            serde_json::from_str(&saved).map_err(|error| error.to_string())?;
        validate_manifest(&manifest)?;
        let (catalog, diagnostics) = restore(&manifest)?;
        let mut sources = Vec::new();
        for input in &manifest.inputs {
            let path = root.join(relative_path(&input.path)?);
            safe_path(&path, false)?;
            sources.push(path);
        }
        let mut application_bytes = BTreeMap::new();
        for input in &manifest.application_inputs {
            let path = root.join(relative_path(&input.path)?);
            safe_path(&path, false)?;
            application_bytes.insert(input.path.clone(), read_bounded(&path)?);
        }
        let mut workspace = Self::open(sources)?;
        workspace.catalog = Arc::new(catalog);
        workspace.integration_input_root = Some(root.to_owned());
        workspace.project = Some(ProjectMembership {
            path,
            manifest,
            saved: saved.clone(),
            current: saved,
            application_bytes,
            extension_diagnostics: diagnostics,
        });
        workspace.refresh()?;
        // The hint never authorizes application slots: actual source semantics determine the profile.
        if !workspace
            .project
            .as_ref()
            .unwrap()
            .manifest
            .application_inputs
            .is_empty()
        {
            let sources = workspace
                .integration_sources()
                .map_err(super::integration_errors)?;
            let members = &workspace
                .project
                .as_ref()
                .unwrap()
                .manifest
                .application_inputs;
            let matches = if let Some(slots) =
                crate::integration::ecu::workspace_application_slots(&sources)
                    .map_err(super::integration_errors)?
            {
                slots.len() == members.len()
                    && slots.iter().all(|slot| {
                        members.iter().any(|input| {
                            input.producer_slot == slot.producer_slot
                                && slot.source_paths == [input.path.clone()]
                        })
                    })
            } else {
                // Unrelated target-only restrictions do not authorize or block legacy ownership.
                crate::integration::inspect_inputs_native(&sources, &workspace.catalog)
                    .map_err(super::integration_errors)?
                    .component_contract()
                    .map_err(super::integration_errors)?;
                members.len() == 1
                    && members.iter().all(|input| {
                        input.producer_slot == crate::generator::delivery::APPLICATION_SLOT
                            && input.path == crate::integration::APPLICATION_SOURCE_PATH
                    })
            };
            if !matches {
                return Err(crate::product_message!(
                    "backend.arxml.project.application_live_slot_mismatch"
                ));
            }
        }
        workspace.ensure_sources_current()?;
        Ok(workspace)
    }

    pub fn accept_definition_catalog(
        &mut self,
        path: &Path,
        cache_root: &Path,
    ) -> Result<ExtensionDefinitionIdentity, crate::message::LocalizedText> {
        self.ensure_sources_current()?;
        let mut candidate = self.clone();
        let mut catalog = (*candidate.catalog).clone();
        let identity = catalog.accept_extension(path, cache_root)?;
        candidate.catalog = Arc::new(catalog);
        candidate.sync_project_acceptance()?;
        candidate.refresh()?;
        candidate.revision += 1;
        *self = candidate;
        Ok(identity)
    }

    pub fn remove_definition_catalog(
        &mut self,
        id: &str,
    ) -> Result<(), crate::message::LocalizedText> {
        self.ensure_sources_current()?;
        let mut candidate = self.clone();
        let mut catalog = (*candidate.catalog).clone();
        catalog.remove_extension(id)?;
        candidate.catalog = Arc::new(catalog);
        candidate.sync_project_acceptance()?;
        candidate.refresh()?;
        candidate.revision += 1;
        *self = candidate;
        Ok(())
    }

    fn sync_project_acceptance(&mut self) -> Result<(), crate::message::LocalizedText> {
        if let Some(project) = &mut self.project {
            project.manifest.accepted_extension_definitions = self.catalog.accepted_extensions();
            project.current = render_manifest(&project.manifest)?;
            project.extension_diagnostics.clear();
        }
        Ok(())
    }

    pub fn project_manifest(&self) -> Option<&ProjectManifest> {
        self.project.as_ref().map(|project| &project.manifest)
    }

    pub fn definition_catalog(&self) -> &crate::definitions::DefinitionCatalog {
        &self.catalog
    }
}
