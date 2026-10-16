// @vitest-environment jsdom
import React from 'react';
import { act, cleanup, fireEvent, render, within } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { localize, message, previewLanguage, translate } from '../src/i18n';
import { createBatchEditing, fieldRejection } from '../src/workbench/batchEditing';
import { createDeliveryActions } from '../src/workbench/deliveryActions';
import { createSettingsActions } from '../src/workbench/settingsActions';
import { CopyText, Dialog } from '../src/workbench/Dialog';
import { DescriptorField } from '../src/workbench/ObjectEditor';
import { SettingsPage } from '../src/pages/SettingsPage';
import { headline } from '../src/workbench/forms';
import { workspaceTitle } from '../src/workbench/labels';
import { initialState } from '../src/workbench/state';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => false, invoke: vi.fn() }));

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  previewLanguage('system');
});

function session(overrides = {}) {
  const stateRef = { current: { ...initialState(), ...overrides } };
  const result = {
    get state() {
      return stateRef.current;
    },
    stateRef,
    capabilitiesRef: { current: { fingerprint: 'before' } },
    epoch: { current: 1 },
    running: { current: false },
    pendingGuard: { current: null },
    pendingReplacement: { current: null },
    changeConfirmation: { current: null },
    call: vi.fn(),
    patchState(patch) {
      stateRef.current = { ...stateRef.current, ...patch };
    },
    field(key) {
      return (value) =>
        result.patchState({
          [key]: typeof value === 'function' ? value(stateRef.current[key]) : value,
        });
    },
  };
  return result;
}

const rangeField = {
  fieldId: 'field-period',
  objectId: 'object',
  definitionId: '/AUTOSAR/EcucDefs/Com/ComConfig/ComTimeBase/ComMainFunctionPeriod',
  kind: 'integer',
  elementKind: 'ECUC-NUMERICAL-PARAM-VALUE',
  unit: null,
  minimum: '1',
  maximum: '65535',
  enumeration: [],
  lowerMultiplicity: 1,
  upperMultiplicity: 1,
  defaultValue: null,
  defaultOrigin: null,
  writable: true,
  reason: null,
  current: { state: 'explicit', value: { kind: 'integer', lexeme: '10' } },
  reference: null,
};

const valueChange = (lexeme) => ({
  op: 'set-value',
  changeId: rangeField.fieldId,
  field: { kind: 'existing', fieldId: rangeField.fieldId },
  expected: rangeField.current,
  value: { state: 'explicit', value: { kind: 'integer', lexeme } },
});

// The shape `apply_configuration_change` returns for a value the definition rejects.
const fieldRejected = [
  message('backend.arxml.changes.field_change_context', {
    change_id: rangeField.fieldId,
    field: JSON.stringify({ kind: 'existing', fieldId: rangeField.fieldId }),
  }),
  message('backend.definitions.validation.integer_out_of_range'),
];

function batch(current) {
  return createBatchEditing(current, { invalidateAfterEdit: vi.fn(), acceptIntegration: vi.fn() });
}

test('a rejected value is reported on its field with name and range, and retracted by editing it', async () => {
  previewLanguage('zh-CN');
  const current = session({
    projection: { workspaceEpoch: 1, definitionFingerprint: 'defs', fields: [rangeField] },
    changes: [valueChange('0')],
  });
  current.call.mockRejectedValue(fieldRejected);
  await batch(current).prepareChanges();
  const { notice, fieldErrors } = current.stateRef.current;
  expect(notice).toMatchObject({ tone: 'error', fieldId: rangeField.fieldId });
  expect(localize(notice.text)).toContain('ComMainFunctionPeriod');
  expect(localize(notice.text)).not.toContain('changeId');
  expect(localize(fieldErrors[rangeField.fieldId])).toContain('1–65535');
  // The raw backend evidence stays available for copying.
  expect(localize(notice.detail)).toContain('changeId=field-period');

  batch(current).stageChange(valueChange('20'));
  expect(current.stateRef.current.fieldErrors).toEqual({});
  expect(current.stateRef.current.notice).toBeNull();
});

test('a rejection of another field stays when an unrelated draft changes', async () => {
  const other = {
    ...valueChange('5'),
    changeId: 'other',
    field: { kind: 'existing', fieldId: 'other' },
  };
  const current = session({
    projection: { workspaceEpoch: 1, definitionFingerprint: 'defs', fields: [rangeField] },
    changes: [valueChange('0')],
  });
  current.call.mockRejectedValue(fieldRejected);
  await batch(current).prepareChanges();
  batch(current).stageChange(other);
  expect(Object.keys(current.stateRef.current.fieldErrors)).toEqual([rangeField.fieldId]);
  expect(current.stateRef.current.notice).not.toBeNull();
});

test('errors without a field context keep the generic message and are not attached to a field', () => {
  expect(fieldRejection(message('backend.workbench.cancel_first'), [rangeField])).toBeNull();
  expect(
    fieldRejection(
      [
        message('backend.arxml.changes.field_change_context', {
          change_id: 'x',
          field: JSON.stringify({ kind: 'existing', fieldId: 'unknown' }),
        }),
        'reason',
      ],
      [rangeField],
    ),
  ).toBeNull();
});

test('a new preview clears the previous rejection before the backend answers', async () => {
  const current = session({
    projection: { workspaceEpoch: 1, definitionFingerprint: 'defs', fields: [rangeField] },
    changes: [valueChange('0')],
    notice: { tone: 'error', text: 'old failure' },
    fieldErrors: { [rangeField.fieldId]: 'old' },
  });
  let seen;
  current.call.mockImplementation(async () => {
    seen = {
      notice: current.stateRef.current.notice,
      fieldErrors: current.stateRef.current.fieldErrors,
    };
    return { changeRevision: 'preview' };
  });
  await batch(current).prepareChanges();
  expect(seen).toEqual({ notice: null, fieldErrors: {} });
});

test('an applied batch reports how many changes it applied', async () => {
  const changes = [valueChange('20')];
  const current = session({
    changes,
    changePreview: { changeRevision: 'preview' },
    preparedChangeSet: { changes },
  });
  current.call
    .mockResolvedValueOnce({ projection: { objects: [], dirty: true }, createdIds: [] })
    .mockResolvedValue({ name: 'Ecu', dirty: true, files: [], issues: [] });
  expect(await batch(current).applyChanges()).toBe(true);
  expect(current.stateRef.current.notice).toEqual({
    tone: 'success',
    text: message('workflow.batch.applied', { count: 1 }),
  });
});

test('the field shows the rejection, links it for assistive technology and flags out-of-range input', () => {
  previewLanguage('zh-CN');
  const controller = {
    ...initialState(),
    actionReason: () => null,
    stageChange: vi.fn(),
    fieldErrors: {
      [rangeField.fieldId]: message('workflow.error.fieldRange', {
        reason: 'x',
        minimum: '1',
        maximum: '65535',
      }),
    },
  };
  const view = render(React.createElement(DescriptorField, { field: rangeField, controller }));
  const input = view.container.querySelector('input');
  expect(input.getAttribute('aria-invalid')).toBe('true');
  const error = view.container.querySelector(`#field-error-${rangeField.fieldId}`);
  expect(input.getAttribute('aria-describedby').split(' ')).toContain(error.id);
  expect(error.textContent).toContain('1–65535');

  const clean = { ...controller, fieldErrors: {} };
  view.rerender(React.createElement(DescriptorField, { field: rangeField, controller: clean }));
  expect(input.getAttribute('aria-invalid')).toBe('false');
  expect(view.container.querySelector('.field-error')).toBeNull();

  const staged = { ...clean, changes: [valueChange('70000')] };
  view.rerender(React.createElement(DescriptorField, { field: rangeField, controller: staged }));
  expect(input.getAttribute('aria-invalid')).toBe('true');
  expect(view.container.querySelector('.field-error').textContent).toContain(
    translate('editor.objects.outOfRange', { minimum: '1', maximum: '65535' }),
  );
});

function delivery(current, overrides = {}) {
  const dependencies = {
    run: async (_operation, job, onSuccess) => onSuccess(await job()),
    acceptView: vi.fn(),
    markStage: vi.fn(),
    continueReplacement: vi.fn(),
    unapplied: false,
    acceptIntegration: vi.fn(),
    confirmAction: vi.fn(),
    ...overrides,
  };
  return { actions: createDeliveryActions(current, dependencies), dependencies };
}

test('saving without changes says so instead of opening an empty confirmation', async () => {
  const current = session();
  current.call.mockResolvedValue({
    revision: 'r',
    files: [{ path: 'ecuc.arxml', changed: false }],
  });
  delivery(current).actions.saveProject();
  await vi.waitFor(() => expect(current.stateRef.current.notice).not.toBeNull());
  expect(current.stateRef.current.savePreview).toBeNull();
  expect(current.stateRef.current.notice).toEqual({
    tone: 'info',
    text: message('workflow.save.noChanges'),
  });
});

test('a confirmed save reports the number of files it wrote', async () => {
  const current = session({
    savePreview: {
      revision: 'r',
      files: [
        { path: 'a.arxml', changed: true },
        { path: 'b.arxml', changed: false },
      ],
    },
  });
  current.call.mockResolvedValue({ workspace: { dirty: false, issues: [] }, error: null });
  delivery(current).actions.confirmSave();
  await vi.waitFor(() => expect(current.stateRef.current.notice?.tone).toBe('success'));
  expect(current.stateRef.current.notice.text).toEqual(
    message('workflow.save.saved', { count: 1 }),
  );
});

test('validation summarises automatic and explicit findings once and opens problems only when found', async () => {
  const error = { severity: 'error', code: 'E', message: 'm', path: '/A' };
  const warning = { severity: 'warning', code: 'W', message: 'm', path: '/B' };
  const current = session();
  current.call.mockImplementation(async (command) =>
    command === 'validate_project'
      ? { dirty: false, issues: [warning] }
      : { diagnostics: [error, error] },
  );
  const { actions, dependencies } = delivery(current);
  actions.validateProject();
  await vi.waitFor(() => expect(current.stateRef.current.notice).not.toBeNull());
  expect(current.stateRef.current.notice).toEqual({
    tone: 'error',
    text: message('workflow.validation.summary', { errors: 1, warnings: 1 }),
  });
  expect(dependencies.markStage).toHaveBeenCalledWith(
    'validate',
    'failed',
    message('workflow.stage.blockingErrors', { count: 1 }),
  );
  expect(current.stateRef.current.toolWindow).toBe('problems');

  const clean = session();
  clean.call.mockImplementation(async (command) =>
    command === 'validate_project' ? { dirty: false, issues: [] } : { diagnostics: [] },
  );
  delivery(clean).actions.validateProject();
  await vi.waitFor(() => expect(clean.stateRef.current.notice?.tone).toBe('success'));
  expect(clean.stateRef.current.toolWindow).toBeNull();
});

test('a rejected tool path is attached to the field holding it', async () => {
  const current = session({
    toolDraft: { compiler: '/missing/gcc', objdump: '', git: '', python: '' },
  });
  current.call.mockRejectedValue(
    message('backend.workbench.configured_tool_path_invalid', { path: '/missing/gcc' }),
  );
  await createSettingsActions(current, { invalidateOperation: vi.fn() }).configureTools();
  const state = current.stateRef.current;
  expect(Object.keys(state.toolErrors)).toEqual(['compiler']);
  expect(state.settingsNotice).toMatchObject({
    tone: 'error',
    text: message('controller.settings.toolsRejected'),
  });

  current.call.mockResolvedValue(undefined);
  await createSettingsActions(current, { invalidateOperation: vi.fn() }).configureTools();
  expect(current.stateRef.current.toolErrors).toEqual({});
  expect(current.stateRef.current.settingsNotice.tone).toBe('success');
});

test('the settings page marks the rejected tool and clears the mark when the path changes', () => {
  previewLanguage('en');
  const setToolErrors = vi.fn();
  const controller = {
    ...initialState(),
    settingsOpen: true,
    native: true,
    capabilities: { environmentOverrides: [], executionTools: null, toolError: null },
    toolDraft: { compiler: '/missing/gcc', objdump: '', git: '', python: '' },
    toolErrors: {
      compiler: message('backend.workbench.configured_tool_path_invalid', { path: '/missing/gcc' }),
    },
    setToolDraft: vi.fn(),
    setToolErrors,
  };
  const view = render(React.createElement(SettingsPage, { controller }));
  fireEvent.click(view.getByRole('tab', { name: translate('shell.settings.tools') }));
  const input = view.container.querySelector('[aria-invalid="true"]');
  expect(input).not.toBeNull();
  const error = view.container.querySelector(`#${input.getAttribute('aria-describedby')}`);
  expect(error.textContent).toContain('/missing/gcc');
  fireEvent.change(input, { target: { value: '/usr/bin/gcc' } });
  expect(setToolErrors).toHaveBeenCalledWith({ compiler: undefined });
});

test('dialogs report operation results inside the dialog with copyable detail', async () => {
  previewLanguage('en');
  const clipboard = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal('navigator', { ...navigator, clipboard: { writeText: clipboard } });
  const view = render(
    React.createElement(
      Dialog,
      {
        title: 'Preview',
        onClose: vi.fn(),
        notice: { tone: 'error', text: 'Save failed', detail: 'raw backend detail' },
        footer: React.createElement('button', { type: 'button' }, 'Confirm'),
      },
      'body',
    ),
  );
  const dialog = view.getByRole('dialog');
  const alert = within(dialog).getByRole('alert');
  expect(alert.textContent).toContain('Save failed');
  expect(alert.textContent).toContain('raw backend detail');
  await act(async () => {
    fireEvent.click(
      within(alert).getByRole('button', { name: translate('editor.common.copyDetails') }),
    );
  });
  expect(clipboard).toHaveBeenCalledWith('raw backend detail');
});

test('copying confirms success visibly and announces it', async () => {
  previewLanguage('zh-CN');
  vi.useFakeTimers();
  try {
    vi.stubGlobal('navigator', {
      ...navigator,
      clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
    const view = render(React.createElement(CopyText, { text: 'x', label: '复制详情' }));
    const button = view.getByRole('button', { name: '复制详情' });
    await act(async () => {
      fireEvent.click(button);
    });
    expect(button.textContent).toBe(translate('editor.dialog.copied'));
    expect(view.getByRole('status').textContent).toBe(translate('editor.dialog.copied'));
    act(() => vi.advanceTimersByTime(2000));
    expect(button.textContent).toBe('复制详情');
  } finally {
    vi.useRealTimers();
  }
});

test('a context prefix ending in a colon keeps its first reason in the headline', () => {
  previewLanguage('zh-CN');
  const text = headline([
    message('backend.arxml.application.initialization_failed'),
    'denied',
    'more',
  ]);
  expect(localize(text)).toContain('denied');
  expect(localize(text)).not.toContain('more');
  expect(localize(headline([message('workflow.save.noChanges'), 'detail']))).not.toContain(
    'detail',
  );
});

test('a direct ARXML import is marked as not saved as a project', () => {
  previewLanguage('zh-CN');
  const workspace = { name: 'ecuc.arxml' };
  expect(workspaceTitle({ workspace, memberProjectPath: '/p/project.autosar' })).toBe('ecuc.arxml');
  expect(workspaceTitle({ workspace, memberProjectPath: null })).toBe(
    translate('shell.app.unsavedImport', { name: 'ecuc.arxml' }),
  );
});

test('the delivery page reports the latest result of generate, build and host verification', async () => {
  previewLanguage('zh-CN');
  const { IntegrationDelivery } = await import('../src/IntegrationDelivery');
  const base = {
    ...initialState(),
    workspace: { name: 'Ecu', dirty: false, files: [], issues: [] },
    native: true,
    unapplied: false,
    capabilities: { nativeExecution: true, toolError: null, ruleError: null, targets: [] },
    actionReason: () => null,
    applicationWarnings: [],
    applicationRecoveryFiles: [],
  };
  const guidance = (view) =>
    view.container.querySelector('.page-guidance [role="status"]').textContent;
  const view = render(React.createElement(IntegrationDelivery, { controller: base, active: true }));
  expect(guidance(view)).toBe(translate('shell.delivery.directoryHint'));
  const generated = { outputDirectory: '/out', files: [{}, {}] };
  const steps = [
    [{ generated }, translate('shell.delivery.generated', { count: 2, directory: '/out' })],
    [
      { generated, built: { binaryPath: '/build/ecu', log: '' } },
      translate('shell.delivery.built', { path: '/build/ecu' }),
    ],
    [
      {
        generated,
        built: { binaryPath: '/build/ecu', log: '' },
        virtualResult: { passed: true, log: '' },
      },
      translate('shell.delivery.verified'),
    ],
  ];
  for (const [state, expected] of steps) {
    view.rerender(
      React.createElement(IntegrationDelivery, { controller: { ...base, ...state }, active: true }),
    );
    expect(guidance(view)).toBe(expected);
  }
  const failed = {
    ...base,
    generated,
    stages: { ...base.stages, build: { state: 'failed', detail: 'cc: not found' } },
  };
  view.rerender(React.createElement(IntegrationDelivery, { controller: failed, active: true }));
  expect(view.container.querySelector('.page-guidance.error').textContent).toContain(
    'cc: not found',
  );
});
