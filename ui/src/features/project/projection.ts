import { composedMessage } from '../../i18n';
import type { WorkspaceView } from '../../ipc/types';
import { errorText } from '../../shared/ui/forms';
import type { WorkbenchSession } from '../../app/session';

export function createProjectProjection({ capabilitiesRef, call, patchState }: WorkbenchSession) {
  function acceptView(view: WorkspaceView) {
    patchState({ savePreview: null, generationPreview: null, workspace: view });
  }

  async function refreshProjection() {
    if (!capabilitiesRef.current?.hasWorkspace) return;
    try {
      const projection = await call('project_projection');
      patchState({ projection, memberProjectPath: projection.projectPath });
    } catch (error) {
      patchState({
        notice: {
          tone: 'error',
          text: composedMessage('controller.project.projectionFailed', { error: errorText(error) }),
        },
      });
    }
  }

  return { acceptView, refreshProjection };
}
