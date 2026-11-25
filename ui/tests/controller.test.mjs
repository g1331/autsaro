// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { useWorkbench } from '../src/app/useWorkbench';
import { message, previewLanguage } from '../src/i18n';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: native.invoke }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ setTitle: vi.fn().mockResolvedValue(undefined) }),
}));

const capabilities = {
  fingerprint: 'original',
  definitionFingerprint: 'definitions',
  hasWorkspace: false,
  target: 'windows-x64-controlled-v1',
  appearance: 'system',
  language: 'system',
  actions: [],
};

function reply(inputFingerprint, accepted = capabilities) {
  return { value: null, inputFingerprint, capabilities: accepted };
}

async function controller() {
  native.invoke.mockResolvedValueOnce(capabilities);
  let hook;
  await act(async () => {
    hook = renderHook(() => useWorkbench());
  });
  return hook.result;
}

afterEach(() => {
  cleanup();
  native.invoke.mockReset();
  previewLanguage('system');
});

test('the composed controller updates form state without exposing workspace replacement setters', async () => {
  const result = await controller();
  expect(result.current.setWorkspace).toBeUndefined();
  expect(result.current.setCapabilities).toBeUndefined();
  act(() => result.current.setProjectName('Example'));
  expect(result.current.projectName).toBe('Example');
  expect(typeof result.current.applyChanges).toBe('function');
});

test('a reply from before cancellation cannot publish capabilities or release the next operation', async () => {
  const result = await controller();
  const previous = Promise.withResolvers();
  const next = Promise.withResolvers();
  const cancelled = { ...capabilities, fingerprint: 'after-cancellation' };
  native.invoke
    .mockReturnValueOnce(previous.promise)
    .mockResolvedValueOnce(reply(capabilities.fingerprint, cancelled))
    .mockReturnValueOnce(next.promise);

  let previousSave;
  act(() => {
    previousSave = result.current.configureAppearance();
  });
  await act(async () => result.current.cancelOperation());
  let nextSave;
  act(() => {
    nextSave = result.current.configureAppearance();
  });

  await act(async () => {
    previous.resolve(reply(capabilities.fingerprint, { ...capabilities, fingerprint: 'late' }));
    await previousSave;
  });
  expect(result.current.capabilities).toBe(cancelled);
  expect(result.current.busy).toEqual(message('activity.saveAppearance'));
  expect(result.current.operationLog.map((entry) => entry.command)).toEqual(['cancel_operation']);
  expect(native.invoke.mock.calls).toEqual([
    ['workbench_capabilities'],
    ['configure_appearance', { appearance: 'system', fingerprint: capabilities.fingerprint }],
    ['cancel_operation', { fingerprint: capabilities.fingerprint }],
    ['configure_appearance', { appearance: 'system', fingerprint: cancelled.fingerprint }],
  ]);

  await act(async () => {
    next.resolve(reply(cancelled.fingerprint, cancelled));
    await nextSave;
  });
  expect(result.current.busy).toBeNull();
  expect(result.current.operationLog.map((entry) => entry.command)).toEqual([
    'cancel_operation',
    'configure_appearance',
  ]);
});

test('an invalid echoed fingerprint rejects the reply and refreshes capabilities once', async () => {
  const result = await controller();
  native.invoke
    .mockResolvedValueOnce(reply('other-request', { ...capabilities, fingerprint: 'wrong' }))
    .mockResolvedValueOnce(capabilities);
  act(() => result.current.setLanguageDraft('en'));
  await act(async () => result.current.configureLanguage());

  expect(native.invoke.mock.calls).toEqual([
    ['workbench_capabilities'],
    ['configure_language', { language: 'en', fingerprint: capabilities.fingerprint }],
    ['workbench_capabilities'],
  ]);
  expect(result.current.capabilities).toBe(capabilities);
  expect(result.current.savedLanguage).toBe('system');
  expect(result.current.languageDraft).toBe('en');
  expect(result.current.settingsNotice).toEqual({
    tone: 'error',
    text: message('controller.error.staleDelivery'),
  });
});

test.each(['project switch', 'integration panel'])(
  'an applied edit from the %s still requires saving when its readback fails',
  async (entry) => {
    const result = await controller();
    const workspace = {
      name: 'Saved ECU',
      dirty: false,
      integrationCandidate: true,
      applicationInitialized: true,
      issues: [],
    };
    const projection = { dirty: false, diagnostics: [], fields: [], capabilities: [] };
    const inspection = {
      diagnostics: [],
      description: {
        signals: [{ port: 'Speed', canId: 100 }],
        schedule: { entities: [{ application: true, periodMs: 20 }] },
      },
    };
    const preview = { files: [{ path: 'ecu.arxml', changed: true }], revision: 'save-revision' };
    const failedSave = message('workflow.error.saveIncomplete');
    let current = capabilities;
    let applied = false;
    let saved = false;
    native.invoke.mockImplementation(async (command, args) => {
      if (command === 'workbench_capabilities') return current;
      const respond = (value) => ({ ...reply(args.fingerprint, current), value });
      switch (command) {
        case 'open_project':
          current = { ...capabilities, hasWorkspace: true, fingerprint: 'opened' };
          return respond(workspace);
        case 'inspect_integration':
          return respond(inspection);
        case 'edit_integration':
          applied = true;
          current = { ...current, fingerprint: 'edited' };
          return respond({
            ...inspection,
            description: { ...inspection.description, signals: [{ port: 'Speed', canId: 101 }] },
          });
        case 'workspace_view':
          throw new Error('workspace view unavailable');
        case 'project_projection':
          if (applied && !saved) throw new Error('projection unavailable');
          return respond(projection);
        case 'preview_integration_save':
          return respond(preview);
        case 'save_integration':
          return respond({ workspace, error: saved ? null : failedSave });
        case 'close_project':
          current = capabilities;
          return respond(null);
        default:
          throw new Error(`Unexpected command: ${command}`);
      }
    });

    act(() => result.current.setImportPaths(['ecu.arxml']));
    await act(async () => result.current.importProject());
    expect(result.current.workspace).toBe(workspace);
    expect(result.current.projection).toBe(projection);
    expect(result.current.integrationInspection).toBe(inspection);
    act(() => {
      result.current.setIntegrationIds({ Speed: '101' });
      result.current.setIntegrationUnapplied(true);
    });

    if (entry === 'project switch') {
      await act(async () => result.current.startProject());
      await act(async () => result.current.resolveGuard('apply'));
      expect(native.invoke).toHaveBeenCalledWith('project_projection', { fingerprint: 'edited' });
    } else {
      await act(async () => result.current.applyIntegration());
    }
    expect(native.invoke).toHaveBeenCalledWith('workspace_view', { fingerprint: 'edited' });
    expect(result.current.workspace.dirty).toBe(false);
    expect(result.current.projection.dirty).toBe(false);
    expect(result.current.integrationUnapplied).toBe(false);
    expect(result.current.integrationIds).toEqual({ Speed: '101' });

    const calls = (command) => native.invoke.mock.calls.filter(([name]) => name === command);
    await act(async () => result.current.startProject());
    expect(calls('close_project')).toHaveLength(0);
    expect(result.current.guard?.kind).toBe('project');
    await act(async () => result.current.resolveGuard('cancel'));

    // Saving is also available from the ordinary save entry after the aborted switch.
    await act(async () => result.current.requestSave());
    expect(result.current.integrationPreview).toBe(preview);
    await act(async () => result.current.saveIntegration());
    expect(result.current.stages.save).toEqual({ state: 'failed', detail: failedSave });
    expect(result.current.integrationSavePending).toBe(true);

    await act(async () => result.current.startProject());
    expect(result.current.guard?.kind).toBe('project');
    expect(calls('close_project')).toHaveLength(0);
    await act(async () => result.current.resolveGuard('apply'));
    expect(result.current.integrationPreview).toBe(preview);
    saved = true;
    await act(async () => result.current.saveIntegration());
    expect(calls('edit_integration')).toHaveLength(1);
    expect(calls('save_integration')).toEqual([
      ['save_integration', { fingerprint: 'edited', revision: 'save-revision' }],
      ['save_integration', { fingerprint: 'edited', revision: 'save-revision' }],
    ]);
    expect(calls('close_project')).toHaveLength(1);
    expect(result.current.workspace).toBeNull();
    expect(result.current.integrationSavePending).toBe(false);
  },
);
