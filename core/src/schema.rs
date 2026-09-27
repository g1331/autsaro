use crate::model::Issue;
use libxml::parser::{Parser, ParserOptions};
use libxml::schemas::{SchemaParserContext, SchemaValidationContext};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zip::ZipArchive;

pub const SCHEMA_ZIP: &str =
    "docs/official/R24-11/FO/MethodologyAndTemplates/AUTOSAR_FO_MMOD_XMLSchema.zip";
static SCHEMA_LOCK: Mutex<()> = Mutex::new(());

pub fn schema_archive(repo: &Path) -> PathBuf {
    repo.join(SCHEMA_ZIP)
}

fn unpack_schema(archive: &Path) -> Result<PathBuf, String> {
    let file = fs::File::open(archive)
        .map_err(|e| format!("无法打开本地 R24-11 XSD 包 {}: {e}", archive.display()))?;
    let mut zip = ZipArchive::new(file).map_err(|e| format!("XSD 包损坏: {e}"))?;
    let archive_modified = fs::metadata(archive)
        .and_then(|m| m.modified())
        .map_err(|e| e.to_string())?;
    let target = std::env::temp_dir().join(format!("autosar-r24-11-schema-{}", std::process::id()));
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    for name in ["AUTOSAR_00053.xsd", "xml.xsd"] {
        let mut member = zip
            .by_name(name)
            .map_err(|e| format!("XSD 包缺少 {name}: {e}"))?;
        let path = target.join(name);
        if fs::metadata(&path).is_ok_and(|metadata| {
            metadata.len() == member.size()
                && metadata
                    .modified()
                    .is_ok_and(|modified| modified >= archive_modified)
        }) {
            continue;
        }
        let mut bytes = Vec::with_capacity(member.size() as usize);
        member.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        fs::write(&path, bytes).map_err(|e| e.to_string())?;
    }
    Ok(target.join("AUTOSAR_00053.xsd"))
}

pub fn validate_files(archive: &Path, files: &[(&Path, &str)]) -> Result<Vec<Issue>, String> {
    let _validation_guard = SCHEMA_LOCK.lock().map_err(|_| "XSD 校验器状态锁损坏")?;
    let schema_path = unpack_schema(archive)?;
    let mut parser = SchemaParserContext::from_file(&schema_path.to_string_lossy());
    let mut validator = SchemaValidationContext::from_parser(&mut parser).map_err(|errors| {
        format!(
            "AUTOSAR_00053.xsd 无法加载: {}",
            errors
                .iter()
                .map(|e| e.message.as_deref().unwrap_or(""))
                .collect::<Vec<_>>()
                .join("; ")
        )
    })?;
    let xml_parser = Parser::default();
    let mut issues = Vec::new();
    for (path, text) in files {
        if text.contains("<!DOCTYPE") || text.contains("<!ENTITY") {
            issues.push(Issue {
                file: Some(path.display().to_string()),
                ..Issue::error("XML_DTD", "不接受外部实体或 DTD", None)
            });
            continue;
        }
        let doc = match xml_parser.parse_string_with_options(
            text.as_bytes(),
            ParserOptions {
                recover: false,
                no_net: true,
                no_error: false,
                no_warning: false,
                ..ParserOptions::default()
            },
        ) {
            Ok(doc) => doc,
            Err(error) => {
                issues.push(Issue {
                    file: Some(path.display().to_string()),
                    ..Issue::error("XML_PARSE", format!("XML 解析失败: {error}"), None)
                });
                continue;
            }
        };
        if let Err(errors) = validator.validate_document(&doc) {
            for error in errors {
                issues.push(Issue {
                    file: Some(path.display().to_string()),
                    ..Issue::error(
                        "R24_XSD",
                        error.message.as_deref().unwrap_or("").trim().to_owned(),
                        None,
                    )
                });
            }
        }
    }
    Ok(issues)
}
