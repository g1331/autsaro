use super::{DiagnosticCategory, PlanDescription, PlanDiagnostic, ValidatedIntegrationPlan};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IntegrationEdit {
    /// Selected S/R port identities, never short names or inferred routes.
    #[serde(default)]
    pub can_ids: BTreeMap<String, u32>,
    pub application_period_ms: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationInspection {
    pub profile: String,
    pub description: Option<PlanDescription>,
    pub diagnostics: Vec<PlanDiagnostic>,
}

pub(crate) struct FieldEdit {
    pub object: String,
    pub field: String,
    pub parameter: bool,
    pub value: String,
}

pub(crate) fn issue(
    plan: &ValidatedIntegrationPlan,
    code: &str,
    object: &str,
    message: crate::message::LocalizedText,
) -> Vec<PlanDiagnostic> {
    vec![PlanDiagnostic {
        category: DiagnosticCategory::Input,
        code: code.into(),
        object: Some(object.into()),
        file: plan
            .description()
            .objects
            .iter()
            .find(|item| item.path == object)
            .map(|item| item.file.clone()),
        message,
        remedy: crate::product_message!("backend.integration.editor.plan_repair_preview_required"),
    }]
}

pub(crate) fn fields(
    plan: &ValidatedIntegrationPlan,
    changes: &IntegrationEdit,
) -> Result<Vec<FieldEdit>, Vec<PlanDiagnostic>> {
    fields_including_unchanged(plan, changes, false)
}

pub(crate) fn fields_including_unchanged(
    plan: &ValidatedIntegrationPlan,
    changes: &IntegrationEdit,
    include_unchanged: bool,
) -> Result<Vec<FieldEdit>, Vec<PlanDiagnostic>> {
    let description = plan.description();
    let mut edits: BTreeMap<(String, String, bool), String> = BTreeMap::new();
    for (port, id) in &changes.can_ids {
        let channel = description
            .signals
            .iter()
            .find(|channel| &channel.port == port)
            .ok_or_else(|| {
                issue(
                    plan,
                    "EDIT_UNSUPPORTED",
                    port,
                    crate::product_message!(
                        "backend.integration.editor.only_selected_sr_can_ids_editable"
                    ),
                )
            })?;
        if *id > 0x7ff {
            return Err(issue(
                plan,
                "EDIT_RANGE",
                port,
                crate::product_message!(
                    "backend.integration.editor.standard_classical_can_id_out_of_range"
                ),
            ));
        }
        if !include_unchanged && *id == channel.can_id {
            continue;
        }
        edits.insert(
            (channel.trigger.clone(), "IDENTIFIER".into(), false),
            id.to_string(),
        );
        let parameter = if channel.receive {
            "CanIfRxPduCanId"
        } else {
            "CanIfTxPduCanId"
        };
        edits.insert(
            (channel.can_if_pdu.clone(), parameter.into(), true),
            id.to_string(),
        );
    }
    if let Some(period) = changes.application_period_ms {
        // Every application timing event shares one OS trigger and period on
        // both the mainline and the historical chain.
        let application = description
            .schedule
            .entities
            .iter()
            .find(|entity| entity.application)
            .ok_or_else(|| {
                issue(
                    plan,
                    "EDIT_UNSUPPORTED",
                    "/",
                    crate::product_message!(
                        "backend.integration.editor.application_period_invalid"
                    ),
                )
            })?;
        if period == 0 || period > description.schedule.counter_maximum {
            return Err(issue(
                plan,
                "EDIT_RANGE",
                &application.event,
                crate::product_message!("backend.integration.editor.application_period_invalid"),
            ));
        }
        if include_unchanged || period != application.period_ms {
            let seconds = format!("{}.{:03}", period / 1000, period % 1000);
            let shared: Vec<_> = description
                .schedule
                .entities
                .iter()
                .filter(|entity| entity.trigger() == application.trigger())
                .collect();
            for entity in &shared {
                edits.insert(
                    (entity.event.clone(), "PERIOD".into(), false),
                    seconds.clone(),
                );
            }
            // Transmit PDU periods follow the trigger that sends them. The
            // mainline sends from the Com Tx main function, so it only moves with the
            // application when both share this trigger. Any other BSW main
            // function on the trigger keeps its own timing and refuses.
            let transmit_main = description
                .com_runtime
                .as_ref()
                .map(|com| &com.transmit_main)
                .filter(|main| shared.iter().any(|entity| entity.symbol == main.symbol));
            let transmit_follows = transmit_main.is_some();
            let modes: Vec<_> = if transmit_follows {
                description
                    .signals
                    .iter()
                    .filter(|channel| !channel.receive)
                    .flat_map(|tx| {
                        description.configuration.iter().filter(move |record| {
                            record.path.starts_with(&format!("{}/", tx.com_pdu))
                                && record.definition.ends_with("/ComTxMode")
                        })
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let autostart: Vec<_> = description
                .configuration
                .iter()
                .filter(|record| {
                    record.path.starts_with(&format!(
                        "{}/",
                        application
                            .schedule_table
                            .as_deref()
                            .unwrap_or(&application.alarm)
                    )) && record
                        .definition
                        .ends_with(if application.schedule_table.is_some() {
                            "/OsScheduleTableAutostart"
                        } else {
                            "/OsAlarmAutostart"
                        })
                })
                .collect();
            let foreign = shared.iter().any(|entity| {
                !entity.application && transmit_main.is_none_or(|main| entity.symbol != main.symbol)
            });
            let transmitters = description
                .signals
                .iter()
                .filter(|channel| !channel.receive)
                .count();
            if foreign
                || (transmit_follows && (transmitters == 0 || modes.len() != transmitters))
                || autostart.len() != 1
            {
                return Err(issue(
                    plan,
                    "EDIT_UNSAFE",
                    application.trigger(),
                    crate::product_message!(
                        "backend.integration.editor.period_com_mode_timing_autostart_required"
                    ),
                ));
            }
            for mode in modes {
                edits.insert(
                    (mode.path.clone(), "ComTxModeTimePeriod".into(), true),
                    seconds.clone(),
                );
            }
            if let Some(main) = transmit_main {
                edits.insert(
                    (main.path.clone(), "ComMainTxTimeBase".into(), true),
                    seconds.clone(),
                );
            }
            if let Some(table) = &application.schedule_table {
                let offset = application
                    .expiry_offset
                    .ok_or_else(|| PlanDiagnostic::plan_premise(table))?;
                if period <= offset {
                    return Err(issue(
                        plan,
                        "EDIT_RANGE",
                        table,
                        crate::product_message!(
                            "backend.integration.editor.table_initial_expiry_offset_exceeds_period"
                        ),
                    ));
                }
                edits.insert(
                    (table.clone(), "OsScheduleTableDuration".into(), true),
                    period.to_string(),
                );
                edits.insert(
                    (
                        autostart[0].path.clone(),
                        "OsScheduleTableStartValue".into(),
                        true,
                    ),
                    (period - offset).to_string(),
                );
            } else {
                for parameter in ["OsAlarmAlarmTime", "OsAlarmCycleTime"] {
                    edits.insert(
                        (autostart[0].path.clone(), parameter.into(), true),
                        period.to_string(),
                    );
                }
            }
        }
    }
    Ok(edits
        .into_iter()
        .map(|((object, field, parameter), value)| FieldEdit {
            object,
            field,
            parameter,
            value,
        })
        .collect())
}
