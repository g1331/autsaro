import { composedMessage, message } from '../i18n';
import type { Text } from '../i18n';
import type {
  BuildResult,
  GenerateResult,
  GenerationPreview,
  IntegrationInspection,
  PreflightReport,
  SaveOutcome,
  SavePreview,
  VirtualResult,
  WorkspaceView,
} from '../types';
import type { Stage, StageState } from './forms';
import { distinct } from './forms';
import type { ProjectProjection } from './projectTypes';
import type { WorkbenchOperation } from './operation';
import type { WorkbenchSession } from './session';
import { stageDefaults } from './useDelivery';

interface Dependencies {
  run: <T>(
    operation: WorkbenchOperation,
    job: () => Promise<T>,
    onSuccess: (result: T) => void,
    stage?: Stage,
  ) => Promise<void>;
  acceptView: (view: WorkspaceView) => void;
  markStage: (key: Stage, state: StageState, detail: Text) => void;
  continueReplacement: (saved: WorkspaceView, error: Text | null) => Promise<void>;
  unapplied: boolean;
  acceptIntegration: (report: IntegrationInspection) => void;
}

export function createDeliveryActions(session: WorkbenchSession, dependencies: Dependencies) {
  const { run, acceptView, markStage, continueReplacement, unapplied, acceptIntegration } =
    dependencies;
  const { call, stateRef, field, state, patchState } = session;
  const {
    savePreview,
    workspace,
    stages,
    generationPreview,
    generationKind,
    generated,
    buildDirectory,
    built,
  } = session.state;
  const setSavePreview = session.field('savePreview');
  const setPreviewPath = session.field('previewPath');
  const setNotice = session.field('notice');
  const setStages = session.field('stages');
  const setGenerated = session.field('generated');
  const setBuilt = session.field('built');
  const setVirtualResult = session.field('virtualResult');
  const setOperationIssues = session.field('operationIssues');
  const setGenerationPreview = session.field('generationPreview');
  const setGenerationPreviewPath = session.field('generationPreviewPath');

  function saveProject() {
    void run(
      { kind: 'savePreview', label: message('workflow.action.previewSave') },
      () => call<SavePreview>('preview_save_project'),
      (preview) => {
        if (!preview.files.some((file) => file.changed) && !stateRef.current.savingForReplacement) {
          setNotice({ tone: 'info', text: message('workflow.save.noChanges') });
          return;
        }
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
      { kind: 'action', label: message('workflow.action.save') },
      () => call<SaveOutcome>('save_project', { revision: savePreview.revision }),
      (result) => {
        acceptView(result.workspace);
        if (result.error) {
          markStage('save', 'failed', result.error);
          setNotice({ tone: 'error', text: result.error });
        } else if (result.workspace.dirty) {
          markStage('save', 'failed', message('workflow.stage.backendUnsaved'));
          setNotice({ tone: 'error', text: message('workflow.error.saveIncomplete') });
        } else {
          setStages({
            ...stageDefaults,
            save: { state: 'done', detail: message('workflow.stage.configurationSaved') },
          });
          setNotice({
            tone: 'success',
            text: message('workflow.save.saved', {
              count: savePreview.files.filter((file) => file.changed).length,
            }),
          });
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
      { kind: 'action', label: message('workflow.action.validate') },
      async () => {
        const view = await call<WorkspaceView>('validate_project');
        // Automatic diagnostics are only in the projection; count both sources.
        const projection = await call<ProjectProjection>('project_projection');
        return { view, projection };
      },
      ({ view, projection }) => {
        acceptView(view);
        patchState({ projection });
        setOperationIssues([]);
        const found = [...distinct(projection.diagnostics), ...distinct(view.issues)];
        const count = found.filter((issue) => issue.severity === 'error').length;
        const warnings = found.filter((issue) => issue.severity === 'warning').length;
        markStage(
          'validate',
          count ? 'failed' : 'done',
          count
            ? message('workflow.stage.blockingErrors', { count })
            : message('workflow.stage.validationDone'),
        );
        if (count) {
          markStage('generate', 'stale', message('workflow.stage.validationBlocked'));
          markStage('build', 'stale', message('workflow.stage.validationBlocked'));
          markStage('virtual', 'stale', message('workflow.stage.validationBlocked'));
        }
        setNotice({
          tone: count ? 'error' : warnings ? 'warning' : 'success',
          text: message('workflow.validation.summary', { errors: count, warnings }),
        });
        if (count || warnings) field('toolWindow')('problems');
      },
      'validate',
    );
  }

  function previewIntegrationSave() {
    void run(
      { kind: 'savePreview', label: message('workflow.action.previewIntegrationSave') },
      () => call<SavePreview>('preview_integration_save'),
      (preview) => {
        if (!preview.files.some((file) => file.changed) && !stateRef.current.savingForReplacement) {
          patchState({ integrationIssues: [] });
          setNotice({ tone: 'info', text: message('workflow.save.noChanges') });
          return;
        }
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
      { kind: 'action', label: message('workflow.action.saveIntegration') },
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
              detail:
                result.error ??
                (result.workspace.dirty
                  ? message('workflow.stage.projectUnsaved')
                  : message('workflow.stage.integrationSaved')),
            },
          },
          integrationNotice: result.error ?? message('workflow.integration.savedNotGenerated'),
        });
        if (result.error) setNotice({ tone: 'error', text: result.error });
        else if (!result.workspace.dirty)
          setNotice({
            tone: 'success',
            text: message('workflow.save.saved', {
              count: preview.files.filter((file) => file.changed).length,
            }),
          });
        void continueReplacement(result.workspace, result.error);
      },
      'save',
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
      { kind: 'action', label: message('workflow.action.previewEcu') },
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
        markStage('validate', 'done', message('workflow.stage.integrationValidated'));
      },
    );
  }

  function generateEcu() {
    // The preview dialog is the only confirmation; it names the files and directory.
    if (!generationPreview) return;
    void run(
      { kind: 'action', label: message('workflow.action.generateEcu') },
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
        markStage('build', 'pending', message('workflow.stage.currentNotBuilt'));
        markStage('virtual', 'pending', message('workflow.stage.currentNotVerified'));
      },
      'generate',
    );
  }

  function preflightEcu() {
    if (workspace?.dirty || state.projection?.dirty || unapplied) return;
    void run(
      { kind: 'action', label: message('workflow.action.preflight') },
      () => call<PreflightReport>('preflight_ecu', { handoff: generationKind === 'handoff' }),
      (report) => {
        field('preflight')(report);
        setNotice({
          tone: report.status === 'failed' ? 'error' : 'info',
          text: composedMessage(
            'workflow.preflight.result',
            {
              status: message(
                report.status === 'not_run'
                  ? 'workflow.preflight.notRun'
                  : report.status === 'passed'
                    ? 'workflow.preflight.passed'
                    : 'workflow.preflight.failed',
              ),
              logs: report.logs,
            },
            { fingerprint: report.fingerprint },
          ),
        });
      },
    );
  }

  function buildEcu() {
    if (!generated || stages.generate.state !== 'done' || !buildDirectory.trim()) return;
    void run(
      { kind: 'action', label: message('workflow.action.buildEcu') },
      () => {
        setBuilt(null);
        setVirtualResult(null);
        markStage('virtual', 'stale', message('workflow.stage.rebuilding'));
        return call<BuildResult>('build_ecu', {
          outputDirectory: generated.outputDirectory,
          buildDirectory,
        });
      },
      (value) => {
        setBuilt(value);
        setVirtualResult(null);
        markStage('build', 'done', value.binaryPath);
        markStage('virtual', 'pending', message('workflow.stage.buildNotVerified'));
      },
      'build',
    );
  }

  function verifyEcu() {
    if (!generated || !built || stages.build.state !== 'done') return;
    void run(
      { kind: 'action', label: message('workflow.action.verifyEcu') },
      () => {
        setVirtualResult(null);
        return call<VirtualResult>('verify_ecu', { outputDirectory: generated.outputDirectory });
      },
      (value) => {
        setVirtualResult(value);
        markStage(
          'virtual',
          value.passed ? 'done' : 'failed',
          value.passed
            ? message('workflow.stage.ecuVerified')
            : message('workflow.stage.ecuFailed'),
        );
      },
      'virtual',
    );
  }
  return {
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
  };
}
