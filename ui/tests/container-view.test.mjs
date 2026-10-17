// @vitest-environment jsdom
import { createElement } from 'react';
import { cleanup, fireEvent, render, within } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { localize, message, previewLanguage, translate } from '../src/i18n';
import { createBatchEditing } from '../src/workbench/batchEditing';
import {
  ConfigurationDocument,
  ContainerView,
  alternateBase,
  outsideRange,
} from '../src/workbench/ContainerView';
import { ObjectInspector } from '../src/workbench/ObjectEditor';
import { ProjectTree } from '../src/workbench/ProjectTree';
import { fieldState } from '../src/workbench/projectGroups';
import { initialState } from '../src/workbench/state';
import { withCommands } from './support/commands.mjs';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => false, invoke: vi.fn() }));

beforeEach(() => previewLanguage('en'));
afterEach(() => {
  cleanup();
  previewLanguage('system');
});

const object = (objectId, parentId, shortName, definitionId, kind = 'ECUC-CONTAINER-VALUE') => ({
  objectId,
  parentId,
  kind,
  shortName,
  definitionId,
  path: `/Ecuc/${shortName}`,
  sourceId: 'source',
  writable: true,
  reason: null,
});
const definition = (definitionId, lower, upper, extra = {}) => ({
  definitionId,
  elementKind: 'ECUC-PARAM-CONF-CONTAINER-DEF',
  kind: null,
  lowerMultiplicity: lower,
  upperMultiplicity: upper,
  unit: null,
  minimum: null,
  maximum: null,
  enumeration: [],
  defaultValue: null,
  defaultOrigin: null,
  referenceDestinations: [],
  writable: true,
  reason: null,
  ...extra,
});
const field = (fieldId, extra = {}) => ({
  ...definition(`/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/${fieldId}`, 1, 1, {
    elementKind: 'ECUC-NUMERICAL-PARAM-VALUE',
    kind: 'integer',
  }),
  fieldId,
  objectId: 'init',
  current: { state: 'explicit', value: { kind: 'integer', lexeme: '10' } },
  reference: null,
  ...extra,
});

const canIf = object(
  'canif',
  null,
  'CanIf',
  '/AUTOSAR/EcucDefs/CanIf',
  'ECUC-MODULE-CONFIGURATION-VALUES',
);
const init = object('init', 'canif', 'CanIfInitCfg', '/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg');
const buffer = object(
  'buf1',
  'init',
  'Buffer1',
  '/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfBufferCfg',
);
const period = field('Period', {
  minimum: '1',
  maximum: '65535',
  unit: 'ms',
  defaultValue: { kind: 'integer', lexeme: '5' },
  defaultOrigin: 'definition',
});
const locked = field('Locked', { writable: false, reason: 'Fixed by the profile' });
const choice = field('Mode', {
  kind: 'enumeration',
  enumeration: ['FAST', 'SLOW'],
  current: { state: 'absent' },
  upperMultiplicity: null,
  lowerMultiplicity: 0,
});

function controller(extra = {}) {
  return withCommands({
    ...initialState(),
    projection: {
      profile: 'p',
      release: 'R24-11',
      objects: [canIf, init, buffer],
      fields: [period, locked, choice],
      diagnostics: [],
      referenceCandidates: [],
      instanceDefinitions: [
        {
          parentId: 'init',
          definition: definition('/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfBufferCfg', 1, null),
          fields: [],
          children: [],
          referenceOptions: [],
          recursiveDefinitions: [],
        },
        {
          parentId: 'init',
          definition: definition('/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfInitHohCfg', 2, 3),
          fields: [],
          children: [],
          referenceOptions: [],
          recursiveDefinitions: [],
        },
      ],
      sources: [{ sourceId: 'source', path: '/project/ecuc.arxml' }],
      references: [],
    },
    activeObjectId: 'init',
    objectSelection: ['init'],
    savedValues: {
      Period: fieldState(period),
      Locked: fieldState(locked),
      Mode: fieldState(choice),
    },
    stageChange: vi.fn(),
    applyField: vi.fn(),
    discardChanges: vi.fn(),
    setActiveFieldId: vi.fn(),
    selectObject: vi.fn(),
    guardContext: vi.fn(),
    setInspectorTab: vi.fn(),
    ...extra,
  });
}

const valueChange = (fieldId, lexeme) => ({
  op: 'set-value',
  changeId: fieldId,
  field: { kind: 'existing', fieldId },
  expected: period.current,
  value: { state: 'explicit', value: { kind: 'integer', lexeme } },
});

test('integers accept decimal and 0x input for the range hint and the alternate base', () => {
  expect(alternateBase('255')).toBe('0xFF');
  expect(alternateBase('0x1f')).toBe('31');
  expect(alternateBase('-16')).toBe('-0x10');
  expect(alternateBase('0x')).toBeNull();
  const range = { minimum: '1', maximum: '65535' };
  expect(outsideRange('0x10000', range)).toBe(true);
  expect(outsideRange('0xFFFF', range)).toBe(false);
  expect(outsideRange('0', range)).toBe(true);
  // Exceeds Number precision; compared exactly.
  expect(
    outsideRange('18446744073709551616', { minimum: '0', maximum: '18446744073709551615' }),
  ).toBe(true);
});

test('the parameter grid shows definition facts, read-only values as text and the change marks', () => {
  const c = controller({ changes: [valueChange('Period', '0x20')] });
  const view = render(createElement(ContainerView, { controller: c }));
  const row = view.container.querySelector('[data-parameter-row="Period"]');
  const cells = [...row.children].map((cell) => cell.textContent);
  expect(cells.slice(2, 6)).toEqual(['ms', '5', '1..65535', '1..1']);
  expect(row.querySelector('input').value).toBe('0x20');
  expect(row.querySelector('.value-cell small').textContent).toBe('32');
  expect(row.className).toContain('unapplied');
  expect(within(row).getByText(translate('editor.container.unapplied'))).toBeTruthy();
  expect(within(row).getByText(translate('editor.container.nonDefault'))).toBeTruthy();
  // A read-only parameter is selectable text with its reason, not a disabled input.
  const lockedRow = view.container.querySelector('[data-parameter-row="Locked"]');
  expect(lockedRow.querySelector('input, select')).toBeNull();
  expect(lockedRow.textContent).toContain('Fixed by the profile');
  expect(view.container.querySelector('[data-parameter-row="Mode"]').textContent).toContain('0..*');
});

test('Enter applies only a staged cell, Escape reverts it, and focus names the current parameter', () => {
  const c = controller({ changes: [valueChange('Period', '20')] });
  const view = render(createElement(ContainerView, { controller: c }));
  const input = view.container.querySelector('[data-field-id="Period"]');
  fireEvent.focus(input);
  expect(c.setActiveFieldId).toHaveBeenCalledWith('Period');
  fireEvent.change(input, { target: { value: '21' } });
  expect(c.stageChange).toHaveBeenCalledWith(valueChange('Period', '21'));
  fireEvent.keyDown(input, { key: 'Enter' });
  expect(c.applyField).toHaveBeenCalledWith('Period');
  const escape = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
  const outside = vi.fn();
  document.addEventListener('keydown', outside);
  input.dispatchEvent(escape);
  document.removeEventListener('keydown', outside);
  expect(c.discardChanges).toHaveBeenCalledWith('Period');
  expect(outside).not.toHaveBeenCalled();
  // An unchanged cell lets Enter and Escape through.
  const mode = view.container.querySelector('[data-field-id="Mode"]');
  fireEvent.keyDown(mode, { key: 'Enter' });
  expect(c.applyField).toHaveBeenCalledTimes(1);
});

test('a busy apply keeps the input focused but read-only and ignores new input', () => {
  const c = controller({ busy: 'Applying', changes: [valueChange('Period', '20')] });
  const view = render(createElement(ContainerView, { controller: c }));
  const input = view.container.querySelector('[data-field-id="Period"]');
  expect(input.readOnly).toBe(true);
  expect(input.disabled).toBe(false);
  fireEvent.change(view.container.querySelector('[data-field-id="Mode"]'), {
    target: { value: 'FAST' },
  });
  expect(c.stageChange).not.toHaveBeenCalled();
});

test('the cell shows its rejection, links it for assistive technology and flags out-of-range input', () => {
  const rejection = message('workflow.error.fieldRange', {
    reason: 'x',
    minimum: '1',
    maximum: '65535',
  });
  const c = controller({ fieldErrors: { Period: rejection } });
  const view = render(createElement(ContainerView, { controller: c }));
  const input = view.container.querySelector('[data-field-id="Period"]');
  expect(input.getAttribute('aria-invalid')).toBe('true');
  const error = view.container.querySelector('#field-error-Period');
  expect(input.getAttribute('aria-describedby')).toBe(error.id);
  expect(error.textContent).toContain('1–65535');

  view.rerender(createElement(ContainerView, { controller: controller() }));
  expect(input.getAttribute('aria-invalid')).toBe('false');
  expect(view.container.querySelector('#field-error-Period')).toBeNull();

  view.rerender(
    createElement(ContainerView, {
      controller: controller({ changes: [valueChange('Period', '0x10000')] }),
    }),
  );
  expect(input.getAttribute('aria-invalid')).toBe('true');
  expect(view.container.querySelector('#field-error-Period').textContent).toContain(
    translate('editor.objects.outOfRange', { minimum: '1', maximum: '65535' }),
  );
});

test('the sub-container table groups children by definition with multiplicity and missing rows', () => {
  const c = controller();
  const view = render(createElement(ContainerView, { controller: c }));
  const table = view.container.querySelector('.child-table');
  const groups = [...table.querySelectorAll('.row-group th')].map((cell) => cell.textContent);
  expect(groups[0]).toContain('CanIfBufferCfg');
  expect(groups[0]).toContain('1..*');
  expect(groups[1]).toContain('CanIfInitHohCfg');
  expect(groups[1]).toContain('0 · multiplicity 2..3');
  expect(table.querySelector('.row-missing').textContent).toContain('2 CanIfInitHohCfg missing');
  fireEvent.click(within(table).getByRole('button', { name: 'Buffer1' }));
  expect(c.selectObject).toHaveBeenCalledWith('buf1');
  // Adding stages a creation under the current object; the inspector names it.
  fireEvent.click(within(table).getByRole('button', { name: 'Add CanIfInitHohCfg' }));
  expect(c.stageChange).toHaveBeenCalledWith(
    expect.objectContaining({
      op: 'create-instance',
      parent: { kind: 'existing', objectId: 'init' },
      definitionId: '/AUTOSAR/EcucDefs/CanIf/CanIfInitCfg/CanIfInitHohCfg',
    }),
  );
});

test('a full child definition offers no add action', () => {
  const full = controller();
  full.projection.instanceDefinitions[1].definition.upperMultiplicity = 0;
  const view = render(createElement(ContainerView, { controller: full }));
  expect(view.queryByRole('button', { name: 'Add CanIfInitHohCfg' })).toBeNull();
  expect(view.getByRole('button', { name: 'Add CanIfBufferCfg' })).toBeTruthy();
});

test('without a current object the document shows the module overview', () => {
  const c = controller({ activeObjectId: null, objectSelection: [] });
  const view = render(createElement(ContainerView, { controller: c }));
  const row = view.container.querySelector('[data-object-row="canif"]');
  expect([...row.children].map((cell) => cell.textContent).slice(0, 3)).toEqual([
    'CanIf',
    'CanIf',
    '2',
  ]);
  fireEvent.click(within(row).getByRole('button', { name: 'CanIf' }));
  expect(c.selectObject).toHaveBeenCalledWith('canif');
});

test('all objects mark the current row and check boxes only for a batch selection', () => {
  const view = render(createElement(ConfigurationDocument, { controller: controller() }));
  fireEvent.click(view.getByRole('tab', { name: translate('editor.container.allObjects') }));
  const current = view.container.querySelector('[data-object-row="init"]');
  expect(current.getAttribute('aria-current')).toBe('true');
  expect(current.querySelector('input[type=checkbox]').checked).toBe(false);
  view.rerender(
    createElement(ConfigurationDocument, {
      controller: controller({ objectSelection: ['init', 'buf1'] }),
    }),
  );
  expect(view.container.querySelector('[data-object-row="buf1"] input').checked).toBe(true);
  expect(
    view.container.querySelector('[data-object-row="buf1"]').hasAttribute('aria-current'),
  ).toBe(false);
});

test('the inspector describes the focused parameter without editing it', () => {
  const view = render(
    createElement(ObjectInspector, { controller: controller({ activeFieldId: 'Period' }) }),
  );
  const details = view.getByRole('region', { name: 'Period' });
  expect(details.textContent).toContain('1..1');
  expect(details.textContent).toContain('1..65535');
  expect(details.textContent).toContain('5 · definition');
  expect(details.querySelector('input, select')).toBeNull();
  expect(view.container.querySelector('[data-field-id]')).toBeNull();
});

test('the tree context menu offers allowed children, rename and removal', async () => {
  const c = controller({ setTreeRevealId: vi.fn() });
  const view = render(createElement(ProjectTree, { controller: c }));
  fireEvent.contextMenu(view.getByRole('treeitem', { name: 'CanIfInitCfg' }));
  const menu = within(await view.findByRole('menu'));
  expect(menu.getAllByRole('menuitem').map((item) => item.textContent)).toEqual([
    'Add CanIfBufferCfg',
    'Add CanIfInitHohCfg',
    'Rename',
    translate('editor.objects.removeInstance'),
  ]);
  fireEvent.click(menu.getByRole('menuitem', { name: translate('editor.objects.removeInstance') }));
  expect(c.stageChange).toHaveBeenCalledWith({
    op: 'remove-instance',
    changeId: 'init',
    object: { kind: 'existing', objectId: 'init' },
    expectedShortName: 'CanIfInitCfg',
  });
  cleanup();
  // While changes cannot be prepared, the items stay readable but do nothing.
  const busy = controller({ setTreeRevealId: vi.fn(), actionReason: () => 'Busy' });
  const blocked = render(createElement(ProjectTree, { controller: busy }));
  fireEvent.contextMenu(blocked.getByRole('treeitem', { name: 'CanIfInitCfg' }));
  const items = within(await blocked.findByRole('menu')).getAllByRole('menuitem');
  expect(items.every((item) => item.getAttribute('aria-disabled') === 'true')).toBe(true);
  fireEvent.click(items[0]);
  expect(busy.stageChange).not.toHaveBeenCalled();
});

test('an atomic component node starts user application initialization from its menu', async () => {
  const component = {
    ...object('swc', null, 'EchoApplication', null, 'APPLICATION-SW-COMPONENT-TYPE'),
    writable: false,
  };
  const c = controller({
    setTreeRevealId: vi.fn(),
    workspace: { dirty: false, integrationCandidate: true },
    previewApplicationInitialization: vi.fn(),
  });
  c.projection = { ...c.projection, objects: [...c.projection.objects, component] };
  const view = render(createElement(ProjectTree, { controller: withCommands(c) }));
  fireEvent.contextMenu(view.getByRole('treeitem', { name: 'EchoApplication' }));
  const menu = within(await view.findByRole('menu'));
  // A read-only component offers only the initialization, not rename or removal.
  expect(menu.getAllByRole('menuitem').map((item) => item.textContent)).toEqual([
    translate('action.initializeApplication'),
  ]);
  fireEvent.click(menu.getByRole('menuitem'));
  expect(c.previewApplicationInitialization).toHaveBeenCalled();
  cleanup();
  // Without integration slots the item explains why it cannot run.
  const blocked = render(
    createElement(ProjectTree, {
      controller: withCommands({ ...c, workspace: { dirty: false, integrationCandidate: false } }),
    }),
  );
  fireEvent.contextMenu(blocked.getByRole('treeitem', { name: 'EchoApplication' }));
  const item = within(await blocked.findByRole('menu')).getByRole('menuitem');
  expect(item.getAttribute('aria-disabled')).toBe('true');
  expect(document.getElementById(item.getAttribute('aria-describedby')).textContent).toBe(
    translate('shell.reason.noSlots'),
  );
});

test('the tree menu reads availability when it opens, not when the rows were memoized', async () => {
  const c = controller({ setTreeRevealId: vi.fn() });
  const view = render(createElement(ProjectTree, { controller: c }));
  // Same projection and drafts: the memoized rows are reused, only availability changes.
  const busy = { ...c, actionReason: () => 'Busy', stageChange: vi.fn() };
  view.rerender(createElement(ProjectTree, { controller: busy }));
  fireEvent.contextMenu(view.getByRole('treeitem', { name: 'CanIfInitCfg' }));
  const items = within(await view.findByRole('menu')).getAllByRole('menuitem');
  expect(items.every((item) => item.getAttribute('aria-disabled') === 'true')).toBe(true);
});

test('Enter while an input method composes text does not apply', () => {
  const c = controller({ changes: [valueChange('Period', '20')] });
  const view = render(createElement(ContainerView, { controller: c }));
  fireEvent.keyDown(view.container.querySelector('[data-field-id="Period"]'), {
    key: 'Enter',
    isComposing: true,
  });
  expect(c.applyField).not.toHaveBeenCalled();
});

test('applying one field keeps the rejection notice of another field', async () => {
  const projection = { workspaceEpoch: 'e', definitionFingerprint: 'd', fields: [period, choice] };
  const notice = { tone: 'error', text: 'Mode rejected', fieldId: 'Mode' };
  const current = session({ projection, changes: [valueChange('Period', '20')], notice });
  let seen;
  current.call.mockImplementation(async () => {
    seen = current.stateRef.current.notice;
    throw new Error('stop');
  });
  await batch(current).applyField('Period');
  expect(seen).toBe(notice);
});

function session(overrides = {}) {
  const stateRef = { current: { ...initialState(), ...overrides } };
  const result = {
    stateRef,
    capabilitiesRef: { current: { fingerprint: 'input' } },
    epoch: { current: 1 },
    running: { current: false },
    changeConfirmation: { current: null },
    call: vi.fn(),
    patchState(patch) {
      stateRef.current = { ...stateRef.current, ...patch };
    },
    field(key) {
      return (value) => result.patchState({ [key]: value });
    },
  };
  return result;
}

const batch = (current) =>
  createBatchEditing(current, { invalidateAfterEdit: vi.fn(), acceptIntegration: vi.fn() });

test('applying one field prepares and applies only its change and keeps the other drafts', async () => {
  const mine = valueChange('Period', '20');
  const other = valueChange('Mode', 'FAST');
  const projection = { workspaceEpoch: 'e', definitionFingerprint: 'd', fields: [period, choice] };
  const current = session({ projection, changes: [mine, other], fieldErrors: { Mode: 'kept' } });
  const confirmation = vi.fn();
  current.changeConfirmation.current = confirmation;
  current.call.mockImplementation(async (command) =>
    command === 'prepare_configuration_change'
      ? { changeRevision: 'r1' }
      : command === 'apply_configuration_change'
        ? { projection: { ...projection, objects: [], dirty: true }, createdIds: [] }
        : { name: 'Ecu', dirty: true },
  );
  expect(await batch(current).applyField('Period')).toBe(true);
  const changeSet = {
    workspaceEpoch: 'e',
    inputFingerprint: 'input',
    definitionFingerprint: 'd',
    changes: [mine],
  };
  expect(current.call.mock.calls.slice(0, 2)).toEqual([
    ['prepare_configuration_change', { changeSet }],
    ['apply_configuration_change', { changeSet, changeRevision: 'r1' }],
  ]);
  expect(current.stateRef.current.changes).toEqual([other]);
  expect(current.stateRef.current.fieldErrors).toEqual({ Mode: 'kept' });
  expect(current.stateRef.current.notice.text).toEqual(
    message('workflow.batch.fieldApplied', { field: 'Period' }),
  );
  // The batch confirmation belongs to the preview dialog, not to a single apply.
  expect(confirmation).not.toHaveBeenCalled();
});

test('a rejected field apply keeps its draft and attaches the reason to the cell', async () => {
  const mine = valueChange('Period', '20');
  const projection = { workspaceEpoch: 'e', definitionFingerprint: 'd', fields: [period] };
  const current = session({ projection, changes: [mine] });
  current.call.mockRejectedValue(message('backend.workbench.cancel_first'));
  expect(await batch(current).applyField('Period')).toBe(false);
  const state = current.stateRef.current;
  expect(state.changes).toEqual([mine]);
  expect(state.fieldErrors.Period).toBeTruthy();
  expect(state.notice).toMatchObject({ tone: 'error', fieldId: 'Period' });
  expect(localize(state.notice.text)).toContain('Period');
  expect(current.call).toHaveBeenCalledTimes(1);
});
