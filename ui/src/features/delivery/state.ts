import type { Text } from '../../i18n';
import type {
  BuildResult,
  BuildTarget,
  GenerateResult,
  GenerationPreview,
  Issue,
  PreflightReport,
  SavePreview,
  VirtualResult,
} from '../../ipc/types';
import type { Stage, StageRecord } from './stages';
import type { ApplicationInitializationPreview } from '../../ipc/types';
import { stageDefaults } from './stages';

export type DeliveryState = {
  stages: Record<Stage, StageRecord>;
  generated: GenerateResult | null;
  generationPreview: GenerationPreview | null;
  generationPreviewPath: string;
  generationKind: 'project' | 'handoff';
  buildTarget: BuildTarget;
  buildDirectory: string;
  built: BuildResult | null;
  virtualResult: VirtualResult | null;
  operationIssues: Issue[];
  savePreview: SavePreview | null;
  previewPath: string;
  ecuOutputDirectory: string;
  ecuImportDirectory: string;
  handoffImportOpen: boolean;
  handoffImportDestination: string;
  preflight: PreflightReport | null;
  applicationPreview: ApplicationInitializationPreview | null;
  applicationWarnings: Text[];
  applicationRecoveryFiles: string[];
};

export function initialDeliveryState(): DeliveryState {
  return {
    stages: stageDefaults,
    generated: null,
    generationPreview: null,
    generationPreviewPath: '',
    generationKind: 'project',
    buildTarget: 'windows-x64-controlled-v1',
    buildDirectory: '',
    built: null,
    virtualResult: null,
    operationIssues: [],
    savePreview: null,
    previewPath: '',
    ecuOutputDirectory: '',
    ecuImportDirectory: '',
    handoffImportOpen: false,
    handoffImportDestination: '',
    preflight: null,
    applicationPreview: null,
    applicationWarnings: [],
    applicationRecoveryFiles: [],
  };
}
