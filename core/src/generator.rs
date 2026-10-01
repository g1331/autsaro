use crate::arxml::Workspace;
use crate::model::{
    BuildReport, DiagnosticView, Direction, GenerationPreview, GenerationPreviewFile,
    GenerationReport, Issue, SignalView,
};
use crate::target::BuildTarget;
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::fmt::Write;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};


pub(crate) fn handoff_readme(
    diagnostic: Option<&DiagnosticView>,
    target: BuildTarget,
    handoff: bool,
) -> Result<String, String> {
    let binary = if target == BuildTarget::WindowsX64ControlledV1 {
        "ecu_host.exe"
    } else {
        "ecu_host"
    };
    let mut run = format!("../build/{binary}");
    let mut notes = String::new();
    if let Some(diagnostic) = diagnostic {
        if diagnostic.dtc.is_some() {
            run.push_str(" --nvm ../state/ecu.nvm");
            notes.push_str("Create the external `../state/` directory first. `--nvm` is an exclusive host DTC state file; a missing file is initialized, while a damaged or mismatched existing file stops startup.\n\n");
        }
        if diagnostic.security_enabled {
            run.push_str(" --security-key ../state/ecu.key --security-state ../state/ecu.security");
            notes.push_str("Create `../state/ecu.key` separately as exactly 32 raw secret bytes. It is not generated or part of the source package. Each ECU needs its own security state file; a missing key or damaged state stops startup. This security profile is supported only by the Windows target.\n\n");
        }
    }
    let inputs = if handoff {
        "Saved original ARXML inputs are included under `inputs/` and mapped in `handoff.json`. Review their raw contents before sharing. Reimport with the same-version workbench and the separately obtained matching XSD, then regenerate the recorded explicit target to reproduce the complete source closure."
    } else {
        "The original ARXML sources are not included. Retain them separately to edit or regenerate this source project, or generate the versioned host handoff profile when saved original inputs must travel with it."
    };
    let asset = crate::resources::AssetInventory::embedded()
        .get("runtime/generated-README.md")
        .ok_or("The trusted host delivery README template is missing")?;
    let mut remaining = std::str::from_utf8(asset.bytes).map_err(|error| error.to_string())?;
    let mut result =
        String::with_capacity(remaining.len() + run.len() + notes.len() + inputs.len());
    while let Some((prefix, tail)) = remaining.split_once("{{") {
        result.push_str(prefix);
        let (name, tail) = tail
            .split_once("}}")
            .ok_or("Unclosed host README placeholder")?;
        result.push_str(match name {
            "TARGET" => target.spec().id,
            "BINARY" => binary,
            "RUN_COMMAND" => &run,
            "RUN_NOTES" => notes.trim_end(),
            "INPUT_NOTE" => inputs,
            _ => return Err(format!("Unknown host README placeholder: {name}")),
        });
        remaining = tail;
    }
    result.push_str(remaining);
    Ok(result)
}

fn config_source(
    name: &str,
    frames: &[crate::model::FrameView],
    signals: &[SignalView],
    diagnostic: Option<&DiagnosticView>,
) -> Result<(String, String, String), String> {
    let mut source = String::from("#include \"Ecu_Config.h\"\n#include \"Dcm_Externals.h\"\n");
    if diagnostic.is_some() {
        source.push_str("#include \"Rte.h\"\n");
    }
    let mut externals = String::from(
        "#ifndef DCM_EXTERNALS_H\n#define DCM_EXTERNALS_H\n\n#include <stdint.h>\n#include \"Ecu_DcmCallbackTypes.h\"\n\n",
    );
    source.push_str("\nstatic const EcuSignalConfig signals[] = {\n");
    let mut frame_rows = Vec::new();
    let mut map = format!("ECU {name}\n# ID 映射由已验证的 ARXML 路径按字典序稳定生成\n");
    let mut signal_ids = std::collections::BTreeMap::new();
    let mut next_id = 0usize;
    for frame in frames {
        let mut frame_signals: Vec<_> = signals
            .iter()
            .filter(|s| s.frame_path == frame.path)
            .collect();
        frame_signals.sort_by(|a, b| a.path.cmp(&b.path));
        let first = next_id;
        for signal in frame_signals {
            writeln!(
                source,
                "    {{ {}u, {}u, {}u, {}u }},",
                next_id, signal.start_bit, signal.length, signal.initial_value
            )
            .unwrap();
            writeln!(
                map,
                "SIGNAL {} {} frame={} bits={}:{} initial={}",
                next_id,
                signal.path,
                frame.path,
                signal.start_bit,
                signal.length,
                signal.initial_value
            )
            .unwrap();
            signal_ids.insert(signal.path.as_str(), next_id);
            next_id += 1;
        }
        frame_rows.push(format!(
            "    {{ {}u, {}u, {}u, {}u, {}u, {}u, {}u }},",
            frame.id,
            frame.dlc,
            matches!(frame.direction, Direction::Tx) as u8,
            first,
            next_id - first,
            frame.period_ms.unwrap_or(0),
            frame.timeout_ms.unwrap_or(0)
        ));
        writeln!(
            map,
            "FRAME {} {} id={} dlc={} direction={} period={} timeout={}",
            frame_rows.len() - 1,
            frame.path,
            frame.id,
            frame.dlc,
            if matches!(frame.direction, Direction::Tx) {
                "tx"
            } else {
                "rx"
            },
            frame.period_ms.unwrap_or(0),
            frame.timeout_ms.unwrap_or(0)
        )
        .unwrap();
    }
    source.push_str("};\n\nstatic const EcuFrameConfig frames[] = {\n");
    for row in frame_rows {
        writeln!(source, "{row}").unwrap();
    }
    source.push_str("};\n\n");
    let diagnostic_ref = if let Some(diagnostic) = diagnostic {
        source.push_str("static const uint16_t diagnostic_signal_ids[] = { ");
        write!(
            map,
            "DIAGNOSTIC request={} response={} s3={} nas={} nbs={} ncr={} did={} signals=",
            diagnostic.request_id,
            diagnostic.response_id,
            diagnostic.s3_ms,
            diagnostic.n_as_ms,
            diagnostic.n_bs_ms,
            diagnostic.n_cr_ms,
            diagnostic.did
        )
        .unwrap();
        for (index, path) in diagnostic.signal_paths.iter().enumerate() {
            let id = signal_ids
                .get(path.as_str())
                .ok_or_else(|| format!("诊断 DID 信号没有生成 ID: {path}"))?;
            if index != 0 {
                source.push_str(", ");
                map.push(',');
            }
            write!(source, "{id}u").unwrap();
            write!(map, "{id}").unwrap();
        }
        source.push_str(" };\n");
        map.push('\n');
        source.push('\n');
        for (index, path) in diagnostic.signal_paths.iter().enumerate() {
            let id = signal_ids
                .get(path.as_str())
                .ok_or_else(|| format!("诊断 DID 信号没有生成 ID: {path}"))?;
            writeln!(
                externals,
                "Std_ReturnType Ecu_DcmRead_{index}(uint8_t *data);"
            )
            .unwrap();
            writeln!(
                source,
                "Std_ReturnType Ecu_DcmRead_{index}(uint8_t *data) {{"
            )
            .unwrap();
            source.push_str("    uint32_t value;\n    uint8_t valid;\n");
            writeln!(source, "    if (data == NULL || Rte_ReadSignal({id}u, &value, &valid) != ECU_OK || valid == 0u) {{ return E_NOT_OK; }}").unwrap();
            source.push_str("    data[0] = (uint8_t)(value >> 24u);\n");
            source.push_str("    data[1] = (uint8_t)(value >> 16u);\n");
            source.push_str("    data[2] = (uint8_t)(value >> 8u);\n");
            source.push_str("    data[3] = (uint8_t)value;\n    return E_OK;\n}\n");
        }
        source.push_str("static const EcuDidReadFunction diagnostic_readers[] = { ");
        for index in 0..diagnostic.signal_paths.len() {
            if index != 0 {
                source.push_str(", ");
            }
            write!(source, "Ecu_DcmRead_{index}").unwrap();
        }
        source.push_str(" };\n");
        let writer_ref = if diagnostic.write_enabled {
            writeln!(map, "WRITE_DID did={}", diagnostic.did).unwrap();
            source.push('\n');
            for (index, path) in diagnostic.signal_paths.iter().enumerate() {
                let id = signal_ids
                    .get(path.as_str())
                    .ok_or_else(|| format!("诊断 DID 信号没有生成 ID: {path}"))?;
                writeln!(externals, "Std_ReturnType Ecu_DcmWrite_{index}(const uint8_t *data, Dcm_NegativeResponseCodeType *error_code);").unwrap();
                writeln!(source, "Std_ReturnType Ecu_DcmWrite_{index}(const uint8_t *data, Dcm_NegativeResponseCodeType *error_code) {{").unwrap();
                source.push_str("    uint32_t value;\n");
                source.push_str("    if (error_code == NULL) { return E_NOT_OK; }\n");
                source.push_str("    if (data == NULL) { *error_code = DCM_E_GENERALPROGRAMMINGFAILURE; return E_NOT_OK; }\n");
                source.push_str("    value = ((uint32_t)data[0] << 24) | ((uint32_t)data[1] << 16) | ((uint32_t)data[2] << 8) | (uint32_t)data[3];\n");
                writeln!(
                    source,
                    "    if (Rte_WriteSignal({id}u, value) != ECU_OK) {{"
                )
                .unwrap();
                source.push_str("        *error_code = DCM_E_GENERALPROGRAMMINGFAILURE;\n        return E_NOT_OK;\n    }\n    return E_OK;\n}\n");
            }
            source.push_str("static const EcuDidWriteFunction diagnostic_writers[] = { ");
            for index in 0..diagnostic.signal_paths.len() {
                if index != 0 {
                    source.push_str(", ");
                }
                write!(source, "Ecu_DcmWrite_{index}").unwrap();
            }
            source.push_str(" };\n");
            "diagnostic_writers"
        } else {
            "NULL"
        };
        let routine_ref = if let Some(rid) = diagnostic.reset_routine_id {
            writeln!(map, "RESET_ROUTINE id={rid}").unwrap();
            source
                .push_str("\nstatic EcuStatus Ecu_HostRestoreDid(void) {\n    EcuStatus status;\n");
            for path in &diagnostic.signal_paths {
                let id = signal_ids
                    .get(path.as_str())
                    .ok_or_else(|| format!("诊断 DID 信号没有生成 ID: {path}"))?;
                let initial_value = signals
                    .iter()
                    .find(|signal| signal.path == *path)
                    .ok_or_else(|| format!("诊断 DID 信号不存在: {path}"))?
                    .initial_value;
                writeln!(
                    source,
                    "    status = Rte_WriteSignal({id}u, {initial_value}u);"
                )
                .unwrap();
                source.push_str("    if (status != ECU_OK) { return status; }\n");
            }
            source.push_str("    return ECU_OK;\n}\n");
            writeln!(source, "static const EcuResetRoutineConfig diagnostic_reset_routine = {{ {rid}u, Ecu_HostRestoreDid }};").unwrap();
            "&diagnostic_reset_routine"
        } else {
            "NULL"
        };
        let dtc_ref = if let Some(dtc) = &diagnostic.dtc {
            let (index, frame) = frames
                .iter()
                .enumerate()
                .find(|(_, frame)| frame.path == dtc.monitor_frame_path)
                .ok_or_else(|| format!("DTC 监控帧未生成: {}", dtc.monitor_frame_path))?;
            writeln!(
                map,
                "DTC code={} frame={} id={} dlc={} timeout={}",
                dtc.code,
                index,
                frame.id,
                frame.dlc,
                frame.timeout_ms.unwrap_or(0)
            )
            .unwrap();
            writeln!(
                source,
                "static const EcuDtcConfig dtc = {{ {}u, {}u }};",
                dtc.code, index
            )
            .unwrap();
            "&dtc"
        } else {
            "NULL"
        };
        if diagnostic.security_enabled {
            map.push_str("SECURITY level=1 seed=16 key=16 attempts=3 delay=5000\n");
        }
        writeln!(source, "static const EcuDiagnosticConfig diagnostic = {{ {}u, {}u, {}u, {}u, {}u, {}u, {}u, diagnostic_signal_ids, {}u, {dtc_ref}, diagnostic_readers, {writer_ref}, {routine_ref}, {}u, {}u }};\n",
            diagnostic.request_id, diagnostic.response_id, diagnostic.s3_ms, diagnostic.n_as_ms, diagnostic.n_bs_ms, diagnostic.n_cr_ms, diagnostic.did, diagnostic.signal_paths.len(), diagnostic.security_enabled as u8, frames.len()).unwrap();
        "&diagnostic"
    } else {
        "NULL"
    };
    writeln!(source, "const EcuConfig Ecu_Config = {{ \"{name}\", frames, sizeof(frames) / sizeof(frames[0]), signals, sizeof(signals) / sizeof(signals[0]), {diagnostic_ref} }};").unwrap();
    externals.push_str("\n#endif\n");
    Ok((source, map, externals))
}

fn file_digest(path: &Path) -> Result<String, String> {
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

fn check_entries(
    dir: &Path,
    root: &Path,
    names: &[String],
) -> Result<(), String> {
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

pub(crate) fn render_host_profile(
    workspace: &mut Workspace,
    target: BuildTarget,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let (frames, signals) = workspace.checked_profile()?;
    let diagnostic = workspace.diagnostic_profile();
    if target == BuildTarget::LinuxX64ControlledV1
        && diagnostic
            .is_some_and(|item| item.security_enabled)
    {
        return Err("0x27 主机安全档案目前仅支持 Windows 目标".into());
    }
    let (generated, map, externals) =
        config_source(workspace.name(), &frames, &signals, diagnostic)?;
    Ok(vec![
        ("Dcm_Externals.h".into(), externals.into_bytes()),
        ("Ecu_Config.c".into(), generated.into_bytes()),
        ("profile.txt".into(), map.into_bytes()),
    ])
}


fn output_path(output: &Path) -> Result<PathBuf, String> {
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

pub fn preview_generate(
    workspace: &mut Workspace,
    output: &Path,
    target: BuildTarget,
) -> Result<GenerationPreview, String> {
    crate::prepare_host_project(workspace, target, false)?.preview(output)
}

pub fn preview_handoff(
    workspace: &mut Workspace,
    output: &Path,
    target: BuildTarget,
) -> Result<GenerationPreview, String> {
    crate::prepare_host_project(workspace, target, true)?.preview(output)
}

pub fn generate_previewed(
    workspace: &mut Workspace,
    output: &Path,
    revision: &str,
    target: BuildTarget,
) -> Result<GenerationReport, String> {
    crate::prepare_host_project(workspace, target, false)?.generate_previewed(output, revision)
}

pub fn generate_handoff_previewed(
    workspace: &mut Workspace,
    output: &Path,
    revision: &str,
    target: BuildTarget,
) -> Result<GenerationReport, String> {
    crate::prepare_host_project(workspace, target, true)?.generate_previewed(output, revision)
}

pub fn generate_handoff(
    workspace: &mut Workspace,
    output: &Path,
    target: BuildTarget,
) -> Result<GenerationReport, String> {
    generate_prepared(
        crate::prepare_host_project(workspace, target, true)?.into_files(),
        output,
        None,
    )
}

pub fn open_handoff(output: &Path, schema_archive: PathBuf) -> Result<Workspace, String> {
    let names =
        verify_build_input(output).map_err(|error| format!("交付包完整性检查失败: {error}"))?;
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(output.join("handoff.json")).map_err(|e| format!("交付映射缺失: {e}"))?,
    )
    .map_err(|e| format!("交付映射无效: {e}"))?;
    if metadata["format"] != "autosar-host-handoff-v1"
        || metadata["release"] != "CP/FO R24-11"
        || metadata["toolVersion"] != env!("CARGO_PKG_VERSION")
    {
        return Err("交付包格式、规范版次、工具版本或目标不匹配".into());
    }
    let target: BuildTarget = serde_json::from_value(metadata["target"].clone())
        .map_err(|error| format!("交付包目标不受支持: {error}"))?;
    let sources = metadata["sources"]
        .as_array()
        .filter(|items| !items.is_empty())
        .ok_or("交付包没有输入映射")?;
    let mut paths = Vec::with_capacity(sources.len());
    let mut original_names = std::collections::BTreeMap::new();
    for (index, source) in sources.iter().enumerate() {
        let expected = format!("inputs/{index:03}.arxml");
        if source["path"] != expected
            || !source["originalName"].as_str().is_some_and(|name| {
                !name.is_empty()
                    && name != "."
                    && name != ".."
                    && !name.contains('/')
                    && !name.contains('\\')
            })
            || source["packageRoots"].as_array().is_none()
        {
            return Err(format!("交付输入映射无效: {expected}"));
        }
        let path = output.join(&expected);
        if !names.contains(&expected) || source["sha256"] != file_digest(&path)? {
            return Err(format!("交付输入缺失或摘要不匹配: {expected}"));
        }
        original_names.insert(
            path.canonicalize().map_err(|error| error.to_string())?,
            source["originalName"].as_str().unwrap().to_owned(),
        );
        paths.push(path);
    }
    if names
        .iter()
        .filter(|name| name.starts_with("inputs/"))
        .count()
        != paths.len()
    {
        return Err("交付包输入清单与映射不一致".into());
    }
    let mut workspace = Workspace::open(paths, schema_archive)?;
    workspace.restore_handoff_source_names(original_names)?;
    let expected = crate::prepare_host_project(&mut workspace, target, true)?.into_files();
    let expected_names: Vec<_> = expected
        .iter()
        .filter(|(name, _)| name != "files.list" && name != "files.sha256")
        .map(|(name, _)| name.clone())
        .collect();
    if names != expected_names {
        return Err("重建主机交接包的完整文件闭包不匹配".into());
    }
    for (name, bytes) in expected {
        if fs::read(output.join(&name)).map_err(|error| error.to_string())? != bytes {
            return Err(format!("重建主机交接包的来源字节不匹配: {name}"));
        }
    }
    Ok(workspace)
}

pub fn generate(
    workspace: &mut Workspace,
    output: &Path,
    target: BuildTarget,
) -> Result<GenerationReport, String> {
    generate_prepared(
        crate::prepare_host_project(workspace, target, false)?.into_files(),
        output,
        None,
    )
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

pub fn build(
    project: &Path,
    output: &Path,
    settings: &crate::target::ExecutionSettings,
) -> Result<BuildReport, String> {
    verify_build_input(project)?;
    let project = project.canonicalize().map_err(|error| error.to_string())?;
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(project.join("target.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if metadata["format"] != "autosar-build-target-v1" || metadata["profile"] != "host" {
        return Err("The source package is not the selected legacy host profile".into());
    }
    let target: BuildTarget =
        serde_json::from_value(metadata["target"].clone()).map_err(|error| error.to_string())?;
    let output = output_path(output)?;
    let capture = reserve_directory(&std::env::temp_dir(), "host-build", OsStr::new("private"))?;
    let log = crate::integration::handoff::run_tool(
        &project,
        settings,
        vec![
            "build".into(),
            "--project".into(),
            project.as_os_str().into(),
            "--output".into(),
            output.as_os_str().into(),
            "--mode".into(),
            "host".into(),
        ],
        &capture,
    )
    .map_err(|error| format!("{error}; diagnostics retained at {}", capture.display()))?;
    verify_build_input(&project)?;
    let binary = output.join(if target == BuildTarget::WindowsX64ControlledV1 {
        "ecu_host.exe"
    } else {
        "ecu_host"
    });
    if !binary.is_file() {
        return Err("Successful legacy build did not produce its declared native binary".into());
    }
    fs::remove_dir_all(capture).map_err(|error| error.to_string())?;
    Ok(BuildReport {
        binary_path: binary.display().to_string(),
        log,
    })
}
