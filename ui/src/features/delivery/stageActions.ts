import { message } from '../../i18n';
import type { Text } from '../../i18n';
import type { WorkbenchSession } from '../../app/session';
import type { Stage, StageState } from './stages';

export function createStageActions({ field, patchState }: WorkbenchSession) {
  const setStages = field('stages');

  function invalidateAfterEdit() {
    patchState({
      stages: {
        save: { state: 'stale', detail: message('controller.stage.editedSave') },
        validate: { state: 'stale', detail: message('controller.stage.editedValidate') },
        generate: { state: 'stale', detail: message('controller.stage.editedGenerate') },
        build: { state: 'stale', detail: message('controller.stage.editedBuild') },
        virtual: { state: 'stale', detail: message('controller.stage.editedVirtual') },
      },
      generationPreview: null,
      generated: null,
      built: null,
      virtualResult: null,
      operationIssues: [],
      preflight: null,
      notice: null,
    });
  }

  function markStage(key: Stage, state: StageState, detail: Text) {
    setStages((previous) => ({ ...previous, [key]: { state, detail } }));
  }

  return { invalidateAfterEdit, markStage };
}
