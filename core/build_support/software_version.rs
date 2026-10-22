//! Read the three literal uint8 fields owned by a delivered software-version header.
pub fn read(source: &str, prefix: &str) -> Result<String, String> {
    let mut values = Vec::new();
    for part in ["MAJOR", "MINOR", "PATCH"] {
        let name = format!("{prefix}_SW_{part}_VERSION");
        let definitions: Vec<_> = source
            .lines()
            .map(|line| line.split_whitespace().collect::<Vec<_>>())
            .filter(|tokens| {
                tokens.first() == Some(&"#define") && tokens.get(1) == Some(&name.as_str())
            })
            .collect();
        let value = match definitions.as_slice() {
            [tokens] if tokens.len() == 3 => tokens[2]
                .strip_suffix('U')
                .filter(|text| {
                    !text.is_empty()
                        && (text.len() == 1 || !text.starts_with('0'))
                        && text.bytes().all(|byte| byte.is_ascii_digit())
                })
                .and_then(|text| text.parse::<u8>().ok()),
            _ => None,
        }
        .ok_or_else(|| format!("{name}: expected one literal uint8 definition with U suffix"))?;
        values.push(value.to_string());
    }
    Ok(values.join("."))
}
