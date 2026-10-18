import { afterEach, expect, test, vi } from 'vitest';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

const state = vi.hoisted(() => ({ scratch: '', checks: new Map() }));
vi.mock('../../tests/desktop/support/workbench.mjs', () => ({
  dialogNamed: () => 'null',
  openWorkbench: async () => ({
    scratch: state.scratch,
    check: (name, body) => state.checks.set(name, body),
    evaluate: async (expression) =>
      expression.includes('protocol:')
        ? { protocol: 'tauri:', hostname: 'localhost', port: '' }
        : false,
    capabilities: async () => ({
      ruleError: null,
      ruleSetIdentity: { sha256: 'sha' },
      ruleCoverage: [1],
      toolError: true,
    }),
  }),
}));
afterEach(async () => {
  await rm(state.scratch, { recursive: true, force: true });
  state.checks.clear();
  vi.resetModules();
});

const initial = {
  id: 1,
  phase: 'completed',
  route: '/session/execute/sync',
  script: 'Boolean(window.__TAURI_INTERNALS__)',
  probe: { value: true },
};
const reload = {
  ...initial,
  id: 2,
  script: 'performance.timeOrigin !== 1 && Boolean(window.__TAURI_INTERNALS__)',
};

for (const [name, requests, error] of [
  ['one request with two phases', [{ ...initial, phase: 'started' }, initial], /Reloaded/],
  ['an unfinished reload', [initial, { ...reload, phase: 'started' }], /Reloaded/],
  ['a false reload sample', [initial, { ...reload, probe: { value: false } }], /Reloaded/],
  ['only a reloaded document', [reload], /Initial/],
  ['both successful document boundaries', [initial, reload], null],
]) {
  test(`startup evidence distinguishes ${name}`, async () => {
    state.scratch = await mkdtemp(path.join(os.tmpdir(), 'desktop-startup-evidence-'));
    await writeFile(
      path.join(state.scratch, 'webdriver.jsonl'),
      requests.map(JSON.stringify).join('\n'),
    );
    await import('../../tests/desktop/startup.test.mjs');
    const run = state.checks.get('packaged-builtin-boundary')();
    if (error) await expect(run).rejects.toThrow(error);
    else await expect(run).resolves.toBeUndefined();
  });
}
