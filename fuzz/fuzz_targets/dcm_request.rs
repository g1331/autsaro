#![no_main]
// Only `cargo fuzz` builds the C drivers (build.rs).
#[cfg(fuzzing)]
libfuzzer_sys::fuzz_target!(|data: &[u8]| autosar_config_core_fuzz::dcm_request(data));
