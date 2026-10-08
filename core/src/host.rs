mod process;
mod profile;
mod protocol;
mod scenarios;

use crate::{execution::ProcessOwner, model::RunReport};
use std::path::Path;

pub fn run(
    first: &Path,
    binary_a: &Path,
    second: &Path,
    binary_b: &Path,
    owner: &ProcessOwner,
) -> Result<RunReport, crate::message::LocalizedText> {
    scenarios::run(first, binary_a, second, binary_b, owner)
}

pub fn run_diagnostic(
    dir: &Path,
    binary: &Path,
    owner: &ProcessOwner,
) -> Result<RunReport, crate::message::LocalizedText> {
    scenarios::run_diagnostic(dir, binary, owner)
}
