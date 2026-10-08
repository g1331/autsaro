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
import type { Selection, Stage, StageState } from './forms';
import type { WorkbenchOperation } from './operation';
import type { DocumentTab } from './projectTypes';
import type { WorkbenchSession } from './session';
import { stageDefaults } from './useDelivery';

interface Dependencies {
  run: <T>(
    operation: WorkbenchOperation,
    job: () => Promise<T>,
    onSuccess: (result: T) => void,
    stage?: Stage,
    allowed?: 'frame' | 'diagnostic' | 'dtc',
  ) => Promise<void>;
  acceptView: (view: WorkspaceView, requested?: Selection | null) => void;
  markStage: (key: Stage, state: StageState, detail: Text) => void;
  continueReplacement: (saved: WorkspaceView, error: Text | null) => Promise<void>;
  chooseDirectory: (onChoose: (path: string) => void) => Promise<void>;
  openDocument: (tab: DocumentTab) => void;
  unapplied: boolean;
  acceptIntegration: (report: IntegrationInspection) => void;
  confirmAction: (message: Text) => Promise<boolean>;
}

export function createDeliveryActions(session: WorkbenchSession, dependencies: Dependencies) {
  const {
    run,
    acceptView,
    markStage,
    continueReplacement,
    chooseDirectory,
    openDocument,
    unapplied,
    acceptIntegration,
    confirmAction,
  } = dependencies;
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
    peerDirectory,
    peerBinaryPath,
  } = session.state;
  const setSavePreview = session.field('savePreview');
  const setPreviewPath = session.field('previewPath');
  const setNotice = session.field('notice');
  const setStages = session.field('stages');
  const setGenerated = session.field('generated');
  const setBuilt = session.field('built');
  const setVirtualResult = session.field('virtualResult');
  const setOperationIssues = session.field('operationIssues');
  const setGenerationKind = session.field('generationKind');
  const setGenerationPreview = session.field('generationPreview');
  const setGenerationPreviewPath = session.field('generationPreviewPath');
  const setHandoffGenerated = session.field('handoffGenerated');
  const setVirtualKind = session.field('virtualKind');

  function saveProject() {
    void run(
      { kind: 'savePreview', label: message('workflow.action.previewSave') },
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
      () => call<WorkspaceView>('validate_project'),
      (view) => {
        acceptView(view);
        setOperationIssues([]);
        const count = view.issues.filter((issue) => issue.severity === 'error').length;
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
        field('toolWindow')('problems');
      },
      'validate',
    );
  }

  function generateProject(handoff = false) {
    if (workspace?.dirty || state.projection?.dirty) {
      setNotice({ tone: 'error', text: message('workflow.error.saveBeforeGenerate') });
      return;
    }
    if (stages.validate.state !== 'done') {
      setNotice({ tone: 'error', text: message('workflow.error.validateBeforeGenerate') });
      return;
    }
    void chooseDirectory((directory) => {
      void run(
        {
          kind: 'action',
          label: handoff
            ? message('workflow.action.previewHandoff')
            : message('workflow.action.previewGenerate'),
        },
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
    markStage('build', 'stale', message('workflow.stage.waitingGeneration'));
    markStage('virtual', 'stale', message('workflow.stage.waitingGeneration'));
    void run(
      { kind: 'action', label: message('workflow.action.generate') },
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
          markStage('generate', 'failed', message('workflow.stage.generationIncomplete'));
          setNotice({ tone: 'error', text: message('workflow.error.generationFailed') });
        } else {
          setGenerated(result);
          setHandoffGenerated(generationKind === 'handoff');
          setBuilt(null);
          setVirtualResult(null);
          markStage('generate', 'done', result.outputDirectory);
          markStage('build', 'pending', message('workflow.stage.generatedNotBuilt'));
          markStage('virtual', 'pending', message('workflow.stage.twoEcusNotRun'));
        }
      },
      'generate',
    );
  }

  function buildProject() {
    if (!generated || !buildDirectory.trim() || stages.generate.state !== 'done') return;
    setBuilt(null);
    setVirtualResult(null);
    markStage('virtual', 'stale', message('workflow.stage.waitingBuild'));
    void run(
      { kind: 'action', label: message('workflow.action.build') },
      () =>
        call<BuildResult>('build_project', {
          outputDirectory: generated.outputDirectory,
          buildDirectory,
        }),
      (result) => {
        field('toolWindow')('build');
        if (!result.binaryPath) {
          markStage('build', 'failed', message('workflow.stage.binaryMissing'));
          setNotice({ tone: 'error', text: message('workflow.error.binaryMissing') });
        } else {
          setBuilt(result);
          setVirtualResult(null);
          markStage('build', 'done', result.binaryPath);
          markStage('virtual', 'pending', message('workflow.stage.twoEcusNotRun'));
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
      { kind: 'action', label: message('workflow.action.runVirtual') },
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
          result.passed
            ? message('workflow.stage.virtualPassed')
            : message('workflow.stage.virtualFailed'),
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
      { kind: 'action', label: message('workflow.action.runDiagnostic') },
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
          result.passed
            ? message('workflow.stage.diagnosticPassed')
            : message('workflow.stage.diagnosticFailed'),
        );
      },
      'virtual',
    );
  }

  function previewIntegrationSave() {
    void run(
      { kind: 'savePreview', label: message('workflow.action.previewIntegrationSave') },
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
        setNotice({
          tone: 'info',
          text: message('workflow.generation.sourcePreview', { count: preview.files.length }),
        });
      },
    );
  }

  async function generateEcu() {
    if (!generationPreview) return;
    if (
      !(await confirmAction(
        message('workflow.confirm.generateFiles', {
          count: generationPreview.files.length,
          directory: generationPreview.outputDirectory,
        }),
      ))
    ) {
      setNotice({ tone: 'info', text: message('workflow.generation.cancelled') });
      return;
    }
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
        setVirtualKind('signal');
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
  };
}
