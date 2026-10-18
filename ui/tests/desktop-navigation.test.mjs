import { afterEach, expect, test, vi } from 'vitest';
import { mkdtemp, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { setImmediate } from 'node:timers/promises';

const state = vi.hoisted(() => ({ session: null, after: [] }));
vi.mock('node:test', () => ({ after: (hook) => state.after.push(hook), test: vi.fn() }));
vi.mock('../../tests/desktop/support/driver.mjs', () => ({ launch: () => state.session }));
import { openWorkbench } from '../../tests/desktop/support/workbench.mjs';

afterEach(async () => {
  for (const hook of state.after.splice(0)) await hook();
  await rm(state.session.scratch, { recursive: true, force: true });
});

async function session() {
  state.session = {
    scratch: await mkdtemp(path.join(os.tmpdir(), 'desktop-navigation-')),
    platform: 'linux',
    refresh: vi.fn(async () => {}),
    evaluate: vi.fn(async (expression) => {
      if (expression === 'performance.timeOrigin') return 1;
      if (expression.includes("invoke('workbench_capabilities')")) return { fingerprint: 'fp' };
      if (expression.includes('configure_language'))
        return { ok: true, reply: { inputFingerprint: 'fp', value: null } };
      return true;
    }),
    evaluateReadiness: vi.fn(async () => ({ value: true })),
    screenshot: vi.fn(async () => {}),
    resize: vi.fn(async () => {}),
    diagnoseStartup: vi.fn(async () => {}),
    close: vi.fn(async () => {}),
  };
  return state.session;
}

for (const boundary of ['startup language reload', 'source-preserving refresh']) {
  async function prepare() {
    const current = await session();
    const workbench = boundary === 'source-preserving refresh' ? await openWorkbench('test') : null;
    current.evaluateReadiness.mockClear();
    return {
      current,
      run: () => (workbench ? workbench.refreshSurface('project.json') : openWorkbench('test')),
    };
  }

  test(`${boundary} waits for navigation before probing the new document`, async () => {
    const { current, run } = await prepare();
    let finish;
    let began;
    const started = new Promise((resolve) => (began = resolve));
    current.refresh.mockImplementation(() => {
      began();
      return new Promise((resolve) => (finish = resolve));
    });
    const stoppedAtProbe = new Error('new document probe');
    current.evaluateReadiness.mockImplementation(async (expression) => {
      if (expression.includes('performance.timeOrigin')) throw stoppedAtProbe;
      return { value: true };
    });
    const operation = run();
    const result = expect(operation).rejects.toBe(stoppedAtProbe);
    await started;
    // Drain the continuation queue without imposing a wall-clock delay.
    await setImmediate();
    const reloadProbes = () =>
      current.evaluateReadiness.mock.calls.filter(([expression]) =>
        expression.includes('performance.timeOrigin'),
      );
    try {
      expect(reloadProbes()).toEqual([]);
    } finally {
      finish();
      await result;
    }
    expect(reloadProbes()).toHaveLength(1);
  });

  test(`${boundary} propagates navigation failure before any new document probe`, async () => {
    const { current, run } = await prepare();
    const failure = new Error('WebDriver refresh failed');
    current.refresh.mockImplementation(() => {
      const rejected = Promise.reject(failure);
      rejected.catch(() => undefined);
      return rejected;
    });
    current.evaluateReadiness.mockImplementation(async (expression) => {
      if (expression.includes('performance.timeOrigin')) throw new Error('unexpected probe');
      return { value: true };
    });
    await expect(run()).rejects.toBe(failure);
    expect(
      current.evaluateReadiness.mock.calls.filter(([expression]) =>
        expression.includes('performance.timeOrigin'),
      ),
    ).toEqual([]);
  });
}
