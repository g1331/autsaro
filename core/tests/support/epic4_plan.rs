use autosar_config_core::integration::{
    InputSource, PlanDependencies, RuntimeCatalog, build_plan, inspect_inputs,
};
use std::fs;
use std::path::Path;

#[test]
fn integration_diagnostics_keep_message_identity_and_source_evidence() {
    let source = InputSource::new("用户/input.arxml", vec![0xff]).unwrap_err();
    let wire = serde_json::to_value(&source).unwrap();
    assert_eq!(wire["code"], "XML_ENCODING");
    assert_eq!(wire["file"], "用户/input.arxml");
    assert_eq!(wire["object"], "/");
    assert_eq!(
        wire["message"]["key"],
        "backend.integration.mod.input_not_utf8_xml"
    );
    assert!(
        wire["message"]["params"]["error"]
            .as_str()
            .unwrap()
            .contains("utf-8")
    );
    assert_eq!(
        wire["remedy"]["key"],
        "backend.integration.mod.convert_source_to_utf8_arxml"
    );
    assert!(wire["message"].get("english").is_none());

    let bytes = b"<AUTOSAR><!-- retained evidence --></AUTOSAR>".to_vec();
    let source = InputSource::new("用户/input.arxml", bytes.clone()).unwrap();
    let identity = source.sha256();
    let diagnostic = InputSource::new("../input.arxml", bytes.clone()).unwrap_err();
    let messages = autosar_config_core::message::LocalizedText::messages([
        diagnostic.message.clone(),
        diagnostic.remedy.clone(),
        autosar_config_core::message::LocalizedText::evidence("用户/<tool output>"),
    ]);
    let wire = serde_json::to_value(messages).unwrap();
    assert_eq!(
        wire[0]["key"],
        "backend.integration.mod.input_identity_relative_arxml_required"
    );
    assert_eq!(
        wire[1]["key"],
        "backend.integration.mod.use_unique_portable_relative_path"
    );
    assert_eq!(wire[2], "用户/<tool output>");
    assert_eq!(source.bytes(), bytes);
    assert_eq!(source.sha256(), identity);
}

pub fn inputs() -> Vec<InputSource> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let directory = root.join("core/tests/fixtures/epic4/positive");
    let mut paths: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            InputSource::new(
                path.file_name().unwrap().to_str().unwrap(),
                fs::read(&path).unwrap(),
            )
            .unwrap()
        })
        .collect()
}

pub fn validated_integration_plan() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime =
        RuntimeCatalog::from_repository(root).unwrap_or_else(|issues| panic!("{issues:?}"));
    let original = inputs();
    let plan = build_plan(&original, &dependencies, &runtime)
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    let description = plan.description();
    assert_eq!(description.profile, "epic4-win64-sr-cs-v1");
    assert_eq!(description.sources.len(), 7);
    assert_eq!(description.signals.len(), 2);
    assert_eq!(description.routes.len(), 4);
    assert_eq!(description.events.len(), 3);
    assert_eq!(description.symbols.len(), 25);
    assert_eq!(
        description
            .symbols
            .iter()
            .find(|symbol| symbol.symbol == "EchoApplication_ReadData")
            .unwrap()
            .definition_owner,
        "story-4.15:application implementation"
    );
    assert_eq!(
        description
            .handles
            .iter()
            .filter(|handle| handle.domain == "com_signal")
            .map(|handle| (handle.path.as_str(), handle.handle))
            .collect::<Vec<_>>(),
        [
            ("/Configuration/Com/Config/RxValue", 0),
            ("/Configuration/Com/Config/TxValue", 1)
        ]
    );
    let mut reversed = original.clone();
    reversed.reverse();
    let reordered = build_plan(&reversed, &dependencies, &runtime)
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(
        serde_json::to_vec(description).unwrap(),
        serde_json::to_vec(reordered.description()).unwrap()
    );
    let paths: Vec<_> = original
        .iter()
        .map(|source| {
            root.join("core/tests/fixtures/epic4/positive")
                .join(source.logical_path())
        })
        .collect();
    let workspace =
        autosar_config_core::Workspace::open_legacy(paths, dependencies.xsd_archive.clone())
            .unwrap();
    let workspace_plan = workspace
        .integration_plan_legacy(&runtime, dependencies.mod_archive.clone())
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(
        serde_json::to_vec(description).unwrap(),
        serde_json::to_vec(workspace_plan.description()).unwrap()
    );
    let scratch = super::Scratch::new();
    let copied: Vec<_> = original
        .iter()
        .map(|source| {
            let path = scratch.0.join(source.logical_path());
            fs::write(&path, source.bytes()).unwrap();
            path
        })
        .collect();
    let stale = autosar_config_core::Workspace::open_legacy(
        copied.clone(),
        dependencies.xsd_archive.clone(),
    )
    .unwrap();
    let mut external = fs::read(&copied[0]).unwrap();
    external.extend_from_slice(b"\n<!-- external edit -->\n");
    fs::write(&copied[0], &external).unwrap();
    let issues = stale
        .integration_plan_legacy(&runtime, dependencies.mod_archive.clone())
        .err()
        .expect("A stale workspace must not produce an integration plan");
    assert!(issues.iter().any(|issue| issue.code == "SOURCE_CHANGED"));
    assert_eq!(fs::read(&copied[0]).unwrap(), external);
    let cases: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("core/tests/fixtures/epic4/negative/cases.json")).unwrap(),
    )
    .unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let mut mutated = original.clone();
        for override_path in case["overrides"].as_array().unwrap() {
            let path = root
                .join("core/tests/fixtures/epic4")
                .join(override_path.as_str().unwrap());
            let file = path.file_name().unwrap().to_str().unwrap();
            let slot = mutated
                .iter_mut()
                .find(|source| source.logical_path() == file)
                .unwrap();
            *slot = InputSource::new(file, fs::read(&path).unwrap()).unwrap();
        }
        let issues = match build_plan(&mutated, &dependencies, &runtime) {
            Ok(_) => panic!("{case} was accepted"),
            Err(issues) => issues,
        };
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == case["category"].as_str().unwrap()
                    && issue.file.as_deref() == case["file"].as_str()
                    && matches!(&issue.remedy, autosar_config_core::message::LocalizedText::Message(message) if message.key.starts_with("backend."))),
            "{case}: {issues:?}"
        );
    }
    let output = super::tooling::run_public_command(
        super::tooling::python_command()
            .args(["-m", "autosar_tooling", "bsw-catalog", "--probe"])
            .current_dir(root),
        &scratch.0,
        "bsw-catalog",
        std::time::Duration::from_secs(180),
    );
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn plan_additional_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime =
        RuntimeCatalog::from_repository(root).unwrap_or_else(|issues| panic!("{issues:?}"));
    for (file, before, after, expected) in [
        (
            "application.arxml",
            "<SYMBOL>EchoApplication_ReadData</SYMBOL>",
            "<SYMBOL>Com_Init</SYMBOL>",
            "SYMBOL_PRODUCER_DUPLICATE",
        ),
        (
            "ecuc.arxml",
            "/CanTpBs</DEFINITION-REF>\n                          <VALUE>0</VALUE>",
            "/CanTpBs</DEFINITION-REF>\n                          <VALUE>1</VALUE>",
            "SERVICE_UNSUPPORTED",
        ),
        (
            "ecuc.arxml",
            "/CanTpRxNSduId</DEFINITION-REF>\n                          <VALUE>0</VALUE>",
            "/CanTpRxNSduId</DEFINITION-REF>\n                          <VALUE>-1</VALUE>",
            "CONFIGURATION_IDENTIFIER",
        ),
    ] {
        let mut mutated = inputs();
        let source = mutated
            .iter_mut()
            .find(|source| source.logical_path() == file)
            .unwrap();
        let text = std::str::from_utf8(source.bytes()).unwrap();
        assert_eq!(text.matches(before).count(), 1, "{before}");
        *source = InputSource::new(file, text.replace(before, after).into_bytes()).unwrap();
        let issues = match build_plan(&mutated, &dependencies, &runtime) {
            Ok(_) => panic!("{expected} accepted"),
            Err(issues) => issues,
        };
        assert!(
            issues.iter().any(|issue| issue.code == expected),
            "{issues:?}"
        );
    }
    let mut unknown = inputs();
    let ecuc = unknown
        .iter_mut()
        .find(|source| source.logical_path() == "ecuc.arxml")
        .unwrap();
    let mut text = std::str::from_utf8(ecuc.bytes()).unwrap().to_owned();
    let document = roxmltree::Document::parse(&text).unwrap();
    let module = document
        .descendants()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
                && node.children().any(|child| {
                    child.is_element()
                        && child.tag_name().name() == "DEFINITION-REF"
                        && child.text() == Some("/AUTOSAR/EcucDefs/Can")
                })
        })
        .unwrap();
    let containers = module
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "CONTAINERS")
        .unwrap();
    let start = containers.range().start;
    let insertion = start + text[start..].find('>').unwrap() + 1;
    text.insert_str(insertion, "<ECUC-CONTAINER-VALUE><SHORT-NAME>General</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-PARAM-CONF-CONTAINER-DEF\">/AUTOSAR/EcucDefs/Can/CanGeneral</DEFINITION-REF><PARAMETER-VALUES><ECUC-NUMERICAL-PARAM-VALUE><DEFINITION-REF DEST=\"ECUC-BOOLEAN-PARAM-DEF\">/AUTOSAR/EcucDefs/Can/CanGeneral/CanDevErrorDetect</DEFINITION-REF><VALUE>false</VALUE></ECUC-NUMERICAL-PARAM-VALUE></PARAMETER-VALUES></ECUC-CONTAINER-VALUE>");
    *ecuc = InputSource::new("ecuc.arxml", text.into_bytes()).unwrap();
    let issues = match build_plan(&unknown, &dependencies, &runtime) {
        Ok(_) => panic!("Unknown necessary target parameter accepted"),
        Err(issues) => issues,
    };
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "TARGET_PARAMETER_UNSUPPORTED"),
        "{issues:?}"
    );
    let mut collision = inputs();
    for source in &mut collision {
        let text = std::str::from_utf8(source.bytes()).unwrap();
        let text = if source.logical_path() == "types.arxml" {
            text.replace(
                "<SHORT-NAME>uint32</SHORT-NAME>",
                "<SHORT-NAME>U32</SHORT-NAME>",
            )
        } else {
            text.to_owned()
        };
        let text = text
            .replace("/Types/uint32", "/Types/U32")
            .replace("Dcm_DataElement_ApplicationValueType", "uint32");
        *source = InputSource::new(source.logical_path(), text.into_bytes()).unwrap();
    }
    let issues = match build_plan(&collision, &dependencies, &runtime) {
        Ok(_) => panic!("Colliding array/scalar C type accepted"),
        Err(issues) => issues,
    };
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "SYMBOL_NORMALIZATION_COLLISION"),
        "{issues:?}"
    );
    let mut renamed = inputs();
    for source in &mut renamed {
        let text = std::str::from_utf8(source.bytes())
            .unwrap()
            .replace("EchoApplication", "LocalApplication")
            .replace("RxValue", "InputValue")
            .replace("TxValue", "OutputValue")
            .replace(
                "<IDENTIFIER>800</IDENTIFIER>",
                "<IDENTIFIER>1100</IDENTIFIER>",
            )
            .replace(
                "<IDENTIFIER>801</IDENTIFIER>",
                "<IDENTIFIER>1101</IDENTIFIER>",
            );
        let text = if source.logical_path() == "ecuc.arxml" {
            text.replace("<VALUE>800</VALUE>", "<VALUE>1100</VALUE>")
                .replace("<VALUE>801</VALUE>", "<VALUE>1101</VALUE>")
        } else {
            text
        };
        *source = InputSource::new(source.logical_path(), text.into_bytes()).unwrap();
    }
    let plan =
        build_plan(&renamed, &dependencies, &runtime).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(
        plan.description().component.as_ref().unwrap().component,
        "/Application/LocalApplication"
    );
    assert_eq!(
        plan.description()
            .signals
            .iter()
            .map(|signal| signal.can_id)
            .collect::<Vec<_>>(),
        [1100, 1101]
    );
    let mut retained = inputs();
    let unrelated = retained
        .iter_mut()
        .find(|source| source.logical_path() == "unrelated.arxml")
        .unwrap();
    let text = std::str::from_utf8(unrelated.bytes())
        .unwrap()
        .replace("UnrelatedUint16", "Value");
    *unrelated = InputSource::new("unrelated.arxml", text.into_bytes()).unwrap();
    let plan = build_plan(&retained, &dependencies, &runtime)
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert!(
        plan.description()
            .objects
            .iter()
            .any(|object| object.path == "/Unrelated/Value")
    );
    assert!(
        plan.description()
            .objects
            .iter()
            .any(|object| object.path == "/Types/ValueInterface/Value")
    );
    assert_eq!(
        plan.sources()
            .iter()
            .find(|source| source.logical_path() == "unrelated.arxml")
            .unwrap()
            .bytes(),
        retained
            .iter()
            .find(|source| source.logical_path() == "unrelated.arxml")
            .unwrap()
            .bytes()
    );
    let mut automatic = inputs();
    let mut unrelated_network = inputs();
    let source = unrelated_network
        .iter_mut()
        .find(|source| source.logical_path() == "unrelated.arxml")
        .unwrap();
    let extra = "<CAN-CLUSTER><SHORT-NAME>OtherNetwork</SHORT-NAME><CAN-CLUSTER-VARIANTS><CAN-CLUSTER-CONDITIONAL><BAUDRATE>250000</BAUDRATE><PHYSICAL-CHANNELS><CAN-PHYSICAL-CHANNEL><SHORT-NAME>OtherChannel</SHORT-NAME></CAN-PHYSICAL-CHANNEL></PHYSICAL-CHANNELS></CAN-CLUSTER-CONDITIONAL></CAN-CLUSTER-VARIANTS></CAN-CLUSTER>";
    let text = std::str::from_utf8(source.bytes())
        .unwrap()
        .replace("</ELEMENTS>", &format!("{extra}</ELEMENTS>"));
    *source = InputSource::new("unrelated.arxml", text.into_bytes()).unwrap();
    let plan = build_plan(&unrelated_network, &dependencies, &runtime)
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert!(
        plan.description()
            .objects
            .iter()
            .any(|object| object.path == "/Unrelated/OtherNetwork/OtherChannel")
    );
    assert_eq!(
        plan.sources()
            .iter()
            .find(|source| source.logical_path() == "unrelated.arxml")
            .unwrap()
            .bytes(),
        unrelated_network
            .iter()
            .find(|source| source.logical_path() == "unrelated.arxml")
            .unwrap()
            .bytes()
    );
    let ecuc = automatic
        .iter_mut()
        .find(|source| source.logical_path() == "ecuc.arxml")
        .unwrap();
    let mut text = std::str::from_utf8(ecuc.bytes()).unwrap().to_owned();
    let document = roxmltree::Document::parse(&text).unwrap();
    let mut ranges: Vec<_> = document
        .descendants()
        .filter(|node| {
            node.is_element()
                && node.tag_name().name() == "ECUC-NUMERICAL-PARAM-VALUE"
                && node.children().any(|child| {
                    child.is_element()
                        && child.tag_name().name() == "DEFINITION-REF"
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/OsEventMask"))
                })
        })
        .map(|node| node.range())
        .collect();
    assert_eq!(ranges.len(), 3);
    ranges.reverse();
    for range in ranges {
        text.replace_range(range, "");
    }
    *ecuc = InputSource::new("ecuc.arxml", text.into_bytes()).unwrap();
    let plan = build_plan(&automatic, &dependencies, &runtime)
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(
        plan.description()
            .events
            .iter()
            .map(|event| (event.path.as_str(), event.mask))
            .collect::<Vec<_>>(),
        [
            ("/Configuration/Os/Ev_App", 1),
            ("/Configuration/Os/Ev_IO", 2),
            ("/Configuration/Os/Ev_Work", 4),
        ]
    );
}

pub fn graph_identity_and_references() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let original = inputs();
    let checked =
        inspect_inputs(&original, &dependencies).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(checked.sources().len(), 7);
    let component = checked
        .component_contract()
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(component.period_ms, 10);
    assert_eq!(component.periodic_symbol, "EchoApplication_Periodic");
    assert_eq!(component.service.array_length, 4);
    assert_eq!(
        component.service.runnable_symbol,
        "EchoApplication_ReadData"
    );
    assert_eq!(
        component
            .data_ports
            .iter()
            .map(|port| port.api_symbol.as_str())
            .collect::<Vec<_>>(),
        ["Rte_Read_RxValue_Value", "Rte_Write_TxValue_Value"]
    );
    let schedule = checked
        .schedule_contract()
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(schedule.task, "/Configuration/Os/Task_Ecu");
    assert_eq!(schedule.counter_maximum, 65535);
    assert_eq!(
        schedule
            .entities
            .iter()
            .map(|entity| (entity.symbol.as_str(), entity.period_ms, entity.position))
            .collect::<Vec<_>>(),
        [
            ("Can_MainFunction_Wakeup", 1, 1),
            ("CanTp_AdvanceTime", 1, 2),
            ("Com_AdvanceTime", 1, 3),
            ("EchoApplication_Periodic", 10, 4),
            ("Com_TriggerTransmit", 10, 5),
            ("Dcm_AdvanceTime", 1, 6),
        ]
    );
    let channels = checked
        .signal_channels()
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(
        channels
            .iter()
            .map(|channel| (
                channel.can_id,
                channel.can_if_handle,
                channel.dlc,
                channel.receive,
                channel.deadline_ms,
                channel.transmit_period_ms
            ))
            .collect::<Vec<_>>(),
        [
            (800, 0, 4, true, Some(30), None),
            (801, 1, 4, false, None, Some(10))
        ]
    );
    let catalog =
        RuntimeCatalog::from_repository(root).unwrap_or_else(|issues| panic!("{issues:?}"));
    let bsw = checked
        .current_bsw_contracts(&catalog)
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(bsw.len(), 17);
    let receive = bsw
        .iter()
        .find(|entry| entry.symbol == "Com_GetSignal")
        .unwrap();
    assert_eq!(receive.return_type, "EcuStatus");
    assert_eq!(
        receive
            .arguments
            .iter()
            .map(|argument| (argument.native_type.as_str(), argument.direction.as_str()))
            .collect::<Vec<_>>(),
        [
            ("uint16_t", "IN"),
            ("uint32_t *", "OUT"),
            ("uint8_t *", "OUT")
        ]
    );
    let diagnostic = checked
        .diagnostic_contract()
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(diagnostic.did, 0x1234);
    assert_eq!(
        (diagnostic.request_can_id, diagnostic.response_can_id),
        (0x700, 0x708)
    );
    assert_eq!(
        (
            diagnostic.p2_ms,
            diagnostic.p2_star_ms,
            diagnostic.n_as_ms,
            diagnostic.n_bs_ms,
            diagnostic.n_cr_ms,
            diagnostic.buffer_bytes
        ),
        (50, 5000, 200, 200, 200, 64)
    );
    assert_eq!(diagnostic.sessions, [1, 3]);
    let objects = checked.objects();
    assert!(objects.iter().any(
        |object| object.path == "/Application/EchoApplication/RxValue"
            && object.kind == "R-PORT-PROTOTYPE"
            && object.file == "application.arxml"
    ));
    assert!(objects.iter().any(
        |object| object.path == "/Services/DcmService/ApplicationValue"
            && object.kind == "R-PORT-PROTOTYPE"
    ));
    assert!(
        checked
            .identities()
            .iter()
            .any(|source| source.logical_path == "unrelated.arxml" && source.roles == ["retained"])
    );
    for source in checked.sources() {
        let before = original
            .iter()
            .find(|before| before.logical_path() == source.logical_path())
            .unwrap();
        assert_eq!(before.bytes(), source.bytes());
        assert_eq!(before.sha256(), source.sha256());
    }
    let mut reversed = original.clone();
    reversed.reverse();
    let reordered =
        inspect_inputs(&reversed, &dependencies).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(
        serde_json::to_value(checked.identities()).unwrap(),
        serde_json::to_value(reordered.identities()).unwrap()
    );
    assert_eq!(
        serde_json::to_value(objects).unwrap(),
        serde_json::to_value(reordered.objects()).unwrap()
    );
    for (case, code) in [
        ("broken-reference", "REFERENCE_UNRESOLVED"),
        ("wrong-dest", "REFERENCE_DEST"),
        ("structure-invalid", "R24_XSD"),
    ] {
        let mut mutated = original.clone();
        let slot = mutated
            .iter_mut()
            .find(|source| source.logical_path() == "application.arxml")
            .unwrap();
        *slot = InputSource::new(
            "application.arxml",
            fs::read(root.join(format!(
                "core/tests/fixtures/epic4/negative/{case}/application.arxml"
            )))
            .unwrap(),
        )
        .unwrap();
        let issues = match inspect_inputs(&mutated, &dependencies) {
            Ok(_) => panic!("{case} was accepted"),
            Err(issues) => issues,
        };
        assert!(
            issues.iter().any(|issue| issue.code == code
                && issue.file.as_deref() == Some("application.arxml")
                && issue.object.is_some()
                && matches!(&issue.remedy, autosar_config_core::message::LocalizedText::Message(message) if message.key.starts_with("backend."))),
            "{issues:?}"
        );
    }
    for name in [
        "../input.arxml",
        "a/../input.arxml",
        "a//input.arxml",
        "a/./input.arxml",
        "C:/input.arxml",
        "a\\input.arxml",
        "input.arxml.",
        "a/CON.arxml",
        "a/LPT1.arxml",
        "a?/input.arxml",
        "a\u{0}/input.arxml",
    ] {
        assert!(
            InputSource::new(name, original[0].bytes().to_vec()).is_err(),
            "{name}"
        );
    }
    let mut duplicate = original.clone();
    duplicate.push(
        InputSource::new(
            "APPLICATION.ARXML",
            original
                .iter()
                .find(|source| source.logical_path() == "application.arxml")
                .unwrap()
                .bytes()
                .to_vec(),
        )
        .unwrap(),
    );
    assert!(match inspect_inputs(&duplicate, &dependencies) {
        Ok(_) => false,
        Err(issues) => issues[0].code == "SOURCE_DUPLICATE",
    });
    for (first, second) in [("Ä.arxml", "ä.arxml"), ("Σ.arxml", "ς.arxml")] {
        let aliases = [
            InputSource::new(first, original[0].bytes().to_vec()).unwrap(),
            InputSource::new(second, original[0].bytes().to_vec()).unwrap(),
        ];
        let issues = inspect_inputs(&aliases, &dependencies).err().unwrap();
        assert_eq!(issues[0].code, "SOURCE_DUPLICATE");
    }
    let missing = PlanDependencies {
        xsd_archive: dependencies.xsd_archive,
        mod_archive: root.join(".scratch/epic4/nonexistent-definition-archive.zip"),
    };
    assert!(match inspect_inputs(&original, &missing) {
        Ok(_) => false,
        Err(issues) => issues[0].code == "DEPENDENCY_MISSING",
    });
}

pub fn bsw_semantic_rejections() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let catalog =
        RuntimeCatalog::from_repository(root).unwrap_or_else(|issues| panic!("{issues:?}"));
    let mut mutated = inputs();
    let bsw = mutated
        .iter_mut()
        .find(|source| source.logical_path() == "bsw.arxml")
        .unwrap();
    *bsw = InputSource::new(
        "bsw.arxml",
        fs::read(root.join("core/tests/fixtures/epic4/negative/missing-bsw-entry/bsw.arxml"))
            .unwrap(),
    )
    .unwrap();
    let result = inspect_inputs(&mutated, &dependencies)
        .and_then(|input| input.current_bsw_contracts(&catalog));
    let issues = match result {
        Ok(_) => panic!("Missing BSW entry accepted"),
        Err(issues) => issues,
    };
    assert!(
        issues.iter().any(|issue| issue.code == "BSW_ENTRY_MISSING"
            && issue.object.as_deref() == Some("/Bsw/Com_AdvanceTime")),
        "{issues:?}"
    );
    let mut mutated = inputs();
    let types = mutated
        .iter_mut()
        .find(|source| source.logical_path() == "types.arxml")
        .unwrap();
    let text = std::str::from_utf8(types.bytes()).unwrap();
    assert_eq!(
        text.matches("<NATIVE-DECLARATION>uint32_t</NATIVE-DECLARATION>")
            .count(),
        1
    );
    *types = InputSource::new(
        "types.arxml",
        text.replace(
            "<NATIVE-DECLARATION>uint32_t</NATIVE-DECLARATION>",
            "<NATIVE-DECLARATION>uint16_t</NATIVE-DECLARATION>",
        )
        .into_bytes(),
    )
    .unwrap();
    let result = inspect_inputs(&mutated, &dependencies)
        .and_then(|input| input.current_bsw_contracts(&catalog));
    let issues = match result {
        Ok(_) => panic!("BSW ABI mismatch accepted"),
        Err(issues) => issues,
    };
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "BSW_SIGNATURE_CONFLICT"
                && issue.file.as_deref() == Some("bsw.arxml")),
        "{issues:?}"
    );
}

pub fn communication_semantic_rejections() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    for (case, code, object) in [
        (
            "direction-conflict",
            "DIRECTION_CONFLICT",
            "/Extract/ReferenceEcu/CanConnector/RxValueFramePort",
        ),
        (
            "length-conflict",
            "LENGTH_CONFLICT",
            "/Extract/RxValueSignal",
        ),
        (
            "transformer",
            "TRANSFORMER_UNSUPPORTED",
            "/Extract/RxValueSignal",
        ),
    ] {
        let mut mutated = inputs();
        let extract = mutated
            .iter_mut()
            .find(|source| source.logical_path() == "extract.arxml")
            .unwrap();
        *extract = InputSource::new(
            "extract.arxml",
            fs::read(root.join(format!(
                "core/tests/fixtures/epic4/negative/{case}/extract.arxml"
            )))
            .unwrap(),
        )
        .unwrap();
        let result =
            inspect_inputs(&mutated, &dependencies).and_then(|input| input.signal_channels());
        let issues = match result {
            Ok(_) => panic!("{case} accepted"),
            Err(issues) => issues,
        };
        assert!(
            issues.iter().any(|issue| issue.code == code
                && issue.file.as_deref() == Some("extract.arxml")
                && issue.object.as_deref() == Some(object)),
            "{case}: {issues:?}"
        );
    }
    let mut renamed = inputs();
    for source in &mut renamed {
        let text = std::str::from_utf8(source.bytes())
            .unwrap()
            .replace(
                "<IDENTIFIER>800</IDENTIFIER>",
                "<IDENTIFIER>1100</IDENTIFIER>",
            )
            .replace(
                "<IDENTIFIER>801</IDENTIFIER>",
                "<IDENTIFIER>1101</IDENTIFIER>",
            );
        let text = if source.logical_path() == "ecuc.arxml" {
            text.replace("<VALUE>800</VALUE>", "<VALUE>1100</VALUE>")
                .replace("<VALUE>801</VALUE>", "<VALUE>1101</VALUE>")
        } else {
            text
        };
        *source = InputSource::new(source.logical_path(), text.into_bytes()).unwrap();
    }
    let result = inspect_inputs(&renamed, &dependencies)
        .and_then(|input| input.signal_channels())
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(
        result
            .iter()
            .map(|channel| channel.can_id)
            .collect::<Vec<_>>(),
        [1100, 1101]
    );
}

pub fn schedule_semantic_rejections() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let mut mutated = inputs();
    let application = mutated
        .iter_mut()
        .find(|source| source.logical_path() == "application.arxml")
        .unwrap();
    *application = InputSource::new(
        "application.arxml",
        fs::read(
            root.join("core/tests/fixtures/epic4/negative/period-alarm-conflict/application.arxml"),
        )
        .unwrap(),
    )
    .unwrap();
    let result =
        inspect_inputs(&mutated, &dependencies).and_then(|input| input.schedule_contract());
    let issues = match result {
        Ok(_) => panic!("Mismatched application/alarm period accepted"),
        Err(issues) => issues,
    };
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "PERIOD_ALARM_CONFLICT"
                && issue.file.as_deref() == Some("application.arxml")
                && issue.object.as_deref()
                    == Some("/Application/EchoApplication/Behavior/Periodic10ms")),
        "{issues:?}"
    );
    let mut mutated = inputs();
    let ecuc = mutated
        .iter_mut()
        .find(|source| source.logical_path() == "ecuc.arxml")
        .unwrap();
    let old = "/RtePositionInTask</DEFINITION-REF>\n                      <VALUE>4</VALUE>";
    let text = std::str::from_utf8(ecuc.bytes()).unwrap();
    assert_eq!(text.matches(old).count(), 1);
    *ecuc = InputSource::new(
        "ecuc.arxml",
        text.replace(
            old,
            "/RtePositionInTask</DEFINITION-REF>\n                      <VALUE>3</VALUE>",
        )
        .into_bytes(),
    )
    .unwrap();
    let result =
        inspect_inputs(&mutated, &dependencies).and_then(|input| input.schedule_contract());
    let issues = match result {
        Ok(_) => panic!("Duplicate task position accepted"),
        Err(issues) => issues,
    };
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "SCHEDULE_NOT_UNIQUE"
                && issue.file.as_deref() == Some("ecuc.arxml")),
        "{issues:?}"
    );
}

pub fn component_semantic_rejections() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let original = inputs();
    let cases: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("core/tests/fixtures/epic4/negative/cases.json")).unwrap(),
    )
    .unwrap();
    let selected = [
        "type-conflict",
        "missing-type-map",
        "multiple-instances",
        "reentrant",
        "unselected-variant",
        "missing-service-client",
        "queued-sr",
        "implicit-access",
        "mode-access",
    ];
    for case in cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| selected.contains(&case["id"].as_str().unwrap()))
    {
        let mut mutated = original.clone();
        for override_path in case["overrides"].as_array().unwrap() {
            let path = root
                .join("core/tests/fixtures/epic4")
                .join(override_path.as_str().unwrap());
            let file = path.file_name().unwrap().to_str().unwrap();
            let slot = mutated
                .iter_mut()
                .find(|source| source.logical_path() == file)
                .unwrap();
            *slot = InputSource::new(file, fs::read(&path).unwrap()).unwrap();
        }
        let result = inspect_inputs(&mutated, &dependencies)
            .and_then(|inspected| inspected.component_contract());
        let issues = match result {
            Ok(_) => panic!("{case} was accepted"),
            Err(issues) => issues,
        };
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == case["category"].as_str().unwrap()
                    && issue.file.as_deref() == case["file"].as_str()
                    && matches!(&issue.remedy, autosar_config_core::message::LocalizedText::Message(message) if message.key.starts_with("backend."))),
            "{case}: {issues:?}"
        );
    }
    let mut renamed = original.clone();
    for source in &mut renamed {
        let text = std::str::from_utf8(source.bytes())
            .unwrap()
            .replace("EchoApplication", "LocalApplication")
            .replace("RxValue", "InputValue")
            .replace("TxValue", "OutputValue");
        *source = InputSource::new(source.logical_path(), text.into_bytes()).unwrap();
    }
    let inspected =
        inspect_inputs(&renamed, &dependencies).unwrap_or_else(|issues| panic!("{issues:?}"));
    let contract = inspected
        .component_contract()
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(contract.component, "/Application/LocalApplication");
    assert_eq!(contract.periodic_symbol, "LocalApplication_Periodic");
    assert_eq!(
        contract
            .data_ports
            .iter()
            .map(|port| port.api_symbol.as_str())
            .collect::<Vec<_>>(),
        ["Rte_Read_InputValue_Value", "Rte_Write_OutputValue_Value"]
    );
}
