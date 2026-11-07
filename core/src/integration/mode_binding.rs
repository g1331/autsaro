//! Synchronized mode groups: one SchM-owned machine per publisher port instance.
use super::component::{one, referenced};
use super::composition::{Component, CompositionContract, Endpoint};
use super::graph::Graph;
use super::{ContractArgument, DiagnosticCategory, PlanDiagnostic, SymbolContract, c_layout};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct ModePort {
    pub port: String,
    pub prototype: String,
    pub group: String,
    pub read: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct ModeApi {
    pub point: String,
    pub runnable: String,
    pub port: String,
    pub symbol: String,
    pub alias: String,
    pub switch: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct ModeMachine {
    pub publisher: Endpoint,
    pub mapping: Option<String>,
    pub bsw_behavior: String,
    pub bsw_group: String,
    pub module: String,
    pub group: String,
    pub initial: u32,
    pub values: Vec<(String, u32)>,
    pub readers: Vec<Endpoint>,
    pub apis: Vec<ModeApi>,
    pub bsw_managed: bool,
    pub diagnostic: bool,
    pub managers: Vec<String>,
    pub manager_symbols: Vec<String>,
    pub switch_symbol: String,
    pub mode_symbol: String,
    pub variable: String,
}

fn reject(graph: &Graph, at: usize, code: &str) -> Vec<PlanDiagnostic> {
    let at = if graph.elements[at].tag == "MODE-ACCESS-POINT" {
        graph.children(at, "IDENT").first().copied().unwrap_or(at)
    } else {
        at
    };
    vec![graph.diagnostic(
        at,
        DiagnosticCategory::Input,
        code,
        crate::product_message!("backend.integration.composition.contract_invalid", "code" => code),
        crate::product_message!("backend.integration.composition.repair_contract"),
    )]
}

pub(super) fn port(
    graph: &Graph,
    port: usize,
    interface: usize,
) -> Result<ModePort, Vec<PlanDiagnostic>> {
    let prototype = one(
        graph,
        port,
        graph.descendants(interface, "MODE-GROUP"),
        "MODE_TYPE_CONFLICT",
    )?;
    let group = referenced(graph, prototype, "TYPE-TREF", "MODE_TYPE_CONFLICT")?;
    let read = graph.elements[port].tag == "R-PORT-PROTOTYPE";
    let specs = graph.descendants(
        port,
        if read {
            "MODE-SWITCH-RECEIVER-COM-SPEC"
        } else {
            "MODE-SWITCH-SENDER-COM-SPEC"
        },
    );
    if specs.len() > 1
        || specs.iter().any(|spec| {
            graph
                .target(*spec, "MODE-GROUP-REF")
                .is_some_and(|target| target != prototype)
                || matches!(graph.text(*spec, "ENHANCED-MODE-API"), Some("true" | "1"))
                || matches!(
                    graph.text(*spec, "SUPPORTS-ASYNCHRONOUS-MODE-SWITCH"),
                    Some("true" | "1")
                )
        })
    {
        return Err(reject(graph, port, "MODE_API_UNSUPPORTED"));
    }
    Ok(ModePort {
        port: graph.elements[port].object.clone(),
        prototype: graph.elements[prototype].object.clone(),
        group: graph.elements[group].object.clone(),
        read,
    })
}

struct DeclarationValues {
    initial: u32,
    values: Vec<(String, u32)>,
}

fn declarations(graph: &Graph, group: usize) -> Result<DeclarationValues, Vec<PlanDiagnostic>> {
    let declarations = graph.descendants(group, "MODE-DECLARATION");
    let alphabetic = graph.text(group, "CATEGORY") == Some("ALPHABETIC_ORDER");
    if !alphabetic && graph.text(group, "CATEGORY") != Some("EXPLICIT_ORDER") {
        return Err(reject(graph, group, "MODE_TYPE_CONFLICT"));
    }
    let mut ordered = declarations.clone();
    if alphabetic {
        ordered.sort_by_key(|index| graph.text(*index, "SHORT-NAME").unwrap_or(""));
    }
    let mut values = Vec::new();
    let mut used = BTreeSet::new();
    let mut initial = None;
    for (ordinal, declaration) in ordered.iter().enumerate() {
        let value = if alphabetic {
            u32::try_from(ordinal).ok()
        } else {
            graph
                .text(*declaration, "VALUE")
                .and_then(|value| value.parse::<u32>().ok())
        }
        .ok_or_else(|| reject(graph, *declaration, "MODE_TYPE_CONFLICT"))?;
        if !used.insert(value)
            || graph
                .text(group, "ON-TRANSITION-VALUE")
                .and_then(|text| text.parse::<u32>().ok())
                == Some(value)
        {
            return Err(reject(graph, *declaration, "MODE_TYPE_CONFLICT"));
        }
        if graph.target(group, "INITIAL-MODE-REF") == Some(*declaration) {
            initial = Some(value);
        }
        values.push((
            graph
                .text(*declaration, "SHORT-NAME")
                .unwrap_or("")
                .to_owned(),
            value,
        ));
    }
    Ok(DeclarationValues {
        initial: initial.ok_or_else(|| reject(graph, group, "MODE_INITIAL_VALUE"))?,
        values,
    })
}

fn selected_behavior(
    graph: &Graph,
    mapping: usize,
    behavior: usize,
) -> Result<(usize, String), Vec<PlanDiagnostic>> {
    let owner = graph
        .ancestor(behavior, "BSW-MODULE-DESCRIPTION")
        .ok_or_else(|| reject(graph, mapping, "BSW_MAPPING_INVALID"))?;
    let implementations: Vec<_> = graph
        .of_kind("BSW-IMPLEMENTATION")
        .into_iter()
        .filter(|implementation| {
            graph
                .target(*implementation, "BEHAVIOR-REF")
                .is_some_and(|target| graph.within(target, owner))
        })
        .collect();
    if implementations.len() != 1
        || graph.target(implementations[0], "BEHAVIOR-REF") != Some(behavior)
        || graph.text(implementations[0], "PROGRAMMING-LANGUAGE") != Some("C")
    {
        return Err(reject(graph, mapping, "BSW_MAPPING_INVALID"));
    }
    Ok((
        owner,
        graph.text(owner, "SHORT-NAME").unwrap_or("").to_owned(),
    ))
}

pub(super) fn inspect(
    graph: &Graph,
    plan: &CompositionContract,
) -> Result<Vec<ModeMachine>, Vec<PlanDiagnostic>> {
    let mut machines = Vec::new();
    let mut bound_groups = BTreeSet::new();
    for component in &plan.components {
        for port in component.mode_ports.iter().filter(|port| !port.read) {
            let group = graph.index(&port.group)?;
            // Existing Dcm contracts are implicit source mappings. Only these two
            // selected groups have actual publishers in the fixed Dcm backend.
            let fixed = component
                .diagnostic_mode_ports
                .iter()
                .any(|fixed| fixed.port == port.port);
            if fixed
                && !matches!(
                    super::short_name(&port.group),
                    "DcmDiagnosticSessionControl"
                )
                && !super::short_name(&port.group).starts_with("DcmAuthenticationState_")
            {
                continue;
            }
            let rows: Vec<_> = graph
                .of_kind("SWC-BSW-MAPPING")
                .into_iter()
                .flat_map(|mapping| {
                    graph
                        .descendants(mapping, "SWC-BSW-SYNCHRONIZED-MODE-GROUP-PROTOTYPE")
                        .into_iter()
                        .filter_map(move |row| {
                            let iref = graph
                                .children(row, "SWC-MODE-GROUP-IREF")
                                .first()
                                .copied()?;
                            (graph.target(iref, "CONTEXT-P-PORT-REF")
                                == graph.objects.get(&port.port).copied())
                            .then_some((mapping, row, iref))
                        })
                })
                .collect();
            let (mapping, behavior, bsw_group, module, managed) = if rows.is_empty() && fixed {
                let owner = graph.index("/AUTOSAR_Dcm/Dcm")?;
                let implementations: Vec<_> = graph
                    .of_kind("BSW-IMPLEMENTATION")
                    .into_iter()
                    .filter_map(|implementation| {
                        graph
                            .target(implementation, "BEHAVIOR-REF")
                            .filter(|behavior| graph.within(*behavior, owner))
                    })
                    .collect();
                let behavior = one(graph, owner, implementations, "BSW_MAPPING_INVALID")?;
                (
                    None,
                    behavior,
                    graph.index(&format!(
                        "/AUTOSAR_Dcm/Dcm/{}",
                        super::short_name(&port.group)
                    ))?,
                    "Dcm".to_owned(),
                    true,
                )
            } else {
                if rows.len() != 1 {
                    return Err(reject(
                        graph,
                        graph.index(&port.port)?,
                        "MODE_MAPPING_INVALID",
                    ));
                }
                let (mapping, row, iref) = rows[0];
                let behavior =
                    referenced(graph, mapping, "BSW-BEHAVIOR-REF", "BSW_MAPPING_INVALID")?;
                let (owner, module) = selected_behavior(graph, mapping, behavior)?;
                if graph.elements[graph.index(&component.component)?].tag
                    != "SERVICE-SW-COMPONENT-TYPE"
                    || graph
                        .of_kind("SWC-IMPLEMENTATION")
                        .into_iter()
                        .filter(|implementation| {
                            graph.target(*implementation, "BEHAVIOR-REF")
                                == graph.objects.get(&component.behavior).copied()
                                && graph.text(*implementation, "PROGRAMMING-LANGUAGE") == Some("C")
                        })
                        .count()
                        != 1
                {
                    return Err(reject(graph, mapping, "BSW_MAPPING_INVALID"));
                }
                let bsw_group =
                    referenced(graph, row, "BSW-MODE-GROUP-REF", "MODE_MAPPING_INVALID")?;
                if graph.target(mapping, "SWC-BEHAVIOR-REF")
                    != graph.objects.get(&component.behavior).copied()
                    || !graph.within(bsw_group, owner)
                    || graph.target(iref, "TARGET-MODE-GROUP-REF")
                        != graph.objects.get(&port.prototype).copied()
                    || graph.target(bsw_group, "TYPE-TREF") != Some(group)
                {
                    return Err(reject(graph, mapping, "MODE_MAPPING_INVALID"));
                }
                let managed = graph
                    .descendants(behavior, "MANAGED-MODE-GROUPS")
                    .into_iter()
                    .any(|refs| {
                        graph
                            .descendants(refs, "MODE-DECLARATION-GROUP-PROTOTYPE-REF")
                            .into_iter()
                            .any(|reference| {
                                graph.elements[reference].text == graph.elements[bsw_group].object
                            })
                    });
                (
                    Some(graph.elements[mapping].object.clone()),
                    behavior,
                    bsw_group,
                    module,
                    managed,
                )
            };
            if fixed
                && (module != "Dcm"
                    || graph.elements[bsw_group].object
                        != format!("/AUTOSAR_Dcm/Dcm/{}", super::short_name(&port.group)))
            {
                return Err(reject(
                    graph,
                    graph.index(&port.port)?,
                    "MODE_MAPPING_INVALID",
                ));
            }
            if !bound_groups.insert(bsw_group) {
                return Err(reject(
                    graph,
                    graph.index(&port.port)?,
                    if fixed {
                        "SERVICE_CLIENT_MISSING"
                    } else {
                        "MODE_MULTIPLE_PUBLISHERS"
                    },
                ));
            }
            let declarations = declarations(graph, group)?;
            let bsw_name = super::short_name(&graph.elements[bsw_group].object);
            let mut machine = ModeMachine {
                publisher: Endpoint {
                    instance: component.instance.clone(),
                    port: port.port.clone(),
                    member: port.prototype.clone(),
                },
                mapping,
                bsw_behavior: graph.elements[behavior].object.clone(),
                bsw_group: graph.elements[bsw_group].object.clone(),
                module: module.clone(),
                group: port.group.clone(),
                initial: declarations.initial,
                values: declarations.values,
                readers: Vec::new(),
                apis: Vec::new(),
                bsw_managed: managed,
                diagnostic: fixed,
                manager_symbols: Vec::new(),
                managers: graph
                    .descendants(behavior, "MANAGED-MODE-GROUPS")
                    .into_iter()
                    .filter(|refs| {
                        graph
                            .descendants(*refs, "MODE-DECLARATION-GROUP-PROTOTYPE-REF")
                            .into_iter()
                            .any(|reference| {
                                graph.elements[reference].text == graph.elements[bsw_group].object
                            })
                    })
                    .filter_map(|refs| graph.elements[refs].parent)
                    .map(|entity| graph.elements[entity].object.clone())
                    .collect(),
                switch_symbol: format!("SchM_Switch_{module}_{bsw_name}"),
                mode_symbol: format!("SchM_Mode_{module}_{bsw_name}"),
                variable: format!(
                    "mode_{}_{}",
                    super::component::c_name(super::short_name(&component.component)),
                    super::component::c_name(super::short_name(&port.port))
                ),
            };
            for entity in &machine.managers {
                let entity = graph.index(entity)?;
                let entry = referenced(
                    graph,
                    entity,
                    "IMPLEMENTED-ENTRY-REF",
                    "BSW_MAPPING_INVALID",
                )?;
                machine
                    .manager_symbols
                    .push(graph.text(entry, "SHORT-NAME").unwrap_or("").to_owned());
            }
            if fixed_dcm(&machine) {
                machine.bsw_managed = true;
                machine.variable =
                    if super::short_name(&machine.group) == "DcmDiagnosticSessionControl" {
                        "diagnostic_session_mode"
                    } else {
                        "authentication_mode"
                    }
                    .into();
            }
            for connection in &plan.mode_connections {
                if connection.provider == machine.publisher {
                    machine.readers.push(connection.requester.clone());
                }
            }
            for reader in &plan.components {
                collect_apis(graph, reader, &mut machine)?;
            }
            if !machine.bsw_managed && !machine.apis.iter().any(|api| api.switch) {
                return Err(reject(
                    graph,
                    graph.index(&port.port)?,
                    "MODE_PUBLISHER_MISSING",
                ));
            }
            machines.push(machine);
        }
    }
    for mapping in graph.of_kind("SWC-BSW-MAPPING") {
        for row in graph.descendants(mapping, "SWC-BSW-SYNCHRONIZED-MODE-GROUP-PROTOTYPE") {
            let group = referenced(graph, row, "BSW-MODE-GROUP-REF", "MODE_MAPPING_INVALID")?;
            if !machines.iter().any(|machine| {
                machine.mapping.as_deref() == Some(graph.elements[mapping].object.as_str())
                    && machine.bsw_group == graph.elements[group].object
            }) {
                return Err(reject(graph, mapping, "MODE_MAPPING_INVALID"));
            }
        }
    }
    for implementation in graph.of_kind("BSW-IMPLEMENTATION") {
        let Some(behavior) = graph.target(implementation, "BEHAVIOR-REF") else {
            continue;
        };
        for reference in graph
            .descendants(behavior, "MANAGED-MODE-GROUPS")
            .into_iter()
            .flat_map(|groups| graph.descendants(groups, "MODE-DECLARATION-GROUP-PROTOTYPE-REF"))
        {
            if !machines.iter().any(|machine| {
                machine.bsw_behavior == graph.elements[behavior].object
                    && machine.bsw_group == graph.elements[reference].text
            }) {
                return Err(reject(graph, reference, "MODE_MAPPING_INVALID"));
            }
        }
    }
    // Every accepted mode point and explicit synchronization must have a consumer.
    for component in &plan.components {
        for point in graph
            .descendants(graph.index(&component.behavior)?, "MODE-ACCESS-POINT")
            .into_iter()
            .chain(graph.descendants(graph.index(&component.behavior)?, "MODE-SWITCH-POINT"))
        {
            if !machines.iter().any(|machine| {
                machine
                    .apis
                    .iter()
                    .any(|api| api.point == point_path(graph, point))
            }) {
                return Err(reject(graph, point, "MODE_ACCESS_UNCONNECTED"));
            }
        }
    }
    Ok(machines)
}

fn point_target(
    graph: &Graph,
    point: usize,
    switch: bool,
) -> Result<(usize, usize), Vec<PlanDiagnostic>> {
    let iref = one(
        graph,
        point,
        graph.children(point, "MODE-GROUP-IREF"),
        "MODE_MAPPING_INVALID",
    )?;
    let context = if switch {
        if graph.elements[iref].children.iter().any(|child| {
            !matches!(
                graph.elements[*child].tag.as_str(),
                "CONTEXT-P-PORT-REF" | "TARGET-MODE-GROUP-REF"
            )
        }) {
            return Err(reject(graph, point, "MODE_MAPPING_INVALID"));
        }
        iref
    } else {
        let wrappers = &graph.elements[iref].children;
        if wrappers.len() != 1
            || !matches!(
                graph.elements[wrappers[0]].tag.as_str(),
                "P-MODE-GROUP-IN-ATOMIC-SWC-INSTANCE-REF"
                    | "R-MODE-GROUP-IN-ATOMIC-SWC-INSTANCE-REF"
            )
        {
            return Err(reject(graph, point, "MODE_MAPPING_INVALID"));
        }
        wrappers[0]
    };
    let refs = graph
        .children(context, "CONTEXT-R-PORT-REF")
        .into_iter()
        .chain(graph.children(context, "CONTEXT-P-PORT-REF"))
        .collect();
    let reference = one(graph, point, refs, "MODE_MAPPING_INVALID")?;
    let port = graph.index(&graph.elements[reference].text)?;
    let prototype = referenced(
        graph,
        context,
        "TARGET-MODE-GROUP-REF",
        "MODE_MAPPING_INVALID",
    )?;
    Ok((port, prototype))
}

fn collect_apis(
    graph: &Graph,
    component: &Component,
    machine: &mut ModeMachine,
) -> Result<(), Vec<PlanDiagnostic>> {
    let behavior = graph.index(&component.behavior)?;
    for point in graph
        .descendants(behavior, "MODE-ACCESS-POINT")
        .into_iter()
        .chain(graph.descendants(behavior, "MODE-SWITCH-POINT"))
    {
        let switch = graph.elements[point].tag == "MODE-SWITCH-POINT";
        let (port, prototype) = point_target(graph, point, switch)?;
        let Some(endpoint) = machine
            .readers
            .iter()
            .chain(std::iter::once(&machine.publisher))
            .find(|endpoint| {
                endpoint.instance == component.instance
                    && endpoint.port == graph.elements[port].object
            })
        else {
            continue;
        };
        if graph.elements[prototype].object != endpoint.member
            || !graph.within(port, graph.index(&component.component)?)
        {
            return Err(reject(graph, point, "MODE_TYPE_CONFLICT"));
        }
        if switch && (machine.bsw_managed || endpoint != &machine.publisher) {
            return Err(reject(graph, point, "MODE_MULTIPLE_PUBLISHERS"));
        }
        let runnable = graph
            .ancestor(point, "RUNNABLE-ENTITY")
            .ok_or_else(|| reject(graph, point, "MODE_MAPPING_INVALID"))?;
        let operation = if switch { "Switch" } else { "Mode" };
        let alias = format!(
            "Rte_{operation}_{}_{}",
            super::short_name(&endpoint.port),
            super::short_name(&endpoint.member)
        );
        machine.apis.push(ModeApi {
            point: point_path(graph, point),
            runnable: graph.elements[runnable].object.clone(),
            port: endpoint.port.clone(),
            symbol: format!(
                "Rte_{operation}_{}_{}_{}",
                super::short_name(&component.component),
                super::short_name(&endpoint.port),
                super::short_name(&endpoint.member)
            ),
            alias,
            switch,
        });
    }
    Ok(())
}

impl ModeMachine {
    pub(super) fn native_type(&self) -> String {
        format!("Rte_ModeType_{}", super::short_name(&self.group))
    }

    pub(super) fn base_type(&self) -> &'static str {
        let maximum = self
            .values
            .iter()
            .map(|(_, value)| *value)
            .max()
            .unwrap_or(0);
        if maximum <= u32::from(u8::MAX) {
            "uint8"
        } else if maximum <= u32::from(u16::MAX) {
            "uint16"
        } else {
            "uint32"
        }
    }
}

pub(super) fn symbols(plan: &CompositionContract) -> Vec<SymbolContract> {
    let mut symbols: BTreeMap<String, SymbolContract> = BTreeMap::new();
    for machine in &plan.mode_machines {
        for api in &machine.apis {
            let Some(component) = plan.components.iter().find(|component| {
                component
                    .mode_ports
                    .iter()
                    .any(|port| port.port == api.port)
            }) else {
                continue;
            };
            let symbol = symbols
                .entry(api.symbol.clone())
                .or_insert_with(|| SymbolContract {
                    symbol: api.symbol.clone(),
                    return_type: if api.switch {
                        "Std_ReturnType".into()
                    } else {
                        machine.native_type()
                    },
                    arguments: if api.switch {
                        vec![ContractArgument {
                            name: "mode".into(),
                            native_type: machine.native_type(),
                            direction: "IN".into(),
                        }]
                    } else {
                        Vec::new()
                    },
                    declaration_owner: component.header.clone(),
                    definition_owner: "src/Rte.c".into(),
                    consumers: Vec::new(),
                });
            symbol.consumers.push(api.runnable.clone());
        }
    }
    symbols.into_values().collect()
}

pub(super) fn type_declarations(plan: &CompositionContract) -> String {
    let mut text = String::new();
    let mut groups = BTreeSet::new();
    for machine in &plan.mode_machines {
        if fixed_dcm(machine) || !groups.insert(&machine.group) {
            continue;
        }
        text.push_str(&format!(
            "/** Mode value selected by the input declaration group. */\ntypedef {} {};\n",
            machine.base_type(),
            machine.native_type()
        ));
        let group = super::short_name(&machine.group);
        let values: Vec<_> = machine
            .values
            .iter()
            .map(|(name, value)| (format!("RTE_MODE_{group}_{name}"), format!("{value}u")))
            .collect();
        text.push_str(&c_layout::commented_defines(
            &values,
            "Declared mode value.",
        ));
    }
    text
}

pub(super) fn validate_owners(
    graph: &Graph,
    plan: &CompositionContract,
    symbols: &[SymbolContract],
    catalog: &super::catalog::RuntimeCatalog,
) -> Result<(), Vec<PlanDiagnostic>> {
    for machine in &plan.mode_machines {
        let behavior = graph.index(&machine.bsw_behavior)?;
        let at = graph.index(
            machine
                .mapping
                .as_deref()
                .unwrap_or(&machine.publisher.port),
        )?;
        let (owner, module) = selected_behavior(graph, at, behavior)?;
        let entries = graph.descendants(owner, "BSW-MODULE-ENTRY-REF");
        if entries.is_empty()
            || entries.iter().any(|reference| {
                let entry = &graph.elements[*reference].text;
                !symbols.iter().any(|symbol| {
                    symbol.consumers.contains(entry)
                        && catalog.module(&symbol.symbol) == Some(module.as_str())
                })
            })
        {
            return Err(reject(graph, at, "BSW_EXECUTION_CONFLICT"));
        }
        if !matches!(
            module.as_str(),
            "Can" | "CanIf" | "CanTp" | "Com" | "ComM" | "BswM" | "Dcm" | "Det" | "PduR" | "LSduR"
        ) {
            return Err(reject(graph, at, "MODE_API_UNSUPPORTED"));
        }
        for manager in &machine.managers {
            let manager = graph.index(manager)?;
            let entry = referenced(
                graph,
                manager,
                "IMPLEMENTED-ENTRY-REF",
                "BSW_MAPPING_INVALID",
            )?;
            let entry_path = &graph.elements[entry].object;
            if !entries
                .iter()
                .any(|reference| graph.elements[*reference].text == *entry_path)
                || !symbols.iter().any(|symbol| {
                    symbol.consumers.contains(entry_path)
                        && catalog.module(&symbol.symbol) == Some(module.as_str())
                })
            {
                return Err(reject(graph, at, "BSW_EXECUTION_CONFLICT"));
            }
        }
        if !graph.within(graph.index(&machine.bsw_group)?, owner) {
            return Err(reject(graph, at, "MODE_MAPPING_INVALID"));
        }
    }
    Ok(())
}

pub(super) fn declarations_for(plan: &CompositionContract, module: &str) -> String {
    let mut text = String::new();
    for machine in plan
        .mode_machines
        .iter()
        .filter(|machine| machine.module == module && !fixed_dcm(machine))
    {
        for (symbol, returns, parameters, brief) in [
            (
                &machine.switch_symbol,
                "Std_ReturnType".to_owned(),
                vec![format!("{} mode", machine.native_type())],
                "Publish the immediate mode; returns SCHM_E_OK.",
            ),
            (
                &machine.mode_symbol,
                machine.native_type(),
                vec!["void".into()],
                "Read the current mode; returns the last published declaration value.",
            ),
        ] {
            let mut contract = Vec::new();
            if parameters[0] != "void" {
                contract.push(c_layout::ContractLine::Detail(
                    "@param[in] mode Mode declaration value.",
                ));
                contract.push(c_layout::ContractLine::Detail(
                    "@return SCHM_E_OK after publication.",
                ));
            } else {
                contract.push(c_layout::ContractLine::Detail(
                    "@return Current mode declaration value.",
                ));
            }
            contract.extend([
                c_layout::ContractLine::Condition(
                    "Preconditions",
                    "SchM initialized; mode is a declared value.",
                ),
                c_layout::ContractLine::Condition(
                    "Context",
                    "ECU task or callback, outside exclusive areas.",
                ),
                c_layout::ContractLine::Condition(
                    "Reentrancy",
                    "reentrant; shared SchM state is protected.",
                ),
                c_layout::ContractLine::Condition("Ownership", "no pointer is retained."),
            ]);
            text.push_str(&c_layout::contract_comment(brief, contract, true));
            text.push_str(&c_layout::signature(
                &format!("RTE_Rte_CODE {returns}"),
                symbol,
                &parameters,
                ";",
                true,
            ));
            text.push('\n');
        }
    }
    if text.is_empty() {
        text
    } else {
        format!(
            "{}{}{}{}{}",
            c_layout::linkage(true),
            c_layout::section("RTE_Rte", "Rte", "CODE", true),
            text,
            c_layout::section("RTE_Rte", "Rte", "CODE", false),
            c_layout::linkage(false)
        )
    }
}

struct Storage {
    native: String,
    variable: String,
    switch: String,
    observer: String,
    group: String,
}

impl From<&ModeMachine> for Storage {
    fn from(machine: &ModeMachine) -> Self {
        Self {
            native: machine.native_type(),
            variable: machine.variable.clone(),
            switch: machine.switch_symbol.clone(),
            observer: machine.mode_symbol.clone(),
            group: super::short_name(&machine.group).into(),
        }
    }
}

/// The same SchM producer renders fixed Dcm bridges and ordinary synchronized groups.
pub(super) fn storage(plan: &CompositionContract, scope: &str, authentication: &str) -> String {
    let fixed: Vec<_> = [
        ("DcmDiagnosticSessionControl", "diagnostic_session_mode"),
        (authentication, "authentication_mode"),
    ]
    .into_iter()
    .map(|(group, variable)| {
        plan.mode_machines
            .iter()
            .find(|machine| fixed_dcm(machine) && super::short_name(&machine.group) == group)
            .map(Storage::from)
            // The no-diagnostic library retains the existing public Dcm bridges.
            .unwrap_or_else(|| Storage {
                native: format!("Rte_ModeType_{group}"),
                variable: variable.into(),
                switch: format!("SchM_Switch_Dcm_{group}"),
                observer: format!("SchM_Mode_Dcm_{group}"),
                group: group.into(),
            })
    })
    .collect();
    let ordinary: Vec<_> = plan
        .mode_machines
        .iter()
        .filter(|machine| !fixed_dcm(machine))
        .map(Storage::from)
        .collect();
    format!(
        "{}{}",
        render_storage(&fixed, scope, true),
        render_storage(&ordinary, "Rte", false)
    )
}

fn render_storage(machines: &[Storage], scope: &str, diagnostic: bool) -> String {
    if machines.is_empty() {
        return String::new();
    }
    let prefix = format!("RTE_{scope}");
    let mut text = String::new();
    if diagnostic {
        text.push('\n');
    }
    text.push_str(&c_layout::section(
        &prefix,
        "Rte",
        "VAR_CLEARED_UNSPECIFIED",
        true,
    ));
    for machine in machines {
        let declaration = format!("static {} {}", machine.native, machine.variable);
        let note = if diagnostic {
            ""
        } else {
            " /**< SCHM_MODE; Init resets. */"
        };
        let attribute = format!("{prefix}_VAR_CLEARED_UNSPECIFIED;{note}");
        if declaration.len() + attribute.len() < 100 {
            text.push_str(&format!("{declaration} {attribute}\n"));
        } else {
            text.push_str(&format!("{declaration}\n    {attribute}\n"));
        }
    }
    text.push_str(&c_layout::section(
        &prefix,
        "Rte",
        "VAR_CLEARED_UNSPECIFIED",
        false,
    ));
    text.push_str(&c_layout::section(&prefix, "Rte", "CODE", true));
    let methods: Vec<_> = if diagnostic {
        machines.iter().rev().collect()
    } else {
        machines.iter().collect()
    };
    for machine in methods {
        text.push_str(&storage_functions(
            machine,
            if diagnostic {
                "RTE_CODE"
            } else {
                "RTE_Rte_CODE"
            },
            diagnostic,
        ));
    }
    text.push_str(&c_layout::section(&prefix, "Rte", "CODE", false));
    text
}

fn storage_functions(machine: &Storage, placement: &str, diagnostic: bool) -> String {
    let switch = c_layout::signature(
        &format!("{placement} Std_ReturnType"),
        &machine.switch,
        &[format!("{} mode", machine.native)],
        "",
        true,
    );
    let mode = c_layout::signature(
        &format!("{placement} {}", machine.native),
        &machine.observer,
        &["void".into()],
        "",
        true,
    );
    let retained_switch = if diagnostic && machine.group == "DcmAuthenticationState_Physical" {
        "/* MR12 RULE 8.7 VIOLATION: retain this public mode publisher's C linkage. */\n"
    } else {
        ""
    };
    let retained_mode = if diagnostic
        && matches!(
            machine.group.as_str(),
            "DcmAuthenticationState_Physical" | "DcmDiagnosticSessionControl"
        ) {
        "/* MR12 RULE 8.7 VIOLATION: retain this public mode observer's C linkage. */\n"
    } else {
        ""
    };
    format!(
        r#"{retained_switch}{switch}
{{
    Can_HostAreaEnter(CAN_HOST_AREA_SCHM_MODE);
    {variable} = mode;
    Can_HostAreaExit(CAN_HOST_AREA_SCHM_MODE);
    return SCHM_E_OK;
}}
{retained_mode}{mode}
{{
    {native} mode;
    Can_HostAreaEnter(CAN_HOST_AREA_SCHM_MODE);
    mode = {variable};
    Can_HostAreaExit(CAN_HOST_AREA_SCHM_MODE);
    return mode;
}}
"#,
        variable = machine.variable,
        native = machine.native
    )
}

pub(super) fn rte_functions(plan: &CompositionContract) -> String {
    let mut text = String::new();
    let mut emitted = BTreeSet::new();
    for machine in &plan.mode_machines {
        for api in &machine.apis {
            if !emitted.insert(&api.symbol) {
                continue;
            }
            let component = plan.components.iter().find(|component| {
                component
                    .mode_ports
                    .iter()
                    .any(|port| port.port == api.port)
            });
            let Some(component) = component else {
                continue;
            };
            let scope = super::short_name(&component.component);
            text.push_str(&c_layout::section(
                &format!("RTE_{scope}"),
                "Rte",
                "CODE",
                true,
            ));
            let returns = if api.switch {
                "Std_ReturnType".into()
            } else {
                machine.native_type()
            };
            let parameters = if api.switch {
                vec![format!("{} mode", machine.native_type())]
            } else {
                vec!["void".into()]
            };
            let signature = c_layout::signature(
                &format!("RTE_{scope}_CODE {returns}"),
                &api.symbol,
                &parameters,
                "",
                true,
            );
            let call = if api.switch {
                format!("{}(mode)", machine.switch_symbol)
            } else {
                format!("{}()", machine.mode_symbol)
            };
            text.push_str(&format!("{signature}\n{{\n    return {call};\n}}\n"));
            text.push_str(&c_layout::section(
                &format!("RTE_{scope}"),
                "Rte",
                "CODE",
                false,
            ));
        }
    }
    text
}

pub(super) fn fixed_dcm(machine: &ModeMachine) -> bool {
    machine.diagnostic
}

fn point_path(graph: &Graph, point: usize) -> String {
    graph.children(point, "IDENT").first().map_or_else(
        || graph.elements[point].object.clone(),
        |ident| graph.elements[*ident].object.clone(),
    )
}

impl ModeMachine {
    pub(super) fn delivered_group(&self) -> String {
        format!(
            "/AUTOSAR_{0}/{0}/{1}",
            self.module,
            super::short_name(&self.bsw_group)
        )
    }
}
