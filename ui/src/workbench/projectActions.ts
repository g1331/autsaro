import { composedMessage, message, ProductError, translate } from '../i18n';
import type { Text } from '../i18n';
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
  markStage: (key: Stage, state: StageState, detail: Text) => void;
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
      const path = await open({
        directory: true,
        multiple: false,
        title: translate('workflow.dialog.chooseDirectory'),
      });
      if (typeof path === 'string') onChoose(path);
    } catch (error) {
      setNotice({
        tone: 'error',
        text: composedMessage('workflow.error.chooseDirectory', { error: errorText(error) }),
      });
    }
  }

  async function chooseBinary(onChoose: (path: string) => void) {
    if (!native || busy) return;
    try {
      const path = await open({
        multiple: false,
        title: translate('workflow.dialog.chooseBinary'),
      });
      if (typeof path === 'string') onChoose(path);
    } catch (error) {
      setNotice({
        tone: 'error',
        text: composedMessage('workflow.error.chooseBinary', { error: errorText(error) }),
      });
    }
  }

  async function chooseFiles() {
    if (!native || busy) return;
    try {
      const paths = await open({
        multiple: true,
        filters: [{ name: 'AUTOSAR ARXML', extensions: ['arxml'] }],
        title: translate('workflow.dialog.chooseArxml'),
      });
      if (paths) {
        const selected = Array.isArray(paths) ? paths : [paths];
        setImportPaths(selected);
        setImportPathText(selected.join('\n'));
      }
    } catch (error) {
      setNotice({
        tone: 'error',
        text: composedMessage('workflow.error.chooseFiles', { error: errorText(error) }),
      });
    }
  }

  async function confirmAction(prompt: Text): Promise<boolean> {
    if (busy || integrationProcessing) return false;
    setBusy(message('workflow.action.confirm'));
    try {
      return await requestConfirmation(prompt);
    } catch (error) {
      setNotice({
        tone: 'error',
        text: composedMessage('workflow.error.confirm', { error: errorText(error) }),
      });
      return false;
    } finally {
      setBusy(null);
    }
  }

  async function choose(selectionNext: Selection) {
    await guardContext(message('workflow.action.chooseObject'), () => {
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
    await guardContext(message('workflow.action.createObject'), () => {
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
      integrationNotice: message('workflow.integration.unchecked'),
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
    await replaceProject(message('workflow.action.switchProject'), async () => {
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
        integrationNotice: message('workflow.integration.unchecked'),
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
    void replaceProject(message('workflow.action.newProject'), async () => {
      const current = stateRef.current;
      if (!current.projectDirectory || !current.projectName.trim()) {
        setNotice({ tone: 'error', text: message('workflow.project.requireNameDirectory') });
        return;
      }
      await run(
        { kind: 'action', label: message('workflow.action.previewNewProject') },
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
      setNotice({ tone: 'error', text: message('workflow.project.requireArxml') });
      return;
    }
    void replaceProject(message('workflow.action.importArxml'), () =>
      run(
        { kind: 'action', label: message('workflow.action.importProject') },
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
    await replaceProject(message('workflow.action.replaceWithHandoff'), async () => {
      field('handoffImportOpen')(false);
      await run(
        { kind: 'handoffImport', label: message('workflow.action.importHandoff') },
        () => call<WorkspaceView>('open_handoff_project', { directory, newWorkspaceDirectory }),
        (view) => {
          field('handoffImportDestination')('');
          applyProject(view);
        },
      );
    });
  }

  async function previewApplicationInitialization() {
    await guardContext(message('workflow.action.initializeApplication'), () =>
      run(
        { kind: 'action', label: message('workflow.action.previewApplication') },
        () => call<ApplicationInitializationPreview>('preview_application_initialization'),
        (applicationPreview) => field('applicationPreview')(applicationPreview),
      ),
    );
  }

  async function initializeApplicationPreviewed() {
    const preview = stateRef.current.applicationPreview;
    if (!preview) return;
    await run(
      { kind: 'action', label: message('workflow.action.initializeApplication') },
      () => call<ApplicationInitializationOutcome>('initialize_application_previewed', { preview }),
      (outcome) => {
        patchState({
          applicationPreview: null,
          projection: outcome.projection,
          applicationWarnings: outcome.warnings,
          applicationRecoveryFiles: outcome.retainedRecoveryFiles,
        });
        invalidateAfterEdit();
        if (!outcome.projection.dirty)
          markStage('save', 'done', message('workflow.stage.applicationWritten'));
        setNotice({
          tone: 'info',
          text: message('workflow.application.initialized', {
            warnings: outcome.warnings.length,
            recoveryFiles: outcome.retainedRecoveryFiles.length,
          }),
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
    await guardContext(message('workflow.action.closeDocument'), async () => {
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

  async function guardContext(title: Text, action: () => void | Promise<void>) {
    if (running.current || stateRef.current.guard) return;
    rememberDialogOpener();
    if (!hasDrafts()) {
      await action();
      return;
    }
    pendingGuard.current = action;
    patchState({ guard: { kind: 'context', title } });
  }

  async function replaceProject(title: Text, action: () => void | Promise<void>) {
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
    setBusy(message('workflow.action.applyDrafts'));
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
        if (!owner) throw new ProductError(message('workflow.error.selectSignalFrame'));
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
          if (!owner) throw new ProductError(message('workflow.error.signalFrameMissing'));
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
            applicationPeriodMs: intInRange(
              current.integrationPeriod,
              message('workflow.field.applicationPeriod'),
              1,
              2147483647,
            ),
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
          ? composedMessage('workflow.error.draftsPartiallyApplied', { error: errorText(error) })
          : composedMessage('workflow.error.draftsRejected', { error: errorText(error) }),
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
      await guardContext(message('workflow.action.previewSave'), requestSave);
      return;
    }
    if (stateRef.current.workspace?.integrationCandidate) previewIntegrationSave();
    else saveProject();
  }

  async function continueReplacement(saved: WorkspaceView, error: Text | null) {
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
    else await guardContext(message('workflow.action.chooseConfiguration'), select);
  }

  async function readSource(sourceId: string) {
    if (!native || running.current) return;
    const generation = epoch.current;
    running.current = true;
    setBusy(message('workflow.action.readSource'));
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
        setNotice({
          tone: 'error',
          text: composedMessage('workflow.error.readSource', { error: errorText(error) }),
        });
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
        filters: [{ name: translate('workflow.dialog.catalogFilter'), extensions: ['json'] }],
        title: translate('workflow.dialog.chooseCatalog'),
      });
      if (typeof catalogPath !== 'string') return;
      if (!(await confirmAction(message('workflow.confirm.acceptCatalog', { path: catalogPath }))))
        return;
      await guardContext(message('workflow.action.acceptDefinitions'), () =>
        run(
          { kind: 'action', label: message('workflow.action.acceptDefinitions') },
          () => call<ProjectProjection>('import_definition_catalog', { catalogPath }),
          (projection) => {
            patchState({ projection });
            invalidateAfterEdit();
            setSettingsNotice(message('workflow.definitions.accepted'));
          },
        ),
      );
    } catch (error) {
      setSettingsNotice(
        composedMessage('workflow.error.importDefinitions', { error: errorText(error) }),
      );
    }
  }

  async function removeDefinitionCatalog(catalogId: string) {
    if (!(await confirmAction(message('workflow.confirm.removeCatalog', { catalogId })))) return;
    await guardContext(message('workflow.action.removeDefinitions'), () =>
      run(
        { kind: 'action', label: message('workflow.action.removeDefinitions') },
        () => call<ProjectProjection>('remove_definition_catalog', { catalogId }),
        (projection) => {
          patchState({ projection });
          invalidateAfterEdit();
          setSettingsNotice(message('workflow.definitions.changed'));
        },
      ),
    );
  }

  async function openMemberProject() {
    await replaceProject(message('workflow.action.openMemberProject'), async () => {
      const path = await open({
        multiple: false,
        filters: [{ name: translate('workflow.dialog.projectFilter'), extensions: ['json'] }],
        title: translate('workflow.dialog.chooseMemberProject'),
      });
      if (typeof path !== 'string') return;
      await run(
        { kind: 'action', label: message('workflow.action.openMemberProject') },
        () => call<WorkspaceView>('open_member_project', { path }),
        (view) => {
          applyProject(view);
          field('memberProjectPath')(path);
        },
      );
    });
  }

  async function previewSaveAs() {
    await guardContext(message('workflow.action.saveAs'), async () => {
      const current = stateRef.current;
      if (!current.projectDirectory || !current.projectName.trim()) {
        setNotice({ tone: 'error', text: message('workflow.project.requireNameDirectory') });
        return;
      }
      await run(
        { kind: 'action', label: message('workflow.action.previewSaveAs') },
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
        label:
          current.projectPreviewKind === 'create'
            ? message('workflow.action.createProject')
            : message('workflow.action.saveAs'),
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
