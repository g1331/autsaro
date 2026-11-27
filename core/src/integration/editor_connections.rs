//! Choices for existing channel bindings and PduR forwarding entries.
use super::{PlanDescription, editor::FieldEdit};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct ConnectionEdit {
    pub object: String,
    pub definition: String,
    pub target: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct ConnectionChoice {
    pub object: String,
    pub definition: String,
    pub current: String,
    pub options: Vec<String>,
}

pub(crate) fn choices(plan: &PlanDescription) -> Vec<ConnectionChoice> {
    let records: BTreeMap<_, _> = plan
        .configuration
        .iter()
        .map(|record| (&record.path, record))
        .collect();
    let mut bindings = Vec::new();
    for owner in &plan.configuration {
        for (definition, targets) in &owner.references {
            let name = definition.rsplit('/').next().unwrap_or("");
            if !matches!(
                name,
                "CanControllerRef"
                    | "CanIfCtrlCanCtrlRef"
                    | "CanIfHrhCanCtrlIdRef"
                    | "CanIfHthCanCtrlIdRef"
                    | "CanIfRxPduHrhIdRef"
                    | "CanIfTxPduBufferRef"
                    | "CanIfBufferHthRef"
                    | "BswMComMChannelRef"
                    | "CanIfHthIdSymRef"
                    | "CanIfHrhIdSymRef"
                    | "ComMUserChannel"
                    | "DcmDslProtocolComMChannelRef"
                    | "CanTpRxNPduRef"
                    | "CanTpTxNPduRef"
                    | "CanTpTxFcNPduRef"
                    | "CanTpRxFcNPduRef"
                    | "PduRSrcPduRRef"
                    | "PduRDestPduRRef"
            ) {
                continue;
            }
            let [current] = targets.as_slice() else {
                continue;
            };
            let Some(target) = records.get(current) else {
                continue;
            };
            let forwarding = matches!(name, "PduRSrcPduRRef" | "PduRDestPduRRef");
            if forwarding
                && !plan
                    .routes
                    .iter()
                    .any(|route| route.path == owner.path && route.gateway.is_some())
            {
                continue;
            }
            let options = plan
                .configuration
                .iter()
                .filter(|candidate| candidate.definition == target.definition)
                .map(|candidate| candidate.path.clone())
                .collect::<Vec<_>>();
            bindings.push(ConnectionChoice {
                object: owner.path.clone(),
                definition: definition.clone(),
                current: current.clone(),
                options,
            });
        }
    }
    if let Some(composition) = &plan.composition {
        let mut maps = std::collections::BTreeSet::new();
        for signal in &plan.signals {
            if maps.insert((&signal.mapping_set, &signal.application_type)) {
                let options = composition
                    .types
                    .iter()
                    .filter(|ty| {
                        ty.array_length.is_none()
                            && matches!(
                                ty.native_type.as_str(),
                                "uint8" | "sint8" | "uint16" | "sint16" | "uint32" | "sint32"
                            )
                    })
                    .map(|ty| ty.path.clone())
                    .collect();
                bindings.push(ConnectionChoice {
                    object: signal.mapping_set.clone(),
                    definition: signal.application_type.clone(),
                    current: signal.implementation_type.clone(),
                    options,
                });
            }
        }
    }
    for transport in plan
        .diagnostic_transport
        .iter()
        .flat_map(|transport| transport.connections())
    {
        for (owner, fd, length) in [
            (
                &transport.rx_npdu,
                transport.request_fd,
                transport.rx_max_dl,
            ),
            (&transport.tx_npdu, transport.response_fd, transport.tx_dl),
        ] {
            for (name, current, options) in [
                (
                    "frameFormat",
                    if fd { "FD" } else { "CLASSICAL" }.to_owned(),
                    vec!["FD".into(), "CLASSICAL".into()],
                ),
                (
                    "wireLength",
                    length.to_string(),
                    [8, 12, 16, 20, 24, 32, 48, 64]
                        .map(|length| length.to_string())
                        .to_vec(),
                ),
            ] {
                bindings.push(ConnectionChoice {
                    object: owner.clone(),
                    definition: name.into(),
                    current,
                    options,
                });
            }
        }
        bindings.push(ConnectionChoice {
            object: transport.rx_npdu.clone(),
            definition: "minimumLength".into(),
            current: transport.rx_min_dl.to_string(),
            options: vec!["0".into(), "8".into()],
        });
    }
    // Several CanTp connections may share one transmit N-PDU.
    let mut seen = std::collections::BTreeSet::new();
    bindings.retain(|binding| seen.insert((binding.object.clone(), binding.definition.clone())));
    bindings
}

impl ConnectionEdit {
    pub(crate) fn resolve(
        &self,
        plan: &PlanDescription,
        choices: &[ConnectionChoice],
        settings: &BTreeMap<String, Vec<super::SignalSetting>>,
    ) -> Option<Vec<FieldEdit>> {
        let choice = choices.iter().find(|choice| {
            self.object == choice.object
                && self.definition == choice.definition
                && choice.options.contains(&self.target)
        })?;
        let field = |object: &str, name: &str, parameter: bool, value: &str| FieldEdit {
            object: object.into(),
            field: name.into(),
            parameter,
            value: value.into(),
        };
        let mut signals = plan
            .signals
            .iter()
            .filter(|signal| {
                signal.mapping_set == self.object && signal.application_type == self.definition
            })
            .peekable();
        if signals.peek().is_some() {
            let ty = plan
                .composition
                .as_ref()?
                .types
                .iter()
                .find(|ty| ty.path == self.target)?;
            let base = ty.base_reference.as_deref()?;
            let bits: u16 = ty
                .native_type
                .trim_start_matches(['s', 'u'])
                .strip_prefix("int")?
                .parse()
                .ok()?;
            let mut edits = vec![field(&self.object, &self.definition, false, &self.target)];
            for signal in signals {
                if !signal.network_from_com_spec {
                    edits.push(field(&signal.signal, "BASE-TYPE-REF", false, base));
                }
                edits.push(field(
                    &signal.com_signal,
                    "ComSignalType",
                    true,
                    &ty.native_type.to_ascii_uppercase(),
                ));
                let explicit_width = settings.get(&signal.port).is_some_and(|settings| {
                    settings
                        .iter()
                        .any(|setting| matches!(setting, super::SignalSetting::BitLength(_)))
                });
                if signal.bit_length > bits && !explicit_width {
                    edits.extend(super::SignalSetting::BitLength(bits).fields(signal));
                }
            }
            return Some(edits);
        }
        if choice.definition.starts_with('/') {
            return Some(vec![field(
                &choice.object,
                &choice.definition,
                true,
                &self.target,
            )]);
        }
        let transport = plan
            .diagnostic_transport
            .as_ref()?
            .connections()
            .find(|transport| {
                transport.rx_npdu == self.object || transport.tx_npdu == self.object
            })?;
        let receive = transport.rx_npdu == self.object;
        let (wire, can_if, extended) = if receive {
            (
                &transport.rx_wire_objects,
                &transport.can_if_rx_pdu,
                transport.request_extended,
            )
        } else {
            (
                &transport.tx_wire_objects,
                &transport.can_if_tx_pdu,
                transport.response_extended,
            )
        };
        let [frame, pdu, trigger] = wire;
        match self.definition.as_str() {
            "frameFormat" => {
                let fd = self.target == "FD";
                let format = if extended { "EXTENDED" } else { "STANDARD" };
                let suffix = if fd {
                    "FD_CAN"
                } else if receive {
                    "NO_FD_CAN"
                } else {
                    "CAN"
                };
                Some(vec![
                    field(
                        trigger,
                        "CAN-FD-FRAME-SUPPORT",
                        false,
                        if fd { "true" } else { "false" },
                    ),
                    field(
                        can_if,
                        if receive {
                            "CanIfRxPduCanIdType"
                        } else {
                            "CanIfTxPduCanIdType"
                        },
                        true,
                        &format!("{format}_{suffix}"),
                    ),
                ])
            }
            "wireLength" => Some(vec![
                field(frame, "FRAME-LENGTH", false, &self.target),
                field(pdu, "LENGTH", false, &self.target),
                field(&self.object, "PduLength", true, &self.target),
            ]),
            "minimumLength" => {
                let enabled = self.target == "8";
                let mut edits = vec![field(
                    can_if,
                    "CanIfRxPduDataLengthCheck",
                    true,
                    if enabled { "true" } else { "false" },
                )];
                if enabled {
                    edits.push(field(can_if, "CanIfRxPduDataLength", true, "8"));
                }
                Some(edits)
            }
            _ => None,
        }
    }
}
