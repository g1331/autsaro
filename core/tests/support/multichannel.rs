//! ARXML variations of the reference application with one independent echo per CAN channel.
use super::mainline::{named, parameter, single_component_sources};
use autosar_config_core::integration::InputSource;
use roxmltree::{Document, Node};
use std::collections::BTreeMap;

type Edits = BTreeMap<usize, Vec<String>>;

fn text<'a>(node: Node<'a, '_>, tag: &str) -> Option<&'a str> {
    node.children()
        .find(|child| child.has_tag_name(tag))
        .and_then(|child| child.text())
}

fn definition(node: Node<'_, '_>, kind: &str) -> bool {
    text(node, "DEFINITION-REF").is_some_and(|path| path.rsplit('/').next() == Some(kind))
}

fn set(xml: &str, kind: &str, value: usize) -> String {
    let doc = Document::parse(xml).unwrap();
    let range = doc
        .descendants()
        .find(|node| definition(*node, kind))
        .and_then(|node| node.children().find(|child| child.has_tag_name("VALUE")))
        .unwrap_or_else(|| panic!("missing {kind}: {xml}"))
        .range();
    format!(
        "{}<VALUE>{value}</VALUE>{}",
        &xml[..range.start],
        &xml[range.end..]
    )
}

fn retain_signal_ports(xml: &str) -> String {
    let doc = Document::parse(xml).unwrap();
    let mut result = xml.to_owned();
    let excluded = doc
        .descendants()
        .filter(|node| text(*node, "SHORT-NAME").is_some_and(|name| name.starts_with("Diag")))
        .map(|node| node.range())
        .collect::<Vec<_>>();
    for range in excluded.into_iter().rev() {
        result.replace_range(range, "");
    }
    result
}

fn signal_suffix(xml: &str, suffix: &str) -> String {
    xml.replace("RxValue", &format!("RxValue{suffix}"))
        .replace("TxValue", &format!("TxValue{suffix}"))
}

fn network(xml: &str, channel: usize) -> String {
    xml.replace("ReferenceCan", &format!("ReferenceCan{channel}"))
        .replace("CanConnector", &format!("CanConnector{channel}"))
        .replace("CanController", &format!("CanController{channel}"))
}

fn add(edits: &mut Edits, node: Node<'_, '_>, xml: String) {
    edits.entry(node.range().end).or_default().push(xml);
}

fn extend_application(input: &str, count: usize) -> String {
    let doc = Document::parse(input).unwrap();
    let mut edits = Edits::new();
    for original in doc.descendants().filter(|node| {
        [
            ("R-PORT-PROTOTYPE", "RxValue"),
            ("P-PORT-PROTOTYPE", "TxValue"),
            ("VARIABLE-ACCESS", "RRxValue"),
            ("VARIABLE-ACCESS", "PTxValue"),
        ]
        .iter()
        .any(|(tag, name)| named(*node, tag, name))
    }) {
        for channel in 1..count {
            add(
                &mut edits,
                original,
                signal_suffix(&input[original.range()], &channel.to_string()),
            );
        }
    }
    apply(input, edits)
}

fn extend_extract(input: &str, count: usize) -> String {
    let doc = Document::parse(input).unwrap();
    let mut edits = Edits::new();
    for original in doc.descendants().filter(|node| node.is_element()) {
        let tag = original.tag_name().name();
        let name = text(original, "SHORT-NAME").unwrap_or_default();
        let topology = matches!(
            tag,
            "CAN-CLUSTER" | "CAN-COMMUNICATION-CONTROLLER" | "CAN-COMMUNICATION-CONNECTOR"
        );
        let mapping = tag == "SENDER-RECEIVER-TO-SIGNAL-MAPPING"
            && (input[original.range()].contains("/Ingress/RxValue")
                || input[original.range()].contains("/Ingress/TxValue"));
        let descriptor = matches!(
            tag,
            "CAN-FRAME" | "I-SIGNAL-I-PDU" | "I-SIGNAL" | "SYSTEM-SIGNAL"
        ) && (name.starts_with("RxValue") || name.starts_with("TxValue"));
        let fibex = tag == "FIBEX-ELEMENT-REF-CONDITIONAL"
            && input[original.range()].contains("/Extract/ReferenceCan");
        if topology || mapping || descriptor || fibex {
            for channel in 1..count {
                let xml = &input[original.range()];
                let cloned = if topology {
                    network(&retain_signal_ports(xml), channel)
                } else {
                    network(xml, channel)
                };
                add(
                    &mut edits,
                    original,
                    signal_suffix(&cloned, &channel.to_string()),
                );
            }
        }
    }
    apply(input, edits)
}

fn mode_callout(xml: &str, channel: usize) -> String {
    let controller = if channel == 0 {
        "Controller".to_owned()
    } else {
        format!("Controller{channel}")
    };
    xml.replace(
        "Ecu_HostBusSM_ConfirmMode(0u, ",
        &format!(
            "Ecu_HostBusSM_ConfirmControllerMode({channel}u, CanIfConf_CanIfCtrlCfg_{controller}, "
        ),
    )
}

fn extend_ecuc(mut input: String, count: usize) -> String {
    let doc = Document::parse(&input).unwrap();
    let positions = doc
        .descendants()
        .filter(|node| {
            definition(*node, "RteBswPositionInTask") || definition(*node, "RtePositionInTask")
        })
        .filter_map(|node| node.children().find(|child| child.has_tag_name("VALUE")))
        .filter_map(|node| {
            let value = node.text()?.parse::<usize>().ok()?;
            (value != 0).then(|| (node.range(), value + count - 1))
        })
        .collect::<Vec<_>>();
    for (range, position) in positions.into_iter().rev() {
        input.replace_range(range, &format!("<VALUE>{position}</VALUE>"));
    }
    // Convert the legacy single-channel callouts before adding explicit siblings.
    let doc = Document::parse(&input).unwrap();
    let mut edits = Edits::new();
    for original in doc
        .descendants()
        .filter(|node| node.has_tag_name("ECUC-CONTAINER-VALUE"))
    {
        let original_xml = &input[original.range()];
        let name = text(original, "SHORT-NAME").unwrap();
        let kind = text(original, "DEFINITION-REF")
            .unwrap()
            .rsplit('/')
            .next()
            .unwrap();
        let signal = name.starts_with("RxValue") || name.starts_with("TxValue");
        for channel in 1..count {
            let suffix = channel.to_string();
            let driver = count - 1 - channel;
            let xml = match kind {
                "CanController" => set(
                    &original_xml
                        .replace("/Controller/", &format!("/Controller{channel}/"))
                        .replace(
                            "<SHORT-NAME>Controller</",
                            &format!("<SHORT-NAME>Controller{channel}</"),
                        ),
                    "CanControllerId",
                    driver,
                ),
                "CanHardwareObject" => {
                    let offset = usize::from(name == "Transmit");
                    let with_controller = original_xml.replace(
                        "/Can/Config/Controller</",
                        &format!("/Can/Config/Controller{channel}</"),
                    );
                    set(
                        &with_controller.replace(
                            &format!("<SHORT-NAME>{name}</"),
                            &format!("<SHORT-NAME>{name}{channel}</"),
                        ),
                        "CanObjectId",
                        2 * driver + offset,
                    )
                }
                "CanIfCtrlCfg" => set(
                    &original_xml
                        .replace(
                            "/Can/Config/Controller</",
                            &format!("/Can/Config/Controller{channel}</"),
                        )
                        .replace(
                            "<SHORT-NAME>Controller</",
                            &format!("<SHORT-NAME>Controller{channel}</"),
                        ),
                    "CanIfCtrlId",
                    channel,
                ),
                "CanIfHrhCfg" | "CanIfHthCfg" | "CanIfBufferCfg" => original_xml
                    .replace(
                        &format!("<SHORT-NAME>{name}</"),
                        &format!("<SHORT-NAME>{name}{channel}</"),
                    )
                    .replace(
                        "/Controller/Controller</",
                        &format!("/Controller/Controller{channel}</"),
                    )
                    .replace(
                        "/Can/Config/Receive</",
                        &format!("/Can/Config/Receive{channel}</"),
                    )
                    .replace(
                        "/Can/Config/Transmit</",
                        &format!("/Can/Config/Transmit{channel}</"),
                    )
                    .replace(
                        "/CanIf/Config/Handles/Transmit</",
                        &format!("/CanIf/Config/Handles/Transmit{channel}</"),
                    ),
                "ComMChannel" => set(
                    &original_xml.replace(
                        "<SHORT-NAME>Host</",
                        &format!("<SHORT-NAME>Host{channel}</"),
                    ),
                    "ComMChannelId",
                    channel,
                ),
                "RteBswEventToTaskMapping" if name == "ComM_MainFunction_Host" => set(
                    &original_xml.replace(
                        "ComM_MainFunction_Host",
                        &format!("ComM_MainFunction_Host{channel}"),
                    ),
                    "RteBswPositionInTask",
                    channel,
                ),
                "BswMModeRequestPort"
                | "BswMModeCondition"
                | "BswMLogicalExpression"
                | "BswMRule"
                | "BswMAction"
                | "BswMActionList" => {
                    let mut xml = mode_callout(original_xml, channel);
                    for item in [
                        "CurrentMode",
                        "NoCondition",
                        "SilentCondition",
                        "FullCondition",
                        "NoExpression",
                        "SilentExpression",
                        "FullExpression",
                        "NoRule",
                        "SilentRule",
                        "FullRule",
                        "NoAction",
                        "SilentAction",
                        "FullAction",
                        "NoList",
                        "SilentList",
                        "FullList",
                    ] {
                        xml = xml.replace(item, &format!("{item}{channel}"));
                    }
                    xml.replace(
                        "/ComM/Config/Host</",
                        &format!("/ComM/Config/Host{channel}</"),
                    )
                }
                _ if signal => {
                    let xml = signal_suffix(original_xml, &suffix);
                    match kind {
                        "ComUserSignal" => set(&xml, "ComUserCbkHandleId", 17 + channel),
                        "CanIfRxPduCfg" => set(
                            &xml.replace(
                                "/Handles/Receive</",
                                &format!("/Handles/Receive{channel}</"),
                            ),
                            "CanIfRxPduId",
                            channel + 1,
                        ),
                        "CanIfTxPduCfg" => set(
                            &xml.replace(
                                "/TransmitBuffer</",
                                &format!("/TransmitBuffer{channel}</"),
                            ),
                            "CanIfTxPduId",
                            channel + 1,
                        ),
                        "PduRSrcPdu" => set(
                            &xml,
                            "PduRSourcePduHandleId",
                            2 + 2 * channel + usize::from(name.starts_with("Tx")),
                        ),
                        "PduRDestPdu" => set(
                            &xml,
                            "PduRDestPduHandleId",
                            2 + 2 * channel + usize::from(name.starts_with("Tx")),
                        ),
                        "ComSignal" | "ComIPdu" | "Pdu" | "PduRRoutingPath" => xml,
                        _ => continue,
                    }
                }
                _ => continue,
            };
            add(&mut edits, original, xml);
        }
    }
    input = apply(&input, edits);
    mode_callout(&input, 0)
}

fn extend_bsw(input: &str, count: usize) -> String {
    let doc = Document::parse(input).unwrap();
    let mut edits = Edits::new();
    for original in doc.descendants().filter(|node| {
        text(*node, "SHORT-NAME").is_some_and(|name| name.starts_with("ComM_MainFunction_Host"))
            || (node.has_tag_name("BSW-MODULE-ENTRY-REF-CONDITIONAL")
                && text(*node, "BSW-MODULE-ENTRY-REF")
                    == Some("/AUTOSAR_ComM/BswModuleEntrys/ComM_MainFunction_Host"))
    }) {
        for channel in 1..count {
            add(
                &mut edits,
                original,
                input[original.range()].replace(
                    "ComM_MainFunction_Host",
                    &format!("ComM_MainFunction_Host{channel}"),
                ),
            );
        }
    }
    apply(input, edits)
}

/// Pair the existing standard Rx endpoint with an independent extended Rx
/// endpoint carrying the same raw identifier, on every physical channel.
fn extended_receive(input: &str, file: &str, count: usize) -> String {
    let doc = Document::parse(input).unwrap();
    let mut edits = Edits::new();
    for channel in 0..count {
        let suffix = if channel == 0 {
            String::new()
        } else {
            channel.to_string()
        };
        let signal = format!("RxValue{suffix}");
        let extended = format!("RxExtended{suffix}");
        for node in doc.descendants().filter(|node| node.is_element()) {
            let name = text(node, "SHORT-NAME").unwrap_or_default();
            let tag = node.tag_name().name();
            let original = &input[node.range()];
            let mut cloned = original.replace(&signal, &extended);
            match file {
                "ingress.arxml"
                    if (tag == "R-PORT-PROTOTYPE" && name == signal)
                        || (tag == "VARIABLE-ACCESS" && name == format!("R{signal}")) => {}
                "extract.arxml"
                    if tag == "SENDER-RECEIVER-TO-SIGNAL-MAPPING"
                        && original.contains(&format!("/Ingress/{signal}</CONTEXT-PORT-REF>")) => {}
                "extract.arxml"
                    if [
                        "FramePort",
                        "PduPort",
                        "SignalPort",
                        "Trigger",
                        "SignalTrigger",
                        "PduTrigger",
                        "Frame",
                        "Pdu",
                        "Signal",
                        "SystemSignal",
                    ]
                    .iter()
                    .any(|ending| name == format!("{signal}{ending}"))
                        && matches!(
                            tag,
                            "FRAME-PORT"
                                | "I-PDU-PORT"
                                | "I-SIGNAL-PORT"
                                | "CAN-FRAME-TRIGGERING"
                                | "I-SIGNAL-TRIGGERING"
                                | "PDU-TRIGGERING"
                                | "CAN-FRAME"
                                | "I-SIGNAL-I-PDU"
                                | "I-SIGNAL"
                                | "SYSTEM-SIGNAL",
                        ) =>
                {
                    cloned = cloned.replace(
                        "<CAN-ADDRESSING-MODE>STANDARD",
                        "<CAN-ADDRESSING-MODE>EXTENDED",
                    );
                }
                "ecuc.arxml"
                    if tag == "ECUC-CONTAINER-VALUE"
                        && [
                            signal.clone(),
                            format!("{signal}Pdu"),
                            format!("{signal}Source"),
                            format!("{signal}Destination"),
                        ]
                        .contains(&name.to_owned()) =>
                {
                    if definition(node, "CanIfRxPduCfg") {
                        cloned = set(&cloned, "CanIfRxPduId", count + 1 + channel).replace(
                            "<VALUE>STANDARD_NO_FD_CAN</VALUE>",
                            "<VALUE>EXTENDED_NO_FD_CAN</VALUE>",
                        );
                    } else if definition(node, "ComUserSignal") {
                        cloned = set(&cloned, "ComUserCbkHandleId", 17 + count + channel);
                    } else if definition(node, "PduRSrcPdu") {
                        cloned = set(&cloned, "PduRSourcePduHandleId", 2 + (2 * count) + channel);
                    } else if definition(node, "PduRDestPdu") {
                        cloned = set(&cloned, "PduRDestPduHandleId", 2 + (2 * count) + channel);
                    }
                }
                _ => continue,
            }
            add(&mut edits, node, cloned);
        }
    }
    apply(input, edits)
}

fn apply(input: &str, edits: Edits) -> String {
    let mut result = input.to_owned();
    for (at, additions) in edits.into_iter().rev() {
        result.insert_str(at, &additions.join("\n"));
    }
    result
}

pub(crate) fn multichannel_sources(count: usize) -> Vec<InputSource> {
    let mut inputs = single_component_sources();
    for input in &mut inputs {
        let path = input.logical_path();
        let xml = std::str::from_utf8(input.bytes()).unwrap();
        let changed = match path {
            "ingress.arxml" => extend_application(xml, count),
            "extract.arxml" => extend_extract(xml, count),
            "ecuc.arxml" => extend_ecuc(xml.to_owned(), count),
            "bsw.arxml" => extend_bsw(xml, count),
            _ => continue,
        };
        let mixed = extended_receive(&changed, path, count);
        *input = InputSource::new(path, mixed.into_bytes()).unwrap();
    }
    for channel in 0..count {
        let hardware = if channel == 0 {
            "Receive".into()
        } else {
            format!("Receive{channel}")
        };
        parameter(&mut inputs, &hardware, "CanIdType", "MIXED");
    }
    let id = count - 1;
    parameter(
        &mut inputs,
        "Controller",
        "CanControllerId",
        &id.to_string(),
    );
    parameter(&mut inputs, "Receive", "CanObjectId", &(2 * id).to_string());
    parameter(
        &mut inputs,
        "Transmit",
        "CanObjectId",
        &(2 * id + 1).to_string(),
    );
    inputs
}

/// Two FD diagnostic links reuse the same CAN IDs on different physical buses.
pub(crate) fn multichannel_diagnostic_sources() -> Vec<InputSource> {
    diagnostic_sources(false)
}

/// Two links on one bus share the response N-PDU, with distinct request IDs.
pub(crate) fn shared_tx_diagnostic_sources() -> Vec<InputSource> {
    diagnostic_sources(true)
}

fn diagnostic_sources(shared_tx: bool) -> Vec<InputSource> {
    let mut inputs = multichannel_sources(2);
    for input in &mut inputs {
        let file = input.logical_path();
        if !matches!(file, "extract.arxml" | "ecuc.arxml") {
            continue;
        }
        let xml = std::str::from_utf8(input.bytes()).unwrap();
        let doc = Document::parse(xml).unwrap();
        let mut edits = Edits::new();
        for node in doc.descendants().filter(|node| node.is_element()) {
            let name = text(node, "SHORT-NAME").unwrap_or_default();
            let tag = node.tag_name().name();
            let is_diagnostic = name.starts_with("DiagRequest") || name.starts_with("DiagResponse");
            let original = &xml[node.range()];
            let mut second = original
                .replace("DiagRequest", "DiagRequest1")
                .replace("DiagResponse", "DiagResponse1");
            if file == "extract.arxml" && is_diagnostic {
                if shared_tx && name.starts_with("DiagResponse") {
                    continue;
                }
                match tag {
                    "CAN-FRAME" | "N-PDU" => add(&mut edits, node, second),
                    "FRAME-PORT" | "I-PDU-PORT" | "CAN-FRAME-TRIGGERING" | "PDU-TRIGGERING" => {
                        let location = doc
                            .descendants()
                            .find(|candidate| {
                                candidate.has_tag_name(tag)
                                    && text(*candidate, "SHORT-NAME").is_some_and(|name| {
                                        if shared_tx {
                                            name.starts_with("RxValue")
                                                && !name.starts_with("RxValue1")
                                        } else {
                                            name.starts_with("RxValue1")
                                        }
                                    })
                            })
                            .unwrap();
                        if shared_tx {
                            second = second.replace("<IDENTIFIER>1792</", "<IDENTIFIER>1793</");
                        } else {
                            second = network(&second, 1);
                        }
                        add(&mut edits, location, second);
                    }
                    _ => {}
                }
            } else if file == "ecuc.arxml" && tag == "ECUC-CONTAINER-VALUE" {
                if shared_tx {
                    if name.starts_with("DiagResponse")
                        && (definition(node, "CanIfTxPduCfg") || name == "DiagResponsePdu")
                    {
                        continue;
                    }
                    second = second.replace("DiagResponse1Pdu", "DiagResponsePdu");
                }
                let parameters: &[(&str, usize)] = if definition(node, "CanTpChannel") {
                    second = second
                        .replace(
                            "<SHORT-NAME>PhysicalChannel</",
                            "<SHORT-NAME>PhysicalChannel1</",
                        )
                        .replace("<SHORT-NAME>Frame</", "<SHORT-NAME>Frame1</")
                        .replace("<SHORT-NAME>FlowControl</", "<SHORT-NAME>FlowControl1</");
                    &[
                        ("CanTpRxNSduId", 1),
                        ("CanTpRxNPduId", 1),
                        ("CanTpTxFcNPduConfirmationPduId", usize::from(!shared_tx)),
                        ("CanTpTxNSduId", 1),
                        ("CanTpTxNPduConfirmationPduId", usize::from(!shared_tx)),
                        ("CanTpRxFcNPduId", 1),
                    ]
                } else if definition(node, "DcmDslConnection") {
                    second = second
                        .replace("<SHORT-NAME>Connection</", "<SHORT-NAME>Connection1</")
                        .replace("<SHORT-NAME>Physical</", "<SHORT-NAME>Physical1</")
                        .replace("<SHORT-NAME>Receive</", "<SHORT-NAME>Receive1</")
                        .replace("<SHORT-NAME>Transmit</", "<SHORT-NAME>Transmit1</");
                    if !shared_tx {
                        second = second.replace("/ComM/Config/Host</", "/ComM/Config/Host1</");
                    }
                    &[
                        ("DcmDslProtocolRxConnectionId", 1),
                        ("DcmDslProtocolRxPduId", 1),
                        ("DcmDslTxConfirmationPduId", 1),
                    ]
                } else if !is_diagnostic {
                    continue;
                } else if definition(node, "CanIfRxPduCfg") {
                    if shared_tx {
                        second = set(&second, "CanIfRxPduCanId", 1793);
                    } else {
                        second = second.replace("/Handles/Receive</", "/Handles/Receive1</");
                    }
                    &[("CanIfRxPduId", 5)]
                } else if definition(node, "CanIfTxPduCfg") {
                    second = second.replace("/TransmitBuffer</", "/TransmitBuffer1</");
                    &[("CanIfTxPduId", 3)]
                } else if definition(node, "PduRSrcPdu") {
                    &[("PduRSourcePduHandleId", 20)]
                } else if definition(node, "PduRDestPdu") {
                    &[("PduRDestPduHandleId", 20)]
                } else if definition(node, "Pdu") || definition(node, "PduRRoutingPath") {
                    &[]
                } else {
                    continue;
                };
                for (parameter, value) in parameters {
                    second = set(&second, parameter, *value);
                }
                add(&mut edits, node, second);
            }
        }
        *input = InputSource::new(file, apply(xml, edits).into_bytes()).unwrap();
    }
    use super::mainline::{enable_can_fd, xml_edit};
    for channel in 0..2 {
        let suffix = if channel == 0 { "" } else { "1" };
        enable_can_fd(&mut inputs, suffix);
        for (direction, rx) in [("Request", true), ("Response", false)] {
            if shared_tx && !rx && channel == 1 {
                continue;
            }
            let name = format!("Diag{direction}{suffix}");
            let global_pdu = format!("{name}Pdu");
            let dl = if !rx && channel == 1 { "12" } else { "64" };
            parameter(&mut inputs, &global_pdu, "PduLength", dl);
            parameter(
                &mut inputs,
                &name,
                if rx {
                    "CanIfRxPduCanIdType"
                } else {
                    "CanIfTxPduCanIdType"
                },
                "STANDARD_FD_CAN",
            );
            if rx {
                parameter(&mut inputs, &name, "CanIfRxPduDataLengthCheck", "false");
            }
            let network_suffix = if shared_tx { "" } else { suffix };
            let trigger = format!("/Extract/ReferenceCan{network_suffix}/Channel/{name}Trigger");
            xml_edit(
                &mut inputs,
                "ecuc.arxml",
                |node| named(node, "ECUC-CONTAINER-VALUE", &global_pdu),
                |xml| {
                    xml.replace("</PARAMETER-VALUES>", &format!(r#"</PARAMETER-VALUES><REFERENCE-VALUES>
                    <ECUC-REFERENCE-VALUE>
                      <DEFINITION-REF DEST="ECUC-FOREIGN-REFERENCE-DEF">/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu/SysTPduToFrameTriggeringRef</DEFINITION-REF>
                      <VALUE-REF DEST="CAN-FRAME-TRIGGERING">{trigger}</VALUE-REF>
                    </ECUC-REFERENCE-VALUE></REFERENCE-VALUES>"#))
                },
            );
            for (tag, ending) in [
                ("CAN-FRAME-TRIGGERING", "Trigger"),
                ("CAN-FRAME", "Frame"),
                ("N-PDU", "Pdu"),
            ] {
                xml_edit(
                    &mut inputs,
                    "extract.arxml",
                    |node| named(node, tag, &format!("{name}{ending}")),
                    |xml| {
                        xml.replace("<CAN-FD-FRAME-SUPPORT>false", "<CAN-FD-FRAME-SUPPORT>true")
                            .replace("<FRAME-LENGTH>8", &format!("<FRAME-LENGTH>{dl}"))
                            .replace("<LENGTH>8", &format!("<LENGTH>{dl}"))
                    },
                );
            }
        }
    }
    parameter(&mut inputs, "General", "CanTpPaddingByte", "204");
    inputs
}
