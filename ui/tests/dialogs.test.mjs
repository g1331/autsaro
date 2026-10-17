// @vitest-environment jsdom
import React from 'react';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { cleanup, fireEvent, render, within } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { message, previewLanguage, translate } from '../src/i18n';
import { CommandPalette } from '../src/workbench/CommandPalette';
import { Dialog } from '../src/workbench/Dialog';
import { PreviewDialogs } from '../src/workbench/PreviewDialogs';
import { IntegrationPanel } from '../src/IntegrationPanel';
import { SettingsPage } from '../src/pages/SettingsPage';
import { initialState } from '../src/workbench/state';
import { withCommands } from './support/commands.mjs';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => false, invoke: vi.fn() }));

afterEach(() => {
  cleanup();
  previewLanguage('system');
});

const commands = [
  { id: 'save', label: 'Preview save', reason: '', run: vi.fn() },
  { id: 'build', label: 'Build ECU', reason: 'Generate the ECU source first', run: vi.fn() },
  { id: 'open', label: 'Open project…', shortcut: 'Ctrl+O', reason: '', run: vi.fn() },
  { id: 'settings', label: 'Settings', reason: '', run: vi.fn() },
];

test('command palette lists unavailable commands with their reason, matches case-insensitively, moves with arrows, runs on Enter and shows an empty state', () => {
  previewLanguage('en');
  const onRun = vi.fn();
  const view = render(React.createElement(CommandPalette, { commands, onRun, onClose: vi.fn() }));
  const input = view.getByRole('combobox');
  expect(document.activeElement).toBe(input);
  fireEvent.change(input, { target: { value: 'preview' } });
  expect(view.getAllByRole('option').map((option) => option.textContent)).toEqual(['Preview save']);
  fireEvent.change(input, { target: { value: '' } });
  const build = view.getByRole('option', { name: /Build ECU/ });
  expect(build.getAttribute('aria-disabled')).toBe('true');
  expect(build.textContent).toContain('Generate the ECU source first');
  // The first available command is highlighted; ↓ skips the unavailable one, Enter runs.
  expect(view.getAllByRole('option')[0].getAttribute('aria-selected')).toBe('true');
  fireEvent.keyDown(input, { key: 'ArrowDown' });
  expect(input.getAttribute('aria-activedescendant')).toBe(view.getAllByRole('option')[2].id);
  fireEvent.keyDown(input, { key: 'Enter' });
  expect(onRun).toHaveBeenCalledWith('open');
  fireEvent.change(input, { target: { value: 'zzzz' } });
  expect(view.queryAllByRole('option')).toHaveLength(0);
  expect(view.getByText(translate('shell.app.noCommands'))).toBeTruthy();
});

test('dialog close is blocked while busy, says why and offers cancel', () => {
  previewLanguage('en');
  const onClose = vi.fn();
  const onCancelBusy = vi.fn();
  const props = { title: 'Preview', onClose, onCancelBusy };
  const view = render(
    React.createElement(Dialog, { ...props, busy: message('activity.save') }, 'Body'),
  );
  const close = view.getByRole('button', {
    name: translate('editor.dialog.closeLabel', { title: 'Preview' }),
  });
  expect(close.disabled).toBe(true);
  const reason = document.getElementById(close.getAttribute('aria-describedby'));
  expect(reason.textContent).toContain(
    translate('editor.dialog.closeBlocked', { operation: translate('activity.save') }),
  );
  fireEvent.keyDown(document.activeElement ?? document.body, { key: 'Escape' });
  expect(onClose).not.toHaveBeenCalled();
  fireEvent.click(
    within(reason).getByRole('button', { name: translate('action.cancelOperation') }),
  );
  expect(onCancelBusy).toHaveBeenCalled();
  view.rerender(React.createElement(Dialog, { ...props, busy: null }, 'Body'));
  expect(close.disabled).toBe(false);
  fireEvent.keyDown(document.activeElement ?? document.body, { key: 'Escape' });
  expect(onClose).toHaveBeenCalledTimes(1);
});

const preview = { path: 'out/Rte.c', before: null, after: 'int x;', status: 'added' };
const controller = (overrides) => ({
  ...initialState(),
  changes: [],
  integrationUnapplied: false,
  cancelOperation: vi.fn(),
  resolveGuard: vi.fn(),
  ...overrides,
});

test.each([
  [
    'draft guard',
    { guard: { kind: 'context', title: 'Switch' }, changes: [{}, {}] },
    'alertdialog',
    'confirm',
  ],
  [
    'generation preview',
    {
      generationPreview: { outputDirectory: '/out', revision: 'r', files: [preview] },
      generationPreviewPath: preview.path,
    },
    'dialog',
    'preview',
  ],
  ['handoff import', { handoffImportOpen: true }, 'dialog', 'form'],
])('%s uses the dialog size for its purpose', (_, overrides, role, size) => {
  const view = render(React.createElement(PreviewDialogs, { controller: controller(overrides) }));
  const dialog = view.getByRole(role);
  expect([...dialog.classList]).toContain(size);
  expect(view.getAllByRole(role)).toHaveLength(1);
});

test('the draft guard states what is pending instead of yes/no flags', () => {
  previewLanguage('en');
  const view = render(
    React.createElement(PreviewDialogs, {
      controller: controller({
        guard: { kind: 'context', title: 'Switch' },
        changes: [{}, {}],
        workspace: { dirty: true },
      }),
    }),
  );
  const dialog = view.getByRole('alertdialog');
  expect(dialog.textContent).toContain(translate('workflow.guard.unapplied', { count: 2 }));
  expect(dialog.textContent).toContain(translate('workflow.guard.unsaved'));
  expect(dialog.textContent).not.toMatch(/\b(Yes|No)\b/);
});

test.each(['appearance', 'language', 'definitions', 'tools'])(
  'settings category %s has at most one primary action',
  (category) => {
    previewLanguage('zh-CN');
    const view = render(
      React.createElement(SettingsPage, {
        controller: withCommands({
          ...initialState(),
          settingsOpen: true,
          settingsCategory: category,
          native: true,
          capabilities: { environmentOverrides: [], executionTools: null, toolError: null },
        }),
      }),
    );
    expect(view.getByRole('tab', { selected: true }).textContent).toBe(
      translate(`shell.settings.${category}`),
    );
    const dialog = view.getByRole('dialog');
    expect([...dialog.classList]).toContain('settings-dialog');
    expect(dialog.querySelectorAll('.primary-button').length).toBe(
      category === 'definitions' ? 0 : 1,
    );
  },
);

test('the standard save preview renders only in its dialog, not inline in the panel', () => {
  previewLanguage('zh-CN');
  const integrationPreview = {
    revision: 'r',
    files: [{ path: 'ecuc.arxml', changed: true, before: 'OLD-BYTES', after: 'NEW-BYTES' }],
  };
  const panel = {
    ...initialState(),
    workspace: { dirty: true },
    integrationPreview,
    integrationPreviewPath: 'ecuc.arxml',
    integrationIds: {},
    integrationPeriod: '',
    integrationIssues: [],
    actionReason: () => '',
  };
  const view = render(
    React.createElement(
      React.Fragment,
      null,
      React.createElement(IntegrationPanel, { controller: panel }),
      React.createElement(PreviewDialogs, { controller: controller(panel) }),
    ),
  );
  expect(view.getAllByText('NEW-BYTES')).toHaveLength(1);
  expect(view.getByRole('dialog').textContent).toContain('NEW-BYTES');
});

test('modal ownership hides background native scrollbars and restores them on close', () => {
  const style = document.createElement('style');
  style.textContent = readFileSync(path.join(import.meta.dirname, '../src/styles.css'), 'utf8');
  document.head.append(style);
  const root = document.createElement('div');
  root.id = 'root';
  document.body.append(root);
  try {
    const background = React.createElement('div', { 'data-testid': 'scroll-panel' }, 'Workbench');
    const view = render(
      React.createElement(
        React.Fragment,
        null,
        background,
        React.createElement(Dialog, { title: 'Settings', onClose: vi.fn() }, 'Settings body'),
      ),
      { container: root },
    );
    expect(root.getAttribute('aria-hidden')).toBe('true');
    expect(getComputedStyle(view.getByTestId('scroll-panel')).scrollbarWidth).toBe('none');
    expect(getComputedStyle(within(document.body).getByRole('dialog')).scrollbarWidth).not.toBe(
      'none',
    );
    view.rerender(background);
    expect(root.hasAttribute('aria-hidden')).toBe(false);
    expect(getComputedStyle(view.getByTestId('scroll-panel')).scrollbarWidth).toBe('thin');
  } finally {
    cleanup();
    root.remove();
    style.remove();
  }
});
