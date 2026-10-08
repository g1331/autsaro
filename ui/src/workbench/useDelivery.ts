import { message } from '../i18n';
import type { Text } from '../i18n';
import { isTauri } from '@tauri-apps/api/core';
import type { WorkspaceView } from '../types';
import type { Stage, StageRecord, StageState } from './forms';

export const native = isTauri();

export const steps: { key: Stage; label: Text; number: string }[] = [
  { key: 'save', label: message('workflow.step.save'), number: '01' },
  { key: 'validate', label: message('workflow.step.validate'), number: '02' },
  { key: 'generate', label: message('workflow.step.generate'), number: '03' },
  { key: 'build', label: message('workflow.step.build'), number: '04' },
  { key: 'virtual', label: message('workflow.step.virtual'), number: '05' },
];

export const buildSteps = steps.slice(0, 4);

export const stageDefaults: Record<Stage, StageRecord> = {
  save: { state: 'pending', detail: message('workflow.stage.notSaved') },
  validate: { state: 'pending', detail: message('workflow.stage.notValidated') },
  generate: { state: 'pending', detail: message('workflow.stage.notGenerated') },
  build: { state: 'pending', detail: message('workflow.stage.notBuilt') },
  virtual: { state: 'pending', detail: message('workflow.stage.notRun') },
};

export const stageLabels: Record<StageState, Text> = {
  pending: message('workflow.state.pending'),
  running: message('workflow.state.running'),
  done: message('workflow.state.done'),
  failed: message('workflow.state.failed'),
  stale: message('workflow.state.stale'),
};

export function importedSaveStage(view: WorkspaceView): StageRecord {
  if (
    view.issues.some((issue) => issue.code.startsWith('PDU_') || issue.code === 'DIAG_UNSUPPORTED')
  ) {
    return {
      state: 'failed',
      detail: message('workflow.stage.unsupportedImport'),
    };
  }
  return view.dirty
    ? stageDefaults.save
    : { state: 'done', detail: message('workflow.stage.projectSaved') };
}
