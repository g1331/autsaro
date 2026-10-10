import type { Text } from '../i18n';
import {
  composedMessage,
  isLocalizedText,
  message,
  previewLanguage,
  ProductError,
  translate,
  useLocale,
} from '../i18n';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { Dispatch, SetStateAction } from 'react';
import { useEffect, useEffectEvent, useReducer, useRef } from 'react';
import type {
  BuildTarget,
  Frame,
  GenerationPreview,
  IntegrationInspection,
  Issue,
  OwnedVerificationFailure,
  PlanDiagnostic,
  SavePreview,
  Signal,
  WorkbenchCapabilities,
  WorkbenchReply,
  WorkspaceView,
} from '../types';
import { createBatchEditing } from './batchEditing';
import { createDeliveryActions } from './deliveryActions';
import { rememberDialogOpener } from './Dialog';
import type { PreviewDelta, Selection, Stage, StageState } from './forms';
import {
  diagnosticFields,
  draftFor,
  dtcFields,
  errorText,
  findSelection,
  hasUnapplied,
  intInRange,
  previewDelta,
} from './forms';
import { createLegacyEditing } from './legacyEditing';
import type { WorkbenchOperation } from './operation';
import { createProjectActions } from './projectActions';
import type { ConfigurationChange, DocumentTab, ProjectProjection } from './projectTypes';
import type { WorkbenchSession } from './session';
import { createSettingsActions } from './settingsActions';
import type { WorkbenchState } from './state';
import { initialState, reducer } from './state';
import { native } from './useDelivery';

type StateSetters<S> = {
  [K in keyof S as K extends string ? `set${Capitalize<K>}` : never]: Dispatch<
    SetStateAction<S[K]>
  >;
};

export interface Workbench
  extends
    Readonly<WorkbenchState>,
    StateSetters<
      Pick<
        WorkbenchState,
        | 'applicationPreview'
        | 'handoffImportOpen'
        | 'handoffImportDestination'
        | 'handoffImportMode'
        | 'templateId'
        | 'projectPreview'
        | 'toolWindow'
        | 'treeVisible'
        | 'inspectorVisible'
        | 'inspectorTab'
        | 'treeMode'
        | 'treeFilter'
        | 'treeRevealId'
        | 'objectFilter'
        | 'appearanceDraft'
        | 'languageDraft'
        | 'integrationUnapplied'
        | 'draft'
        | 'diagnosticDraft'
        | 'diagnosticSignal'
        | 'diagnosticError'
        | 'dtcDraft'
        | 'dtcError'
        | 'projectName'
        | 'projectDirectory'
        | 'importPaths'
        | 'importPathText'
        | 'creating'
        | 'frameInput'
        | 'signalInput'
        | 'generationPreview'
        | 'generationPreviewPath'
        | 'generationKind'
        | 'virtualResult'
        | 'notice'
        | 'peerDirectory'
        | 'peerBinaryPath'
        | 'previewPath'
        | 'settingsOpen'
        | 'resourceDraft'
        | 'toolDraft'
        | 'settingsNotice'
        | 'integrationIds'
        | 'integrationPeriod'
        | 'integrationPreview'
        | 'integrationPreviewPath'
        | 'ecuImportDirectory'
      >
    > {
  native: boolean;
  executionDisabled: boolean;
  executionReason: Text;
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
  chooseDirectory(onChoose: (path: string) => void): Promise<void>;
  chooseBinary(onChoose: (path: string) => void): Promise<void>;
  chooseFiles(): Promise<void>;
  confirmAction(message: Text): Promise<boolean>;
  choose(selection: Selection): Promise<void>;
  openCreator(kind: 'frame' | 'signal'): Promise<void>;
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
  actionReason(action: string): Text;
  refreshProjection(): Promise<void>;
  selectObject(objectId: string, multiple?: boolean): Promise<void>;
  readSource(sourceId: string): Promise<void>;
  openDocument(tab: DocumentTab): void;
  closeDocument(tab: DocumentTab): Promise<void>;
  guardContext(title: Text, action: () => void | Promise<void>): Promise<void>;
  replaceProject(title: Text, action: () => void | Promise<void>): Promise<void>;
  resolveGuard(choice: 'apply' | 'discard' | 'cancel'): Promise<void>;
  requestSave(): Promise<void>;
  stageChange(change: ConfigurationChange): void;
  discardChanges(changeId?: string): void;
  cancelChangePreview(): void;
  prepareChanges(): Promise<void>;
  applyChanges(): Promise<boolean>;
  configureAppearance(): Promise<void>;
  configureLanguage(): Promise<void>;
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
export function useWorkbench(): Workbench {
  const { language } = useLocale();
  const [state, dispatch] = useReducer(reducer, undefined, initialState);
  useEffect(() => {
    previewLanguage(state.languageDraft);
  }, [state.languageDraft]);
  const updateNativeTitle = useEffectEvent(() => {
    void getCurrentWindow()
      .setTitle(translate('controller.app.title'))
      .catch((error: unknown) => {
        patchState({
          notice: {
            tone: 'error',
            text: composedMessage('controller.error.windowTitle', { error: errorText(error) }),
          },
        });
      });
  });
  useEffect(() => {
    if (native) updateNativeTitle();
  }, [language]);

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
          languageDraft: capabilities.language ?? 'system',
          savedLanguage: capabilities.language ?? 'system',
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
    if (!capabilities) throw new ProductError(message('controller.error.capabilitiesNotReady'));
    const generation = epoch.current;
    const fingerprint = capabilities.fingerprint;
    try {
      const reply = await invoke<WorkbenchReply<T>>(command, { ...payload, fingerprint });
      if (generation !== epoch.current || reply.inputFingerprint !== fingerprint) {
        throw new ProductError(message('controller.error.staleDelivery'));
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
      const detail: Text =
        value && 'log' in value
          ? errorText(value.log)
          : value && 'logs' in value && Array.isArray(value.logs)
            ? (value.logs as Text[])
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
          const presentationOnly =
            command === 'configure_language' || command === 'configure_appearance';
          patchState({
            capabilities: current,
            legacyTarget: current.target,
            ...(presentationOnly
              ? {}
              : { changePreview: null, preparedChangeSet: null, applicationPreview: null }),
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
          next[key] = { state: 'stale', detail: message('controller.operation.cancelledResult') };
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
      patchState({ notice: { tone: 'info', text: message('controller.operation.cancelled') } });
    } catch (error) {
      setNotice({
        tone: 'error',
        text: composedMessage('controller.operation.cancelFailed', { error: errorText(error) }),
      });
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

  const setSettingsOpen = field('settingsOpen');

  const setResourceDraft = field('resourceDraft');

  const setToolDraft = field('toolDraft');

  const setSettingsNotice = field('settingsNotice');

  const setIntegrationIssues = field('integrationIssues');

  const setIntegrationIds = field('integrationIds');

  const setIntegrationPeriod = field('integrationPeriod');

  const setIntegrationPreview = field('integrationPreview');

  const setIntegrationPreviewPath = field('integrationPreviewPath');

  const setIntegrationNotice = field('integrationNotice');

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

  const setBuilt = field('built');

  const setVirtualResult = field('virtualResult');

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
      save: { state: 'stale', detail: message('controller.stage.editedSave') },
      validate: { state: 'stale', detail: message('controller.stage.editedValidate') },
      generate: { state: 'stale', detail: message('controller.stage.editedGenerate') },
      build: { state: 'stale', detail: message('controller.stage.editedBuild') },
      virtual: { state: 'stale', detail: message('controller.stage.editedVirtual') },
    });
    setGenerationPreview(null);
    setGenerated(null);
    setBuilt(null);
    setVirtualResult(null);
    setOperationIssues([]);
    field('preflight')(null);
    setNotice(null);
  }

  function markStage(key: Stage, state: StageState, detail: Text) {
    setStages((previous) => ({ ...previous, [key]: { state, detail } }));
  }

  function requireReady(allowed?: 'frame' | 'diagnostic' | 'dtc') {
    if (!native) throw new ProductError(message('controller.error.desktopRequired'));
    const current = stateRef.current;
    if (current.creating && allowed !== 'frame')
      throw new ProductError(message('controller.error.creatorDraft'));
    if (hasUnapplied(current.workspace, current.draft) && allowed !== 'frame')
      throw new ProductError(message('controller.error.inspectorDraft'));
    if (
      current.workspace &&
      JSON.stringify(current.diagnosticDraft) !==
        JSON.stringify(diagnosticFields(current.workspace.diagnostic)) &&
      allowed !== 'diagnostic'
    )
      throw new ProductError(message('controller.error.diagnosticDraft'));
    if (
      current.workspace?.diagnostic &&
      JSON.stringify(current.dtcDraft) !==
        JSON.stringify(dtcFields(current.workspace.diagnostic.dtc)) &&
      allowed !== 'dtc'
    )
      throw new ProductError(message('controller.error.dtcDraft'));
    if (current.changes.length) throw new ProductError(message('controller.error.batchDraft'));
  }

  async function run<T>(
    operation: WorkbenchOperation,
    job: () => Promise<T>,
    onSuccess: (result: T) => void,
    stage?: Stage,
    allowed?: 'frame' | 'diagnostic' | 'dtc',
  ) {
    const label = operation.label;
    if (running.current) return;
    rememberDialogOpener();
    const generation = epoch.current;
    try {
      requireReady(allowed);
      running.current = true;
      setBusy(label);
      setNotice(null);
      if (stage)
        markStage(stage, 'running', composedMessage('controller.operation.running', { label }));
      const result = await job();
      if (generation !== epoch.current) return;
      onSuccess(result);
      if (generation === epoch.current && capabilitiesRef.current?.hasWorkspace)
        await refreshProjection();
    } catch (error) {
      if (generation !== epoch.current) return;
      const text = errorText(error);
      setNotice({
        tone: 'error',
        text: composedMessage('controller.operation.failed', { label, error: text }),
      });
      if (operation.kind === 'handoffImport') field('handoffImportOpen')(true);
      if (stateRef.current.settingsOpen) setSettingsNotice(text);
      if (
        workspace?.integrationCandidate &&
        Array.isArray(error) &&
        error.length > 0 &&
        error.every(
          (issue: unknown) =>
            typeof issue === 'object' &&
            issue !== null &&
            'category' in issue &&
            typeof issue.category === 'string' &&
            ['input', 'unsupported', 'dependency', 'tool'].includes(issue.category) &&
            'code' in issue &&
            typeof issue.code === 'string' &&
            'file' in issue &&
            (issue.file === null || typeof issue.file === 'string') &&
            'object' in issue &&
            (issue.object === null || typeof issue.object === 'string') &&
            'message' in issue &&
            isLocalizedText(issue.message) &&
            'remedy' in issue &&
            isLocalizedText(issue.remedy),
        )
      ) {
        setIntegrationIssues(error as PlanDiagnostic[]);
        setIntegrationNotice(
          operation.kind === 'integrationEdit'
            ? message('controller.integration.editRejected')
            : composedMessage('controller.operation.failedLabel', { label }),
        );
      }
      if (stage) markStage(stage, 'failed', text);
      if (stateRef.current.savingForReplacement && operation.kind === 'savePreview') {
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
    : message('controller.execution.unsupported');
  const executionDisabled =
    disabled || !state.capabilities?.nativeExecution || Boolean(state.capabilities.toolError);

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
      integrationPeriod: report.description?.component
        ? String(report.description.component.periodMs)
        : '',
      integrationProcessing: false,
    });
  }

  function restoreIntegrationDraft() {
    if (state.integrationInspection) acceptIntegration(state.integrationInspection);
  }

  function inspectIntegration() {
    void run(
      { kind: 'action', label: message('controller.integration.inspect') },
      () => call<IntegrationInspection>('inspect_integration'),
      (report) => {
        acceptIntegration(report);
        setIntegrationNotice(
          message(
            report.description
              ? 'controller.integration.validated'
              : 'controller.integration.failed',
          ),
        );
      },
    );
  }

  function applyIntegration() {
    if (state.integrationInspection?.description?.multi) {
      setIntegrationNotice(message('shell.integration.multiReadOnly'));
      return;
    }
    const ids: Record<string, number> = {};
    try {
      for (const [path, value] of Object.entries(state.integrationIds)) {
        ids[path] = intInRange(value, 'CAN ID', 0, 2047);
      }
      const period = intInRange(
        state.integrationPeriod,
        message('controller.field.applicationPeriod'),
        1,
        2147483647,
      );
      void run(
        { kind: 'integrationEdit', label: message('controller.integration.apply') },
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
          setIntegrationNotice(message('controller.integration.modified'));
        },
      );
    } catch (error) {
      setNotice({ tone: 'error', text: errorText(error) });
    }
  }

  async function reopenIntegration() {
    if (!workspace) return;
    const path = stateRef.current.memberProjectPath;
    await replaceProject(message('controller.project.reopen'), () =>
      run(
        { kind: 'action', label: message('controller.project.reopen') },
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

  function patchState(patch: Partial<WorkbenchState>) {
    stateRef.current = { ...stateRef.current, ...patch };
    dispatch({ type: 'patch', patch });
  }

  function actionReason(action: string) {
    if (!native) return message('controller.error.desktopRequired');
    if (!state.capabilities) return message('controller.capability.reading');
    if (state.busy) return composedMessage('controller.operation.busy', { label: state.busy });
    const capability =
      state.projection?.capabilities.find((item) => item.action === action) ??
      state.capabilities.actions.find((item) => item.action === action);
    return !capability
      ? message('controller.capability.missingAction')
      : !capability.available
        ? (capability.reason ?? message('controller.capability.unavailable'))
        : '';
  }

  async function refreshProjection() {
    if (!capabilitiesRef.current?.hasWorkspace) return;
    try {
      const projection = await call<ProjectProjection>('project_projection');
      patchState({ projection, memberProjectPath: projection.projectPath });
    } catch (error) {
      setNotice({
        tone: 'error',
        text: composedMessage('controller.project.projectionFailed', { error: errorText(error) }),
      });
    }
  }

  async function verificationOwnedFailure(delayMs = 0) {
    await guardContext(message('controller.verification.failure'), () =>
      run(
        { kind: 'action', label: message('controller.verification.failure') },
        () => call<OwnedVerificationFailure>('verification_owned_failure', { delayMs }),
        (result) => {
          field('toolWindow')('log');
          setNotice({
            tone: 'error',
            text: composedMessage(
              'controller.verification.result',
              {
                exitCode:
                  result.exitCode == null
                    ? message('controller.verification.noExitCode')
                    : String(result.exitCode),
              },
              {
                scope: result.scope,
                status: result.status,
                reclaimed: result.descendantsReclaimed,
              },
            ),
          });
        },
      ),
    );
  }

  const session: WorkbenchSession = {
    state,
    stateRef,
    capabilitiesRef,
    epoch,
    running,
    pendingGuard,
    pendingReplacement,
    changeConfirmation,
    field,
    patchState,
    call,
  };
  const {
    chooseDirectory,
    chooseBinary,
    chooseFiles,
    confirmAction,
    choose,
    openCreator,
    applyProject,
    openProjectEntry,
    startProject,
    createProject,
    importProject,
    importHandoff,
    importHandoffDirectory,
    confirmHandoffImport,
    previewApplicationInitialization,
    initializeApplicationPreviewed,
    openDocument,
    closeDocument,
    discardDrafts,
    guardContext,
    replaceProject,
    applyDrafts,
    resolveGuard,
    requestSave,
    continueReplacement,
    cancelSavePreview,
    selectObject,
    readSource,
    importDefinitionCatalog,
    removeDefinitionCatalog,
    openMemberProject,
    previewSaveAs,
    confirmProjectPreview,
  } = createProjectActions(session, {
    invalidateOperation,
    acceptView,
    refreshProjection,
    run,
    invalidateAfterEdit,
    markStage,
    acceptIntegration,
    prepareChanges: (...args) => prepareChanges(...args),
    previewIntegrationSave: (...args) => previewIntegrationSave(...args),
    saveProject: (...args) => saveProject(...args),
  });
  const {
    addFrame,
    addSignal,
    updateSelected,
    configureDiagnostic,
    configureDtc,
    clearDtc,
    clearDiagnostic,
  } = createLegacyEditing(session, {
    run,
    invalidateAfterEdit,
    acceptView,
    focusedFrame,
    diagnosticUnapplied,
    frameUnapplied,
    unapplied,
    confirmAction: (...args) => confirmAction(...args),
    dtcUnapplied,
  });
  const { stageChange, prepareChanges, applyChanges, discardChanges, cancelChangePreview } =
    createBatchEditing(session, { invalidateAfterEdit, acceptIntegration });
  const {
    changeTarget,
    configureResources,
    configureTools,
    configureAppearance,
    configureLanguage,
  } = createSettingsActions(session, { invalidateOperation });
  const {
    saveProject,
    confirmSave,
    validateProject,
    generateProject,
    confirmGenerate,
    buildProject,
    runVirtual,
    runDiagnostic,
    previewIntegrationSave,
    saveIntegration,
    previewEcu,
    generateEcu,
    preflightEcu,
    buildEcu,
    verifyEcu,
    changeEcuOutput,
    changeEcuBuildDirectory,
  } = createDeliveryActions(session, {
    run,
    acceptView,
    markStage,
    continueReplacement,
    chooseDirectory,
    openDocument,
    unapplied,
    acceptIntegration,
    confirmAction,
  });

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
    previewApplicationInitialization,
    initializeApplicationPreviewed,
    setHandoffImportOpen: field('handoffImportOpen'),
    setHandoffImportDestination: field('handoffImportDestination'),
    setHandoffImportMode: field('handoffImportMode'),
    confirmHandoffImport,
    setTemplateId: field('templateId'),
    setProjectPreview: field('projectPreview'),
    cancelSavePreview,
    previewSaveAs,
    confirmProjectPreview,
    applyCurrentDrafts: applyDrafts,
    restoreAllDrafts: discardDrafts,
    setToolWindow: field('toolWindow'),
    setTreeVisible: field('treeVisible'),
    setInspectorVisible: field('inspectorVisible'),
    setInspectorTab: field('inspectorTab'),
    setTreeMode: field('treeMode'),
    setTreeFilter: field('treeFilter'),
    setTreeRevealId: field('treeRevealId'),
    setObjectFilter: field('objectFilter'),
    setAppearanceDraft: field('appearanceDraft'),
    setLanguageDraft: field('languageDraft'),
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
    discardChanges,
    cancelChangePreview,
    prepareChanges,
    applyChanges,
    configureAppearance,
    configureLanguage,
    verificationOwnedFailure,
    importDefinitionCatalog,
    removeDefinitionCatalog,
    openMemberProject,
    native,
    workspace,
    integrationUnapplied,
    setIntegrationUnapplied,
    integrationProcessing,
    selection,
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
    generated,
    generationPreview,
    setGenerationPreview,
    generationPreviewPath,
    setGenerationPreviewPath,
    generationKind,
    setGenerationKind,
    handoffGenerated,
    legacyTarget,
    buildDirectory,
    built,
    virtualResult,
    setVirtualResult,
    virtualKind,
    operationIssues,
    notice,
    setNotice,
    busy,
    peerDirectory,
    setPeerDirectory,
    peerBinaryPath,
    setPeerBinaryPath,
    savePreview,
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
    chooseDirectory,
    chooseBinary,
    chooseFiles,
    confirmAction,
    choose,
    openCreator,
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
    setSettingsOpen,
    setResourceDraft,
    setToolDraft,
    setSettingsNotice,
    configureResources,
    configureTools,
    changeTarget,
    cancelOperation,
    importHandoffDirectory,
    setIntegrationIds,
    setIntegrationPeriod,
    setIntegrationPreview,
    setIntegrationPreviewPath,
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
