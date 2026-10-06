export type Appearance = 'light' | 'dark' | 'system';
export type RuleSetIdentity = { release: string; rulesVersion: string; sha256: string };
export type ValidationScope = 'source-safety' | 'schema' | 'definition' | 'target-generation';
export type ValidationStatus = 'passed' | 'failed' | 'unsupported' | 'not_run';
export type RuleCoverage = {
  ruleId: string;
  scope: ValidationScope;
  subjects: string[];
  supported: boolean;
  reason: string | null;
};
export type ActionCapability = { action: string; available: boolean; reason: string | null };
export type ValueKind =
  'integer' | 'float' | 'boolean' | 'enumeration' | 'string' | 'function-name' | 'reference';
export type TypedValue = { kind: ValueKind; lexeme: string };
export type ValueState = { state: 'absent' } | { state: 'explicit'; value: TypedValue };
export type ObjectRef =
  { kind: 'existing'; objectId: string } | { kind: 'created'; changeId: string };
export type FieldRef =
  | { kind: 'existing'; fieldId: string }
  | { kind: 'new'; object: ObjectRef; definitionId: string; entryKey: string };
export type ReferenceState =
  | { state: 'absent' }
  | { state: 'explicit'; rawPath: string; dest: string; target: ObjectRef | null };
export type DefinitionDescriptor = {
  definitionId: string;
  elementKind: string;
  kind: ValueKind | null;
  lowerMultiplicity: number;
  upperMultiplicity: number | null;
  unit: string | null;
  minimum: string | null;
  maximum: string | null;
  enumeration: string[];
  defaultValue: TypedValue | null;
  defaultOrigin: string | null;
  referenceDestinations: string[];
  writable: boolean;
  reason: string | null;
};
export type FieldDescriptor = DefinitionDescriptor & {
  fieldId: string;
  objectId: string;
  current: ValueState;
  reference: ReferenceState | null;
};
export type SourceProjection = {
  sourceId: string;
  path: string;
  readonly: boolean;
  sha256: string;
};
export type ObjectProjection = {
  objectId: string;
  parentId: string | null;
  sourceId: string;
  path: string;
  shortName: string;
  kind: string;
  definitionId: string | null;
  writable: boolean;
  reason: string | null;
};
export type ReferenceEdge = {
  objectId: string;
  fieldId: string;
  rawPath: string;
  dest: string;
  targetId: string | null;
  reason: string | null;
};
export type ConfigurationDiagnostic = {
  scope: ValidationScope;
  ruleId: string;
  severity: 'error' | 'warning' | 'info';
  code: string;
  message: string;
  remedy: string;
  file: string | null;
  path: string | null;
  sourceId: string | null;
  objectId: string | null;
  fieldId: string | null;
  witness: {
    ruleId: string;
    subjects: string[];
    constraint: string;
    counterexample: string;
  } | null;
};
export type ReferenceCandidate = {
  fieldId: string;
  targetId: string;
  path: string;
  shortName: string;
  dest: string;
  definitionId: string | null;
};
export type ReferenceTarget = {
  targetId: string;
  path: string;
  shortName: string;
  dest: string;
  definitionId: string | null;
};
export type CreatedReferenceTarget = { definitionId: string; dest: string };
export type CreationReferenceOption = {
  fieldDefinitionId: string;
  existingTargets: ReferenceTarget[];
  createdTargets: CreatedReferenceTarget[];
};
export type InstanceDefinitionTemplate = {
  definition: DefinitionDescriptor;
  fields: DefinitionDescriptor[];
  children: DefinitionDescriptor[];
  referenceOptions: CreationReferenceOption[];
};
export type InstanceDefinitionOption = InstanceDefinitionTemplate & {
  parentId: string;
  recursiveDefinitions: InstanceDefinitionTemplate[];
};
export type ExtensionDefinitionView = {
  identity: ExtensionDefinitionIdentity;
  source: string | null;
  consumers: string[];
  available: boolean;
  reason: string | null;
};
export type ScopeValidation = {
  scope: ValidationScope;
  status: ValidationStatus;
  coverage: RuleCoverage[];
  diagnostics: ConfigurationDiagnostic[];
};
export type ExtensionDefinitionIdentity = {
  catalogId: string;
  release: string;
  version: string;
  sha256: string;
};
export type ProjectProjection = {
  workspaceEpoch: string;
  inputFingerprint: string;
  ruleSetIdentity: RuleSetIdentity;
  definitionFingerprint: string;
  release: string;
  profile: string;
  projectPath: string | null;
  referenceCandidates: ReferenceCandidate[];
  instanceDefinitions: InstanceDefinitionOption[];
  extensionDefinitions: ExtensionDefinitionView[];
  sources: SourceProjection[];
  objects: ObjectProjection[];
  fields: FieldDescriptor[];
  references: ReferenceEdge[];
  diagnostics: ConfigurationDiagnostic[];
  validation: ScopeValidation[];
  capabilities: ActionCapability[];
  acceptedExtensionDefinitions: ExtensionDefinitionIdentity[];
  dirty: boolean;
};
export type ConfigurationChange =
  | { op: 'set-value'; changeId: string; field: FieldRef; expected: ValueState; value: ValueState }
  | {
      op: 'set-reference';
      changeId: string;
      field: FieldRef;
      expected: ReferenceState;
      value: ReferenceState;
    }
  | {
      op: 'create-instance';
      changeId: string;
      parent: ObjectRef;
      sourceId: string;
      definitionId: string;
      shortName: string;
    }
  | {
      op: 'rename-instance';
      changeId: string;
      object: ObjectRef;
      expectedShortName: string;
      shortName: string;
    }
  | { op: 'remove-instance'; changeId: string; object: ObjectRef; expectedShortName: string };
export type ChangeSet = {
  workspaceEpoch: string;
  inputFingerprint: string;
  definitionFingerprint: string;
  changes: ConfigurationChange[];
};
export type ChangeImpact = {
  changeId: string;
  sourceId: string;
  objectId: string | null;
  fieldId: string | null;
  path: string;
  before: string;
  after: string;
  incoming: boolean;
};
export type ChangePreview = {
  changeRevision: string;
  inputFingerprint: string;
  definitionFingerprint: string;
  impacts: ChangeImpact[];
  diagnostics: ConfigurationDiagnostic[];
};
export type ChangeOutcome = {
  projection: ProjectProjection;
  selectionId: string | null;
  createdIds: { changeId: string; objectId: string }[];
  createdFields: { entryKey: string; fieldId: string }[];
};
export type DocumentKind =
  | 'configuration'
  | 'communication'
  | 'diagnostic'
  | 'integration'
  | 'delivery'
  | 'source'
  | 'project-entry';
export type DocumentTab = { kind: DocumentKind; sourceId?: string };
export type ToolWindow = 'problems' | 'generation' | 'build' | 'host' | 'log';
export type DraftGuard = { kind: 'context' | 'project'; title: string };
export type ProjectCreationPreview = {
  revision: string;
  templateId: string;
  directory: string;
  name: string;
  files: { path: string; contents: string }[];
  acceptedExtensionDefinitions: ExtensionDefinitionIdentity[];
};
export type ApplicationSlotDescriptor = {
  producerSlot: string;
  componentPath: string;
  sourcePaths: string[];
  generatedHeaders: string[];
  entrySymbols: string[];
};
export type ApplicationInitializationPreview = {
  revision: string;
  slot: ApplicationSlotDescriptor;
  files: { path: string; contents: string }[];
  manifestBefore: string;
  manifestAfter: string;
};
export type ApplicationInitializationOutcome = {
  projection: ProjectProjection;
  warnings: string[];
  retainedRecoveryFiles: string[];
};
