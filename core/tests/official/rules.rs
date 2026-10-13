//! Built-in schema checks compared with the official R24-11 XSD.
use super::*;
use crate::support::rule_inputs::*;
use autosar_config_core::project_model::ValidationStatus;
#[cfg(feature = "official-oracles")]
use autosar_config_core::schema;
use std::path::Path;

#[cfg(feature = "official-oracles")]
#[test]
fn native_schema_oracle_agrees_on_independent_positive_and_negative_examples() {
    let legal = document("<I-SIGNAL><SHORT-NAME>Signal</SHORT-NAME><LENGTH>8</LENGTH></I-SIGNAL>");
    let file = Path::new("oracle-comparison.arxml");
    assert!(
        schema::validate_files(&archive(), &[(file, &legal)])
            .unwrap()
            .is_empty()
    );
    assert_eq!(check(&legal).status, ValidationStatus::Passed);
    let negatives = [
        legal.replace("<SHORT-NAME>Signal</SHORT-NAME>", ""),
        legal.replace(
            "<SHORT-NAME>Signal</SHORT-NAME><LENGTH>8</LENGTH>",
            "<LENGTH>8</LENGTH><SHORT-NAME>Signal</SHORT-NAME>",
        ),
        legal.replace("<LENGTH>8</LENGTH>", "<LENGTH>8</LENGTH><LENGTH>9</LENGTH>"),
        legal.replace("<LENGTH>8</LENGTH>", "<LENGTH>invalid</LENGTH>"),
    ];
    for text in negatives {
        assert!(
            !schema::validate_files(&archive(), &[(file, &text)])
                .unwrap()
                .is_empty()
        );
        assert_eq!(check(&text).status, ValidationStatus::Failed);
    }
    let legal_unknown = document(
        "<COMPU-METHOD><SHORT-NAME>OriginalScale</SHORT-NAME><CATEGORY>LINEAR</CATEGORY></COMPU-METHOD>",
    );
    assert!(
        schema::validate_files(&archive(), &[(file, &legal_unknown)])
            .unwrap()
            .is_empty()
    );
    assert_eq!(check(&legal_unknown).status, ValidationStatus::Unsupported);
}
