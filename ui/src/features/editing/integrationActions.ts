import { useEffect, useEffectEvent } from 'react';
import { message } from '../../i18n';
import type { Text } from '../../i18n';
import { native } from '../../ipc/native';
import type { IntegrationInspection, WorkspaceView } from '../../ipc/types';
import type { RunOperation } from '../../app/operationRunner';
import type { WorkbenchSession } from '../../app/session';
import type { Stage, StageState } from '../delivery/stages';
import { errorText, intInRange } from '../../shared/ui/forms';
import { applicationPeriodDraft, applicationPeriodMs } from './applicationPeriod';

interface Dependencies {
  run: RunOperation;
  markStage: (key: Stage, state: StageState, detail: Text) => void;
  acceptView: (view: WorkspaceView) => void;
  invalidateAfterEdit: () => void;
}

export function createIntegrationActions(session: WorkbenchSession, dependencies: Dependencies) {
  const { state, patchState, call, field, epoch } = session;
  const { run, markStage, acceptView, invalidateAfterEdit } = dependencies;
  const setNotice = field('notice');
  const setIntegrationNotice = field('integrationNotice');

  function acceptIntegration(report: IntegrationInspection) {
    patchState({
      integrationInspection: report,
      integrationIssues: report.diagnostics,
      integrationPreview: null,
      integrationUnapplied: false,
      integrationIds: report.description
        ? Object.fromEntries(
            report.description.signals.map((signal) => [signal.port, String(signal.canId)]),
          )
        : {},
      integrationPeriod: applicationPeriodDraft(report.description),
      integrationProcessing: false,
    });
  }

  function restoreIntegrationDraft() {
    if (state.integrationInspection) acceptIntegration(state.integrationInspection);
  }

  function inspectIntegration() {
    void run(
      { kind: 'action', label: message('activity.validate') },
      () => call('inspect_integration'),
      (report) => {
        acceptIntegration(report);
        setIntegrationNotice(
          message(
            report.description
              ? 'controller.integration.validated'
              : 'controller.integration.failed',
          ),
        );
        // Standard inputs are validated by this inspection; every surface reads this stage.
        markStage(
          'validate',
          report.description ? 'done' : 'failed',
          report.description
            ? message('workflow.stage.integrationValidated')
            : message('workflow.stage.blockingErrors', { count: report.diagnostics.length }),
        );
      },
    );
  }

  function applyIntegration() {
    if (applicationPeriodMs(state.integrationInspection?.description) === null) {
      setIntegrationNotice(message('shell.integration.parametersUnavailable'));
      return;
    }
    const ids: Record<string, number> = {};
    try {
      for (const [path, value] of Object.entries(state.integrationIds)) {
        ids[path] = intInRange(value, 'CAN ID', 0, 0x1fffffff);
      }
      const period = intInRange(
        state.integrationPeriod,
        message('controller.field.applicationPeriod'),
        1,
        2147483647,
      );
      void run(
        { kind: 'integrationEdit', label: message('activity.apply') },
        async () => {
          const generation = epoch.current;
          const report = await call('edit_integration', {
            changes: { canIds: ids, applicationPeriodMs: period },
          });
          if (generation !== epoch.current) return;
          patchState({ integrationSavePending: true });
          invalidateAfterEdit();
          acceptIntegration(report);
          setIntegrationNotice(message('controller.integration.modified'));
          return call('workspace_view');
        },
        (view) => {
          if (!view) return;
          acceptView(view);
          patchState({ integrationSavePending: false });
        },
      );
    } catch (error) {
      setNotice({ tone: 'error', text: errorText(error) });
    }
  }

  return { acceptIntegration, inspectIntegration, applyIntegration, restoreIntegrationDraft };
}

export function useInspectIntegration(
  { state, running }: WorkbenchSession,
  inspectIntegration: () => void,
) {
  const { workspace } = state;
  const inspectCurrentIntegration = useEffectEvent(() => inspectIntegration());
  useEffect(() => {
    if (
      !native ||
      !workspace ||
      state.integrationInspection ||
      state.integrationIssues.length ||
      !state.capabilities
    )
      return;
    if (!workspace.integrationCandidate && state.activeDocument.kind !== 'integration') return;
    if (!running.current) inspectCurrentIntegration();
  }, [
    workspace,
    state.activeDocument.kind,
    state.integrationInspection,
    state.integrationIssues.length,
    state.capabilities,
    running,
  ]);
}
