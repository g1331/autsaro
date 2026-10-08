import { composedMessage, message, ProductError } from '../i18n';
import type { IntegrationInspection, WorkspaceView } from '../types';
import {
  diagnosticFields,
  draftFor,
  dtcFields,
  errorText,
  findSelection,
  hasUnapplied,
} from './forms';
import type { ChangeOutcome, ChangePreview, ChangeSet, ConfigurationChange } from './projectTypes';
import type { WorkbenchSession } from './session';

interface Dependencies {
  invalidateAfterEdit: () => void;
  acceptIntegration: (report: IntegrationInspection) => void;
}

export function createBatchEditing(session: WorkbenchSession, dependencies: Dependencies) {
  const { invalidateAfterEdit, acceptIntegration } = dependencies;
  const { stateRef, running, capabilitiesRef, epoch, call, patchState, changeConfirmation } =
    session;
  const setBusy = session.field('busy');
  const setNotice = session.field('notice');
  const setIntegrationNotice = session.field('integrationNotice');

  function stageChange(change: ConfigurationChange) {
    const changes = stateRef.current.changes;
    const index = changes.findIndex((item) => item.changeId === change.changeId);
    replaceDrafts(
      index < 0
        ? [...changes, change]
        : changes.map((item, position) => (position === index ? change : item)),
    );
  }

  function replaceDrafts(changes: ConfigurationChange[]) {
    patchState({ changes, changePreview: null, preparedChangeSet: null });
    changeConfirmation.current?.(false);
    changeConfirmation.current = null;
  }

  function discardChanges(changeId?: string) {
    replaceDrafts(
      changeId === undefined
        ? []
        : stateRef.current.changes.filter((item) => item.changeId !== changeId),
    );
  }

  function cancelChangePreview() {
    patchState({ changePreview: null, preparedChangeSet: null });
    changeConfirmation.current?.(false);
    changeConfirmation.current = null;
  }

  async function prepareChanges() {
    const current = stateRef.current;
    if (!current.projection || !current.changes.length || running.current) return;
    const changeSet: ChangeSet = {
      workspaceEpoch: current.projection.workspaceEpoch,
      inputFingerprint: capabilitiesRef.current!.fingerprint,
      definitionFingerprint: current.projection.definitionFingerprint,
      changes: current.changes,
    };
    running.current = true;
    setBusy(message('workflow.action.previewBatch'));
    const generation = epoch.current;
    try {
      const preview = await call<ChangePreview>('prepare_configuration_change', { changeSet });
      if (generation !== epoch.current) return;
      if (stateRef.current.changes !== current.changes)
        throw new ProductError(message('workflow.error.draftChanged'));
      patchState({ changePreview: preview, preparedChangeSet: changeSet });
    } catch (error) {
      if (generation !== epoch.current) return;
      patchState({ changePreview: null, preparedChangeSet: null });
      setNotice({
        tone: 'error',
        text: composedMessage('workflow.error.batchPreview', { error: errorText(error) }),
      });
    } finally {
      if (generation === epoch.current) {
        running.current = false;
        setBusy(null);
      }
    }
  }

  async function applyChanges(): Promise<boolean> {
    const before = stateRef.current;
    const { changePreview, preparedChangeSet } = before;
    if (!changePreview || !preparedChangeSet || running.current) return false;
    running.current = true;
    setBusy(message('workflow.action.applyBatch'));
    const generation = epoch.current;
    const fingerprintBefore = capabilitiesRef.current?.fingerprint;
    try {
      const result = await call<ChangeOutcome>('apply_configuration_change', {
        changeSet: preparedChangeSet,
        changeRevision: changePreview.changeRevision,
      });
      if (generation !== epoch.current) return false;
      if (fingerprintBefore !== capabilitiesRef.current?.fingerprint) invalidateAfterEdit();
      const objectIds = new Set(result.projection.objects.map((object) => object.objectId));
      const selectedIds = stateRef.current.objectSelection.filter((id) => objectIds.has(id));
      const requestedId = result.selectionId ?? result.createdIds[0]?.objectId;
      const nextId =
        requestedId && objectIds.has(requestedId)
          ? requestedId
          : stateRef.current.activeObjectId && objectIds.has(stateRef.current.activeObjectId)
            ? stateRef.current.activeObjectId
            : null;
      if (nextId && !selectedIds.includes(nextId)) selectedIds.push(nextId);
      // The batch is committed; a later read failure must not retain its draft
      // or report the already-published changes as rejected.
      patchState({
        projection: result.projection,
        workspace: before.workspace
          ? { ...before.workspace, dirty: result.projection.dirty }
          : null,
        changes: [],
        changePreview: null,
        preparedChangeSet: null,
        activeObjectId: nextId,
        treeRevealId: nextId,
        objectSelection: selectedIds,
        integrationInspection: null,
        integrationIssues: [],
        integrationPreview: null,
      });
      let view: WorkspaceView;
      try {
        view = await call<WorkspaceView>('workspace_view');
      } catch (error) {
        if (generation !== epoch.current) return false;
        setNotice({
          tone: 'error',
          text: composedMessage('workflow.error.batchReadView', { error: errorText(error) }),
        });
        changeConfirmation.current?.(true);
        changeConfirmation.current = null;
        return true;
      }
      if (generation !== epoch.current) return false;
      const current = stateRef.current;
      const selection = findSelection(view, current.selection);
      patchState({
        projection: result.projection,
        workspace: view,
        selection,
        draft: hasUnapplied(before.workspace, current.draft)
          ? current.draft
          : draftFor(view, selection),
        diagnosticDraft:
          JSON.stringify(current.diagnosticDraft) ===
          JSON.stringify(diagnosticFields(before.workspace?.diagnostic ?? null))
            ? diagnosticFields(view.diagnostic)
            : current.diagnosticDraft,
        dtcDraft:
          JSON.stringify(current.dtcDraft) ===
          JSON.stringify(dtcFields(before.workspace?.diagnostic?.dtc ?? null))
            ? dtcFields(view.diagnostic?.dtc ?? null)
            : current.dtcDraft,
        changes: [],
        changePreview: null,
        preparedChangeSet: null,
        activeObjectId: nextId,
        treeRevealId: nextId,
        objectSelection: selectedIds,
      });
      if (view.integrationCandidate || before.integrationInspection) {
        patchState({
          integrationInspection: null,
          integrationIssues: [],
          integrationPreview: null,
          integrationNotice: message('workflow.integration.rechecking'),
        });
        try {
          const report = await call<IntegrationInspection>('inspect_integration');
          if (generation !== epoch.current) return false;
          const integration = stateRef.current;
          acceptIntegration(report);
          if (integration.integrationUnapplied)
            patchState({
              integrationIds: integration.integrationIds,
              integrationPeriod: integration.integrationPeriod,
              integrationUnapplied: true,
            });
          setIntegrationNotice(
            report.description
              ? message('workflow.integration.recheckedUnsaved')
              : message('workflow.integration.failed'),
          );
        } catch (error) {
          if (generation !== epoch.current) return false;
          const text = errorText(error);
          setIntegrationNotice(
            composedMessage('workflow.error.integrationRecheckRequired', { error: text }),
          );
          setNotice({
            tone: 'error',
            text: composedMessage('workflow.error.batchRecheck', { error: text }),
          });
        }
      }
      changeConfirmation.current?.(true);
      changeConfirmation.current = null;
      return true;
    } catch (error) {
      if (generation !== epoch.current) return false;
      patchState({ changePreview: null, preparedChangeSet: null });
      setNotice({
        tone: 'error',
        text: composedMessage('workflow.error.batchRejected', { error: errorText(error) }),
      });
      changeConfirmation.current?.(false);
      changeConfirmation.current = null;
      return false;
    } finally {
      if (generation === epoch.current) {
        running.current = false;
        setBusy(null);
      }
    }
  }
  return { stageChange, prepareChanges, applyChanges, discardChanges, cancelChangePreview };
}
