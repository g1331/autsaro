// @vitest-environment jsdom
import React from 'react';
import { act, cleanup, fireEvent, render, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { previewLanguage, translate } from '../src/i18n';
import { SettingsPage } from '../src/pages/SettingsPage';
import { PreviewDialogs } from '../src/workbench/PreviewDialogs';
import { useWorkbench } from '../src/workbench/useWorkbench';

const native = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn(), confirm: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: native.invoke }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ setTitle: vi.fn().mockResolvedValue(undefined) }),
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: native.open, confirm: native.confirm }));

const catalogPath = '/catalogs/lab/catalog.json';
const identity = { catalogId: 'acceptance-lab', release: 'R24-11', version: '1.0.0', sha256: 'ab' };
const capabilities = {
  fingerprint: 'input',
  target: 'linux-x64-controlled-v1',
  targets: ['linux-x64-controlled-v1'],
  nativeExecution: true,
  hasWorkspace: true,
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
  language: 'en',
  actions: [],
  verificationMode: false,
};
const projection = (accepted) => ({
  workspaceEpoch: 'epoch',
  inputFingerprint: 'input',
  definitionFingerprint: 'definitions',
  projectPath: null,
  dirty: false,
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
  acceptedExtensionDefinitions: accepted,
});

let controller;
let accepted;
const confirmSpy = vi.fn();

beforeEach(() => {
  previewLanguage('en');
  accepted = [];
  vi.stubGlobal('window', Object.assign(window, { confirm: confirmSpy }));
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      disconnect() {}
    },
  );
  native.invoke.mockImplementation(async (command) => {
    if (command === 'workbench_capabilities') return capabilities;
    if (command === 'import_definition_catalog') accepted = [identity];
    else if (command === 'remove_definition_catalog') accepted = [];
    else throw new Error(`Unexpected command: ${command}`);
    return { value: projection(accepted), capabilities, inputFingerprint: 'input' };
  });
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  vi.unstubAllGlobals();
  localStorage.clear();
  previewLanguage('system');
});

async function renderSettings() {
  function Consumer() {
    controller = useWorkbench();
    return React.createElement(
      React.Fragment,
      null,
      React.createElement(SettingsPage, { controller }),
      React.createElement(PreviewDialogs, { controller }),
    );
  }
  const view = render(React.createElement(Consumer));
  await waitFor(() => expect(controller.capabilities).not.toBeNull());
  act(() => {
    controller.setSettingsCategory('definitions');
    controller.setSettingsOpen(true);
  });
  return view;
}

const catalogCalls = () =>
  native.invoke.mock.calls.filter(([command]) => /definition_catalog$/.test(command));
const confirmDialog = (view, title) => view.findByRole('alertdialog', { name: translate(title) });

test('accepting a catalog asks in the app, names its path, and acts only on confirm', async () => {
  const view = await renderSettings();
  native.open.mockResolvedValue(catalogPath);

  act(() => {
    void controller.importDefinitionCatalog();
  });
  let dialog = within(await confirmDialog(view, 'activity.acceptDefinitions'));
  expect(dialog.getByText(catalogPath)).toBeTruthy();
  expect(dialog.getByText(translate('workflow.confirm.acceptCatalog'))).toBeTruthy();
  fireEvent.click(dialog.getByRole('button', { name: translate('action.cancel') }));
  await waitFor(() => expect(view.queryByRole('alertdialog')).toBeNull());
  expect(catalogCalls()).toEqual([]);
  expect(accepted).toEqual([]);

  act(() => {
    void controller.importDefinitionCatalog();
  });
  dialog = within(await confirmDialog(view, 'activity.acceptDefinitions'));
  fireEvent.click(dialog.getByRole('button', { name: translate('action.confirmAccept') }));
  await waitFor(() =>
    expect(controller.projection?.acceptedExtensionDefinitions).toEqual([identity]),
  );
  expect(catalogCalls().map(([command, payload]) => [command, payload.catalogPath])).toEqual([
    ['import_definition_catalog', catalogPath],
  ]);
  expect(native.confirm).not.toHaveBeenCalled();
  expect(confirmSpy).not.toHaveBeenCalled();
});

test('removing a catalog uses the danger style, names the catalog, and acts only on confirm', async () => {
  const view = await renderSettings();
  native.open.mockResolvedValue(catalogPath);
  act(() => {
    void controller.importDefinitionCatalog();
  });
  fireEvent.click(
    within(await confirmDialog(view, 'activity.acceptDefinitions')).getByRole('button', {
      name: translate('action.confirmAccept'),
    }),
  );
  const remove = await view.findByRole('button', { name: translate('action.removeDefinitions') });
  native.invoke.mockClear();

  fireEvent.click(remove);
  let dialog = within(await confirmDialog(view, 'activity.removeDefinitions'));
  expect(
    dialog.getByText(
      translate('workflow.confirm.removeCatalog', { catalogId: identity.catalogId }),
    ),
  ).toBeTruthy();
  const danger = dialog.getByRole('button', { name: translate('action.removeDefinitions') });
  expect(danger.className).toContain('danger-button');
  fireEvent.click(dialog.getByRole('button', { name: translate('action.cancel') }));
  await waitFor(() => expect(view.queryByRole('alertdialog')).toBeNull());
  expect(catalogCalls()).toEqual([]);
  expect(controller.projection.acceptedExtensionDefinitions).toEqual([identity]);

  fireEvent.click(await view.findByRole('button', { name: translate('action.removeDefinitions') }));
  dialog = within(await confirmDialog(view, 'activity.removeDefinitions'));
  fireEvent.click(dialog.getByRole('button', { name: translate('action.removeDefinitions') }));
  await waitFor(() => expect(controller.projection.acceptedExtensionDefinitions).toEqual([]));
  expect(catalogCalls().map(([command, payload]) => [command, payload.catalogId])).toEqual([
    ['remove_definition_catalog', identity.catalogId],
  ]);
  expect(native.confirm).not.toHaveBeenCalled();
  expect(confirmSpy).not.toHaveBeenCalled();
});
