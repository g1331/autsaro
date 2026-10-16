//! Line layout of generated C under the repository `.clang-format` (#129).
//!
//! Emitters render each text once in the shape clang-format gives it; the
//! formatter never runs during generation (ADR 0014). Only the shapes that
//! generated code uses are modeled, and `test_generated_format.py` checks the
//! generated samples. clang-format weighs several breaks of an over-long
//! function head by penalty; the rules here follow its usual choice.

/// `ColumnLimit` of `.clang-format`.
const COLUMNS: usize = 100;

fn fits(text: &str) -> bool {
    text.lines().all(|line| line.len() <= COLUMNS)
}

/// A function head `{prefix} {name}({params}){end}`; `end` is `;` for a
/// declaration and empty for a definition, whose brace follows on its own line.
/// `sectioned` marks a leading memory-section macro: clang-format does not
/// take it as part of the return type and prefers breaking before the name.
pub(super) fn signature(
    prefix: &str,
    name: &str,
    params: &[String],
    end: &str,
    sectioned: bool,
) -> String {
    let head = format!("{prefix} {name}");
    if let Some(text) = aligned(&head, params, end) {
        return text;
    }
    let alone = format!("{name}({}){end}", params.join(", "));
    if sectioned && fits(&alone) {
        return format!("{prefix}\n{alone}");
    }
    if head.len() < COLUMNS {
        return wrapped("", &head, params, end);
    }
    let rest = aligned(name, params, end).unwrap_or_else(|| wrapped("", name, params, end));
    format!("{prefix}\n{rest}")
}

/// A parameter declaration; a pointer's `*` binds to the name (`uint32 *data`).
pub(super) fn parameter(native_type: &str, name: &str) -> String {
    match native_type.strip_suffix('*') {
        Some(base) => format!("{base}*{name}"),
        None => format!("{native_type} {name}"),
    }
}

/// One line, or one parameter per line aligned after `(` when every line fits.
fn aligned(head: &str, items: &[String], end: &str) -> Option<String> {
    let one = format!("{head}({}){end}", items.join(", "));
    if fits(&one) {
        return Some(one);
    }
    if items.len() < 2 {
        return None;
    }
    let pad = " ".repeat(head.len() + 1);
    let text = format!("{head}({}){end}", items.join(&format!(",\n{pad}")));
    fits(&text).then_some(text)
}

/// Breaks after `(`: all items on one continuation line, or one per line.
fn wrapped(indent: &str, head: &str, items: &[String], end: &str) -> String {
    let next = format!("{indent}    {}){end}", items.join(", "));
    if fits(&next) {
        return format!("{indent}{head}(\n{next}");
    }
    let items = items.join(&format!(",\n{indent}    "));
    format!("{indent}{head}(\n{indent}    {items}){end}")
}

/// A call statement `{indent}{head}({args}){end}`, such as `return f(a, b);`
/// in a body; arguments that do not fit move to a continuation line.
pub(super) fn call(indent: &str, head: &str, args: &[String], end: &str) -> String {
    let one = format!("{indent}{head}({}){end}", args.join(", "));
    if fits(&one) {
        return one;
    }
    wrapped(indent, head, args, end)
}

/// A definition whose body is one statement, or empty; on one line when it fits.
pub(super) fn short_function(head: &str, statement: &str) -> String {
    let body = if statement.is_empty() {
        "{}".to_owned()
    } else {
        format!("{{ {statement} }}")
    };
    let one = format!("{head} {body}");
    if fits(&one) {
        one
    } else if statement.is_empty() {
        format!("{head}\n{{\n}}")
    } else {
        format!("{head}\n{{\n    {statement}\n}}")
    }
}

/// A braced initializer `{declaration} {attribute} = {...};`. It stays on one
/// line when it fits and is no table; otherwise each item takes a line and a
/// trailing comma keeps clang-format from packing them. An item is one line, such as a
/// table row `{a, b}`, or `Row` fields that break the same way when too long.
pub(super) fn initializer(declaration: &str, attribute: &str, items: &[Item]) -> String {
    let head = if attribute.is_empty() {
        declaration.to_owned()
    } else {
        format!("{declaration} {attribute}")
    };
    // A table keeps one row per line even when it would fit on one.
    let table = items.iter().any(|item| matches!(item, Item::Row(_)));
    let one = format!("{head} = {};", inline(items));
    if fits(&one) && !table {
        return one;
    }
    if head.len() + " = {".len() <= COLUMNS || attribute.is_empty() {
        return format!("{head} = {{\n{}}};", lines("    ", items));
    }
    // clang-format breaks before the section attribute of an over-long head.
    let continued = format!("{declaration}\n    {attribute} = {};", inline(items));
    if fits(&continued) && !table {
        return continued;
    }
    format!(
        "{declaration}\n    {attribute} = {{\n{}}};",
        lines("        ", items)
    )
}

/// One initializer item.
pub(super) enum Item {
    Value(String),
    Row(Vec<String>),
}

impl Item {
    fn inline(&self) -> String {
        match self {
            Item::Value(value) => value.clone(),
            Item::Row(fields) => format!("{{{}}}", fields.join(", ")),
        }
    }
}

fn inline(items: &[Item]) -> String {
    format!(
        "{{{}}}",
        items
            .iter()
            .map(Item::inline)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn lines(indent: &str, items: &[Item]) -> String {
    let mut text = String::new();
    for item in items {
        let line = format!("{indent}{},", item.inline());
        match item {
            Item::Row(fields) if !fits(&line) => {
                text.push_str(&format!("{indent}{{\n"));
                for field in fields {
                    text.push_str(&format!("{indent}    {field},\n"));
                }
                text.push_str(&format!("{indent}}},\n"));
            }
            _ => text.push_str(&format!("{line}\n")),
        }
    }
    text
}

/// A preprocessor disjunction; long conditions retain explicit continuation lines.
pub(super) fn condition(directive: &str, clauses: &[String]) -> String {
    let line = format!("#{directive} {}", clauses.join(" || "));
    if fits(&line) {
        return format!("{line}\n");
    }
    let mut output = String::new();
    let mut line = format!("#{directive} ");
    for clause in clauses {
        if line.ends_with(' ') {
            line.push_str(clause);
        } else if line.len() + 4 + clause.len() <= COLUMNS {
            line.push_str(&format!(" || {clause}"));
        } else {
            output.push_str(&format!(
                "{:<width$}\\\n",
                format!("{line} ||"),
                width = COLUMNS - 1
            ));
            line = format!("    {clause}");
        }
    }
    output.push_str(&format!("{line}\n"));
    output
}

/// Consecutive `#define name value` lines, each ending with a newline. Values
/// align in runs (`AlignConsecutiveMacros: Consecutive`); a definition too long
/// for one line continues after a backslash and ends the run, as does a run
/// whose alignment would pass the column limit.
pub(super) fn defines<N: AsRef<str>, V: AsRef<str>>(entries: &[(N, V)]) -> String {
    let mut text = String::new();
    let mut run: Vec<(&str, &str)> = Vec::new();
    for (name, value) in entries {
        let (name, value) = (name.as_ref(), value.as_ref());
        if format!("#define {name} {value}").len() > COLUMNS {
            flush(&mut text, &mut run);
            text.push_str(&format!(
                "{:<width$}\\\n    {value}\n",
                format!("#define {name}"),
                width = COLUMNS - 1
            ));
            continue;
        }
        let width = run.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
        let room = run.iter().map(|(_, value)| value.len()).max().unwrap_or(0);
        if "#define ".len() + width.max(name.len()) + 1 + room.max(value.len()) > COLUMNS {
            flush(&mut text, &mut run);
        }
        run.push((name, value));
    }
    flush(&mut text, &mut run);
    text
}

fn flush(text: &mut String, run: &mut Vec<(&str, &str)>) {
    let width = run.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
    for (name, value) in run.drain(..) {
        text.push_str(&format!("#define {name:<width$} {value}\n"));
    }
}

/// A comment line `{prefix}{text}` wrapped at spaces like `ReflowComments`,
/// each continuation repeating `prefix`; ends with a newline.
pub(super) fn comment(prefix: &str, text: &str) -> String {
    let mut output = String::new();
    let mut line = prefix.to_owned();
    for word in text.split(' ') {
        if line.len() > prefix.len() && line.len() + 1 + word.len() > COLUMNS {
            output.push_str(&format!("{line}\n"));
            line = prefix.to_owned();
        } else if line.len() > prefix.len() {
            line.push(' ');
        }
        line.push_str(word);
    }
    output.push_str(&format!("{line}\n"));
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn signatures_follow_clang_format_breaks() {
        let params = strings(&["CounterType CounterID", "TimeInMicrosecondsType *Value"]);
        assert_eq!(
            signature(
                "StatusType",
                "OsService_GetCounterValue",
                &params,
                ";",
                false
            ),
            "StatusType OsService_GetCounterValue(CounterType CounterID, TimeInMicrosecondsType *Value);"
        );
        assert_eq!(
            signature(
                "RTE_CODE StatusType",
                "OsService_SystemCounter_GetCounterValue",
                &params,
                "",
                true
            ),
            "RTE_CODE StatusType OsService_SystemCounter_GetCounterValue(CounterType CounterID,\n                                                            TimeInMicrosecondsType *Value)"
        );
        let mode = strings(&["Rte_ModeType_DcmAuthenticationState_Physical mode"]);
        let name = "SchM_Switch_Dcm_DcmAuthenticationState_Physical";
        assert_eq!(
            signature("Std_ReturnType", name, &mode, ";", false),
            format!("Std_ReturnType {name}(\n    {});", mode[0])
        );
        assert_eq!(
            signature("RTE_CODE Std_ReturnType", name, &mode, "", true),
            format!("RTE_CODE Std_ReturnType\n{name}({})", mode[0])
        );
    }

    #[test]
    fn initializers_keep_short_lists_and_break_long_ones() {
        let items = [Item::Value("0u".into()), Item::Value("0u".into())];
        assert_eq!(
            initializer("static const T x", "", &items),
            "static const T x = {0u, 0u};"
        );
        let head = "static const Ecu_HostBusSM_ConfigType bus_config";
        let attribute = "RTE_Ecu_HostBusSM_CONFIG_DATA_PREBUILD_UNSPECIFIED";
        assert_eq!(
            initializer(head, attribute, &items),
            format!("{head}\n    {attribute} = {{0u, 0u}};")
        );
        let long: Vec<_> = (0..30)
            .map(|value| Item::Value(format!("{value}u")))
            .collect();
        let text = initializer("const T x", "", &long);
        assert!(text.starts_with("const T x = {\n    0u,\n    1u,\n"));
        assert!(text.ends_with("    29u,\n};"));
        let row = Item::Row((0..30).map(|value| format!("{value}u")).collect());
        assert!(initializer("const T x[]", "", &[row]).contains("    {\n        0u,\n"));
    }

    #[test]
    fn defines_align_runs_and_wrap_long_values() {
        assert_eq!(
            defines(&[("RTE_E_OK", "0U"), ("RTE_E_LIMIT", "130U")]),
            "#define RTE_E_OK    0U\n#define RTE_E_LIMIT 130U\n"
        );
        let wrapped = defines(&[("A", "x".repeat(95))]);
        let first = wrapped.lines().next().unwrap_or_default();
        assert_eq!((first.len(), first.ends_with('\\')), (COLUMNS, true));
        let name = "N".repeat(70);
        let value = "V".repeat(80);
        assert_eq!(
            defines(&[(name.as_str(), "1"), ("M", value.as_str())]),
            format!("#define {name} 1\n#define M {value}\n")
        );
    }

    #[test]
    fn comments_wrap_at_the_last_fitting_space() {
        let text = "word ".repeat(30);
        let wrapped = comment(" * ", text.trim_end());
        assert!(
            wrapped
                .lines()
                .all(|line| line.len() <= COLUMNS && line.starts_with(" * "))
        );
        assert_eq!(wrapped.lines().count(), 2);
    }
}
