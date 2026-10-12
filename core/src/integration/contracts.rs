use super::{DiagnosticCategory, PlanDiagnostic, ValidatedIntegrationPlan};
use crate::{GenerationPreview, GenerationReport, generator};
use std::collections::BTreeMap;
use std::fmt::Write;
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
        .multi
        .as_ref()
        .map(|multi| multi.composition.as_str())
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
        let multi = self.description().multi.as_ref().ok_or_else(|| {
            reject(
                self,
                crate::product_message!("backend.integration.multi.consumer_unsupported").into(),
            )
        })?;
        self.multi_contract_files(multi)
    }
}

impl ValidatedIntegrationPlan {
    /// Generate actual bounded multi RTE producers and their component headers.
    /// This source fragment requires configured COM and caller-owned application
    /// producers; it does not replace complete sealed ECU/OS preparation.
    pub(super) fn rte_runtime_files(&self) -> Result<ComponentContractFiles, Vec<PlanDiagnostic>> {
        let mut files: BTreeMap<_, _> = self
            .component_contract_files()?
            .into_files()
            .into_iter()
            .collect();
        files.extend(super::multi_rte::files(self));
        let integer = |kind: super::CommunicationIntegerType| match kind {
            super::CommunicationIntegerType::Uint8 => "uint8",
            super::CommunicationIntegerType::Uint16 => "uint16",
            super::CommunicationIntegerType::Uint32 => "uint32",
        };
        let communication = self.description().communication_runtime.as_ref().unwrap();
        files.insert("include/ComStack_Cfg.h".into(), format!("/** @file EcuC-selected communication widths. */\n#ifndef COMSTACK_CFG_H\n#define COMSTACK_CFG_H\n#include \"Platform_Types.h\"\ntypedef {} PduIdType;\ntypedef {} PduLengthType;\n#endif\n", integer(communication.pdu_id_type), integer(communication.pdu_length_type)).into_bytes());
        files.insert(
            "include/ComStack_Types.h".into(),
            include_bytes!("../../../runtime/multi/include/ComStack_Types.h").to_vec(),
        );
        files.insert(
            "include/Com.h".into(),
            include_bytes!("../../../runtime/multi/include/Com.h").to_vec(),
        );
        let rte = files.get_mut("include/Rte.h").unwrap();
        *rte = String::from_utf8(std::mem::take(rte))
            .unwrap()
            .replace(
                "#include \"Rte_Type.h\"",
                "#include \"Rte_Type.h\"\n#include \"Rte_Main.h\"\n#define RTE_E_OK 0U\n#define RTE_E_LIMIT 130U",
            )
            .into_bytes();
        files.insert("README.md".into(), b"# Multi-component RTE sources\n\nThese sources implement the validated local communication graph, synchronous calls, generated Dcm service bridge and handle-bearing COM freshness notifications. Rte_Start initializes each receiver's own explicit initial value; Rte_Stop retires access. Invoke lifecycle only from the trusted ECU context, after required BSW/SchM initialization and before BSW shutdown. User application algorithms are separate caller-owned producers. The configured COM implementation, complete SchM/BSW configuration, OS owner and immutable ECU package are provided by complete ECU preparation; this source fragment alone is not a runnable ECU.\n".to_vec());
        Ok(ComponentContractFiles {
            files: files.into_iter().collect(),
        })
    }

    fn multi_contract_files(
        &self,
        multi: &super::multi::MultiComponentContract,
    ) -> Result<ComponentContractFiles, Vec<PlanDiagnostic>> {
        let description = self.description();
        let mut types = String::from("#include \"Std_Types.h\"\n");
        for datatype in multi.array_types.values() {
            writeln!(types, "typedef uint8 {datatype}[4];").unwrap();
        }
        let mut files = vec![
            // The same platform and standard types the delivered ECU compiles with.
            (
                "include/Platform_Types.h".into(),
                include_bytes!("../../../runtime/multi/include/Platform_Types.h").to_vec(),
            ),
            (
                "include/Std_Types.h".into(),
                include_bytes!("../../../runtime/multi/include/Std_Types.h").to_vec(),
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
            body.push_str("#if !defined(RTE_CORE)\n");
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
            body.push_str("#endif\n");
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
