import assert from 'node:assert/strict';
import path from 'node:path';
import { dialogNamed, openWorkbench, displayPaths, explicit, json } from './support/workbench.mjs';

const {
  check,
  evaluate,
  screenshot,
  resize,
  key,
  pointer,
  media,
  scratch,
  visible,
  until,
  capabilities,
  invoke,
  projection,
  click,
  input,
  label,
  refreshSurface,
  clipboard,
  apply,
  save,
  fieldBy,
  selectObject,
  configurationView,
  importAuthoredInputs,
} = await openWorkbench('workbench-ui');

let authored;
check('themes-settings-layout-and-keyboard', async () => {
  authored = await importAuthoredInputs();
  const view = await projection();
  const field = fieldBy(view, '/OsEventMask');
  await selectObject(field.objectId);
  await input(`[data-field-id=${json(field.fieldId)}]`, '18446744073709551614');
  const draftSelector = `[data-field-id=${json(field.fieldId)}]`;
  for (const [width, height] of [
    [1480, 920],
    [1080, 720],
  ]) {
    await resize(width, height);
    await click('设置');
    await until(`Boolean(${dialogNamed('设置')})`, 'settings dialog');
    for (const theme of ['light', 'dark', 'system']) {
      await click('外观', "document.querySelector('[role=dialog]')");
      await input(label('主题'), theme);
      await click('保存外观', "document.querySelector('[role=dialog]')");
      await until(
        `(async()=> (await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).appearance === ${json(theme)})()`,
        'persisted theme',
      );
      assert.equal(
        (await projection()).workspaceEpoch,
        view.workspaceEpoch,
        'Appearance cannot rebuild the workspace',
      );
      assert.equal(
        await evaluate(`getComputedStyle(document.querySelector('.tree-content')).scrollbarWidth`),
        'none',
        'Native background scrollbars are suppressed while a portalled modal owns interaction',
      );
      assert.notEqual(
        await evaluate(`getComputedStyle(document.querySelector('.dialog-body')).scrollbarWidth`),
        'none',
        'The active dialog keeps its own scrolling',
      );
      await screenshot(`layout-${width}x${height}-${theme}.png`);
    }
    for (const scheme of ['dark', 'light']) {
      const reduceMotion = scheme === 'dark';
      await media(scheme, reduceMotion);
      await until(
        `window.matchMedia('(prefers-color-scheme: dark)').matches === ${scheme === 'dark'} && window.matchMedia('(prefers-reduced-motion: reduce)').matches === ${reduceMotion}`,
        'actual native system media change',
      );
      await until(
        `document.documentElement.dataset.theme === ${json(scheme)}`,
        'system theme event updates actual workbench',
      );
      if (reduceMotion) {
        await until(
          `getComputedStyle(document.querySelector('[aria-label="设置"]')).transitionDuration.split(',').every(value => parseFloat(value) === 0)`,
          'reduced-motion styles update after the native preference event',
        );
        const transitions = await evaluate(
          `getComputedStyle(document.querySelector('[aria-label="设置"]')).transitionDuration.split(',').map(value=>parseFloat(value))`,
        );
        assert(
          transitions.every((duration) => duration === 0),
          'Reduced motion must disable actual core-action transitions',
        );
      }
      assert.equal(
        (await capabilities()).appearance,
        'system',
        'System events cannot rewrite the saved appearance preference',
      );
      assert.deepEqual(
        (await projection()).sources,
        view.sources,
        'System media events cannot mutate source identities',
      );
      assert.equal(
        await evaluate(`document.querySelector(${json(draftSelector)}).value`),
        '18446744073709551614',
      );
      await screenshot(`layout-${width}x${height}-system-event-${scheme}.png`);
    }
    await click('执行工具', "document.querySelector('[role=dialog]')");
    const caps = await capabilities();
    await input(label('编译器'), path.join(scratch, 'must-not-be-saved'));
    await click('外观', "document.querySelector('[role=dialog]')");
    await input(label('主题'), 'light');
    await click('保存外观', "document.querySelector('[role=dialog]')");
    await until(
      `(async()=> (await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).appearance === 'light')()`,
      'category save',
    );
    assert.deepEqual(
      (await capabilities()).executionTools,
      caps.executionTools,
      'Saving appearance must not save another category draft',
    );
    await key('Tab', { shift: true });
    assert(
      await evaluate(`document.querySelector('[role=dialog]').contains(document.activeElement)`),
      'Dialog must trap keyboard focus',
    );
    await key('Escape');
    await until(`!document.querySelector('[role=dialog]')`, 'Escape closes settings');
    assert.equal(
      await evaluate(`getComputedStyle(document.querySelector('.tree-content')).scrollbarWidth`),
      'thin',
      'Background scrollbars return when the modal closes',
    );
    assert.equal(
      await evaluate(`document.querySelector(${json(draftSelector)}).value`),
      '18446744073709551614',
    );
    assert.deepEqual((await capabilities()).executionTools, caps.executionTools);
    const bounds = await evaluate(
      `({viewport:[innerWidth,innerHeight],body:[document.body.scrollWidth,document.body.scrollHeight],toolbar:[...document.querySelectorAll('.context-toolbar button')].filter(${visible}).map(node=>({name:node.getAttribute('aria-label')??node.textContent.trim(),box:node.getBoundingClientRect().toJSON()}))})`,
    );
    assert(
      bounds.toolbar.every(
        (button) =>
          button.box.x >= 0 &&
          button.box.right <= bounds.viewport[0] &&
          button.box.y >= 0 &&
          button.box.bottom <= bounds.viewport[1],
      ),
      'Core actions must remain in usable viewport',
    );
  }
  await click('问题', "document.querySelector('.tool-tabs')");
  await until(`Boolean(document.querySelector('.validation-scopes'))`, 'automatic checks strip');
  const scopeGeometry = await evaluate(`(() => {
    const strip = document.querySelector('.validation-scopes');
    return [...strip.querySelectorAll(':scope > p, summary')].map(node => {
      const text = [...node.childNodes].find(child => child.nodeType === Node.TEXT_NODE && child.textContent.trim());
      const range = document.createRange(); range.selectNode(text);
      const box = node.getBoundingClientRect(), glyph = range.getBoundingClientRect();
      return {height: box.height, textOffset: glyph.y - box.y, lineHeight: getComputedStyle(node).lineHeight};
    });
  })()`);
  assert.equal(scopeGeometry.length, 5, 'Caption and all four automatic-check stages are visible');
  for (const item of scopeGeometry.slice(1)) {
    assert(Math.abs(item.height - scopeGeometry[0].height) < 1, 'Stage and caption heights match');
    assert(
      Math.abs(item.textOffset - scopeGeometry[0].textOffset) < 1,
      'Stage and caption text align',
    );
    assert.equal(item.lineHeight, scopeGeometry[0].lineHeight);
  }
  await screenshot('automatic-checks-aligned.png');
  await key('4', { alt: true });
  await until(
    `Boolean(document.querySelector('[role=region][aria-label="构建"]'))`,
    'Alt+4 opens build',
  );
  await key('9', { alt: true });
  await until(
    `Boolean(document.querySelector('[role=region][aria-label="主机验证"]'))`,
    'Alt+9 opens host',
  );
  await configurationView('全部对象');
  await input(label('搜索对象表名称、路径、定义、值'), 'does-not-match');
  await until(
    `document.querySelector('.filter-retained') && document.querySelector('.batch-bar').textContent.includes('1 项未应用的更改')`,
    'filtered-out current object keeps its draft',
  );
  await input(label('搜索对象表名称、路径、定义、值'), '');
  await configurationView('容器');
  assert.equal(
    await evaluate(`document.querySelector(${json(draftSelector)}).value`),
    '18446744073709551614',
    'Filtering must not discard hidden object draft',
  );
  // A container the tree shows, so navigation stays in the container view.
  const second = view.objects.find(
    (object) =>
      object.objectId !== field.objectId &&
      object.writable &&
      object.kind === 'ECUC-CONTAINER-VALUE',
  );
  await selectObject(second.objectId);
  // Draft guards are alert dialogs (Radix AlertDialog).
  await until(`Boolean(document.querySelector('[role=alertdialog]'))`, 'context draft guard');
  await click('取消', "document.querySelector('[role=alertdialog]')");
  assert.equal(
    await evaluate(`document.querySelector(${json(draftSelector)}).value`),
    '18446744073709551614',
  );
  await input(draftSelector, '18446744073709551616');
  await selectObject(second.objectId);
  await click('应用并继续', "document.querySelector('[role=alertdialog]')");
  await until(`Boolean(document.querySelector('[role=alert]'))`, 'invalid apply rejected');
  assert.equal(
    await evaluate(`document.querySelector('[data-object-title]').dataset.objectTitle`),
    field.objectId,
    'Failed apply must not navigate',
  );
  assert.equal(
    await evaluate(`document.querySelector(${json(draftSelector)}).value`),
    '18446744073709551616',
    'Failed apply must retain draft',
  );
  await selectObject(second.objectId);
  await until(
    `Boolean(document.querySelector('[role=alertdialog]'))`,
    'retained invalid draft guard',
  );
  await click('放弃并继续', "document.querySelector('[role=alertdialog]')");
  await until(
    `document.querySelector('[data-object-title]')?.dataset.objectTitle === ${json(second.objectId)}`,
    'explicit draft discard',
  );
});
check('language-preview-save-preserves-project-and-draft', async () => {
  const before = await projection();
  const field = fieldBy(before, '/OsEventMask');
  await selectObject(field.objectId);
  const draftSelector = `[data-field-id=${json(field.fieldId)}]`;
  const draftValue = '18446744073709551614';
  await input(draftSelector, draftValue);
  const caps = await capabilities();
  await click('设置');
  await click('界面语言', "document.querySelector('[role=dialog]')");
  await input(label('界面语言'), 'en');
  await until(`document.documentElement.lang === 'en'`, 'English language preview');
  assert.equal((await capabilities()).language, 'zh-CN', 'Preview must not save');
  assert.equal((await capabilities()).fingerprint, caps.fingerprint);
  await screenshot('language-english-preview.png');
  await key('Escape');
  await until(`document.documentElement.lang === 'zh-CN'`, 'Canceled language preview');
  assert.equal(await evaluate(`document.querySelector(${json(draftSelector)}).value`), draftValue);
  await click('设置');
  await click('界面语言', "document.querySelector('[role=dialog]')");
  await input(label('界面语言'), 'en');
  await until(`document.documentElement.lang === 'en'`, 'English settings');
  await click('Save language', "document.querySelector('[role=dialog]')");
  await until(
    `(async()=> (await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).language === 'en')()`,
    'Saved English preference',
  );
  await until(
    `Boolean([...document.querySelectorAll('[role=dialog] button')].find(button => button.textContent.trim() === 'Save language' && !button.disabled && button.getAttribute('aria-busy') === 'false'))`,
    'English language save completed in the actual settings consumer',
  );
  await key('Escape');
  await until(`!document.querySelector('[role=dialog]')`, 'English settings close');
  assert.equal((await capabilities()).fingerprint, caps.fingerprint);
  const after = await projection();
  assert.deepEqual(after.sources, before.sources);
  assert.deepEqual(after.diagnostics, before.diagnostics);
  assert.equal(await evaluate(`document.querySelector(${json(draftSelector)}).value`), draftValue);
  await screenshot('language-english-saved.png');
  await click('Settings');
  await click('Interface language', "document.querySelector('[role=dialog]')");
  await input(label('Interface language'), 'zh-CN');
  await until(`document.documentElement.lang === 'zh-CN'`, 'Chinese settings');
  await click('保存语言', "document.querySelector('[role=dialog]')");
  await until(
    `(async()=> (await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).language === 'zh-CN')()`,
    'Saved Chinese preference',
  );
  await until(
    `Boolean([...document.querySelectorAll('[role=dialog] button')].find(button => button.textContent.trim() === '保存语言' && !button.disabled && button.getAttribute('aria-busy') === 'false'))`,
    'Chinese language save completed in the actual settings consumer',
  );
  await key('Escape');
  await until(`!document.querySelector('[role=dialog]')`, 'Chinese settings close');
  assert.equal((await capabilities()).fingerprint, caps.fingerprint);
  assert.equal(await evaluate(`document.querySelector(${json(draftSelector)}).value`), draftValue);
  // Restore the clean editor needed by the following independent scenarios.
  await refreshSurface(null, authored.sources);
});
check('problem-focus', async () => {
  const view = await projection();
  const issueIndex = view.diagnostics.findIndex((issue) => issue.objectId && issue.fieldId);
  const issue = view.diagnostics[issueIndex];
  assert(issue, 'Authored missing required fields must produce a genuine field diagnostic');
  await click('问题', "document.querySelector('[aria-label=" + json('底部工具窗口') + "]')");
  const fingerprint = (await capabilities()).fingerprint;
  const details = await evaluate(`(async () => {
    const reply = await window.__TAURI_INTERNALS__.invoke('project_projection', {fingerprint:${json(fingerprint)}});
    if (reply.inputFingerprint !== ${json(fingerprint)}) throw new Error('Diagnostic projection request echo changed');
    return JSON.stringify(reply.value.diagnostics[${issueIndex}], null, 2);
  })()`);
  assert.deepEqual(displayPaths(JSON.parse(details)), issue);
  // The table sorts by severity and module; the row is found by code, object and parameter.
  const parameter = view.fields
    .find((field) => field.fieldId === issue.fieldId)
    .definitionId.split('/')
    .pop();
  const row = `[...document.querySelectorAll('.problems-table tbody tr')].filter(${visible}).find(row => row.dataset.problemCode === ${json(issue.code)} && row.textContent.includes(${json(issue.path ?? issue.file)}) && row.textContent.includes(${json(` · ${parameter}`)}))`;
  const copiedDiagnostic = await clipboard(details, row, '复制详情');
  const location = await evaluate(`(() => {
    const row = ${row};
    if (!row) throw new Error('Actual projected diagnostic row identity changed');
    const button = [...row.querySelectorAll('button')].find(button => button.textContent.trim() === '定位');
    if (!button) throw new Error('Genuine issue locator missing');
    button.scrollIntoView({block:'nearest'}); const box=button.getBoundingClientRect();
    return {x:box.x+box.width/2,y:box.y+box.height/2};
  })()`);
  await pointer(location.x, location.y);
  await until(
    `document.activeElement?.dataset.fieldId === ${json(issue.fieldId)} || document.activeElement?.dataset.objectTitle === ${json(issue.objectId)}`,
    'real issue focus handoff',
  );
  assert(
    await evaluate(
      `document.querySelector('[data-object-title]').dataset.objectTitle === ${json(issue.objectId)}`,
    ),
  );
});
