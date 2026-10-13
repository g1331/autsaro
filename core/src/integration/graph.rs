use super::{DiagnosticCategory, InputSource, PlanDiagnostic};
use roxmltree::Document;
use std::collections::{BTreeMap, HashMap};

const NS: &str = "http://autosar.org/schema/r4.0";

#[derive(Clone, Debug)]
pub(super) struct Element {
    pub tag: String,
    pub text: String,
    pub attributes: BTreeMap<String, String>,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
    pub file: String,
    pub object: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReferencePolicy {
    ConsumerScoped,
}

/// A lossless-input semantic index. Original bytes remain in InputSource;
/// this derived graph never becomes the authority for saving XML.
pub(super) struct Graph {
    pub elements: Vec<Element>,
    pub objects: BTreeMap<String, usize>,
    pub external: BTreeMap<String, String>,
    /// Built from the product's built-in rules rather than external archives.
    pub builtin: bool,
}

impl Graph {
    /// Index original source elements without qualifying unrelated references.
    /// Before invoking target consumers, the caller must qualify their complete
    /// object closure with `reference_diagnostics_for`; no unknown kind is inferred.
    pub fn from_catalog_target_scope(
        sources: &[InputSource],
        catalog: &crate::definitions::DefinitionCatalog,
    ) -> Result<Self, Vec<PlanDiagnostic>> {
        Self::build(
            sources,
            Self::catalog_external(catalog),
            ReferencePolicy::ConsumerScoped,
        )
    }

    fn catalog_external(
        catalog: &crate::definitions::DefinitionCatalog,
    ) -> BTreeMap<String, String> {
        catalog
            .definitions()
            .map(|definition| {
                (
                    definition.definition_id.clone(),
                    definition.element_kind.clone(),
                )
            })
            .collect()
    }

    fn build(
        sources: &[InputSource],
        external: BTreeMap<String, String>,
        reference_policy: ReferencePolicy,
    ) -> Result<Self, Vec<PlanDiagnostic>> {
        let documents = sources
            .iter()
            .map(|source| {
                let text = source.text().map_err(|diagnostic| vec![diagnostic])?;
                Document::parse(text).map_err(|error| {
                    vec![PlanDiagnostic::at_source(
                        source,
                        "XML_PARSE",
                        crate::product_message!("backend.integration.graph.xml_parse_failed", "error" => error),
                        crate::product_message!("backend.integration.graph.xml_syntax_repair_required"),
                    )]
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::build_documents(
            sources
                .iter()
                .zip(&documents)
                .map(|(source, document)| (source.logical_path.as_str(), document)),
            external,
            reference_policy,
        )
    }

    fn build_documents<'a, 'input>(
        sources: impl Iterator<Item = (&'a str, &'a Document<'input>)>,
        external: BTreeMap<String, String>,
        reference_policy: ReferencePolicy,
    ) -> Result<Self, Vec<PlanDiagnostic>>
    where
        'input: 'a,
    {
        let mut graph = Self {
            elements: Vec::new(),
            objects: BTreeMap::new(),
            external,
            builtin: reference_policy == ReferencePolicy::ConsumerScoped,
        };
        let mut diagnostics = Vec::new();
        for (file, document) in sources {
            let mut indices = HashMap::new();
            for node in document
                .descendants()
                .filter(|node| node.is_element() && node.tag_name().namespace() == Some(NS))
            {
                let parent = node
                    .parent()
                    .and_then(|parent| indices.get(&parent.id()).copied());
                let index = graph.elements.len();
                let object = path(node);
                graph.elements.push(Element {
                    tag: node.tag_name().name().into(),
                    text: if node.children().any(|child| child.is_element()) {
                        String::new()
                    } else {
                        crate::definitions::xml_text_trimmed(node).into_owned()
                    },
                    attributes: node
                        .attributes()
                        .map(|a| (a.name().into(), a.value().into()))
                        .collect(),
                    children: Vec::new(),
                    parent,
                    file: file.into(),
                    object: object.clone(),
                });
                if let Some(parent) = parent {
                    graph.elements[parent].children.push(index);
                }
                indices.insert(node.id(), index);
                if short_name(node).is_some() && node.tag_name().name() != "AR-PACKAGE" {
                    if graph.objects.insert(object.clone(), index).is_some() {
                        diagnostics.push(graph.diagnostic(
                            index,
                            DiagnosticCategory::Input,
                            "OBJECT_DUPLICATE",
                            crate::product_message!(
                                "backend.integration.graph.duplicate_autosar_object_path"
                            ),
                            crate::product_message!(
                                "backend.integration.graph.duplicate_object_path_repair_required"
                            ),
                        ));
                    }
                    if graph.external.contains_key(&object) {
                        diagnostics.push(graph.diagnostic(
                            index,
                            DiagnosticCategory::Input,
                            "DEFINITION_SHADOWED",
                            crate::product_message!("backend.integration.graph.input_object_shadows_pinned_ecuc_definition"),
                            crate::product_message!("backend.integration.graph.local_mod_definition_replacement_remove_required"),
                        ));
                    }
                }
            }
        }
        if reference_policy == ReferencePolicy::ConsumerScoped {
            return if diagnostics.is_empty() {
                Ok(graph)
            } else {
                Err(diagnostics)
            };
        }
        let mut selected: Vec<_> = graph
            .of_kind("SYSTEM")
            .into_iter()
            .filter(|index| graph.text(*index, "CATEGORY") == Some("ECU_EXTRACT"))
            .collect();
        for system in selected.clone() {
            for root in graph.descendants(system, "ROOT-SW-COMPOSITION-PROTOTYPE") {
                if let Some(composition) = graph.target(root, "SOFTWARE-COMPOSITION-TREF") {
                    for instance in graph.descendants(composition, "SW-COMPONENT-PROTOTYPE") {
                        if let Some(component) = graph.target(instance, "TYPE-TREF") {
                            selected.push(component);
                        }
                    }
                }
            }
        }
        for root in selected {
            for (tag, code) in [
                ("QUEUED-RECEIVER-COM-SPEC", "QUEUED_UNSUPPORTED"),
                ("QUEUED-SENDER-COM-SPEC", "QUEUED_UNSUPPORTED"),
                ("DATA-READ-ACCESSS", "IMPLICIT_UNSUPPORTED"),
                ("DATA-WRITE-ACCESSS", "IMPLICIT_UNSUPPORTED"),
                ("MODE-ACCESS-POINTS", "MODE_UNSUPPORTED"),
                ("ASYNCHRONOUS-SERVER-CALL-POINT", "ASYNC_UNSUPPORTED"),
                ("VARIATION-POINT", "VARIANT_UNSELECTED"),
                ("DATA-TRANSFORMATIONS", "TRANSFORMER_UNSUPPORTED"),
            ] {
                for index in graph.descendants(root, tag) {
                    diagnostics.push(graph.diagnostic(index, DiagnosticCategory::Unsupported,
                        code, crate::product_message!("backend.integration.graph.target_profile_tag_unsupported", "tag" => tag),
                        crate::product_message!("backend.integration.graph.explicit_sr_synchronous_service_profile_required")));
                }
            }
        }
        diagnostics.extend(graph.audit_references(|_, _| true));
        if diagnostics.is_empty() {
            Ok(graph)
        } else {
            Err(diagnostics)
        }
    }

    pub fn of_kind(&self, kind: &str) -> Vec<usize> {
        self.objects
            .values()
            .copied()
            .filter(|index| self.elements[*index].tag == kind)
            .collect()
    }

    pub fn children(&self, index: usize, tag: &str) -> Vec<usize> {
        self.elements[index]
            .children
            .iter()
            .copied()
            .filter(|index| self.elements[*index].tag == tag)
            .collect()
    }

    pub fn text(&self, index: usize, tag: &str) -> Option<&str> {
        self.children(index, tag)
            .first()
            .map(|index| self.elements[*index].text.as_str())
    }

    pub fn descendants(&self, index: usize, tag: &str) -> Vec<usize> {
        let mut result = Vec::new();
        let mut pending = self.elements[index].children.clone();
        while let Some(child) = pending.pop() {
            if self.elements[child].tag == tag {
                result.push(child);
            }
            pending.extend(self.elements[child].children.iter().rev().copied());
        }
        result.sort_unstable();
        result
    }

    pub fn target(&self, index: usize, tag: &str) -> Option<usize> {
        self.text(index, tag)
            .and_then(|path| self.objects.get(path).copied())
    }

    pub fn within(&self, mut index: usize, ancestor: usize) -> bool {
        loop {
            if index == ancestor {
                return true;
            }
            match self.elements[index].parent {
                Some(parent) => index = parent,
                None => return false,
            }
        }
    }

    pub fn ancestor(&self, mut index: usize, tag: &str) -> Option<usize> {
        while let Some(parent) = self.elements[index].parent {
            if self.elements[parent].tag == tag {
                return Some(parent);
            }
            index = parent;
        }
        None
    }

    pub fn diagnostic(
        &self,
        index: usize,
        category: DiagnosticCategory,
        code: &str,
        message: crate::message::LocalizedText,
        remedy: crate::message::LocalizedText,
    ) -> PlanDiagnostic {
        let node = &self.elements[index];
        PlanDiagnostic {
            category,
            code: code.into(),
            file: Some(node.file.clone()),
            object: Some(node.object.clone()),
            message: message.into(),
            remedy: remedy.into(),
        }
    }

    /// Some object to locate diagnostics about the inputs as a whole.
    pub fn context(&self) -> Result<usize, Vec<PlanDiagnostic>> {
        self.objects.values().next().copied().ok_or_else(|| {
            vec![PlanDiagnostic {
                category: DiagnosticCategory::Input,
                code: "INPUT_OBJECTS_MISSING".into(),
                file: None,
                object: None,
                message: crate::product_message!("backend.integration.graph.objects_missing"),
                remedy: crate::product_message!(
                    "backend.integration.graph.supply_object_and_correct_typed_reference"
                ),
            }]
        })
    }

    /// The element of a complete AUTOSAR path; a missing object is an
    /// unresolved reference located at that path.
    pub fn index(&self, path: &str) -> Result<usize, Vec<PlanDiagnostic>> {
        self.objects.get(path).copied().ok_or_else(|| {
            vec![PlanDiagnostic {
                category: DiagnosticCategory::Input,
                code: "REFERENCE_UNRESOLVED".into(),
                file: None,
                object: Some(path.into()),
                message: crate::message::LocalizedText::messages([
                    crate::product_message!("backend.integration.graph.referenced_autosar_object_missing"),
                    crate::product_message!("backend.integration.graph.reference_target", "target" => path),
                ]),
                remedy: crate::product_message!(
                    "backend.integration.graph.supply_object_and_correct_typed_reference"
                ),
            }]
        })
    }

    pub fn reference_diagnostics_for(
        &self,
        consumers: &std::collections::BTreeSet<String>,
    ) -> Vec<PlanDiagnostic> {
        self.audit_references(|_, element| consumers.contains(&element.object))
    }

    /// Qualify only references consumed by a structural source-owner proof.
    /// This permits excluding unrelated ECUC definition authorization.
    pub fn reference_diagnostics_for_indices(
        &self,
        consumers: &std::collections::BTreeSet<usize>,
    ) -> Vec<PlanDiagnostic> {
        self.audit_references(|index, _| consumers.contains(&index))
    }

    fn audit_references(&self, include: impl Fn(usize, &Element) -> bool) -> Vec<PlanDiagnostic> {
        let graph = self;
        let mut diagnostics = Vec::new();
        for (index, element) in graph
            .elements
            .iter()
            .enumerate()
            .filter(|(index, element)| include(*index, element))
        {
            if let Some(destination) = element.attributes.get("DEST") {
                let kind = graph
                    .objects
                    .get(&element.text)
                    .map(|index| graph.elements[*index].tag.as_str())
                    .or_else(|| graph.external.get(&element.text).map(String::as_str));
                let (code, message) = match kind {
                    None => (
                        "REFERENCE_UNRESOLVED",
                        crate::product_message!(
                            "backend.integration.graph.referenced_autosar_object_missing"
                        ),
                    ),
                    Some(kind)
                        if kind != destination
                            && !(kind == "MODE-GROUP"
                                && destination == "MODE-DECLARATION-GROUP-PROTOTYPE") =>
                    {
                        (
                            "REFERENCE_DEST",
                            crate::product_message!(
                                "backend.integration.graph.reference_dest_kind_mismatch"
                            ),
                        )
                    }
                    Some(_) => continue,
                };
                if kind.is_none() && destination == "R-PORT-PROTOTYPE" {
                    if let Some((parent, parent_index)) =
                        graph.objects.iter().rev().find(|(path, index)| {
                            element.text.starts_with(&format!("{path}/"))
                                && graph.elements[**index].tag == "SERVICE-SW-COMPONENT-TYPE"
                        })
                    {
                        let mut issue = graph.diagnostic(*parent_index, DiagnosticCategory::Input,
                            "SERVICE_CLIENT_MISSING", crate::product_message!("backend.integration.graph.declared_service_client_port_missing", "parent" => parent),
                            crate::product_message!("backend.integration.graph.service_client_local_provider_binding_required"));
                        issue.object = Some(element.text.clone());
                        diagnostics.push(issue);
                    }
                }
                if kind.is_none() && destination == "BSW-MODULE-ENTRY" {
                    let mut issue = graph.diagnostic(
                        index,
                        DiagnosticCategory::Input,
                        "BSW_ENTRY_MISSING",
                        crate::product_message!(
                            "backend.integration.graph.bsw_entry_reference_missing"
                        ),
                        crate::product_message!(
                            "backend.integration.graph.supply_bsw_entry_and_implementation"
                        ),
                    );
                    issue.object = Some(element.text.clone());
                    diagnostics.push(issue);
                }
                diagnostics.push(graph.diagnostic(
                    index, DiagnosticCategory::Input, code,
                    crate::message::LocalizedText::messages([message, crate::product_message!("backend.integration.graph.reference_target", "target" => element.text)]),
                    crate::product_message!("backend.integration.graph.supply_object_and_correct_typed_reference"),
                ));
            }
        }
        diagnostics
    }
}

impl Graph {
    fn integration_consumers(
        &self,
        catalog: &crate::definitions::DefinitionCatalog,
    ) -> std::collections::BTreeSet<String> {
        let mut pending = self.of_kind("SYSTEM");
        pending.extend(
            self.of_kind("ECUC-MODULE-CONFIGURATION-VALUES")
                .into_iter()
                .filter(|index| {
                    self.text(*index, "DEFINITION-REF")
                        .and_then(|id| id.strip_prefix("/AUTOSAR/EcucDefs/"))
                        .is_some_and(|name| {
                            matches!(
                                name,
                                "ComM"
                                    | "BswM"
                                    | "Can"
                                    | "CanIf"
                                    | "CanTp"
                                    | "Com"
                                    | "Dcm"
                                    | "EcuC"
                                    | "Os"
                                    | "PduR"
                                    | "Rte"
                            )
                        })
                }),
        );
        let mut visited = std::collections::BTreeSet::new();
        let mut consumers = std::collections::BTreeSet::new();
        while let Some(index) = pending.pop() {
            if !visited.insert(index) {
                continue;
            }
            let element = &self.elements[index];
            if element.tag.starts_with("ECUC-")
                && self
                    .text(index, "DEFINITION-REF")
                    .is_some_and(|id| catalog.get(id).is_none())
            {
                continue;
            }
            consumers.insert(element.object.clone());
            pending.extend(element.children.iter().copied());
            if element.attributes.contains_key("DEST") {
                if let Some(target) = self.objects.get(&element.text) {
                    pending.push(*target);
                }
            }
        }
        consumers
    }
}

// Kept beside the private integration graph so native definition validation can
// reuse existing cross-module rules without exposing graph implementation types.
impl crate::definitions::DefinitionCatalog {
    /// Mainline cross-module constraints for native definition validation;
    /// the flag reports whether the integration graph itself resolved.
    pub(crate) fn mainline_definition_constraints<'a, 'input>(
        &self,
        files: impl Iterator<Item = (&'a str, &'a Document<'input>)>,
    ) -> (bool, Vec<PlanDiagnostic>)
    where
        'input: 'a,
    {
        let external = Graph::catalog_external(self);
        match Graph::build_documents(files, external, ReferencePolicy::ConsumerScoped) {
            Ok(graph) => {
                let consumers = graph.integration_consumers(self);
                let references = graph.reference_diagnostics_for(&consumers);
                if !references.is_empty() {
                    return (false, references);
                }
                // Built-in validation always plans on the mainline.
                let result = super::plan::inspect_composition(&graph, Some(self));
                (true, result.err().unwrap_or_default())
            }
            Err(diagnostics) => (false, diagnostics),
        }
    }
}

fn short_name<'a, 'input>(node: roxmltree::Node<'a, 'input>) -> Option<std::borrow::Cow<'a, str>> {
    node.children()
        .find(|child| {
            child.is_element()
                && child.tag_name().namespace() == Some(NS)
                && child.tag_name().name() == "SHORT-NAME"
        })
        .map(crate::definitions::xml_text_trimmed)
}

fn path(node: roxmltree::Node<'_, '_>) -> String {
    let mut names: Vec<_> = node.ancestors().filter_map(short_name).collect();
    names.reverse();
    format!("/{}", names.join("/"))
}

#[cfg(test)]
mod tests {
    use super::Graph;

    #[test]
    fn missing_objects_are_located_input_diagnostics() {
        let graph = Graph {
            elements: Vec::new(),
            objects: Default::default(),
            external: Default::default(),
            builtin: true,
        };
        let missing = graph.index("/Pkg/Missing").unwrap_err();
        assert_eq!(missing[0].code, "REFERENCE_UNRESOLVED");
        assert_eq!(missing[0].object.as_deref(), Some("/Pkg/Missing"));
        assert_eq!(
            graph.context().unwrap_err()[0].code,
            "INPUT_OBJECTS_MISSING"
        );
    }
}
