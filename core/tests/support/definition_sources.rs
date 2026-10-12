//! Definition documents shared by the default and official tests.

pub(crate) fn document(elements: &str) -> String {
    format!(
        "<AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Project</SHORT-NAME><ELEMENTS>{elements}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>"
    )
}
