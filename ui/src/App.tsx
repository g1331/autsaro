import { useState } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { ArrowRight, Boxes, Cable, CircleAlert, CircleCheck, FileCode2, FileInput, FolderOpen, FolderPlus, Hammer, HardDrive, ListChecks, MonitorPlay, Plus, Save, Waypoints } from 'lucide-react';
import type { BuildResult, Frame, GenerateResult, Issue, Signal, VirtualResult, WorkspaceView } from './types';

type Stage = 'save' | 'validate' | 'generate' | 'build' | 'virtual';
type StageState = 'pending' | 'running' | 'done' | 'failed' | 'stale';
type StageRecord = { state: StageState; detail: string };
type Selection = { kind: 'file' | 'frame' | 'signal'; path: string };
type FrameFields = { name: string; id: string; dlc: string; direction: 'tx' | 'rx'; periodMs: string; timeoutMs: string };
type SignalFields = { name: string; startBit: string; length: string; initialValue: string };
type FrameChanges = Pick<Frame, 'name' | 'id' | 'dlc' | 'direction' | 'periodMs' | 'timeoutMs'>;
type SignalChanges = Pick<Signal, 'name' | 'startBit' | 'length' | 'initialValue'>;
type Draft = { kind: 'frame'; path: string; fields: FrameFields } | { kind: 'signal'; path: string; fields: SignalFields } | null;
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
  const unapplied = hasUnapplied(workspace, draft);
  const issues = [...(workspace?.issues ?? []), ...operationIssues];
  const errorCount = issues.filter(issue => issue.severity === 'error').length;

  function acceptView(view: WorkspaceView, requested?: Selection | null) {
    const next = findSelection(view, requested === undefined ? selection : requested);
    setWorkspace(view);
    setSelection(next);
    setDraft(draftFor(view, next));
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
  function requireReady(allowUnapplied = false) {
    if (!native) throw new Error('需要桌面运行环境');
    if (unapplied && !allowUnapplied) throw new Error('检查器中有未应用的更改，请先应用或还原');
  }
  async function run<T>(label: string, job: () => Promise<T>, onSuccess: (result: T) => void, stage?: Stage, allowUnapplied = false) {
    if (busy) return;
    try {
      requireReady(allowUnapplied);
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
    if (unapplied && !window.confirm('检查器中有未应用的更改，确定放弃并切换对象？')) return;
    setSelection(selectionNext);
    if (workspace) setDraft(draftFor(workspace, selectionNext));
    setCreating(null);
    setPage('editor');
  }
  function openCreator(kind: 'frame' | 'signal') {
    if (!native || busy || !workspace) return;
    if (unapplied && !window.confirm('检查器中有未应用的更改，确定放弃并创建对象？')) return;
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
  }
  function startProject() {
    if (!confirmDiscard()) return;
    setWorkspace(null);
    setSelection(null);
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
        }, undefined, true);
      } else {
        const owner = workspace.frames.find(frame => frame.path === workspace.signals.find(item => item.path === draft.path)?.framePath);
        if (!owner) throw new Error('所属帧不存在，无法修改信号');
        const changes = signalChanges(draft.fields, owner);
        const oldPath = draft.path;
        void run('修改信号', () => invoke<WorkspaceView>('update_signal', { path: oldPath, changes }), view => {
          invalidateAfterEdit();
          acceptView(view, { kind: 'signal', path: view.signals.find(signal => signal.path === oldPath)?.path ?? view.signals.find(signal => signal.name === changes.name && signal.framePath === owner.path)?.path ?? '' });
        }, undefined, true);
      }
    } catch (error) { setNotice({ tone: 'error', text: errorText(error) }); }
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
      setVirtualResult(result);
      setPage('virtual');
      markStage('virtual', result.passed ? 'done' : 'failed', result.passed ? '双 ECU 虚拟运行通过' : '双 ECU 虚拟运行未通过');
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
              </>}
              {page === 'diagnostics' && <div className="workflow-page diagnostic-view">
                <div className="section-header"><div><p className="eyebrow">VALIDATION REPORT</p><h2>诊断</h2><p>按来源文件与配置对象定位问题。校验结果只代表当前项目状态。</p></div><button type="button" className="primary-button compact" onClick={validateProject} disabled={disabled || unapplied}><ListChecks aria-hidden="true" size={15} />运行校验</button></div>
                {unapplied && <div className="page-guidance">检查器中有未应用的更改。<button type="button" onClick={() => setPage('editor')}>返回配置</button></div>}
                <div className={`diagnostic-summary${errorCount ? ' has-errors' : ''}`}><strong>{errorCount}</strong> 个错误 <span>·</span> {issues.length - errorCount} 个警告 / 提示</div>
                {issues.length ? <ul className="issue-list">{issues.map((issue, index) => {
                  const target = issueTarget(issue, workspace);
                  return <li key={`${issue.code}-${issue.path ?? issue.file ?? ''}-${index}`} className={issue.severity}><span className="issue-icon" aria-hidden="true">{issue.severity === 'error' ? '!' : issue.severity === 'warning' ? '△' : 'i'}</span><div><div className="issue-meta">{issue.severity.toUpperCase()} <span>/</span> {issue.code}</div><p>{issue.message}</p>{(issue.path || issue.file) && <button type="button" className="issue-location" onClick={() => { if (target) choose(target); }} disabled={!target}>{issue.file && <span>{issue.file}</span>}{issue.path && <span>{issue.path}</span>}</button>}</div></li>;
                })}</ul> : <p className="empty-message">{stages.validate.state === 'done' ? '本次校验完成，未发现诊断。' : '尚未运行校验。'}</p>}
              </div>}
              {page === 'build' && <div className="workflow-page delivery-view">
                <div className="section-header"><div><p className="eyebrow">GENERATION / BUILD</p><h2>生成与构建</h2><p>从已保存、无阻断错误的配置生成独立 C99 工程，再构建主机目标。</p></div></div>
                <ol className="stage-list">{buildSteps.map(step => <li key={step.key} className={`stage ${stages[step.key].state}`}><span className="stage-number">{step.number}</span><div className="stage-copy"><strong>{step.label}</strong><small title={stages[step.key].detail}>{stages[step.key].detail}</small></div><span className="stage-pill">{stages[step.key].state === 'done' && <CircleCheck aria-hidden="true" size={13} />}{stageLabels[stages[step.key].state]}</span></li>)}</ol>
                {(unapplied || workspace.dirty || stages.validate.state !== 'done') && <div className="page-guidance">生成前须应用更改、保存配置并完成无阻断错误的校验。<button type="button" onClick={() => setPage(unapplied || workspace.dirty ? 'editor' : 'diagnostics')}>前往{unapplied || workspace.dirty ? '配置' : '诊断'}</button></div>}
                <div className="delivery-actions"><button type="button" className="primary-button compact" onClick={generateProject} disabled={disabled || unapplied || workspace.dirty || stages.validate.state !== 'done'}><Boxes aria-hidden="true" size={15} />选择目录并生成工程</button><button type="button" className="outline-button" onClick={buildProject} disabled={disabled || unapplied || stages.generate.state !== 'done'}><Hammer aria-hidden="true" size={15} />构建生成工程</button></div>
                {generated && <div className="result-section"><h3>生成工程位置</h3><p className="mono path-text">{generated.outputDirectory}</p><details><summary>工程文件 · {generated.files.length}</summary><ul>{generated.files.map((file, index) => <li key={`${file}-${index}`} className="mono">{file}</li>)}</ul></details></div>}
                {built && <div className="result-section"><h3>构建二进制</h3><p className="mono path-text">{built.binaryPath}</p><details><summary>构建日志</summary><pre>{built.log}</pre></details></div>}
              </div>}
              {page === 'virtual' && <div className="workflow-page virtual-view">
                <div className="section-header"><div><p className="eyebrow">HOST VIRTUAL BUS</p><h2>双 ECU 虚拟运行</h2><p>两个独立 C99 ECU 进程经虚拟 CAN 总线交换信号，并检查超时与故障路径。</p></div></div>
                <div className={`virtual-status stage ${stages.virtual.state}`}><span className="stage-number">05</span><div className="stage-copy"><strong>主机虚拟闭环</strong><small>{stages.virtual.detail}</small></div><span className="stage-pill">{stages.virtual.state === 'done' && <CircleCheck aria-hidden="true" size={13} />}{stageLabels[stages.virtual.state]}</span></div>
                {(!built || stages.build.state !== 'done' || unapplied) && <div className="page-guidance">当前配置尚未完成可运行的主机目标构建。<button type="button" onClick={() => setPage(unapplied ? 'editor' : 'build')}>前往{unapplied ? '配置' : '生成与构建'}</button></div>}
                <div className="peer-section"><h3>对端 ECU 工程</h3><p>选择另一份已生成并构建的 ECU 工程目录；当前工程与对端在虚拟总线上运行。</p><div className="path-picker"><input readOnly value={peerDirectory} placeholder="选择对端生成工程目录" aria-label="对端生成工程目录" /><button type="button" onClick={() => void chooseDirectory(setPeerDirectory)} disabled={disabled || stages.build.state !== 'done'}><FolderOpen aria-hidden="true" size={15} />选择对端目录</button></div><button type="button" className="primary-button compact" onClick={runVirtual} disabled={disabled || unapplied || stages.build.state !== 'done' || !peerDirectory}><MonitorPlay aria-hidden="true" size={15} />运行虚拟闭环</button></div>
                {virtualResult && <div className="result-section"><h3>运行结果 · {virtualResult.passed ? '通过' : '未通过'}</h3><ul className="events-list">{virtualResult.events.map((event, index) => <li key={index} className="mono">{event}</li>)}</ul><details><summary>完整运行日志</summary><pre>{virtualResult.log}</pre></details></div>}
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
