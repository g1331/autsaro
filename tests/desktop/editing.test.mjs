import assert from 'node:assert/strict';
import { openWorkbench, explicit, existing } from './support/workbench.mjs';

const {
  check,
  invoke,
  projection,
  input,
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
