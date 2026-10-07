// Shared support modules serve different test layers; only registered tests run.
#![allow(dead_code)]

// Product-authored configuration tests run without official archives or a C compiler.
use workspace::Scratch;
#[cfg(feature = "official-oracles")]
use workspace::archive;

#[path = "support/epic7_definitions.rs"]
mod epic7_definitions;
#[path = "support/epic7_delivery.rs"]
mod epic7_delivery;
#[path = "support/epic7_rules.rs"]
mod epic7_rules;
#[path = "support/epic7_workspace.rs"]
mod epic7_workspace;
#[path = "support/tooling.rs"]
mod tooling;
#[path = "support/workspace.rs"]
mod workspace;
