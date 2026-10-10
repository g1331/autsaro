import type { LanguagePreference, LocalizedText } from './i18n';
import type {
  ActionCapability,
  Appearance,
  RuleCoverage,
  RuleSetIdentity,
} from './workbench/projectTypes';

export type BuildTarget = 'windows-x64-controlled-v1' | 'linux-x64-controlled-v1';

export type PreflightReport = {
  status: 'not_run' | 'passed' | 'failed';
  fingerprint: string;
  logs: LocalizedText[];
};
export type OwnedVerificationFailure = {
  exitCode: number | null;
  status: string;
  scope: string;
  stdoutPath: string;
  stderrPath: string;
  descendantsReclaimed: boolean;
  log: string;
};

export interface ExecutionTools {
  compiler: string;
  objdump: string;
  git: string;
  python: string;
}

export interface WorkbenchCapabilities {
  fingerprint: string;
  target: BuildTarget;
  targets: BuildTarget[];
  nativeExecution: boolean;
  hasWorkspace: boolean;
  xsdArchive: string | null;
  modArchive: string | null;
  resourceError: LocalizedText | null;
  executionTools: ExecutionTools | null;
  configuredExecutionTools: ExecutionTools | null;
  toolError: LocalizedText | null;
  environmentOverrides: string[];
  operation: { id: number; stage: LocalizedText } | null;
  ruleSetIdentity: RuleSetIdentity | null;
  ruleError: LocalizedText | null;
  ruleCoverage: RuleCoverage[];
  definitionFingerprint: string | null;
  appearance: Appearance;
  language: LanguagePreference;
  actions: ActionCapability[];
  verificationMode: boolean;
}

export interface WorkbenchReply<T> {
  value: T;
  capabilities: WorkbenchCapabilities;
  inputFingerprint: string;
}

export interface SaveOutcome {
  workspace: WorkspaceView;
  error: LocalizedText | null;
}

export type Issue = {
  severity: 'error' | 'warning' | 'info';
  code: string;
  message: LocalizedText;
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
  nAsMs: number;
  nBsMs: number;
  nCrMs: number;
  did: number;
  signalPaths: string[];
  writeEnabled: boolean;
  resetRoutineId: number | null;
  securityEnabled: boolean;
  dtc: DtcView | null;
};

export type WorkspaceView = {
  integrationCandidate?: boolean;
  name: string;
  files: { path: string; readonly: boolean; retainedCount: number }[];
  frames: Frame[];
  signals: Signal[];
  diagnostic: DiagnosticView | null;
  issues: Issue[];
  dirty: boolean;
};

export type PlanDiagnostic = {
  category: 'input' | 'unsupported' | 'dependency' | 'tool';
  code: string;
  file: string | null;
  object: string | null;
  message: LocalizedText;
  remedy: LocalizedText;
};

export type IntegrationInspection = {
  profile: string;
  diagnostics: PlanDiagnostic[];
  description: null | {
    sources: { logicalPath: string; rawSha256: string; roles: string[] }[];
    component?: { component: string; instance: string; periodMs: number };
    multi?: { components: { component: string; instance: string }[] };
    signals: { port: string; canId: number; receive: boolean; dlc: number }[];
    diagnostic?: { did: number; requestCanId: number; responseCanId: number };
  };
};

export type SavePreview = {
  revision: string;
  files: { path: string; changed: boolean; before: string | null; after: string | null }[];
};

export type GenerateResult = {
  outputDirectory: string;
  previousOutputDirectory: string | null;
  files: string[];
  issues: Issue[];
};

export type GenerationPreview = {
  outputDirectory: string;
  revision: string;
  files: {
    path: string;
    status: string;
    before: string | null;
    after: string | null;
    owner?: string | null;
    producerId?: string | null;
    snapshotOf?: string | null;
  }[];
};

export type BuildResult = { binaryPath: string; log: string };
export type VirtualResult = { passed: boolean; log: LocalizedText; events: LocalizedText[] };
