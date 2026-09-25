use crate::arxml::Workspace;
use crate::model::{BuildReport, DiagnosticView, Direction, GenerationReport, Issue, SignalView};
use std::fmt::Write;
use std::fs;
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

fn expected_paths(dir: &Path, names: &[String]) -> Result<(), String> {
    if !dir.exists() { return Ok(()); }
    if dir.is_dir() && fs::read_dir(dir).map_err(|e| e.to_string())?.next().is_none() { return Ok(()); }
    let manifest = dir.join("files.list");
    let previous = fs::read_to_string(&manifest).map_err(|_| format!("输出目录非本工具生成，拒绝覆盖: {}", dir.display()))?;
    let listed: std::collections::BTreeSet<_> = previous.lines().collect();
    let requested: std::collections::BTreeSet<_> = names.iter().map(String::as_str).collect();
    if listed != requested { return Err("已有输出清单不匹配，拒绝删除或覆盖其他生成版本".into()); }
    for entry in walk_files(dir)? {
        let name = entry.strip_prefix(dir).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
        if name != "files.list" && name != "ecu_host.exe" && name != "ecu_host" && !listed.contains(name.as_str()) {
            return Err(format!("输出目录含用户文件，拒绝替换: {name}"));
        }
    }
    Ok(())
}

fn walk_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() { files.extend(walk_files(&entry.path())?); } else { files.push(entry.path()); }
    }
    Ok(files)
}

pub fn generate(workspace: &mut Workspace, output: &Path) -> Result<GenerationReport, String> {
    let (frames, signals) = workspace.checked_profile()?;
    let sources = source_files(&runtime_dir())?;
    let (generated, map, externals) = config_source(workspace.name(), &frames, &signals, workspace.view().diagnostic.as_ref())?;
    let mut names: Vec<String> = sources.iter().map(|(_,name)| name.clone()).collect();
    names.extend(["Dcm_Externals.h".into(), "Ecu_Config.c".into(), "profile.txt".into()]);
    names.sort();
    expected_paths(output, &names)?;
    let parent = output.parent().ok_or("输出目录须有父目录")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let stage = parent.join(format!(".autosar-config-{}-{}", std::process::id(), workspace.name()));
    fs::create_dir(&stage).map_err(|e| format!("临时生成目录不可创建: {e}"))?;
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
        let backup = parent.join(format!(".autosar-config-backup-{}-{}", std::process::id(), workspace.name()));
        if output.exists() { fs::rename(output, &backup).map_err(|e| e.to_string())?; }
        if let Err(error) = fs::rename(&stage, output) {
            if backup.exists() { let _ = fs::rename(&backup, output); }
            return Err(error.to_string());
        }
        if backup.exists() { fs::remove_dir_all(backup).map_err(|e| e.to_string())?; }
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_dir_all(stage); }
    result?;
    Ok(GenerationReport { output_directory: output.display().to_string(), files: names, issues: Vec::<Issue>::new() })
}

pub fn build(output: &Path) -> Result<BuildReport, String> {
    if !output.join("files.list").is_file() || !output.join("Ecu_Config.c").is_file() { return Err("须先生成完整 C99 工程".into()); }
    let cc = std::env::var_os("AUTOSAR_CC").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("gcc"));
    let mut sources = fs::read_dir(output.join("src")).map_err(|e| e.to_string())?.map(|entry| entry.map(|e| e.path()).map_err(|e| e.to_string())).collect::<Result<Vec<_>,_>>()?;
    sources.retain(|p| p.extension().is_some_and(|e| e == "c"));
    sources.sort();
    sources.push(output.join("Ecu_Config.c"));
    let binary = output.join(if cfg!(windows) { "ecu_host.exe" } else { "ecu_host" });
    let result = Command::new(cc).arg("-std=c99").arg("-Wall").arg("-Wextra").arg("-Werror").arg("-pedantic").arg("-I").arg(output.join("include")).args(sources).arg("-o").arg(&binary).output().map_err(|e| format!("无法启动 C99 编译器: {e}"))?;
    let log = format!("{}{}", String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
    if !result.status.success() { return Err(format!("C99 构建失败: {log}")); }
    Ok(BuildReport { binary_path: binary.display().to_string(), log: if log.is_empty() { "C99 构建成功".into() } else { log } })
}
