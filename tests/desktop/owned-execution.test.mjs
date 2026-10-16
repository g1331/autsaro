import assert from 'node:assert/strict';
import { access, readdir } from 'node:fs/promises';
import path from 'node:path';
import { openWorkbench, sha256, json } from './support/workbench.mjs';

const {
  check,
  evaluate,
  key,
  pointer,
  scratch,
  until,
  capabilities,
  invoke,
  projection,
  click,
  input,
  label,
  menu,
  clipboard,
  capturedLogs,
  sourceBytes,
  unchanged,
  importAuthoredInputs,
} = await openWorkbench('owned-execution');

let authored;
check('declared-consumer-tools-and-real-owned-failure', async () => {
  authored = await importAuthoredInputs();
  // The test process declares the tools; the application only receives them
  // through its settings UI.
  const tools = {
    compiler: process.env.AUTOSAR_CC,
    objdump: process.env.AUTOSAR_OBJDUMP,
    git: process.env.AUTOSAR_GIT,
    python: process.env.AUTOSAR_PYTHON,
  };
  for (const name of ['compiler', 'objdump', 'git', 'python']) {
    assert.equal(
      typeof tools?.[name],
      'string',
      `Declare ${name} only for the independent consumer/performance phase`,
    );
    assert(path.isAbsolute(tools[name]));
    await access(tools[name]);
  }
  await click('设置');
  await click('执行工具', "document.querySelector('[role=dialog]')");
  for (const [name, field] of [
    ['compiler', '编译器保存路径'],
    ['objdump', 'Objdump保存路径'],
    ['git', 'Git保存路径'],
    ['python', 'Python 解释器保存路径'],
  ]) {
    await input(label(field), tools[name]);
  }
  await click('保存执行工具', "document.querySelector('[role=dialog]')");
  await until(
    `(async()=>!(await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).toolError)()`,
    'declared independent tools accepted',
  );
  await click('关闭', "document.querySelector('[role=dialog]')");
  await until(`!document.querySelector('[role=dialog]')`, 'actual execution settings closed');
  const temporary = path.join(scratch, 'app-temp');
  const before = new Set(await readdir(temporary));
  await menu('验证：受管长日志失败');
  await until(
    `document.querySelector('.notice[role=alert]')?.textContent.includes('退出码 23') && !(document.querySelector('.context-toolbar [role=status]'))`,
    'genuine retained nonzero failure',
  );
  const directories = (await readdir(temporary)).filter(
    (name) => !before.has(name) && name.startsWith('autosar-owned-verification-'),
  );
  assert.equal(directories.length, 1);
  const actualNotice = await evaluate(`document.querySelector('.notice[role=alert]').textContent`);
  assert(
    actualNotice.includes(' · Exited · 退出码 23 · 后代进程已回收：false'),
    'The childless probe must retain its actual Exited/23 result without claiming descendant reclamation',
  );
  const actual = await capturedLogs(path.join(temporary, directories[0]));
  const lines = actual.text.replaceAll('\r\n', '\n').trimEnd().split('\n');
  assert.equal(lines.length, 20001);
  assert.equal(lines[0], `Owned failure detail 000000: ${'x'.repeat(100)}`);
  assert.equal(lines[19999], `Owned failure detail 019999: ${'x'.repeat(100)}`);
  assert.equal(lines[20000], 'Deliberate native acceptance failure');
  const tail = actual.text.slice(-12000);
  const owned = `([...document.querySelectorAll('.owned-log')].find(node => node.querySelector('pre')?.textContent === ${json(tail)}))`;
  const summary = await evaluate(`(() => {
    const node = ${owned}; if (!node) throw new Error('Actual owned failure log is missing from native UI');
    const summary = node.closest('details').querySelector('summary');
    summary.scrollIntoView({block:'nearest'}); const box=summary.getBoundingClientRect();
    return {x:box.x+box.width/2,y:box.y+box.height/2};
  })()`);
  await pointer(summary.x, summary.y);
  await until(`Boolean(${owned}?.getClientRects().length)`, 'actual owned failure details opened');
  assert.equal(await evaluate(`${owned}.querySelector('pre').textContent`), tail);
  const copied = await clipboard(actual.text, owned);
});
check('owned-cancel-and-late-reply-fence', async () => {
  const before = await projection();
  const bytes = await sourceBytes(before);
  const completedOwnedLogs = `[...document.querySelectorAll('.tool-content details')].filter(node => node.querySelector('summary')?.textContent === 'verification_owned_failure · 失败' && node.querySelector('pre')?.textContent.includes('Owned failure detail ')).length`;
  const completed = await evaluate(completedOwnedLogs);
  assert.equal(completed, 1, 'The previous genuine exit-23 result is the completion baseline');
  await menu('验证：受管取消');
  await until(
    `(async()=> (await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation?.stage?.key === 'backend.operation.owned_verification_failure')()`,
    'real delayed owned operation active',
  );
  await key('4', { alt: true });
  await until(
    `Boolean(document.querySelector('[role=region][aria-label="构建"]'))`,
    'non-destructive tools remain available during real owned execution',
  );
  await click('取消', "document.querySelector('.context-toolbar')");
  await until(
    `(async()=>!(await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation)()`,
    'actual owned process closure acknowledged',
  );
  await new Promise((resolve) => setTimeout(resolve, 5100));
  assert.equal((await capabilities()).operation, null);
  const after = await projection();
  assert.equal(after.workspaceEpoch, before.workspaceEpoch);
  assert.deepEqual(after.sources, before.sources);
  await unchanged(bytes);
  await click('操作日志', "document.querySelector('[aria-label=" + json('底部工具窗口') + "]')");
  assert.equal(
    await evaluate(completedOwnedLogs),
    completed,
    'Cancelled/late owned replies must not publish another returned owned result',
  );
});
