// @vitest-environment jsdom
import React from 'react';
import { act, cleanup, fireEvent, render, renderHook, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { i18n, message, previewLanguage, translate } from '../src/i18n';
import { useWorkbench } from '../src/workbench/useWorkbench';
import { SettingsPage } from '../src/pages/SettingsPage';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  setTitle: vi.fn().mockResolvedValue(undefined),
}));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ setTitle: mocks.setTitle }),
}));

const capabilities = {
  fingerprint: 'project-input',
  target: 'windows-x64-controlled-v1',
  targets: ['windows-x64-controlled-v1'],
  nativeExecution: false,
  hasWorkspace: false,
  xsdArchive: null,
  modArchive: null,
  resourceError: null,
  executionTools: null,
  configuredExecutionTools: null,
  toolError: null,
  environmentOverrides: [],
  operation: null,
  ruleSetIdentity: null,
  ruleError: null,
  ruleCoverage: [],
  definitionFingerprint: 'definitions',
  appearance: 'system',
  actions: [],
  verificationMode: false,
};

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  localStorage.clear();
  previewLanguage('system');
});

async function controller(language = 'zh-CN', rejectSave = false) {
  mocks.invoke.mockImplementation(async (command) => {
    if (command === 'workbench_capabilities') return { ...capabilities, language };
    if (command === 'configure_language') {
      if (rejectSave) throw message('controller.error.name');
      return {
        value: null,
        capabilities: { ...capabilities, language: 'en' },
        inputFingerprint: capabilities.fingerprint,
      };
    }
    throw new Error(`Unexpected command: ${command}`);
  });
  let hook;
  await act(async () => {
    hook = renderHook(() => useWorkbench());
  });
  return hook;
}

test('old desktop settings resolve to system; explicit saved preference is restored', async () => {
  mocks.invoke.mockImplementation(async () => capabilities);
  let legacy;
  await act(async () => {
    legacy = renderHook(() => useWorkbench());
  });
  expect(legacy.result.current.savedLanguage).toBe('system');
  legacy.unmount();
  const saved = await controller('en');
  expect(saved.result.current.savedLanguage).toBe('en');
  expect(saved.result.current.languageDraft).toBe('en');
});

test('desktop save uses configure_language and preserves drafts, previews and owned results', async () => {
  const { result } = await controller();
  act(() => {
    result.current.setProjectName('Raw 用户名');
    result.current.setFrameInput({ ...result.current.frameInput, name: 'Draft', id: '321' });
    result.current.setGenerationPreview({
      revision: 'owned',
      files: [],
      outputDirectory: 'C:\\output',
    });
    result.current.setVirtualResult({
      passed: true,
      log: [
        message('controller.integration.validated'),
        ['外部证据', [message('controller.error.staleDelivery')]],
      ],
      events: [
        message('controller.integration.validated'),
        ['machine-event', message('controller.error.staleDelivery')],
      ],
    });
    result.current.setLanguageDraft('en');
  });
  const before = result.current;
  await act(async () => {
    await result.current.configureLanguage();
  });
  expect(mocks.invoke).toHaveBeenCalledWith('configure_language', {
    fingerprint: 'project-input',
    language: 'en',
  });
  expect(result.current.savedLanguage).toBe('en');
  expect(result.current.frameInput).toBe(before.frameInput);
  expect(result.current.generationPreview).toBe(before.generationPreview);
  expect(result.current.virtualResult).toBe(before.virtualResult);
  expect(result.current.stages).toBe(before.stages);
  expect(result.current.operationGeneration).toBe(before.operationGeneration);
  expect(result.current.projectName).toBe('Raw 用户名');
  expect(result.current.capabilities.fingerprint).toBe('project-input');
  expect(mocks.setTitle).toHaveBeenCalled();
});

test('rejected desktop language persistence retains saved preference and preview ownership', async () => {
  const { result } = await controller('zh-CN', true);
  act(() => {
    result.current.setGenerationPreview({
      revision: 'owned',
      files: [],
      outputDirectory: 'C:\\output',
    });
    result.current.setLanguageDraft('en');
  });
  const before = result.current;
  await act(async () => {
    await result.current.configureLanguage();
  });
  expect(result.current.savedLanguage).toBe('zh-CN');
  expect(result.current.languageDraft).toBe('en');
  expect(result.current.generationPreview).toBe(before.generationPreview);
  expect(result.current.operationGeneration).toBe(before.operationGeneration);
  expect(result.current.settingsNotice).toEqual(message('controller.error.name'));
  act(() => result.current.setLanguageDraft(result.current.savedLanguage));
  expect(result.current.languageDraft).toBe('zh-CN');
});

test.each(['success', 'failure'])(
  'language save %s cannot race Close or Escape in the settings consumer',
  async (outcome) => {
    let complete;
    let reject;
    const pending = new Promise((resolve, rejectPromise) => {
      complete = resolve;
      reject = rejectPromise;
    });
    mocks.invoke.mockImplementation(async (command) => {
      if (command === 'workbench_capabilities') return { ...capabilities, language: 'zh-CN' };
      if (command === 'configure_language') return pending;
      throw new Error(`Unexpected command: ${command}`);
    });
    let current;
    function SettingsConsumer() {
      current = useWorkbench();
      return React.createElement(SettingsPage, { controller: current });
    }
    const view = render(React.createElement(SettingsConsumer));
    await waitFor(() => expect(current.capabilities).not.toBeNull());
    act(() => {
      current.setSettingsOpen(true);
      current.setFrameInput({ ...current.frameInput, name: 'RetainedDraft', id: '321' });
      current.setVirtualResult({ passed: true, log: 'external tool evidence', events: [] });
    });
    const frameDraft = current.frameInput;
    const ownedResult = current.virtualResult;
    const stages = current.stages;
    const generation = current.operationGeneration;
    fireEvent.change(view.getByRole('combobox', { name: translate('shell.settings.language') }), {
      target: { value: 'en' },
    });
    expect(i18n.language).toBe('en');
    fireEvent.click(view.getByRole('button', { name: translate('shell.settings.saveLanguage') }));
    const close = view.getByRole('button', { name: translate('shell.settings.close') });
    expect(close.disabled).toBe(true);
    expect(
      view.getByRole('combobox', { name: translate('shell.settings.language') }).disabled,
    ).toBe(true);
    expect(
      view.getByRole('button', { name: translate('shell.settings.saveLanguage') }).disabled,
    ).toBe(true);
    fireEvent.click(close);
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(current.settingsOpen).toBe(true);
    expect(current.savedLanguage).toBe('zh-CN');
    expect(current.languageDraft).toBe('en');
    expect(current.busy).toBeNull();
    await act(async () => {
      if (outcome === 'success')
        complete({
          value: null,
          capabilities: { ...capabilities, language: 'en' },
          inputFingerprint: capabilities.fingerprint,
        });
      else reject(message('controller.error.name'));
    });
    expect(current.languageSaving).toBe(false);
    expect(current.savedLanguage).toBe(outcome === 'success' ? 'en' : 'zh-CN');
    expect(current.languageDraft).toBe('en');
    expect(current.frameInput).toBe(frameDraft);
    expect(current.virtualResult).toBe(ownedResult);
    expect(current.stages).toBe(stages);
    expect(current.operationGeneration).toBe(generation);
    expect(view.getByRole('status').textContent).not.toBe('');
    if (outcome === 'failure')
      expect(view.getByRole('status').textContent).toBe(translate('controller.error.name'));
    // Disabled focused controls may leave native focus on the document body.
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(current.settingsOpen).toBe(false);
    expect(current.languageDraft).toBe(current.savedLanguage);
    expect(i18n.language).toBe(outcome === 'success' ? 'en' : 'zh-CN');
    act(() => current.setSettingsOpen(true));
    const unsavedPreview = outcome === 'success' ? 'zh-CN' : 'en';
    fireEvent.change(view.getByRole('combobox', { name: translate('shell.settings.language') }), {
      target: { value: unsavedPreview },
    });
    fireEvent.keyDown(view.getByRole('dialog'), { key: 'Escape' });
    expect(current.languageDraft).toBe(current.savedLanguage);
    expect(i18n.language).toBe(outcome === 'success' ? 'en' : 'zh-CN');
  },
);
