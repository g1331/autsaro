// @vitest-environment jsdom
import React from 'react';
import { act, cleanup, fireEvent, render, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { localize, message, previewLanguage, translate } from '../src/i18n';
import { useWorkbench } from '../src/workbench/useWorkbench';
import { IntegrationPanel } from '../src/IntegrationPanel';
import { OwnedLog } from '../src/workbench/Dialog';
import { ToolWindows } from '../src/workbench/ToolWindows';

const native = vi.hoisted(() => ({
  invoke: vi.fn(),
  setTitle: vi.fn().mockResolvedValue(undefined),
}));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: native.invoke }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ setTitle: native.setTitle }),
}));

const capabilities = {
  fingerprint: 'unchanged-input',
  target: 'windows-x64-controlled-v1',
  targets: [],
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
  language: 'zh-CN',
  actions: [],
  verificationMode: false,
};
const workspace = {
  integrationCandidate: true,
  name: 'RawProject',
  dirty: false,
  files: [],
  frames: [],
  signals: [],
  diagnostic: null,
  issues: [],
};
const baseProjection = {
  workspaceEpoch: 'epoch',
  inputFingerprint: 'unchanged-input',
  definitionFingerprint: 'definitions',
  projectPath: 'C:\\用户\\project.autosar',
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
};
const applicationPreview = {
  revision: 'preview',
  slot: {},
  files: [],
  manifestBefore: 'raw-before',
  manifestAfter: 'raw-after',
};
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  vi.unstubAllGlobals();
  previewLanguage('system');
});

async function openConsumer(rejection, projection = baseProjection) {
  let opened = false;
  native.invoke.mockImplementation(async (command) => {
    const currentCapabilities = { ...capabilities, hasWorkspace: opened };
    if (command === 'workbench_capabilities') return currentCapabilities;
    if (command === 'open_handoff_project') {
      opened = true;
      return {
        value: workspace,
        capabilities: { ...capabilities, hasWorkspace: true },
        inputFingerprint: 'unchanged-input',
      };
    }
    if (command === 'project_projection')
      return {
        value: projection,
        capabilities: currentCapabilities,
        inputFingerprint: 'unchanged-input',
      };
    if (command === 'inspect_integration')
      return {
        value: { profile: 'standard', diagnostics: [], description: null },
        capabilities: currentCapabilities,
        inputFingerprint: 'unchanged-input',
      };
    if (command === 'preview_application_initialization')
      return {
        value: applicationPreview,
        capabilities: currentCapabilities,
        inputFingerprint: 'unchanged-input',
      };
    if (command === 'initialize_application_previewed') throw rejection;
    throw new Error(`Unexpected command: ${command}`);
  });
  let controller;
  function Consumer() {
    controller = useWorkbench();
    return React.createElement(
      React.Fragment,
      null,
      React.createElement(IntegrationPanel, { controller }),
      React.createElement(ToolWindows, { controller }),
      controller.notice
        ? React.createElement(OwnedLog, {
            text: controller.notice.text,
            label: message('controller.integration.inspect'),
          })
        : null,
    );
  }
  const view = render(React.createElement(Consumer));
  await waitFor(() => expect(controller.capabilities).not.toBeNull());
  act(() => {
    controller.setHandoffImportMode('legacy');
    controller.setEcuImportDirectory('C:\\handoff');
  });
  await act(async () => {
    await controller.confirmHandoffImport();
  });
  await waitFor(() => expect(controller.integrationInspection).not.toBeNull());
  return { view, current: () => controller };
}

test('application recovery message aggregates remain feedback rather than integration diagnostics', async () => {
  const raw = 'C:\\用户\\project.autosar.autosar.bak: hard-link permission denied <raw>';
  const rejection = [
    message('backend.arxml.application.initialization_failed'),
    raw,
    message('backend.arxml.application.recovery_details'),
    [
      [
        message('backend.arxml.application.original_manifest_backup_retained', {
          path: 'C:\\用户\\project.autosar.autosar.bak',
        }),
        'external recovery evidence 原文',
      ],
    ],
  ];
  const { view, current } = await openConsumer(rejection);
  await act(async () => {
    await current().previewApplicationInitialization();
  });
  await act(async () => {
    await current().initializeApplicationPreviewed();
  });
  expect(current().integrationIssues).toEqual([]);
  expect(view.container.querySelector('.integration-view [role="alert"]')).toBeNull();
  const feedback = view.container.querySelector('.owned-log pre');
  const chinese = feedback.textContent;
  expect(chinese).toContain(raw);
  expect(chinese).toContain('external recovery evidence 原文');
  act(() => current().setLanguageDraft('en'));
  expect(view.container.querySelector('.owned-log pre')).toBe(feedback);
  expect(feedback.textContent).not.toBe(chinese);
  expect(feedback.textContent).toContain(raw);
  expect(feedback.textContent).toContain('external recovery evidence 原文');
  expect(feedback.textContent).not.toMatch(/{{|}}|\[object Object\]/);
  expect(current().capabilities.fingerprint).toBe('unchanged-input');
});

test('legitimate plan diagnostic arrays still render as localized integration diagnostics', async () => {
  const diagnostic = {
    category: 'dependency',
    code: 'APPLICATION_RECOVERY',
    file: 'C:\\source.arxml',
    object: '/RawProject/Object',
    message: message('backend.arxml.application.initialization_failed'),
    remedy: [message('backend.arxml.application.recovery_details'), 'external 原文'],
  };
  const { view, current } = await openConsumer([diagnostic]);
  await act(async () => {
    await current().previewApplicationInitialization();
  });
  await act(async () => {
    await current().initializeApplicationPreviewed();
  });
  expect(current().integrationIssues).toEqual([diagnostic]);
  const alert = view.container.querySelector('.integration-view [role="alert"]');
  expect(alert.textContent).toContain('APPLICATION_RECOVERY');
  expect(alert.textContent).toContain(diagnostic.file);
  expect(alert.textContent).toContain(diagnostic.object);
  const chinese = alert.textContent;
  act(() => current().setLanguageDraft('en'));
  expect(alert.textContent).not.toBe(chinese);
  expect(alert.textContent).toContain('external 原文');
  expect(alert.textContent).not.toMatch(/{{|}}|\[object Object\]/);
});

test('Problems renders serialized cardinality child names, bounds and actual counts in both languages', async () => {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      disconnect() {}
    },
  );
  const diagnostics = [
    { names: 'SHORT-NAME', min: '1', max: '1', count: '0' },
    { names: 'LENGTH', min: '0', max: '1', count: '2' },
  ].map(({ names, min, max, count }) => ({
    scope: 'schema',
    ruleId: 'ARXML-CARDINALITY',
    severity: 'error',
    code: `CARDINALITY_${names}`,
    message: [message('backend.rules.constraint_cardinality', { names, min, max }), count],
    remedy: message('backend.rules.remedy_schema'),
    file: 'C:\\用户\\unchanged.arxml',
    path: `/RawProject/${names}`,
    sourceId: 'source',
    objectId: null,
    fieldId: null,
    witness: {
      ruleId: 'ARXML-CARDINALITY',
      subjects: [names],
      constraint: message('backend.rules.constraint_cardinality', { names, min, max }),
      counterexample: count,
    },
  }));
  const projection = {
    ...baseProjection,
    sources: [{ sourceId: 'source', path: 'C:\\用户\\unchanged.arxml' }],
    diagnostics,
  };
  const { view, current } = await openConsumer(undefined, projection);
  act(() => current().setToolWindow('problems'));
  const clipboard = vi.fn().mockResolvedValue(undefined);
  const previousClipboard = Object.getOwnPropertyDescriptor(navigator, 'clipboard');
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: { writeText: clipboard },
  });
  try {
    let firstLanguage;
    for (const language of ['zh-CN', 'en']) {
      act(() => current().setLanguageDraft(language));
      const rows = view.container.querySelectorAll('.problem-list > li');
      expect(rows).toHaveLength(2);
      for (let index = 0; index < rows.length; index++) {
        const issue = diagnostics[index];
        const content = rows[index].querySelector('p').textContent;
        expect(content).toContain(issue.witness.subjects[0]);
        expect(content).toContain(issue.message[0].params.min);
        expect(content).toContain(issue.message[0].params.max);
        expect(content).toMatch(new RegExp(`\\b${issue.witness.counterexample}\\b`));
        expect(rows[index].textContent).toContain(issue.path);
        expect(rows[index].textContent).not.toMatch(/{{|}}|\[object Object\]/);
        await act(async () => {
          fireEvent.click(
            Array.from(rows[index].querySelectorAll('button')).find(
              (button) => button.textContent === translate('editor.common.copyDetails'),
            ),
          );
        });
        expect(JSON.parse(clipboard.mock.calls.at(-1)[0])).toEqual(issue);
      }
      const rendered = Array.from(rows, (row) => row.textContent).join('\n');
      if (language === 'zh-CN') firstLanguage = rendered;
      else expect(rendered).not.toBe(firstLanguage);
      expect(current().projection).toBe(projection);
    }
  } finally {
    if (previousClipboard) Object.defineProperty(navigator, 'clipboard', previousClipboard);
    else delete navigator.clipboard;
  }
  expect(localize(diagnostics[1].witness.counterexample)).toBe('2');
});
