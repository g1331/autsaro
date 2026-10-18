import { afterEach, expect, test } from 'vitest';
import { createServer } from 'node:http';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { WebDriver } from '../../tests/desktop/support/webdriver.mjs';

const cleanups = [];
afterEach(async () => {
  for (const cleanup of cleanups.splice(0).reverse()) await cleanup();
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
  expect(entries.map((entry) => entry.route)).toEqual([
    '/session/refresh',
    '/session/execute/sync',
  ]);
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
  expect(JSON.parse((await readFile(log, 'utf8')).trim()).error).toEqual({
    error: 'no such window',
    message: 'closed',
  });
});
