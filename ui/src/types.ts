export type Issue = {
  severity: 'error' | 'warning' | 'info';
  code: string;
  message: string;
  path?: string;
  file?: string;
};

export type Frame = {
  path: string;
  name: string;
  id: number;
  dlc: number;
  direction: 'tx' | 'rx';
  periodMs: number | null;
  timeoutMs: number | null;
};

export type Signal = {
  path: string;
  name: string;
  framePath: string;
  startBit: number;
  length: number;
  initialValue: number;
};

export type DtcView = {
  path: string;
  code: number;
  monitorFramePath: string;
};

export type DiagnosticView = {
  path: string;
  requestId: number;
  responseId: number;
  s3Ms: number;
  nBsMs: number;
  nCrMs: number;
  did: number;
  signalPaths: string[];
  writeEnabled: boolean;
  resetRoutineId: number | null;
  dtc: DtcView | null;
};

export type WorkspaceView = {
  name: string;
  files: { path: string; readonly: boolean; retainedCount: number }[];
  frames: Frame[];
  signals: Signal[];
  diagnostic: DiagnosticView | null;
  issues: Issue[];
  dirty: boolean;
};

export type GenerateResult = {
  outputDirectory: string;
  previousOutputDirectory: string | null;
  files: string[];
  issues: Issue[];
};

export type BuildResult = { binaryPath: string; log: string };
export type VirtualResult = { passed: boolean; log: string; events: string[] };
