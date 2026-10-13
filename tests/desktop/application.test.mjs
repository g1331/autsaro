import assert from 'node:assert/strict';
import { access, mkdir, readFile, writeFile, unlink } from 'node:fs/promises';
import path from 'node:path';
import { openWorkbench, sha256, json, SLOT } from './support/workbench.mjs';

const {
  check,
  scratch,
  until,
  capabilities,
  invoke,
  projection,
  click,
  menu,
  refreshSurface,
  sourceBytes,
  unchanged,
  refuse,
  create,
  exportSource,
  packageBytes,
} = await openWorkbench('application');

check('live-application-initialization-and-immutable-snapshot', async () => {
  await create('ApplicationProbe');
  const before = await projection();
  const manifestPath = before.projectPath;
  const root = path.dirname(manifestPath);
  const manifestBytes = await readFile(manifestPath);
  const originalSources = await sourceBytes(before);
  const preview = await invoke('preview_application_initialization');
  assert.equal(preview.slots.length, 1);
  assert.equal(preview.slots[0].producerSlot, SLOT);
  assert.equal(preview.files.length, 1);
  assert.deepEqual(
    preview.slots[0].sourcePaths,
    preview.files.map((file) => file.path),
  );
  assert(
    preview.slots[0].generatedHeaders.some((header) => header.startsWith('include/Rte_')),
    'Actual application contract supplies generated RTE headers',
  );
  const live = path.join(root, preview.files[0].path);
  await assert.rejects(access(live), { code: 'ENOENT' });
  assert.deepEqual(
    await readFile(manifestPath),
    manifestBytes,
    'Initialization preview cannot change membership',
  );
  await refuse(
    'initialize_application_previewed',
    { preview: { ...preview, revision: '0'.repeat(64) } },
    'Modified application preview cannot publish',
  );
  await mkdir(path.dirname(live), { recursive: true });
  const occupied = Buffer.from(
    '/* Independently created user file: initialization must not replace it. */\n',
  );
  await writeFile(live, occupied);
  await refuse(
    'initialize_application_previewed',
    { preview },
    'Application initialization is create-only',
  );
  assert.deepEqual(await readFile(live), occupied);
  assert.deepEqual(await readFile(manifestPath), manifestBytes);
  await unlink(live);
  const initializationCommand = '预览初始化用户应用…';
  await menu(initializationCommand);
  await until(
    `Boolean([...document.querySelectorAll('[role=dialog]')].find(node=>node.textContent.includes(${json(SLOT)})))`,
    'real reviewed application initialization',
  );
  assert.deepEqual(await readFile(manifestPath), manifestBytes);
  await assert.rejects(access(live), { code: 'ENOENT' });
  await click('确认创建用户应用与成员记录', "document.querySelector('[role=dialog]')");
  await until(
    `!document.querySelector('[role=dialog]')`,
    'actual application initialization confirmation',
  );
  assert.deepEqual(await readFile(live), Buffer.from(preview.files[0].contents));
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
  assert.deepEqual(manifest.applicationInputs, [
    {
      path: preview.files[0].path,
      producerSlot: preview.slots[0].producerSlot,
    },
  ]);
  await unchanged(originalSources);
  await refuse(
    'preview_application_initialization',
    {},
    'Declared live application cannot be initialized again',
  );
  // Real user behavior: bound the committed/transmitted uint32 value to its
  // lower 31 bits, retaining all declared entry symbols and generated ABI.
  const seed = preview.files[0].contents;
  const writeLine = /^        committed = value;$/m;
  assert.equal([...seed.matchAll(new RegExp(writeLine.source, 'gm'))].length, 1);
  const userCode = seed.replace(writeLine, '        value &= 0x7FFFFFFFu;\n$&');
  await writeFile(live, userCode);
  await refreshSurface(manifestPath);
  const delivered = await exportSource('GatewayUserApplication-source');
  assert.deepEqual(
    await readFile(path.join(delivered.directory, 'app/EchoApplication/EchoApplication.c')),
    Buffer.from(userCode),
    'Compiled user snapshot must contain the actual live bytes',
  );
  const ledger = JSON.parse(
    await readFile(path.join(delivered.directory, 'meta/workbench-ownership.json'), 'utf8'),
  );
  const owner = ledger.files.find(
    (entry) => entry.path === 'app/EchoApplication/EchoApplication.c',
  );
  assert.equal(owner.owner, 'user-application');
  assert.equal(owner.producerId, preview.slots[0].producerSlot);
  assert.equal(owner.snapshotOf, preview.files[0].path);
  const immutable = await packageBytes(delivered.directory);
  const output = path.join(scratch, 'deliveries', 'RefusedLateApplication-source');
  const prepared = await invoke('preview_ecu_project', {
    outputDirectory: output,
    handoff: true,
  });
  const preparedFingerprint = (await capabilities()).fingerprint;
  const laterBytes = Buffer.from(userCode + '\n/* independently changed after preview */\n');
  await writeFile(live, laterBytes);
  await assert.rejects(
    invoke(
      'generate_ecu_project',
      {
        outputDirectory: output,
        handoff: true,
        revision: prepared.revision,
      },
      preparedFingerprint,
    ),
  );
  await assert.rejects(access(output), { code: 'ENOENT' });
  assert.deepEqual(await readFile(live), laterBytes);
  await unchanged(immutable);
  await writeFile(live, userCode);
  await refreshSurface(manifestPath);
  const copiedRoot = path.join(scratch, 'projects', 'CopiedLiveApplication');
  const saveAs = await invoke('preview_save_as_project', {
    directory: copiedRoot,
    name: 'CopiedLiveApplication',
  });
  assert.deepEqual(await readFile(live), Buffer.from(userCode));
  await invoke('save_as_project_previewed', { preview: saveAs });
  assert.deepEqual(
    await readFile(path.join(copiedRoot, preview.files[0].path)),
    Buffer.from(userCode),
    'Save-as preserves actual live user bytes',
  );
  assert.deepEqual(
    JSON.parse(await readFile(path.join(copiedRoot, 'workbench-project.json'), 'utf8'))
      .applicationInputs,
    manifest.applicationInputs,
  );
  await unchanged(immutable);
  const importedRoot = path.join(scratch, 'projects', 'ImportedUserApplicationSnapshot');
  await invoke('open_handoff_project', {
    directory: delivered.directory,
    newWorkspaceDirectory: importedRoot,
  });
  assert.deepEqual(
    await readFile(path.join(importedRoot, preview.files[0].path)),
    Buffer.from(userCode),
    'V2 import restores an independent user application source, not a package path',
  );
  await unchanged(immutable);
  assert.deepEqual(await readFile(live), Buffer.from(userCode));
  await refreshSurface(path.join(importedRoot, 'workbench-project.json'));
});
