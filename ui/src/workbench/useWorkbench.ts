import { useEffect, useEffectEvent, useReducer, useRef } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { requestConfirmation } from '../confirmation';
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
import type { IntegrationInspection, PlanDiagnostic, PreflightReport } from '../types';
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
  Page,
} from './forms';
import type { PreviewDelta } from './forms';
import { native, stageDefaults, importedSaveStage } from './useDelivery';

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
  source: 'empty' | 'import';
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
  page: Page;
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
  confirmDiscard(): Promise<boolean>;
  choose(selection: Selection): Promise<void>;
  openCreator(kind: 'frame' | 'signal'): Promise<void>;
  applyProject(view: WorkspaceView): void;
  startProject(): Promise<void>;
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
    page: 'editor',
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
  };
}

export function useWorkbench(): Workbench {
  const [state, dispatch] = useReducer(reducer, undefined, initialState);

  const capabilitiesRef = useRef<WorkbenchCapabilities | null>(null);

  const epoch = useRef(0);

  const running = useRef(false);

  useEffect(() => {
    if (!native) return;
    let live = true;
    const operationEpoch = epoch;
    void invoke<WorkbenchCapabilities>('workbench_capabilities')
      .then((capabilities) => {
        if (!live) return;
        capabilitiesRef.current = capabilities;
        dispatch({
          type: 'patch',
          patch: {
            capabilities,
            legacyTarget: capabilities.target,
            settingsOpen: Boolean(capabilities.resourceError),
            resourceDraft: {
              xsdArchive: capabilities.xsdArchive ?? '',
              modArchive: capabilities.modArchive ?? '',
            },
            toolDraft: capabilities.executionTools ?? {
              compiler: '',
              objdump: '',
              git: '',
              python: '',
            },
          },
        });
      })
      .catch((error) => {
        if (live)
          dispatch({ type: 'patch', patch: { notice: { tone: 'error', text: errorText(error) } } });
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
      capabilitiesRef.current = reply.capabilities;
      dispatch({
        type: 'patch',
        patch: { capabilities: reply.capabilities, legacyTarget: reply.capabilities.target },
      });
      return reply.value;
    } catch (error) {
      if (generation === epoch.current) {
        const current = await invoke<WorkbenchCapabilities>('workbench_capabilities');
        if (generation === epoch.current) {
          capabilitiesRef.current = current;
          dispatch({
            type: 'patch',
            patch: { capabilities: current, legacyTarget: current.target },
          });
        }
      }
      throw error;
    }
  }

  function invalidateOperation() {
    epoch.current++;
    running.current = false;
    setStages((previous) => {
      const next = { ...previous };
      for (const key of Object.keys(next) as Stage[]) {
        if (next[key].state === 'running') {
          next[key] = { state: 'stale', detail: '操作已取消；旧结果未提交' };
        }
      }
      return next;
    });
    dispatch({
      type: 'patch',
      patch: { busy: null, integrationProcessing: false, operationGeneration: epoch.current },
    });
  }

  async function cancelOperation() {
    invalidateOperation();
    try {
      await call<void>('cancel_operation');
      dispatch({
        type: 'patch',
        patch: { notice: { tone: 'info', text: '操作已取消，受管进程已关闭；旧结果未提交。' } },
      });
    } catch (error) {
      setNotice({ tone: 'error', text: `取消未确认：${errorText(error)}` });
    }
  }

  async function changeTarget(target: BuildTarget) {
    invalidateOperation();
    try {
      await call<void>('select_build_target', { target });
      dispatch({
        type: 'patch',
        patch: {
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
      dispatch({
        type: 'patch',
        patch: {
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
    try {
      await call<void>('configure_execution_tools', { tools: state.toolDraft });
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
    }
  }

  function field<K extends keyof WorkbenchState>(
    key: K,
  ): Dispatch<SetStateAction<WorkbenchState[K]>> {
    return (value) =>
      dispatch({
        type: 'patch',
        patch: (previous) => ({
          [key]:
            typeof value === 'function'
              ? (value as (previous: WorkbenchState[K]) => WorkbenchState[K])(previous[key])
              : value,
        }),
      });
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
    page,
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

  const setPage = field('page');

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

  const unapplied = frameUnapplied || diagnosticUnapplied || dtcUnapplied || integrationUnapplied;

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
    const next = findSelection(view, requested === undefined ? selection : requested);
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
    setNotice(null);
  }

  function markStage(key: Stage, state: StageState, detail: string) {
    setStages((previous) => ({ ...previous, [key]: { state, detail } }));
  }

  function requireReady(allowed?: 'frame' | 'diagnostic' | 'dtc') {
    if (!native) throw new Error('需要桌面运行环境');
    if (frameUnapplied && allowed !== 'frame')
      throw new Error('检查器中有未应用的更改，请先应用或还原');
    if (diagnosticUnapplied && allowed !== 'diagnostic')
      throw new Error('DoCAN 配置有未应用的更改，请先应用或还原');
    if (dtcUnapplied && allowed !== 'dtc')
      throw new Error('故障记忆有未应用的更改，请先应用或还原');
  }

  async function run<T>(
    label: string,
    job: () => Promise<T>,
    onSuccess: (result: T) => void,
    stage?: Stage,
    allowed?: 'frame' | 'diagnostic' | 'dtc',
  ) {
    if (running.current) return;
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
    } catch (error) {
      if (generation !== epoch.current) return;
      const text = errorText(error);
      setNotice({ tone: 'error', text: `${label}失败：${text}` });
      if (workspace?.integrationCandidate && Array.isArray(error)) {
        setIntegrationIssues(error as PlanDiagnostic[]);
        setIntegrationNotice(label === '应用标准参数' ? '编辑被拒绝，原配置保持' : `${label}失败`);
      }
      if (stage) markStage(stage, 'failed', text);
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

  async function confirmDiscard(): Promise<boolean> {
    return (
      (!workspace?.dirty && !unapplied) ||
      (await confirmAction('当前配置或检查器有尚未保存的更改。切换项目会丢失这些更改，确定继续？'))
    );
  }

  async function choose(selectionNext: Selection) {
    if (busy || integrationProcessing) return;
    if (frameUnapplied && !(await confirmAction('检查器中有未应用的更改，确定放弃并切换对象？')))
      return;
    setSelection(selectionNext);
    if (workspace) setDraft(draftFor(workspace, selectionNext));
    setCreating(null);
    setPage('editor');
  }

  async function openCreator(kind: 'frame' | 'signal') {
    if (!native || busy || !workspace) return;
    if (diagnosticUnapplied) {
      setDiagnosticError('请先应用或还原 DoCAN 配置草稿，再添加帧或信号');
      return;
    }
    if (dtcUnapplied) {
      setDtcError('请先应用或还原故障记忆草稿，再添加帧或信号');
      return;
    }
    if (frameUnapplied && !(await confirmAction('检查器中有未应用的更改，确定放弃并创建对象？')))
      return;
    if (workspace) setDraft(draftFor(workspace, selection));
    setCreating(kind);
    setNotice(null);
    if (kind === 'frame') setFrameInput(newFrame);
    else setSignalInput(newSignal);
  }

  function applyProject(view: WorkspaceView) {
    setIntegrationUnapplied(false);
    invalidateOperation();
    setIntegrationProcessing(false);
    dispatch({
      type: 'patch',
      patch: {
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
      },
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
    setPage(view.integrationCandidate ? 'integration' : 'editor');
    acceptView(view, initialSelection(view));
    setDiagnosticError('');
  }

  async function startProject() {
    if (integrationProcessing) return;
    if (!(await confirmDiscard())) return;
    try {
      await call<void>('close_project');
      invalidateOperation();
    } catch (error) {
      setNotice({ tone: 'error', text: errorText(error) });
      return;
    }
    dispatch({
      type: 'patch',
      patch: {
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
      },
    });
    setIntegrationUnapplied(false);
    setSelection(null);
    setDiagnosticDraft(diagnosticFields(null));
    setDiagnosticSignal('');
    setDiagnosticError('');
    setDtcDraft(dtcFields(null));
    setDtcError('');
    setDraft(null);
    setSource('empty');
    setProjectName('');
    setProjectDirectory('');
    setImportPaths([]);
    setImportPathText('');
    setNotice(null);
  }

  function createProject() {
    if (!projectDirectory) {
      setNotice({ tone: 'error', text: '请先选择项目目录' });
      return;
    }
    if (!projectName.trim()) {
      setNotice({ tone: 'error', text: '请输入项目名称' });
      return;
    }
    void run(
      '创建项目',
      () =>
        call<WorkspaceView>('create_project', {
          directory: projectDirectory,
          name: projectName.trim(),
        }),
      applyProject,
    );
  }

  function importProject() {
    if (!importPaths.length) {
      setNotice({ tone: 'error', text: '请至少选择一份 ARXML 文件' });
      return;
    }
    void run(
      '导入项目',
      () => call<WorkspaceView>('open_project', { paths: importPaths }),
      applyProject,
    );
  }

  function importHandoff() {
    void chooseDirectory((directory) => {
      void run(
        '导入交付包',
        () => call<WorkspaceView>('open_handoff_project', { directory }),
        applyProject,
      );
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
    if (!savePreview || !savePreview.files.some((file) => file.changed)) return;
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
        setPage('diagnostics');
      },
      'validate',
    );
  }

  function generateProject(handoff = false) {
    if (workspace?.dirty) {
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
        setPage('build');
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
        setPage('build');
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
        setPage('virtual');
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
        setPage('virtual');
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
    !native ||
    !state.capabilities ||
    Boolean(state.capabilities.resourceError) ||
    Boolean(busy) ||
    Boolean(unsupportedIssue);
  const executionReason = state.capabilities?.nativeExecution
    ? (state.capabilities.toolError ?? '')
    : '未执行：本机不支持所选目标执行';
  const executionDisabled =
    disabled || !state.capabilities?.nativeExecution || Boolean(state.capabilities.toolError);

  function importHandoffDirectory(directory: string) {
    void run(
      '导入交付包',
      () => call<WorkspaceView>('open_handoff_project', { directory }),
      applyProject,
    );
  }

  function acceptIntegration(report: IntegrationInspection) {
    dispatch({
      type: 'patch',
      patch: {
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
      },
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
        dispatch({
          type: 'patch',
          patch: {
            integrationPreview: preview,
            integrationIssues: [],
            integrationPreviewPath:
              preview.files.find((file) => file.changed)?.path ?? preview.files[0]?.path ?? '',
          },
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
        dispatch({
          type: 'patch',
          patch: {
            integrationPreview: null,
            generated: null,
            built: null,
            virtualResult: null,
            stages: {
              ...stageDefaults,
              save: {
                state: result.error ? 'failed' : 'done',
                detail: result.error ?? '标准输入已保存',
              },
            },
            integrationNotice: result.error ?? '标准输入已保存，尚未生成运行工程',
          },
        });
        if (result.error) setNotice({ tone: 'error', text: result.error });
      },
      'save',
    );
  }

  async function reopenIntegration() {
    if (!workspace || !(await confirmDiscard())) return;
    void run(
      '重开标准输入',
      () =>
        call<WorkspaceView>('open_project', { paths: workspace.files.map((file) => file.path) }),
      applyProject,
    );
  }

  function changeEcuOutput(path: string) {
    dispatch({
      type: 'patch',
      patch: {
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
      },
    });
  }

  function changeEcuBuildDirectory(path: string) {
    dispatch({
      type: 'patch',
      patch: {
        buildDirectory: path,
        built: null,
        virtualResult: null,
        stages: { ...stages, build: stageDefaults.build, virtual: stageDefaults.virtual },
      },
    });
  }

  function previewEcu() {
    if (workspace?.dirty || unapplied || !state.ecuOutputDirectory.trim()) return;
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
    if (workspace?.dirty || unapplied) return;
    void run(
      '原生编译预检',
      () => call<PreflightReport>('preflight_ecu', { handoff: generationKind === 'handoff' }),
      (report) => {
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

  const inspectCurrentIntegration = useEffectEvent(() => inspectIntegration());
  useEffect(() => {
    if (
      !native ||
      !workspace ||
      state.integrationInspection ||
      state.integrationIssues.length ||
      state.capabilities?.resourceError ||
      !state.capabilities
    )
      return;
    if (!workspace.integrationCandidate && page !== 'integration') return;
    if (!running.current) inspectCurrentIntegration();
  }, [
    workspace,
    page,
    state.integrationInspection,
    state.integrationIssues.length,
    state.capabilities,
  ]);

  return {
    ...state,
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
    page,
    setPage,
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
    confirmDiscard,
    choose,
    openCreator,
    applyProject,
    startProject,
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
