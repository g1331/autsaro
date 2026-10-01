use autosar_config_core::integration::{InputSource, PlanDependencies, RuntimeCatalog, build_plan};
use autosar_config_core::schema;
use std::fs;
use std::path::Path;
use std::time::Duration;

pub fn verify() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let scratch = super::Scratch::new();
    for counter in ["SystemCounter", "AuxClock"] {
        let inputs = super::epic4_plan::inputs()
            .iter()
            .map(|input| {
                let mut text = String::from_utf8(input.bytes().to_vec()).unwrap();
                if input.logical_path() == "ecuc.arxml" {
                    let parameter = text
                        .find("OsCounterMaxAllowedValue</DEFINITION-REF>")
                        .unwrap();
                    let start = parameter + text[parameter..].find("<VALUE>").unwrap() + 7;
                    let end = start + text[start..].find("</VALUE>").unwrap();
                    text.replace_range(start..end, "15");
                }
                InputSource::new(
                    input.logical_path(),
                    text.replace("SystemCounter", counter).into_bytes(),
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let plan = build_plan(&inputs, &dependencies, &runtime).unwrap();
        let files = plan
            .ecu_integration_files(super::tooling::native_target())
            .unwrap();
        let project = scratch.0.join(counter);
        let preview = files.preview(&project).unwrap();
        files
            .generate_previewed(&project, &preview.revision)
            .unwrap();
        let xml_path = project.join("os/Os_Service.arxml");
        let xml = fs::read_to_string(&xml_path).unwrap();
        let issues =
            schema::validate_files(&schema::schema_archive(root), &[(&xml_path, &xml)]).unwrap();
        assert!(issues.is_empty(), "{issues:?}");
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let text = |tag: &str| {
            doc.descendants()
                .filter(|node| node.has_tag_name(tag))
                .filter_map(|node| node.text())
                .collect::<Vec<_>>()
        };
        assert_eq!(text("IS-SERVICE"), ["true"]);
        assert_eq!(text("SERVICE-KIND"), ["OPERATING-SYSTEM"]);
        assert_eq!(text("DIRECTION"), ["OUT", "INOUT", "OUT"]);
        assert_eq!(text("TYPE-TREF"), ["/OsServices/TimeInMicrosecondsType"; 3]);
        assert_eq!(text("VALUE-TYPE-TREF"), ["/OsServices/CounterType"]);
        assert_eq!(
            text("PROVIDED-INTERFACE-TREF"),
            [format!("/OsServices/OsService_{counter}")]
        );
        assert_eq!(text("PORT-REF"), ["/OsServices/Os/OsService"]);
        assert_eq!(
            text("POSSIBLE-ERROR-REF"),
            [
                format!("/OsServices/OsService_{counter}/E_OS_ID"),
                format!("/OsServices/OsService_{counter}/E_OS_ID"),
                format!("/OsServices/OsService_{counter}/E_OS_VALUE"),
            ]
        );
        assert_eq!(text("BASE-TYPE-SIZE"), ["32", "64"]);
        assert_eq!(text("ERROR-CODE"), ["1", "3", "7", "8"]);
        assert_eq!(text("VALUE"), ["0"]);
        assert_eq!(
            text("SYMBOL"),
            [
                format!("OsService_{counter}_GetCounterValue"),
                format!("OsService_{counter}_GetElapsedValue")
            ]
        );
        for reference in doc
            .descendants()
            .filter(|node| node.is_element() && node.attribute("DEST").is_some())
        {
            let path = reference.text().unwrap();
            let mut matches = 0;
            for target in doc
                .descendants()
                .filter(|node| node.has_tag_name(reference.attribute("DEST").unwrap()))
            {
                let names = target
                    .ancestors()
                    .filter_map(|parent| {
                        parent
                            .children()
                            .find(|child| child.has_tag_name("SHORT-NAME"))
                            .and_then(|child| child.text())
                    })
                    .collect::<Vec<_>>();
                let actual = format!("/{}", names.into_iter().rev().collect::<Vec<_>>().join("/"));
                if actual == path {
                    matches += 1;
                }
            }
            assert_eq!(matches, 1, "unresolved or ambiguous reference: {path}");
        }
        let stage = scratch.0.join(format!("native-{counter}"));
        fs::create_dir(&stage).unwrap();
        let mut command = super::tooling::python_command();
        command
            .args(["os-counter-service", "--directory"])
            .arg(&stage)
            .arg("--project")
            .arg(&project)
            .arg("--counter")
            .arg(counter);
        let output = super::epic4_ecu::run_public_command(
            &mut command,
            &stage,
            "counter-service",
            Duration::from_secs(180),
        );
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    for (old, new) in [
        (
            "<SYMBOL>EchoApplication_Periodic</SYMBOL>",
            "<SYMBOL>OsService_SystemCounter_GetCounterValue</SYMBOL>",
        ),
        ("EchoApplication", "Os"),
    ] {
        let inputs = super::epic4_plan::inputs()
            .iter()
            .map(|input| {
                let text = String::from_utf8(input.bytes().to_vec()).unwrap();
                InputSource::new(input.logical_path(), text.replace(old, new).into_bytes()).unwrap()
            })
            .collect::<Vec<_>>();
        let candidate = build_plan(&inputs, &dependencies, &runtime).unwrap();
        assert!(
            candidate
                .ecu_integration_files(super::tooling::native_target())
                .is_err(),
            "service collision {new}"
        );
    }
}
