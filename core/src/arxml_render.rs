use crate::model::{Direction, FrameView, SignalView};
use std::fmt::Write;

fn ref_path(project: &str, name: &str) -> String {
    format!("/{project}/{name}")
}

fn param(name: &str, kind: &str, value: impl std::fmt::Display, definition: &str) -> String {
    format!("<ECUC-{kind}-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-{definition}-PARAM-DEF\">{name}</DEFINITION-REF><VALUE>{value}</VALUE></ECUC-{kind}-PARAM-VALUE>")
}

fn number(path: &str, name: &str, value: impl std::fmt::Display) -> String {
    param(&format!("{path}/{name}"), "NUMERICAL", value, "INTEGER")
}

fn decimal(path: &str, name: &str, milliseconds: u32) -> String {
    param(&format!("{path}/{name}"), "NUMERICAL", format!("{}.{:03}", milliseconds / 1000, milliseconds % 1000), "FLOAT")
}

fn choice(path: &str, name: &str, value: &str) -> String {
    param(&format!("{path}/{name}"), "TEXTUAL", value, "ENUMERATION")
}

fn text(path: &str, name: &str, value: impl std::fmt::Display) -> String {
    param(&format!("{path}/{name}"), "TEXTUAL", value, "STRING")
}

fn reference(path: &str, name: &str, dest: &str, target: &str) -> String {
    format!("<ECUC-REFERENCE-VALUE><DEFINITION-REF DEST=\"ECUC-REFERENCE-DEF\">{path}/{name}</DEFINITION-REF><VALUE-REF DEST=\"{dest}\">{target}</VALUE-REF></ECUC-REFERENCE-VALUE>")
}

fn container(name: &str, definition: &str, params: &str, refs: &str, children: &str) -> String {
    let mut value = format!("<ECUC-CONTAINER-VALUE><SHORT-NAME>{name}</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">{definition}</DEFINITION-REF>");
    if !params.is_empty() {
        write!(value, "<PARAMETER-VALUES>{params}</PARAMETER-VALUES>").unwrap();
    }
    if !refs.is_empty() {
        write!(value, "<REFERENCE-VALUES>{refs}</REFERENCE-VALUES>").unwrap();
    }
    if !children.is_empty() {
        write!(value, "<SUB-CONTAINERS>{children}</SUB-CONTAINERS>").unwrap();
    }
    value.push_str("</ECUC-CONTAINER-VALUE>");
    value
}

pub fn render_profile(project: &str, frames: &[FrameView], signals: &[SignalView]) -> String {
    let mut elements = String::new();
    let mut com_children = String::new();
    let mut canif_children = String::new();
    for (frame_index, frame) in frames.iter().enumerate() {
        let pdu_path = ref_path(project, &format!("Pdu_{}", frame.name));
        let frame_signals: Vec<_> = signals.iter().filter(|s| s.frame_path == frame.path).collect();
        let mut mappings = String::new();
        let mut com_refs = String::new();
        for signal in &frame_signals {
            write!(mappings, "<I-SIGNAL-TO-I-PDU-MAPPING><SHORT-NAME>Map_{}</SHORT-NAME><I-SIGNAL-REF DEST=\"I-SIGNAL\">{}</I-SIGNAL-REF><PACKING-BYTE-ORDER>MOST-SIGNIFICANT-BYTE-LAST</PACKING-BYTE-ORDER><START-POSITION>{}</START-POSITION></I-SIGNAL-TO-I-PDU-MAPPING>", signal.name, ref_path(project, &format!("ISignal_{}", signal.name)), signal.start_bit).unwrap();
            com_refs.push_str(&reference("/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu", "ComIPduSignalRef", "ECUC-CONTAINER-VALUE", &signal.path));
        }
        write!(elements, "<I-SIGNAL-I-PDU><SHORT-NAME>Pdu_{}</SHORT-NAME><LENGTH>{}</LENGTH>", frame.name, frame.dlc).unwrap();
        if !mappings.is_empty() {
            write!(elements, "<I-SIGNAL-TO-PDU-MAPPINGS>{mappings}</I-SIGNAL-TO-PDU-MAPPINGS>").unwrap();
        }
        elements.push_str("</I-SIGNAL-I-PDU>");
        let com_path = "/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu";
        let com_params = format!("{}{}", choice(com_path, "ComIPduDirection", match frame.direction { Direction::Tx => "SEND", Direction::Rx => "RECEIVE" }), number(com_path, "ComIPduHandleId", frame_index));
        let com_pdu_ref = reference(com_path, "ComPduIdRef", "I-SIGNAL-I-PDU", &pdu_path);
        let tx_children = if let Some(period) = frame.period_ms {
            let mode_path = format!("{com_path}/ComTxIPdu/ComTxModeTrue/ComTxMode");
            let mode = container("Mode", &mode_path, &format!("{}{}", choice(&mode_path, "ComTxModeMode", "PERIODIC"), decimal(&mode_path, "ComTxModeTimePeriod", period)), "", "");
            let true_mode = container("TrueMode", &format!("{com_path}/ComTxIPdu/ComTxModeTrue"), "", "", &mode);
            container("Tx", &format!("{com_path}/ComTxIPdu"), "", "", &true_mode)
        } else { String::new() };
        com_children.push_str(&container(&format!("Pdu_{}", frame.name), com_path, &com_params, &(com_pdu_ref + &com_refs), &tx_children));
        let (canif_def, canif_params, canif_ref) = match frame.direction {
            Direction::Tx => {
                let path = "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfTxPduCfg";
                (path, format!("{}{}{}", number(path, "CanIfTxPduCanId", frame.id), number(path, "CanIfTxPduId", frame_index), choice(path, "CanIfTxPduCanIdType", "STANDARD_CAN")), reference(path, "CanIfTxPduRef", "I-SIGNAL-I-PDU", &pdu_path))
            }
            Direction::Rx => {
                let path = "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfRxPduCfg";
                (path, format!("{}{}{}", number(path, "CanIfRxPduCanId", frame.id), number(path, "CanIfRxPduId", frame_index), number(path, "CanIfRxPduDataLength", frame.dlc)), reference(path, "CanIfRxPduRef", "I-SIGNAL-I-PDU", &pdu_path))
            }
        };
        canif_children.push_str(&container(&format!("Can_{}", frame.name), canif_def, &canif_params, &canif_ref, ""));
    }
    for (signal_index, signal) in signals.iter().enumerate() {
        write!(elements, "<I-SIGNAL><SHORT-NAME>ISignal_{}</SHORT-NAME><LENGTH>{}</LENGTH></I-SIGNAL>", signal.name, signal.length).unwrap();
        let path = "/AUTOSAR/EcucDefs/Com/ComConfig/ComSignal";
        let ty = if signal.length == 1 { "BOOLEAN" } else if signal.length <= 8 { "UINT8" } else if signal.length <= 16 { "UINT16" } else { "UINT32" };
        let mut params = format!("{}{}{}{}{}{}", number(path, "ComBitPosition", signal.start_bit), number(path, "ComBitSize", signal.length), number(path, "ComHandleId", signal_index), choice(path, "ComSignalEndianness", "LITTLE_ENDIAN"), text(path, "ComSignalInitValue", signal.initial_value), choice(path, "ComSignalType", ty));
        if let Some(frame) = frames.iter().find(|f| f.path == signal.frame_path) {
            if let Some(timeout) = frame.timeout_ms {
                params.push_str(&decimal(path, "ComTimeout", timeout));
            }
        }
        com_children.push_str(&container(&signal.name, path, &params, "", ""));
    }
    if !com_children.is_empty() {
        let config = container("ComConfig", "/AUTOSAR/EcucDefs/Com/ComConfig", "", "", &com_children);
        write!(elements, "<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>ComCfg</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-MODULE-DEF\">/AUTOSAR/EcucDefs/Com</DEFINITION-REF><CONTAINERS>{config}</CONTAINERS></ECUC-MODULE-CONFIGURATION-VALUES>").unwrap();
    }
    if !canif_children.is_empty() {
        let config = container("CanIfInitCfg", "/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg", "", "", &canif_children);
        write!(elements, "<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>CanIfCfg</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-MODULE-DEF\">/AUTOSAR/EcucDefs/CanIf</DEFINITION-REF><CONTAINERS>{config}</CONTAINERS></ECUC-MODULE-CONFIGURATION-VALUES>").unwrap();
    }
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>{project}</SHORT-NAME><ELEMENTS>{elements}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>\n")
}
