// Shared support modules serve different test layers; only registered tests run.
#![allow(dead_code)]

// Product-authored configuration tests run without official archives or a C compiler.
use workspace::Scratch;
#[cfg(feature = "official-oracles")]
use workspace::archive;

#[path = "support/epic7_definitions.rs"]
mod epic7_definitions;
#[path = "support/epic7_delivery.rs"]
mod epic7_delivery;
#[path = "support/epic7_rules.rs"]
mod epic7_rules;
#[path = "support/epic7_workspace.rs"]
mod epic7_workspace;
#[path = "support/tooling.rs"]
mod tooling;
#[path = "support/workspace.rs"]
mod workspace;

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
fn model_diagnostics_keep_machine_codes_and_paths_outside_product_prose() {
    let frame = autosar_config_core::FrameView {
        path: "/用户/Frame".into(),
        name: "Frame".into(),
        id: 0x800,
        dlc: 8,
        direction: autosar_config_core::Direction::Tx,
        period_ms: Some(10),
        timeout_ms: None,
    };
    let issues = autosar_config_core::model::validate_profile(&[frame], &[]);
    let issue = issues
        .iter()
        .find(|issue| issue.code == "CAN_FRAME_RANGE")
        .unwrap();
    let value = serde_json::to_value(issue).unwrap();
    assert_eq!(value["code"], "CAN_FRAME_RANGE");
    assert_eq!(value["path"], "/用户/Frame");
    assert_eq!(value["message"]["key"], "backend.model.can_frame_range");
    assert!(value["message"]["params"].as_object().unwrap().is_empty());
}
