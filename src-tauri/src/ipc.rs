//! Test-only generation from the same command list used by the native handler.
use crate::{configuration, delivery, editing, project, settings, verification};
use heck::ToLowerCamelCase;
use std::path::Path;
use tauri_specta::{BuilderConfiguration, LanguageExt};

struct Bindings;

impl LanguageExt for Bindings {
    type Error = specta_typescript::Error;

    fn export(self, config: &BuilderConfiguration, path: &Path) -> Result<(), Self::Error> {
        specta_typescript::Typescript::default().export(config, path)?;
        let mut output = std::fs::read_to_string(path)?;
        output.push_str("\nexport type CommandMap = {\n");
        for command in &config.commands {
            let name = command.name();
            let function = name.to_lower_camel_case();
            output.push_str(&format!("  {name}: {{ args: "));
            if command.args().is_empty() {
                output.push_str("Record<string, never>");
            } else {
                output.push('{');
                for (index, (name, _)) in command.args().iter().enumerate() {
                    output.push_str(&format!(
                        " {}: Parameters<typeof commands.{function}>[{index}];",
                        name.to_lower_camel_case()
                    ));
                }
                output.push('}');
            }
            output.push_str(&format!(
                "; result: Awaited<ReturnType<typeof commands.{function}>> }};\n"
            ));
        }
        output.push_str("};\n");
        std::fs::write(path, output)?;
        Ok(())
    }
}

fn check_bindings(update: bool) {
    let path = std::env::temp_dir().join(format!("autsaro-ipc-{}.ts", std::process::id()));
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(ipc_commands!(tauri_specta::collect_commands))
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
        // Preserve the existing serde_json number representation, including its JS precision limit.
        .dangerously_cast_bigints_to_number()
        .export(Bindings, &path)
        .unwrap();
    let filename = if cfg!(feature = "native-webdriver") {
        "bindings.ts"
    } else {
        "bindings.production.ts"
    };
    let committed = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ui/src/ipc")
        .join(filename);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let status = std::process::Command::new("node")
        .arg(root.join("ui/node_modules/prettier/bin/prettier.cjs"))
        .arg("--config")
        .arg(root.join("ui/.prettierrc.json"))
        .arg("--write")
        .arg(&path)
        .status()
        .expect("install locked UI dependencies with npm ci --prefix ui");
    assert!(status.success(), "format generated IPC declarations");
    let generated = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    if update {
        std::fs::write(&committed, &generated).unwrap();
    }
    assert!(
        std::fs::read(&committed).unwrap() == generated,
        "{} differs from Rust; run npm run ipc:generate --prefix ui",
        committed.display()
    );
}

#[test]
fn ipc_bindings_match_rust() {
    check_bindings(false);
}

#[test]
#[ignore = "explicit generation writes the committed IPC declarations"]
fn update_ipc_bindings_match_rust() {
    check_bindings(true);
}

#[test]
fn serialized_dto_bytes_keep_names_tags_omissions_nulls_and_numbers() {
    fn round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(wire: &str) {
        let value: T = serde_json::from_str(wire).unwrap();
        assert_eq!(serde_json::to_string(&value).unwrap(), wire);
    }
    round_trip::<autosar_config_core::LocalizedText>(
        r#"[{"key":"wire.message","params":{"count":4294967295,"none":null,"raw":"9007199254740993"}},"raw evidence"]"#,
    );
    round_trip::<autosar_config_core::WorkspaceView>(
        r#"{"integrationCandidate":true,"applicationInitialized":false,"name":"wire","files":[{"path":"source.arxml","readonly":true,"retainedCount":1}],"issues":[{"severity":"error","code":"WIRE","message":"raw"}],"dirty":false}"#,
    );
    round_trip::<autosar_config_core::GenerationPreview>(
        r#"{"outputDirectory":"output","revision":"revision","files":[{"path":"source.c","status":"added","before":null,"after":"bytes"}]}"#,
    );
    round_trip::<autosar_config_core::project_model::ChangeSet>(
        r#"{"workspaceEpoch":"epoch","inputFingerprint":"input","definitionFingerprint":"definition","changes":[{"op":"set-value","changeId":"change","field":{"kind":"existing","fieldId":"field"},"expected":{"state":"absent"},"value":{"state":"explicit","value":{"kind":"integer","lexeme":"18446744073709551614"}}},{"op":"set-reference","changeId":"reference","field":{"kind":"new","object":{"kind":"created","changeId":"object"},"definitionId":"definition","entryKey":"entry"},"expected":{"state":"absent"},"value":{"state":"explicit","rawPath":"/Object","dest":"ECUC-CONTAINER-VALUE","target":null}}]}"#,
    );
}
