// @vitest-environment jsdom
import { createElement } from 'react';
import { cleanup, fireEvent, render } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { ProjectTree } from '../src/workbench/ProjectTree';
import { ToolWindows } from '../src/workbench/ToolWindows';
import { initialState } from '../src/workbench/state';

afterEach(() => {
  cleanup();
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
          scope: 'source',
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
  fireEvent.click(view.getByRole('button', { name: '定位真实来源' }));
  expect(controller.readSource).toHaveBeenCalledWith('source');
  view.rerender(
    createElement(ToolWindows, {
      controller: { ...controller, toolWindow: 'build', busy: 'Running' },
    }),
  );
  expect(view.queryByText('Source diagnostic')).toBeNull();
  expect(view.getByRole('tabpanel', { name: '构建' })).toBeTruthy();
});
