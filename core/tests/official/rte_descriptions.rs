//! RTE artifacts use the fixed schema, including allocations and per-entry dependencies.
use super::*;
use std::path::Path;

#[cfg(feature = "official-oracles")]
#[test]
fn rte_bsw_description_validates_against_the_fixed_r24_schema() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let archive = autosar_config_core::schema::schema_archive(root);
    let plan = build(&inputs()).unwrap();
    let files = prepared_pipeline_files(&plan);
    let (path, bytes) = files
        .iter()
        .find(|(path, _)| path == "generated/descriptions/Host_Implementation.arxml")
        .unwrap();
    let xml = std::str::from_utf8(bytes).unwrap();
    let issues =
        autosar_config_core::schema::validate_files(&archive, &[(Path::new(path), xml)]).unwrap();
    assert!(issues.is_empty(), "{issues:?}");
}
