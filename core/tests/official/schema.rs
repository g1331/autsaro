//! Mainline inputs accepted by the official R24-11 XSD.
use super::*;
use autosar_config_core::integration::InputSource;
use std::path::Path;

/// The independent R24-11 schema oracle accepts inputs before the native plan judges them.
#[cfg(feature = "official-oracles")]
fn assert_official_schema_accepts(sources: &[InputSource]) {
    let files: Vec<_> = sources
        .iter()
        .map(|source| {
            (
                std::path::PathBuf::from(source.logical_path()),
                String::from_utf8(source.bytes().to_vec()).unwrap(),
            )
        })
        .collect();
    let borrowed: Vec<_> = files
        .iter()
        .map(|(path, text)| (path.as_path(), text.as_str()))
        .collect();
    let issues =
        autosar_config_core::schema::validate_files(&workspace::archive(), &borrowed).unwrap();
    assert!(issues.is_empty(), "official R24-11 XSD: {issues:?}");
}

#[cfg(feature = "official-oracles")]
#[test]
fn native_pipeline_inputs_also_pass_the_official_schema() {
    let sources = inputs();
    assert_official_schema_accepts(&sources);
    build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
}

#[cfg(feature = "official-oracles")]
#[test]
fn execution_constraint_rejections_use_officially_valid_arxml() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for exclusive_area in [false, true] {
        let mut sources = inputs();
        if exclusive_area {
            change(
                &mut sources,
                "process.arxml",
                "<EVENTS>",
                "<EXCLUSIVE-AREAS><EXCLUSIVE-AREA><SHORT-NAME>Lock</SHORT-NAME></EXCLUSIVE-AREA></EXCLUSIVE-AREAS><EVENTS>",
            );
            change(
                &mut sources,
                "process.arxml",
                "<CAN-BE-INVOKED-CONCURRENTLY>false",
                "<CAN-ENTER-EXCLUSIVE-AREA-REFS><CAN-ENTER-EXCLUSIVE-AREA-REF DEST=\"EXCLUSIVE-AREA\">/Application/Process/Behavior/Lock</CAN-ENTER-EXCLUSIVE-AREA-REF></CAN-ENTER-EXCLUSIVE-AREA-REFS><CAN-BE-INVOKED-CONCURRENTLY>false",
            );
        } else {
            change(
                &mut sources,
                "process.arxml",
                "<CAN-BE-INVOKED-CONCURRENTLY>false",
                "<MINIMUM-START-INTERVAL>0.001</MINIMUM-START-INTERVAL><CAN-BE-INVOKED-CONCURRENTLY>false",
            );
        }
        let files = sources
            .iter()
            .map(|source| {
                (
                    Path::new(source.logical_path()),
                    std::str::from_utf8(source.bytes()).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let issues = autosar_config_core::schema::validate_files(
            &autosar_config_core::schema::schema_archive(root),
            &files,
        )
        .unwrap();
        assert!(issues.is_empty(), "{issues:?}");
        assert!(build(&sources).is_err());
        assert!(!normal_validation(&sources).diagnostics.is_empty());
    }
}

#[cfg(feature = "official-oracles")]
#[test]
fn zero_offset_and_multiple_operations_are_accepted_by_official_schema_and_native_inputs() {
    let mut sources = pipeline_operation_inputs();
    change(
        &mut sources,
        "process.arxml",
        "<PERIOD>0.01</PERIOD>",
        "<OFFSET>0</OFFSET><PERIOD>0.01</PERIOD>",
    );
    assert_official_schema_accepts(&sources);
    assert!(normal_validation(&sources).diagnostics.is_empty());
    build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    change(
        &mut sources,
        "process.arxml",
        "<OFFSET>0</OFFSET>",
        "<OFFSET>0.001</OFFSET>",
    );
    assert_official_schema_accepts(&sources);
    let errors = build(&sources).err().unwrap();
    assert!(
        errors
            .iter()
            .any(|issue| issue.code == "PERIOD_UNSUPPORTED"),
        "{errors:?}"
    );
}

#[cfg(feature = "official-oracles")]
#[test]
fn template_official_legal_inputs_reject_nonzero_execution_constraints() {
    for (index, sources) in template_execution_variants().into_iter().enumerate() {
        let expected = if index == 0 {
            "PERIOD_UNSUPPORTED"
        } else {
            "MINIMUM_START_INTERVAL_UNSUPPORTED"
        };
        assert_official_schema_accepts(&sources);
        let issues = build(&sources).err().expect("template constraint accepted");
        assert!(
            issues.iter().any(|issue| issue.code == expected),
            "{issues:?}"
        );
    }
}
