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
  GenerationPreview,
  IntegrationInspection,
  Issue,
  OwnedVerificationFailure,
  PlanDiagnostic,
  SavePreview,
  WorkbenchCapabilities,
  WorkbenchReply,
  WorkspaceView,
} from '../types';
import { createBatchEditing } from './batchEditing';
import type { Command } from './commands';
import { workbenchCommands } from './commands';
import { createDeliveryActions } from './deliveryActions';
import { rememberDialogOpener } from './Dialog';
import type { PreviewDelta, Stage, StageState } from './forms';
import {
  applicationPeriodDraft,
  applicationPeriodMs,
  distinct,
  errorText,
  headline,
  intInRange,
  previewDelta,
} from './forms';
import type { WorkbenchOperation } from './operation';
import { createProjectActions } from './projectActions';
import type {
  ConfigurationChange,
  ConfigurationDiagnostic,
  DocumentTab,
  ProjectProjection,
} from './projectTypes';
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
        | 'projectName'
        | 'projectDirectory'
        | 'importPaths'
        | 'importPathText'
        | 'generationPreview'
        | 'generationPreviewPath'
        | 'generationKind'
        | 'notice'
        | 'previewPath'
        | 'settingsOpen'
        | 'settingsCategory'
        | 'toolDraft'
        | 'settingsNotice'
        | 'toolErrors'
        | 'integrationIds'
        | 'integrationPeriod'
        | 'integrationPreview'
        | 'integrationPreviewPath'
        | 'ecuImportDirectory'
      >
    > {
  native: boolean;
  executionReason: Text;
  /** Availability, reason and action of every user command; see `workbenchCommands`. */
  commands: Command[];
  unapplied: boolean;
  /** Automatic diagnostics, explicit validation findings and standard-input findings, each listed once. */
  diagnostics: ConfigurationDiagnostic[];
  issues: Issue[];
  problemCount: number;
  errorCount: number;
  previewFile: SavePreview['files'][number] | undefined;
  delta: PreviewDelta | null;
  generationPreviewFile: GenerationPreview['files'][number] | undefined;
  generationDelta: PreviewDelta | null;
  disabled: boolean;
  chooseDirectory(onChoose: (path: string) => void): Promise<void>;
  chooseBinary(onChoose: (path: string) => void): Promise<void>;
  chooseFiles(): Promise<void>;
  confirmAction(message: Text): Promise<boolean>;
  startProject(source?: 'empty' | 'import'): Promise<void>;
  openProjectEntry(source: WorkbenchState['source']): void;
  createProject(): void;
  importProject(): void;
  importHandoff(): void;
  saveProject(): void;
  confirmSave(): void;
  validateProject(): void;
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
  generateEcu(): void;
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
          buildTarget: capabilities.target,
          appearanceDraft: capabilities.appearance,
          languageDraft: capabilities.language ?? 'system',
          savedLanguage: capabilities.language ?? 'system',
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
        buildTarget: reply.capabilities.target,
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
            buildTarget: current.target,
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
        ...(['integrationIds', 'integrationPeriod', 'changes'].includes(key as string)
          ? { changePreview: null, preparedChangeSet: null, applicationPreview: null }
          : {}),
      } as Partial<WorkbenchState>;
      stateRef.current = { ...previous, ...patch };
      dispatch({ type: 'patch', patch });
    };
  }

  const setSettingsOpen = field('settingsOpen');

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
    source,
    projectName,
    projectDirectory,
    importPaths,
    importPathText,
    stages,
    generated,
    generationPreview,
    generationPreviewPath,
    generationKind,
    buildTarget,
    buildDirectory,
    built,
    operationIssues,
    notice,
    busy,
    savePreview,
    previewPath,
  } = state;

  const setWorkspace = field('workspace');

  const setIntegrationUnapplied = field('integrationUnapplied');

  const setIntegrationProcessing = field('integrationProcessing');

  const setProjectName = field('projectName');

  const setProjectDirectory = field('projectDirectory');

  const setImportPaths = field('importPaths');

  const setImportPathText = field('importPathText');

  const setStages = field('stages');

  const setGenerated = field('generated');

  const setGenerationPreview = field('generationPreview');

  const setGenerationPreviewPath = field('generationPreviewPath');

  const setGenerationKind = field('generationKind');

  const setBuilt = field('built');

  const setOperationIssues = field('operationIssues');

  const setNotice = field('notice');

  const setBusy = field('busy');

  const setSavePreview = field('savePreview');

  const setPreviewPath = field('previewPath');

  const unapplied = integrationUnapplied || state.changes.length > 0;

  const diagnostics = distinct(state.projection?.diagnostics ?? []);

  const issues = distinct([...(workspace?.issues ?? []), ...operationIssues]);

  const integrationProblems = distinct(state.integrationIssues);

  const problemCount = diagnostics.length + issues.length + integrationProblems.length;

  const errorCount =
    diagnostics.filter((issue) => issue.severity === 'error').length +
    issues.filter((issue) => issue.severity === 'error').length +
    integrationProblems.length;

  function acceptView(view: WorkspaceView) {
    setSavePreview(null);
    setGenerationPreview(null);
    setWorkspace(view);
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
    field('virtualResult')(null);
    setOperationIssues([]);
    field('preflight')(null);
    setNotice(null);
  }

  function markStage(key: Stage, state: StageState, detail: Text) {
    setStages((previous) => ({ ...previous, [key]: { state, detail } }));
  }

  function requireReady() {
    if (!native) throw new ProductError(message('controller.error.desktopRequired'));
    if (stateRef.current.changes.length)
      throw new ProductError(message('controller.error.batchDraft'));
  }

  async function run<T>(
    operation: WorkbenchOperation,
    job: () => Promise<T>,
    onSuccess: (result: T) => void,
    stage?: Stage,
  ) {
    const label = operation.label;
    if (running.current) return;
    rememberDialogOpener();
    const generation = epoch.current;
    try {
      requireReady();
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
        text: composedMessage('controller.operation.failed', { label, error: headline(error) }),
        detail: text,
      });
      if (operation.kind === 'handoffImport') field('handoffImportOpen')(true);
      if (stateRef.current.settingsOpen) setSettingsNotice({ tone: 'error', text });
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
      integrationPeriod: applicationPeriodDraft(report.description),
      integrationProcessing: false,
    });
  }

  function restoreIntegrationDraft() {
    if (state.integrationInspection) acceptIntegration(state.integrationInspection);
  }

  function inspectIntegration() {
    void run(
      { kind: 'action', label: message('activity.validate') },
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
        // Standard inputs are validated by this inspection; every surface reads this stage.
        markStage(
          'validate',
          report.description ? 'done' : 'failed',
          report.description
            ? message('workflow.stage.integrationValidated')
            : message('workflow.stage.blockingErrors', { count: report.diagnostics.length }),
        );
      },
    );
  }

  function applyIntegration() {
    if (applicationPeriodMs(state.integrationInspection?.description) === null) {
      setIntegrationNotice(message('shell.integration.parametersUnavailable'));
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
        { kind: 'integrationEdit', label: message('activity.apply') },
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
    await replaceProject(message('activity.reopen'), () =>
      run(
        { kind: 'action', label: message('activity.reopen') },
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
    await guardContext(message('activity.verificationFailure'), () =>
      run(
        { kind: 'action', label: message('activity.verificationFailure') },
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
  const { stageChange, prepareChanges, applyChanges, discardChanges, cancelChangePreview } =
    createBatchEditing(session, { invalidateAfterEdit, acceptIntegration });
  const { changeTarget, configureTools, configureAppearance, configureLanguage } =
    createSettingsActions(session, { invalidateOperation });
  const {
    saveProject,
    confirmSave,
    validateProject,
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
    unapplied,
    acceptIntegration,
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

  const controller: Omit<Workbench, 'commands'> = {
    ...state,
    setApplicationPreview: field('applicationPreview'),
    previewApplicationInitialization,
    initializeApplicationPreviewed,
    setHandoffImportOpen: field('handoffImportOpen'),
    setHandoffImportDestination: field('handoffImportDestination'),
    confirmHandoffImport,
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
    source,
    projectName,
    setProjectName,
    projectDirectory,
    setProjectDirectory,
    importPaths,
    setImportPaths,
    importPathText,
    setImportPathText,
    stages,
    generated,
    generationPreview,
    setGenerationPreview,
    generationPreviewPath,
    setGenerationPreviewPath,
    generationKind,
    setGenerationKind,
    buildTarget,
    buildDirectory,
    built,
    operationIssues,
    notice,
    setNotice,
    busy,
    savePreview,
    previewPath,
    setPreviewPath,
    unapplied,
    diagnostics,
    issues,
    problemCount,
    errorCount,
    chooseDirectory,
    chooseBinary,
    chooseFiles,
    confirmAction,
    startProject,
    openProjectEntry,
    createProject,
    importProject,
    importHandoff,
    saveProject,
    confirmSave,
    validateProject,
    previewFile,
    delta,
    generationPreviewFile,
    generationDelta,
    disabled,
    setSettingsOpen,
    setSettingsCategory: field('settingsCategory'),
    setToolDraft,
    setSettingsNotice,
    setToolErrors: field('toolErrors'),
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
    executionReason,
  };
  return { ...controller, commands: workbenchCommands(controller) };
}
