// @vitest-environment jsdom
import { createElement } from 'react';
import { act, cleanup, fireEvent, render } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { ProjectTree } from '../src/workbench/ProjectTree';
import { ToolWindows, toolLabels } from '../src/workbench/ToolWindows';
import { EditorPage } from '../src/pages/EditorPage';
import { localize, message, previewLanguage, translate } from '../src/i18n';
import { initialState } from '../src/workbench/state';

afterEach(() => {
  cleanup();
  previewLanguage('system');
  vi.unstubAllGlobals();
});

test('execution updates reuse tree rows, keep current actions, and still apply filters', () => {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      disconnect() {}
    },
  );
  const readKind = vi.fn();
  const object = {
    objectId: 'object',
    parentId: null,
    shortName: 'First',
    path: '/First',
    definitionId: 'defs',
    get kind() {
      readKind();
      return 'AR-PACKAGE';
    },
  };
  const controller = {
    ...initialState(),
    workspace: { name: 'Example' },
    projection: { objects: [object], fields: [], sources: [] },
    selectObject: vi.fn(),
    setTreeVisible: vi.fn(),
    setTreeMode: vi.fn(),
    setTreeFilter: vi.fn(),
    setTreeRevealId: vi.fn(),
  };
  const view = render(createElement(ProjectTree, { controller }));
  const initialReads = readKind.mock.calls.length;
  const selectObject = vi.fn();
  const executing = { ...controller, busy: 'Running', selectObject };
  view.rerender(createElement(ProjectTree, { controller: executing }));
  expect(readKind).toHaveBeenCalledTimes(initialReads);
  fireEvent.click(view.getByRole('treeitem', { name: 'First' }));
  expect(selectObject).toHaveBeenCalledWith('object', false);
  expect(controller.selectObject).not.toHaveBeenCalled();
  view.rerender(
    createElement(ProjectTree, { controller: { ...executing, treeFilter: 'missing' } }),
  );
  expect(view.queryByRole('treeitem', { name: 'First' })).toBeNull();
  view.unmount();
});

test('tool windows mount active content and retain source navigation when switched', async () => {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      disconnect() {}
    },
  );
  const controller = {
    ...initialState(),
    toolWindow: 'log',
    projection: {
      objects: [],
      fields: [],
      validation: [],
      sources: [{ sourceId: 'source', path: '/Source.arxml' }],
      diagnostics: [
        {
          severity: 'error',
          code: 'SOURCE_ERROR',
          scope: 'source-safety',
          message: 'Source diagnostic',
          sourceId: 'source',
        },
      ],
    },
    issues: [],
    readSource: vi.fn(),
    guardContext: (_title, action) => action(),
    setTreeVisible: vi.fn(),
    setInspectorVisible: vi.fn(),
    setInspectorTab: vi.fn(),
    setTreeFilter: vi.fn(),
    setObjectFilter: vi.fn(),
    setToolWindow: vi.fn(),
  };
  const view = render(createElement(ToolWindows, { controller }));
  expect(view.queryByText('Source diagnostic')).toBeNull();
  view.rerender(
    createElement(ToolWindows, { controller: { ...controller, toolWindow: 'problems' } }),
  );
  expect(view.getByText('Source diagnostic')).toBeTruthy();
  fireEvent.click(view.getByRole('button', { name: translate('editor.tools.locateSource') }));
  expect(controller.readSource).toHaveBeenCalledWith('source');
  view.rerender(
    createElement(ToolWindows, {
      controller: { ...controller, toolWindow: 'build', busy: 'Running' },
    }),
  );
  expect(view.queryByText('Source diagnostic')).toBeNull();
  expect(view.getByRole('tabpanel', { name: localize(toolLabels.build) })).toBeTruthy();
});

test('changing language renders stored diagnostics and preserves editor drafts', () => {
  previewLanguage('zh-CN');
  const state = initialState();
  const draft = { ...state.diagnosticDraft, requestId: '0x777' };
  const diagnostic = message('editor.diagnostic.signalUnavailable');
  const workspace = { name: 'Example', frames: [], signals: [], diagnostic: null };
  const controller = {
    ...state,
    workspace,
    diagnosticDraft: draft,
    diagnosticError: diagnostic,
    eligibleSignals: [],
    eligibleMonitorFrames: [],
    issues: [],
    setDiagnosticDraft: vi.fn(),
  };
  const view = render(createElement(EditorPage, { controller, section: 'diagnostic' }));
  const input = view.getByDisplayValue('0x777');
  const originalDiagnostic = view.getByRole('alert').textContent;
  act(() => previewLanguage('en'));
  expect(view.getByRole('alert').textContent).toBe(localize(diagnostic));
  expect(view.getByRole('alert').textContent).not.toBe(originalDiagnostic);
  expect(view.getByDisplayValue('0x777')).toBe(input);
  expect(controller.diagnosticDraft).toBe(draft);
  expect(controller.workspace).toBe(workspace);
  expect(controller.diagnosticError).toBe(diagnostic);
  expect(controller.setDiagnosticDraft).not.toHaveBeenCalled();
  act(() => previewLanguage('zh-CN'));
  expect(view.getByRole('alert').textContent).toBe(originalDiagnostic);
  expect(view.getByDisplayValue('0x777')).toBe(input);
});
