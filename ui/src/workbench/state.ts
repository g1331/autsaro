import type {
  BuildResult,
  BuildTarget,
  ExecutionTools,
  GenerateResult,
  GenerationPreview,
  IntegrationInspection,
  Issue,
  PlanDiagnostic,
  PreflightReport,
  SavePreview,
  VirtualResult,
  WorkbenchCapabilities,
  WorkspaceView,
} from '../types';
import type {
  DiagnosticFields,
  Draft,
  DtcFields,
  FrameFields,
  Notice,
  Selection,
  SignalFields,
  Stage,
  StageRecord,
} from './forms';
import { diagnosticFields, dtcFields, newFrame, newSignal } from './forms';
import type {
  Appearance,
  ApplicationInitializationPreview,
  ChangePreview,
  ChangeSet,
  ConfigurationChange,
  DocumentTab,
  DraftGuard,
  ProjectCreationPreview,
  ProjectProjection,
  ToolWindow,
} from './projectTypes';
import { stageDefaults } from './useDelivery';
export type WorkbenchState = {
  workspace: WorkspaceView | null;
  integrationUnapplied: boolean;
  integrationProcessing: boolean;
  selection: Selection | null;
  draft: Draft;
  diagnosticDraft: DiagnosticFields;
  diagnosticSignal: string;
  diagnosticError: string;
  dtcDraft: DtcFields;
  dtcError: string;
  source: 'empty' | 'import' | 'save-as';
  projectName: string;
  projectDirectory: string;
  importPaths: string[];
  importPathText: string;
  creating: 'frame' | 'signal' | null;
  frameInput: FrameFields;
  signalInput: SignalFields;
  stages: Record<Stage, StageRecord>;
  generated: GenerateResult | null;
  generationPreview: GenerationPreview | null;
  generationPreviewPath: string;
  generationKind: 'project' | 'handoff';
  handoffGenerated: boolean;
  legacyTarget: BuildTarget;
  buildDirectory: string;
  built: BuildResult | null;
  virtualResult: VirtualResult | null;
  virtualKind: 'signal' | 'diagnostic' | null;
  operationIssues: Issue[];
  notice: Notice;
  busy: string | null;
  peerDirectory: string;
  peerBinaryPath: string;
  savePreview: SavePreview | null;
  previewPath: string;
  capabilities: WorkbenchCapabilities | null;
  settingsOpen: boolean;
  resourceDraft: { xsdArchive: string; modArchive: string };
  toolDraft: ExecutionTools;
  settingsNotice: string;
  operationGeneration: number;
  integrationInspection: IntegrationInspection | null;
  integrationIssues: PlanDiagnostic[];
  integrationIds: Record<string, string>;
  integrationPeriod: string;
  integrationPreview: SavePreview | null;
  integrationPreviewPath: string;
  integrationNotice: string;
  ecuOutputDirectory: string;
  ecuImportDirectory: string;
  handoffImportOpen: boolean;
  handoffImportDestination: string;
  handoffImportMode: 'v2' | 'legacy';
  projection: ProjectProjection | null;
  objectSelection: string[];
  activeObjectId: string | null;
  activeSourceId: string | null;
  sourceText: string | null;
  changes: ConfigurationChange[];
  changePreview: ChangePreview | null;
  preparedChangeSet: ChangeSet | null;
  guard: DraftGuard | null;
  tabs: DocumentTab[];
  activeDocument: DocumentTab;
  toolWindow: ToolWindow | null;
  treeVisible: boolean;
  inspectorVisible: boolean;
  inspectorTab: 'properties' | 'references';
  treeMode: 'objects' | 'files';
  treeFilter: string;
  treeRevealId: string | null;
  objectFilter: string;
  appearanceDraft: Appearance;
  preflight: PreflightReport | null;
  operationLog: { command: string; outcome: 'done' | 'failed'; detail: string }[];
  templateId: 'can-empty-v1' | 'can-signals-v1' | 'standard-ecu-v1';
  projectPreview: ProjectCreationPreview | null;
  projectPreviewKind: 'create' | 'save-as';
  applicationPreview: ApplicationInitializationPreview | null;
  applicationWarnings: string[];
  applicationRecoveryFiles: string[];
  replacementReady: boolean;
  savingForReplacement: boolean;
  memberProjectPath: string | null;
};

type Action = {
  type: 'patch';
  patch: Partial<WorkbenchState> | ((state: WorkbenchState) => Partial<WorkbenchState>);
};
export function reducer(state: WorkbenchState, action: Action): WorkbenchState {
  return { ...state, ...(typeof action.patch === 'function' ? action.patch(state) : action.patch) };
}
export function initialState(): WorkbenchState {
  return {
    workspace: null,
    integrationUnapplied: false,
    integrationProcessing: false,
    selection: null,
    draft: null,
    diagnosticDraft: diagnosticFields(null),
    diagnosticSignal: '',
    diagnosticError: '',
    dtcDraft: dtcFields(null),
    dtcError: '',
    source: 'empty',
    projectName: '',
    projectDirectory: '',
    importPaths: [],
    importPathText: '',
    creating: null,
    frameInput: newFrame,
    signalInput: newSignal,
    stages: stageDefaults,
    generated: null,
    generationPreview: null,
    generationPreviewPath: '',
    generationKind: 'project',
    handoffGenerated: false,
    legacyTarget: 'windows-x64-controlled-v1',
    buildDirectory: '',
    built: null,
    virtualResult: null,
    virtualKind: null,
    operationIssues: [],
    notice: null,
    busy: null,
    peerDirectory: '',
    peerBinaryPath: '',
    savePreview: null,
    previewPath: '',
    capabilities: null,
    settingsOpen: false,
    resourceDraft: { xsdArchive: '', modArchive: '' },
    toolDraft: { compiler: '', objdump: '', git: '', python: '' },
    settingsNotice: '',
    operationGeneration: 0,
    integrationInspection: null,
    integrationIssues: [],
    integrationIds: {},
    integrationPeriod: '',
    integrationPreview: null,
    integrationPreviewPath: '',
    integrationNotice: '尚未检查标准输入',
    ecuOutputDirectory: '',
    ecuImportDirectory: '',
    handoffImportOpen: false,
    handoffImportDestination: '',
    handoffImportMode: 'v2',
    projection: null,
    objectSelection: [],
    activeObjectId: null,
    activeSourceId: null,
    sourceText: null,
    changes: [],
    changePreview: null,
    preparedChangeSet: null,
    guard: null,
    tabs: [{ kind: 'configuration' }],
    activeDocument: { kind: 'configuration' },
    toolWindow: null,
    treeVisible: true,
    inspectorVisible: true,
    inspectorTab: 'properties',
    treeMode: 'objects',
    treeFilter: '',
    treeRevealId: null,
    objectFilter: '',
    appearanceDraft: 'system',
    preflight: null,
    operationLog: [],
    templateId: 'can-empty-v1',
    projectPreview: null,
    projectPreviewKind: 'create',
    applicationPreview: null,
    applicationWarnings: [],
    applicationRecoveryFiles: [],
    replacementReady: false,
    savingForReplacement: false,
    memberProjectPath: null,
  };
}
