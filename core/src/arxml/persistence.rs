use super::*;

pub(super) struct Patch {
    pub(super) range: std::ops::Range<usize>,
    pub(super) value: String,
}

pub(super) fn patch_child(
    node: Node<'_, '_>,
    child_name: &str,
    value: String,
    patches: &mut Vec<Patch>,
) -> Result<(), crate::message::LocalizedText> {
    let leaf = node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == child_name)
        .ok_or_else(|| {
            crate::product_message!(
                "backend.arxml.persistence.missing_child",
                "path" => path_of(node),
                "child" => child_name
            )
        })?;
    let mut content = leaf.children();
    let text = content.next().filter(|n| n.is_text()).ok_or_else(|| {
        crate::product_message!("backend.arxml.persistence.value_not_simple_text")
    })?;
    if content.next().is_some() {
        return Err(crate::product_message!(
            "backend.arxml.persistence.value_has_mixed_content"
        ));
    }
    patches.push(Patch {
        range: text.range(),
        value,
    });
    Ok(())
}

pub(super) fn patch_param(
    node: Node<'_, '_>,
    name: &str,
    value: String,
    patches: &mut Vec<Patch>,
) -> Result<(), crate::message::LocalizedText> {
    let mut found = node
        .descendants()
        .filter(|n| {
            n.is_element()
                && matches!(
                    n.tag_name().name(),
                    "ECUC-NUMERICAL-PARAM-VALUE" | "ECUC-TEXTUAL-PARAM-VALUE"
                )
        })
        .filter(|n| definition(*n).is_some_and(|d| d.ends_with(&format!("/{name}"))));
    let parameter = found.next().ok_or_else(|| {
        crate::product_message!(
            "backend.arxml.persistence.missing_parameter",
            "name" => name
        )
    })?;
    if found.next().is_some() {
        return Err(crate::product_message!(
            "backend.arxml.persistence.multiple_parameter_variants",
            "name" => name
        ));
    }
    patch_child(parameter, "VALUE", value, patches)
}

pub(super) fn apply_patches(
    text: &mut String,
    patches: &mut Vec<Patch>,
) -> Result<(), crate::message::LocalizedText> {
    patches.sort_by_key(|patch| std::cmp::Reverse(patch.range.start));
    let mut next_start = text.len();
    for patch in patches {
        if patch.range.end > next_start {
            return Err(crate::product_message!(
                "backend.arxml.persistence.overlapping_patch_ranges"
            ));
        }
        next_start = patch.range.start;
        text.replace_range(patch.range.clone(), &patch.value);
    }
    Ok(())
}

fn restore_backup(
    original: &Path,
    backup: &Path,
    installed: Option<&str>,
) -> Result<(), crate::message::LocalizedText> {
    restore_backup_with_publish(original, backup, installed, |from, to| {
        fs::hard_link(from, to)
    })
}

fn restore_backup_with_publish(
    original: &Path,
    backup: &Path,
    installed: Option<&str>,
    publish: impl Fn(&Path, &Path) -> std::io::Result<()>,
) -> Result<(), crate::message::LocalizedText> {
    let recovery = backup.with_extension("rollback");
    let captured = recovery.join("original");
    let mut captured_ours = false;
    if installed.is_some() {
        fs::create_dir(&recovery).map_err(|e| format!("{}: {e}", recovery.display()))?;
        let reservation = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&captured);
        match reservation {
            Ok(handle) => drop(handle),
            Err(error) => {
                let cleanup = fs::remove_dir(&recovery);
                return Err(crate::message::LocalizedText::messages([
                    format!("{}: {error}", captured.display()).into(),
                    cleanup
                        .err()
                        .map(|e| format!("{}: {e}", recovery.display()))
                        .unwrap_or_default()
                        .into(),
                ]));
            }
        }
        // A reserved ordinary file also rejects a raced directory before moving it.
        match fs::rename(original, &captured) {
            Ok(()) => {
                // Inspect the captured file, never unlink a path after checking its bytes.
                captured_ours = fs::read_to_string(&captured)
                    .is_ok_and(|current| Some(current.as_str()) == installed);
                if !captured_ours {
                    let restored = publish(&captured, original);
                    if restored.is_ok() {
                        fs::remove_file(&captured)
                            .map_err(|e| format!("{}: {e}", captured.display()))?;
                        fs::remove_dir(&recovery)
                            .map_err(|e| format!("{}: {e}", recovery.display()))?;
                    }
                    let error = crate::product_message!(
                        "backend.arxml.persistence.external_file_preserved",
                        "original" => original.display(),
                        "backup" => backup.display()
                    );
                    return Err(match restored {
                        Ok(()) => error,
                        Err(e) => crate::message::LocalizedText::messages([
                            error,
                            format!("{} -> {}: {e}", captured.display(), original.display()).into(),
                        ]),
                    });
                }
            }
            Err(error) => {
                fs::remove_file(&captured)
                    .map_err(|e| format!("{}: {e}; {error}", captured.display()))?;
                fs::remove_dir(&recovery)
                    .map_err(|e| format!("{}: {e}; {error}", recovery.display()))?;
                if error.kind() != std::io::ErrorKind::NotFound {
                    return Err(format!(
                        "{} -> {}: {error}",
                        original.display(),
                        captured.display()
                    )
                    .into());
                }
            }
        }
    }
    publish(backup, original).map_err(|e| {
        if captured_ours {
            format!(
                "{} -> {}: {e}; {}",
                backup.display(),
                original.display(),
                captured.display()
            )
        } else {
            format!("{} -> {}: {e}", backup.display(), original.display())
        }
    })?;
    if captured_ours {
        fs::remove_file(&captured).map_err(|e| format!("{}: {e}", captured.display()))?;
        fs::remove_dir(&recovery).map_err(|e| format!("{}: {e}", recovery.display()))?;
    }
    fs::remove_file(backup).map_err(|e| format!("{}: {e}", backup.display()))?;
    Ok(())
}

#[cfg(test)]
fn install_staged(
    file: &SourceFile,
    stage: &Path,
    backup: &Path,
) -> Result<(), crate::message::LocalizedText> {
    install_staged_with_publish(file, stage, backup, |from, to| fs::hard_link(from, to))
}

fn install_staged_with_publish(
    file: &SourceFile,
    stage: &Path,
    backup: &Path,
    publish: impl Fn(&Path, &Path) -> std::io::Result<()>,
) -> Result<(), crate::message::LocalizedText> {
    // Reserve the backup and verify hard-link support before moving the original.
    publish(stage, backup)
        .map_err(|e| format!("{} -> {}: {e}", stage.display(), backup.display()))?;
    if let Err(error) = fs::rename(&file.path, backup) {
        let cleanup = fs::remove_file(backup);
        return Err(crate::message::LocalizedText::messages([
            format!("{} -> {}: {error}", file.path.display(), backup.display()).into(),
            cleanup
                .err()
                .map(|e| format!("{}: {e}", backup.display()))
                .unwrap_or_default()
                .into(),
        ]));
    }
    let result = fs::read_to_string(backup)
        .map_err(|e| {
            crate::message::LocalizedText::messages([
                crate::product_message!(
                    "backend.arxml.persistence.original_file_recheck_failed",
                    "path" => backup.display()
                ),
                e.to_string().into(),
            ])
        })
        .and_then(|actual| {
            if actual != file.saved {
                return Err(crate::product_message!(
                    "backend.arxml.persistence.external_modification_overwrite_refused",
                    "path" => file.path.display()
                ));
            }
            publish(stage, &file.path)
                .map_err(|e| format!("{} -> {}: {e}", stage.display(), file.path.display()).into())
        });
    if let Err(error) = result {
        return Err(match restore_backup(&file.path, backup, None) {
            Ok(()) => error,
            Err(rollback) => crate::message::LocalizedText::messages([
                error,
                crate::product_message!("backend.arxml.persistence.rollback_problems"),
                rollback,
            ]),
        });
    }
    // Keep the staged link until the transaction finishes so failures stay recoverable.
    Ok(())
}

/// A validated immutable save snapshot; the complete disk transaction stays here.
pub struct PreparedSave {
    workspace: Workspace,
}

pub struct SaveFailure {
    workspace: Workspace,
    message: crate::message::LocalizedText,
}

impl SaveFailure {
    pub fn into_parts(self) -> (Workspace, crate::message::LocalizedText) {
        (self.workspace, self.message)
    }
}

impl std::fmt::Debug for SaveFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SaveFailure")
            .field("message", &self.message)
            .finish()
    }
}

impl std::fmt::Display for SaveFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for SaveFailure {}

impl PreparedSave {
    pub(super) fn validated(
        workspace: Workspace,
        revision: &str,
    ) -> Result<Self, crate::message::LocalizedText> {
        if workspace.save_revision() != revision {
            return Err(crate::product_message!(
                "backend.arxml.persistence.save_preview_stale"
            ));
        }
        workspace.ensure_sources_current()?;
        Ok(Self { workspace })
    }

    pub fn commit(mut self) -> Result<Workspace, SaveFailure> {
        match self.workspace.save_sources() {
            Ok(()) => Ok(self.workspace),
            Err(message) => Err(SaveFailure {
                workspace: self.workspace,
                message,
            }),
        }
    }
}

impl Workspace {
    pub fn prepare_save_previewed(
        mut self,
        revision: &str,
    ) -> Result<PreparedSave, crate::message::LocalizedText> {
        self.ensure_save_allowed()?;
        PreparedSave::validated(self, revision)
    }

    pub(super) fn commit_patches(
        &mut self,
        mut patches: Vec<Vec<Patch>>,
        matches: impl FnOnce(&Workspace) -> bool,
    ) -> Result<(), crate::message::LocalizedText> {
        let mut previous = Vec::new();
        for (index, edits) in patches.iter_mut().enumerate() {
            if edits.is_empty() {
                continue;
            }
            previous.push((index, self.files[index].text.clone()));
            if let Err(error) = apply_patches(&mut self.files[index].text, edits) {
                for (i, text) in previous {
                    self.files[i].text = text;
                }
                return Err(error);
            }
        }
        let checked = self.refresh().and_then(|_| {
            if let Some(issue) = self
                .issues
                .iter()
                .find(|i| matches!(i.severity, Severity::Error))
            {
                Err(crate::message::LocalizedText::messages([
                    crate::product_message!(
                        "backend.arxml.persistence.blocking_diagnostic",
                        "code" => issue.code
                    ),
                    issue.message.clone(),
                ]))
            } else if matches(self) {
                Ok(())
            } else {
                Err(crate::product_message!(
                    "backend.arxml.persistence.edited_model_mismatch"
                ))
            }
        });
        if let Err(error) = checked {
            for (i, text) in previous {
                self.files[i].text = text;
            }
            let _ = self.refresh();
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn patch_imported_frame(
        &mut self,
        old: &FrameView,
        new: &FrameView,
    ) -> Result<(), crate::message::LocalizedText> {
        let global_pdu = self.global_pdu_for(&old.path)?;
        let mut patches: Vec<Vec<Patch>> = (0..self.files.len()).map(|_| Vec::new()).collect();
        let mut found_id = false;
        let mut found_dlc = false;
        let mut found_global_length = false;
        let mut found_period = false;
        let mut found_timeout = 0usize;
        let receive_signals: BTreeSet<_> = self
            .signals
            .iter()
            .filter(|s| s.frame_path == old.path)
            .map(|s| s.path.as_str())
            .collect();
        for (index, file) in self.files.iter().enumerate() {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element()) {
                if old.dlc != new.dlc
                    && node.tag_name().name() == "I-SIGNAL-I-PDU"
                    && path_of(node) == old.path
                {
                    patch_child(node, "LENGTH", new.dlc.to_string(), &mut patches[index])?;
                    found_dlc = true;
                }
                if old.dlc != new.dlc
                    && node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                    && path_of(node) == global_pdu
                {
                    patch_param(node, "PduLength", new.dlc.to_string(), &mut patches[index])?;
                    found_global_length = true;
                }
                if node.tag_name().name() == "ECUC-CONTAINER-VALUE" {
                    let def = definition(node).unwrap_or_default();
                    if def.ends_with("/CanIfTxPduCfg")
                        && ref_value(node, "CanIfTxPduRef").as_deref() == Some(&global_pdu)
                        && old.id != new.id
                    {
                        patch_param(
                            node,
                            "CanIfTxPduCanId",
                            new.id.to_string(),
                            &mut patches[index],
                        )?;
                        found_id = true;
                    }
                    if def.ends_with("/CanIfRxPduCfg")
                        && ref_value(node, "CanIfRxPduRef").as_deref() == Some(&global_pdu)
                    {
                        if old.id != new.id {
                            patch_param(
                                node,
                                "CanIfRxPduCanId",
                                new.id.to_string(),
                                &mut patches[index],
                            )?;
                            found_id = true;
                        }
                        if old.dlc != new.dlc {
                            patch_param(
                                node,
                                "CanIfRxPduDataLength",
                                new.dlc.to_string(),
                                &mut patches[index],
                            )?;
                        }
                    }
                    if old.period_ms != new.period_ms
                        && def.ends_with("/ComIPdu")
                        && ref_value(node, "ComPduIdRef").as_deref() == Some(&global_pdu)
                    {
                        patch_param(
                            node,
                            "ComTxModeTimePeriod",
                            seconds(new.period_ms.ok_or_else(|| {
                                crate::product_message!(
                                    "backend.arxml.persistence.transmit_period_required"
                                )
                            })?),
                            &mut patches[index],
                        )?;
                        found_period = true;
                    }
                    if old.timeout_ms != new.timeout_ms
                        && receive_signals.contains(path_of(node).as_str())
                    {
                        patch_param(
                            node,
                            "ComTimeout",
                            seconds(new.timeout_ms.ok_or_else(|| {
                                crate::product_message!(
                                    "backend.arxml.persistence.receive_timeout_required"
                                )
                            })?),
                            &mut patches[index],
                        )?;
                        found_timeout += 1;
                    }
                }
                if old.id != new.id && node.tag_name().name() == "CAN-FRAME-TRIGGERING" {
                    return Err(crate::product_message!(
                        "backend.arxml.persistence.identifier_requires_network_sync"
                    ));
                }
                if old.dlc != new.dlc && node.tag_name().name() == "CAN-FRAME" {
                    return Err(crate::product_message!(
                        "backend.arxml.persistence.dlc_requires_network_sync"
                    ));
                }
            }
        }
        if (old.id != new.id && !found_id)
            || (old.dlc != new.dlc && (!found_dlc || !found_global_length))
            || (old.period_ms != new.period_ms && !found_period)
            || (old.timeout_ms != new.timeout_ms && found_timeout != receive_signals.len())
        {
            return Err(crate::product_message!(
                "backend.arxml.persistence.standard_parameters_not_locatable"
            ));
        }
        self.commit_patches(patches, |w| {
            w.frames.iter().any(|f| {
                f.path == old.path
                    && f.id == new.id
                    && f.dlc == new.dlc
                    && f.period_ms == new.period_ms
                    && f.timeout_ms == new.timeout_ms
            })
        })?;
        Ok(())
    }

    pub(super) fn patch_imported_signal(
        &mut self,
        old: &SignalView,
        new: &SignalView,
    ) -> Result<(), crate::message::LocalizedText> {
        let mut patches: Vec<Vec<Patch>> = (0..self.files.len()).map(|_| Vec::new()).collect();
        let mut found_com = false;
        let mut found_mapping = false;
        let mut found_system_signal = false;
        let layout_changed = old.start_bit != new.start_bit || old.length != new.length;
        let system_path = format!("/{}/ISignal_{}", self.name, old.name);
        for (index, file) in self.files.iter().enumerate() {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element()) {
                if node.tag_name().name() == "ECUC-CONTAINER-VALUE" && path_of(node) == old.path {
                    if old.start_bit != new.start_bit {
                        patch_param(
                            node,
                            "ComBitPosition",
                            new.start_bit.to_string(),
                            &mut patches[index],
                        )?;
                    }
                    if old.length != new.length {
                        patch_param(
                            node,
                            "ComBitSize",
                            new.length.to_string(),
                            &mut patches[index],
                        )?;
                        patch_param(
                            node,
                            "ComSignalType",
                            signal_type(new.length).into(),
                            &mut patches[index],
                        )?;
                    }
                    if old.initial_value != new.initial_value {
                        patch_param(
                            node,
                            "ComSignalInitValue",
                            new.initial_value.to_string(),
                            &mut patches[index],
                        )?;
                    }
                    found_com = true;
                }
                if layout_changed
                    && node.tag_name().name() == "I-SIGNAL-TO-I-PDU-MAPPING"
                    && child_text(node, "I-SIGNAL-REF").as_deref() == Some(&system_path)
                    && node.ancestors().any(|a| {
                        a.tag_name().name() == "I-SIGNAL-I-PDU" && path_of(a) == old.frame_path
                    })
                {
                    if old.start_bit != new.start_bit {
                        patch_child(
                            node,
                            "START-POSITION",
                            new.start_bit.to_string(),
                            &mut patches[index],
                        )?;
                    }
                    found_mapping = true;
                }
                if old.length != new.length
                    && node.tag_name().name() == "I-SIGNAL"
                    && path_of(node) == system_path
                {
                    patch_child(node, "LENGTH", new.length.to_string(), &mut patches[index])?;
                    found_system_signal = true;
                }
            }
        }
        if !found_com
            || (layout_changed && !found_mapping)
            || (old.length != new.length && !found_system_signal)
        {
            return Err(crate::product_message!(
                "backend.arxml.persistence.incomplete_signal_system_mapping"
            ));
        }
        self.commit_patches(patches, |w| {
            w.signals.iter().any(|s| {
                s.path == old.path
                    && s.start_bit == new.start_bit
                    && s.length == new.length
                    && s.initial_value == new.initial_value
            })
        })?;
        Ok(())
    }

    pub(super) fn ensure_sources_current(&self) -> Result<(), crate::message::LocalizedText> {
        for file in &self.files {
            super::project::safe_path(&file.path, false)?;
        }
        if let Some(project) = &self.project {
            super::project::safe_path(&project.path, false)?;
            if super::project::read_bounded(&project.path)? != project.saved.as_bytes() {
                return Err(crate::product_message!(
                    "backend.arxml.persistence.project_manifest_externally_modified"
                ));
            }
            for (path, saved) in &project.application_bytes {
                let path = project
                    .path
                    .parent()
                    .ok_or_else(|| {
                        crate::product_message!("backend.arxml.persistence.project_root_missing")
                    })?
                    .join(path);
                super::project::safe_path(&path, false)?;
                if super::project::read_bounded(&path)? != *saved {
                    return Err(crate::product_message!(
                        "backend.arxml.persistence.application_source_externally_modified"
                    ));
                }
            }
        }
        for file in &self.files {
            let disk = super::project::read_bounded(&file.path)?;
            if disk != file.saved.as_bytes() {
                return Err(crate::product_message!(
                    "backend.arxml.persistence.stale_source_reimport_required",
                    "path" => file.path.display()
                ));
            }
        }
        Ok(())
    }

    pub(super) fn save_revision(&self) -> String {
        self.save_revision_for_project(self.project.as_ref())
    }

    pub(super) fn save_revision_for_project(
        &self,
        membership: Option<&super::project::ProjectMembership>,
    ) -> String {
        let mut digest = Sha256::new();
        for file in &self.files {
            let path = file.path.to_string_lossy();
            for bytes in [path.as_bytes(), file.saved.as_bytes(), file.text.as_bytes()] {
                digest.update((bytes.len() as u64).to_le_bytes());
                digest.update(bytes);
            }
        }
        digest.update(
            serde_json::to_vec(&crate::rules::rule_set_identity())
                .expect("Rule identity serializes"),
        );
        digest.update(
            serde_json::to_vec(&self.definition_fingerprint())
                .expect("Definition identity serializes"),
        );
        if let Some(project) = membership {
            for bytes in [
                project.path.to_string_lossy().as_bytes(),
                project.saved.as_bytes(),
                project.current.as_bytes(),
            ] {
                digest.update((bytes.len() as u64).to_le_bytes());
                digest.update(bytes);
            }
            for (path, bytes) in &project.application_bytes {
                digest.update(path.as_bytes());
                digest.update((bytes.len() as u64).to_le_bytes());
                digest.update(bytes);
            }
        }
        format!("{:x}", digest.finalize())
    }

    pub fn preview_save(&mut self) -> Result<SavePreview, crate::message::LocalizedText> {
        self.ensure_save_allowed()?;
        Ok(SavePreview {
            revision: self.save_revision(),
            files: self.save_preview_files(),
        })
    }

    pub fn save_previewed(
        &mut self,
        revision: &str,
    ) -> Result<WorkspaceView, crate::message::LocalizedText> {
        if self.save_revision() != revision {
            return Err(crate::product_message!(
                "backend.arxml.persistence.save_preview_stale"
            ));
        }
        self.save()
    }

    pub fn save(&mut self) -> Result<WorkspaceView, crate::message::LocalizedText> {
        self.ensure_save_allowed()?;
        // Validation includes references across every imported file, including files this
        // edit leaves untouched. A stale untouched file would invalidate that result.
        self.save_sources()?;
        Ok(self.view())
    }

    pub(super) fn ensure_save_allowed(&mut self) -> Result<(), crate::message::LocalizedText> {
        self.validate()?;
        self.ensure_no_recovery_backups()?;
        if self.uses_legacy_validation() {
            if let Some(issue) = self
                .issues
                .iter()
                .find(|issue| matches!(issue.severity, Severity::Error))
            {
                return Err(crate::message::LocalizedText::messages([
                    crate::product_message!(
                        "backend.arxml.persistence.blocking_diagnostic",
                        "code" => issue.code
                    ),
                    issue.message.clone(),
                ]));
            }
        } else {
            // Target-generation failures do not close safe configuration persistence.
            for scope in &self.snapshot.validation {
                if matches!(
                    scope.scope,
                    crate::project_model::ValidationScope::SourceSafety
                        | crate::project_model::ValidationScope::Schema
                ) {
                    if let Some(issue) = scope
                        .diagnostics
                        .iter()
                        .find(|issue| matches!(issue.severity, Severity::Error))
                    {
                        return Err(crate::message::LocalizedText::messages([
                            crate::product_message!(
                                "backend.arxml.persistence.blocking_diagnostic",
                                "code" => issue.code
                            ),
                            issue.message.clone(),
                        ]));
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn save_preview_files(&self) -> Vec<SavePreviewFile> {
        let mut files: Vec<_> = self
            .files
            .iter()
            .map(|file| {
                let changed = file.saved != file.text;
                SavePreviewFile {
                    path: file.path.display().to_string(),
                    changed,
                    before: changed.then(|| file.saved.clone()),
                    after: changed.then(|| file.text.clone()),
                }
            })
            .collect();
        if let Some(project) = &self.project {
            let changed = project.saved != project.current;
            files.push(SavePreviewFile {
                path: project.path.display().to_string(),
                changed,
                before: changed.then(|| project.saved.clone()),
                after: changed.then(|| project.current.clone()),
            });
        }
        files
    }

    fn ensure_no_recovery_backups(&self) -> Result<(), crate::message::LocalizedText> {
        for path in self
            .files
            .iter()
            .map(|file| &file.path)
            .chain(self.project.iter().map(|project| &project.path))
        {
            let prefix_path = path.with_extension("arxml.autosar-config-");
            let prefix = prefix_path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    crate::product_message!("backend.arxml.persistence.recovery_path_not_utf_eight")
                })?;
            for entry in fs::read_dir(path.parent().ok_or_else(|| {
                crate::product_message!("backend.arxml.persistence.source_directory_missing")
            })?)
            .map_err(|error| error.to_string())?
            {
                let entry = entry.map_err(|error| error.to_string())?;
                if entry.file_name().to_str().is_some_and(|name| {
                    name.starts_with(prefix)
                        && (name.ends_with(".bak") || name.ends_with(".rollback"))
                }) {
                    return Err(crate::product_message!(
                        "backend.arxml.persistence.unrecovered_backup_blocks_save",
                        "path" => entry.path().display()
                    ));
                }
            }
        }
        Ok(())
    }

    pub(super) fn save_sources(&mut self) -> Result<(), crate::message::LocalizedText> {
        self.save_sources_with_cleanup(
            |stage| fs::remove_file(stage),
            |backup| fs::remove_file(backup),
        )
    }

    fn save_sources_with_cleanup(
        &mut self,
        remove_stage: impl Fn(&Path) -> std::io::Result<()>,
        remove_backup: impl Fn(&Path) -> std::io::Result<()>,
    ) -> Result<(), crate::message::LocalizedText> {
        self.save_sources_with_publish(remove_stage, remove_backup, |from, to| {
            fs::hard_link(from, to)
        })
    }

    fn save_sources_with_publish(
        &mut self,
        remove_stage: impl Fn(&Path) -> std::io::Result<()>,
        remove_backup: impl Fn(&Path) -> std::io::Result<()>,
        publish: impl Fn(&Path, &Path) -> std::io::Result<()>,
    ) -> Result<(), crate::message::LocalizedText> {
        self.ensure_sources_current()?;
        self.ensure_no_recovery_backups()?;
        let mut dirty = self
            .files
            .iter()
            .filter(|file| file.text != file.saved)
            .cloned()
            .collect::<Vec<_>>();
        if let Some(project) = &self.project {
            if project.current != project.saved {
                dirty.push(SourceFile {
                    path: project.path.clone(),
                    text: project.current.clone(),
                    saved: project.saved.clone(),
                    original_name: None,
                });
            }
        }
        let mut staged = Vec::new();
        for (index, file) in dirty.iter().enumerate() {
            let suffix = format!("arxml.autosar-config-{}-{index}", std::process::id());
            let stage = file.path.with_extension(format!("{suffix}.tmp"));
            let backup = file.path.with_extension(format!("{suffix}.bak"));
            let prepared = (|| -> Result<(), crate::message::LocalizedText> {
                if backup.exists() {
                    return Err(crate::product_message!(
                        "backend.arxml.persistence.recovery_backup_already_exists",
                        "path" => backup.display()
                    ));
                }
                if fs::read_to_string(&file.path).map_err(|e| e.to_string())? != file.saved {
                    return Err(crate::product_message!(
                        "backend.arxml.persistence.external_modification_overwrite_refused",
                        "path" => file.path.display()
                    ));
                }
                let mut handle = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&stage)
                    .map_err(|e| {
                        crate::message::LocalizedText::messages([
                            crate::product_message!(
                                "backend.arxml.persistence.staged_file_creation_failed",
                                "path" => stage.display()
                            ),
                            e.to_string().into(),
                        ])
                    })?;
                let written = std::io::Write::write_all(&mut handle, file.text.as_bytes())
                    .and_then(|_| handle.sync_all());
                drop(handle);
                if written.is_err() {
                    let _ = fs::remove_file(&stage);
                }
                written.map_err(|e| {
                    crate::message::LocalizedText::messages([
                        crate::product_message!(
                            "backend.arxml.persistence.staged_file_write_failed",
                            "path" => stage.display()
                        ),
                        e.to_string().into(),
                    ])
                })
            })();
            if let Err(error) = prepared {
                for (_, stage, _) in &staged {
                    let _ = fs::remove_file(stage);
                }
                return Err(error);
            }
            staged.push((file, stage, backup));
        }
        for index in 0..staged.len() {
            let (file, stage, backup) = &staged[index];
            if let Err(error) = install_staged_with_publish(file, stage, backup, &publish) {
                let mut rollback_errors = Vec::new();
                for (file, _, backup) in staged[..index].iter().rev() {
                    if let Err(rollback) = restore_backup(&file.path, backup, Some(&file.text)) {
                        rollback_errors.push(rollback);
                    }
                }
                for (_, stage, _) in &staged {
                    let _ = fs::remove_file(stage);
                }
                return Err(crate::message::LocalizedText::messages([
                    crate::product_message!("backend.arxml.persistence.arxml_save_failed"),
                    error,
                    crate::product_message!("backend.arxml.persistence.rollback_problems"),
                    crate::message::LocalizedText::messages(rollback_errors),
                ]));
            }
        }
        // Publication is complete. Keep the in-memory baseline aligned with disk
        // even when a backup must remain for manual recovery.
        for file in &mut self.files {
            file.saved = file.text.clone();
        }
        if let Some(project) = &mut self.project {
            project.saved = project.current.clone();
        }
        let mut cleanup_error = None;
        for (file, stage, backup) in &staged {
            if let Err(error) = remove_stage(stage) {
                cleanup_error = Some(crate::message::LocalizedText::messages([
                    crate::product_message!(
                        "backend.arxml.persistence.linked_stage_cleanup_failed_after_save",
                        "stage" => stage.display(),
                        "backup" => backup.display()
                    ),
                    error.to_string().into(),
                ]));
                break;
            }
            if fs::read_to_string(backup).ok().as_deref() != Some(&file.saved) {
                cleanup_error = Some(crate::product_message!(
                    "backend.arxml.persistence.backup_contents_changed",
                    "path" => backup.display()
                ));
                break;
            }
            if let Err(error) = remove_backup(backup) {
                cleanup_error = Some(crate::message::LocalizedText::messages([
                    crate::product_message!(
                        "backend.arxml.persistence.backup_cleanup_failed_after_save",
                        "path" => backup.display()
                    ),
                    error.to_string().into(),
                ]));
                break;
            }
        }
        if let Some(error) = cleanup_error {
            return Err(error);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SourceFile, install_staged, install_staged_with_publish, restore_backup,
        restore_backup_with_publish,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("autosar-save-{}-{nonce}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn published_save_keeps_memory_current_when_backup_cleanup_fails() {
        let root = Scratch::new();
        let directory = root.0.join("Project");
        let preview =
            crate::Workspace::preview_project_creation(&directory, "Project", "can-signals-v1")
                .unwrap();
        let mut workspace = crate::Workspace::create_project_previewed(&preview).unwrap();
        let source = workspace.files[0].path.clone();
        let before = workspace.files[0].saved.clone();
        workspace.files[0].text.push_str("\n<!-- saved edit -->\n");
        let project = workspace.project.as_mut().unwrap();
        project.current.push('\n');
        let error = workspace
            .save_sources_with_cleanup(
                |stage| fs::remove_file(stage),
                |_| {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "backup is locked",
                    ))
                },
            )
            .unwrap_err();
        assert_eq!(
            error,
            crate::message::LocalizedText::messages([
                crate::product_message!(
                    "backend.arxml.persistence.backup_cleanup_failed_after_save",
                    "path" => source
                        .with_extension(format!("arxml.autosar-config-{}-0.bak", std::process::id()))
                        .display()
                ),
                std::io::Error::new(std::io::ErrorKind::PermissionDenied, "backup is locked",)
                    .to_string()
                    .into(),
            ])
        );
        assert_ne!(fs::read_to_string(&source).unwrap(), before);
        for file in &workspace.files {
            assert_eq!(file.saved, file.text);
            assert_eq!(fs::read_to_string(&file.path).unwrap(), file.saved);
        }
        let project = workspace.project.as_ref().unwrap();
        assert_eq!(project.saved, project.current);
        assert_eq!(fs::read_to_string(&project.path).unwrap(), project.saved);
        workspace.ensure_sources_current().unwrap();
        assert!(!workspace.view().dirty);
        assert!(workspace.ensure_no_recovery_backups().is_err());
        let backup =
            source.with_extension(format!("arxml.autosar-config-{}-0.bak", std::process::id()));
        assert_eq!(fs::read_to_string(backup).unwrap(), before);
    }

    #[test]
    fn published_save_stage_cleanup_failure_retains_clean_baseline_and_backup() {
        let root = Scratch::new();
        let mut workspace = crate::Workspace::create(&root.0.join("Project"), "Project").unwrap();
        let source = workspace.files[0].path.clone();
        let before = workspace.files[0].saved.clone();
        workspace.files[0].text.push_str("\n<!-- saved edit -->\n");
        let error = workspace
            .save_sources_with_cleanup(
                |_| {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "stage is locked",
                    ))
                },
                |backup| fs::remove_file(backup),
            )
            .unwrap_err();
        let stage =
            source.with_extension(format!("arxml.autosar-config-{}-0.tmp", std::process::id()));
        let backup =
            source.with_extension(format!("arxml.autosar-config-{}-0.bak", std::process::id()));
        assert!(error.to_string().contains(&stage.display().to_string()));
        assert!(error.to_string().contains(&backup.display().to_string()));
        assert_eq!(
            fs::read_to_string(&source).unwrap(),
            workspace.files[0].saved
        );
        assert_eq!(
            fs::read_to_string(&stage).unwrap(),
            workspace.files[0].saved
        );
        assert_eq!(fs::read_to_string(&backup).unwrap(), before);
        assert!(!workspace.view().dirty);
        assert!(workspace.ensure_no_recovery_backups().is_err());
        fs::write(&stage, "external edit through linked stage").unwrap();
        assert_eq!(
            fs::read_to_string(&source).unwrap(),
            "external edit through linked stage"
        );
        assert!(workspace.save_sources().is_err());
        assert_eq!(
            fs::read_to_string(&source).unwrap(),
            "external edit through linked stage"
        );
        assert_eq!(fs::read_to_string(&backup).unwrap(), before);
    }

    #[test]
    fn unsupported_backup_reservation_leaves_original_untouched() {
        let root = Scratch::new();
        let original = root.0.join("Ecu.arxml");
        let stage = root.0.join("Ecu.tmp");
        let backup = root.0.join("Ecu.bak");
        let file = SourceFile {
            path: original.clone(),
            saved: "original".into(),
            text: "ours".into(),
            original_name: None,
        };
        fs::write(&original, &file.saved).unwrap();
        fs::write(&stage, &file.text).unwrap();
        let error = install_staged_with_publish(&file, &stage, &backup, |from, to| {
            assert_eq!(from, stage);
            assert_eq!(to, backup);
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "hard links unsupported",
            ))
        })
        .unwrap_err();
        assert!(error.to_string().contains("hard links unsupported"));
        assert_eq!(fs::read_to_string(&original).unwrap(), "original");
        assert_eq!(fs::read_to_string(&stage).unwrap(), "ours");
        assert!(!backup.exists());
        fs::write(&backup, "external raced backup").unwrap();
        assert!(install_staged(&file, &stage, &backup).is_err());
        assert_eq!(fs::read_to_string(&original).unwrap(), "original");
        assert_eq!(
            fs::read_to_string(&backup).unwrap(),
            "external raced backup"
        );
    }

    #[test]
    fn later_manifest_publication_conflict_rolls_back_source_transaction() {
        let root = Scratch::new();
        let preview = crate::Workspace::preview_project_creation(
            &root.0.join("Project"),
            "Project",
            "can-signals-v1",
        )
        .unwrap();
        let mut workspace = crate::Workspace::create_project_previewed(&preview).unwrap();
        let source = workspace.files[0].path.clone();
        let source_before = workspace.files[0].saved.clone();
        workspace.files[0]
            .text
            .push_str("\n<!-- pending edit -->\n");
        let source_pending = workspace.files[0].text.clone();
        let project = workspace.project.as_mut().unwrap();
        let manifest = project.path.clone();
        let manifest_before = project.saved.clone();
        project.current.push('\n');
        let manifest_pending = project.current.clone();
        let error = workspace
            .save_sources_with_publish(
                |stage| fs::remove_file(stage),
                |backup| fs::remove_file(backup),
                |from, to| {
                    if to == manifest {
                        fs::write(to, "external manifest during publication").unwrap();
                    }
                    fs::hard_link(from, to)
                },
            )
            .unwrap_err();
        let backup =
            manifest.with_extension(format!("arxml.autosar-config-{}-1.bak", std::process::id()));
        assert!(error.to_string().contains(&backup.display().to_string()));
        assert_eq!(fs::read_to_string(&source).unwrap(), source_before);
        assert_eq!(
            fs::read_to_string(&manifest).unwrap(),
            "external manifest during publication"
        );
        assert_eq!(fs::read_to_string(&backup).unwrap(), manifest_before);
        assert!(
            !source
                .with_extension(format!("arxml.autosar-config-{}-0.bak", std::process::id()))
                .exists()
        );
        assert_eq!(workspace.files[0].saved, source_before);
        assert_eq!(workspace.files[0].text, source_pending);
        let project = workspace.project.as_ref().unwrap();
        assert_eq!(project.saved, manifest_before);
        assert_eq!(project.current, manifest_pending);
        assert!(workspace.view().dirty);
        assert!(workspace.ensure_no_recovery_backups().is_err());
        assert!(workspace.save_sources().is_err());
        assert_eq!(
            fs::read_to_string(&manifest).unwrap(),
            "external manifest during publication"
        );
        assert_eq!(fs::read_to_string(&backup).unwrap(), manifest_before);
    }

    #[test]
    fn rollback_only_recovery_directory_blocks_before_mutation() {
        let root = Scratch::new();
        let mut workspace = crate::Workspace::create(&root.0.join("Project"), "Project").unwrap();
        let source = workspace.files[0].path.clone();
        let before = workspace.files[0].saved.clone();
        workspace.files[0]
            .text
            .push_str("\n<!-- pending edit -->\n");
        let recovery = source.with_extension("arxml.autosar-config-old-0.rollback");
        fs::create_dir(&recovery).unwrap();
        fs::write(recovery.join("original"), "retained recovery bytes").unwrap();
        let error = workspace.save_sources().unwrap_err();
        assert!(error.to_string().contains(&recovery.display().to_string()));
        assert_eq!(fs::read_to_string(&source).unwrap(), before);
        assert_eq!(workspace.files[0].saved, before);
        assert!(workspace.view().dirty);
        assert_eq!(
            fs::read_to_string(recovery.join("original")).unwrap(),
            "retained recovery bytes"
        );
    }

    #[test]
    fn rollback_preserves_external_directory_at_original_path() {
        let root = Scratch::new();
        let original = root.0.join("Ecu.arxml");
        let backup = root.0.join("Ecu.bak");
        fs::create_dir(&original).unwrap();
        fs::write(original.join("external"), "directory contents").unwrap();
        fs::write(&backup, "original").unwrap();
        assert!(restore_backup(&original, &backup, Some("ours")).is_err());
        assert!(original.is_dir());
        assert_eq!(
            fs::read_to_string(original.join("external")).unwrap(),
            "directory contents"
        );
        assert_eq!(fs::read_to_string(&backup).unwrap(), "original");
        assert!(!backup.with_extension("rollback").exists());
    }

    #[test]
    fn staged_save_preserves_external_edit_between_preparation_and_installation() {
        let root = Scratch::new();
        let original = root.0.join("Ecu.arxml");
        let stage = root.0.join("Ecu.tmp");
        let backup = root.0.join("Ecu.bak");
        let file = SourceFile {
            path: original.clone(),
            saved: "original".into(),
            text: "ours".into(),
            original_name: None,
        };
        fs::write(&original, &file.saved).unwrap();
        fs::write(&stage, &file.text).unwrap();
        fs::write(&original, "external").unwrap();
        let conflict = install_staged(&file, &stage, &backup).unwrap_err();
        assert_eq!(
            conflict,
            crate::product_message!(
                "backend.arxml.persistence.external_modification_overwrite_refused",
                "path" => original.display()
            )
        );
        assert_eq!(fs::read_to_string(&original).unwrap(), "external");
        assert!(!backup.exists());
        assert!(stage.exists());

        fs::write(&original, &file.saved).unwrap();
        install_staged(&file, &stage, &backup).unwrap();
        assert_eq!(fs::read_to_string(&original).unwrap(), "ours");
        assert_eq!(fs::read_to_string(&backup).unwrap(), "original");
        restore_backup(&original, &backup, Some(&file.text)).unwrap();
        assert_eq!(fs::read_to_string(&original).unwrap(), "original");
    }

    #[test]
    fn staged_save_refuses_external_file_created_at_final_publication() {
        let root = Scratch::new();
        let original = root.0.join("Ecu.arxml");
        let stage = root.0.join("Ecu.tmp");
        let backup = root.0.join("Ecu.bak");
        let file = SourceFile {
            path: original.clone(),
            saved: "original".into(),
            text: "ours".into(),
            original_name: None,
        };
        fs::write(&original, &file.saved).unwrap();
        fs::write(&stage, &file.text).unwrap();
        let error = install_staged_with_publish(&file, &stage, &backup, |from, to| {
            if to == original {
                assert_eq!(fs::read_to_string(&backup).unwrap(), "original");
                fs::write(to, "external").unwrap();
            }
            fs::hard_link(from, to)
        })
        .unwrap_err();
        assert!(error.to_string().contains(&backup.display().to_string()));
        assert_eq!(fs::read_to_string(&original).unwrap(), "external");
        assert_eq!(fs::read_to_string(&backup).unwrap(), "original");
        assert_eq!(fs::read_to_string(&stage).unwrap(), "ours");
    }

    #[test]
    fn rollback_publication_conflict_preserves_external_and_recovery_files() {
        for installed in [None, Some("ours"), Some("external replaced ours")] {
            let root = Scratch::new();
            let original = root.0.join("Ecu.arxml");
            let backup = root.0.join("Ecu.bak");
            fs::write(&backup, "original").unwrap();
            if let Some(text) = installed {
                fs::write(&original, text).unwrap();
            }
            let error = restore_backup_with_publish(
                &original,
                &backup,
                installed.map(|_| "ours"),
                |from, to| {
                    fs::write(to, "external during restore").unwrap();
                    fs::hard_link(from, to)
                },
            )
            .unwrap_err();
            assert_eq!(
                fs::read_to_string(&original).unwrap(),
                "external during restore"
            );
            assert_eq!(fs::read_to_string(&backup).unwrap(), "original");
            if let Some(text) = installed {
                let captured = backup.with_extension("rollback").join("original");
                assert_eq!(fs::read_to_string(&captured).unwrap(), text);
                assert!(error.to_string().contains(&captured.display().to_string()));
            }
        }
    }

    #[test]
    fn rollback_never_removes_a_new_external_original() {
        let root = Scratch::new();
        let original = root.0.join("Ecu.arxml");
        let backup = root.0.join("Ecu.bak");
        fs::write(&original, "external").unwrap();
        fs::write(&backup, "original").unwrap();
        assert!(restore_backup(&original, &backup, Some("ours")).is_err());
        assert_eq!(fs::read_to_string(&original).unwrap(), "external");
        assert_eq!(fs::read_to_string(&backup).unwrap(), "original");
    }
}
