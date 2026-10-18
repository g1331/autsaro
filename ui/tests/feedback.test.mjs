// @vitest-environment jsdom
import React from 'react';
import { act, cleanup, fireEvent, render, within } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { composedMessage, localize, message, previewLanguage, translate } from '../src/i18n';
import { createBatchEditing, fieldRejection } from '../src/workbench/batchEditing';
import { createDeliveryActions } from '../src/workbench/deliveryActions';
import { createSettingsActions } from '../src/workbench/settingsActions';
import { CopyText, Dialog } from '../src/workbench/Dialog';
import { SettingsPage } from '../src/pages/SettingsPage';
import { headline } from '../src/workbench/forms';
import { workspaceTitle } from '../src/workbench/labels';
import { initialState } from '../src/workbench/state';
import { withCommands } from './support/commands.mjs';

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

const projection = {
  workspaceEpoch: 1,
  definitionFingerprint: 'defs',
  fields: [rangeField],
  objects: [{ objectId: 'object', path: '/Com/ComConfig/ComTimeBase', sourceId: 'source' }],
  sources: [{ sourceId: 'source', path: 'Com.arxml' }],
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
    projection,
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
  expect(localize(notice.detail)).toContain('field-period');
  // The details in the problems window name the object, the parameter and its file.
  expect(notice.rejected).toBe(true);
  expect(current.stateRef.current.rejection.items).toMatchObject([
    {
      severity: 'error',
      path: '/Com/ComConfig/ComTimeBase · ComMainFunctionPeriod',
      file: 'Com.arxml',
      objectId: 'object',
      fieldId: rangeField.fieldId,
    },
  ]);

  batch(current).stageChange(valueChange('20'));
  expect(current.stateRef.current.fieldErrors).toEqual({});
  expect(current.stateRef.current.notice).toBeNull();
  expect(current.stateRef.current.rejection).toBeNull();
});

test('definition range rejections retain their bounds in both languages', () => {
  for (const language of ['zh-CN', 'en']) {
    previewLanguage(language);
    for (const kind of ['integer', 'float']) {
      const reason = message(`backend.definitions.validation.${kind}_out_of_range`);
      const result = fieldRejection([fieldRejected[0], reason], [rangeField]);
      expect(localize(result.message)).toBe(
        translate('workflow.error.fieldRange', {
          reason: localize(reason),
          minimum: '1',
          maximum: '65535',
        }),
      );
    }
  }
});

test('Counter period rejections show only the backend reason in both languages', () => {
  const period = { ...rangeField, minimum: '0', maximum: null };
  const reason = message('backend.integration.editor.application_period_invalid');
  for (const language of ['zh-CN', 'en']) {
    previewLanguage(language);
    const result = fieldRejection([fieldRejected[0], reason], [period]);
    expect(localize(result.message)).toBe(localize(reason));
  }
});

test('composed rejections end in exactly one sentence mark whether or not the backend reason has one', () => {
  const expected = {
    'zh-CN': {
      reasons: ['取值超出范围', '取值超出范围。'],
      cell: 'PERIOD 未应用，原配置保持：取值超出范围。修正后按 Enter 重新应用。',
      batch: '更改被拒绝，原配置保持。PERIOD：取值超出范围。修正后重新预览。',
      range: '取值超出范围（允许范围 1–65535）',
      ranged:
        'PERIOD 未应用，原配置保持：取值超出范围（允许范围 1–65535）。修正后按 Enter 重新应用。',
      tool: '尚未配置执行工具（找不到编译器）：',
      detail: '找不到编译器。',
    },
    en: {
      reasons: ['Value is out of range', 'Value is out of range.'],
      cell: 'PERIOD was not applied; the configuration is unchanged: Value is out of range. Correct it and press Enter to apply again.',
      batch:
        'Changes rejected; the configuration is unchanged. PERIOD: Value is out of range. Correct it and preview again.',
      range: 'Value is out of range (allowed range 1–65535)',
      ranged:
        'PERIOD was not applied; the configuration is unchanged: Value is out of range (allowed range 1–65535). Correct it and press Enter to apply again.',
      tool: 'Execution tools are not configured (Compiler not found).',
      detail: 'Compiler not found.',
    },
  };
  const params = { field: 'PERIOD' };
  for (const language of ['zh-CN', 'en']) {
    previewLanguage(language);
    const want = expected[language];
    for (const reason of want.reasons) {
      const range = composedMessage(
        'workflow.error.fieldRange',
        { reason },
        { minimum: '1', maximum: '65535' },
      );
      expect(localize(range)).toBe(want.range);
      expect(
        localize(composedMessage('workflow.error.fieldRejected', { error: reason }, params)),
      ).toBe(want.cell);
      expect(
        localize(composedMessage('workflow.error.batchFieldRejected', { error: reason }, params)),
      ).toBe(want.batch);
      // The bracketed range stays before the single closing mark of the notice.
      expect(
        localize(composedMessage('workflow.error.fieldRejected', { error: range }, params)),
      ).toBe(want.ranged);
    }
    for (const detail of [want.detail, want.detail.slice(0, -1)]) {
      expect(translate('shell.settings.toolMissing', { detail })).toContain(want.tool);
    }
  }
});

test('a rejection of another field stays when an unrelated draft changes', async () => {
  const other = {
    ...valueChange('5'),
    changeId: 'other',
    field: { kind: 'existing', fieldId: 'other' },
  };
  const current = session({
    projection,
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
    projection,
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

function delivery(current, overrides = {}) {
  const dependencies = {
    run: async (_operation, job, onSuccess) => onSuccess(await job()),
    acceptView: vi.fn(),
    markStage: vi.fn(),
    continueReplacement: vi.fn(),
    unapplied: false,
    acceptIntegration: vi.fn(),
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

function settingsWithResults() {
  const current = session({
    generated: { files: ['output.c'] },
    built: { binaryPath: '/build/ecu' },
    virtualResult: { passed: true },
    generationPreview: { revision: 'owned' },
    notice: { tone: 'success', text: 'previous result' },
    stages: Object.fromEntries(
      Object.keys(initialState().stages).map((key) => [
        key,
        { state: 'done', detail: `previous ${key}` },
      ]),
    ),
  });
  const invalidateOperation = vi.fn(() => current.epoch.current++);
  return {
    current,
    invalidateOperation,
    actions: createSettingsActions(current, { invalidateOperation }),
  };
}

test('target selection invalidates the old operation and clears each previous output on success', async () => {
  const { current, actions, invalidateOperation } = settingsWithResults();
  const before = current.state;
  current.call.mockImplementation(async () => {
    expect(invalidateOperation).toHaveBeenCalledExactlyOnceWith();
    expect(current.state).toBe(before);
  });
  await actions.changeTarget('linux-x64-controlled-v1');
  expect(current.call).toHaveBeenCalledExactlyOnceWith('select_build_target', {
    target: 'linux-x64-controlled-v1',
  });
  for (const key of ['generated', 'built', 'virtualResult', 'generationPreview', 'notice']) {
    expect(current.state[key], key).toBeNull();
  }
  for (const key of ['generate', 'build', 'virtual']) {
    expect(current.state.stages[key], key).toEqual(initialState().stages[key]);
  }
  for (const key of ['save', 'validate']) {
    expect(current.state.stages[key], key).toBe(before.stages[key]);
  }
});

test.each(['zh-CN', 'en'])(
  'target refusal retains outputs and reports the localized error in %s',
  async (language) => {
    previewLanguage(language);
    const { current, actions, invalidateOperation } = settingsWithResults();
    const before = current.state;
    const error = message('controller.error.capabilitiesNotReady');
    current.call.mockRejectedValue(error);
    await actions.changeTarget('linux-x64-controlled-v1');
    expect(invalidateOperation).toHaveBeenCalledExactlyOnceWith();
    expect(current.call).toHaveBeenCalledExactlyOnceWith('select_build_target', {
      target: 'linux-x64-controlled-v1',
    });
    expect(current.state.notice).toEqual({ tone: 'error', text: error });
    expect(localize(current.state.notice.text)).toBe(translate(error.key));
    expect(localize(current.state.notice.text)).not.toBe(error.key);
    for (const key of ['generated', 'built', 'virtualResult', 'generationPreview', 'stages']) {
      expect(current.state[key], key).toBe(before[key]);
    }
  },
);

test.each(['zh-CN', 'en'])(
  'appearance refusal releases its operation and permits a fresh save in %s',
  async (language) => {
    previewLanguage(language);
    const { current, actions } = settingsWithResults();
    current.patchState({ appearanceDraft: 'dark' });
    const before = current.state;
    let reject;
    current.call.mockImplementationOnce(
      () =>
        new Promise((_, fail) => {
          reject = fail;
        }),
    );
    const saving = actions.configureAppearance();
    expect(current.running.current).toBe(true);
    expect(current.state.busy).toEqual(message('activity.saveAppearance'));
    expect(current.call).toHaveBeenCalledExactlyOnceWith('configure_appearance', {
      appearance: 'dark',
    });
    await actions.configureAppearance();
    expect(current.call).toHaveBeenCalledTimes(1);
    const error = message('controller.error.capabilitiesNotReady');
    reject(error);
    await saving;
    expect(current.state.settingsNotice).toEqual({ tone: 'error', text: error });
    expect(localize(current.state.settingsNotice.text)).toBe(translate(error.key));
    expect(localize(current.state.settingsNotice.text)).not.toBe(error.key);
    expect(current.running.current).toBe(false);
    expect(current.state.busy).toBeNull();
    for (const key of [
      'generated',
      'built',
      'virtualResult',
      'generationPreview',
      'stages',
      'notice',
    ]) {
      expect(current.state[key], key).toBe(before[key]);
    }
    current.patchState({ appearanceDraft: 'light' });
    current.call.mockResolvedValueOnce(undefined);
    await actions.configureAppearance();
    expect(current.call).toHaveBeenNthCalledWith(2, 'configure_appearance', {
      appearance: 'light',
    });
    expect(current.state.settingsNotice).toEqual({
      tone: 'success',
      text: message('controller.settings.appearanceSaved'),
    });
    expect(current.running.current).toBe(false);
    expect(current.state.busy).toBeNull();
  },
);

test.each(['success', 'failure'])(
  'old-epoch appearance %s cannot release a newer pending operation',
  async (outcome) => {
    const { current, actions } = settingsWithResults();
    let resolveOld;
    let rejectOld;
    let resolveNew;
    current.call
      .mockImplementationOnce(
        () =>
          new Promise((resolve, reject) => {
            resolveOld = resolve;
            rejectOld = reject;
          }),
      )
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveNew = resolve;
          }),
      );
    const oldSave = actions.configureAppearance();
    // Match invalidateOperation's ownership transition before starting the new epoch.
    current.epoch.current++;
    current.running.current = false;
    current.patchState({ busy: null, appearanceDraft: 'light' });
    const newSave = actions.configureAppearance();
    expect(current.call).toHaveBeenCalledTimes(2);
    const busy = current.state.busy;
    if (outcome === 'success') resolveOld();
    else rejectOld(message('controller.error.staleDelivery'));
    await oldSave;
    expect(current.running.current).toBe(true);
    expect(current.state.busy).toBe(busy);
    await actions.configureAppearance();
    expect(current.call).toHaveBeenCalledTimes(2);
    resolveNew();
    await newSave;
    expect(current.running.current).toBe(false);
    expect(current.state.busy).toBeNull();
  },
);

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
    settingsCategory: 'tools',
    native: true,
    capabilities: { environmentOverrides: [], executionTools: null, toolError: null },
    toolDraft: { compiler: '/missing/gcc', objdump: '', git: '', python: '' },
    toolErrors: {
      compiler: message('backend.workbench.configured_tool_path_invalid', { path: '/missing/gcc' }),
    },
    setToolDraft: vi.fn(),
    setToolErrors,
  };
  const view = render(React.createElement(SettingsPage, { controller: withCommands(controller) }));
  // The dialog is portalled out of the render container.
  const input = view.baseElement.querySelector('[aria-invalid="true"]');
  expect(input).not.toBeNull();
  const error = view.baseElement.querySelector(`#${input.getAttribute('aria-describedby')}`);
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
    fireEvent.click(within(alert).getByRole('button', { name: translate('action.copyDetails') }));
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
  const view = render(
    React.createElement(IntegrationDelivery, { controller: withCommands(base), active: true }),
  );
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
      React.createElement(IntegrationDelivery, {
        controller: withCommands({ ...base, ...state }),
        active: true,
      }),
    );
    expect(guidance(view)).toBe(expected);
  }
  const failed = {
    ...base,
    generated,
    stages: { ...base.stages, build: { state: 'failed', detail: 'cc: not found' } },
  };
  view.rerender(
    React.createElement(IntegrationDelivery, { controller: withCommands(failed), active: true }),
  );
  expect(view.container.querySelector('.page-guidance.error').textContent).toContain(
    'cc: not found',
  );
});
