use crate::arxml::Workspace;
use crate::model::{DiagnosticView, Direction, SignalView};
use crate::target::BuildTarget;
use std::fmt::Write;

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
    source.push_str(
        r#"
static const uint8_t allowed_sids[] = {
    0x10u, 0x3eu, 0x27u, 0x22u, 0x2eu, 0x31u, 0x85u, 0x19u, 0x14u,
};
const EcuPolicyConfig Ecu_Policy = {
    .tx_confirmation = ECU_TX_SYNCHRONOUS,
    .rx_time_order = ECU_TIME_BEFORE_RX,
    .tx_padding_dlc = 0u,
    .wft_max = 0u,
    .wait_when_wft_max_zero = ECU_WAIT_RESTART,
    .max_read_dids = 0u,
    .read_did_sessions = 0x08u,
    .allowed_sids = allowed_sids,
    .allowed_sid_count = sizeof(allowed_sids) / sizeof(allowed_sids[0]),
    .p2_ms = 50u,
    .p2_star_ms = 500u,
};
"#,
    );
    for (label, table, receive) in [
        ("Receive", "receive", true),
        ("Transmit", "transmit", false),
    ] {
        let count = frames
            .iter()
            .filter(|frame| matches!(frame.direction, Direction::Rx) == receive)
            .count()
            + usize::from(diagnostic.is_some());
        if count != 0 {
            writeln!(source, "static const Ecu{label}Route {table}_routes[] = {{").unwrap();
            for (index, frame) in frames
                .iter()
                .enumerate()
                .filter(|(_, frame)| matches!(frame.direction, Direction::Rx) == receive)
            {
                if receive {
                    writeln!(
                        source,
                        "    {{ {}u, {}u, {index}u, {index}u, ECU_ROUTE_COM }},",
                        frame.id, frame.dlc
                    )
                    .unwrap();
                } else {
                    writeln!(source, "    {{ {index}u, {index}u, ECU_ROUTE_COM }},").unwrap();
                }
            }
            if let Some(connection) = diagnostic {
                let handle = frames.len();
                if receive {
                    writeln!(
                        source,
                        "    {{ {}u, 0u, {handle}u, {handle}u, ECU_ROUTE_CANTP }},",
                        connection.request_id
                    )
                    .unwrap();
                } else {
                    writeln!(source, "    {{ {handle}u, {handle}u, ECU_ROUTE_CANTP }},").unwrap();
                }
            }
            source.push_str("};\n");
        }
        if count == 0 {
            writeln!(
                source,
                "const Ecu{label}Route *const Ecu_{label}Routes = NULL;"
            )
            .unwrap();
        } else {
            writeln!(
                source,
                "const Ecu{label}Route *const Ecu_{label}Routes = {table}_routes;"
            )
            .unwrap();
        }
        writeln!(source, "const size_t Ecu_{label}RouteCount = {count}u;").unwrap();
    }
    externals.push_str("\n#endif\n");
    Ok((source, map, externals))
}
pub(crate) fn render_host_profile(
    workspace: &mut Workspace,
    target: BuildTarget,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let (frames, signals) = workspace.checked_profile()?;
    let diagnostic = workspace.diagnostic_profile();
    if target == BuildTarget::LinuxX64ControlledV1
        && diagnostic.is_some_and(|item| item.security_enabled)
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
