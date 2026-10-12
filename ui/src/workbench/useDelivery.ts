import { message } from '../i18n';
import { isTauri } from '@tauri-apps/api/core';
import type { WorkspaceView } from '../types';
import type { Stage, StageRecord } from './forms';

export const native = isTauri();

export const stageDefaults: Record<Stage, StageRecord> = {
  save: { state: 'pending', detail: message('workflow.stage.notSaved') },
  validate: { state: 'pending', detail: message('workflow.stage.notValidated') },
  generate: { state: 'pending', detail: message('workflow.stage.notGenerated') },
  build: { state: 'pending', detail: message('workflow.stage.notBuilt') },
  virtual: { state: 'pending', detail: message('workflow.stage.notRun') },
};

export function importedSaveStage(view: WorkspaceView): StageRecord {
  return view.dirty
    ? stageDefaults.save
    : { state: 'done', detail: message('workflow.stage.projectSaved') };
}
