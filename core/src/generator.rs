use crate::arxml::Workspace;
use crate::model::{
    BuildReport, DiagnosticView, Direction, GenerationPreview, GenerationPreviewFile,
    GenerationReport, Issue, SignalView,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::fmt::Write;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

fn runtime_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../runtime")
}

fn source_files(dir: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let mut files = Vec::new();
    for subdir in ["include", "src"] {
        let entries =
            fs::read_dir(dir.join(subdir)).map_err(|e| format!("运行代码缺失 {subdir}: {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "c" || ext == "h") {
                files.push((
                    path,
                    format!("{subdir}/{}", entry.file_name().to_string_lossy()),
                ));
            }
        }
    }
    files.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(files)
}

fn handoff_readme(diagnostic: Option<&DiagnosticView>) -> String {
    let mut run = String::from(".\\ecu_host.exe");
    let mut notes = String::new();
    if let Some(diagnostic) = diagnostic {
        if diagnostic.dtc.is_some() {
            run.push_str(" --nvm .\\ecu.nvm");
            notes.push_str("The `--nvm` path is an exclusive host DTC state file. A missing file is initialized; a damaged or mismatched existing file stops startup.\n\n");
        }
        if diagnostic.security_enabled {
            run.push_str(" --security-key .\\ecu.key --security-state .\\ecu.security");
            notes.push_str("Create `ecu.key` locally as exactly 32 raw secret bytes before starting. It is not generated or listed in the manifest; do not include it when handing off the source project. Give each ECU its own security state file. A missing key or damaged state stops startup.\n\n");
        }
    }
    include_str!("../../runtime/generated-README.md")
        .replace("{{RUN_COMMAND}}", &run)
        .replace("{{RUN_NOTES}}", notes.trim_end())
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

fn reserve_directory(parent: &Path, role: &str, output_name: &OsStr) -> Result<PathBuf, String> {
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
    allow_host_binary: bool,
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
            if name != "include"
                && name != "src"
                && !(name == "inputs" && names.iter().any(|item| item.starts_with("inputs/")))
            {
                return Err(format!("输出目录含用户目录，拒绝替换: {name}"));
            }
            check_entries(&path, root, names, allow_host_binary)?;
        } else if name == "ecu_host.exe" || name == "ecu_host" {
            if !allow_host_binary || dir != root || !kind.is_file() {
                return Err(format!(
                    "输出目录含已构建的二进制文件 {name}，拒绝替换并保留原目录；请选择新的空输出目录，或由文件所有者明确移走旧二进制后重试"
                ));
            }
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

fn verify_generated_output_with_binary(
    dir: &Path,
    names: &[String],
    allow_host_binary: bool,
) -> Result<(), String> {
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
    check_entries(dir, dir, names, allow_host_binary)?;
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

fn verify_generated_output(dir: &Path, names: &[String]) -> Result<(), String> {
    verify_generated_output_with_binary(dir, names, false)
}

fn verify_build_input(output: &Path) -> Result<(), String> {
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
        .map_err(|e| format!("生成工程完整性检查失败，拒绝构建: {e}"))
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

fn prepared_files(workspace: &mut Workspace) -> Result<Vec<(String, Vec<u8>)>, String> {
    let (frames, signals) = workspace.checked_profile()?;
    let diagnostic = workspace.view().diagnostic;
    if !cfg!(windows)
        && diagnostic
            .as_ref()
            .is_some_and(|item| item.security_enabled)
    {
        return Err("0x27 主机安全档案目前仅支持 Windows 目标".into());
    }
    let (generated, map, externals) =
        config_source(workspace.name(), &frames, &signals, diagnostic.as_ref())?;
    let mut files = Vec::new();
    for (source, name) in source_files(&runtime_dir())? {
        files.push((
            name,
            fs::read(&source).map_err(|e| format!("无法读取运行代码 {}: {e}", source.display()))?,
        ));
    }
    files.extend([
        (
            "README.md".into(),
            handoff_readme(diagnostic.as_ref()).into_bytes(),
        ),
        (
            "build.ps1".into(),
            include_bytes!("../../runtime/generated-build.ps1").to_vec(),
        ),
        ("Dcm_Externals.h".into(), externals.into_bytes()),
        ("Ecu_Config.c".into(), generated.into_bytes()),
        ("profile.txt".into(), map.into_bytes()),
    ]);
    Ok(seal_files(files))
}

fn prepared_handoff_files(workspace: &mut Workspace) -> Result<Vec<(String, Vec<u8>)>, String> {
    let sources = workspace.handoff_sources()?;
    let mut files = prepared_files(workspace)?;
    files.truncate(files.len() - 2);
    let mut mappings = Vec::with_capacity(sources.len());
    for (index, source) in sources.into_iter().enumerate() {
        let path = format!("inputs/{index:03}.arxml");
        mappings.push(json!({
            "path": path,
            "originalName": source.original_name,
            "packageRoots": source.package_roots,
            "sha256": format!("{:x}", Sha256::digest(&source.contents)),
        }));
        files.push((path, source.contents));
    }
    let metadata = json!({
        "format": "autosar-host-handoff-v1",
        "release": "CP/FO R24-11",
        "toolVersion": env!("CARGO_PKG_VERSION"),
        "target": "Windows host virtual ECU; MinGW GCC",
        "sources": mappings,
    });
    let mut metadata = serde_json::to_vec_pretty(&metadata).map_err(|e| e.to_string())?;
    metadata.push(b'\n');
    files.push(("handoff.json".into(), metadata));
    let readme = files
        .iter_mut()
        .find(|(name, _)| name == "README.md")
        .ok_or("交付说明缺失")?;
    let original = String::from_utf8(readme.1.clone()).map_err(|e| e.to_string())?;
    let old_note = "The ARXML sources are not included in this directory. Retain them separately if you need to edit or regenerate this project.";
    if !original.contains(old_note) {
        return Err("生成工程说明的来源输入声明已变化，拒绝误导性交付".into());
    }
    let replacement = format!(
        "Saved ARXML inputs are included under `inputs/` and mapped in `handoff.json`. Review their original contents before sharing. The XSD, workbench, and verification result are not included here. This package records tool version {} and targets only the fixed Windows host profile.\n\n## Reimport and reproduce\n\n1. Move this complete directory to the receiving machine. Check `files.list` and `files.sha256` before relying on its contents; SHA-256 detects accidental modification, not publisher authenticity.\n2. Prepare the same-version workbench from its source checkout and a separately, legally obtained R24-11 XSD archive as described in the workbench README. In the workbench choose “导入可重建主机交付包” and select this directory. That command checks the manifest, mapping, version, source closure and supported host profile before opening the inputs.\n3. Validate the imported configuration and generate to a new empty output directory. Compare `Ecu_Config.c`, `Dcm_Externals.h`, `profile.txt`, `build.ps1`, `include/` and `src/` with this delivery; `README.md` and manifests differ because the new directory is a plain generated project.\n4. Run its `build.ps1` with PowerShell and MinGW GCC. Generating or building alone does not verify host behavior; use the separate fixed reference bundle and its offline `verify.ps1` for that result. Do not interpret either result as MCU or full AUTOSAR conformance evidence.\n",
        env!("CARGO_PKG_VERSION")
    );
    readme.1 = original.replace(old_note, &replacement).into_bytes();
    Ok(seal_files(files))
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
) -> Result<GenerationPreview, String> {
    let files = prepared_files(workspace)?;
    preview_prepared(&files, output)
}

pub fn preview_handoff(
    workspace: &mut Workspace,
    output: &Path,
) -> Result<GenerationPreview, String> {
    let files = prepared_handoff_files(workspace)?;
    preview_prepared(&files, output)
}

pub fn generate_previewed(
    workspace: &mut Workspace,
    output: &Path,
    revision: &str,
) -> Result<GenerationReport, String> {
    let files = prepared_files(workspace)?;
    if preview_prepared(&files, output)?.revision != revision {
        return Err("生成预览已失效：配置、运行源码或旧输出已变化；请重新预览".into());
    }
    generate_prepared(files, output, Some(revision))
}

pub fn generate_handoff_previewed(
    workspace: &mut Workspace,
    output: &Path,
    revision: &str,
) -> Result<GenerationReport, String> {
    let files = prepared_handoff_files(workspace)?;
    if preview_prepared(&files, output)?.revision != revision {
        return Err("交付包预览已失效：来源、运行源码或旧输出已变化；请重新预览".into());
    }
    generate_prepared(files, output, Some(revision))
}

pub fn generate_handoff(
    workspace: &mut Workspace,
    output: &Path,
) -> Result<GenerationReport, String> {
    generate_prepared(prepared_handoff_files(workspace)?, output, None)
}

pub fn open_handoff(output: &Path, schema_archive: PathBuf) -> Result<Workspace, String> {
    let list = fs::read_to_string(output.join("files.list"))
        .map_err(|e| format!("交付包缺少文件清单: {e}"))?;
    let names: Vec<String> = list.lines().map(str::to_owned).collect();
    if names.is_empty()
        || list != format!("{}\n", names.join("\n"))
        || names.windows(2).any(|pair| pair[0] >= pair[1])
        || names.iter().any(|name| {
            name.contains('\\')
                || Path::new(name)
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_)))
        })
    {
        return Err("交付包清单格式或路径无效".into());
    }
    verify_generated_output_with_binary(output, &names, true)
        .map_err(|e| format!("交付包完整性检查失败: {e}"))?;
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(output.join("handoff.json")).map_err(|e| format!("交付映射缺失: {e}"))?,
    )
    .map_err(|e| format!("交付映射无效: {e}"))?;
    if metadata["format"] != "autosar-host-handoff-v1"
        || metadata["release"] != "CP/FO R24-11"
        || metadata["toolVersion"] != env!("CARGO_PKG_VERSION")
        || metadata["target"] != "Windows host virtual ECU; MinGW GCC"
    {
        return Err("交付包格式、规范版次、工具版本或目标不匹配".into());
    }
    let sources = metadata["sources"]
        .as_array()
        .filter(|items| !items.is_empty())
        .ok_or("交付包没有输入映射")?;
    let mut paths = Vec::with_capacity(sources.len());
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
    workspace.checked_profile()?;
    let actual_sources = workspace.handoff_sources()?;
    for (index, (declared, actual)) in sources.iter().zip(actual_sources).enumerate() {
        if declared["packageRoots"] != json!(actual.package_roots) {
            return Err(format!("交付输入 {index} 的逻辑包根与 ARXML 不一致"));
        }
    }
    Ok(workspace)
}

pub fn generate(workspace: &mut Workspace, output: &Path) -> Result<GenerationReport, String> {
    generate_prepared(prepared_files(workspace)?, output, None)
}

pub(crate) fn generate_prepared(
    files: Vec<(String, Vec<u8>)>,
    output: &Path,
    expected_revision: Option<&str>,
) -> Result<GenerationReport, String> {
    let output = output_path(output)?;
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

pub fn build(output: &Path) -> Result<BuildReport, String> {
    if !output.join("files.list").is_file() || !output.join("Ecu_Config.c").is_file() {
        return Err("须先生成完整 C99 工程".into());
    }
    let metadata = fs::symlink_metadata(output).map_err(|e| e.to_string())?;
    if is_reparse_point(&metadata) || !metadata.file_type().is_dir() {
        return Err(format!(
            "构建目录不是普通目录或是重解析点: {}",
            output.display()
        ));
    }
    let binary = output.join(if cfg!(windows) {
        "ecu_host.exe"
    } else {
        "ecu_host"
    });
    let occupied = || {
        format!(
            "已有构建二进制 {}，拒绝覆盖；请选新的空目录重新生成并构建，或由文件所有者明确移走旧二进制后重试",
            binary.display()
        )
    };
    match fs::symlink_metadata(&binary) {
        Ok(_) => return Err(occupied()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    verify_build_input(output)?;
    let cc = std::env::var_os("AUTOSAR_CC")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("gcc"));
    let mut sources = fs::read_dir(output.join("src"))
        .map_err(|e| e.to_string())?
        .map(|entry| entry.map(|e| e.path()).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    sources.retain(|p| p.extension().is_some_and(|e| e == "c"));
    sources.sort();
    sources.push(output.join("Ecu_Config.c"));
    let stage = reserve_directory(
        output.parent().ok_or("构建目录须有父目录")?,
        "build",
        binary.file_name().unwrap(),
    )?;
    let staged_binary = stage.join(binary.file_name().unwrap());
    let mut command = Command::new(cc);
    command
        .arg("-std=c99")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-pedantic")
        .arg("-I")
        .arg(output.join("include"))
        .args(sources)
        .arg("-o")
        .arg(&staged_binary);
    if cfg!(windows) {
        command.arg("-lbcrypt");
    }
    let result = command.output().map_err(|e| {
        format!(
            "无法启动 C99 编译器: {e}；临时目录保留在 {}",
            stage.display()
        )
    })?;
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    if !result.status.success() {
        return Err(format!(
            "C99 构建失败: {log}；临时目录保留在 {}",
            stage.display()
        ));
    }
    verify_build_input(output).map_err(|e| {
        format!(
            "编译期间{e}；临时编译产物保留在 {}",
            staged_binary.display()
        )
    })?;
    fs::hard_link(&staged_binary, &binary).map_err(|error| {
        let reason = if error.kind() == std::io::ErrorKind::AlreadyExists {
            occupied()
        } else {
            format!("无法安装已编译的二进制: {error}")
        };
        format!("{reason}；临时编译产物保留在 {}", staged_binary.display())
    })?;
    let cleanup = fs::remove_file(&staged_binary).and_then(|_| fs::remove_dir(&stage));
    let log = match cleanup {
        Ok(()) if log.is_empty() => "C99 构建成功".into(),
        Ok(()) => log,
        Err(error) => format!(
            "{log}C99 构建成功，但临时目录 {} 未完全清理: {error}",
            stage.display()
        ),
    };
    Ok(BuildReport {
        binary_path: binary.display().to_string(),
        log,
    })
}
