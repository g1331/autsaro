import { composedMessage, message, translate } from '../i18n';
import type { Text } from '../i18n';
import { open } from '@tauri-apps/plugin-dialog';
import type { IntegrationInspection, WorkspaceView } from '../types';
import { rememberDialogOpener } from './Dialog';
import type { Stage, StageState } from './forms';
import { applicationPeriodDraft, errorText, intInRange } from './forms';
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
  acceptView: (view: WorkspaceView) => void;
  refreshProjection: () => Promise<void>;
  run: <T>(
    operation: WorkbenchOperation,
    job: () => Promise<T>,
    onSuccess: (result: T) => void,
    stage?: Stage,
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
    catalogDecision,
    pendingReplacement,
  } = session;
  const { busy, integrationProcessing, importPaths } = session.state;
  const setNotice = session.field('notice');
  const setImportPaths = session.field('importPaths');
  const setImportPathText = session.field('importPathText');
  const setBusy = session.field('busy');
  const setIntegrationUnapplied = session.field('integrationUnapplied');
  const setIntegrationProcessing = session.field('integrationProcessing');
  const setStages = session.field('stages');
  const setGenerated = session.field('generated');
  const setBuilt = session.field('built');
  const setVirtualResult = session.field('virtualResult');
  const setOperationIssues = session.field('operationIssues');
  const setBuildDirectory = session.field('buildDirectory');
  const setSource = session.field('source');
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

  /** The in-app dialog for a definition catalog; resolves with the user's decision. */
  function confirmCatalog(kind: 'accept' | 'remove', subject: string): Promise<boolean> {
    if (busy || integrationProcessing || catalogDecision.current) return Promise.resolve(false);
    rememberDialogOpener();
    return new Promise((resolve) => {
      catalogDecision.current = resolve;
      patchState({ catalogConfirmation: { kind, subject } });
    });
  }

  function resolveCatalogConfirmation(accepted: boolean) {
    const decide = catalogDecision.current;
    catalogDecision.current = null;
    patchState({ catalogConfirmation: null });
    decide?.(accepted);
  }

  /** Opening lands on configuration; reopening the same project keeps the standard input in view. */
  function applyProject(view: WorkspaceView, reopened = false) {
    const keepIntegration =
      reopened &&
      view.integrationCandidate &&
      stateRef.current.activeDocument.kind === 'integration';
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
      integrationNotice: message('controller.integration.notInspected'),
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
      tabs: view.integrationCandidate
        ? [{ kind: 'configuration' }, { kind: 'integration' }]
        : [{ kind: 'configuration' }],
      activeDocument: { kind: keepIntegration ? 'integration' : 'configuration' },
      preflight: null,
      memberProjectPath: null,
    });
    setStages({ ...stageDefaults, save: importedSaveStage(view) });
    setGenerated(null);
    setBuilt(null);
    setVirtualResult(null);
    setOperationIssues([]);
    setBuildDirectory('');
    acceptView(view);
    void refreshProjection();
  }

  function openProjectEntry(nextSource: WorkbenchState['source']) {
    setSource(nextSource);
    if (stateRef.current.workspace) openDocument({ kind: 'project-entry' });
  }

  async function startProject(nextSource: 'empty' | 'import' = 'empty') {
    if (integrationProcessing) return;
    await replaceProject(message('activity.switchProject'), async () => {
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
        stages: { ...stageDefaults },
        operationIssues: [],
        integrationInspection: null,
        integrationIssues: [],
        integrationIds: {},
        integrationPeriod: '',
        integrationPreview: null,
        integrationPreviewPath: '',
        integrationNotice: message('controller.integration.notInspected'),
        ecuOutputDirectory: '',
        ecuImportDirectory: '',
        buildDirectory: '',
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
      setSource(nextSource);
      setProjectName('');
      setProjectDirectory('');
      setImportPaths([]);
      setImportPathText('');
      setNotice(null);
    });
  }

  function createProject() {
    void replaceProject(message('activity.newProject'), async () => {
      const current = stateRef.current;
      if (!current.projectDirectory || !current.projectName.trim()) {
        setNotice({ tone: 'error', text: message('workflow.project.requireNameDirectory') });
        return;
      }
      await run(
        { kind: 'action', label: message('activity.previewNewProject') },
        () =>
          call<ProjectCreationPreview>('preview_project_creation', {
            directory: current.projectDirectory,
            name: current.projectName.trim(),
            templateId: 'standard-ecu',
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
    void replaceProject(message('activity.importArxml'), () =>
      run(
        { kind: 'action', label: message('activity.importArxml') },
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
    });
  }

  async function confirmHandoffImport() {
    const current = stateRef.current;
    const directory = current.ecuImportDirectory;
    const newWorkspaceDirectory = current.handoffImportDestination;
    if (!directory || !newWorkspaceDirectory) return;
    await replaceProject(message('activity.replaceWithHandoff'), async () => {
      field('handoffImportOpen')(false);
      await run(
        { kind: 'handoffImport', label: message('activity.importHandoff') },
        () => call<WorkspaceView>('open_handoff_project', { directory, newWorkspaceDirectory }),
        (view) => {
          field('handoffImportDestination')('');
          applyProject(view);
        },
      );
    });
  }

  async function previewApplicationInitialization() {
    await guardContext(message('activity.initializeApplication'), () =>
      run(
        { kind: 'action', label: message('activity.previewApplication') },
        () => call<ApplicationInitializationPreview>('preview_application_initialization'),
        (applicationPreview) => field('applicationPreview')(applicationPreview),
      ),
    );
  }

  async function initializeApplicationPreviewed() {
    const preview = stateRef.current.applicationPreview;
    if (!preview) return;
    await run(
      { kind: 'action', label: message('activity.initializeApplication') },
      async () => {
        const outcome = await call<ApplicationInitializationOutcome>(
          'initialize_application_previewed',
          { preview },
        );
        // The member list now names the application; only the workspace view reports that.
        return { outcome, view: await call<WorkspaceView>('workspace_view') };
      },
      ({ outcome, view }) => {
        patchState({
          workspace: view,
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
    await guardContext(message('activity.closeDocument'), async () => {
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
    return current.integrationUnapplied || current.changes.length > 0;
  }

  function discardDrafts() {
    const current = stateRef.current;
    patchState({
      changes: [],
      changePreview: null,
      preparedChangeSet: null,
      applicationPreview: null,
      integrationUnapplied: false,
      integrationIds: current.integrationInspection?.description
        ? Object.fromEntries(
            current.integrationInspection.description.signals.map((signal) => [
              signal.port,
              String(signal.canId),
            ]),
          )
        : {},
      integrationPeriod: applicationPeriodDraft(current.integrationInspection?.description),
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
    setBusy(message('activity.apply'));
    let view = current.workspace;
    let applied = false;
    try {
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
      await guardContext(message('activity.previewSave'), requestSave);
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
    else await guardContext(message('activity.chooseConfiguration'), select);
  }

  async function readSource(sourceId: string) {
    if (!native || running.current) return;
    const generation = epoch.current;
    running.current = true;
    setBusy(message('activity.readSource'));
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
      if (!(await confirmCatalog('accept', catalogPath))) return;
      await guardContext(message('activity.acceptDefinitions'), () =>
        run(
          { kind: 'action', label: message('activity.acceptDefinitions') },
          () => call<ProjectProjection>('import_definition_catalog', { catalogPath }),
          (projection) => {
            patchState({ projection });
            invalidateAfterEdit();
            setSettingsNotice({ tone: 'success', text: message('workflow.definitions.accepted') });
          },
        ),
      );
    } catch (error) {
      setSettingsNotice({
        tone: 'error',
        text: composedMessage('workflow.error.importDefinitions', { error: errorText(error) }),
      });
    }
  }

  async function removeDefinitionCatalog(catalogId: string) {
    if (!(await confirmCatalog('remove', catalogId))) return;
    await guardContext(message('activity.removeDefinitions'), () =>
      run(
        { kind: 'action', label: message('activity.removeDefinitions') },
        () => call<ProjectProjection>('remove_definition_catalog', { catalogId }),
        (projection) => {
          patchState({ projection });
          invalidateAfterEdit();
          setSettingsNotice({ tone: 'success', text: message('workflow.definitions.changed') });
        },
      ),
    );
  }

  async function openMemberProject() {
    await replaceProject(message('activity.openProject'), async () => {
      const path = await open({
        multiple: false,
        filters: [{ name: translate('workflow.dialog.projectFilter'), extensions: ['json'] }],
        title: translate('workflow.dialog.chooseMemberProject'),
      });
      if (typeof path !== 'string') return;
      await run(
        { kind: 'action', label: message('activity.openProject') },
        () => call<WorkspaceView>('open_member_project', { path }),
        (view) => {
          applyProject(view);
          field('memberProjectPath')(path);
        },
      );
    });
  }

  async function previewSaveAs() {
    await guardContext(message('activity.saveAs'), async () => {
      const current = stateRef.current;
      if (!current.projectDirectory) {
        setNotice({ tone: 'error', text: message('workflow.project.requireDirectory') });
        return;
      }
      await run(
        { kind: 'action', label: message('activity.previewSaveAs') },
        () =>
          call<ProjectCreationPreview>('preview_save_as_project', {
            directory: current.projectDirectory,
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
            ? message('activity.createProject')
            : message('activity.saveAs'),
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
        setNotice({
          tone: 'success',
          text: message(
            current.projectPreviewKind === 'create'
              ? 'workflow.project.created'
              : 'workflow.project.savedAs',
            { name: view.name, directory: preview.directory },
          ),
        });
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
    resolveCatalogConfirmation,
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
