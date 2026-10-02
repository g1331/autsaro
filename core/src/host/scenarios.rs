use super::process::{EcuProcess, SecurityFiles, TempNvm};
use super::profile::{DiagnosticProfile, DtcProfile, Profile, profile};
use super::protocol::{
    diagnostic_error, diagnostic_frames, diagnostic_request, expected_payload, hex_payload,
    parse_frame, parse_value, prepare, security_seed, security_unlock, send_payload, signal_value,
    tick, value,
};
use crate::{execution::ProcessOwner, model::RunReport};
use std::fs;
use std::path::Path;

fn route_and_check(
    a_ecu: &mut EcuProcess,
    b_ecu: &mut EcuProcess,
    a: &Profile,
    b: &Profile,
    a_out: &[String],
    b_out: &[String],
    events: &mut Vec<String>,
) -> Result<u32, String> {
    let mut bus = Vec::new();
    for (owner, lines) in [(0, a_out), (1, b_out)] {
        for line in lines {
            let (id, payload) = parse_frame(line)?;
            bus.push((id, owner, payload));
        }
    }
    bus.sort_by_key(|(id, _, _)| *id);
    if bus.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err("同一时隙两个 ECU 发送相同 CAN ID，仲裁无法区分冲突载荷".into());
    }
    let mut matched = [0u32; 2];
    let mut observed_rx_id = None;
    for (id, owner, payload) in &bus {
        let (tx, rx, receiver, salt) = if *owner == 0 {
            (a, b, &mut *b_ecu, 0x1357_9BDF)
        } else {
            (b, a, &mut *a_ecu, 0x2468_ACE0)
        };
        let tx_frame = tx
            .frames
            .iter()
            .find(|f| f.tx && f.id == *id && f.dlc as usize == payload.len())
            .ok_or("ECU 发送了未配置的 CAN 帧")?;
        let expected = expected_payload(tx_frame, tx, salt);
        if *payload != expected {
            return Err(format!(
                "CAN 位向量不匹配：id={id} 预期={expected:02X?} 实际={payload:02X?}"
            ));
        }
        events.push(format!(
            "总线 ID 优先级 id={id} payload={payload:02X?} 与独立位向量一致"
        ));
        if let Some(rx_frame) = rx.frames.iter().find(|f| !f.tx && f.id == *id) {
            let hex = payload
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<String>();
            receiver.query(
                &[format!("R {id} {} {hex}", payload.len())],
                rx.signals[0].id,
            )?;
            for signal in rx.signals.iter().filter(|s| s.frame == rx_frame.path) {
                let (_, response) = receiver.query(&[], signal.id)?;
                let (actual, valid) = parse_value(&response)?;
                let expected = signal_value(payload, signal);
                if !valid || actual != expected {
                    return Err(format!(
                        "接收信号 {} 不匹配：预期 {expected} valid=1，实际 {actual} valid={valid}",
                        signal.id
                    ));
                }
                events.push(format!(
                    "接收 signal={} value={} valid=1",
                    signal.id, actual
                ));
            }
            matched[*owner] += 1;
            if *owner == 0 {
                observed_rx_id = Some(*id);
            }
        }
    }
    if matched.contains(&0) {
        return Err("两个 ECU 必须各自发送至少一个由对端接收的信号帧".into());
    }
    Ok(observed_rx_id.unwrap())
}

pub(super) fn run(
    first: &Path,
    binary_a: &Path,
    second: &Path,
    binary_b: &Path,
    owner: &ProcessOwner,
) -> Result<RunReport, String> {
    crate::generator::output::verify_build_input(first)?;
    crate::generator::output::verify_build_input(second)?;
    let a = profile(first)?;
    let b = profile(second)?;
    if a.text == b.text {
        return Err("两个虚拟 ECU 配置相同；请分别生成不同项目".into());
    }
    if !binary_a.is_file() || !binary_b.is_file() {
        return Err("两个生成目录均须先完成 C99 构建".into());
    }
    let a_nvm = a.dtc.as_ref().map(|_| TempNvm::new());
    let b_nvm = b.dtc.as_ref().map(|_| TempNvm::new());
    let a_security = if a.diagnostic.as_ref().is_some_and(|d| d.security_enabled) {
        Some(SecurityFiles::new()?)
    } else {
        None
    };
    let b_security = if b.diagnostic.as_ref().is_some_and(|d| d.security_enabled) {
        Some(SecurityFiles::new()?)
    } else {
        None
    };
    let mut ecu_a = EcuProcess::start(
        binary_a,
        a_nvm.as_ref().map(|state| state.path.as_path()),
        a_security.as_ref(),
        owner,
    )?;
    let mut ecu_b = EcuProcess::start(
        binary_b,
        b_nvm.as_ref().map(|state| state.path.as_path()),
        b_security.as_ref(),
        owner,
    )?;
    let mut events = Vec::new();
    let outcome = (|| -> Result<(), String> {
        let max_period = a
            .frames
            .iter()
            .chain(&b.frames)
            .filter(|f| f.tx)
            .map(|f| f.period)
            .max()
            .ok_or("没有周期发送帧")? as u64;
        prepare(&mut ecu_a, &a, 0x1357_9BDF)?;
        prepare(&mut ecu_b, &b, 0x2468_ACE0)?;
        tick(&mut ecu_a, &a, 0)?;
        tick(&mut ecu_b, &b, 0)?;
        let a_out = tick(&mut ecu_a, &a, max_period)?;
        let b_out = tick(&mut ecu_b, &b, max_period)?;
        let b_rx_id = route_and_check(&mut ecu_a, &mut ecu_b, &a, &b, &a_out, &b_out, &mut events)?;

        let max_timeout = a
            .frames
            .iter()
            .chain(&b.frames)
            .filter(|f| !f.tx)
            .map(|f| f.timeout)
            .max()
            .ok_or("没有接收超时配置")? as u64;
        let timeout_tick = max_period + max_timeout + 1;
        let dropped_a = tick(&mut ecu_a, &a, timeout_tick)?;
        let dropped_b = tick(&mut ecu_b, &b, timeout_tick)?;
        events.push(format!(
            "故障注入：丢弃 {} + {} 个发送帧",
            dropped_a.len(),
            dropped_b.len()
        ));
        for (ecu, profile) in [(&mut ecu_a, &a), (&mut ecu_b, &b)] {
            for signal in profile
                .signals
                .iter()
                .filter(|s| profile.frames.iter().any(|f| !f.tx && f.path == s.frame))
            {
                let (_, response) = ecu.query(&[], signal.id)?;
                if parse_value(&response)?.1 {
                    return Err(format!("接收信号 {} 在超时后仍有效", signal.id));
                }
            }
        }
        events.push(format!("虚拟时钟推进至 {timeout_tick}ms，接收信号全部失效"));

        ecu_a.query(&["M 2".into()], a.signals[0].id)?;
        let off_out = tick(&mut ecu_a, &a, timeout_tick + max_period)?;
        if !off_out.is_empty() {
            return Err("BUS_OFF 状态仍然发出了 CAN 帧".into());
        }
        events.push("BUS_OFF 状态禁止发送".into());
        ecu_a.query(&["M 1".into()], a.signals[0].id)?;
        let on_out = tick(&mut ecu_a, &a, timeout_tick + 2 * max_period)?;
        if on_out.is_empty() {
            return Err("控制器恢复 STARTED 后没有发送".into());
        }
        events.push("STARTED 恢复周期发送".into());

        let configured_dlc = b
            .frames
            .iter()
            .find(|f| !f.tx && f.id == b_rx_id)
            .ok_or("接收帧丢失")?
            .dlc;
        let wrong_dlc = if configured_dlc == 1 { 2 } else { 1 };
        ecu_b.command(&format!(
            "R {b_rx_id} {wrong_dlc} {}",
            "00".repeat(wrong_dlc as usize)
        ))?;
        loop {
            let line = ecu_b.next_line()?;
            if line == "E FRAME_DLC" {
                break;
            }
            if line.starts_with("E ") {
                return Err(format!("错误帧返回意外状态: {line}"));
            }
        }
        events.push("错误 DLC 被 CanIf 拒绝".into());
        Ok(())
    })();
    let closed_a = ecu_a.finish();
    let closed_b = ecu_b.finish();
    let outcome = outcome.and(closed_a).and(closed_b);
    match outcome {
        Ok(()) => Ok(RunReport {
            passed: true,
            log: "双 ECU CAN 位向量、超时、BUS_OFF 与 DLC 错误路径通过".into(),
            events,
        }),
        Err(error) => Ok(RunReport {
            passed: false,
            log: error,
            events,
        }),
    }
}
pub(super) fn run_diagnostic(
    dir: &Path,
    binary: &Path,
    owner: &ProcessOwner,
) -> Result<RunReport, String> {
    crate::generator::output::verify_build_input(dir)?;
    let profile = profile(dir)?;
    let diagnostic = profile.diagnostic.as_ref().ok_or("生成配置不含诊断连接")?;
    if diagnostic.signal_ids.is_empty()
        || diagnostic.signal_ids.len() > 8
        || diagnostic.request_id == diagnostic.response_id
        || diagnostic.s3_ms < 5000
        || diagnostic.n_as_ms == 0
        || diagnostic.n_bs_ms == 0
        || diagnostic.n_cr_ms == 0
    {
        return Err("诊断清单超出支持范围".into());
    }
    if !binary.is_file() {
        return Err("诊断虚拟 ECU 尚未完成 C99 构建".into());
    }
    let salt = 0x1357_9BDF;
    let initial_nvm = profile.dtc.as_ref().map(|_| TempNvm::new());
    let security = if diagnostic.security_enabled {
        Some(SecurityFiles::new()?)
    } else {
        None
    };
    let mut ecu = EcuProcess::start(
        binary,
        initial_nvm.as_ref().map(|state| state.path.as_path()),
        security.as_ref(),
        owner,
    )?;
    let mut events = Vec::new();
    let outcome = (|| -> Result<(), String> {
        let fence = profile.signals[0].id;
        let request = diagnostic.request_id;
        let did = diagnostic.did;
        let did_request = format!("R {request} 4 0322{did:04X}");
        let mut expected_did = vec![0x62, (did >> 8) as u8, did as u8];
        for (index, id) in diagnostic.signal_ids.iter().enumerate() {
            if diagnostic.signal_ids[..index].contains(id) {
                return Err("诊断清单重复引用信号".into());
            }
            let signal = profile
                .signals
                .iter()
                .find(|signal| signal.id == *id)
                .ok_or("诊断清单引用不存在的信号")?;
            if signal.length != 32
                || !profile
                    .frames
                    .iter()
                    .any(|frame| frame.tx && frame.path == signal.frame)
            {
                return Err("诊断 DID 只能引用 32 位发送信号".into());
            }
            expected_did.extend_from_slice(&value(signal, salt).to_be_bytes());
        }
        prepare(&mut ecu, &profile, salt)?;
        let active_session_did = format!("R {request} 4 0322F186");
        let default_session = diagnostic_request(&mut ecu, fence, active_session_did.clone())?;
        diagnostic_frames(
            &default_session,
            &[vec![0x04, 0x62, 0xF1, 0x86, 0x01]],
            &profile,
            diagnostic,
            salt,
        )?;
        let before_session = diagnostic_request(&mut ecu, fence, did_request.clone())?;
        diagnostic_frames(
            &before_session,
            &[vec![0x03, 0x7F, 0x22, 0x31]],
            &profile,
            diagnostic,
            salt,
        )?;
        let mixed_default =
            diagnostic_request(&mut ecu, fence, format!("R {request} 6 0522{did:04X}F186"))?;
        diagnostic_frames(
            &mixed_default,
            &[vec![0x04, 0x62, 0xF1, 0x86, 0x01]],
            &profile,
            diagnostic,
            salt,
        )?;
        let before_write =
            diagnostic_request(&mut ecu, fence, format!("R {request} 4 032E{did:04X}"))?;
        diagnostic_frames(
            &before_write,
            &[vec![
                0x03,
                0x7F,
                0x2E,
                if diagnostic.write_enabled { 0x31 } else { 0x11 },
            ]],
            &profile,
            diagnostic,
            salt,
        )?;
        let rid = diagnostic.reset_routine_id.unwrap_or(0xF001);
        let before_routine =
            diagnostic_request(&mut ecu, fence, format!("R {request} 5 043101{rid:04X}"))?;
        diagnostic_frames(
            &before_routine,
            &[vec![
                0x03,
                0x7F,
                0x31,
                if diagnostic.reset_routine_id.is_some() {
                    0x31
                } else {
                    0x11
                },
            ]],
            &profile,
            diagnostic,
            salt,
        )?;
        events.push("默认会话拒绝受限 DID".into());

        let session = diagnostic_request(&mut ecu, fence, format!("R {request} 3 021003"))?;
        diagnostic_frames(
            &session,
            &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]],
            &profile,
            diagnostic,
            salt,
        )?;
        let extended_session = diagnostic_request(&mut ecu, fence, active_session_did.clone())?;
        diagnostic_frames(
            &extended_session,
            &[vec![0x04, 0x62, 0xF1, 0x86, 0x03]],
            &profile,
            diagnostic,
            salt,
        )?;
        let unknown_did = if did == 0xF187 { 0xF188 } else { 0xF187 };
        let unknown_response = diagnostic_request(
            &mut ecu,
            fence,
            format!("R {request} 4 0322{unknown_did:04X}"),
        )?;
        diagnostic_frames(
            &unknown_response,
            &[vec![0x03, 0x7F, 0x22, 0x31]],
            &profile,
            diagnostic,
            salt,
        )?;
        let short_did = diagnostic_request(&mut ecu, fence, format!("R {request} 3 0222F1"))?;
        diagnostic_frames(
            &short_did,
            &[vec![0x03, 0x7F, 0x22, 0x13]],
            &profile,
            diagnostic,
            salt,
        )?;
        let malformed_pair =
            diagnostic_request(&mut ecu, fence, format!("R {request} 5 0422F18612"))?;
        diagnostic_frames(
            &malformed_pair,
            &[vec![0x03, 0x7F, 0x22, 0x13]],
            &profile,
            diagnostic,
            salt,
        )?;
        let unknown_pair = diagnostic_request(
            &mut ecu,
            fence,
            format!("R {request} 6 0522{unknown_did:04X}{unknown_did:04X}"),
        )?;
        diagnostic_frames(
            &unknown_pair,
            &[vec![0x03, 0x7F, 0x22, 0x31]],
            &profile,
            diagnostic,
            salt,
        )?;
        let mixed_unknown = diagnostic_request(
            &mut ecu,
            fence,
            format!("R {request} 6 0522{unknown_did:04X}F186"),
        )?;
        diagnostic_frames(
            &mixed_unknown,
            &[vec![0x04, 0x62, 0xF1, 0x86, 0x03]],
            &profile,
            diagnostic,
            salt,
        )?;
        let session = diagnostic_request(&mut ecu, fence, format!("R {request} 3 021001"))?;
        diagnostic_frames(
            &session,
            &[vec![0x06, 0x50, 0x01, 0x00, 0x32, 0x00, 0x32]],
            &profile,
            diagnostic,
            salt,
        )?;
        let default_session = diagnostic_request(&mut ecu, fence, active_session_did.clone())?;
        diagnostic_frames(
            &default_session,
            &[vec![0x04, 0x62, 0xF1, 0x86, 0x01]],
            &profile,
            diagnostic,
            salt,
        )?;
        let session = diagnostic_request(&mut ecu, fence, format!("R {request} 3 021003"))?;
        diagnostic_frames(
            &session,
            &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]],
            &profile,
            diagnostic,
            salt,
        )?;
        let suppressed = diagnostic_request(&mut ecu, fence, format!("R {request} 3 023E80"))?;
        diagnostic_frames(&suppressed, &[], &profile, diagnostic, salt)?;
        events.push("扩展会话与 TesterPresent 抑制响应通过".into());

        let first = diagnostic_request(&mut ecu, fence, did_request.clone())?;
        if expected_did.len() <= 7 {
            let mut single = vec![expected_did.len() as u8];
            single.extend_from_slice(&expected_did);
            diagnostic_frames(&first, &[single], &profile, diagnostic, salt)?;
            events.push("读取实时 DID 单帧载荷通过".into());
        } else {
            let mut ff = vec![
                0x10 | ((expected_did.len() >> 8) as u8 & 0x0F),
                expected_did.len() as u8,
            ];
            ff.extend_from_slice(&expected_did[..6]);
            diagnostic_frames(&first, &[ff], &profile, diagnostic, salt)?;
            let cf = diagnostic_request(&mut ecu, fence, format!("R {request} 3 300000"))?;
            let mut expected = Vec::new();
            let mut offset = 6;
            let mut sequence = 1u8;
            while offset < expected_did.len() {
                let end = (offset + 7).min(expected_did.len());
                let mut frame = vec![0x20 | sequence];
                frame.extend_from_slice(&expected_did[offset..end]);
                expected.push(frame);
                offset = end;
                sequence = (sequence + 1) & 0x0F;
            }
            diagnostic_frames(&cf, &expected, &profile, diagnostic, salt)?;
            events.push(format!(
                "实时 DID {} 字节多帧响应及流控通过",
                expected_did.len()
            ));

            let pending = diagnostic_request(&mut ecu, fence, did_request.clone())?;
            let mut ff = vec![
                0x10 | ((expected_did.len() >> 8) as u8 & 0x0F),
                expected_did.len() as u8,
            ];
            ff.extend_from_slice(&expected_did[..6]);
            diagnostic_frames(&pending, &[ff], &profile, diagnostic, salt)?;
            diagnostic_error(
                &mut ecu,
                format!("T {}", diagnostic.n_bs_ms as u64 + 1),
                "TP_TIMEOUT",
            )?;
            events.push("N_Bs 流控超时终止发送".into());
        }

        for (request_dids, configured_first) in [
            (format!("{did:04X}F186"), true),
            (format!("F186{did:04X}"), false),
        ] {
            let mut combined = vec![0x62];
            if configured_first {
                combined.extend_from_slice(&expected_did[1..]);
            }
            combined.extend_from_slice(&[0xF1, 0x86, 0x03]);
            if !configured_first {
                combined.extend_from_slice(&expected_did[1..]);
            }
            let first =
                diagnostic_request(&mut ecu, fence, format!("R {request} 6 0522{request_dids}"))?;
            let mut ff = vec![
                0x10 | ((combined.len() >> 8) as u8 & 0x0F),
                combined.len() as u8,
            ];
            ff.extend_from_slice(&combined[..6]);
            diagnostic_frames(&first, &[ff], &profile, diagnostic, salt)?;
            let rest = diagnostic_request(&mut ecu, fence, format!("R {request} 3 300000"))?;
            let expected: Vec<_> = combined[6..]
                .chunks(7)
                .enumerate()
                .map(|(index, chunk)| {
                    let mut frame = vec![0x20 | ((index as u8 + 1) & 0x0F)];
                    frame.extend_from_slice(chunk);
                    frame
                })
                .collect();
            diagnostic_frames(&rest, &expected, &profile, diagnostic, salt)?;
        }
        events.push("0x22 多 DID 按请求顺序经流控返回实时值和活动会话；无效 DID、默认会话受限 DID 与错误长度按范围处理".into());

        let padded = [0x22, (did >> 8) as u8, did as u8, 0, 0, 0, 0, 0, 0];
        let request_ff = format!("R {request} 8 1009{}", hex_payload(&padded[..6]));
        let flow = diagnostic_request(&mut ecu, fence, request_ff.clone())?;
        diagnostic_frames(&flow, &[vec![0x30, 0x00, 0x00]], &profile, diagnostic, salt)?;
        diagnostic_error(&mut ecu, format!("R {request} 4 22000000"), "TP_SEQUENCE")?;
        let flow = diagnostic_request(&mut ecu, fence, request_ff)?;
        diagnostic_frames(&flow, &[vec![0x30, 0x00, 0x00]], &profile, diagnostic, salt)?;
        let next_time = diagnostic.n_bs_ms as u64 + diagnostic.n_cr_ms as u64 + 2;
        diagnostic_error(&mut ecu, format!("T {next_time}"), "TP_TIMEOUT")?;
        let heartbeat = diagnostic_request(&mut ecu, fence, format!("R {request} 3 023E00"))?;
        diagnostic_frames(
            &heartbeat,
            &[vec![0x02, 0x7E, 0x00]],
            &profile,
            diagnostic,
            salt,
        )?;
        events.push("接收序号错误、N_Cr 超时与后续请求恢复通过".into());

        let session = diagnostic_request(&mut ecu, fence, format!("R {request} 3 021003"))?;
        diagnostic_frames(
            &session,
            &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]],
            &profile,
            diagnostic,
            salt,
        )?;
        let expiry = next_time + diagnostic.s3_ms as u64 + 1;
        let timed = diagnostic_request(&mut ecu, fence, format!("T {expiry}"))?;
        diagnostic_frames(&timed, &[], &profile, diagnostic, salt)?;
        let expired = diagnostic_request(&mut ecu, fence, did_request)?;
        diagnostic_frames(
            &expired,
            &[vec![0x03, 0x7F, 0x22, 0x31]],
            &profile,
            diagnostic,
            salt,
        )?;
        let default_session = diagnostic_request(&mut ecu, fence, active_session_did)?;
        diagnostic_frames(
            &default_session,
            &[vec![0x04, 0x62, 0xF1, 0x86, 0x01]],
            &profile,
            diagnostic,
            salt,
        )?;
        events.push("0xF186 活动会话 DID 在默认、扩展、主动切回和 S3 回退后与实际状态一致；未知 DID 与错误长度被拒绝".into());
        events.push("S3 超时恢复默认会话".into());
        Ok(())
    })();
    let outcome = outcome.and_then(|()| ecu.finish());
    drop(ecu);
    let outcome = outcome.and_then(|()| match &profile.dtc {
        Some(dtc) => verify_persistent_dtc(
            binary,
            &profile,
            diagnostic,
            dtc,
            salt,
            security.as_ref(),
            &mut events,
            owner,
        ),
        None => Ok(()),
    });
    let outcome = outcome.and_then(|()| {
        if diagnostic.write_enabled {
            verify_writable_did(
                binary,
                &profile,
                diagnostic,
                salt,
                security.as_ref(),
                &mut events,
                owner,
            )
        } else {
            Ok(())
        }
    });
    let outcome = outcome.and_then(|()| {
        if diagnostic.security_enabled {
            verify_security(binary, &profile, diagnostic, salt, &mut events, owner)
        } else {
            Ok(())
        }
    });
    match outcome {
        Ok(()) => {
            let mut log = if profile.dtc.is_some() {
                "独立测试器验证 CAN 诊断会话、实时 DID、流控及 Dem/NvM DTC 跨进程保持".to_owned()
            } else {
                "独立测试器验证 CAN 诊断会话、实时 DID、传输流控与故障恢复".to_owned()
            };
            if diagnostic.write_enabled {
                log.push_str("；扩展会话 0x2E 易失写入");
            }
            if diagnostic.reset_routine_id.is_some() {
                log.push_str("；0x31 恢复 DID 初值例程");
            }
            if diagnostic.security_enabled {
                log.push_str("；0x27 单级安全访问与受限操作");
            }
            Ok(RunReport {
                passed: true,
                log,
                events,
            })
        }
        Err(error) => Ok(RunReport {
            passed: false,
            log: error,
            events,
        }),
    }
}

fn verify_persistent_dtc(
    binary: &Path,
    profile: &Profile,
    diagnostic: &DiagnosticProfile,
    dtc: &DtcProfile,
    salt: u32,
    security: Option<&SecurityFiles>,
    events: &mut Vec<String>,
    owner: &ProcessOwner,
) -> Result<(), String> {
    let state = TempNvm::new();
    let fence = profile.signals[0].id;
    let request = diagnostic.request_id;
    let read_dtc = format!("R {request} 4 03190208");
    let count_dtc = format!("R {request} 4 03190108");
    let supported_dtc = format!("R {request} 3 02190A");
    let counted = |count| vec![0x06, 0x59, 0x01, 0x7F, 0x01, 0x00, count];
    let empty = vec![0x03, 0x59, 0x02, 0x7F];
    let reported = |status| {
        vec![
            0x07,
            0x59,
            0x02,
            0x7F,
            (dtc.code >> 16) as u8,
            (dtc.code >> 8) as u8,
            dtc.code as u8,
            status,
        ]
    };
    let supported = |status| {
        vec![
            0x07,
            0x59,
            0x0A,
            0x7F,
            (dtc.code >> 16) as u8,
            (dtc.code >> 8) as u8,
            dtc.code as u8,
            status,
        ]
    };
    {
        let mut ecu = EcuProcess::start(binary, Some(&state.path), security, owner)?;
        prepare(&mut ecu, profile, salt)?;
        let before = diagnostic_request(&mut ecu, fence, read_dtc.clone())?;
        diagnostic_frames(
            &before,
            std::slice::from_ref(&empty),
            profile,
            diagnostic,
            salt,
        )?;
        let all = diagnostic_request(&mut ecu, fence, supported_dtc.clone())?;
        diagnostic_frames(&all, &[supported(0x50)], profile, diagnostic, salt)?;
        for (payload, nrc) in [
            ("03190A00", 0x13),
            ("0119", 0x13),
            ("021903", 0x12),
            ("02198A", 0x12),
        ] {
            let invalid = diagnostic_request(
                &mut ecu,
                fence,
                format!("R {request} {} {payload}", payload.len() / 2),
            )?;
            diagnostic_frames(
                &invalid,
                &[vec![0x03, 0x7F, 0x19, nrc]],
                profile,
                diagnostic,
                salt,
            )?;
        }
        let received = diagnostic_request(
            &mut ecu,
            fence,
            format!("R {} {} {}", dtc.id, dtc.dlc, "00".repeat(dtc.dlc as usize)),
        )?;
        diagnostic_frames(&received, &[], profile, diagnostic, salt)?;
        let all = diagnostic_request(&mut ecu, fence, supported_dtc.clone())?;
        diagnostic_frames(&all, &[supported(0x00)], profile, diagnostic, salt)?;
        let count = diagnostic_request(&mut ecu, fence, count_dtc.clone())?;
        diagnostic_frames(&count, &[counted(0)], profile, diagnostic, salt)?;
        let session = diagnostic_request(&mut ecu, fence, format!("R {request} 3 021003"))?;
        diagnostic_frames(
            &session,
            &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]],
            profile,
            diagnostic,
            salt,
        )?;
        if diagnostic.security_enabled {
            security_unlock(&mut ecu, profile, diagnostic, salt)?;
        }
        let off = diagnostic_request(&mut ecu, fence, format!("R {request} 3 028502"))?;
        diagnostic_frames(&off, &[vec![0x02, 0xC5, 0x02]], profile, diagnostic, salt)?;
        let received = diagnostic_request(
            &mut ecu,
            fence,
            format!("R {} {} {}", dtc.id, dtc.dlc, "00".repeat(dtc.dlc as usize)),
        )?;
        diagnostic_frames(&received, &[], profile, diagnostic, salt)?;
        let timed = diagnostic_request(&mut ecu, fence, format!("T {}", dtc.timeout as u64 + 1))?;
        diagnostic_frames(&timed, &[], profile, diagnostic, salt)?;
        let suppressed = diagnostic_request(&mut ecu, fence, read_dtc.clone())?;
        diagnostic_frames(
            &suppressed,
            std::slice::from_ref(&empty),
            profile,
            diagnostic,
            salt,
        )?;
        let all = diagnostic_request(&mut ecu, fence, supported_dtc.clone())?;
        diagnostic_frames(&all, &[supported(0x00)], profile, diagnostic, salt)?;
        let count = diagnostic_request(&mut ecu, fence, count_dtc.clone())?;
        diagnostic_frames(&count, &[counted(0)], profile, diagnostic, salt)?;
        let on = diagnostic_request(&mut ecu, fence, format!("R {request} 3 028501"))?;
        diagnostic_frames(&on, &[vec![0x02, 0xC5, 0x01]], profile, diagnostic, salt)?;
        let received = diagnostic_request(
            &mut ecu,
            fence,
            format!("R {} {} {}", dtc.id, dtc.dlc, "00".repeat(dtc.dlc as usize)),
        )?;
        diagnostic_frames(&received, &[], profile, diagnostic, salt)?;
        let timed = diagnostic_request(
            &mut ecu,
            fence,
            format!("T {}", 2 * (dtc.timeout as u64 + 1)),
        )?;
        diagnostic_frames(&timed, &[], profile, diagnostic, salt)?;
        let failed = diagnostic_request(&mut ecu, fence, read_dtc.clone())?;
        diagnostic_frames(&failed, &[reported(0x2F)], profile, diagnostic, salt)?;
        let all = diagnostic_request(&mut ecu, fence, supported_dtc.clone())?;
        diagnostic_frames(&all, &[supported(0x2F)], profile, diagnostic, salt)?;
        let count = diagnostic_request(&mut ecu, fence, count_dtc.clone())?;
        diagnostic_frames(&count, &[counted(1)], profile, diagnostic, salt)?;
        ecu.finish()?;
    }
    events.push("接收帧超时产生真实 Dem DTC，进程结束前写入 NvM".into());
    events.push(
        "0x85/0x02 禁用 DTC 设置时 Rx 超时不记录故障；0x85/0x01 恢复后新超时写入 Dem/NvM".into(),
    );
    {
        let mut ecu = EcuProcess::start(binary, Some(&state.path), security, owner)?;
        prepare(&mut ecu, profile, salt)?;
        let recovered = diagnostic_request(&mut ecu, fence, read_dtc.clone())?;
        diagnostic_frames(&recovered, &[reported(0x6D)], profile, diagnostic, salt)?;
        let all = diagnostic_request(&mut ecu, fence, supported_dtc.clone())?;
        diagnostic_frames(&all, &[supported(0x6D)], profile, diagnostic, salt)?;
        let count = diagnostic_request(&mut ecu, fence, count_dtc.clone())?;
        diagnostic_frames(&count, &[counted(1)], profile, diagnostic, salt)?;
        let passed = diagnostic_request(
            &mut ecu,
            fence,
            format!("R {} {} {}", dtc.id, dtc.dlc, "00".repeat(dtc.dlc as usize)),
        )?;
        diagnostic_frames(&passed, &[], profile, diagnostic, salt)?;
        let recovered_pass = diagnostic_request(&mut ecu, fence, read_dtc.clone())?;
        diagnostic_frames(
            &recovered_pass,
            &[reported(0x2C)],
            profile,
            diagnostic,
            salt,
        )?;
        let count = diagnostic_request(&mut ecu, fence, count_dtc.clone())?;
        diagnostic_frames(&count, &[counted(1)], profile, diagnostic, salt)?;
        let denied = diagnostic_request(&mut ecu, fence, format!("R {request} 5 0414FFFFFF"))?;
        diagnostic_frames(
            &denied,
            &[vec![0x03, 0x7F, 0x14, 0x7F]],
            profile,
            diagnostic,
            salt,
        )?;
        let session = diagnostic_request(&mut ecu, fence, format!("R {request} 3 021003"))?;
        diagnostic_frames(
            &session,
            &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]],
            profile,
            diagnostic,
            salt,
        )?;
        if diagnostic.security_enabled {
            security_unlock(&mut ecu, profile, diagnostic, salt)?;
        }
        let wrong_group = diagnostic_request(&mut ecu, fence, format!("R {request} 5 0414000001"))?;
        diagnostic_frames(
            &wrong_group,
            &[vec![0x03, 0x7F, 0x14, 0x31]],
            profile,
            diagnostic,
            salt,
        )?;
        let cleared = diagnostic_request(&mut ecu, fence, format!("R {request} 5 0414FFFFFF"))?;
        diagnostic_frames(&cleared, &[vec![0x01, 0x54]], profile, diagnostic, salt)?;
        let now_empty = diagnostic_request(&mut ecu, fence, read_dtc.clone())?;
        diagnostic_frames(
            &now_empty,
            std::slice::from_ref(&empty),
            profile,
            diagnostic,
            salt,
        )?;
        let all = diagnostic_request(&mut ecu, fence, supported_dtc.clone())?;
        diagnostic_frames(&all, &[supported(0x50)], profile, diagnostic, salt)?;
        let count = diagnostic_request(&mut ecu, fence, count_dtc.clone())?;
        diagnostic_frames(&count, &[counted(0)], profile, diagnostic, salt)?;
        ecu.finish()?;
    }
    {
        let mut ecu = EcuProcess::start(binary, Some(&state.path), security, owner)?;
        let after_restart = diagnostic_request(&mut ecu, fence, read_dtc)?;
        diagnostic_frames(&after_restart, &[empty], profile, diagnostic, salt)?;
        let count = diagnostic_request(&mut ecu, fence, count_dtc)?;
        diagnostic_frames(&count, &[counted(0)], profile, diagnostic, salt)?;
        let all = diagnostic_request(&mut ecu, fence, supported_dtc)?;
        diagnostic_frames(&all, &[supported(0x50)], profile, diagnostic, salt)?;
        ecu.finish()?;
    }
    events.push("重启后 DTC 保持、默认会话拒绝清除、扩展会话清除跨重启生效".into());
    events.push("0x19/0x01 状态掩码计数与 0x19/0x02 在超时、重启、清除前后一致".into());
    events.push("0x19/0x0A 在无故障、暂停记录、超时、重启与清除后均报告配置的 DTC 和当前状态；错误长度与不支持子功能被拒绝后可恢复".into());
    fs::write(&state.path, [0u8; 64]).map_err(|e| format!("故障注入 NvM 损坏失败: {e}"))?;
    EcuProcess::expect_corrupt_state_refusal(binary, Some(&state.path), security, owner)?;
    events.push("双份 NvM 状态损坏在启动时被拒绝，未伪造空 DTC".into());
    Ok(())
}

fn expect_did_bytes(
    ecu: &mut EcuProcess,
    profile: &Profile,
    diagnostic: &DiagnosticProfile,
    salt: u32,
    data: &[u8],
) -> Result<(), String> {
    let fence = profile.signals[0].id;
    let request = diagnostic.request_id;
    let did = diagnostic.did;
    let mut expected = vec![0x62, (did >> 8) as u8, did as u8];
    expected.extend_from_slice(data);
    let first = diagnostic_request(ecu, fence, format!("R {request} 4 0322{did:04X}"))?;
    if expected.len() <= 7 {
        let mut single = vec![expected.len() as u8];
        single.extend_from_slice(&expected);
        return diagnostic_frames(&first, &[single], profile, diagnostic, salt);
    }
    let mut ff = vec![
        0x10 | ((expected.len() >> 8) as u8 & 0x0F),
        expected.len() as u8,
    ];
    ff.extend_from_slice(&expected[..6]);
    diagnostic_frames(&first, &[ff], profile, diagnostic, salt)?;
    let following = diagnostic_request(ecu, fence, format!("R {request} 3 300000"))?;
    let mut frames = Vec::new();
    for (index, chunk) in expected[6..].chunks(7).enumerate() {
        let mut cf = vec![0x20 | ((index + 1) as u8 & 0x0F)];
        cf.extend_from_slice(chunk);
        frames.push(cf);
    }
    diagnostic_frames(&following, &frames, profile, diagnostic, salt)
}

fn verify_writable_did(
    binary: &Path,
    profile: &Profile,
    diagnostic: &DiagnosticProfile,
    salt: u32,
    security: Option<&SecurityFiles>,
    events: &mut Vec<String>,
    owner: &ProcessOwner,
) -> Result<(), String> {
    let storage = profile.dtc.as_ref().map(|_| TempNvm::new());
    let mut ecu = EcuProcess::start(
        binary,
        storage.as_ref().map(|state| state.path.as_path()),
        security,
        owner,
    )?;
    let request = diagnostic.request_id;
    let fence = profile.signals[0].id;
    let did = diagnostic.did;
    prepare(&mut ecu, profile, salt)?;
    let session = diagnostic_request(&mut ecu, fence, format!("R {request} 3 021003"))?;
    diagnostic_frames(
        &session,
        &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]],
        profile,
        diagnostic,
        salt,
    )?;
    if diagnostic.security_enabled {
        security_unlock(&mut ecu, profile, diagnostic, salt)?;
    }
    let other_did = if did == u16::MAX { did - 1 } else { did + 1 };
    let unknown = diagnostic_request(
        &mut ecu,
        fence,
        format!("R {request} 4 032E{other_did:04X}"),
    )?;
    diagnostic_frames(
        &unknown,
        &[vec![0x03, 0x7F, 0x2E, 0x31]],
        profile,
        diagnostic,
        salt,
    )?;
    let truncated = diagnostic_request(&mut ecu, fence, format!("R {request} 4 032E{did:04X}"))?;
    diagnostic_frames(
        &truncated,
        &[vec![0x03, 0x7F, 0x2E, 0x13]],
        profile,
        diagnostic,
        salt,
    )?;

    let signals: Vec<_> = diagnostic
        .signal_ids
        .iter()
        .map(|id| {
            profile
                .signals
                .iter()
                .find(|signal| signal.id == *id)
                .ok_or("写入 DID 引用不存在的信号")
        })
        .collect::<Result<_, _>>()?;
    let written: Vec<u32> = signals.iter().map(|signal| !value(signal, salt)).collect();
    let mut record = vec![0x2E, (did >> 8) as u8, did as u8];
    for value in &written {
        record.extend_from_slice(&value.to_be_bytes());
    }
    let positive = vec![0x03, 0x6E, (did >> 8) as u8, did as u8];
    if record.len() <= 7 {
        let mut sf = vec![record.len() as u8];
        sf.extend_from_slice(&record);
        let response = diagnostic_request(
            &mut ecu,
            fence,
            format!("R {request} {} {}", sf.len(), hex_payload(&sf)),
        )?;
        diagnostic_frames(&response, &[positive], profile, diagnostic, salt)?;
    } else {
        let mut ff = vec![
            0x10 | ((record.len() >> 8) as u8 & 0x0F),
            record.len() as u8,
        ];
        ff.extend_from_slice(&record[..6]);
        let flow = diagnostic_request(
            &mut ecu,
            fence,
            format!("R {request} {} {}", ff.len(), hex_payload(&ff)),
        )?;
        diagnostic_frames(&flow, &[vec![0x30, 0x00, 0x00]], profile, diagnostic, salt)?;
        let chunk_count = (record.len() - 6).div_ceil(7);
        for (index, chunk) in record[6..].chunks(7).enumerate() {
            let mut cf = vec![0x20 | ((index + 1) as u8 & 0x0F)];
            cf.extend_from_slice(chunk);
            let response = diagnostic_request(
                &mut ecu,
                fence,
                format!("R {request} {} {}", cf.len(), hex_payload(&cf)),
            )?;
            let expected = if index + 1 == chunk_count {
                std::slice::from_ref(&positive)
            } else {
                &[]
            };
            diagnostic_frames(&response, expected, profile, diagnostic, salt)?;
        }
    }
    let mut bytes = Vec::with_capacity(written.len() * 4);
    for value in &written {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
    expect_did_bytes(&mut ecu, profile, diagnostic, salt, &bytes)?;
    for (signal, expected) in signals.iter().zip(&written) {
        let (frames, response) = ecu.query(&[], signal.id)?;
        if !frames.is_empty() || parse_value(&response)? != (*expected, true) {
            return Err(format!("0x2E 未更新实际 Com 信号 {}", signal.id));
        }
    }
    let monitored_frame = profile
        .frames
        .iter()
        .find(|frame| frame.path == signals[0].frame && frame.tx)
        .ok_or("写入 DID 未关联发送帧")?;
    let periodic = tick(&mut ecu, profile, monitored_frame.period as u64)?;
    let mut observed = false;
    for line in periodic {
        let (id, payload) = parse_frame(&line)?;
        let frame = profile
            .frames
            .iter()
            .find(|frame| frame.tx && frame.id == id && frame.dlc as usize == payload.len())
            .ok_or("0x2E 后 ECU 发送了未知 CAN 帧")?;
        for signal in profile
            .signals
            .iter()
            .filter(|signal| signal.frame == frame.path)
        {
            let expected = diagnostic
                .signal_ids
                .iter()
                .position(|id| *id == signal.id)
                .map(|index| written[index])
                .unwrap_or_else(|| value(signal, salt));
            if signal_value(&payload, signal) != expected {
                return Err(format!("0x2E 后 CAN 帧未携带更新的信号 {}", signal.id));
            }
        }
        observed |= frame.id == monitored_frame.id;
    }
    if !observed {
        return Err("0x2E 后未观察到被写入信号的周期 CAN 帧".into());
    }
    events.push("扩展会话 0x2E 实际写入 Com 信号，0x22 与周期 CAN 均观察到新值".into());
    if let Some(rid) = diagnostic.reset_routine_id {
        let session = diagnostic_request(&mut ecu, fence, format!("R {request} 3 021003"))?;
        diagnostic_frames(
            &session,
            &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]],
            profile,
            diagnostic,
            salt,
        )?;
        let other_rid = if rid == u16::MAX { rid - 1 } else { rid + 1 };
        let unknown = diagnostic_request(
            &mut ecu,
            fence,
            format!("R {request} 5 043101{other_rid:04X}"),
        )?;
        diagnostic_frames(
            &unknown,
            &[vec![0x03, 0x7F, 0x31, 0x31]],
            profile,
            diagnostic,
            salt,
        )?;
        let unsupported =
            diagnostic_request(&mut ecu, fence, format!("R {request} 5 043102{rid:04X}"))?;
        diagnostic_frames(
            &unsupported,
            &[vec![0x03, 0x7F, 0x31, 0x12]],
            profile,
            diagnostic,
            salt,
        )?;
        let malformed =
            diagnostic_request(&mut ecu, fence, format!("R {request} 6 053101{rid:04X}00"))?;
        diagnostic_frames(
            &malformed,
            &[vec![0x03, 0x7F, 0x31, 0x13]],
            profile,
            diagnostic,
            salt,
        )?;
        for (signal, expected) in signals.iter().zip(&written) {
            let (frames, response) = ecu.query(&[], signal.id)?;
            if !frames.is_empty() || parse_value(&response)? != (*expected, true) {
                return Err(format!("被拒绝的 0x31 请求改动了信号 {}", signal.id));
            }
        }
        if diagnostic.security_enabled {
            security_unlock(&mut ecu, profile, diagnostic, salt)?;
        }
        let restored =
            diagnostic_request(&mut ecu, fence, format!("R {request} 5 043101{rid:04X}"))?;
        diagnostic_frames(
            &restored,
            &[vec![0x04, 0x71, 0x01, (rid >> 8) as u8, rid as u8]],
            profile,
            diagnostic,
            salt,
        )?;
        let mut initial = Vec::with_capacity(signals.len() * 4);
        for signal in &signals {
            initial.extend_from_slice(&signal.initial.to_be_bytes());
        }
        expect_did_bytes(&mut ecu, profile, diagnostic, salt, &initial)?;
        for signal in &signals {
            let (frames, response) = ecu.query(&[], signal.id)?;
            if !frames.is_empty() || parse_value(&response)? != (signal.initial, true) {
                return Err(format!("0x31 例程未恢复实际 Com 信号 {}", signal.id));
            }
        }
        events.push("0x31/0x01 例程将可写 DID 恢复至配置初值，0x22 与 Com 信号均观察到恢复".into());
    }
    let expired_at = monitored_frame.period as u64 + diagnostic.s3_ms as u64 + 1;
    for line in tick(&mut ecu, profile, expired_at)? {
        let (id, _) = parse_frame(&line)?;
        if id == diagnostic.response_id {
            return Err("会话超时推进中出现未请求的诊断响应".into());
        }
    }
    let denied = diagnostic_request(&mut ecu, fence, format!("R {request} 4 032E{did:04X}"))?;
    diagnostic_frames(
        &denied,
        &[vec![0x03, 0x7F, 0x2E, 0x31]],
        profile,
        diagnostic,
        salt,
    )?;
    if let Some(rid) = diagnostic.reset_routine_id {
        let denied = diagnostic_request(&mut ecu, fence, format!("R {request} 5 043101{rid:04X}"))?;
        diagnostic_frames(
            &denied,
            &[vec![0x03, 0x7F, 0x31, 0x31]],
            profile,
            diagnostic,
            salt,
        )?;
    }
    ecu.finish()?;
    drop(ecu);

    let mut restarted = EcuProcess::start(
        binary,
        storage.as_ref().map(|state| state.path.as_path()),
        security,
        owner,
    )?;
    for signal in &signals {
        let (frames, response) = restarted.query(&[], signal.id)?;
        if !frames.is_empty() || parse_value(&response)? != (signal.initial, true) {
            return Err(format!(
                "易失 DID 信号 {} 在重启后未恢复配置初值",
                signal.id
            ));
        }
    }
    let session = diagnostic_request(&mut restarted, fence, format!("R {request} 3 021003"))?;
    diagnostic_frames(
        &session,
        &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]],
        profile,
        diagnostic,
        salt,
    )?;
    let mut initial = Vec::with_capacity(signals.len() * 4);
    for signal in &signals {
        initial.extend_from_slice(&signal.initial.to_be_bytes());
    }
    expect_did_bytes(&mut restarted, profile, diagnostic, salt, &initial)?;
    restarted.finish()?;
    events.push("S3 回默认会话拒绝写入；ECU 重启后 DID 恢复初值且未误称 NvM 持久化".into());
    Ok(())
}

fn verify_security(
    binary: &Path,
    profile: &Profile,
    diagnostic: &DiagnosticProfile,
    salt: u32,
    events: &mut Vec<String>,
    owner: &ProcessOwner,
) -> Result<(), String> {
    let files = SecurityFiles::new()?;
    let nvm = profile.dtc.as_ref().map(|_| TempNvm::new());
    let fence = profile.signals[0].id;
    let request = diagnostic.request_id;
    let mut protected = Vec::new();
    if diagnostic.write_enabled {
        let mut data = vec![0x2e, (diagnostic.did >> 8) as u8, diagnostic.did as u8];
        data.resize(3 + diagnostic.signal_ids.len() * 4, 0u8);
        protected.push(data);
    }
    if let Some(rid) = diagnostic.reset_routine_id {
        protected.push(vec![0x31, 0x01, (rid >> 8) as u8, rid as u8]);
    }
    if profile.dtc.is_some() {
        protected.push(vec![0x14, 0xff, 0xff, 0xff]);
        protected.push(vec![0x85, 0x01]);
    }
    let check_protected = |ecu: &mut EcuProcess, unlocked: bool| -> Result<(), String> {
        for payload in &protected {
            let result = send_payload(ecu, fence, request, payload)?;
            let expected = if !unlocked {
                vec![0x03, 0x7f, payload[0], 0x33]
            } else {
                match payload[0] {
                    0x2e => vec![0x03, 0x6e, payload[1], payload[2]],
                    0x31 => vec![0x04, 0x71, 0x01, payload[2], payload[3]],
                    0x14 => vec![0x01, 0x54],
                    0x85 => vec![0x02, 0xc5, 0x01],
                    _ => return Err("安全档案含未知受保护服务".into()),
                }
            };
            diagnostic_frames(&result, &[expected], profile, diagnostic, salt)?;
        }
        Ok(())
    };
    {
        let mut ecu = EcuProcess::start(
            binary,
            nvm.as_ref().map(|s| s.path.as_path()),
            Some(&files),
            owner,
        )?;
        let before_session = send_payload(&mut ecu, fence, request, &[0x27, 0x01])?;
        diagnostic_frames(
            &before_session,
            &[vec![0x03, 0x7f, 0x27, 0x7f]],
            profile,
            diagnostic,
            salt,
        )?;
        let session = send_payload(&mut ecu, fence, request, &[0x10, 0x03])?;
        diagnostic_frames(
            &session,
            &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]],
            profile,
            diagnostic,
            salt,
        )?;
        check_protected(&mut ecu, false)?;
        let out_of_order = send_payload(
            &mut ecu,
            fence,
            request,
            &[0x27, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        )?;
        diagnostic_frames(
            &out_of_order,
            &[vec![0x03, 0x7f, 0x27, 0x24]],
            profile,
            diagnostic,
            salt,
        )?;
        let used_key = security_unlock(&mut ecu, profile, diagnostic, salt)?;
        if security_seed(&mut ecu, profile, diagnostic, salt)? != [0u8; 16] {
            return Err("已解锁级别请求 seed 时未返回零 seed".into());
        }
        let mut replay = vec![0x27, 0x02];
        replay.extend_from_slice(&used_key);
        let rejected_replay = send_payload(&mut ecu, fence, request, &replay)?;
        diagnostic_frames(
            &rejected_replay,
            &[vec![0x03, 0x7f, 0x27, 0x24]],
            profile,
            diagnostic,
            salt,
        )?;
        check_protected(&mut ecu, true)?;
        let repeated_session = send_payload(&mut ecu, fence, request, &[0x10, 0x03])?;
        diagnostic_frames(
            &repeated_session,
            &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]],
            profile,
            diagnostic,
            salt,
        )?;
        check_protected(&mut ecu, false)?;
        for attempt in 1..=3 {
            let _seed = security_seed(&mut ecu, profile, diagnostic, salt)?;
            let mut wrong = vec![0x27, 0x02];
            wrong.resize(18, 0u8);
            let result = send_payload(&mut ecu, fence, request, &wrong)?;
            diagnostic_frames(
                &result,
                &[vec![
                    0x03,
                    0x7f,
                    0x27,
                    if attempt == 3 { 0x36 } else { 0x35 },
                ]],
                profile,
                diagnostic,
                salt,
            )?;
        }
        let delayed = send_payload(&mut ecu, fence, request, &[0x27, 0x01])?;
        diagnostic_frames(
            &delayed,
            &[vec![0x03, 0x7f, 0x27, 0x37]],
            profile,
            diagnostic,
            salt,
        )?;
        ecu.finish()?;
    }
    {
        let mut ecu = EcuProcess::start(
            binary,
            nvm.as_ref().map(|s| s.path.as_path()),
            Some(&files),
            owner,
        )?;
        let session = send_payload(&mut ecu, fence, request, &[0x10, 0x03])?;
        diagnostic_frames(
            &session,
            &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]],
            profile,
            diagnostic,
            salt,
        )?;
        let delayed = send_payload(&mut ecu, fence, request, &[0x27, 0x01])?;
        diagnostic_frames(
            &delayed,
            &[vec![0x03, 0x7f, 0x27, 0x37]],
            profile,
            diagnostic,
            salt,
        )?;
        let _ = tick(&mut ecu, profile, 5000)?;
        let session = send_payload(&mut ecu, fence, request, &[0x10, 0x03])?;
        diagnostic_frames(
            &session,
            &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]],
            profile,
            diagnostic,
            salt,
        )?;
        security_unlock(&mut ecu, profile, diagnostic, salt)?;
        let _ = tick(&mut ecu, profile, 5000 + diagnostic.s3_ms as u64 + 1)?;
        let session = send_payload(&mut ecu, fence, request, &[0x10, 0x03])?;
        diagnostic_frames(
            &session,
            &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]],
            profile,
            diagnostic,
            salt,
        )?;
        check_protected(&mut ecu, false)?;
        ecu.finish()?;
    }
    fs::write(&files.state.path, [0u8; 16]).map_err(|e| format!("无法注入安全计数损坏: {e}"))?;
    EcuProcess::expect_corrupt_state_refusal(
        binary,
        nvm.as_ref().map(|state| state.path.as_path()),
        Some(&files),
        owner,
    )?;
    events
        .push("0x27 seed/key 解锁、受保护操作、错误 key 次数/延时、重启保持与 S3 复锁通过".into());
    events.push("损坏的安全失败计数文件在启动时被拒绝".into());
    Ok(())
}
