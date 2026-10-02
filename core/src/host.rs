mod process;
mod profile;
mod protocol;
mod scenarios;

use crate::model::RunReport;
use std::path::Path;

pub fn run(
    first: &Path,
    binary_a: &Path,
    second: &Path,
    binary_b: &Path,
) -> Result<RunReport, String> {
    scenarios::run(first, binary_a, second, binary_b)
}

pub fn run_diagnostic(dir: &Path, binary: &Path) -> Result<RunReport, String> {
    scenarios::run_diagnostic(dir, binary)
}
