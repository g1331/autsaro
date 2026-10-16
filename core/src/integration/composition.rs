//! Source-derived component composition contract for the bounded flat, single-owner profile.
use super::component::{DataPort, c_name, implementation, milliseconds, name, one, referenced};
use super::graph::Graph;
use super::{ContractArgument, DiagnosticCategory, PlanDiagnostic, SymbolContract};
use serde::Serialize;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

pub const PROFILE: &str = "singlecore-ecu-v1";

// Fixed R24-11 Dcm declarations. Dormant groups do not enable their services.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DcmModeDefinition {
    pub group: Cow<'static, str>,
    pub port: Cow<'static, str>,
    pub prototype: Cow<'static, str>,
    pub modes: &'static [&'static str],
}
pub(super) const DCM_MODES: &[DcmModeDefinition] = &[
    DcmModeDefinition {
        group: Cow::Borrowed("DcmDiagnosticSessionControl"),
        port: Cow::Borrowed("DiagnosticSessionControlModeSwitchInterface"),
        prototype: Cow::Borrowed("diagnosticSession"),
        modes: &[
            "DCM_DEFAULT_SESSION",
            "DCM_PROGRAMMING_SESSION",
            "DCM_EXTENDED_DIAGNOSTIC_SESSION",
            "DCM_SAFETY_SYSTEM_DIAGNOSTIC_SESSION",
        ],
    },
    DcmModeDefinition {
        group: Cow::Borrowed("DcmEcuReset"),
        port: Cow::Borrowed("EcuResetModeSwitchInterface"),
        prototype: Cow::Borrowed("ecuReset"),
        modes: &[
            "DCM_NONE",
            "DCM_HARD",
            "DCM_KEYONOFF",
            "DCM_SOFT",
            "DCM_JUMPTOBOOTLOADER",
            "DCM_JUMPTOSYSSUPPLIERBOOTLOADER",
            "DCM_EXECUTE",
        ],
    },
    DcmModeDefinition {
        group: Cow::Borrowed("DcmModeRapidPowerShutDown"),
        port: Cow::Borrowed("ModeRapidPowerShutDownModeSwitchInterface"),
        prototype: Cow::Borrowed("modeRapidPowerShutDown"),
        modes: &[
            "DCM_ENABLE_RAPIDPOWERSHUTDOWN",
            "DCM_DISABLE_RAPIDPOWERSHUTDOWN",
        ],
    },
    DcmModeDefinition {
        group: Cow::Borrowed("DcmControlDTCSetting"),
        port: Cow::Borrowed("ControlDTCSettingModeSwitchInterface"),
        prototype: Cow::Borrowed("controlDTCSetting"),
        modes: &["DCM_ENABLEDTCSETTING", "DCM_DISABLEDTCSETTING"],
    },
    DcmModeDefinition {
        group: Cow::Borrowed("DcmSecurityAccess"),
        port: Cow::Borrowed("SecurityAccessModeSwitchInterface"),
        prototype: Cow::Borrowed("securityAccess"),
        modes: &["DCM_SEC_LEV_LOCKED"],
    },
];

#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub instance: String,
    pub port: String,
    pub member: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub path: String,
    pub provider: Endpoint,
    pub requester: Endpoint,
    pub service: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub port: String,
    pub interface: String,
    pub operation: String,
    pub application_error: Option<String>,
    pub read: bool,
    pub api_symbol: String,
    pub implementation_symbol: String,
    pub arguments: Vec<ContractArgument>,
    pub argument_types: Vec<String>,
    pub runnable: String,
    pub callers: Vec<String>,
    pub call_points: Vec<String>,
    pub event: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Runnable {
    pub path: String,
    pub symbol: String,
    pub event: Option<String>,
    pub period_ms: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataAccess {
    pub path: String,
    pub endpoint: Endpoint,
    pub runnable: String,
    pub read: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticSessionPort {
    pub port: String,
    pub interface: String,
    pub prototype: String,
    pub mode_group: String,
    pub mode_type: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    pub component: String,
    pub instance: String,
    pub ecu: String,
    pub ecu_mapping: String,
    pub rte_instance: String,
    pub behavior: String,
    pub header: String,
    pub interfaces: BTreeMap<String, String>,
    pub diagnostic_session_port: Option<DiagnosticSessionPort>,
    pub diagnostic_mode_ports: Vec<DiagnosticSessionPort>,
    pub data_ports: Vec<DataPort>,
    pub data_accesses: Vec<DataAccess>,
    pub operations: Vec<Operation>,
    pub runnables: Vec<Runnable>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkEndpoint {
    pub endpoint: Endpoint,
    pub network: String,
    pub channel: String,
    pub transport: super::SignalChannel,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionContract {
    pub system: String,
    pub composition: String,
    pub ecu: String,
    pub generation: super::rte_configuration::RteGenerationContract,
    pub components: Vec<Component>,
    pub connections: Vec<Connection>,
    pub network_endpoints: Vec<NetworkEndpoint>,
    pub array_types: BTreeMap<String, String>,
    pub types: Vec<super::rte_types::RteDataType>,
    pub dcm_modes: Vec<DcmModeDefinition>,
}

pub(super) fn dcm_data_type_names(data: &str) -> [String; 3] {
    let data = c_name(super::short_name(data));
    [
        format!("Dcm_DataElement_{data}Type"),
        format!("Dcm_DataElement_{data}_ArrayType"),
        format!("Dcm_DataArrayTypeUint8_{data}Type"),
    ]
}

impl CompositionContract {
    pub(super) fn data_connected(&self, endpoint: &Endpoint) -> bool {
        self.network_endpoints
            .iter()
            .any(|network| &network.endpoint == endpoint)
            || self.connections.iter().any(|connection| {
                !connection.service
                    && (&connection.provider == endpoint || &connection.requester == endpoint)
            })
    }
    /// Standard names for the implemented four-byte synchronous Dcm data interface.
    pub(super) fn diagnostic_types(
        &self,
        diagnostic: &super::diagnostic::DiagnosticContract,
    ) -> Result<(String, String), Vec<PlanDiagnostic>> {
        let native = self
            .components
            .iter()
            .flat_map(|component| &component.operations)
            .find(|operation| operation.port == diagnostic.client_port)
            .and_then(|operation| operation.arguments.first())
            .map(|argument| argument.native_type.clone())
            .ok_or_else(|| PlanDiagnostic::plan_premise(&diagnostic.client_port))?;
        let mut parameter = native;
        let mut typedefs = String::new();
        for alias in dcm_data_type_names(&diagnostic.data) {
            // Rte_Type.h already declares source array types, all checked as uint8[4].
            if !self.array_types.values().any(|native| native == &alias) {
                typedefs.push_str(&format!("typedef {parameter} {alias};\n"));
                parameter = alias;
            }
        }
        let declarations = if typedefs.is_empty() {
            typedefs
        } else {
            format!(
                "/** Selected UINT8_N data (SWS_Dcm_91051, SWS_Dcm_91063, SWS_Dcm_01121). */\n{typedefs}"
            )
        };
        Ok((declarations, parameter))
    }

    /// The Dcm authentication state mode group every planned composition declares.
    pub(super) fn authentication_modes(&self) -> Result<&DcmModeDefinition, Vec<PlanDiagnostic>> {
        self.dcm_modes
            .iter()
            .find(|definition| definition.group.starts_with("DcmAuthenticationState_"))
            .ok_or_else(|| PlanDiagnostic::plan_premise(&self.composition))
    }
}

fn fail(graph: &Graph, index: usize, code: &str) -> Vec<PlanDiagnostic> {
    vec![graph.diagnostic(
        index,
        DiagnosticCategory::Input,
        code,
        crate::product_message!("backend.integration.composition.contract_invalid", "code" => code),
        crate::product_message!("backend.integration.composition.repair_contract"),
    )]
}

fn path(graph: &Graph, index: usize) -> String {
    graph.elements[index].object.clone()
}

fn reference_descendant(
    graph: &Graph,
    index: usize,
    tag: &str,
) -> Result<usize, Vec<PlanDiagnostic>> {
    let reference = one(
        graph,
        index,
        graph.descendants(index, tag),
        "REFERENCE_UNRESOLVED",
    )?;
    graph
        .objects
        .get(&graph.elements[reference].text)
        .copied()
        .ok_or_else(|| fail(graph, reference, "REFERENCE_UNRESOLVED"))
}

pub(super) fn zero_seconds(value: &str) -> bool {
    let mantissa = match value.split_once(['e', 'E']) {
        Some((mantissa, exponent)) if exponent.parse::<i32>().is_ok() => mantissa,
        Some(_) => return false,
        None => value,
    };
    let mantissa = mantissa
        .strip_prefix('+')
        .or_else(|| mantissa.strip_prefix('-'))
        .unwrap_or(mantissa);
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    (!whole.is_empty() || !fraction.is_empty())
        && whole
            .bytes()
            .chain(fraction.bytes())
            .all(|byte| byte == b'0')
}

fn unsigned_encoding(graph: &Graph, implementation: usize) -> Result<(), Vec<PlanDiagnostic>> {
    if graph
        .text(implementation, "TYPE-EMITTER")
        .is_some_and(|emitter| emitter != "RTE")
    {
        return Err(fail(graph, implementation, "TYPE_CONFLICT"));
    }
    let base = reference_descendant(graph, implementation, "BASE-TYPE-REF")?;
    if graph.text(base, "BASE-TYPE-ENCODING") != Some("NONE")
        || graph.text(base, "CATEGORY") != Some("FIXED_LENGTH")
    {
        return Err(fail(graph, base, "TYPE_CONFLICT"));
    }
    Ok(())
}

fn native_type(
    graph: &Graph,
    argument: usize,
    behavior: usize,
    arrays: &mut BTreeMap<String, String>,
) -> Result<(String, String), Vec<PlanDiagnostic>> {
    let datatype = referenced(graph, argument, "TYPE-TREF", "TYPE_MAP_MISSING")?;
    if graph.elements[datatype].tag == "APPLICATION-PRIMITIVE-DATA-TYPE" {
        if graph.text(datatype, "CATEGORY") != Some("VALUE") {
            return Err(fail(graph, datatype, "TYPE_CONFLICT"));
        }
        let mapped = implementation(graph, argument, behavior, datatype)?;
        unsigned_encoding(graph, mapped)?;
        return Ok((
            c_name(&name(graph, mapped)),
            format!("{}:{}", path(graph, datatype), path(graph, mapped)),
        ));
    }
    if graph.elements[datatype].tag == "IMPLEMENTATION-DATA-TYPE"
        && graph.text(datatype, "CATEGORY") == Some("VALUE")
    {
        let base = reference_descendant(graph, datatype, "BASE-TYPE-REF")?;
        unsigned_encoding(graph, datatype)?;
        if graph.text(base, "NATIVE-DECLARATION") == Some("uint32")
            && graph.text(base, "BASE-TYPE-SIZE") == Some("32")
        {
            return Ok((c_name(&name(graph, datatype)), path(graph, datatype)));
        }
    }
    if graph
        .text(datatype, "TYPE-EMITTER")
        .is_some_and(|emitter| emitter != "RTE")
    {
        return Err(fail(graph, datatype, "TYPE_CONFLICT"));
    }
    if graph.elements[datatype].tag != "IMPLEMENTATION-DATA-TYPE"
        || graph.text(datatype, "CATEGORY") != Some("ARRAY")
    {
        return Err(fail(graph, argument, "SERVICE_TYPE_CONFLICT"));
    }
    let element = one(
        graph,
        datatype,
        graph.descendants(datatype, "IMPLEMENTATION-DATA-TYPE-ELEMENT"),
        "SERVICE_TYPE_CONFLICT",
    )?;
    let element_type = reference_descendant(graph, element, "IMPLEMENTATION-DATA-TYPE-REF")?;
    let base = reference_descendant(graph, element_type, "BASE-TYPE-REF")?;
    unsigned_encoding(graph, element_type)?;
    if graph.text(element, "CATEGORY") != Some("TYPE_REFERENCE")
        || graph.text(element, "ARRAY-SIZE") != Some("4")
        || graph.text(element, "ARRAY-SIZE-SEMANTICS") != Some("FIXED-SIZE")
        || graph.text(element_type, "CATEGORY") != Some("VALUE")
        || graph.text(base, "BASE-TYPE-SIZE") != Some("8")
        || graph.text(base, "NATIVE-DECLARATION") != Some("uint8")
        || graph.text(argument, "DIRECTION") != Some("OUT")
    {
        return Err(fail(graph, argument, "SERVICE_TYPE_CONFLICT"));
    }
    let native = c_name(&name(graph, datatype));
    arrays.insert(path(graph, datatype), native.clone());
    Ok((native, path(graph, datatype)))
}

// The selected Dcm ReadData C/S operation includes E_NOT_OK even though its
// application server must always supply valid data (SWS_Dcm_00686).
fn application_error(
    graph: &Graph,
    interface: usize,
    operation: usize,
) -> Result<Option<String>, Vec<PlanDiagnostic>> {
    let interface_name = name(graph, interface);
    let dcm_read = name(graph, operation) == "ReadData"
        && interface_name.starts_with("DataServices_")
        && (graph.text(interface, "SERVICE-KIND") == Some("DIAGNOSTIC-COMMUNICATION-MANAGER")
            || graph
                .of_kind("ECUC-CONTAINER-VALUE")
                .into_iter()
                .any(|index| {
                    graph.text(index, "DEFINITION-REF")
                        == Some("/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsp/DcmDspData")
                        && interface_name == format!("DataServices_{}", name(graph, index))
                }));
    let errors = graph.descendants(operation, "POSSIBLE-ERROR-REF");
    if !dcm_read && errors.is_empty() {
        return Ok(None);
    }
    if dcm_read && errors.len() == 1 {
        let error = graph.objects.get(&graph.elements[errors[0]].text).copied();
        if error.is_some_and(|error| {
            graph.within(error, interface)
                && graph.elements[error].tag == "APPLICATION-ERROR"
                && name(graph, error) == "E_NOT_OK"
                && graph.text(error, "ERROR-CODE").is_some_and(|code| {
                    let code = code.trim();
                    let (digits, radix) = if let Some(hex) =
                        code.strip_prefix("0x").or_else(|| code.strip_prefix("0X"))
                    {
                        (hex, 16)
                    } else if let Some(binary) =
                        code.strip_prefix("0b").or_else(|| code.strip_prefix("0B"))
                    {
                        (binary, 2)
                    } else {
                        (code, 10)
                    };
                    u8::from_str_radix(digits, radix) == Ok(1)
                })
        }) {
            return Ok(Some(format!("RTE_E_{}_E_NOT_OK", c_name(&interface_name))));
        }
    }
    Err(fail(graph, operation, "APPLICATION_ERROR_UNSUPPORTED"))
}

pub(super) fn dcm_mode_definitions(
    graph: &Graph,
    context: usize,
) -> Result<Vec<DcmModeDefinition>, Vec<PlanDiagnostic>> {
    let mut connections: Vec<_> = graph.of_kind("ECUC-CONTAINER-VALUE").into_iter().filter(|index| graph.text(*index, "DEFINITION-REF") == Some("/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsl/DcmDslProtocol/DcmDslProtocolRow/DcmDslConnection/DcmDslMainConnection")).collect();
    connections.sort_unstable();
    // A second declaration in source order is the unsupported connection, not a BSW entry.
    let connection_context = connections.get(1).copied().unwrap_or(context);
    let connection = one(
        graph,
        connection_context,
        connections,
        "DIAGNOSTIC_REFERENCE",
    )?;
    let suffix = name(graph, connection);
    let mut definitions = DCM_MODES.to_vec();
    definitions.push(DcmModeDefinition {
        group: Cow::Owned(format!("DcmAuthenticationState_{suffix}")),
        port: Cow::Owned(format!("AuthenticationStateModeSwitchInterface_{suffix}")),
        prototype: Cow::Borrowed("authenticationState"),
        modes: &["DCM_DEAUTHENTICATED", "DCM_AUTHENTICATED"],
    });
    Ok(definitions)
}

fn inspect_dcm_declarations(
    graph: &Graph,
    context: usize,
    definitions: &[DcmModeDefinition],
) -> Result<(), Vec<PlanDiagnostic>> {
    let dcm = graph
        .objects
        .get("/AUTOSAR_Dcm/Dcm")
        .copied()
        .ok_or_else(|| fail(graph, context, "TYPE_CONFLICT"))?;
    let provided = graph.descendants(dcm, "MODE-DECLARATION-GROUP-PROTOTYPE");
    if provided.len() != definitions.len() {
        return Err(fail(graph, dcm, "TYPE_CONFLICT"));
    }
    for definition in definitions {
        let group = graph
            .objects
            .get(&format!("/AUTOSAR_Dcm/{}", definition.group))
            .copied()
            .ok_or_else(|| fail(graph, dcm, "TYPE_CONFLICT"))?;
        let prototype = one(
            graph,
            dcm,
            provided
                .iter()
                .copied()
                .filter(|index| name(graph, *index) == definition.group)
                .collect(),
            "TYPE_CONFLICT",
        )?;
        let modes = graph.descendants(group, "MODE-DECLARATION");
        if graph.target(prototype, "TYPE-TREF") != Some(group)
            || graph.text(group, "CATEGORY") != Some("EXPLICIT_ORDER")
            || graph.text(group, "ON-TRANSITION-VALUE") != Some("255")
            || graph.target(group, "INITIAL-MODE-REF").is_none_or(|index| {
                !modes.contains(&index) || name(graph, index) != definition.modes[0]
            })
            || modes.len() != definition.modes.len()
            || !definition.modes.iter().enumerate().all(|(value, mode)| {
                modes.iter().any(|index| {
                    name(graph, *index) == *mode
                        && graph.text(*index, "VALUE") == Some(value.to_string().as_str())
                })
            })
        {
            return Err(fail(graph, group, "TYPE_CONFLICT"));
        }
    }
    Ok(())
}

pub(super) fn inspect(graph: &Graph) -> Result<CompositionContract, Vec<PlanDiagnostic>> {
    inspect_with_transport(graph, true)
}

pub(super) fn inspect_ownership(graph: &Graph) -> Result<CompositionContract, Vec<PlanDiagnostic>> {
    inspect_with_transport(graph, false)
}

fn inspect_with_transport(
    graph: &Graph,
    include_transport: bool,
) -> Result<CompositionContract, Vec<PlanDiagnostic>> {
    let generation = super::rte_configuration::inspect(graph)?;
    let context = *graph
        .objects
        .values()
        .next()
        .ok_or_else(Vec::<PlanDiagnostic>::new)?;
    let dcm_modes = dcm_mode_definitions(graph, context)?;
    inspect_dcm_declarations(graph, context, &dcm_modes)?;
    let system = one(graph, context, graph.of_kind("SYSTEM"), "TARGET_NOT_UNIQUE")?;
    if graph.text(system, "CATEGORY") != Some("ECU_EXTRACT") {
        return Err(fail(graph, system, "TARGET_CATEGORY"));
    }
    let ecu = one(
        graph,
        system,
        graph.of_kind("ECU-INSTANCE"),
        "TARGET_NOT_UNIQUE",
    )?;
    let root = one(
        graph,
        system,
        graph.descendants(system, "ROOT-SW-COMPOSITION-PROTOTYPE"),
        "MULTIPLE_INSTANCES",
    )?;
    let composition = referenced(
        graph,
        root,
        "SOFTWARE-COMPOSITION-TREF",
        "MULTIPLE_INSTANCES",
    )?;
    if graph.elements[composition].tag != "COMPOSITION-SW-COMPONENT-TYPE"
        || !graph
            .descendants(composition, "DELEGATION-SW-CONNECTOR")
            .is_empty()
    {
        return Err(fail(graph, composition, "NESTED_COMPOSITION_UNSUPPORTED"));
    }
    // N >= 1: service components alone own no periodic schedule. Unresolved
    // types are left to the per-instance reference diagnostics below.
    if !graph
        .descendants(composition, "SW-COMPONENT-PROTOTYPE")
        .into_iter()
        .any(|instance| {
            graph.target(instance, "TYPE-TREF").is_none_or(|component| {
                graph.elements[component].tag == "APPLICATION-SW-COMPONENT-TYPE"
            })
        })
    {
        return Err(fail(graph, composition, "APPLICATION_COMPONENT_MISSING"));
    }
    let mut components = Vec::new();
    let mut types = BTreeSet::new();
    let mut arrays = BTreeMap::new();
    let mut common_period = None;
    for instance in graph.descendants(composition, "SW-COMPONENT-PROTOTYPE") {
        let component = referenced(graph, instance, "TYPE-TREF", "MULTIPLE_INSTANCES")?;
        if !matches!(
            graph.elements[component].tag.as_str(),
            "APPLICATION-SW-COMPONENT-TYPE" | "SERVICE-SW-COMPONENT-TYPE"
        ) || !types.insert(component)
        {
            return Err(fail(graph, instance, "MULTIPLE_INSTANCES"));
        }
        let behavior = one(
            graph,
            component,
            graph.descendants(component, "SWC-INTERNAL-BEHAVIOR"),
            "TYPE_CONFLICT",
        )?;
        if !matches!(
            graph.text(behavior, "SUPPORTS-MULTIPLE-INSTANTIATION"),
            Some("false" | "0")
        ) {
            return Err(fail(graph, behavior, "MULTIPLE_INSTANCES"));
        }
        for tag in [
            "QUEUED-RECEIVER-COM-SPEC",
            "QUEUED-SENDER-COM-SPEC",
            "ASYNCHRONOUS-SERVER-CALL-POINT",
            "MODE-ACCESS-POINTS",
            "DATA-READ-ACCESSS",
            "DATA-WRITE-ACCESSS",
            "VARIATION-POINT",
            "EXCLUSIVE-AREA",
            "EXCLUSIVE-AREA-REF",
            "CAN-ENTER-EXCLUSIVE-AREA-REF",
            "RUNS-INSIDE-EXCLUSIVE-AREA-REF",
            "EXCLUSIVE-AREA-NESTING-ORDER-REF",
        ] {
            if let Some(index) = graph.descendants(component, tag).first() {
                return Err(fail(graph, *index, "UNSUPPORTED_COMPONENT_FEATURE"));
            }
        }
        let mapping = one(
            graph,
            instance,
            graph
                .descendants(system, "SWC-TO-ECU-MAPPING")
                .into_iter()
                .filter(|mapping| {
                    graph
                        .descendants(*mapping, "TARGET-COMPONENT-REF")
                        .iter()
                        .any(|reference| graph.elements[*reference].text == path(graph, instance))
                })
                .collect(),
            "ECU_MAPPING",
        )?;
        if graph.target(mapping, "ECU-INSTANCE-REF") != Some(ecu) {
            return Err(fail(graph, mapping, "ECU_MAPPING"));
        }
        let rte_instance = one(
            graph,
            instance,
            graph
                .of_kind("ECUC-CONTAINER-VALUE")
                .into_iter()
                .filter(|index| {
                    super::schedule::definition_is(graph, *index, "RteSwComponentInstance")
                        && super::schedule::value(
                            graph,
                            *index,
                            "RteSoftwareComponentInstanceRef",
                            true,
                        ) == Some(graph.elements[instance].object.as_str())
                })
                .collect(),
            "INSTANCE_MAPPING",
        )?;
        for events in graph.children(behavior, "EVENTS") {
            for event in &graph.elements[events].children {
                if !matches!(
                    graph.elements[*event].tag.as_str(),
                    "TIMING-EVENT" | "OPERATION-INVOKED-EVENT"
                ) {
                    return Err(fail(graph, *event, "EVENT_UNSUPPORTED"));
                }
            }
        }
        let mut runnables = Vec::new();
        for runnable in graph.descendants(behavior, "RUNNABLE-ENTITY") {
            if graph
                .text(runnable, "MINIMUM-START-INTERVAL")
                .is_some_and(|value| !zero_seconds(value))
            {
                return Err(fail(graph, runnable, "MINIMUM_START_INTERVAL_UNSUPPORTED"));
            }
            if !matches!(
                graph.text(runnable, "CAN-BE-INVOKED-CONCURRENTLY"),
                Some("false" | "0")
            ) {
                return Err(fail(graph, runnable, "REENTRANCY_UNSUPPORTED"));
            }
            let events = graph
                .descendants(behavior, "TIMING-EVENT")
                .into_iter()
                .chain(graph.descendants(behavior, "OPERATION-INVOKED-EVENT"))
                .filter(|event| graph.target(*event, "START-ON-EVENT-REF") == Some(runnable))
                .collect::<Vec<_>>();
            // The existing Dcm client is invoked by BSW rather than an application event.
            if events.is_empty() && graph.elements[component].tag == "SERVICE-SW-COMPONENT-TYPE" {
                let symbol = graph.text(runnable, "SYMBOL").unwrap_or("").to_owned();
                if !super::catalog::c_identifier(&symbol) {
                    return Err(fail(graph, runnable, "SYMBOL_PRODUCER_DUPLICATE"));
                }
                runnables.push(Runnable {
                    path: path(graph, runnable),
                    symbol,
                    event: None,
                    period_ms: None,
                });
                continue;
            }
            let event = one(graph, runnable, events, "SCHEDULE_NOT_UNIQUE")?;
            let period = if graph.elements[event].tag == "TIMING-EVENT" {
                let period = graph
                    .text(event, "PERIOD")
                    .and_then(milliseconds)
                    .ok_or_else(|| fail(graph, event, "PERIOD_UNSUPPORTED"))?;
                if graph
                    .text(event, "OFFSET")
                    .is_some_and(|value| !zero_seconds(value))
                    || common_period.is_some_and(|value| value != period)
                {
                    return Err(fail(graph, event, "PERIOD_UNSUPPORTED"));
                }
                common_period = Some(period);
                Some(period)
            } else {
                None
            };
            let symbol = graph.text(runnable, "SYMBOL").unwrap_or("").to_owned();
            if !super::catalog::c_identifier(&symbol) {
                return Err(fail(graph, runnable, "SYMBOL_PRODUCER_DUPLICATE"));
            }
            runnables.push(Runnable {
                path: path(graph, runnable),
                symbol,
                event: Some(path(graph, event)),
                period_ms: period,
            });
        }
        // Every event must target a runnable owned by this behavior.
        for event in graph
            .descendants(behavior, "TIMING-EVENT")
            .into_iter()
            .chain(graph.descendants(behavior, "OPERATION-INVOKED-EVENT"))
        {
            if !runnables.iter().any(|runnable| {
                runnable.event.as_deref() == Some(graph.elements[event].object.as_str())
            }) {
                return Err(fail(graph, event, "EVENT_OWNERSHIP"));
            }
        }
        let mut data_ports = Vec::new();
        let mut interfaces = BTreeMap::new();
        let mut data_accesses = Vec::new();
        let mut operations = Vec::new();
        let mut diagnostic_session_port = None;
        let mut diagnostic_mode_ports = Vec::new();
        for port in graph
            .descendants(component, "P-PORT-PROTOTYPE")
            .into_iter()
            .chain(graph.descendants(component, "R-PORT-PROTOTYPE"))
        {
            let read = graph.elements[port].tag == "R-PORT-PROTOTYPE";
            let interface = referenced(
                graph,
                port,
                if read {
                    "REQUIRED-INTERFACE-TREF"
                } else {
                    "PROVIDED-INTERFACE-TREF"
                },
                "TYPE_CONFLICT",
            )?;
            interfaces.insert(path(graph, port), path(graph, interface));
            if graph.elements[interface].tag == "SENDER-RECEIVER-INTERFACE" {
                let element = one(
                    graph,
                    port,
                    graph.descendants(interface, "VARIABLE-DATA-PROTOTYPE"),
                    "TYPE_CONFLICT",
                )?;
                let application = referenced(graph, element, "TYPE-TREF", "TYPE_MAP_MISSING")?;
                if graph.elements[application].tag != "APPLICATION-PRIMITIVE-DATA-TYPE"
                    || graph.text(application, "CATEGORY") != Some("VALUE")
                {
                    return Err(fail(graph, element, "TYPE_CONFLICT"));
                }
                let implementation = implementation(graph, port, behavior, application)?;
                unsigned_encoding(graph, implementation)?;
                let specification = one(
                    graph,
                    port,
                    graph.descendants(
                        port,
                        if read {
                            "NONQUEUED-RECEIVER-COM-SPEC"
                        } else {
                            "NONQUEUED-SENDER-COM-SPEC"
                        },
                    ),
                    "TYPE_CONFLICT",
                )?;
                let initial = one(
                    graph,
                    specification,
                    graph.descendants(specification, "VALUE"),
                    "INIT_VALUE_REQUIRED",
                )?;
                let initial_value = graph.elements[initial]
                    .text
                    .parse::<u32>()
                    .map_err(|_| fail(graph, initial, "INIT_VALUE_REQUIRED"))?;
                if graph.target(specification, "DATA-ELEMENT-REF") != Some(element)
                    || !graph.descendants(interface, "INVALID-VALUE").is_empty()
                    || !graph.descendants(application, "INVALID-VALUE").is_empty()
                    || !graph
                        .descendants(implementation, "INVALID-VALUE")
                        .is_empty()
                {
                    return Err(fail(graph, specification, "TYPE_CONFLICT"));
                }
                let accesses = graph
                    .descendants(behavior, "PORT-PROTOTYPE-REF")
                    .into_iter()
                    .filter(|reference| graph.elements[*reference].text == path(graph, port))
                    .collect::<Vec<_>>();
                if accesses.is_empty() {
                    return Err(fail(graph, port, "PORT_ACCESS_MISSING"));
                }
                let mut owners = BTreeSet::new();
                for access in accesses {
                    let owner = graph
                        .ancestor(access, "RUNNABLE-ENTITY")
                        .ok_or_else(|| fail(graph, access, "PORT_ACCESS_OWNERSHIP"))?;
                    let variable = graph
                        .ancestor(access, "VARIABLE-ACCESS")
                        .ok_or_else(|| fail(graph, access, "PORT_ACCESS_OWNERSHIP"))?;
                    if reference_descendant(graph, variable, "TARGET-DATA-PROTOTYPE-REF")?
                        != element
                        || !runnables
                            .iter()
                            .any(|runnable| runnable.path == path(graph, owner))
                        || graph
                            .ancestor(
                                access,
                                if read {
                                    "DATA-RECEIVE-POINT-BY-ARGUMENTS"
                                } else {
                                    "DATA-SEND-POINTS"
                                },
                            )
                            .is_none()
                    {
                        return Err(fail(graph, access, "PORT_ACCESS_OWNERSHIP"));
                    }
                    data_accesses.push(DataAccess {
                        path: path(graph, variable),
                        endpoint: Endpoint {
                            instance: path(graph, instance),
                            port: path(graph, port),
                            member: path(graph, element),
                        },
                        runnable: path(graph, owner),
                        read,
                    });
                    owners.insert(path(graph, owner));
                }
                // Every port access above recorded its owning runnable.
                let Some(runnable) = owners.into_iter().next() else {
                    return Err(fail(graph, port, "PORT_ACCESS_OWNERSHIP"));
                };
                data_ports.push(DataPort {
                    path: path(graph, port),
                    name: name(graph, port),
                    element: path(graph, element),
                    element_name: name(graph, element),
                    application_type: path(graph, application),
                    implementation_type: path(graph, implementation),
                    c_type: c_name(&name(graph, implementation)),
                    read,
                    api_symbol: format!(
                        "Rte_{}_{}_{}",
                        if read { "Read" } else { "Write" },
                        c_name(&name(graph, port)),
                        c_name(&name(graph, element))
                    ),
                    runnable,
                    initial_value,
                    handle_never_received: read
                        && matches!(
                            graph.text(specification, "HANDLE-NEVER-RECEIVED"),
                            Some("true" | "1")
                        ),
                    alive_timeout_ms: if read {
                        let value = graph
                            .text(specification, "ALIVE-TIMEOUT")
                            .ok_or_else(|| fail(graph, specification, "FRESHNESS_REQUIRED"))?;
                        if zero_seconds(value) {
                            Some(0)
                        } else {
                            Some(
                                milliseconds(value).ok_or_else(|| {
                                    fail(graph, specification, "FRESHNESS_REQUIRED")
                                })?,
                            )
                        }
                    } else {
                        None
                    },
                });
            } else if graph.elements[interface].tag == "CLIENT-SERVER-INTERFACE" {
                let interface_operations = graph.descendants(interface, "CLIENT-SERVER-OPERATION");
                if interface_operations.is_empty() {
                    return Err(fail(graph, interface, "SERVICE_TYPE_CONFLICT"));
                }
                let specs = graph.descendants(
                    port,
                    if read {
                        "CLIENT-COM-SPEC"
                    } else {
                        "SERVER-COM-SPEC"
                    },
                );
                let mut specified = BTreeSet::new();
                for spec in &specs {
                    let operation = graph.target(*spec, "OPERATION-REF");
                    if !operation.is_some_and(|operation| {
                        interface_operations.contains(&operation) && specified.insert(operation)
                    }) {
                        return Err(fail(graph, *spec, "SERVICE_TYPE_CONFLICT"));
                    }
                }
                for operation in interface_operations {
                    let application_error = application_error(graph, interface, operation)?;
                    if graph
                        .text(operation, "NO-RETURN-VALUE-PROVIDED")
                        .is_some_and(|value| !matches!(value, "false" | "0"))
                    {
                        return Err(fail(graph, operation, "APPLICATION_ERROR_UNSUPPORTED"));
                    }
                    let specs = graph
                        .descendants(
                            port,
                            if read {
                                "CLIENT-COM-SPEC"
                            } else {
                                "SERVER-COM-SPEC"
                            },
                        )
                        .into_iter()
                        .filter(|spec| graph.target(*spec, "OPERATION-REF") == Some(operation))
                        .collect();
                    let spec = one(graph, port, specs, "SERVICE_TYPE_CONFLICT")?;
                    if !read && graph.text(spec, "QUEUE-LENGTH") != Some("1") {
                        return Err(fail(graph, spec, "SERVICE_TYPE_CONFLICT"));
                    }
                    let mut arguments = Vec::new();
                    let mut argument_types = Vec::new();
                    for argument in graph.descendants(operation, "ARGUMENT-DATA-PROTOTYPE") {
                        if graph
                            .text(argument, "SERVER-ARGUMENT-IMPL-POLICY")
                            .is_some_and(|policy| policy != "USE-ARGUMENT-TYPE")
                        {
                            return Err(fail(graph, argument, "SERVICE_TYPE_CONFLICT"));
                        }
                        let direction = graph.text(argument, "DIRECTION").unwrap_or("");
                        if !matches!(direction, "IN" | "OUT" | "INOUT") {
                            return Err(fail(graph, argument, "ARGUMENT_DIRECTION"));
                        }
                        let (native, identity) =
                            native_type(graph, argument, behavior, &mut arrays)?;
                        arguments.push(ContractArgument {
                            name: c_name(&name(graph, argument)),
                            native_type: if !arrays.values().any(|array| array == &native)
                                && direction != "IN"
                            {
                                format!("{native} *")
                            } else {
                                native
                            },
                            direction: direction.into(),
                        });
                        argument_types.push(identity);
                    }
                    let mut callers = Vec::new();
                    let mut call_points = Vec::new();
                    let (runnable, event) = if read {
                        let calls = graph
                            .descendants(behavior, "SYNCHRONOUS-SERVER-CALL-POINT")
                            .into_iter()
                            .filter(|call| {
                                graph.descendants(*call, "CONTEXT-R-PORT-REF").iter().any(
                                    |reference| {
                                        graph.elements[*reference].text == path(graph, port)
                                    },
                                ) && graph
                                    .descendants(*call, "TARGET-REQUIRED-OPERATION-REF")
                                    .iter()
                                    .any(|reference| {
                                        graph.elements[*reference].text == path(graph, operation)
                                    })
                            })
                            .collect::<Vec<_>>();
                        if calls.is_empty() {
                            return Err(fail(graph, port, "SERVICE_CLIENT_MISSING"));
                        }
                        for call in &calls {
                            let owner = graph
                                .ancestor(*call, "RUNNABLE-ENTITY")
                                .ok_or_else(|| fail(graph, *call, "SERVICE_CLIENT_MISSING"))?;
                            if !graph.within(owner, behavior)
                                || (graph.elements[component].tag
                                    == "APPLICATION-SW-COMPONENT-TYPE"
                                    && !runnables
                                        .iter()
                                        .any(|runnable| runnable.path == path(graph, owner)))
                            {
                                return Err(fail(graph, *call, "CALL_POINT_OWNERSHIP"));
                            }
                            callers.push(path(graph, owner));
                            call_points.push(path(graph, *call));
                        }
                        (callers[0].clone(), None)
                    } else {
                        let events = graph
                            .descendants(behavior, "OPERATION-INVOKED-EVENT")
                            .into_iter()
                            .filter(|event| {
                                graph.descendants(*event, "CONTEXT-P-PORT-REF").iter().any(
                                    |reference| {
                                        graph.elements[*reference].text == path(graph, port)
                                    },
                                ) && graph
                                    .descendants(*event, "TARGET-PROVIDED-OPERATION-REF")
                                    .iter()
                                    .any(|reference| {
                                        graph.elements[*reference].text == path(graph, operation)
                                    })
                            })
                            .collect();
                        let event = one(graph, port, events, "SERVICE_CLIENT_MISSING")?;
                        let runnable =
                            referenced(graph, event, "START-ON-EVENT-REF", "EVENT_OWNERSHIP")?;
                        (path(graph, runnable), Some(path(graph, event)))
                    };
                    let api = format!(
                        "Rte_Call_{}_{}",
                        c_name(&name(graph, port)),
                        c_name(&name(graph, operation))
                    );
                    operations.push(Operation {
                        port: path(graph, port),
                        interface: path(graph, interface),
                        operation: path(graph, operation),
                        application_error,
                        read,
                        api_symbol: api.clone(),
                        implementation_symbol: format!(
                            "Rte_Call_{}_{}",
                            c_name(&name(graph, component)),
                            api.trim_start_matches("Rte_Call_")
                        ),
                        arguments,
                        argument_types,
                        runnable,
                        callers,
                        call_points,
                        event,
                    });
                }
            } else if graph.elements[interface].tag == "MODE-SWITCH-INTERFACE" {
                // Only the fixed Dcm BSW service publisher is in scope; this
                // does not enable mode communication for application SWCs.
                if read
                    || graph.elements[component].tag != "SERVICE-SW-COMPONENT-TYPE"
                    || diagnostic_mode_ports
                        .iter()
                        .any(|mode: &DiagnosticSessionPort| {
                            mode.interface == path(graph, interface)
                        })
                    || graph.text(interface, "IS-SERVICE") != Some("true")
                {
                    return Err(fail(graph, port, "TYPE_CONFLICT"));
                }
                let prototype = one(
                    graph,
                    interface,
                    graph.descendants(interface, "MODE-GROUP"),
                    "TYPE_CONFLICT",
                )?;
                let group = referenced(graph, prototype, "TYPE-TREF", "TYPE_CONFLICT")?;
                let dcm = graph
                    .objects
                    .get("/AUTOSAR_Dcm/Dcm")
                    .copied()
                    .ok_or_else(|| fail(graph, port, "TYPE_CONFLICT"))?;
                let definition = dcm_modes
                    .iter()
                    .find(|definition| {
                        path(graph, group) == format!("/AUTOSAR_Dcm/{}", definition.group)
                    })
                    .ok_or_else(|| fail(graph, port, "TYPE_CONFLICT"))?;
                let provided = one(
                    graph,
                    dcm,
                    graph
                        .descendants(dcm, "MODE-DECLARATION-GROUP-PROTOTYPE")
                        .into_iter()
                        .filter(|index| name(graph, *index) == definition.group)
                        .collect(),
                    "TYPE_CONFLICT",
                )?;
                let spec = one(
                    graph,
                    port,
                    graph.descendants(port, "MODE-SWITCH-SENDER-COM-SPEC"),
                    "TYPE_CONFLICT",
                )?;
                if name(graph, provided) != definition.group
                    || graph.target(provided, "TYPE-TREF") != Some(group)
                    || graph.target(spec, "MODE-GROUP-REF") != Some(prototype)
                    || graph.text(spec, "ENHANCED-MODE-API") != Some("false")
                    || path(graph, group) != format!("/AUTOSAR_Dcm/{}", definition.group)
                {
                    return Err(fail(graph, port, "TYPE_CONFLICT"));
                }
                let modes = graph.descendants(group, "MODE-DECLARATION");
                if modes.len() != definition.modes.len()
                    || !definition.modes.iter().enumerate().all(|(value, mode)| {
                        modes.iter().any(|index| {
                            name(graph, *index) == *mode
                                && graph.text(*index, "VALUE") == Some(value.to_string().as_str())
                        })
                    })
                    || graph.target(group, "INITIAL-MODE-REF").is_none_or(|index| {
                        !modes.contains(&index) || name(graph, index) != definition.modes[0]
                    })
                    || name(graph, port) != definition.port
                    || name(graph, interface) != format!("Dcm_{}", definition.port)
                    || name(graph, prototype) != definition.prototype
                    || graph.text(group, "CATEGORY") != Some("EXPLICIT_ORDER")
                    || graph.text(group, "ON-TRANSITION-VALUE") != Some("255")
                {
                    return Err(fail(graph, port, "TYPE_CONFLICT"));
                }
                let maps: Vec<_> = graph
                    .descendants(behavior, "DATA-TYPE-MAPPING-REF")
                    .into_iter()
                    .filter_map(|reference| {
                        graph.objects.get(&graph.elements[reference].text).copied()
                    })
                    .flat_map(|mapping| graph.descendants(mapping, "MODE-REQUEST-TYPE-MAP"))
                    .filter(|mapping| graph.target(*mapping, "MODE-GROUP-REF") == Some(group))
                    .collect();
                let mapping = one(graph, prototype, maps, "TYPE_CONFLICT")?;
                let mode_type = referenced(
                    graph,
                    mapping,
                    "IMPLEMENTATION-DATA-TYPE-REF",
                    "TYPE_CONFLICT",
                )?;
                let base_ref = one(
                    graph,
                    mode_type,
                    graph.descendants(mode_type, "BASE-TYPE-REF"),
                    "TYPE_CONFLICT",
                )?;
                let base = graph
                    .objects
                    .get(&graph.elements[base_ref].text)
                    .copied()
                    .ok_or_else(|| fail(graph, base_ref, "TYPE_CONFLICT"))?;
                if name(graph, mode_type) != format!("Rte_ModeType_{}", definition.group)
                    || graph.text(mode_type, "CATEGORY") != Some("VALUE")
                    || graph.text(base, "BASE-TYPE-SIZE") != Some("8")
                    || graph.text(base, "BASE-TYPE-ENCODING") != Some("NONE")
                    || graph.text(base, "NATIVE-DECLARATION") != Some("uint8")
                {
                    return Err(fail(graph, mode_type, "TYPE_CONFLICT"));
                }
                let mode = DiagnosticSessionPort {
                    port: path(graph, port),
                    interface: path(graph, interface),
                    prototype: path(graph, prototype),
                    mode_group: path(graph, group),
                    mode_type: path(graph, mode_type),
                };
                if definition.group == "DcmDiagnosticSessionControl" {
                    diagnostic_session_port = Some(mode.clone());
                }
                diagnostic_mode_ports.push(mode);
            } else {
                return Err(fail(graph, port, "TYPE_CONFLICT"));
            }
        }
        for event in graph.descendants(behavior, "OPERATION-INVOKED-EVENT") {
            if operations
                .iter()
                .filter(|operation| {
                    !operation.read
                        && operation.event.as_deref() == Some(graph.elements[event].object.as_str())
                })
                .count()
                != 1
            {
                return Err(fail(graph, event, "EVENT_OWNERSHIP"));
            }
        }
        for access in graph.descendants(behavior, "VARIABLE-ACCESS") {
            if data_accesses
                .iter()
                .filter(|checked| checked.path == path(graph, access))
                .count()
                != 1
            {
                return Err(fail(graph, access, "PORT_ACCESS_OWNERSHIP"));
            }
        }
        for runnable in runnables.iter().filter(|runnable| runnable.event.is_none()) {
            if operations
                .iter()
                .filter(|operation| operation.read && operation.callers.contains(&runnable.path))
                .count()
                != 1
            {
                return Err(fail(
                    graph,
                    graph.index(&runnable.path)?,
                    "SERVICE_CLIENT_MISSING",
                ));
            }
        }
        for call in graph.descendants(behavior, "SYNCHRONOUS-SERVER-CALL-POINT") {
            if !operations
                .iter()
                .any(|operation| operation.call_points.contains(&path(graph, call)))
            {
                return Err(fail(graph, call, "CALL_POINT_OWNERSHIP"));
            }
        }
        components.push(Component {
            component: path(graph, component),
            instance: path(graph, instance),
            ecu: path(graph, ecu),
            ecu_mapping: path(graph, mapping),
            rte_instance: path(graph, rte_instance),
            behavior: path(graph, behavior),
            header: format!("include/Rte_{}.h", c_name(&name(graph, component))),
            interfaces,
            diagnostic_session_port,
            diagnostic_mode_ports,
            data_ports,
            data_accesses,
            operations,
            runnables,
        });
    }
    for rte_instance in graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| super::schedule::definition_is(graph, *index, "RteSwComponentInstance"))
    {
        if !components
            .iter()
            .any(|component| component.rte_instance == path(graph, rte_instance))
        {
            return Err(fail(graph, rte_instance, "INSTANCE_MAPPING"));
        }
    }
    for mapping in graph.descendants(system, "SWC-TO-ECU-MAPPING") {
        if graph.target(mapping, "ECU-INSTANCE-REF") != Some(ecu) {
            return Err(fail(graph, mapping, "ECU_MAPPING"));
        }
        let members = graph.descendants(mapping, "COMPONENT-IREF");
        if members.is_empty() {
            return Err(fail(graph, mapping, "ECU_MAPPING"));
        }
        let mut mapped_instances = BTreeSet::new();
        for iref in members {
            if graph.target(iref, "CONTEXT-COMPOSITION-REF") != Some(root)
                || !graph
                    .target(iref, "TARGET-COMPONENT-REF")
                    .is_some_and(|instance| {
                        mapped_instances.insert(instance)
                            && components
                                .iter()
                                .any(|component| component.instance == path(graph, instance))
                    })
            {
                return Err(fail(graph, iref, "ECU_MAPPING"));
            }
        }
    }
    let mut connections = Vec::new();
    let mut receivers = BTreeSet::new();
    for connector in graph.descendants(composition, "ASSEMBLY-SW-CONNECTOR") {
        let provider = one(
            graph,
            connector,
            graph.children(connector, "PROVIDER-IREF"),
            "CONNECTOR_OWNERSHIP",
        )?;
        let requester = one(
            graph,
            connector,
            graph.children(connector, "REQUESTER-IREF"),
            "CONNECTOR_OWNERSHIP",
        )?;
        let p_instance = referenced(
            graph,
            provider,
            "CONTEXT-COMPONENT-REF",
            "CONNECTOR_OWNERSHIP",
        )?;
        let r_instance = referenced(
            graph,
            requester,
            "CONTEXT-COMPONENT-REF",
            "CONNECTOR_OWNERSHIP",
        )?;
        let p_port = referenced(graph, provider, "TARGET-P-PORT-REF", "CONNECTOR_DIRECTION")?;
        let r_port = referenced(graph, requester, "TARGET-R-PORT-REF", "CONNECTOR_DIRECTION")?;
        let p_component = components
            .iter()
            .find(|component| component.instance == path(graph, p_instance))
            .ok_or_else(|| fail(graph, provider, "CONNECTOR_OWNERSHIP"))?;
        let r_component = components
            .iter()
            .find(|component| component.instance == path(graph, r_instance))
            .ok_or_else(|| fail(graph, requester, "CONNECTOR_OWNERSHIP"))?;
        if !graph.within(p_port, graph.index(&p_component.component)?)
            || !graph.within(r_port, graph.index(&r_component.component)?)
            || graph.elements[p_port].tag != "P-PORT-PROTOTYPE"
            || graph.elements[r_port].tag != "R-PORT-PROTOTYPE"
            || graph.target(p_port, "PROVIDED-INTERFACE-TREF")
                != graph.target(r_port, "REQUIRED-INTERFACE-TREF")
        {
            return Err(fail(graph, connector, "TYPE_CONFLICT"));
        }
        if let Some(p) = p_component
            .data_ports
            .iter()
            .find(|port| port.path == path(graph, p_port))
        {
            let r = r_component
                .data_ports
                .iter()
                .find(|port| port.path == path(graph, r_port))
                .ok_or_else(|| fail(graph, connector, "TYPE_CONFLICT"))?;
            if p.element != r.element
                || p.application_type != r.application_type
                || p.implementation_type != r.implementation_type
            {
                return Err(fail(graph, connector, "TYPE_CONFLICT"));
            }
            let endpoint = Endpoint {
                instance: r_component.instance.clone(),
                port: r.path.clone(),
                member: r.element.clone(),
            };
            if !receivers.insert(endpoint.clone()) {
                return Err(fail(graph, connector, "MULTIPLE_PRODUCERS"));
            }
            connections.push(Connection {
                path: path(graph, connector),
                provider: Endpoint {
                    instance: p_component.instance.clone(),
                    port: p.path.clone(),
                    member: p.element.clone(),
                },
                requester: endpoint,
                service: false,
            });
        } else {
            for r in r_component
                .operations
                .iter()
                .filter(|operation| operation.port == path(graph, r_port))
            {
                let p = p_component
                    .operations
                    .iter()
                    .find(|operation| {
                        operation.port == path(graph, p_port) && operation.operation == r.operation
                    })
                    .ok_or_else(|| fail(graph, connector, "SERVICE_TYPE_CONFLICT"))?;
                if p.argument_types != r.argument_types
                    || p.arguments
                        .iter()
                        .map(|argument| (&argument.native_type, &argument.direction))
                        .collect::<Vec<_>>()
                        != r.arguments
                            .iter()
                            .map(|argument| (&argument.native_type, &argument.direction))
                            .collect::<Vec<_>>()
                {
                    return Err(fail(graph, connector, "SERVICE_TYPE_CONFLICT"));
                }
                let endpoint = Endpoint {
                    instance: r_component.instance.clone(),
                    port: r.port.clone(),
                    member: r.operation.clone(),
                };
                if !receivers.insert(endpoint.clone()) {
                    return Err(fail(graph, connector, "MULTIPLE_PRODUCERS"));
                }
                connections.push(Connection {
                    path: path(graph, connector),
                    provider: Endpoint {
                        instance: p_component.instance.clone(),
                        port: p.port.clone(),
                        member: p.operation.clone(),
                    },
                    requester: endpoint,
                    service: true,
                });
            }
        }
    }
    let mut network_endpoints = Vec::new();
    for component in &components {
        let mut network_ports = Vec::new();
        for port in &component.data_ports {
            let endpoint = Endpoint {
                instance: component.instance.clone(),
                port: port.path.clone(),
                member: port.element.clone(),
            };
            let mappings = graph
                .descendants(system, "SENDER-RECEIVER-TO-SIGNAL-MAPPING")
                .into_iter()
                .filter(|mapping| {
                    graph
                        .descendants(*mapping, "CONTEXT-COMPONENT-REF")
                        .iter()
                        .any(|reference| graph.elements[*reference].text == component.instance)
                        && graph
                            .descendants(*mapping, "CONTEXT-PORT-REF")
                            .iter()
                            .any(|reference| graph.elements[*reference].text == port.path)
                })
                .collect::<Vec<_>>();
            let local = connections.iter().any(|connection| {
                connection.provider == endpoint || connection.requester == endpoint
            });
            if mappings.len() > 1 || (!mappings.is_empty() && local) {
                return Err(fail(graph, graph.index(&port.path)?, "ENDPOINT_BINDING"));
            }
            if mappings.is_empty()
                && !local
                && port.read
                && generation.strict_unconnected_r_port_check
                && !matches!(
                    graph.text(graph.index(&port.path)?, "MAY-BE-UNCONNECTED"),
                    Some("true" | "1")
                )
            {
                return Err(fail(graph, graph.index(&port.path)?, "UNCONNECTED_R_PORT"));
            }
            if mappings.is_empty() {
                let context = graph.index(&port.path)?;
                if port.read {
                    let spec = one(
                        graph,
                        context,
                        graph.descendants(context, "NONQUEUED-RECEIVER-COM-SPEC"),
                        "TYPE_CONFLICT",
                    )?;
                    if local
                        && (!graph.text(spec, "ALIVE-TIMEOUT").is_some_and(zero_seconds)
                            || graph.text(spec, "HANDLE-TIMEOUT-TYPE") != Some("NONE"))
                    {
                        return Err(fail(graph, spec, "LOCAL_FRESHNESS_UNSUPPORTED"));
                    }
                }
            } else {
                if port.read {
                    let context = graph.index(&port.path)?;
                    let spec = one(
                        graph,
                        context,
                        graph.descendants(context, "NONQUEUED-RECEIVER-COM-SPEC"),
                        "TYPE_CONFLICT",
                    )?;
                    if graph.text(spec, "HANDLE-TIMEOUT-TYPE") != Some("NONE") {
                        return Err(fail(graph, spec, "NETWORK_FRESHNESS_UNSUPPORTED"));
                    }
                }
                network_ports.push(port.clone());
            }
        }
        // Live-source ownership needs application identities, not target transport eligibility.
        if include_transport {
            let signals = super::signals::inspect_ports(
                graph,
                &network_ports,
                &component.instance,
                common_period.ok_or_else(|| fail(graph, composition, "SCHEDULE_NOT_UNIQUE"))?,
            )?;
            for transport in signals {
                let channel = graph
                    .ancestor(graph.index(&transport.trigger)?, "CAN-PHYSICAL-CHANNEL")
                    .ok_or_else(|| fail(graph, composition, "SIGNAL_MAPPING"))?;
                let network = graph
                    .ancestor(channel, "CAN-CLUSTER")
                    .ok_or_else(|| fail(graph, channel, "SIGNAL_MAPPING"))?;
                let port = component
                    .data_ports
                    .iter()
                    .find(|port| port.path == transport.port)
                    .ok_or_else(|| PlanDiagnostic::plan_premise(&transport.port))?;
                network_endpoints.push(NetworkEndpoint {
                    endpoint: Endpoint {
                        instance: component.instance.clone(),
                        port: port.path.clone(),
                        member: port.element.clone(),
                    },
                    network: path(graph, network),
                    channel: path(graph, channel),
                    transport,
                });
            }
        }
        for operation in component
            .operations
            .iter()
            .filter(|operation| operation.read)
        {
            if generation.strict_unconnected_r_port_check
                && !connections.iter().any(|connection| {
                    connection.requester
                        == (Endpoint {
                            instance: component.instance.clone(),
                            port: operation.port.clone(),
                            member: operation.operation.clone(),
                        })
                })
                && !matches!(
                    graph.text(graph.index(&operation.port)?, "MAY-BE-UNCONNECTED"),
                    Some("true" | "1")
                )
            {
                return Err(fail(
                    graph,
                    graph.index(&operation.port)?,
                    "UNCONNECTED_R_PORT",
                ));
            }
        }
    }
    for mapping in graph.descendants(system, "SENDER-RECEIVER-TO-SIGNAL-MAPPING") {
        let iref = one(
            graph,
            mapping,
            graph.children(mapping, "DATA-ELEMENT-IREF"),
            "SIGNAL_MAPPING",
        )?;
        let instance = reference_descendant(graph, iref, "CONTEXT-COMPONENT-REF")?;
        let port = referenced(graph, iref, "CONTEXT-PORT-REF", "SIGNAL_MAPPING")?;
        let member = referenced(graph, iref, "TARGET-DATA-PROTOTYPE-REF", "SIGNAL_MAPPING")?;
        if graph.target(iref, "CONTEXT-COMPOSITION-REF") != Some(root)
            || if include_transport {
                !network_endpoints.iter().any(|checked| {
                    checked.endpoint
                        == (Endpoint {
                            instance: path(graph, instance),
                            port: path(graph, port),
                            member: path(graph, member),
                        })
                })
            } else {
                !components.iter().any(|component| {
                    component.instance == path(graph, instance)
                        && component.data_ports.iter().any(|data| {
                            data.path == path(graph, port) && data.element == path(graph, member)
                        })
                })
            }
        {
            return Err(fail(graph, mapping, "SIGNAL_MAPPING"));
        }
    }
    if !network_endpoints.is_empty() {
        let channel = super::component::selected_channel(graph, ecu)?;
        if network_endpoints
            .iter()
            .any(|endpoint| endpoint.channel != path(graph, channel))
        {
            return Err(fail(graph, channel, "SIGNAL_MAPPING"));
        }
    }
    for component in graph.of_kind("APPLICATION-SW-COMPONENT-TYPE") {
        if !types.contains(&component) {
            return Err(fail(graph, component, "UNINSTANTIATED_COMPONENT"));
        }
    }
    // Traverse actual runnable call edges, including every synchronous call point.
    let mut calls: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for connection in connections.iter().filter(|connection| connection.service) {
        let provider = components
            .iter()
            .find(|component| component.instance == connection.provider.instance)
            .ok_or_else(|| PlanDiagnostic::plan_premise(&connection.provider.instance))?;
        let requester = components
            .iter()
            .find(|component| component.instance == connection.requester.instance)
            .ok_or_else(|| PlanDiagnostic::plan_premise(&connection.requester.instance))?;
        let server = provider
            .operations
            .iter()
            .find(|operation| {
                operation.port == connection.provider.port
                    && operation.operation == connection.provider.member
            })
            .ok_or_else(|| PlanDiagnostic::plan_premise(&connection.provider.member))?;
        let client = requester
            .operations
            .iter()
            .find(|operation| {
                operation.port == connection.requester.port
                    && operation.operation == connection.requester.member
            })
            .ok_or_else(|| PlanDiagnostic::plan_premise(&connection.requester.member))?;
        for caller in &client.callers {
            calls
                .entry(caller.clone())
                .or_default()
                .push(server.runnable.clone());
        }
    }
    for origin in calls.keys() {
        let mut pending = vec![origin.as_str()];
        let mut visited = BTreeSet::new();
        while let Some(runnable) = pending.pop() {
            if !visited.insert(runnable) {
                continue;
            }
            for target in calls.get(runnable).into_iter().flatten() {
                if target == origin {
                    return Err(fail(graph, graph.index(origin)?, "SERVICE_RECURSION"));
                }
                pending.push(target);
            }
        }
    }
    let plan = CompositionContract {
        system: path(graph, system),
        composition: path(graph, composition),
        ecu: path(graph, ecu),
        generation,
        components,
        connections,
        network_endpoints,
        array_types: arrays,
        types: super::rte_types::inspect(graph)?,
        dcm_modes,
    };
    check_names(graph, &plan)?;
    Ok(plan)
}

pub(super) fn data_symbol(component: &Component, port: &DataPort) -> String {
    format!(
        "Rte_{}_{}_{}_{}",
        if port.read { "Read" } else { "Write" },
        c_name(super::short_name(&component.component)),
        c_name(&port.name),
        c_name(&port.element_name),
    )
}

// These are the actual delivered OS/target and locked FreeRTOS symbol families.
// Source component names cannot claim a producer in those existing namespaces.
fn runtime_namespace(name: &str) -> bool {
    [
        // Generated OS counter service producers (os_service.rs):
        // OsService_<Counter>_<Operation> and its Rte_Call_OsService_ port API.
        "OsService_",
        "Rte_Call_OsService_",
        "Os_",
        "OS_",
        "Ecu_",
        "Arti_",
        "ARTI_",
        "E_OS_",
        "OSMEMORY_",
        "OSServiceId_",
        "OSError_",
        "eTask",
        "pcQueue",
        "pcTask",
        "pcTimer",
        "pvPort",
        "pvTask",
        "pvTimer",
        "pxPort",
        "ucQueue",
        "ucStreamBuffer",
        "ulPort",
        "ulTask",
        "uxEventGroup",
        "uxList",
        "uxQueue",
        "uxStreamBuffer",
        "uxTask",
        "uxTimer",
        "vEventGroup",
        "vList",
        "vMessageBuffer",
        "vPort",
        "vQueue",
        "vStreamBuffer",
        "vTask",
        "vTimer",
        "xEventGroup",
        "xMessageBuffer",
        "xPort",
        "xQueue",
        "xStreamBuffer",
        "xTask",
        "xTimer",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
}

// These actual generated runtime scopes have fixed MemMap producer ownership.
// Component and partition scopes must not merge with their sections or prefixes.
pub(super) fn reserved_memory_scope(name: &str) -> bool {
    matches!(
        name,
        "Rte"
            | "Os"
            | "Det"
            | "Ecu"
            | "Can"
            | "CanIf"
            | "CanTp"
            | "Com"
            | "Dcm"
            | "PduR"
            | "LSduR"
            | "ComM"
            | "BswM"
            | "Ecu_HostBusSM"
    )
}

fn check_names(graph: &Graph, plan: &CompositionContract) -> Result<(), Vec<PlanDiagnostic>> {
    let mut filenames = BTreeSet::from([
        "INCLUDE/RTE.H".to_owned(),
        "INCLUDE/RTE_TYPE.H".to_owned(),
        "INCLUDE/RTE_DATAHANDLETYPE.H".to_owned(),
        "INCLUDE/RTE_CFG.H".to_owned(),
        "INCLUDE/RTE_COM.H".to_owned(),
        "INCLUDE/RTE_MAIN.H".to_owned(),
        "INCLUDE/RTE_MEMMAP.H".to_owned(),
        "INCLUDE/RTE_DCM_TYPE.H".to_owned(),
        "INCLUDE/RTE_COMM_TYPE.H".to_owned(),
        "INCLUDE/STD_TYPES.H".to_owned(),
    ]);
    let mut names: BTreeSet<String> = "RTE_H RTE_TYPE_H RTE_COM_H Rte_COMCbk Rte_COMCbkRxTOut CbkHandleIdType STD_TYPES_H NULL_PTR uint8 uint16 uint32 uint64 sint8 sint16 sint32 sint64 EcuStatus boolean Std_ReturnType Std_VersionInfoType TRUE FALSE E_OK E_NOT_OK RTE_E_COM_STOPPED RTE_E_NEVER_RECEIVED RTE_E_MAX_AGE_EXCEEDED intptr_t uintptr_t intmax_t uintmax_t INTMAX_MIN INTMAX_MAX UINTMAX_MAX INTPTR_MIN INTPTR_MAX UINTPTR_MAX PTRDIFF_MIN PTRDIFF_MAX SIZE_MAX SIG_ATOMIC_MIN SIG_ATOMIC_MAX WCHAR_MIN WCHAR_MAX WINT_MIN WINT_MAX INTMAX_C UINTMAX_C".split_whitespace().map(str::to_owned).collect();
    names.extend("float32 float64 VoidPtr ConstVoidPtr sint8_least sint16_least sint32_least uint8_least uint16_least uint32_least PLATFORM_TYPES_H CPU_TYPE_8 CPU_TYPE_16 CPU_TYPE_32 CPU_TYPE_64 CPU_TYPE MSB_FIRST LSB_FIRST CPU_BIT_ORDER HIGH_BYTE_FIRST LOW_BYTE_FIRST CPU_BYTE_ORDER PLATFORM_AR_RELEASE_MAJOR_VERSION PLATFORM_AR_RELEASE_MINOR_VERSION PLATFORM_AR_RELEASE_REVISION_VERSION STD_TYPES_AR_RELEASE_MAJOR_VERSION STD_TYPES_AR_RELEASE_MINOR_VERSION STD_TYPES_AR_RELEASE_REVISION_VERSION COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION COMSTACKTYPE_AR_RELEASE_MINOR_VERSION COMSTACKTYPE_AR_RELEASE_REVISION_VERSION".split_whitespace().map(str::to_owned));
    names.extend("DCM_INITIAL DCM_PENDING DCM_CANCEL DCM_FORCE_RCRRP_OK DCM_RES_POS_OK DCM_RES_POS_NOT_OK DCM_RES_NEG_OK DCM_RES_NEG_NOT_OK DCM_OBD_ON_CAN DCM_OBD_ON_FLEXRAY DCM_OBD_ON_IP DCM_UDS_ON_CAN DCM_UDS_ON_FLEXRAY DCM_UDS_ON_IP DCM_ROE_ON_CAN DCM_ROE_ON_FLEXRAY DCM_ROE_ON_IP DCM_PERIODICTRANS_ON_CAN DCM_PERIODICTRANS_ON_FLEXRAY DCM_PERIODICTRANS_ON_IP DCM_NO_ACTIVE_PROTOCOL DCM_UDS_ON_LIN DCM_SUPPLIER_1 DCM_SUPPLIER_2 DCM_SUPPLIER_3 DCM_SUPPLIER_4 DCM_SUPPLIER_5 DCM_SUPPLIER_6 DCM_SUPPLIER_7 DCM_SUPPLIER_8 DCM_SUPPLIER_9 DCM_SUPPLIER_10 DCM_SUPPLIER_11 DCM_SUPPLIER_12 DCM_SUPPLIER_13 DCM_SUPPLIER_14 DCM_SUPPLIER_15 DCM_POS_RESP DCM_E_GENERALREJECT DCM_E_SERVICENOTSUPPORTED DCM_E_SUBFUNCTIONNOTSUPPORTED DCM_E_INCORRECTMESSAGELENGTHORINVALIDFORMAT DCM_E_RESPONSETOOLONG DCM_E_BUSYREPEATREQUEST DCM_E_CONDITIONSNOTCORRECT DCM_E_REQUESTSEQUENCEERROR DCM_E_NORESPONSEFROMSUBNETCOMPONENT DCM_E_FAILUREPREVENTSEXECUTIONOFREQUESTEDACTION DCM_E_REQUESTOUTOFRANGE DCM_E_SECURITYACCESSDENIED DCM_E_INVALIDKEY DCM_E_EXCEEDNUMBEROFATTEMPTS DCM_E_REQUIREDTIMEDELAYNOTEXPIRED DCM_E_UPLOADDOWNLOADNOTACCEPTED DCM_E_TRANSFERDATASUSPENDED DCM_E_GENERALPROGRAMMINGFAILURE DCM_E_WRONGBLOCKSEQUENCECOUNTER DCM_E_SUBFUNCTIONNOTSUPPORTEDINACTIVESESSION DCM_E_SERVICENOTSUPPORTEDINACTIVESESSION DCM_E_RPMTOOHIGH DCM_E_RPMTOOLOW DCM_E_ENGINEISRUNNING DCM_E_ENGINEISNOTRUNNING DCM_E_ENGINERUNTIMETOOLOW DCM_E_TEMPERATURETOOHIGH DCM_E_TEMPERATURETOOLOW DCM_E_VEHICLESPEEDTOOHIGH DCM_E_VEHICLESPEEDTOOLOW DCM_E_THROTTLE_PEDALTOOHIGH DCM_E_THROTTLE_PEDALTOOLOW DCM_E_TRANSMISSIONRANGENOTINNEUTRAL DCM_E_TRANSMISSIONRANGENOTINGEAR DCM_E_BRAKESWITCH_NOTCLOSED DCM_E_SHIFTERLEVERNOTINPARK DCM_E_TORQUECONVERTERCLUTCHLOCKED DCM_E_VOLTAGETOOHIGH DCM_E_VOLTAGETOOLOW DCM_E_VMSCNC_0 DCM_E_VMSCNC_1 DCM_E_VMSCNC_2 DCM_E_VMSCNC_3 DCM_E_VMSCNC_4 DCM_E_VMSCNC_5 DCM_E_VMSCNC_6 DCM_E_VMSCNC_7 DCM_E_VMSCNC_8 DCM_E_VMSCNC_9 DCM_E_VMSCNC_A DCM_E_VMSCNC_B DCM_E_VMSCNC_C DCM_E_VMSCNC_D DCM_E_VMSCNC_E DCM_SEC_LEV_LOCKED DCM_DEFAULT_SESSION DCM_PROGRAMMING_SESSION DCM_EXTENDED_DIAGNOSTIC_SESSION DCM_SAFETY_SYSTEM_DIAGNOSTIC_SESSION".split_whitespace().map(str::to_owned));
    names.extend("Dcm_OpStatusType Dcm_ConfirmationStatusType Dcm_ProtocolType Dcm_NegativeResponseCodeType Dcm_ConfigType Dcm_MsgItemType Dcm_MsgType Dcm_MsgLenType Dcm_MsgAddInfoType Dcm_IdContextType Dcm_MsgContextType Dcm_ReadDidType Ecu_DcmAuthenticationModeType".split_whitespace().map(str::to_owned));
    names.extend("RTE_CORE RTE_MAIN_H RTE_CFG_H RTE_E_OK RTE_E_LIMIT Rte_Start Rte_Stop SchM_ConfigType SchM_Init SchM_Start SchM_StartTiming SchM_Deinit RTE_CODE RTE_VAR_CLEARED RTE_VAR_INIT RTE_CONST RTE_E_UNCONNECTED rte_started rte_allocated SchM_Switch_Dcm_DcmDiagnosticSessionControl SchM_Mode_Dcm_DcmDiagnosticSessionControl Dcm_SecLevelType Dcm_SesCtrlType Rte_ModeType_DcmDiagnosticSessionControl Dcm_GetSecurityLevel Dcm_GetSesCtrlType Dcm_ResetToDefaultSession Can_HostTransmitToken Can_HostCompleteTransmit CanIf_GetControllerRxErrorCounter CanIf_GetControllerTxErrorCounter CanTp_CancelReceive CanTp_CancelTransmit Com_TriggerIPDUSend ComM_GetInhibitionStatus ComM_SetECUGroupClassification ComM_GetCurrentPNCComMode ComM_EcuM_WakeUpIndication".split_whitespace().map(str::to_owned));
    names.extend("COM_H COMSTACK_TYPES_H COMSTACK_CFG_H ECU_TARGET_H ECU_HOST_BATCH_H ECU_STATUS_H AUTOSAR_OS_TARGET_H AUTOSAR_OS_H AUTOSAR_OS_TIME_H STD_HIGH STD_LOW STD_ACTIVE STD_IDLE STD_ON STD_OFF COM_SERVICE_NOT_AVAILABLE COM_BUSY COM_UNINIT COM_INIT ECU_TARGET_UNPREPARED ECU_TARGET_INITIALIZING ECU_TARGET_READY ECU_TARGET_FAILED ECU_TARGET_CLOSED ECU_TARGET_OUTPUT_CAPACITY ECU_BATCH_CAPACITY ECU_BATCH_MAX_SPAN ECU_BATCH_LINE_CAPACITY ECU_BATCH_WATCHDOG_MS ECU_BATCH_BEGIN ECU_BATCH_RX ECU_BATCH_COMMIT RTE_MEMMAP_ACTIVE Ecu_TargetIsOwner Ecu_TargetCheckLifecycleContext Com_SendSignal Com_ReceiveSignal PduIdType PduLengthType PduInfoType BufReq_ReturnType TpDataStateType RetryInfoType NetworkHandleType Com_StatusType Com_SignalIdType Com_IpduGroupIdType Com_PduConfigType Com_NotificationType Com_TransmitType Com_ConfigType OS_MAX_TASKS OS_MAX_PRIORITY OS_MAX_ACTIVATIONS OS_BASIC_TASK OS_EXTENDED_TASK OS_MAX_RESOURCES OS_MAX_INTERNAL_RESOURCES OS_MAX_INTERRUPTS OS_MAX_ISR_PRIORITY OS_SCHEDULE_FULL OS_SCHEDULE_NON".split_whitespace().map(str::to_owned));
    for index in 0..plan
        .components
        .iter()
        .map(|component| component.data_ports.iter().filter(|port| port.read).count())
        .sum::<usize>()
    {
        for suffix in ["", "_received", "_expired"] {
            names.insert(format!("rte_value_{index}{suffix}"));
        }
    }
    names.extend("StatusType TaskType TaskRefType TaskStateType TaskStateRefType AppModeType OSDEFAULTAPPMODE DONOTCARE TotalNumberOfCores ResourceType EventMaskType EventMaskRefType CounterType AlarmType ApplicationType INVALID_OSAPPLICATION ApplicationStateType ApplicationStateRefType APPLICATION_ACCESSIBLE APPLICATION_TERMINATED TrustedFunctionIndexType TrustedFunctionParameterRefType AccessType OSMEMORY_IS_READABLE OSMEMORY_IS_WRITEABLE OSMEMORY_IS_EXECUTABLE OSMEMORY_IS_STACKSPACE ObjectAccessType ACCESS NO_ACCESS ObjectTypeType OBJECT_TASK OBJECT_ISR OBJECT_ALARM OBJECT_RESOURCE OBJECT_COUNTER OBJECT_SCHEDULETABLE MemoryStartAddressType MemorySizeType ISRType INVALID_ISR INVALID_TASK ProtectionReturnType PRO_IGNORE PRO_TERMINATETASKISR PRO_TERMINATEAPPL PRO_SHUTDOWN PRO_PREVENT_ARRIVAL_RATE RestartType PhysicalTimeType CoreIdType SpinlockIdType INVALID_SPINLOCK TryToGetSpinlockType TRYTOGETSPINLOCK_NOSUCCESS TRYTOGETSPINLOCK_SUCCESS IdleModeType IDLE_NO_HALT AreaIdType OSServiceIdType ScheduleTableType ScheduleTableStatusType ScheduleTableStatusRefType SCHEDULETABLE_STOPPED SCHEDULETABLE_NEXT SCHEDULETABLE_WAITING SCHEDULETABLE_RUNNING SCHEDULETABLE_RUNNING_AND_SYNCHRONOUS TickType TickRefType AlarmBaseType AlarmBaseRefType RES_SCHEDULER RUNNING WAITING READY SUSPENDED".split_whitespace().map(str::to_owned));
    names.extend("ActivateTask CancelAlarm ChainTask ClearEvent ClearPendingInterrupt ControlIdle DisableAllInterrupts DisableInterruptSource EnableAllInterrupts EnableInterruptSource GetActiveApplicationMode GetAlarm GetAlarmBase GetCoreID GetCounterValue GetElapsedValue GetEvent GetISRID GetResource GetScheduleTableStatus GetTaskID GetTaskState IncrementCounter NextScheduleTable ReleaseResource ResumeAllInterrupts ResumeOSInterrupts Schedule SetAbsAlarm SetRelAlarm SetEvent ShutdownHook ShutdownOS StartOS StartScheduleTableAbs StartScheduleTableRel StartupHook StopScheduleTable SuspendAllInterrupts SuspendOSInterrupts TerminateTask WaitEvent isOsStarted ErrorHook PostTaskHook PreTaskHook".split_whitespace().map(str::to_owned));
    names.extend("main abort exit _Exit malloc calloc realloc free memcpy memmove memset memcmp memchr strlen strcpy strncpy strcat strncat strcmp strncmp strchr strrchr strspn strcspn strpbrk strstr strtok strcoll strxfrm strerror printf fprintf sprintf snprintf vprintf vfprintf vsprintf vsnprintf scanf fscanf sscanf getchar putchar puts fputs fgets fopen fclose freopen fread fwrite fflush feof ferror clearerr fseek ftell rewind fgetpos fsetpos remove rename tmpfile tmpnam setbuf setvbuf ungetc fgetc getc fputc putc perror atoi atol atoll atof strtol strtoul strtoll strtoull strtod strtof strtold rand srand getenv system bsearch qsort abs labs llabs div ldiv lldiv mblen mbtowc wctomb mbstowcs wcstombs signal raise setjmp longjmp assert atexit clock time difftime mktime asctime ctime gmtime localtime strftime size_t ptrdiff_t wchar_t FILE fpos_t tm jmp_buf va_list sig_atomic_t clock_t time_t div_t ldiv_t lldiv_t".split_whitespace().map(str::to_owned));
    for width in [8, 16, 32, 64] {
        for signedness in ["int", "uint"] {
            for size in ["", "_least", "_fast"] {
                names.insert(format!("{signedness}{size}{width}_t"));
            }
        }
        for prefix in [
            "INT",
            "UINT",
            "INT_LEAST",
            "UINT_LEAST",
            "INT_FAST",
            "UINT_FAST",
        ] {
            names.insert(format!("{prefix}{width}_MAX"));
            names.insert(format!("{prefix}{width}_MIN"));
        }
        names.insert(format!("INT{width}_C"));
        names.insert(format!("UINT{width}_C"));
    }
    for module in ["COM", "OS"] {
        for field in ["MAJOR", "MINOR", "REVISION"] {
            names.insert(format!("{module}_AR_RELEASE_{field}_VERSION"));
        }
    }
    names.insert("COM_CFG_H".into());
    for signal in graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| {
            graph
                .text(*index, "DEFINITION-REF")
                .is_some_and(|definition| definition.ends_with("/ComSignal"))
        })
    {
        if !names.insert(super::com::symbolic_name(&graph.elements[signal].object)) {
            return Err(fail(graph, signal, "CONTRACT_NAME_COLLISION"));
        }
    }
    for component in &plan.components {
        let type_name = component
            .header
            .trim_start_matches("include/Rte_")
            .trim_end_matches(".h");
        names.insert(format!("{type_name}_MEMMAP_ACTIVE"));
        for segment in ["CODE", "VAR_CLEARED_UNSPECIFIED"] {
            for suffix in ["", "_ACTIVE"] {
                names.insert(format!("{type_name}_{segment}{suffix}"));
            }
            for direction in ["START", "STOP"] {
                names.insert(format!("{type_name}_{direction}_SEC_{segment}"));
            }
        }
        for segment in [
            "CODE",
            "VAR_CLEARED_UNSPECIFIED",
            "VAR_INIT_UNSPECIFIED",
            "CONST_UNSPECIFIED",
        ] {
            names.insert(format!("RTE_{type_name}_{segment}"));
            names.insert(format!("RTE_{type_name}_{segment}_ACTIVE"));
            for direction in ["START", "STOP"] {
                names.insert(format!("RTE_{type_name}_{direction}_SEC_{segment}"));
            }
        }
    }
    names.extend(
        super::rte_types::STATUSES
            .iter()
            .map(|(name, _)| (*name).to_owned()),
    );
    names.extend(
        [
            "RTE_APPLICATION_HEADER_FILE",
            "RTE_DATA_HANDLE_TYPE_H",
            "SCHM_CAN_TYPE_H",
            "SCHM_CANIF_TYPE_H",
            "SCHM_CANTP_TYPE_H",
            "SCHM_COM_TYPE_H",
            "SCHM_COMM_TYPE_H",
            "SCHM_BSWM_TYPE_H",
            "SCHM_DCM_TYPE_H",
            "SCHM_DET_TYPE_H",
            "SCHM_PDUR_TYPE_H",
            "SCHM_LSDUR_TYPE_H",
            "Rte_Instance",
            "Rte_IsInfrastructureError",
            "Rte_HasOverlayedError",
            "Rte_ApplicationError",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    for component in &plan.components {
        let scope = super::short_name(&component.component);
        for prefix in ["Rte_CDS", "Rte_Instance", "Rte_Inst"] {
            names.insert(format!("{prefix}_{scope}"));
        }
        names.insert(format!("RTE_{scope}_TYPE_H"));
        for runnable in &component.runnables {
            names.insert(format!(
                "RTE_RUNNABLE_{}",
                super::short_name(&runnable.path)
            ));
        }
        for port in &component.data_ports {
            names.insert(format!("Rte_InitValue_{}_{}", port.name, port.element_name));
        }
    }
    for datatype in plan.types.iter().filter(|datatype| {
        datatype.array_length.is_some() || datatype.symbol != datatype.native_type
    }) {
        if c_name(&datatype.symbol) != datatype.symbol
            || super::contracts::reserved_identifier(&datatype.symbol)
            || runtime_namespace(&datatype.symbol)
            || !names.insert(datatype.symbol.clone())
        {
            return Err(fail(
                graph,
                graph.index(&datatype.path)?,
                "CONTRACT_NAME_COLLISION",
            ));
        }
    }
    let errors: BTreeSet<_> = plan
        .components
        .iter()
        .flat_map(|component| &component.operations)
        .filter_map(|operation| operation.application_error.as_ref())
        .collect();
    for error in errors {
        if !names.insert(error.clone()) {
            return Err(fail(
                graph,
                graph.index(&plan.composition)?,
                "CONTRACT_NAME_COLLISION",
            ));
        }
    }
    for data in graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| {
            graph.text(*index, "DEFINITION-REF")
                == Some("/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsp/DcmDspData")
        })
    {
        for alias in dcm_data_type_names(&graph.elements[data].object) {
            if !plan.array_types.values().any(|native| native == &alias) && !names.insert(alias) {
                return Err(fail(graph, data, "CONTRACT_NAME_COLLISION"));
            }
        }
    }
    for component in &plan.components {
        let index = graph.index(&component.component)?;
        let type_name = component
            .header
            .trim_start_matches("include/Rte_")
            .trim_end_matches(".h");
        let guard = format!("RTE_{}_H", type_name.to_uppercase());
        if super::contracts::reserved_identifier(type_name)
            || reserved_memory_scope(type_name)
            || !filenames.insert(component.header.to_uppercase())
            || !filenames.insert(format!("INCLUDE/RTE_{}_TYPE.H", type_name.to_uppercase()))
            || !filenames.insert(format!("INCLUDE/{}_MEMMAP.H", type_name.to_uppercase()))
            || !names.insert(guard.clone())
        {
            return Err(fail(graph, index, "CONTRACT_NAME_COLLISION"));
        }
        let mut aliases = BTreeSet::new();
        for port in &component.data_ports {
            if !aliases.insert(port.api_symbol.clone())
                || !names.insert(data_symbol(component, port))
            {
                return Err(fail(graph, index, "SYMBOL_NORMALIZATION_COLLISION"));
            }
        }
        for operation in component
            .operations
            .iter()
            .filter(|operation| operation.read)
        {
            if !aliases.insert(operation.api_symbol.clone())
                || !names.insert(operation.implementation_symbol.clone())
            {
                return Err(fail(graph, index, "SYMBOL_NORMALIZATION_COLLISION"));
            }
        }
        for runnable in &component.runnables {
            if super::contracts::reserved_identifier(&runnable.symbol)
                || runtime_namespace(&runnable.symbol)
                || !names.insert(runnable.symbol.clone())
            {
                return Err(fail(
                    graph,
                    graph.index(&runnable.path)?,
                    "SYMBOL_PRODUCER_DUPLICATE",
                ));
            }
        }
    }
    // RTE wrappers include every component header and call other components.
    // Check parameters only after registering the complete producer namespace.
    for component in &plan.components {
        let index = graph.index(&component.component)?;
        let aliases = component
            .data_ports
            .iter()
            .map(|port| port.api_symbol.clone())
            .chain(
                component
                    .operations
                    .iter()
                    .filter(|operation| operation.read)
                    .map(|operation| operation.api_symbol.clone()),
            )
            .collect::<BTreeSet<_>>();
        if aliases.iter().any(|alias| names.contains(alias)) {
            return Err(fail(graph, index, "CONTRACT_NAME_COLLISION"));
        }
        let mut local_names = names.clone();
        local_names.extend(aliases);
        for operation in &component.operations {
            let mut arguments = BTreeSet::new();
            if operation.arguments.iter().any(|argument| {
                super::contracts::reserved_identifier(&argument.name)
                    || local_names.contains(&argument.name)
                    || !arguments.insert(argument.name.clone())
            }) {
                return Err(fail(
                    graph,
                    graph.index(&operation.operation)?,
                    "CONTRACT_NAME_COLLISION",
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn symbols(plan: &CompositionContract) -> Vec<SymbolContract> {
    let mut symbols = Vec::new();
    for component in &plan.components {
        for port in &component.data_ports {
            symbols.push(SymbolContract {
                symbol: data_symbol(component, port),
                return_type: "Std_ReturnType".into(),
                arguments: vec![ContractArgument {
                    name: "data".into(),
                    native_type: if port.read {
                        format!("{} *", port.c_type)
                    } else {
                        port.c_type.clone()
                    },
                    direction: if port.read { "OUT" } else { "IN" }.into(),
                }],
                declaration_owner: component.header.clone(),
                definition_owner: "src/Rte.c".into(),
                consumers: component
                    .data_accesses
                    .iter()
                    .filter(|access| access.endpoint.port == port.path)
                    .map(|access| access.runnable.clone())
                    .collect(),
            });
        }
        for operation in component
            .operations
            .iter()
            .filter(|operation| operation.read)
        {
            symbols.push(SymbolContract {
                symbol: operation.implementation_symbol.clone(),
                return_type: "Std_ReturnType".into(),
                arguments: operation.arguments.clone(),
                declaration_owner: component.header.clone(),
                definition_owner: "src/Rte.c".into(),
                consumers: operation.callers.clone(),
            });
        }
        for runnable in &component.runnables {
            let operation = component.operations.iter().find(|operation| {
                operation.runnable == runnable.path && (!operation.read || runnable.event.is_none())
            });
            symbols.push(SymbolContract {
                symbol: runnable.symbol.clone(),
                return_type: if runnable.event.is_none()
                    || operation.is_some_and(|operation| operation.application_error.is_some())
                {
                    "Std_ReturnType"
                } else {
                    "void"
                }
                .into(),
                arguments: operation
                    .map(|operation| operation.arguments.clone())
                    .unwrap_or_default(),
                declaration_owner: component.header.clone(),
                definition_owner: if runnable.event.is_none() {
                    "src/Rte.c".into()
                } else {
                    format!(
                        "application/{}.c",
                        c_name(super::short_name(&component.component))
                    )
                },
                consumers: if runnable.event.is_none() {
                    vec!["runtime/bsw/src/Dcm.c".into()]
                } else if let Some(operation) = operation {
                    plan.connections
                        .iter()
                        .filter(|connection| {
                            connection.provider.instance == component.instance
                                && connection.provider.port == operation.port
                                && connection.provider.member == operation.operation
                        })
                        .map(|connection| connection.requester.port.clone())
                        .collect()
                } else {
                    runnable.event.iter().cloned().collect()
                },
            });
        }
    }
    symbols
}
