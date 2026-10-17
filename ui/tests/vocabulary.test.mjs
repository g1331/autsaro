// @vitest-environment jsdom
import React from 'react';
import { readFileSync, readdirSync } from 'node:fs';
import { act, cleanup, fireEvent, render, waitFor, within } from '@testing-library/react';
import { computeAccessibleName } from 'dom-accessibility-api';
import { afterEach, expect, test, vi } from 'vitest';
import backendCatalog from '../../core/src/messages.json';
import {
  composedMessage,
  localize,
  message,
  previewLanguage,
  resources,
  translate,
} from '../src/i18n';
import { actionResources } from '../src/i18n/actions';
import { NoticeBanner } from '../src/workbench/Dialog';
import { headline } from '../src/workbench/forms';
import App from '../src/App';

const native = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: native.invoke }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    setTitle: vi.fn().mockResolvedValue(undefined),
    onCloseRequested: vi.fn().mockResolvedValue(() => {}),
    onResized: vi.fn().mockResolvedValue(() => {}),
    isMaximized: vi.fn().mockResolvedValue(false),
  }),
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: native.open, confirm: vi.fn() }));

window.matchMedia ??= () => ({
  matches: false,
  addEventListener: () => {},
  removeEventListener: () => {},
});

globalThis.ResizeObserver ??= class {
  observe() {}
  disconnect() {}
};

afterEach(() => {
  cleanup();
  localStorage.clear();
  previewLanguage('system');
});

const languages = ['zh-CN', 'en'];

/** Opens a menubar menu the way a keyboard user does (Radix opens on ↓). */
function openMenu(view, menu) {
  const trigger = within(view.getByRole('menubar')).getByRole('menuitem', {
    name: translate(`shell.menu.${menu}`),
  });
  act(() => trigger.focus());
  fireEvent.keyDown(trigger, { key: 'ArrowDown' });
  return within(view.getByRole('menu'));
}

const catalogs = {
  ui: Object.fromEntries(languages.map((language) => [language, resources[language].translation])),
  backend: backendCatalog,
};

test('no catalog uses development modifiers, and Chinese copy says 工程 for a project', () => {
  for (const [name, catalog] of Object.entries(catalogs)) {
    for (const [key, text] of Object.entries(catalog['zh-CN'])) {
      expect(text, `${name}:${key}`).not.toMatch(/真实|后台|后端|项目/);
    }
    for (const [key, text] of Object.entries(catalog.en)) {
      expect(text, `${name}:${key}`).not.toMatch(/\bbackend\b/i);
    }
  }
});

test('interface copy never names internal identifiers such as objectId, batch keys or IPC', () => {
  for (const language of languages) {
    for (const [key, text] of Object.entries(catalogs.ui[language])) {
      expect(text, key).not.toMatch(/objectId|changeId|批次键|batch key|\bIPC\b|\bDTO\b/i);
    }
  }
});

// Stage, tab and owner names and dialog titles name things; they may share a word with a command.
const nouns =
  /^(shell\.(stage|owner)\.|editor\.tools\.tab\.|backend\.operation\.|shell\.settings\.title$|workflow\.handoff\.title$|editor\.common\.select$)/;

test('each action has exactly one label key, and no other key relabels it', () => {
  for (const language of languages) {
    const labels = new Map();
    for (const [key, text] of Object.entries(actionResources[language])) {
      if (!key.startsWith('action.')) continue;
      const word = text.replace(/…$/, '');
      expect(labels.get(word), `${language}:${key} repeats ${labels.get(word)}`).toBeUndefined();
      labels.set(word, key);
    }
    for (const [key, text] of Object.entries(catalogs.ui[language])) {
      if (/^(action|activity)\./.test(key) || key.startsWith('shell.ipc.') || nouns.test(key))
        continue;
      expect(labels.get(text.replace(/…$/, '')), `${language}:${key} duplicates an action`).toBe(
        undefined,
      );
    }
  }
});

test('every operation is named by an activity key, so busy and failure text stays grammatical', () => {
  const sources = ['src/workbench', 'src']
    .flatMap((directory) =>
      readdirSync(directory)
        .filter((file) => /\.tsx?$/.test(file))
        .map((file) => readFileSync(`${directory}/${file}`, 'utf8')),
    )
    .join('\n');
  const named = [
    ...sources.matchAll(/kind: '\w+', label: message\('([^']+)'\)/g),
    ...sources.matchAll(/(?:setBusy|guardContext|replaceProject)\(\s*message\('([^']+)'\)/g),
  ].map((match) => match[1]);
  expect(named.length).toBeGreaterThan(20);
  for (const key of named) expect(key).toMatch(/^activity\./);
  for (const [key, text] of Object.entries(actionResources.en))
    if (key.startsWith('activity.')) expect(text, key).toMatch(/^[A-Z][a-z]*ing\b/);

  previewLanguage('en');
  const label = message('activity.previewSave');
  expect(localize(composedMessage('controller.operation.busy', { label }))).toBe(
    'Previewing the save…',
  );
  expect(localize(composedMessage('controller.operation.failed', { label, error: 'denied' }))).toBe(
    'Previewing the save failed: denied',
  );
  previewLanguage('zh-CN');
  expect(localize(composedMessage('controller.operation.failed', { label, error: '拒绝' }))).toBe(
    '预览保存失败：拒绝',
  );
});

test('error codes and change identifiers stay in the copyable detail, not the sentence', () => {
  previewLanguage('zh-CN');
  const stale = message('backend.workbench.stale_delivery');
  const view = render(
    React.createElement(NoticeBanner, {
      notice: {
        tone: 'error',
        text: composedMessage('controller.operation.failed', {
          label: message('activity.build'),
          error: stale,
        }),
        detail: stale,
      },
    }),
  );
  const alert = view.getByRole('alert');
  expect(alert.querySelector('.notice-body > span').textContent).toBe(
    '构建失败：工程、规则、工具或目标已变化，本次操作未生效',
  );
  expect(alert.querySelector('details pre').textContent).toContain('STALE_DELIVERY');
  expect(within(alert).getByRole('button', { name: translate('action.copyDetails') })).toBeTruthy();

  const cause = message('backend.arxml.changes.ambiguous_instance_path_cannot_rename');
  const reply = [
    message('backend.arxml.changes.object_change_context', {
      change_id: 'create-1',
      object_id: 'object-7',
    }),
    cause,
  ];
  expect(localize(headline(reply))).toBe(localize(cause));
});

async function openWorkbench(language) {
  localStorage.setItem('autsaro.language', language);
  previewLanguage(language);
  let opened = false;
  const capabilities = () => ({
    fingerprint: 'input',
    target: 'linux-x64-controlled-v1',
    targets: ['linux-x64-controlled-v1'],
    nativeExecution: true,
    hasWorkspace: opened,
    settingsError: null,
    settingsNotice: null,
    executionTools: null,
    configuredExecutionTools: null,
    toolError: null,
    environmentOverrides: [],
    operation: null,
    ruleSetIdentity: null,
    ruleError: null,
    ruleCoverage: [],
    definitionFingerprint: 'definitions',
    appearance: 'light',
    language,
    actions: ['open', 'create', 'edit', 'save'].map((action) => ({
      action,
      available: true,
      reason: null,
    })),
    verificationMode: false,
  });
  const reply = (value) => ({ value, capabilities: capabilities(), inputFingerprint: 'input' });
  native.open.mockResolvedValue(['/inputs/application.arxml']);
  native.invoke.mockImplementation(async (command) => {
    if (command === 'workbench_capabilities') return capabilities();
    if (command === 'open_project') {
      opened = true;
      return reply({
        integrationCandidate: true,
        name: 'Echo',
        dirty: true,
        files: [],
        issues: [],
      });
    }
    if (command === 'project_projection')
      return reply({
        workspaceEpoch: 'epoch',
        profile: 'unrecognized',
        release: 'R24-11',
        inputFingerprint: 'input',
        definitionFingerprint: 'definitions',
        projectPath: null,
        dirty: true,
        sources: [],
        objects: [],
        fields: [],
        references: [],
        diagnostics: [],
        validation: [],
        capabilities: [],
        referenceCandidates: [],
        instanceDefinitions: [],
        extensionDefinitions: [],
      });
    if (command === 'inspect_integration')
      return reply({
        profile: 'singlecore-ecu-v1',
        diagnostics: [],
        description: {
          sources: [],
          composition: { components: [{ component: '/App/Echo', instance: '/App/Comp/Echo' }] },
          schedule: { entities: [{ application: true, periodMs: 10 }] },
          signals: [
            { port: '/App/Echo/RxValue', canId: 0x320, receive: true, dlc: 4 },
            { port: '/App/Echo/TxValue', canId: 0x321, receive: false, dlc: 4 },
          ],
        },
      });
    return reply(null);
  });
  const view = render(React.createElement(App));
  await waitFor(() => expect(view.getByRole('button', { name: translate('action.importArxml') })));
  return view;
}

/** Visible label text of a form control: its <label> text without nested controls. */
function visibleLabel(control) {
  const label = control.closest('label');
  if (!label) return null;
  const copy = label.cloneNode(true);
  copy.querySelectorAll('input, select, textarea, button, option').forEach((node) => node.remove());
  return copy.textContent.replace(/\s+/g, ' ').trim() || null;
}

function expectLabelsMatchNames(container, where) {
  for (const button of container.querySelectorAll('button')) {
    const visible = button.textContent.replace(/\s+/g, ' ').trim();
    const name = button.getAttribute('aria-label');
    if (visible && name) expect(name, `${where}: button "${visible}"`).toBe(visible);
  }
  for (const control of container.querySelectorAll('input, select, textarea')) {
    const visible = visibleLabel(control);
    if (visible)
      expect(computeAccessibleName(control), `${where}: field "${visible}"`).toBe(visible);
  }
}

test.each(languages)(
  '%s: menus, toolbar and palette share one label per action; visible labels are the accessible names',
  async (language) => {
    const view = await openWorkbench(language);
    expectLabelsMatchNames(view.container, 'new project');
    fireEvent.click(view.getByRole('button', { name: translate('action.importArxml') }));
    expectLabelsMatchNames(view.container, 'import');
    await act(async () => {
      fireEvent.click(view.getByRole('button', { name: translate('action.chooseArxml') }));
    });
    await act(async () => {
      fireEvent.click(view.getByRole('button', { name: translate('action.import') }));
    });
    await waitFor(() =>
      expect(view.getAllByRole('button', { name: translate('action.validate') })),
    );

    // Every toolbar command renders the label its menu entry uses.
    const toolbar = within(view.container.querySelector('.context-toolbar')).getAllByRole('button');
    expect(toolbar.map((button) => button.textContent)).toEqual(
      expect.arrayContaining([translate('action.previewSave'), translate('action.validate')]),
    );
    const menuItems = [];
    for (const menu of ['file', 'edit', 'view', 'tools', 'help']) {
      menuItems.push(
        ...openMenu(view, menu)
          .getAllByRole('menuitem')
          .map((item) => item.firstChild.textContent),
      );
      fireEvent.keyDown(view.getByRole('menu'), { key: 'Escape' });
    }
    for (const button of toolbar)
      if (!button.querySelector('kbd') && button.textContent)
        expect(menuItems).toContain(button.textContent);
    for (const key of [
      'action.newProject',
      'action.importArxml',
      'action.openProject',
      'action.importHandoff',
      'action.saveAs',
      'action.closeProject',
      'action.settings',
      'action.apply',
      'action.revert',
      'action.previewSource',
      'action.preflight',
      'action.initializeApplication',
      'action.cancelOperation',
    ])
      expect(menuItems).toContain(translate(key));

    fireEvent.click(view.getByRole('button', { name: /Ctrl\+K/ }));
    const palette = view.getByRole('dialog');
    // The palette also lists unavailable commands, so its name does not say "available".
    expect(
      within(palette).getByRole('combobox', { name: translate('shell.app.searchPlaceholder') }),
    ).toBeTruthy();
    const commands = within(palette)
      .getAllByRole('option')
      .map((option) => option.firstChild.textContent);
    expect(commands).toContain(translate('action.previewSave'));
    // Commands without a menu entry (page-only actions) still read their action label.
    const actionLabels = Object.entries(actionResources[language])
      .filter(([key]) => key.startsWith('action.'))
      .map(([, label]) => label);
    for (const command of commands) expect([...menuItems, ...actionLabels]).toContain(command);
    expectLabelsMatchNames(palette, 'palette');
    fireEvent.keyDown(palette, { key: 'Escape' });

    fireEvent.click(view.getByRole('button', { name: translate('shell.settings.title') }));
    const settings = view.getByRole('dialog');
    for (const category of ['appearance', 'language', 'definitions', 'tools']) {
      fireEvent.click(
        within(settings).getByRole('tab', { name: translate(`shell.settings.${category}`) }),
      );
      expectLabelsMatchNames(settings, `settings ${category}`);
    }
    fireEvent.click(within(settings).getByRole('button', { name: translate('action.cancel') }));

    fireEvent.click(
      openMenu(view, 'view').getByRole('menuitem', {
        name: translate('shell.document.configuration'),
      }),
    );
    // The configuration page names the profile, never shows its identifier.
    expect(view.container.querySelector('.document-heading').textContent).toContain(
      `${translate('shell.profile.unrecognized')} · R24-11`,
    );

    fireEvent.click(
      openMenu(view, 'view').getByRole('menuitem', {
        name: translate('shell.document.integration'),
      }),
    );
    await waitFor(() => expect(view.getByDisplayValue('10')));
    expectLabelsMatchNames(view.container, 'standard inputs');
    // Each CAN ID field is named by its own port, and the period keeps its unit.
    expect(
      view.getByRole('textbox', {
        name: `${translate('shell.integration.receiveId')} · /App/Echo/RxValue`,
      }).value,
    ).toBe('800');
    expect(view.getByRole('textbox', { name: translate('shell.integration.period') }).value).toBe(
      '10',
    );

    fireEvent.click(
      openMenu(view, 'view').getByRole('menuitem', { name: translate('shell.document.delivery') }),
    );
    expectLabelsMatchNames(view.container, 'source delivery');
    for (const key of ['shell.delivery.output', 'shell.delivery.buildDirectory'])
      expect(view.getByRole('textbox', { name: translate(key) })).toBeTruthy();
  },
);
