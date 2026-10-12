import assert from 'node:assert/strict';
import { readFile, writeFile, rename } from 'node:fs/promises';
import path from 'node:path';
import { createExtensionInputs } from './support/inputs.mjs';
import { openWorkbench, explicit, existing, sha256, json } from './support/workbench.mjs';

const {
  check,
  scratch,
  invoke,
  projection,
  input,
  refreshSurface,
  sourceBytes,
  unchanged,
  changeSet,
  valueChange,
  apply,
  refuse,
  save,
  fieldBy,
  create,
  importAuthoredInputs,
} = await openWorkbench('value-editing');

let authored;
check('seven-value-types-and-cross-source-batch', async () => {
  authored = await importAuthoredInputs();
  const view = await projection();
  assert.deepEqual(
    new Set(view.fields.filter((field) => field.writable).map((field) => field.kind)),
    new Set(['integer', 'float', 'boolean', 'enumeration', 'string', 'function-name', 'reference']),
  );
  const expected = [
    ['/OsEventMask', '18446744073709551615'],
    ['/ComTimeout', '0.0500'],
    ['/CanDevErrorDetect', 'true'],
    ['/OsTaskSchedule', 'NON'],
    ['/CanIfInitCfgSet', 'IndependentNativeInit'],
    ['/DcmDspDataReadFnc', 'NativeAcceptance_Read'],
  ];
  const fields = expected.map(([suffix, lexeme]) => [fieldBy(view, suffix), lexeme]);
  assert(
    new Set(
      fields.map(
        ([field]) => view.objects.find((object) => object.objectId === field.objectId).sourceId,
      ),
    ).size >= 4,
    'Batch must cross real file/module boundaries',
  );
  const referenceField = fieldBy(view, '/OsTaskEventRef');
  const candidate = view.referenceCandidates.find(
    (candidate) =>
      candidate.fieldId === referenceField.fieldId &&
      candidate.path === '/Acceptance/Os/DiagnosticReady',
  );
  assert(candidate);
  assert.notEqual(
    candidate.targetId,
    referenceField.reference.target.objectId,
    'Ordinary reference edit must select a genuinely different target',
  );
  const changes = fields.map(([field, lexeme]) => valueChange(field, lexeme));
  changes.push({
    op: 'set-reference',
    changeId: 'actual-reference',
    field: { kind: 'existing', fieldId: referenceField.fieldId },
    expected: referenceField.reference,
    value: {
      state: 'explicit',
      rawPath: candidate.path,
      dest: candidate.dest,
      target: existing(candidate.targetId),
    },
  });
  const applied = await apply(view, changes);
  assert.equal(
    applied.outcome.projection.references.find((edge) => edge.fieldId === referenceField.fieldId)
      .rawPath,
    '/Acceptance/Os/DiagnosticReady',
  );
  for (const [field, lexeme] of fields)
    assert.equal(
      applied.outcome.projection.fields.find((item) => item.fieldId === field.fieldId).current.value
        .lexeme,
      lexeme,
    );
  const preserved = view.sources.find((source) => path.basename(source.path) === 'unknown.arxml');
  const bytes = await readFile(preserved.path);
  const unknownObject = view.objects.find((object) => object.shortName === 'PreservedType');
  assert(unknownObject && !unknownObject.writable);
  assert(
    view.validation.some((scope) => scope.status === 'unsupported' || scope.status === 'failed'),
    'Partial/unknown input must not be all-domain passed',
  );
  await save();
  assert.deepEqual(await readFile(preserved.path), bytes);
  const reopened = await refreshSurface(null, authored.sources);
  for (const [suffix, lexeme] of expected)
    assert.equal(fieldBy(reopened, suffix).current.value.lexeme, lexeme);
  await refuse(
    'prepare_configuration_change',
    {
      changeSet: changeSet(reopened, [
        valueChange(fieldBy(reopened, '/OsEventMask'), '9007199254740993'),
        valueChange(fieldBy(reopened, '/ComTimeout'), 'NaN'),
      ]),
    },
    'Nonfinite member refuses entire batch',
  );
  await refuse(
    'prepare_configuration_change',
    {
      changeSet: changeSet(reopened, [
        valueChange(fieldBy(reopened, '/DcmDspDataReadFnc'), 'Read()'),
      ]),
    },
    'Illegal function-name refuses edit',
  );
  const badReference = {
    op: 'set-reference',
    changeId: 'wrong-destination',
    field: {
      kind: 'existing',
      fieldId: fieldBy(reopened, '/OsTaskEventRef').fieldId,
    },
    expected: fieldBy(reopened, '/OsTaskEventRef').reference,
    value: {
      state: 'explicit',
      rawPath: candidate.path,
      dest: 'ECUC-MODULE-CONFIGURATION-VALUES',
      target: existing(reopened.objects.find((object) => object.path === candidate.path).objectId),
    },
  };
  await refuse(
    'prepare_configuration_change',
    { changeSet: changeSet(reopened, [badReference]) },
    'Wrong reference DEST',
  );
});
check('absent-default-explicit-empty-and-removal', async () => {
  const view = await projection();
  const defaultField = fieldBy(view, '/CanVersionInfoApi');
  assert.deepEqual(defaultField.current, { state: 'absent' });
  assert.deepEqual(defaultField.defaultValue, {
    kind: 'boolean',
    lexeme: 'false',
  });
  assert(defaultField.defaultOrigin.startsWith('builtin:'));
  const callback = fieldBy(view, '/DcmDspDataReadFnc');
  const text = fieldBy(view, '/CanIfInitCfgSet');
  const source = view.sources.find((source) => path.basename(source.path) === 'can.arxml');
  assert(
    !(await readFile(source.path, 'utf8')).includes('CanVersionInfoApi'),
    'A default is not an implicitly written value',
  );
  const result = await apply(view, [
    valueChange(defaultField, defaultField.defaultValue.lexeme),
    valueChange(text, ''),
    { ...valueChange(callback, ''), value: { state: 'absent' } },
  ]);
  assert.deepEqual(
    fieldBy(result.outcome.projection, '/CanVersionInfoApi').current,
    explicit('boolean', 'false'),
  );
  assert.deepEqual(
    fieldBy(result.outcome.projection, '/CanIfInitCfgSet').current,
    explicit('string', ''),
  );
  assert.deepEqual(fieldBy(result.outcome.projection, '/DcmDspDataReadFnc').current, {
    state: 'absent',
  });
  await save();
  const reopened = await refreshSurface(null, authored.sources);
  assert.deepEqual(fieldBy(reopened, '/CanVersionInfoApi').current, explicit('boolean', 'false'));
  assert.deepEqual(fieldBy(reopened, '/CanIfInitCfgSet').current, explicit('string', ''));
  assert.deepEqual(fieldBy(reopened, '/DcmDspDataReadFnc').current, {
    state: 'absent',
  });
});
check('source-safety-rejections', async () => {
  for (const [kind, source] of Object.entries(authored.safety))
    await refuse('open_project', { paths: [source] }, `${kind} source-safety refusal`);
  const view = await projection();
  const field = fieldBy(view, '/OsEventMask');
  await apply(view, [valueChange(field, '9007199254740993')]);
  const preview = await invoke('preview_save_project');
  const current = await projection();
  const source = current.sources.find((source) => path.basename(source.path) === 'os.arxml');
  const saved = await readFile(source.path);
  const ownedBeforeRefusal = await invoke('read_project_source', {
    sourceId: source.sourceId,
  });
  const changed = Buffer.concat([saved, Buffer.from('\n<!-- external change -->\n')]);
  await writeFile(source.path, changed);
  await assert.rejects(invoke('save_project', { revision: preview.revision }));
  assert.deepEqual(
    await readFile(source.path),
    changed,
    'Refused save must retain externally changed bytes',
  );
  assert.equal(
    await invoke('read_project_source', { sourceId: source.sourceId }),
    ownedBeforeRefusal,
    'Owned source snapshot remains intact after external refusal',
  );
  await writeFile(source.path, saved);
  await save();
});
check('extension-isolation-and-default-preservation', async () => {
  const extension = await createExtensionInputs(scratch);
  const original = await projection();
  for (const [name, catalogPath] of Object.entries(extension.variants))
    await refuse('import_definition_catalog', { catalogPath }, `Extension ${name} rejection`);
  const accepted = await invoke('import_definition_catalog', {
    catalogPath: extension.valid,
  });
  assert.notEqual(accepted.definitionFingerprint, original.definitionFingerprint);
  assert.deepEqual(accepted.ruleSetIdentity, original.ruleSetIdentity);
  assert.deepEqual(accepted.acceptedExtensionDefinitions, [extension.identity]);
  const cached = path.join(
    scratch,
    'app-config',
    'definition-catalogs',
    extension.identity.sha256,
    'lab.arxml',
  );
  const cacheBytes = await readFile(cached);
  await writeFile(cached, Buffer.concat([cacheBytes, Buffer.from('\n<!-- corruption -->')]));
  await refuse(
    'import_definition_catalog',
    { catalogPath: extension.valid },
    'Existing immutable cache corruption',
  );
  await writeFile(cached, cacheBytes);
  const packages = accepted.objects.filter((object) => object.path === '/Acceptance');
  assert(
    packages.length > 1 && packages.every((object) => object.writable),
    'Split package fragments support source-local child creation',
  );
  const parent = packages.find(
    (object) =>
      object.sourceId ===
      accepted.sources.find((source) => path.basename(source.path) === 'os.arxml').sourceId,
  );
  assert(parent, 'Extension creation names the owned package source');
  const created = await apply(accepted, [
    {
      op: 'create-instance',
      changeId: 'extension-module',
      parent: existing(parent.objectId),
      sourceId: parent.sourceId,
      definitionId: '/AcceptanceVendor/Lab',
      shortName: 'LabConfiguration',
    },
    {
      op: 'create-instance',
      changeId: 'extension-config',
      parent: { kind: 'created', changeId: 'extension-module' },
      sourceId: parent.sourceId,
      definitionId: '/AcceptanceVendor/Lab/Config',
      shortName: 'Config',
    },
  ]);
  const counter = fieldBy(created.outcome.projection, '/AcceptanceVendor/Lab/Config/Counter');
  assert.deepEqual(counter.current, { state: 'absent' });
  assert.deepEqual(counter.defaultValue, {
    kind: 'integer',
    lexeme: '9007199254740993',
  });
  const originalBytes = await sourceBytes(created.outcome.projection);
  const directory = path.join(scratch, 'projects', 'ExtensionConsumers');
  const portable = await invoke('preview_save_as_project', {
    directory,
    name: 'ExtensionConsumers',
  });
  await invoke('save_as_project_previewed', { preview: portable });
  await unchanged(originalBytes);
  const manifest = path.join(directory, 'workbench-project.json');
  assert.deepEqual(JSON.parse(await readFile(manifest, 'utf8')).acceptedExtensionDefinitions, [
    extension.identity,
  ]);
  await invoke('open_member_project', { path: manifest });
  assert.deepEqual((await projection()).acceptedExtensionDefinitions, [extension.identity]);
  const portableBytes = [
    ...(await sourceBytes(await projection())),
    { path: manifest, bytes: await readFile(manifest) },
  ];
  const cacheRoot = path.dirname(cached);
  const unavailableCache = path.join(scratch, 'cache-temporarily-unavailable');
  await rename(cacheRoot, unavailableCache);
  const missing = await invoke('open_member_project', { path: manifest });
  const missingView = await projection();
  assert.deepEqual(missingView.acceptedExtensionDefinitions, [extension.identity]);
  assert(
    missingView.extensionDefinitions.some(
      (item) => item.identity.catalogId === extension.identity.catalogId && !item.available,
    ),
  );
  const missingConsumer = missingView.objects.find(
    (object) => object.definitionId === '/AcceptanceVendor/Lab/Config',
  );
  assert(missingConsumer, 'The persisted extension consumer remains present');
  assert.equal(
    missingConsumer.writable,
    false,
    'Only the missing-extension consumer loses edit capability',
  );
  assert.equal(
    fieldBy(missingView, '/OsEventMask').writable,
    true,
    'Unrelated builtin consumer stays editable',
  );
  assert(missing.files.length > 0);
  await unchanged(portableBytes);
  await rename(unavailableCache, cacheRoot);
  await invoke('open_member_project', { path: manifest });
  const removed = await invoke('remove_definition_catalog', {
    catalogId: extension.identity.catalogId,
  });
  assert.deepEqual(removed.acceptedExtensionDefinitions, []);
  assert.deepEqual(removed.ruleSetIdentity, original.ruleSetIdentity);
  await unchanged(portableBytes);
});
