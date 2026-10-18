import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import test from 'node:test';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { descendants, processTree, launch, until as waitFor } from './support/driver.mjs';

test('reused parent PID does not adopt older session processes', () => {
  // Session processes whose short-lived smss.exe (8040) exited before the
  // driver tree reused its PID; their ParentProcessId still says 8040.
  const listed = [
    { pid: 8052, parent: 8040, name: 'csrss.exe', identity: '134030000000000000' },
    { pid: 8100, parent: 8040, name: 'winlogon.exe', identity: '134030000000000001' },
    { pid: 8148, parent: 8100, name: 'fontdrvhost.exe', identity: '134030000000000002' },
    { pid: 6096, parent: 8100, name: 'dwm.exe', identity: '134030000000000003' },
    { pid: 1000, parent: 900, name: 'tauri-driver.exe', identity: '134040000000000000' },
    { pid: 8040, parent: 1000, name: 'msedgedriver.exe', identity: '134040000000000001' },
    { pid: 2000, parent: 8040, name: 'autosar-config-desktop.exe', identity: '134040000000000001' },
    { pid: 2100, parent: 2000, name: 'msedgewebview2.exe', identity: '134040000000000002' },
    { pid: 2200, parent: 2000, name: 'unknown-start.exe', identity: 'null' },
  ];
  assert.deepEqual(
    descendants(listed, 1000).map((item) => item.pid),
    [1000, 8040, 2000, 2100],
  );
});

test('a real child process is attributed to its live parent', async () => {
  const child = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 60000)'], {
    stdio: 'ignore',
  });
  try {
    await once(child, 'spawn');
    const tree = await processTree(process.pid);
    assert(
      tree.some((item) => item.pid === child.pid),
      'a residual child below the root must remain visible to the close() check',
    );
  } finally {
    child.kill();
  }
});

test('a failed transport still proves complete owned native process exit', async () => {
  const session = await launch('failed-request-cleanup');
  const originalFetch = globalThis.fetch;
  const failure = new Error('controlled response transport loss');
  const expression = '({native: typeof window.__TAURI_INTERNALS__.invoke})';
  let injected = false;
  try {
    await waitFor(
      () =>
        session.evaluateReadiness(
          "Boolean(window.__TAURI_INTERNALS__ && document.querySelector('button'))",
        ),
      'native startup before controlled transport loss',
      60000,
    );
    globalThis.fetch = async (url, options) => {
      const response = await originalFetch(url, options);
      if (options?.body && JSON.parse(options.body).args?.[0] === expression) {
        const result = await response.json();
        assert.equal(
          JSON.parse(result.value).value.native,
          'function',
          'real native page evaluated first',
        );
        injected = true;
        throw failure;
      }
      return response;
    };
    await assert.rejects(session.evaluate(expression), (error) => error === failure);
    if (process.platform === 'linux') {
      const owner = JSON.parse(await readFile(path.join(session.scratch, 'session.json'), 'utf8'));
      const tree = await processTree(owner.driverPid);
      const application = tree.find((item) => item.command === process.env.AUTOSAR_DESKTOP_APP);
      assert(application, 'the stopped application must belong to this native session');
      process.kill(application.pid, 'SIGSTOP');
      await waitFor(
        async () =>
          (await processTree(owner.driverPid)).some(
            (item) =>
              item.pid === application.pid &&
              item.identity === application.identity &&
              item.state.startsWith('T'),
          ),
        'owned application stopped before failure-path cleanup',
      );
    }
  } finally {
    globalThis.fetch = originalFetch;
    if (injected) await assert.rejects(session.close(), (error) => error === failure);
    else await session.close();
  }
  const closed = JSON.parse(
    await readFile(path.join(session.scratch, 'process-tree.json'), 'utf8'),
  );
  assert(closed.length > 0, 'cleanup must retain the actual owned process tree');
  const remaining = new Map(
    (await processTree(process.pid)).map((item) => [item.pid, item.identity]),
  );
  for (const item of closed)
    assert.notEqual(remaining.get(item.pid), item.identity, `owned process survived: ${item.pid}`);
});
