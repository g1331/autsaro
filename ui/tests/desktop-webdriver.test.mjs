import { afterEach, expect, test, vi } from 'vitest';
import { createServer } from 'node:http';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { WebDriver } from '../../tests/desktop/support/webdriver.mjs';

const cleanups = [];
afterEach(async () => {
  for (const cleanup of cleanups.splice(0).reverse()) await cleanup();
  vi.restoreAllMocks();
});

async function driver(handler) {
  const scratch = await mkdtemp(path.join(os.tmpdir(), 'desktop-webdriver-'));
  cleanups.push(() => rm(scratch, { recursive: true, force: true }));
  const server = createServer(handler);
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  cleanups.push(() => {
    server.closeAllConnections();
    return new Promise((resolve) => server.close(resolve));
  });
  const log = path.join(scratch, 'webdriver.jsonl');
  return { client: new WebDriver(`http://127.0.0.1:${server.address().port}`, log), log };
}

test('native refresh waits for the navigation response before a new frame query', async () => {
  let completeNavigation;
  let sawRefresh;
  const started = new Promise((resolve) => (sawRefresh = resolve));
  const requests = [];
  const { client, log } = await driver(async (request, response) => {
    let body = '';
    for await (const chunk of request) body += chunk;
    requests.push({ method: request.method, url: request.url, body: JSON.parse(body) });
    if (request.url.endsWith('/refresh')) {
      completeNavigation = () => response.end(JSON.stringify({ value: null }));
    } else response.end(JSON.stringify({ value: { newDocument: true } }));
    sawRefresh();
  });
  const navigation = (async () => {
    await client.refresh('/session/native');
    return client.request('POST', '/session/native/execute/sync', {
      script: 'return {newDocument: true}',
      args: [],
    });
  })();
  cleanups.push(async () => {
    completeNavigation?.();
    await navigation.catch(() => undefined);
  });
  await started;
  expect(requests).toEqual([{ method: 'POST', url: '/session/native/refresh', body: {} }]);
  completeNavigation();
  expect(await navigation).toEqual({ newDocument: true });
  expect(requests.map((request) => request.url)).toEqual([
    '/session/native/refresh',
    '/session/native/execute/sync',
  ]);
  const entries = (await readFile(log, 'utf8')).trim().split('\n').map(JSON.parse);
  expect(
    entries.filter((entry) => entry.phase === 'completed').map((entry) => entry.route),
  ).toEqual(['/session/refresh', '/session/execute/sync']);
});

test('native refresh preserves the navigation error without retry or a frame query', async () => {
  let count = 0;
  const { client, log } = await driver((_request, response) => {
    count += 1;
    response.writeHead(404, { 'Content-Type': 'application/json' });
    response.end(JSON.stringify({ value: { error: 'no such window', message: 'closed' } }));
  });
  await expect(client.refresh('/session/native')).rejects.toThrow('no such window');
  expect(count).toBe(1);
  expect((await readFile(log, 'utf8')).trim().split('\n').map(JSON.parse).at(-1).error).toEqual({
    error: 'no such window',
    message: 'closed',
  });
});

test('an aborted native request retains its exact script and paired terminal evidence', async () => {
  const controller = new AbortController();
  vi.spyOn(AbortSignal, 'timeout').mockReturnValue(controller.signal);
  let received;
  const started = new Promise((resolve) => (received = resolve));
  const { client, log } = await driver(() => received());
  const expression = 'window.__TAURI_INTERNALS__.invoke("project_projection")';
  const operation = client.request('POST', '/session/native/execute/async', {
    script: 'evaluate',
    args: [expression],
  });
  const rejected = expect(operation).rejects.toThrow('controlled transport failure');
  cleanups.push(async () => {
    controller.abort(new Error('controlled transport failure'));
    await rejected;
  });
  await started;
  const pending = (await readFile(log, 'utf8')).trim().split('\n').map(JSON.parse);
  expect(pending).toEqual([
    expect.objectContaining({
      id: 1,
      phase: 'started',
      route: '/session/execute/async',
      script: expression,
    }),
  ]);
  controller.abort(new Error('controlled transport failure'));
  await rejected;
  const entries = (await readFile(log, 'utf8')).trim().split('\n').map(JSON.parse);
  expect(entries).toHaveLength(2);
  expect(entries[1]).toEqual(
    expect.objectContaining({
      id: 1,
      phase: 'failed',
      script: expression,
      error: 'Error: controlled transport failure',
      durationMs: expect.any(Number),
      at: expect.any(Number),
    }),
  );
  await expect(client.request('GET', '/session/native/screenshot')).rejects.toThrow(
    'controlled transport failure',
  );
  await expect(client.request('DELETE', '/session/native')).rejects.toThrow(
    'controlled transport failure',
  );
  expect((await readFile(log, 'utf8')).trim().split('\n')).toHaveLength(2);
  expect(AbortSignal.timeout).toHaveBeenCalledExactlyOnceWith(300000);
});

test('a truncated response records its parsing failure against the started request', async () => {
  const { client, log } = await driver((_request, response) => response.end('{"value":'));
  await expect(client.request('GET', '/session/native/screenshot')).rejects.toThrow();
  const entries = (await readFile(log, 'utf8')).trim().split('\n').map(JSON.parse);
  expect(entries.map(({ id, phase }) => ({ id, phase }))).toEqual([
    { id: 1, phase: 'started' },
    { id: 1, phase: 'failed' },
  ]);
  expect(entries[1]).toEqual(
    expect.objectContaining({
      route: '/session/screenshot',
      status: 200,
      error: expect.stringContaining('SyntaxError'),
    }),
  );
});

// Run the exact function serialized by the native session, without replacing
// any invocation inside the expression.
test('page evaluation records entry, pending resolution and callback handoff', async () => {
  const { evaluateInPage } = await import('../../tests/desktop/support/webdriver.mjs');
  const stages = [];
  const page = {
    __TAURI_INTERNALS__: { callbacks: new Map([[7, () => {}]]) },
    __AUTOSAR_DESKTOP_OBSERVER__: (sample) => stages.push({ ...sample }),
  };
  let resolve;
  page.pending = new Promise((done) => (resolve = done));
  vi.stubGlobal('window', page);
  vi.stubGlobal('location', { href: 'tauri://localhost' });
  vi.stubGlobal('document', { readyState: 'complete' });
  try {
    const result = new Promise((done) => evaluateInPage('window.pending', 41, done));
    await Promise.resolve();
    expect(stages.map(({ stage }) => stage)).toEqual(['entered']);
    expect(page.__AUTOSAR_DESKTOP_EVALUATION__).toEqual(
      expect.objectContaining({
        id: 41,
        nativeCallbacks: 1,
        expression: 'window.pending',
        readyState: 'complete',
      }),
    );
    resolve({ native: 'unchanged' });
    expect(JSON.parse(await result)).toEqual({ value: { native: 'unchanged' } });
    expect(stages.map(({ stage }) => stage)).toEqual(['entered', 'resolved', 'callback']);
    stages.length = 0;
    const error = await new Promise((done) =>
      evaluateInPage('Promise.reject(new Error("original"))', 42, done),
    );
    expect(JSON.parse(error)).toEqual({ error: 'Error: original' });
    expect(stages.map(({ stage }) => stage)).toEqual(['entered', 'rejected', 'callback']);
  } finally {
    vi.unstubAllGlobals();
  }
});

test('an observer failure is retained without replacing the evaluated result', async () => {
  const { evaluateInPage } = await import('../../tests/desktop/support/webdriver.mjs');
  const page = {
    __AUTOSAR_DESKTOP_OBSERVER__: () => {
      throw new Error('observer lost');
    },
  };
  vi.stubGlobal('window', page);
  vi.stubGlobal('location', { href: 'tauri://localhost' });
  vi.stubGlobal('document', { readyState: 'complete' });
  try {
    expect(JSON.parse(await new Promise((done) => evaluateInPage('true', 43, done)))).toEqual({
      value: true,
    });
    expect(page.__AUTOSAR_DESKTOP_EVALUATION__.observerError).toBe('Error: observer lost');
  } finally {
    vi.unstubAllGlobals();
  }
});

test('evaluation crosses WebDriver as opaque text without losing nested JSON values', async () => {
  const { evaluateInPage } = await import('../../tests/desktop/support/webdriver.mjs');
  const payload = Array.from({ length: 1000 }, (_, i) => ({
    objectId: `object-${i}`,
    fields: [{ value: `路径/"\\\n${i} Ω`, current: null, enabled: i % 2 === 0, count: i }],
  }));
  vi.stubGlobal('window', { payload });
  vi.stubGlobal('location', { href: 'tauri://localhost' });
  vi.stubGlobal('document', { readyState: 'complete' });
  try {
    const wire = await new Promise((done) => evaluateInPage('window.payload', 44, done));
    expect(typeof wire).toBe('string');
    expect(JSON.parse(wire)).toEqual({ value: payload });
  } finally {
    vi.unstubAllGlobals();
  }
});

test('evaluation serialization failures return one error callback instead of leaving a request pending', async () => {
  const { evaluateInPage } = await import('../../tests/desktop/support/webdriver.mjs');
  const payload = {};
  payload.self = payload;
  const stages = [];
  vi.stubGlobal('window', {
    payload,
    __AUTOSAR_DESKTOP_OBSERVER__: (sample) => stages.push(sample.stage),
  });
  vi.stubGlobal('location', { href: 'tauri://localhost' });
  vi.stubGlobal('document', { readyState: 'complete' });
  const done = vi.fn();
  try {
    const encoded = await new Promise((resolve) =>
      evaluateInPage('window.payload', 45, (value) => {
        done(value);
        resolve(value);
      }),
    );
    expect(JSON.parse(encoded).error).toMatch(/TypeError.*circular/i);
    expect(done).toHaveBeenCalledTimes(1);
    expect(stages).toEqual(['entered', 'resolved', 'rejected', 'callback']);
    const empty = await new Promise((resolve) =>
      evaluateInPage('({missing:undefined, items:[undefined], value:NaN})', 46, resolve),
    );
    expect(JSON.parse(empty)).toEqual({ value: { missing: null, items: [null], value: null } });
  } finally {
    vi.unstubAllGlobals();
  }
});
