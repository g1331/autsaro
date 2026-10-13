//! Product messages keep their identity and parameters on the wire.
use super::*;

#[test]
fn workspace_product_errors_keep_message_identity_on_the_wire() {
    let error = autosar_config_core::Workspace::open(Vec::new())
        .err()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&error).unwrap(),
        serde_json::json!({
            "key": "backend.arxml.input_files_required",
            "params": {}
        })
    );
    assert_eq!(error.to_string(), "Select at least one .arxml file.");
}

#[test]
fn workspace_localized_path_parameters_preserve_user_bytes() {
    let scratch = Scratch::new();
    let path = scratch.0.join("用户-raw-evidence.txt");
    std::fs::write(&path, "<AUTOSAR/>").unwrap();
    let error = autosar_config_core::Workspace::open(vec![path.clone()])
        .err()
        .unwrap();
    let value = serde_json::to_value(&error).unwrap();
    assert_eq!(value["key"], "backend.arxml.unique_arxml_required");
    assert_eq!(
        value["params"]["path"],
        std::fs::canonicalize(&path).unwrap().display().to_string()
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"<AUTOSAR/>");
}

#[test]
fn integration_diagnostics_keep_message_identity_and_source_evidence() {
    use autosar_config_core::integration::InputSource;
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
