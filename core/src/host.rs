use crate::model::RunReport;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

#[derive(Clone)]
struct Signal { id: u16, frame: String, start: u8, length: u8 }
#[derive(Clone)]
struct Frame { path: String, id: u32, dlc: u8, tx: bool, period: u32, timeout: u32 }
struct Profile { frames: Vec<Frame>, signals: Vec<Signal>, text: String }

fn profile(dir: &Path) -> Result<Profile, String> {
    let text = fs::read_to_string(dir.join("profile.txt")).map_err(|e| format!("配置清单缺失: {e}"))?;
    let mut frames = Vec::new();
    let mut signals = Vec::new();
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
                signals.push(Signal { id: cols[1].parse().map_err(|_| "信号 ID 无效")?, frame: field("frame=").ok_or("缺少信号帧引用")?.into(), start: start.parse().map_err(|_| "起始位无效")?, length: length.parse().map_err(|_| "长度无效")? });
            }
            _ => {}
        }
    }
    if frames.is_empty() || signals.is_empty() { return Err("生成配置缺少帧或信号".into()); }
    Ok(Profile { frames, signals, text })
}

struct EcuProcess {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
}
impl EcuProcess {
    fn start(path: &Path) -> Result<Self, String> {
        let mut child = Command::new(path).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("无法启动 ECU {}: {e}", path.display()))?;
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
    let mut ecu_a = EcuProcess::start(&binary_a)?;
    let mut ecu_b = EcuProcess::start(&binary_b)?;
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
