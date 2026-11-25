// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { useWorkbench } from '../src/app/useWorkbench';
import { message, previewLanguage } from '../src/i18n';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: native.invoke }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ setTitle: vi.fn().mockResolvedValue(undefined) }),
}));

const capabilities = {
  fingerprint: 'original',
  definitionFingerprint: 'definitions',
  hasWorkspace: false,
  target: 'windows-x64-controlled-v1',
  appearance: 'system',
  language: 'system',
  actions: [],
};

function reply(inputFingerprint, accepted = capabilities) {
  return { value: null, inputFingerprint, capabilities: accepted };
}

async function controller() {
  native.invoke.mockResolvedValueOnce(capabilities);
  let hook;
  await act(async () => {
    hook = renderHook(() => useWorkbench());
  });
  return hook.result;
}

afterEach(() => {
  cleanup();
  native.invoke.mockReset();
  previewLanguage('system');
});

test('the composed controller updates form state without exposing workspace replacement setters', async () => {
  const result = await controller();
  expect(result.current.setWorkspace).toBeUndefined();
  expect(result.current.setCapabilities).toBeUndefined();
  act(() => result.current.setProjectName('Example'));
  expect(result.current.projectName).toBe('Example');
  expect(typeof result.current.applyChanges).toBe('function');
});

test('a reply from before cancellation cannot publish capabilities or release the next operation', async () => {
  const result = await controller();
  const previous = Promise.withResolvers();
  const next = Promise.withResolvers();
  const cancelled = { ...capabilities, fingerprint: 'after-cancellation' };
  native.invoke
    .mockReturnValueOnce(previous.promise)
    .mockResolvedValueOnce(reply(capabilities.fingerprint, cancelled))
    .mockReturnValueOnce(next.promise);

  let previousSave;
  act(() => {
    previousSave = result.current.configureAppearance();
  });
  await act(async () => result.current.cancelOperation());
  let nextSave;
  act(() => {
    nextSave = result.current.configureAppearance();
  });

  await act(async () => {
    previous.resolve(reply(capabilities.fingerprint, { ...capabilities, fingerprint: 'late' }));
    await previousSave;
  });
  expect(result.current.capabilities).toBe(cancelled);
  expect(result.current.busy).toEqual(message('activity.saveAppearance'));
  expect(result.current.operationLog.map((entry) => entry.command)).toEqual(['cancel_operation']);
  expect(native.invoke.mock.calls).toEqual([
    ['workbench_capabilities'],
    ['configure_appearance', { appearance: 'system', fingerprint: capabilities.fingerprint }],
    ['cancel_operation', { fingerprint: capabilities.fingerprint }],
    ['configure_appearance', { appearance: 'system', fingerprint: cancelled.fingerprint }],
  ]);

  await act(async () => {
    next.resolve(reply(cancelled.fingerprint, cancelled));
    await nextSave;
  });
  expect(result.current.busy).toBeNull();
  expect(result.current.operationLog.map((entry) => entry.command)).toEqual([
    'cancel_operation',
    'configure_appearance',
  ]);
});

test('an invalid echoed fingerprint rejects the reply and refreshes capabilities once', async () => {
  const result = await controller();
  native.invoke
    .mockResolvedValueOnce(reply('other-request', { ...capabilities, fingerprint: 'wrong' }))
    .mockResolvedValueOnce(capabilities);
  act(() => result.current.setLanguageDraft('en'));
  await act(async () => result.current.configureLanguage());

  expect(native.invoke.mock.calls).toEqual([
    ['workbench_capabilities'],
    ['configure_language', { language: 'en', fingerprint: capabilities.fingerprint }],
    ['workbench_capabilities'],
  ]);
  expect(result.current.capabilities).toBe(capabilities);
  expect(result.current.savedLanguage).toBe('system');
  expect(result.current.languageDraft).toBe('en');
  expect(result.current.settingsNotice).toEqual({
    tone: 'error',
    text: message('controller.error.staleDelivery'),
  });
});
