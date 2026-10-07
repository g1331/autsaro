import assert from 'node:assert/strict';
import test from 'node:test';
import { loadSource } from './load-typescript.mjs';

const { importedSaveStage } = await loadSource('../src/workbench/useDelivery.ts', {
  '@tauri-apps/api/core': 'export const isTauri = () => false;',
});

test('unsupported imports are blocked even when no edits are pending', () => {
  for (const code of ['PDU_UNSUPPORTED', 'DIAG_UNSUPPORTED']) {
    assert.equal(importedSaveStage({ dirty: false, issues: [{ code }] }).state, 'failed');
  }
});

test('unsaved configuration remains pending while saved configuration is done', () => {
  assert.equal(importedSaveStage({ dirty: true, issues: [] }).state, 'pending');
  assert.equal(importedSaveStage({ dirty: false, issues: [] }).state, 'done');
});
