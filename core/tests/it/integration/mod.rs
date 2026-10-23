//! Planning, generation and editing of the mainline integration.
use super::*;

mod communication;
mod contracts;
mod descriptions;
mod det;
mod diagnostic_configuration;
mod diagnostics;
mod handoff;
mod linked_edits;
mod modes;
mod os;
mod os_configuration;
mod rte;
mod rte_vendor;
mod scheduling;
mod single_component;
mod symbolic_names;
mod validation;
mod workbench;
mod workspace;

#[test]
fn input_sha256_preserves_lowercase_hex_and_leading_zero_bytes() {
    use autosar_config_core::integration::InputSource;

    for (bytes, expected) in [
        (
            "",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            "abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        (
            "286",
            "00328ce57bbc14b33bd6695bc8eb32cdf2fb5f3a7d89ec14a42825e15d39df60",
        ),
    ] {
        let source = InputSource::new("input.arxml", bytes.as_bytes().to_vec()).unwrap();
        assert_eq!(source.sha256(), expected);
    }
}
