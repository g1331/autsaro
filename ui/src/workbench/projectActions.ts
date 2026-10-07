import { open } from '@tauri-apps/plugin-dialog';
import { requestConfirmation } from '../confirmation';
import type { IntegrationInspection, WorkspaceView } from '../types';
import { rememberDialogOpener } from './Dialog';
import type { Selection, Stage, StageState } from './forms';
import {
  diagnosticChanges,
  diagnosticFields,
  draftFor,
  dtcChanges,
  dtcFields,
  errorText,
  frameChanges,
  hasUnapplied,
  initialSelection,
  intInRange,
  newFrame,
  newSignal,
  signalChanges,
} from './forms';
import type { WorkbenchOperation } from './operation';
import type {
  ApplicationInitializationOutcome,
  ApplicationInitializationPreview,
  DocumentTab,
  ProjectCreationPreview,
  ProjectProjection,
} from './projectTypes';
import type { WorkbenchSession } from './session';
import type { WorkbenchState } from './state';
import { importedSaveStage, native, stageDefaults } from './useDelivery';

interface Dependencies {
  invalidateOperation: () => void;
  acceptView: (view: WorkspaceView, requested?: Selection | null) => void;
  refreshProjection: () => Promise<void>;
  run: <T>(
    operation: WorkbenchOperation,
    job: () => Promise<T>,
    onSuccess: (result: T) => void,
    stage?: Stage,
    allowed?: 'frame' | 'diagnostic' | 'dtc',
  ) => Promise<void>;
  invalidateAfterEdit: () => void;
  markStage: (key: Stage, state: StageState, detail: string) => void;
  acceptIntegration: (report: IntegrationInspection) => void;
  prepareChanges: () => Promise<void>;
  previewIntegrationSave: () => void;
  saveProject: () => void;
}

export function createProjectActions(session: WorkbenchSession, dependencies: Dependencies) {
  const {
    invalidateOperation,
    acceptView,
    refreshProjection,
    run,
    invalidateAfterEdit,
    markStage,
    acceptIntegration,
    prepareChanges,
    previewIntegrationSave,
    saveProject,
  } = dependencies;
  const {
    stateRef,
    field,
    patchState,
    call,
    running,
    pendingGuard,
    epoch,
    changeConfirmation,
    pendingReplacement,
  } = session;
  const { busy, integrationProcessing, workspace, importPaths } = session.state;
  const setNotice = session.field('notice');
  const setImportPaths = session.field('importPaths');
  const setImportPathText = session.field('importPathText');
  const setBusy = session.field('busy');
  const setSelection = session.field('selection');
  const setDraft = session.field('draft');
  const setCreating = session.field('creating');
  const setFrameInput = session.field('frameInput');
  const setSignalInput = session.field('signalInput');
  const setIntegrationUnapplied = session.field('integrationUnapplied');
  const setIntegrationProcessing = session.field('integrationProcessing');
  const setStages = session.field('stages');
  const setGenerated = session.field('generated');
  const setBuilt = session.field('built');
  const setVirtualResult = session.field('virtualResult');
  const setOperationIssues = session.field('operationIssues');
  const setPeerDirectory = session.field('peerDirectory');
  const setPeerBinaryPath = session.field('peerBinaryPath');
  const setBuildDirectory = session.field('buildDirectory');
  const setDiagnosticError = session.field('diagnosticError');
  const setSource = session.field('source');
  const setDiagnosticDraft = session.field('diagnosticDraft');
  const setDiagnosticSignal = session.field('diagnosticSignal');
  const setDtcDraft = session.field('dtcDraft');
  const setDtcError = session.field('dtcError');
  const setProjectName = session.field('projectName');
  const setProjectDirectory = session.field('projectDirectory');
  const setSettingsNotice = session.field('settingsNotice');

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
        { kind: 'action', label: '预览新建工程' },
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
        { kind: 'action', label: '导入项目' },
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
        { kind: 'handoffImport', label: '导入交付包' },
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
        { kind: 'action', label: '预览用户应用初始化' },
        () => call<ApplicationInitializationPreview>('preview_application_initialization'),
        (applicationPreview) => field('applicationPreview')(applicationPreview),
      ),
    );
  }

  async function initializeApplicationPreviewed() {
    const preview = stateRef.current.applicationPreview;
    if (!preview) return;
    await run(
      { kind: 'action', label: '初始化用户应用' },
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
    let applied = false;
    try {
      if (current.creating === 'frame') {
        const values = frameChanges(current.frameInput);
        view = await call<WorkspaceView>('add_frame', values);
        applied = true;
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
        applied = true;
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
        applied = true;
      }
      if (
        JSON.stringify(current.diagnosticDraft) !==
        JSON.stringify(diagnosticFields(current.workspace.diagnostic))
      ) {
        view = await call<WorkspaceView>('configure_diagnostic', {
          settings: diagnosticChanges(current.diagnosticDraft, view),
        });
        patchState({ workspace: view, diagnosticDraft: diagnosticFields(view.diagnostic) });
        applied = true;
      }
      if (
        current.workspace.diagnostic &&
        JSON.stringify(current.dtcDraft) !==
          JSON.stringify(dtcFields(current.workspace.diagnostic.dtc))
      ) {
        view = await call<WorkspaceView>('configure_dtc', dtcChanges(current.dtcDraft, view));
        patchState({ workspace: view, dtcDraft: dtcFields(view.diagnostic?.dtc ?? null) });
        applied = true;
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
        applied = true;
        acceptIntegration(report);
        patchState({ integrationUnapplied: false });
        view = await call<WorkspaceView>('workspace_view');
        patchState({ workspace: view });
      }
      if (generation !== epoch.current) return false;
      if (view !== current.workspace) invalidateAfterEdit();
      await refreshProjection();
    } catch (error) {
      if (generation !== epoch.current) return false;
      if (applied) invalidateAfterEdit();
      await refreshProjection();
      setNotice({
        tone: 'error',
        text: applied
          ? `部分草稿已应用；其余输入保持，请修正后继续：${errorText(error)}`
          : `应用被拒绝，保持当前位置与未应用输入：${errorText(error)}`,
      });
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
          { kind: 'action', label: '接纳扩展定义' },
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
        { kind: 'action', label: '移除扩展定义' },
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
        { kind: 'action', label: '打开成员工程' },
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
        { kind: 'action', label: '预览保存为工程' },
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
      {
        kind: 'action',
        label: current.projectPreviewKind === 'create' ? '创建工程' : '保存为工程',
      },
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
  return {
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
    hasDrafts,
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
  };
}
