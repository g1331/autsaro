//! Tests that need neither native tools nor official archives.
//!
//! Shared support modules serve every binary; each uses only part of them.
#![allow(dead_code)]

#[path = "../support/mod.rs"]
mod support;
use support::*;
use support::{mainline::*, os_variants::*, workspace::Scratch};

mod c_analysis;
mod definitions;
mod delivery;
mod integration;
mod messages;
mod rules;
mod workspace;
