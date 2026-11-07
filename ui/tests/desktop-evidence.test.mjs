import { afterEach, expect, test, vi } from 'vitest';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

const fixture = vi.hoisted(() => ({ mode: '', children: [], guardReads: [] }));
vi.mock('node:child_process', async () => {
  const { EventEmitter } = await import('node:events');
  const { PassThrough } = await import('node:stream');
  const execFile = vi.fn();
  execFile[Symbol.for('nodejs.util.promisify.custom')] = async (_command, args) => ({
    stdout: `Build ID: ${
      fixture.mode === 'unknown'
        ? '0'.repeat(40)
        : args[1] === '/usr/bin/WebKitWebDriver'
          ? '6d666ac8741bb707b84b83af8c45b5aeda575aec'
          : '6d0db877d1bda65539bf0c84fd6f72773b56ed4b'
    }`,
    stderr: '',
  });
  return {
    execFile,
    spawn: vi.fn((command) => {
      const child = Object.assign(new EventEmitter(), {
        pid: 44000 + fixture.children.length,
        command,
        stdout: new PassThrough(),
        stderr: new PassThrough(),
        exitCode: null,
        signalCode: null,
      });
      const finish = (code, signal) => {
        child.exitCode = code;
        child.signalCode = signal;
        child.stdout.end();
        child.stderr.end();
        child.emit('close', code, signal);
      };
      child.kill = vi.fn((signal) => {
        queueMicrotask(() => finish(0, signal));
        return true;
      });
      let scheduled = false;
      child.stderr.on('newListener', (event) => {
        if (event !== 'data' || scheduled) return;
        scheduled = true;
        setImmediate(() => {
          if (command === 'tcpdump') child.stderr.write('listening on lo\n');
          else child.emit('error', new Error('controlled bpftrace spawn EACCES'));
        });
      });
      fixture.children.push(child);
      return child;
    }),
  };
});
vi.mock('node:fs/promises', async (original) => {
  const actual = await original();
  const guards = new Map([
    [0x4537b, '488d75c0'],
    [0x4597f, '488b7db0'],
    [0x3bf98, '488d7d80'],
    [0x3bfce, 'e82dddfdff'],
    [0x71e1f0, 'f30f1efa'],
    [0x7272a0, 'f30f1efa'],
    [0x1004940, 'f30f1efa'],
    [0x1004fc5, 'e8065070ff'],
    [0x1005cf0, 'f30f1efa'],
  ]);
  return {
    ...actual,
    readlink: async (file) =>
      String(file).startsWith('/proc/') ? '/usr/bin/WebKitWebDriver' : actual.readlink(file),
    realpath: async (file) => file,
    readdir: async (file, ...args) =>
      String(file).startsWith('/proc/') ? [String(process.pid)] : actual.readdir(file, ...args),
    readFile: async (file, ...args) => {
      if (!String(file).startsWith('/proc/')) return actual.readFile(file, ...args);
      if (file.endsWith('/stat')) {
        const fields = Array.from({ length: 22 }, () => '0');
        fields[0] = 'S';
        fields[19] = '71';
        return `${process.pid} (owned fixture) ${fields.join(' ')}`;
      }
      return file.endsWith('/wchan') ? 'ep_poll' : 'owned mappings';
    },
    open: async (file) => ({
      read: async (buffer, offset, length, position) => {
        fixture.guardReads.push([file, position]);
        const expected = guards.get(position);
        if (!expected) throw new Error(`Unexpected guard read: ${position}`);
        Buffer.from(expected, 'hex').copy(buffer, offset);
        return { bytesRead: length, buffer };
      },
      close: async () => {},
    }),
  };
});
import { readBoundaryEvidence, startEvidence } from '../../tests/desktop/support/evidence.mjs';

const cleanups = [];
const originalGetuid = Object.getOwnPropertyDescriptor(process, 'getuid');
afterEach(async () => {
  for (const cleanup of cleanups.splice(0).reverse()) await cleanup();
  if (originalGetuid) Object.defineProperty(process, 'getuid', originalGetuid);
  else delete process.getuid;
});

test.each(['unknown', 'startup-failure'])(
  '%s retains explicit degradation and remaining evidence without a desktop',
  async (mode) => {
    Object.defineProperty(process, 'getuid', { configurable: true, value: () => 0 });
    fixture.mode = mode;
    fixture.children.length = 0;
    fixture.guardReads.length = 0;
    const directory = await mkdtemp(path.join(os.tmpdir(), '311-evidence-'));
    cleanups.push(() => rm(directory, { recursive: true, force: true }));
    const exitListeners = process.listenerCount('exit');
    const evidence = await startEvidence(
      directory,
      { port: 31001, nativePort: 31002 },
      async () => [{ pid: process.pid, name: 'WebKitWebDriver', identity: '71' }],
    );
    try {
      await evidence.attach();
      await evidence.record({ id: 9, evaluation: 23, phase: 'started' });
      const response = await fetch(evidence.endpoint, {
        method: 'POST',
        body: JSON.stringify({ id: 23, stage: 'entered' }),
      });
      expect(response.status).toBe(204);
      await evidence.record({
        id: 9,
        evaluation: 23,
        phase: 'failed',
        error: 'TimeoutError: controlled receipt timeout',
      });
    } finally {
      await evidence.close();
    }
    const { status, events } = await readBoundaryEvidence(directory);
    expect(status.stage).toBe('unavailable');
    expect(events).toEqual([]);
    await expect(
      readFile(path.join(directory, 'webkit-boundaries.stdout.log')),
    ).rejects.toMatchObject({ code: 'ENOENT' });
    if (mode === 'unknown') {
      expect(status.error).toContain('Unsupported ELF identity: /usr/bin/WebKitWebDriver');
      expect(fixture.guardReads).toEqual([]);
      expect(fixture.children.map((child) => child.command)).toEqual(['tcpdump']);
    } else {
      expect(status.error).toContain('controlled bpftrace spawn EACCES');
      expect(fixture.guardReads).toHaveLength(9);
      expect(fixture.children.map((child) => child.command)).toEqual(['tcpdump', 'bpftrace']);
    }
    const records = async (file) =>
      (await readFile(path.join(directory, file), 'utf8')).trim().split('\n').map(JSON.parse);
    expect(
      (await records('request-intervals.jsonl')).map((row) => [row.id, row.phase, row.error]),
    ).toEqual([
      [9, 'started', undefined],
      [9, 'failed', 'TimeoutError: controlled receipt timeout'],
    ]);
    expect((await records('page-lifecycle.jsonl'))[0].sample).toEqual({ id: 23, stage: 'entered' });
    expect(
      (await records('interval-threads.jsonl')).some((row) => row.current?.identity === '71'),
    ).toBe(true);
    const closed = (await records('probe-status.jsonl')).filter((row) => row.stage === 'closed');
    expect(closed.map((row) => row.name).sort()).toEqual(
      fixture.children
        .map((child) => (child.command === 'bpftrace' ? 'webkit-boundaries' : 'tcpdump'))
        .sort(),
    );
    expect(fixture.children.every((child) => child.exitCode !== null)).toBe(true);
    expect(process.listenerCount('exit')).toBe(exitListeners);
    expect(
      JSON.parse(await readFile(path.join(directory, 'probe-capture.json'), 'utf8')).errors,
    ).toEqual([]);
  },
);
