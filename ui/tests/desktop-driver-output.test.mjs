import { afterEach, expect, test, vi } from 'vitest';
import { PassThrough } from 'node:stream';
import path from 'node:path';

const state = vi.hoisted(() => ({ append: vi.fn() }));
vi.mock('node:fs/promises', async (original) => ({
  ...(await original()),
  appendFile: (...args) => state.append(...args),
}));
import { captureDriverOutput } from '../../tests/desktop/support/driver.mjs';

afterEach(() => vi.resetAllMocks());

const streams = () => ({ stdout: new PassThrough(), stderr: new PassThrough() });

test('driver output keeps split records and UTF-8 bytes in order while a write is pending', async () => {
  const driver = streams();
  const writes = [];
  const observed = [];
  let release;
  let entered;
  let stdoutWritten;
  const blocked = new Promise((resolve) => (release = resolve));
  const firstWrite = new Promise((resolve) => (entered = resolve));
  const otherStream = new Promise((resolve) => (stdoutWritten = resolve));
  let stderrCalls = 0;
  state.append.mockImplementation(async (file, chunk) => {
    if (path.basename(file) === 'driver.stderr.log' && ++stderrCalls === 1) {
      entered();
      await blocked;
    }
    writes.push({ file: path.basename(file), chunk: Buffer.from(chunk) });
    if (path.basename(file) === 'driver.stdout.log') stdoutWritten();
  });
  const complete = captureDriverOutput(driver, '/evidence', (chunk) => observed.push(chunk));
  const record = Buffer.from('AUTOSAR_NATIVE_REQUEST {"id":1,"stage":"returned","text":"项目"}\n');
  const split = record.indexOf(Buffer.from('项目')) + 1;
  driver.stderr.write(record.subarray(0, split));
  await firstWrite;
  driver.stderr.end(record.subarray(split));
  driver.stdout.end('driver ready\n');
  await otherStream;
  try {
    expect(stderrCalls).toBe(1);
  } finally {
    release();
  }
  await expect(complete).resolves.toEqual([null, null]);
  const bytes = (file) =>
    Buffer.concat(writes.filter((write) => write.file === file).map((write) => write.chunk));
  expect(bytes('driver.stderr.log')).toEqual(record);
  expect(Buffer.concat(observed)).toEqual(record);
  expect(bytes('driver.stdout.log').toString()).toBe('driver ready\n');
});

test('driver output retains a file write failure while consuming the remaining stream', async () => {
  const driver = streams();
  const failure = Object.assign(new Error('capture denied'), { code: 'EACCES' });
  state.append.mockRejectedValue(failure);
  const complete = captureDriverOutput(driver, '/evidence');
  driver.stdout.end();
  driver.stderr.end('native command returned\n');
  await expect(complete).resolves.toEqual([null, failure]);
});

test('driver output retains a source stream failure', async () => {
  const driver = streams();
  const failure = new Error('driver output stream failed');
  const complete = captureDriverOutput(driver, '/evidence');
  driver.stdout.end();
  driver.stderr.destroy(failure);
  await expect(complete).resolves.toEqual([null, failure]);
});
