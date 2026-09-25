import { useState } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { ArrowRight, Boxes, Cable, CircleAlert, CircleCheck, FileCode2, FileInput, FolderOpen, FolderPlus, Hammer, HardDrive, ListChecks, MonitorPlay, Plus, Save, Waypoints } from 'lucide-react';
import type { BuildResult, DiagnosticView, DtcView, Frame, GenerateResult, Issue, Signal, VirtualResult, WorkspaceView } from './types';

type Stage = 'save' | 'validate' | 'generate' | 'build' | 'virtual';
type StageState = 'pending' | 'running' | 'done' | 'failed' | 'stale';
type StageRecord = { state: StageState; detail: string };
type Selection = { kind: 'file' | 'frame' | 'signal'; path: string };
type FrameFields = { name: string; id: string; dlc: string; direction: 'tx' | 'rx'; periodMs: string; timeoutMs: string };
type SignalFields = { name: string; startBit: string; length: string; initialValue: string };
type FrameChanges = Pick<Frame, 'name' | 'id' | 'dlc' | 'direction' | 'periodMs' | 'timeoutMs'>;
type SignalChanges = Pick<Signal, 'name' | 'startBit' | 'length' | 'initialValue'>;
type Draft = { kind: 'frame'; path: string; fields: FrameFields } | { kind: 'signal'; path: string; fields: SignalFields } | null;
type DiagnosticFields = { requestId: string; responseId: string; s3Ms: string; nBsMs: string; nCrMs: string; did: string; signalPaths: string[]; writeEnabled: boolean };
type DiagnosticChanges = Pick<DiagnosticView, 'requestId' | 'responseId' | 's3Ms' | 'nBsMs' | 'nCrMs' | 'did' | 'signalPaths' | 'writeEnabled'>;
type DtcFields = { code: string; monitorFramePath: string };
type Notice = { tone: 'error' | 'info'; text: string } | null;
type Page = 'editor' | 'diagnostics' | 'build' | 'virtual';

const native = isTauri();
const steps: { key: Stage; label: string; number: string }[] = [
  { key: 'save', label: '保存配置', number: '01' },
  { key: 'validate', label: '校验项目', number: '02' },
  { key: 'generate', label: '生成 C99 工程', number: '03' },
  { key: 'build', label: '构建主机目标', number: '04' },
  { key: 'virtual', label: '主机虚拟闭环', number: '05' },
];
const buildSteps = steps.slice(0, 4);
const stageDefaults: Record<Stage, StageRecord> = {
  save: { state: 'pending', detail: '尚未保存' },
  validate: { state: 'pending', detail: '尚未校验' },
  generate: { state: 'pending', detail: '尚未生成' },
  build: { state: 'pending', detail: '尚未构建' },
  virtual: { state: 'pending', detail: '尚未运行' },
};
const stageLabels: Record<StageState, string> = {
  pending: '待执行', running: '进行中', done: '已完成', failed: '未通过', stale: '已过期',
};
const frameFields = (frame: Frame): FrameFields => ({
  name: frame.name, id: String(frame.id), dlc: String(frame.dlc), direction: frame.direction,
  periodMs: frame.periodMs === null ? '' : String(frame.periodMs),
  timeoutMs: frame.timeoutMs === null ? '' : String(frame.timeoutMs),
});
const signalFields = (signal: Signal): SignalFields => ({
  name: signal.name, startBit: String(signal.startBit), length: String(signal.length), initialValue: String(signal.initialValue),
});
const newFrame: FrameFields = { name: '', id: '', dlc: '8', direction: 'tx', periodMs: '100', timeoutMs: '' };
const newSignal: SignalFields = { name: '', startBit: '0', length: '8', initialValue: '0' };
const newDiagnostic: DiagnosticFields = { requestId: '', responseId: '', s3Ms: '', nBsMs: '', nCrMs: '', did: '', signalPaths: [], writeEnabled: false };
const diagnosticFields = (diagnostic: DiagnosticView | null): DiagnosticFields => diagnostic ? {
  requestId: `0x${diagnostic.requestId.toString(16).toUpperCase()}`,
  responseId: `0x${diagnostic.responseId.toString(16).toUpperCase()}`,
  s3Ms: String(diagnostic.s3Ms), nBsMs: String(diagnostic.nBsMs), nCrMs: String(diagnostic.nCrMs),
  did: `0x${diagnostic.did.toString(16).toUpperCase().padStart(4, '0')}`,
  signalPaths: [...diagnostic.signalPaths], writeEnabled: diagnostic.writeEnabled,
} : { ...newDiagnostic, signalPaths: [] };
const dtcFields = (dtc: DtcView | null): DtcFields => dtc
  ? { code: `0x${dtc.code.toString(16).toUpperCase().padStart(6, '0')}`, monitorFramePath: dtc.monitorFramePath }
  : { code: '', monitorFramePath: '' };

function dtcChanges(fields: DtcFields, view: WorkspaceView): { code: number; monitorFramePath: string } {
  const code = canNumber(fields.code, 'DTC 代码', 0xFFFFFE);
  if (code < 0x100) throw new Error('DTC 代码须为 0x000100–0xFFFFFE 的 24-bit 整数');
  const frame = view.frames.find(item => item.path === fields.monitorFramePath);
  if (!frame || frame.direction !== 'rx' || !frame.timeoutMs || frame.timeoutMs <= 0
    || !view.signals.some(signal => signal.framePath === frame.path)) {
    throw new Error('监测帧须为当前项目中含至少一个信号、接收超时大于 0 的 Rx CAN 帧');
  }
  return { code, monitorFramePath: frame.path };
}

function canNumber(value: string, label: string, max: number): number {
  const input = value.trim();
  if (!/^(?:0x[0-9a-f]+|\d+)$/i.test(input)) throw new Error(`${label}须为十进制或 0x 开头的十六进制整数`);
  const parsed = Number(input);
  if (!Number.isSafeInteger(parsed) || parsed < 0 || parsed > max) throw new Error(`${label}须为 0–${max} 的整数`);
  return parsed;
}
function diagnosticChanges(fields: DiagnosticFields, view: WorkspaceView): DiagnosticChanges {
  const requestId = canNumber(fields.requestId, '请求 CAN ID', 2047);
  const responseId = canNumber(fields.responseId, '响应 CAN ID', 2047);
  if (requestId === responseId || view.frames.some(frame => frame.id === requestId || frame.id === responseId)) {
    throw new Error('请求与响应 CAN ID 须不同，且不可与现有 Com 帧 CAN ID 冲突');
  }
  const did = canNumber(fields.did, 'DID', 65535);
  if (did === 0xF186) throw new Error('DID 0xF186 保留给当前会话标识，请选择其他 DID');
  if (fields.signalPaths.length < 1 || fields.signalPaths.length > 8 || new Set(fields.signalPaths).size !== fields.signalPaths.length) {
    throw new Error('按顺序选择 1–8 个不同的 32-bit Tx 信号');
  }
  if (fields.signalPaths.some(path => !view.signals.some(signal => signal.path === path && signal.length === 32 && view.frames.some(frame => frame.path === signal.framePath && frame.direction === 'tx')))) {
    throw new Error('所选信号须为当前项目中的 32-bit Tx 信号');
  }
  return {
    requestId, responseId, did,
    s3Ms: intInRange(fields.s3Ms, 'S3 (ms)', 5000, 2147483647),
    nBsMs: intInRange(fields.nBsMs, 'N_Bs (ms)', 1, 2147483647),
    nCrMs: intInRange(fields.nCrMs, 'N_Cr (ms)', 1, 2147483647),
    signalPaths: fields.signalPaths, writeEnabled: fields.writeEnabled,
  };
}

function labelFromPath(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
}
function intInRange(value: string, label: string, min: number, max: number): number {
  const parsed = Number(value);
  if (!/^\d+$/.test(value.trim()) || !Number.isSafeInteger(parsed) || parsed < min || parsed > max) {
    throw new Error(`${label}须为 ${min}–${max} 的整数`);
  }
  return parsed;
}
function requiredName(value: string): string {
  const name = value.trim();
  if (!name || !/^[A-Za-z][A-Za-z0-9_]*$/.test(name)) {
    throw new Error('名称须以英文字母开头，仅包含字母、数字和下划线');
  }
  return name;
}
function frameChanges(fields: FrameFields): FrameChanges {
  const direction = fields.direction;
  const periodMs = direction === 'tx' ? intInRange(fields.periodMs, '发送周期 (ms)', 1, 2147483647) : null;
  const timeoutMs = direction === 'rx' ? intInRange(fields.timeoutMs, '接收超时 (ms)', 1, 2147483647) : null;
  return {
    name: requiredName(fields.name), id: intInRange(fields.id, '标准 CAN ID', 0, 2047),
    dlc: intInRange(fields.dlc, 'DLC', 1, 8), direction, periodMs, timeoutMs,
  };
}
function signalChanges(fields: SignalFields, frame: Frame): SignalChanges {
  const length = intInRange(fields.length, '信号长度', 1, 32);
  const startBit = intInRange(fields.startBit, '起始位', 0, 63);
  if (startBit + length > frame.dlc * 8) throw new Error('信号位范围超出所属帧 DLC');
  return {
    name: requiredName(fields.name), startBit, length,
    initialValue: intInRange(fields.initialValue, '初始值', 0, 2 ** length - 1),
  };
}
function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
function initialSelection(view: WorkspaceView): Selection | null {
  if (view.frames.length) return { kind: 'frame', path: view.frames[0].path };
  if (view.files.length) return { kind: 'file', path: view.files[0].path };
  return null;
}
function findSelection(view: WorkspaceView, selection: Selection | null): Selection | null {
  if (selection?.kind === 'frame' && view.frames.some(frame => frame.path === selection.path)) return selection;
  if (selection?.kind === 'signal' && view.signals.some(signal => signal.path === selection.path)) return selection;
  if (selection?.kind === 'file' && view.files.some(file => file.path === selection.path)) return selection;
  return initialSelection(view);
}
function draftFor(view: WorkspaceView, selection: Selection | null): Draft {
  if (selection?.kind === 'frame') {
    const frame = view.frames.find(item => item.path === selection.path);
    if (frame) return { kind: 'frame', path: frame.path, fields: frameFields(frame) };
  }
  if (selection?.kind === 'signal') {
    const signal = view.signals.find(item => item.path === selection.path);
    if (signal) return { kind: 'signal', path: signal.path, fields: signalFields(signal) };
  }
  return null;
}
function hasUnapplied(view: WorkspaceView | null, draft: Draft): boolean {
  if (!view || !draft) return false;
  const original = draftFor(view, { kind: draft.kind, path: draft.path });
  return !original || JSON.stringify(original.fields) !== JSON.stringify(draft.fields);
}

function issueTarget(issue: Issue, view: WorkspaceView): Selection | null {
  if (issue.path && view.signals.some(signal => signal.path === issue.path)) return { kind: 'signal', path: issue.path };
  if (issue.path && view.frames.some(frame => frame.path === issue.path)) return { kind: 'frame', path: issue.path };
  if (issue.file && view.files.some(file => file.path === issue.file)) return { kind: 'file', path: issue.file };
  return null;
}

export default function App() {
  const [workspace, setWorkspace] = useState<WorkspaceView | null>(null);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [draft, setDraft] = useState<Draft>(null);
  const [diagnosticDraft, setDiagnosticDraft] = useState<DiagnosticFields>(() => diagnosticFields(null));
  const [diagnosticSignal, setDiagnosticSignal] = useState('');
  const [diagnosticError, setDiagnosticError] = useState('');
  const [dtcDraft, setDtcDraft] = useState<DtcFields>(() => dtcFields(null));
  const [dtcError, setDtcError] = useState('');
  const [source, setSource] = useState<'empty' | 'import'>('empty');
  const [projectName, setProjectName] = useState('');
  const [projectDirectory, setProjectDirectory] = useState('');
  const [importPaths, setImportPaths] = useState<string[]>([]);
  const [creating, setCreating] = useState<'frame' | 'signal' | null>(null);
  const [frameInput, setFrameInput] = useState<FrameFields>(newFrame);
  const [signalInput, setSignalInput] = useState<SignalFields>(newSignal);
  const [stages, setStages] = useState<Record<Stage, StageRecord>>(stageDefaults);
  const [generated, setGenerated] = useState<GenerateResult | null>(null);
  const [built, setBuilt] = useState<BuildResult | null>(null);
  const [virtualResult, setVirtualResult] = useState<VirtualResult | null>(null);
  const [virtualKind, setVirtualKind] = useState<'signal' | 'diagnostic' | null>(null);
  const [operationIssues, setOperationIssues] = useState<Issue[]>([]);
  const [notice, setNotice] = useState<Notice>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [page, setPage] = useState<Page>('editor');
  const [peerDirectory, setPeerDirectory] = useState('');

  const currentFrame = selection?.kind === 'frame' ? workspace?.frames.find(item => item.path === selection.path) : undefined;
  const currentSignal = selection?.kind === 'signal' ? workspace?.signals.find(item => item.path === selection.path) : undefined;
  const signalFrame = currentSignal ? workspace?.frames.find(item => item.path === currentSignal.framePath) : undefined;
  const currentFile = selection?.kind === 'file' ? workspace?.files.find(item => item.path === selection.path) : undefined;
  const focusedFrame = currentFrame ?? signalFrame;
  const frameUnapplied = hasUnapplied(workspace, draft);
  const diagnosticUnapplied = Boolean(workspace && JSON.stringify(diagnosticDraft) !== JSON.stringify(diagnosticFields(workspace.diagnostic)));
  const dtcUnapplied = Boolean(workspace?.diagnostic && JSON.stringify(dtcDraft) !== JSON.stringify(dtcFields(workspace.diagnostic.dtc)));
  const unapplied = frameUnapplied || diagnosticUnapplied || dtcUnapplied;
  const eligibleSignals = workspace?.signals.filter(signal => signal.length === 32 && workspace.frames.some(frame => frame.path === signal.framePath && frame.direction === 'tx')) ?? [];
  const eligibleMonitorFrames = workspace?.frames.filter(frame => frame.direction === 'rx' && (frame.timeoutMs ?? 0) > 0
    && workspace.signals.some(signal => signal.framePath === frame.path)) ?? [];
  const issues = [...(workspace?.issues ?? []), ...operationIssues];
  const errorCount = issues.filter(issue => issue.severity === 'error').length;

  function acceptView(view: WorkspaceView, requested?: Selection | null) {
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
    setOperationIssues([]);
    setNotice(null);
  }
  function markStage(key: Stage, state: StageState, detail: string) {
    setStages(previous => ({ ...previous, [key]: { state, detail } }));
  }
  function requireReady(allowed?: 'frame' | 'diagnostic' | 'dtc') {
    if (!native) throw new Error('需要桌面运行环境');
    if (frameUnapplied && allowed !== 'frame') throw new Error('检查器中有未应用的更改，请先应用或还原');
    if (diagnosticUnapplied && allowed !== 'diagnostic') throw new Error('DoCAN 配置有未应用的更改，请先应用或还原');
    if (dtcUnapplied && allowed !== 'dtc') throw new Error('故障记忆有未应用的更改，请先应用或还原');
  }
  async function run<T>(label: string, job: () => Promise<T>, onSuccess: (result: T) => void, stage?: Stage, allowed?: 'frame' | 'diagnostic' | 'dtc') {
    if (busy) return;
    try {
      requireReady(allowed);
      setBusy(label);
      setNotice(null);
      if (stage) markStage(stage, 'running', `${label}中…`);
      const result = await job();
      onSuccess(result);
    } catch (error) {
      const text = errorText(error);
      setNotice({ tone: 'error', text: `${label}失败：${text}` });
      if (stage) markStage(stage, 'failed', text);
    } finally {
      setBusy(null);
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
  async function chooseFiles() {
    if (!native || busy) return;
    try {
      const paths = await open({ multiple: true, filters: [{ name: 'AUTOSAR ARXML', extensions: ['arxml'] }], title: '选择项目 ARXML 文件' });
      if (paths) setImportPaths(Array.isArray(paths) ? paths : [paths]);
    } catch (error) {
      setNotice({ tone: 'error', text: `选择文件失败：${errorText(error)}` });
    }
  }
  function confirmDiscard(): boolean {
    return !workspace?.dirty && !unapplied || window.confirm('当前配置或检查器有尚未保存的更改。切换项目会丢失这些更改，确定继续？');
  }
  function choose(selectionNext: Selection) {
    if (frameUnapplied && !window.confirm('检查器中有未应用的更改，确定放弃并切换对象？')) return;
    setSelection(selectionNext);
    if (workspace) setDraft(draftFor(workspace, selectionNext));
    setCreating(null);
    setPage('editor');
  }
  function openCreator(kind: 'frame' | 'signal') {
    if (!native || busy || !workspace) return;
    if (diagnosticUnapplied) { setDiagnosticError('请先应用或还原 DoCAN 配置草稿，再添加帧或信号'); return; }
    if (dtcUnapplied) { setDtcError('请先应用或还原故障记忆草稿，再添加帧或信号'); return; }
    if (frameUnapplied && !window.confirm('检查器中有未应用的更改，确定放弃并创建对象？')) return;
    if (workspace) setDraft(draftFor(workspace, selection));
    setCreating(kind);
    setNotice(null);
    if (kind === 'frame') setFrameInput(newFrame);
    else setSignalInput(newSignal);
  }
  function applyProject(view: WorkspaceView) {
    setStages({ ...stageDefaults, save: view.dirty ? stageDefaults.save : { state: 'done', detail: '项目配置已保存' } });
    setGenerated(null);
    setBuilt(null);
    setVirtualResult(null);
    setOperationIssues([]);
    setPeerDirectory('');
    setCreating(null);
    setPage('editor');
    acceptView(view, initialSelection(view));
    setDiagnosticError('');
  }
  function startProject() {
    if (!confirmDiscard()) return;
    setWorkspace(null);
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
    setNotice(null);
  }
  function createProject() {
    if (!projectDirectory) { setNotice({ tone: 'error', text: '请先选择项目目录' }); return; }
    if (!projectName.trim()) { setNotice({ tone: 'error', text: '请输入项目名称' }); return; }
    void run('创建项目', () => invoke<WorkspaceView>('create_project', { directory: projectDirectory, name: projectName.trim() }), applyProject);
  }
  function importProject() {
    if (!importPaths.length) { setNotice({ tone: 'error', text: '请至少选择一份 ARXML 文件' }); return; }
    void run('导入项目', () => invoke<WorkspaceView>('open_project', { paths: importPaths }), applyProject);
  }
  function addFrame() {
    let values: FrameChanges;
    try { values = frameChanges(frameInput); } catch (error) { setNotice({ tone: 'error', text: errorText(error) }); return; }
    void run('添加帧', () => invoke<WorkspaceView>('add_frame', values), view => {
      invalidateAfterEdit();
      acceptView(view, { kind: 'frame', path: view.frames.find(frame => frame.name === values.name)?.path ?? '' });
      setCreating(null);
    });
  }
  function addSignal() {
    if (!focusedFrame) return;
    let values: SignalChanges;
    try { values = signalChanges(signalInput, focusedFrame); } catch (error) { setNotice({ tone: 'error', text: errorText(error) }); return; }
    const framePath = focusedFrame.path;
    void run('添加信号', () => invoke<WorkspaceView>('add_signal', { framePath, ...values }), view => {
      invalidateAfterEdit();
      acceptView(view, { kind: 'signal', path: view.signals.find(signal => signal.name === values.name && signal.framePath === framePath)?.path ?? '' });
      setCreating(null);
    });
  }
  function updateSelected() {
    if (!draft || !workspace) return;
    try {
      if (draft.kind === 'frame') {
        const changes = frameChanges(draft.fields);
        const oldPath = draft.path;
        void run('修改帧', () => invoke<WorkspaceView>('update_frame', { path: oldPath, changes }), view => {
          invalidateAfterEdit();
          acceptView(view, { kind: 'frame', path: view.frames.find(frame => frame.path === oldPath)?.path ?? view.frames.find(frame => frame.name === changes.name)?.path ?? '' });
        }, undefined, 'frame');
      } else {
        const owner = workspace.frames.find(frame => frame.path === workspace.signals.find(item => item.path === draft.path)?.framePath);
        if (!owner) throw new Error('所属帧不存在，无法修改信号');
        const changes = signalChanges(draft.fields, owner);
        const oldPath = draft.path;
        void run('修改信号', () => invoke<WorkspaceView>('update_signal', { path: oldPath, changes }), view => {
          invalidateAfterEdit();
          acceptView(view, { kind: 'signal', path: view.signals.find(signal => signal.path === oldPath)?.path ?? view.signals.find(signal => signal.name === changes.name && signal.framePath === owner.path)?.path ?? '' });
        }, undefined, 'frame');
      }
    } catch (error) { setNotice({ tone: 'error', text: errorText(error) }); }
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
    void run('配置 DoCAN', () => invoke<WorkspaceView>('configure_diagnostic', values).catch(error => {
      setDiagnosticError(errorText(error));
      throw error;
    }), view => {
      invalidateAfterEdit();
      acceptView(view);
    }, undefined, 'diagnostic');
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
    void run('配置故障记忆', () => invoke<WorkspaceView>('configure_dtc', values).catch(error => {
      setDtcError(errorText(error));
      throw error;
    }), view => {
      invalidateAfterEdit();
      acceptView(view);
    }, undefined, 'dtc');
  }
  function clearDtc() {
    if (!workspace?.diagnostic?.dtc || unapplied) return;
    if (!window.confirm('移除当前故障记忆配置？应用后仍需保存 ARXML 才会写入文件。')) return;
    void run('移除故障记忆', () => invoke<WorkspaceView>('clear_dtc'), view => {
      invalidateAfterEdit();
      acceptView(view);
    });
  }

  function clearDiagnostic() {
    if (!workspace?.diagnostic || diagnosticUnapplied || dtcUnapplied || frameUnapplied) return;
    if (!window.confirm('移除当前工程的诊断配置？应用后仍需保存 ARXML 才会写入文件。')) return;
    void run('移除 DoCAN', () => invoke<WorkspaceView>('clear_diagnostic'), view => {
      invalidateAfterEdit();
      acceptView(view);
    }, undefined, 'diagnostic');
  }
  function saveProject() {
    void run('保存', () => invoke<WorkspaceView>('save_project'), view => {
      acceptView(view);
      if (view.dirty) {
        markStage('save', 'failed', '后端仍报告未保存修改');
        setNotice({ tone: 'error', text: '保存未完成：项目仍标记为未保存' });
      } else markStage('save', 'done', '配置项目已保存');
    }, 'save');
  }
  function validateProject() {
    void run('校验', () => invoke<WorkspaceView>('validate_project'), view => {
      acceptView(view);
      setOperationIssues([]);
      const count = view.issues.filter(issue => issue.severity === 'error').length;
      markStage('validate', count ? 'failed' : 'done', count ? `${count} 个错误阻断生成` : '校验完成；无阻断错误');
      if (count) {
        markStage('generate', 'stale', '当前校验有阻断错误');
        markStage('build', 'stale', '当前校验有阻断错误');
        markStage('virtual', 'stale', '当前校验有阻断错误');
      }
      setPage('diagnostics');
    }, 'validate');
  }
  function generateProject() {
    if (workspace?.dirty) { setNotice({ tone: 'error', text: '请先保存配置，再生成工程' }); return; }
    if (stages.validate.state !== 'done') { setNotice({ tone: 'error', text: '请先完成无阻断错误的校验' }); return; }
    void chooseDirectory(directory => {
      setGenerated(null);
      setBuilt(null);
      setVirtualResult(null);
      markStage('build', 'stale', '等待新生成工程');
      markStage('virtual', 'stale', '等待新生成工程');
      void run('生成', () => invoke<GenerateResult>('generate_project', { outputDirectory: directory }), result => {
        setOperationIssues(result.issues);
        setPage('build');
        if (result.issues.some(issue => issue.severity === 'error') || !result.outputDirectory || !result.files.length) {
          markStage('generate', 'failed', '生成结果有错误或缺少工程文件');
          setNotice({ tone: 'error', text: '生成未通过，请查看诊断；不能视为工程已构建' });
        } else {
          setGenerated(result);
          setBuilt(null);
          setVirtualResult(null);
          markStage('generate', 'done', result.outputDirectory);
          markStage('build', 'pending', '尚未构建生成工程');
          markStage('virtual', 'pending', '尚未运行两个 ECU');
        }
      }, 'generate');
    });
  }
  function buildProject() {
    if (!generated || stages.generate.state !== 'done') return;
    setBuilt(null);
    setVirtualResult(null);
    markStage('virtual', 'stale', '等待本次构建结果');
    void run('构建', () => invoke<BuildResult>('build_project', { outputDirectory: generated.outputDirectory }), result => {
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
    }, 'build');
  }
  function runVirtual() {
    if (!generated || stages.generate.state !== 'done' || stages.build.state !== 'done' || !peerDirectory) return;
    setVirtualResult(null);
    void run('主机虚拟运行', () => invoke<VirtualResult>('run_virtual', {
      firstOutputDirectory: generated.outputDirectory, secondOutputDirectory: peerDirectory,
    }), result => {
      setVirtualKind('signal');
      setVirtualResult(result);
      setPage('virtual');
      markStage('virtual', result.passed ? 'done' : 'failed', result.passed ? '双 ECU 虚拟运行通过' : '双 ECU 虚拟运行未通过');
    }, 'virtual');
  }
  function runDiagnostic() {
    if (!workspace?.diagnostic || !generated || stages.generate.state !== 'done' || stages.build.state !== 'done' || unapplied) return;
    setVirtualResult(null);
    void run('诊断独立测试', () => invoke<VirtualResult>('run_diagnostic', {
      outputDirectory: generated.outputDirectory,
    }), result => {
      setVirtualKind('diagnostic');
      setVirtualResult(result);
      setPage('virtual');
      markStage('virtual', result.passed ? 'done' : 'failed', result.passed ? '诊断独立测试器通过' : '诊断独立测试器未通过');
    }, 'virtual');
  }

  const disabled = !native || Boolean(busy);
  return (
    <div className="app-shell">
      {!workspace ? (
        <main className="source-page">
          <div className="source-layout">
            <section className="source-primary" aria-label="项目来源">
              <div className="source-head">
                <p className="eyebrow">AUTOSAR CLASSIC / R24-11</p>
                <h1>配置项目</h1>
                <p>从空项目开始，或导入同一 ECU 的多份 ARXML。配置始终保存在原始文件集合中。</p>
              </div>
              <div className="source-switch" role="group" aria-label="选择项目来源">
                <button type="button" className={source === 'empty' ? 'selected' : ''} onClick={() => setSource('empty')}><FolderPlus aria-hidden="true" size={17} />新建空项目</button>
                <button type="button" className={source === 'import' ? 'selected' : ''} onClick={() => setSource('import')}><FileInput aria-hidden="true" size={17} />导入 ARXML</button>
              </div>
              <section className="source-body" aria-label={source === 'empty' ? '新建项目' : '导入项目'}>
                {source === 'empty' ? <>
                  <div className="source-fields">
                    <label>项目名称<input value={projectName} onChange={event => setProjectName(event.target.value)} placeholder="例如：Powertrain_ECU" autoComplete="off" /></label>
                    <label>项目目录<div className="path-picker"><input readOnly value={projectDirectory} placeholder="选择保存 ARXML 的目录" aria-label="项目目录" /><button type="button" onClick={() => void chooseDirectory(setProjectDirectory)} disabled={disabled}><FolderOpen aria-hidden="true" size={15} />选择目录</button></div></label>
                    <p className="field-help">生成的 C99 工程稍后选择独立输出目录，不覆盖项目源文件。</p>
                    <button type="button" className="primary-button" onClick={createProject} disabled={disabled}>{busy === '创建项目' ? '创建中…' : '创建并进入工作区'} <ArrowRight aria-hidden="true" size={16} /></button>
                  </div>
                </> : <>
                  <div className="source-fields">
                    <p className="import-copy">选择同一项目的全部 ARXML。保留未支持的内容；无法安全编辑时内核会拒绝操作。</p>
                    <button type="button" className="outline-button" onClick={() => void chooseFiles()} disabled={disabled}><FileInput aria-hidden="true" size={16} />选择多份 .arxml 文件</button>
                    <div className="import-list" aria-live="polite">{importPaths.length ? importPaths.map(path => <div key={path}><FileCode2 aria-hidden="true" size={15} /><span title={path}>{path}</span></div>) : <p>尚未选择文件</p>}</div>
                    <button type="button" className="primary-button" onClick={importProject} disabled={disabled || !importPaths.length}>{busy === '导入项目' ? '导入中…' : `导入 ${importPaths.length} 份文件`} <ArrowRight aria-hidden="true" size={16} /></button>
                  </div>
                </>}
              </section>
            </section>
          </div>
          {!native && <p className="environment-warning" role="status">需要桌面运行环境。浏览器预览不可选择本地文件、保存、生成、构建或运行。</p>}
          {notice && <p className={`notice ${notice.tone}`} role="alert">{notice.text}</p>}
        </main>
      ) : (
        <main className="workbench">
          <aside className="project-rail">
            <div className="workspace-heading"><div><p className="eyebrow">当前项目</p><h1 title={workspace.name}>{workspace.name}</h1><p className={workspace.dirty ? 'dirty-label' : ''}>{workspace.dirty ? '未保存修改' : '配置已保存'}</p></div><button type="button" className="quiet-button" aria-label="切换项目" title="切换项目" onClick={startProject} disabled={Boolean(busy)}><FolderOpen aria-hidden="true" size={16} /></button></div>
          <nav className="project-nav" aria-label="项目工作页">
            <button type="button" className={page === 'editor' ? 'active' : ''} aria-current={page === 'editor' ? 'page' : undefined} onClick={() => setPage('editor')}><Cable aria-hidden="true" size={16} />配置{workspace.dirty && <span className="nav-alert">未保存</span>}</button>
            <button type="button" className={page === 'diagnostics' ? 'active' : ''} aria-current={page === 'diagnostics' ? 'page' : undefined} onClick={() => setPage('diagnostics')}><CircleAlert aria-hidden="true" size={16} />诊断{issues.length > 0 && <span className="nav-count">{issues.length}</span>}</button>
            <button type="button" className={page === 'build' ? 'active' : ''} aria-current={page === 'build' ? 'page' : undefined} onClick={() => setPage('build')}><HardDrive aria-hidden="true" size={16} />生成与构建</button>
            <button type="button" className={page === 'virtual' ? 'active' : ''} aria-current={page === 'virtual' ? 'page' : undefined} onClick={() => setPage('virtual')}><MonitorPlay aria-hidden="true" size={16} />虚拟运行</button>
          </nav>
            {page === 'editor' && <nav className="tree-pane" aria-label="工程树">
              <div className="tree-section-title">项目文件 <span>{workspace.files.length}</span></div>
              <div className="tree-items">{workspace.files.map(file => <button key={file.path} type="button" className={`tree-item ${selection?.kind === 'file' && selection.path === file.path ? 'active' : ''}`} onClick={() => choose({ kind: 'file', path: file.path })} title={file.path}><span className="tree-symbol"><FileCode2 aria-hidden="true" size={15} /></span><span className="tree-name">{labelFromPath(file.path)}</span>{file.readonly && <span className="tree-tail" title="只读">只读</span>}</button>)}</div>
              <div className="tree-section-title with-action">CAN 帧 <span>{workspace.frames.length}</span><button type="button" aria-label="添加 CAN 帧" title="添加 CAN 帧" onClick={() => openCreator('frame')} disabled={disabled}><Plus aria-hidden="true" size={16} /></button></div>
              <div className="tree-items">{workspace.frames.map(frame => <div key={frame.path}><button type="button" className={`tree-item ${selection?.kind === 'frame' && selection.path === frame.path ? 'active' : ''}`} onClick={() => choose({ kind: 'frame', path: frame.path })} title={frame.path}><span className="tree-symbol"><Cable aria-hidden="true" size={15} /></span><span className="tree-name">{frame.name}</span><span className="tree-tail">{frame.direction.toUpperCase()}</span></button>
                {workspace.signals.filter(signal => signal.framePath === frame.path).map(signal => <button key={signal.path} type="button" className={`tree-item nested ${selection?.kind === 'signal' && selection.path === signal.path ? 'active' : ''}`} onClick={() => choose({ kind: 'signal', path: signal.path })} title={signal.path}><span className="tree-symbol"><Waypoints aria-hidden="true" size={14} /></span><span className="tree-name">{signal.name}</span></button>)}</div>)}
                {!workspace.frames.length && <p className="empty-tree">暂无帧。添加标准 CAN 帧以开始配置。</p>}
              </div>{workspace.files.some(file => file.retainedCount > 0) && <div className="tree-footer"><span>保留项只读</span><strong>{workspace.files.reduce((count, file) => count + file.retainedCount, 0)}</strong></div>}
            </nav>}
          </aside>
          <div className="workspace-content">
            <div className={`workspace-grid${page === 'editor' ? '' : ' single-page'}`}>
            <section className="main-pane" aria-label={page === 'editor' ? '配置工作区' : '项目工作页'}>
              {page === 'editor' && <><div className="section-header"><div><p className="eyebrow">CAN COMMUNICATION</p><h2>帧与信号</h2><p>仅支持标准 11-bit CAN、DLC 1–8、原始无符号小端信号。</p></div><div className="section-actions"><button type="button" className="outline-button small" onClick={saveProject} disabled={disabled || unapplied}><Save aria-hidden="true" size={15} />保存 ARXML</button><button type="button" className="outline-button small" onClick={() => openCreator('frame')} disabled={disabled}><Plus aria-hidden="true" size={15} />添加帧</button></div></div>
                <div className="table-wrap"><table><caption>CAN 帧配置</caption><thead><tr><th scope="col">帧名称</th><th scope="col">CAN ID</th><th scope="col">DLC</th><th scope="col">方向</th><th scope="col">周期 / 超时</th><th scope="col">信号</th></tr></thead><tbody>{workspace.frames.map(frame => <tr key={frame.path} className={focusedFrame?.path === frame.path ? 'selected-row' : ''} onClick={() => choose({ kind: 'frame', path: frame.path })}><td><button type="button" className="table-link" onClick={event => { event.stopPropagation(); choose({ kind: 'frame', path: frame.path }); }}>{frame.name}</button></td><td className="mono">0x{frame.id.toString(16).toUpperCase().padStart(3, '0')}</td><td className="mono">{frame.dlc}</td><td><span className={`direction ${frame.direction}`}>{frame.direction.toUpperCase()}</span></td><td className="mono">{frame.direction === 'tx' ? `${frame.periodMs ?? '—'} ms` : `${frame.timeoutMs ?? '—'} ms`}</td><td className="mono">{workspace.signals.filter(signal => signal.framePath === frame.path).length}</td></tr>)}{!workspace.frames.length && <tr><td colSpan={6} className="empty-cell">项目尚无 CAN 帧。使用“添加帧”开始配置。</td></tr>}</tbody></table></div>
                <div className="section-header secondary"><div><p className="eyebrow">FRAME MAPPING</p><h2>{focusedFrame ? `${focusedFrame.name} · 信号` : '全部信号'}</h2><p>{focusedFrame ? `帧路径：${focusedFrame.path}` : '选择一帧可查看信号映射与引用。'}</p></div><button type="button" className="outline-button small" onClick={() => openCreator('signal')} disabled={disabled || !focusedFrame}><Plus aria-hidden="true" size={15} />添加信号</button></div>
                <div className="table-wrap"><table><caption>信号配置</caption><thead><tr><th scope="col">信号名称</th><th scope="col">所属帧</th><th scope="col">起始位</th><th scope="col">长度</th><th scope="col">初始值</th><th scope="col">编码</th></tr></thead><tbody>{workspace.signals.filter(signal => !focusedFrame || signal.framePath === focusedFrame.path).map(signal => <tr key={signal.path} className={currentSignal?.path === signal.path ? 'selected-row' : ''} onClick={() => choose({ kind: 'signal', path: signal.path })}><td><button type="button" className="table-link" onClick={event => { event.stopPropagation(); choose({ kind: 'signal', path: signal.path }); }}>{signal.name}</button></td><td>{workspace.frames.find(frame => frame.path === signal.framePath)?.name ?? signal.framePath}</td><td className="mono">{signal.startBit}</td><td className="mono">{signal.length} bit</td><td className="mono">{signal.initialValue}</td><td>uint / LE</td></tr>)}{!workspace.signals.some(signal => !focusedFrame || signal.framePath === focusedFrame.path) && <tr><td colSpan={6} className="empty-cell">当前范围内暂无信号。</td></tr>}</tbody></table></div>
                <section className="diagnostic-editor" aria-labelledby="diagnostic-editor-title">
                  <div className="section-header secondary"><div><p className="eyebrow">HOST VIRTUAL / DoCAN</p><h2 id="diagnostic-editor-title">诊断通信配置</h2><p>单条 11-bit 物理连接；扩展会话中一个 DID 按顺序读取实时 32-bit Tx 信号。</p></div><span className="diagnostic-state">{workspace.diagnostic ? '已配置' : '未配置'}</span></div>
                  {workspace.diagnostic && <p className="diagnostic-path">配置路径 <span className="mono path-text">{workspace.diagnostic.path}</span></p>}
                  <div className="diagnostic-fields form-fields">
                    <div className="form-pair">
                      <label>请求 CAN ID <small>0–2047 · 十进制或 0x 十六进制</small><input value={diagnosticDraft.requestId} onChange={event => setDiagnosticDraft({ ...diagnosticDraft, requestId: event.target.value })} placeholder="例如 0x700" disabled={disabled} autoComplete="off" /></label>
                      <label>响应 CAN ID <small>不与 Com 帧冲突</small><input value={diagnosticDraft.responseId} onChange={event => setDiagnosticDraft({ ...diagnosticDraft, responseId: event.target.value })} placeholder="例如 0x708" disabled={disabled} autoComplete="off" /></label>
                    </div>
                    <div className="diagnostic-timers">
                      <label>S3 <small>ms · 5000–2147483647</small><input type="number" min="5000" max="2147483647" step="1" value={diagnosticDraft.s3Ms} onChange={event => setDiagnosticDraft({ ...diagnosticDraft, s3Ms: event.target.value })} disabled={disabled} /></label>
                      <label>N_Bs <small>ms · 1–2147483647</small><input type="number" min="1" max="2147483647" step="1" value={diagnosticDraft.nBsMs} onChange={event => setDiagnosticDraft({ ...diagnosticDraft, nBsMs: event.target.value })} disabled={disabled} /></label>
                      <label>N_Cr <small>ms · 1–2147483647</small><input type="number" min="1" max="2147483647" step="1" value={diagnosticDraft.nCrMs} onChange={event => setDiagnosticDraft({ ...diagnosticDraft, nCrMs: event.target.value })} disabled={disabled} /></label>
                    </div>
                    <label>DID <small>0–65535 · 0xF186 保留</small><input value={diagnosticDraft.did} onChange={event => setDiagnosticDraft({ ...diagnosticDraft, did: event.target.value })} placeholder="例如 0xF190" disabled={disabled} autoComplete="off" /></label>
                    <div className="signal-picker">
                      <label htmlFor="diagnostic-signal">DID 信号顺序 <small>1–8 个 32-bit Tx 信号；每项占 4 字节</small></label>
                      <div className="signal-picker-controls"><select id="diagnostic-signal" value={diagnosticSignal} onChange={event => setDiagnosticSignal(event.target.value)} disabled={disabled || diagnosticDraft.signalPaths.length >= 8 || !eligibleSignals.some(signal => !diagnosticDraft.signalPaths.includes(signal.path))}>
                        <option value="">选择 32-bit Tx 信号</option>
                        {eligibleSignals.filter(signal => !diagnosticDraft.signalPaths.includes(signal.path)).map(signal => <option key={signal.path} value={signal.path}>{signal.name} · {workspace.frames.find(frame => frame.path === signal.framePath)?.name} · {signal.path}</option>)}
                      </select><button type="button" className="outline-button small" onClick={() => { if (diagnosticSignal) { setDiagnosticDraft({ ...diagnosticDraft, signalPaths: [...diagnosticDraft.signalPaths, diagnosticSignal] }); setDiagnosticSignal(''); } }} disabled={disabled || !diagnosticSignal || diagnosticDraft.signalPaths.length >= 8}><Plus aria-hidden="true" size={14} />添加</button></div>
                      {!eligibleSignals.length && <p className="field-help">先在 Tx CAN 帧中配置至少一个 32-bit 信号。</p>}
                      <ol className="diagnostic-signal-list">{diagnosticDraft.signalPaths.map((path, index) => {
                        const signal = workspace.signals.find(item => item.path === path);
                        return <li key={path}><span className="signal-order">{String(index + 1).padStart(2, '0')}</span><span className="signal-description"><strong>{signal?.name ?? '信号不可用'}</strong><small className="mono path-text">{path}</small></span><div className="signal-order-actions"><button type="button" aria-label={`上移 ${signal?.name ?? path}`} onClick={() => { const paths = [...diagnosticDraft.signalPaths]; [paths[index - 1], paths[index]] = [paths[index], paths[index - 1]]; setDiagnosticDraft({ ...diagnosticDraft, signalPaths: paths }); }} disabled={disabled || index === 0}>↑</button><button type="button" aria-label={`下移 ${signal?.name ?? path}`} onClick={() => { const paths = [...diagnosticDraft.signalPaths]; [paths[index], paths[index + 1]] = [paths[index + 1], paths[index]]; setDiagnosticDraft({ ...diagnosticDraft, signalPaths: paths }); }} disabled={disabled || index === diagnosticDraft.signalPaths.length - 1}>↓</button><button type="button" aria-label={`移除 ${signal?.name ?? path}`} onClick={() => setDiagnosticDraft({ ...diagnosticDraft, signalPaths: diagnosticDraft.signalPaths.filter(item => item !== path) })} disabled={disabled}>移除</button></div></li>;
                      })}</ol>
                    </div>
                    <label className="diagnostic-write"><input type="checkbox" checked={diagnosticDraft.writeEnabled} onChange={event => setDiagnosticDraft({ ...diagnosticDraft, writeEnabled: event.target.checked })} disabled={disabled} /><span>允许扩展会话写入此 DID (0x2E)<small>仅修改主机虚拟运行的应用状态；不写入 flash/NvM，不提供 0x27 安全解锁。重启后恢复初始值。</small></span></label>
                  </div>
                  {diagnosticError && <p className="diagnostic-error" role="alert">{diagnosticError}</p>}
                  <div className="diagnostic-actions"><button type="button" className="primary-button compact" onClick={configureDiagnostic} disabled={disabled || !diagnosticUnapplied || dtcUnapplied || frameUnapplied}>{workspace.diagnostic ? '应用诊断更改' : '创建诊断配置'}</button><button type="button" className="quiet-button" onClick={() => { setDiagnosticDraft(diagnosticFields(workspace.diagnostic)); setDiagnosticSignal(''); setDiagnosticError(''); }} disabled={disabled || !diagnosticUnapplied}>还原草稿</button>{workspace.diagnostic && <button type="button" className="quiet-button" onClick={clearDiagnostic} disabled={disabled || diagnosticUnapplied || dtcUnapplied || frameUnapplied}>移除诊断配置</button>}{diagnosticUnapplied && <span role="status">未应用的诊断草稿</span>}</div>
                  <section className="dtc-editor" aria-labelledby="dtc-editor-title">
                    <div className="dtc-heading"><div><p className="eyebrow">HOST VIRTUAL / DEM + NVM</p><h3 id="dtc-editor-title">故障记忆 <span className="dtc-optional">可选 · 单个 DTC</span></h3></div><span className="diagnostic-state">{workspace.diagnostic?.dtc ? '已配置' : '未配置'}</span></div>
                    <p className="dtc-copy">有效 Rx 帧首次到达后，若该帧超时，Dem 记录真实故障状态；主机 NvM 文件持久化状态，独立测试器使用隔离存储并重启 ECU 验证。仅支持 0x19/0x02 状态掩码读取与扩展会话 0x14/0xFFFFFF 清除；不代表完整 Dem/NvM、硬件或标准符合性。</p>
                    {!workspace.diagnostic ? <p className="field-help">先应用上方 DoCAN 配置，再添加故障记忆。</p> : <>
                      {workspace.diagnostic.dtc && <p className="diagnostic-path">配置路径 <span className="mono path-text">{workspace.diagnostic.dtc.path}</span></p>}
                      <div className="dtc-fields form-fields">
                        <label>DTC 代码 <small>24-bit · 0x000100–0xFFFFFE · 十进制或 0x 十六进制</small><input value={dtcDraft.code} onChange={event => { setDtcDraft({ ...dtcDraft, code: event.target.value }); setDtcError(''); }} placeholder="例如 0x123456" disabled={disabled} autoComplete="off" aria-invalid={Boolean(dtcError)} aria-describedby={dtcError ? 'dtc-error' : undefined} /></label>
                        <label>监测 Rx CAN 帧 <small>至少一个信号 · 接收超时大于 0</small><select value={dtcDraft.monitorFramePath} onChange={event => { setDtcDraft({ ...dtcDraft, monitorFramePath: event.target.value }); setDtcError(''); }} disabled={disabled || !eligibleMonitorFrames.length} aria-invalid={Boolean(dtcError)} aria-describedby={dtcError ? 'dtc-error' : undefined}>
                          <option value="">选择监测帧</option>
                          {dtcDraft.monitorFramePath && !eligibleMonitorFrames.some(frame => frame.path === dtcDraft.monitorFramePath) && <option value={dtcDraft.monitorFramePath}>原监测帧已不符合条件 · 请重新选择</option>}
                          {eligibleMonitorFrames.map(frame => <option key={frame.path} value={frame.path}>{frame.name} · 0x{frame.id.toString(16).toUpperCase().padStart(3, '0')} · {frame.timeoutMs} ms</option>)}
                        </select></label>
                      </div>
                      {!eligibleMonitorFrames.length && <p className="field-help">先添加一条接收超时大于 0 的 Rx 帧，并在帧中添加至少一个信号。</p>}
                      {dtcError && <p id="dtc-error" className="diagnostic-error" role="alert">{dtcError}</p>}
                      <div className="diagnostic-actions"><button type="button" className="primary-button compact" onClick={configureDtc} disabled={disabled || !dtcUnapplied || diagnosticUnapplied || frameUnapplied}>{workspace.diagnostic.dtc ? '应用故障记忆更改' : '配置故障记忆'}</button><button type="button" className="quiet-button" onClick={() => { setDtcDraft(dtcFields(workspace.diagnostic?.dtc ?? null)); setDtcError(''); }} disabled={disabled || !dtcUnapplied}>还原草稿</button>{workspace.diagnostic.dtc && <button type="button" className="quiet-button" onClick={clearDtc} disabled={disabled || unapplied}>移除故障记忆</button>}{dtcUnapplied && <span role="status">未应用的故障记忆草稿</span>}</div>
                    </>}
                  </section>
                </section>
              </>}
              {page === 'diagnostics' && <div className="workflow-page diagnostic-view">
                <div className="section-header"><div><p className="eyebrow">VALIDATION REPORT</p><h2>诊断</h2><p>按来源文件与配置对象定位问题。校验结果只代表当前项目状态。</p></div><button type="button" className="primary-button compact" onClick={validateProject} disabled={disabled || unapplied}><ListChecks aria-hidden="true" size={15} />运行校验</button></div>
                {unapplied && <div className="page-guidance">配置页有未应用的更改。<button type="button" onClick={() => setPage('editor')}>返回配置</button></div>}
                <div className={`diagnostic-summary${errorCount ? ' has-errors' : ''}`}><strong>{errorCount}</strong> 个错误 <span>·</span> {issues.length - errorCount} 个警告 / 提示</div>
                {issues.length ? <ul className="issue-list">{issues.map((issue, index) => {
                  const target = issueTarget(issue, workspace);
                  return <li key={`${issue.code}-${issue.path ?? issue.file ?? ''}-${index}`} className={issue.severity}><span className="issue-icon" aria-hidden="true">{issue.severity === 'error' ? '!' : issue.severity === 'warning' ? '△' : 'i'}</span><div><div className="issue-meta">{issue.severity.toUpperCase()} <span>/</span> {issue.code}</div><p>{issue.message}</p>{(issue.path || issue.file) && <button type="button" className="issue-location" onClick={() => { if (target) choose(target); }} disabled={!target}>{issue.file && <span>{issue.file}</span>}{issue.path && <span>{issue.path}</span>}</button>}</div></li>;
                })}</ul> : <p className="empty-message">{stages.validate.state === 'done' ? '本次校验完成，未发现诊断。' : '尚未运行校验。'}</p>}
              </div>}
              {page === 'build' && <div className="workflow-page delivery-view">
                <div className="section-header"><div><p className="eyebrow">GENERATION / BUILD</p><h2>生成与构建</h2><p>从已保存、无阻断错误的配置生成独立 C99 工程，再构建主机目标。</p></div></div>
                <ol className="stage-list">{buildSteps.map(step => { const state = unapplied && stages[step.key].state === 'done' ? 'stale' : stages[step.key].state; return <li key={step.key} className={`stage ${state}`}><span className="stage-number">{step.number}</span><div className="stage-copy"><strong>{step.label}</strong><small title={stages[step.key].detail}>{state === 'stale' && unapplied ? '草稿未应用，需重新保存并校验' : stages[step.key].detail}</small></div><span className="stage-pill">{state === 'done' && <CircleCheck aria-hidden="true" size={13} />}{stageLabels[state]}</span></li>; })}</ol>
                {(unapplied || workspace.dirty || stages.validate.state !== 'done') && <div className="page-guidance">生成前须应用更改、保存配置并完成无阻断错误的校验。<button type="button" onClick={() => setPage(unapplied || workspace.dirty ? 'editor' : 'diagnostics')}>前往{unapplied || workspace.dirty ? '配置' : '诊断'}</button></div>}
                <div className="delivery-actions"><button type="button" className="primary-button compact" onClick={generateProject} disabled={disabled || unapplied || workspace.dirty || stages.validate.state !== 'done'}><Boxes aria-hidden="true" size={15} />选择目录并生成工程</button><button type="button" className="outline-button" onClick={buildProject} disabled={disabled || unapplied || stages.generate.state !== 'done'}><Hammer aria-hidden="true" size={15} />构建生成工程</button></div>
                {generated && !unapplied && !workspace.dirty && stages.generate.state === 'done' && <div className="result-section"><h3>生成工程位置</h3><p className="mono path-text">{generated.outputDirectory}</p><details><summary>工程文件 · {generated.files.length}</summary><ul>{generated.files.map((file, index) => <li key={`${file}-${index}`} className="mono">{file}</li>)}</ul></details></div>}
                {built && !unapplied && !workspace.dirty && stages.build.state === 'done' && <div className="result-section"><h3>构建二进制</h3><p className="mono path-text">{built.binaryPath}</p><details><summary>构建日志</summary><pre>{built.log}</pre></details></div>}
              </div>}
              {page === 'virtual' && <div className="workflow-page virtual-view">
                <div className="section-header"><div><p className="eyebrow">HOST VIRTUAL BUS</p><h2>主机虚拟运行</h2><p>双 ECU 信号闭环与独立测试器诊断验证分别运行；结果不代表真实硬件符合性。</p></div></div>
                <div className={`virtual-status stage ${unapplied && stages.virtual.state === 'done' ? 'stale' : stages.virtual.state}`}><span className="stage-number">05</span><div className="stage-copy"><strong>主机虚拟验证</strong><small>{unapplied && stages.virtual.state === 'done' ? '草稿未应用，结果已过期' : stages.virtual.detail}</small></div><span className="stage-pill">{!unapplied && stages.virtual.state === 'done' && <CircleCheck aria-hidden="true" size={13} />}{unapplied && stages.virtual.state === 'done' ? stageLabels.stale : stageLabels[stages.virtual.state]}</span></div>
                {(!built || stages.build.state !== 'done' || unapplied) && <div className="page-guidance">当前配置尚未完成可运行的主机目标构建。<button type="button" onClick={() => setPage(unapplied ? 'editor' : 'build')}>前往{unapplied ? '配置' : '生成与构建'}</button></div>}
                <div className="peer-section"><h3>对端 ECU 工程</h3><p>选择另一份已生成并构建的 ECU 工程目录；当前工程与对端在虚拟总线上运行。</p><div className="path-picker"><input readOnly value={peerDirectory} placeholder="选择对端生成工程目录" aria-label="对端生成工程目录" /><button type="button" onClick={() => void chooseDirectory(setPeerDirectory)} disabled={disabled || stages.build.state !== 'done'}><FolderOpen aria-hidden="true" size={15} />选择对端目录</button></div><button type="button" className="primary-button compact" onClick={runVirtual} disabled={disabled || unapplied || stages.build.state !== 'done' || !peerDirectory}><MonitorPlay aria-hidden="true" size={15} />运行虚拟闭环</button></div>
                {workspace.diagnostic && <div className="peer-section"><h3>诊断独立测试器</h3><p>对当前生成 ECU 注入物理诊断 CAN 帧，核对会话、实时 DID、流控、超时与故障恢复；无需对端 ECU 工程。{workspace.diagnostic.dtc && '配置故障记忆时，还核对有效 Rx 后的超时 DTC、0x19/0x02 查询、扩展会话 0x14/0xFFFFFF 清除及隔离主机存储中的跨进程重启持久化。'}</p><button type="button" className="primary-button compact" onClick={runDiagnostic} disabled={disabled || unapplied || stages.build.state !== 'done'}><MonitorPlay aria-hidden="true" size={15} />验证诊断连接</button></div>}
                {virtualResult && !unapplied && !workspace.dirty && (stages.virtual.state === 'done' || stages.virtual.state === 'failed') && <div className="result-section"><h3>{virtualKind === 'diagnostic' ? '诊断独立测试器' : '双 ECU 信号闭环'} · {virtualResult.passed ? '通过' : '未通过'}</h3><ul className="events-list">{virtualResult.events.map((event, index) => <li key={index} className="mono">{event}</li>)}</ul><details><summary>完整运行日志</summary><pre>{virtualResult.log}</pre></details></div>}
                <div className="hardware-note"><CircleAlert aria-hidden="true" size={16} /><span>真实硬件未验证；主机虚拟运行结果不代表已上板。</span></div>
              </div>}
            </section>
            {page === 'editor' && <aside className="inspector" aria-label="对象检查器"><div className="pane-overline">OBJECT INSPECTOR</div>
              {creating ? <><p className="eyebrow">CREATE / {creating.toUpperCase()}</p><h2>{creating === 'frame' ? '添加 CAN 帧' : '添加信号'}</h2><p className="inspector-intro">{creating === 'frame' ? '标准 11-bit ID，最多 8 字节。' : `所属帧：${focusedFrame?.name ?? '未选择'}`}</p>
                {creating === 'frame' ? <FrameForm fields={frameInput} onChange={setFrameInput} disabled={disabled} /> : <SignalForm fields={signalInput} onChange={setSignalInput} disabled={disabled} />}
                <div className="inspector-actions"><button type="button" className="primary-button compact" onClick={creating === 'frame' ? addFrame : addSignal} disabled={disabled || (creating === 'signal' && !focusedFrame)}>添加{creating === 'frame' ? '帧' : '信号'}</button><button type="button" className="quiet-button" onClick={() => setCreating(null)}>取消</button></div></>
              : currentFile ? <><p className="eyebrow">SOURCE FILE</p><h2 title={currentFile.path}>{labelFromPath(currentFile.path)}</h2><p className="inspector-intro">源文件属于当前项目配置集合。</p><dl className="property-list"><div><dt>绝对路径</dt><dd className="mono path-text">{currentFile.path}</dd></div><div><dt>访问</dt><dd>{currentFile.readonly ? '只读' : '可写'}</dd></div><div><dt>保留项</dt><dd>{currentFile.retainedCount}</dd></div></dl><div className="inspector-block"><h3>保留策略</h3><p>未支持内容保持原文件语义；编辑关联对象时由内核判断引用安全。不提供保留项的直接编辑。</p></div></>
              : currentFrame && draft?.kind === 'frame' ? <><p className="eyebrow">CAN FRAME / {currentFrame.direction.toUpperCase()}</p><h2>{currentFrame.name}</h2><p className="inspector-intro mono path-text">{currentFrame.path}</p><FrameForm fields={draft.fields} onChange={fields => setDraft({ ...draft, fields })} disabled={disabled} /><div className="inspector-actions"><button type="button" className="primary-button compact" onClick={updateSelected} disabled={disabled || !unapplied}>应用更改</button><button type="button" className="quiet-button" onClick={() => setDraft(draftFor(workspace, selection))} disabled={!unapplied}>还原</button></div><ReferenceView frame={currentFrame} signals={workspace.signals.filter(signal => signal.framePath === currentFrame.path)} /></>
              : currentSignal && draft?.kind === 'signal' ? <><p className="eyebrow">CAN SIGNAL / UINT LE</p><h2>{currentSignal.name}</h2><p className="inspector-intro mono path-text">{currentSignal.path}</p><SignalForm fields={draft.fields} onChange={fields => setDraft({ ...draft, fields })} disabled={disabled} /><div className="inspector-actions"><button type="button" className="primary-button compact" onClick={updateSelected} disabled={disabled || !unapplied}>应用更改</button><button type="button" className="quiet-button" onClick={() => setDraft(draftFor(workspace, selection))} disabled={!unapplied}>还原</button></div>{signalFrame && <ReferenceView frame={signalFrame} signal={currentSignal} />}</>
              : <div className="inspector-empty"><strong>选择对象</strong><p>在工程树或表格中选择文件、帧或信号，以检查属性及真实引用。</p></div>}
              {selection && issues.some(issue => issue.path === selection.path || issue.file === selection.path) && <div className="inspector-issues"><h3>相关诊断</h3>{issues.filter(issue => issue.path === selection.path || issue.file === selection.path).map((issue, index) => <p key={`${issue.code}-${index}`} className={issue.severity}>{issue.code} · {issue.message}</p>)}</div>}
            </aside>}
          </div>
          {!native && <p className="environment-warning" role="status">需要桌面运行环境。当前仅能查看界面，不能执行文件或内核操作。</p>}
          {notice && <p className={`notice ${notice.tone}`} role="alert">{notice.text}</p>}
          </div>
        </main>
      )}
    </div>
  );
}

function FrameForm({ fields, onChange, disabled }: { fields: FrameFields; onChange: (fields: FrameFields) => void; disabled: boolean }) {
  return <div className="form-fields"><label>帧名称<input value={fields.name} onChange={event => onChange({ ...fields, name: event.target.value })} disabled={disabled} autoComplete="off" /></label>
    <div className="form-pair"><label>标准 CAN ID <small>0–2047 · 十进制</small><input type="number" min="0" max="2047" step="1" value={fields.id} onChange={event => onChange({ ...fields, id: event.target.value })} disabled={disabled} /></label><label>DLC <small>字节</small><input type="number" min="1" max="8" step="1" value={fields.dlc} onChange={event => onChange({ ...fields, dlc: event.target.value })} disabled={disabled} /></label></div>
    <label>方向<select value={fields.direction} onChange={event => onChange({ ...fields, direction: event.target.value as 'tx' | 'rx', periodMs: event.target.value === 'tx' ? fields.periodMs || '100' : '', timeoutMs: event.target.value === 'rx' ? fields.timeoutMs || '500' : '' })} disabled={disabled}><option value="tx">TX · 周期发送</option><option value="rx">RX · 接收超时</option></select></label>
    {fields.direction === 'tx' ? <label>发送周期 <small>ms</small><input type="number" min="1" step="1" value={fields.periodMs} onChange={event => onChange({ ...fields, periodMs: event.target.value })} disabled={disabled} /></label> : <label>接收超时 <small>ms</small><input type="number" min="1" step="1" value={fields.timeoutMs} onChange={event => onChange({ ...fields, timeoutMs: event.target.value })} disabled={disabled} /></label>}
  </div>;
}
function SignalForm({ fields, onChange, disabled }: { fields: SignalFields; onChange: (fields: SignalFields) => void; disabled: boolean }) {
  return <div className="form-fields"><label>信号名称<input value={fields.name} onChange={event => onChange({ ...fields, name: event.target.value })} disabled={disabled} autoComplete="off" /></label><div className="form-pair"><label>起始位 <small>0–63</small><input type="number" min="0" max="63" step="1" value={fields.startBit} onChange={event => onChange({ ...fields, startBit: event.target.value })} disabled={disabled} /></label><label>长度 <small>1–32 bit</small><input type="number" min="1" max="32" step="1" value={fields.length} onChange={event => onChange({ ...fields, length: event.target.value })} disabled={disabled} /></label></div><label>初始值 <small>原始无符号值</small><input type="number" min="0" step="1" value={fields.initialValue} onChange={event => onChange({ ...fields, initialValue: event.target.value })} disabled={disabled} /></label><p className="field-help">按 little-endian 位序写入所属 CAN 帧；位范围不可超过帧 DLC。</p></div>;
}
function ReferenceView({ frame, signal, signals }: { frame: Frame; signal?: Signal; signals?: Signal[] }) {
  return <div className="reference-view"><h3>引用与依赖</h3><p>以下关系来自当前配置模型，不推断未显示的跨模块引用。</p><div className="reference-chain">{signal && <><div><small>信号</small><strong>{signal.path}</strong></div><span aria-hidden="true">↓</span></>}<div><small>所属 CAN 帧</small><strong>{frame.path}</strong></div><span aria-hidden="true">↓</span><div><small>总线标识</small><strong>标准 11-bit · 0x{frame.id.toString(16).toUpperCase().padStart(3, '0')} · {frame.direction.toUpperCase()}</strong></div></div>{signals && <p className="reference-note">关联信号：{signals.length ? signals.map(item => item.name).join('、') : '无'}</p>}</div>;
}
