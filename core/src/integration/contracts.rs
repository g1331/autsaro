use super::component::c_name;
use super::{DiagnosticCategory, PlanDiagnostic, ValidatedIntegrationPlan};
use crate::{GenerationPreview, GenerationReport, generator};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::Path;

fn reserved_identifier(name: &str) -> bool {
    name.starts_with('_') || "auto break case char const continue default do double else enum extern float for goto if inline int long register restrict return short signed sizeof static struct switch typedef union unsigned void volatile while _Bool _Complex _Imaginary"
        .split_whitespace().any(|keyword| keyword == name)
}

/// Deterministic W2 headers and provenance, with no runtime implementations
/// or test stubs. Construction requires a validated integration plan.
pub struct ComponentContractFiles {
    files: Vec<(String, Vec<u8>)>,
}

impl ComponentContractFiles {
    pub fn files(&self) -> &[(String, Vec<u8>)] {
        &self.files
    }

    pub(crate) fn into_files(self) -> Vec<(String, Vec<u8>)> {
        self.files
    }

    /// Inspect all proposed files and existing-output integrity without writing.
    pub fn preview(&self, output: &Path) -> Result<GenerationPreview, String> {
        generator::output::preview_prepared(&self.files, output)
    }

    /// Install exactly the previewed bytes. The existing generator preserves
    /// prior output and refuses stale previews or modified/user-owned files.
    pub fn generate_previewed(
        &self,
        output: &Path,
        revision: &str,
    ) -> Result<GenerationReport, String> {
        generator::output::generate_prepared(self.files.clone(), output, Some(revision))
    }
}

fn header(guard: &str, body: &str) -> Vec<u8> {
    format!(
        "/** @file\n * @brief Generated W2 component contract; runtime implementation is separate.\n */\n#ifndef {guard}\n#define {guard}\n\n{body}\n#endif\n"
    )
    .into_bytes()
}

fn declaration(output: &mut String, brief: &str, prototype: &str, parameters: &str) {
    writeln!(output, "/** @brief {brief}").unwrap();
    output.push_str(parameters);
    output.push_str(" */\n");
    writeln!(output, "{prototype}\n").unwrap();
}

fn reject(plan: &ValidatedIntegrationPlan, message: &str) -> Vec<PlanDiagnostic> {
    let component = &plan.description().component;
    vec![PlanDiagnostic {
        category: DiagnosticCategory::Input,
        code: "CONTRACT_NAME_COLLISION".into(),
        file: plan
            .description()
            .objects
            .iter()
            .find(|object| object.path == component.component)
            .map(|object| object.file.clone()),
        object: Some(component.component.clone()),
        message: message.into(),
        remedy: "Use distinct non-reserved component, type and API names in the input description."
            .into(),
    }]
}

impl ValidatedIntegrationPlan {
    /// Generate the standalone contract from this same checked plan. No XML
    /// is reparsed and no destination is touched until an explicit preview install.
    pub fn component_contract_files(&self) -> Result<ComponentContractFiles, Vec<PlanDiagnostic>> {
        let description = self.description();
        let component = &description.component;
        let application_name = c_name(component.component.rsplit('/').next().unwrap());
        let client = description
            .objects
            .iter()
            .find(|object| object.path == component.service.client_instance)
            .ok_or_else(|| {
                reject(
                    self,
                    "The validated service client instance has no identity.",
                )
            })?;
        // The prototype is the component-scoped API owner; its full identity
        // remains in contract.json, while distinct header names are checked below.
        let service_name = c_name(client.path.rsplit('/').next().unwrap());
        let application_file = format!("include/Rte_{application_name}.h");
        let service_file = format!("include/Rte_{service_name}.h");
        let datatype = c_name(component.service.array_type.rsplit('/').next().unwrap());
        let mut names = BTreeSet::new();
        for name in [&datatype, &application_name, &service_name] {
            if reserved_identifier(name) {
                return Err(reject(
                    self,
                    "A generated identifier would occupy a reserved C namespace.",
                ));
            }
        }
        for name in [
            "include/Std_Types.h",
            "include/Rte_Type.h",
            "include/Rte.h",
            application_file.as_str(),
            service_file.as_str(),
        ] {
            if !names.insert(name.to_uppercase()) {
                return Err(reject(
                    self,
                    "Generated header filenames collide on the target filesystem.",
                ));
            }
        }
        let macros = [
            "RTE_H",
            "RTE_TYPE_H",
            "STD_TYPES_H",
            "E_OK",
            "E_NOT_OK",
            "RTE_E_COM_STOPPED",
            "RTE_E_NEVER_RECEIVED",
            "RTE_E_MAX_AGE_EXCEEDED",
            "uint8",
            "uint32",
            "uint8_t",
            "uint32_t",
            "Std_ReturnType",
        ];
        let app_guard = format!("RTE_{}_H", application_name.to_uppercase());
        let service_guard = format!("RTE_{}_H", service_name.to_uppercase());
        let mut identifiers: BTreeSet<&str> = macros.into_iter().collect();
        for name in [
            app_guard.as_str(),
            service_guard.as_str(),
            datatype.as_str(),
        ] {
            if !identifiers.insert(name) {
                return Err(reject(
                    self,
                    "A type or header guard collides with another generated identifier.",
                ));
            }
        }
        for symbol in &description.symbols {
            if reserved_identifier(&symbol.symbol) || !identifiers.insert(&symbol.symbol) {
                return Err(reject(
                    self,
                    "A generated external symbol collides with a type or header macro.",
                ));
            }
        }
        let mut application = String::from("#include \"Rte.h\"\n\n");
        for port in &component.data_ports {
            if port.read {
                declaration(
                    &mut application,
                    "Read the explicit nonqueued uint32 data element.",
                    &format!("Std_ReturnType {}(uint32 *data);", port.api_symbol),
                    " * @param[out] data Non-NULL pointer to one uint32, including initial/last value.\n * @return E_OK, RTE_E_NEVER_RECEIVED, RTE_E_MAX_AGE_EXCEEDED, or infrastructure error.\n",
                );
            } else {
                declaration(
                    &mut application,
                    "Write the explicit uint32 data element through RTE.",
                    &format!("Std_ReturnType {}(uint32 data);", port.api_symbol),
                    " * @param[in] data Value to submit.\n * @return E_OK on successful submission, or an infrastructure error.\n",
                );
            }
        }
        declaration(
            &mut application,
            "Execute the non-reentrant periodic runnable in its mapped task.",
            &format!("void {}(void);", component.periodic_symbol),
            "",
        );
        declaration(
            &mut application,
            "Encode the initialized application snapshot as four OUT bytes.",
            &format!(
                "Std_ReturnType {}({datatype} Data);",
                component.service.runnable_symbol
            ),
            " * @param[out] Data Non-NULL writable array with capacity four bytes; not retained.\n * @return E_OK; the supported synchronous application server has no application error.\n",
        );
        let mut service = String::from("#include \"Rte.h\"\n\n");
        let symbols: Vec<_> = description
            .symbols
            .iter()
            .filter(|symbol| symbol.declaration_owner == "story-4.11:service contract header")
            .collect();
        if symbols.len() != 2 {
            return Err(reject(
                self,
                "The validated service must have one caller and one synchronous RTE binding.",
            ));
        }
        for symbol in symbols {
            declaration(
                &mut service,
                "Read four snapshot bytes synchronously in the calling task.",
                &format!("Std_ReturnType {}({datatype} Data);", symbol.symbol),
                " * @param[out] Data Non-NULL writable array with capacity four bytes; not retained.\n * @return E_OK or an RTE infrastructure error; no OpStatus, NRC or pending operation.\n",
            );
        }
        let mut files = vec![
            (
                "include/Std_Types.h".into(),
                include_bytes!("../../../runtime/include/Std_Types.h").to_vec(),
            ),
            (
                "include/Rte_Type.h".into(),
                header(
                    "RTE_TYPE_H",
                    &format!(
                        "#include <stdint.h>\n#include \"Std_Types.h\"\n\n/** @brief Eight-bit unsigned application storage. */\ntypedef uint8_t uint8;\n/** @brief Thirty-two-bit unsigned application storage. */\ntypedef uint32_t uint32;\n/** @brief Fixed four-byte synchronous service OUT value. */\ntypedef uint8 {datatype}[4];\n"
                    ),
                ),
            ),
            (
                "include/Rte.h".into(),
                header(
                    "RTE_H",
                    "#include \"Rte_Type.h\"\n\n/** @brief Communication infrastructure is stopped. */\n#define RTE_E_COM_STOPPED 128U\n/** @brief No data has been received since startup. */\n#define RTE_E_NEVER_RECEIVED 133U\n/** @brief Last received data exceeded its configured age. */\n#define RTE_E_MAX_AGE_EXCEEDED 64U\n",
                ),
            ),
            (application_file.clone(), header(&app_guard, &application)),
            (service_file.clone(), header(&service_guard, &service)),
        ];
        let mut ownership = BTreeMap::new();
        for symbol in &description.symbols {
            let owner = match symbol.declaration_owner.as_str() {
                "story-4.11:component contract header" => &application_file,
                "story-4.11:service contract header" => &service_file,
                _ => continue,
            };
            ownership.insert(&symbol.symbol, owner);
        }
        let provenance = serde_json::json!({
            "format": "autosar-component-contract-v1", "profile": description.profile,
            "sources": description.sources, "validationDependencies": description.validation_dependencies,
            "component": component, "declarationOwners": ownership,
            "runtimeImplementation": "Separate W3 ECU integration stage; no stubs are delivered."
        });
        let mut bytes = serde_json::to_vec_pretty(&provenance)
            .map_err(|error| reject(self, &error.to_string()))?;
        bytes.push(b'\n');
        files.push(("contract.json".into(), bytes));
        files.push(("README.md".into(), format!(
            "# Component contract\n\nGenerated from the validated standard input plan. Include `{}` in the application and `{}` in the service consumer. Compile with C99 and this delivery's `include/` directory. Functions have one declaration owner; `Rte.h` supplies shared types and status constants.\n\nThe synchronous service OUT typedef has four writable bytes; its parameter decays to `uint8 *` under C99. Callers must supply that capacity. The application server returns E_OK after filling all four bytes; a service client must detect infrastructure failure. No test stubs or runtime implementations are delivered. Linking and runtime behavior belong to subsequent ECU integration.\n\n`contract.json` records source identities and declaration ownership. `files.list` and `files.sha256` protect generated files during preview and replacement. Keep edited application sources outside this generated contract directory.\n",
            application_file.trim_start_matches("include/"), service_file.trim_start_matches("include/")
        ).into_bytes()));
        Ok(ComponentContractFiles {
            files: generator::output::seal_files(files),
        })
    }
}
