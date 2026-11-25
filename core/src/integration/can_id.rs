//! Classical CAN endpoint identity and CanIf receive policy.
use super::{graph::Graph, schedule::value};

pub(super) const EXTENDED: u32 = 0x8000_0000;
pub(super) const EXTENDED_MAX: u32 = 0x1fff_ffff;

pub(super) fn trigger_id(graph: &Graph, trigger: usize) -> Option<u32> {
    if !matches!(
        graph.text(trigger, "CAN-FD-FRAME-SUPPORT"),
        None | Some("false" | "0")
    ) {
        return None;
    }
    encode(
        graph.text(trigger, "IDENTIFIER"),
        graph.text(trigger, "CAN-ADDRESSING-MODE")?,
    )
}

pub(super) fn pdu_id(graph: &Graph, pdu: usize, receive: bool) -> Option<(u32, bool)> {
    let prefix = if receive { "CanIfRxPdu" } else { "CanIfTxPdu" };
    let (format, accepts_fd) = match value(graph, pdu, &format!("{prefix}CanIdType"), false)? {
        "STANDARD_CAN" => ("STANDARD", receive),
        "EXTENDED_CAN" => ("EXTENDED", receive),
        "STANDARD_NO_FD_CAN" if receive => ("STANDARD", false),
        "EXTENDED_NO_FD_CAN" if receive => ("EXTENDED", false),
        _ => return None,
    };
    Some((
        encode(value(graph, pdu, &format!("{prefix}CanId"), false), format)?,
        accepts_fd,
    ))
}

pub(super) fn encoded(id: u32, extended: bool) -> u32 {
    id | if extended { EXTENDED } else { 0 }
}

fn encode(value: Option<&str>, format: &str) -> Option<u32> {
    let id = u32::try_from(crate::definitions::unsigned_integer(value?)?).ok()?;
    match format {
        "STANDARD" if id <= 0x7ff => Some(id),
        "EXTENDED" if id <= EXTENDED_MAX => Some(id | EXTENDED),
        _ => None,
    }
}
