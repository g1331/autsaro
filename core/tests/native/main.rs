//! Tests that build and run C with the controlled target toolchain
//! (`--features native-tests`); no official archives are needed.
#![cfg(any(windows, target_os = "linux"))]
// Shared support modules serve every binary; each uses only part of them.
#![allow(dead_code)]

#[path = "../support/mod.rs"]
mod support;
use support::*;
use support::{mainline::*, os_variants::*, workspace::Scratch};

// Native consumers must use the delivery for the executing compiler's target.
fn prepared_pipeline_files(
    plan: &autosar_config_core::integration::ValidatedIntegrationPlan,
) -> Vec<(String, Vec<u8>)> {
    support::mainline::prepared_pipeline_files_for_target(plan, tooling::native_target())
}

mod c_analysis;
mod delivery;
mod host;
mod integration;
mod links;
mod runtime_contracts;
mod sanitizers;
