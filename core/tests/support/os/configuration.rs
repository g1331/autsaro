use autosar_config_core::integration::{InputSource, PlanDependencies, RuntimeCatalog, build_plan};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

pub fn verify() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let original = super::epic4_plan::inputs();
    let marker = "<ECUC-NUMERICAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-BOOLEAN-PARAM-DEF\">/AUTOSAR/EcucDefs/Os/OsOS/OsUseResScheduler</DEFINITION-REF><VALUE>false</VALUE></ECUC-NUMERICAL-PARAM-VALUE>";
    let replaced = |replacement: &str| {
        original
            .iter()
            .map(|source| {
                let text = String::from_utf8(source.bytes().to_vec()).unwrap();
                InputSource::new(
                    source.logical_path(),
                    text.replace(marker, replacement).into_bytes(),
                )
                .unwrap()
            })
            .collect::<Vec<_>>()
    };
    let scratch = super::Scratch::new();
    for (boolean, enabled) in [("false", false), ("true", true), ("0", false), ("1", true)] {
        let replacement =
            marker.replace("<VALUE>false</VALUE>", &format!("<VALUE>{boolean}</VALUE>"));
        let mut inputs = replaced(&replacement);
        if boolean == "1" {
            let ecuc = inputs
                .iter()
                .position(|input| input.logical_path() == "ecuc.arxml")
                .unwrap();
            let mut xml = String::from_utf8(inputs[ecuc].bytes().to_vec()).unwrap();
            let doc = roxmltree::Document::parse(&xml).unwrap();
            let module = doc
                .descendants()
                .find(|node| {
                    node.has_tag_name("ECUC-MODULE-CONFIGURATION-VALUES")
                        && node.children().any(|child| {
                            child.has_tag_name("DEFINITION-REF")
                                && child.text() == Some("/AUTOSAR/EcucDefs/Os")
                        })
                })
                .unwrap();
            let containers = module
                .children()
                .find(|node| node.has_tag_name("CONTAINERS"))
                .unwrap();
            let at = containers.range().end - "</CONTAINERS>".len();
            xml.insert_str(at, "<ECUC-CONTAINER-VALUE><SHORT-NAME>RES_SCHEDULER</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">/AUTOSAR/EcucDefs/Os/OsResource</DEFINITION-REF><PARAMETER-VALUES><ECUC-TEXTUAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-ENUMERATION-PARAM-DEF\">/AUTOSAR/EcucDefs/Os/OsResource/OsResourceProperty</DEFINITION-REF><VALUE>STANDARD</VALUE></ECUC-TEXTUAL-PARAM-VALUE></PARAMETER-VALUES></ECUC-CONTAINER-VALUE>");
            inputs[ecuc] = InputSource::new("ecuc.arxml", xml.into_bytes()).unwrap();
            let false_inputs = inputs
                .iter()
                .map(|input| {
                    InputSource::new(
                        input.logical_path(),
                        String::from_utf8(input.bytes().to_vec())
                            .unwrap()
                            .replace(
                                "OsUseResScheduler</DEFINITION-REF><VALUE>1</VALUE>",
                                "OsUseResScheduler</DEFINITION-REF><VALUE>0</VALUE>",
                            )
                            .into_bytes(),
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>();
            assert!(build_plan(&false_inputs, &dependencies, &runtime).is_err());
        }
        let plan = build_plan(&inputs, &dependencies, &runtime).unwrap();
        let files = plan
            .ecu_integration_files(super::tooling::native_target())
            .unwrap();
        let project = scratch.0.join(format!("source-{boolean}"));
        let preview = files.preview(&project).unwrap();
        files
            .generate_previewed(&project, &preview.revision)
            .unwrap();
        {
            let moved = scratch.0.join(format!("moved scheduler {boolean}"));
            fs::rename(project, &moved).unwrap();
            let output = scratch.0.join(format!("build-{boolean}"));
            let mut compile = super::tooling::ecu_build_command(
                &moved,
                &output,
                "test",
                Some(&root.join("core/tests/fixtures/ecu_scheduler_control.c")),
            );
            let result = super::tooling::run_public_command(
                &mut compile,
                &scratch.0,
                &format!("compile-{boolean}"),
                Duration::from_secs(180),
            );
            assert!(
                result.status.success(),
                "{}{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            let mut run = Command::new(super::tooling::native_binary(&output, "ecu_probe"));
            run.arg(if enabled { "true" } else { "false" });
            let result = super::tooling::run_public_command(
                &mut run,
                &scratch.0,
                &format!("run-{boolean}"),
                Duration::from_secs(15),
            );
            assert!(
                result.status.success(),
                "{}{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(String::from_utf8_lossy(&result.stdout).contains(&format!(
                "scheduler_consumer enabled={} calls=2 reason=0",
                usize::from(enabled)
            )));
        }
    }
    for replacement in [
        String::new(),
        format!("{marker}{marker}"),
        marker.replace("false", "TRUE"),
        marker.replace("false", "2"),
    ] {
        assert!(build_plan(&replaced(&replacement), &dependencies, &runtime).is_err());
    }
    for (old, new) in [
        (
            "/Os/OsOS/OsUseResScheduler",
            "/Os/OsOS/OsHooks/OsUseResScheduler",
        ),
        ("<VALUE>SC1</VALUE>", "<VALUE>SC2</VALUE>"),
    ] {
        let inputs = original
            .iter()
            .map(|input| {
                let text = String::from_utf8(input.bytes().to_vec()).unwrap();
                InputSource::new(input.logical_path(), text.replace(old, new).into_bytes()).unwrap()
            })
            .collect::<Vec<_>>();
        assert!(build_plan(&inputs, &dependencies, &runtime).is_err());
    }
    let ecuc = original
        .iter()
        .find(|input| input.logical_path() == "ecuc.arxml")
        .unwrap();
    let xml = String::from_utf8(ecuc.bytes().to_vec()).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    for name in [
        "OsStatus",
        "OsScalabilityClass",
        "OsUseGetServiceId",
        "OsUseParameterAccess",
        "OsStartupHook",
        "OsErrorHook",
    ] {
        let parameter = doc
            .descendants()
            .find(|node| {
                node.has_tag_name("DEFINITION-REF")
                    && node
                        .text()
                        .is_some_and(|text| text.ends_with(&format!("/{name}")))
            })
            .unwrap()
            .parent()
            .unwrap();
        let mut incomplete = xml.clone();
        incomplete.replace_range(parameter.range(), "");
        let inputs = original
            .iter()
            .map(|input| {
                InputSource::new(
                    input.logical_path(),
                    if input.logical_path() == "ecuc.arxml" {
                        incomplete.as_bytes().to_vec()
                    } else {
                        input.bytes().to_vec()
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        assert!(
            build_plan(&inputs, &dependencies, &runtime).is_err(),
            "missing {name}"
        );
    }
    let mut misplaced = xml.replace(marker, "");
    let hooks_start = misplaced.find("/Os/OsOS/OsHooks</DEFINITION-REF>").unwrap();
    let at = hooks_start
        + misplaced[hooks_start..].find("<PARAMETER-VALUES>").unwrap()
        + "<PARAMETER-VALUES>".len();
    misplaced.insert_str(at, marker);
    let inputs = original
        .iter()
        .map(|input| {
            InputSource::new(
                input.logical_path(),
                if input.logical_path() == "ecuc.arxml" {
                    misplaced.as_bytes().to_vec()
                } else {
                    input.bytes().to_vec()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    assert!(
        build_plan(&inputs, &dependencies, &runtime).is_err(),
        "misplaced valid scheduler definition"
    );
    for parameter in ["OsUseResScheduler", "OsUseGetServiceId", "OsErrorHook"] {
        let definition = format!(
            r#"<ECUC-MODULE-DEF><SHORT-NAME>VendorOs</SHORT-NAME><LOWER-MULTIPLICITY>0</LOWER-MULTIPLICITY><UPPER-MULTIPLICITY>1</UPPER-MULTIPLICITY><POST-BUILD-VARIANT-SUPPORT>false</POST-BUILD-VARIANT-SUPPORT><SUPPORTED-CONFIG-VARIANTS><SUPPORTED-CONFIG-VARIANT>VARIANT-PRE-COMPILE</SUPPORTED-CONFIG-VARIANT></SUPPORTED-CONFIG-VARIANTS><CONTAINERS><ECUC-PARAM-CONF-CONTAINER-DEF><SHORT-NAME>OsOS</SHORT-NAME><LOWER-MULTIPLICITY>1</LOWER-MULTIPLICITY><UPPER-MULTIPLICITY>1</UPPER-MULTIPLICITY><ORIGIN>Vendor</ORIGIN><PARAMETERS><ECUC-BOOLEAN-PARAM-DEF><SHORT-NAME>{parameter}</SHORT-NAME><LOWER-MULTIPLICITY>1</LOWER-MULTIPLICITY><UPPER-MULTIPLICITY>1</UPPER-MULTIPLICITY><SCOPE>LOCAL</SCOPE><ORIGIN>Vendor</ORIGIN><POST-BUILD-VARIANT-VALUE>false</POST-BUILD-VARIANT-VALUE><VALUE-CONFIG-CLASSES><ECUC-VALUE-CONFIGURATION-CLASS><CONFIG-CLASS>PRE-COMPILE</CONFIG-CLASS><CONFIG-VARIANT>VARIANT-PRE-COMPILE</CONFIG-VARIANT></ECUC-VALUE-CONFIGURATION-CLASS></VALUE-CONFIG-CLASSES><SYMBOLIC-NAME-VALUE>false</SYMBOLIC-NAME-VALUE><DEFAULT-VALUE>true</DEFAULT-VALUE></ECUC-BOOLEAN-PARAM-DEF></PARAMETERS></ECUC-PARAM-CONF-CONTAINER-DEF></CONTAINERS></ECUC-MODULE-DEF>"#
        );
        let added = xml.replace("</ELEMENTS>", &format!("{definition}</ELEMENTS>"));
        let inputs = original
            .iter()
            .map(|input| {
                InputSource::new(
                    input.logical_path(),
                    if input.logical_path() == "ecuc.arxml" {
                        added.as_bytes().to_vec()
                    } else {
                        input.bytes().to_vec()
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        assert!(
            build_plan(&inputs, &dependencies, &runtime).is_ok(),
            "retained vendor definition {parameter}"
        );
        let official = format!(
            "/AUTOSAR/EcucDefs/Os/OsOS/{}{parameter}",
            if parameter == "OsErrorHook" {
                "OsHooks/"
            } else {
                ""
            }
        );
        let aliased = added.replace(
            &official,
            &format!("/Configuration/VendorOs/OsOS/{parameter}"),
        );
        let inputs = original
            .iter()
            .map(|input| {
                InputSource::new(
                    input.logical_path(),
                    if input.logical_path() == "ecuc.arxml" {
                        aliased.as_bytes().to_vec()
                    } else {
                        input.bytes().to_vec()
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let diagnostics = match build_plan(&inputs, &dependencies, &runtime) {
            Ok(_) => panic!("vendor parameter replaced standard OS policy: {parameter}"),
            Err(diagnostics) => diagnostics,
        };
        assert!(
            diagnostics
                .iter()
                .any(|issue| issue.code == "OS_CONFIGURATION"),
            "{diagnostics:?}"
        );
    }
}
