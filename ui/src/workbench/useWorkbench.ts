import { useEffect, useEffectEvent, useReducer, useRef } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { requestConfirmation } from '../confirmation';
import { rememberDialogOpener } from './Dialog';
import type {
  BuildResult,
  BuildTarget,
  Frame,
  Signal,
  GenerationPreview,
  GenerateResult,
  Issue,
  SavePreview,
  VirtualResult,
  WorkspaceView,
} from '../types';
import type { ExecutionTools, SaveOutcome, WorkbenchCapabilities, WorkbenchReply } from '../types';
import type {
  IntegrationInspection,
  OwnedVerificationFailure,
  PlanDiagnostic,
  PreflightReport,
} from '../types';
import {
  newFrame,
  newSignal,
  diagnosticFields,
  dtcFields,
  dtcChanges,
  diagnosticChanges,
  frameChanges,
  signalChanges,
  errorText,
  previewDelta,
  initialSelection,
  findSelection,
  draftFor,
  hasUnapplied,
  intInRange,
} from './forms';
import type {
  Stage,
  StageState,
  StageRecord,
  Selection,
  FrameFields,
  SignalFields,
  FrameChanges,
  SignalChanges,
  Draft,
  DiagnosticFields,
  DiagnosticChanges,
  DtcFields,
  Notice,
} from './forms';
import type { PreviewDelta } from './forms';
import { native, stageDefaults, importedSaveStage } from './useDelivery';
import type {
  ApplicationInitializationOutcome,
  ApplicationInitializationPreview,
  Appearance,
  ChangeOutcome,
  ChangePreview,
  ChangeSet,
  ConfigurationChange,
  DocumentTab,
  DraftGuard,
  ProjectCreationPreview,
  ProjectProjection,
  ToolWindow,
} from './projectTypes';

type WorkbenchState = {
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

type StateSetters<S> = {
  [K in keyof S as K extends string ? `set${Capitalize<K>}` : never]: Dispatch<
    SetStateAction<S[K]>
  >;
};

export interface Workbench extends WorkbenchState, StateSetters<WorkbenchState> {
  native: boolean;
  executionDisabled: boolean;
  executionReason: string;
  currentFrame: Frame | undefined;
  currentSignal: Signal | undefined;
  signalFrame: Frame | undefined;
  currentFile: WorkspaceView['files'][number] | undefined;
  focusedFrame: Frame | undefined;
  frameUnapplied: boolean;
  diagnosticUnapplied: boolean;
  dtcUnapplied: boolean;
  unapplied: boolean;
  eligibleSignals: Signal[];
  eligibleMonitorFrames: Frame[];
  issues: Issue[];
  errorCount: number;
  unsupportedIssue: Issue | undefined;
  previewFile: SavePreview['files'][number] | undefined;
  delta: PreviewDelta | null;
  generationPreviewFile: GenerationPreview['files'][number] | undefined;
  generationDelta: PreviewDelta | null;
  disabled: boolean;
  acceptView(view: WorkspaceView, requested?: Selection | null): void;
  invalidateAfterEdit(): void;
  markStage(key: Stage, state: StageState, detail: string): void;
  requireReady(allowed?: 'frame' | 'diagnostic' | 'dtc'): void;
  run<T>(
    label: string,
    job: () => Promise<T>,
    onSuccess: (result: T) => void,
    stage?: Stage,
    allowed?: 'frame' | 'diagnostic' | 'dtc',
  ): Promise<void>;
  chooseDirectory(onChoose: (path: string) => void): Promise<void>;
  chooseBinary(onChoose: (path: string) => void): Promise<void>;
  chooseFiles(): Promise<void>;
  confirmAction(message: string): Promise<boolean>;
  choose(selection: Selection): Promise<void>;
  openCreator(kind: 'frame' | 'signal'): Promise<void>;
  applyProject(view: WorkspaceView): void;
  startProject(source?: 'empty' | 'import'): Promise<void>;
  openProjectEntry(source: WorkbenchState['source']): void;
  createProject(): void;
  importProject(): void;
  importHandoff(): void;
  addFrame(): void;
  addSignal(): void;
  updateSelected(): void;
  configureDiagnostic(): void;
  configureDtc(): void;
  clearDtc(): Promise<void>;
  clearDiagnostic(): Promise<void>;
  saveProject(): void;
  confirmSave(): void;
  validateProject(): void;
  generateProject(handoff?: boolean): void;
  confirmGenerate(): void;
  buildProject(): void;
  runVirtual(): void;
  runDiagnostic(): void;
  configureResources(): Promise<void>;
  configureTools(): Promise<void>;
  changeTarget(target: BuildTarget): Promise<void>;
  cancelOperation(): Promise<void>;
  importHandoffDirectory(directory: string): void;
  confirmHandoffImport(): Promise<void>;
  inspectIntegration(): void;
  applyIntegration(): void;
  previewIntegrationSave(): void;
  saveIntegration(): void;
  reopenIntegration(): Promise<void>;
  restoreIntegrationDraft(): void;
  previewEcu(): void;
  generateEcu(): Promise<void>;
  preflightEcu(): void;
  buildEcu(): void;
  verifyEcu(): void;
  changeEcuOutput(path: string): void;
  changeEcuBuildDirectory(path: string): void;
  actionReason(action: string): string;
  refreshProjection(): Promise<void>;
  selectObject(objectId: string, multiple?: boolean): Promise<void>;
  readSource(sourceId: string): Promise<void>;
  openDocument(tab: DocumentTab): void;
  closeDocument(tab: DocumentTab): Promise<void>;
  guardContext(title: string, action: () => void | Promise<void>): Promise<void>;
  replaceProject(title: string, action: () => void | Promise<void>): Promise<void>;
  resolveGuard(choice: 'apply' | 'discard' | 'cancel'): Promise<void>;
  requestSave(): Promise<void>;
  stageChange(change: ConfigurationChange): void;
  prepareChanges(): Promise<void>;
  applyChanges(): Promise<boolean>;
  configureAppearance(): Promise<void>;
  importDefinitionCatalog(): Promise<void>;
  removeDefinitionCatalog(catalogId: string): Promise<void>;
  openMemberProject(): Promise<void>;
  previewSaveAs(): Promise<void>;
  confirmProjectPreview(): Promise<void>;
  previewApplicationInitialization(): Promise<void>;
  initializeApplicationPreviewed(): Promise<void>;
  cancelSavePreview(): void;
  applyCurrentDrafts(): Promise<boolean>;
  verificationOwnedFailure(delayMs?: number): Promise<void>;
  restoreAllDrafts(): void;
}
type Action = {
  type: 'patch';
  patch: Partial<WorkbenchState> | ((state: WorkbenchState) => Partial<WorkbenchState>);
};
function reducer(state: WorkbenchState, action: Action): WorkbenchState {
  return { ...state, ...(typeof action.patch === 'function' ? action.patch(state) : action.patch) };
}
function initialState(): WorkbenchState {
  return {
    workspace: null,
    integrationUnapplied: false,
    integrationProcessing: false,
    selection: null,
    draft: null,
    diagnosticDraft: (() => diagnosticFields(null))(),
    diagnosticSignal: '',
    diagnosticError: '',
    dtcDraft: (() => dtcFields(null))(),
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

export function useWorkbench(): Workbench {
  const [state, dispatch] = useReducer(reducer, undefined, initialState);

  const capabilitiesRef = useRef<WorkbenchCapabilities | null>(null);

  const epoch = useRef(0);

  const running = useRef(false);
  const stateRef = useRef(state);
  stateRef.current = state;
  const pendingGuard = useRef<(() => void | Promise<void>) | null>(null);
  const pendingReplacement = useRef<(() => void | Promise<void>) | null>(null);
  const changeConfirmation = useRef<((applied: boolean) => void) | null>(null);

  useEffect(() => {
    if (!native) return;
    let live = true;
    const operationEpoch = epoch;
    void invoke<WorkbenchCapabilities>('workbench_capabilities')
      .then((capabilities) => {
        if (!live) return;
        capabilitiesRef.current = capabilities;
        patchState({
          capabilities,
          legacyTarget: capabilities.target,
          appearanceDraft: capabilities.appearance,
          resourceDraft: {
            xsdArchive: capabilities.xsdArchive ?? '',
            modArchive: capabilities.modArchive ?? '',
          },
          toolDraft: capabilities.configuredExecutionTools ?? {
            compiler: '',
            objdump: '',
            git: '',
            python: '',
          },
        });
      })
      .catch((error) => {
        if (live) patchState({ notice: { tone: 'error', text: errorText(error) } });
      });
    return () => {
      live = false;
      operationEpoch.current++;
    };
  }, []);

  async function call<T>(command: string, payload: Record<string, unknown> = {}): Promise<T> {
    const capabilities = capabilitiesRef.current;
    if (!capabilities) throw new Error('工作台能力尚未就绪');
    const generation = epoch.current;
    const fingerprint = capabilities.fingerprint;
    try {
      const reply = await invoke<WorkbenchReply<T>>(command, { ...payload, fingerprint });
      if (generation !== epoch.current || reply.inputFingerprint !== fingerprint) {
        throw new Error('STALE_DELIVERY: 当前操作结果已过期');
      }
      const previous = capabilitiesRef.current;
      capabilitiesRef.current = reply.capabilities;
      patchState({
        capabilities: reply.capabilities,
        legacyTarget: reply.capabilities.target,
        changePreview:
          previous?.fingerprint !== reply.capabilities.fingerprint ||
          previous?.definitionFingerprint !== reply.capabilities.definitionFingerprint
            ? null
            : stateRef.current.changePreview,
        preparedChangeSet:
          previous?.fingerprint !== reply.capabilities.fingerprint ||
          previous?.definitionFingerprint !== reply.capabilities.definitionFingerprint
            ? null
            : stateRef.current.preparedChangeSet,
        applicationPreview:
          previous?.fingerprint !== reply.capabilities.fingerprint ||
          previous?.definitionFingerprint !== reply.capabilities.definitionFingerprint
            ? null
            : stateRef.current.applicationPreview,
      });
      const value = typeof reply.value === 'object' && reply.value !== null ? reply.value : null;
      const detail =
        value && 'log' in value
          ? String(value.log)
          : value && 'logs' in value && Array.isArray(value.logs)
            ? value.logs.join('\n')
            : command;
      const failed = Boolean(
        value &&
        (('status' in value && value.status === 'failed') ||
          ('passed' in value && value.passed === false) ||
          ('exitCode' in value && typeof value.exitCode === 'number' && value.exitCode !== 0)),
      );
      field('operationLog')((logs) => [
        ...logs,
        { command, outcome: failed ? 'failed' : 'done', detail },
      ]);
      return reply.value;
    } catch (error) {
      if (generation === epoch.current)
        field('operationLog')((logs) => [
          ...logs,
          { command, outcome: 'failed', detail: errorText(error) },
        ]);
      if (generation === epoch.current) {
        const current = await invoke<WorkbenchCapabilities>('workbench_capabilities');
        if (generation === epoch.current) {
          capabilitiesRef.current = current;
          patchState({
            capabilities: current,
            legacyTarget: current.target,
            changePreview: null,
            preparedChangeSet: null,
            applicationPreview: null,
          });
        }
      }
      throw error;
    }
  }

  function invalidateOperation() {
    epoch.current++;
    running.current = false;
    changeConfirmation.current?.(false);
    changeConfirmation.current = null;
    pendingGuard.current = null;
    pendingReplacement.current = null;
    patchState({
      guard: null,
      changePreview: null,
      preparedChangeSet: null,
      applicationPreview: null,
      savingForReplacement: false,
      replacementReady: false,
    });
    setStages((previous) => {
      const next = { ...previous };
      for (const key of Object.keys(next) as Stage[]) {
        if (next[key].state === 'running') {
          next[key] = { state: 'stale', detail: '操作已取消；旧结果未提交' };
        }
      }
      return next;
    });
    patchState({ busy: null, integrationProcessing: false, operationGeneration: epoch.current });
  }

  async function cancelOperation() {
    invalidateOperation();
    try {
      await call<void>('cancel_operation');
      patchState({ notice: { tone: 'info', text: '操作已取消，受管进程已关闭；旧结果未提交。' } });
    } catch (error) {
      setNotice({ tone: 'error', text: `取消未确认：${errorText(error)}` });
    }
  }

  async function changeTarget(target: BuildTarget) {
    invalidateOperation();
    try {
      await call<void>('select_build_target', { target });
      patchState({
        generationPreview: null,
        generated: null,
        built: null,
        virtualResult: null,
        notice: null,
        stages: {
          ...stages,
          generate: stageDefaults.generate,
          build: stageDefaults.build,
          virtual: stageDefaults.virtual,
        },
      });
    } catch (error) {
      setNotice({ tone: 'error', text: errorText(error) });
    }
  }

  async function configureResources() {
    if (running.current) return;
    setSettingsNotice('');
    try {
      await call<void>('configure_validation_resources', state.resourceDraft);
      patchState({
        generationPreview: null,
        generated: null,
        built: null,
        virtualResult: null,
        notice: null,
        stages: {
          ...stages,
          validate: { state: 'stale', detail: '规范档案已变化，须重新校验' },
          generate: stageDefaults.generate,
          build: stageDefaults.build,
          virtual: stageDefaults.virtual,
        },
      });
      setSettingsNotice('规范档案已核对固定摘要并保存；须重新校验。');
    } catch (error) {
      setSettingsNotice(errorText(error));
    }
  }

  async function configureTools() {
    if (running.current) return;
    setSettingsNotice('');
    const generation = epoch.current;
    running.current = true;
    setBusy('保存执行工具');
    try {
      await call<void>('configure_execution_tools', { tools: stateRef.current.toolDraft });
      setGenerated(null);
      setBuilt(null);
      setVirtualResult(null);
      setGenerationPreview(null);
      setNotice(null);
      setStages((previous) => ({
        ...previous,
        generate: stageDefaults.generate,
        build: stageDefaults.build,
        virtual: stageDefaults.virtual,
      }));
      setSettingsNotice('执行工具已保存；原生预检尚未执行。');
    } catch (error) {
      setSettingsNotice(errorText(error));
    } finally {
      if (generation === epoch.current) {
        running.current = false;
        setBusy(null);
      }
    }
  }

  function field<K extends keyof WorkbenchState>(
    key: K,
  ): Dispatch<SetStateAction<WorkbenchState[K]>> {
    return (value) => {
      const previous = stateRef.current;
      const resolved =
        typeof value === 'function'
          ? (value as (previous: WorkbenchState[K]) => WorkbenchState[K])(previous[key])
          : value;
      const patch = {
        [key]: resolved,
        ...([
          'draft',
          'diagnosticDraft',
          'dtcDraft',
          'integrationIds',
          'integrationPeriod',
          'changes',
        ].includes(key as string)
          ? { changePreview: null, preparedChangeSet: null, applicationPreview: null }
          : {}),
      } as Partial<WorkbenchState>;
      stateRef.current = { ...previous, ...patch };
      dispatch({ type: 'patch', patch });
    };
  }

  const setCapabilities = field('capabilities');

  const setSettingsOpen = field('settingsOpen');

  const setResourceDraft = field('resourceDraft');

  const setToolDraft = field('toolDraft');

  const setSettingsNotice = field('settingsNotice');

  const setOperationGeneration = field('operationGeneration');

  const setIntegrationInspection = field('integrationInspection');

  const setIntegrationIssues = field('integrationIssues');

  const setIntegrationIds = field('integrationIds');

  const setIntegrationPeriod = field('integrationPeriod');

  const setIntegrationPreview = field('integrationPreview');

  const setIntegrationPreviewPath = field('integrationPreviewPath');

  const setIntegrationNotice = field('integrationNotice');

  const setEcuOutputDirectory = field('ecuOutputDirectory');

  const setEcuImportDirectory = field('ecuImportDirectory');

  const {
    workspace,
    integrationUnapplied,
    integrationProcessing,
    selection,
    draft,
    diagnosticDraft,
    diagnosticSignal,
    diagnosticError,
    dtcDraft,
    dtcError,
    source,
    projectName,
    projectDirectory,
    importPaths,
    importPathText,
    creating,
    frameInput,
    signalInput,
    stages,
    generated,
    generationPreview,
    generationPreviewPath,
    generationKind,
    handoffGenerated,
    legacyTarget,
    buildDirectory,
    built,
    virtualResult,
    virtualKind,
    operationIssues,
    notice,
    busy,
    peerDirectory,
    peerBinaryPath,
    savePreview,
    previewPath,
  } = state;

  const setWorkspace = field('workspace');

  const setIntegrationUnapplied = field('integrationUnapplied');

  const setIntegrationProcessing = field('integrationProcessing');

  const setSelection = field('selection');

  const setDraft = field('draft');

  const setDiagnosticDraft = field('diagnosticDraft');

  const setDiagnosticSignal = field('diagnosticSignal');

  const setDiagnosticError = field('diagnosticError');

  const setDtcDraft = field('dtcDraft');

  const setDtcError = field('dtcError');

  const setSource = field('source');

  const setProjectName = field('projectName');

  const setProjectDirectory = field('projectDirectory');

  const setImportPaths = field('importPaths');

  const setImportPathText = field('importPathText');

  const setCreating = field('creating');

  const setFrameInput = field('frameInput');

  const setSignalInput = field('signalInput');

  const setStages = field('stages');

  const setGenerated = field('generated');

  const setGenerationPreview = field('generationPreview');

  const setGenerationPreviewPath = field('generationPreviewPath');

  const setGenerationKind = field('generationKind');

  const setHandoffGenerated = field('handoffGenerated');

  const setLegacyTarget = field('legacyTarget');

  const setBuildDirectory = field('buildDirectory');

  const setBuilt = field('built');

  const setVirtualResult = field('virtualResult');

  const setVirtualKind = field('virtualKind');

  const setOperationIssues = field('operationIssues');

  const setNotice = field('notice');

  const setBusy = field('busy');

  const setPeerDirectory = field('peerDirectory');

  const setPeerBinaryPath = field('peerBinaryPath');

  const setSavePreview = field('savePreview');

  const setPreviewPath = field('previewPath');

  const currentFrame =
    selection?.kind === 'frame'
      ? workspace?.frames.find((item) => item.path === selection.path)
      : undefined;

  const currentSignal =
    selection?.kind === 'signal'
      ? workspace?.signals.find((item) => item.path === selection.path)
      : undefined;

  const signalFrame = currentSignal
    ? workspace?.frames.find((item) => item.path === currentSignal.framePath)
    : undefined;

  const currentFile =
    selection?.kind === 'file'
      ? workspace?.files.find((item) => item.path === selection.path)
      : undefined;

  const focusedFrame = currentFrame ?? signalFrame;

  const frameUnapplied = hasUnapplied(workspace, draft);

  const diagnosticUnapplied = Boolean(
    workspace &&
    JSON.stringify(diagnosticDraft) !== JSON.stringify(diagnosticFields(workspace.diagnostic)),
  );

  const dtcUnapplied = Boolean(
    workspace?.diagnostic &&
    JSON.stringify(dtcDraft) !== JSON.stringify(dtcFields(workspace.diagnostic.dtc)),
  );

  const unapplied =
    frameUnapplied ||
    diagnosticUnapplied ||
    dtcUnapplied ||
    integrationUnapplied ||
    state.changes.length > 0 ||
    Boolean(creating);

  const eligibleSignals =
    workspace?.signals.filter(
      (signal) =>
        signal.length === 32 &&
        workspace.frames.some(
          (frame) => frame.path === signal.framePath && frame.direction === 'tx',
        ),
    ) ?? [];

  const eligibleMonitorFrames =
    workspace?.frames.filter(
      (frame) =>
        frame.direction === 'rx' &&
        (frame.timeoutMs ?? 0) > 0 &&
        workspace.signals.some((signal) => signal.framePath === frame.path),
    ) ?? [];

  const issues = [...(workspace?.issues ?? []), ...operationIssues];

  const errorCount = issues.filter((issue) => issue.severity === 'error').length;

  function acceptView(view: WorkspaceView, requested?: Selection | null) {
    setSavePreview(null);
    setGenerationPreview(null);
    const next = findSelection(
      view,
      requested === undefined ? stateRef.current.selection : requested,
    );
    setWorkspace(view);
    setSelection(next);
    setDraft(draftFor(view, next));
    setDiagnosticDraft(diagnosticFields(view.diagnostic));
    setDiagnosticSignal('');
    setDiagnosticError('');
    setDtcDraft(dtcFields(view.diagnostic?.dtc ?? null));
    setDtcError('');
  }

  function invalidateAfterEdit() {
    setStages({
      save: { state: 'stale', detail: '配置已修改，尚未保存' },
      validate: { state: 'stale', detail: '配置已修改，需重新校验' },
      generate: { state: 'stale', detail: '生成结果基于旧配置' },
      build: { state: 'stale', detail: '构建结果基于旧配置' },
      virtual: { state: 'stale', detail: '运行结果基于旧配置' },
    });
    setGenerationPreview(null);
    setGenerated(null);
    setBuilt(null);
    setVirtualResult(null);
    setOperationIssues([]);
    field('preflight')(null);
    setNotice(null);
  }

  function markStage(key: Stage, state: StageState, detail: string) {
    setStages((previous) => ({ ...previous, [key]: { state, detail } }));
  }

  function requireReady(allowed?: 'frame' | 'diagnostic' | 'dtc') {
    if (!native) throw new Error('需要桌面运行环境');
    const current = stateRef.current;
    if (current.creating && allowed !== 'frame')
      throw new Error('创建表单尚未应用，请先应用或还原');
    if (hasUnapplied(current.workspace, current.draft) && allowed !== 'frame')
      throw new Error('检查器中有未应用的更改，请先应用或还原');
    if (
      current.workspace &&
      JSON.stringify(current.diagnosticDraft) !==
        JSON.stringify(diagnosticFields(current.workspace.diagnostic)) &&
      allowed !== 'diagnostic'
    )
      throw new Error('DoCAN 配置有未应用的更改，请先应用或还原');
    if (
      current.workspace?.diagnostic &&
      JSON.stringify(current.dtcDraft) !==
        JSON.stringify(dtcFields(current.workspace.diagnostic.dtc)) &&
      allowed !== 'dtc'
    )
      throw new Error('故障记忆有未应用的更改，请先应用或还原');
    if (current.changes.length) throw new Error('批次草稿尚未应用，请预览应用或还原');
  }

  async function run<T>(
    label: string,
    job: () => Promise<T>,
    onSuccess: (result: T) => void,
    stage?: Stage,
    allowed?: 'frame' | 'diagnostic' | 'dtc',
  ) {
    if (running.current) return;
    rememberDialogOpener();
    const generation = epoch.current;
    try {
      requireReady(allowed);
      running.current = true;
      setBusy(label);
      setNotice(null);
      if (stage) markStage(stage, 'running', `${label}中…`);
      const result = await job();
      if (generation !== epoch.current) return;
      onSuccess(result);
      if (generation === epoch.current && capabilitiesRef.current?.hasWorkspace)
        await refreshProjection();
    } catch (error) {
      if (generation !== epoch.current) return;
      const text = errorText(error);
      setNotice({ tone: 'error', text: `${label}失败：${text}` });
      if (label === '导入交付包') field('handoffImportOpen')(true);
      if (stateRef.current.settingsOpen) setSettingsNotice(text);
      if (workspace?.integrationCandidate && Array.isArray(error)) {
        setIntegrationIssues(error as PlanDiagnostic[]);
        setIntegrationNotice(label === '应用标准参数' ? '编辑被拒绝，原配置保持' : `${label}失败`);
      }
      if (stage) markStage(stage, 'failed', text);
      if (
        stateRef.current.savingForReplacement &&
        (label === '预览保存' || label === '预览标准输入保存')
      ) {
        pendingReplacement.current = null;
        patchState({ savingForReplacement: false, replacementReady: false });
      }
    } finally {
      if (generation === epoch.current) {
        running.current = false;
        setBusy(null);
        setIntegrationProcessing(false);
      }
    }
  }

  async function chooseDirectory(onChoose: (path: string) => void) {
    if (!native || busy) return;
    try {
      const path = await open({ directory: true, multiple: false, title: '选择目录' });
      if (typeof path === 'string') onChoose(path);
    } catch (error) {
      setNotice({ tone: 'error', text: `选择目录失败：${errorText(error)}` });
    }
  }

  async function chooseBinary(onChoose: (path: string) => void) {
    if (!native || busy) return;
    try {
      const path = await open({ multiple: false, title: '选择已构建的主机二进制' });
      if (typeof path === 'string') onChoose(path);
    } catch (error) {
      setNotice({ tone: 'error', text: `选择二进制失败：${errorText(error)}` });
    }
  }

  async function chooseFiles() {
    if (!native || busy) return;
    try {
      const paths = await open({
        multiple: true,
        filters: [{ name: 'AUTOSAR ARXML', extensions: ['arxml'] }],
        title: '选择项目 ARXML 文件',
      });
      if (paths) {
        const selected = Array.isArray(paths) ? paths : [paths];
        setImportPaths(selected);
        setImportPathText(selected.join('\n'));
      }
    } catch (error) {
      setNotice({ tone: 'error', text: `选择文件失败：${errorText(error)}` });
    }
  }

  async function confirmAction(message: string): Promise<boolean> {
    if (busy || integrationProcessing) return false;
    setBusy('确认操作');
    try {
      return await requestConfirmation(message);
    } catch (error) {
      setNotice({ tone: 'error', text: `确认操作失败：${errorText(error)}` });
      return false;
    } finally {
      setBusy(null);
    }
  }

  async function choose(selectionNext: Selection) {
    await guardContext('切换对象', () => {
      setSelection(selectionNext);
      if (stateRef.current.workspace) setDraft(draftFor(stateRef.current.workspace, selectionNext));
      setCreating(null);
      openDocument({ kind: 'communication' });
      const object = stateRef.current.projection?.objects.find(
        (item) => item.path === selectionNext.path,
      );
      field('activeObjectId')(object?.objectId ?? null);
      field('objectSelection')(object ? [object.objectId] : []);
      field('treeRevealId')(object?.objectId ?? null);
      field('treeMode')('objects');
    });
  }

  async function openCreator(kind: 'frame' | 'signal') {
    if (!native || busy || !workspace || workspace.integrationCandidate) return;
    await guardContext('创建对象', () => {
      setDraft(draftFor(stateRef.current.workspace!, stateRef.current.selection));
      setCreating(kind);
      field('inspectorVisible')(true);
      openDocument({ kind: 'communication' });
      setNotice(null);
      if (kind === 'frame') setFrameInput(newFrame);
      else setSignalInput(newSignal);
    });
  }

  function applyProject(view: WorkspaceView) {
    setIntegrationUnapplied(false);
    invalidateOperation();
    setIntegrationProcessing(false);
    patchState({
      integrationInspection: null,
      integrationIssues: [],
      integrationIds: {},
      integrationPeriod: '',
      integrationPreview: null,
      integrationPreviewPath: '',
      integrationNotice: '尚未检查标准输入',
      ecuOutputDirectory: '',
      ecuImportDirectory: '',
      generationKind: view.integrationCandidate ? 'handoff' : 'project',
      projection: null,
      applicationPreview: null,
      applicationWarnings: [],
      applicationRecoveryFiles: [],
      objectSelection: [],
      activeObjectId: null,
      activeSourceId: null,
      sourceText: null,
      changes: [],
      changePreview: null,
      preparedChangeSet: null,
      tabs: [
        { kind: view.integrationCandidate ? 'integration' : 'communication' },
        { kind: 'configuration' },
      ],
      activeDocument: { kind: view.integrationCandidate ? 'integration' : 'communication' },
      preflight: null,
      memberProjectPath: null,
    });
    setStages({ ...stageDefaults, save: importedSaveStage(view) });
    setGenerated(null);
    setBuilt(null);
    setVirtualResult(null);
    setOperationIssues([]);
    setPeerDirectory('');
    setPeerBinaryPath('');
    setBuildDirectory('');
    setCreating(null);
    acceptView(view, initialSelection(view));
    setDiagnosticError('');
    void refreshProjection();
  }

  function openProjectEntry(nextSource: WorkbenchState['source']) {
    setSource(nextSource);
    if (stateRef.current.workspace) openDocument({ kind: 'project-entry' });
  }

  async function startProject(nextSource: 'empty' | 'import' = 'empty') {
    if (integrationProcessing) return;
    await replaceProject('切换工程', async () => {
      try {
        await call<void>('close_project');
        invalidateOperation();
      } catch (error) {
        setNotice({ tone: 'error', text: errorText(error) });
        return;
      }
      patchState({
        workspace: null,
        savePreview: null,
        previewPath: '',
        generationPreview: null,
        generationPreviewPath: '',
        generated: null,
        built: null,
        virtualResult: null,
        virtualKind: null,
        stages: { ...stageDefaults },
        operationIssues: [],
        integrationInspection: null,
        integrationIssues: [],
        integrationIds: {},
        integrationPeriod: '',
        integrationPreview: null,
        integrationPreviewPath: '',
        integrationNotice: '尚未检查标准输入',
        ecuOutputDirectory: '',
        ecuImportDirectory: '',
        peerDirectory: '',
        peerBinaryPath: '',
        buildDirectory: '',
        handoffGenerated: false,
        projection: null,
        applicationPreview: null,
        applicationWarnings: [],
        applicationRecoveryFiles: [],
        activeObjectId: null,
        objectSelection: [],
        changes: [],
        sourceText: null,
        activeSourceId: null,
      });
      setIntegrationUnapplied(false);
      setSelection(null);
      setDiagnosticDraft(diagnosticFields(null));
      setDiagnosticSignal('');
      setDiagnosticError('');
      setDtcDraft(dtcFields(null));
      setDtcError('');
      setDraft(null);
      setSource(nextSource);
      setProjectName('');
      setProjectDirectory('');
      setImportPaths([]);
      setImportPathText('');
      setNotice(null);
    });
  }

  function createProject() {
    void replaceProject('新建工程', async () => {
      const current = stateRef.current;
      if (!current.projectDirectory || !current.projectName.trim()) {
        setNotice({ tone: 'error', text: '请输入工程名称并选择新空目录。' });
        return;
      }
      await run(
        '预览新建工程',
        () =>
          call<ProjectCreationPreview>('preview_project_creation', {
            directory: current.projectDirectory,
            name: current.projectName.trim(),
            templateId: current.templateId,
          }),
        (projectPreview) => patchState({ projectPreview, projectPreviewKind: 'create' }),
      );
    });
  }

  function importProject() {
    if (!importPaths.length) {
      setNotice({ tone: 'error', text: '请至少选择一份 ARXML 文件' });
      return;
    }
    void replaceProject('导入 ARXML', () =>
      run(
        '导入项目',
        () => call<WorkspaceView>('open_project', { paths: importPaths }),
        applyProject,
      ),
    );
  }

  function importHandoff() {
    if (!native || running.current) return;
    rememberDialogOpener();
    setNotice(null);
    patchState({
      handoffImportOpen: true,
      ecuImportDirectory: '',
      handoffImportDestination: '',
      handoffImportMode: 'v2',
    });
  }

  function addFrame() {
    let values: FrameChanges;
    try {
      values = frameChanges(frameInput);
    } catch (error) {
      setNotice({ tone: 'error', text: errorText(error) });
      return;
    }
    void run(
      '添加帧',
      () => call<WorkspaceView>('add_frame', values),
      (view) => {
        invalidateAfterEdit();
        acceptView(view, {
          kind: 'frame',
          path: view.frames.find((frame) => frame.name === values.name)?.path ?? '',
        });
        setCreating(null);
      },
      undefined,
      'frame',
    );
  }

  function addSignal() {
    if (!focusedFrame) return;
    let values: SignalChanges;
    try {
      values = signalChanges(signalInput, focusedFrame);
    } catch (error) {
      setNotice({ tone: 'error', text: errorText(error) });
      return;
    }
    const framePath = focusedFrame.path;
    void run(
      '添加信号',
      () => call<WorkspaceView>('add_signal', { framePath, ...values }),
      (view) => {
        invalidateAfterEdit();
        acceptView(view, {
          kind: 'signal',
          path:
            view.signals.find(
              (signal) => signal.name === values.name && signal.framePath === framePath,
            )?.path ?? '',
        });
        setCreating(null);
      },
      undefined,
      'frame',
    );
  }

  function updateSelected() {
    if (!draft || !workspace) return;
    try {
      if (draft.kind === 'frame') {
        const changes = frameChanges(draft.fields);
        const oldPath = draft.path;
        void run(
          '修改帧',
          () => call<WorkspaceView>('update_frame', { path: oldPath, changes }),
          (view) => {
            invalidateAfterEdit();
            acceptView(view, {
              kind: 'frame',
              path:
                view.frames.find((frame) => frame.path === oldPath)?.path ??
                view.frames.find((frame) => frame.name === changes.name)?.path ??
                '',
            });
          },
          undefined,
          'frame',
        );
      } else {
        const owner = workspace.frames.find(
          (frame) =>
            frame.path === workspace.signals.find((item) => item.path === draft.path)?.framePath,
        );
        if (!owner) throw new Error('所属帧不存在，无法修改信号');
        const changes = signalChanges(draft.fields, owner);
        const oldPath = draft.path;
        void run(
          '修改信号',
          () => call<WorkspaceView>('update_signal', { path: oldPath, changes }),
          (view) => {
            invalidateAfterEdit();
            acceptView(view, {
              kind: 'signal',
              path:
                view.signals.find((signal) => signal.path === oldPath)?.path ??
                view.signals.find(
                  (signal) => signal.name === changes.name && signal.framePath === owner.path,
                )?.path ??
                '',
            });
          },
          undefined,
          'frame',
        );
      }
    } catch (error) {
      setNotice({ tone: 'error', text: errorText(error) });
    }
  }

  function configureDiagnostic() {
    if (!workspace) return;
    let values: DiagnosticChanges;
    try {
      values = diagnosticChanges(diagnosticDraft, workspace);
      setDiagnosticError('');
    } catch (error) {
      setDiagnosticError(errorText(error));
      return;
    }
    void run(
      '配置 DoCAN',
      () =>
        call<WorkspaceView>('configure_diagnostic', { settings: values }).catch((error) => {
          setDiagnosticError(errorText(error));
          throw error;
        }),
      (view) => {
        invalidateAfterEdit();
        acceptView(view);
      },
      undefined,
      'diagnostic',
    );
  }

  function configureDtc() {
    if (!workspace?.diagnostic || diagnosticUnapplied || frameUnapplied) return;
    let values: { code: number; monitorFramePath: string };
    try {
      values = dtcChanges(dtcDraft, workspace);
      setDtcError('');
    } catch (error) {
      setDtcError(errorText(error));
      return;
    }
    void run(
      '配置故障记忆',
      () =>
        call<WorkspaceView>('configure_dtc', values).catch((error) => {
          setDtcError(errorText(error));
          throw error;
        }),
      (view) => {
        invalidateAfterEdit();
        acceptView(view);
      },
      undefined,
      'dtc',
    );
  }

  async function clearDtc() {
    if (!workspace?.diagnostic?.dtc || unapplied) return;
    if (!(await confirmAction('移除当前故障记忆配置？应用后仍需保存 ARXML 才会写入文件。'))) return;
    void run(
      '移除故障记忆',
      () => call<WorkspaceView>('clear_dtc'),
      (view) => {
        invalidateAfterEdit();
        acceptView(view);
      },
    );
  }

  async function clearDiagnostic() {
    if (!workspace?.diagnostic || diagnosticUnapplied || dtcUnapplied || frameUnapplied) return;
    if (!(await confirmAction('移除当前工程的诊断配置？应用后仍需保存 ARXML 才会写入文件。')))
      return;
    void run(
      '移除 DoCAN',
      () => call<WorkspaceView>('clear_diagnostic'),
      (view) => {
        invalidateAfterEdit();
        acceptView(view);
      },
      undefined,
      'diagnostic',
    );
  }

  function saveProject() {
    void run(
      '预览保存',
      () => call<SavePreview>('preview_save_project'),
      (preview) => {
        setSavePreview(preview);
        setPreviewPath(
          preview.files.find((file) => file.changed)?.path ?? preview.files[0]?.path ?? '',
        );
      },
    );
  }

  function confirmSave() {
    if (
      !savePreview ||
      (!savePreview.files.some((file) => file.changed) && !stateRef.current.savingForReplacement)
    )
      return;
    void run(
      '保存',
      () => call<SaveOutcome>('save_project', { revision: savePreview.revision }),
      (result) => {
        acceptView(result.workspace);
        if (result.error) {
          markStage('save', 'failed', result.error);
          setNotice({ tone: 'error', text: result.error });
        } else if (result.workspace.dirty) {
          markStage('save', 'failed', '后端仍报告未保存修改');
          setNotice({ tone: 'error', text: '保存未完成：项目仍标记为未保存' });
        } else {
          setStages({ ...stageDefaults, save: { state: 'done', detail: '配置项目已保存' } });
          setGenerated(null);
          setBuilt(null);
          setVirtualResult(null);
        }
        void continueReplacement(result.workspace, result.error);
      },
      'save',
    );
  }

  function validateProject() {
    void run(
      '校验',
      () => call<WorkspaceView>('validate_project'),
      (view) => {
        acceptView(view);
        setOperationIssues([]);
        const count = view.issues.filter((issue) => issue.severity === 'error').length;
        markStage(
          'validate',
          count ? 'failed' : 'done',
          count ? `${count} 个错误阻断生成` : '校验完成；无阻断错误',
        );
        if (count) {
          markStage('generate', 'stale', '当前校验有阻断错误');
          markStage('build', 'stale', '当前校验有阻断错误');
          markStage('virtual', 'stale', '当前校验有阻断错误');
        }
        field('toolWindow')('problems');
      },
      'validate',
    );
  }

  function generateProject(handoff = false) {
    if (workspace?.dirty || state.projection?.dirty) {
      setNotice({ tone: 'error', text: '请先保存配置，再生成工程' });
      return;
    }
    if (stages.validate.state !== 'done') {
      setNotice({ tone: 'error', text: '请先完成无阻断错误的校验' });
      return;
    }
    void chooseDirectory((directory) => {
      void run(
        handoff ? '预览可重建交付包' : '预览生成',
        () =>
          call<GenerationPreview>(
            handoff ? 'preview_handoff_project' : 'preview_generate_project',
            { outputDirectory: directory },
          ),
        (preview) => {
          setGenerationKind(handoff ? 'handoff' : 'project');
          setGenerationPreview(preview);
          setGenerationPreviewPath(
            preview.files.find((file) => file.status === 'changed')?.path ??
              preview.files.find((file) => file.status === 'new')?.path ??
              preview.files[0]?.path ??
              '',
          );
        },
      );
    });
  }

  function confirmGenerate() {
    if (!generationPreview) return;
    const preview = generationPreview;
    setGenerated(null);
    setBuilt(null);
    setVirtualResult(null);
    markStage('build', 'stale', '等待新生成工程');
    markStage('virtual', 'stale', '等待新生成工程');
    void run(
      '生成',
      () =>
        call<GenerateResult>(
          generationKind === 'handoff' ? 'generate_handoff_project' : 'generate_project',
          { outputDirectory: preview.outputDirectory, revision: preview.revision },
        ).catch((error) => {
          setGenerationPreview(null);
          throw error;
        }),
      (result) => {
        setGenerationPreview(null);
        setOperationIssues(result.issues);
        openDocument({ kind: 'delivery' });
        if (
          result.issues.some((issue) => issue.severity === 'error') ||
          !result.outputDirectory ||
          !result.files.length
        ) {
          markStage('generate', 'failed', '生成结果有错误或缺少工程文件');
          setNotice({ tone: 'error', text: '生成未通过，请查看诊断；不能视为工程已构建' });
        } else {
          setGenerated(result);
          setHandoffGenerated(generationKind === 'handoff');
          setBuilt(null);
          setVirtualResult(null);
          markStage('generate', 'done', result.outputDirectory);
          markStage('build', 'pending', '尚未构建生成工程');
          markStage('virtual', 'pending', '尚未运行两个 ECU');
        }
      },
      'generate',
    );
  }

  function buildProject() {
    if (!generated || !buildDirectory.trim() || stages.generate.state !== 'done') return;
    setBuilt(null);
    setVirtualResult(null);
    markStage('virtual', 'stale', '等待本次构建结果');
    void run(
      '构建',
      () =>
        call<BuildResult>('build_project', {
          outputDirectory: generated.outputDirectory,
          buildDirectory,
        }),
      (result) => {
        field('toolWindow')('build');
        if (!result.binaryPath) {
          markStage('build', 'failed', '构建未返回二进制文件路径');
          setNotice({ tone: 'error', text: '构建未通过：未返回二进制文件路径' });
        } else {
          setBuilt(result);
          setVirtualResult(null);
          markStage('build', 'done', result.binaryPath);
          markStage('virtual', 'pending', '尚未运行两个 ECU');
        }
      },
      'build',
    );
  }

  function runVirtual() {
    if (
      !generated ||
      !built ||
      stages.generate.state !== 'done' ||
      stages.build.state !== 'done' ||
      !peerDirectory ||
      !peerBinaryPath
    )
      return;
    setVirtualResult(null);
    void run(
      '主机虚拟运行',
      () =>
        call<VirtualResult>('run_virtual', {
          firstOutputDirectory: generated.outputDirectory,
          secondOutputDirectory: peerDirectory,
          firstBinaryPath: built.binaryPath,
          secondBinaryPath: peerBinaryPath,
        }),
      (result) => {
        setVirtualKind('signal');
        setVirtualResult(result);
        field('toolWindow')('host');
        markStage(
          'virtual',
          result.passed ? 'done' : 'failed',
          result.passed ? '双 ECU 虚拟运行通过' : '双 ECU 虚拟运行未通过',
        );
      },
      'virtual',
    );
  }

  function runDiagnostic() {
    if (
      !workspace?.diagnostic ||
      !generated ||
      !built ||
      stages.generate.state !== 'done' ||
      stages.build.state !== 'done' ||
      unapplied
    )
      return;
    setVirtualResult(null);
    void run(
      '诊断独立测试',
      () =>
        call<VirtualResult>('run_diagnostic', {
          outputDirectory: generated.outputDirectory,
          binaryPath: built.binaryPath,
        }),
      (result) => {
        setVirtualKind('diagnostic');
        setVirtualResult(result);
        field('toolWindow')('host');
        markStage(
          'virtual',
          result.passed ? 'done' : 'failed',
          result.passed ? '诊断独立测试器通过' : '诊断独立测试器未通过',
        );
      },
      'virtual',
    );
  }

  const unsupportedIssue = workspace?.issues.find(
    (issue) => issue.code.startsWith('PDU_') || issue.code === 'DIAG_UNSUPPORTED',
  );

  const previewFile = savePreview?.files.find((file) => file.path === previewPath);

  const delta =
    previewFile?.before !== null &&
    previewFile?.after !== null &&
    previewFile?.before !== undefined &&
    previewFile?.after !== undefined
      ? previewDelta(previewFile.before, previewFile.after)
      : null;

  const generationPreviewFile = generationPreview?.files.find(
    (file) => file.path === generationPreviewPath,
  );

  const generationDelta =
    generationPreviewFile?.status === 'changed' &&
    generationPreviewFile.before !== null &&
    generationPreviewFile.after !== null
      ? previewDelta(generationPreviewFile.before, generationPreviewFile.after, false)
      : null;

  const disabled =
    !native || !state.capabilities || Boolean(state.capabilities.ruleError) || Boolean(busy);
  const executionReason = state.capabilities?.nativeExecution
    ? (state.capabilities.toolError ?? '')
    : '未执行：本机不支持所选目标执行';
  const executionDisabled =
    disabled || !state.capabilities?.nativeExecution || Boolean(state.capabilities.toolError);

  function importHandoffDirectory(directory: string) {
    if (!native || running.current) return;
    rememberDialogOpener();
    setNotice(null);
    patchState({
      handoffImportOpen: true,
      ecuImportDirectory: directory,
      handoffImportDestination: '',
      handoffImportMode: 'v2',
    });
  }

  async function confirmHandoffImport() {
    const current = stateRef.current;
    const directory = current.ecuImportDirectory;
    const newWorkspaceDirectory =
      current.handoffImportMode === 'v2' ? current.handoffImportDestination : null;
    if (!directory || (current.handoffImportMode === 'v2' && !newWorkspaceDirectory)) return;
    await replaceProject('导入交付包并替换工程', async () => {
      field('handoffImportOpen')(false);
      await run(
        '导入交付包',
        () => call<WorkspaceView>('open_handoff_project', { directory, newWorkspaceDirectory }),
        (view) => {
          field('handoffImportDestination')('');
          applyProject(view);
        },
      );
    });
  }
  async function previewApplicationInitialization() {
    await guardContext('初始化用户应用', () =>
      run(
        '预览用户应用初始化',
        () => call<ApplicationInitializationPreview>('preview_application_initialization'),
        (applicationPreview) => field('applicationPreview')(applicationPreview),
      ),
    );
  }

  async function initializeApplicationPreviewed() {
    const preview = stateRef.current.applicationPreview;
    if (!preview) return;
    await run(
      '初始化用户应用',
      () => call<ApplicationInitializationOutcome>('initialize_application_previewed', { preview }),
      (outcome) => {
        patchState({
          applicationPreview: null,
          projection: outcome.projection,
          applicationWarnings: outcome.warnings,
          applicationRecoveryFiles: outcome.retainedRecoveryFiles,
        });
        invalidateAfterEdit();
        if (!outcome.projection.dirty) markStage('save', 'done', '用户应用与成员清单已按预览写入');
        setNotice({
          tone: 'info',
          text: `用户应用初始化已返回；${outcome.warnings.length} 条警告，${outcome.retainedRecoveryFiles.length} 个待处理恢复文件。旧生成、构建与运行结果已失效。`,
        });
      },
    );
  }

  function acceptIntegration(report: IntegrationInspection) {
    patchState({
      integrationInspection: report,
      integrationIssues: report.diagnostics,
      integrationPreview: null,
      integrationUnapplied: false,
      integrationIds: report.description
        ? Object.fromEntries(
            report.description.signals.map((signal) => [signal.port, String(signal.canId)]),
          )
        : {},
      integrationPeriod: report.description ? String(report.description.component.periodMs) : '',
      integrationProcessing: false,
    });
  }

  function restoreIntegrationDraft() {
    if (state.integrationInspection) acceptIntegration(state.integrationInspection);
  }

  function inspectIntegration() {
    void run(
      '检查标准输入',
      () => call<IntegrationInspection>('inspect_integration'),
      (report) => {
        acceptIntegration(report);
        setIntegrationNotice(
          report.description ? '标准输入已校验，尚未生成运行工程' : '输入未通过，无法生成',
        );
      },
    );
  }

  function applyIntegration() {
    const ids: Record<string, number> = {};
    try {
      for (const [path, value] of Object.entries(state.integrationIds)) {
        ids[path] = intInRange(value, 'CAN ID', 0, 2047);
      }
      const period = intInRange(state.integrationPeriod, '应用周期', 1, 2147483647);
      void run(
        '应用标准参数',
        async () => {
          const report = await call<IntegrationInspection>('edit_integration', {
            changes: { canIds: ids, applicationPeriodMs: period },
          });
          const view = await call<WorkspaceView>('workspace_view');
          return { report, view };
        },
        ({ report, view }) => {
          acceptView(view);
          invalidateAfterEdit();
          acceptIntegration(report);
          setIntegrationNotice('修改已通过同一计划校验，尚未保存');
        },
      );
    } catch (error) {
      setNotice({ tone: 'error', text: errorText(error) });
    }
  }

  function previewIntegrationSave() {
    void run(
      '预览标准输入保存',
      () => call<SavePreview>('preview_integration_save'),
      (preview) => {
        patchState({
          integrationPreview: preview,
          integrationIssues: [],
          integrationPreviewPath:
            preview.files.find((file) => file.changed)?.path ?? preview.files[0]?.path ?? '',
        });
      },
    );
  }

  function saveIntegration() {
    const preview = state.integrationPreview;
    if (!preview) return;
    void run(
      '保存标准输入',
      () => call<SaveOutcome>('save_integration', { revision: preview.revision }),
      (result) => {
        acceptView(result.workspace);
        patchState({
          integrationPreview: null,
          generated: null,
          built: null,
          virtualResult: null,
          stages: {
            ...stageDefaults,
            save: {
              state: result.error || result.workspace.dirty ? 'failed' : 'done',
              detail: result.error ?? (result.workspace.dirty ? '工程仍未保存' : '标准输入已保存'),
            },
          },
          integrationNotice: result.error ?? '标准输入已保存，尚未生成运行工程',
        });
        if (result.error) setNotice({ tone: 'error', text: result.error });
        void continueReplacement(result.workspace, result.error);
      },
      'save',
    );
  }

  async function reopenIntegration() {
    if (!workspace) return;
    const path = stateRef.current.memberProjectPath;
    await replaceProject('重开来源', () =>
      run(
        '重开来源',
        () =>
          path
            ? call<WorkspaceView>('open_member_project', { path })
            : call<WorkspaceView>('open_project', {
                paths: workspace.files.map((file) => file.path),
              }),
        (view) => {
          applyProject(view);
          field('memberProjectPath')(path);
        },
      ),
    );
  }

  function changeEcuOutput(path: string) {
    patchState({
      ecuOutputDirectory: path,
      generationPreview: null,
      generated: null,
      built: null,
      virtualResult: null,
      stages: {
        ...stages,
        generate: stageDefaults.generate,
        build: stageDefaults.build,
        virtual: stageDefaults.virtual,
      },
    });
  }

  function changeEcuBuildDirectory(path: string) {
    patchState({
      buildDirectory: path,
      built: null,
      virtualResult: null,
      stages: { ...stages, build: stageDefaults.build, virtual: stageDefaults.virtual },
    });
  }

  function previewEcu() {
    if (
      workspace?.dirty ||
      state.projection?.dirty ||
      unapplied ||
      !state.ecuOutputDirectory.trim()
    )
      return;
    void run(
      '预览 ECU 交付',
      async () => {
        const inspection = await call<IntegrationInspection>('inspect_integration');
        if (!inspection.description) throw inspection.diagnostics;
        const preview = await call<GenerationPreview>('preview_ecu_project', {
          outputDirectory: state.ecuOutputDirectory,
          handoff: generationKind === 'handoff',
        });
        return { inspection, preview };
      },
      ({ inspection, preview }) => {
        acceptIntegration(inspection);
        setGenerationPreview(preview);
        setGenerationPreviewPath(preview.files[0]?.path ?? '');
        markStage('validate', 'done', '标准输入已校验');
        setNotice({
          tone: 'info',
          text: `纯源码预览 ${preview.files.length} 个文件；尚未写入输出目录。`,
        });
      },
    );
  }

  async function generateEcu() {
    if (!generationPreview) return;
    if (
      !(await confirmAction(
        `确认将 ${generationPreview.files.length} 个文件写入 ${generationPreview.outputDirectory}？`,
      ))
    ) {
      setNotice({ tone: 'info', text: '已取消生成，输出目录未改动。' });
      return;
    }
    void run(
      '生成 ECU',
      () =>
        call<GenerateResult>('generate_ecu_project', {
          outputDirectory: generationPreview.outputDirectory,
          handoff: generationKind === 'handoff',
          revision: generationPreview.revision,
        }),
      (value) => {
        setGenerated(value);
        setGenerationPreview(null);
        setBuilt(null);
        setVirtualResult(null);
        setOperationIssues(value.issues);
        markStage(
          'generate',
          value.issues.some((issue) => issue.severity === 'error') ? 'failed' : 'done',
          value.outputDirectory,
        );
        markStage('build', 'pending', '尚未构建本次工程');
        markStage('virtual', 'pending', '尚未验证本次工程');
      },
      'generate',
    );
  }

  function preflightEcu() {
    if (workspace?.dirty || state.projection?.dirty || unapplied) return;
    void run(
      '原生编译预检',
      () => call<PreflightReport>('preflight_ecu', { handoff: generationKind === 'handoff' }),
      (report) => {
        field('preflight')(report);
        setNotice({
          tone: report.status === 'failed' ? 'error' : 'info',
          text: `编译预检：${report.status}；源码身份：${report.fingerprint}。${report.logs.join('\n')}`,
        });
      },
    );
  }

  function buildEcu() {
    if (!generated || stages.generate.state !== 'done' || !buildDirectory.trim()) return;
    void run(
      '构建 ECU',
      () => {
        setBuilt(null);
        setVirtualResult(null);
        markStage('virtual', 'stale', '重新构建中，旧运行结果已失效');
        return call<BuildResult>('build_ecu', {
          outputDirectory: generated.outputDirectory,
          buildDirectory,
        });
      },
      (value) => {
        setBuilt(value);
        setVirtualResult(null);
        markStage('build', 'done', value.binaryPath);
        markStage('virtual', 'pending', '尚未验证本次构建');
      },
      'build',
    );
  }

  function verifyEcu() {
    if (!generated || !built || stages.build.state !== 'done') return;
    void run(
      '验证 ECU 主机行为',
      () => {
        setVirtualResult(null);
        return call<VirtualResult>('verify_ecu', { outputDirectory: generated.outputDirectory });
      },
      (value) => {
        setVirtualResult(value);
        setVirtualKind('signal');
        markStage(
          'virtual',
          value.passed ? 'done' : 'failed',
          value.passed ? '本次 CAN/DID/N_Cr 验证通过' : '主机行为验证失败',
        );
      },
      'virtual',
    );
  }

  function patchState(patch: Partial<WorkbenchState>) {
    stateRef.current = { ...stateRef.current, ...patch };
    dispatch({ type: 'patch', patch });
  }

  function actionReason(action: string) {
    if (!native) return '需要桌面运行环境';
    if (!state.capabilities) return '正在读取后台能力';
    if (state.busy) return `正在${state.busy}`;
    const capability =
      state.projection?.capabilities.find((item) => item.action === action) ??
      state.capabilities.actions.find((item) => item.action === action);
    return !capability
      ? '后台未提供该动作'
      : !capability.available
        ? (capability.reason ?? '后台未提供此能力')
        : '';
  }

  async function refreshProjection() {
    if (!capabilitiesRef.current?.hasWorkspace) return;
    try {
      const projection = await call<ProjectProjection>('project_projection');
      patchState({ projection, memberProjectPath: projection.projectPath });
    } catch (error) {
      setNotice({ tone: 'error', text: `读取工程投影失败：${errorText(error)}` });
    }
  }

  function openDocument(tab: DocumentTab) {
    const current = stateRef.current;
    const same = (other: DocumentTab) => other.kind === tab.kind && other.sourceId === tab.sourceId;
    patchState({
      tabs: current.tabs.some(same) ? current.tabs : [...current.tabs, tab],
      activeDocument: tab,
    });
  }

  async function closeDocument(tab: DocumentTab) {
    await guardContext('关闭文档', async () => {
      const current = stateRef.current;
      const tabs = current.tabs.filter(
        (other) => other.kind !== tab.kind || other.sourceId !== tab.sourceId,
      );
      const closingActive =
        current.activeDocument.kind === tab.kind &&
        current.activeDocument.sourceId === tab.sourceId;
      const activeDocument = closingActive
        ? (tabs[0] ?? { kind: 'configuration' as const })
        : current.activeDocument;
      patchState({
        tabs: tabs.length ? tabs : [{ kind: 'configuration' }],
        activeDocument,
      });
      if (closingActive && activeDocument.kind === 'source' && activeDocument.sourceId)
        await readSource(activeDocument.sourceId);
    });
  }

  function hasDrafts(current = stateRef.current) {
    return (
      hasUnapplied(current.workspace, current.draft) ||
      Boolean(
        current.workspace &&
        JSON.stringify(current.diagnosticDraft) !==
          JSON.stringify(diagnosticFields(current.workspace.diagnostic)),
      ) ||
      Boolean(
        current.workspace?.diagnostic &&
        JSON.stringify(current.dtcDraft) !==
          JSON.stringify(dtcFields(current.workspace.diagnostic.dtc)),
      ) ||
      current.integrationUnapplied ||
      current.changes.length > 0 ||
      Boolean(current.creating)
    );
  }

  function discardDrafts() {
    const current = stateRef.current;
    patchState({
      draft: current.workspace ? draftFor(current.workspace, current.selection) : null,
      diagnosticDraft: diagnosticFields(current.workspace?.diagnostic ?? null),
      dtcDraft: dtcFields(current.workspace?.diagnostic?.dtc ?? null),
      diagnosticSignal: '',
      diagnosticError: '',
      dtcError: '',
      changes: [],
      changePreview: null,
      preparedChangeSet: null,
      applicationPreview: null,
      creating: null,
      integrationUnapplied: false,
      integrationIds: current.integrationInspection?.description
        ? Object.fromEntries(
            current.integrationInspection.description.signals.map((signal) => [
              signal.port,
              String(signal.canId),
            ]),
          )
        : {},
      integrationPeriod: current.integrationInspection?.description
        ? String(current.integrationInspection.description.component.periodMs)
        : '',
    });
  }

  async function guardContext(title: string, action: () => void | Promise<void>) {
    if (running.current || stateRef.current.guard) return;
    rememberDialogOpener();
    if (!hasDrafts()) {
      await action();
      return;
    }
    pendingGuard.current = action;
    patchState({ guard: { kind: 'context', title } });
  }

  async function replaceProject(title: string, action: () => void | Promise<void>) {
    if (running.current || stateRef.current.guard) return;
    rememberDialogOpener();
    if (!hasDrafts() && !stateRef.current.workspace?.dirty && !stateRef.current.projection?.dirty) {
      await action();
      return;
    }
    pendingGuard.current = action;
    patchState({ guard: { kind: 'project', title } });
  }

  async function applyDrafts(): Promise<boolean> {
    const current = stateRef.current;
    if (!current.workspace || running.current) return false;
    const generation = epoch.current;
    running.current = true;
    setBusy('应用草稿');
    let view = current.workspace;
    try {
      if (current.creating === 'frame') {
        const values = frameChanges(current.frameInput);
        view = await call<WorkspaceView>('add_frame', values);
        const frame = view.frames.find((item) => item.name === values.name);
        const next = frame ? { kind: 'frame' as const, path: frame.path } : current.selection;
        patchState({
          workspace: view,
          creating: null,
          selection: next,
          draft: draftFor(view, next),
        });
      } else if (current.creating === 'signal') {
        const framePath =
          current.selection?.kind === 'frame'
            ? current.selection.path
            : view.signals.find((item) => item.path === current.selection?.path)?.framePath;
        const owner = view.frames.find((item) => item.path === framePath);
        if (!owner) throw new Error('请选择待创建信号的真实所属帧');
        const values = signalChanges(current.signalInput, owner);
        view = await call<WorkspaceView>('add_signal', { framePath: owner.path, ...values });
        const signal = view.signals.find(
          (item) => item.framePath === owner.path && item.name === values.name,
        );
        const next = signal ? { kind: 'signal' as const, path: signal.path } : current.selection;
        patchState({
          workspace: view,
          creating: null,
          selection: next,
          draft: draftFor(view, next),
        });
      }
      if (hasUnapplied(view, current.draft) && current.draft) {
        if (current.draft.kind === 'frame') {
          view = await call<WorkspaceView>('update_frame', {
            path: current.draft.path,
            changes: frameChanges(current.draft.fields),
          });
        } else {
          const owner = view.frames.find(
            (frame) =>
              frame.path ===
              view.signals.find((signal) => signal.path === current.draft?.path)?.framePath,
          );
          if (!owner) throw new Error('信号所属帧不存在');
          view = await call<WorkspaceView>('update_signal', {
            path: current.draft.path,
            changes: signalChanges(current.draft.fields, owner),
          });
        }
        patchState({ workspace: view, draft: draftFor(view, current.selection) });
      }
      if (
        JSON.stringify(current.diagnosticDraft) !==
        JSON.stringify(diagnosticFields(current.workspace.diagnostic))
      ) {
        view = await call<WorkspaceView>('configure_diagnostic', {
          settings: diagnosticChanges(current.diagnosticDraft, view),
        });
        patchState({ workspace: view, diagnosticDraft: diagnosticFields(view.diagnostic) });
      }
      if (
        current.workspace.diagnostic &&
        JSON.stringify(current.dtcDraft) !==
          JSON.stringify(dtcFields(current.workspace.diagnostic.dtc))
      ) {
        view = await call<WorkspaceView>('configure_dtc', dtcChanges(current.dtcDraft, view));
        patchState({ workspace: view, dtcDraft: dtcFields(view.diagnostic?.dtc ?? null) });
      }
      if (current.integrationUnapplied) {
        const canIds = Object.fromEntries(
          Object.entries(current.integrationIds).map(([path, value]) => [
            path,
            intInRange(value, 'CAN ID', 0, 2047),
          ]),
        );
        const report = await call<IntegrationInspection>('edit_integration', {
          changes: {
            canIds,
            applicationPeriodMs: intInRange(current.integrationPeriod, '应用周期', 1, 2147483647),
          },
        });
        view = await call<WorkspaceView>('workspace_view');
        acceptIntegration(report);
        patchState({ workspace: view, integrationUnapplied: false });
      }
      if (generation !== epoch.current) return false;
      if (view !== current.workspace) invalidateAfterEdit();
      await refreshProjection();
    } catch (error) {
      if (generation !== epoch.current) return false;
      setNotice({
        tone: 'error',
        text: `应用被拒绝，保持当前位置与未应用输入：${errorText(error)}`,
      });
      await refreshProjection();
      return false;
    } finally {
      if (generation === epoch.current) {
        running.current = false;
        setBusy(null);
      }
    }
    if (current.changes.length) {
      await prepareChanges();
      if (!stateRef.current.changePreview) return false;
      return await new Promise<boolean>((resolve) => {
        changeConfirmation.current = resolve;
      });
    }
    return true;
  }

  async function resolveGuard(choice: 'apply' | 'discard' | 'cancel') {
    const guard = stateRef.current.guard;
    const action = pendingGuard.current;
    if (!guard) return;
    patchState({ guard: null });
    pendingGuard.current = null;
    if (choice === 'cancel') return;
    if (choice === 'discard') {
      discardDrafts();
      await action?.();
      return;
    }
    if (!(await applyDrafts())) return;
    if (guard.kind === 'project') {
      pendingReplacement.current = action;
      patchState({ savingForReplacement: true });
      await requestSave();
    } else await action?.();
  }

  async function requestSave() {
    if (hasDrafts()) {
      await guardContext('预览保存', requestSave);
      return;
    }
    if (stateRef.current.workspace?.integrationCandidate) previewIntegrationSave();
    else saveProject();
  }

  async function continueReplacement(saved: WorkspaceView, error: string | null) {
    if (error || saved.dirty || !pendingReplacement.current) return;
    patchState({ replacementReady: true });
  }

  function cancelSavePreview() {
    if (running.current) return;
    pendingReplacement.current = null;
    patchState({
      savePreview: null,
      integrationPreview: null,
      savingForReplacement: false,
      replacementReady: false,
    });
  }

  async function selectObject(objectId: string, multiple = false) {
    const object = stateRef.current.projection?.objects.find((item) => item.objectId === objectId);
    if (!object) return;
    const select = () => {
      const current = stateRef.current;
      patchState({
        activeObjectId: objectId,
        activeSourceId: object.sourceId,
        treeMode: 'objects',
        treeRevealId: objectId,
        objectSelection: multiple
          ? current.objectSelection.includes(objectId)
            ? current.objectSelection.filter((id) => id !== objectId)
            : [...current.objectSelection, objectId]
          : [objectId],
        inspectorVisible: true,
      });
      openDocument({ kind: 'configuration' });
    };
    if (multiple) select();
    else await guardContext('切换配置对象', select);
  }

  async function readSource(sourceId: string) {
    if (!native || running.current) return;
    const generation = epoch.current;
    running.current = true;
    setBusy('读取源原文');
    patchState({ activeSourceId: sourceId, sourceText: null, treeMode: 'files' });
    openDocument({ kind: 'source', sourceId });
    try {
      const contents = await call<string>('read_project_source', { sourceId });
      const current = stateRef.current;
      if (
        generation !== epoch.current ||
        current.activeDocument.kind !== 'source' ||
        current.activeDocument.sourceId !== sourceId ||
        current.activeSourceId !== sourceId
      )
        return;
      patchState({ sourceText: contents });
    } catch (error) {
      if (generation === epoch.current)
        setNotice({ tone: 'error', text: `原文读取失败：${errorText(error)}` });
    } finally {
      if (generation === epoch.current) {
        running.current = false;
        setBusy(null);
      }
    }
  }

  function stageChange(change: ConfigurationChange) {
    const changes = stateRef.current.changes;
    const index = changes.findIndex((item) => item.changeId === change.changeId);
    field('changes')(
      index < 0
        ? [...changes, change]
        : changes.map((item, position) => (position === index ? change : item)),
    );
  }

  async function prepareChanges() {
    const current = stateRef.current;
    if (!current.projection || !current.changes.length || running.current) return;
    const changeSet: ChangeSet = {
      workspaceEpoch: current.projection.workspaceEpoch,
      inputFingerprint: capabilitiesRef.current!.fingerprint,
      definitionFingerprint: current.projection.definitionFingerprint,
      changes: current.changes,
    };
    running.current = true;
    setBusy('预览批次影响');
    const generation = epoch.current;
    try {
      const preview = await call<ChangePreview>('prepare_configuration_change', { changeSet });
      if (generation !== epoch.current) return;
      if (stateRef.current.changes !== current.changes) throw new Error('草稿已变化，请重新预览');
      patchState({ changePreview: preview, preparedChangeSet: changeSet });
    } catch (error) {
      if (generation !== epoch.current) return;
      patchState({ changePreview: null, preparedChangeSet: null });
      setNotice({ tone: 'error', text: `批次被拒绝：${errorText(error)}` });
    } finally {
      if (generation === epoch.current) {
        running.current = false;
        setBusy(null);
      }
    }
  }

  async function applyChanges(): Promise<boolean> {
    const before = stateRef.current;
    const { changePreview, preparedChangeSet } = before;
    if (!changePreview || !preparedChangeSet || running.current) return false;
    running.current = true;
    setBusy('原子应用批次');
    const generation = epoch.current;
    const fingerprintBefore = capabilitiesRef.current?.fingerprint;
    try {
      const result = await call<ChangeOutcome>('apply_configuration_change', {
        changeSet: preparedChangeSet,
        changeRevision: changePreview.changeRevision,
      });
      if (generation !== epoch.current) return false;
      if (fingerprintBefore !== capabilitiesRef.current?.fingerprint) invalidateAfterEdit();
      const objectIds = new Set(result.projection.objects.map((object) => object.objectId));
      const selectedIds = stateRef.current.objectSelection.filter((id) => objectIds.has(id));
      const requestedId = result.selectionId ?? result.createdIds[0]?.objectId;
      const nextId =
        requestedId && objectIds.has(requestedId)
          ? requestedId
          : stateRef.current.activeObjectId && objectIds.has(stateRef.current.activeObjectId)
            ? stateRef.current.activeObjectId
            : null;
      if (nextId && !selectedIds.includes(nextId)) selectedIds.push(nextId);
      // The batch is committed; a later read failure must not retain its draft
      // or report the already-published changes as rejected.
      patchState({
        projection: result.projection,
        workspace: before.workspace
          ? { ...before.workspace, dirty: result.projection.dirty }
          : null,
        changes: [],
        changePreview: null,
        preparedChangeSet: null,
        activeObjectId: nextId,
        treeRevealId: nextId,
        objectSelection: selectedIds,
        integrationInspection: null,
        integrationIssues: [],
        integrationPreview: null,
      });
      let view: WorkspaceView;
      try {
        view = await call<WorkspaceView>('workspace_view');
      } catch (error) {
        if (generation !== epoch.current) return false;
        setNotice({ tone: 'error', text: `批次已应用；读取工程视图失败：${errorText(error)}` });
        changeConfirmation.current?.(true);
        changeConfirmation.current = null;
        return true;
      }
      if (generation !== epoch.current) return false;
      const current = stateRef.current;
      const selection = findSelection(view, current.selection);
      patchState({
        projection: result.projection,
        workspace: view,
        selection,
        draft: hasUnapplied(before.workspace, current.draft)
          ? current.draft
          : draftFor(view, selection),
        diagnosticDraft:
          JSON.stringify(current.diagnosticDraft) ===
          JSON.stringify(diagnosticFields(before.workspace?.diagnostic ?? null))
            ? diagnosticFields(view.diagnostic)
            : current.diagnosticDraft,
        dtcDraft:
          JSON.stringify(current.dtcDraft) ===
          JSON.stringify(dtcFields(before.workspace?.diagnostic?.dtc ?? null))
            ? dtcFields(view.diagnostic?.dtc ?? null)
            : current.dtcDraft,
        changes: [],
        changePreview: null,
        preparedChangeSet: null,
        activeObjectId: nextId,
        treeRevealId: nextId,
        objectSelection: selectedIds,
      });
      if (view.integrationCandidate || before.integrationInspection) {
        patchState({
          integrationInspection: null,
          integrationIssues: [],
          integrationPreview: null,
          integrationNotice: '配置已变化，正在重新检查标准输入',
        });
        try {
          const report = await call<IntegrationInspection>('inspect_integration');
          if (generation !== epoch.current) return false;
          const integration = stateRef.current;
          acceptIntegration(report);
          if (integration.integrationUnapplied)
            patchState({
              integrationIds: integration.integrationIds,
              integrationPeriod: integration.integrationPeriod,
              integrationUnapplied: true,
            });
          setIntegrationNotice(
            report.description ? '标准输入已重新检查，尚未保存' : '输入未通过，无法生成',
          );
        } catch (error) {
          if (generation !== epoch.current) return false;
          const text = errorText(error);
          setIntegrationNotice(`标准输入须重新检查：${text}`);
          setNotice({ tone: 'error', text: `批次已应用；重新检查标准输入失败：${text}` });
        }
      }
      changeConfirmation.current?.(true);
      changeConfirmation.current = null;
      return true;
    } catch (error) {
      if (generation !== epoch.current) return false;
      patchState({ changePreview: null, preparedChangeSet: null });
      setNotice({ tone: 'error', text: `整批应用被拒绝，草稿保留：${errorText(error)}` });
      changeConfirmation.current?.(false);
      changeConfirmation.current = null;
      return false;
    } finally {
      if (generation === epoch.current) {
        running.current = false;
        setBusy(null);
      }
    }
  }

  async function verificationOwnedFailure(delayMs = 0) {
    await guardContext('验证专用受管失败', () =>
      run(
        '验证专用受管失败',
        () => call<OwnedVerificationFailure>('verification_owned_failure', { delayMs }),
        (result) => {
          field('toolWindow')('log');
          setNotice({
            tone: 'error',
            text: `${result.scope} · ${result.status} · exit ${result.exitCode ?? '无退出码'} · descendants reclaimed: ${result.descendantsReclaimed}`,
          });
        },
      ),
    );
  }

  async function configureAppearance() {
    if (running.current) return;
    const generation = epoch.current;
    running.current = true;
    setBusy('保存外观');
    try {
      await call<void>('configure_appearance', { appearance: stateRef.current.appearanceDraft });
      setSettingsNotice('外观已保存；工程输入保持。');
    } catch (error) {
      setSettingsNotice(errorText(error));
    } finally {
      if (generation === epoch.current) {
        running.current = false;
        setBusy(null);
      }
    }
  }

  async function importDefinitionCatalog() {
    if (!native || running.current) return;
    try {
      const catalogPath = await open({
        multiple: false,
        filters: [{ name: 'Definition catalog', extensions: ['json'] }],
        title: '选择扩展 catalog.json',
      });
      if (typeof catalogPath !== 'string') return;
      if (
        !(await confirmAction(
          `明确接纳此扩展定义目录？后台将核对版次、完整成员和两层摘要；失败保留旧目录。\\n${catalogPath}`,
        ))
      )
        return;
      await guardContext('接纳扩展定义', () =>
        run(
          '接纳扩展定义',
          () => call<ProjectProjection>('import_definition_catalog', { catalogPath }),
          (projection) => {
            patchState({ projection });
            invalidateAfterEdit();
            setSettingsNotice('扩展已明确接纳；请预览保存工程以持久化接纳集合。');
          },
        ),
      );
    } catch (error) {
      setSettingsNotice(`导入被拒绝：${errorText(error)}`);
    }
  }

  async function removeDefinitionCatalog(catalogId: string) {
    if (
      !(await confirmAction(
        `移除工程接纳的扩展 ${catalogId}？源文件保留，受影响消费者将失去对应定义。`,
      ))
    )
      return;
    await guardContext('移除扩展定义', () =>
      run(
        '移除扩展定义',
        () => call<ProjectProjection>('remove_definition_catalog', { catalogId }),
        (projection) => {
          patchState({ projection });
          invalidateAfterEdit();
          setSettingsNotice('接纳集合已改变，尚未保存；原 ARXML 与本机缓存保持。');
        },
      ),
    );
  }

  async function openMemberProject() {
    await replaceProject('打开成员工程', async () => {
      const path = await open({
        multiple: false,
        filters: [{ name: 'Workbench project', extensions: ['json'] }],
        title: '选择 workbench-project.json',
      });
      if (typeof path !== 'string') return;
      await run(
        '打开成员工程',
        () => call<WorkspaceView>('open_member_project', { path }),
        (view) => {
          applyProject(view);
          field('memberProjectPath')(path);
        },
      );
    });
  }

  async function previewSaveAs() {
    await guardContext('保存为工程', async () => {
      const current = stateRef.current;
      if (!current.projectDirectory || !current.projectName.trim()) {
        setNotice({ tone: 'error', text: '请输入工程名称并选择新空目录。' });
        return;
      }
      await run(
        '预览保存为工程',
        () =>
          call<ProjectCreationPreview>('preview_save_as_project', {
            directory: current.projectDirectory,
            name: current.projectName,
          }),
        (projectPreview) => patchState({ projectPreview, projectPreviewKind: 'save-as' }),
      );
    });
  }

  async function confirmProjectPreview() {
    const current = stateRef.current;
    const preview = current.projectPreview;
    if (!preview) return;
    await run(
      current.projectPreviewKind === 'create' ? '创建工程' : '保存为工程',
      () =>
        call<WorkspaceView>(
          current.projectPreviewKind === 'create'
            ? 'create_project_previewed'
            : 'save_as_project_previewed',
          { preview },
        ),
      (view) => {
        const manifest = preview.files.find(
          (file) => file.path.split(/[\\/]/).pop() === 'workbench-project.json',
        );
        patchState({ projectPreview: null });
        applyProject(view);
        if (manifest)
          field('memberProjectPath')(
            /^(?:[A-Za-z]:[\\/]|[\\/])/.test(manifest.path)
              ? manifest.path
              : `${preview.directory.replace(/[\\/]$/, '')}/${manifest.path}`,
          );
      },
    );
  }

  const resumeReplacement = useEffectEvent(async () => {
    if (!stateRef.current.replacementReady || stateRef.current.busy || running.current) return;
    const action = pendingReplacement.current;
    pendingReplacement.current = null;
    patchState({ replacementReady: false, savingForReplacement: false });
    await action?.();
  });
  useEffect(() => {
    void resumeReplacement();
  }, [state.replacementReady, state.busy]);

  const inspectCurrentIntegration = useEffectEvent(() => inspectIntegration());
  useEffect(() => {
    if (
      !native ||
      !workspace ||
      state.integrationInspection ||
      state.integrationIssues.length ||
      !state.capabilities
    )
      return;
    if (!workspace.integrationCandidate && state.activeDocument.kind !== 'integration') return;
    if (!running.current) inspectCurrentIntegration();
  }, [
    workspace,
    state.activeDocument.kind,
    state.integrationInspection,
    state.integrationIssues.length,
    state.capabilities,
  ]);

  return {
    ...state,
    setApplicationPreview: field('applicationPreview'),
    setApplicationWarnings: field('applicationWarnings'),
    setApplicationRecoveryFiles: field('applicationRecoveryFiles'),
    previewApplicationInitialization,
    initializeApplicationPreviewed,
    setHandoffImportOpen: field('handoffImportOpen'),
    setHandoffImportDestination: field('handoffImportDestination'),
    setHandoffImportMode: field('handoffImportMode'),
    confirmHandoffImport,
    setTemplateId: field('templateId'),
    setProjectPreview: field('projectPreview'),
    setProjectPreviewKind: field('projectPreviewKind'),
    setReplacementReady: field('replacementReady'),
    setSavingForReplacement: field('savingForReplacement'),
    cancelSavePreview,
    setMemberProjectPath: field('memberProjectPath'),
    previewSaveAs,
    confirmProjectPreview,
    applyCurrentDrafts: applyDrafts,
    restoreAllDrafts: discardDrafts,
    setProjection: field('projection'),
    setObjectSelection: field('objectSelection'),
    setActiveObjectId: field('activeObjectId'),
    setActiveSourceId: field('activeSourceId'),
    setSourceText: field('sourceText'),
    setChanges: field('changes'),
    setChangePreview: (value) => {
      field('changePreview')(value);
      if (value === null) {
        changeConfirmation.current?.(false);
        changeConfirmation.current = null;
      }
    },
    setPreparedChangeSet: field('preparedChangeSet'),
    setGuard: field('guard'),
    setTabs: field('tabs'),
    setActiveDocument: field('activeDocument'),
    setToolWindow: field('toolWindow'),
    setTreeVisible: field('treeVisible'),
    setInspectorVisible: field('inspectorVisible'),
    setInspectorTab: field('inspectorTab'),
    setTreeMode: field('treeMode'),
    setTreeFilter: field('treeFilter'),
    setTreeRevealId: field('treeRevealId'),
    setObjectFilter: field('objectFilter'),
    setAppearanceDraft: field('appearanceDraft'),
    setPreflight: field('preflight'),
    setOperationLog: field('operationLog'),
    actionReason,
    refreshProjection,
    selectObject,
    readSource,
    openDocument,
    closeDocument,
    guardContext,
    replaceProject,
    resolveGuard,
    requestSave,
    stageChange,
    prepareChanges,
    applyChanges,
    configureAppearance,
    verificationOwnedFailure,
    importDefinitionCatalog,
    removeDefinitionCatalog,
    openMemberProject,
    native,
    workspace,
    setWorkspace,
    integrationUnapplied,
    setIntegrationUnapplied,
    integrationProcessing,
    setIntegrationProcessing,
    selection,
    setSelection,
    draft,
    setDraft,
    diagnosticDraft,
    setDiagnosticDraft,
    diagnosticSignal,
    setDiagnosticSignal,
    diagnosticError,
    setDiagnosticError,
    dtcDraft,
    setDtcDraft,
    dtcError,
    setDtcError,
    source,
    setSource,
    projectName,
    setProjectName,
    projectDirectory,
    setProjectDirectory,
    importPaths,
    setImportPaths,
    importPathText,
    setImportPathText,
    creating,
    setCreating,
    frameInput,
    setFrameInput,
    signalInput,
    setSignalInput,
    stages,
    setStages,
    generated,
    setGenerated,
    generationPreview,
    setGenerationPreview,
    generationPreviewPath,
    setGenerationPreviewPath,
    generationKind,
    setGenerationKind,
    handoffGenerated,
    setHandoffGenerated,
    legacyTarget,
    setLegacyTarget,
    buildDirectory,
    setBuildDirectory,
    built,
    setBuilt,
    virtualResult,
    setVirtualResult,
    virtualKind,
    setVirtualKind,
    operationIssues,
    setOperationIssues,
    notice,
    setNotice,
    busy,
    setBusy,
    peerDirectory,
    setPeerDirectory,
    peerBinaryPath,
    setPeerBinaryPath,
    savePreview,
    setSavePreview,
    previewPath,
    setPreviewPath,
    currentFrame,
    currentSignal,
    signalFrame,
    currentFile,
    focusedFrame,
    frameUnapplied,
    diagnosticUnapplied,
    dtcUnapplied,
    unapplied,
    eligibleSignals,
    eligibleMonitorFrames,
    issues,
    errorCount,
    acceptView,
    invalidateAfterEdit,
    markStage,
    requireReady,
    run,
    chooseDirectory,
    chooseBinary,
    chooseFiles,
    confirmAction,
    choose,
    openCreator,
    applyProject,
    startProject,
    openProjectEntry,
    createProject,
    importProject,
    importHandoff,
    addFrame,
    addSignal,
    updateSelected,
    configureDiagnostic,
    configureDtc,
    clearDtc,
    clearDiagnostic,
    saveProject,
    confirmSave,
    validateProject,
    generateProject,
    confirmGenerate,
    buildProject,
    runVirtual,
    runDiagnostic,
    unsupportedIssue,
    previewFile,
    delta,
    generationPreviewFile,
    generationDelta,
    disabled,
    setCapabilities,
    setSettingsOpen,
    setResourceDraft,
    setToolDraft,
    setSettingsNotice,
    setOperationGeneration,
    configureResources,
    configureTools,
    changeTarget,
    cancelOperation,
    importHandoffDirectory,
    setIntegrationInspection,
    setIntegrationIssues,
    setIntegrationIds,
    setIntegrationPeriod,
    setIntegrationPreview,
    setIntegrationPreviewPath,
    setIntegrationNotice,
    setEcuOutputDirectory,
    setEcuImportDirectory,
    inspectIntegration,
    applyIntegration,
    previewIntegrationSave,
    saveIntegration,
    reopenIntegration,
    restoreIntegrationDraft,
    previewEcu,
    generateEcu,
    preflightEcu,
    buildEcu,
    verifyEcu,
    changeEcuOutput,
    changeEcuBuildDirectory,
    executionDisabled,
    executionReason,
  };
}
