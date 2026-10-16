//! Comparisons with the licensed official R24-11 archives
//! (`--features official-oracles`).
// Shared support modules serve every binary; each uses only part of them.
#![allow(dead_code)]

#[path = "../support/mod.rs"]
mod support;
use support::*;
use support::{
    mainline::*,
    os_variants::*,
    workspace::{Scratch, archive},
};

mod definitions;
mod os_descriptions;
mod reference_inputs;
mod rte_descriptions;
mod rules;
mod schema;
