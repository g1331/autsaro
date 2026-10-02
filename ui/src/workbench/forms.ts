import type { DiagnosticView, DtcView, Frame, Issue, Signal, WorkspaceView } from '../types';

export type Stage = 'save' | 'validate' | 'generate' | 'build' | 'virtual';

export type StageState = 'pending' | 'running' | 'done' | 'failed' | 'stale';

export type StageRecord = { state: StageState; detail: string };

export type Selection = { kind: 'file' | 'frame' | 'signal'; path: string };

export type FrameFields = {
  name: string;
  id: string;
  dlc: string;
  direction: 'tx' | 'rx';
  periodMs: string;
  timeoutMs: string;
};

export type SignalFields = { name: string; startBit: string; length: string; initialValue: string };

export type FrameChanges = Pick<
  Frame,
  'name' | 'id' | 'dlc' | 'direction' | 'periodMs' | 'timeoutMs'
>;

export type SignalChanges = Pick<Signal, 'name' | 'startBit' | 'length' | 'initialValue'>;

export type Draft =
  | { kind: 'frame'; path: string; fields: FrameFields }
  | { kind: 'signal'; path: string; fields: SignalFields }
  | null;

export type DiagnosticFields = {
  requestId: string;
  responseId: string;
  s3Ms: string;
  nAsMs: string;
  nBsMs: string;
  nCrMs: string;
  did: string;
  signalPaths: string[];
  writeEnabled: boolean;
  resetRoutineId: string;
  securityEnabled: boolean;
};

export type DiagnosticChanges = Pick<
  DiagnosticView,
  | 'requestId'
  | 'responseId'
  | 's3Ms'
  | 'nAsMs'
  | 'nBsMs'
  | 'nCrMs'
  | 'did'
  | 'signalPaths'
  | 'writeEnabled'
  | 'resetRoutineId'
  | 'securityEnabled'
>;

export type DtcFields = { code: string; monitorFramePath: string };

export type Notice = { tone: 'error' | 'info'; text: string } | null;

export type Page = 'editor' | 'diagnostics' | 'build' | 'virtual' | 'integration';

export const frameFields = (frame: Frame): FrameFields => ({
  name: frame.name,
  id: String(frame.id),
  dlc: String(frame.dlc),
  direction: frame.direction,
  periodMs: frame.periodMs === null ? '' : String(frame.periodMs),
  timeoutMs: frame.timeoutMs === null ? '' : String(frame.timeoutMs),
});

export const signalFields = (signal: Signal): SignalFields => ({
  name: signal.name,
  startBit: String(signal.startBit),
  length: String(signal.length),
  initialValue: String(signal.initialValue),
});

export const newFrame: FrameFields = {
  name: '',
  id: '',
  dlc: '8',
  direction: 'tx',
  periodMs: '100',
  timeoutMs: '',
};

export const newSignal: SignalFields = { name: '', startBit: '0', length: '8', initialValue: '0' };

export const newDiagnostic: DiagnosticFields = {
  requestId: '',
  responseId: '',
  s3Ms: '',
  nAsMs: '',
  nBsMs: '',
  nCrMs: '',
  did: '',
  signalPaths: [],
  writeEnabled: false,
  resetRoutineId: '',
  securityEnabled: false,
};

export const diagnosticFields = (diagnostic: DiagnosticView | null): DiagnosticFields =>
  diagnostic
    ? {
        requestId: `0x${diagnostic.requestId.toString(16).toUpperCase()}`,
        responseId: `0x${diagnostic.responseId.toString(16).toUpperCase()}`,
        s3Ms: String(diagnostic.s3Ms),
        nAsMs: String(diagnostic.nAsMs),
        nBsMs: String(diagnostic.nBsMs),
        nCrMs: String(diagnostic.nCrMs),
        did: `0x${diagnostic.did.toString(16).toUpperCase().padStart(4, '0')}`,
        signalPaths: [...diagnostic.signalPaths],
        writeEnabled: diagnostic.writeEnabled,
        resetRoutineId:
          diagnostic.resetRoutineId === null
            ? ''
            : `0x${diagnostic.resetRoutineId.toString(16).toUpperCase().padStart(4, '0')}`,
        securityEnabled: diagnostic.securityEnabled,
      }
    : { ...newDiagnostic, signalPaths: [] };

export const dtcFields = (dtc: DtcView | null): DtcFields =>
  dtc
    ? {
        code: `0x${dtc.code.toString(16).toUpperCase().padStart(6, '0')}`,
        monitorFramePath: dtc.monitorFramePath,
      }
    : { code: '', monitorFramePath: '' };

export function dtcChanges(
  fields: DtcFields,
  view: WorkspaceView,
): { code: number; monitorFramePath: string } {
  const code = canNumber(fields.code, 'DTC 代码', 0xfffffe);
  if (code < 0x100) throw new Error('DTC 代码须为 0x000100–0xFFFFFE 的 24-bit 整数');
  const frame = view.frames.find((item) => item.path === fields.monitorFramePath);
  if (
    !frame ||
    frame.direction !== 'rx' ||
    !frame.timeoutMs ||
    frame.timeoutMs <= 0 ||
    !view.signals.some((signal) => signal.framePath === frame.path)
  ) {
    throw new Error('监测帧须为当前项目中含至少一个信号、接收超时大于 0 的 Rx CAN 帧');
  }
  return { code, monitorFramePath: frame.path };
}

export function canNumber(value: string, label: string, max: number): number {
  const input = value.trim();
  if (!/^(?:0x[0-9a-f]+|\d+)$/i.test(input))
    throw new Error(`${label}须为十进制或 0x 开头的十六进制整数`);
  const parsed = Number(input);
  if (!Number.isSafeInteger(parsed) || parsed < 0 || parsed > max)
    throw new Error(`${label}须为 0–${max} 的整数`);
  return parsed;
}

export function diagnosticChanges(
  fields: DiagnosticFields,
  view: WorkspaceView,
): DiagnosticChanges {
  const requestId = canNumber(fields.requestId, '请求 CAN ID', 2047);
  const responseId = canNumber(fields.responseId, '响应 CAN ID', 2047);
  if (
    requestId === responseId ||
    view.frames.some((frame) => frame.id === requestId || frame.id === responseId)
  ) {
    throw new Error('请求与响应 CAN ID 须不同，且不可与现有 Com 帧 CAN ID 冲突');
  }
  const did = canNumber(fields.did, 'DID', 65535);
  if (did === 0xf186) throw new Error('DID 0xF186 保留给当前会话标识，请选择其他 DID');
  if (
    fields.signalPaths.length < 1 ||
    fields.signalPaths.length > 8 ||
    new Set(fields.signalPaths).size !== fields.signalPaths.length
  ) {
    throw new Error('按顺序选择 1–8 个不同的 32-bit Tx 信号');
  }
  if (
    fields.signalPaths.some(
      (path) =>
        !view.signals.some(
          (signal) =>
            signal.path === path &&
            signal.length === 32 &&
            view.frames.some(
              (frame) => frame.path === signal.framePath && frame.direction === 'tx',
            ),
        ),
    )
  ) {
    throw new Error('所选信号须为当前项目中的 32-bit Tx 信号');
  }
  if (!fields.writeEnabled && fields.resetRoutineId.trim()) {
    throw new Error('启用 0x31/0x01 复位例程前，须先允许 0x2E 写入此 DID');
  }
  const resetRoutineId = fields.resetRoutineId.trim()
    ? canNumber(fields.resetRoutineId, '复位例程 RID', 65535)
    : null;
  if (fields.securityEnabled && !fields.writeEnabled && !view.diagnostic?.dtc) {
    throw new Error('启用 0x27 前，须先启用 DID 写入或配置故障记忆');
  }
  return {
    requestId,
    responseId,
    did,
    s3Ms: intInRange(fields.s3Ms, 'S3 (ms)', 5000, 2147483647),
    nAsMs: intInRange(fields.nAsMs, 'N_As (ms)', 1, 2147483647),
    nBsMs: intInRange(fields.nBsMs, 'N_Bs (ms)', 1, 2147483647),
    nCrMs: intInRange(fields.nCrMs, 'N_Cr (ms)', 1, 2147483647),
    signalPaths: fields.signalPaths,
    writeEnabled: fields.writeEnabled,
    resetRoutineId,
    securityEnabled: fields.securityEnabled,
  };
}

export function labelFromPath(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
}

export function intInRange(value: string, label: string, min: number, max: number): number {
  const parsed = Number(value);
  if (
    !/^\d+$/.test(value.trim()) ||
    !Number.isSafeInteger(parsed) ||
    parsed < min ||
    parsed > max
  ) {
    throw new Error(`${label}须为 ${min}–${max} 的整数`);
  }
  return parsed;
}

export function requiredName(value: string): string {
  const name = value.trim();
  if (!name || !/^[A-Za-z][A-Za-z0-9_]*$/.test(name)) {
    throw new Error('名称须以英文字母开头，仅包含字母、数字和下划线');
  }
  return name;
}

export function frameChanges(fields: FrameFields): FrameChanges {
  const direction = fields.direction;
  const periodMs =
    direction === 'tx' ? intInRange(fields.periodMs, '发送周期 (ms)', 1, 2147483647) : null;
  const timeoutMs =
    direction === 'rx' ? intInRange(fields.timeoutMs, '接收超时 (ms)', 1, 2147483647) : null;
  return {
    name: requiredName(fields.name),
    id: intInRange(fields.id, '标准 CAN ID', 0, 2047),
    dlc: intInRange(fields.dlc, 'DLC', 1, 8),
    direction,
    periodMs,
    timeoutMs,
  };
}

export function signalChanges(fields: SignalFields, frame: Frame): SignalChanges {
  const length = intInRange(fields.length, '信号长度', 1, 32);
  const startBit = intInRange(fields.startBit, '起始位', 0, 63);
  if (startBit + length > frame.dlc * 8) throw new Error('信号位范围超出所属帧 DLC');
  return {
    name: requiredName(fields.name),
    startBit,
    length,
    initialValue: intInRange(fields.initialValue, '初始值', 0, 2 ** length - 1),
  };
}

export function errorText(error: unknown): string {
  if (Array.isArray(error)) {
    return error
      .map((item) =>
        item && typeof item === 'object' && 'message' in item
          ? `${'code' in item ? String(item.code) + ': ' : ''}${String(item.message)}${'remedy' in item ? '\n' + String(item.remedy) : ''}`
          : String(item),
      )
      .join('\n');
  }
  return error instanceof Error ? error.message : String(error);
}

export interface PreviewDelta {
  oldStart: number;
  newStart: number;
  oldText: string;
  newText: string;
}

export function previewDelta(before: string, after: string, xml = true): PreviewDelta {
  // Generated ARXML often puts many XML elements on one physical line.
  // Break only adjacent tags for display; the complete source stays available below.
  const oldLines = (xml ? before.replaceAll('><', '>\n<') : before).split('\n');
  const newLines = (xml ? after.replaceAll('><', '>\n<') : after).split('\n');
  let first = 0;
  while (first < oldLines.length && first < newLines.length && oldLines[first] === newLines[first])
    first++;
  let tail = 0;
  while (
    tail < oldLines.length - first &&
    tail < newLines.length - first &&
    oldLines[oldLines.length - 1 - tail] === newLines[newLines.length - 1 - tail]
  )
    tail++;
  return {
    oldStart: first + 1,
    newStart: first + 1,
    oldText: oldLines.slice(first, oldLines.length - tail).join('\n'),
    newText: newLines.slice(first, newLines.length - tail).join('\n'),
  };
}

export function initialSelection(view: WorkspaceView): Selection | null {
  if (view.frames.length) return { kind: 'frame', path: view.frames[0].path };
  if (view.files.length) return { kind: 'file', path: view.files[0].path };
  return null;
}

export function findSelection(view: WorkspaceView, selection: Selection | null): Selection | null {
  if (selection?.kind === 'frame' && view.frames.some((frame) => frame.path === selection.path))
    return selection;
  if (selection?.kind === 'signal' && view.signals.some((signal) => signal.path === selection.path))
    return selection;
  if (selection?.kind === 'file' && view.files.some((file) => file.path === selection.path))
    return selection;
  return initialSelection(view);
}

export function draftFor(view: WorkspaceView, selection: Selection | null): Draft {
  if (selection?.kind === 'frame') {
    const frame = view.frames.find((item) => item.path === selection.path);
    if (frame) return { kind: 'frame', path: frame.path, fields: frameFields(frame) };
  }
  if (selection?.kind === 'signal') {
    const signal = view.signals.find((item) => item.path === selection.path);
    if (signal) return { kind: 'signal', path: signal.path, fields: signalFields(signal) };
  }
  return null;
}

export function hasUnapplied(view: WorkspaceView | null, draft: Draft): boolean {
  if (!view || !draft) return false;
  const original = draftFor(view, { kind: draft.kind, path: draft.path });
  return !original || JSON.stringify(original.fields) !== JSON.stringify(draft.fields);
}

export function issueTarget(issue: Issue, view: WorkspaceView): Selection | null {
  if (issue.path && view.signals.some((signal) => signal.path === issue.path))
    return { kind: 'signal', path: issue.path };
  if (issue.path && view.frames.some((frame) => frame.path === issue.path))
    return { kind: 'frame', path: issue.path };
  if (issue.file && view.files.some((file) => file.path === issue.file))
    return { kind: 'file', path: issue.file };
  return null;
}
