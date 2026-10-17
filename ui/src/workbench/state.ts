import type { LanguagePreference, Text } from '../i18n';
import { message, readLanguagePreference } from '../i18n';
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
import type { Notice, Stage, StageRecord } from './forms';
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
  source: 'empty' | 'import' | 'save-as';
  projectName: string;
  projectDirectory: string;
  importPaths: string[];
  importPathText: string;
  stages: Record<Stage, StageRecord>;
  generated: GenerateResult | null;
  generationPreview: GenerationPreview | null;
  generationPreviewPath: string;
  generationKind: 'project' | 'handoff';
  buildTarget: BuildTarget;
  buildDirectory: string;
  built: BuildResult | null;
  virtualResult: VirtualResult | null;
  operationIssues: Issue[];
  notice: Notice;
  busy: Text | null;
  savePreview: SavePreview | null;
  previewPath: string;
  capabilities: WorkbenchCapabilities | null;
  settingsOpen: boolean;
  settingsCategory: 'appearance' | 'language' | 'definitions' | 'tools';
  toolDraft: ExecutionTools;
  settingsNotice: Notice;
  /** Rejections attached to the inspector field that caused them, by fieldId. */
  fieldErrors: Record<string, Text>;
  toolErrors: Partial<Record<keyof ExecutionTools, Text>>;
  operationGeneration: number;
  integrationInspection: IntegrationInspection | null;
  integrationIssues: PlanDiagnostic[];
  integrationIds: Record<string, string>;
  integrationPeriod: string;
  integrationPreview: SavePreview | null;
  integrationPreviewPath: string;
  integrationNotice: Text;
  ecuOutputDirectory: string;
  ecuImportDirectory: string;
  handoffImportOpen: boolean;
  handoffImportDestination: string;
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
  languageDraft: LanguagePreference;
  savedLanguage: LanguagePreference;
  languageSaving: boolean;
  preflight: PreflightReport | null;
  operationLog: { command: string; outcome: 'done' | 'failed'; detail: Text }[];
  projectPreview: ProjectCreationPreview | null;
  projectPreviewKind: 'create' | 'save-as';
  applicationPreview: ApplicationInitializationPreview | null;
  applicationWarnings: Text[];
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
    source: 'empty',
    projectName: '',
    projectDirectory: '',
    importPaths: [],
    importPathText: '',
    stages: stageDefaults,
    generated: null,
    generationPreview: null,
    generationPreviewPath: '',
    generationKind: 'project',
    buildTarget: 'windows-x64-controlled-v1',
    buildDirectory: '',
    built: null,
    virtualResult: null,
    operationIssues: [],
    notice: null,
    busy: null,
    savePreview: null,
    previewPath: '',
    capabilities: null,
    settingsOpen: false,
    settingsCategory: 'appearance',
    toolDraft: { compiler: '', objdump: '', git: '', python: '' },
    settingsNotice: null,
    fieldErrors: {},
    toolErrors: {},
    operationGeneration: 0,
    integrationInspection: null,
    integrationIssues: [],
    integrationIds: {},
    integrationPeriod: '',
    integrationPreview: null,
    integrationPreviewPath: '',
    integrationNotice: message('controller.integration.notInspected'),
    ecuOutputDirectory: '',
    ecuImportDirectory: '',
    handoffImportOpen: false,
    handoffImportDestination: '',
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
    languageDraft: readLanguagePreference(),
    savedLanguage: readLanguagePreference(),
    languageSaving: false,
    preflight: null,
    operationLog: [],
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
