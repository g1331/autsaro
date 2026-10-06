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
) -> Result<(), String> {
    let leaf = node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == child_name)
        .ok_or_else(|| format!("{} 缺少 {child_name}，拒绝不安全编辑", path_of(node)))?;
    let mut content = leaf.children();
    let text = content
        .next()
        .filter(|n| n.is_text())
        .ok_or("值不是简单文本；拒绝不安全编辑")?;
    if content.next().is_some() {
        return Err("值有混合内容；拒绝不安全编辑".into());
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
) -> Result<(), String> {
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
    let parameter = found
        .next()
        .ok_or_else(|| format!("缺少 {name}，拒绝不安全编辑"))?;
    if found.next().is_some() {
        return Err(format!("{name} 存在多个变体，拒绝不安全编辑"));
    }
    patch_child(parameter, "VALUE", value, patches)
}

pub(super) fn apply_patches(text: &mut String, patches: &mut Vec<Patch>) -> Result<(), String> {
    patches.sort_by_key(|patch| std::cmp::Reverse(patch.range.start));
    let mut next_start = text.len();
    for patch in patches {
        if patch.range.end > next_start {
            return Err("编辑范围重叠；保留原 ARXML".into());
        }
        next_start = patch.range.start;
        text.replace_range(patch.range.clone(), &patch.value);
    }
    Ok(())
}

fn restore_backup(original: &Path, backup: &Path, installed: Option<&str>) -> Result<(), String> {
    if original.exists() {
        let owned = installed
            .is_some_and(|text| fs::read_to_string(original).is_ok_and(|current| current == text));
        if !owned {
            return Err(format!(
                "外部文件 {} 未覆盖；原备份保留在 {}",
                original.display(),
                backup.display()
            ));
        }
        fs::remove_file(original).map_err(|e| format!("{}: {e}", original.display()))?;
    }
    fs::rename(backup, original)
        .map_err(|e| format!("{} -> {}: {e}", backup.display(), original.display()))
}

fn install_staged(file: &SourceFile, stage: &Path, backup: &Path) -> Result<(), String> {
    fs::rename(&file.path, backup)
        .map_err(|e| format!("{} -> {}: {e}", file.path.display(), backup.display()))?;
    let result = fs::read_to_string(backup)
        .map_err(|e| format!("无法复核原文件 {}: {e}", backup.display()))
        .and_then(|actual| {
            if actual != file.saved {
                return Err(format!(
                    "文件已被外部修改，拒绝覆盖: {}",
                    file.path.display()
                ));
            }
            fs::rename(stage, &file.path)
                .map_err(|e| format!("{} -> {}: {e}", stage.display(), file.path.display()))
        });
    if let Err(error) = result {
        return Err(match restore_backup(&file.path, backup, None) {
            Ok(()) => error,
            Err(rollback) => format!("{error}; 回滚问题: {rollback}"),
        });
    }
    Ok(())
}

/// A validated immutable save snapshot; the complete disk transaction stays here.
pub struct PreparedSave {
    workspace: Workspace,
}

pub struct SaveFailure {
    workspace: Workspace,
    message: String,
}

impl SaveFailure {
    pub fn into_parts(self) -> (Workspace, String) {
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
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SaveFailure {}

impl PreparedSave {
    pub(super) fn validated(workspace: Workspace, revision: &str) -> Result<Self, String> {
        if workspace.save_revision() != revision {
            return Err("配置已在预览后改变，请重新查看 ARXML 改动再保存".into());
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
    pub fn prepare_save_previewed(mut self, revision: &str) -> Result<PreparedSave, String> {
        self.ensure_save_allowed()?;
        PreparedSave::validated(self, revision)
    }

    pub(super) fn commit_patches(
        &mut self,
        mut patches: Vec<Vec<Patch>>,
        matches: impl FnOnce(&Workspace) -> bool,
    ) -> Result<(), String> {
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
                Err(format!("{}: {}", issue.code, issue.message))
            } else if matches(self) {
                Ok(())
            } else {
                Err("编辑后的 ARXML 与配置模型不一致".into())
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
    ) -> Result<(), String> {
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
                            seconds(new.period_ms.ok_or("发送周期不可为空")?),
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
                            seconds(new.timeout_ms.ok_or("接收超时不可为空")?),
                            &mut patches[index],
                        )?;
                        found_timeout += 1;
                    }
                }
                if old.id != new.id && node.tag_name().name() == "CAN-FRAME-TRIGGERING" {
                    return Err(
                        "导入项目包含 CAN 网络触发配置，修改标识符需同步网络模型；已阻止不安全编辑"
                            .into(),
                    );
                }
                if old.dlc != new.dlc && node.tag_name().name() == "CAN-FRAME" {
                    return Err(
                        "导入项目包含 CAN-FRAME，修改 DLC 需同步网络模型；已阻止不安全编辑".into(),
                    );
                }
            }
        }
        if (old.id != new.id && !found_id)
            || (old.dlc != new.dlc && (!found_dlc || !found_global_length))
            || (old.period_ms != new.period_ms && !found_period)
            || (old.timeout_ms != new.timeout_ms && found_timeout != receive_signals.len())
        {
            return Err("导入项目缺少可定位的标准参数，已拒绝修改且保留原文件".into());
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
    ) -> Result<(), String> {
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
            return Err("信号系统映射不完整；拒绝破坏未知引用或位布局".into());
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

    pub(super) fn ensure_sources_current(&self) -> Result<(), String> {
        for file in &self.files {
            super::project::safe_path(&file.path, false)?;
        }
        if let Some(project) = &self.project {
            super::project::safe_path(&project.path, false)?;
            if super::project::read_bounded(&project.path)? != project.saved.as_bytes() {
                return Err(
                    "Project manifest was externally modified; reopen before changing or saving."
                        .into(),
                );
            }
            for (path, saved) in &project.application_bytes {
                let path = project
                    .path
                    .parent()
                    .ok_or("Project root is missing.")?
                    .join(path);
                super::project::safe_path(&path, false)?;
                if super::project::read_bounded(&path)? != *saved {
                    return Err(
                        "Application source changed outside the owned project snapshot.".into(),
                    );
                }
            }
        }
        for file in &self.files {
            let disk = super::project::read_bounded(&file.path)?;
            if disk != file.saved.as_bytes() {
                return Err(format!(
                    "文件已被外部修改，拒绝基于过期配置继续；请重新导入项目: {}",
                    file.path.display()
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

    pub fn preview_save(&mut self) -> Result<SavePreview, String> {
        self.ensure_save_allowed()?;
        Ok(SavePreview {
            revision: self.save_revision(),
            files: self.save_preview_files(),
        })
    }

    pub fn save_previewed(&mut self, revision: &str) -> Result<WorkspaceView, String> {
        if self.save_revision() != revision {
            return Err("配置已在预览后改变，请重新查看 ARXML 改动再保存".into());
        }
        self.save()
    }

    pub fn save(&mut self) -> Result<WorkspaceView, String> {
        self.ensure_save_allowed()?;
        // Validation includes references across every imported file, including files this
        // edit leaves untouched. A stale untouched file would invalidate that result.
        self.save_sources()?;
        Ok(self.view())
    }

    pub(super) fn ensure_save_allowed(&mut self) -> Result<(), String> {
        self.validate()?;
        self.ensure_no_recovery_backups()?;
        if self.uses_legacy_validation() {
            if let Some(issue) = self
                .issues
                .iter()
                .find(|issue| matches!(issue.severity, Severity::Error))
            {
                return Err(format!("{}: {}", issue.code, issue.message));
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
                        return Err(format!("{}: {}", issue.code, issue.message));
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

    fn ensure_no_recovery_backups(&self) -> Result<(), String> {
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
                .ok_or("Recovery file path is not UTF-8.")?;
            for entry in fs::read_dir(path.parent().ok_or("Source directory is missing.")?)
                .map_err(|error| error.to_string())?
            {
                let entry = entry.map_err(|error| error.to_string())?;
                if entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".bak"))
                {
                    return Err(format!(
                        "Unrecovered source backup prevents saving: {}",
                        entry.path().display()
                    ));
                }
            }
        }
        Ok(())
    }

    pub(super) fn save_sources(&mut self) -> Result<(), String> {
        self.save_sources_with_cleanup(|backup| fs::remove_file(backup))
    }

    fn save_sources_with_cleanup(
        &mut self,
        remove_backup: impl Fn(&Path) -> std::io::Result<()>,
    ) -> Result<(), String> {
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
            let prepared = (|| -> Result<(), String> {
                if backup.exists() {
                    return Err(format!("待恢复备份已存在，拒绝覆盖: {}", backup.display()));
                }
                if fs::read_to_string(&file.path).map_err(|e| e.to_string())? != file.saved {
                    return Err(format!(
                        "文件已被外部修改，拒绝覆盖: {}",
                        file.path.display()
                    ));
                }
                let mut handle = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&stage)
                    .map_err(|e| format!("暂存文件不可创建 {}: {e}", stage.display()))?;
                let written = std::io::Write::write_all(&mut handle, file.text.as_bytes())
                    .and_then(|_| handle.sync_all());
                drop(handle);
                if written.is_err() {
                    let _ = fs::remove_file(&stage);
                }
                written.map_err(|e| format!("暂存文件写入失败 {}: {e}", stage.display()))
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
            if let Err(error) = install_staged(file, stage, backup) {
                let mut rollback_errors = Vec::new();
                for (file, _, backup) in staged[..index].iter().rev() {
                    if let Err(rollback) = restore_backup(&file.path, backup, Some(&file.text)) {
                        rollback_errors.push(rollback);
                    }
                }
                for (_, stage, _) in &staged {
                    let _ = fs::remove_file(stage);
                }
                return Err(format!(
                    "ARXML 保存失败: {error}; 回滚问题: {}",
                    rollback_errors.join("; ")
                ));
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
        for (file, _, backup) in &staged {
            if fs::read_to_string(backup).ok().as_deref() != Some(&file.saved) {
                cleanup_error = Some(format!(
                    "备份内容发生变化，保留备份供检查: {}",
                    backup.display()
                ));
                break;
            }
            if let Err(error) = remove_backup(backup) {
                cleanup_error = Some(format!(
                    "已保存 ARXML，但无法清理备份 {}: {error}",
                    backup.display()
                ));
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
    use super::{SourceFile, install_staged, restore_backup};
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
            .save_sources_with_cleanup(|_| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "backup is locked",
                ))
            })
            .unwrap_err();
        assert!(error.contains("无法清理备份"), "{error}");
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
        assert!(conflict.contains("外部修改"), "{conflict}");
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
