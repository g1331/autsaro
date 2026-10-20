import { expect, test, vi } from 'vitest';
const nativeInvoke = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: nativeInvoke }));
import { invoke } from '../src/ipc';

test('typed invocation retains native function identity, payload bytes and Promise', () => {
  const payload = { fingerprint: 'revision', sourceId: 'source' };
  const promise = Promise.resolve({ value: 'source bytes', inputFingerprint: 'revision' });
  nativeInvoke.mockReturnValueOnce(promise);
  expect(invoke).toBe(nativeInvoke);
  expect(invoke('read_project_source', payload)).toBe(promise);
  expect(nativeInvoke).toHaveBeenCalledExactlyOnceWith('read_project_source', payload);
  expect(nativeInvoke.mock.calls[0][1]).toBe(payload);
  expect(JSON.stringify(nativeInvoke.mock.calls[0][1])).toBe(
    '{"fingerprint":"revision","sourceId":"source"}',
  );
});
