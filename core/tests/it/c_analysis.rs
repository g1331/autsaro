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
}
