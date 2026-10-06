use crate::model::Severity;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleSetIdentity {
    pub release: String,
    pub rules_version: String,
    pub sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValidationScope {
    SourceSafety,
    Schema,
    Definition,
    TargetGeneration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Passed,
    Failed,
    Unsupported,
    NotRun,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleCoverage {
    pub rule_id: String,
    pub scope: ValidationScope,
    pub subjects: Vec<String>,
    pub supported: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationWitness {
    pub rule_id: String,
    pub subjects: Vec<String>,
    pub constraint: String,
    pub counterexample: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationDiagnostic {
    pub scope: ValidationScope,
    pub rule_id: String,
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub remedy: String,
    pub file: Option<String>,
    pub path: Option<String>,
    pub source_id: Option<String>,
    pub object_id: Option<String>,
    pub field_id: Option<String>,
    pub witness: Option<ValidationWitness>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeValidation {
    pub scope: ValidationScope,
    pub status: ValidationStatus,
    pub coverage: Vec<RuleCoverage>,
    pub diagnostics: Vec<ConfigurationDiagnostic>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValueKind {
    Integer,
    Float,
    Boolean,
    Enumeration,
    String,
    FunctionName,
    Reference,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypedValue {
    pub kind: ValueKind,
    pub lexeme: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum ValueState {
    Absent,
    Explicit { value: TypedValue },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
pub enum ObjectRef {
    Existing { object_id: String },
    Created { change_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
pub enum FieldRef {
    Existing {
        field_id: String,
    },
    New {
        object: ObjectRef,
        definition_id: String,
        entry_key: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
pub enum ReferenceState {
    Absent,
    Explicit {
        raw_path: String,
        dest: String,
        target: Option<ObjectRef>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionDescriptor {
    pub definition_id: String,
    pub element_kind: String,
    pub kind: Option<ValueKind>,
    pub lower_multiplicity: u32,
    pub upper_multiplicity: Option<u32>,
    pub unit: Option<String>,
    pub minimum: Option<String>,
    pub maximum: Option<String>,
    pub enumeration: Vec<String>,
    pub default_value: Option<TypedValue>,
    pub default_origin: Option<String>,
    pub reference_destinations: Vec<String>,
    pub writable: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDescriptor {
    pub field_id: String,
    pub object_id: String,
    #[serde(flatten)]
    pub definition: DefinitionDescriptor,
    pub current: ValueState,
    pub reference: Option<ReferenceState>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceProjection {
    pub source_id: String,
    pub path: String,
    pub readonly: bool,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectProjection {
    pub object_id: String,
    pub parent_id: Option<String>,
    pub source_id: String,
    pub path: String,
    pub short_name: String,
    pub kind: String,
    pub definition_id: Option<String>,
    pub writable: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceEdge {
    pub object_id: String,
    pub field_id: String,
    pub raw_path: String,
    pub dest: String,
    pub target_id: Option<String>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionDefinitionIdentity {
    pub catalog_id: String,
    pub release: String,
    pub version: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionCapability {
    pub action: String,
    pub available: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceCandidate {
    pub field_id: String,
    pub target_id: String,
    pub path: String,
    pub short_name: String,
    pub dest: String,
    pub definition_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceTarget {
    pub target_id: String,
    pub path: String,
    pub short_name: String,
    pub dest: String,
    pub definition_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedReferenceTarget {
    pub definition_id: String,
    pub dest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreationReferenceOption {
    pub field_definition_id: String,
    pub existing_targets: Vec<ReferenceTarget>,
    pub created_targets: Vec<CreatedReferenceTarget>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceDefinitionTemplate {
    pub definition: DefinitionDescriptor,
    pub fields: Vec<DefinitionDescriptor>,
    pub children: Vec<DefinitionDescriptor>,
    pub reference_options: Vec<CreationReferenceOption>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceDefinitionOption {
    pub parent_id: String,
    pub definition: DefinitionDescriptor,
    pub fields: Vec<DefinitionDescriptor>,
    pub children: Vec<DefinitionDescriptor>,
    pub reference_options: Vec<CreationReferenceOption>,
    pub recursive_definitions: Vec<InstanceDefinitionTemplate>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionDefinitionView {
    pub identity: ExtensionDefinitionIdentity,
    pub source: Option<String>,
    pub consumers: Vec<String>,
    pub available: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProjection {
    pub workspace_epoch: String,
    pub input_fingerprint: String,
    pub rule_set_identity: RuleSetIdentity,
    pub definition_fingerprint: String,
    pub release: String,
    pub profile: String,
    pub project_path: Option<String>,
    pub sources: Vec<SourceProjection>,
    pub objects: Vec<ObjectProjection>,
    pub fields: Vec<FieldDescriptor>,
    pub references: Vec<ReferenceEdge>,
    pub reference_candidates: Vec<ReferenceCandidate>,
    pub instance_definitions: Vec<InstanceDefinitionOption>,
    pub extension_definitions: Vec<ExtensionDefinitionView>,
    pub diagnostics: Vec<ConfigurationDiagnostic>,
    pub validation: Vec<ScopeValidation>,
    pub capabilities: Vec<ActionCapability>,
    pub accepted_extension_definitions: Vec<ExtensionDefinitionIdentity>,
    pub dirty: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case", rename_all_fields = "camelCase")]
pub enum ConfigurationChange {
    SetValue {
        change_id: String,
        field: FieldRef,
        expected: ValueState,
        value: ValueState,
    },
    SetReference {
        change_id: String,
        field: FieldRef,
        expected: ReferenceState,
        value: ReferenceState,
    },
    CreateInstance {
        change_id: String,
        parent: ObjectRef,
        source_id: String,
        definition_id: String,
        short_name: String,
    },
    RenameInstance {
        change_id: String,
        object: ObjectRef,
        expected_short_name: String,
        short_name: String,
    },
    RemoveInstance {
        change_id: String,
        object: ObjectRef,
        expected_short_name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSet {
    pub workspace_epoch: String,
    pub input_fingerprint: String,
    pub definition_fingerprint: String,
    pub changes: Vec<ConfigurationChange>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeImpact {
    pub change_id: String,
    pub source_id: String,
    pub object_id: Option<String>,
    pub field_id: Option<String>,
    pub path: String,
    pub before: String,
    pub after: String,
    pub incoming: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePreview {
    pub change_revision: String,
    pub input_fingerprint: String,
    pub definition_fingerprint: String,
    pub impacts: Vec<ChangeImpact>,
    pub diagnostics: Vec<ConfigurationDiagnostic>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedObject {
    pub change_id: String,
    pub object_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedField {
    pub entry_key: String,
    pub field_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeOutcome {
    pub projection: ProjectProjection,
    pub selection_id: Option<String>,
    pub created_ids: Vec<CreatedObject>,
    pub created_fields: Vec<CreatedField>,
}
