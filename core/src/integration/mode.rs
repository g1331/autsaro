//! Source-backed CDD channels and immediate, static BswM user callouts.
use super::component::milliseconds;
use super::graph::Graph;
use super::schedule::{ScheduleContract, definition_is, value, values};
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct ModeUser {
    pub path: String,
    pub handle: u16,
    pub channels: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct ModeRule {
    pub path: String,
    pub channel: String,
    pub mode: u8,
    pub condition: String,
    pub expression: String,
    pub action_list: String,
    pub action: String,
    pub callout: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct ModeChannel {
    pub channel: String,
    pub channel_handle: u8,
    pub can_if_controller: String,
    pub main_symbol: String,
    pub period_ms: u32,
    pub input: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
pub struct ModeRuntimeContract {
    pub channels: Vec<ModeChannel>,
    pub minimum_full_ms: u32,
    pub bus_prefix: String,
    pub users: Vec<ModeUser>,
    pub bswm_configuration: String,
    pub include: String,
    pub rules: Vec<ModeRule>,
    pub dcm_connections: BTreeMap<String, String>,
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
fn containers(graph: &Graph, kind: &str) -> Vec<usize> {
    graph
        .of_kind("ECUC-CONTAINER-VALUE")
        .into_iter()
        .filter(|index| definition_is(graph, *index, kind))
        .collect()
}
fn one(graph: &Graph, kind: &str) -> Result<usize, Vec<PlanDiagnostic>> {
    let found = containers(graph, kind);
    if found.len() == 1 {
        Ok(found[0])
    } else {
        Err(fail(
            graph,
            found.get(1).or_else(|| found.first()).copied().unwrap_or(0),
            "MODE_CONFIGURATION",
        ))
    }
}
fn reference(
    graph: &Graph,
    index: usize,
    name: &str,
    kind: &str,
) -> Result<usize, Vec<PlanDiagnostic>> {
    value(graph, index, name, true)
        .and_then(|path| graph.objects.get(path).copied())
        .filter(|target| definition_is(graph, *target, kind))
        .ok_or_else(|| fail(graph, index, "MODE_REFERENCE"))
}
fn child(graph: &Graph, owner: usize, kind: &str) -> Result<usize, Vec<PlanDiagnostic>> {
    let found: Vec<_> = containers(graph, kind)
        .into_iter()
        .filter(|index| graph.within(*index, owner))
        .collect();
    if found.len() == 1 {
        Ok(found[0])
    } else {
        Err(fail(graph, owner, "MODE_CONFIGURATION"))
    }
}
fn disabled(graph: &Graph, index: usize, names: &str) -> Result<(), Vec<PlanDiagnostic>> {
    for name in names.split_whitespace() {
        if !matches!(value(graph, index, name, false), Some("false" | "0")) {
            return Err(fail(graph, index, "MODE_FEATURE_UNSUPPORTED"));
        }
    }
    Ok(())
}
fn mode(text: Option<&str>) -> Option<u8> {
    match text {
        Some("COMM_NO_COMMUNICATION") => Some(0),
        Some("COMM_SILENT_COMMUNICATION") => Some(1),
        Some("COMM_FULL_COMMUNICATION") => Some(2),
        _ => None,
    }
}

pub(super) fn inspect(
    graph: &Graph,
    schedule: &ScheduleContract,
    communication: &super::can_runtime::CommunicationRuntimeContract,
    transport: Option<&super::diagnostic::DiagnosticTransportContract>,
) -> Result<ModeRuntimeContract, Vec<PlanDiagnostic>> {
    let general = one(graph, "ComMGeneral")?;
    disabled(
        graph,
        general,
        "ComMDevErrorDetect ComMDynamicPncToChannelMappingSupport ComMModeLimitationEnabled ComMPncSupport ComMResetAfterForcingNoComm ComMSynchronousWakeUp ComMVersionInfoApi ComMWakeupInhibitionEnabled",
    )?;
    // Only the disabled inhibition features read the classification; it is checked, not emitted.
    value(graph, general, "ComMEcuGroupClassification", false)
        .and_then(|text| text.parse::<u8>().ok())
        .ok_or_else(|| fail(graph, general, "MODE_HANDLE"))?;
    let minimum_full_ms = value(graph, general, "ComMTMinFullComModeDuration", false)
        .and_then(milliseconds)
        .filter(|duration| (1..=65000).contains(duration))
        .ok_or_else(|| fail(graph, general, "MODE_TIMEBASE"))?;
    let channel_objects = containers(graph, "ComMChannel");
    if channel_objects.len() != communication.channels.len() {
        let source = channel_objects
            .get(communication.channels.len())
            .copied()
            .unwrap_or(general);
        return Err(fail(graph, source, "MODE_CONFIGURATION"));
    }
    let mut channels = BTreeMap::new();
    let mut handles = BTreeSet::new();
    for channel in channel_objects {
        if value(graph, channel, "ComMBusType", false) != Some("COMM_BUS_TYPE_CDD")
            || value(graph, channel, "ComMCDDBusPrefix", false) != Some("Ecu_HostBusSM")
        {
            return Err(fail(graph, channel, "MODE_BUS_PROVIDER"));
        }
        disabled(
            graph,
            channel,
            "ComMFullCommRequestNotificationEnabled ComMNoCom ComMNoWakeup ComMNoWakeUpInhibitionNvmStorage",
        )?;
        let management = child(graph, channel, "ComMNetworkManagement")?;
        if value(graph, management, "ComMNmVariant", false) != Some("NONE") {
            return Err(fail(graph, management, "MODE_NM_VARIANT"));
        }
        let channel_handle = value(graph, channel, "ComMChannelId", false)
            .and_then(|text| text.parse::<u8>().ok())
            .ok_or_else(|| fail(graph, channel, "MODE_HANDLE"))?;
        let period_ms = value(graph, channel, "ComMMainFunctionPeriod", false)
            .and_then(milliseconds)
            .filter(|period| *period == schedule.counter_tick_ms)
            .ok_or_else(|| fail(graph, channel, "MODE_TIMEBASE"))?;
        let name = graph.text(channel, "SHORT-NAME").unwrap_or("");
        if !super::catalog::c_identifier(name) {
            return Err(fail(graph, channel, "MODE_MAIN_SYMBOL"));
        }
        let main_symbol = format!("ComM_MainFunction_{name}");
        if !handles.insert(channel_handle)
            || !schedule.entities.iter().any(|entity| {
                !entity.application && entity.symbol == main_symbol && entity.period_ms == period_ms
            })
        {
            return Err(fail(graph, channel, "MODE_TIMEBASE"));
        }
        channels.insert(
            channel,
            ModeChannel {
                channel: graph.elements[channel].object.clone(),
                channel_handle,
                can_if_controller: String::new(),
                main_symbol,
                period_ms,
                input: String::new(),
            },
        );
    }
    let configured_users = containers(graph, "ComMUser");
    if configured_users.is_empty() || configured_users.len() > usize::from(u16::MAX) {
        return Err(fail(graph, general, "MODE_USER_MAPPING"));
    }
    let mut mapped = BTreeMap::<usize, BTreeSet<usize>>::new();
    let mut covered = BTreeSet::new();
    for mapping in containers(graph, "ComMUserPerChannel") {
        let user = reference(graph, mapping, "ComMUserChannel", "ComMUser")?;
        let channel = graph
            .ancestor(mapping, "ECUC-CONTAINER-VALUE")
            .filter(|channel| channels.contains_key(channel))
            .ok_or_else(|| fail(graph, mapping, "MODE_USER_MAPPING"))?;
        if !mapped.entry(user).or_default().insert(channel) {
            return Err(fail(graph, mapping, "MODE_USER_MAPPING"));
        }
        covered.insert(channel);
    }
    if covered.len() != channels.len() {
        return Err(fail(graph, general, "MODE_USER_MAPPING"));
    }
    let mut users = Vec::new();
    let mut handles = BTreeSet::new();
    for user in configured_users {
        let handle = value(graph, user, "ComMUserIdentifier", false)
            .and_then(|text| text.parse::<u16>().ok())
            .filter(|handle| *handle != u16::MAX)
            .ok_or_else(|| fail(graph, user, "MODE_HANDLE"))?;
        let user_channels = mapped
            .get(&user)
            .filter(|_| handles.insert(handle))
            .ok_or_else(|| fail(graph, user, "MODE_USER_MAPPING"))?;
        users.push(ModeUser {
            path: graph.elements[user].object.clone(),
            handle,
            channels: user_channels
                .iter()
                .map(|channel| channels[channel].channel.clone())
                .collect(),
        });
    }
    users.sort_by(|left, right| left.path.cmp(&right.path));
    let bswm_general = one(graph, "BswMGeneral")?;
    disabled(
        graph,
        bswm_general,
        "BswMCanSMEnabled BswMDcmEnabled BswMDevErrorDetect BswMEcuMEnabled BswMEthIfEnabled BswMEthSMEnabled BswMFrSMEnabled BswMGenericRequestEnabled BswMJ1939DcmEnabled BswMJ1939NmEnabled BswMLinSMEnabled BswMLinTPEnabled BswMNmEnabled BswMNvMEnabled BswMSdControlEnabled BswMSdEnabled BswMVersionInfoApi",
    )?;
    if !matches!(
        value(graph, bswm_general, "BswMComMEnabled", false),
        Some("true" | "1")
    ) {
        return Err(fail(graph, bswm_general, "MODE_FEATURE_UNSUPPORTED"));
    }
    let include = child(graph, bswm_general, "BswMUserIncludeFiles")?;
    if values(graph, include, "BswMUserIncludeFile", false) != ["Ecu_HostBusSM.h"] {
        return Err(fail(graph, include, "MODE_INCLUDE"));
    }
    let config = one(graph, "BswMConfig")?;
    let arbitration = child(graph, config, "BswMArbitration")?;
    let control = child(graph, config, "BswMModeControl")?;
    let mut inputs = BTreeMap::new();
    for input in containers(graph, "BswMModeRequestPort") {
        if !graph.within(input, arbitration)
            || value(graph, input, "BswMRequestProcessing", false) != Some("BSWM_IMMEDIATE")
        {
            return Err(fail(graph, input, "MODE_PROCESSING"));
        }
        let indication = child(graph, input, "BswMComMIndication")?;
        let channel = reference(graph, indication, "BswMComMChannelRef", "ComMChannel")?;
        let config = channels
            .get_mut(&channel)
            .filter(|config| config.input.is_empty())
            .ok_or_else(|| fail(graph, indication, "MODE_REFERENCE"))?;
        config.input = graph.elements[input].object.clone();
        let initial = child(graph, input, "BswMModeInitValue")?;
        if mode(value(graph, initial, "BswMBswModeInitValue", false)) != Some(0) {
            return Err(fail(graph, initial, "MODE_INITIAL_VALUE"));
        }
        inputs.insert(input, channel);
    }
    if inputs.len() != channels.len() {
        return Err(fail(graph, arbitration, "MODE_REFERENCE"));
    }
    let mut rules = Vec::new();
    let mut legacy_callouts = Vec::new();
    let mut modes = BTreeSet::new();
    let mut conditions = BTreeSet::new();
    let mut expressions = BTreeSet::new();
    let mut lists = BTreeSet::new();
    let mut actions = BTreeSet::new();
    for rule in containers(graph, "BswMRule") {
        if !graph.within(rule, arbitration)
            || value(graph, rule, "BswMRuleInitState", false) != Some("BSWM_FALSE")
            || !values(graph, rule, "BswMRuleFalseActionList", true).is_empty()
        {
            return Err(fail(graph, rule, "MODE_RULE"));
        }
        disabled(graph, rule, "BswMNestedExecutionOnly")?;
        let expression = reference(
            graph,
            rule,
            "BswMRuleExpressionRef",
            "BswMLogicalExpression",
        )?;
        if !graph.within(expression, arbitration)
            || !matches!(
                value(graph, expression, "BswMLogicalOperator", false),
                None | Some("BSWM_AND")
            )
        {
            return Err(fail(graph, expression, "MODE_RULE"));
        }
        let condition = reference(graph, expression, "BswMArgumentRef", "BswMModeCondition")?;
        if !graph.within(condition, arbitration)
            || value(graph, condition, "BswMConditionType", false) != Some("BSWM_EQUALS")
        {
            return Err(fail(graph, condition, "MODE_RULE"));
        }
        let input = reference(graph, condition, "BswMConditionMode", "BswMModeRequestPort")?;
        let channel = inputs
            .get(&input)
            .ok_or_else(|| fail(graph, condition, "MODE_REFERENCE"))?;
        let config = channels
            .get_mut(channel)
            .ok_or_else(|| fail(graph, condition, "MODE_REFERENCE"))?;
        let channel_handle = config.channel_handle;
        let expected = child(graph, condition, "BswMBswMode")?;
        let mode = mode(value(graph, expected, "BswMBswRequestedMode", false))
            .ok_or_else(|| fail(graph, expected, "MODE_RULE"))?;
        let list = reference(graph, rule, "BswMRuleTrueActionList", "BswMActionList")?;
        if !graph.within(list, control)
            || value(graph, list, "BswMActionListExecution", false) != Some("BSWM_CONDITION")
            || !matches!(
                value(graph, list, "BswMActionListPriority", false),
                None | Some("0")
            )
        {
            return Err(fail(graph, list, "MODE_RULE"));
        }
        let item = child(graph, list, "BswMActionListItem")?;
        if value(graph, item, "BswMActionListItemIndex", false) != Some("0") {
            return Err(fail(graph, item, "MODE_RULE"));
        }
        disabled(graph, item, "BswMAbortOnFail")?;
        let action = reference(graph, item, "BswMActionListItemRef", "BswMAction")?;
        if !graph.within(action, control) {
            return Err(fail(graph, action, "MODE_REFERENCE"));
        }
        let callout = child(graph, action, "BswMUserCallout")?;
        let text = value(graph, callout, "BswMUserCalloutFunction", false).unwrap_or("");
        let mode_name = [
            "COMM_NO_COMMUNICATION",
            "COMM_SILENT_COMMUNICATION",
            "COMM_FULL_COMMUNICATION",
        ][usize::from(mode)];
        let expected = format!("Ecu_HostBusSM_ConfirmMode({channel_handle}u, {mode_name})");
        let text = if text == format!("Ecu_HostBusSM_ApplyMode({channel_handle}u, {mode_name})") {
            legacy_callouts.push(graph.diagnostic(
                callout,
                DiagnosticCategory::Input,
                "MODE_LEGACY_CALLOUT",
                crate::product_message!("backend.integration.mode.legacy_callout"),
                crate::product_message!(
                    "backend.integration.mode.replace_callout",
                    "callout" => expected.clone()
                ),
            ));
            expected.as_str()
        } else {
            text
        };
        // ECUC_BswM_00843 carries a complete static C call. The symbolic CanIf
        // handle is resolved against the configured CanIfCtrlCanCtrlRef entries.
        let controller = communication
            .channels
            .iter()
            .find(|controller| {
                let symbol = super::symbolic_names::name(
                    "CanIf",
                    "CanIfCtrlCfg",
                    &controller.can_if_controller,
                );
                text == format!(
                    "Ecu_HostBusSM_ConfirmControllerMode({channel_handle}u, {symbol}, {mode_name})"
                )
            })
            .or_else(|| {
                (communication.channels.len() == 1 && text == expected)
                    .then(|| &communication.channels[0])
            })
            .ok_or_else(|| fail(graph, callout, "MODE_STATIC_CALLOUT"))?;
        if !config.can_if_controller.is_empty()
            && config.can_if_controller != controller.can_if_controller
        {
            return Err(fail(graph, callout, "MODE_REFERENCE"));
        }
        config
            .can_if_controller
            .clone_from(&controller.can_if_controller);
        if !modes.insert((*channel, mode))
            || !conditions.insert(condition)
            || !expressions.insert(expression)
            || !lists.insert(list)
            || !actions.insert(action)
        {
            return Err(fail(graph, rule, "MODE_RULE"));
        }
        rules.push(ModeRule {
            path: graph.elements[rule].object.clone(),
            channel: config.channel.clone(),
            mode,
            condition: graph.elements[condition].object.clone(),
            expression: graph.elements[expression].object.clone(),
            action_list: graph.elements[list].object.clone(),
            action: graph.elements[action].object.clone(),
            callout: text.into(),
        });
    }
    if modes.len() != channels.len() * 3
        || containers(graph, "BswMModeCondition").len() != conditions.len()
        || containers(graph, "BswMLogicalExpression").len() != expressions.len()
        || containers(graph, "BswMActionList").len() != lists.len()
        || containers(graph, "BswMAction").len() != actions.len()
    {
        return Err(fail(graph, config, "MODE_RULE"));
    }
    if !legacy_callouts.is_empty() {
        return Err(legacy_callouts);
    }
    let mut bound = BTreeSet::new();
    for (source, channel) in &channels {
        if !bound.insert(&channel.can_if_controller) {
            return Err(fail(graph, *source, "MODE_REFERENCE"));
        }
    }
    rules.sort_by(|left, right| (&left.channel, left.mode).cmp(&(&right.channel, right.mode)));
    let mut dcm_connections = BTreeMap::new();
    for connection in containers(graph, "DcmDslMainConnection") {
        let channel = reference(
            graph,
            connection,
            "DcmDslProtocolComMChannelRef",
            "ComMChannel",
        )?;
        let configured = channels
            .get(&channel)
            .ok_or_else(|| fail(graph, connection, "MODE_REFERENCE"))?;
        if let Some(transport) = transport {
            let controller = communication
                .channels
                .iter()
                .find(|entry| entry.physical_channel == transport.physical_channel)
                .ok_or_else(|| fail(graph, connection, "MODE_REFERENCE"))?;
            if configured.can_if_controller != controller.can_if_controller {
                return Err(fail(graph, connection, "MODE_REFERENCE"));
            }
        }
        dcm_connections.insert(
            graph.elements[connection].object.clone(),
            configured.channel.clone(),
        );
    }
    let mut channels = channels.into_values().collect::<Vec<_>>();
    channels.sort_by_key(|channel| channel.channel_handle);
    Ok(ModeRuntimeContract {
        channels,
        minimum_full_ms,
        bus_prefix: "Ecu_HostBusSM".into(),
        users,
        bswm_configuration: graph.elements[config].object.clone(),
        include: "Ecu_HostBusSM.h".into(),
        rules,
        dcm_connections,
    })
}
