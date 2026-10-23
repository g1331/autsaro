//! Generation of the static-analysis samples (no compiler or official archives).
use super::*;

/// Writes the samples to `AUTOSAR_C_ANALYSIS_SAMPLES` when set, for CI and
/// `c-check`; otherwise to a scratch directory that is removed afterwards.
#[test]
fn generated_c_analysis_samples() {
    let scratch = Scratch::new();
    let output = std::env::var_os("AUTOSAR_C_ANALYSIS_SAMPLES")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| scratch.0.join("samples"));
    assert!(
        !output.exists(),
        "Sample output must be new; refusing to overwrite {}",
        output.display()
    );
    analysis_samples::generate(&output, &scratch);
    for (sample, id, length) in [
        ("standard-ecu", "UINT16", "UINT16"),
        ("user-application", "UINT16", "UINT16"),
        ("component-pipeline", "UINT8", "UINT32"),
        ("no-diagnostic", "UINT8", "UINT8"),
    ] {
        let metadata: serde_json::Value = serde_json::from_slice(
            &std::fs::read(output.join(sample).join("meta/integration.json")).unwrap(),
        )
        .unwrap();
        let target: serde_json::Value = serde_json::from_slice(
            &std::fs::read(output.join(sample).join("target.json")).unwrap(),
        )
        .unwrap();
        let flags = target["compilerFlags"].as_array().unwrap();
        assert!(
            flags.iter().any(|flag| flag == "-fexceptions"),
            "{sample}: native unwind contract"
        );
        assert!(
            !flags
                .iter()
                .any(|flag| flag.as_str().unwrap().starts_with("-D_GNU_SOURCE")),
            "{sample}: reserved feature selection"
        );
        let communication = &metadata["plan"]["communicationRuntime"];
        assert_eq!(communication["pduIdType"], id, "{sample}: PDU ID selection");
        assert_eq!(
            communication["pduLengthType"], length,
            "{sample}: PDU length selection"
        );
    }
}
