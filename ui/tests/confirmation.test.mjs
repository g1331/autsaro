import assert from 'node:assert/strict';
import test from 'node:test';
import { loadSource } from './load-typescript.mjs';

test('native cancellation is awaited and never falls through to browser confirmation', async (context) => {
  const previous = globalThis.window;
  globalThis.window = {
    confirm: () => {
      throw new Error('browser confirmation was used');
    },
  };
  context.after(() => {
    globalThis.window = previous;
  });
  const { requestConfirmation } = await loadSource('../src/confirmation.ts', {
    '@tauri-apps/api/core': 'export const isTauri = () => true;',
    '@tauri-apps/plugin-dialog': 'export const confirm = async () => false;',
  });
  assert.equal(await requestConfirmation('Discard edits?'), false);
});

test('browser confirmation preserves the user decision', async (context) => {
  const previous = globalThis.window;
  globalThis.window = { confirm: (message) => message === 'Accept?' };
  context.after(() => {
    globalThis.window = previous;
  });
  const { requestConfirmation } = await loadSource('../src/confirmation.ts', {
    '@tauri-apps/api/core': 'export const isTauri = () => false;',
    '@tauri-apps/plugin-dialog':
      'export const confirm = () => { throw new Error("native dialog used"); };',
  });
  assert.equal(await requestConfirmation('Accept?'), true);
  assert.equal(await requestConfirmation('Discard?'), false);
});
