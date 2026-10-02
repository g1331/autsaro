use std::fs;
use std::path::Path;

#[derive(Clone)]
pub(super) struct Signal {
    pub(super) id: u16,
    pub(super) frame: String,
    pub(super) start: u8,
    pub(super) length: u8,
    pub(super) initial: u32,
}
#[derive(Clone)]
pub(super) struct Frame {
    pub(super) path: String,
    pub(super) id: u32,
    pub(super) dlc: u8,
    pub(super) tx: bool,
    pub(super) period: u32,
    pub(super) timeout: u32,
}
pub(super) struct DiagnosticProfile {
    pub(super) request_id: u32,
    pub(super) response_id: u32,
    pub(super) s3_ms: u32,
    pub(super) n_as_ms: u32,
    pub(super) n_bs_ms: u32,
    pub(super) n_cr_ms: u32,
    pub(super) did: u16,
    pub(super) signal_ids: Vec<u16>,
    pub(super) write_enabled: bool,
    pub(super) reset_routine_id: Option<u16>,
    pub(super) security_enabled: bool,
}
pub(super) struct DtcProfile {
    pub(super) code: u32,
    frame_index: usize,
    pub(super) id: u32,
    pub(super) dlc: u8,
    pub(super) timeout: u32,
}
pub(super) struct Profile {
    pub(super) frames: Vec<Frame>,
    pub(super) signals: Vec<Signal>,
    pub(super) diagnostic: Option<DiagnosticProfile>,
    pub(super) dtc: Option<DtcProfile>,
    pub(super) text: String,
}

pub(super) fn profile(dir: &Path) -> Result<Profile, String> {
    let text =
        fs::read_to_string(dir.join("profile.txt")).map_err(|e| format!("配置清单缺失: {e}"))?;
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
                    id: field("id=")
                        .ok_or("清单缺少 id")?
                        .parse()
                        .map_err(|_| "帧 ID 无效")?,
                    dlc: field("dlc=")
                        .ok_or("清单缺少 dlc")?
                        .parse()
                        .map_err(|_| "DLC 无效")?,
                    tx: field("direction=") == Some("tx"),
                    period: field("period=")
                        .ok_or("清单缺少周期")?
                        .parse()
                        .map_err(|_| "周期无效")?,
                    timeout: field("timeout=")
                        .ok_or("清单缺少超时")?
                        .parse()
                        .map_err(|_| "超时无效")?,
                });
            }
            Some("SIGNAL") if cols.len() >= 6 => {
                let field = |name: &str| cols.iter().find_map(|c| c.strip_prefix(name));
                let (start, length) = field("bits=")
                    .ok_or("缺少信号位段")?
                    .split_once(':')
                    .ok_or("位段无效")?;
                signals.push(Signal {
                    id: cols[1].parse().map_err(|_| "信号 ID 无效")?,
                    frame: field("frame=").ok_or("缺少信号帧引用")?.into(),
                    start: start.parse().map_err(|_| "起始位无效")?,
                    length: length.parse().map_err(|_| "长度无效")?,
                    initial: field("initial=")
                        .ok_or("缺少信号初值")?
                        .parse()
                        .map_err(|_| "信号初值无效")?,
                });
            }
            Some("DIAGNOSTIC") if (cols.len() == 8) || (cols.len() == 9) => {
                let field = |name: &str| cols.iter().find_map(|column| column.strip_prefix(name));
                let ids = field("signals=")
                    .ok_or("诊断清单缺少 DID 信号")?
                    .split(',')
                    .map(|id| id.parse::<u16>().map_err(|_| "诊断信号 ID 无效".to_owned()))
                    .collect::<Result<Vec<_>, _>>()?;
                let n_bs_ms = field("nbs=")
                    .ok_or("诊断 N_Bs 缺失")?
                    .parse()
                    .map_err(|_| "诊断 N_Bs 无效")?;
                let n_as_ms = field("nas=")
                    .map(|value| value.parse().map_err(|_| "诊断 N_As 无效"))
                    .transpose()?
                    .unwrap_or(n_bs_ms);
                let config = DiagnosticProfile {
                    request_id: field("request=")
                        .ok_or("诊断请求 ID 缺失")?
                        .parse()
                        .map_err(|_| "诊断请求 ID 无效")?,
                    response_id: field("response=")
                        .ok_or("诊断响应 ID 缺失")?
                        .parse()
                        .map_err(|_| "诊断响应 ID 无效")?,
                    s3_ms: field("s3=")
                        .ok_or("诊断 S3 缺失")?
                        .parse()
                        .map_err(|_| "诊断 S3 无效")?,
                    n_as_ms,
                    n_bs_ms,
                    n_cr_ms: field("ncr=")
                        .ok_or("诊断 N_Cr 缺失")?
                        .parse()
                        .map_err(|_| "诊断 N_Cr 无效")?,
                    did: field("did=")
                        .ok_or("诊断 DID 缺失")?
                        .parse()
                        .map_err(|_| "诊断 DID 无效")?,
                    signal_ids: ids,
                    write_enabled: false,
                    reset_routine_id: None,
                    security_enabled: false,
                };
                if diagnostic.replace(config).is_some() {
                    return Err("诊断清单含多个连接".into());
                }
            }
            Some("DIAGNOSTIC") => return Err("诊断清单字段数量错误".into()),
            Some("WRITE_DID") if cols.len() == 2 => {
                let did = cols[1]
                    .strip_prefix("did=")
                    .ok_or("写入 DID 清单缺少编号")?
                    .parse::<u16>()
                    .map_err(|_| "写入 DID 编号无效")?;
                if write_did.replace(did).is_some() {
                    return Err("诊断清单含多个写入 DID".into());
                }
            }
            Some("WRITE_DID") => return Err("写入 DID 清单字段数量错误".into()),
            Some("RESET_ROUTINE") if cols.len() == 2 => {
                let id = cols[1]
                    .strip_prefix("id=")
                    .ok_or("例程清单缺少 RID")?
                    .parse::<u16>()
                    .map_err(|_| "例程 RID 无效")?;
                if reset_routine.replace(id).is_some() {
                    return Err("诊断清单含多个例程".into());
                }
            }
            Some("RESET_ROUTINE") => return Err("例程清单字段数量错误".into()),
            Some("SECURITY")
                if cols
                    == [
                        "SECURITY",
                        "level=1",
                        "seed=16",
                        "key=16",
                        "attempts=3",
                        "delay=5000",
                    ] =>
            {
                if security {
                    return Err("诊断清单含多个安全档案".into());
                }
                security = true;
            }
            Some("SECURITY") => return Err("安全档案清单不受支持".into()),
            Some("DTC") if cols.len() == 6 => {
                let field = |name: &str| cols.iter().find_map(|column| column.strip_prefix(name));
                let config = DtcProfile {
                    code: field("code=")
                        .ok_or("DTC 编码缺失")?
                        .parse()
                        .map_err(|_| "DTC 编码无效")?,
                    frame_index: field("frame=")
                        .ok_or("DTC 监控帧索引缺失")?
                        .parse()
                        .map_err(|_| "DTC 监控帧索引无效")?,
                    id: field("id=")
                        .ok_or("DTC 监控 CAN ID 缺失")?
                        .parse()
                        .map_err(|_| "DTC 监控 CAN ID 无效")?,
                    dlc: field("dlc=")
                        .ok_or("DTC 监控帧 DLC 缺失")?
                        .parse()
                        .map_err(|_| "DTC 监控帧 DLC 无效")?,
                    timeout: field("timeout=")
                        .ok_or("DTC 监控超时缺失")?
                        .parse()
                        .map_err(|_| "DTC 监控超时无效")?,
                };
                if dtc.replace(config).is_some() {
                    return Err("诊断清单含多个 DTC".into());
                }
            }
            Some("DTC") => return Err("DTC 清单字段数量错误".into()),
            _ => {}
        }
    }
    if frames.is_empty() || signals.is_empty() {
        return Err("生成配置缺少帧或信号".into());
    }
    if let Some(did) = write_did {
        let configured = diagnostic.as_mut().ok_or("写入 DID 缺少诊断连接")?;
        if configured.did != did {
            return Err("写入 DID 与诊断连接不一致".into());
        }
        configured.write_enabled = true;
    }
    if let Some(id) = reset_routine {
        let configured = diagnostic.as_mut().ok_or("例程缺少诊断连接")?;
        if !configured.write_enabled {
            return Err("恢复 DID 例程需要可写 DID".into());
        }
        configured.reset_routine_id = Some(id);
    }
    if security {
        let configured = diagnostic.as_mut().ok_or("安全档案缺少诊断连接")?;
        if !configured.write_enabled && dtc.is_none() {
            return Err("安全档案没有受保护操作".into());
        }
        configured.security_enabled = true;
    }
    if let Some(config) = &dtc {
        let frame = frames.get(config.frame_index).ok_or("DTC 监控帧不存在")?;
        if diagnostic.is_none()
            || config.code < 0x100
            || config.code >= 0xFFFFFF
            || frame.tx
            || frame.id != config.id
            || frame.dlc != config.dlc
            || frame.timeout == 0
            || frame.timeout != config.timeout
            || !signals.iter().any(|signal| signal.frame == frame.path)
        {
            return Err("DTC 清单与受支持的 Rx 超时监控配置不一致".into());
        }
    }
    Ok(Profile {
        frames,
        signals,
        diagnostic,
        dtc,
        text,
    })
}
