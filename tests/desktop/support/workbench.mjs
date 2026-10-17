// Shared workbench operations for the native desktop tests. Every action goes
// through the real WebView, native dialogs and the unchanged Tauri IPC transport.
import assert from 'node:assert/strict';
import { after, test } from 'node:test';
import { access, mkdir, readFile, writeFile, readdir, open as openFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { launch } from './driver.mjs';
import { createBuiltinInputs } from './inputs.mjs';
import { standardInspectionReady } from './readiness.mjs';

export const explicit = (kind, lexeme) => ({
  state: 'explicit',
  value: { kind, lexeme },
});
export const existing = (objectId) => ({ kind: 'existing', objectId });
export const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
export const json = JSON.stringify;
// Independent identities of the built-in standard ECU template on the mainline.
export const SLOT = 'singlecore-ecu-v1:/Application/ReferenceComposition/EchoApplicationInstance';
export const applicationPeriod = (description) =>
  description.schedule.entities.find((entity) => entity.application).periodMs;

/**
 * Start the application for one test file, select the Chinese interface used
 * by the assertions and return the workbench operations. The session closes
 * after the file's tests.
 */
// Windows replies carry verbatim `\\?\D:\…` paths; compare them in the form
// the workbench displays (ui/src/pathDisplay.ts).
export function displayPaths(value) {
  if (typeof value === 'string') return value.replace(/^\\\\\?\\(?=[A-Za-z]:\\)/, '');
  if (Array.isArray(value)) return value.map(displayPaths);
  if (value && typeof value === 'object')
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [key, displayPaths(item)]),
    );
  return value;
}

export async function openWorkbench(name) {
  const session = await launch(name);
  const { evaluate, screenshot, resize, key, pointer, media, scratch, platform } = session;
  const ipcLog = await openFile(path.join(scratch, 'ipc.jsonl'), 'w');
  let failed = false;
  let closing;
  const shutdown = () =>
    (closing ??= (async () => {
      try {
        await ipcLog.close();
      } finally {
        await session.close();
      }
    })());
  after(async () => {
    try {
      if (failed) {
        await screenshot('failure.png').catch(() => undefined);
        await writeFile(
          path.join(scratch, 'failure.txt'),
          String(await evaluate('document.body.innerText').catch((error) => error)),
        );
      }
      assert.equal(
        await evaluate('window.__TAURI_INTERNALS__.invoke === window.__BUILTIN_NATIVE_TRANSPORT__'),
        true,
        'Never replace the native IPC transport',
      );
    } finally {
      await shutdown();
    }
  });
  // Later checks in a file build on earlier state, so stop the chain at the
  // first failure instead of reporting its consequences.
  function check(title, body) {
    return test(title, async (t) => {
      if (failed) return t.skip('an earlier check in this file failed');
      try {
        await body(t);
        await screenshot(`${title.replaceAll(/[^a-z0-9-]/gi, '-')}.png`);
      } catch (error) {
        failed = true;
        throw error;
      }
    });
  }
  const visible = `node => node.getClientRects().length > 0 && !node.closest('[hidden]')`;
  async function until(expression, description, timeout = 60000) {
    const deadline = Date.now() + timeout;
    while (Date.now() < deadline) {
      if (await evaluate(expression)) return;
      await new Promise((resolve) => setTimeout(resolve, 75));
    }
    const mediaState = description.includes('reduced-motion')
      ? await evaluate(`({ reduced: matchMedia('(prefers-reduced-motion: reduce)').matches,
          controls: [...document.querySelectorAll('[aria-label="设置"]')].map(node => ({
            tag: node.tagName, duration: getComputedStyle(node).transitionDuration,
          })), styles: [...document.styleSheets].map(sheet => sheet.href) })`)
      : null;
    throw new Error(
      `Native surface timeout (${description}): ${expression}${mediaState ? `; observed=${JSON.stringify(mediaState)}` : ''}`,
    );
  }
  async function capabilities() {
    return evaluate("window.__TAURI_INTERNALS__.invoke('workbench_capabilities')");
  }
  async function invoke(command, payload = {}, fingerprint) {
    const current = fingerprint ?? (await capabilities()).fingerprint;
    const started = performance.now();
    const result = await evaluate(`(async () => {
      try { return { ok: true, reply: await window.__TAURI_INTERNALS__.invoke(${json(command)}, ${json({ ...payload, fingerprint: current })}) }; }
      catch (error) { return { ok: false, error }; }
    })()`);
    await ipcLog.writeFile(
      json({
        command,
        payload,
        fingerprint: current,
        durationMs: performance.now() - started,
        ...result,
      }) + '\n',
    );
    if (!result.ok) throw Object.assign(new Error(json(result.error)), { ipc: result.error });
    assert.equal(
      result.reply.inputFingerprint,
      current,
      `${command}: request echo must stay distinct from current capabilities`,
    );
    // Previews go back to the backend as issued; everything else is compared.
    return command.startsWith('preview_') ? result.reply.value : displayPaths(result.reply.value);
  }
  async function metrics() {
    const value = await invoke('verification_metrics');
    for (const name of [
      'sourceReads',
      'snapshotBuilds',
      'ruleLoads',
      'grammarLoads',
      'definitionReads',
      'schemaChecks',
      'sourceScans',
    ]) {
      assert.match(
        value[name],
        /^\d+$/,
        `Actual work counter must be an exact decimal string: ${name}`,
      );
    }
    return value;
  }
  async function projection() {
    const view = await invoke('project_projection');
    assert.equal(typeof view.workspaceEpoch, 'string');
    assert.equal(typeof view.definitionFingerprint, 'string');
    return view;
  }
  async function click(text, scope = 'document') {
    const buttonQuery = `(() => {
      const root = ${scope};
      if (!root) return null;
      const nodes = [...root.querySelectorAll('button')].filter(${visible});
      return nodes.find(node => {
        const label = node.cloneNode(true);
        label.querySelectorAll('small,kbd').forEach(extra => extra.remove());
        return label.textContent.trim() === ${json(text)} || node.getAttribute('aria-label') === ${json(text)};
      });
    })()`;
    // Unavailable commands stay focusable with aria-disabled; they are not ready either.
    const ready = (button) =>
      `Boolean(${button} && !${button}.disabled && ${button}.getAttribute('aria-disabled') !== 'true')`;
    await until(
      `(() => { const button = ${buttonQuery}; return ${ready('button')}; })()`,
      `actual button ready: ${text}`,
    );
    const point = await evaluate(`(() => {
      const button = ${buttonQuery};
      if (!${ready('button')}) throw new Error('Actual button missing/disabled: ' + ${json(text)});
      button.scrollIntoView({block:'nearest',inline:'nearest'});
      const box = button.getBoundingClientRect();
      return {x:box.x+box.width/2,y:box.y+box.height/2};
    })()`);
    await pointer(point.x, point.y);
  }
  async function input(target, value) {
    const description = typeof target === 'string' ? target : target.field;
    await evaluate(`(() => {
      const node = ${typeof target === 'string' ? `document.querySelector(${json(target)})` : target.expression};
      if (!node || node.disabled || node.readOnly || !node.getClientRects().length) throw new Error('Actual input not editable: ' + ${json(description)});
      node.focus();
      if (node.tagName === 'SELECT') { node.value = ${json(value)}; node.dispatchEvent(new Event('change', {bubbles: true})); }
      else { Object.getOwnPropertyDescriptor(node.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype, 'value').set.call(node, ${json(value)}); node.dispatchEvent(new Event('input', {bubbles: true})); }
    })()`);
  }
  const label = (name) => `[aria-label=${json(name)}]`;
  /** A form control named by its visible <label> text (or the label's leading words), as users find it. */
  const field = (name) => ({
    field: name,
    expression: `[...document.querySelectorAll('label')].find((node) => {
      const copy = node.cloneNode(true);
      copy.querySelectorAll('input,select,textarea,button,option').forEach((child) => child.remove());
      const text = copy.textContent.replace(/\\s+/g, ' ').trim();
      return text === ${json(name)} || text.startsWith(${json(name + ' ')});
    })?.querySelector('input,select,textarea')`,
  });
  async function menu(command) {
    // Global shortcuts leave text fields alone; leave the field as a user would.
    await evaluate(
      `document.activeElement?.closest('input, textarea, select') ? document.activeElement.blur() : null; true`,
    );
    await key('k', { ctrl: true });
    await until(
      `Boolean(document.querySelector('[aria-label="搜索当前可用命令"]'))`,
      'command search',
    );
    await input(label('搜索当前可用命令'), command);
    await click(command, "document.querySelector('[role=dialog]')");
  }
  async function choose(kind, selected, action, observeSelected) {
    await action();
    await session.choose(kind, selected);
    assert.equal(
      await observeSelected(),
      selected,
      'Actual native chooser result must match the requested path',
    );
  }
  async function refreshSurface(manifest, sources) {
    assert.equal(
      await evaluate('window.__TAURI_INTERNALS__.invoke === window.__BUILTIN_NATIVE_TRANSPORT__'),
      true,
    );
    const previousBoot = await evaluate('performance.timeOrigin');
    await evaluate('setTimeout(() => location.reload(), 0); true');
    await until(
      `performance.timeOrigin !== ${previousBoot} && Boolean(window.__TAURI_INTERNALS__ && document.querySelector('.start-page'))`,
      'public application reload',
    );
    await evaluate('window.__BUILTIN_NATIVE_TRANSPORT__ = window.__TAURI_INTERNALS__.invoke');
    await until(
      `Boolean([...document.querySelectorAll('.start-page button')].find(node => node.textContent.trim() === '打开工程…' && node.getAttribute('aria-disabled') !== 'true'))`,
      'native capabilities restored',
    );
    if (manifest) return openMember(manifest);
    await menu('导入 ARXML…');
    await input(field('ARXML 路径（每行一份）'), sources.join('\n'));
    await click('导入');
    await until(
      `Boolean(document.querySelector('.document-tabs')) && !document.querySelector('[role=dialog]')`,
      'public source reimport',
    );
    await awaitImportedSources(sources);
    return projection();
  }
  async function awaitImportedSources(sources) {
    const expected = [...sources].sort();
    const deadline = Date.now() + 60000;
    do {
      const idle = await evaluate(
        `!document.querySelector('[role=dialog]') && !document.querySelector('[role=status]')?.textContent.startsWith('正在')`,
      );
      if (idle && !(await capabilities()).operation) {
        const view = await projection();
        if (
          JSON.stringify(view.sources.map((source) => source.path).sort()) ===
          JSON.stringify(expected)
        )
          return;
      }
      await new Promise((resolve) => setTimeout(resolve, 75));
    } while (Date.now() < deadline);
    throw new Error(
      'Actual requested source set was not published with import operation completed',
    );
  }
  async function memorySample(stage) {
    const processes = await session.memory();
    assert(
      processes.some((process) => process.workingSet > 0 || process.VmRSS > 0),
      'Actual process resident memory must be observable',
    );
    const sample = { stage, processes };
    await ipcLog.writeFile(json({ memory: sample }) + '\n');
    return sample;
  }
  async function clipboard(expected, scope, action = '复制日志') {
    const text = expected.replaceAll('\r\n', '\n');
    await click(action, scope);
    const deadline = Date.now() + 60000;
    let copied;
    while (Date.now() < deadline) {
      copied = await session.clipboard().catch(() => undefined);
      if (copied === text) return { sha256: sha256(Buffer.from(text)), length: text.length };
      await new Promise((resolve) => setTimeout(resolve, 75));
    }
    assert.equal(copied, text, 'Complete text must reach the isolated native clipboard');
  }
  async function capturedLogs(directory) {
    const files = [];
    async function collect(current) {
      for (const entry of await readdir(current, { withFileTypes: true })) {
        const member = path.join(current, entry.name);
        assert(!entry.isSymbolicLink(), 'Owned verification logs cannot escape through links');
        if (entry.isDirectory()) await collect(member);
        else if (entry.name === 'stdout.log' || entry.name === 'stderr.log') files.push(member);
      }
    }
    await collect(directory);
    const stdout = files.filter((name) => path.basename(name) === 'stdout.log');
    const stderr = files.filter((name) => path.basename(name) === 'stderr.log');
    assert.equal(stdout.length, 1);
    assert.equal(stderr.length, 1);
    return {
      stdout: stdout[0],
      stderr: stderr[0],
      text: (await readFile(stdout[0], 'utf8')) + (await readFile(stderr[0], 'utf8')),
    };
  }
  async function sourceBytes(view) {
    return Promise.all(
      view.sources.map(async (source) => ({
        sourceId: source.sourceId,
        path: source.path,
        bytes: await readFile(source.path),
      })),
    );
  }
  async function unchanged(snapshot) {
    for (const source of snapshot)
      assert.deepEqual(
        await readFile(source.path),
        source.bytes,
        `Preserve original bytes: ${source.path}`,
      );
  }
  function changeSet(view, changes) {
    return {
      workspaceEpoch: view.workspaceEpoch,
      inputFingerprint: view.inputFingerprint,
      definitionFingerprint: view.definitionFingerprint,
      changes,
    };
  }
  function valueChange(field, lexeme, id = field.fieldId) {
    assert(field.writable, `Expected writable descriptor: ${field.definitionId}`);
    return {
      op: 'set-value',
      changeId: id,
      field: { kind: 'existing', fieldId: field.fieldId },
      expected: field.current,
      value: explicit(field.kind, lexeme),
    };
  }
  async function apply(view, changes) {
    const batch = changeSet(view, changes);
    const bytes = await sourceBytes(view);
    const preview = await invoke('prepare_configuration_change', {
      changeSet: batch,
    });
    await unchanged(bytes);
    assert.deepEqual(await projection(), view, 'prepare must not publish even a partial graph');
    const outcome = await invoke('apply_configuration_change', {
      changeSet: batch,
      changeRevision: preview.changeRevision,
    });
    await unchanged(bytes);
    return { preview, outcome, batch };
  }
  async function refuse(command, payload, name, fingerprint) {
    const before = await projection();
    const bytes = await sourceBytes(before);
    await assert.rejects(invoke(command, payload, fingerprint), undefined, name);
    assert.deepEqual(
      await projection(),
      before,
      `${name}: rejection must retain all sources, IDs and current revision`,
    );
    await unchanged(bytes);
  }
  async function save() {
    const before = await projection();
    const bytes = await sourceBytes(before);
    const preview = await invoke('preview_save_project');
    await unchanged(bytes);
    const outcome = await invoke('save_project', {
      revision: preview.revision,
    });
    for (const file of preview.files) {
      if (file.after !== null)
        assert.deepEqual(
          await readFile(file.path),
          Buffer.from(file.after),
          `Saved bytes match actual preview: ${file.path}`,
        );
    }
    assert.equal((await projection()).dirty, false);
    return { preview, outcome };
  }
  function fieldBy(view, suffix) {
    const found = view.fields.find((field) => field.definitionId.endsWith(suffix));
    assert(found, `Actual backend descriptor missing: ${suffix}`);
    return found;
  }
  async function create(name) {
    await menu('新建工程…');
    await until(`Boolean(${field('工程名称').expression})`, 'new project entry');
    await input(field('工程名称'), name);
    const directory = path.join(scratch, 'projects', name);
    await mkdir(directory);
    await choose(
      'directory',
      directory,
      () => click('选择…', "document.querySelector('.start-page')"),
      async () => {
        await until(
          `document.querySelector(${json(label('新工程目录'))})?.value === ${json(directory)}`,
          'native directory result',
        );
        return evaluate(`document.querySelector(${json(label('新工程目录'))}).value`);
      },
    );
    await click('预览工程文件');
    await until(
      `Boolean(document.querySelector('[aria-label="确认新建工程"]'))`,
      'original template preview',
    );
    assert.deepEqual(await readdir(directory), [], 'Template preview must not create source files');
    await click('确认创建');
    await until(
      `Boolean(document.querySelector('.document-tabs')) && !document.querySelector('[role=dialog]')`,
      'created workbench',
    );
    const manifest = path.join(directory, 'workbench-project.json');
    const saved = JSON.parse(await readFile(manifest, 'utf8'));
    assert.deepEqual(saved.acceptedExtensionDefinitions, []);
    assert.deepEqual(saved.applicationInputs, []);
    // The new project starts its automatic standard inspection; later
    // operations would otherwise make that inspection stale.
    await awaitStandardInspectionReady();
    const view = await projection();
    assert.equal(view.projectPath, manifest);
    return { directory, manifest, view };
  }
  async function awaitProjectSurfaceReady(timeout) {
    await until(
      `!document.querySelector('[role=status]')?.textContent.startsWith('正在') && (!document.querySelector('[aria-label="标准输入工作区"]') || !document.querySelector('[aria-label="标准输入工作区"] .integration-fields label:first-child input') || Boolean(document.querySelector('[aria-label="标准输入工作区"] .integration-fields label:first-child input:not(:disabled)')))`,
      'actual opened project and automatic inspection operation completed',
      timeout,
    );
    await until(
      `(async () => !(await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation)()`,
      'actual project automatic inspection backend operation completed',
      timeout,
    );
  }
  async function awaitStandardInspectionReady(timeout) {
    await until(
      `(${standardInspectionReady.toString()})(document)`,
      'automatic standard inspection result published and editor ready',
      timeout,
    );
    await awaitProjectSurfaceReady(timeout);
  }
  async function openMember(manifest) {
    let view;
    await choose(
      'file',
      manifest,
      () => menu('打开工程…'),
      async () => {
        await until(
          `!document.querySelector('[role=dialog]') && Boolean(document.querySelector('.document-tabs'))`,
          'member project opened',
        );
        const deadline = Date.now() + 60000;
        do {
          try {
            view = await invoke('project_projection');
          } catch (error) {
            // Opening the selected member may publish between this read's
            // capability snapshot and request. Retry only this read-only poll.
            if (error.ipc?.key !== 'backend.workbench.stale_delivery') throw error;
          }
          if (view?.projectPath === manifest) break;
          await new Promise((resolve) => setTimeout(resolve, 75));
        } while (Date.now() < deadline);
        await awaitProjectSurfaceReady();
        view = await projection();
        return view.projectPath;
      },
    );
    assert.equal(view.projectPath, manifest);
    return view;
  }
  async function selectObject(objectId, multi = false) {
    await click('打开配置对象文档');
    await input(label('搜索对象表名称、路径、定义、值'), '');
    await evaluate(`(() => {
      const row = [...document.querySelectorAll('[data-object-row]')].find(node => node.dataset.objectRow === ${json(objectId)});
      if (!row) throw new Error('Real object row missing');
      const node = row.querySelector(${json(multi ? 'input[type=checkbox]' : 'button.table-link')});
      node.focus(); node.click();
    })()`);
    await until(
      `document.querySelector('[data-object-title]')?.dataset.objectTitle === ${json(objectId)} || Boolean(document.querySelector('[role=dialog]'))`,
      'object focus',
    );
  }
  async function sourceSurface(view) {
    await click('文件', "document.querySelector('[aria-label=" + json('工程树视图') + "]')");
    const source = view.sources[0];
    await click(
      path.basename(source.path),
      "document.querySelector('[aria-label=" + json('工程树') + "]')",
    );
    await until(
      `Boolean(document.querySelector('[aria-label="完整源原文"]'))`,
      'real source document',
    );
    const actual = await invoke('read_project_source', {
      sourceId: source.sourceId,
    });
    assert.deepEqual(Buffer.from(actual), await readFile(source.path));
    const rendered = await evaluate(
      `document.querySelector('[aria-label="完整源原文"]').textContent`,
    );
    assert.equal(rendered, actual.slice(0, 40000));
    if (view.sources.length > 1) {
      const otherDocuments = await evaluate(
        `[...document.querySelectorAll('.document-tabs button[aria-label^="关闭 "]')].map(node => node.getAttribute('aria-label')).filter(name => name !== ${json(`关闭 ${path.basename(source.path)}`)})`,
      );
      for (const name of otherDocuments)
        await click(name, "document.querySelector('.document-tabs')");
      await until(
        `document.querySelectorAll('.document-tabs [role=tab]').length === 1 && document.querySelector('.document-tabs [role=tab][aria-selected=true]')?.textContent === ${json(path.basename(source.path))}`,
        'source A is the sole existing fallback document before opening B',
      );
      const second = view.sources[1];
      const secondText = await invoke('read_project_source', {
        sourceId: second.sourceId,
      });
      assert.notEqual(secondText, actual, 'Raw fallback requires two genuinely different sources');
      await click(
        path.basename(second.path),
        "document.querySelector('[aria-label=" + json('工程树') + "]')",
      );
      await until(
        `document.querySelector('[aria-label="完整源原文"]')?.textContent === ${json(secondText.slice(0, 40000))}`,
        'second raw source content',
      );
      await click(`关闭 ${path.basename(second.path)}`, "document.querySelector('.document-tabs')");
      await until(
        `document.querySelector('.document-tabs [role=tab][aria-selected=true]')?.textContent === ${json(path.basename(source.path))} && document.querySelector('[aria-label="完整源原文"]')?.textContent === ${json(actual.slice(0, 40000))}`,
        'closed active source restores actual fallback bytes and tab header',
      );
      await clipboard(actual, "document.querySelector('.text-snapshot')", '复制');
      await screenshot('source-fallback-raw-A.png');
    }
    await click(
      path.basename(source.path),
      "document.querySelector('[aria-label=" + json('工程树') + "]')",
    );
    assert.equal(
      await evaluate(
        `[...document.querySelectorAll('.document-tabs [role=tab]')].filter(node => node.textContent === ${json(path.basename(source.path))}).length`,
      ),
      1,
      'Source tab must be reused',
    );
    await click('对象', "document.querySelector('[aria-label=" + json('工程树视图') + "]')");
    await click('打开配置对象文档');
  }
  async function exportSource(name) {
    const output = path.join(scratch, 'deliveries', name);
    const payload = { outputDirectory: output, handoff: true };
    const before = await projection();
    const preview = await invoke('preview_ecu_project', payload);
    await assert.rejects(access(output), { code: 'ENOENT' });
    assert.deepEqual(await projection(), before);
    await invoke('generate_ecu_project', {
      ...payload,
      revision: preview.revision,
    });
    for (const file of preview.files) {
      if (file.after !== null)
        assert.deepEqual(
          await readFile(path.join(output, file.path)),
          Buffer.from(file.after),
          `Installed exact prepared source: ${file.path}`,
        );
    }
    const handoff = JSON.parse(await readFile(path.join(output, 'meta/handoff.json'), 'utf8'));
    assert.equal(handoff.format, 'autosar-workbench-handoff-v4');
    assert.deepEqual(handoff.resourceIdentities.ruleSetIdentity, before.ruleSetIdentity);
    assert.deepEqual(handoff.resourceIdentities.requiredExtensionDefinitions, []);
    const ledger = JSON.parse(
      await readFile(path.join(output, 'meta/workbench-ownership.json'), 'utf8'),
    );
    assert.equal(ledger.formatVersion, 3);
    const sealed = (await readFile(path.join(output, 'meta/files.list'), 'utf8'))
      .trim()
      .split(/\r?\n/);
    for (const forbidden of [
      'AUTOSAR_00053.xsd',
      'catalog.json',
      '.exe',
      'node_modules',
      'target/debug',
    ]) {
      assert(
        !sealed.some((file) => file.includes(forbidden)),
        `Configuration delivery contains development dependency: ${forbidden}`,
      );
    }
    const target = JSON.parse(await readFile(path.join(output, 'target.json'), 'utf8'));
    const packageEvidence = {
      name,
      directory: output,
      profile: target.profile,
      target: target.target,
      source: 'passed',
      build: 'not_run',
      behavior: 'not_run',
      identity: before.ruleSetIdentity,
      inputs: await invoke('workspace_view'),
    };
    return packageEvidence;
  }
  async function packageBytes(directory) {
    const names = (await readFile(path.join(directory, 'meta/files.list'), 'utf8'))
      .trim()
      .split(/\r?\n/);
    return Promise.all(
      [...names, 'meta/files.list', 'meta/files.sha256'].map(async (name) => ({
        path: path.join(directory, name),
        bytes: await readFile(path.join(directory, name)),
      })),
    );
  }
  async function resealPrivateVariant(directory) {
    const ledgerPath = path.join(directory, 'meta/workbench-ownership.json');
    const ledger = JSON.parse(await readFile(ledgerPath, 'utf8'));
    for (const entry of ledger.files)
      entry.sha256 = sha256(await readFile(path.join(directory, entry.path)));
    await writeFile(ledgerPath, json(ledger, null, 2) + '\n');
    const names = (await readFile(path.join(directory, 'meta/files.list'), 'utf8'))
      .trim()
      .split(/\r?\n/);
    const hashes = await Promise.all(
      [...names, 'meta/files.list'].map(
        async (name) => `${sha256(await readFile(path.join(directory, name)))}  ${name}`,
      ),
    );
    await writeFile(path.join(directory, 'meta/files.sha256'), hashes.join('\n') + '\n');
  }
  // Hand-authored multi-module ARXML imported through the public import dialog.
  async function importAuthoredInputs() {
    const authored = await createBuiltinInputs(scratch);
    await menu('导入 ARXML…');
    await input(field('ARXML 路径（每行一份）'), authored.sources.join('\n'));
    await click('导入');
    await until(
      `Boolean(document.querySelector('.document-tabs')) && !document.querySelector('[role=dialog]')`,
      'authored multi-module import',
    );
    await awaitImportedSources(authored.sources);
    return authored;
  }
  let initialCapabilities;
  try {
    await until(
      `Boolean(window.__TAURI_INTERNALS__ && document.querySelector('button'))`,
      'native startup',
    );
    initialCapabilities = await capabilities();
    // Keep the Chinese assertions deterministic on every host locale.
    const startupBoot = await evaluate('performance.timeOrigin');
    await invoke('configure_language', { language: 'zh-CN' });
    await evaluate('setTimeout(() => location.reload(), 0); true');
    await until(
      `performance.timeOrigin !== ${startupBoot} && Boolean(window.__TAURI_INTERNALS__ && document.querySelector('[aria-label="设置"]'))`,
      'explicit Chinese native startup',
    );
    await evaluate(`window.__BUILTIN_NATIVE_TRANSPORT__ = window.__TAURI_INTERNALS__.invoke`);
    await resize(1480, 920);
  } catch (error) {
    await shutdown();
    throw error;
  }
  return {
    check,
    initialCapabilities,
    evaluate,
    screenshot,
    resize,
    key,
    pointer,
    media,
    scratch,
    platform,
    visible,
    until,
    capabilities,
    invoke,
    metrics,
    projection,
    click,
    input,
    label,
    field,
    menu,
    choose,
    refreshSurface,
    awaitImportedSources,
    memorySample,
    clipboard,
    capturedLogs,
    sourceBytes,
    unchanged,
    changeSet,
    valueChange,
    apply,
    refuse,
    save,
    fieldBy,
    create,
    awaitProjectSurfaceReady,
    awaitStandardInspectionReady,
    openMember,
    selectObject,
    sourceSurface,
    exportSource,
    packageBytes,
    resealPrivateVariant,
    importAuthoredInputs,
  };
}
