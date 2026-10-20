import type { Text } from '../i18n';
export type {
  Appearance,
  RuleSetIdentity,
  ValidationScope,
  ValidationStatus,
  RuleCoverage,
  ActionCapability,
  ValueKind,
  TypedValue,
  ValueState,
  ObjectRef,
  FieldRef,
  ReferenceState,
  DefinitionDescriptor,
  FieldDescriptor,
  SourceProjection,
  ObjectProjection,
  ReferenceEdge,
  ConfigurationDiagnostic,
  ReferenceCandidate,
  ReferenceTarget,
  CreatedReferenceTarget,
  CreationReferenceOption,
  InstanceDefinitionTemplate,
  InstanceDefinitionOption,
  ExtensionDefinitionView,
  ScopeValidation,
  ExtensionDefinitionIdentity,
  ProjectProjection,
  ConfigurationChange,
  ChangeSet,
  ChangeImpact,
  ChangePreview,
  ChangeOutcome,
  ProjectCreationPreview,
  ApplicationSlotDescriptor,
  ApplicationInitializationPreview,
  ApplicationInitializationOutcome,
} from '../ipc/bindings';

export type DocumentKind =
  'configuration' | 'integration' | 'delivery' | 'source' | 'project-entry';
export type DocumentTab = { kind: DocumentKind; sourceId?: string };
export type ToolWindow = 'problems' | 'generation' | 'build' | 'host' | 'log';
export type DraftGuard = { kind: 'context' | 'project'; title: Text };
/** An extension definition catalog waiting for the user's decision; `subject` is its path or ID. */
export type CatalogConfirmation = { kind: 'accept' | 'remove'; subject: string };
