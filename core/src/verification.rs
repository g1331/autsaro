//! Native acceptance instrumentation. Compiled only for the verification build.
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static SOURCE_READS: AtomicU64 = AtomicU64::new(0);
static SNAPSHOT_BUILDS: AtomicU64 = AtomicU64::new(0);
static RULE_LOADS: AtomicU64 = AtomicU64::new(0);
static GRAMMAR_LOADS: AtomicU64 = AtomicU64::new(0);
static DEFINITION_READS: AtomicU64 = AtomicU64::new(0);
static SCHEMA_CHECKS: AtomicU64 = AtomicU64::new(0);
static SOURCE_SCANS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
pub enum Phase {
    SourceRead,
    SnapshotBuild,
    DefinitionRead,
    SchemaCheck,
    SourceScan,
}

pub fn before(phase: Phase) {
    let (counter, variable) = match phase {
        Phase::SourceRead => (&SOURCE_READS, "AUTOSAR_VERIFY_DELAY_SOURCE_READ_MS"),
        Phase::SnapshotBuild => (&SNAPSHOT_BUILDS, "AUTOSAR_VERIFY_DELAY_SNAPSHOT_BUILD_MS"),
        Phase::DefinitionRead => (&DEFINITION_READS, "AUTOSAR_VERIFY_DELAY_DEFINITION_READ_MS"),
        Phase::SchemaCheck => (&SCHEMA_CHECKS, "AUTOSAR_VERIFY_DELAY_SCHEMA_CHECK_MS"),
        Phase::SourceScan => (&SOURCE_SCANS, "AUTOSAR_VERIFY_DELAY_SOURCE_SCAN_MS"),
    };
    counter.fetch_add(1, Ordering::Relaxed);
    if let Some(milliseconds) = std::env::var(variable)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value <= 5_000)
    {
        std::thread::sleep(Duration::from_millis(milliseconds));
    }
}

pub fn record_rule_load() {
    RULE_LOADS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_grammar_load() {
    GRAMMAR_LOADS.fetch_add(1, Ordering::Relaxed);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub source_reads: String,
    pub snapshot_builds: String,
    pub rule_loads: String,
    pub grammar_loads: String,
    pub definition_reads: String,
    pub schema_checks: String,
    pub source_scans: String,
}

pub fn metrics() -> Metrics {
    Metrics {
        source_reads: SOURCE_READS.load(Ordering::Relaxed).to_string(),
        snapshot_builds: SNAPSHOT_BUILDS.load(Ordering::Relaxed).to_string(),
        rule_loads: RULE_LOADS.load(Ordering::Relaxed).to_string(),
        grammar_loads: GRAMMAR_LOADS.load(Ordering::Relaxed).to_string(),
        definition_reads: DEFINITION_READS.load(Ordering::Relaxed).to_string(),
        schema_checks: SCHEMA_CHECKS.load(Ordering::Relaxed).to_string(),
        source_scans: SOURCE_SCANS.load(Ordering::Relaxed).to_string(),
    }
}
