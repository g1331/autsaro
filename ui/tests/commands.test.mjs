// @vitest-environment jsdom
import React from 'react';
import { act, cleanup, fireEvent, render, within } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { message, previewLanguage, translate } from '../src/i18n';
import { initialState } from '../src/workbench/state';
import { WorkbenchMenubar } from '../src/workbench/menu';
import { withCommands } from './support/commands.mjs';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => false, invoke: vi.fn() }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    onCloseRequested: async () => () => {},
    onResized: async () => () => {},
    isMaximized: async () => false,
    destroy: vi.fn(),
  }),
}));
let controller;
vi.mock('../src/workbench/useWorkbench', () => ({ useWorkbench: () => controller }));
const { default: App } = await import('../src/App');

const actions = [
  'setNotice',
  'replaceProject',
  'openProjectEntry',
  'openMemberProject',
  'importHandoff',
  'importHandoffDirectory',
  'requestSave',
  'startProject',
  'setSettingsOpen',
  'setSettingsCategory',
  'applyCurrentDrafts',
  'restoreAllDrafts',
  'setTreeVisible',
  'setInspectorVisible',
  'setInspectorTab',
  'openDocument',
  'closeDocument',
  'readSource',
  'setToolWindow',
  'inspectIntegration',
  'validateProject',
  'previewApplicationInitialization',
  'chooseDirectory',
  'changeEcuOutput',
  'changeEcuBuildDirectory',
  'setEcuImportDirectory',
  'previewEcu',
  'generateEcu',
  'preflightEcu',
  'buildEcu',
  'verifyEcu',
  'cancelOperation',
  'prepareChanges',
  'discardChanges',
  'importDefinitionCatalog',
  'verificationOwnedFailure',
  'setTreeMode',
  'setTreeFilter',
  'setTreeRevealId',
  'setGenerationKind',
  'changeTarget',
];
const compilerMissing = message('backend.workbench.compiler_required');

/** The controller `useWorkbench` would return: a saved standard ECU project on the delivery page. */
function workbench(overrides = {}) {
  const { workspace, capabilities, ...rest } = overrides;
  controller = withCommands({
    ...initialState(),
    ...Object.fromEntries(actions.map((name) => [name, vi.fn()])),
    guardContext: vi.fn((_title, run) => run()),
    native: true,
    disabled: false,
    unapplied: false,
    diagnostics: [],
    issues: [],
    problemCount: 0,
    errorCount: 0,
    capabilities: {
      nativeExecution: true,
      toolError: compilerMissing,
      ruleError: null,
      actions: [],
      environmentOverrides: [],
      verificationMode: false,
      ...capabilities,
    },
    executionReason:
      capabilities && 'toolError' in capabilities
        ? (capabilities.toolError ?? '')
        : compilerMissing,
    workspace: {
      name: 'Ecu',
      dirty: false,
      files: [],
      issues: [],
      integrationCandidate: true,
      applicationInitialized: true,
      ...workspace,
    },
    tabs: [{ kind: 'delivery' }],
    activeDocument: { kind: 'delivery' },
    treeVisible: false,
    inspectorVisible: false,
    ...rest,
  });
  return controller;
}

beforeEach(() => {
  previewLanguage('zh-CN');
  vi.stubGlobal('matchMedia', () => ({
    matches: false,
    addEventListener() {},
    removeEventListener() {},
  }));
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  previewLanguage('system');
});

const describedText = (element) =>
  element
    .getAttribute('aria-describedby')
    ?.split(' ')
    .map((id) => document.getElementById(id)?.textContent ?? '')
    .join(' ');
const page = () => within(document.querySelector('[data-testid="ecu-delivery"]'));
const toolWindow = () => within(document.querySelector('.tool-content'));
const trigger = (name) =>
  document.querySelector(`[role="menubar"]`).querySelectorAll('button')[
    ['file', 'edit', 'view', 'tools', 'help'].indexOf(name)
  ];
function openMenu(name) {
  const button = trigger(name);
  act(() => button.focus());
  fireEvent.keyDown(button, { key: 'ArrowDown' });
  return within(document.querySelector('[role="menu"]'));
}

test('page, tool window and menu show one reason for an unavailable command, and its remedy (A3, A4, A10)', () => {
  workbench({
    toolWindow: 'build',
    generated: { outputDirectory: '/out', files: [] },
    stages: { ...initialState().stages, generate: { state: 'done', detail: '' } },
    buildDirectory: '/build',
  });
  render(React.createElement(App));
  const reason = translate('backend.workbench.compiler_required');
  expect(reason).toBe('未配置编译器');
  const entries = [
    page().getByRole('button', { name: translate('action.build') }),
    toolWindow().getByRole('button', { name: translate('action.build') }),
    openMenu('tools').getByRole('menuitem', { name: translate('action.build') }),
  ];
  for (const entry of entries) {
    expect(entry.getAttribute('aria-disabled')).toBe('true');
    expect(describedText(entry)).toBe(reason);
  }
  fireEvent.keyDown(document.activeElement, { key: 'Escape' });
  fireEvent.click(entries[0]);
  expect(controller.buildEcu).not.toHaveBeenCalled();
  // The reason is visible beside the group with the entry that resolves it.
  const row = page().getByText(reason).closest('li');
  fireEvent.click(within(row).getByRole('button', { name: translate('action.executionTools') }));
  expect(controller.setSettingsCategory).toHaveBeenCalledWith('tools');
  expect(controller.setSettingsOpen).toHaveBeenCalledWith(true);
});

test('an available command runs from the page and the tool window alike', () => {
  workbench({
    capabilities: { toolError: null },
    toolWindow: 'host',
    built: { binaryPath: '/build/ecu', log: '' },
    stages: { ...initialState().stages, build: { state: 'done', detail: '' } },
  });
  render(React.createElement(App));
  for (const button of [
    page().getByRole('button', { name: translate('action.verifyHost') }),
    toolWindow().getByRole('button', { name: translate('action.verifyHost') }),
  ]) {
    expect(button.hasAttribute('aria-disabled')).toBe(false);
    fireEvent.click(button);
  }
  expect(controller.verifyEcu).toHaveBeenCalledTimes(2);
});

test('the toolbar delivery button is disabled with the workspace reason', () => {
  workbench({ disabled: true, capabilities: { ruleError: 'rules unavailable' } });
  render(React.createElement(App));
  const toolbar = within(document.querySelector('.context-toolbar'));
  const delivery = toolbar.getByRole('button', { name: translate('shell.document.delivery') });
  expect(delivery.getAttribute('aria-disabled')).toBe('true');
  expect(describedText(delivery)).toBe('rules unavailable');
  fireEvent.click(delivery);
  expect(controller.openDocument).not.toHaveBeenCalled();
  cleanup();
  // While busy, the toolbar's own operation status is the reason.
  workbench({ disabled: true, busy: message('activity.build') });
  render(React.createElement(App));
  const busy = within(document.querySelector('.context-toolbar')).getByRole('button', {
    name: translate('shell.document.delivery'),
  });
  expect(describedText(busy)).toBe(
    translate('shell.busy', { operation: translate('activity.build') }),
  );
});

test.each(['zh-CN', 'en'])(
  'ECU delivery explains an uninitialized application with the APPLICATION_UNINITIALIZED remedy (#111, %s)',
  (language) => {
    previewLanguage(language);
    workbench({ workspace: { applicationInitialized: false }, ecuOutputDirectory: '/out' });
    render(React.createElement(App));
    const remedy = translate('backend.prepared.application_initialization_required');
    const preview = page().getByRole('button', { name: translate('action.previewSource') });
    const generate = page().getByRole('button', { name: translate('action.confirmGenerate') });
    for (const button of [preview, generate]) {
      expect(button.getAttribute('aria-disabled')).toBe('true');
      expect(describedText(button)).toBe(remedy);
    }
    fireEvent.click(preview);
    expect(controller.previewEcu).not.toHaveBeenCalled();
    const item = openMenu('tools').getByRole('menuitem', {
      name: translate('action.previewSource'),
    });
    expect(describedText(item)).toBe(remedy);
    fireEvent.keyDown(document.activeElement, { key: 'Escape' });
    const row = page().getByText(remedy).closest('li');
    fireEvent.click(
      within(row).getByRole('button', { name: translate('action.initializeApplication') }),
    );
    expect(controller.previewApplicationInitialization).toHaveBeenCalled();
  },
);

test('an initialized application can be previewed', () => {
  workbench({ ecuOutputDirectory: '/out' });
  render(React.createElement(App));
  const preview = page().getByRole('button', { name: translate('action.previewSource') });
  expect(preview.hasAttribute('aria-disabled')).toBe(false);
  fireEvent.click(preview);
  expect(controller.previewEcu).toHaveBeenCalled();
});

test('global shortcuts leave text fields alone and use the physical key', () => {
  workbench({ toolWindow: 'build' });
  render(React.createElement(App));
  const field = page().getByLabelText(translate('shell.delivery.output'));
  act(() => field.focus());
  for (const code of ['KeyK', 'KeyS', 'KeyO'])
    expect(fireEvent.keyDown(field, { key: code.at(-1).toLowerCase(), code, ctrlKey: true })).toBe(
      true,
    );
  expect(fireEvent.keyDown(field, { key: 'Escape', code: 'Escape' })).toBe(true);
  expect(document.querySelector('[role="dialog"]')).toBeNull();
  expect(controller.requestSave).not.toHaveBeenCalled();
  expect(controller.setToolWindow).not.toHaveBeenCalled();

  act(() => field.blur());
  fireEvent.keyDown(document.body, { key: 'Escape', code: 'Escape' });
  expect(controller.setToolWindow).toHaveBeenCalledWith(null);
  fireEvent.keyDown(document.body, { key: 'ы', code: 'KeyS', ctrlKey: true });
  expect(controller.requestSave).toHaveBeenCalled();
  fireEvent.keyDown(document.body, { key: 'л', code: 'KeyK', ctrlKey: true });
  expect(document.querySelector('[role="dialog"]')).not.toBeNull();
});

test('menus open with ↓, move with ←→, close with Tab, and focus unavailable items with their reason (I7)', async () => {
  workbench();
  render(React.createElement(App));
  const file = openMenu('file');
  expect(document.querySelector('[role="menu"]').getAttribute('aria-labelledby')).toBe(
    trigger('file').id,
  );
  expect(file.getByRole('menuitem', { name: translate('action.newProject') })).toBeTruthy();
  fireEvent.keyDown(document.activeElement, { key: 'ArrowRight' });
  const menu = document.querySelector('[role="menu"]');
  expect(menu.getAttribute('aria-labelledby')).toBe(trigger('edit').id);
  // Every Edit item is unavailable without drafts, yet ↓ moves focus onto them and their reason.
  fireEvent.keyDown(document.activeElement, { key: 'ArrowDown' });
  const edit = within(menu);
  const apply = edit.getByRole('menuitem', { name: translate('action.apply') });
  expect(document.activeElement).toBe(apply);
  expect(apply.getAttribute('aria-disabled')).toBe('true');
  expect(describedText(apply)).toBe(translate('shell.reason.noChanges'));
  fireEvent.keyDown(apply, { key: 'ArrowDown' });
  // Radix moves roving focus on the next task.
  await act(() => new Promise((resolve) => setTimeout(resolve)));
  expect(document.activeElement.textContent).toContain(translate('action.revert'));
  fireEvent.keyDown(document.activeElement, { key: 'Enter' });
  expect(controller.restoreAllDrafts).not.toHaveBeenCalled();
  expect(document.querySelector('[role="menu"]')).not.toBeNull();
  fireEvent.keyDown(document.activeElement, { key: 'Tab' });
  expect(document.querySelector('[role="menu"]')).toBeNull();

  openMenu('file');
  fireEvent.keyDown(document.activeElement, { key: 'End' });
  await act(() => new Promise((resolve) => setTimeout(resolve)));
  fireEvent.keyDown(document.activeElement, { key: 'Enter' });
  expect(controller.setSettingsOpen).toHaveBeenCalledWith(true);
});

test('menus separate groups and show a reason shared within a group once at its end', () => {
  const command = (id, group, reason = '') => ({ id, label: id, group, reason, run: () => {} });
  render(
    React.createElement(WorkbenchMenubar, {
      label: 'Menu',
      run: vi.fn(),
      commands: [
        command('new', 'open'),
        command('save', 'save', 'Open a project first'),
        command('save-as', 'save', 'Open a project first'),
        command('close', 'close', 'Open a project first'),
        command('settings', 'settings'),
      ].map((entry) => ({ ...entry, menu: 'file' })),
    }),
  );
  openMenu('file');
  const rows = [...document.querySelector('[role="menu"]').children].map((node) =>
    node.getAttribute('role') === 'separator'
      ? '---'
      : node.tagName === 'SMALL'
        ? `(${node.textContent})`
        : node.textContent,
  );
  expect(rows).toEqual([
    'new',
    '---',
    'save',
    'save-as',
    '(Open a project first)',
    '---',
    'close',
    '(Open a project first)',
    '---',
    'settings',
  ]);
});
