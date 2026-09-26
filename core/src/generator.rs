use crate::arxml::Workspace;
use crate::model::{BuildReport, DiagnosticView, Direction, GenerationReport, Issue, SignalView};
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
        let entries = fs::read_dir(dir.join(subdir)).map_err(|e| format!("运行代码缺失 {subdir}: {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "c" || ext == "h") {
                files.push((path, format!("{subdir}/{}", entry.file_name().to_string_lossy())));
            }
        }
    }
    files.sort_by(|a,b| a.1.cmp(&b.1));
    Ok(files)
}

fn config_source(name: &str, frames: &[crate::model::FrameView], signals: &[SignalView], diagnostic: Option<&DiagnosticView>) -> Result<(String, String, String), String> {
    let mut source = String::from("#include \"Ecu_Config.h\"\n#include \"Dcm_Externals.h\"\n");
    if diagnostic.is_some() { source.push_str("#include \"Rte.h\"\n"); }
    let mut externals = String::from("#ifndef DCM_EXTERNALS_H\n#define DCM_EXTERNALS_H\n\n#include <stdint.h>\n#include \"Ecu_DcmCallbackTypes.h\"\n\n");
    source.push_str("\nstatic const EcuSignalConfig signals[] = {\n");
    let mut frame_rows = Vec::new();
    let mut map = format!("ECU {name}\n# ID 映射由已验证的 ARXML 路径按字典序稳定生成\n");
    let mut signal_ids = std::collections::BTreeMap::new();
    let mut next_id = 0usize;
    for frame in frames {
        let mut frame_signals: Vec<_> = signals.iter().filter(|s| s.frame_path == frame.path).collect();
        frame_signals.sort_by(|a,b| a.path.cmp(&b.path));
        let first = next_id;
        for signal in frame_signals {
            writeln!(source, "    {{ {}u, {}u, {}u, {}u }},", next_id, signal.start_bit, signal.length, signal.initial_value).unwrap();
            writeln!(map, "SIGNAL {} {} frame={} bits={}:{} initial={}", next_id, signal.path, frame.path, signal.start_bit, signal.length, signal.initial_value).unwrap();
            signal_ids.insert(signal.path.as_str(), next_id);
            next_id += 1;
        }
        frame_rows.push(format!("    {{ {}u, {}u, {}u, {}u, {}u, {}u, {}u }},", frame.id, frame.dlc, matches!(frame.direction, Direction::Tx) as u8, first, next_id - first, frame.period_ms.unwrap_or(0), frame.timeout_ms.unwrap_or(0)));
        writeln!(map, "FRAME {} {} id={} dlc={} direction={} period={} timeout={}", frame_rows.len() - 1, frame.path, frame.id, frame.dlc, if matches!(frame.direction, Direction::Tx) {"tx"} else {"rx"}, frame.period_ms.unwrap_or(0), frame.timeout_ms.unwrap_or(0)).unwrap();
    }
    source.push_str("};\n\nstatic const EcuFrameConfig frames[] = {\n");
    for row in frame_rows { writeln!(source, "{row}").unwrap(); }
    source.push_str("};\n\n");
    let diagnostic_ref = if let Some(diagnostic) = diagnostic {
        source.push_str("static const uint16_t diagnostic_signal_ids[] = { ");
        write!(map, "DIAGNOSTIC request={} response={} s3={} nbs={} ncr={} did={} signals=",
            diagnostic.request_id, diagnostic.response_id, diagnostic.s3_ms, diagnostic.n_bs_ms, diagnostic.n_cr_ms,
            diagnostic.did).unwrap();
        for (index, path) in diagnostic.signal_paths.iter().enumerate() {
            let id = signal_ids.get(path.as_str()).ok_or_else(|| format!("诊断 DID 信号没有生成 ID: {path}"))?;
            if index != 0 { source.push_str(", "); map.push(','); }
            write!(source, "{id}u").unwrap();
            write!(map, "{id}").unwrap();
        }
        source.push_str(" };\n");
        map.push('\n');
        source.push('\n');
        for (index, path) in diagnostic.signal_paths.iter().enumerate() {
            let id = signal_ids.get(path.as_str()).ok_or_else(|| format!("诊断 DID 信号没有生成 ID: {path}"))?;
            writeln!(externals, "Std_ReturnType Ecu_DcmRead_{index}(uint8_t *data);").unwrap();
            writeln!(source, "Std_ReturnType Ecu_DcmRead_{index}(uint8_t *data) {{").unwrap();
            source.push_str("    uint32_t value;\n    uint8_t valid;\n");
            writeln!(source, "    if (data == NULL || Rte_ReadSignal({id}u, &value, &valid) != ECU_OK || valid == 0u) {{ return E_NOT_OK; }}").unwrap();
            source.push_str("    data[0] = (uint8_t)(value >> 24u);\n");
            source.push_str("    data[1] = (uint8_t)(value >> 16u);\n");
            source.push_str("    data[2] = (uint8_t)(value >> 8u);\n");
            source.push_str("    data[3] = (uint8_t)value;\n    return E_OK;\n}\n");
        }
        source.push_str("static const EcuDidReadFunction diagnostic_readers[] = { ");
        for index in 0..diagnostic.signal_paths.len() {
            if index != 0 { source.push_str(", "); }
            write!(source, "Ecu_DcmRead_{index}").unwrap();
        }
        source.push_str(" };\n");
        let writer_ref = if diagnostic.write_enabled {
            writeln!(map, "WRITE_DID did={}", diagnostic.did).unwrap();
            source.push('\n');
            for (index, path) in diagnostic.signal_paths.iter().enumerate() {
                let id = signal_ids.get(path.as_str()).ok_or_else(|| format!("诊断 DID 信号没有生成 ID: {path}"))?;
                writeln!(externals, "Std_ReturnType Ecu_DcmWrite_{index}(const uint8_t *data, Dcm_NegativeResponseCodeType *error_code);").unwrap();
                writeln!(source, "Std_ReturnType Ecu_DcmWrite_{index}(const uint8_t *data, Dcm_NegativeResponseCodeType *error_code) {{").unwrap();
                source.push_str("    uint32_t value;\n");
                source.push_str("    if (error_code == NULL) { return E_NOT_OK; }\n");
                source.push_str("    if (data == NULL) { *error_code = DCM_E_GENERALPROGRAMMINGFAILURE; return E_NOT_OK; }\n");
                source.push_str("    value = ((uint32_t)data[0] << 24) | ((uint32_t)data[1] << 16) | ((uint32_t)data[2] << 8) | (uint32_t)data[3];\n");
                writeln!(source, "    if (Rte_WriteSignal({id}u, value) != ECU_OK) {{").unwrap();
                source.push_str("        *error_code = DCM_E_GENERALPROGRAMMINGFAILURE;\n        return E_NOT_OK;\n    }\n    return E_OK;\n}\n");
            }
            source.push_str("static const EcuDidWriteFunction diagnostic_writers[] = { ");
            for index in 0..diagnostic.signal_paths.len() {
                if index != 0 { source.push_str(", "); }
                write!(source, "Ecu_DcmWrite_{index}").unwrap();
            }
            source.push_str(" };\n");
            "diagnostic_writers"
        } else { "NULL" };
        let routine_ref = if let Some(rid) = diagnostic.reset_routine_id {
            writeln!(map, "RESET_ROUTINE id={rid}").unwrap();
            source.push_str("\nstatic EcuStatus Ecu_HostRestoreDid(void) {\n    EcuStatus status;\n");
            for path in &diagnostic.signal_paths {
                let id = signal_ids.get(path.as_str()).ok_or_else(|| format!("诊断 DID 信号没有生成 ID: {path}"))?;
                let initial_value = signals.iter().find(|signal| signal.path == *path)
                    .ok_or_else(|| format!("诊断 DID 信号不存在: {path}"))?.initial_value;
                writeln!(source, "    status = Rte_WriteSignal({id}u, {initial_value}u);").unwrap();
                source.push_str("    if (status != ECU_OK) { return status; }\n");
            }
            source.push_str("    return ECU_OK;\n}\n");
            writeln!(source, "static const EcuResetRoutineConfig diagnostic_reset_routine = {{ {rid}u, Ecu_HostRestoreDid }};").unwrap();
            "&diagnostic_reset_routine"
        } else { "NULL" };
        let dtc_ref = if let Some(dtc) = &diagnostic.dtc {
            let (index, frame) = frames.iter().enumerate().find(|(_, frame)| frame.path == dtc.monitor_frame_path)
                .ok_or_else(|| format!("DTC 监控帧未生成: {}", dtc.monitor_frame_path))?;
            writeln!(map, "DTC code={} frame={} id={} dlc={} timeout={}", dtc.code, index, frame.id, frame.dlc, frame.timeout_ms.unwrap_or(0)).unwrap();
            writeln!(source, "static const EcuDtcConfig dtc = {{ {}u, {}u }};", dtc.code, index).unwrap();
            "&dtc"
        } else { "NULL" };
        writeln!(source, "static const EcuDiagnosticConfig diagnostic = {{ {}u, {}u, {}u, {}u, {}u, {}u, diagnostic_signal_ids, {}u, {dtc_ref}, diagnostic_readers, {writer_ref}, {routine_ref} }};\n",
            diagnostic.request_id, diagnostic.response_id, diagnostic.s3_ms, diagnostic.n_bs_ms, diagnostic.n_cr_ms, diagnostic.did, diagnostic.signal_paths.len()).unwrap();
        "&diagnostic"
    } else { "NULL" };
    writeln!(source, "const EcuConfig Ecu_Config = {{ \"{name}\", frames, sizeof(frames) / sizeof(frames[0]), signals, sizeof(signals) / sizeof(signals[0]), {diagnostic_ref} }};").unwrap();
    externals.push_str("\n#endif\n");
    Ok((source, map, externals))
}

fn file_digest(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| format!("生成文件缺失或无法读取 {}: {e}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = file.read(&mut buffer).map_err(|e| format!("生成文件无法读取 {}: {e}", path.display()))?;
        if count == 0 { break; }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn integrity_record(dir: &Path, names: &[String]) -> Result<String, String> {
    let mut record = String::new();
    for name in names.iter().map(String::as_str).chain(std::iter::once("files.list")) {
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
    { metadata.file_type().is_symlink() }
}

fn reserve_directory(parent: &Path, role: &str, output_name: &OsStr) -> Result<PathBuf, String> {
    for suffix in 0u64.. {
        let candidate = parent.join(format!(".autosar-config-{role}-{}-{suffix}", std::process::id()));
        if candidate.file_name() == Some(output_name) { continue; }
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
        let name = path.strip_prefix(root).map_err(|e| e.to_string())?.to_str()
            .ok_or("输出目录包含非 UTF-8 文件名")?.replace('\\', "/");
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if is_reparse_point(&metadata) { return Err(format!("输出目录含链接或重解析点，拒绝替换: {name}")); }
        let kind = metadata.file_type();
        if kind.is_dir() {
            if name != "include" && name != "src" { return Err(format!("输出目录含用户目录，拒绝替换: {name}")); }
            check_entries(&path, root, names)?;
        } else if name == "ecu_host.exe" || name == "ecu_host" {
            return Err(format!("输出目录含已构建的二进制文件 {name}，拒绝替换并保留原目录；请选择新的空输出目录，或由文件所有者明确移走旧二进制后重试"));
        } else if !kind.is_file() ||
            (name != "files.list" && name != "files.sha256" && names.binary_search(&name).is_err()) {
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
        return Err(format!("输出目录不是普通目录或是重解析点，拒绝覆盖: {}", dir.display()));
    }
    if fs::read_dir(dir).map_err(|e| e.to_string())?.next().is_none() { return Ok(()); }
    let previous = fs::read_to_string(dir.join("files.list"))
        .map_err(|_| format!("输出目录缺少受支持的文件清单，拒绝覆盖: {}", dir.display()))?;
    if previous != format!("{}\n", names.join("\n")) {
        return Err("已有输出清单不匹配，拒绝删除或覆盖其他生成版本".into());
    }
    check_entries(dir, dir, names)?;
    let recorded = fs::read_to_string(dir.join("files.sha256"))
        .map_err(|_| format!("输出目录缺少完整性记录，拒绝覆盖旧版生成目录: {}", dir.display()))?;
    if recorded != integrity_record(dir, names)? {
        return Err("生成文件或完整性记录已被修改，拒绝覆盖用户内容".into());
    }
    Ok(())
}

pub fn generate(workspace: &mut Workspace, output: &Path) -> Result<GenerationReport, String> {
    let (frames, signals) = workspace.checked_profile()?;
    let sources = source_files(&runtime_dir())?;
    let (generated, map, externals) = config_source(workspace.name(), &frames, &signals, workspace.view().diagnostic.as_ref())?;
    let mut names: Vec<String> = sources.iter().map(|(_,name)| name.clone()).collect();
    names.extend(["Dcm_Externals.h".into(), "Ecu_Config.c".into(), "profile.txt".into()]);
    names.sort();
    let output_name = output.file_name().ok_or("输出目录须有名称")?;
    let parent = output.parent().ok_or("输出目录须有父目录")?;
    if output_name == "." || output_name == ".." || parent.join(output_name) != output {
        return Err("输出目录须为明确的命名路径，不能以 . 或 .. 结尾".into());
    }
    let parent = if parent.is_absolute() { parent.to_path_buf() }
        else { std::env::current_dir().map_err(|e| e.to_string())?.join(parent) };
    fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
    let output = parent.join(output_name);
    verify_generated_output(&output, &names)?;
    let stage = reserve_directory(&parent, "stage", output_name)?;
    let result = (|| {
        for (source, name) in sources {
            let target = stage.join(&name);
            fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::copy(source, target).map_err(|e| e.to_string())?;
        }
        fs::write(stage.join("Ecu_Config.c"), generated).map_err(|e| e.to_string())?;
        fs::write(stage.join("Dcm_Externals.h"), externals).map_err(|e| e.to_string())?;
        fs::write(stage.join("profile.txt"), map).map_err(|e| e.to_string())?;
        fs::write(stage.join("files.list"), names.join("\n") + "\n").map_err(|e| e.to_string())?;
        fs::write(stage.join("files.sha256"), integrity_record(&stage, &names)?).map_err(|e| e.to_string())?;
        verify_generated_output(&output, &names)?;
        let existing = match fs::symlink_metadata(&output) {
            Ok(metadata) if is_reparse_point(&metadata) || !metadata.file_type().is_dir() =>
                return Err(format!("输出目录已变为链接或非普通目录，拒绝替换: {}", output.display())),
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.to_string()),
        };
        let backup = if existing {
            let backup_root = reserve_directory(&parent, "backup", output_name)?;
            let preserved = backup_root.join(output_name);
            fs::rename(&output, &preserved).map_err(|e|
                format!("无法保留旧生成工程 {} 至 {}: {e}", output.display(), preserved.display()))?;
            Some(preserved)
        } else { None };
        if let Err(error) = fs::rename(&stage, &output) {
            let recovery = backup.as_ref().map(|path|
                format!("；原输出保留在 {}", path.display())).unwrap_or_default();
            return Err(format!("无法安装新生成工程: {error}{recovery}"));
        }
        Ok(backup)
    })();
    let backup = result.map_err(|error| format!("{error}；临时生成目录保留在 {}", stage.display()))?;
    Ok(GenerationReport { output_directory: output.display().to_string(),
        previous_output_directory: backup.map(|path| path.display().to_string()), files: names, issues: Vec::<Issue>::new() })
}

pub fn build(output: &Path) -> Result<BuildReport, String> {
    if !output.join("files.list").is_file() || !output.join("Ecu_Config.c").is_file() { return Err("须先生成完整 C99 工程".into()); }
    let metadata = fs::symlink_metadata(output).map_err(|e| e.to_string())?;
    if is_reparse_point(&metadata) || !metadata.file_type().is_dir() {
        return Err(format!("构建目录不是普通目录或是重解析点: {}", output.display()));
    }
    let binary = output.join(if cfg!(windows) { "ecu_host.exe" } else { "ecu_host" });
    let occupied = || format!("已有构建二进制 {}，拒绝覆盖；请选新的空目录重新生成并构建，或由文件所有者明确移走旧二进制后重试", binary.display());
    match fs::symlink_metadata(&binary) {
        Ok(_) => return Err(occupied()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    let cc = std::env::var_os("AUTOSAR_CC").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("gcc"));
    let mut sources = fs::read_dir(output.join("src")).map_err(|e| e.to_string())?.map(|entry| entry.map(|e| e.path()).map_err(|e| e.to_string())).collect::<Result<Vec<_>,_>>()?;
    sources.retain(|p| p.extension().is_some_and(|e| e == "c"));
    sources.sort();
    sources.push(output.join("Ecu_Config.c"));
    let stage = reserve_directory(output.parent().ok_or("构建目录须有父目录")?, "build", binary.file_name().unwrap())?;
    let staged_binary = stage.join(binary.file_name().unwrap());
    let result = Command::new(cc).arg("-std=c99").arg("-Wall").arg("-Wextra").arg("-Werror").arg("-pedantic").arg("-I")
        .arg(output.join("include")).args(sources).arg("-o").arg(&staged_binary).output()
        .map_err(|e| format!("无法启动 C99 编译器: {e}；临时目录保留在 {}", stage.display()))?;
    let log = format!("{}{}", String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
    if !result.status.success() { return Err(format!("C99 构建失败: {log}；临时目录保留在 {}", stage.display())); }
    fs::hard_link(&staged_binary, &binary).map_err(|error| {
        let reason = if error.kind() == std::io::ErrorKind::AlreadyExists { occupied() }
            else { format!("无法安装已编译的二进制: {error}") };
        format!("{reason}；临时编译产物保留在 {}", staged_binary.display())
    })?;
    let cleanup = fs::remove_file(&staged_binary).and_then(|_| fs::remove_dir(&stage));
    let log = match cleanup {
        Ok(()) if log.is_empty() => "C99 构建成功".into(),
        Ok(()) => log,
        Err(error) => format!("{log}C99 构建成功，但临时目录 {} 未完全清理: {error}", stage.display()),
    };
    Ok(BuildReport { binary_path: binary.display().to_string(), log })
}
