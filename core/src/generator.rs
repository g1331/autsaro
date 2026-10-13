// Generation and integration report PlanDiagnostic instead of panicking and
// render each output once; core/clippy.toml lists the disallowed text patching.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_methods,
    clippy::disallowed_macros
)]

pub mod delivery;
pub(crate) mod output;

pub use output::{StagedBuild, StagedGeneration};
