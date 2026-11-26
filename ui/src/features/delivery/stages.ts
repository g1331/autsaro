import { message } from '../../i18n';
import type { Text } from '../../i18n';
import type { WorkspaceView } from '../../ipc/types';

export type Stage = 'save' | 'validate' | 'generate' | 'build' | 'virtual';

export type StageState = 'pending' | 'running' | 'done' | 'failed' | 'stale';

export type StageRecord = { state: StageState; detail: Text };

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
