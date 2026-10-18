// @vitest-environment jsdom
import { createElement } from 'react';
import { act, cleanup, fireEvent, render, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { ProjectTree } from '../src/workbench/ProjectTree';
import {
  fieldState,
  groupedObjectTree,
  objectGroup,
  objectMarks,
} from '../src/workbench/projectGroups';
import { applyPatch, initialState } from '../src/workbench/state';
import { previewLanguage } from '../src/i18n';

const object = (objectId, parentId, kind, shortName, definitionId = null) => ({
  objectId,
  parentId,
  kind,
  shortName,
  definitionId,
  path: `/${objectId}`,
  sourceId: 'source',
  writable: true,
  reason: null,
});
const field = (fieldId, objectId, lexeme) => ({
  fieldId,
  objectId,
  definitionId: `/AUTOSAR/EcucDefs/X/${fieldId}`,
  current: { state: 'explicit', value: { kind: 'integer', lexeme } },
  reference: null,
  defaultValue: null,
});

// Two same-named packages, as importing several files produces, plus BSW description details.
const objects = [
  object('pkgA', null, 'AR-PACKAGE', 'Acceptance'),
  object('pkgB', null, 'AR-PACKAGE', 'Acceptance'),
  object('com', 'pkgA', 'ECUC-MODULE-CONFIGURATION-VALUES', 'ComCfg', '/AUTOSAR/EcucDefs/Com'),
  object('can', 'pkgB', 'ECUC-MODULE-CONFIGURATION-VALUES', 'Can', '/AUTOSAR/EcucDefs/Can'),
  object(
    'canGeneral',
    'can',
    'ECUC-CONTAINER-VALUE',
    'CanGeneral',
    '/AUTOSAR/EcucDefs/Can/CanGeneral',
  ),
  object('desc', 'pkgB', 'BSW-MODULE-DESCRIPTION', 'Can'),
  object('swc', 'pkgA', 'APPLICATION-SW-COMPONENT-TYPE', 'Echo'),
  object('port', 'swc', 'P-PORT-PROTOTYPE', 'Out'),
  object('frame', 'pkgA', 'CAN-FRAME', 'Frame1'),
  object('type', 'pkgA', 'IMPLEMENTATION-DATA-TYPE', 'uint8'),
  object('odd', null, 'SOMETHING-ELSE', 'Odd'),
];

beforeEach(() => {
  previewLanguage('en');
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      disconnect() {}
    },
  );
});
afterEach(() => {
  cleanup();
  previewLanguage('system');
  vi.unstubAllGlobals();
});

test('the object view groups elements by category and sorts BSW modules by module name', () => {
  const tree = groupedObjectTree(objects);
  expect(tree.map((node) => node.key)).toEqual([
    'group:modules',
    'group:application',
    'group:communication',
    'group:types',
    'group:bsw',
    'group:other',
  ]);
  // Packages are not nodes; Can sorts before Com by module name, not by short name.
  expect(tree[0].children.map((node) => node.key)).toEqual(['can', 'com']);
  expect(tree[0].children[0].children.map((node) => node.key)).toEqual(['canGeneral']);
  expect(tree[1].children[0].children.map((node) => node.key)).toEqual(['port']);
  const keys = (nodes) => nodes.flatMap((node) => [node.key, ...keys(node.children)]);
  expect(keys(tree)).not.toContain('pkgA');
  expect(keys(tree)).not.toContain('pkgB');
  expect(objectGroup({ kind: 'BSW-IMPLEMENTATION' })).toBe('bsw');
  expect(objectGroup({ kind: 'SYSTEM' })).toBe('communication');
  // Empty groups are omitted.
  expect(groupedObjectTree([objects[2]]).map((node) => node.key)).toEqual(['group:modules']);
});

test('marks sum diagnostics, saved-value differences and drafts over the subtree', () => {
  const tree = groupedObjectTree(objects);
  const fields = [field('f1', 'canGeneral', '1'), field('f2', 'com', '2')];
  const saved = {
    f1: fieldState(fields[0]),
    f2: fieldState({ ...fields[1], current: { state: 'absent' } }),
  };
  const marks = objectMarks(tree, {
    objects,
    fields,
    diagnostics: [
      { objectId: 'canGeneral', fieldId: null, severity: 'error' },
      { objectId: null, fieldId: 'f1', severity: 'warning' },
      { objectId: 'can', fieldId: null, severity: 'info' },
    ],
    integrationIssues: [{ object: '/frame' }, { object: '/missing' }],
    saved,
    changes: [
      {
        op: 'set-value',
        changeId: 'f1',
        field: { kind: 'existing', fieldId: 'f1' },
        expected: fields[0].current,
        value: { state: 'absent' },
      },
    ],
  });
  expect(marks.get('canGeneral')).toEqual({
    errors: 1,
    warnings: 1,
    modified: false,
    unapplied: true,
  });
  expect(marks.get('can')).toEqual({ errors: 1, warnings: 1, modified: false, unapplied: true });
  expect(marks.get('com')).toEqual({ errors: 0, warnings: 0, modified: true, unapplied: false });
  expect(marks.get('group:modules')).toEqual({
    errors: 1,
    warnings: 1,
    modified: true,
    unapplied: true,
  });
  expect(marks.get('frame')?.errors).toBe(1);
  expect(marks.get('swc')?.errors).toBe(0);
});

test('only a projection without unsaved changes becomes the saved baseline', () => {
  const clean = { dirty: false, fields: [field('f1', 'canGeneral', '1')] };
  const state = applyPatch(initialState(), { projection: clean });
  expect(state.savedValues).toEqual({ f1: fieldState(clean.fields[0]) });
  const edited = applyPatch(state, {
    projection: { dirty: true, fields: [field('f1', 'canGeneral', '7')] },
  });
  expect(edited.savedValues).toBe(state.savedValues);
  expect(applyPatch(edited, { projection: null }).savedValues).toBeNull();
});

function controllerFor(extra = {}) {
  return {
    ...initialState(),
    workspace: { name: 'Example' },
    projection: {
      objects,
      fields: [field('f1', 'canGeneral', '1')],
      sources: [],
      diagnostics: [{ objectId: 'canGeneral', fieldId: null, severity: 'error' }],
    },
    savedValues: { f1: fieldState(field('f1', 'canGeneral', '0')) },
    selectObject: vi.fn(),
    setTreeVisible: vi.fn(),
    setTreeMode: vi.fn(),
    setTreeFilter: vi.fn(),
    setTreeRevealId: vi.fn(),
    ...extra,
  };
}

test('the tree renders groups with marks, collapses BSW descriptions and walks back to the group', () => {
  const controller = controllerFor();
  const view = render(createElement(ProjectTree, { controller }));
  const modules = view.getByRole('treeitem', { name: 'BSW modules' });
  expect(modules.getAttribute('aria-level')).toBe('1');
  expect(modules.getAttribute('aria-description')).toBe('1 error · Modified, not saved');
  const can = view.getByRole('treeitem', { name: 'Can' });
  expect(can.getAttribute('aria-level')).toBe('2');
  expect(can.getAttribute('title')).toContain('/can');
  expect(view.queryAllByRole('treeitem', { name: 'Acceptance' })).toEqual([]);
  // BSW descriptions start collapsed; the "Can" description is not shown.
  const bsw = view.getByRole('treeitem', { name: 'BSW descriptions' });
  expect(bsw.getAttribute('aria-expanded')).toBe('false');
  expect(view.getAllByRole('treeitem', { name: 'Can' })).toHaveLength(1);
  fireEvent.click(bsw);
  expect(view.getAllByRole('treeitem', { name: 'Can' })).toHaveLength(2);
  expect(controller.selectObject).not.toHaveBeenCalled();
  fireEvent.click(can);
  expect(controller.selectObject).toHaveBeenCalledWith('can', false);
  // ArrowLeft on an expanded module collapses it, then moves to its group.
  const tree = view.getByRole('tree');
  can.focus();
  fireEvent.keyDown(can, { key: 'ArrowLeft' });
  expect(view.queryByRole('treeitem', { name: 'CanGeneral' })).toBeNull();
  fireEvent.keyDown(view.container.querySelector('[data-tree-key="can"]'), { key: 'ArrowLeft' });
  expect(document.activeElement).toBe(view.getByRole('treeitem', { name: 'BSW modules' }));
  expect(within(tree).getByRole('treeitem', { name: 'Other' })).toBeTruthy();
});

test('filtering keeps the group of a matching member and hides the rest', () => {
  const view = render(
    createElement(ProjectTree, { controller: controllerFor({ treeFilter: 'out' }) }),
  );
  expect(view.getByRole('treeitem', { name: 'Application components' })).toBeTruthy();
  expect(view.getByRole('treeitem', { name: 'Out' })).toBeTruthy();
  expect(view.queryByRole('treeitem', { name: 'BSW modules' })).toBeNull();
});

test('revealing an object expands its collapsed group', () => {
  const view = render(
    createElement(ProjectTree, { controller: controllerFor({ treeRevealId: 'desc' }) }),
  );
  expect(
    view.getByRole('treeitem', { name: 'BSW descriptions' }).getAttribute('aria-expanded'),
  ).toBe('true');
});

test('a wide tree shares global menu keyboard listeners across its object rows', () => {
  const add = vi.spyOn(document, 'addEventListener');
  try {
    const wide = Array.from({ length: 32 }, (_, i) =>
      object(`wide-${i}`, null, 'IMPLEMENTATION-DATA-TYPE', `Type_${i}`),
    );
    const view = render(
      createElement(ProjectTree, {
        controller: controllerFor({
          projection: { objects: wide, fields: [], sources: [], diagnostics: [] },
        }),
      }),
    );
    expect(view.getAllByRole('treeitem')).toHaveLength(33);
    const keyboard = add.mock.calls.filter(([type]) => type === 'keydown').length;
    expect(keyboard).toBeLessThanOrEqual(1);
    add.mockClear();
    fireEvent.keyDown(view.getByRole('treeitem', { name: 'Type_0' }), { key: 'ArrowDown' });
    const pointer = add.mock.calls.filter(
      ([type]) => type === 'pointerdown' || type === 'pointermove',
    ).length;
    expect(pointer).toBeLessThanOrEqual(2);
  } finally {
    add.mockRestore();
  }
});

test('the shared tree menu resolves nested targets, refuses groups and restores row focus', async () => {
  const first = object('first', null, 'IMPLEMENTATION-DATA-TYPE', 'First');
  const second = object('second', null, 'IMPLEMENTATION-DATA-TYPE', 'Second');
  const readonly = {
    ...object('readonly', null, 'IMPLEMENTATION-DATA-TYPE', 'Readonly'),
    writable: false,
  };
  const c = controllerFor({
    projection: { objects: [first, second, readonly], fields: [], sources: [], diagnostics: [] },
    activeObjectId: 'second',
    actionReason: () => null,
    stageChange: vi.fn(),
    setInspectorTab: vi.fn(),
    guardContext: (_message, action) => action(),
  });
  const view = render(createElement(ProjectTree, { controller: c }));
  fireEvent.contextMenu(view.getByRole('treeitem', { name: 'Types' }));
  expect(view.queryByRole('menu')).toBeNull();
  fireEvent.contextMenu(view.getByRole('treeitem', { name: 'Readonly' }));
  expect(view.queryByRole('menu')).toBeNull();
  const firstRow = view.getByRole('treeitem', { name: 'First' });
  firstRow.focus();
  fireEvent.contextMenu(firstRow.querySelector('.tree-name'));
  fireEvent.keyDown(await view.findByRole('menu'), { key: 'Escape' });
  await waitFor(() => expect(document.activeElement).toBe(firstRow));
  const secondRow = view.getByRole('treeitem', { name: 'Second' });
  fireEvent.contextMenu(secondRow.querySelector('.tree-name'));
  const menu = await view.findByRole('menu');
  fireEvent.click(within(menu).getByRole('menuitem', { name: /Remove/ }));
  expect(c.stageChange).toHaveBeenCalledWith({
    op: 'remove-instance',
    changeId: 'second',
    object: { kind: 'existing', objectId: 'second' },
    expectedShortName: 'Second',
  });
});

test.each(['removed', 'readonly', 'files'])(
  'a shared tree menu stays closed after an invalid %s target returns',
  async (change) => {
    const c = controllerFor({ actionReason: () => null });
    const view = render(createElement(ProjectTree, { controller: c }));
    fireEvent.contextMenu(view.getByRole('treeitem', { name: 'Can' }));
    await view.findByRole('menu');
    const invalid =
      change === 'files'
        ? { ...c, treeMode: 'files' }
        : {
            ...c,
            projection: {
              ...c.projection,
              objects:
                change === 'removed'
                  ? c.projection.objects.filter((item) => item.objectId !== 'can')
                  : c.projection.objects.map((item) =>
                      item.objectId === 'can' ? { ...item, writable: false } : item,
                    ),
            },
          };
    view.rerender(createElement(ProjectTree, { controller: invalid }));
    await waitFor(() => expect(view.queryByRole('menu')).toBeNull());
    view.rerender(createElement(ProjectTree, { controller: c }));
    expect(view.queryByRole('menu')).toBeNull();
    fireEvent.contextMenu(view.getByRole('treeitem', { name: 'Can' }));
    expect(await view.findByRole('menu')).toBeTruthy();
  },
);

test('touch long press resolves the row and rejects readonly objects', () => {
  vi.useFakeTimers();
  try {
    const writable = object('touch', null, 'IMPLEMENTATION-DATA-TYPE', 'Touch');
    const readonly = {
      ...object('locked', null, 'IMPLEMENTATION-DATA-TYPE', 'Locked'),
      writable: false,
    };
    const view = render(
      createElement(ProjectTree, {
        controller: controllerFor({
          projection: { objects: [writable, readonly], fields: [], sources: [], diagnostics: [] },
          actionReason: () => null,
        }),
      }),
    );
    const press = (name) => {
      const event = new Event('pointerdown', { bubbles: true, cancelable: true });
      Object.defineProperties(event, {
        pointerType: { value: 'touch' },
        clientX: { value: 10 },
        clientY: { value: 20 },
      });
      fireEvent(view.getByRole('treeitem', { name }).querySelector('.tree-name'), event);
      act(() => vi.advanceTimersByTime(700));
    };
    press('Locked');
    expect(view.queryByRole('menu')).toBeNull();
    press('Touch');
    expect(view.getByRole('menu')).toBeTruthy();
  } finally {
    cleanup();
    vi.useRealTimers();
  }
});

test('large trees bound mounted rows while keyboard and reveal reach the complete logical tree', () => {
  const many = Array.from({ length: 500 }, (_, i) =>
    object(`type${i}`, null, 'IMPLEMENTATION-DATA-TYPE', `Type${String(i).padStart(3, '0')}`),
  );
  const controller = controllerFor({
    projection: { objects: many, fields: [], sources: [], diagnostics: [] },
  });
  const view = render(createElement(ProjectTree, { controller }));
  const tree = view.getByRole('tree');
  expect(view.getAllByRole('treeitem').length).toBeLessThan(60);
  expect(tree.firstElementChild.style.height).toBe('15030px');
  const first = view.getByRole('treeitem', { name: 'Types' });
  act(() => first.focus());
  fireEvent.keyDown(first, { key: 'End' });
  const last = view.getByRole('treeitem', { name: 'Type499' });
  expect(document.activeElement).toBe(last);
  expect(last.getAttribute('aria-posinset')).toBe('500');
  expect(last.getAttribute('aria-setsize')).toBe('500');
  expect(tree.scrollTop).toBeGreaterThan(14000);
  fireEvent.keyDown(last, { key: 'ArrowUp' });
  expect(document.activeElement).toBe(view.getByRole('treeitem', { name: 'Type498' }));
  fireEvent.keyDown(document.activeElement, { key: 'ArrowLeft' });
  expect(document.activeElement).toBe(view.getByRole('treeitem', { name: 'Types' }));
  expect(tree.scrollTop).toBe(0);
  view.rerender(
    createElement(ProjectTree, { controller: { ...controller, treeRevealId: 'type400' } }),
  );
  expect(view.getByRole('treeitem', { name: 'Type400' })).toBeTruthy();
  expect(view.getAllByRole('treeitem').length).toBeLessThan(60);
});

test('scrolling retains focus and filtering a distant viewport still mounts its matches', () => {
  const many = Array.from({ length: 500 }, (_, i) =>
    object(`type${i}`, null, 'IMPLEMENTATION-DATA-TYPE', `Type${String(i).padStart(3, '0')}`),
  );
  const controller = controllerFor({
    projection: { objects: many, fields: [], sources: [], diagnostics: [] },
  });
  const view = render(createElement(ProjectTree, { controller }));
  const tree = view.getByRole('tree');
  const first = view.getByRole('treeitem', { name: 'Type000' });
  act(() => first.focus());
  fireEvent.scroll(tree, { target: { scrollTop: 14000 } });
  expect(document.activeElement).toBe(first);
  expect(view.getByRole('treeitem', { name: 'Type499' })).toBeTruthy();
  expect(view.getAllByRole('treeitem').length).toBeLessThan(60);
  view.rerender(
    createElement(ProjectTree, { controller: { ...controller, treeFilter: 'Type200' } }),
  );
  expect(view.getByRole('treeitem', { name: 'Type200' })).toBeTruthy();
  expect(view.getAllByRole('treeitem')).toHaveLength(2);
});
