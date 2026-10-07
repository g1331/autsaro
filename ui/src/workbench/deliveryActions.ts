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
  markStage: (key: Stage, state: StageState, detail: string) => void;
  continueReplacement: (saved: WorkspaceView, error: string | null) => Promise<void>;
  chooseDirectory: (onChoose: (path: string) => void) => Promise<void>;
  openDocument: (tab: DocumentTab) => void;
  unapplied: boolean;
  acceptIntegration: (report: IntegrationInspection) => void;
  confirmAction: (message: string) => Promise<boolean>;
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
      { kind: 'savePreview', label: '预览保存' },
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
      { kind: 'action', label: '保存' },
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
      { kind: 'action', label: '校验' },
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
        { kind: 'action', label: handoff ? '预览可重建交付包' : '预览生成' },
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
      { kind: 'action', label: '生成' },
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
      { kind: 'action', label: '构建' },
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
      { kind: 'action', label: '主机虚拟运行' },
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
      { kind: 'action', label: '诊断独立测试' },
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

  function previewIntegrationSave() {
    void run(
      { kind: 'savePreview', label: '预览标准输入保存' },
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
      { kind: 'action', label: '保存标准输入' },
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
      { kind: 'action', label: '预览 ECU 交付' },
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
      { kind: 'action', label: '生成 ECU' },
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
      { kind: 'action', label: '原生编译预检' },
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
      { kind: 'action', label: '构建 ECU' },
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
      { kind: 'action', label: '验证 ECU 主机行为' },
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
