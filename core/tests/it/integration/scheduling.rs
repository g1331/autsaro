//! Runnable scheduling, shared OS events and execution constraints.
use super::*;

#[test]
fn shared_os_event_requires_one_effective_period_and_preserves_rejected_sources() {
    let mut sources = conflicting_multi_schedule_sources();
    for reverse in [false, true] {
        if reverse {
            sources.reverse();
        }
        rejects_in_both(&sources, "SCHEDULE_NOT_UNIQUE");
        let issues = build(&sources)
            .err()
            .expect("conflicting schedule accepted");
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "SCHEDULE_NOT_UNIQUE"
                    && issue.message
                        == autosar_config_core::product_message!(
                            "backend.integration.schedule.shared_event_period_mismatch"
                        )
                    && issue.file.as_deref() == Some("ecuc.arxml")
                    && issue
                        .object
                        .as_deref()
                        .is_some_and(|object| object.contains("/Configuration/Rte/"))),
            "{issues:?}"
        );
    }
    assert_rejected_multi_sources_preserved(&sources);
}

#[test]
fn shared_os_event_accepts_distinct_equivalent_alarm_and_table_triggers() {
    for tables in [false, true] {
        let mut sources = equivalent_multi_schedule_triggers(tables);
        for reverse in [false, true] {
            if reverse {
                sources.reverse();
            }
            assert!(normal_validation(&sources).diagnostics.is_empty());
            let plan = build(&sources).unwrap();
            let app = plan
                .description()
                .schedule
                .entities
                .iter()
                .filter(|entity| entity.application)
                .collect::<Vec<_>>();
            assert!(
                app.iter().all(|entity| entity.period_ms == 10
                    && entity.os_event == "/Configuration/Os/Ev_App")
            );
            if tables {
                let phases = app
                    .iter()
                    .filter_map(|entity| entity.table_start.zip(entity.expiry_offset))
                    .collect::<std::collections::BTreeSet<_>>();
                assert_eq!(phases, std::collections::BTreeSet::from([(10, 0), (8, 2)]));
            } else {
                assert_eq!(
                    app.iter()
                        .map(|entity| entity.trigger())
                        .collect::<std::collections::BTreeSet<_>>()
                        .len(),
                    2
                );
            }
        }
    }
    let mut sources = equivalent_multi_schedule_triggers(true);
    parameter(
        &mut sources,
        "Table_Observe",
        "OsScheduleTblExpPointOffset",
        "1",
    );
    rejects_in_both(&sources, "PERIOD_SCHEDULE_TABLE_CONFLICT");
}

#[test]
fn network_freshness_and_runnable_execution_constraints_are_explicit() {
    for (before, after) in [
        (
            "<HANDLE-NEVER-RECEIVED>true</HANDLE-NEVER-RECEIVED>",
            "<HANDLE-NEVER-RECEIVED>false</HANDLE-NEVER-RECEIVED>",
        ),
        (
            "<HANDLE-TIMEOUT-TYPE>NONE</HANDLE-TIMEOUT-TYPE>",
            "<HANDLE-TIMEOUT-TYPE>REPLACE</HANDLE-TIMEOUT-TYPE>",
        ),
    ] {
        let mut sources = inputs();
        change(&mut sources, "ingress.arxml", before, after);
        rejects_in_both(&sources, "NETWORK_FRESHNESS_UNSUPPORTED");
    }
    let mut sources = inputs();
    change(
        &mut sources,
        "process.arxml",
        "<CAN-BE-INVOKED-CONCURRENTLY>false",
        "<MINIMUM-START-INTERVAL>0</MINIMUM-START-INTERVAL><CAN-BE-INVOKED-CONCURRENTLY>false",
    );
    assert!(build(&sources).is_ok());
    change(
        &mut sources,
        "process.arxml",
        "<MINIMUM-START-INTERVAL>0",
        "<MINIMUM-START-INTERVAL>0.001",
    );
    rejects_in_both(&sources, "MINIMUM_START_INTERVAL_UNSUPPORTED");
    let mut sources = inputs();
    change(
        &mut sources,
        "process.arxml",
        "<EVENTS>",
        "<EXCLUSIVE-AREAS><EXCLUSIVE-AREA><SHORT-NAME>Lock</SHORT-NAME></EXCLUSIVE-AREA></EXCLUSIVE-AREAS><EVENTS>",
    );
    change(
        &mut sources,
        "process.arxml",
        "<CAN-BE-INVOKED-CONCURRENTLY>false",
        "<CAN-ENTER-EXCLUSIVE-AREA-REFS><CAN-ENTER-EXCLUSIVE-AREA-REF DEST=\"EXCLUSIVE-AREA\">/Application/Process/Behavior/Lock</CAN-ENTER-EXCLUSIVE-AREA-REF></CAN-ENTER-EXCLUSIVE-AREA-REFS><CAN-BE-INVOKED-CONCURRENTLY>false",
    );
    let issues = build(&sources).err().unwrap();
    assert!(
        issues
            .iter()
            .any(|issue| issue.file.as_deref() == Some("process.arxml")),
        "{issues:?}"
    );
    // The bounded normal grammar rejects this official declaration before semantic planning.
    assert!(!normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn template_rejects_nonzero_execution_constraints_and_keeps_exact_zero() {
    for (index, sources) in template_execution_variants().into_iter().enumerate() {
        // The mainline schedules every timing event on one zero-phase period.
        let expected = if index == 0 {
            "PERIOD_UNSUPPORTED"
        } else {
            "MINIMUM_START_INTERVAL_UNSUPPORTED"
        };
        let issues = build(&sources)
            .err()
            .expect("template execution constraint accepted");
        assert!(
            issues.iter().any(|issue| issue.code == expected),
            "{issues:?}"
        );
        let mut zero = sources;
        replace_all(&mut zero, "<OFFSET>0.001</OFFSET>", "<OFFSET>0e1</OFFSET>");
        replace_all(
            &mut zero,
            "<MINIMUM-START-INTERVAL>0.001</MINIMUM-START-INTERVAL>",
            "<MINIMUM-START-INTERVAL>0.000</MINIMUM-START-INTERVAL>",
        );
        assert!(build(&zero).is_ok());
        assert!(normal_validation(&zero).diagnostics.is_empty());
    }
}
