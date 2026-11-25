import { composedMessage, isLocalizedText, message, ProductError } from '../i18n';
import type { Text } from '../i18n';
import { native } from '../ipc/native';
import type { PlanDiagnostic } from '../ipc/types';
import { rememberDialogOpener } from '../shared/ui/Dialog';
import { errorText, headline } from '../shared/ui/forms';
import type { Stage, StageState } from '../features/delivery/stages';
import type { WorkbenchOperation } from './operation';
import type { WorkbenchSession } from './session';

export type RunOperation = <T>(
  operation: WorkbenchOperation,
  job: () => Promise<T>,
  onSuccess: (result: T) => void,
  stage?: Stage,
) => Promise<void>;

export function createOperationRunner(
  session: WorkbenchSession,
  refreshProjection: () => Promise<void>,
  markStage: (key: Stage, state: StageState, detail: Text) => void,
): RunOperation {
  const { running, epoch, stateRef, capabilitiesRef, pendingReplacement, field, patchState } =
    session;
  const { workspace } = session.state;
  const setBusy = field('busy');
  const setNotice = field('notice');
  const setSettingsNotice = field('settingsNotice');
  const setIntegrationIssues = field('integrationIssues');
  const setIntegrationNotice = field('integrationNotice');
  const setIntegrationProcessing = field('integrationProcessing');

  function requireReady() {
    if (!native) throw new ProductError(message('controller.error.desktopRequired'));
    if (stateRef.current.changes.length)
      throw new ProductError(message('controller.error.batchDraft'));
  }

  async function run<T>(
    operation: WorkbenchOperation,
    job: () => Promise<T>,
    onSuccess: (result: T) => void,
    stage?: Stage,
  ) {
    const label = operation.label;
    if (running.current) return;
    rememberDialogOpener();
    const generation = epoch.current;
    try {
      requireReady();
      running.current = true;
      setBusy(label);
      setNotice(null);
      if (stage)
        markStage(stage, 'running', composedMessage('controller.operation.running', { label }));
      const result = await job();
      if (generation !== epoch.current) return;
      onSuccess(result);
      if (generation === epoch.current && capabilitiesRef.current?.hasWorkspace)
        await refreshProjection();
    } catch (error) {
      if (generation !== epoch.current) return;
      const text = errorText(error);
      setNotice({
        tone: 'error',
        text: composedMessage('controller.operation.failed', { label, error: headline(error) }),
        detail: text,
      });
      if (operation.kind === 'handoffImport') field('handoffImportOpen')(true);
      if (stateRef.current.settingsOpen) setSettingsNotice({ tone: 'error', text });
      if (
        workspace?.integrationCandidate &&
        Array.isArray(error) &&
        error.length > 0 &&
        error.every(
          (issue: unknown) =>
            typeof issue === 'object' &&
            issue !== null &&
            'category' in issue &&
            typeof issue.category === 'string' &&
            ['input', 'unsupported', 'dependency', 'tool'].includes(issue.category) &&
            'code' in issue &&
            typeof issue.code === 'string' &&
            'file' in issue &&
            (issue.file === null || typeof issue.file === 'string') &&
            'object' in issue &&
            (issue.object === null || typeof issue.object === 'string') &&
            'message' in issue &&
            isLocalizedText(issue.message) &&
            'remedy' in issue &&
            isLocalizedText(issue.remedy),
        )
      ) {
        setIntegrationIssues(error as PlanDiagnostic[]);
        setIntegrationNotice(
          operation.kind === 'integrationEdit'
            ? message('controller.integration.editRejected')
            : composedMessage('controller.operation.failedLabel', { label }),
        );
      }
      if (stage) markStage(stage, 'failed', text);
      if (stateRef.current.savingForReplacement && operation.kind === 'savePreview') {
        pendingReplacement.current = null;
        patchState({ savingForReplacement: false, replacementReady: false });
      }
    } finally {
      if (generation === epoch.current) {
        running.current = false;
        setBusy(null);
        setIntegrationProcessing(false);
      }
    }
  }

  return run;
}
