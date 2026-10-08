//! Product-owned text uses explicit stable message identities; external evidence remains raw.
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;

include!(concat!(env!("OUT_DIR"), "/message_templates.rs"));

const fn template_index(key: &str) -> Option<usize> {
    let key = key.as_bytes();
    let mut first = 0;
    let mut last = ENGLISH_TEMPLATES.len();
    while first < last {
        let middle = first + (last - first) / 2;
        let candidate = ENGLISH_TEMPLATES[middle].0.as_bytes();
        let mut position = 0;
        while position < candidate.len()
            && position < key.len()
            && candidate[position] == key[position]
        {
            position += 1;
        }
        if position == candidate.len() && position == key.len() {
            return Some(middle);
        }
        let less = if position == candidate.len() || position == key.len() {
            candidate.len() < key.len()
        } else {
            candidate[position] < key[position]
        };
        if less {
            first = middle + 1;
        } else {
            last = middle;
        }
    }
    None
}

#[doc(hidden)]
pub const fn contains_message(key: &str) -> bool {
    template_index(key).is_some()
}

fn english_template(key: &str) -> Option<&'static str> {
    template_index(key).map(|index| ENGLISH_TEMPLATES[index].1)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LocalizedText {
    Message(ProductMessage),
    Raw(String),
    Messages(Vec<LocalizedText>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductMessage {
    pub key: Cow<'static, str>,
    pub params: BTreeMap<Cow<'static, str>, serde_json::Value>,
}

impl LocalizedText {
    pub fn product(
        key: &'static str,
        params: BTreeMap<Cow<'static, str>, serde_json::Value>,
    ) -> Self {
        Self::Message(ProductMessage {
            key: Cow::Borrowed(key),
            params,
        })
    }

    pub fn evidence(value: impl Into<String>) -> Self {
        Self::Raw(value.into())
    }

    pub fn messages(values: impl IntoIterator<Item = Self>) -> Self {
        Self::Messages(values.into_iter().collect())
    }
}

// Conversions are for external errors only. Product producers use product_message! explicitly.
impl From<String> for LocalizedText {
    fn from(value: String) -> Self {
        Self::Raw(value)
    }
}
impl From<&str> for LocalizedText {
    fn from(value: &str) -> Self {
        Self::Raw(value.into())
    }
}
impl std::fmt::Display for LocalizedText {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raw(text) => output.write_str(text),
            Self::Messages(messages) => {
                for (index, message) in messages.iter().enumerate() {
                    if index != 0 {
                        output.write_str("\n")?;
                    }
                    write!(output, "{message}")?;
                }
                Ok(())
            }
            Self::Message(message) => {
                let mut remaining = english_template(&message.key).ok_or(std::fmt::Error)?;
                while let Some(start) = remaining.find("{{") {
                    output.write_str(&remaining[..start])?;
                    let placeholder = &remaining[start + 2..];
                    let end = placeholder.find("}}").ok_or(std::fmt::Error)?;
                    let value = message
                        .params
                        .get(&placeholder[..end])
                        .ok_or(std::fmt::Error)?;
                    match value {
                        serde_json::Value::String(value) => output.write_str(value)?,
                        value => write!(output, "{value}")?,
                    }
                    remaining = &placeholder[end + 2..];
                }
                output.write_str(remaining)
            }
        }
    }
}
impl std::error::Error for LocalizedText {}

#[macro_export]
macro_rules! product_message {
    ($key:literal $(, $name:literal => $value:expr)* $(,)?) => {{
        const _: () = assert!($crate::message::contains_message($key), "Missing backend localization key");
        let params = std::collections::BTreeMap::from([
            $((std::borrow::Cow::Borrowed($name), serde_json::Value::String(($value).to_string())),)*
        ]);
        $crate::message::LocalizedText::product($key, params)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_message_keeps_evidence_and_wire_contract() {
        let evidence = "用户/路径/<tool output>";
        let message = crate::product_message!("backend.test.evidence", "evidence" => evidence);
        assert_eq!(
            serde_json::to_value(&message).unwrap(),
            serde_json::json!({"key":"backend.test.evidence", "params":{"evidence":evidence}})
        );
        assert_eq!(message.to_string(), format!("Evidence: {evidence}"));
        assert_eq!(
            serde_json::to_value(LocalizedText::evidence(evidence)).unwrap(),
            evidence
        );
        let restored: LocalizedText =
            serde_json::from_value(serde_json::to_value(&message).unwrap()).unwrap();
        assert_eq!(message, restored);
        assert_eq!(restored.to_string(), format!("Evidence: {evidence}"));
    }
    #[test]
    fn collections_preserve_each_message_identity() {
        let message = crate::product_message!("backend.test.collection");
        let value = serde_json::to_value(LocalizedText::messages([
            message,
            LocalizedText::evidence("tool"),
        ]))
        .unwrap();
        assert_eq!(value[0]["key"], "backend.test.collection");
        assert_eq!(value[1], "tool");
    }

    #[test]
    fn interpolation_does_not_reinterpret_user_text() {
        let message = crate::product_message!(
            "backend.test.evidence",
            "evidence" => "用户/{{other}}",
            "other" => "must not replace evidence",
        );
        assert_eq!(message.to_string(), "Evidence: 用户/{{other}}");
    }
}
