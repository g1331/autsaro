#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| autosar_config_core_fuzz::project_import(data));
