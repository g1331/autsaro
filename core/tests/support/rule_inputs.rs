//! ARXML documents and checks shared by rule tests.
use autosar_config_core::rules;
use std::path::Path;

pub(crate) fn document(elements: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\" \
         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
         xsi:schemaLocation=\"http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd\">\
         <AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Original</SHORT-NAME>\
         <ELEMENTS>{elements}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>"
    )
}

pub(crate) fn check(text: &str) -> autosar_config_core::project_model::ScopeValidation {
    rules::validate_native(&[(Path::new("native-input.arxml"), text)]).unwrap()
}
