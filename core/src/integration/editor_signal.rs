//! Linked representations of one signal setting in the System Template and ECUC.
use super::{SignalChannel, editor::FieldEdit};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(tag = "field", content = "value", rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub enum SignalSetting {
    Extended(bool),
    StartBit(u16),
    BigEndian(bool),
    FrameLength(u8),
    Fd(bool),
    Dual(bool),
    BitLength(u16),
    PduLength(u8),
}

impl SignalSetting {
    pub(crate) fn current(signal: &SignalChannel) -> [Self; 7] {
        [
            Self::Extended(signal.extended),
            Self::Fd(signal.fd),
            Self::BitLength(signal.bit_length),
            Self::StartBit(signal.start_bit),
            Self::BigEndian(signal.big_endian),
            Self::FrameLength(signal.dlc),
            Self::PduLength(signal.pdu_length),
        ]
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Extended(_) => "id-format",
            Self::Fd(_) => "frame-format",
            Self::Dual(_) => "receive-format",
            Self::BitLength(_) => "signal-width",
            Self::StartBit(_) => "start-bit",
            Self::BigEndian(_) => "byte-order",
            Self::FrameLength(_) => "frame-length",
            Self::PduLength(_) => "pdu-length",
        }
    }

    pub(crate) fn parse_cell(self, signal: &SignalChannel, field: &str, raw: &str) -> Option<Self> {
        let number = || crate::definitions::unsigned_integer(raw);
        match self {
            Self::BitLength(_) => Some(Self::BitLength(number()?.try_into().ok()?)),
            Self::Dual(_) => None,
            Self::Fd(_) => match raw {
                "true" | "1" => Some(Self::Fd(true)),
                "false" | "0" => Some(Self::Fd(false)),
                _ => None,
            },
            Self::StartBit(_) => Some(Self::StartBit(number()?.try_into().ok()?)),
            Self::FrameLength(_) => Some(Self::FrameLength(number()?.try_into().ok()?)),
            Self::PduLength(_) => Some(Self::PduLength(number()?.try_into().ok()?)),
            Self::Extended(_) | Self::BigEndian(_) => [false, true]
                .into_iter()
                .map(|flag| {
                    if matches!(self, Self::Extended(_)) {
                        Self::Extended(flag)
                    } else {
                        Self::BigEndian(flag)
                    }
                })
                .find(|setting| {
                    setting
                        .fields(signal)
                        .iter()
                        .any(|leaf| leaf.field == field && leaf.value == raw)
                }),
        }
    }

    pub(crate) fn can_if_settings(raw: &str, signal: &SignalChannel) -> Option<Vec<Self>> {
        let (format, fd, dual) = super::can_id::policy(raw, signal.receive)?;
        Some(vec![
            Self::Extended(format == "EXTENDED"),
            Self::Fd(if dual { signal.fd } else { fd != 0 }),
            Self::Dual(dual),
        ])
    }

    pub(crate) fn fields(self, s: &SignalChannel) -> Vec<FieldEdit> {
        let leaf = |object: &str, field: &str, parameter: bool, value: String| FieldEdit {
            object: object.into(),
            field: field.into(),
            parameter,
            value,
        };
        match self {
            Self::Extended(extended) => {
                let format = if extended { "EXTENDED" } else { "STANDARD" };
                let suffix = if s.accepts_fd {
                    "CAN"
                } else if s.fd {
                    "FD_CAN"
                } else if s.receive {
                    "NO_FD_CAN"
                } else {
                    "CAN"
                };
                vec![
                    leaf(&s.trigger, "CAN-ADDRESSING-MODE", false, format.into()),
                    leaf(
                        &s.can_if_pdu,
                        if s.receive {
                            "CanIfRxPduCanIdType"
                        } else {
                            "CanIfTxPduCanIdType"
                        },
                        true,
                        format!("{format}_{suffix}"),
                    ),
                ]
            }
            Self::Fd(fd) => {
                let mut fields = vec![leaf(
                    &s.trigger,
                    "CAN-FD-FRAME-SUPPORT",
                    false,
                    fd.to_string(),
                )];
                if !s.accepts_fd {
                    let prefix = if s.extended { "EXTENDED" } else { "STANDARD" };
                    let suffix = if fd {
                        "FD_CAN"
                    } else if s.receive {
                        "NO_FD_CAN"
                    } else {
                        "CAN"
                    };
                    fields.push(leaf(
                        &s.can_if_pdu,
                        if s.receive {
                            "CanIfRxPduCanIdType"
                        } else {
                            "CanIfTxPduCanIdType"
                        },
                        true,
                        format!("{prefix}_{suffix}"),
                    ));
                }
                fields
            }
            Self::BitLength(bits) => vec![
                leaf(&s.signal, "LENGTH", false, bits.to_string()),
                leaf(&s.com_signal, "ComBitSize", true, bits.to_string()),
            ],
            Self::Dual(_) => Self::Extended(s.extended).fields(s),
            Self::StartBit(bit) => vec![
                leaf(&s.signal_mapping, "START-POSITION", false, bit.to_string()),
                leaf(&s.com_signal, "ComBitPosition", true, bit.to_string()),
            ],
            Self::BigEndian(big) => vec![
                leaf(
                    &s.signal_mapping,
                    "PACKING-BYTE-ORDER",
                    false,
                    if big {
                        "MOST-SIGNIFICANT-BYTE-FIRST"
                    } else {
                        "MOST-SIGNIFICANT-BYTE-LAST"
                    }
                    .into(),
                ),
                leaf(
                    &s.com_signal,
                    "ComSignalEndianness",
                    true,
                    if big { "BIG_ENDIAN" } else { "LITTLE_ENDIAN" }.into(),
                ),
            ],
            Self::FrameLength(length) => {
                vec![leaf(&s.frame, "FRAME-LENGTH", false, length.to_string())]
            }
            Self::PduLength(length) => {
                let mut fields = vec![
                    leaf(&s.system_pdu, "LENGTH", false, length.to_string()),
                    leaf(&s.global_pdu, "PduLength", true, length.to_string()),
                ];
                if s.receive {
                    fields.push(leaf(
                        &s.can_if_pdu,
                        "CanIfRxPduDataLength",
                        true,
                        length.to_string(),
                    ));
                }
                fields
            }
        }
    }
}
