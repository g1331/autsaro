//! CAN endpoint identity and CanIf receive policy.
use super::{graph::Graph, schedule::value};

pub(super) const EXTENDED: u32 = 0x8000_0000;
pub(super) const FD: u32 = 0x4000_0000;
pub(super) const EXTENDED_MAX: u32 = 0x1fff_ffff;

pub(super) fn trigger_id(graph: &Graph, trigger: usize) -> Option<u32> {
    let flag = match graph.text(trigger, "CAN-FD-FRAME-SUPPORT") {
        None | Some("false" | "0") => 0,
        Some("true" | "1") => FD,
        _ => return None,
    };
    Some(
        encode(
            graph.text(trigger, "IDENTIFIER"),
            graph.text(trigger, "CAN-ADDRESSING-MODE")?,
        )? | flag,
    )
}

pub(super) fn pdu_id(graph: &Graph, pdu: usize, receive: bool) -> Option<(u32, bool)> {
    let prefix = if receive { "CanIfRxPdu" } else { "CanIfTxPdu" };
    let (format, fd, accepts_fd) = match value(graph, pdu, &format!("{prefix}CanIdType"), false)? {
        "STANDARD_CAN" => ("STANDARD", 0, receive),
        "EXTENDED_CAN" => ("EXTENDED", 0, receive),
        "STANDARD_NO_FD_CAN" if receive => ("STANDARD", 0, false),
        "EXTENDED_NO_FD_CAN" if receive => ("EXTENDED", 0, false),
        "STANDARD_FD_CAN" => ("STANDARD", FD, false),
        "EXTENDED_FD_CAN" => ("EXTENDED", FD, false),
        _ => return None,
    };
    Some((
        encode(value(graph, pdu, &format!("{prefix}CanId"), false), format)? | fd,
        accepts_fd,
    ))
}

pub(super) fn encoded(id: u32, extended: bool) -> u32 {
    id | if extended { EXTENDED } else { 0 }
}

fn encode(value: Option<&str>, format: &str) -> Option<u32> {
    let id = crate::definitions::unsigned_integer(value?)?;
    match format {
        "STANDARD" if id <= 0x7ff => Some(id),
        "EXTENDED" if id <= EXTENDED_MAX => Some(id | EXTENDED),
        _ => None,
    }
}

pub(super) fn accepts(route: u32, dual: bool, frame: u32) -> bool {
    route == frame || (dual && route == (frame & !FD))
}

pub(super) fn wire_length(id: u32, length: u8) -> bool {
    length <= 8 || (id & FD != 0 && matches!(length, 12 | 16 | 20 | 24 | 32 | 48 | 64))
}
