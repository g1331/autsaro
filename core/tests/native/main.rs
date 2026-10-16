//! Tests that build and run C with the controlled target toolchain
//! (`--features native-tests`); no official archives are needed.
#![cfg(any(windows, target_os = "linux"))]
// Shared support modules serve every binary; each uses only part of them.
#![allow(dead_code)]

#[path = "../support/mod.rs"]
mod support;
use support::*;
use support::{mainline::*, os_variants::*, workspace::Scratch};

mod c_analysis;
mod delivery;
mod host;
mod integration;
mod links;
mod runtime_contracts;
mod sanitizers;
