use super::*;
use crate::project_model::*;
use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static EPOCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) fn new_epoch() -> String {
    let mut digest = Sha256::new();
    digest.update(std::process::id().to_le_bytes());
    digest.update(EPOCH_SEQUENCE.fetch_add(1, Ordering::Relaxed).to_le_bytes());
    digest.update(format!("{:?}", std::time::SystemTime::now()).as_bytes());
    format!("{:x}", digest.finalize())
}

#[derive(Clone)]
pub(super) struct IndexedObject {
    pub view: ObjectProjection,
    pub source: usize,
    pub range: Range<usize>,
    pub name_range: Range<usize>,
    pub unsafe_semantics: bool,
    pub ordinal: usize,
}

#[derive(Clone)]
pub(super) struct IndexedField {
    pub view: FieldDescriptor,
    pub source: usize,
    pub entry_range: Option<Range<usize>>,
    pub value_range: Option<Range<usize>>,
    pub element: String,
    pub ordinal: usize,
}

#[derive(Clone, Default)]
pub(super) struct SourceSnapshot {
    pub sources: Vec<SourceProjection>,
    pub objects: Vec<IndexedObject>,
    pub fields: Vec<IndexedField>,
    pub references: Vec<ReferenceEdge>,
    pub reference_candidates: Vec<ReferenceCandidate>,
    pub instance_definitions: Vec<InstanceDefinitionOption>,
    pub extension_definitions: Vec<ExtensionDefinitionView>,
    pub validation: Vec<ScopeValidation>,
    pub object_by_id: BTreeMap<String, usize>,
    pub field_by_id: BTreeMap<String, usize>,
    pub paths: BTreeMap<String, Vec<usize>>,
    pub profile: String,
    pub definition_fingerprint: String,
    pub integration_candidate: bool,
    pub rule_fault: Option<String>,
    pub opaque_values: Vec<String>,
}

pub(super) fn simple_text_range(node: Node<'_, '_>) -> Result<Range<usize>, String> {
    if node.children().any(|child| !child.is_text()) {
        return Err("Mixed XML content cannot be edited safely.".into());
    }
    let range = node.range();
    let text = node.document().input_text();
    let original = &text[range.clone()];
    if original.trim_end().ends_with("/>") {
        return Err("Self-closing XML values require an explicit entry replacement.".into());
    }
    let mut quote = None;
    let start = original
        .bytes()
        .position(|byte| {
            if let Some(active) = quote {
                if byte == active {
                    quote = None;
                }
                false
            } else if byte == b'\'' || byte == b'"' {
                quote = Some(byte);
                false
            } else {
                byte == b'>'
            }
        })
        .ok_or("Missing XML opening tag.")?
        + range.start
        + 1;
    let end = original.rfind('<').ok_or("Missing XML closing tag.")? + range.start;
    Ok(start..end)
}

fn unsafe_object(node: Node<'_, '_>) -> bool {
    node.ancestors()
        .filter(|node| node.is_element())
        .any(|ancestor| {
            ancestor.children().any(|child| {
                child.is_element()
                    && matches!(
                        child.tag_name().name(),
                        "VARIATION-POINT" | "SW-SYSCOND" | "INSTANCE-REF"
                    )
            })
        })
        || node.descendants().any(|child| {
            child.is_element()
                && (matches!(
                    child.tag_name().name(),
                    "VARIATION-POINT" | "ECUC-INSTANCE-REFERENCE-VALUE"
                ) || child.tag_name().name().ends_with("-IREF"))
        })
}

fn readonly_definition(id: String, element: &str, reason: &str) -> DefinitionDescriptor {
    DefinitionDescriptor {
        definition_id: id,
        element_kind: element.into(),
        kind: None,
        lower_multiplicity: 0,
        upper_multiplicity: None,
        unit: None,
        minimum: None,
        maximum: None,
        enumeration: Vec::new(),
        default_value: None,
        default_origin: None,
        reference_destinations: Vec::new(),
        writable: false,
        reason: Some(reason.into()),
    }
}

fn creation_reference_options(
    catalog: &crate::definitions::DefinitionCatalog,
    snapshot: &SourceSnapshot,
    fields: &[DefinitionDescriptor],
) -> Vec<CreationReferenceOption> {
    fields
        .iter()
        .filter(|field| field.writable && field.kind == Some(ValueKind::Reference))
        .map(|field| {
            let definitions = catalog.reference_target_definitions(&field.definition_id);
            let allowed = |definition: Option<&str>, dest: &str| {
                field
                    .reference_destinations
                    .iter()
                    .any(|allowed| allowed == dest)
                    && (definitions.is_empty()
                        || definition
                            .is_some_and(|id| definitions.iter().any(|allowed| allowed == id)))
            };
            let existing_targets = snapshot
                .objects
                .iter()
                .filter(|target| {
                    !target.unsafe_semantics
                        && allowed(target.view.definition_id.as_deref(), &target.view.kind)
                        && snapshot
                            .paths
                            .get(&target.view.path)
                            .is_some_and(|indices| indices.len() == 1)
                })
                .map(|target| ReferenceTarget {
                    target_id: target.view.object_id.clone(),
                    path: target.view.path.clone(),
                    short_name: target.view.short_name.clone(),
                    dest: target.view.kind.clone(),
                    definition_id: target.view.definition_id.clone(),
                })
                .collect();
            let created_targets = catalog
                .definitions()
                .filter(|definition| definition.writable && definition.kind.is_none())
                .filter_map(|definition| {
                    let dest = if definition.element_kind == "ECUC-MODULE-DEF" {
                        "ECUC-MODULE-CONFIGURATION-VALUES"
                    } else {
                        "ECUC-CONTAINER-VALUE"
                    };
                    allowed(Some(&definition.definition_id), dest).then(|| CreatedReferenceTarget {
                        definition_id: definition.definition_id.clone(),
                        dest: dest.into(),
                    })
                })
                .collect();
            CreationReferenceOption {
                field_definition_id: field.definition_id.clone(),
                existing_targets,
                created_targets,
            }
        })
        .collect()
}

fn definition_template(
    catalog: &crate::definitions::DefinitionCatalog,
    snapshot: &SourceSnapshot,
    definition: &DefinitionDescriptor,
) -> InstanceDefinitionTemplate {
    let children = catalog.children(&definition.definition_id);
    let fields: Vec<_> = children
        .iter()
        .filter(|child| child.kind.is_some())
        .map(|child| (*child).clone())
        .collect();
    let reference_options = creation_reference_options(catalog, snapshot, &fields);
    InstanceDefinitionTemplate {
        definition: definition.clone(),
        fields,
        children: children
            .into_iter()
            .filter(|child| child.kind.is_none())
            .cloned()
            .collect(),
        reference_options,
    }
}

impl Workspace {
    pub(super) fn rebuild_snapshot(&mut self) -> Result<(), String> {
        #[cfg(feature = "verification-metrics")]
        crate::verification::before(crate::verification::Phase::SnapshotBuild);
        let previous = self.snapshot.clone();
        let mut snapshot = SourceSnapshot::default();
        let mut next_identity = self.next_identity;
        let epoch = &self.epoch;
        let mut allocate = |category: &str| {
            next_identity += 1;
            let mut digest = Sha256::new();
            digest.update(epoch.as_bytes());
            digest.update(category.as_bytes());
            digest.update(next_identity.to_le_bytes());
            format!("{:x}", digest.finalize())
        };
        let old_objects: BTreeMap<_, _> = previous
            .objects
            .iter()
            .map(|object| {
                (
                    (
                        object.view.source_id.clone(),
                        object.view.path.clone(),
                        object.view.kind.clone(),
                        object.ordinal,
                    ),
                    object.view.object_id.clone(),
                )
            })
            .collect();
        let old_fields: BTreeMap<_, _> = previous
            .fields
            .iter()
            .map(|field| {
                (
                    (
                        field.view.object_id.clone(),
                        field.view.definition.definition_id.clone(),
                        field.element.clone(),
                        field.ordinal,
                    ),
                    field.view.field_id.clone(),
                )
            })
            .collect();
        for (source, file) in self.files.iter().enumerate() {
            let path = file.path.display().to_string();
            let source_id = previous
                .sources
                .iter()
                .find(|item| item.path == path)
                .map(|item| item.source_id.clone())
                .unwrap_or_else(|| allocate("source"));
            snapshot.sources.push(SourceProjection {
                source_id: source_id.clone(),
                path,
                readonly: false,
                sha256: format!("{:x}", Sha256::digest(file.text.as_bytes())),
            });
            #[cfg(feature = "verification-metrics")]
            crate::verification::before(crate::verification::Phase::SourceScan);
            let document = Document::parse(&file.text).map_err(|error| error.to_string())?;
            for node in document.descendants().filter(|node| node.is_element()) {
                if node.tag_name().namespace() != Some(NS)
                    && !node.children().any(|child| child.is_element())
                {
                    if let Some(value) = node.text().filter(|value| value.contains('/')) {
                        snapshot.opaque_values.push(value.into());
                    }
                }
                for attribute in node.attributes().filter(|attribute| {
                    attribute.value().contains('/')
                        && (node.tag_name().namespace() != Some(NS)
                            || attribute.namespace().is_some_and(|namespace| {
                                namespace != XSI
                                    && namespace != "http://www.w3.org/XML/1998/namespace"
                            }))
                }) {
                    snapshot.opaque_values.push(attribute.value().into());
                }
            }
            snapshot.integration_candidate |= document.descendants().any(|node| {
                node.is_element()
                    && node.tag_name().name() == "SYSTEM"
                    && child_text(node, "CATEGORY").as_deref() == Some("ECU_EXTRACT")
            });
            let mut node_objects = std::collections::HashMap::new();
            let mut object_ordinals = BTreeMap::new();
            for node in document.descendants().filter(|node| {
                node.is_element()
                    && node.tag_name().namespace() == Some(NS)
                    && child_text(*node, "SHORT-NAME").is_some()
            }) {
                let path = path_of(node);
                let kind = node.tag_name().name().to_owned();
                let short_name_node = node
                    .children()
                    .find(|child| child.is_element() && child.tag_name().name() == "SHORT-NAME")
                    .unwrap();
                let definition_id = definition(node);
                let descriptor = definition_id.as_deref().and_then(|id| self.catalog.get(id));
                let unsupported_edition = node
                    .ancestors()
                    .filter(|ancestor| {
                        ancestor.is_element()
                            && ancestor.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES"
                    })
                    .any(|module| {
                        child_text(module, "ECUC-DEF-EDITION")
                            .is_some_and(|edition| edition != "4.10.0")
                    });
                let unsafe_semantics = unsupported_edition
                    || unsafe_object(node)
                    || simple_text_range(short_name_node).is_err();
                let writable = !unsafe_semantics
                    && (kind == "AR-PACKAGE" || descriptor.is_some_and(|item| item.writable));
                let reason = if unsupported_edition {
                    Some(
                        "Unsupported ECUC definition edition is read-only; R24-11 requires 4.10.0."
                            .into(),
                    )
                } else if unsafe_semantics {
                    Some("Variant or instance-reference impact is not safely determined.".into())
                } else if writable {
                    None
                } else {
                    descriptor
                        .and_then(|item| item.reason.clone())
                        .or_else(|| Some("No supported writable instance definition.".into()))
                };
                let ordinal = object_ordinals
                    .entry((path.clone(), kind.clone()))
                    .or_insert(0);
                let occurrence = *ordinal;
                *ordinal += 1;
                let object_id = old_objects
                    .get(&(source_id.clone(), path.clone(), kind.clone(), occurrence))
                    .cloned()
                    .unwrap_or_else(|| allocate("object"));
                let parent_id = node
                    .ancestors()
                    .skip(1)
                    .find_map(|ancestor| node_objects.get(&ancestor.id()).cloned());
                node_objects.insert(node.id(), object_id.clone());
                let index = snapshot.objects.len();
                snapshot.paths.entry(path.clone()).or_default().push(index);
                snapshot.object_by_id.insert(object_id.clone(), index);
                snapshot.objects.push(IndexedObject {
                    view: ObjectProjection {
                        object_id,
                        parent_id,
                        source_id: source_id.clone(),
                        path,
                        short_name: child_text(node, "SHORT-NAME").unwrap(),
                        kind,
                        definition_id,
                        writable,
                        reason,
                    },
                    source,
                    range: node.range(),
                    name_range: simple_text_range(short_name_node)
                        .unwrap_or_else(|_| short_name_node.range()),
                    unsafe_semantics,
                    ordinal: occurrence,
                });
            }
            let mut ordinals: BTreeMap<(String, String, String), usize> = BTreeMap::new();
            for node in document
                .descendants()
                .filter(|node| node.is_element() && node.tag_name().namespace() == Some(NS))
            {
                let element = node.tag_name().name();
                let is_parameter = matches!(
                    element,
                    "ECUC-NUMERICAL-PARAM-VALUE"
                        | "ECUC-TEXTUAL-PARAM-VALUE"
                        | "ECUC-REFERENCE-VALUE"
                );
                let product_reference = element == "SD"
                    && matches!(
                        node.attribute("GID"),
                        Some("SystemPduRef" | "ComSignalRef" | "MonitorFrameRef" | "SessionRef")
                    )
                    && node.parent_element().is_some_and(|parent| {
                        parent.tag_name().name() == "SDG"
                            && matches!(
                                parent.attribute("GID"),
                                Some(
                                    "AutosarWorkbenchGlobalPduV1"
                                        | "AutosarWorkbenchDiagnostic"
                                        | "AutosarWorkbenchDtc"
                                        | "AutosarWorkbenchHostRestoreDidV1"
                                )
                            )
                    });
                let is_reference = (element.ends_with("-REF")
                    && !matches!(element, "DEFINITION-REF" | "VALUE-REF"))
                    || product_reference;
                let is_scalar = !matches!(
                    element,
                    "SHORT-NAME" | "DEFINITION-REF" | "VALUE" | "VALUE-REF"
                ) && node.text().is_some()
                    && !node.children().any(|child| child.is_element());
                if !is_parameter && !is_reference && !is_scalar {
                    continue;
                }
                let Some(object_id) = node
                    .ancestors()
                    .skip(1)
                    .find_map(|ancestor| node_objects.get(&ancestor.id()))
                else {
                    continue;
                };
                let object = &snapshot.objects[*snapshot.object_by_id.get(object_id).unwrap()];
                let definition_id = if is_parameter {
                    definition(node).unwrap_or_default()
                } else {
                    format!(
                        "{}#{element}{}",
                        object.view.kind,
                        node.attribute("GID").unwrap_or("")
                    )
                };
                let value_node = if is_parameter {
                    node.children().find(|child| {
                        child.is_element()
                            && matches!(child.tag_name().name(), "VALUE" | "VALUE-REF")
                    })
                } else {
                    Some(node)
                };
                let mut descriptor =
                    self.catalog
                        .get(&definition_id)
                        .cloned()
                        .unwrap_or_else(|| {
                            readonly_definition(
                                definition_id.clone(),
                                element,
                                "Unknown definition or unsupported source field.",
                            )
                        });
                let permitted_owner = !is_parameter
                    || object.view.definition_id.as_deref().is_some_and(|id| {
                        self.catalog
                            .children(id)
                            .iter()
                            .any(|child| child.definition_id == descriptor.definition_id)
                    });
                let foreign_context = node
                    .ancestors()
                    .skip(1)
                    .take_while(|ancestor| !node_objects.contains_key(&ancestor.id()))
                    .any(|ancestor| {
                        ancestor.is_element() && ancestor.tag_name().namespace() != Some(NS)
                    });
                if !object.view.writable
                    || !permitted_owner
                    || foreign_context
                    || value_node.is_none_or(|value| value.children().any(|child| !child.is_text()))
                {
                    descriptor.writable = false;
                    descriptor.reason = Some(
                        "Unknown ownership, variant, expression or mixed content is read-only."
                            .into(),
                    );
                }
                let reference = (element == "ECUC-REFERENCE-VALUE" || is_reference).then(|| {
                    value_node
                        .map(|value| ReferenceState::Explicit {
                            raw_path: super::xml::literal_text(value).unwrap_or_default(),
                            dest: value
                                .attribute("DEST")
                                .unwrap_or(if product_reference {
                                    "product-reference"
                                } else {
                                    ""
                                })
                                .into(),
                            target: None,
                        })
                        .unwrap_or(ReferenceState::Absent)
                });
                let current = if reference.is_some() {
                    ValueState::Absent
                } else {
                    value_node
                        .map(|value| ValueState::Explicit {
                            value: TypedValue {
                                kind: descriptor.kind.unwrap_or(ValueKind::String),
                                lexeme: super::xml::literal_text(value).unwrap_or_default(),
                            },
                        })
                        .unwrap_or(ValueState::Absent)
                };
                let key = (object_id.clone(), definition_id.clone(), element.to_owned());
                let ordinal = *ordinals.entry(key.clone()).or_default();
                *ordinals.get_mut(&key).unwrap() += 1;
                let field_id = old_fields
                    .get(&(
                        object_id.clone(),
                        definition_id,
                        element.to_owned(),
                        ordinal,
                    ))
                    .cloned()
                    .unwrap_or_else(|| allocate("field"));
                snapshot
                    .field_by_id
                    .insert(field_id.clone(), snapshot.fields.len());
                snapshot.fields.push(IndexedField {
                    view: FieldDescriptor {
                        field_id,
                        object_id: object_id.clone(),
                        definition: descriptor,
                        current,
                        reference,
                    },
                    source,
                    entry_range: Some(node.range()),
                    value_range: value_node.and_then(|value| simple_text_range(value).ok()),
                    element: element.into(),
                    ordinal,
                });
            }
        }
        // Descriptors for absent fields carry default provenance without materializing XML.
        let present: BTreeSet<_> = snapshot
            .fields
            .iter()
            .filter_map(|field| {
                let descriptor = self.catalog.get(&field.view.definition.definition_id)?;
                Some((
                    *snapshot.object_by_id.get(&field.view.object_id)?,
                    descriptor.definition_id.as_str(),
                ))
            })
            .collect();
        for (object_index, object) in snapshot.objects.iter().enumerate() {
            let Some(definition_id) = &object.view.definition_id else {
                continue;
            };
            for descriptor in self
                .catalog
                .children(definition_id)
                .into_iter()
                .filter(|item| item.kind.is_some())
            {
                if present.contains(&(object_index, descriptor.definition_id.as_str())) {
                    continue;
                }
                let mut descriptor = descriptor.clone();
                if object.unsafe_semantics {
                    descriptor.writable = false;
                    descriptor.reason = object.view.reason.clone();
                }
                let element = field_element(descriptor.kind.unwrap());
                let field_id = old_fields
                    .get(&(
                        object.view.object_id.clone(),
                        descriptor.definition_id.clone(),
                        element.into(),
                        0,
                    ))
                    .cloned()
                    .unwrap_or_else(|| allocate("field"));
                let reference = (descriptor.kind == Some(ValueKind::Reference))
                    .then_some(ReferenceState::Absent);
                snapshot
                    .field_by_id
                    .insert(field_id.clone(), snapshot.fields.len());
                snapshot.fields.push(IndexedField {
                    view: FieldDescriptor {
                        field_id,
                        object_id: object.view.object_id.clone(),
                        definition: descriptor,
                        current: ValueState::Absent,
                        reference,
                    },
                    source: object.source,
                    entry_range: None,
                    value_range: None,
                    element: element.into(),
                    ordinal: 0,
                });
            }
        }
        for field in &mut snapshot.fields {
            let Some(ReferenceState::Explicit {
                raw_path,
                dest,
                target,
            }) = &mut field.view.reference
            else {
                continue;
            };
            if dest == "product-reference" {
                if let Some(indices) = snapshot
                    .paths
                    .get(raw_path)
                    .filter(|indices| indices.len() == 1)
                {
                    *dest = snapshot.objects[indices[0]].view.kind.clone();
                }
            }
            let matches = snapshot
                .paths
                .get(raw_path)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let target_id = if matches.len() == 1 && snapshot.objects[matches[0]].view.kind == *dest
            {
                Some(snapshot.objects[matches[0]].view.object_id.clone())
            } else {
                None
            };
            *target = target_id
                .clone()
                .map(|object_id| ObjectRef::Existing { object_id });
            snapshot.references.push(ReferenceEdge {
                object_id: field.view.object_id.clone(),
                field_id: field.view.field_id.clone(),
                raw_path: raw_path.clone(),
                dest: dest.clone(),
                target_id: target_id.clone(),
                reason: target_id
                    .is_none()
                    .then(|| "Reference path is missing, ambiguous or has the wrong DEST.".into()),
            });
        }
        let files: Vec<_> = self
            .files
            .iter()
            .map(|file| (file.path.as_path(), file.text.as_str()))
            .collect();
        let safety = ScopeValidation {
            scope: ValidationScope::SourceSafety,
            status: ValidationStatus::Passed,
            coverage: vec![RuleCoverage {
                rule_id: "source-safety".into(),
                scope: ValidationScope::SourceSafety,
                subjects: Vec::new(),
                supported: true,
                reason: None,
            }],
            diagnostics: Vec::new(),
        };
        match crate::rules::validate_native(&files) {
            Ok(native) => {
                snapshot.validation = vec![safety, native, self.catalog.validate_documents(&files)?]
            }
            Err(message) => {
                snapshot.rule_fault = Some(message.clone());
                let diagnostic = ConfigurationDiagnostic {
                    scope: ValidationScope::Schema, rule_id: "builtin.inventory".into(), severity: Severity::Error,
                    code: "BUILTIN_RULES".into(), message, remedy: "Repair the installed matching product inventory; official archives cannot replace it.".into(),
                    file: None, path: None, source_id: None, object_id: None, field_id: None, witness: None,
                };
                snapshot.validation = vec![
                    safety,
                    ScopeValidation {
                        scope: ValidationScope::Schema,
                        status: ValidationStatus::NotRun,
                        coverage: Vec::new(),
                        diagnostics: vec![diagnostic],
                    },
                    ScopeValidation {
                        scope: ValidationScope::Definition,
                        status: ValidationStatus::NotRun,
                        coverage: Vec::new(),
                        diagnostics: Vec::new(),
                    },
                ];
                for object in &mut snapshot.objects {
                    object.view.writable = false;
                    object.view.reason = Some("Product rule inventory is unavailable; source-only viewing remains available.".into());
                }
                for field in &mut snapshot.fields {
                    field.view.definition.writable = false;
                    field.view.definition.reason = Some("Product rule inventory is unavailable; source-only viewing remains available.".into());
                }
            }
        }
        if let Some(project) = &self.project {
            if let Some(scope) = snapshot
                .validation
                .iter_mut()
                .find(|scope| scope.scope == ValidationScope::Definition)
            {
                scope
                    .diagnostics
                    .extend(project.extension_diagnostics.clone());
                if scope
                    .diagnostics
                    .iter()
                    .any(|issue| matches!(issue.severity, Severity::Error))
                {
                    scope.status = ValidationStatus::Failed;
                }
            }
        }
        snapshot.profile = if snapshot.integration_candidate {
            crate::integration::PROFILE.into()
        } else if !self.frames.is_empty()
            || self.files.iter().any(|file| self.is_managed_file(file))
        {
            "host-can-v1".into()
        } else {
            "unrecognized".into()
        };
        let target_diagnostics = self
            .issues
            .iter()
            .filter(|_| snapshot.profile != crate::integration::PROFILE)
            .map(|issue| ConfigurationDiagnostic {
                scope: ValidationScope::TargetGeneration,
                rule_id: issue.code.clone(),
                severity: issue.severity.clone(),
                code: issue.code.clone(),
                message: issue.message.clone(),
                remedy: "Resolve the target-specific input constraint before generating.".into(),
                file: issue.file.clone(),
                path: issue.path.clone(),
                source_id: None,
                object_id: None,
                field_id: None,
                witness: None,
            })
            .collect::<Vec<_>>();
        snapshot.validation.push(ScopeValidation {
            scope: ValidationScope::TargetGeneration,
            status: if snapshot.rule_fault.is_some() {
                ValidationStatus::NotRun
            } else if target_diagnostics
                .iter()
                .any(|issue| matches!(issue.severity, Severity::Error))
            {
                ValidationStatus::Failed
            } else {
                ValidationStatus::NotRun
            },
            coverage: Vec::new(),
            diagnostics: target_diagnostics,
        });
        for scope in &mut snapshot.validation {
            for diagnostic in &mut scope.diagnostics {
                bind_diagnostic(
                    diagnostic,
                    &snapshot.sources,
                    &snapshot.objects,
                    &snapshot.fields,
                );
            }
        }
        for field in snapshot.fields.iter().filter(|field| {
            field.view.definition.writable
                && field.view.definition.kind == Some(ValueKind::Reference)
        }) {
            let definitions = self
                .catalog
                .reference_target_definitions(&field.view.definition.definition_id);
            for target in snapshot.objects.iter().filter(|target| {
                !target.unsafe_semantics
                    && field
                        .view
                        .definition
                        .reference_destinations
                        .contains(&target.view.kind)
                    && (definitions.is_empty()
                        || target
                            .view
                            .definition_id
                            .as_ref()
                            .is_some_and(|id| definitions.contains(id)))
                    && snapshot
                        .paths
                        .get(&target.view.path)
                        .is_some_and(|indices| indices.len() == 1)
            }) {
                snapshot.reference_candidates.push(ReferenceCandidate {
                    field_id: field.view.field_id.clone(),
                    target_id: target.view.object_id.clone(),
                    path: target.view.path.clone(),
                    short_name: target.view.short_name.clone(),
                    dest: target.view.kind.clone(),
                    definition_id: target.view.definition_id.clone(),
                });
            }
        }
        let templates: BTreeMap<_, _> = self
            .catalog
            .definitions()
            .filter(|definition| definition.kind.is_none() && definition.writable)
            .map(|definition| {
                (
                    definition.definition_id.clone(),
                    definition_template(&self.catalog, &snapshot, definition),
                )
            })
            .collect();
        for parent in snapshot
            .objects
            .iter()
            .filter(|parent| parent.view.writable)
        {
            let definitions: Vec<_> = if parent.view.kind == "AR-PACKAGE" {
                self.catalog
                    .definitions()
                    .filter(|definition| definition.element_kind == "ECUC-MODULE-DEF")
                    .collect()
            } else if let Some(id) = &parent.view.definition_id {
                self.catalog.children(id)
            } else {
                Vec::new()
            };
            for definition in definitions
                .into_iter()
                .filter(|definition| definition.kind.is_none() && definition.writable)
            {
                let template = templates
                    .get(&definition.definition_id)
                    .expect("Writable instance metadata has a template");
                let prefix = format!("{}/", definition.definition_id);
                let recursive_definitions = templates
                    .range(prefix.clone()..)
                    .take_while(|(id, _)| id.starts_with(&prefix))
                    .map(|(_, template)| template.clone())
                    .collect();
                snapshot
                    .instance_definitions
                    .push(InstanceDefinitionOption {
                        parent_id: parent.view.object_id.clone(),
                        definition: definition.clone(),
                        fields: template.fields.clone(),
                        children: template.children.clone(),
                        reference_options: template.reference_options.clone(),
                        recursive_definitions,
                    });
            }
        }
        let accepted = self.catalog.accepted_extensions();
        if !accepted.is_empty() {
            let owners: BTreeMap<_, _> = self
                .catalog
                .definitions()
                .map(|definition| {
                    (
                        definition.definition_id.as_str(),
                        self.catalog
                            .required_extensions(&[definition.definition_id.clone()]),
                    )
                })
                .collect();
            let mut consumers: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
            for (definition, object_id) in snapshot
                .objects
                .iter()
                .filter_map(|object| {
                    object
                        .view
                        .definition_id
                        .as_deref()
                        .map(|definition| (definition, &object.view.object_id))
                })
                .chain(snapshot.fields.iter().map(|field| {
                    (
                        field.view.definition.definition_id.as_str(),
                        &field.view.object_id,
                    )
                }))
            {
                if let Some(identities) = owners.get(definition) {
                    for identity in identities {
                        consumers
                            .entry(&identity.catalog_id)
                            .or_default()
                            .insert(object_id.clone());
                    }
                }
            }
            for identity in accepted {
                let available = owners
                    .values()
                    .any(|identities| identities.contains(&identity));
                let owned = consumers
                    .remove(identity.catalog_id.as_str())
                    .unwrap_or_default();
                snapshot.extension_definitions.push(ExtensionDefinitionView { identity, source: None,
                    consumers: owned.into_iter().collect(), available,
                    reason: (!available).then(|| "Exact catalog metadata is unavailable; affected consumers remain read-only until explicit restoration.".into()) });
            }
        }
        snapshot.definition_fingerprint = self
            .catalog
            .fingerprint(&crate::rules::trusted_rule_set_identity());
        self.next_identity = next_identity;
        self.snapshot = Arc::new(snapshot);
        Ok(())
    }

    pub fn source_text(&self, source_id: &str) -> Result<String, String> {
        let source = self
            .snapshot
            .sources
            .iter()
            .position(|source| source.source_id == source_id)
            .ok_or("Source ID does not belong to this workspace epoch.")?;
        Ok(self.files[source].text.clone())
    }

    pub fn project_projection(&self, input_fingerprint: &str) -> Result<ProjectProjection, String> {
        let identity = crate::rules::rule_set_identity()
            .unwrap_or_else(|_| crate::rules::trusted_rule_set_identity());
        Ok(ProjectProjection {
            workspace_epoch: self.epoch.clone(),
            input_fingerprint: input_fingerprint.into(),
            definition_fingerprint: self.snapshot.definition_fingerprint.clone(),
            release: identity.release.clone(),
            rule_set_identity: identity,
            profile: self.snapshot.profile.clone(),
            sources: self.snapshot.sources.clone(),
            project_path: self
                .project
                .as_ref()
                .map(|project| project.path.display().to_string()),
            objects: self
                .snapshot
                .objects
                .iter()
                .map(|object| object.view.clone())
                .collect(),
            fields: self
                .snapshot
                .fields
                .iter()
                .map(|field| field.view.clone())
                .collect(),
            references: self.snapshot.references.clone(),
            reference_candidates: self.snapshot.reference_candidates.clone(),
            instance_definitions: self.snapshot.instance_definitions.clone(),
            extension_definitions: self.snapshot.extension_definitions.clone(),
            diagnostics: self
                .snapshot
                .validation
                .iter()
                .flat_map(|scope| scope.diagnostics.clone())
                .collect(),
            validation: self.snapshot.validation.clone(),
            capabilities: vec![
                ActionCapability {
                    action: "prepare-change".into(),
                    available: self.snapshot.rule_fault.is_none(),
                    reason: self.snapshot.rule_fault.clone(),
                },
                ActionCapability {
                    action: "save".into(),
                    available: self.snapshot.rule_fault.is_none()
                        && !self.snapshot.validation.iter().any(|scope| {
                            scope.scope == ValidationScope::Schema
                                && scope
                                    .diagnostics
                                    .iter()
                                    .any(|issue| matches!(issue.severity, Severity::Error))
                        }),
                    reason: self.snapshot.rule_fault.clone(),
                },
                ActionCapability {
                    action: "source-text".into(),
                    available: true,
                    reason: None,
                },
            ],
            accepted_extension_definitions: self.catalog.accepted_extensions(),
            dirty: self.is_dirty(),
        })
    }

    pub fn input_fingerprint(&self) -> Result<String, String> {
        self.input_fingerprint_for_project(self.revision, self.project.as_ref())
    }

    pub(super) fn input_fingerprint_for_project(
        &self,
        revision: u64,
        membership: Option<&super::project::ProjectMembership>,
    ) -> Result<String, String> {
        let identity = crate::rules::rule_set_identity()?;
        let mut digest = Sha256::new();
        for bytes in [
            self.epoch.as_bytes(),
            &revision.to_le_bytes(),
            self.save_revision_for_project(membership).as_bytes(),
            serde_json::to_string(&identity)
                .map_err(|error| error.to_string())?
                .as_bytes(),
            self.snapshot.definition_fingerprint.as_bytes(),
        ] {
            digest.update((bytes.len() as u64).to_le_bytes());
            digest.update(bytes);
        }
        Ok(format!("{:x}", digest.finalize()))
    }

    pub(super) fn is_dirty(&self) -> bool {
        self.files.iter().any(|file| file.saved != file.text)
            || self
                .project
                .as_ref()
                .is_some_and(|project| project.current != project.saved)
            || (self.project.is_none() && !self.catalog.accepted_extensions().is_empty())
    }
}

pub(super) fn field_element(kind: ValueKind) -> &'static str {
    match kind {
        ValueKind::Integer | ValueKind::Float | ValueKind::Boolean => "ECUC-NUMERICAL-PARAM-VALUE",
        ValueKind::Reference => "ECUC-REFERENCE-VALUE",
        _ => "ECUC-TEXTUAL-PARAM-VALUE",
    }
}

fn bind_diagnostic(
    diagnostic: &mut ConfigurationDiagnostic,
    sources: &[SourceProjection],
    objects: &[IndexedObject],
    fields: &[IndexedField],
) {
    diagnostic.source_id = diagnostic
        .file
        .as_ref()
        .and_then(|file| sources.iter().find(|source| source.path == *file))
        .map(|source| source.source_id.clone());
    let matching: Vec<_> = objects
        .iter()
        .filter(|object| {
            diagnostic
                .path
                .as_ref()
                .is_some_and(|path| *path == object.view.path)
                && diagnostic
                    .source_id
                    .as_ref()
                    .is_none_or(|source| *source == object.view.source_id)
        })
        .collect();
    if matching.len() == 1 {
        diagnostic.object_id = Some(matching[0].view.object_id.clone());
    }
    if let Some(witness) = &mut diagnostic.witness {
        let mut normalized = Vec::with_capacity(witness.subjects.len() + 2);
        for subject in std::mem::take(&mut witness.subjects) {
            if let Some((start, end)) = subject
                .strip_prefix("entry-range:")
                .and_then(|range| range.split_once(':'))
                .and_then(|(start, end)| {
                    Some((start.parse::<usize>().ok()?, end.parse::<usize>().ok()?))
                })
            {
                let range = start..end;
                if let Some(field) = fields.iter().find(|field| {
                    field.entry_range.as_ref() == Some(&range)
                        && diagnostic
                            .source_id
                            .as_ref()
                            .is_some_and(|id| sources[field.source].source_id == *id)
                }) {
                    diagnostic.field_id = Some(field.view.field_id.clone());
                    diagnostic.object_id = Some(field.view.object_id.clone());
                    normalized.push(field.view.field_id.clone());
                } else if let Some(object) = objects.iter().find(|object| {
                    object.range == range
                        && diagnostic
                            .source_id
                            .as_ref()
                            .is_some_and(|id| object.view.source_id == *id)
                }) {
                    diagnostic.object_id = Some(object.view.object_id.clone());
                    normalized.push(object.view.object_id.clone());
                }
                continue;
            }
            let matches: Vec<_> = objects
                .iter()
                .filter(|object| object.view.path == subject)
                .collect();
            if matches.len() == 1 {
                normalized.push(matches[0].view.object_id.clone());
            } else {
                normalized.push(subject);
            }
        }
        witness.subjects = normalized;
        if let Some(object) = &diagnostic.object_id {
            witness.subjects.push(object.clone());
            let definition = serde_json::from_str::<DefinitionDescriptor>(&witness.constraint).ok();
            if diagnostic.field_id.is_none() {
                if let Some(definition) = definition {
                    let matching: Vec<_> = fields
                        .iter()
                        .filter(|field| {
                            field.view.object_id == *object
                                && field.view.definition.definition_id == definition.definition_id
                        })
                        .collect();
                    if matching.len() == 1 {
                        diagnostic.field_id = Some(matching[0].view.field_id.clone());
                        witness.subjects.push(matching[0].view.field_id.clone());
                    } else {
                        // Ambiguous provenance must not let an identical old error move
                        // to another entry. Exact range markers above avoid this branch.
                        for field in matching {
                            witness.subjects.push(field.view.field_id.clone());
                        }
                    }
                }
            }
        } else if let Some(source) = &diagnostic.source_id {
            witness.subjects.push(source.clone());
        }
        witness.subjects.sort();
        witness.subjects.dedup();
    }
}
