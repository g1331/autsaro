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
pub struct DiagnosticView {
    pub path: String,
    pub request_id: u32,
    pub response_id: u32,
    pub s3_ms: u32,
    pub n_bs_ms: u32,
    pub n_cr_ms: u32,
    pub did: u16,
    pub signal_paths: Vec<String>,
    pub write_enabled: bool,
    pub reset_routine_id: Option<u16>,
    pub dtc: Option<DtcView>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DtcView {
    pub path: String,
    pub code: u32,
    pub monitor_frame_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceView {
    pub name: String,
    pub files: Vec<FileView>,
    pub frames: Vec<FrameView>,
    pub signals: Vec<SignalView>,
    pub diagnostic: Option<DiagnosticView>,
    pub issues: Vec<Issue>,
    pub dirty: bool,
    pub routine_migration_pending: bool,
    pub pdu_migration_pending: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationReport {
    pub output_directory: String,
    pub previous_output_directory: Option<String>,
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

pub fn validate_diagnostic(diagnostic: &DiagnosticView, frames: &[FrameView], signals: &[SignalView]) -> Vec<Issue> {
    let mut issues = Vec::new();
    let path = Some(diagnostic.path.clone());
    if diagnostic.request_id > 0x7ff || diagnostic.response_id > 0x7ff ||
        diagnostic.request_id == diagnostic.response_id ||
        frames.iter().any(|frame| frame.id == diagnostic.request_id || frame.id == diagnostic.response_id) {
        issues.push(Issue::error("DIAG_CAN_ID", "诊断请求/响应须使用不与 Com 帧冲突的不同 11 位 CAN 标识符", path.clone()));
    }
    if !(5000..=i32::MAX as u32).contains(&diagnostic.s3_ms) ||
        !(1..=i32::MAX as u32).contains(&diagnostic.n_bs_ms) ||
        !(1..=i32::MAX as u32).contains(&diagnostic.n_cr_ms) {
        issues.push(Issue::error("DIAG_TIMING", "S3 至少 5000 ms，N_Bs/N_Cr 须为正毫秒，计时器不得超过 2^31-1 ms", path.clone()));
    }
    if diagnostic.did == 0xf186 {
        issues.push(Issue::error("DIAG_DID", "DID 0xF186 保留给活动会话", path.clone()));
    }
    if !(1..=8).contains(&diagnostic.signal_paths.len()) {
        issues.push(Issue::error("DIAG_SIGNAL_COUNT", "诊断 DID 须绑定 1–8 个信号", path.clone()));
    }
    for (index, signal_path) in diagnostic.signal_paths.iter().enumerate() {
        if diagnostic.signal_paths[..index].contains(signal_path) {
            issues.push(Issue::error("DIAG_SIGNAL_DUPLICATE", "诊断 DID 信号不能重复", Some(signal_path.clone())));
        }
        let valid = signals.iter().find(|signal| &signal.path == signal_path).is_some_and(|signal|
            signal.length == 32 && frames.iter().any(|frame| frame.path == signal.frame_path && matches!(frame.direction, Direction::Tx)));
        if !valid {
            issues.push(Issue::error("DIAG_SIGNAL", "诊断 DID 只支持存在的 32 位 Tx Com 信号", Some(signal_path.clone())));
        }
    }
    if diagnostic.reset_routine_id.is_some() && !diagnostic.write_enabled {
        issues.push(Issue::error("RESET_ROUTINE_WRITE", "重置例程要求 DID 可通过 0x2E 写入", path.clone()));
    }
    if let Some(dtc) = &diagnostic.dtc {
        if !(0x100..=0xfffffe).contains(&dtc.code) {
            issues.push(Issue::error("DTC_RANGE", "UDS DTC 须为 0x000100–0xFFFFFE，低于 0x100 或全 DTC 组代码不可用", Some(dtc.path.clone())));
        }
        if !frames.iter().any(|frame| frame.path == dtc.monitor_frame_path &&
            matches!(frame.direction, Direction::Rx) && frame.timeout_ms.unwrap_or(0) > 0 &&
            signals.iter().any(|signal| signal.frame_path == frame.path)) {
            issues.push(Issue::error("DTC_MONITOR", "DTC 须绑定至少含一个信号且有正超时的 Rx CAN 帧", Some(dtc.monitor_frame_path.clone())));
        }
    }
    issues
}
