use crate::model::{GenerationPreview, GenerationPreviewFile, GenerationReport, Issue};
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::fmt::Write;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub(super) fn file_digest(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|e| format!("生成文件缺失或无法读取 {}: {e}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| format!("生成文件无法读取 {}: {e}", path.display()))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn integrity_record(dir: &Path, names: &[String]) -> Result<String, String> {
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
) -> Result<PathBuf, String> {
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
            Err(error) => return Err(format!("无法保留临时目录 {}: {error}", candidate.display())),
        }
    }
    Err("无法分配唯一的临时目录".into())
}

fn check_entries(dir: &Path, root: &Path, names: &[String]) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("输出目录包含非 UTF-8 文件名")?
            .replace('\\', "/");
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if is_reparse_point(&metadata) {
            return Err(format!("输出目录含链接或重解析点，拒绝替换: {name}"));
        }
        let kind = metadata.file_type();
        if kind.is_dir() {
            // A directory is generated only when it is an ancestor of a
            // declared file. This also covers nested kernel/OS/input trees;
            // arbitrary owner directories, even empty ones, still refuse.
            let prefix = format!("{name}/");
            if !names.iter().any(|item| item.starts_with(&prefix)) {
                return Err(format!("输出目录含用户目录，拒绝替换: {name}"));
            }
            check_entries(&path, root, names)?;
        } else if !kind.is_file()
            || (name != "files.list"
                && name != "files.sha256"
                && names.binary_search(&name).is_err())
        {
            return Err(format!("输出目录含用户文件或链接，拒绝替换: {name}"));
        }
    }
    Ok(())
}

fn verify_generated_output(dir: &Path, names: &[String]) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(dir) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    if is_reparse_point(&metadata) || !metadata.file_type().is_dir() {
        return Err(format!(
            "输出目录不是普通目录或是重解析点，拒绝覆盖: {}",
            dir.display()
        ));
    }
    if fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .next()
        .is_none()
    {
        return Ok(());
    }
    let previous = fs::read_to_string(dir.join("files.list"))
        .map_err(|_| format!("输出目录缺少受支持的文件清单，拒绝覆盖: {}", dir.display()))?;
    if previous != format!("{}\n", names.join("\n")) {
        return Err("已有输出清单不匹配，拒绝删除或覆盖其他生成版本".into());
    }
    check_entries(dir, dir, names)?;
    let recorded = fs::read_to_string(dir.join("files.sha256")).map_err(|_| {
        format!(
            "输出目录缺少完整性记录，拒绝覆盖旧版生成目录: {}",
            dir.display()
        )
    })?;
    if recorded != integrity_record(dir, names)? {
        return Err("生成文件或完整性记录已被修改，拒绝覆盖用户内容".into());
    }
    Ok(())
}

pub(crate) fn verify_build_input(output: &Path) -> Result<Vec<String>, String> {
    let list = fs::read_to_string(output.join("files.list"))
        .map_err(|e| format!("生成工程缺少可读文件清单: {e}"))?;
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
        return Err("生成工程文件清单格式或路径无效，拒绝构建".into());
    }
    verify_generated_output(output, &names)
        .map_err(|e| format!("生成工程完整性检查失败，拒绝构建: {e}"))?;
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
pub(super) fn output_path(output: &Path) -> Result<PathBuf, String> {
    let output_name = output.file_name().ok_or("输出目录须有名称")?;
    let parent = output.parent().ok_or("输出目录须有父目录")?;
    if output_name == "." || output_name == ".." || parent.join(output_name) != output {
        return Err("输出目录须为明确的命名路径，不能以 . 或 .. 结尾".into());
    }
    let parent = if parent.is_absolute() {
        parent.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
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
) -> Result<GenerationPreview, String> {
    let output = output_path(output)?;
    let names = file_names(files);
    verify_generated_output(&output, &names)?;
    let mut digest = Sha256::new();
    digest.update(output.to_string_lossy().as_bytes());
    let mut changes = Vec::new();
    for (name, after) in files {
        let path = output.join(name);
        let before = match fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("无法读取旧生成文件 {}: {error}", path.display())),
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
            path: name.clone(),
            status: status.into(),
            before: if status == "changed" {
                Some(
                    String::from_utf8(before.unwrap())
                        .map_err(|_| format!("旧生成文件不是 UTF-8 文本: {name}"))?,
                )
            } else {
                None
            },
            after: if status != "unchanged" {
                Some(
                    String::from_utf8(after.clone())
                        .map_err(|_| format!("新生成文件不是 UTF-8 文本: {name}"))?,
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
) -> Result<GenerationPreview, String> {
    inspect_prepared(files, output, true)
}

/// An owned private source stage; dropping it never alters the destination.
pub struct StagedGeneration {
    files: Vec<(String, Vec<u8>)>,
    names: Vec<String>,
    output: PathBuf,
    stage: Option<PathBuf>,
    revision: String,
}

impl StagedGeneration {
    pub(crate) fn new(
        files: Vec<(String, Vec<u8>)>,
        output: &Path,
        expected_revision: Option<&str>,
    ) -> Result<Self, String> {
        let output = output_path(output)?;
        let revision = inspect_prepared(&files, &output, false)?.revision;
        if expected_revision.is_some_and(|expected| expected != revision) {
            return Err("生成预览已失效：配置、来源、目标或旧输出已变化；请重新预览".into());
        }
        let names = file_names(&files);
        let parent = output.parent().ok_or("输出目录须有父目录")?;
        let output_name = output.file_name().ok_or("输出目录须有名称")?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let stage = reserve_directory(parent, "stage", output_name)?;
        let staged = Self {
            files,
            names,
            output,
            stage: Some(stage),
            revision,
        };
        for (name, contents) in &staged.files {
            let target = staged.stage.as_ref().unwrap().join(name);
            fs::create_dir_all(target.parent().unwrap()).map_err(|error| error.to_string())?;
            fs::write(target, contents).map_err(|error| error.to_string())?;
        }
        Ok(staged)
    }

    /// Call while holding the workbench's fingerprint/operation commit lock.
    pub fn commit(mut self) -> Result<GenerationReport, String> {
        if inspect_prepared(&self.files, &self.output, false)?.revision != self.revision {
            return Err("生成预览已失效：旧输出在确认期间变化；请重新预览".into());
        }
        let output = &self.output;
        let parent = output.parent().ok_or("输出目录须有父目录")?;
        let output_name = output.file_name().ok_or("输出目录须有名称")?;
        let existing = match fs::symlink_metadata(&output) {
            Ok(metadata) if is_reparse_point(&metadata) || !metadata.file_type().is_dir() => {
                return Err(format!(
                    "输出目录已变为链接或非普通目录，拒绝替换: {}",
                    output.display()
                ));
            }
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.to_string()),
        };
        let backup = if existing {
            let backup_root = reserve_directory(parent, "backup", output_name)?;
            let preserved = backup_root.join(output_name);
            fs::rename(&output, &preserved).map_err(|e| {
                format!(
                    "无法保留旧生成工程 {} 至 {}: {e}",
                    output.display(),
                    preserved.display()
                )
            })?;
            Some(preserved)
        } else {
            None
        };
        if let Err(error) = fs::rename(self.stage.as_ref().unwrap(), output) {
            let recovery = backup
                .as_ref()
                .map(|path| format!("；原输出保留在 {}", path.display()))
                .unwrap_or_default();
            return Err(format!("无法安装新生成工程: {error}{recovery}"));
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
) -> Result<GenerationReport, String> {
    StagedGeneration::new(files, output, expected_revision)?.commit()
}

/// Native build output remains private until a fingerprint-checked commit.
pub struct StagedBuild {
    root: PathBuf,
    directory: PathBuf,
    output: PathBuf,
}

impl StagedBuild {
    pub fn new(project: &Path, output: &Path) -> Result<Self, String> {
        let output = output_path(output)?;
        for path in output.ancestors().chain(project.ancestors()) {
            match fs::symlink_metadata(path) {
                Ok(metadata) if is_reparse_point(&metadata) => {
                    return Err(format!("构建路径不能经过链接: {}", path.display()));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        let project = project.canonicalize().map_err(|error| error.to_string())?;
        let requested_parent = output.parent().ok_or("构建输出须有父目录")?;
        let mut existing = requested_parent;
        while !existing.try_exists().map_err(|error| error.to_string())? {
            existing = existing.parent().ok_or("构建输出须有现存父目录")?;
        }
        let parent = existing
            .canonicalize()
            .map_err(|error| error.to_string())?
            .join(
                requested_parent
                    .strip_prefix(existing)
                    .map_err(|error| error.to_string())?,
            );
        let output = parent.join(output.file_name().ok_or("构建输出须有名称")?);
        if output.starts_with(&project) || project.starts_with(&output) {
            return Err("构建输出必须位于生成源码之外".into());
        }
        check_empty_build_output(&output)?;
        fs::create_dir_all(&parent).map_err(|error| error.to_string())?;
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

    pub fn commit(mut self) -> Result<PathBuf, String> {
        check_empty_build_output(&self.output)?;
        if self.output.exists() {
            fs::remove_dir(&self.output).map_err(|error| error.to_string())?;
        }
        fs::rename(&self.directory, &self.output).map_err(|error| error.to_string())?;
        Ok(std::mem::take(&mut self.output))
    }
}

impl Drop for StagedBuild {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn check_empty_build_output(output: &Path) -> Result<(), String> {
    match fs::symlink_metadata(output) {
        Ok(metadata) if is_reparse_point(&metadata) || !metadata.is_dir() => {
            Err("构建输出不能是链接或非普通目录".into())
        }
        Ok(_) => {
            if fs::read_dir(output)
                .map_err(|error| error.to_string())?
                .next()
                .is_some()
            {
                Err("构建输出必须为新目录或空目录；所有者文件未修改".into())
            } else {
                Ok(())
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}
