use crate::model::Issue;
use libxml::parser::{Parser, ParserOptions};
use libxml::schemas::{SchemaParserContext, SchemaValidationContext};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::ZipArchive;

pub const SCHEMA_ZIP: &str =
    "docs/official/R24-11/FO/MethodologyAndTemplates/AUTOSAR_FO_MMOD_XMLSchema.zip";
pub const MOD_ZIP: &str =
    "docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_MOD_ECUConfigurationParameters.zip";
pub const SAMPLE_ZIP: &str =
    "docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_EXP_ModelingShowCases.zip";
pub const XSD_SHA256: &str = "9db3ab1d2ec4db7cc8ff09f1259ff93a7a5945a9500d4cd3ea4a7090f2a25766";
static NEXT_SCHEMA_DIR: AtomicU64 = AtomicU64::new(0);

pub fn schema_archive(repo: &Path) -> PathBuf {
    reference_archive(repo, "AUTOSAR_XSD_ARCHIVE", SCHEMA_ZIP)
}

/// Locate development reference inputs using the same overrides as the doctor.
/// Builtin product rules do not use these archives.
pub fn reference_archive(repo: &Path, variable: &str, relative: &str) -> PathBuf {
    let selected = std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(relative));
    if selected.is_absolute() {
        selected
    } else {
        repo.join(selected)
    }
}
struct PrivateSchemaDirectory(PathBuf);

impl PrivateSchemaDirectory {
    fn create() -> Result<Self, String> {
        for _ in 0..16 {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| error.to_string())?
                .as_nanos();
            let target = std::env::temp_dir().join(format!(
                "autosar-r24-11-schema-{}-{nonce}-{}",
                std::process::id(),
                NEXT_SCHEMA_DIR.fetch_add(1, Ordering::Relaxed)
            ));
            #[cfg(unix)]
            let mut builder = fs::DirBuilder::new();
            #[cfg(not(unix))]
            let builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&target) {
                Ok(()) => return Ok(Self(target)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.to_string()),
            }
        }
        Err("无法创建唯一的 XSD 校验临时目录".into())
    }

    fn schema_path(&self) -> PathBuf {
        self.0.join("AUTOSAR_00053.xsd")
    }
}

impl Drop for PrivateSchemaDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn unpack_schema(archive: &Path) -> Result<PrivateSchemaDirectory, String> {
    let mut file = fs::File::open(archive)
        .map_err(|error| format!("无法打开本地 R24-11 XSD 包 {}: {error}", archive.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    if format!("{:x}", digest.finalize()) != XSD_SHA256 {
        return Err("R24-11 XSD 包与固定内容身份不符".into());
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| error.to_string())?;
    let mut zip = ZipArchive::new(file).map_err(|error| format!("XSD 包损坏: {error}"))?;
    const ALLOWED: [&str; 5] = [
        "_disclaimer.txt",
        "_readme.txt",
        "AUTOSAR_00053.xsd",
        "autosar.soc",
        "xml.xsd",
    ];
    for index in 0..zip.len() {
        let member = zip.by_index(index).map_err(|error| error.to_string())?;
        if !ALLOWED.contains(&member.name())
            || member.is_dir()
            || member
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(format!("XSD 包包含不允许的路径或链接: {}", member.name()));
        }
    }
    let target = PrivateSchemaDirectory::create()?;
    for name in ["AUTOSAR_00053.xsd", "xml.xsd"] {
        let mut member = zip
            .by_name(name)
            .map_err(|error| format!("XSD 包缺少 {name}: {error}"))?;
        let mut output =
            fs::File::create(target.0.join(name)).map_err(|error| error.to_string())?;
        std::io::copy(&mut member, &mut output).map_err(|error| error.to_string())?;
    }
    Ok(target)
}

pub fn validate_files(archive: &Path, files: &[(&Path, &str)]) -> Result<Vec<Issue>, String> {
    let directory = unpack_schema(archive)?;
    let schema_path = directory.schema_path();
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

/// Product-authored offline checks, explicitly separate from the pinned oracle
/// and the legacy XSD identity used by validate_files.
pub fn validate_native_files(
    files: &[(&Path, &str)],
) -> Result<crate::project_model::ScopeValidation, String> {
    crate::rules::validate_native(files)
}
