import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import test from 'node:test';
import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { descendants, processTree, launch, until as waitFor } from './support/driver.mjs';
import { readBoundaryEvidence } from './support/evidence.mjs';

test('reused parent PID does not adopt older session processes', () => {
  // Session processes whose short-lived smss.exe (8040) exited before the
  // driver tree reused its PID; their ParentProcessId still says 8040.
  const listed = [
    {
      pid: 8052,
      parent: 8040,
      name: 'csrss.exe',
      identity: '134030000000000000',
    },
    {
      pid: 8100,
      parent: 8040,
      name: 'winlogon.exe',
      identity: '134030000000000001',
    },
    {
      pid: 8148,
      parent: 8100,
      name: 'fontdrvhost.exe',
      identity: '134030000000000002',
    },
    {
      pid: 6096,
      parent: 8100,
      name: 'dwm.exe',
      identity: '134030000000000003',
    },
    {
      pid: 1000,
      parent: 900,
      name: 'tauri-driver.exe',
      identity: '134040000000000000',
    },
    {
      pid: 8040,
      parent: 1000,
      name: 'msedgedriver.exe',
      identity: '134040000000000001',
    },
    {
      pid: 2000,
      parent: 8040,
      name: 'autosar-config-desktop.exe',
      identity: '134040000000000001',
    },
    {
      pid: 2100,
      parent: 2000,
      name: 'msedgewebview2.exe',
      identity: '134040000000000002',
    },
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

test('a driver output write failure is reported after complete owned native exit', async () => {
  const session = await launch('failed-output-cleanup');
  const output = path.join(session.scratch, 'driver.stderr.log');
  let injected = false;
  let captureErrors;
  try {
    await waitFor(
      () => session.evaluateReadiness('Boolean(window.__TAURI_INTERNALS__)'),
      'native startup before controlled output failure',
      60000,
    );
    await session.evaluate("window.__TAURI_INTERNALS__.invoke('workbench_capabilities')");
    await waitFor(
      async () =>
        (
          await readFile(output, 'utf8').catch((error) => {
            if (error.code === 'ENOENT') return '';
            throw error;
          })
        ).includes('AUTOSAR_NATIVE_REQUEST '),
      'real native command evidence before output failure',
    );
    await rename(output, path.join(session.scratch, 'driver.stderr.before-fault.log'));
    await mkdir(output);
    injected = true;
    await session.evaluate("window.__TAURI_INTERNALS__.invoke('workbench_capabilities')");
  } finally {
    if (injected)
      await assert.rejects(session.close(), (error) => {
        assert(error instanceof AggregateError);
        assert(
          error.errors.some(
            (item) => item.path === output && ['EISDIR', 'EACCES', 'EPERM'].includes(item.code),
          ),
          error,
        );
        captureErrors = error.errors.map((item) => ({
          message: String(item),
          path: item.path,
          code: item.code,
          cause: item.cause ? String(item.cause) : undefined,
        }));
        return true;
      });
    else await session.close();
  }
  await writeFile(
    path.join(session.scratch, 'output-capture-failure.json'),
    JSON.stringify(captureErrors, null, 2),
  );
  const closed = JSON.parse(
    await readFile(path.join(session.scratch, 'process-tree.json'), 'utf8'),
  );
  assert(closed.length > 0, 'output failure must retain the actual owned process tree');
  const remaining = new Map(
    (await processTree(process.pid)).map((item) => [item.pid, item.identity]),
  );
  for (const item of closed)
    assert.notEqual(remaining.get(item.pid), item.identity, `owned process survived: ${item.pid}`);
});

test(
  'a controlled native response timeout retains request evidence before cleanup',
  {
    skip: process.platform !== 'linux',
  },
  async () => {
    const session = await launch('request-evidence-timeout', {
      requestEvidence: true,
    });
    const originalFetch = globalThis.fetch;
    const expression = `window.__TAURI_INTERNALS__.invoke('workbench_capabilities', {}, {
    headers: {'x-autosar-evaluation': String(window.__AUTOSAR_DESKTOP_EVALUATION__.id)}
  })`;
    let injected = false;
    try {
      await waitFor(
        () => session.evaluateReadiness('Boolean(window.__TAURI_INTERNALS__)'),
        'native startup before controlled response timeout',
        60000,
      );
      globalThis.fetch = async (url, options) => {
        const response = await originalFetch(url, options);
        const body = options?.body && JSON.parse(options.body);
        if (body?.args?.[0]?.includes(expression)) {
          // The native result crosses both HTTP hops and the real parser first.
          // Only this test withholds Node receipt until an interval is observed.
          const result = await response.json();
          assert.equal(typeof JSON.parse(result.value).value.fingerprint, 'string');
          await waitFor(async () => {
            const records = (
              await readFile(path.join(session.scratch, 'interval-threads.jsonl'), 'utf8')
            )
              .trim()
              .split('\n')
              .map(JSON.parse);
            return records.some(
              (record) =>
                record.request?.evaluation === body.args[1] && record.withinRequest && record.delta,
            );
          }, 'a thread interval for the deliberately withheld native response');
          injected = true;
          throw new DOMException('Controlled Node response receipt timeout', 'TimeoutError');
        }
        return response;
      };
      await assert.rejects(session.evaluate(expression), {
        name: 'TimeoutError',
      });
      await session.failureProcesses();
    } finally {
      globalThis.fetch = originalFetch;
      if (injected) await assert.rejects(session.close(), { name: 'TimeoutError' });
      else await session.close();
    }
    const read = (name) => readFile(path.join(session.scratch, name), 'utf8');
    const requests = (await read('webdriver.jsonl')).trim().split('\n').map(JSON.parse);
    const failed = requests.find((record) => record.phase === 'failed');
    assert(failed?.evaluation, 'timeout must retain its exact evaluation identity');
    const page = (await read('page-lifecycle.jsonl')).trim().split('\n').map(JSON.parse);
    assert.deepEqual(
      page
        .filter((record) => record.sample.id === failed.evaluation)
        .map((record) => record.sample.stage),
      ['entered', 'resolved', 'callback'],
    );
    const { status, events: boundaries } = await readBoundaryEvidence(session.scratch);
    const marker = `/*autosar-evaluation:${failed.evaluation}*/`;
    if (status.stage === 'ready') {
      assert(!boundaries.some((record) => record.stage === 'probe-error'));
      const marked = (record) =>
        record.prefixHex &&
        Buffer.from(record.prefixHex.replaceAll(' ', ''), 'hex')
          .toString(record.is8Bit === 0 ? 'utf16le' : 'latin1')
          .includes(marker);
      const send = boundaries.find(
        (record) => record.stage.startsWith('driver-send') && marked(record),
      );
      assert(send, 'the failed evaluation must bind an outgoing Automation sequence');
      for (const stage of [
        'driver-send',
        'ui-command',
        'ui-response',
        'driver-dispatch',
        'driver-parse',
        'driver-reconstruct',
      ])
        assert(
          boundaries.some((record) => record.stage === stage && record.sequence === send.sequence),
          `missing boundary for Automation sequence ${send.sequence}: ${stage}`,
        );
      const parsed = boundaries.find(
        (record) => record.stage === 'driver-parse' && record.sequence === send.sequence,
      );
      const rebuilt = boundaries.find(
        (record) => record.stage === 'driver-reconstruct' && record.sequence === send.sequence,
      );
      assert.equal(parsed.handler, rebuilt.handler);
      assert(parsed.bufferByteLength > 0);
      assert.equal(rebuilt.rootKind, 4, 'the complete result remains an opaque JSON string');
      const web = boundaries.find((record) => record.stage === 'web-command' && marked(record));
      assert(web, 'the same evaluation must reach the WebProcess');
      const javascript = boundaries.find(
        (record) => record.stage === 'web-javascript' && record.commandToken === web.commandToken,
      );
      const callback = boundaries.find(
        (record) => record.stage === 'web-response' && record.commandToken === web.commandToken,
      );
      assert(javascript && callback);
      assert.equal(javascript.frame, callback.frame);
      assert.equal(javascript.callback, callback.callback);
    } else {
      assert.equal(status.stage, 'unavailable');
      assert(status.error || status.text, 'unavailable probes must retain their reason');
      assert.deepEqual(boundaries, []);
    }
    const native = (await read('driver.stderr.log'))
      .split('\n')
      .filter((line) => line.startsWith('AUTOSAR_NATIVE_REQUEST '))
      .map((line) => JSON.parse(line.slice('AUTOSAR_NATIVE_REQUEST '.length)))
      .filter((record) => record.evaluation === String(failed.evaluation));
    assert.deepEqual(
      native.map((record) => record.stage),
      ['entered', 'returned'],
    );
    assert.equal(native[0].id, native[1].id);
    assert.equal(JSON.parse(await read('probe-capture.json')).errors.length, 0);
    const intervals = (await read('interval-threads.jsonl')).trim().split('\n').map(JSON.parse);
    assert(
      intervals.some(
        (record) =>
          record.request?.evaluation === failed.evaluation && record.withinRequest && record.delta,
      ),
    );
    const statuses = (await read('probe-status.jsonl')).trim().split('\n').map(JSON.parse);
    const started = statuses.filter((record) => record.stage === 'spawned');
    for (const collector of started)
      assert(
        statuses.some((record) => record.name === collector.name && record.stage === 'closed'),
        `${collector.name} cleanup must be recorded`,
      );
    assert(JSON.parse(await read('process-tree.json')).length > 0);
    if (status.stage === 'ready') {
      assert((await read('tcpdump.stderr.log')).includes('0 packets dropped by kernel'));
      const owner = JSON.parse(await read('session.json'));
      const packetBytes = await readFile(path.join(session.scratch, 'transport-headers.pcap'));
      assert.equal(packetBytes.readUInt32LE(20), 1, 'loopback capture uses Ethernet framing');
      const requestHops = new Set();
      for (let offset = 24; offset + 16 <= packetBytes.length;) {
        const length = packetBytes.readUInt32LE(offset + 8);
        const packet = packetBytes.subarray(offset + 16, offset + 16 + length);
        offset += 16 + length;
        if (packet.length < 54 || packet.readUInt16BE(12) !== 0x0800) continue;
        const ip = packet.subarray(14);
        if (ip[9] !== 6) continue;
        const tcp = ip.subarray((ip[0] & 15) * 4);
        const payload = tcp.subarray((tcp[12] >> 4) * 4);
        if (payload.includes(Buffer.from(marker))) requestHops.add(tcp.readUInt16BE(2));
      }
      assert(requestHops.has(owner.port), 'failed evaluation must reach tauri-driver HTTP hop');
      assert(requestHops.has(owner.nativePort), 'failed evaluation must reach native HTTP hop');
    }
  },
);
