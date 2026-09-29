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

/// A lossless-input semantic index. Original bytes remain in InputSource;
/// this derived graph never becomes the authority for saving XML.
pub(super) struct Graph {
    pub elements: Vec<Element>,
    pub objects: BTreeMap<String, usize>,
    pub external: BTreeMap<String, String>,
}

impl Graph {
    pub fn new(sources: &[InputSource], definitions: &str) -> Result<Self, Vec<PlanDiagnostic>> {
        let mut graph = Self {
            elements: Vec::new(),
            objects: BTreeMap::new(),
            external: BTreeMap::new(),
        };
        let mut diagnostics = Vec::new();
        let external = Document::parse(definitions).map_err(|error| {
            vec![PlanDiagnostic::dependency(
                "MOD_PARSE",
                format!("ECUC definition XML cannot be parsed: {error}"),
                "Provide the pinned, unmodified R24-11 MOD archive.",
            )]
        })?;
        for node in external
            .descendants()
            .filter(|node| node.is_element() && node.tag_name().namespace() == Some(NS))
        {
            if short_name(node).is_some() {
                graph
                    .external
                    .insert(path(node), node.tag_name().name().into());
            }
        }
        for source in sources {
            let text = source.text().map_err(|diagnostic| vec![diagnostic])?;
            let document = Document::parse(text).map_err(|error| {
                vec![PlanDiagnostic::at_source(
                    source,
                    "XML_PARSE",
                    format!("XML cannot be parsed: {error}"),
                    "Repair the XML syntax before checking the integration plan.",
                )]
            })?;
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
                    text: node.text().unwrap_or("").trim().into(),
                    attributes: node
                        .attributes()
                        .map(|a| (a.name().into(), a.value().into()))
                        .collect(),
                    children: Vec::new(),
                    parent,
                    file: source.logical_path.clone(),
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
                            "Two input objects have the same complete AUTOSAR path.",
                            "Remove the duplicate object or give it a distinct complete path.",
                        ));
                    }
                    if graph.external.contains_key(&object) {
                        diagnostics.push(graph.diagnostic(
                            index,
                            DiagnosticCategory::Input,
                            "DEFINITION_SHADOWED",
                            "An input object shadows a pinned ECUC definition.",
                            "Remove the local replacement of the external MOD definition.",
                        ));
                    }
                }
            }
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
                        code, format!("{tag} affects the selected target and is outside its supported profile."),
                        "Use explicit nonqueued S/R and synchronous local services with fully selected variants, without implicit, mode or transformer access."));
                }
            }
        }
        for (index, element) in graph.elements.iter().enumerate() {
            if let Some(destination) = element.attributes.get("DEST") {
                let kind = graph
                    .objects
                    .get(&element.text)
                    .map(|index| graph.elements[*index].tag.as_str())
                    .or_else(|| graph.external.get(&element.text).map(String::as_str));
                let (code, message) = match kind {
                    None => (
                        "REFERENCE_UNRESOLVED",
                        "A referenced AUTOSAR object is missing.",
                    ),
                    Some(kind) if kind != destination => (
                        "REFERENCE_DEST",
                        "The reference DEST differs from the target object kind.",
                    ),
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
                            "SERVICE_CLIENT_MISSING", format!("The declared service client port is absent beneath {parent}."),
                            "Supply the standard service client port and bind its synchronous call to the local provider.");
                        issue.object = Some(element.text.clone());
                        diagnostics.push(issue);
                    }
                }
                if kind.is_none() && destination == "BSW-MODULE-ENTRY" {
                    let mut issue = graph.diagnostic(index, DiagnosticCategory::Input,
                        "BSW_ENTRY_MISSING", "A declared BSW implementation or schedulable entity refers to a missing entry.",
                        "Supply the BSW entry description and its matching implementation/signature; do not substitute an empty entry.");
                    issue.object = Some(element.text.clone());
                    diagnostics.push(issue);
                }
                diagnostics.push(graph.diagnostic(
                    index, DiagnosticCategory::Input, code,
                    format!("{message} Reference: {}", element.text),
                    "Supply the referenced object and correct the typed reference; do not add an SDG substitute.",
                ));
            }
        }
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
        message: impl Into<String>,
        remedy: &str,
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
}

fn short_name<'a, 'input>(node: roxmltree::Node<'a, 'input>) -> Option<&'a str> {
    node.children()
        .find(|child| {
            child.is_element()
                && child.tag_name().namespace() == Some(NS)
                && child.tag_name().name() == "SHORT-NAME"
        })
        .and_then(|child| child.text())
}

fn path(node: roxmltree::Node<'_, '_>) -> String {
    let mut names: Vec<_> = node.ancestors().filter_map(short_name).collect();
    names.reverse();
    format!("/{}", names.join("/"))
}
