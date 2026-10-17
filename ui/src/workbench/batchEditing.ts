import { composedMessage, isMessage, message, ProductError } from '../i18n';
import type { Text } from '../i18n';
import type { IntegrationInspection, WorkspaceView } from '../types';
import { errorText, headline } from './forms';
import type { Notice } from './forms';
import type {
  ChangeOutcome,
  ChangePreview,
  ChangeSet,
  ConfigurationChange,
  FieldDescriptor,
} from './projectTypes';
import type { WorkbenchSession } from './session';

/** Display name of a field, as its inspector label shows it. */
export function fieldName(field: Pick<FieldDescriptor, 'definitionId'>): string {
  return field.definitionId.split('#').pop()?.split('/').pop() ?? field.definitionId;
}

/**
 * The backend prefixes a rejected value with the change context naming its field.
 * Returns the field and the reason without that context, or null for other errors.
 */
export function fieldRejection(
  error: unknown,
  fields: readonly FieldDescriptor[],
): { field: FieldDescriptor; message: Text } | null {
  if (!Array.isArray(error) || error.length < 2) return null;
  const [context, ...reason] = error;
  if (!isMessage(context) || context.key !== 'backend.arxml.changes.field_change_context')
    return null;
  let reference: unknown;
  try {
    reference = JSON.parse(String(context.params.field));
  } catch {
    return null;
  }
  const fieldId =
    typeof reference === 'object' && reference !== null && 'fieldId' in reference
      ? reference.fieldId
      : null;
  const field = fields.find((item) => item.fieldId === fieldId);
  if (!field) return null;
  const detail = errorText(reason.length === 1 ? reason[0] : reason);
  return {
    field,
    message:
      field.minimum !== null || field.maximum !== null
        ? composedMessage(
            'workflow.error.fieldRange',
            { reason: detail },
            { minimum: field.minimum ?? '−∞', maximum: field.maximum ?? '+∞' },
          )
        : detail,
  };
}

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

  function rejection(
    key: string,
    error: unknown,
  ): { notice: Notice; fieldErrors: Record<string, Text> } {
    const detail = errorText(error);
    const field = fieldRejection(error, stateRef.current.projection?.fields ?? []);
    if (!field)
      return {
        notice: { tone: 'error', text: composedMessage(key, { error: headline(error) }), detail },
        fieldErrors: {},
      };
    return {
      notice: {
        tone: 'error',
        text: composedMessage(
          'workflow.error.batchFieldRejected',
          { error: field.message },
          { field: fieldName(field.field) },
        ),
        detail,
        fieldId: field.field.fieldId,
      },
      fieldErrors: { [field.field.fieldId]: field.message },
    };
  }

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
    // A rejection belongs to the value that caused it; editing that field retracts it.
    const previous = stateRef.current.changes;
    const edited = [
      ...previous.filter((change) => !changes.includes(change)),
      ...changes.filter((change) => !previous.includes(change)),
    ].flatMap((change) =>
      'field' in change && change.field.kind === 'existing' ? [change.field.fieldId] : [],
    );
    const fieldErrors = Object.fromEntries(
      Object.entries(stateRef.current.fieldErrors).filter(([fieldId]) => !edited.includes(fieldId)),
    );
    const notice = stateRef.current.notice;
    patchState({
      changes,
      changePreview: null,
      preparedChangeSet: null,
      fieldErrors,
      ...(notice?.fieldId && edited.includes(notice.fieldId) ? { notice: null } : {}),
    });
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
    patchState({ busy: message('activity.previewChanges'), notice: null, fieldErrors: {} });
    const generation = epoch.current;
    try {
      const preview = await call<ChangePreview>('prepare_configuration_change', { changeSet });
      if (generation !== epoch.current) return;
      if (stateRef.current.changes !== current.changes)
        throw new ProductError(message('workflow.error.draftChanged'));
      patchState({ changePreview: preview, preparedChangeSet: changeSet });
    } catch (error) {
      if (generation !== epoch.current) return;
      patchState({
        changePreview: null,
        preparedChangeSet: null,
        ...rejection('workflow.error.batchPreview', error),
      });
    } finally {
      if (generation === epoch.current) {
        running.current = false;
        setBusy(null);
      }
    }
  }

  async function applyChanges(): Promise<boolean> {
    const { changePreview, preparedChangeSet } = stateRef.current;
    if (!changePreview || !preparedChangeSet || running.current) return false;
    const generation = epoch.current;
    const applied = await commit(preparedChangeSet, changePreview.changeRevision);
    if (generation === epoch.current) {
      changeConfirmation.current?.(applied);
      changeConfirmation.current = null;
    }
    return applied;
  }

  /**
   * Applies the draft of one existing field on its own (ADR 0020 decision 3): the same
   * prepare and apply transaction as a batch, without the preview dialog. Other drafts stay.
   */
  async function applyField(fieldId: string): Promise<boolean> {
    const current = stateRef.current;
    const change = current.changes.find(
      (item) => 'field' in item && item.field.kind === 'existing' && item.field.fieldId === fieldId,
    );
    if (!current.projection || !change || running.current) return false;
    const changeSet: ChangeSet = {
      workspaceEpoch: current.projection.workspaceEpoch,
      inputFingerprint: capabilitiesRef.current!.fingerprint,
      definitionFingerprint: current.projection.definitionFingerprint,
      changes: [change],
    };
    return commit(
      changeSet,
      async () =>
        (await call<ChangePreview>('prepare_configuration_change', { changeSet })).changeRevision,
      fieldId,
    );
  }

  /** Publishes a change set; on success only its own changes leave the drafts. */
  async function commit(
    changeSet: ChangeSet,
    revision: string | (() => Promise<string>),
    fieldId?: string,
  ): Promise<boolean> {
    const before = stateRef.current;
    const remaining = () =>
      stateRef.current.changes.filter((change) => !changeSet.changes.includes(change));
    const otherErrors = () =>
      fieldId === undefined
        ? {}
        : Object.fromEntries(
            Object.entries(stateRef.current.fieldErrors).filter(([id]) => id !== fieldId),
          );
    running.current = true;
    // A single apply leaves another field's rejection notice with that field's cell error.
    const notice = stateRef.current.notice;
    const keepNotice = fieldId !== undefined && notice?.fieldId && notice.fieldId !== fieldId;
    patchState({
      busy: message('activity.apply'),
      notice: keepNotice ? notice : null,
      fieldErrors: otherErrors(),
    });
    const generation = epoch.current;
    const fingerprintBefore = capabilitiesRef.current?.fingerprint;
    try {
      const changeRevision = typeof revision === 'string' ? revision : await revision();
      if (generation !== epoch.current) return false;
      const result = await call<ChangeOutcome>('apply_configuration_change', {
        changeSet,
        changeRevision,
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
        changes: remaining(),
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
        return true;
      }
      if (generation !== epoch.current) return false;
      patchState({
        projection: result.projection,
        workspace: view,
        changes: remaining(),
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
              : message('controller.integration.failed'),
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
      if (!stateRef.current.notice) {
        const field = before.projection?.fields.find((item) => item.fieldId === fieldId);
        setNotice({
          tone: 'success',
          text: field
            ? message('workflow.batch.fieldApplied', { field: fieldName(field) })
            : message('workflow.batch.applied', { count: changeSet.changes.length }),
        });
      }
      return true;
    } catch (error) {
      if (generation !== epoch.current) return false;
      const rejected = rejection('workflow.error.draftsRejected', error);
      const field = before.projection?.fields.find((item) => item.fieldId === fieldId);
      if (field) {
        // A single field's rejection stays on its cell even when the backend names no field.
        const reason = rejected.fieldErrors[field.fieldId] ?? errorText(error);
        rejected.fieldErrors = { [field.fieldId]: reason };
        rejected.notice = {
          tone: 'error',
          text: composedMessage(
            'workflow.error.fieldRejected',
            { error: reason },
            { field: fieldName(field) },
          ),
          detail: errorText(error),
          fieldId: field.fieldId,
        };
      }
      patchState({
        changePreview: null,
        preparedChangeSet: null,
        notice: rejected.notice,
        fieldErrors: { ...otherErrors(), ...rejected.fieldErrors },
      });
      return false;
    } finally {
      if (generation === epoch.current) {
        running.current = false;
        setBusy(null);
      }
    }
  }
  return {
    stageChange,
    prepareChanges,
    applyChanges,
    applyField,
    discardChanges,
    cancelChangePreview,
  };
}
