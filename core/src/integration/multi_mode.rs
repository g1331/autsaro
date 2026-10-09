//! Source-backed single CDD channel and immediate, static BswM user callouts.
use super::component::milliseconds;
use super::graph::Graph;
use super::schedule::{ScheduleContract, definition_is, value, values};
use super::{DiagnosticCategory, PlanDiagnostic};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeUser {
    pub path: String,
    pub handle: u16,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeRule {
    pub path: String,
    pub mode: u8,
    pub condition: String,
    pub expression: String,
    pub action_list: String,
    pub action: String,
    pub callout: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeRuntimeContract {
    pub channel: String,
    pub channel_handle: u8,
    pub main_symbol: String,
    pub period_ms: u32,
    pub minimum_full_ms: u32,
    pub ecu_group_classification: u8,
    pub bus_prefix: String,
    pub users: Vec<ModeUser>,
    pub bswm_configuration: String,
    pub input: String,
    pub initial_mode: u8,
    pub include: String,
    pub rules: Vec<ModeRule>,
    pub dcm_connections: Vec<String>,
}
fn fail(graph: &Graph, index: usize, code: &str) -> Vec<PlanDiagnostic> {
    vec![graph.diagnostic(
        index,
        DiagnosticCategory::Input,
        code,
        crate::product_message!("backend.integration.multi.contract_invalid", "code" => code),
        crate::product_message!("backend.integration.multi.repair_contract"),
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
            found.first().copied().unwrap_or(0),
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
) -> Result<ModeRuntimeContract, Vec<PlanDiagnostic>> {
    let general = one(graph, "ComMGeneral")?;
    disabled(
        graph,
        general,
        "ComMDevErrorDetect ComMDynamicPncToChannelMappingSupport ComMModeLimitationEnabled ComMPncSupport ComMResetAfterForcingNoComm ComMSynchronousWakeUp ComMVersionInfoApi ComMWakeupInhibitionEnabled",
    )?;
    let ecu_group_classification = value(graph, general, "ComMEcuGroupClassification", false)
        .and_then(|text| text.parse::<u8>().ok())
        .ok_or_else(|| fail(graph, general, "MODE_HANDLE"))?;
    let minimum_full_ms = value(graph, general, "ComMTMinFullComModeDuration", false)
        .and_then(milliseconds)
        .filter(|duration| (1..=65000).contains(duration))
        .ok_or_else(|| fail(graph, general, "MODE_TIMEBASE"))?;
    let channel = one(graph, "ComMChannel")?;
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
    let configured_users = containers(graph, "ComMUser");
    let mappings = containers(graph, "ComMUserPerChannel");
    if configured_users.is_empty()
        || configured_users.len() > 32
        || mappings.len() != configured_users.len()
    {
        return Err(fail(graph, channel, "MODE_USER_MAPPING"));
    }
    let mut mapped = BTreeSet::new();
    for mapping in mappings {
        let user = reference(graph, mapping, "ComMUserChannel", "ComMUser")?;
        if !graph.within(mapping, channel) || !mapped.insert(user) {
            return Err(fail(graph, mapping, "MODE_USER_MAPPING"));
        }
    }
    let mut users = Vec::new();
    let mut handles = BTreeSet::new();
    for user in configured_users {
        let handle = value(graph, user, "ComMUserIdentifier", false)
            .and_then(|text| text.parse::<u16>().ok())
            .filter(|handle| *handle != u16::MAX)
            .ok_or_else(|| fail(graph, user, "MODE_HANDLE"))?;
        if !mapped.contains(&user) || !handles.insert(handle) {
            return Err(fail(graph, user, "MODE_USER_MAPPING"));
        }
        users.push(ModeUser {
            path: graph.elements[user].object.clone(),
            handle,
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
    let input = one(graph, "BswMModeRequestPort")?;
    if !graph.within(input, arbitration)
        || value(graph, input, "BswMRequestProcessing", false) != Some("BSWM_IMMEDIATE")
    {
        return Err(fail(graph, input, "MODE_PROCESSING"));
    }
    let indication = child(graph, input, "BswMComMIndication")?;
    if reference(graph, indication, "BswMComMChannelRef", "ComMChannel")? != channel {
        return Err(fail(graph, indication, "MODE_REFERENCE"));
    }
    let initial = child(graph, input, "BswMModeInitValue")?;
    if mode(value(graph, initial, "BswMBswModeInitValue", false)) != Some(0) {
        return Err(fail(graph, initial, "MODE_INITIAL_VALUE"));
    }
    let mut rules = Vec::new();
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
            || reference(graph, condition, "BswMConditionMode", "BswMModeRequestPort")? != input
        {
            return Err(fail(graph, condition, "MODE_RULE"));
        }
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
        if text != format!("Ecu_HostBusSM_ApplyMode({channel_handle}u, {mode_name})") {
            return Err(fail(graph, callout, "MODE_STATIC_CALLOUT"));
        }
        if !modes.insert(mode)
            || !conditions.insert(condition)
            || !expressions.insert(expression)
            || !lists.insert(list)
            || !actions.insert(action)
        {
            return Err(fail(graph, rule, "MODE_RULE"));
        }
        rules.push(ModeRule {
            path: graph.elements[rule].object.clone(),
            mode,
            condition: graph.elements[condition].object.clone(),
            expression: graph.elements[expression].object.clone(),
            action_list: graph.elements[list].object.clone(),
            action: graph.elements[action].object.clone(),
            callout: text.into(),
        });
    }
    if modes != BTreeSet::from([0, 1, 2])
        || containers(graph, "BswMModeCondition").len() != conditions.len()
        || containers(graph, "BswMLogicalExpression").len() != expressions.len()
        || containers(graph, "BswMActionList").len() != lists.len()
        || containers(graph, "BswMAction").len() != actions.len()
    {
        return Err(fail(graph, config, "MODE_RULE"));
    }
    rules.sort_by_key(|rule| rule.mode);
    let mut dcm_connections = Vec::new();
    for connection in containers(graph, "DcmDslMainConnection") {
        if reference(
            graph,
            connection,
            "DcmDslProtocolComMChannelRef",
            "ComMChannel",
        )? != channel
        {
            return Err(fail(graph, connection, "MODE_REFERENCE"));
        }
        dcm_connections.push(graph.elements[connection].object.clone());
    }
    dcm_connections.sort();
    if !schedule.entities.iter().any(|entity| {
        !entity.application
            && entity.symbol == format!("ComM_MainFunction_{name}")
            && entity.period_ms == period_ms
    }) {
        return Err(fail(graph, channel, "MODE_TIMEBASE"));
    }
    Ok(ModeRuntimeContract {
        channel: graph.elements[channel].object.clone(),
        channel_handle,
        main_symbol: format!("ComM_MainFunction_{name}"),
        period_ms,
        minimum_full_ms,
        ecu_group_classification,
        bus_prefix: "Ecu_HostBusSM".into(),
        users,
        bswm_configuration: graph.elements[config].object.clone(),
        input: graph.elements[input].object.clone(),
        initial_mode: 0,
        include: "Ecu_HostBusSM.h".into(),
        rules,
        dcm_connections,
    })
}
