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
    for suffix in 0u64.. {
        let candidate = parent.join(format!(
            ".autosar-config-{role}-{}-{suffix}",
            std::process::id()
        ));
        if candidate.file_name() == Some(output_name) {
            continue;
        }
        match fs::create_dir(&candidate) {
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

pub(crate) fn preview_prepared(
    files: &[(String, Vec<u8>)],
    output: &Path,
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
pub(crate) fn generate_prepared(
    files: Vec<(String, Vec<u8>)>,
    output: &Path,
    expected_revision: Option<&str>,
) -> Result<GenerationReport, String> {
    let output = output_path(output)?;
    if let Some(revision) = expected_revision
        && preview_prepared(&files, &output)?.revision != revision
    {
        return Err("生成预览已失效：配置、来源、目标或旧输出已变化；请重新预览".into());
    }
    let names = file_names(&files);
    let parent = output.parent().ok_or("输出目录须有父目录")?;
    let output_name = output.file_name().ok_or("输出目录须有名称")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    verify_generated_output(&output, &names)?;
    let stage = reserve_directory(parent, "stage", output_name)?;
    let result = (|| {
        for (name, contents) in &files {
            let target = stage.join(name);
            fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::write(target, contents).map_err(|e| e.to_string())?;
        }
        verify_generated_output(&output, &names)?;
        if let Some(revision) = expected_revision
            && preview_prepared(&files, &output)?.revision != revision
        {
            return Err("生成预览已失效：旧输出在确认期间变化；请重新预览".into());
        }
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
        if let Err(error) = fs::rename(&stage, &output) {
            let recovery = backup
                .as_ref()
                .map(|path| format!("；原输出保留在 {}", path.display()))
                .unwrap_or_default();
            return Err(format!("无法安装新生成工程: {error}{recovery}"));
        }
        Ok(backup)
    })();
    let backup =
        result.map_err(|error| format!("{error}；临时生成目录保留在 {}", stage.display()))?;
    Ok(GenerationReport {
        output_directory: output.display().to_string(),
        previous_output_directory: backup.map(|path| path.display().to_string()),
        files: names,
        issues: Vec::<Issue>::new(),
    })
}
