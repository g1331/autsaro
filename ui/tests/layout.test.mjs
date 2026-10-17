// @vitest-environment jsdom
import React from 'react';
import { cleanup, render, within } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { previewLanguage, translate } from '../src/i18n';
import { ObjectEditor } from '../src/workbench/ObjectEditor';
import { IntegrationDelivery } from '../src/IntegrationDelivery';
import { stageDefaults } from '../src/workbench/useDelivery';
import { initialState } from '../src/workbench/state';
import { withCommands } from './support/commands.mjs';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => false, invoke: vi.fn() }));

afterEach(() => {
  cleanup();
  previewLanguage('system');
});

const preview = { path: 'out/Rte.c', before: null, after: 'int x;', status: 'added' };

function delivery(overrides = {}) {
  return withCommands({
    ...initialState(),
    workspace: { dirty: false },
    native: true,
    capabilities: { nativeExecution: true, toolError: null },
    stages: stageDefaults,
    unapplied: false,
    actionReason: () => '',
    applicationWarnings: [],
    applicationRecoveryFiles: [],
    ...overrides,
  });
}

test('delivery steps keep each path beside its chooser, one primary each, and no page-level generate', () => {
  previewLanguage('zh-CN');
  const generationPreview = { outputDirectory: '/out', revision: 'r', files: [preview] };
  const view = render(
    React.createElement(IntegrationDelivery, {
      controller: delivery({ generationPreview, ecuOutputDirectory: '/out' }),
      active: true,
    }),
  );
  for (const field of ['shell.delivery.output', 'shell.delivery.buildDirectory']) {
    const input = view.getByRole('textbox', { name: translate(field) });
    const button = within(input.parentElement).getByRole('button', {
      name: translate('action.browse'),
    });
    expect(button.parentElement).toBe(input.parentElement);
    expect(input.parentElement.className).toBe('path-picker');
  }
  const steps = view.container.querySelectorAll('.delivery-step');
  // Generate, build and host verification; handoff import is in the File menu and user
  // application initialization on the component node (ADR 0020 D7).
  expect(steps).toHaveLength(3);
  expect(
    view.queryByRole('button', { name: translate('action.initializeApplication') }),
  ).toBeNull();
  expect(view.queryByRole('button', { name: translate('action.importHandoff') })).toBeNull();
  // Steps this product does not perform are one note, not stages.
  expect(view.container.querySelectorAll('.stage-list > li')).toHaveLength(5);
  expect(view.getByText(translate('shell.delivery.outsideProduct'))).toBeTruthy();
  for (const step of steps)
    expect(step.querySelectorAll('.primary-button').length).toBeLessThanOrEqual(1);
  // The preview exists, but generation is confirmed only inside its dialog.
  expect(view.queryByText(preview.after)).toBeNull();
  for (const action of ['action.confirmGenerate', 'action.confirmExport'])
    expect(view.queryByRole('button', { name: translate(action) })).toBeNull();
});

test('object rows stay on one line: no repeated kind, reasons and paths move to titles', () => {
  previewLanguage('zh-CN');
  const objects = [
    {
      objectId: 'a',
      shortName: 'EchoApplication',
      kind: 'APPLICATION-SW-COMPONENT-TYPE',
      definitionId: 'APPLICATION-SW-COMPONENT-TYPE',
      path: '/Application/EchoApplication',
      writable: false,
      reason: 'Read-only in this profile',
    },
    {
      objectId: 'b',
      shortName: 'OsTask',
      kind: 'ECUC-CONTAINER-VALUE',
      definitionId: '/AUTOSAR/EcucDefs/Os/OsTask',
      path: '/Ecuc/Os/OsTask',
      writable: true,
      reason: null,
    },
  ];
  const view = render(
    React.createElement(ObjectEditor, {
      controller: withCommands({
        ...initialState(),
        projection: { objects, fields: [], profile: 'p', release: 'R24-11' },
        objectFilter: '',
        objectSelection: [],
        changes: [],
      }),
    }),
  );
  const [first, second] = view.container.querySelectorAll('tbody tr');
  const cells = (row) => [...row.querySelectorAll('td')].map((cell) => cell.textContent);
  expect(cells(first).slice(1)).toEqual([
    'EchoApplication',
    'APPLICATION-SW-COMPONENT-TYPE',
    '—',
    translate('editor.common.readonly'),
  ]);
  expect(cells(second)[3]).toBe('/AUTOSAR/EcucDefs/Os/OsTask');
  expect(first.querySelectorAll('small')).toHaveLength(0);
  expect(first.querySelector('td:last-child').title).toContain('Read-only in this profile');
  expect(first.querySelector('td:nth-child(2)').title).toBe('/Application/EchoApplication');
});
