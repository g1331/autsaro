//! ECUC symbolic handles and the selected synthesized LSduR configuration.
use super::{PlanDescription, PlanDiagnostic};
use std::collections::BTreeMap;

// R24-11 MOD symbolicNameValue parameters accepted by this profile.
const PARAMETERS: &[(&str, &str)] = &[
    ("CanController", "CanControllerId"),
    ("CanHardwareObject", "CanObjectId"),
    ("ComMChannel", "ComMChannelId"),
    ("ComMUser", "ComMUserIdentifier"),
    ("DetModule", "DetModuleId"),
    ("CanIfCtrlCfg", "CanIfCtrlId"),
    ("CanIfRxPduCfg", "CanIfRxPduId"),
    ("CanIfTxPduCfg", "CanIfTxPduId"),
    ("CanTpRxNSdu", "CanTpRxNSduId"),
    ("CanTpTxNSdu", "CanTpTxNSduId"),
    ("CanTpRxNPdu", "CanTpRxNPduId"),
    ("CanTpRxFcNPdu", "CanTpRxFcNPduId"),
    ("CanTpTxNPdu", "CanTpTxNPduConfirmationPduId"),
    ("CanTpTxFcNPdu", "CanTpTxFcNPduConfirmationPduId"),
    ("ComIPduGroup", "ComIPduGroupHandleId"),
    ("DcmDslMainConnection", "DcmDslProtocolRxConnectionId"),
    ("DcmDslProtocolRow", "DcmDslProtocolType"),
    ("DcmDslProtocolRx", "DcmDslProtocolRxPduId"),
    ("DcmDslProtocolTx", "DcmDslTxConfirmationPduId"),
    ("DcmDspSessionRow", "DcmDspSessionLevel"),
    ("PduRSrcPdu", "PduRSourcePduHandleId"),
    ("PduRDestPdu", "PduRDestPduHandleId"),
];

/// TPS_ECUC_02108: module prefix, definition-container short name, value short name.
pub(super) fn name(module: &str, container: &str, path: &str) -> String {
    format!(
        "{module}Conf_{container}_{}",
        super::component::c_name(super::short_name(path))
    )
}

pub(super) fn configured_kind(definition: &str) -> bool {
    let kind = super::short_name(definition);
    matches!(kind, "ComSignal" | "ComIPdu")
        || PARAMETERS.iter().any(|(container, _)| *container == kind)
}

pub(super) struct Names {
    pub headers: BTreeMap<String, Vec<(String, String)>>,
    parameters: BTreeMap<String, Vec<String>>,
    references: BTreeMap<(String, String), String>,
}

impl Names {
    pub fn new(plan: &PlanDescription) -> Result<Self, Vec<PlanDiagnostic>> {
        let mut result = Self {
            headers: BTreeMap::new(),
            parameters: BTreeMap::new(),
            references: BTreeMap::new(),
        };
        for record in &plan.configuration {
            let module = record.definition.split('/').nth(3).unwrap_or("");
            let symbol = name(module, super::short_name(&record.definition), &record.path);
            for (parameter, values) in &record.parameters {
                if PARAMETERS.iter().any(|(container, name)| {
                    *container == super::short_name(&record.definition)
                        && *name == super::short_name(parameter)
                }) {
                    let value = values
                        .first()
                        .and_then(|value| {
                            // SWS_Dcm_00979; diagnostic::inspect_transport admits only this protocol.
                            if super::short_name(parameter) == "DcmDslProtocolType" {
                                (value == "DCM_UDS_ON_CAN").then_some(3)
                            } else {
                                value.parse::<u32>().ok()
                            }
                        })
                        .ok_or_else(|| PlanDiagnostic::plan_premise(&record.path))?;
                    result
                        .headers
                        .entry(module.into())
                        .or_default()
                        .push((symbol.clone(), format!("{value}u")));
                    result
                        .parameters
                        .entry(super::short_name(parameter).into())
                        .or_default()
                        .push(symbol.clone());
                }
            }
            for (reference, targets) in &record.references {
                for target in targets {
                    result.references.insert(
                        (super::short_name(reference).into(), target.clone()),
                        symbol.clone(),
                    );
                }
            }
        }
        for handle in plan
            .handles
            .iter()
            .filter(|handle| handle.domain == "com_signal")
        {
            result.add("Com", "ComSignal", &handle.path, handle.handle);
        }
        for signal in &plan.signals {
            let route = plan
                .routes
                .iter()
                .find(|route| route.pdu == signal.global_pdu)
                .ok_or_else(|| PlanDiagnostic::plan_premise(&signal.global_pdu))?;
            result.add(
                "Com",
                "ComIPdu",
                &signal.com_pdu,
                u32::from(if signal.receive {
                    route.destination_handle
                } else {
                    route.source_handle
                }),
            );
        }
        // LSduR owns the synthesized entry IDs. Outgoing IDs belong to the
        // actual destination modules and are imported from their headers.
        for route in &plan.routes {
            let handle = if route.transport {
                let tp = plan
                    .diagnostic_transport
                    .as_ref()
                    .ok_or_else(|| PlanDiagnostic::plan_premise(&route.path))?;
                if route.receive {
                    tp.request_can_if_handle
                } else {
                    tp.transmit_npdu
                }
            } else if route.receive {
                plan.signals
                    .iter()
                    .find(|signal| signal.global_pdu == route.pdu)
                    .ok_or_else(|| PlanDiagnostic::plan_premise(&route.path))?
                    .can_if_handle
            } else {
                route.destination_handle
            };
            result.add(
                "LSduR",
                "LSduRPathSourcePdu",
                &route.source,
                u32::from(handle),
            );
        }
        for definitions in result.headers.values_mut() {
            definitions.sort();
            definitions.dedup();
        }
        Ok(result)
    }

    fn add(&mut self, module: &str, container: &str, path: &str, value: u32) {
        self.headers.entry(module.into()).or_default().push((
            name(module, container, path),
            format!(
                "{value}{}",
                if container == "ComSignal" { "U" } else { "u" }
            ),
        ));
    }

    /// Parameters used here have one selected instance, checked by their owner.
    pub fn parameter(&self, parameter: &str) -> Result<&str, Vec<PlanDiagnostic>> {
        self.parameters
            .get(parameter)
            .and_then(|names| names.first())
            .map(String::as_str)
            .ok_or_else(|| PlanDiagnostic::plan_premise(parameter))
    }

    pub fn reference(&self, reference: &str, target: &str) -> Result<&str, Vec<PlanDiagnostic>> {
        self.references
            .get(&(reference.into(), target.into()))
            .map(String::as_str)
            .ok_or_else(|| PlanDiagnostic::plan_premise(target))
    }
}
