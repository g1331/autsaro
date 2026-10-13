import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createPipelineInputs } from './support/inputs.mjs';
import { openWorkbench, existing, json } from './support/workbench.mjs';

const {
  check,
  evaluate,
  screenshot,
  resize,
  key,
  scratch,
  until,
  invoke,
  projection,
  click,
  input,
  label,
  menu,
  refreshSurface,
  sourceBytes,
  unchanged,
  changeSet,
  refuse,
  save,
  create,
  openMember,
  selectObject,
  exportSource,
} = await openWorkbench('composition-editing');

let pipeline;
check('composition-shared-editing-and-source-protection', async () => {
  pipeline = await createPipelineInputs(scratch);
  await openMember(pipeline.manifest);
  let view = await projection();
  assert.equal(view.profile, 'singlecore-ecu-v1');
  assert.equal(view.diagnostics.filter((issue) => issue.severity === 'error').length, 0);
  const original = await sourceBytes(view);
  const periodic = view.objects.find(
    (object) => object.path === '/Application/Process/Behavior/Periodic10ms',
  );
  const period = view.fields.find(
    (field) => field.objectId === periodic.objectId && field.definitionId === 'TIMING-EVENT#PERIOD',
  );
  assert(period?.writable);
  const longPeriodicName = 'Periodic10msForPipelineProcessingWithAnExplicitLongObjectIdentity';
  await selectObject(periodic.objectId);
  await input(label('实例名称'), longPeriodicName);
  await input(`[data-field-id=${json(period.fieldId)}]`, '0.0100');
  await resize(1080, 720);
  await screenshot('composition-long-name-narrow-draft.png');
  await click('预览更改');
  await until(
    `Boolean(document.querySelector('[aria-label="确认原子应用批次"]'))`,
    'composition field batch preview',
  );
  await until(
    `[...document.querySelectorAll('[role=dialog] button')].some(node => node.textContent.trim() === '确认应用全部变化' && !node.disabled)`,
    'composition preview ready for keyboard cancel',
  );
  await until(
    `Boolean(document.activeElement?.closest('[role=dialog]'))`,
    'composition preview receives focus',
  );
  await key('Escape');
  await until(`!document.querySelector('[role=dialog]')`, 'composition preview cancel');
  assert.equal(
    await evaluate(`document.querySelector('[data-field-id="${period.fieldId}"]').value`),
    '0.0100',
  );
  await click('预览更改');
  await until(
    `Boolean(document.querySelector('[aria-label="确认原子应用批次"]'))`,
    'composition field preview again',
  );
  await click('确认应用全部变化', "document.querySelector('[role=dialog]')");
  await until(`!document.querySelector('[role=dialog]')`, 'composition batch applied');
  await until(
    `(async () => !(await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation)()`,
    'composition batch operation completed',
  );
  await save();
  view = await refreshSurface(pipeline.manifest);
  assert.equal(
    view.fields.find(
      (field) =>
        field.definitionId === 'TIMING-EVENT#PERIOD' &&
        field.objectId ===
          view.objects.find((object) => object.shortName === longPeriodicName).objectId,
    ).current.value.lexeme,
    '0.0100',
  );
  await selectObject(view.objects.find((object) => object.shortName === longPeriodicName).objectId);
  await screenshot('composition-long-name-narrow-reopened.png');
  await resize(1480, 920);
  await unchanged(
    original.filter(
      (source) => !['process.arxml', 'ecuc.arxml'].includes(path.basename(source.path)),
    ),
  );
  const mappingSource = original.find((source) => path.basename(source.path) === 'ecuc.arxml');
  assert.deepEqual(
    await readFile(mappingSource.path),
    Buffer.from(
      mappingSource.bytes
        .toString('utf8')
        .replaceAll(
          periodic.path,
          `${periodic.path.slice(0, periodic.path.lastIndexOf('/') + 1)}${longPeriodicName}`,
        ),
    ),
    'Event rename changes only the known OS mapping reference bytes',
  );
  view = await projection();
  const connector = view.objects.find(
    (object) => object.path === '/Application/Pipeline/IngressObserve',
  );
  await selectObject(connector.objectId);
  const provider = view.fields.find(
    (field) =>
      field.objectId === connector.objectId &&
      field.definitionId === 'ASSEMBLY-SW-CONNECTOR#PROVIDER-IREF/CONTEXT-COMPONENT-REF',
  );
  const port = view.fields.find(
    (field) =>
      field.objectId === connector.objectId &&
      field.definitionId === 'ASSEMBLY-SW-CONNECTOR#PROVIDER-IREF/TARGET-P-PORT-REF',
  );
  const instanceTarget = view.referenceCandidates.find(
    (candidate) =>
      candidate.fieldId === provider.fieldId &&
      candidate.path === '/Application/Pipeline/ProcessInstance',
  );
  const portTarget = view.referenceCandidates.find(
    (candidate) =>
      candidate.fieldId === port.fieldId && candidate.path === '/Application/Process/Result',
  );
  await input(
    `[data-field-id=${json(provider.fieldId)}]`,
    `${instanceTarget.targetId}:${instanceTarget.dest}`,
  );
  await input(`[data-field-id=${json(port.fieldId)}]`, `${portTarget.targetId}:${portTarget.dest}`);
  await click('预览更改');
  await until(
    `Boolean(document.querySelector('[aria-label="确认原子应用批次"]'))`,
    'composition paired endpoint preview',
  );
  await click('确认应用全部变化', "document.querySelector('[role=dialog]')");
  await until(`!document.querySelector('[role=dialog]')`, 'composition paired endpoint applied');
  await until(
    `(async () => !(await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation)()`,
    'composition paired endpoint operation completed',
  );
  await save();
  await refreshSurface(pipeline.manifest);
  const initialization = await invoke('preview_application_initialization');
  assert.equal(initialization.slots.length, 3);
  await menu('预览初始化用户应用…');
  await until(`Boolean(document.querySelector('[role=dialog]'))`, 'composition all source slots preview');
  await click('确认创建用户应用与成员记录', "document.querySelector('[role=dialog]')");
  await until(`!document.querySelector('[role=dialog]')`, 'composition create-only initialization');
  await until(
    `(async () => !(await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation)()`,
    'composition initialization operation completed',
  );
  const users = [];
  for (const file of initialization.files) {
    const bytes = Buffer.from(file.contents + '\n/* User-owned composition source. */\n');
    await writeFile(path.join(pipeline.directory, file.path), bytes);
    users.push({ ...file, bytes });
  }
  await refreshSurface(pipeline.manifest);
  const delivered = await exportSource('Pipeline-source');
  for (const file of users) {
    assert.deepEqual(await readFile(path.join(pipeline.directory, file.path)), file.bytes);
    assert.deepEqual(
      await readFile(path.join(delivered.directory, 'src', path.basename(file.path))),
      file.bytes,
    );
  }
  view = await projection();
  const component = view.objects.find((object) => object.path === '/Application/Process');
  await refuse(
    'prepare_configuration_change',
    {
      changeSet: changeSet(view, [
        {
          op: 'rename-instance',
          changeId: 'unsafe-member-rename',
          object: existing(component.objectId),
          expectedShortName: 'Process',
          shortName: 'Compute',
        },
      ]),
    },
    'Initialized composition member identity must remain stable',
  );
});
