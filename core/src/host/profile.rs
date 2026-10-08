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

pub(super) fn profile(dir: &Path) -> Result<Profile, crate::message::LocalizedText> {
    let text = fs::read_to_string(dir.join("profile.txt")).map_err(
        |e| crate::product_message!("backend.host.profile.inventory_missing", "error" => e),
    )?;
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
                        .ok_or_else(|| crate::product_message!("backend.host.profile.id_missing"))?
                        .parse()
                        .map_err(|_| {
                            crate::product_message!("backend.host.profile.frame_id_invalid")
                        })?,
                    dlc: field("dlc=")
                        .ok_or_else(|| crate::product_message!("backend.host.profile.dlc_missing"))?
                        .parse()
                        .map_err(|_| crate::product_message!("backend.host.profile.dlc_invalid"))?,
                    tx: field("direction=") == Some("tx"),
                    period: field("period=")
                        .ok_or_else(|| {
                            crate::product_message!("backend.host.profile.period_missing")
                        })?
                        .parse()
                        .map_err(|_| {
                            crate::product_message!("backend.host.profile.period_invalid")
                        })?,
                    timeout: field("timeout=")
                        .ok_or_else(|| {
                            crate::product_message!("backend.host.profile.timeout_missing")
                        })?
                        .parse()
                        .map_err(|_| {
                            crate::product_message!("backend.host.profile.timeout_invalid")
                        })?,
                });
            }
            Some("SIGNAL") if cols.len() >= 6 => {
                let field = |name: &str| cols.iter().find_map(|c| c.strip_prefix(name));
                let (start, length) = field("bits=")
                    .ok_or_else(|| crate::product_message!("backend.host.profile.bits_missing"))?
                    .split_once(':')
                    .ok_or_else(|| crate::product_message!("backend.host.profile.bits_invalid"))?;
                signals.push(Signal {
                    id: cols[1].parse().map_err(|_| {
                        crate::product_message!("backend.host.profile.signal_id_invalid")
                    })?,
                    frame: field("frame=")
                        .ok_or_else(|| {
                            crate::product_message!("backend.host.profile.frame_missing")
                        })?
                        .into(),
                    start: start.parse().map_err(|_| {
                        crate::product_message!("backend.host.profile.start_invalid")
                    })?,
                    length: length.parse().map_err(|_| {
                        crate::product_message!("backend.host.profile.length_invalid")
                    })?,
                    initial: field("initial=")
                        .ok_or_else(|| {
                            crate::product_message!("backend.host.profile.initial_missing")
                        })?
                        .parse()
                        .map_err(|_| {
                            crate::product_message!("backend.host.profile.initial_invalid")
                        })?,
                });
            }
            Some("DIAGNOSTIC") if (cols.len() == 8) || (cols.len() == 9) => {
                let field = |name: &str| cols.iter().find_map(|column| column.strip_prefix(name));
                let ids = field("signals=")
                    .ok_or_else(|| {
                        crate::product_message!("backend.host.profile.did_signals_missing")
                    })?
                    .split(',')
                    .map(|id| {
                        id.parse::<u16>().map_err(|_| {
                            crate::product_message!(
                                "backend.host.profile.diagnostic_signal_invalid"
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let n_bs_ms = field("nbs=")
                    .ok_or_else(|| crate::product_message!("backend.host.profile.nbs_missing"))?
                    .parse()
                    .map_err(|_| crate::product_message!("backend.host.profile.nbs_invalid"))?;
                let n_as_ms = field("nas=")
                    .map(|value| {
                        value.parse().map_err(|_| {
                            crate::product_message!("backend.host.profile.nas_invalid")
                        })
                    })
                    .transpose()?
                    .unwrap_or(n_bs_ms);
                let config = DiagnosticProfile {
                    request_id: field("request=")
                        .ok_or_else(|| {
                            crate::product_message!("backend.host.profile.request_missing")
                        })?
                        .parse()
                        .map_err(|_| {
                            crate::product_message!("backend.host.profile.request_invalid")
                        })?,
                    response_id: field("response=")
                        .ok_or_else(|| {
                            crate::product_message!("backend.host.profile.response_missing")
                        })?
                        .parse()
                        .map_err(|_| {
                            crate::product_message!("backend.host.profile.response_invalid")
                        })?,
                    s3_ms: field("s3=")
                        .ok_or_else(|| crate::product_message!("backend.host.profile.s3_missing"))?
                        .parse()
                        .map_err(|_| crate::product_message!("backend.host.profile.s3_invalid"))?,
                    n_as_ms,
                    n_bs_ms,
                    n_cr_ms: field("ncr=")
                        .ok_or_else(|| crate::product_message!("backend.host.profile.ncr_missing"))?
                        .parse()
                        .map_err(|_| crate::product_message!("backend.host.profile.ncr_invalid"))?,
                    did: field("did=")
                        .ok_or_else(|| crate::product_message!("backend.host.profile.did_missing"))?
                        .parse()
                        .map_err(|_| crate::product_message!("backend.host.profile.did_invalid"))?,
                    signal_ids: ids,
                    write_enabled: false,
                    reset_routine_id: None,
                    security_enabled: false,
                };
                if diagnostic.replace(config).is_some() {
                    return Err(crate::product_message!(
                        "backend.host.profile.multiple_connections"
                    ));
                }
            }
            Some("DIAGNOSTIC") => {
                return Err(crate::product_message!(
                    "backend.host.profile.diagnostic_fields"
                ));
            }
            Some("WRITE_DID") if cols.len() == 2 => {
                let did = cols[1]
                    .strip_prefix("did=")
                    .ok_or_else(|| {
                        crate::product_message!("backend.host.profile.write_id_missing")
                    })?
                    .parse::<u16>()
                    .map_err(|_| {
                        crate::product_message!("backend.host.profile.write_id_invalid")
                    })?;
                if write_did.replace(did).is_some() {
                    return Err(crate::product_message!(
                        "backend.host.profile.multiple_write"
                    ));
                }
            }
            Some("WRITE_DID") => {
                return Err(crate::product_message!("backend.host.profile.write_fields"));
            }
            Some("RESET_ROUTINE") if cols.len() == 2 => {
                let id = cols[1]
                    .strip_prefix("id=")
                    .ok_or_else(|| crate::product_message!("backend.host.profile.rid_missing"))?
                    .parse::<u16>()
                    .map_err(|_| crate::product_message!("backend.host.profile.rid_invalid"))?;
                if reset_routine.replace(id).is_some() {
                    return Err(crate::product_message!(
                        "backend.host.profile.multiple_routines"
                    ));
                }
            }
            Some("RESET_ROUTINE") => {
                return Err(crate::product_message!(
                    "backend.host.profile.routine_fields"
                ));
            }
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
                    return Err(crate::product_message!(
                        "backend.host.profile.multiple_security"
                    ));
                }
                security = true;
            }
            Some("SECURITY") => {
                return Err(crate::product_message!(
                    "backend.host.profile.security_unsupported"
                ));
            }
            Some("DTC") if cols.len() == 6 => {
                let field = |name: &str| cols.iter().find_map(|column| column.strip_prefix(name));
                let config = DtcProfile {
                    code: field("code=")
                        .ok_or_else(|| {
                            crate::product_message!("backend.host.profile.dtc_code_missing")
                        })?
                        .parse()
                        .map_err(|_| {
                            crate::product_message!("backend.host.profile.dtc_code_invalid")
                        })?,
                    frame_index: field("frame=")
                        .ok_or_else(|| {
                            crate::product_message!("backend.host.profile.dtc_index_missing")
                        })?
                        .parse()
                        .map_err(|_| {
                            crate::product_message!("backend.host.profile.dtc_index_invalid")
                        })?,
                    id: field("id=")
                        .ok_or_else(|| {
                            crate::product_message!("backend.host.profile.dtc_id_missing")
                        })?
                        .parse()
                        .map_err(|_| {
                            crate::product_message!("backend.host.profile.dtc_id_invalid")
                        })?,
                    dlc: field("dlc=")
                        .ok_or_else(|| {
                            crate::product_message!("backend.host.profile.dtc_dlc_missing")
                        })?
                        .parse()
                        .map_err(|_| {
                            crate::product_message!("backend.host.profile.dtc_dlc_invalid")
                        })?,
                    timeout: field("timeout=")
                        .ok_or_else(|| {
                            crate::product_message!("backend.host.profile.dtc_timeout_missing")
                        })?
                        .parse()
                        .map_err(|_| {
                            crate::product_message!("backend.host.profile.dtc_timeout_invalid")
                        })?,
                };
                if dtc.replace(config).is_some() {
                    return Err(crate::product_message!("backend.host.profile.multiple_dtc"));
                }
            }
            Some("DTC") => return Err(crate::product_message!("backend.host.profile.dtc_fields")),
            _ => {}
        }
    }
    if frames.is_empty() || signals.is_empty() {
        return Err(crate::product_message!(
            "backend.host.profile.frames_missing"
        ));
    }
    if let Some(did) = write_did {
        let configured = diagnostic
            .as_mut()
            .ok_or_else(|| crate::product_message!("backend.host.profile.write_connection"))?;
        if configured.did != did {
            return Err(crate::product_message!(
                "backend.host.profile.write_mismatch"
            ));
        }
        configured.write_enabled = true;
    }
    if let Some(id) = reset_routine {
        let configured = diagnostic
            .as_mut()
            .ok_or_else(|| crate::product_message!("backend.host.profile.routine_connection"))?;
        if !configured.write_enabled {
            return Err(crate::product_message!(
                "backend.host.profile.writable_required"
            ));
        }
        configured.reset_routine_id = Some(id);
    }
    if security {
        let configured = diagnostic
            .as_mut()
            .ok_or_else(|| crate::product_message!("backend.host.profile.security_connection"))?;
        if !configured.write_enabled && dtc.is_none() {
            return Err(crate::product_message!(
                "backend.host.profile.security_operations"
            ));
        }
        configured.security_enabled = true;
    }
    if let Some(config) = &dtc {
        let frame = frames
            .get(config.frame_index)
            .ok_or_else(|| crate::product_message!("backend.host.profile.dtc_frame_missing"))?;
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
            return Err(crate::product_message!("backend.host.profile.dtc_mismatch"));
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
