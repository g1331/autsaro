// @vitest-environment jsdom
import React from 'react';
import { act, cleanup, fireEvent, render, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { composedMessage, localize, message, previewLanguage, translate } from '../src/i18n';
import { BuildPage } from '../src/pages/BuildPage';
import { IntegrationDelivery } from '../src/IntegrationDelivery';
import { CopyText, OwnedLog } from '../src/workbench/Dialog';
import { PreviewDialogs } from '../src/workbench/PreviewDialogs';
import { SourceEntry } from '../src/workbench/SourceEntry';
import { ToolWindows } from '../src/workbench/ToolWindows';
import { useWorkbench } from '../src/workbench/useWorkbench';

const native = vi.hoisted(() => ({
  invoke: vi.fn(),
  setTitle: vi.fn().mockResolvedValue(undefined),
  open: vi.fn(),
  confirm: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: native.invoke }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ setTitle: native.setTitle }),
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: native.open, confirm: native.confirm }));

const rawOutput = String.raw`\\?\C:\工作目录\output`;
const rawBuild = String.raw`\\?\C:\工作目录\build`;
const rawBinary = String.raw`\\?\C:\工作目录\build\ecu.exe`;
const rawBackup = String.raw`\\?\UNC\server\share\backup`;
const rawLog = String.raw`compiler: \\?\C:\目录\file.c`;
const rawSource = String.raw`\\?\C:\工作目录\Application.arxml`;
const rawUncSource = String.raw`\\?\UNC\server\share\System.arxml`;
const logicalPath = '/Package/Component/Port';

const capabilities = {
  fingerprint: 'unchanged-input',
  target: 'windows-x64-controlled-v1',
  targets: ['windows-x64-controlled-v1'],
  nativeExecution: true,
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
  language: 'en',
  actions: ['open', 'create', 'edit'].map((action) => ({
    action,
    available: true,
    reason: null,
  })),
  verificationMode: false,
};
const generationPreview = {
  revision: 'sealed-output-revision',
  outputDirectory: rawOutput,
  files: [
    {
      path: 'src/Application.c',
      status: 'new',
      owner: 'application',
      producerId: '/Package/Component',
      before: null,
      after: rawLog,
    },
  ],
};
const generated = {
  outputDirectory: rawOutput,
  previousOutputDirectory: rawBackup,
  files: ['src/Application.c'],
  issues: [],
};

let restoreClipboard;
function captureClipboard() {
  const writeText = vi.fn().mockResolvedValue(undefined);
  const previous = Object.getOwnPropertyDescriptor(navigator, 'clipboard');
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: { writeText },
  });
  restoreClipboard = () => {
    if (previous) Object.defineProperty(navigator, 'clipboard', previous);
    else delete navigator.clipboard;
  };
  return writeText;
}

async function renderWorkbench(integrationCandidate = false) {
  let opened = false;
  const workspace = {
    integrationCandidate,
    name: 'PathProject',
    dirty: false,
    files: [],
    frames: [],
    signals: [],
    diagnostic: null,
    issues: [],
  };
  const projection = {
    workspaceEpoch: 'epoch',
    inputFingerprint: 'unchanged-input',
    definitionFingerprint: 'definitions',
    projectPath: String.raw`\\?\C:\工作目录\project.autosar`,
    dirty: false,
    sources: [
      { sourceId: 'disk-source', path: rawSource },
      { sourceId: 'unc-source', path: rawUncSource },
    ],
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
  const reply = (value) => ({
    value,
    capabilities: { ...capabilities, hasWorkspace: opened },
    inputFingerprint: 'unchanged-input',
  });
  native.open.mockReset();
  native.confirm.mockReset().mockResolvedValue(true);
  native.invoke.mockImplementation(async (command) => {
    if (command === 'workbench_capabilities') return { ...capabilities, hasWorkspace: opened };
    if (command === 'open_project') {
      opened = true;
      return reply(workspace);
    }
    if (command === 'project_projection') return reply(projection);
    if (command === 'validate_project') return reply(workspace);
    if (command === 'inspect_integration')
      return reply({
        profile: 'standard',
        diagnostics: [],
        description: { component: { periodMs: 10 }, signals: [] },
      });
    if (command === 'preview_generate_project' || command === 'preview_ecu_project')
      return reply(generationPreview);
    if (command === 'generate_project' || command === 'generate_ecu_project')
      return reply(generated);
    if (command === 'build_project') return reply({ binaryPath: rawBinary, log: rawLog });
    throw new Error(`Unexpected command: ${command}`);
  });
  let controller;
  function Consumer() {
    controller = useWorkbench();
    return React.createElement(
      React.Fragment,
      null,
      !controller.workspace
        ? React.createElement(SourceEntry, { controller, logo: '' })
        : integrationCandidate
          ? React.createElement(IntegrationDelivery, { controller, active: true })
          : React.createElement(BuildPage, { controller }),
      React.createElement(PreviewDialogs, { controller }),
    );
  }
  const view = render(React.createElement(Consumer));
  await waitFor(() => expect(controller.capabilities).not.toBeNull());
  act(() => controller.openProjectEntry('import'));
  return { view, current: () => controller };
}

async function chooseSources({ view, current }) {
  native.open.mockResolvedValueOnce([rawSource, rawUncSource]);
  await act(async () => {
    fireEvent.click(view.getByRole('button', { name: translate('editor.source.chooseFiles') }));
  });
  await waitFor(() => expect(current().importPaths).toEqual([rawSource, rawUncSource]));
}

async function importSources(consumer) {
  await chooseSources(consumer);
  await act(async () => {
    fireEvent.click(consumer.view.getByRole('button', { name: translate('editor.source.enter') }));
  });
  await waitFor(() => {
    expect(consumer.current().workspace).not.toBeNull();
    expect(consumer.current().projection).not.toBeNull();
    expect(consumer.current().busy).toBeNull();
  });
}

function buildController() {
  return {
    workspace: { dirty: false },
    unapplied: false,
    issues: [],
    integrationIssues: [],
    native: false,
    disabled: false,
    legacyTarget: 'windows-x64-controlled-v1',
    executionDisabled: true,
    executionReason: '',
    buildDirectory: rawBuild,
    stages: {
      save: { state: 'done', detail: '' },
      validate: { state: 'done', detail: '' },
      generate: { state: 'done', detail: rawOutput },
      build: { state: 'done', detail: rawBinary },
      virtual: { state: 'done', detail: '' },
    },
    generated: {
      outputDirectory: rawOutput,
      previousOutputDirectory: rawBackup,
      files: ['src/Application.c'],
    },
    built: { binaryPath: rawBinary, log: rawLog },
    handoffGenerated: false,
    changeEcuBuildDirectory: vi.fn(),
    changeTarget: vi.fn(),
    chooseDirectory: vi.fn(),
    generateProject: vi.fn(),
    buildProject: vi.fn(),
  };
}
beforeEach(() => {
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
  restoreClipboard?.();
  restoreClipboard = undefined;
  vi.clearAllMocks();
  vi.unstubAllGlobals();
  localStorage.clear();
  previewLanguage('system');
});

test('BuildPage displays ordinary Windows paths in text, stage titles, and the directory input', () => {
  previewLanguage('en');
  const controller = buildController();
  const view = render(React.createElement(BuildPage, { controller }));

  expect(view.getByLabelText(translate('shell.build.directory')).value).toBe(
    String.raw`C:\工作目录\build`,
  );
  expect(
    Array.from(view.container.querySelectorAll('.path-text'), (node) => node.textContent),
  ).toEqual([
    String.raw`C:\工作目录\output`,
    String.raw`\\server\share\backup`,
    String.raw`C:\工作目录\build\ecu.exe`,
  ]);
  const details = view.container.querySelectorAll('.stage-copy small');
  expect(details[2].textContent).toBe(String.raw`C:\工作目录\output`);
  expect(details[2].title).toBe(String.raw`C:\工作目录\output`);
  expect(details[3].textContent).toBe(String.raw`C:\工作目录\build\ecu.exe`);
  expect(details[3].title).toBe(String.raw`C:\工作目录\build\ecu.exe`);
});

test('ToolWindows copies displayed output and binary paths but preserves the complete compiler log', async () => {
  previewLanguage('en');
  const clipboard = captureClipboard();
  const controller = { ...buildController(), toolWindow: 'generation' };
  const view = render(React.createElement(ToolWindows, { controller }));
  expect(view.container.querySelector('.path-text').textContent).toBe(
    String.raw`C:\工作目录\output`,
  );
  expect(view.container.querySelector('.tool-content > div > p').textContent).toContain(
    String.raw`C:\工作目录\output`,
  );
  expect(view.container.querySelector('.tool-content > div > p').textContent).not.toContain(
    '\\\\?\\',
  );
  await act(async () => {
    fireEvent.click(view.getByRole('button', { name: translate('editor.tools.copyOutput') }));
  });
  expect(clipboard).toHaveBeenLastCalledWith(String.raw`C:\工作目录\output`);

  view.rerender(
    React.createElement(ToolWindows, { controller: { ...controller, toolWindow: 'build' } }),
  );
  expect(view.container.querySelector('.path-text').textContent).toBe(
    String.raw`C:\工作目录\build\ecu.exe`,
  );
  expect(view.container.querySelector('.tool-content > div > p').textContent).toContain(
    String.raw`C:\工作目录\build\ecu.exe`,
  );
  expect(view.container.querySelector('.tool-content > div > p').textContent).not.toContain(
    '\\\\?\\',
  );
  await act(async () => {
    fireEvent.click(view.getByRole('button', { name: translate('editor.tools.copyBinary') }));
  });
  expect(clipboard).toHaveBeenLastCalledWith(String.raw`C:\工作目录\build\ecu.exe`);
  expect(view.container.querySelector('.owned-log pre').textContent).toBe(rawLog);
  await act(async () => {
    fireEvent.click(view.getByRole('button', { name: translate('editor.dialog.copyLog') }));
  });
  expect(clipboard).toHaveBeenLastCalledWith(rawLog);
  expect(controller.generated.outputDirectory).toBe(String.raw`\\?\C:\工作目录\output`);
  expect(controller.built.binaryPath).toBe(String.raw`\\?\C:\工作目录\build\ecu.exe`);
});

test('diagnostic file paths are displayed cleanly while logical identities and copied evidence remain raw', async () => {
  previewLanguage('en');
  const clipboard = captureClipboard();
  const diagnostics = [
    {
      scope: 'schema',
      severity: 'error',
      code: 'FILE',
      message: 'File diagnostic',
      remedy: 'Retain source bytes',
      file: rawUncSource,
      path: null,
      sourceId: null,
      objectId: null,
      fieldId: null,
    },
    {
      scope: 'schema',
      severity: 'error',
      code: 'OBJECT',
      message: 'Object diagnostic',
      remedy: 'Retain object identity',
      file: rawSource,
      path: logicalPath,
      sourceId: null,
      objectId: null,
      fieldId: null,
    },
  ];
  const controller = {
    ...buildController(),
    toolWindow: 'problems',
    projection: {
      sources: [],
      objects: [],
      fields: [],
      validation: [],
      diagnostics,
    },
    issues: [],
    integrationIssues: [],
  };
  const view = render(React.createElement(ToolWindows, { controller }));
  expect(
    Array.from(view.container.querySelectorAll('.path-text'), (node) => node.textContent),
  ).toEqual([String.raw`\\server\share\System.arxml`, '/Package/Component/Port']);
  const rows = view.container.querySelectorAll('.problem-list > li');
  await act(async () => {
    fireEvent.click(
      within(rows[0]).getByRole('button', { name: translate('editor.common.copyDetails') }),
    );
  });
  expect(JSON.parse(clipboard.mock.calls.at(-1)[0])).toEqual(diagnostics[0]);
  expect(JSON.parse(clipboard.mock.calls.at(-1)[0]).file).toBe(
    String.raw`\\?\UNC\server\share\System.arxml`,
  );

  view.rerender(
    React.createElement(CopyText, { text: logicalPath, label: 'Copy object identity' }),
  );
  await act(async () => {
    fireEvent.click(view.getByRole('button', { name: 'Copy object identity' }));
  });
  expect(clipboard).toHaveBeenLastCalledWith('/Package/Component/Port');
});

test.each(['\n', '\r\n'])(
  'SourceEntry displays selected disk and UNC paths without changing the unedited %j import list',
  async (lineEnding) => {
    const consumer = await renderWorkbench();
    await chooseSources(consumer);
    const rawText = [rawSource, rawUncSource].join(lineEnding);
    act(() => consumer.current().setImportPathText(rawText));
    expect(consumer.view.getByLabelText(translate('editor.source.paths')).value).toBe(
      'C:\\工作目录\\Application.arxml\n\\\\server\\share\\System.arxml',
    );
    expect(consumer.current().importPathText).toBe(rawText);
    expect(consumer.current().importPaths).toEqual([
      String.raw`\\?\C:\工作目录\Application.arxml`,
      String.raw`\\?\UNC\server\share\System.arxml`,
    ]);
    await act(async () => {
      fireEvent.click(
        consumer.view.getByRole('button', { name: translate('editor.source.enter') }),
      );
    });
    await waitFor(() => expect(consumer.current().workspace).not.toBeNull());
    expect(native.invoke).toHaveBeenCalledWith('open_project', {
      paths: [
        String.raw`\\?\C:\工作目录\Application.arxml`,
        String.raw`\\?\UNC\server\share\System.arxml`,
      ],
      fingerprint: 'unchanged-input',
    });
  },
);

test('real ECU preview and confirmation retain raw directories and revision across rendering and language changes', async () => {
  const consumer = await renderWorkbench(true);
  await importSources(consumer);
  const { view, current } = consumer;
  const clipboard = captureClipboard();
  native.open.mockResolvedValueOnce(rawOutput);
  await act(async () => {
    fireEvent.click(view.getByRole('button', { name: translate('shell.delivery.chooseOutput') }));
  });
  native.open.mockResolvedValueOnce(rawBuild);
  await act(async () => {
    fireEvent.click(
      view.getByRole('button', { name: translate('shell.delivery.chooseBuildDirectory') }),
    );
  });
  act(() => current().setEcuImportDirectory(rawBackup));
  expect(view.getByLabelText(translate('shell.delivery.output')).value).toBe(
    String.raw`C:\工作目录\output`,
  );
  expect(view.getByLabelText(translate('shell.delivery.buildInput')).value).toBe(
    String.raw`C:\工作目录\build`,
  );
  expect(view.getByLabelText(translate('shell.delivery.importDirectory')).value).toBe(
    String.raw`\\server\share\backup`,
  );
  expect(current().ecuOutputDirectory).toBe(String.raw`\\?\C:\工作目录\output`);
  expect(current().buildDirectory).toBe(String.raw`\\?\C:\工作目录\build`);
  expect(current().ecuImportDirectory).toBe(String.raw`\\?\UNC\server\share\backup`);

  await act(async () => {
    fireEvent.click(view.getByRole('button', { name: translate('shell.delivery.preview') }));
  });
  await waitFor(() => {
    expect(current().generationPreview).toBe(generationPreview);
    expect(current().busy).toBeNull();
  });
  expect(native.invoke).toHaveBeenCalledWith('preview_ecu_project', {
    outputDirectory: String.raw`\\?\C:\工作目录\output`,
    handoff: true,
    fingerprint: 'unchanged-input',
  });
  let dialog = within(view.getByRole('dialog'));
  expect(dialog.getByText(String.raw`C:\工作目录\output`).textContent).toBe(
    String.raw`C:\工作目录\output`,
  );
  expect(dialog.getByLabelText(translate('workflow.generation.snapshotAfter')).textContent).toBe(
    rawLog,
  );
  await act(async () => {
    fireEvent.click(
      dialog.getByRole('button', { name: translate('workflow.generation.copyAfter') }),
    );
  });
  expect(clipboard).toHaveBeenLastCalledWith(rawLog);
  act(() => current().setLanguageDraft('zh-CN'));
  expect(view.getByLabelText(translate('shell.delivery.output')).value).toBe(
    String.raw`C:\工作目录\output`,
  );
  expect(current().ecuOutputDirectory).toBe(String.raw`\\?\C:\工作目录\output`);
  expect(current().buildDirectory).toBe(String.raw`\\?\C:\工作目录\build`);
  expect(current().generationPreview).toBe(generationPreview);
  expect(current().generationPreview.revision).toBe('sealed-output-revision');
  act(() => current().setLanguageDraft('en'));
  dialog = within(view.getByRole('dialog'));
  await act(async () => {
    fireEvent.click(
      dialog.getByRole('button', { name: translate('workflow.generation.confirmExport') }),
    );
  });
  await waitFor(() => {
    expect(current().generated).toBe(generated);
    expect(current().busy).toBeNull();
  });
  expect(native.confirm).toHaveBeenCalledWith(String.raw`Write 1 file to C:\工作目录\output?`, {
    title: translate('workflow.action.confirm'),
    kind: 'warning',
  });
  expect(native.invoke).toHaveBeenCalledWith('generate_ecu_project', {
    outputDirectory: String.raw`\\?\C:\工作目录\output`,
    handoff: true,
    revision: 'sealed-output-revision',
    fingerprint: 'unchanged-input',
  });
  expect(current().generationPreview).toBeNull();
  expect(current().ecuOutputDirectory).toBe(String.raw`\\?\C:\工作目录\output`);
  expect(current().buildDirectory).toBe(String.raw`\\?\C:\工作目录\build`);

  fireEvent.change(view.getByLabelText(translate('shell.delivery.output')), {
    target: { value: String.raw`D:\用户选择\output` },
  });
  expect(current().ecuOutputDirectory).toBe(String.raw`D:\用户选择\output`);
  expect(current().generated).toBeNull();
  fireEvent.change(view.getByLabelText(translate('shell.delivery.buildInput')), {
    target: { value: String.raw`D:\用户选择\build` },
  });
  expect(current().buildDirectory).toBe(String.raw`D:\用户选择\build`);
});

test('BuildPage keeps picked build paths raw for real generation and build IPC, then accepts user edits', async () => {
  const consumer = await renderWorkbench();
  await importSources(consumer);
  const { view, current } = consumer;
  await act(async () => current().validateProject());
  await waitFor(() => {
    expect(current().stages.validate.state).toBe('done');
    expect(current().busy).toBeNull();
  });
  native.open.mockResolvedValueOnce(rawOutput);
  await act(async () => {
    fireEvent.click(view.getByRole('button', { name: translate('shell.build.preview') }));
  });
  await waitFor(() => {
    expect(current().generationPreview).toBe(generationPreview);
    expect(current().busy).toBeNull();
  });
  expect(native.invoke).toHaveBeenCalledWith('preview_generate_project', {
    outputDirectory: String.raw`\\?\C:\工作目录\output`,
    fingerprint: 'unchanged-input',
  });
  await act(async () => {
    fireEvent.click(
      within(view.getByRole('dialog')).getByRole('button', {
        name: translate('workflow.generation.confirm'),
      }),
    );
  });
  await waitFor(() => {
    expect(current().generated).toBe(generated);
    expect(current().busy).toBeNull();
  });
  expect(native.invoke).toHaveBeenCalledWith('generate_project', {
    outputDirectory: String.raw`\\?\C:\工作目录\output`,
    revision: 'sealed-output-revision',
    fingerprint: 'unchanged-input',
  });
  native.open.mockResolvedValueOnce(rawBuild);
  await act(async () => {
    fireEvent.click(view.getByRole('button', { name: translate('shell.build.chooseDirectory') }));
  });
  expect(view.getByLabelText(translate('shell.build.directory')).value).toBe(
    String.raw`C:\工作目录\build`,
  );
  expect(current().buildDirectory).toBe(String.raw`\\?\C:\工作目录\build`);
  await act(async () => {
    fireEvent.click(view.getByRole('button', { name: translate('shell.build.build') }));
  });
  await waitFor(() => {
    expect(current().stages.build.state).toBe('done');
    expect(current().busy).toBeNull();
  });
  expect(native.invoke).toHaveBeenCalledWith('build_project', {
    outputDirectory: String.raw`\\?\C:\工作目录\output`,
    buildDirectory: String.raw`\\?\C:\工作目录\build`,
    fingerprint: 'unchanged-input',
  });
  expect(view.container.querySelectorAll('.result-section > p.path-text')[1].textContent).toBe(
    String.raw`C:\工作目录\build\ecu.exe`,
  );
  expect(view.container.querySelector('.owned-log pre').textContent).toBe(rawLog);
  fireEvent.change(view.getByLabelText(translate('shell.build.directory')), {
    target: { value: String.raw`D:\用户选择\build` },
  });
  expect(current().buildDirectory).toBe(String.raw`D:\用户选择\build`);
  expect(current().built).toBeNull();
  await act(async () => {
    fireEvent.click(view.getByRole('button', { name: translate('shell.build.build') }));
  });
  await waitFor(() => {
    expect(current().stages.build.state).toBe('done');
    expect(current().busy).toBeNull();
  });
  expect(native.invoke).toHaveBeenCalledWith('build_project', {
    outputDirectory: String.raw`\\?\C:\工作目录\output`,
    buildDirectory: String.raw`D:\用户选择\build`,
    fingerprint: 'unchanged-input',
  });
});

test.each([
  [
    'zh-CN',
    String.raw`初始化绝不会覆盖已有路径：C:\工作目录\Application.arxml`,
    String.raw`确认将 1 个文件写入 C:\工作目录\output？`,
  ],
  [
    'en',
    String.raw`Initialization never overwrites an existing path: C:\工作目录\Application.arxml`,
    String.raw`Write 1 file to C:\工作目录\output?`,
  ],
])(
  '%s product messages format path parameters without mutating descriptors or raw log copies',
  async (language, initialization, confirmation) => {
    previewLanguage(language);
    const clipboard = captureClipboard();
    const pathParams = Object.freeze({ path: rawSource });
    const directoryParams = Object.freeze({ count: 1, directory: rawOutput });
    const pathMessage = message(
      'backend.arxml.application.initialization_existing_path',
      pathParams,
    );
    const confirmMessage = message('workflow.confirm.generateFiles', directoryParams);
    const view = render(
      React.createElement(
        React.Fragment,
        null,
        React.createElement(OwnedLog, {
          text: [pathMessage, [confirmMessage]],
          label: 'Product feedback',
        }),
        React.createElement(OwnedLog, { text: rawLog, label: 'Compiler evidence' }),
      ),
    );
    expect(view.getByLabelText('Product feedback').textContent).toBe(
      `${initialization}\n${confirmation}`,
    );
    expect(view.getByLabelText('Compiler evidence').textContent).toBe(
      String.raw`compiler: \\?\C:\目录\file.c`,
    );
    expect(pathMessage.params).toBe(pathParams);
    expect(pathMessage.params).toEqual({ path: String.raw`\\?\C:\工作目录\Application.arxml` });
    expect(confirmMessage.params).toBe(directoryParams);
    expect(confirmMessage.params).toEqual({
      count: 1,
      directory: String.raw`\\?\C:\工作目录\output`,
    });
    const rawSection = view.getByLabelText('Compiler evidence').closest('.owned-log');
    await act(async () => {
      fireEvent.click(
        within(rawSection).getByRole('button', { name: translate('editor.dialog.copyLog') }),
      );
    });
    expect(clipboard).toHaveBeenLastCalledWith(String.raw`compiler: \\?\C:\目录\file.c`);
    expect(localize(composedMessage('workflow.error.batchPreview', { error: rawLog }))).toContain(
      String.raw`compiler: \\?\C:\目录\file.c`,
    );
    expect(
      localize(
        message('backend.arxml.application.initialization_existing_path', { path: logicalPath }),
      ),
    ).toContain('/Package/Component/Port');
  },
);

test.each(['zh-CN', 'en'])(
  '%s file and backup parameters format without changing arbitrary evidence',
  (language) => {
    previewLanguage(language);
    const fileParams = Object.freeze({ file: rawUncSource });
    const backupParams = Object.freeze({ path: rawOutput, backup: rawBackup, error: rawLog });
    const fileMessage = message('backend.arxml.import_xsd_failed', fileParams);
    const backupMessage = message('backend.generation.previous_output_backup_failed', backupParams);
    expect(localize(fileMessage)).toContain(String.raw`\\server\share\System.arxml`);
    expect(localize(fileMessage)).not.toContain('\\\\?\\');
    expect(localize(backupMessage)).toContain(String.raw`C:\工作目录\output`);
    expect(localize(backupMessage)).toContain(String.raw`\\server\share\backup`);
    expect(localize(backupMessage)).toContain(String.raw`compiler: \\?\C:\目录\file.c`);
    expect(fileMessage.params).toBe(fileParams);
    expect(fileParams.file).toBe(String.raw`\\?\UNC\server\share\System.arxml`);
    expect(backupMessage.params).toBe(backupParams);
    expect(backupParams).toEqual({
      path: String.raw`\\?\C:\工作目录\output`,
      backup: String.raw`\\?\UNC\server\share\backup`,
      error: String.raw`compiler: \\?\C:\目录\file.c`,
    });
  },
);

test('the readonly project directory displays the picked path without changing controller state', async () => {
  const { view, current } = await renderWorkbench();
  act(() => current().openProjectEntry('empty'));
  native.open.mockResolvedValueOnce(rawOutput);
  await act(async () => {
    fireEvent.click(view.getByRole('button', { name: translate('editor.common.select') }));
  });
  const directory = view.getByLabelText(translate('editor.source.directory'));
  expect(directory.readOnly).toBe(true);
  expect(directory.value).toBe(String.raw`C:\工作目录\output`);
  expect(current().projectDirectory).toBe(String.raw`\\?\C:\工作目录\output`);
});

test('application preview formats source files but preserves component identities, relative paths, and copied slot evidence', async () => {
  previewLanguage('en');
  const clipboard = captureClipboard();
  const slot = {
    producerSlot: 'application',
    componentPath: logicalPath,
    sourcePaths: [rawSource, rawUncSource, 'application/Application.c'],
    generatedHeaders: ['include/Application.h'],
    entrySymbols: ['Application_Main'],
  };
  const preview = {
    revision: 'application-revision',
    slots: [slot],
    files: [],
    manifestBefore: rawLog,
    manifestAfter: rawLog,
  };
  const view = render(
    React.createElement(PreviewDialogs, { controller: { applicationPreview: preview } }),
  );
  expect(
    Array.from(view.container.querySelectorAll('dl .path-text'), (node) => node.textContent),
  ).toEqual([
    'application',
    '/Package/Component/Port',
    String.raw`C:\工作目录\Application.arxml`,
    String.raw`\\server\share\System.arxml`,
    'application/Application.c',
    'include/Application.h',
    'Application_Main',
  ]);
  expect(view.getByLabelText(translate('workflow.application.manifestBefore')).textContent).toBe(
    rawLog,
  );
  expect(view.getByLabelText(translate('workflow.application.manifestAfter')).textContent).toBe(
    rawLog,
  );
  await act(async () => {
    fireEvent.click(view.getByRole('button', { name: translate('workflow.application.copySlot') }));
  });
  expect(JSON.parse(clipboard.mock.calls.at(-1)[0])).toEqual([
    {
      producerSlot: 'application',
      componentPath: '/Package/Component/Port',
      sourcePaths: [
        String.raw`\\?\C:\工作目录\Application.arxml`,
        String.raw`\\?\UNC\server\share\System.arxml`,
        'application/Application.c',
      ],
      generatedHeaders: ['include/Application.h'],
      entrySymbols: ['Application_Main'],
    },
  ]);
});

test.each(['zh-CN', 'en'])(
  'application preview reviews every trusted slot and seed in %s',
  async (language) => {
    previewLanguage(language);
    const clipboard = captureClipboard();
    const names = ['Ingress', 'ProcessWithALongComponentNameForReview', 'Observe'];
    const slots = names.map((name) => ({
      producerSlot: `singlecore-multi-swc-v1:/Application/Pipeline/${name}Instance`,
      componentPath: `/Application/${name}`,
      sourcePaths: [`application/${name}.c`],
      generatedHeaders: [`include/Rte_${name}.h`, 'include/Rte.h'],
      entrySymbols: [`${name}_Periodic`],
    }));
    const preview = {
      revision: 'complete-multi-application-revision',
      slots,
      files: names.map((name) => ({
        path: `application/${name}.c`,
        contents: `void ${name}_Periodic(void) {}\n`,
      })),
      manifestBefore: '{"applicationInputs":[]}',
      manifestAfter: JSON.stringify({
        applicationInputs: slots.map((slot) => ({
          path: slot.sourcePaths[0],
          producerSlot: slot.producerSlot,
        })),
      }),
    };
    const initialize = vi.fn();
    const view = render(
      React.createElement(PreviewDialogs, {
        controller: { applicationPreview: preview, initializeApplicationPreviewed: initialize },
      }),
    );
    for (const slot of slots) {
      expect(view.getByText(slot.componentPath)).toBeTruthy();
      expect(view.getByText(slot.producerSlot).classList.contains('path-text')).toBe(true);
      expect(view.getByText(slot.entrySymbols[0]).classList.contains('path-text')).toBe(true);
    }
    await act(async () => {
      fireEvent.click(
        view.getByRole('button', { name: translate('workflow.application.copySlot') }),
      );
    });
    expect(JSON.parse(clipboard.mock.calls.at(-1)[0])).toEqual(slots);
    for (const file of preview.files) {
      fireEvent.click(view.getByRole('button', { name: file.path }));
      expect(view.getByLabelText(translate('workflow.application.seedSnapshot')).textContent).toBe(
        file.contents,
      );
    }
    fireEvent.click(view.getByRole('button', { name: translate('workflow.application.confirm') }));
    expect(initialize).toHaveBeenCalledTimes(1);
    expect(preview.revision).toBe('complete-multi-application-revision');
  },
);
