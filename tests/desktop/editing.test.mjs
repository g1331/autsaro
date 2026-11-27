import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { openWorkbench, explicit, existing } from './support/workbench.mjs';

const {
  check,
  evaluate,
  key,
  exportSource,
  invoke,
  projection,
  input,
  field,
  menu,
  click,
  until,
  unchanged,
  sourceBytes,
  awaitStandardInspectionReady,
  refreshSurface,
  changeSet,
  valueChange,
  apply,
  refuse,
  save,
  fieldBy,
  create,
} = await openWorkbench('editing');

let editing;
check('batch-rejection-and-byte-preservation', async () => {
  editing = await create('EditingProbe');
  const view = await projection();
  const id = fieldBy(view, '/CanIfTxPduCanId');
  const bitSize = fieldBy(view, '/ComBitSize');
  const batch = changeSet(view, [valueChange(id, '866'), valueChange(bitSize, '65')]);
  await refuse(
    'prepare_configuration_change',
    { changeSet: batch },
    'One invalid member rejects entire batch',
  );
  const good = changeSet(view, [valueChange(id, '866')]);
  const preview = await invoke('prepare_configuration_change', {
    changeSet: good,
  });
  const changedDraft = changeSet(view, [valueChange(id, '867')]);
  await refuse(
    'apply_configuration_change',
    { changeSet: changedDraft, changeRevision: preview.changeRevision },
    'Changed draft invalidates confirmation',
  );
  await refuse(
    'prepare_configuration_change',
    {
      changeSet: changeSet(view, [
        valueChange(id, '866', 'first'),
        valueChange(id, '867', 'second'),
      ]),
    },
    'Conflicting fields reject full batch',
  );
  await refuse(
    'read_project_source',
    { sourceId: view.sources[0].path },
    'Arbitrary filesystem path is not a source identity',
  );
  await apply(view, [valueChange(id, '866')]);
  await refuse(
    'apply_configuration_change',
    { changeSet: good, changeRevision: preview.changeRevision },
    'Old input identity cannot be reused',
    view.inputFingerprint,
  );
  await save();
  await refreshSurface(editing.manifest);
});
check('structure-and-incoming-reference-closure', async () => {
  const view = await projection();
  const signal = view.objects.find(
    (object) => object.definitionId === '/AUTOSAR/EcucDefs/Com/ComConfig/ComSignal',
  );
  assert(signal);
  const incoming = view.references.filter((edge) => edge.targetId === signal.objectId);
  assert(incoming.length > 0, 'Rename must exercise actual incoming references');
  const renameChange = {
    op: 'rename-instance',
    changeId: 'rename-signal',
    object: existing(signal.objectId),
    expectedShortName: signal.shortName,
    shortName: 'Measurement',
  };
  const result = await apply(view, [renameChange]);
  assert(result.preview.impacts.some((impact) => impact.incoming));
  const renamed = result.outcome.projection.objects.find(
    (object) => object.objectId === signal.objectId,
  );
  assert.equal(renamed.shortName, 'Measurement');
  for (const edge of incoming) {
    const actual = result.outcome.projection.references.find(
      (item) => item.fieldId === edge.fieldId,
    );
    assert.equal(actual.rawPath, renamed.path);
    assert.equal(actual.targetId, signal.objectId);
  }
  const current = await projection();
  await refuse(
    'prepare_configuration_change',
    {
      changeSet: changeSet(current, [
        {
          op: 'remove-instance',
          changeId: 'remove-in-use',
          object: existing(signal.objectId),
          expectedShortName: renamed.shortName,
        },
      ]),
    },
    'Inbound reference removal is unsafe',
  );
  const parent = current.objects.find(
    (object) => object.definitionId === '/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection',
  );
  assert(parent);
  const createChange = {
    op: 'create-instance',
    changeId: 'new-pdu',
    parent: existing(parent.objectId),
    sourceId: parent.sourceId,
    definitionId: `${parent.definitionId}/Pdu`,
    shortName: 'SpareAcceptancePdu',
  };
  const incomplete = await invoke('prepare_configuration_change', {
    changeSet: changeSet(current, [createChange]),
  });
  assert(incomplete.changeRevision, 'Missing required values permit creation preview');
  const length = {
    op: 'set-value',
    changeId: 'new-length',
    field: {
      kind: 'new',
      object: { kind: 'created', changeId: 'new-pdu' },
      definitionId: `${parent.definitionId}/Pdu/PduLength`,
      entryKey: 'length',
    },
    expected: { state: 'absent' },
    value: explicit('integer', '4'),
  };
  const created = await apply(current, [length, createChange]);
  const id = created.outcome.createdIds.find((item) => item.changeId === 'new-pdu').objectId;
  const fieldId = created.outcome.createdFields.find((item) => item.entryKey === 'length').fieldId;
  assert.equal(
    created.outcome.projection.fields.find((field) => field.fieldId === fieldId).current.value
      .lexeme,
    '4',
  );
  const removed = await apply(created.outcome.projection, [
    {
      op: 'remove-instance',
      changeId: 'remove-spare',
      object: existing(id),
      expectedShortName: createChange.shortName,
    },
  ]);
  assert(!removed.outcome.projection.objects.some((object) => object.objectId === id));
  const repeated = await apply(removed.outcome.projection, [createChange, length]);
  assert.notEqual(repeated.outcome.createdIds[0].objectId, id);
  await apply(repeated.outcome.projection, [
    {
      op: 'remove-instance',
      changeId: 'remove-second-spare',
      object: existing(repeated.outcome.createdIds[0].objectId),
      expectedShortName: createChange.shortName,
    },
  ]);
  await save();
});

check('extended IDs through page and Edit menu survive save and reopen', async () => {
  const { manifest } = await create('ExtendedIds');
  const view = await projection();
  const hardware = view.fields.filter((item) => item.definitionId.endsWith('/CanIdType'));
  assert.ok(hardware.length);
  await apply(
    view,
    hardware.map((item) => valueChange(item, 'MIXED')),
  );
  await save();
  await refreshSurface(manifest);
  await awaitStandardInspectionReady();
  const original = await sourceBytes(await projection());
  const tx = (await invoke('inspect_integration')).description.signals.find(
    (item) => !item.receive,
  );
  const id = field('发送 CAN ID · ' + tx.port);
  const format = field('CAN ID 格式 · ' + tx.port);
  await input(format, 'true');
  for (const [value, submit] of [
    [0x1234567, click],
    [0x1fffffff, menu],
    [65, click],
  ]) {
    await input(id, String(value));
    await submit('应用更改');
    await until(
      `document.querySelector('.integration-fields') && [...document.querySelectorAll('.integration-view button')].some(node => node.textContent.trim() === '应用更改' && node.disabled) && Boolean(document.querySelector('.integration-fields label:first-child input:not(:disabled)'))`,
      'draft applied',
    );
    const report = await invoke('inspect_integration');
    const edited = report.description.signals.find((item) => item.port === tx.port);
    assert.equal(edited.canId, value);
    assert.equal(edited.extended, true);
    await unchanged(original);
  }
  await input(field('帧长度（字节） · ' + tx.port), '9');
  await click('应用更改');
  await until(
    `document.querySelector('.integration-view [role=alert]')?.textContent.includes('FRAME-LENGTH')`,
    'rejected frame size identifies the source field',
  );
  assert.equal(
    (await invoke('inspect_integration')).description.signals.find((item) => item.port === tx.port)
      .dlc,
    tx.dlc,
  );
  await unchanged(original);
  await click('还原更改');
  await save();
  await refreshSurface(manifest);
  const reopened = (await invoke('inspect_integration')).description.signals.find(
    (item) => item.port === tx.port,
  );
  assert.equal(reopened.canId, 65);
  assert.equal(reopened.extended, true);
  const result = await projection();
  assert.ok(
    result.fields.some(
      (item) => item.linkedEdit?.kind === 'can-id' && item.linkedEdit.groupId === tx.port,
    ),
  );
  const invalid = { changes: { canIds: { [tx.port]: 0x20000000 }, applicationPeriodMs: null } };
  await refuse('edit_integration', invalid, '29-bit CAN ID range');
  const initialization = await invoke('preview_application_initialization');
  await invoke('initialize_application_previewed', { preview: initialization });
  await exportSource('ExtendedIds');
});

check('layer selection previews semantic differences and saves the selected variant', async () => {
  const { directory, manifest } = await create('SchedulerVariants');
  const projectionBefore = await projection();
  const initialization = await invoke('preview_application_initialization');
  await invoke('initialize_application_previewed', { preview: initialization });
  const project = JSON.parse(await readFile(manifest, 'utf8'));
  const members = project.inputs.map((item) => item.path).sort();
  const digest = createHash('sha256');
  function frame(bytes) {
    const length = Buffer.alloc(8);
    length.writeBigUInt64LE(BigInt(bytes.length));
    digest.update(length).update(bytes);
  }
  frame(Buffer.from('autosar-project-base-v1'));
  for (const member of members) {
    frame(Buffer.from(member));
    frame(
      createHash('sha256')
        .update(await readFile(path.join(directory, member)))
        .digest(),
    );
  }
  const reference = { baseId: 'shared', revision: 'first', sourceSha256: digest.digest('hex') };
  const empty = { members: [], operations: [] };
  const priority = fieldBy(projectionBefore, '/OsTaskPriority');
  const owner = projectionBefore.objects.find((item) => item.objectId === priority.objectId);
  const target = {
    object: { release: 'R24-11', definitionType: owner.definitionId, path: owner.path },
    field: priority.definitionId,
  };
  const selected = priority.current.value.lexeme === '8' ? '7' : '8';
  project.layers = {
    baseReference: reference,
    base: {
      reference,
      release: 'R24-11',
      rules: projectionBefore.ruleSetIdentity,
      extensions: [],
      members,
    },
    project: empty,
    groups: {
      scheduler: {
        exactlyOne: true,
        options: {
          platform: empty,
          urgent: {
            members: [],
            operations: [
              {
                target,
                action: { action: 'set', values: [{ kind: 'integer', lexeme: selected }] },
              },
            ],
          },
        },
      },
    },
    selection: { scheduler: ['platform'] },
  };
  await writeFile(manifest, JSON.stringify(project));
  await refreshSurface(manifest);
  await menu('标准输入');
  const radio = field('urgent').expression;
  await until(`Boolean(${radio})`, 'configured variant choice');
  await evaluate(`(${radio}).focus()`);
  await key(' ');
  await click('比较变体');
  await until(
    `document.querySelector('.integration-view').innerText.includes('OsTaskPriority')`,
    'semantic priority difference',
  );
  await click('应用更改');
  await until(
    `document.querySelector('.integration-view input[type=radio]:checked')?.parentElement.textContent.trim() === 'urgent' && [...document.querySelectorAll('.integration-view button')].some(node => node.textContent.trim() === '预览保存' && !node.disabled)`,
    'selected variant applied',
  );
  await save();
  await refreshSurface(manifest);
  const reopened = await invoke('inspect_integration');
  assert.deepEqual(reopened.layers.groups.scheduler.selected, ['urgent']);
  await exportSource('SchedulerVariants');
});
