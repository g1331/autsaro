import assert from 'node:assert/strict';
import { test } from 'vitest';

import { importedSaveStage } from '../src/workbench/useDelivery';

test('unsupported imports are blocked even when no edits are pending', () => {
  for (const code of ['PDU_UNSUPPORTED', 'DIAG_UNSUPPORTED']) {
    const stage = importedSaveStage({ dirty: false, issues: [{ code }] });
    assert.equal(stage.state, 'failed');
    assert.deepEqual(stage.detail, { key: 'workflow.stage.unsupportedImport', params: {} });
  }
});

test('unsaved configuration remains pending while saved configuration is done', () => {
  const pending = importedSaveStage({ dirty: true, issues: [] });
  const saved = importedSaveStage({ dirty: false, issues: [] });
  assert.equal(pending.state, 'pending');
  assert.deepEqual(pending.detail, { key: 'workflow.stage.notSaved', params: {} });
  assert.equal(saved.state, 'done');
  assert.deepEqual(saved.detail, { key: 'workflow.stage.projectSaved', params: {} });
});
