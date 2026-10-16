use super::c_layout;
use super::{DiagnosticCategory, PlanDiagnostic, ValidatedIntegrationPlan};
use crate::{GenerationPreview, GenerationReport, generator};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn reserved_identifier(name: &str) -> bool {
    name.starts_with('_') || "auto break case char const continue default do double else enum extern float for goto if inline int long register restrict return short signed sizeof static struct switch typedef union unsigned void volatile while _Bool _Complex _Imaginary"
        .split_whitespace().any(|keyword| keyword == name)
}

/// Deterministic component artifacts from a validated integration plan.
/// Contract generation provides declarations; runtime generation also supplies
/// actual RTE producers. Neither entry generates application algorithms.
pub struct ComponentContractFiles {
    files: Vec<(String, Vec<u8>)>,
}

impl ComponentContractFiles {
    pub fn files(&self) -> &[(String, Vec<u8>)] {
        &self.files
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
        "/** @file\n * @brief Generated component contract; runtime implementation is separate.\n */\n#ifndef {guard}\n#define {guard}\n\n{body}\n#endif\n"
    )
    .into_bytes()
}

fn declaration(output: &mut String, brief: &str, prototype: &str, parameters: &str) {
    output.push_str(&format!("/** @brief {brief}\n"));
    output.push_str(parameters);
    output.push_str(" */\n");
    output.push_str(&format!("{prototype}\n\n"));
}

fn reject(
    plan: &ValidatedIntegrationPlan,
    message: crate::message::LocalizedText,
) -> Vec<PlanDiagnostic> {
    let description = plan.description();
    let object = description
        .composition
        .as_ref()
        .map(|composition| composition.composition.as_str())
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
        message,
        remedy: crate::product_message!(
            "backend.integration.contracts.distinct_nonreserved_names_required"
        ),
    }]
}

impl ValidatedIntegrationPlan {
    /// Generate the standalone contract from this same checked plan. No XML
    /// is reparsed and no destination is touched until an explicit preview install.
    pub fn component_contract_files(&self) -> Result<ComponentContractFiles, Vec<PlanDiagnostic>> {
        let composition = self.description().composition.as_ref().ok_or_else(|| {
            reject(
                self,
                crate::product_message!("backend.integration.composition.consumer_unsupported"),
            )
        })?;
        self.composition_contract_files(composition)
    }
}

impl ValidatedIntegrationPlan {
    fn composition_contract_files(
        &self,
        composition: &super::composition::CompositionContract,
    ) -> Result<ComponentContractFiles, Vec<PlanDiagnostic>> {
        let mut files = Vec::new();
        // The same platform and standard types the delivered ECU compiles with.
        for name in ["Platform_Types.h", "Std_Types.h", "ComStack_Types.h"] {
            let asset = super::outputs::trusted_asset(&format!("runtime/bsw/include/{name}"))?;
            files.push((format!("include/{name}"), asset.bytes.to_vec()));
        }
        files.extend(super::can_runtime::type_headers(
            self.description().ecu_communication()?,
        ));
        files.extend(self.contract_headers(composition)?);
        files.push((
            "include/Rte_Dcm_Type.h".into(),
            self.dcm_type_header(composition)?.into_bytes(),
        ));
        files.push(("README.md".into(), b"# Component contracts\n\nInclude the generated header for your component in each application translation unit. Standard component API macros select unique implementation symbols. Shared application types are declared in Rte_Type.h. Synchronous servers with no application errors return void; their RTE clients return Std_ReturnType. OUT and INOUT arguments require non-NULL writable caller-owned storage, and fixed byte arrays require four bytes.\n\nThis delivery contains component-level declarations and selected relationships from the validated source-derived plan, not the complete ECU plan. contract.json records component, endpoint, type, selected mapping/schedule, symbol ownership and validation-source identities. Runtime implementation and OS execution are separate stages. Local Read retains each receiver's initial value until publication; handleNeverReceived selects whether it reports RTE_E_NEVER_RECEIVED before that publication. Unconnected Read returns its initial value and RTE_E_UNCONNECTED; unconnected Write discards and succeeds; unconnected Call preserves OUT/INOUT and returns RTE_E_UNCONNECTED. Network Read retains initial/last values and reports reception/age status; COM_SERVICE_NOT_AVAILABLE maps to RTE_E_COM_STOPPED. This contract kit contains declarations; generate the ECU project to obtain these runtime implementations. Keep user sources outside this generated directory.\n".to_vec()));
        Ok(ComponentContractFiles {
            files: generator::output::seal_files(files),
        })
    }

    /// The same trusted Dcm type producer is imported by both delivery stages.
    pub(super) fn dcm_type_header(
        &self,
        composition: &super::composition::CompositionContract,
    ) -> Result<String, Vec<PlanDiagnostic>> {
        let group = &composition.authentication_modes()?.group;
        let declarations = self
            .description()
            .diagnostic
            .as_ref()
            .map(|diagnostic| composition.diagnostic_types(diagnostic))
            .transpose()?
            .map_or(String::new(), |types| types.0);
        let addition = format!(
            "{declarations}typedef uint8 Rte_ModeType_{group};\n{}",
            c_layout::defines(&[
                (format!("RTE_MODE_{group}_DCM_DEAUTHENTICATED"), "0u"),
                (format!("RTE_MODE_{group}_DCM_AUTHENTICATED"), "1u"),
                (format!("RTE_TRANSITION_{group}"), "255u"),
            ])
        );
        let asset = super::outputs::trusted_asset("runtime/bsw/include/Rte_Dcm_Type.h")?;
        super::outputs::render(
            &String::from_utf8_lossy(asset.bytes),
            "include/Rte_Dcm_Type.h",
            &[
                super::outputs::Edit::Once(
                    "\"Std_Types.h\"",
                    "\"Std_Types.h\"\n#include \"Rte_Type.h\"",
                ),
                super::outputs::Edit::BeforeGuardEnd(&addition),
            ],
        )
    }

    /// Component headers, shared RTE types and provenance. The ECU variant of
    /// shares the same public interface as the standalone contract.
    pub(super) fn contract_headers(
        &self,
        composition: &super::composition::CompositionContract,
    ) -> Result<Vec<(String, Vec<u8>)>, Vec<PlanDiagnostic>> {
        let description = self.description();
        let mut types = String::from("#include \"Rte.h\"\n");
        for datatype in &composition.types {
            if datatype.symbol == datatype.native_type && datatype.array_length.is_none() {
                continue;
            }
            let suffix = datatype
                .array_length
                .map(|length| format!("[{length}]"))
                .unwrap_or_default();
            types.push_str(&format!(
                "typedef {} {}{suffix};\n",
                datatype.native_type, datatype.symbol
            ));
        }
        let mut files = vec![
            (
                "include/Rte_Cfg.h".into(),
                header("RTE_CFG_H", "#include \"Std_Types.h\"\n"),
            ),
            ("include/Rte_Type.h".into(), header("RTE_TYPE_H", &types)),
            (
                "include/Rte.h".into(),
                header("RTE_H", &super::rte_types::standard()),
            ),
            (
                "include/Rte_Main.h".into(),
                super::rte::MAIN_HEADER.to_vec(),
            ),
            (
                "include/Rte_DataHandleType.h".into(),
                header("RTE_DATA_HANDLE_TYPE_H", "#include \"Rte_Type.h\"\n"),
            ),
        ];
        for component in &composition.components {
            let scope = super::short_name(&component.component);
            let mut body = format!(
                "#include \"Rte_{scope}_Type.h\"\n#include \"Rte_DataHandleType.h\"\n\n#ifdef __cplusplus\nextern \"C\" {{\n#endif\n\nstruct Rte_CDS_{scope} {{\n    uint8 Rte_Dummy; /**< Empty CDS for the source, single-instance component. */\n}};\ntypedef struct Rte_CDS_{scope} Rte_CDS_{scope};\n#ifndef RTE_CORE\ntypedef const Rte_CDS_{scope} *Rte_Instance;\n#endif\nextern const Rte_CDS_{scope} *const Rte_Inst_{scope};\n\n"
            );
            let init_values: Vec<_> = component
                .data_ports
                .iter()
                .map(|port| {
                    (
                        format!("Rte_InitValue_{}_{}", port.name, port.element_name),
                        format!("{}UL", port.initial_value),
                    )
                })
                .collect();
            body.push_str("#ifndef RTE_CORE\n");
            body.push_str(&c_layout::defines(&init_values));
            body.push('\n');
            let runnables: Vec<_> = component
                .runnables
                .iter()
                .map(|runnable| {
                    (
                        format!("RTE_RUNNABLE_{}", super::short_name(&runnable.path)),
                        runnable.symbol.clone(),
                    )
                })
                .collect();
            body.push_str(&c_layout::defines(&runnables));
            body.push('\n');
            body.push_str("#endif\n");
            let errors: std::collections::BTreeSet<_> = component
                .operations
                .iter()
                .filter_map(|operation| operation.application_error.as_ref())
                .collect();
            for error in errors {
                body.push_str(&c_layout::defines(&[(error.clone(), "1U")]));
            }
            for symbol in description
                .symbols
                .iter()
                .filter(|symbol| symbol.declaration_owner == component.header)
            {
                let parameters = if symbol.arguments.is_empty() {
                    vec!["void".into()]
                } else {
                    symbol
                        .arguments
                        .iter()
                        .map(|argument| c_layout::parameter(&argument.native_type, &argument.name))
                        .collect()
                };
                let mut docs = String::new();
                for argument in &symbol.arguments {
                    docs.push_str(&format!(
                        " * @param[{}] {} {}\n",
                        argument.direction.to_lowercase(),
                        argument.name,
                        if argument.native_type.contains('*')
                            || composition
                                .array_types
                                .values()
                                .any(|native| native == &argument.native_type)
                        {
                            "Non-NULL caller-owned storage; not retained."
                        } else {
                            "Input value."
                        }
                    ));
                }
                if symbol.return_type != "void" {
                    let port = component.data_ports.iter().find(|port| {
                        super::composition::data_symbol(component, port) == symbol.symbol
                    });
                    let status = if let Some(port) = port {
                        let network = composition.network_endpoints.iter().any(|endpoint| {
                            endpoint.endpoint.instance == component.instance
                                && endpoint.endpoint.port == port.path
                        });
                        if network && port.read {
                            "E_OK, configured RTE_E_NEVER_RECEIVED or RTE_E_MAX_AGE_EXCEEDED with initial/last value; COM_SERVICE_NOT_AVAILABLE maps to RTE_E_COM_STOPPED.".to_owned()
                        } else if network {
                            "E_OK for buffer update; COM_SERVICE_NOT_AVAILABLE maps to RTE_E_COM_STOPPED; lower transmission failure is separate.".to_owned()
                        } else if port.read {
                            format!(
                                "This receiver reads initial value {} until publication, then the producer's last value; handleNeverReceived selects the initial status.",
                                port.initial_value
                            )
                        } else {
                            "E_OK on publication, or when an unconnected port discards the value."
                                .to_owned()
                        }
                    } else if symbol.definition_owner.starts_with("application/") {
                        "E_OK with a valid value, including a replacement value on application failure (SWS_Dcm_00686).".to_owned()
                    } else {
                        "E_OK for a completed call; RTE_E_UNCONNECTED preserves OUT/INOUT without calling a server. Connected APIs reject calls before start, outside the owner or with NULL storage."
                            .to_owned()
                    };
                    docs.push_str(&c_layout::comment(" * ", &format!("@return {status}")));
                }
                let runnable = component
                    .runnables
                    .iter()
                    .any(|runnable| runnable.symbol == symbol.symbol);
                let scope = super::short_name(&component.component);
                if !runnable {
                    body.push_str(&api_visibility(component, symbol));
                }
                if runnable {
                    body.push_str(&format!(
                        "#define {scope}_START_SEC_CODE\n#include \"{scope}_MemMap.h\"\n"
                    ));
                }
                let parameters = if runnable && symbol.definition_owner.starts_with("application/")
                {
                    symbol
                        .arguments
                        .iter()
                        .map(|argument| argument.native_type.trim().to_owned())
                        .collect::<Vec<_>>()
                } else {
                    parameters
                };
                let parameters = if parameters.is_empty() {
                    vec!["void".into()]
                } else {
                    parameters
                };
                declaration(
                    &mut body,
                    "Component contract in the configured single-owner task.",
                    &c_layout::signature(
                        &if runnable {
                            format!("{scope}_CODE {}", symbol.return_type)
                        } else {
                            symbol.return_type.clone()
                        },
                        &symbol.symbol,
                        &parameters,
                        ";",
                        runnable,
                    ),
                    &docs,
                );
                if runnable {
                    body.push_str(&format!(
                        "#define {scope}_STOP_SEC_CODE\n#include \"{scope}_MemMap.h\"\n\n"
                    ));
                }
                if !runnable {
                    let alias = component
                        .data_ports
                        .iter()
                        .find(|port| {
                            super::composition::data_symbol(component, port) == symbol.symbol
                        })
                        .map(|port| port.api_symbol.as_str())
                        .or_else(|| {
                            component
                                .operations
                                .iter()
                                .find(|operation| {
                                    operation.read
                                        && operation.implementation_symbol == symbol.symbol
                                })
                                .map(|operation| operation.api_symbol.as_str())
                        });
                    if let Some(alias) = alias {
                        body.push_str(&format!(
                            "#ifndef RTE_CORE\n{}#endif\n",
                            c_layout::defines(&[(alias, symbol.symbol.as_str())])
                        ));
                    }
                    body.push_str("#endif\n");
                }
            }
            let name = component
                .header
                .trim_start_matches("include/Rte_")
                .trim_end_matches(".h");
            body.push_str("#ifdef __cplusplus\n}\n#endif\n");
            let guard = format!("RTE_{}_H", name.to_uppercase());
            let application_header = format!(
                "/** @file Application contract for {name}. */\n#ifndef {guard}\n#ifndef RTE_CORE\n#ifdef RTE_APPLICATION_HEADER_FILE\n#error Multiple application header files included.\n#endif\n#define RTE_APPLICATION_HEADER_FILE\n#endif\n#define {guard}\n{body}\n#endif\n"
            );
            files.push((component.header.clone(), application_header.into_bytes()));
            files.push((format!("include/Rte_{name}_Type.h"), header(&format!("RTE_{name}_TYPE_H"), "#include \"Rte_Type.h\"\n#include \"Rte_Dcm_Type.h\"\n#ifdef __cplusplus\nextern \"C\" {\n#endif\n#ifdef __cplusplus\n}\n#endif\n")));
            files.push((
                format!("include/{name}_MemMap.h"),
                super::memmap::component(name).into_bytes(),
            ));
        }
        let provenance = serde_json::json!({ "format": "autosar-component-contract-v2", "profile": description.profile,
            "sources": description.sources, "validationDependencies": description.validation_dependencies,
            "ruleSetIdentity": description.rule_set_identity, "requiredExtensionDefinitions": description.required_extension_definitions,
            "composition": composition, "schedule": description.schedule,
            "symbols": description.symbols,
            "declarationOwners": description.symbols.iter().filter(|symbol| symbol.declaration_owner.starts_with("include/Rte_")).map(|symbol| (&symbol.symbol, &symbol.declaration_owner)).collect::<BTreeMap<_, _>>() });
        let mut bytes = serde_json::to_vec_pretty(&provenance)
            .map_err(|error| reject(self, error.to_string().into()))?;
        bytes.push(b'\n');
        files.push(("contract.json".into(), bytes));
        Ok(files)
    }
}

fn api_visibility(
    component: &super::composition::Component,
    symbol: &super::SymbolContract,
) -> String {
    let selectors: Vec<_> = component
        .runnables
        .iter()
        .map(|runnable| {
            format!(
                "defined(RTE_RUNNABLEAPI_{})",
                super::short_name(&runnable.path)
            )
        })
        .collect();
    let mut conditions = vec![
        "defined(RTE_CORE)".to_owned(),
        format!(
            "({})",
            selectors
                .iter()
                .map(|selector| format!("!{selector}"))
                .collect::<Vec<_>>()
                .join(" && ")
        ),
    ];
    conditions.extend(
        component
            .runnables
            .iter()
            .filter(|runnable| symbol.consumers.contains(&runnable.path))
            .map(|runnable| {
                format!(
                    "defined(RTE_RUNNABLEAPI_{})",
                    super::short_name(&runnable.path)
                )
            }),
    );
    c_layout::condition("if", &conditions)
}
