//! OS configuration variants of the mainline fixture.
use super::mainline::*;
use autosar_config_core::integration::InputSource;
use autosar_config_core::target::BuildTarget;
use std::path::{Path, PathBuf};

pub(crate) fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

pub(crate) fn replaced(sources: &[InputSource], before: &str, after: &str) -> Vec<InputSource> {
    let mut changed = false;
    let result = sources
        .iter()
        .map(|source| {
            let text = std::str::from_utf8(source.bytes()).unwrap();
            changed |= text.contains(before);
            InputSource::new(
                source.logical_path(),
                text.replace(before, after).into_bytes(),
            )
            .unwrap()
        })
        .collect();
    assert!(changed, "{before} is absent from the real input");
    result
}

/// Prepared files of `sources` with the reference echo application (the same
/// application as `single_component_project`), written to `root/project`.
pub(crate) fn project(
    sources: &[InputSource],
    root: &Path,
    target: BuildTarget,
) -> (PathBuf, Vec<(String, Vec<u8>)>) {
    let directory = root.join("project");
    let files = prepared(sources, root, target).into_files();
    write_files(&directory, &files);
    (directory, files)
}

/// The preparation `project` writes; its application source is `root/Ingress.c`.
pub(crate) fn prepared(
    sources: &[InputSource],
    root: &Path,
    target: BuildTarget,
) -> autosar_config_core::prepared::PreparedProject<'static> {
    use autosar_config_core::{ApplicationSource, prepare_ecu_project_with_applications};
    let plan = build(sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    let fixture_text =
        std::fs::read_to_string(fixture("component-pipeline/applications/Ingress.c")).unwrap();
    let application = root.join("Ingress.c");
    std::fs::write(
        &application,
        fixture_text.replace("        (void)Rte_Write_Value_Value(value);\n", ""),
    )
    .unwrap();
    prepare_ecu_project_with_applications(
        &plan,
        target,
        &[ApplicationSource {
            component_instance: "/Application/Pipeline/IngressInstance".into(),
            path: application,
        }],
    )
    .unwrap_or_else(|issues| panic!("{issues:?}"))
}

pub(crate) fn file<'a>(files: &'a [(String, Vec<u8>)], name: &str) -> &'a str {
    std::str::from_utf8(
        &files
            .iter()
            .find(|(path, _)| path == name)
            .unwrap_or_else(|| panic!("{name} is not delivered"))
            .1,
    )
    .unwrap()
}

pub(crate) fn counter_variant(renamed: bool, maximum: u32) -> Vec<InputSource> {
    let mut sources = if renamed {
        replaced(
            &single_component_sources(),
            "SystemCounter",
            "RenamedCounter",
        )
    } else {
        single_component_sources()
    };
    if maximum != 65535 {
        parameter(
            &mut sources,
            if renamed {
                "RenamedCounter"
            } else {
                "SystemCounter"
            },
            "OsCounterMaxAllowedValue",
            &maximum.to_string(),
        );
    }
    sources
}

pub(crate) const RES_SCHEDULER: &str = "<ECUC-CONTAINER-VALUE><SHORT-NAME>RES_SCHEDULER</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">/AUTOSAR/EcucDefs/Os/OsResource</DEFINITION-REF><PARAMETER-VALUES><ECUC-TEXTUAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-ENUMERATION-PARAM-DEF\">/AUTOSAR/EcucDefs/Os/OsResource/OsResourceProperty</DEFINITION-REF><VALUE>STANDARD</VALUE></ECUC-TEXTUAL-PARAM-VALUE></PARAMETER-VALUES></ECUC-CONTAINER-VALUE>";

/// OsUseResScheduler (ECUC_Os_00040) with an optional explicit RES_SCHEDULER
/// OsResource in the selected OS module.
pub(crate) fn scheduler_variant(value: &str, resource: bool) -> Vec<InputSource> {
    let mut sources = single_component_sources();
    parameter(&mut sources, "OperatingSystem", "OsUseResScheduler", value);
    if resource {
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| named(node, "ECUC-MODULE-CONFIGURATION-VALUES", "Os"),
            |text| {
                let at = text.rfind("</CONTAINERS>").unwrap();
                format!("{}{RES_SCHEDULER}{}", &text[..at], &text[at..])
            },
        );
    }
    sources
}

/// Every OS trigger of the single-application project moved from alarms to
/// repeating schedule tables (expiry point offset 0, sync strategy NONE), as
/// in `os-timing/schedule_tables.arxml`.
pub(crate) fn schedule_table_variant() -> Vec<InputSource> {
    let tables = [InputSource::new(
        "tables.arxml",
        std::fs::read(fixture("os-timing/schedule_tables.arxml")).unwrap(),
    )
    .unwrap()];
    let mut sources = single_component_sources();
    for (alarm, table) in [("Alarm_Work", "Table_Work"), ("Alarm_App", "Table_App")] {
        let replacement = subtree(&tables, "tables.arxml", "ECUC-CONTAINER-VALUE", table);
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| named(node, "ECUC-CONTAINER-VALUE", alarm),
            |_| replacement.clone(),
        );
        sources = replaced(
            &sources,
            &format!("/Configuration/Os/{alarm}</VALUE-REF>"),
            &format!("/Configuration/Os/{table}/Point</VALUE-REF>"),
        );
    }
    sources = replaced(
        &sources,
        "/RteUsedOsAlarmRef<",
        "/RteUsedOsSchTblExpiryPointRef<",
    );
    replaced(
        &sources,
        "/RteBswUsedOsAlarmRef<",
        "/RteBswUsedOsSchTblExpiryPointRef<",
    )
}

/// An OS module renamed to `OsAlt` with OsUseResScheduler TRUE.
#[cfg(any(
    feature = "official-oracles",
    all(feature = "native-tests", any(windows, target_os = "linux"))
))]
pub(crate) fn renamed_os_variant() -> Vec<InputSource> {
    let mut sources = scheduler_variant("true", false);
    sources = replaced(
        &sources,
        "<SHORT-NAME>Os</SHORT-NAME>",
        "<SHORT-NAME>OsAlt</SHORT-NAME>",
    );
    replaced(&sources, "/Configuration/Os/", "/Configuration/OsAlt/")
}
