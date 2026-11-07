import { spawn, execFile } from 'node:child_process';
import { once } from 'node:events';
import {
  appendFile,
  open,
  readFile,
  readdir,
  readlink,
  realpath,
  writeFile,
} from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';
import { promisify } from 'node:util';

const run = promisify(execFile);
const clock = () => ({
  at: Date.now(),
  monoNs: process.hrtime.bigint().toString(),
});
const library = '/usr/lib/x86_64-linux-gnu/libwebkit2gtk-4.1.so.0';
const identities = [
  [
    '/usr/bin/WebKitWebDriver',
    '6d666ac8741bb707b84b83af8c45b5aeda575aec',
    [
      [0x4537b, '488d75c0'],
      [0x4597f, '488b7db0'],
      [0x3bf98, '488d7d80'],
      [0x3bfce, 'e82dddfdff'],
    ],
  ],
  [
    library,
    '6d0db877d1bda65539bf0c84fd6f72773b56ed4b',
    [
      [0x71e1f0, 'f30f1efa'],
      [0x7272a0, 'f30f1efa'],
      [0x1004940, 'f30f1efa'],
      [0x1004fc5, 'e8065070ff'],
      [0x1005cf0, 'f30f1efa'],
    ],
  ],
];

// These counters are kernel clock ticks/fault counts, keyed by start time so a
// recycled TID never inherits a previous thread's interval.
export function threadCounters(stat) {
  const fields = stat
    .slice(stat.lastIndexOf(') ') + 2)
    .trim()
    .split(/\s+/);
  return {
    state: fields[0],
    identity: fields[19],
    minorFaults: Number(fields[7]),
    majorFaults: Number(fields[9]),
    userTicks: Number(fields[11]),
    systemTicks: Number(fields[12]),
  };
}

export async function startEvidence(directory, ports, processTree) {
  let writes = Promise.resolve();
  const errors = [];
  const children = [];
  const abandon = () => {
    for (const { child } of children) child.kill('SIGTERM');
  };
  process.once('exit', abandon);
  const save = (name, value) => {
    writes = writes
      .then(() =>
        appendFile(
          path.join(directory, name),
          typeof value === 'string' ? value : JSON.stringify({ ...clock(), ...value }) + '\n',
        ),
      )
      .catch((error) => {
        errors.push(String(error));
      });
    return writes;
  };
  const server = createServer(async (request, response) => {
    request.setEncoding('utf8');
    let body = '';
    for await (const chunk of request) body += chunk;
    try {
      await save('page-lifecycle.jsonl', {
        received: clock(),
        sample: JSON.parse(body),
      });
      response.setHeader('Access-Control-Allow-Origin', '*');
      response.writeHead(204).end();
    } catch (error) {
      errors.push(String(error));
      response.writeHead(400).end();
    }
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  server.unref();
  const endpoint = `http://127.0.0.1:${server.address().port}/page`;
  async function start(command, args, name, ready, env = process.env) {
    const privileged = process.getuid() !== 0;
    const child = spawn(
      privileged ? 'sudo' : command,
      privileged ? ['-n', '--preserve-env=BPFTRACE_MAX_STRLEN', command, ...args] : args,
      {
        env,
        stdio: ['ignore', 'pipe', 'pipe'],
      },
    );
    const closed = once(child, 'close').catch((error) => [null, String(error)]);
    children.push({ child, closed, name });
    await save('probe-status.jsonl', {
      stage: 'spawned',
      name,
      pid: child.pid,
      command,
      args,
    });
    let text = '';
    let signalReady;
    const readiness = new Promise((resolve) => {
      signalReady = resolve;
    });
    for (const channel of ['stdout', 'stderr'])
      child[channel].on('data', (chunk) => {
        void save(`${name}.${channel}.log`, chunk.toString());
        text = (text + chunk.toString()).slice(-16384);
        if (text.includes(ready)) signalReady(true);
      });
    let timer;
    const available = await Promise.race([
      readiness,
      closed.then(() => false),
      new Promise((resolve) => {
        timer = setTimeout(() => resolve(false), 30000);
      }),
    ]);
    clearTimeout(timer);
    await save('probe-status.jsonl', {
      name,
      stage: available ? 'ready' : 'unavailable',
      text,
    });
    if (!available) child.kill('SIGTERM');
    return available;
  }
  await start(
    'tcpdump',
    [
      '-i',
      'lo',
      '-B',
      '8192',
      '-U',
      '-s',
      '512',
      '-w',
      path.join(directory, 'transport-headers.pcap'),
      `tcp port ${ports.port} or tcp port ${ports.nativePort}`,
    ],
    'tcpdump',
    'listening on',
  );

  let timer;
  let sampling;
  let closed = false;
  let request;
  const previous = new Map();
  async function sample() {
    const context = request;
    const tree = await processTree();
    for (const item of [...tree, { pid: process.pid, name: 'Node' }]) {
      const started = clock();
      try {
        const tasks = await readdir(`/proc/${item.pid}/task`);
        for (const tid of tasks) {
          const readStarted = clock();
          try {
            const current = threadCounters(
              await readFile(`/proc/${item.pid}/task/${tid}/stat`, 'utf8'),
            );
            const key = `${item.pid}/${tid}/${current.identity}`;
            const before = previous.get(key);
            const delta = before
              ? Object.fromEntries(
                  ['minorFaults', 'majorFaults', 'userTicks', 'systemTicks'].map((field) => [
                    field,
                    current[field] - before.current[field],
                  ]),
                )
              : null;
            const wchan = await readFile(`/proc/${item.pid}/task/${tid}/wchan`, 'utf8');
            const finished = clock();
            await save('interval-threads.jsonl', {
              request: context,
              pid: item.pid,
              name: item.name,
              tid,
              readStarted,
              finished,
              current,
              previous: before?.finished,
              previousRequest: before?.request,
              withinRequest: Boolean(
                context && request?.id === context.id && before?.request?.id === context.id,
              ),
              delta,
              wchan,
            });
            previous.set(key, { current, finished, request: context });
          } catch (error) {
            await save('interval-threads.jsonl', {
              request: context,
              pid: item.pid,
              tid,
              readStarted,
              error: String(error),
            });
          }
        }
      } catch (error) {
        await save('interval-threads.jsonl', {
          pid: item.pid,
          started,
          error: String(error),
        });
      }
    }
  }
  function schedule() {
    if (closed) return;
    timer = setTimeout(() => {
      sampling = sample()
        .catch((error) => errors.push(String(error)))
        .finally(schedule);
    }, 1000);
    timer.unref();
  }
  return {
    endpoint,
    async attach() {
      const tree = await processTree();
      await save('probe-owners.jsonl', { tree, node: process.pid });
      for (const item of tree)
        await save('probe-maps.jsonl', {
          pid: item.pid,
          identity: item.identity,
          maps: await readFile(`/proc/${item.pid}/maps`, 'utf8').catch(String),
        });
      try {
        const driver = tree.find((item) => item.name === 'WebKitWebDriver');
        if (
          !driver ||
          (await readlink(`/proc/${driver.pid}/exe`)) !== (await realpath(identities[0][0]))
        )
          throw new Error('Owned native driver does not match the guarded executable');
        for (const [file, buildID, guards] of identities) {
          const actual = (await run('readelf', ['-n', file])).stdout;
          await save('probe-status.jsonl', {
            name: 'webkit-boundaries',
            file,
            expectedBuildID: buildID,
            actualBuildID: actual.match(/Build ID: (\w+)/)?.[1],
          });
          if (!actual.includes(`Build ID: ${buildID}`))
            throw new Error(`Unsupported ELF identity: ${file}`);
          const handle = await open(file);
          try {
            for (const [offset, hex] of guards) {
              const expected = Buffer.from(hex, 'hex');
              const buffer = Buffer.alloc(expected.length);
              await handle.read(buffer, 0, buffer.length, offset);
              if (!buffer.equals(expected))
                throw new Error(`Instruction guard failed: ${file}:${offset}`);
            }
          } finally {
            await handle.close();
          }
        }
        let source = await readFile(new URL('./webkit-probes.bt', import.meta.url), 'utf8');
        source = source
          .replaceAll('DRIVER', identities[0][0])
          .replaceAll('WEBKIT', library)
          .replace('OWNERS', tree.map((item) => `@owned[${item.pid}] = 1;`).join('\n'));
        const script = path.join(directory, 'webkit-probes.bt');
        await writeFile(script, source);
        await start(
          'bpftrace',
          ['--unsafe', '-kk', '-q', script],
          'webkit-boundaries',
          '"stage":"ready"',
          {
            ...process.env,
            BPFTRACE_MAX_STRLEN: '160',
          },
        );
      } catch (error) {
        await save('probe-status.jsonl', {
          name: 'webkit-boundaries',
          stage: 'unavailable',
          error: String(error),
        });
      }
      await sample();
      schedule();
    },
    async record(value) {
      if (value.phase === 'started') request = { id: value.id, evaluation: value.evaluation };
      if (['completed', 'failed'].includes(value.phase)) request = undefined;
      await save('request-intervals.jsonl', { ...value, script: undefined });
      if (value.phase === 'failed') {
        await sampling;
        await sample();
      }
    },
    async close() {
      if (closed) return;
      closed = true;
      clearTimeout(timer);
      await sampling;
      for (const { child, closed: exited, name } of children.reverse()) {
        if (child.exitCode === null && child.signalCode === null) child.kill('SIGINT');
        let timer;
        const result = await Promise.race([
          exited,
          new Promise((resolve) => {
            timer = setTimeout(() => resolve(null), 5000);
          }),
        ]);
        clearTimeout(timer);
        if (!result) {
          child.kill('SIGKILL');
          await exited;
          errors.push(`${name} did not stop at its collection deadline`);
        }
        await save('probe-status.jsonl', { name, stage: 'closed', result });
      }
      server.closeAllConnections();
      await new Promise((resolve) => server.close(resolve));
      await writes;
      await writeFile(
        path.join(directory, 'probe-capture.json'),
        JSON.stringify(
          {
            ...clock(),
            errors,
            limits:
              '512-byte packet snapshots; check tcpdump drops. ' +
              'Bounded hexadecimal string prefixes. Storage lengths use Latin1/UTF16 units. ' +
              'Tick/fault deltas span recorded per-thread reads. ' +
              'Unavailable or lost probes leave stages unobserved.',
          },
          null,
          2,
        ),
      );
      process.off('exit', abandon);
    },
  };
}
