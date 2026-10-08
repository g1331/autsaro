// @vitest-environment jsdom
import React from 'react';
import { act, cleanup, fireEvent, render, renderHook } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import {
  composedMessage,
  i18n,
  isMessage,
  localize,
  message,
  previewLanguage,
  readLanguagePreference,
  resolveLanguage,
  resources,
  translate,
} from '../src/i18n';
import { errorText, intInRange } from '../src/workbench/forms';
import { useWorkbench } from '../src/workbench/useWorkbench';
import { SettingsPage } from '../src/pages/SettingsPage';
import { createSettingsActions } from '../src/workbench/settingsActions';
import { initialState } from '../src/workbench/state';
import { OwnedLog } from '../src/workbench/Dialog';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => false, invoke: vi.fn() }));

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  localStorage.clear();
  previewLanguage('system');
});

test('system resolution and old browser settings use the documented language rule', () => {
  expect(resolveLanguage('system', 'zh-TW')).toBe('zh-CN');
  expect(resolveLanguage('system', 'zh-Hans-CN')).toBe('zh-CN');
  expect(resolveLanguage('system', 'de-DE')).toBe('en');
  expect(resolveLanguage('zh-CN', 'en-US')).toBe('zh-CN');
  expect(readLanguagePreference()).toBe('system');
  localStorage.setItem('autsaro.language', 'en');
  expect(readLanguagePreference()).toBe('en');
});

test('both offline catalogs contain every key and identical interpolation contracts', () => {
  const chinese = resources['zh-CN'].translation;
  const english = resources.en.translation;
  expect(Object.keys(chinese).sort()).toEqual(Object.keys(english).sort());
  for (const key of Object.keys(chinese)) {
    const parameters = (value) =>
      [...value.matchAll(/{{\s*([^},]+)(?:,[^}]*)?\s*}}/g)].map((match) => match[1].trim()).sort();
    expect(parameters(chinese[key]), key).toEqual(parameters(english[key]));
    expect(chinese[key], key).not.toBe('');
    expect(english[key], key).not.toBe('');
  }
});

test('count-bearing product messages select plural forms in the current language', () => {
  const item = message('shell.build.files', { count: 1 });
  const items = message('shell.build.files', { count: 2 });
  previewLanguage('en');
  expect(localize(item)).toContain('1');
  expect(localize(items)).toContain('2');
  expect(localize(item)).not.toBe(localize(items));
  previewLanguage('zh-CN');
  expect(localize(item)).toContain('1');
  expect(localize(items)).toContain('2');
});

test('product failures remain descriptors and retranslate; raw evidence never changes', () => {
  let failure;
  try {
    intInRange('9', message('controller.field.signalLength'), 1, 8);
  } catch (error) {
    failure = errorText(error);
  }
  const raw = 'external.exe: 原始证据 <raw> C:\\用户\\test.arxml';
  const combined = composedMessage('controller.operation.failed', {
    label: message('controller.integration.inspect'),
    error: errorText([failure, raw]),
  });
  previewLanguage('zh-CN');
  const chinese = localize(combined);
  previewLanguage('en');
  const english = localize(combined);
  expect(chinese).not.toBe(english);
  expect(chinese).toContain(raw);
  expect(english).toContain(raw);
  expect(english).toContain('1');
  expect(english).toContain('8');
  expect(localize(raw)).toBe(raw);
  expect(() => translate('missing.product.key')).toThrow(/Missing localization key/);
});
test('recursive wire aggregates render and retranslate in a log consumer and recoverable errors', () => {
  const raw = 'tool.exe: 原始日志 C:\\用户\\input.arxml';
  const log = [
    message('controller.integration.validated'),
    [raw, [message('controller.error.staleDelivery')]],
  ];
  const recoverable = { error: [message('controller.error.name'), [raw, log]] };
  expect(errorText(recoverable.error)).toEqual(recoverable.error);
  previewLanguage('zh-CN');
  const view = render(
    React.createElement(OwnedLog, { text: log, label: message('controller.integration.inspect') }),
  );
  const element = view.container.querySelector('pre');
  const chinese = element.textContent;
  expect(chinese).toBe(localize(log));
  act(() => previewLanguage('en'));
  expect(view.container.querySelector('pre')).toBe(element);
  expect(element.textContent).toBe(localize(log));
  expect(element.textContent).not.toBe(chinese);
  expect(element.textContent).toContain(raw);
  expect(localize(recoverable.error)).toContain(raw);
});

test('wire message parameters accept only the shared scalar parameter contract', () => {
  expect(
    isMessage({
      key: 'product.key',
      params: { text: 'raw', count: 2, enabled: true, absent: null },
    }),
  ).toBe(true);
  expect(isMessage({ key: 'product.key', params: [] })).toBe(false);
  expect(isMessage({ key: 'product.key', params: { nested: {} } })).toBe(false);
  expect(isMessage({ key: 'product.key', params: { nested: [] } })).toBe(false);
});

test('browser language settings preview, cancel and save preserve controller drafts', async () => {
  previewLanguage('zh-CN');
  const { result } = renderHook(() => useWorkbench());
  act(() => {
    result.current.setSettingsOpen(true);
    result.current.setProjectName('用户对象_Example');
    result.current.setFrameInput({ ...result.current.frameInput, name: 'DraftFrame', id: '321' });
  });
  const frameDraft = result.current.frameInput;
  const settings = render(React.createElement(SettingsPage, { controller: result.current }));
  const select = settings.container.querySelector('select[name="language"]');
  expect(select).not.toBeNull();
  act(() => fireEvent.change(select, { target: { value: 'en' } }));
  settings.rerender(React.createElement(SettingsPage, { controller: result.current }));
  expect(i18n.language).toBe('en');
  expect(result.current.projectName).toBe('用户对象_Example');
  expect(result.current.frameInput).toBe(frameDraft);
  expect(result.current.savedLanguage).toBe('system');
  const close = settings.getByRole('button', { name: /^(关闭|Close)$/ });
  act(() => fireEvent.click(close));
  expect(result.current.languageDraft).toBe(result.current.savedLanguage);
  act(() => {
    result.current.setSettingsOpen(true);
    result.current.setLanguageDraft('zh-CN');
  });
  await act(async () => result.current.configureLanguage());
  expect(result.current.savedLanguage).toBe('zh-CN');
  expect(readLanguagePreference()).toBe('zh-CN');
  expect(result.current.frameInput).toBe(frameDraft);
});

function sessionWithResults() {
  const stateRef = {
    current: {
      ...initialState(),
      savedLanguage: 'zh-CN',
      languageDraft: 'en',
      workspace: { dirty: true },
      draft: { kind: 'frame', fields: { id: '321' } },
      projection: { inputFingerprint: 'unchanged' },
      generated: { files: ['output.c'] },
      built: { binaryPath: 'host.exe' },
      virtualResult: { passed: true },
      generationPreview: { revision: 'owned' },
      changePreview: { changeRevision: 'draft-owned' },
    },
  };
  const session = {
    state: stateRef.current,
    stateRef,
    running: { current: false },
    epoch: { current: 4 },
    call: vi.fn(),
    patchState(patch) {
      stateRef.current = { ...stateRef.current, ...patch };
    },
    field(key) {
      return (value) =>
        session.patchState({
          [key]: typeof value === 'function' ? value(stateRef.current[key]) : value,
        });
    },
  };
  return session;
}

test('failed browser persistence retains saved preference and all project/result ownership', async () => {
  localStorage.setItem('autsaro.language', 'zh-CN');
  const current = sessionWithResults();
  const previous = current.stateRef.current;
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
    throw new Error('storage unavailable');
  });
  const actions = createSettingsActions(current, { invalidateOperation: vi.fn() });
  await actions.configureLanguage();
  expect(current.stateRef.current.savedLanguage).toBe('zh-CN');
  expect(localStorage.getItem('autsaro.language')).toBe('zh-CN');
  expect(current.stateRef.current.settingsNotice).not.toBe('');
  for (const key of [
    'workspace',
    'draft',
    'projection',
    'generated',
    'built',
    'virtualResult',
    'generationPreview',
    'changePreview',
    'stages',
  ]) {
    expect(current.stateRef.current[key]).toBe(previous[key]);
  }
  expect(current.epoch.current).toBe(4);
  expect(current.call).not.toHaveBeenCalled();
});
