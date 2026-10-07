import { expect, test, vi } from 'vitest';
import { createBatchEditing } from '../src/workbench/batchEditing';
import { createProjectActions } from '../src/workbench/projectActions';
import { initialState } from '../src/workbench/state';

vi.mock('../src/workbench/Dialog', () => ({ rememberDialogOpener: vi.fn() }));

function session(overrides = {}) {
  const stateRef = { current: { ...initialState(), ...overrides } };
  const result = {
    state: stateRef.current,
    stateRef,
    capabilitiesRef: { current: { fingerprint: 'before' } },
    epoch: { current: 1 },
    running: { current: false },
    pendingGuard: { current: null },
    pendingReplacement: { current: null },
    changeConfirmation: { current: null },
    call: vi.fn(),
    patchState(patch) {
      stateRef.current = { ...stateRef.current, ...patch };
    },
    field(key) {
      return (value) =>
        result.patchState({
          [key]: typeof value === 'function' ? value(stateRef.current[key]) : value,
        });
    },
  };
  return result;
}

function batch(current) {
  return createBatchEditing(current, { invalidateAfterEdit: vi.fn(), acceptIntegration: vi.fn() });
}

test('committed changes are cleared even when refreshing the workspace fails', async () => {
  const current = session({
    changes: [{ changeId: 'edit' }],
    changePreview: { changeRevision: 'preview' },
    preparedChangeSet: { changes: [] },
  });
  const confirmed = vi.fn();
  current.changeConfirmation.current = confirmed;
  current.call
    .mockResolvedValueOnce({ projection: { objects: [], dirty: true }, createdIds: [] })
    .mockRejectedValueOnce(new Error('read failed'));
  expect(await batch(current).applyChanges()).toBe(true);
  expect(current.stateRef.current.changes).toEqual([]);
  expect(current.stateRef.current.notice.text).toContain('批次已应用');
  expect(confirmed).toHaveBeenCalledWith(true);
});

test('rejected batches retain the original draft', async () => {
  const changes = [{ changeId: 'edit' }];
  const current = session({
    changes,
    changePreview: { changeRevision: 'preview' },
    preparedChangeSet: { changes },
  });
  current.call.mockRejectedValue(new Error('stale preview'));
  expect(await batch(current).applyChanges()).toBe(false);
  expect(current.stateRef.current.changes).toBe(changes);
  expect(current.stateRef.current.changePreview).toBeNull();
});

test('a late batch result cannot replace the state after cancellation', async () => {
  const changes = [{ changeId: 'edit' }];
  const current = session({
    changes,
    changePreview: { changeRevision: 'preview' },
    preparedChangeSet: { changes },
  });
  let complete;
  current.call.mockImplementation(
    () =>
      new Promise((resolve) => {
        complete = resolve;
      }),
  );
  const pending = batch(current).applyChanges();
  current.epoch.current += 1;
  complete({ projection: { objects: [], dirty: true }, createdIds: [] });
  expect(await pending).toBe(false);
  expect(current.stateRef.current.changes).toBe(changes);
  expect(current.call).toHaveBeenCalledTimes(1);
});

test('draft changes during preview invalidate the returned preview', async () => {
  const current = session({
    changes: [{ changeId: 'first' }],
    projection: { workspaceEpoch: 1, definitionFingerprint: 'defs' },
  });
  let complete;
  current.call.mockImplementation(
    () =>
      new Promise((resolve) => {
        complete = resolve;
      }),
  );
  const pending = batch(current).prepareChanges();
  current.field('changes')([{ changeId: 'second' }]);
  complete({ changeRevision: 'preview' });
  await pending;
  expect(current.stateRef.current.changePreview).toBeNull();
  expect(current.stateRef.current.notice.text).toContain('草稿已变化');
});

test('editing or removing a draft invalidates its previous prepared change set', () => {
  const current = session({
    changes: [{ changeId: 'first' }],
    changePreview: { changeRevision: 'old' },
    preparedChangeSet: { changes: [] },
  });
  const actions = batch(current);
  actions.stageChange({ changeId: 'second' });
  expect(current.stateRef.current.changePreview).toBeNull();
  expect(current.stateRef.current.preparedChangeSet).toBeNull();
  actions.discardChanges('first');
  expect(current.stateRef.current.changes).toEqual([{ changeId: 'second' }]);
});

test('canceling the draft guard does not execute the pending context change', async () => {
  const current = session({ changes: [{ changeId: 'edit' }] });
  const action = vi.fn();
  const project = createProjectActions(current, {});
  await project.guardContext('切换对象', action);
  expect(current.stateRef.current.guard).not.toBeNull();
  await project.resolveGuard('cancel');
  expect(action).not.toHaveBeenCalled();
  expect(current.stateRef.current.changes).toEqual([{ changeId: 'edit' }]);
  expect(current.pendingGuard.current).toBeNull();
});

test('a later rejected draft keeps the earlier creation and reports partial application', async () => {
  const workspace = { frames: [], signals: [], diagnostic: null };
  const current = session({ workspace, creating: 'frame' });
  current.stateRef.current.frameInput = {
    ...current.stateRef.current.frameInput,
    name: 'Send',
    id: '100',
  };
  const diagnosticDraft = { ...current.stateRef.current.diagnosticDraft, requestId: 'invalid' };
  current.stateRef.current.diagnosticDraft = diagnosticDraft;
  const created = {
    ...workspace,
    dirty: true,
    frames: [
      {
        name: 'Send',
        path: '/Send',
        id: 100,
        dlc: 8,
        direction: 'tx',
        periodMs: 100,
        timeoutMs: null,
      },
    ],
  };
  current.call.mockResolvedValueOnce(created);
  const invalidateAfterEdit = vi.fn();
  const project = createProjectActions(current, {
    invalidateAfterEdit,
    refreshProjection: vi.fn(),
  });
  expect(await project.applyDrafts()).toBe(false);
  expect(current.stateRef.current.workspace).toBe(created);
  expect(current.stateRef.current.creating).toBeNull();
  expect(current.stateRef.current.diagnosticDraft).toBe(diagnosticDraft);
  expect(current.stateRef.current.notice.text).toContain('部分草稿已应用');
  expect(invalidateAfterEdit).toHaveBeenCalledOnce();
  expect(current.call).toHaveBeenCalledTimes(1);
});

test('an acknowledged integration edit is cleared before a failed workspace refresh', async () => {
  const current = session({
    workspace: { frames: [], signals: [], diagnostic: null },
    integrationUnapplied: true,
    integrationIds: { port: '100' },
    integrationPeriod: '20',
  });
  const report = { description: { signals: [] } };
  current.call.mockResolvedValueOnce(report).mockRejectedValueOnce(new Error('view unavailable'));
  const acceptIntegration = vi.fn();
  const invalidateAfterEdit = vi.fn();
  const project = createProjectActions(current, {
    acceptIntegration,
    invalidateAfterEdit,
    refreshProjection: vi.fn(),
  });
  expect(await project.applyDrafts()).toBe(false);
  expect(current.stateRef.current.integrationUnapplied).toBe(false);
  expect(acceptIntegration).toHaveBeenCalledWith(report);
  expect(invalidateAfterEdit).toHaveBeenCalledOnce();
  expect(current.stateRef.current.notice.text).toContain('部分草稿已应用');
});
