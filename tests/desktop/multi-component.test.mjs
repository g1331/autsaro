import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createMultiComponentInputs } from './support/inputs.mjs';
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
} = await openWorkbench('multi-component');

let multi;
check('multi-component-shared-editing-and-source-protection', async () => {
  multi = await createMultiComponentInputs(scratch);
  await openMember(multi.manifest);
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
  await screenshot('multi-long-name-narrow-draft.png');
  await click('预览更改');
  await until(
    `Boolean(document.querySelector('[aria-label="确认原子应用批次"]'))`,
    'multi field batch preview',
  );
  await until(
    `[...document.querySelectorAll('[role=dialog] button')].some(node => node.textContent.trim() === '确认应用全部变化' && !node.disabled)`,
    'multi preview ready for keyboard cancel',
  );
  await until(
    `Boolean(document.activeElement?.closest('[role=dialog]'))`,
    'multi preview receives focus',
  );
  await key('Escape');
  await until(`!document.querySelector('[role=dialog]')`, 'multi preview cancel');
  assert.equal(
    await evaluate(`document.querySelector('[data-field-id="${period.fieldId}"]').value`),
    '0.0100',
  );
  await click('预览更改');
  await until(
    `Boolean(document.querySelector('[aria-label="确认原子应用批次"]'))`,
    'multi field preview again',
  );
  await click('确认应用全部变化', "document.querySelector('[role=dialog]')");
  await until(`!document.querySelector('[role=dialog]')`, 'multi batch applied');
  await until(
    `(async () => !(await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation)()`,
    'multi batch operation completed',
  );
  await save();
  view = await refreshSurface(multi.manifest);
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
  await screenshot('multi-long-name-narrow-reopened.png');
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
    'multi paired endpoint preview',
  );
  await click('确认应用全部变化', "document.querySelector('[role=dialog]')");
  await until(`!document.querySelector('[role=dialog]')`, 'multi paired endpoint applied');
  await until(
    `(async () => !(await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation)()`,
    'multi paired endpoint operation completed',
  );
  await save();
  await refreshSurface(multi.manifest);
  const initialization = await invoke('preview_application_initialization');
  assert.equal(initialization.slots.length, 3);
  await menu('预览初始化用户应用…');
  await until(`Boolean(document.querySelector('[role=dialog]'))`, 'multi all source slots preview');
  await click('确认创建用户应用与成员记录', "document.querySelector('[role=dialog]')");
  await until(`!document.querySelector('[role=dialog]')`, 'multi create-only initialization');
  await until(
    `(async () => !(await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation)()`,
    'multi initialization operation completed',
  );
  const users = [];
  for (const file of initialization.files) {
    const bytes = Buffer.from(file.contents + '\n/* User-owned multi-component source. */\n');
    await writeFile(path.join(multi.directory, file.path), bytes);
    users.push({ ...file, bytes });
  }
  await refreshSurface(multi.manifest);
  const delivered = await exportSource('MultiComponent-source');
  for (const file of users) {
    assert.deepEqual(await readFile(path.join(multi.directory, file.path)), file.bytes);
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
    'Initialized multi-component member identity must remain stable',
  );
});
