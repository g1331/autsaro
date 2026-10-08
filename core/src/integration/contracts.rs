use super::component::c_name;
use super::{DiagnosticCategory, PlanDiagnostic, ValidatedIntegrationPlan};
use crate::{GenerationPreview, GenerationReport, generator};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::Path;

pub(super) fn reserved_identifier(name: &str) -> bool {
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
    pub fn preview(
        &self,
        output: &Path,
    ) -> Result<GenerationPreview, crate::message::LocalizedText> {
        generator::output::preview_prepared(&self.files, output)
    }

    /// Install exactly the previewed bytes. The existing generator preserves
    /// prior output and refuses stale previews or modified/user-owned files.
    pub fn generate_previewed(
        &self,
        output: &Path,
        revision: &str,
    ) -> Result<GenerationReport, crate::message::LocalizedText> {
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

fn reject(
    plan: &ValidatedIntegrationPlan,
    message: crate::message::LocalizedText,
) -> Vec<PlanDiagnostic> {
    let description = plan.description();
    let object = description
        .component
        .as_ref()
        .map(|component| component.component.as_str())
        .or_else(|| {
            description
                .multi
                .as_ref()
                .map(|multi| multi.composition.as_str())
        })
        .unwrap_or("/");
    vec![PlanDiagnostic {
        category: DiagnosticCategory::Input,
        code: "CONTRACT_NAME_COLLISION".into(),
        file: plan
            .description()
            .objects
            .iter()
            .find(|identity| identity.path == object)
            .map(|object| object.file.clone()),
        object: Some(object.into()),
        message: message.into(),
        remedy: crate::product_message!(
            "backend.integration.contracts.distinct_nonreserved_names_required"
        )
        .into(),
    }]
}

impl ValidatedIntegrationPlan {
    /// Generate the standalone contract from this same checked plan. No XML
    /// is reparsed and no destination is touched until an explicit preview install.
    pub fn component_contract_files(&self) -> Result<ComponentContractFiles, Vec<PlanDiagnostic>> {
        let description = self.description();
        if let Some(multi) = &description.multi {
            return self.multi_contract_files(multi);
        }
        let component = description
            .legacy_component()
            .map_err(|message| reject(self, message))?;
        let application_name = c_name(component.component.rsplit('/').next().unwrap());
        let client = description
            .objects
            .iter()
            .find(|object| object.path == component.service.client_instance)
            .ok_or_else(|| {
                reject(
                    self,
                    crate::product_message!(
                        "backend.integration.contracts.service_client_identity_missing"
                    ),
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
                    crate::product_message!(
                        "backend.integration.contracts.generated_identifier_reserved_namespace"
                    ),
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
                    crate::product_message!(
                        "backend.integration.contracts.generated_header_filename_collision"
                    ),
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
                    crate::product_message!(
                        "backend.integration.contracts.type_or_header_guard_collision"
                    ),
                ));
            }
        }
        for symbol in &description.symbols {
            if reserved_identifier(&symbol.symbol) || !identifiers.insert(&symbol.symbol) {
                return Err(reject(
                    self,
                    crate::product_message!(
                        "backend.integration.contracts.generated_external_symbol_collision"
                    ),
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
                crate::product_message!(
                    "backend.integration.contracts.service_caller_rte_binding_required"
                ),
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
            .map_err(|error| reject(self, error.to_string().into()))?;
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

impl ValidatedIntegrationPlan {
    fn multi_contract_files(
        &self,
        multi: &super::multi::MultiComponentContract,
    ) -> Result<ComponentContractFiles, Vec<PlanDiagnostic>> {
        let description = self.description();
        let mut types = String::from(
            "#include <stdint.h>\n#include \"Std_Types.h\"\n\ntypedef uint8_t uint8;\ntypedef uint32_t uint32;\n",
        );
        for datatype in multi.array_types.values() {
            writeln!(types, "typedef uint8 {datatype}[4];").unwrap();
        }
        let mut files = vec![
            (
                "include/Std_Types.h".into(),
                include_bytes!("../../../runtime/include/Std_Types.h").to_vec(),
            ),
            ("include/Rte_Type.h".into(), header("RTE_TYPE_H", &types)),
            (
                "include/Rte.h".into(),
                header(
                    "RTE_H",
                    "#include \"Rte_Type.h\"\n\n#define RTE_E_COM_STOPPED 128U\n#define RTE_E_NEVER_RECEIVED 133U\n#define RTE_E_MAX_AGE_EXCEEDED 64U\n",
                ),
            ),
        ];
        for component in &multi.components {
            let mut body = String::from("#include \"Rte.h\"\n\n");
            for symbol in description
                .symbols
                .iter()
                .filter(|symbol| symbol.declaration_owner == component.header)
            {
                let parameters = if symbol.arguments.is_empty() {
                    "void".into()
                } else {
                    symbol
                        .arguments
                        .iter()
                        .map(|argument| format!("{} {}", argument.native_type, argument.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let mut docs = String::new();
                for argument in &symbol.arguments {
                    writeln!(
                        docs,
                        " * @param[{}] {} {}",
                        argument.direction.to_lowercase(),
                        argument.name,
                        if argument.native_type.contains('*')
                            || multi
                                .array_types
                                .values()
                                .any(|native| native == &argument.native_type)
                        {
                            "Non-NULL caller-owned storage; not retained."
                        } else {
                            "Input value."
                        }
                    )
                    .unwrap();
                }
                if symbol.return_type != "void" {
                    let port = component
                        .data_ports
                        .iter()
                        .find(|port| super::multi::data_symbol(component, port) == symbol.symbol);
                    let status = if let Some(port) = port {
                        let network = multi.network_endpoints.iter().any(|endpoint| {
                            endpoint.endpoint.instance == component.instance
                                && endpoint.endpoint.port == port.path
                        });
                        if network && port.read {
                            "Planned E_OK, RTE_E_NEVER_RECEIVED or RTE_E_MAX_AGE_EXCEEDED with configured initial/last value; COM_SERVICE_NOT_AVAILABLE maps to RTE_E_COM_STOPPED. Network timeout runtime scope is pending.".to_owned()
                        } else if network {
                            "Planned E_OK for buffer update; COM_SERVICE_NOT_AVAILABLE maps to RTE_E_COM_STOPPED; lower transmission failure is separate.".to_owned()
                        } else if port.read {
                            format!(
                                "Planned E_OK; this receiver reads its explicit initial value {} until the first publication, then the producer's last value.",
                                port.initial_value
                            )
                        } else {
                            "Planned E_OK on publication; independent of CAN availability."
                                .to_owned()
                        }
                    } else {
                        "Planned E_OK or an RTE infrastructure error for the synchronous client."
                            .to_owned()
                    };
                    writeln!(docs, " * @return {status} Declaration only; runtime behavior is not implemented here.").unwrap();
                }
                declaration(
                    &mut body,
                    "Component contract in the configured single-owner task.",
                    &format!("{} {}({parameters});", symbol.return_type, symbol.symbol),
                    &docs,
                );
            }
            for port in &component.data_ports {
                writeln!(
                    body,
                    "#define {} {}",
                    port.api_symbol,
                    super::multi::data_symbol(component, port)
                )
                .unwrap();
            }
            for operation in component
                .operations
                .iter()
                .filter(|operation| operation.read)
            {
                writeln!(
                    body,
                    "#define {} {}",
                    operation.api_symbol, operation.implementation_symbol
                )
                .unwrap();
            }
            let name = component
                .header
                .trim_start_matches("include/Rte_")
                .trim_end_matches(".h");
            files.push((
                component.header.clone(),
                header(&format!("RTE_{}_H", name.to_uppercase()), &body),
            ));
        }
        let provenance = serde_json::json!({ "format": "autosar-component-contract-v1", "profile": description.profile,
            "sources": description.sources, "validationDependencies": description.validation_dependencies,
            "ruleSetIdentity": description.rule_set_identity, "requiredExtensionDefinitions": description.required_extension_definitions,
            "multi": multi, "schedule": description.schedule,
            "symbols": description.symbols,
            "declarationOwners": description.symbols.iter().filter(|symbol| symbol.declaration_owner.starts_with("include/Rte_")).map(|symbol| (&symbol.symbol, &symbol.declaration_owner)).collect::<BTreeMap<_, _>>() });
        let mut bytes = serde_json::to_vec_pretty(&provenance)
            .map_err(|error| reject(self, error.to_string().into()))?;
        bytes.push(b'\n');
        files.push(("contract.json".into(), bytes));
        files.push(("README.md".into(), b"# Component contracts\n\nInclude the generated header for your component in each application translation unit. Standard component API macros select unique implementation symbols. Shared application types are declared in Rte_Type.h. Synchronous servers with no application errors return void; their RTE clients return Std_ReturnType. OUT and INOUT arguments require non-NULL writable caller-owned storage, and fixed byte arrays require four bytes.\n\nThis delivery contains component-level declarations and selected relationships from the validated source-derived plan, not the complete ECU plan. contract.json records component, endpoint, type, selected mapping/schedule, symbol ownership and validation-source identities. Runtime implementation and OS execution are separate stages. Planned local Read returns each receiver's explicit initial value until publication and E_OK; local Write returns E_OK. Planned network Read retains initial/last values and reports E_OK, RTE_E_NEVER_RECEIVED or RTE_E_MAX_AGE_EXCEEDED; network Read/Write map COM_SERVICE_NOT_AVAILABLE to RTE_E_COM_STOPPED. Planned synchronous Call returns E_OK or an RTE infrastructure error. No runtime behavior is implemented here, and the network timeout/COM deadline-monitoring runtime scope remains pending. Keep user sources outside this generated directory.\n".to_vec()));
        Ok(ComponentContractFiles {
            files: generator::output::seal_files(files),
        })
    }
}
