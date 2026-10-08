import type { Text } from '../i18n';
import { composedMessage, isMessage, message, ProductError } from '../i18n';
import type { DiagnosticView, DtcView, Frame, Issue, Signal, WorkspaceView } from '../types';

export type Stage = 'save' | 'validate' | 'generate' | 'build' | 'virtual';

export type StageState = 'pending' | 'running' | 'done' | 'failed' | 'stale';

export type StageRecord = { state: StageState; detail: Text };

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

export type Notice = { tone: 'error' | 'info'; text: Text } | null;


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
  const code = canNumber(fields.code, message('controller.field.dtcCode'), 0xfffffe);
  if (code < 0x100) throw new ProductError(message('controller.error.dtcRange'));
  const frame = view.frames.find((item) => item.path === fields.monitorFramePath);
  if (
    !frame ||
    frame.direction !== 'rx' ||
    !frame.timeoutMs ||
    frame.timeoutMs <= 0 ||
    !view.signals.some((signal) => signal.framePath === frame.path)
  ) {
    throw new ProductError(message('controller.error.monitorFrame'));
  }
  return { code, monitorFramePath: frame.path };
}

export function canNumber(value: string, label: Text, max: number): number {
  const input = value.trim();
  if (!/^(?:0x[0-9a-f]+|\d+)$/i.test(input))
    throw new ProductError(composedMessage('controller.error.numberFormat', { label }));
  const parsed = Number(input);
  if (!Number.isSafeInteger(parsed) || parsed < 0 || parsed > max)
    throw new ProductError(
      composedMessage('controller.error.integerRange', { label }, { min: 0, max }),
    );
  return parsed;
}

export function diagnosticChanges(
  fields: DiagnosticFields,
  view: WorkspaceView,
): DiagnosticChanges {
  const requestId = canNumber(fields.requestId, message('controller.field.requestId'), 2047);
  const responseId = canNumber(fields.responseId, message('controller.field.responseId'), 2047);
  if (
    requestId === responseId ||
    view.frames.some((frame) => frame.id === requestId || frame.id === responseId)
  ) {
    throw new ProductError(message('controller.error.canIdConflict'));
  }
  const did = canNumber(fields.did, 'DID', 65535);
  if (did === 0xf186) throw new ProductError(message('controller.error.reservedDid'));
  if (
    fields.signalPaths.length < 1 ||
    fields.signalPaths.length > 8 ||
    new Set(fields.signalPaths).size !== fields.signalPaths.length
  ) {
    throw new ProductError(message('controller.error.signalCount'));
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
    throw new ProductError(message('controller.error.txSignals'));
  }
  if (!fields.writeEnabled && fields.resetRoutineId.trim()) {
    throw new ProductError(message('controller.error.routineWrite'));
  }
  const resetRoutineId = fields.resetRoutineId.trim()
    ? canNumber(fields.resetRoutineId, message('controller.field.rid'), 65535)
    : null;
  if (fields.securityEnabled && !fields.writeEnabled && !view.diagnostic?.dtc) {
    throw new ProductError(message('controller.error.securityRequiresWrite'));
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

export function intInRange(value: string, label: Text, min: number, max: number): number {
  const parsed = Number(value);
  if (
    !/^\d+$/.test(value.trim()) ||
    !Number.isSafeInteger(parsed) ||
    parsed < min ||
    parsed > max
  ) {
    throw new ProductError(
      composedMessage('controller.error.integerRange', { label }, { min, max }),
    );
  }
  return parsed;
}

export function requiredName(value: string): string {
  const name = value.trim();
  if (!name || !/^[A-Za-z][A-Za-z0-9_]*$/.test(name)) {
    throw new ProductError(message('controller.error.name'));
  }
  return name;
}

export function frameChanges(fields: FrameFields): FrameChanges {
  const direction = fields.direction;
  const periodMs =
    direction === 'tx'
      ? intInRange(fields.periodMs, message('controller.field.period'), 1, 2147483647)
      : null;
  const timeoutMs =
    direction === 'rx'
      ? intInRange(fields.timeoutMs, message('controller.field.timeout'), 1, 2147483647)
      : null;
  return {
    name: requiredName(fields.name),
    id: intInRange(fields.id, message('controller.field.canId'), 0, 2047),
    dlc: intInRange(fields.dlc, 'DLC', 1, 8),
    direction,
    periodMs,
    timeoutMs,
  };
}

export function signalChanges(fields: SignalFields, frame: Frame): SignalChanges {
  const length = intInRange(fields.length, message('controller.field.signalLength'), 1, 32);
  const startBit = intInRange(fields.startBit, message('controller.field.startBit'), 0, 63);
  if (startBit + length > frame.dlc * 8)
    throw new ProductError(message('controller.error.signalDlc'));
  return {
    name: requiredName(fields.name),
    startBit,
    length,
    initialValue: intInRange(
      fields.initialValue,
      message('controller.field.initialValue'),
      0,
      2 ** length - 1,
    ),
  };
}

export function errorText(error: unknown): Text {
  if (error instanceof ProductError) return error.text;
  if (isMessage(error)) return error;
  if (Array.isArray(error)) return error.map(errorText);
  if (error && typeof error === 'object' && 'message' in error && !(error instanceof Error)) {
    const diagnostic = error as { message: unknown; code?: unknown; remedy?: unknown };
    return composedMessage(
      'controller.error.diagnostic',
      {
        detail: errorText(diagnostic.message),
        remedy: diagnostic.remedy == null ? '' : errorText(diagnostic.remedy),
      },
      { code: diagnostic.code == null ? '' : String(diagnostic.code) },
    );
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
