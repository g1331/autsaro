import assert from 'node:assert/strict';
import path from 'node:path';
import { readFile, writeFile } from 'node:fs/promises';
import { openWorkbench } from './support/workbench.mjs';

const {
  check,
  scratch,
  evaluate,
  screenshot,
  key,
  projection,
  selectObject,
  field,
  input,
  click,
  until,
  refreshSurface,
} = await openWorkbench('rejected-change');

const json = JSON.stringify;
const details = `document.querySelector('#rejection-details')`;
const toolsHandle = `document.querySelector('.panel-resize-tools')`;
// Moving focus must not scroll the fixed window frame out of view.
const frameInPlace = `document.querySelector('[role=menubar]').getBoundingClientRect().top >= 0`;

check('rejected-change-summary-details-and-locate', async () => {
  const source = path.join(scratch, 'rejected.arxml');
  await writeFile(
    source,
    '<?xml version="1.0" encoding="UTF-8"?><AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Configuration</SHORT-NAME><ELEMENTS><ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>CanIf</SHORT-NAME><DEFINITION-REF DEST="ECUC-MODULE-DEF">/AUTOSAR/EcucDefs/CanIf</DEFINITION-REF><CONTAINERS><ECUC-CONTAINER-VALUE><SHORT-NAME>CanIfPublicCfg</SHORT-NAME><DEFINITION-REF DEST="ECUC-PARAM-CONF-CONTAINER-DEF">/AUTOSAR/EcucDefs/CanIf/CanIfPublicCfg</DEFINITION-REF></ECUC-CONTAINER-VALUE></CONTAINERS></ECUC-MODULE-CONFIGURATION-VALUES></ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>',
  );
  await refreshSurface(null, [source]);
  const view = await projection();
  const canIf = view.objects.find((o) => o.kind === 'ECUC-MODULE-CONFIGURATION-VALUES');
  const bytes = await readFile(source);
  await selectObject(canIf.objectId);
  // The structure editor still offers a full definition; the definition check refuses it.
  await input(field('添加实例'), '/AUTOSAR/EcucDefs/CanIf/CanIfPublicCfg');
  await input(field('SHORT-NAME'), 'SecondPublicCfg');
  await click('预览更改');
  const summary = `document.querySelector('.notice.summary')`;
  await until(`Boolean(${summary})`, 'rejected change summary');
  assert.equal(await evaluate(`Boolean(document.querySelector('[role=dialog]'))`), false);
  const headline = await evaluate(`${summary}.textContent`);
  assert.match(headline, /查看详情/);
  assert.deepEqual(await readFile(source), bytes, 'A rejected change writes nothing');
  await screenshot('rejected-change-summary.png');

  await click('查看详情', summary);
  await until(`document.activeElement === ${details}`, 'rejection details focused');
  assert(await evaluate(frameInPlace), 'Focusing the details keeps the window frame in place');
  const fields = await evaluate(`(() => {
    const item = ${details}.querySelector('.rejection-item');
    return Object.fromEntries([...item.querySelectorAll('dt')].map(term => [term.textContent, term.nextElementSibling.textContent]));
  })()`);
  assert.equal(fields['严重度'], '错误');
  assert.equal(fields['代码'], 'MULTIPLICITY');
  assert.match(fields['消息'], /CanIfPublicCfg/);
  assert.match(fields['消息'], /允许 1\.\.1 个，现有 2 个/);
  assert.equal(fields['对象路径'], canIf.path);
  assert.equal(fields['来源文件'], view.sources[0].path);
  await screenshot('rejected-change-details.png');

  // The panel height chosen now stays when the tool window is closed and reopened.
  await evaluate(`${toolsHandle}.focus()`);
  const before = Number(await evaluate(`${toolsHandle}.getAttribute('aria-valuenow')`));
  for (let step = 0; step < 4; step++) await key('ArrowUp');
  const resized = Number(await evaluate(`${toolsHandle}.getAttribute('aria-valuenow')`));
  assert.notEqual(resized, before);
  await click('收起工具窗口');
  await until(`!${toolsHandle}`, 'tool window collapsed');
  await click('问题', `document.querySelector('[aria-label=${json('底部工具窗口')}]')`);
  await until(`Boolean(${details})`, 'problems reopened');
  assert.equal(Number(await evaluate(`${toolsHandle}.getAttribute('aria-valuenow')`)), resized);

  // Locating the current object keeps the rejected draft for correction.
  await click('定位', details);
  await until(
    `document.activeElement?.dataset.objectTitle === ${json(canIf.objectId)}`,
    'container heading focused',
  );
  assert.equal(await evaluate(`Boolean(document.querySelector('[role=dialog]'))`), false);
  assert(await evaluate(frameInPlace), 'Locating keeps the window frame in place');
  assert.equal(
    await evaluate(
      `document.querySelector('[aria-label=${json('配置视图')}] [aria-selected=true]')?.textContent`,
    ),
    '容器',
  );
  assert(
    await evaluate(
      `[...document.querySelectorAll('input')].some(node => node.value === 'SecondPublicCfg')`,
    ),
    'The rejected creation is still a draft',
  );
});
