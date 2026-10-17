import assert from 'node:assert/strict';
import path from 'node:path';
import { readFile, writeFile } from 'node:fs/promises';
import { dialogNamed, openWorkbench } from './support/workbench.mjs';

const {
  check,
  scratch,
  projection,
  selectObject,
  field,
  input,
  click,
  until,
  invoke,
  refreshSurface,
  refuse,
} = await openWorkbench('incomplete-configuration');

check('create-apply-save-reopen-incomplete-canif-through-workbench', async () => {
  const source = path.join(scratch, 'incomplete.arxml');
  await writeFile(
    source,
    '<?xml version="1.0" encoding="UTF-8"?><AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Configuration</SHORT-NAME><ELEMENTS><ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>Can</SHORT-NAME><DEFINITION-REF DEST="ECUC-MODULE-DEF">/AUTOSAR/EcucDefs/Can</DEFINITION-REF></ECUC-MODULE-CONFIGURATION-VALUES></ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>',
  );
  await refreshSurface(null, [source]);
  let view = await projection();
  const parent = view.objects.find((o) => o.kind === 'AR-PACKAGE');
  await selectObject(parent.objectId);
  await input(field('添加实例'), '/AUTOSAR/EcucDefs/CanIf');
  await input(field('SHORT-NAME'), 'NewCanIf');
  await click('预览更改');
  await until(`Boolean(${dialogNamed('确认应用更改')})`, 'creation preview');
  await click('应用更改', "document.querySelector('[role=dialog]')");
  await until(
    `!document.querySelector('[role=dialog]') && document.body.innerText.includes('尚未保存')`,
    'creation applied',
  );
  view = await projection();
  const module = view.objects.find((o) => o.shortName === 'NewCanIf');
  assert(module);
  for (const name of ['CanIfCtrlDrvCfg', 'CanIfInitCfg', 'CanIfPublicCfg']) {
    assert(
      view.objects.some(
        (o) => o.parentId === module.objectId && o.definitionId.endsWith(`/${name}`),
      ),
      `Mandatory ${name} exists`,
    );
  }
  const shortfalls = view.diagnostics.filter(
    (d) => d.code === 'MULTIPLICITY_LOWER' && d.path?.startsWith(module.path),
  );
  assert(shortfalls.length > 0);
  assert(shortfalls.every((d) => d.objectId && d.severity === 'error'));
  const bytesBefore = await readFile(source);
  await click('预览保存');
  await until(`Boolean(${dialogNamed('保存配置')})`, 'save preview');
  assert.deepEqual(await readFile(source), bytesBefore);
  await click('确认保存', "document.querySelector('[role=dialog]')");
  await until(`!document.querySelector('[role=dialog]')`, 'save completed');
  assert.equal((await projection()).dirty, false);
  const savedObjects = view.objects.map((o) => [o.path, o.definitionId]);
  const saved = await readFile(source);
  assert.notDeepEqual(saved, bytesBefore);
  await refreshSurface(null, [source]);
  view = await projection();
  assert.deepEqual(
    view.objects.map((o) => [o.path, o.definitionId]),
    savedObjects,
  );
  assert.deepEqual(
    view.diagnostics
      .filter((d) => d.code === 'MULTIPLICITY_LOWER' && d.path?.startsWith(module.path))
      .map((d) => d.path),
    shortfalls.map((d) => d.path),
  );
  const duplicate = {
    op: 'create-instance',
    changeId: 'duplicate',
    parent: {
      kind: 'existing',
      objectId: view.objects.find((o) => o.kind === 'AR-PACKAGE').objectId,
    },
    sourceId: view.sources[0].sourceId,
    definitionId: module.definitionId,
    shortName: 'DuplicateCanIf',
  };
  await refuse(
    'prepare_configuration_change',
    {
      changeSet: {
        workspaceEpoch: view.workspaceEpoch,
        inputFingerprint: view.inputFingerprint,
        definitionFingerprint: view.definitionFingerprint,
        changes: [duplicate],
      },
    },
    'Second CanIf rejects atomically',
  );
  const inspection = await invoke('inspect_integration');
  assert.equal(inspection.description, null);
  assert(
    inspection.diagnostics.some(
      (d) => d.code === 'MULTIPLICITY_LOWER' && d.object?.startsWith(module.path),
    ),
  );
  assert.deepEqual(await readFile(source), saved);
  await click('问题', 'document.querySelector(\'[aria-label="底部工具窗口"]\')');
  await until(
    `document.querySelector('.problems-table')?.innerText.includes('缺少必需的')`,
    'readable mandatory-entry message in the real problems list',
  );
});
