//! Public declarations of the trusted, delivered C99 headers, shared by BSWMD and manuals.
use super::{ContractArgument, PlanDiagnostic};

#[derive(Debug)]
pub(super) struct Entry {
    pub name: String,
    pub result: String,
    pub arguments: Vec<ContractArgument>,
    pub documentation: String,
    pub inline_implementation: bool,
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .enumerate()
            .all(|(i, c)| c == b'_' || c.is_ascii_alphabetic() || (i != 0 && c.is_ascii_digit()))
}

/// Resolve scalar and pointer typedefs from delivered headers, rather than guessing
/// the width of an OS handle or enum from its spelling. Both controlled targets are x64.
pub(super) fn native_aliases(
    headers: &std::collections::BTreeMap<&str, &[u8]>,
) -> Option<std::collections::BTreeMap<String, String>> {
    let mut aliases = std::collections::BTreeMap::new();
    for (_, bytes) in headers.iter().filter(|(path, _)| path.ends_with(".h")) {
        let source = std::str::from_utf8(bytes).ok()?;
        let mut enumeration = false;
        for line in source.lines() {
            if line.trim().starts_with("typedef enum") {
                enumeration = true;
            }
            if let Some(body) = line
                .trim()
                .strip_prefix("typedef ")
                .and_then(|line| line.split_once(';').map(|(body, _)| body))
                && let Some(at) = body.rfind(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                && identifier(&body[at + 1..])
            {
                aliases.insert(body[at + 1..].to_owned(), body[..at + 1].trim().to_owned());
            }
            if enumeration
                && let Some(name) = line
                    .trim()
                    .strip_prefix("} ")
                    .and_then(|line| line.strip_suffix(';'))
                && identifier(name)
            {
                // Public enums use the controlled compiler's 32-bit enum representation.
                aliases.entry(name.into()).or_insert_with(|| "int".into());
                enumeration = false;
            }
        }
    }
    Some(aliases)
}

pub(super) fn native_bits(
    native: &str,
    aliases: &std::collections::BTreeMap<String, String>,
) -> Option<u32> {
    let mut current = native;
    for _ in 0..=aliases.len() {
        if current.contains('*') {
            return Some(64);
        }
        match current {
            // Actor-local ARTI record: x64 address, uint8 validity, uint32 status,
            // uint8 validity, with the controlled ABI's 8-byte struct alignment.
            "Arti_AddressCapture" => return Some(192),
            "boolean" | "uint8" | "uint8_t" | "sint8" | "int8_t" | "char" | "unsigned char" => {
                return Some(8);
            }
            "uint16" | "uint16_t" | "sint16" | "int16_t" => return Some(16),
            "uint32" | "uint32_t" | "sint32" | "int32_t" | "int" => return Some(32),
            "uint64" | "uint64_t" | "sint64" | "int64_t" | "size_t" | "uintptr_t" => {
                return Some(64);
            }
            _ => current = aliases.get(current)?.as_str(),
        }
    }
    None
}

/// This is the deliberately small declaration vocabulary used by our public headers,
/// not a C preprocessor. Function-pointer typedefs, undocumented inline bodies and
/// macro APIs are skipped; documented inline definitions retain their service contract.
/// Unsupported declarations fail closed.
pub(super) fn declarations(source: &str) -> Result<Vec<Entry>, Vec<PlanDiagnostic>> {
    let mut code = String::new();
    let mut comment = String::new();
    let mut entries = Vec::new();
    let mut rest = source;
    let mut depth = 0usize;
    let mut directive = false;
    while !rest.is_empty() {
        if let Some(tail) = rest.strip_prefix("/*") {
            let Some((body, tail)) = tail.split_once("*/") else {
                return Err(PlanDiagnostic::plan_premise("public header comment"));
            };
            if let Some(body) = body.strip_prefix('*') {
                comment = body
                    .lines()
                    .map(|line| line.trim().trim_start_matches('*').trim())
                    .collect::<Vec<_>>()
                    .join("\n");
            }
            code.push(' ');
            rest = tail;
            continue;
        }
        if let Some(tail) = rest.strip_prefix("//") {
            rest = tail.split_once('\n').map_or("", |(_, tail)| tail);
            code.push('\n');
            continue;
        }
        let Some(c) = rest.chars().next() else { break };
        rest = &rest[c.len_utf8()..];
        if directive {
            if c == '\n' && !code.ends_with('\\') {
                directive = false;
                code.clear();
            } else if c != '\r' && c != '\n' {
                code.push(c);
            } else {
                code.clear();
            }
            continue;
        }
        if c == '#' {
            directive = true;
            code.clear();
            continue;
        }
        let inline_definition =
            c == '{' && depth == 0 && code.contains("static inline") && !comment.is_empty();
        if c == '{' {
            // extern "C" wrappers do not hide their public declarations.
            if code.trim() == "extern \"C\"" {
                code.clear();
                continue;
            }
            depth += 1;
        }
        if c == '}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                code.clear();
                comment.clear();
            }
            continue;
        }
        if depth != 0 && !inline_definition {
            continue;
        }
        if c != ';' && !inline_definition {
            code.push(c);
            continue;
        }
        let declaration = code.split_whitespace().collect::<Vec<_>>().join(" ");
        code.clear();
        if declaration.starts_with("typedef ") {
            comment.clear();
            continue;
        }
        let Some((head, parameters)) = declaration.split_once('(') else {
            comment.clear();
            continue;
        };
        let Some(parameters) = parameters.strip_suffix(')') else {
            return Err(PlanDiagnostic::plan_premise(&declaration));
        };
        let Some((result, name)) = head.trim().rsplit_once(' ') else {
            return Err(PlanDiagnostic::plan_premise(&declaration));
        };
        let stars = name.len() - name.trim_start_matches('*').len();
        let name = &name[stars..];
        if !identifier(name) {
            return Err(PlanDiagnostic::plan_premise(&declaration));
        }
        let result = format!("{}{}", result, "*".repeat(stars));
        let result = result
            .trim_start_matches("SCHM_CAN_LINKAGE ")
            .trim_start_matches("SCHM_BSWM_LINKAGE ")
            .trim_start_matches("SCHM_DCM_LINKAGE ")
            .trim_start_matches("OS_MEMMAP ")
            .trim_start_matches("static inline ")
            .trim_start_matches("OS_CODE ")
            .trim_start_matches("CAN_CODE ")
            .trim_start_matches("BSWM_CODE ")
            .trim_start_matches("CANIF_CODE ")
            .trim_start_matches("CANTP_CODE ")
            .trim_start_matches("COM_CODE ")
            .trim_start_matches("COMM_CODE ")
            .trim_start_matches("DCM_CODE ")
            .trim_start_matches("DET_CODE ")
            .trim_start_matches("LSDUR_CODE ")
            .trim_start_matches("PDUR_CODE ")
            .trim_start_matches("ECU_HOSTBUSSM_CODE ")
            .trim_start_matches("RTE_Rte_CODE ")
            .trim_start_matches("RTE_CODE ")
            .trim_start_matches("DET_CALLOUT_CODE ")
            .trim()
            .to_owned();
        let mut arguments = Vec::new();
        if parameters != "void" {
            for argument in parameters.split(',') {
                let argument = argument.trim();
                let at = argument.rfind(|c: char| !c.is_ascii_alphanumeric() && c != '_');
                let (name, native_type) = if let Some(at) = at {
                    let name = argument[at + 1..].trim();
                    if name.is_empty() && argument.ends_with('*') {
                        (format!("argument{}", arguments.len()), argument.to_owned())
                    } else if identifier(name) {
                        (name.to_owned(), argument[..at + 1].trim().to_owned())
                    } else {
                        return Err(PlanDiagnostic::plan_premise(argument));
                    }
                } else if identifier(argument) {
                    (format!("argument{}", arguments.len()), argument.to_owned())
                } else {
                    return Err(PlanDiagnostic::plan_premise(argument));
                };
                let direction = if comment.contains(&format!("@param[in,out] {name}"))
                    || comment.contains(&format!("@param[inout] {name}"))
                {
                    "INOUT"
                } else if comment.contains(&format!("@param[out] {name}")) {
                    "OUT"
                } else {
                    "IN"
                };
                arguments.push(ContractArgument {
                    name,
                    native_type,
                    direction: direction.into(),
                });
            }
        }
        entries.push(Entry {
            inline_implementation: declaration.contains("static inline"),
            name: name.into(),
            result,
            arguments,
            documentation: std::mem::take(&mut comment),
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_declarations_keep_contracts_and_ignore_non_entries() {
        let source = r#"
#ifndef HEADER_H
#define HEADER_H
#ifdef __cplusplus
extern "C" {
#endif
/** Fetch a value.
 * @param[out] Value Borrowed output.
 */
OS_CODE StatusType GetValue(uint8 Id,
                           uint32 *Value);
typedef void (*Callback)(void);
static inline void Local(void) { GetValue(0, 0); }
#define MACRO(x) \
    GetValue(x, 0)
OS_MEMMAP void Stop(void);
#ifdef __cplusplus
}
#endif
#endif
"#;
        let entries = declarations(source).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "GetValue");
        assert_eq!(entries[0].result, "StatusType");
        assert_eq!(entries[0].arguments[1].native_type, "uint32 *");
        assert_eq!(entries[0].arguments[1].direction, "OUT");
        assert!(entries[0].documentation.contains("Fetch a value."));
        assert_eq!(entries[1].name, "Stop");
        assert_eq!(entries[1].result, "void");
        for (linkage, section) in [
            ("SCHM_CAN_LINKAGE", "CAN_CODE"),
            ("SCHM_DCM_LINKAGE", "RTE_Rte_CODE"),
            ("SCHM_BSWM_LINKAGE", "BSWM_CODE"),
        ] {
            let entries = declarations(&format!(
                "/** Poll the module. */\n{linkage} {section} void Poll(void);"
            ))
            .unwrap();
            assert_eq!(entries[0].result, "void");
            assert_eq!(entries[0].name, "Poll");
            assert!(entries[0].documentation.contains("Poll the module."));
        }
        let inline = declarations(
            "/** Read a version. */\nOS_MEMMAP static inline void Version(uint8 *value);\n\
             OS_MEMMAP static inline void Version(uint8 *value) { *value = 1u; }",
        )
        .unwrap();
        assert_eq!(inline.len(), 1);
        assert_eq!(inline[0].name, "Version");
        assert_eq!(inline[0].result, "void");
        assert!(inline[0].inline_implementation);
        assert_eq!(inline[0].arguments[0].native_type, "uint8 *");
        let definition = declarations(
            "/** Read a version.\n * @param[out] value Version output.\n */\n\
             OS_MEMMAP static inline void Version(uint8 *value) { *value = 1u; }",
        )
        .unwrap();
        assert_eq!(definition.len(), 1);
        assert_eq!(definition[0].name, "Version");
        assert_eq!(definition[0].result, "void");
        assert_eq!(definition[0].arguments[0].direction, "OUT");
        assert!(definition[0].inline_implementation);
        assert!(declarations("void Broken(void (*handler)(void));").is_err());
        for module in [
            "BSWM", "CANIF", "CANTP", "COM", "COMM", "DCM", "DET", "LSDUR", "PDUR",
        ] {
            let entries =
                declarations(&format!("{module}_CODE Std_ReturnType Entry(uint8 Value);")).unwrap();
            assert_eq!(entries[0].result, "Std_ReturnType");
            assert_eq!(entries[0].arguments[0].native_type, "uint8");
        }
    }
}
