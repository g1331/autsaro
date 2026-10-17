//! Product-authored guidance lives only in messages.json. Catalog consumers share
//! the same message identities, ranges and defaults, including future module docs.
use crate::message::{LocalizedText, ProductMessage, contains_message};

pub(super) fn parameter(name: &str) -> Option<LocalizedText> {
    let key = format!("backend.definitions.description.{name}");
    contains_message(&key).then(|| {
        LocalizedText::Message(ProductMessage {
            key: key.into(),
            params: Default::default(),
        })
    })
}
