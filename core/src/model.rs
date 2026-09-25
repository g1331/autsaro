use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Issue {
    pub fn error(code: &str, message: impl Into<String>, path: Option<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            message: message.into(),
            path,
            file: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileView {
    pub path: String,
    pub readonly: bool,
    pub retained_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameView {
    pub path: String,
    pub name: String,
    pub id: u32,
    pub dlc: u8,
    pub direction: Direction,
    pub period_ms: Option<u32>,
    pub timeout_ms: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Tx,
    Rx,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalView {
    pub path: String,
    pub name: String,
    pub frame_path: String,
    pub start_bit: u8,
    pub length: u8,
    pub initial_value: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceView {
    pub name: String,
    pub files: Vec<FileView>,
    pub frames: Vec<FrameView>,
    pub signals: Vec<SignalView>,
    pub issues: Vec<Issue>,
    pub dirty: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationReport {
    pub output_directory: String,
    pub files: Vec<String>,
    pub issues: Vec<Issue>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildReport {
    pub binary_path: String,
    pub log: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReport {
    pub passed: bool,
    pub log: String,
    pub events: Vec<String>,
}

pub fn validate_profile(frames: &[FrameView], signals: &[SignalView]) -> Vec<Issue> {
    let mut issues = Vec::new();
    if frames.len() > 32 || signals.len() > 64 {
        issues.push(Issue::error("PROFILE_LIMIT", "最多支持 32 个帧、64 个信号", None));
    }
    for (i, frame) in frames.iter().enumerate() {
        if frame.id > 0x7ff || !(1..=8).contains(&frame.dlc) {
            issues.push(Issue::error("CAN_FRAME_RANGE", "仅支持 11 位 CAN 标识符与 DLC 1–8", Some(frame.path.clone())));
        }
        if frames[..i].iter().any(|prior| prior.id == frame.id) {
            issues.push(Issue::error("CAN_ID_DUPLICATE", "同一 ECU 的 CAN 标识符不能重复", Some(frame.path.clone())));
        }
        match frame.direction {
            Direction::Tx if frame.period_ms.unwrap_or(0) == 0 || frame.timeout_ms.is_some() => {
                issues.push(Issue::error("TX_PERIOD", "发送帧必须有正周期，不能设置接收超时", Some(frame.path.clone())));
            }
            Direction::Rx if frame.timeout_ms.unwrap_or(0) == 0 || frame.period_ms.is_some() => {
                issues.push(Issue::error("RX_TIMEOUT", "接收帧必须有正超时，不能设置发送周期", Some(frame.path.clone())));
            }
            _ => {}
        }
        let mut occupied = 0u64;
        for signal in signals.iter().filter(|signal| signal.frame_path == frame.path) {
            let end = u32::from(signal.start_bit) + u32::from(signal.length);
            if !(1..=32).contains(&signal.length) || end > u32::from(frame.dlc) * 8 {
                issues.push(Issue::error("SIGNAL_RANGE", "信号须为帧内 1–32 位无符号小端原始值", Some(signal.path.clone())));
                continue;
            }
            let mask = ((1u64 << signal.length) - 1) << signal.start_bit;
            if occupied & mask != 0 || (u64::from(signal.initial_value) >> signal.length) != 0 {
                issues.push(Issue::error("SIGNAL_LAYOUT", "信号位重叠或初始值超出位宽", Some(signal.path.clone())));
            }
            occupied |= mask;
        }
    }
    for signal in signals {
        if !frames.iter().any(|frame| frame.path == signal.frame_path) {
            issues.push(Issue::error("MISSING_FRAME", "信号引用的帧不存在", Some(signal.path.clone())));
        }
    }
    issues
}
