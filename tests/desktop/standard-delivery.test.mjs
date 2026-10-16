import assert from 'node:assert/strict';
import { access, mkdir, readFile, writeFile, rename, cp } from 'node:fs/promises';
import path from 'node:path';
import { openWorkbench, existing, sha256, json, applicationPeriod } from './support/workbench.mjs';

const {
  check,
  evaluate,
  scratch,
  until,
  invoke,
  projection,
  click,
  input,
  label,
  menu,
  choose,
  refreshSurface,
  clipboard,
  sourceBytes,
  unchanged,
  apply,
  refuse,
  save,
  create,
  awaitProjectSurfaceReady,
  awaitStandardInspectionReady,
  sourceSurface,
  exportSource,
  packageBytes,
  resealPrivateVariant,
} = await openWorkbench('standard-delivery');

let gatewayDelivery;
check('seven-source-standard-variant', async () => {
  await create('GatewayReference');
  const view = await projection();
  assert.equal(view.sources.length, 7);
  const savedBytes = await sourceBytes(view);
  await until(
    `Boolean(document.querySelector('[aria-label="标准输入工作区"] input[aria-label="接收 CAN ID"]:not(:disabled)'))`,
    'automatic standard inspection editor ready',
  );
  // This is the existing bounded standard editor, not a new private grammar.
  await menu('标准输入');
  await input(label('接收 CAN ID'), '1104');
  await input(label('发送 CAN ID'), '1105');
  await input(label('应用周期'), '20');
  await click('应用并校验修改');
  await until(`document.body.innerText.includes('尚未保存')`, 'real standard variant apply');
  await unchanged(savedBytes);
  await click('预览保存');
  await until(
    `Boolean(document.querySelector('[aria-label="确认保存源配置"]'))`,
    'standard source preview',
  );
  await unchanged(savedBytes);
  await click('确认保存', "document.querySelector('[role=dialog]')");
  await until(`!document.querySelector('[role=dialog]')`, 'confirmed standard save');
  await click('重开来源');
  await until(
    `Boolean(document.querySelector('[aria-label="标准输入工作区"] input[aria-label="接收 CAN ID"]:not(:disabled)'))`,
    'standard reopen operation completed',
  );
  const actual = await invoke('inspect_integration');
  assert.equal(applicationPeriod(actual.description), 20);
  assert.deepEqual(
    actual.description.signals.map((signal) => signal.canId),
    [1104, 1105],
  );
  for (const source of savedBytes.filter((source) =>
    ['types.arxml', 'services.arxml', 'composition.arxml'].includes(path.basename(source.path)),
  ))
    assert.deepEqual(await readFile(source.path), source.bytes);
  await sourceSurface(await projection());
  // Mainline deliveries compile the project's own user application.
  await menu('预览初始化用户应用…');
  await until(
    `Boolean(document.querySelector('[role=dialog]'))`,
    'application initialization preview',
  );
  await click('确认创建用户应用与成员记录', "document.querySelector('[role=dialog]')");
  await until(`!document.querySelector('[role=dialog]')`, 'application initialization confirmed');
  await awaitProjectSurfaceReady();
  gatewayDelivery = await exportSource('GatewayReference-source');
});
check('v2-relocation-snapshot-import-and-refusals', async () => {
  const delivery = gatewayDelivery;
  const previous = await projection();
  const liveBytes = await sourceBytes(previous);
  const originalDirectory = delivery.directory;
  const relocated = path.join(scratch, 'deliveries', 'RelocatedGateway-source');
  await rename(originalDirectory, relocated);
  await assert.rejects(access(originalDirectory), { code: 'ENOENT' });
  delivery.directory = relocated;
  const immutable = await packageBytes(relocated);
  const destination = path.join(scratch, 'projects', 'ImportedGatewaySnapshot');
  await mkdir(destination);
  await choose(
    'directory',
    relocated,
    async () => {
      await menu('导入 ECU 交接包…');
      await click('选择交付包', "document.querySelector('[role=dialog]')");
    },
    async () => {
      await until(
        `document.querySelector('[role=dialog][aria-label="导入封存交付包"] [aria-label="交付包目录"]')?.value === ${json(relocated)}`,
        'actual v2 import destination dialog',
      );
      return evaluate(
        `document.querySelector('[role=dialog][aria-label="导入封存交付包"] [aria-label="交付包目录"]').value`,
      );
    },
  );
  await choose(
    'directory',
    destination,
    () => click('选择新空目录', "document.querySelector('[role=dialog]')"),
    async () => {
      await until(
        `document.querySelector('[role=dialog] [aria-label="新的空 live 工作目录"]')?.value === ${json(destination)}`,
        'actual v2 import destination selection',
      );
      return evaluate(
        `document.querySelector('[role=dialog] [aria-label="新的空 live 工作目录"]').value`,
      );
    },
  );
  await click('核验并导入交付包', "document.querySelector('[role=dialog]')");
  await until(
    `!document.querySelector('[role=dialog]') && Boolean(document.querySelector('.document-tabs'))`,
    'actual v2 source reconstruction published',
  );
  // Inspecting the reconstructed seven sources takes minutes in a debug build.
  await awaitStandardInspectionReady(300000);
  const imported = await projection();
  assert.notEqual(imported.workspaceEpoch, previous.workspaceEpoch);
  assert.equal(imported.sources.length, 7);
  const metadata = JSON.parse(await readFile(path.join(relocated, 'meta/handoff.json'), 'utf8'));
  for (const input of metadata.inputSnapshots) {
    assert.deepEqual(
      await readFile(path.join(destination, input.logicalPath)),
      await readFile(path.join(relocated, input.packagePath)),
      'Reimport reconstructs independent exact source snapshots',
    );
  }
  await unchanged(immutable);
  await unchanged(liveBytes);
  await refuse(
    'read_project_source',
    { sourceId: previous.sources[0].sourceId },
    'V2 import rejects identities from the previous live project',
  );
  const plan = await invoke('inspect_integration');
  assert.equal(applicationPeriod(plan.description), 20);
  assert.deepEqual(
    plan.description.signals.map((signal) => signal.canId),
    [1104, 1105],
  );
  await refuse(
    'open_handoff_project',
    { directory: relocated, newWorkspaceDirectory: destination },
    'V2 import cannot overwrite an occupied live project',
  );
  const retained = await sourceBytes(imported);
  for (const kind of [
    'unknownFormat',
    'ruleIdentity',
    'duplicateSnapshots',
    'ownerElevation',
    'payloadBytes',
  ]) {
    const variant = path.join(scratch, 'deliveries', `v2-refused-${kind}`);
    await cp(relocated, variant, {
      recursive: true,
      errorOnExist: true,
      force: false,
    });
    if (kind === 'ownerElevation') {
      const ledgerPath = path.join(variant, 'meta/workbench-ownership.json');
      const ledger = JSON.parse(await readFile(ledgerPath, 'utf8'));
      ledger.files.find((entry) => entry.path === metadata.inputSnapshots[0].packagePath).owner =
        'user-application';
      await writeFile(ledgerPath, json(ledger));
      await resealPrivateVariant(variant);
    } else if (kind === 'payloadBytes') {
      const source = path.join(variant, metadata.inputSnapshots[0].packagePath);
      await writeFile(
        source,
        Buffer.concat([await readFile(source), Buffer.from('\n<!-- external snapshot edit -->\n')]),
      );
    } else {
      const altered = structuredClone(metadata);
      if (kind === 'unknownFormat') altered.format = 'unknown-handoff-v99';
      if (kind === 'ruleIdentity')
        altered.resourceIdentities.ruleSetIdentity.sha256 = '0'.repeat(64);
      if (kind === 'duplicateSnapshots')
        altered.inputSnapshots.push({ ...altered.inputSnapshots[0] });
      await writeFile(path.join(variant, 'meta/handoff.json'), json(altered));
      const targetPath = path.join(variant, 'target.json');
      const target = JSON.parse(await readFile(targetPath, 'utf8'));
      target.nativeDelivery = altered;
      await writeFile(targetPath, json(target));
      await resealPrivateVariant(variant);
    }
    const variantBytes = await packageBytes(variant);
    const refusedDestination = path.join(scratch, 'projects', `RefusedV2-${kind}`);
    await refuse(
      'open_handoff_project',
      { directory: variant, newWorkspaceDirectory: refusedDestination },
      `V2 ${kind} refusal`,
    );
    await assert.rejects(access(refusedDestination), { code: 'ENOENT' });
    await unchanged(variantBytes);
    await unchanged(retained);
  }
  const generatedMetadata = path.join(relocated, 'meta/handoff.json');
  const originalMetadata = await readFile(generatedMetadata);
  const modifiedMetadata = Buffer.concat([originalMetadata, Buffer.from(' ')]);
  await writeFile(generatedMetadata, modifiedMetadata);
  await refuse(
    'preview_ecu_project',
    { outputDirectory: relocated, handoff: true },
    'Modified generator-owned delivery cannot be silently overwritten',
  );
  assert.deepEqual(await readFile(generatedMetadata), modifiedMetadata);
  await writeFile(generatedMetadata, originalMetadata);
  await unchanged(immutable);
  await refreshSurface(imported.projectPath);
});
