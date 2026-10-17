import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { openWorkbench } from './support/workbench.mjs';

const {
  check,
  evaluate,
  key,
  until,
  invoke,
  metrics,
  projection,
  click,
  input,
  label,
  field,
  menu,
  memorySample,
  importAuthoredInputs,
} = await openWorkbench('responsiveness', { delayOperations: true });

let authored;
check('expanded-input-responsiveness-and-cancel', async () => {
  authored = await importAuthoredInputs();
  const before = await projection();
  await memorySample('before-expanded-input');
  await menu('导入 ARXML…');
  await input(field('ARXML 路径（每行一份）'), authored.large);
  await click('导入');
  await until(
    `(async()=>Boolean((await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation))()`,
    'real pending parse observed',
    10000,
  );
  const responseStart = performance.now();
  await key('4', { alt: true });
  await until(
    `Boolean(document.querySelector('[role=region][aria-label="构建"]'))`,
    'tool remains responsive during parse',
    5000,
  );
  await click('取消操作', "document.querySelector('.context-toolbar')");
  await until(
    `(async()=>!(await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation)()`,
    'actual cancel closure',
  );
  assert.equal(
    (await projection()).workspaceEpoch,
    before.workspaceEpoch,
    'Cancelled parse cannot replace the current project',
  );
  assert.deepEqual((await projection()).sources, before.sources);
  const cancelledResponseMs = performance.now() - responseStart;
  // Complete a separate real open of the expanded input, then compare the
  // same frozen native snapshot across repeated filter/expansion actions.
  await menu('导入 ARXML…');
  await input(field('ARXML 路径（每行一份）'), authored.large);
  await click('导入');
  await until(
    `!document.querySelector('.start-page') && Boolean(document.querySelector('.document-tabs'))`,
    'expanded input published',
    300000,
  );
  const expanded = await projection();
  assert(expanded.objects.some((object) => object.shortName === 'Preserved_15999'));
  await memorySample('expanded-input-opened');
  const beforeFilters = await metrics();
  const timings = [];
  for (const term of ['Preserved_15999', 'Acceptance', 'Preserved_42', '']) {
    const start = performance.now();
    await input(label('搜索工程树名称、路径、定义、值'), term);
    await evaluate(`document.querySelector('.tool-rail [aria-label="构建 Alt+4"]').focus()`);
    await key('4', { alt: true });
    await until(
      `Boolean(document.querySelector('[role=region][aria-label="构建"]'))`,
      'expanded filtering is responsive',
      5000,
    );
    timings.push({ term, elapsedMs: performance.now() - start });
  }
  await evaluate(`document.querySelector('[role=treeitem][aria-expanded]')?.focus()`);
  for (let index = 0; index < 12; index += 1) {
    await key('ArrowLeft');
    await key('ArrowRight');
  }
  const afterFilters = await metrics();
  for (const name of [
    'sourceReads',
    'snapshotBuilds',
    'ruleLoads',
    'grammarLoads',
    'definitionReads',
    'sourceScans',
  ]) {
    assert.equal(
      afterFilters[name],
      beforeFilters[name],
      `Same-snapshot filter/expand must reuse actual ${name}`,
    );
  }
  assert(BigInt(beforeFilters.sourceReads) > 0n && BigInt(beforeFilters.snapshotBuilds) > 0n);
  assert.deepEqual(
    await projection(),
    expanded,
    'Filter/expand cannot alter native source snapshot or IDs',
  );
  await memorySample('expanded-filtered-repeatedly');
});
