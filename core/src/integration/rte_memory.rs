//! RTE-owned receiver allocations shared by C emission and the BSW description.
use super::composition::{CompositionContract, Endpoint};

pub(super) struct Receiver {
    pub endpoint: Endpoint,
    pub scope: String,
    pub name: String,
    pub native_type: String,
    pub initial_value: u32,
    pub received: bool,
    pub expired: bool,
}

pub(super) fn receivers(composition: &CompositionContract) -> Vec<Receiver> {
    let mut output = Vec::new();
    for component in &composition.components {
        for port in component.data_ports.iter().filter(|port| port.read) {
            let endpoint = Endpoint {
                instance: component.instance.clone(),
                port: port.path.clone(),
                member: port.element.clone(),
            };
            let network = composition
                .network_endpoints
                .iter()
                .any(|network| network.endpoint == endpoint);
            output.push(Receiver {
                received: network
                    || (port.handle_never_received && composition.data_connected(&endpoint)),
                expired: network,
                endpoint,
                scope: super::short_name(&component.component).into(),
                name: format!("rte_value_{}", output.len()),
                native_type: port.c_type.clone(),
                initial_value: port.initial_value,
            });
        }
    }
    output
}

#[derive(Default)]
pub(super) struct Description {
    pub constants: String,
    pub variables: String,
    pub types: String,
}

/// InternalBehavior owns variables and constants; MemorySections alone do not describe objects.
pub(super) fn description(
    plan: &super::PlanDescription,
) -> Result<Description, Vec<super::PlanDiagnostic>> {
    use std::collections::BTreeSet;
    let composition = plan.ecu_composition()?;
    let mut scalars = BTreeSet::from([
        "boolean".to_owned(),
        "uint8".to_owned(),
        "uint16".to_owned(),
    ]);
    let mut variables = String::new();
    let mut constants = String::new();
    let object = |tag: &str, name: &str, datatype: &str| {
        format!(
            "<{tag}><SHORT-NAME>{name}</SHORT-NAME><TYPE-TREF DEST=\"IMPLEMENTATION-DATA-TYPE\">/AUTOSAR_Rte/{datatype}</TYPE-TREF></{tag}>"
        )
    };
    for (name, datatype) in [
        ("rte_lifecycle", "RteMemory_uint8"),
        ("schm_lifecycle", "RteMemory_uint8"),
    ] {
        variables.push_str(&object("VARIABLE-DATA-PROTOTYPE", name, datatype));
    }
    for receiver in receivers(composition) {
        scalars.insert(receiver.native_type.clone());
        variables.push_str(&object(
            "VARIABLE-DATA-PROTOTYPE",
            &receiver.name,
            &format!("RteMemory_{}", receiver.native_type),
        ));
        if receiver.received {
            variables.push_str(&object(
                "VARIABLE-DATA-PROTOTYPE",
                &format!("{}_received", receiver.name),
                "RteMemory_boolean",
            ));
        }
        if receiver.expired {
            variables.push_str(&object(
                "VARIABLE-DATA-PROTOTYPE",
                &format!("{}_expired", receiver.name),
                "RteMemory_boolean",
            ));
        }
    }
    for name in ["diagnostic_session_mode", "authentication_mode"] {
        variables.push_str(&object("VARIABLE-DATA-PROTOTYPE", name, "RteMemory_uint8"));
    }
    let props = |body: &str| {
        format!(
            "<SW-DATA-DEF-PROPS><SW-DATA-DEF-PROPS-VARIANTS><SW-DATA-DEF-PROPS-CONDITIONAL>{body}</SW-DATA-DEF-PROPS-CONDITIONAL></SW-DATA-DEF-PROPS-VARIANTS></SW-DATA-DEF-PROPS>"
        )
    };
    let mut types = String::new();
    for native in scalars {
        let underlying = composition
            .types
            .iter()
            .find(|datatype| datatype.symbol == native)
            .map_or(native.as_str(), |datatype| datatype.native_type.as_str());
        let bits = match underlying {
            "boolean" | "uint8" | "sint8" => 8,
            "uint16" | "sint16" => 16,
            "uint64" | "sint64" => 64,
            _ => 32,
        };
        let encoding = if underlying == "boolean" {
            "BOOLEAN"
        } else if underlying.starts_with("sint") {
            "2C"
        } else {
            "NONE"
        };
        types.push_str(&format!("<SW-BASE-TYPE><SHORT-NAME>RteMemoryBase_{native}</SHORT-NAME><CATEGORY>FIXED_LENGTH</CATEGORY><BASE-TYPE-SIZE>{bits}</BASE-TYPE-SIZE><BASE-TYPE-ENCODING>{encoding}</BASE-TYPE-ENCODING><NATIVE-DECLARATION>{native}</NATIVE-DECLARATION></SW-BASE-TYPE><IMPLEMENTATION-DATA-TYPE><SHORT-NAME>RteMemory_{native}</SHORT-NAME><CATEGORY>VALUE</CATEGORY>{}</IMPLEMENTATION-DATA-TYPE>", props(&format!("<BASE-TYPE-REF DEST=\"SW-BASE-TYPE\">/AUTOSAR_Rte/RteMemoryBase_{native}</BASE-TYPE-REF>"))));
    }
    let element = |name: &str, target: &str| {
        format!(
            "<IMPLEMENTATION-DATA-TYPE-ELEMENT><SHORT-NAME>{name}</SHORT-NAME><CATEGORY>TYPE_REFERENCE</CATEGORY>{}</IMPLEMENTATION-DATA-TYPE-ELEMENT>",
            props(&format!(
                "<IMPLEMENTATION-DATA-TYPE-REF DEST=\"IMPLEMENTATION-DATA-TYPE\">/AUTOSAR_Rte/{target}</IMPLEMENTATION-DATA-TYPE-REF>"
            ))
        )
    };
    types.push_str(&format!("<IMPLEMENTATION-DATA-TYPE><SHORT-NAME>SchM_ConfigType</SHORT-NAME><CATEGORY>STRUCTURE</CATEGORY><SUB-ELEMENTS>{}</SUB-ELEMENTS></IMPLEMENTATION-DATA-TYPE>", element("partition", "RteMemory_uint16")));
    constants.push_str(&object(
        "PARAMETER-DATA-PROTOTYPE",
        "Ecu_SchMConfig",
        "SchM_ConfigType",
    ));
    for component in &composition.components {
        let scope = super::short_name(&component.component);
        let cds = format!("Rte_CDS_{scope}");
        let pointer = format!("RteMemoryPointer_{scope}");
        types.push_str(&format!("<IMPLEMENTATION-DATA-TYPE><SHORT-NAME>{cds}</SHORT-NAME><CATEGORY>STRUCTURE</CATEGORY><SUB-ELEMENTS>{}</SUB-ELEMENTS></IMPLEMENTATION-DATA-TYPE>", element("Rte_Dummy", "RteMemory_uint8")));
        let target = props(&format!(
            "<IMPLEMENTATION-DATA-TYPE-REF DEST=\"IMPLEMENTATION-DATA-TYPE\">/AUTOSAR_Rte/{cds}</IMPLEMENTATION-DATA-TYPE-REF><SW-IMPL-POLICY>CONST</SW-IMPL-POLICY>"
        ));
        let pointer_props = props(&format!(
            "<SW-POINTER-TARGET-PROPS><TARGET-CATEGORY>STRUCTURE</TARGET-CATEGORY>{target}</SW-POINTER-TARGET-PROPS>"
        ));
        types.push_str(&format!("<IMPLEMENTATION-DATA-TYPE><SHORT-NAME>{pointer}</SHORT-NAME><CATEGORY>DATA_REFERENCE</CATEGORY>{pointer_props}</IMPLEMENTATION-DATA-TYPE>"));
        constants.push_str(&object(
            "PARAMETER-DATA-PROTOTYPE",
            &format!("Rte_Instance_{scope}"),
            &cds,
        ));
        constants.push_str(&object(
            "PARAMETER-DATA-PROTOTYPE",
            &format!("Rte_Inst_{scope}"),
            &pointer,
        ));
    }
    Ok(Description {
        constants: format!("<CONSTANT-MEMORYS>{constants}</CONSTANT-MEMORYS>"),
        variables: format!("<STATIC-MEMORYS>{variables}</STATIC-MEMORYS>"),
        types,
    })
}
