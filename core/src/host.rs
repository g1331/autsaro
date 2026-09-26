use crate::model::RunReport;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sha2::{Digest, Sha256};

#[derive(Clone)]
struct Signal { id: u16, frame: String, start: u8, length: u8, initial: u32 }
#[derive(Clone)]
struct Frame { path: String, id: u32, dlc: u8, tx: bool, period: u32, timeout: u32 }
struct DiagnosticProfile {
    request_id: u32,
    response_id: u32,
    s3_ms: u32,
    n_bs_ms: u32,
    n_cr_ms: u32,
    did: u16,
    signal_ids: Vec<u16>,
    write_enabled: bool,
    reset_routine_id: Option<u16>,
    security_enabled: bool,
}
struct DtcProfile { code: u32, frame_index: usize, id: u32, dlc: u8, timeout: u32 }
struct Profile { frames: Vec<Frame>, signals: Vec<Signal>, diagnostic: Option<DiagnosticProfile>, dtc: Option<DtcProfile>, text: String }

fn profile(dir: &Path) -> Result<Profile, String> {
    let text = fs::read_to_string(dir.join("profile.txt")).map_err(|e| format!("配置清单缺失: {e}"))?;
    let mut frames = Vec::new();
    let mut signals = Vec::new();
    let mut diagnostic = None;
    let mut dtc = None;
    let mut write_did = None;
    let mut reset_routine = None;
    let mut security = false;
    for line in text.lines() {
        let cols: Vec<_> = line.split_whitespace().collect();
        match cols.first().copied() {
            Some("FRAME") if cols.len() >= 8 => {
                let field = |name: &str| cols.iter().find_map(|c| c.strip_prefix(name));
                frames.push(Frame {
                    path: cols[2].into(),
                    id: field("id=").ok_or("清单缺少 id")?.parse().map_err(|_| "帧 ID 无效")?,
                    dlc: field("dlc=").ok_or("清单缺少 dlc")?.parse().map_err(|_| "DLC 无效")?,
                    tx: field("direction=") == Some("tx"),
                    period: field("period=").ok_or("清单缺少周期")?.parse().map_err(|_| "周期无效")?,
                    timeout: field("timeout=").ok_or("清单缺少超时")?.parse().map_err(|_| "超时无效")?,
                });
            }
            Some("SIGNAL") if cols.len() >= 6 => {
                let field = |name: &str| cols.iter().find_map(|c| c.strip_prefix(name));
                let (start, length) = field("bits=").ok_or("缺少信号位段")?.split_once(':').ok_or("位段无效")?;
                signals.push(Signal { id: cols[1].parse().map_err(|_| "信号 ID 无效")?, frame: field("frame=").ok_or("缺少信号帧引用")?.into(), start: start.parse().map_err(|_| "起始位无效")?, length: length.parse().map_err(|_| "长度无效")?, initial: field("initial=").ok_or("缺少信号初值")?.parse().map_err(|_| "信号初值无效")? });
            }
            Some("DIAGNOSTIC") if cols.len() == 8 => {
                let field = |name: &str| cols.iter().find_map(|column| column.strip_prefix(name));
                let ids = field("signals=").ok_or("诊断清单缺少 DID 信号")?.split(',')
                    .map(|id| id.parse::<u16>().map_err(|_| "诊断信号 ID 无效".to_owned()))
                    .collect::<Result<Vec<_>, _>>()?;
                let config = DiagnosticProfile {
                    request_id: field("request=").ok_or("诊断请求 ID 缺失")?.parse().map_err(|_| "诊断请求 ID 无效")?,
                    response_id: field("response=").ok_or("诊断响应 ID 缺失")?.parse().map_err(|_| "诊断响应 ID 无效")?,
                    s3_ms: field("s3=").ok_or("诊断 S3 缺失")?.parse().map_err(|_| "诊断 S3 无效")?,
                    n_bs_ms: field("nbs=").ok_or("诊断 N_Bs 缺失")?.parse().map_err(|_| "诊断 N_Bs 无效")?,
                    n_cr_ms: field("ncr=").ok_or("诊断 N_Cr 缺失")?.parse().map_err(|_| "诊断 N_Cr 无效")?,
                    did: field("did=").ok_or("诊断 DID 缺失")?.parse().map_err(|_| "诊断 DID 无效")?,
                    signal_ids: ids,
                    write_enabled: false,
                    reset_routine_id: None,
                    security_enabled: false,
                };
                if diagnostic.replace(config).is_some() { return Err("诊断清单含多个连接".into()); }
            }
            Some("DIAGNOSTIC") => return Err("诊断清单字段数量错误".into()),
            Some("WRITE_DID") if cols.len() == 2 => {
                let did = cols[1].strip_prefix("did=").ok_or("写入 DID 清单缺少编号")?
                    .parse::<u16>().map_err(|_| "写入 DID 编号无效")?;
                if write_did.replace(did).is_some() { return Err("诊断清单含多个写入 DID".into()); }
            }
            Some("WRITE_DID") => return Err("写入 DID 清单字段数量错误".into()),
            Some("RESET_ROUTINE") if cols.len() == 2 => {
                let id = cols[1].strip_prefix("id=").ok_or("例程清单缺少 RID")?
                    .parse::<u16>().map_err(|_| "例程 RID 无效")?;
                if reset_routine.replace(id).is_some() { return Err("诊断清单含多个例程".into()); }
            }
            Some("RESET_ROUTINE") => return Err("例程清单字段数量错误".into()),
            Some("SECURITY") if cols == ["SECURITY", "level=1", "seed=16", "key=16", "attempts=3", "delay=5000"] => {
                if security { return Err("诊断清单含多个安全档案".into()); }
                security = true;
            }
            Some("SECURITY") => return Err("安全档案清单不受支持".into()),
            Some("DTC") if cols.len() == 6 => {
                let field = |name: &str| cols.iter().find_map(|column| column.strip_prefix(name));
                let config = DtcProfile {
                    code: field("code=").ok_or("DTC 编码缺失")?.parse().map_err(|_| "DTC 编码无效")?,
                    frame_index: field("frame=").ok_or("DTC 监控帧索引缺失")?.parse().map_err(|_| "DTC 监控帧索引无效")?,
                    id: field("id=").ok_or("DTC 监控 CAN ID 缺失")?.parse().map_err(|_| "DTC 监控 CAN ID 无效")?,
                    dlc: field("dlc=").ok_or("DTC 监控帧 DLC 缺失")?.parse().map_err(|_| "DTC 监控帧 DLC 无效")?,
                    timeout: field("timeout=").ok_or("DTC 监控超时缺失")?.parse().map_err(|_| "DTC 监控超时无效")?,
                };
                if dtc.replace(config).is_some() { return Err("诊断清单含多个 DTC".into()); }
            }
            Some("DTC") => return Err("DTC 清单字段数量错误".into()),
            _ => {}
        }
    }
    if frames.is_empty() || signals.is_empty() { return Err("生成配置缺少帧或信号".into()); }
    if let Some(did) = write_did {
        let configured = diagnostic.as_mut().ok_or("写入 DID 缺少诊断连接")?;
        if configured.did != did { return Err("写入 DID 与诊断连接不一致".into()); }
        configured.write_enabled = true;
    }
    if let Some(id) = reset_routine {
        let configured = diagnostic.as_mut().ok_or("例程缺少诊断连接")?;
        if !configured.write_enabled { return Err("恢复 DID 例程需要可写 DID".into()); }
        configured.reset_routine_id = Some(id);
    }
    if security {
        let configured = diagnostic.as_mut().ok_or("安全档案缺少诊断连接")?;
        if !configured.write_enabled && dtc.is_none() { return Err("安全档案没有受保护操作".into()); }
        configured.security_enabled = true;
    }
    if let Some(config) = &dtc {
        let frame = frames.get(config.frame_index).ok_or("DTC 监控帧不存在")?;
        if diagnostic.is_none() || config.code < 0x100 || config.code >= 0xFFFFFF ||
            frame.tx || frame.id != config.id || frame.dlc != config.dlc || frame.timeout == 0 ||
            frame.timeout != config.timeout || !signals.iter().any(|signal| signal.frame == frame.path) {
            return Err("DTC 清单与受支持的 Rx 超时监控配置不一致".into());
        }
    }
    Ok(Profile { frames, signals, diagnostic, dtc, text })
}

struct TempNvm { path: PathBuf }
impl TempNvm {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("system time before epoch").as_nanos();
        let path = std::env::temp_dir().join(format!("autosar-dtc-{}-{stamp}-{}.nvm", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        Self { path }
    }
}
impl Drop for TempNvm {
    fn drop(&mut self) { let _ = fs::remove_file(&self.path); }
}

struct SecurityFiles { key: TempNvm, state: TempNvm }
impl SecurityFiles {
    fn new() -> Result<Self, String> {
        let files = Self { key: TempNvm::new(), state: TempNvm::new() };
        fs::write(&files.key.path, [0x5au8; 32]).map_err(|e| format!("无法准备隔离的测试密钥: {e}"))?;
        Ok(files)
    }
}

struct EcuProcess {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
}
impl EcuProcess {
    fn start(path: &Path, nvm_path: Option<&Path>, security: Option<&SecurityFiles>) -> Result<Self, String> {
        let mut command = Command::new(path);
        if let Some(storage) = nvm_path { command.arg("--nvm").arg(storage); }
        if let Some(files) = security {
            command.arg("--security-key").arg(&files.key.path)
                .arg("--security-state").arg(&files.state.path);
        }
        let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
            .map_err(|e| format!("无法启动 ECU {}: {e}", path.display()))?;
        let stdin = child.stdin.take().ok_or("ECU stdin 不可用")?;
        let stdout = child.stdout.take().ok_or("ECU stdout 不可用")?;
        let (sender, lines) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line { Ok(line) => { if sender.send(line).is_err() { break; } }, Err(_) => break }
            }
        });
        Ok(Self { child, stdin, lines })
    }
    fn command(&mut self, command: &str) -> Result<(), String> {
        writeln!(self.stdin, "{command}").and_then(|_| self.stdin.flush()).map_err(|e| format!("ECU 管道写入失败: {e}"))
    }
    fn query(&mut self, commands: &[String], fence: u16) -> Result<(Vec<String>, String), String> {
        for command in commands { self.command(command)?; }
        self.command(&format!("G {fence}"))?;
        let mut events = Vec::new();
        loop {
            let line = self.lines.recv_timeout(Duration::from_secs(5)).map_err(|e| format!("ECU 未响应: {e}"))?;
            if line.starts_with("V ") { return Ok((events, line)); }
            if line.starts_with("E ") { return Err(format!("ECU 拒绝命令: {line}")); }
            if !line.starts_with("X ") { return Err(format!("ECU 返回未知协议: {line}")); }
            events.push(line);
        }
    }
}
impl Drop for EcuProcess {
    fn drop(&mut self) { let _ = self.child.kill(); let _ = self.child.wait(); }
}

fn value(signal: &Signal, salt: u32) -> u32 {
    let mask = if signal.length == 32 { u32::MAX } else { (1u32 << signal.length) - 1 };
    (0xA53C_71E9u32.rotate_right((signal.id % 31) as u32) ^ salt) & mask
}
fn expected_payload(frame: &Frame, profile: &Profile, salt: u32) -> Vec<u8> {
    let mut data = vec![0u8; frame.dlc as usize];
    for signal in profile.signals.iter().filter(|s| s.frame == frame.path) {
        let v = value(signal, salt);
        for bit in 0..signal.length { let position = signal.start as usize + bit as usize; data[position / 8] |= (((v >> bit) & 1) as u8) << (position % 8); }
    }
    data
}
fn parse_frame(event: &str) -> Result<(u32, Vec<u8>), String> {
    let mut parts = event.split_whitespace();
    if parts.next() != Some("X") { return Err(format!("不是报文: {event}")); }
    let id = parts.next().ok_or("报文缺少 ID")?.parse().map_err(|_| "报文 ID 无效")?;
    let dlc: usize = parts.next().ok_or("报文缺少 DLC")?.parse().map_err(|_| "报文 DLC 无效")?;
    let data = parts.next().ok_or("报文缺少载荷")?;
    if parts.next().is_some() || data.len() != dlc * 2 { return Err("报文载荷长度不一致".into()); }
    let mut bytes = Vec::new();
    for pair in data.as_bytes().chunks_exact(2) { bytes.push(u8::from_str_radix(std::str::from_utf8(pair).map_err(|e| e.to_string())?, 16).map_err(|_| "报文十六进制无效")?); }
    Ok((id, bytes))
}
fn parse_value(response: &str) -> Result<(u32, bool), String> {
    let words: Vec<_> = response.split_whitespace().collect();
    if words.len() != 4 || words[0] != "V" { return Err(format!("信号响应无效: {response}")); }
    Ok((words[2].parse().map_err(|_| "信号值无效")?, words[3] == "1"))
}
fn signal_value(payload: &[u8], signal: &Signal) -> u32 {
    let mut result = 0;
    for bit in 0..signal.length { let pos = signal.start as usize + bit as usize; result |= (((payload[pos / 8] >> (pos % 8)) & 1) as u32) << bit; }
    result
}
fn prepare(ecu: &mut EcuProcess, profile: &Profile, salt: u32) -> Result<(), String> {
    let commands: Vec<_> = profile.signals.iter().filter(|s| profile.frames.iter().any(|f| f.tx && f.path == s.frame))
        .map(|s| format!("S {} {}", s.id, value(s, salt))).collect();
    let (unexpected, _) = ecu.query(&commands, profile.signals[0].id)?;
    if !unexpected.is_empty() { return Err("时间尚未推进，ECU 已发送 CAN 帧".into()); }
    Ok(())
}

fn tick(ecu: &mut EcuProcess, profile: &Profile, time: u64) -> Result<Vec<String>, String> {
    let (frames, _) = ecu.query(&[format!("T {time}")], profile.signals[0].id)?;
    Ok(frames)
}

fn route_and_check(a_ecu: &mut EcuProcess, b_ecu: &mut EcuProcess, a: &Profile, b: &Profile, a_out: &[String], b_out: &[String], events: &mut Vec<String>) -> Result<u32, String> {
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
        let (tx, rx, receiver, salt) = if *owner == 0 { (a, b, &mut *b_ecu, 0x1357_9BDF) } else { (b, a, &mut *a_ecu, 0x2468_ACE0) };
        let tx_frame = tx.frames.iter().find(|f| f.tx && f.id == *id && f.dlc as usize == payload.len()).ok_or("ECU 发送了未配置的 CAN 帧")?;
        let expected = expected_payload(tx_frame, tx, salt);
        if *payload != expected { return Err(format!("CAN 位向量不匹配：id={id} 预期={expected:02X?} 实际={payload:02X?}")); }
        events.push(format!("总线 ID 优先级 id={id} payload={payload:02X?} 与独立位向量一致"));
        if let Some(rx_frame) = rx.frames.iter().find(|f| !f.tx && f.id == *id) {
            let hex = payload.iter().map(|b| format!("{b:02X}")).collect::<String>();
            receiver.query(&[format!("R {id} {} {hex}", payload.len())], rx.signals[0].id)?;
            for signal in rx.signals.iter().filter(|s| s.frame == rx_frame.path) {
                let (_, response) = receiver.query(&[], signal.id)?;
                let (actual, valid) = parse_value(&response)?;
                let expected = signal_value(payload, signal);
                if !valid || actual != expected { return Err(format!("接收信号 {} 不匹配：预期 {expected} valid=1，实际 {actual} valid={valid}", signal.id)); }
                events.push(format!("接收 signal={} value={} valid=1", signal.id, actual));
            }
            matched[*owner] += 1;
            if *owner == 0 { observed_rx_id = Some(*id); }
        }
    }
    if matched.iter().any(|count| *count == 0) { return Err("两个 ECU 必须各自发送至少一个由对端接收的信号帧".into()); }
    Ok(observed_rx_id.unwrap())
}

pub fn run(first: &Path, second: &Path) -> Result<RunReport, String> {
    let a = profile(first)?;
    let b = profile(second)?;
    if a.text == b.text { return Err("两个虚拟 ECU 配置相同；请分别生成不同项目".into()); }
    let binary_a = first.join(if cfg!(windows) { "ecu_host.exe" } else { "ecu_host" });
    let binary_b = second.join(if cfg!(windows) { "ecu_host.exe" } else { "ecu_host" });
    if !binary_a.is_file() || !binary_b.is_file() { return Err("两个生成目录均须先完成 C99 构建".into()); }
    let a_nvm = a.dtc.as_ref().map(|_| TempNvm::new());
    let b_nvm = b.dtc.as_ref().map(|_| TempNvm::new());
    let a_security = if a.diagnostic.as_ref().is_some_and(|d| d.security_enabled) { Some(SecurityFiles::new()?) } else { None };
    let b_security = if b.diagnostic.as_ref().is_some_and(|d| d.security_enabled) { Some(SecurityFiles::new()?) } else { None };
    let mut ecu_a = EcuProcess::start(&binary_a, a_nvm.as_ref().map(|state| state.path.as_path()), a_security.as_ref())?;
    let mut ecu_b = EcuProcess::start(&binary_b, b_nvm.as_ref().map(|state| state.path.as_path()), b_security.as_ref())?;
    let mut events = Vec::new();
    let outcome = (|| -> Result<(), String> {
        let max_period = a.frames.iter().chain(&b.frames).filter(|f| f.tx).map(|f| f.period).max().ok_or("没有周期发送帧")? as u64;
        prepare(&mut ecu_a, &a, 0x1357_9BDF)?;
        prepare(&mut ecu_b, &b, 0x2468_ACE0)?;
        tick(&mut ecu_a, &a, 0)?;
        tick(&mut ecu_b, &b, 0)?;
        let a_out = tick(&mut ecu_a, &a, max_period)?;
        let b_out = tick(&mut ecu_b, &b, max_period)?;
        let b_rx_id = route_and_check(&mut ecu_a, &mut ecu_b, &a, &b, &a_out, &b_out, &mut events)?;

        let max_timeout = a.frames.iter().chain(&b.frames).filter(|f| !f.tx).map(|f| f.timeout).max().ok_or("没有接收超时配置")? as u64;
        let timeout_tick = max_period + max_timeout + 1;
        let dropped_a = tick(&mut ecu_a, &a, timeout_tick)?;
        let dropped_b = tick(&mut ecu_b, &b, timeout_tick)?;
        events.push(format!("故障注入：丢弃 {} + {} 个发送帧", dropped_a.len(), dropped_b.len()));
        for (ecu, profile) in [(&mut ecu_a, &a), (&mut ecu_b, &b)] {
            for signal in profile.signals.iter().filter(|s| profile.frames.iter().any(|f| !f.tx && f.path == s.frame)) {
                let (_, response) = ecu.query(&[], signal.id)?;
                if parse_value(&response)?.1 { return Err(format!("接收信号 {} 在超时后仍有效", signal.id)); }
            }
        }
        events.push(format!("虚拟时钟推进至 {timeout_tick}ms，接收信号全部失效"));

        ecu_a.query(&["M 2".into()], a.signals[0].id)?;
        let off_out = tick(&mut ecu_a, &a, timeout_tick + max_period)?;
        if !off_out.is_empty() { return Err("BUS_OFF 状态仍然发出了 CAN 帧".into()); }
        events.push("BUS_OFF 状态禁止发送".into());
        ecu_a.query(&["M 1".into()], a.signals[0].id)?;
        let on_out = tick(&mut ecu_a, &a, timeout_tick + 2 * max_period)?;
        if on_out.is_empty() { return Err("控制器恢复 STARTED 后没有发送".into()); }
        events.push("STARTED 恢复周期发送".into());

        let configured_dlc = b.frames.iter().find(|f| !f.tx && f.id == b_rx_id).ok_or("接收帧丢失")?.dlc;
        let wrong_dlc = if configured_dlc == 1 { 2 } else { 1 };
        ecu_b.command(&format!("R {b_rx_id} {wrong_dlc} {}", "00".repeat(wrong_dlc as usize)))?;
        loop {
            let line = ecu_b.lines.recv_timeout(Duration::from_secs(5)).map_err(|e| format!("错误帧未响应: {e}"))?;
            if line == "E FRAME_DLC" { break; }
            if line.starts_with("E ") { return Err(format!("错误帧返回意外状态: {line}")); }
        }
        events.push("错误 DLC 被 CanIf 拒绝".into());
        Ok(())
    })();
    match outcome {
        Ok(()) => Ok(RunReport { passed: true, log: "双 ECU CAN 位向量、超时、BUS_OFF 与 DLC 错误路径通过".into(), events }),
        Err(error) => Ok(RunReport { passed: false, log: error, events }),
    }
}

fn hex_payload(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes { write!(&mut text, "{byte:02X}").expect("writing to a String"); }
    text
}

fn diagnostic_frames(
    lines: &[String], expected: &[Vec<u8>], profile: &Profile, diagnostic: &DiagnosticProfile, salt: u32,
) -> Result<(), String> {
    let mut matched = 0;
    for line in lines {
        let (id, payload) = parse_frame(line)?;
        if id == diagnostic.response_id {
            if expected.get(matched) != Some(&payload) {
                return Err(format!("诊断响应报文不匹配: {line}；期望 {:?}", expected.get(matched)));
            }
            matched += 1;
        } else {
            let frame = profile.frames.iter().find(|frame| frame.tx && frame.id == id && frame.dlc as usize == payload.len())
                .ok_or_else(|| format!("诊断测试收到未配置的报文: {line}"))?;
            if payload != expected_payload(frame, profile, salt) {
                return Err(format!("周期 CAN 帧载荷不匹配: {line}"));
            }
        }
    }
    if matched != expected.len() { return Err(format!("诊断响应数量错误: 收到 {matched}，期望 {}", expected.len())); }
    Ok(())
}

fn diagnostic_request(ecu: &mut EcuProcess, fence: u16, command: String) -> Result<Vec<String>, String> {
    ecu.query(&[command], fence).map(|(frames, _)| frames)
}

fn diagnostic_error(ecu: &mut EcuProcess, command: String, expected: &str) -> Result<(), String> {
    ecu.command(&command)?;
    let line = ecu.lines.recv_timeout(Duration::from_secs(5)).map_err(|e| format!("诊断错误无响应: {e}"))?;
    if line == format!("E {expected}") { Ok(()) } else { Err(format!("诊断错误状态不匹配: {line}，期望 E {expected}")) }
}

fn security_key(seed: &[u8]) -> [u8; 16] {
    let mut inner = [0x36u8; 64];
    let mut outer = [0x5cu8; 64];
    for i in 0..32 { inner[i] ^= 0x5a; outer[i] ^= 0x5a; }
    let mut hash = Sha256::new();
    hash.update(inner);
    hash.update(b"AUTOSAR-HOST-SECURITY-v1");
    hash.update(seed);
    let digest = hash.finalize();
    let mut hash = Sha256::new();
    hash.update(outer);
    hash.update(digest);
    let digest = hash.finalize();
    let mut key = [0u8; 16];
    key.copy_from_slice(&digest[..16]);
    key
}

fn send_payload(ecu: &mut EcuProcess, fence: u16, request: u32, payload: &[u8]) -> Result<Vec<String>, String> {
    if payload.len() <= 7 {
        let mut frame = vec![payload.len() as u8];
        frame.extend_from_slice(payload);
        return diagnostic_request(ecu, fence, format!("R {request} {} {}", frame.len(), hex_payload(&frame)));
    }
    let mut first = vec![0x10 | ((payload.len() >> 8) as u8 & 0x0f), payload.len() as u8];
    first.extend_from_slice(&payload[..6]);
    let flow = diagnostic_request(ecu, fence, format!("R {request} {} {}", first.len(), hex_payload(&first)))?;
    if flow.len() != 1 || parse_frame(&flow[0])?.1 != [0x30, 0x00, 0x00] {
        return Err("多帧诊断请求未得到 CTS 流控".into());
    }
    let mut result = Vec::new();
    for (index, chunk) in payload[6..].chunks(7).enumerate() {
        let mut frame = vec![0x20 | ((index as u8 + 1) & 0x0f)];
        frame.extend_from_slice(chunk);
        result = diagnostic_request(ecu, fence, format!("R {request} {} {}", frame.len(), hex_payload(&frame)))?;
    }
    Ok(result)
}

fn security_seed(ecu: &mut EcuProcess, profile: &Profile, diagnostic: &DiagnosticProfile, salt: u32) -> Result<[u8; 16], String> {
    let fence = profile.signals[0].id;
    let first = send_payload(ecu, fence, diagnostic.request_id, &[0x27, 0x01])?;
    let responses: Vec<_> = first.iter().filter_map(|line| parse_frame(line).ok())
        .filter(|(id, _)| *id == diagnostic.response_id).collect();
    if responses.len() != 1 || responses[0].1.len() != 8 || responses[0].1[..2] != [0x10, 0x12] {
        return Err(format!("0x27 未返回 18 字节多帧 seed 首帧: {first:?}"));
    }
    let mut payload = responses[0].1[2..].to_vec();
    let rest = diagnostic_request(ecu, fence, format!("R {} 3 300000", diagnostic.request_id))?;
    let mut sequence = 1u8;
    for line in &rest {
        let (id, frame) = parse_frame(line)?;
        if id == diagnostic.response_id {
            if frame.first() != Some(&(0x20 | sequence)) { return Err("0x27 seed 响应连续帧序号错误".into()); }
            payload.extend_from_slice(&frame[1..]);
            sequence += 1;
        }
    }
    payload.truncate(18);
    if sequence != 3 || payload.len() != 18 || payload[..2] != [0x67, 0x01] {
        return Err("0x27 seed 响应载荷错误".into());
    }
    diagnostic_frames(&first, &[responses[0].1.clone()], profile, diagnostic, salt)?;
    let mut seed = [0u8; 16];
    seed.copy_from_slice(&payload[2..]);
    Ok(seed)
}

fn security_unlock(ecu: &mut EcuProcess, profile: &Profile, diagnostic: &DiagnosticProfile, salt: u32) -> Result<[u8; 16], String> {
    let seed = security_seed(ecu, profile, diagnostic, salt)?;
    let key = security_key(&seed);
    let mut key_request = vec![0x27, 0x02];
    key_request.extend_from_slice(&key);
    let result = send_payload(ecu, profile.signals[0].id, diagnostic.request_id, &key_request)?;
    diagnostic_frames(&result, &[vec![0x02, 0x67, 0x02]], profile, diagnostic, salt)?;
    Ok(key)
}

pub fn run_diagnostic(dir: &Path) -> Result<RunReport, String> {
    let profile = profile(dir)?;
    let diagnostic = profile.diagnostic.as_ref().ok_or("生成配置不含诊断连接")?;
    if diagnostic.signal_ids.is_empty() || diagnostic.signal_ids.len() > 8 || diagnostic.request_id == diagnostic.response_id
        || diagnostic.s3_ms < 5000 || diagnostic.n_bs_ms == 0 || diagnostic.n_cr_ms == 0 {
        return Err("诊断清单超出支持范围".into());
    }
    let binary = dir.join(if cfg!(windows) { "ecu_host.exe" } else { "ecu_host" });
    if !binary.is_file() { return Err("诊断虚拟 ECU 尚未完成 C99 构建".into()); }
    let salt = 0x1357_9BDF;
    let initial_nvm = profile.dtc.as_ref().map(|_| TempNvm::new());
    let security = if diagnostic.security_enabled { Some(SecurityFiles::new()?) } else { None };
    let mut ecu = EcuProcess::start(&binary, initial_nvm.as_ref().map(|state| state.path.as_path()), security.as_ref())?;
    let mut events = Vec::new();
    let outcome = (|| -> Result<(), String> {
        let fence = profile.signals[0].id;
        let request = diagnostic.request_id;
        let did = diagnostic.did;
        let did_request = format!("R {request} 4 0322{did:04X}");
        let mut expected_did = vec![0x62, (did >> 8) as u8, did as u8];
        for (index, id) in diagnostic.signal_ids.iter().enumerate() {
            if diagnostic.signal_ids[..index].contains(id) { return Err("诊断清单重复引用信号".into()); }
            let signal = profile.signals.iter().find(|signal| signal.id == *id).ok_or("诊断清单引用不存在的信号")?;
            if signal.length != 32 || !profile.frames.iter().any(|frame| frame.tx && frame.path == signal.frame) {
                return Err("诊断 DID 只能引用 32 位发送信号".into());
            }
            expected_did.extend_from_slice(&value(signal, salt).to_be_bytes());
        }
        prepare(&mut ecu, &profile, salt)?;
        let before_session = diagnostic_request(&mut ecu, fence, did_request.clone())?;
        diagnostic_frames(&before_session, &[vec![0x03, 0x7F, 0x22, 0x31]], &profile, diagnostic, salt)?;
        let before_write = diagnostic_request(&mut ecu, fence, format!("R {request} 4 032E{did:04X}"))?;
        diagnostic_frames(&before_write, &[vec![0x03, 0x7F, 0x2E, if diagnostic.write_enabled { 0x31 } else { 0x11 }]],
            &profile, diagnostic, salt)?;
        let rid = diagnostic.reset_routine_id.unwrap_or(0xF001);
        let before_routine = diagnostic_request(&mut ecu, fence, format!("R {request} 5 043101{rid:04X}"))?;
        diagnostic_frames(&before_routine, &[vec![0x03, 0x7F, 0x31,
            if diagnostic.reset_routine_id.is_some() { 0x31 } else { 0x11 }]], &profile, diagnostic, salt)?;
        events.push("默认会话拒绝受限 DID".into());

        let session = diagnostic_request(&mut ecu, fence, format!("R {request} 3 021003"))?;
        diagnostic_frames(&session, &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]], &profile, diagnostic, salt)?;
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
            let mut ff = vec![0x10 | ((expected_did.len() >> 8) as u8 & 0x0F), expected_did.len() as u8];
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
            events.push(format!("实时 DID {} 字节多帧响应及流控通过", expected_did.len()));

            let pending = diagnostic_request(&mut ecu, fence, did_request.clone())?;
            let mut ff = vec![0x10 | ((expected_did.len() >> 8) as u8 & 0x0F), expected_did.len() as u8];
            ff.extend_from_slice(&expected_did[..6]);
            diagnostic_frames(&pending, &[ff], &profile, diagnostic, salt)?;
            diagnostic_error(&mut ecu, format!("T {}", diagnostic.n_bs_ms as u64 + 1), "TP_TIMEOUT")?;
            events.push("N_Bs 流控超时终止发送".into());
        }

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
        diagnostic_frames(&heartbeat, &[vec![0x02, 0x7E, 0x00]], &profile, diagnostic, salt)?;
        events.push("接收序号错误、N_Cr 超时与后续请求恢复通过".into());

        let session = diagnostic_request(&mut ecu, fence, format!("R {request} 3 021003"))?;
        diagnostic_frames(&session, &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]], &profile, diagnostic, salt)?;
        let expiry = next_time + diagnostic.s3_ms as u64 + 1;
        let timed = diagnostic_request(&mut ecu, fence, format!("T {expiry}"))?;
        diagnostic_frames(&timed, &[], &profile, diagnostic, salt)?;
        let expired = diagnostic_request(&mut ecu, fence, did_request)?;
        diagnostic_frames(&expired, &[vec![0x03, 0x7F, 0x22, 0x31]], &profile, diagnostic, salt)?;
        events.push("S3 超时恢复默认会话".into());
        Ok(())
    })();
    let outcome = outcome.and_then(|()| match &profile.dtc {
        Some(dtc) => verify_persistent_dtc(&binary, &profile, diagnostic, dtc, salt, security.as_ref(), &mut events),
        None => Ok(()),
    });
    let outcome = outcome.and_then(|()| {
        if diagnostic.write_enabled {
            verify_writable_did(&binary, &profile, diagnostic, salt, security.as_ref(), &mut events)
        } else {
            Ok(())
        }
    });
    let outcome = outcome.and_then(|()| {
        if diagnostic.security_enabled {
            verify_security(&binary, &profile, diagnostic, salt, &mut events)
        } else { Ok(()) }
    });
    match outcome {
        Ok(()) => {
            let mut log = if profile.dtc.is_some() {
                "独立测试器验证 CAN 诊断会话、实时 DID、流控及 Dem/NvM DTC 跨进程保持".to_owned()
            } else {
                "独立测试器验证 CAN 诊断会话、实时 DID、传输流控与故障恢复".to_owned()
            };
            if diagnostic.write_enabled { log.push_str("；扩展会话 0x2E 易失写入"); }
            if diagnostic.reset_routine_id.is_some() { log.push_str("；0x31 恢复 DID 初值例程"); }
            if diagnostic.security_enabled { log.push_str("；0x27 单级安全访问与受限操作"); }
            Ok(RunReport { passed: true, log, events })
        }
        Err(error) => Ok(RunReport { passed: false, log: error, events }),
    }
}

fn verify_persistent_dtc(
    binary: &Path, profile: &Profile, diagnostic: &DiagnosticProfile, dtc: &DtcProfile,
    salt: u32, security: Option<&SecurityFiles>, events: &mut Vec<String>,
) -> Result<(), String> {
    let state = TempNvm::new();
    let fence = profile.signals[0].id;
    let request = diagnostic.request_id;
    let read_dtc = format!("R {request} 4 03190208");
    let count_dtc = format!("R {request} 4 03190108");
    let counted = |count| vec![0x06, 0x59, 0x01, 0x7F, 0x01, 0x00, count];
    let empty = vec![0x03, 0x59, 0x02, 0x7F];
    let reported = |status| vec![0x07, 0x59, 0x02, 0x7F,
        (dtc.code >> 16) as u8, (dtc.code >> 8) as u8, dtc.code as u8, status];
    {
        let mut ecu = EcuProcess::start(binary, Some(&state.path), security)?;
        prepare(&mut ecu, profile, salt)?;
        let before = diagnostic_request(&mut ecu, fence, read_dtc.clone())?;
        diagnostic_frames(&before, &[empty.clone()], profile, diagnostic, salt)?;
        let count = diagnostic_request(&mut ecu, fence, count_dtc.clone())?;
        diagnostic_frames(&count, &[counted(0)], profile, diagnostic, salt)?;
        let session = diagnostic_request(&mut ecu, fence, format!("R {request} 3 021003"))?;
        diagnostic_frames(&session, &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]], profile, diagnostic, salt)?;
        if diagnostic.security_enabled { security_unlock(&mut ecu, profile, diagnostic, salt)?; }
        let off = diagnostic_request(&mut ecu, fence, format!("R {request} 3 028502"))?;
        diagnostic_frames(&off, &[vec![0x02, 0xC5, 0x02]], profile, diagnostic, salt)?;
        let received = diagnostic_request(&mut ecu, fence,
            format!("R {} {} {}", dtc.id, dtc.dlc, "00".repeat(dtc.dlc as usize)))?;
        diagnostic_frames(&received, &[], profile, diagnostic, salt)?;
        let timed = diagnostic_request(&mut ecu, fence, format!("T {}", dtc.timeout as u64 + 1))?;
        diagnostic_frames(&timed, &[], profile, diagnostic, salt)?;
        let suppressed = diagnostic_request(&mut ecu, fence, read_dtc.clone())?;
        diagnostic_frames(&suppressed, &[empty.clone()], profile, diagnostic, salt)?;
        let count = diagnostic_request(&mut ecu, fence, count_dtc.clone())?;
        diagnostic_frames(&count, &[counted(0)], profile, diagnostic, salt)?;
        let on = diagnostic_request(&mut ecu, fence, format!("R {request} 3 028501"))?;
        diagnostic_frames(&on, &[vec![0x02, 0xC5, 0x01]], profile, diagnostic, salt)?;
        let received = diagnostic_request(&mut ecu, fence,
            format!("R {} {} {}", dtc.id, dtc.dlc, "00".repeat(dtc.dlc as usize)))?;
        diagnostic_frames(&received, &[], profile, diagnostic, salt)?;
        let timed = diagnostic_request(&mut ecu, fence, format!("T {}", 2 * (dtc.timeout as u64 + 1)))?;
        diagnostic_frames(&timed, &[], profile, diagnostic, salt)?;
        let failed = diagnostic_request(&mut ecu, fence, read_dtc.clone())?;
        diagnostic_frames(&failed, &[reported(0x2F)], profile, diagnostic, salt)?;
        let count = diagnostic_request(&mut ecu, fence, count_dtc.clone())?;
        diagnostic_frames(&count, &[counted(1)], profile, diagnostic, salt)?;
    }
    events.push("接收帧超时产生真实 Dem DTC，进程结束前写入 NvM".into());
    events.push("0x85/0x02 禁用 DTC 设置时 Rx 超时不记录故障；0x85/0x01 恢复后新超时写入 Dem/NvM".into());
    {
        let mut ecu = EcuProcess::start(binary, Some(&state.path), security)?;
        prepare(&mut ecu, profile, salt)?;
        let recovered = diagnostic_request(&mut ecu, fence, read_dtc.clone())?;
        diagnostic_frames(&recovered, &[reported(0x6D)], profile, diagnostic, salt)?;
        let count = diagnostic_request(&mut ecu, fence, count_dtc.clone())?;
        diagnostic_frames(&count, &[counted(1)], profile, diagnostic, salt)?;
        let passed = diagnostic_request(&mut ecu, fence,
            format!("R {} {} {}", dtc.id, dtc.dlc, "00".repeat(dtc.dlc as usize)))?;
        diagnostic_frames(&passed, &[], profile, diagnostic, salt)?;
        let recovered_pass = diagnostic_request(&mut ecu, fence, read_dtc.clone())?;
        diagnostic_frames(&recovered_pass, &[reported(0x2C)], profile, diagnostic, salt)?;
        let count = diagnostic_request(&mut ecu, fence, count_dtc.clone())?;
        diagnostic_frames(&count, &[counted(1)], profile, diagnostic, salt)?;
        let denied = diagnostic_request(&mut ecu, fence, format!("R {request} 5 0414FFFFFF"))?;
        diagnostic_frames(&denied, &[vec![0x03, 0x7F, 0x14, 0x7F]], profile, diagnostic, salt)?;
        let session = diagnostic_request(&mut ecu, fence, format!("R {request} 3 021003"))?;
        diagnostic_frames(&session, &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]], profile, diagnostic, salt)?;
        if diagnostic.security_enabled { security_unlock(&mut ecu, profile, diagnostic, salt)?; }
        let wrong_group = diagnostic_request(&mut ecu, fence, format!("R {request} 5 0414000001"))?;
        diagnostic_frames(&wrong_group, &[vec![0x03, 0x7F, 0x14, 0x31]], profile, diagnostic, salt)?;
        let cleared = diagnostic_request(&mut ecu, fence, format!("R {request} 5 0414FFFFFF"))?;
        diagnostic_frames(&cleared, &[vec![0x01, 0x54]], profile, diagnostic, salt)?;
        let now_empty = diagnostic_request(&mut ecu, fence, read_dtc.clone())?;
        diagnostic_frames(&now_empty, &[empty.clone()], profile, diagnostic, salt)?;
        let count = diagnostic_request(&mut ecu, fence, count_dtc.clone())?;
        diagnostic_frames(&count, &[counted(0)], profile, diagnostic, salt)?;
    }
    {
        let mut ecu = EcuProcess::start(binary, Some(&state.path), security)?;
        let after_restart = diagnostic_request(&mut ecu, fence, read_dtc)?;
        diagnostic_frames(&after_restart, &[empty], profile, diagnostic, salt)?;
        let count = diagnostic_request(&mut ecu, fence, count_dtc)?;
        diagnostic_frames(&count, &[counted(0)], profile, diagnostic, salt)?;
    }
    events.push("重启后 DTC 保持、默认会话拒绝清除、扩展会话清除跨重启生效".into());
    events.push("0x19/0x01 状态掩码计数与 0x19/0x02 在超时、重启、清除前后一致".into());
    fs::write(&state.path, [0u8; 64]).map_err(|e| format!("故障注入 NvM 损坏失败: {e}"))?;
    let mut corrupt_command = Command::new(binary);
    corrupt_command.arg("--nvm").arg(&state.path);
    if let Some(files) = security {
        corrupt_command.arg("--security-key").arg(&files.key.path)
            .arg("--security-state").arg(&files.state.path);
    }
    let corrupt = corrupt_command.output()
        .map_err(|e| format!("无法验证 NvM 完整性拒绝路径: {e}"))?;
    if corrupt.status.success() || !String::from_utf8_lossy(&corrupt.stdout).lines().any(|line| line.trim_end_matches('\r') == "E NVM") {
        return Err("损坏的 NvM 状态未在启动时被明确拒绝".into());
    }
    events.push("双份 NvM 状态损坏在启动时被拒绝，未伪造空 DTC".into());
    Ok(())
}

fn expect_did_bytes(
    ecu: &mut EcuProcess, profile: &Profile, diagnostic: &DiagnosticProfile, salt: u32, data: &[u8],
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
    let mut ff = vec![0x10 | ((expected.len() >> 8) as u8 & 0x0F), expected.len() as u8];
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
    binary: &Path, profile: &Profile, diagnostic: &DiagnosticProfile, salt: u32,
    security: Option<&SecurityFiles>, events: &mut Vec<String>,
) -> Result<(), String> {
    let storage = profile.dtc.as_ref().map(|_| TempNvm::new());
    let mut ecu = EcuProcess::start(binary, storage.as_ref().map(|state| state.path.as_path()), security)?;
    let request = diagnostic.request_id;
    let fence = profile.signals[0].id;
    let did = diagnostic.did;
    prepare(&mut ecu, profile, salt)?;
    let session = diagnostic_request(&mut ecu, fence, format!("R {request} 3 021003"))?;
    diagnostic_frames(&session, &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]], profile, diagnostic, salt)?;
    if diagnostic.security_enabled { security_unlock(&mut ecu, profile, diagnostic, salt)?; }
    let other_did = if did == u16::MAX { did - 1 } else { did + 1 };
    let unknown = diagnostic_request(&mut ecu, fence, format!("R {request} 4 032E{other_did:04X}"))?;
    diagnostic_frames(&unknown, &[vec![0x03, 0x7F, 0x2E, 0x31]], profile, diagnostic, salt)?;
    let truncated = diagnostic_request(&mut ecu, fence, format!("R {request} 4 032E{did:04X}"))?;
    diagnostic_frames(&truncated, &[vec![0x03, 0x7F, 0x2E, 0x13]], profile, diagnostic, salt)?;

    let signals: Vec<_> = diagnostic.signal_ids.iter().map(|id|
        profile.signals.iter().find(|signal| signal.id == *id).ok_or("写入 DID 引用不存在的信号"))
        .collect::<Result<_, _>>()?;
    let written: Vec<u32> = signals.iter().map(|signal| !value(signal, salt)).collect();
    let mut record = vec![0x2E, (did >> 8) as u8, did as u8];
    for value in &written { record.extend_from_slice(&value.to_be_bytes()); }
    let positive = vec![0x03, 0x6E, (did >> 8) as u8, did as u8];
    if record.len() <= 7 {
        let mut sf = vec![record.len() as u8];
        sf.extend_from_slice(&record);
        let response = diagnostic_request(&mut ecu, fence, format!("R {request} {} {}", sf.len(), hex_payload(&sf)))?;
        diagnostic_frames(&response, &[positive], profile, diagnostic, salt)?;
    } else {
        let mut ff = vec![0x10 | ((record.len() >> 8) as u8 & 0x0F), record.len() as u8];
        ff.extend_from_slice(&record[..6]);
        let flow = diagnostic_request(&mut ecu, fence, format!("R {request} {} {}", ff.len(), hex_payload(&ff)))?;
        diagnostic_frames(&flow, &[vec![0x30, 0x00, 0x00]], profile, diagnostic, salt)?;
        let chunk_count = (record.len() - 6).div_ceil(7);
        for (index, chunk) in record[6..].chunks(7).enumerate() {
            let mut cf = vec![0x20 | ((index + 1) as u8 & 0x0F)];
            cf.extend_from_slice(chunk);
            let response = diagnostic_request(&mut ecu, fence, format!("R {request} {} {}", cf.len(), hex_payload(&cf)))?;
            let expected = if index + 1 == chunk_count { std::slice::from_ref(&positive) } else { &[] };
            diagnostic_frames(&response, expected, profile, diagnostic, salt)?;
        }
    }
    let mut bytes = Vec::with_capacity(written.len() * 4);
    for value in &written { bytes.extend_from_slice(&value.to_be_bytes()); }
    expect_did_bytes(&mut ecu, profile, diagnostic, salt, &bytes)?;
    for (signal, expected) in signals.iter().zip(&written) {
        let (frames, response) = ecu.query(&[], signal.id)?;
        if !frames.is_empty() || parse_value(&response)? != (*expected, true) {
            return Err(format!("0x2E 未更新实际 Com 信号 {}", signal.id));
        }
    }
    let monitored_frame = profile.frames.iter().find(|frame| frame.path == signals[0].frame && frame.tx)
        .ok_or("写入 DID 未关联发送帧")?;
    let periodic = tick(&mut ecu, profile, monitored_frame.period as u64)?;
    let mut observed = false;
    for line in periodic {
        let (id, payload) = parse_frame(&line)?;
        let frame = profile.frames.iter().find(|frame| frame.tx && frame.id == id && frame.dlc as usize == payload.len())
            .ok_or("0x2E 后 ECU 发送了未知 CAN 帧")?;
        for signal in profile.signals.iter().filter(|signal| signal.frame == frame.path) {
            let expected = diagnostic.signal_ids.iter().position(|id| *id == signal.id)
                .map(|index| written[index]).unwrap_or_else(|| value(signal, salt));
            if signal_value(&payload, signal) != expected {
                return Err(format!("0x2E 后 CAN 帧未携带更新的信号 {}", signal.id));
            }
        }
        observed |= frame.id == monitored_frame.id;
    }
    if !observed { return Err("0x2E 后未观察到被写入信号的周期 CAN 帧".into()); }
    events.push("扩展会话 0x2E 实际写入 Com 信号，0x22 与周期 CAN 均观察到新值".into());
    if let Some(rid) = diagnostic.reset_routine_id {
        let session = diagnostic_request(&mut ecu, fence, format!("R {request} 3 021003"))?;
        diagnostic_frames(&session, &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]], profile, diagnostic, salt)?;
        let other_rid = if rid == u16::MAX { rid - 1 } else { rid + 1 };
        let unknown = diagnostic_request(&mut ecu, fence, format!("R {request} 5 043101{other_rid:04X}"))?;
        diagnostic_frames(&unknown, &[vec![0x03, 0x7F, 0x31, 0x31]], profile, diagnostic, salt)?;
        let unsupported = diagnostic_request(&mut ecu, fence, format!("R {request} 5 043102{rid:04X}"))?;
        diagnostic_frames(&unsupported, &[vec![0x03, 0x7F, 0x31, 0x12]], profile, diagnostic, salt)?;
        let malformed = diagnostic_request(&mut ecu, fence, format!("R {request} 6 053101{rid:04X}00"))?;
        diagnostic_frames(&malformed, &[vec![0x03, 0x7F, 0x31, 0x13]], profile, diagnostic, salt)?;
        for (signal, expected) in signals.iter().zip(&written) {
            let (frames, response) = ecu.query(&[], signal.id)?;
            if !frames.is_empty() || parse_value(&response)? != (*expected, true) {
                return Err(format!("被拒绝的 0x31 请求改动了信号 {}", signal.id));
            }
        }
        if diagnostic.security_enabled { security_unlock(&mut ecu, profile, diagnostic, salt)?; }
        let restored = diagnostic_request(&mut ecu, fence, format!("R {request} 5 043101{rid:04X}"))?;
        diagnostic_frames(&restored, &[vec![0x04, 0x71, 0x01, (rid >> 8) as u8, rid as u8]], profile, diagnostic, salt)?;
        let mut initial = Vec::with_capacity(signals.len() * 4);
        for signal in &signals { initial.extend_from_slice(&signal.initial.to_be_bytes()); }
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
        if id == diagnostic.response_id { return Err("会话超时推进中出现未请求的诊断响应".into()); }
    }
    let denied = diagnostic_request(&mut ecu, fence, format!("R {request} 4 032E{did:04X}"))?;
    diagnostic_frames(&denied, &[vec![0x03, 0x7F, 0x2E, 0x31]], profile, diagnostic, salt)?;
    if let Some(rid) = diagnostic.reset_routine_id {
        let denied = diagnostic_request(&mut ecu, fence, format!("R {request} 5 043101{rid:04X}"))?;
        diagnostic_frames(&denied, &[vec![0x03, 0x7F, 0x31, 0x31]], profile, diagnostic, salt)?;
    }
    drop(ecu);

    let mut restarted = EcuProcess::start(binary, storage.as_ref().map(|state| state.path.as_path()), security)?;
    for signal in &signals {
        let (frames, response) = restarted.query(&[], signal.id)?;
        if !frames.is_empty() || parse_value(&response)? != (signal.initial, true) {
            return Err(format!("易失 DID 信号 {} 在重启后未恢复配置初值", signal.id));
        }
    }
    let session = diagnostic_request(&mut restarted, fence, format!("R {request} 3 021003"))?;
    diagnostic_frames(&session, &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]], profile, diagnostic, salt)?;
    let mut initial = Vec::with_capacity(signals.len() * 4);
    for signal in &signals { initial.extend_from_slice(&signal.initial.to_be_bytes()); }
    expect_did_bytes(&mut restarted, profile, diagnostic, salt, &initial)?;
    events.push("S3 回默认会话拒绝写入；ECU 重启后 DID 恢复初值且未误称 NvM 持久化".into());
    Ok(())
}

fn verify_security(
    binary: &Path, profile: &Profile, diagnostic: &DiagnosticProfile, salt: u32,
    events: &mut Vec<String>,
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
            let expected = if !unlocked { vec![0x03, 0x7f, payload[0], 0x33] }
                else { match payload[0] {
                    0x2e => vec![0x03, 0x6e, payload[1], payload[2]],
                    0x31 => vec![0x04, 0x71, 0x01, payload[2], payload[3]],
                    0x14 => vec![0x01, 0x54],
                    0x85 => vec![0x02, 0xc5, 0x01],
                    _ => return Err("安全档案含未知受保护服务".into()),
                }};
            diagnostic_frames(&result, &[expected], profile, diagnostic, salt)?;
        }
        Ok(())
    };
    {
        let mut ecu = EcuProcess::start(binary, nvm.as_ref().map(|s| s.path.as_path()), Some(&files))?;
        let before_session = send_payload(&mut ecu, fence, request, &[0x27, 0x01])?;
        diagnostic_frames(&before_session, &[vec![0x03, 0x7f, 0x27, 0x7f]], profile, diagnostic, salt)?;
        let session = send_payload(&mut ecu, fence, request, &[0x10, 0x03])?;
        diagnostic_frames(&session, &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]], profile, diagnostic, salt)?;
        check_protected(&mut ecu, false)?;
        let out_of_order = send_payload(&mut ecu, fence, request, &[0x27, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])?;
        diagnostic_frames(&out_of_order, &[vec![0x03, 0x7f, 0x27, 0x24]], profile, diagnostic, salt)?;
        let used_key = security_unlock(&mut ecu, profile, diagnostic, salt)?;
        if security_seed(&mut ecu, profile, diagnostic, salt)? != [0u8; 16] {
            return Err("已解锁级别请求 seed 时未返回零 seed".into());
        }
        let mut replay = vec![0x27, 0x02];
        replay.extend_from_slice(&used_key);
        let rejected_replay = send_payload(&mut ecu, fence, request, &replay)?;
        diagnostic_frames(&rejected_replay, &[vec![0x03, 0x7f, 0x27, 0x24]], profile, diagnostic, salt)?;
        check_protected(&mut ecu, true)?;
        let repeated_session = send_payload(&mut ecu, fence, request, &[0x10, 0x03])?;
        diagnostic_frames(&repeated_session, &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]], profile, diagnostic, salt)?;
        check_protected(&mut ecu, false)?;
        for attempt in 1..=3 {
            let _seed = security_seed(&mut ecu, profile, diagnostic, salt)?;
            let mut wrong = vec![0x27, 0x02];
            wrong.resize(18, 0u8);
            let result = send_payload(&mut ecu, fence, request, &wrong)?;
            diagnostic_frames(&result, &[vec![0x03, 0x7f, 0x27,
                if attempt == 3 { 0x36 } else { 0x35 }]], profile, diagnostic, salt)?;
        }
        let delayed = send_payload(&mut ecu, fence, request, &[0x27, 0x01])?;
        diagnostic_frames(&delayed, &[vec![0x03, 0x7f, 0x27, 0x37]], profile, diagnostic, salt)?;
    }
    {
        let mut ecu = EcuProcess::start(binary, nvm.as_ref().map(|s| s.path.as_path()), Some(&files))?;
        let session = send_payload(&mut ecu, fence, request, &[0x10, 0x03])?;
        diagnostic_frames(&session, &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]], profile, diagnostic, salt)?;
        let delayed = send_payload(&mut ecu, fence, request, &[0x27, 0x01])?;
        diagnostic_frames(&delayed, &[vec![0x03, 0x7f, 0x27, 0x37]], profile, diagnostic, salt)?;
        let _ = tick(&mut ecu, profile, 5000)?;
        let session = send_payload(&mut ecu, fence, request, &[0x10, 0x03])?;
        diagnostic_frames(&session, &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]], profile, diagnostic, salt)?;
        security_unlock(&mut ecu, profile, diagnostic, salt)?;
        let _ = tick(&mut ecu, profile, 5000 + diagnostic.s3_ms as u64 + 1)?;
        let session = send_payload(&mut ecu, fence, request, &[0x10, 0x03])?;
        diagnostic_frames(&session, &[vec![0x06, 0x50, 0x03, 0x00, 0x32, 0x00, 0x32]], profile, diagnostic, salt)?;
        check_protected(&mut ecu, false)?;
    }
    fs::write(&files.state.path, [0u8; 16]).map_err(|e| format!("无法注入安全计数损坏: {e}"))?;
    let mut corrupt = Command::new(binary);
    if let Some(state) = &nvm { corrupt.arg("--nvm").arg(&state.path); }
    let output = corrupt.arg("--security-key").arg(&files.key.path)
        .arg("--security-state").arg(&files.state.path).output()
        .map_err(|e| format!("无法检查损坏安全状态的启动拒绝: {e}"))?;
    if output.status.success() || !String::from_utf8_lossy(&output.stdout).contains("E NVM") {
        return Err("损坏的安全计数文件未在启动时拒绝".into());
    }
    events.push("0x27 seed/key 解锁、受保护操作、错误 key 次数/延时、重启保持与 S3 复锁通过".into());
    events.push("损坏的安全失败计数文件在启动时被拒绝".into());
    Ok(())
}
