//! Synthetic code-user input vectors; no number here claims an allocated identity.
use super::single_component::{rejected_at, rejected_with};
use super::*;
use std::collections::BTreeMap;

fn content<'a>(files: &'a [(String, Vec<u8>)], path: &str) -> &'a [u8] {
    &files.iter().find(|(name, _)| name == path).unwrap().1
}

fn code(files: &[(String, Vec<u8>)]) -> BTreeMap<&str, &[u8]> {
    files
        .iter()
        .filter(|(path, _)| path.ends_with(".c") || path.ends_with(".h"))
        .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
        .collect()
}

#[test]
fn code_user_vendor_input_reaches_only_rte_implementation_and_contract() {
    // Verifies: SWS_Rte_05189, ECUC_Rte_09086
    let control = prepared_pipeline_files(&build(&inputs()).unwrap());
    for (source, expected) in [
        ("0", 0),
        ("1", 1),
        ("42017", 42017),
        ("65535", 65535),
        ("0xA421", 42017),
        (" +42017 ", 42017),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, "Generation", "RteCodeVendorId", source);
        let source_revision = source == "42017";
        if source_revision {
            xml_edit(
                &mut sources,
                "ecuc.arxml",
                |node| named(node, "ECUC-MODULE-CONFIGURATION-VALUES", "Rte"),
                |text| {
                    text.replacen("<DEFINITION-REF", "<ADMIN-DATA><DOC-REVISIONS><DOC-REVISION><REVISION-LABEL>1.2.3</REVISION-LABEL><DATE>2026-10-20</DATE></DOC-REVISION><DOC-REVISION><REVISION-LABEL>3.12.45_test</REVISION-LABEL><DATE>2026-10-21</DATE></DOC-REVISION></DOC-REVISIONS></ADMIN-DATA><DEFINITION-REF", 1)
                },
            );
        }
        assert!(normal_validation(&sources).diagnostics.is_empty());
        let plan = build(&sources).unwrap();
        assert_eq!(
            plan.description()
                .composition
                .as_ref()
                .unwrap()
                .generation
                .code_vendor_id,
            expected
        );
        let files = prepared_pipeline_files(&plan);
        let xml = std::str::from_utf8(content(
            &files,
            "generated/descriptions/Host_Implementation.arxml",
        ))
        .unwrap();
        let doc = roxmltree::Document::parse(xml).unwrap();
        let mut implementations = BTreeMap::new();
        for node in doc
            .descendants()
            .filter(|node| node.has_tag_name("BSW-IMPLEMENTATION"))
        {
            let field = |tag| {
                node.children()
                    .find(|child| child.has_tag_name(tag))
                    .unwrap()
                    .text()
                    .unwrap()
            };
            let name = field("SHORT-NAME");
            assert!(implementations.insert(name, field("VENDOR-ID")).is_none());
            assert_eq!(field("AR-RELEASE-VERSION"), "4.10.0");
            assert_eq!(
                field("SW-VERSION"),
                match name {
                    "RteHostImplementation" if source_revision => "3.12.45",
                    "RteHostImplementation" => "0.0.0",
                    "ArtiHostImplementation" => "1.0.0",
                    _ => "0.1.0",
                }
            );
            if name == "RteHostImplementation" {
                assert_eq!(field("BEHAVIOR-REF"), "/AUTOSAR_Rte/Rte/HostBehavior");
            }
        }
        assert_eq!(
            implementations.remove("RteHostImplementation").unwrap(),
            expected.to_string()
        );
        assert_eq!(implementations.len(), 13);
        assert!(implementations.values().all(|value| *value == "0"));
        // Changing the code user changes neither C/headers nor supplier ABI.
        assert_eq!(code(&files), code(&control));
        let kit = plan.component_contract_files().unwrap();
        let contract: serde_json::Value =
            serde_json::from_slice(content(kit.files(), "contract.json")).unwrap();
        assert_eq!(
            contract["composition"]["generation"]["codeVendorId"],
            expected
        );
        assert_eq!(
            contract["composition"]["generation"]["softwareVersion"],
            if source_revision { "3.12.45" } else { "0.0.0" }
        );
        if source_revision {
            assert_eq!(
                contract["composition"]["generation"]["revisionSuffix"],
                "test"
            );
            assert!(xml.contains("Configuration revision suffix: test"));
        }
        assert!(
            !kit.files()
                .iter()
                .any(|(path, _)| path.ends_with("Host_Implementation.arxml"))
        );
        assert_eq!(
            content(kit.files(), "include/Rte_Version.h"),
            content(&files, "bsw/include/Rte_Version.h")
        );
        sources.reverse();
        assert_eq!(files, prepared_pipeline_files(&build(&sources).unwrap()));
    }
}

fn vendor_parameter(node: roxmltree::Node<'_, '_>) -> bool {
    node.has_tag_name("ECUC-NUMERICAL-PARAM-VALUE")
        && node.children().any(|child| {
            child.has_tag_name("DEFINITION-REF")
                && child.text() == Some("/AUTOSAR/EcucDefs/Rte/RteGeneration/RteCodeVendorId")
        })
}

#[test]
fn code_user_vendor_refusals_locate_and_preserve_the_actual_source() {
    // Verifies: ECUC_Rte_09086
    for value in [
        "-1",
        "65536",
        "0x10000",
        "1.5",
        "unknown",
        "",
        "999999999999999999999999999999999999999999",
    ] {
        let mut sources = inputs();
        parameter(&mut sources, "Generation", "RteCodeVendorId", value);
        rejected_with(
            &sources,
            (
                if value == "unknown" || value.is_empty() {
                    "NATIVE_UNSUPPORTED"
                } else {
                    "VALUE_RANGE"
                },
                "VALUE_RANGE",
            ),
            "ecuc.arxml",
            "/Configuration/Rte/Generation",
        );
        assert_rejected_sources_preserved(&sources);
    }
    for (replacement, error) in [(0, "MULTIPLICITY_LOWER"), (2, "MULTIPLICITY")] {
        let mut sources = inputs();
        xml_edit(&mut sources, "ecuc.arxml", vendor_parameter, |text| {
            text.repeat(replacement)
        });
        rejected_at(
            &sources,
            error,
            "ecuc.arxml",
            "/Configuration/Rte/Generation",
        );
        assert_rejected_sources_preserved(&sources);
    }
    // A numerically valid value on another object is not a RTE generation input.
    let mut misplaced = inputs();
    let source = misplaced
        .iter()
        .find(|source| source.logical_path() == "ecuc.arxml")
        .unwrap();
    let doc = roxmltree::Document::parse(std::str::from_utf8(source.bytes()).unwrap()).unwrap();
    let parameter_xml = doc
        .descendants()
        .find(|node| vendor_parameter(*node))
        .unwrap();
    let parameter_xml = doc.input_text()[parameter_xml.range()].to_owned();
    xml_edit(
        &mut misplaced,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "BswGeneral"),
        |text| {
            text.replace(
                "<PARAMETER-VALUES>",
                &format!("<PARAMETER-VALUES>{parameter_xml}"),
            )
        },
    );
    rejected_at(
        &misplaced,
        "DEFINITION_PARENT",
        "ecuc.arxml",
        "/Configuration/Rte/BswGeneral",
    );
    assert_rejected_sources_preserved(&misplaced);
}

#[test]
fn code_user_vendor_survives_save_reopen_and_contract_seal_refusal() {
    use autosar_config_core::Workspace;
    for value in ["1", "42017", "65535"] {
        let scratch = Scratch::new();
        let root = scratch.0.join("live");
        let mut sources = inputs();
        parameter(&mut sources, "Generation", "RteCodeVendorId", value);
        let mut workspace = live_pipeline_workspace(&root, &sources);
        let initialization = workspace.preview_application_initialization().unwrap();
        workspace
            .initialize_application_previewed(&initialization)
            .unwrap();
        let save = workspace.preview_save().unwrap();
        workspace.save_previewed(&save.revision).unwrap();
        let reopened = Workspace::open_project_manifest(
            &root.join("workbench-project.json"),
            &scratch.0.join("reopen-cache"),
        )
        .unwrap();
        let plan = reopened.saved_integration_plan().unwrap();
        let prepare = || {
            autosar_config_core::prepared::prepare_ecu_project_for_workspace(
                &reopened,
                &plan,
                autosar_config_core::target::BuildTarget::LinuxX64ControlledV1,
                true,
            )
            .unwrap()
        };
        let ecu = scratch.0.join("ecu");
        let preview = prepare().preview(&ecu).unwrap();
        prepare()
            .generate_previewed(&ecu, &preview.revision)
            .unwrap();
        let xml_path = ecu.join("generated/descriptions/Host_Implementation.arxml");
        let xml = std::fs::read_to_string(&xml_path).unwrap();
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let implementation = doc
            .descendants()
            .find(|node| named(*node, "BSW-IMPLEMENTATION", "RteHostImplementation"))
            .unwrap();
        assert_eq!(
            implementation
                .children()
                .find(|node| node.has_tag_name("VENDOR-ID"))
                .unwrap()
                .text(),
            Some(value)
        );
        let seal = std::fs::read(ecu.join("meta/files.sha256")).unwrap();
        let pending = prepare();
        let old = prepare().preview(&ecu).unwrap();
        let source_path = root.join("ecuc.arxml");
        let original = std::fs::read(&source_path).unwrap();
        let mut external = original.clone();
        external.push(b'\n');
        std::fs::write(&source_path, &external).unwrap();
        assert!(pending.generate_previewed(&ecu, &old.revision).is_err());
        assert_eq!(std::fs::read(&source_path).unwrap(), external);
        assert_eq!(std::fs::read(ecu.join("meta/files.sha256")).unwrap(), seal);
        std::fs::write(&source_path, original).unwrap();
        std::fs::write(&xml_path, b"tampered").unwrap();
        assert!(prepare().preview(&ecu).is_err());
        assert_eq!(std::fs::read(&xml_path).unwrap(), b"tampered");
        let files = plan.component_contract_files().unwrap();
        let contract: serde_json::Value =
            serde_json::from_slice(content(files.files(), "contract.json")).unwrap();
        assert_eq!(
            contract["composition"]["generation"]["codeVendorId"],
            value.parse::<u16>().unwrap()
        );
        let output = scratch.0.join("kit");
        let preview = files.preview(&output).unwrap();
        files
            .generate_previewed(&output, &preview.revision)
            .unwrap();
        assert_eq!(
            std::fs::read(output.join("contract.json")).unwrap(),
            content(files.files(), "contract.json")
        );
        std::fs::write(output.join("contract.json"), b"tampered").unwrap();
        assert!(files.preview(&output).is_err());
        assert_eq!(
            std::fs::read(output.join("contract.json")).unwrap(),
            b"tampered"
        );
        for source in &sources {
            assert_eq!(
                std::fs::read(root.join(source.logical_path())).unwrap(),
                source.bytes()
            );
        }
    }
}
