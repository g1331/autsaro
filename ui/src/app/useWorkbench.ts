import { useMemo } from 'react';
import { composedMessage, message } from '../i18n';
import { native } from '../ipc/native';
import { createBatchEditing } from '../features/editing/batchEditing';
import {
  createIntegrationActions,
  useInspectIntegration,
} from '../features/editing/integrationActions';
import { createDeliveryActions } from '../features/delivery/deliveryActions';
import { deliveryPreviews } from '../features/delivery/previews';
import { createStageActions } from '../features/delivery/stageActions';
import { createProjectActions } from '../features/project/projectActions';
import { createProjectProjection } from '../features/project/projection';
import { createSettingsActions } from '../features/settings/settingsActions';
import { workbenchCommands } from './commands';
import { problemSummary } from './feedback';
import { createOperationRunner } from './operationRunner';
import { useWorkbenchSession } from './useWorkbenchSession';

function useWorkbenchController() {
  const session = useWorkbenchSession();
  const { state, field, call, invalidateOperation, cancelOperation } = session;

  const { acceptView, refreshProjection } = createProjectProjection(session);
  const { invalidateAfterEdit, markStage } = createStageActions(session);
  const run = createOperationRunner(session, refreshProjection, markStage);
  const { acceptIntegration, ...integration } = createIntegrationActions(session, {
    acceptView,
    invalidateAfterEdit,
    markStage,
    run,
  });
  const batch = createBatchEditing(session, { invalidateAfterEdit, acceptIntegration });
  const { applyDrafts, discardDrafts, continueReplacement, applyProject, ...project } =
    createProjectActions(session, {
      invalidateOperation,
      acceptView,
      refreshProjection,
      run,
      invalidateAfterEdit,
      acceptIntegration,
      prepareChanges: batch.prepareChanges,
      previewIntegrationSave: () => delivery.previewIntegrationSave(),
      saveProject: () => delivery.saveProject(),
    });
  const unapplied = state.integrationUnapplied || state.changes.length > 0;
  const delivery = createDeliveryActions(session, {
    run,
    acceptView,
    markStage,
    continueReplacement,
    unapplied,
    acceptIntegration,
    invalidateAfterEdit,
    applyProject,
    guardContext: project.guardContext,
    replaceProject: project.replaceProject,
  });
  const settings = createSettingsActions(session, { invalidateOperation });

  useInspectIntegration(session, integration.inspectIntegration);

  function actionReason(action: string) {
    if (!native) return message('controller.error.desktopRequired');
    if (!state.capabilities) return message('controller.capability.reading');
    if (state.busy) return composedMessage('controller.operation.busy', { label: state.busy });
    const capability =
      state.projection?.capabilities.find((item) => item.action === action) ??
      state.capabilities.actions.find((item) => item.action === action);
    return !capability
      ? message('controller.capability.missingAction')
      : !capability.available
        ? (capability.reason ?? message('controller.capability.unavailable'))
        : '';
  }

  async function verificationOwnedFailure(delayMs = 0) {
    await project.guardContext(message('activity.verificationFailure'), () =>
      run(
        { kind: 'action', label: message('activity.verificationFailure') },
        () => call('verification_owned_failure', { delayMs }),
        (result) => {
          field('toolWindow')('log');
          field('notice')({
            tone: 'error',
            text: composedMessage(
              'controller.verification.result',
              {
                exitCode:
                  result.exitCode == null
                    ? message('controller.verification.noExitCode')
                    : String(result.exitCode),
              },
              {
                scope: result.scope,
                status: result.status,
                reclaimed: result.descendantsReclaimed,
              },
            ),
          });
        },
      ),
    );
  }

  const { projection, workspace, operationIssues, integrationIssues } = state;
  const problems = useMemo(
    () => problemSummary({ projection, workspace, operationIssues, integrationIssues }),
    [projection, workspace, operationIssues, integrationIssues],
  );
  const { savePreview, previewPath, generationPreview, generationPreviewPath } = state;
  const previews = useMemo(
    () => deliveryPreviews({ savePreview, previewPath, generationPreview, generationPreviewPath }),
    [savePreview, previewPath, generationPreview, generationPreviewPath],
  );
  const disabled =
    !native || !state.capabilities || Boolean(state.capabilities.ruleError) || Boolean(state.busy);
  const executionReason = state.capabilities?.nativeExecution
    ? (state.capabilities.toolError ?? '')
    : message('controller.execution.unsupported');

  return {
    ...state,
    ...project,
    ...batch,
    ...integration,
    ...delivery,
    ...settings,
    ...problems,
    ...previews,
    native,
    disabled,
    executionReason,
    unapplied,
    actionReason,
    refreshProjection,
    verificationOwnedFailure,
    cancelOperation,
    applyCurrentDrafts: applyDrafts,
    restoreAllDrafts: discardDrafts,
    setApplicationPreview: field('applicationPreview'),
    setHandoffImportOpen: field('handoffImportOpen'),
    setHandoffImportDestination: field('handoffImportDestination'),
    setProjectPreview: field('projectPreview'),
    setToolWindow: field('toolWindow'),
    setTreeVisible: field('treeVisible'),
    setInspectorVisible: field('inspectorVisible'),
    setInspectorTab: field('inspectorTab'),
    setActiveFieldId: field('activeFieldId'),
    setTreeMode: field('treeMode'),
    setTreeFilter: field('treeFilter'),
    setTreeRevealId: field('treeRevealId'),
    setObjectFilter: field('objectFilter'),
    setAppearanceDraft: field('appearanceDraft'),
    setLanguageDraft: field('languageDraft'),
    setSettingsCategory: field('settingsCategory'),
    setToolErrors: field('toolErrors'),
    setSettingsOpen: field('settingsOpen'),
    setToolDraft: field('toolDraft'),
    setSettingsNotice: field('settingsNotice'),
    setIntegrationIds: field('integrationIds'),
    setIntegrationPeriod: field('integrationPeriod'),
    setIntegrationPreview: field('integrationPreview'),
    setIntegrationPreviewPath: field('integrationPreviewPath'),
    setEcuImportDirectory: field('ecuImportDirectory'),
    setIntegrationUnapplied: field('integrationUnapplied'),
    setProjectName: field('projectName'),
    setProjectDirectory: field('projectDirectory'),
    setImportPaths: field('importPaths'),
    setImportPathText: field('importPathText'),
    setGenerationPreview: field('generationPreview'),
    setGenerationPreviewPath: field('generationPreviewPath'),
    setGenerationKind: field('generationKind'),
    setNotice: field('notice'),
    setPreviewPath: field('previewPath'),
  };
}

export type WorkbenchController = ReturnType<typeof useWorkbenchController>;

export function useWorkbench() {
  const controller = useWorkbenchController();
  return { ...controller, commands: workbenchCommands(controller) };
}

export type Workbench = ReturnType<typeof useWorkbench>;
