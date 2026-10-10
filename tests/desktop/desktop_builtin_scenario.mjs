import { runCase } from './test-case.mjs';
import assert from 'node:assert/strict';
import {
  access,
  mkdir,
  readFile,
  writeFile,
  rename,
  cp,
  readdir,
  readlink,
  unlink,
  open,
} from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import {
  createBuiltinInputs,
  createExtensionInputs,
  createMultiComponentInputs,
} from './desktop_builtin_inputs.mjs';
import { assertUnavailableSourceCheckout } from './desktop_scenario.mjs';

const explicit = (kind, lexeme) => ({
  state: 'explicit',
  value: { kind, lexeme },
});
const existing = (objectId) => ({ kind: 'existing', objectId });
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

export async function runBuiltinScenario({
  evaluate,
  screenshot,
  resize,
  key,
  pointer,
  media,
  scratch,
  platform,
}) {
  const scenario = JSON.parse(await readFile(path.join(scratch, 'scenario.json'), 'utf8'));
  const evidence = {
    format: 'autosar-builtin-native-acceptance-v1',
    platform,
    mode: 'builtin-only',
    installed: scenario.installed,
    status: 'failed',
    checks: [],
    packages: [],
    independentConsumer: 'not_run',
    networkIsolation: 'not_run',
    macos: 'not_run',
    snapshotReuse: {
      status: 'not_run',
      reason:
        'Backend read/rule-load/decompression counters are required; not inferred from DOM timing',
    },
    memory: { status: 'not_run', samples: [] },
    error: null,
  };
  const ipcLog = await open(path.join(scratch, 'native-builtin-ipc.jsonl'), 'w');
  let sequence = 0;
  const json = JSON.stringify;
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
    if (!result.ok) throw new Error(json(result.error));
    assert.equal(
      result.reply.inputFingerprint,
      current,
      `${command}: request echo must stay distinct from current capabilities`,
    );
    return result.reply.value;
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
  async function checked(name, body) {
    const result = { name, status: 'running' };
    evidence.checks.push(result);
    try {
      result.detail = await runCase(name, body);
      result.status = 'passed';
      await screenshot(
        `${String(evidence.checks.length).padStart(2, '0')}-${name.replaceAll(/[^a-z0-9-]/gi, '-')}.png`,
      );
    } catch (error) {
      result.status = 'failed';
      result.error = String(error);
      throw error;
    }
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
    await until(
      `(() => { const button = ${buttonQuery}; return Boolean(button && !button.disabled); })()`,
      `actual button ready: ${text}`,
    );
    const point = await evaluate(`(() => {
      const button = ${buttonQuery};
      if (!button || button.disabled) throw new Error('Actual button missing/disabled: ' + ${json(text)});
      button.scrollIntoView({block:'nearest',inline:'nearest'});
      const box = button.getBoundingClientRect();
      return {x:box.x+box.width/2,y:box.y+box.height/2};
    })()`);
    await pointer(point.x, point.y);
  }
  async function input(selector, value) {
    await evaluate(`(() => {
      const node = document.querySelector(${json(selector)});
      if (!node || node.disabled || node.readOnly || !node.getClientRects().length) throw new Error('Actual input not editable: ' + ${json(selector)});
      node.focus();
      if (node.tagName === 'SELECT') { node.value = ${json(value)}; node.dispatchEvent(new Event('change', {bubbles: true})); }
      else { Object.getOwnPropertyDescriptor(node.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype, 'value').set.call(node, ${json(value)}); node.dispatchEvent(new Event('input', {bubbles: true})); }
    })()`);
  }
  const label = (name) => `[aria-label=${json(name)}]`;
  async function menu(command) {
    await key('k', { ctrl: true });
    await until(
      `Boolean(document.querySelector('[aria-label="搜索当前可用命令"]'))`,
      'command search',
    );
    await input(label('搜索当前可用命令'), command);
    await click(command, "document.querySelector('[role=dialog]')");
  }
  async function choose(kind, selected, action, observeSelected) {
    const request = { kind, path: selected, requestId: `path-${++sequence}` };
    const stagedRequest = path.join(scratch, 'native-dialog-request.next.json');
    await writeFile(stagedRequest, json(request));
    await rename(stagedRequest, path.join(scratch, 'native-dialog-request.json'));
    await action();
    const selectedPath = await observeSelected();
    assert.equal(
      selectedPath,
      selected,
      'Actual native chooser result must match its authorization',
    );
    const result = path.join(scratch, 'native-dialog-result.next.json');
    await writeFile(result, json({ ...request, selectedPath }));
    await rename(result, path.join(scratch, 'native-dialog-result.json'));
    const deadline = Date.now() + 60000;
    while (Date.now() < deadline) {
      try {
        const consumed = JSON.parse(
          await readFile(path.join(scratch, 'native-dialog-consumed.json'), 'utf8'),
        );
        if (consumed.requestId === request.requestId) return;
      } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    throw new Error(`Owned native chooser did not select ${selected}`);
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
      `Boolean([...document.querySelectorAll('.start-page button')].find(node => node.textContent.trim() === '打开成员工程…' && !node.disabled))`,
      'native capabilities restored',
    );
    if (manifest) return openMember(manifest);
    await menu('导入 ARXML…');
    await input(label('ARXML 来源路径'), sources.join('\n'));
    await click('导入并进入工程');
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
    const request = { stage, requestId: `memory-${++sequence}` };
    const staged = path.join(scratch, 'native-measure-request.next.json');
    await writeFile(staged, json(request));
    await rename(staged, path.join(scratch, 'native-measure-request.json'));
    const deadline = Date.now() + 10000;
    while (Date.now() < deadline) {
      try {
        const result = JSON.parse(
          await readFile(path.join(scratch, 'native-measure-result.json'), 'utf8'),
        );
        if (result.requestId === request.requestId) {
          assert(
            result.processes.length > 0 &&
              result.processes.some((process) => process.workingSet > 0 || process.VmRSS > 0),
            'Actual process resident memory must be observable',
          );
          evidence.memory.samples.push(result);
          return result;
        }
      } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
      await new Promise((resolve) => setTimeout(resolve, 75));
    }
    throw new Error(`Launcher did not measure owned native memory: ${stage}`);
  }
  async function clipboard(expected, scope, action = '复制完整日志') {
    const text = expected.replaceAll('\r\n', '\n');
    const request = {
      requestId: `clipboard-${++sequence}`,
      expectedSha256: sha256(Buffer.from(text)),
    };
    const staged = path.join(scratch, 'native-clipboard-request.next.json');
    await writeFile(staged, json(request));
    await rename(staged, path.join(scratch, 'native-clipboard-request.json'));
    await click(action, scope);
    const deadline = Date.now() + 60000;
    while (Date.now() < deadline) {
      try {
        const receipt = JSON.parse(
          await readFile(path.join(scratch, 'native-clipboard-result.json'), 'utf8'),
        );
        if (receipt.requestId === request.requestId) {
          assert.equal(
            receipt.scope,
            platform === 'windows' ? 'private-window-station' : 'private-xvfb-display',
          );
          assert.equal(
            await readFile(receipt.contents, 'utf8'),
            text,
            'Complete real owned log must reach the isolated native clipboard',
          );
          return receipt;
        }
      } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
      await new Promise((resolve) => setTimeout(resolve, 75));
    }
    throw new Error('Full owned log was not copied to the isolated native clipboard');
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
  async function create(name, templateId) {
    await menu('新建工程…');
    await until(`Boolean(document.querySelector(${json(label('工程名称'))}))`, 'new project entry');
    await input(label('工程名称'), name);
    await input(label('工程模板'), templateId);
    const directory = path.join(scratch, 'projects', name);
    await mkdir(directory);
    await choose(
      'directory',
      directory,
      () => click('选择', "document.querySelector('.start-page')"),
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
    await click('确认创建工程');
    await until(
      `Boolean(document.querySelector('.document-tabs')) && !document.querySelector('[role=dialog]')`,
      'created workbench',
    );
    const manifest = path.join(directory, 'workbench-project.json');
    const saved = JSON.parse(await readFile(manifest, 'utf8'));
    assert.deepEqual(saved.acceptedExtensionDefinitions, []);
    assert.deepEqual(saved.applicationInputs, []);
    const view = await projection();
    assert.equal(view.projectPath, manifest);
    return { directory, manifest, view };
  }
  async function awaitProjectSurfaceReady() {
    await until(
      `!document.querySelector('[role=status]')?.textContent.startsWith('正在') && (!document.querySelector('[aria-label="标准输入工作区"]') || !document.querySelector('[aria-label="标准输入工作区"] input[aria-label="接收 CAN ID"]') || Boolean(document.querySelector('[aria-label="标准输入工作区"] input[aria-label="接收 CAN ID"]:not(:disabled)')))`,
      'actual opened project and automatic inspection operation completed',
    );
    await until(
      `(async () => !(await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation)()`,
      'actual project automatic inspection backend operation completed',
    );
  }
  async function openMember(manifest) {
    let view;
    await choose(
      'file',
      manifest,
      () => menu('打开成员工程…'),
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
            if (!String(error).includes('STALE_DELIVERY')) throw error;
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
      await clipboard(actual, 'document', '复制完整原文');
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
  async function exportSource(name, standard) {
    const output = path.join(scratch, 'deliveries', name);
    const previewCommand = standard ? 'preview_ecu_project' : 'preview_handoff_project';
    const generateCommand = standard ? 'generate_ecu_project' : 'generate_handoff_project';
    const payload = {
      outputDirectory: output,
      ...(standard ? { handoff: true } : {}),
    };
    const before = await projection();
    const preview = await invoke(previewCommand, payload);
    await assert.rejects(access(output), { code: 'ENOENT' });
    assert.deepEqual(await projection(), before);
    await invoke(generateCommand, { ...payload, revision: preview.revision });
    for (const file of preview.files) {
      if (file.after !== null)
        assert.deepEqual(
          await readFile(path.join(output, file.path)),
          Buffer.from(file.after),
          `Installed exact prepared source: ${file.path}`,
        );
    }
    const handoff = JSON.parse(await readFile(path.join(output, 'handoff.json'), 'utf8'));
    assert.equal(handoff.format, 'autosar-workbench-handoff-v2');
    assert.deepEqual(handoff.resourceIdentities.ruleSetIdentity, before.ruleSetIdentity);
    assert.deepEqual(handoff.resourceIdentities.requiredExtensionDefinitions, []);
    const ledger = JSON.parse(
      await readFile(path.join(output, 'workbench-ownership.json'), 'utf8'),
    );
    assert.equal(ledger.formatVersion, 1);
    const sealed = (await readFile(path.join(output, 'files.list'), 'utf8')).trim().split(/\r?\n/);
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
    evidence.packages.push(packageEvidence);
    return packageEvidence;
  }
  async function packageBytes(directory) {
    const names = (await readFile(path.join(directory, 'files.list'), 'utf8'))
      .trim()
      .split(/\r?\n/);
    return Promise.all(
      [...names, 'files.list', 'files.sha256'].map(async (name) => ({
        path: path.join(directory, name),
        bytes: await readFile(path.join(directory, name)),
      })),
    );
  }
  async function resealPrivateVariant(directory) {
    const ledgerPath = path.join(directory, 'workbench-ownership.json');
    const ledger = JSON.parse(await readFile(ledgerPath, 'utf8'));
    for (const entry of ledger.files)
      entry.sha256 = sha256(await readFile(path.join(directory, entry.path)));
    await writeFile(ledgerPath, json(ledger, null, 2) + '\n');
    const names = (await readFile(path.join(directory, 'files.list'), 'utf8'))
      .trim()
      .split(/\r?\n/);
    const hashes = await Promise.all(
      [...names, 'files.list'].map(
        async (name) => `${sha256(await readFile(path.join(directory, name)))}  ${name}`,
      ),
    );
    await writeFile(path.join(directory, 'files.sha256'), hashes.join('\n') + '\n');
  }
  try {
    assert.equal(typeof scenario.installed, 'boolean');
    assert.equal(scenario.mode, 'builtin-only');
    await until(
      `Boolean(window.__TAURI_INTERNALS__ && document.querySelector('button'))`,
      'native startup',
    );
    // Keep the existing Chinese scenario deterministic on every host locale.
    const startupBoot = await evaluate('performance.timeOrigin');
    await invoke('configure_language', { language: 'zh-CN' });
    await evaluate('setTimeout(() => location.reload(), 0); true');
    await until(
      `performance.timeOrigin !== ${startupBoot} && Boolean(window.__TAURI_INTERNALS__ && document.querySelector('[aria-label="设置"]'))`,
      'explicit Chinese native startup',
    );
    await evaluate(`window.__BUILTIN_NATIVE_TRANSPORT__ = window.__TAURI_INTERNALS__.invoke`);
    const location = await evaluate(
      `({href:location.href, protocol:location.protocol, hostname:location.hostname, port:location.port})`,
    );
    evidence.location = location;
    const boundary = JSON.parse(
      await readFile(path.join(scratch, 'application-environment.json'), 'utf8'),
    );
    assert.equal(boundary.mode, 'builtin-only');
    assert.equal(boundary.installed, scenario.installed);
    let network = null;
    if (scenario.installed) {
      assert(
        (location.protocol === 'tauri:' && location.hostname === 'localhost') ||
          (['http:', 'https:'].includes(location.protocol) &&
            location.hostname === 'tauri.localhost'),
      );
      assert.equal(location.port, '');
      evidence.sourceCheckoutBoundary = await assertUnavailableSourceCheckout(
        scratch,
        scenario.sourceCheckout,
        platform,
      );
      network = JSON.parse(await readFile(path.join(scratch, 'network-isolation.json'), 'utf8'));
      assert.equal(network.platform, platform);
      assert.equal(network.status, 'enforced');
      assert.equal(network.userNetworkChanged, false);
      if (platform === 'linux') {
        const namespace = await readFile('/proc/self/net/route', 'utf8');
        assert(network.namespace !== network.outerNamespace);
        assert.equal(await readlink('/proc/self/ns/net'), network.namespace);
        assert(network.interfaces.every((item) => item.ifname === 'lo'));
        assert(
          !namespace
            .split('\n')
            .slice(1)
            .some((line) => line && !line.startsWith('lo\t')),
          'Driver and native child must have no non-loopback routes',
        );
      } else {
        const launch = JSON.parse(await readFile(path.join(scratch, 'native-launch.json'), 'utf8'));
        assert.equal(launch.clipboardIsolation, true);
        assert.notEqual(launch.windowStation.toLowerCase(), 'winsta0');
        assert.equal(
          launch.appToken.elevated,
          false,
          'Only the controller may be elevated; installed application must be an ordinary user process',
        );
        assert(launch.appToken.integrityRid >= 0x2000 && launch.appToken.integrityRid < 0x3000);
        assert.equal(path.resolve(network.binary), path.resolve(launch.binary));
      }
      evidence.networkIsolation = { status: 'enforced', receipt: network };
    } else {
      assert.equal(location.href.startsWith('http://127.0.0.1:1420/'), true);
      evidence.sourceCheckoutBoundary = {
        status: 'not_run',
        reason: 'development',
      };
      evidence.networkIsolation = { status: 'not_run', reason: 'development' };
    }
    assert(Object.values(boundary.tools).every((value) => value === null));
    await checked(scenario.installed ? 'installed-boundary' : 'development-boundary', async () => {
      const caps = await capabilities();
      assert.equal(caps.ruleError, null);
      assert(caps.ruleSetIdentity?.sha256 && caps.ruleCoverage.length > 0);
      assert(!caps.xsdArchive && !caps.modArchive, 'Official resource settings must be empty');
      assert(caps.toolError, 'Execution tools must not leak into configuration process');
      assert(
        !(await evaluate(`Boolean(document.querySelector('[aria-label="设置"][role=dialog]'))`)),
        'No resource setup gate',
      );
      return {
        capabilities: caps,
        boundary,
        checkout: scenario.installed ? 'unavailable' : 'development',
        network,
      };
    });
    assert.equal(typeof resize, 'function');
    assert.equal(typeof key, 'function');
    assert.equal(typeof pointer, 'function');
    await resize(1480, 920);
    if (!scenario.installed) {
      const multi = await createMultiComponentInputs(scratch);
      await openMember(multi.manifest);
      await checked('multi-component-shared-editing-and-source-protection', async () => {
        let view = await projection();
        assert.equal(view.profile, 'singlecore-multi-swc-v1');
        assert.equal(view.diagnostics.filter((issue) => issue.severity === 'error').length, 0);
        const original = await sourceBytes(view);
        const periodic = view.objects.find(
          (object) => object.path === '/Application/Process/Behavior/Periodic10ms',
        );
        const period = view.fields.find(
          (field) =>
            field.objectId === periodic.objectId && field.definitionId === 'TIMING-EVENT#PERIOD',
        );
        assert(period?.writable);
        const longPeriodicName =
          'Periodic10msForPipelineProcessingWithAnExplicitLongObjectIdentity';
        await selectObject(periodic.objectId);
        await input(label('实例名称'), longPeriodicName);
        await input(`[data-field-id=${json(period.fieldId)}]`, '0.0100');
        await resize(1080, 720);
        await screenshot('multi-long-name-narrow-draft.png');
        await click('预览更改');
        await until(
          `Boolean(document.querySelector('[aria-label="确认原子应用批次"]'))`,
          'multi field batch preview',
        );
        await until(
          `[...document.querySelectorAll('[role=dialog] button')].some(node => node.textContent.trim() === '确认应用全部变化' && !node.disabled)`,
          'multi preview ready for keyboard cancel',
        );
        await until(
          `Boolean(document.activeElement?.closest('[role=dialog]'))`,
          'multi preview receives focus',
        );
        await key('Escape');
        await until(`!document.querySelector('[role=dialog]')`, 'multi preview cancel');
        assert.equal(
          await evaluate(`document.querySelector('[data-field-id="${period.fieldId}"]').value`),
          '0.0100',
        );
        await click('预览更改');
        await until(
          `Boolean(document.querySelector('[aria-label="确认原子应用批次"]'))`,
          'multi field preview again',
        );
        await click('确认应用全部变化', "document.querySelector('[role=dialog]')");
        await until(`!document.querySelector('[role=dialog]')`, 'multi batch applied');
        await until(
          `(async () => !(await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation)()`,
          'multi batch operation completed',
        );
        await save();
        view = await refreshSurface(multi.manifest);
        assert.equal(
          view.fields.find(
            (field) =>
              field.definitionId === 'TIMING-EVENT#PERIOD' &&
              field.objectId ===
                view.objects.find((object) => object.shortName === longPeriodicName).objectId,
          ).current.value.lexeme,
          '0.0100',
        );
        await selectObject(
          view.objects.find((object) => object.shortName === longPeriodicName).objectId,
        );
        await screenshot('multi-long-name-narrow-reopened.png');
        await resize(1480, 920);
        await unchanged(
          original.filter(
            (source) =>
              !source.path.endsWith('/process.arxml') && !source.path.endsWith('/ecuc.arxml'),
          ),
        );
        const mappingSource = original.find((source) => source.path.endsWith('/ecuc.arxml'));
        assert.deepEqual(
          await readFile(mappingSource.path),
          Buffer.from(
            mappingSource.bytes
              .toString('utf8')
              .replaceAll(
                periodic.path,
                `${periodic.path.slice(0, periodic.path.lastIndexOf('/') + 1)}${longPeriodicName}`,
              ),
          ),
          'Event rename changes only the known OS mapping reference bytes',
        );
        view = await projection();
        const connector = view.objects.find(
          (object) => object.path === '/Application/Pipeline/IngressObserve',
        );
        await selectObject(connector.objectId);
        const provider = view.fields.find(
          (field) =>
            field.objectId === connector.objectId &&
            field.definitionId === 'ASSEMBLY-SW-CONNECTOR#PROVIDER-IREF/CONTEXT-COMPONENT-REF',
        );
        const port = view.fields.find(
          (field) =>
            field.objectId === connector.objectId &&
            field.definitionId === 'ASSEMBLY-SW-CONNECTOR#PROVIDER-IREF/TARGET-P-PORT-REF',
        );
        const instanceTarget = view.referenceCandidates.find(
          (candidate) =>
            candidate.fieldId === provider.fieldId &&
            candidate.path === '/Application/Pipeline/ProcessInstance',
        );
        const portTarget = view.referenceCandidates.find(
          (candidate) =>
            candidate.fieldId === port.fieldId && candidate.path === '/Application/Process/Result',
        );
        await input(
          `[data-field-id=${json(provider.fieldId)}]`,
          `${instanceTarget.targetId}:${instanceTarget.dest}`,
        );
        await input(
          `[data-field-id=${json(port.fieldId)}]`,
          `${portTarget.targetId}:${portTarget.dest}`,
        );
        await click('预览更改');
        await until(
          `Boolean(document.querySelector('[aria-label="确认原子应用批次"]'))`,
          'multi paired endpoint preview',
        );
        await click('确认应用全部变化', "document.querySelector('[role=dialog]')");
        await until(`!document.querySelector('[role=dialog]')`, 'multi paired endpoint applied');
        await until(
          `(async () => !(await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation)()`,
          'multi paired endpoint operation completed',
        );
        await save();
        await refreshSurface(multi.manifest);
        const initialization = await invoke('preview_application_initialization');
        assert.equal(initialization.slots.length, 3);
        await menu('预览初始化用户应用…');
        await until(
          `Boolean(document.querySelector('[role=dialog]'))`,
          'multi all source slots preview',
        );
        await click('确认创建用户应用与成员记录', "document.querySelector('[role=dialog]')");
        await until(`!document.querySelector('[role=dialog]')`, 'multi create-only initialization');
        await until(
          `(async () => !(await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation)()`,
          'multi initialization operation completed',
        );
        const users = [];
        for (const file of initialization.files) {
          const bytes = Buffer.from(file.contents + '\n/* User-owned multi-component source. */\n');
          await writeFile(path.join(multi.directory, file.path), bytes);
          users.push({ ...file, bytes });
        }
        await refreshSurface(multi.manifest);
        const delivered = await exportSource('MultiComponent-source', true);
        for (const file of users) {
          assert.deepEqual(await readFile(path.join(multi.directory, file.path)), file.bytes);
          assert.deepEqual(
            await readFile(path.join(delivered.directory, 'src', path.basename(file.path))),
            file.bytes,
          );
        }
        view = await projection();
        const component = view.objects.find((object) => object.path === '/Application/Process');
        await refuse(
          'prepare_configuration_change',
          {
            changeSet: changeSet(view, [
              {
                op: 'rename-instance',
                changeId: 'unsafe-member-rename',
                object: existing(component.objectId),
                expectedShortName: 'Process',
                shortName: 'Compute',
              },
            ]),
          },
          'Initialized multi-component member identity must remain stable',
        );
        return {
          profile: view.profile,
          slots: initialization.slots,
          originalBytesRetained: true,
          liveSourcesRetained: true,
          generatedSnapshotRetained: true,
        };
      });
    }
    const canA = await create('NorthSensor', 'can-signals-v1');
    await checked('can-original-A', async () => {
      const view = await projection();
      await sourceSurface(view);
      const id = fieldBy(view, '/CanIfTxPduCanId');
      const initial = fieldBy(view, '/ComSignalInitValue');
      const applied = await apply(view, [
        valueChange(id, '865'),
        valueChange(initial, '305419896'),
      ]);
      assert.equal(
        fieldBy(applied.outcome.projection, '/CanIfTxPduCanId').current.value.lexeme,
        '865',
      );
      assert.equal(
        fieldBy(applied.outcome.projection, '/ComSignalInitValue').current.value.lexeme,
        '305419896',
      );
      await save();
      const reopened = await refreshSurface(canA.manifest);
      assert.notEqual(reopened.workspaceEpoch, view.workspaceEpoch);
      assert.equal(fieldBy(reopened, '/CanIfTxPduCanId').current.value.lexeme, '865');
      await exportSource('NorthSensor-source', false);
      return { canId: 865, initial: '305419896', sources: reopened.sources };
    });
    await checked('changed-ui-drafts-save-and-entry-preservation', async () => {
      await refreshSurface(canA.manifest);
      async function genericInitial(value, multiple = false) {
        const field = fieldBy(await projection(), '/ComSignalInitValue');
        await selectObject(field.objectId, multiple);
        await input(`[data-field-id=${json(field.fieldId)}]`, value);
        await click('预览更改');
        await until(
          `Boolean(document.querySelector('[aria-label="确认原子应用批次"]'))`,
          'actual generic batch preview',
        );
        await click('确认应用全部变化', "document.querySelector('[role=dialog]')");
        await until(
          `!document.querySelector('[role=dialog]') && !document.querySelector(${json(`[data-field-id=${json(field.fieldId)}]`)})?.disabled`,
          'actual generic batch applied',
        );
        assert.equal(
          fieldBy(await projection(), '/ComSignalInitValue').current.value.lexeme,
          value,
        );
      }
      async function surfaceSave() {
        await click('预览保存');
        await until(
          `Boolean(document.querySelector('[aria-label="确认保存源配置"]')) && [...document.querySelectorAll('[role=dialog] button')].some(node => node.textContent.trim() === '确认保存' && !node.disabled)`,
          'actual save preview ready for confirmation after generic batch',
        );
        await click('确认保存', "document.querySelector('[role=dialog]')");
        await until(`!document.querySelector('[role=dialog]')`, 'actual confirmed save');
      }
      await menu('CAN 通信');
      await click('Command');
      await click('Value');
      await genericInitial('305419897');
      await surfaceSave();
      assert.equal(
        fieldBy(await refreshSurface(canA.manifest), '/ComSignalInitValue').current.value.lexeme,
        '305419897',
      );
      await menu('CAN 通信');
      await click('Command');
      await click('Value');
      const legacyInitial = `(() => [...document.querySelectorAll('label')].find(node => node.childNodes[0]?.textContent.trim() === '初始值')?.querySelector('input'))()`;
      await evaluate(
        `(() => { const node = ${legacyInitial}; if (!node || node.disabled) throw new Error('Actual legacy signal initial editor unavailable'); node.focus(); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(node, '305419899'); node.dispatchEvent(new Event('input', {bubbles:true})); })()`,
      );
      await genericInitial('305419898', true);
      await menu('CAN 通信');
      assert.equal(
        await evaluate(`${legacyInitial}?.value`),
        '305419899',
        'Confirmed generic batch preserves genuinely dirty legacy draft',
      );
      await click('还原');
      assert.equal(
        await evaluate(`${legacyInitial}?.value`),
        '305419898',
        'Explicit legacy restore uses confirmed backend value',
      );
      await surfaceSave();
      assert.equal(
        fieldBy(await refreshSurface(canA.manifest), '/ComSignalInitValue').current.value.lexeme,
        '305419898',
      );
      await genericInitial('305419896');
      await surfaceSave();
      const beforeEntry = await refreshSurface(canA.manifest);
      await menu('新建工程…');
      const unfinishedDirectory = path.join(scratch, 'projects', 'UnfinishedEntry');
      await input(label('工程名称'), 'UnfinishedEntry');
      await mkdir(unfinishedDirectory);
      await choose(
        'directory',
        unfinishedDirectory,
        () => click('选择', "document.querySelector('.start-page')"),
        async () => {
          await until(
            `document.querySelector(${json(label('新工程目录'))})?.value === ${json(unfinishedDirectory)}`,
            'native unfinished entry directory result',
          );
          return evaluate(`document.querySelector(${json(label('新工程目录'))}).value`);
        },
      );
      for (const action of [
        () => click('保存为成员工程', "document.querySelector('.start-page')"),
        () => menu('保存为成员工程…'),
      ]) {
        await action();
        assert.equal(
          await evaluate(`document.querySelector(${json(label('工程名称'))}).value`),
          'UnfinishedEntry',
        );
        assert.equal(
          await evaluate(`document.querySelector(${json(label('新工程目录'))}).value`),
          unfinishedDirectory,
        );
        assert.deepEqual(
          await projection(),
          beforeEntry,
          'Save-as navigation must not implicitly create or replace a project',
        );
        assert.deepEqual(
          await readdir(unfinishedDirectory),
          [],
          'Save-as navigation must not create project files',
        );
      }
      await refreshSurface(canA.manifest);
      return {
        cleanGenericSave: '305419897',
        dirtyLegacyPreserved: '305419899',
        restoredOracleInitial: '305419896',
        saveAsDraftPreserved: true,
      };
    });
    await checked('batch-rejection-and-byte-preservation', async () => {
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
      await refreshSurface(canA.manifest);
      return {
        changedDraft: 'rejected',
        conflict: 'rejected',
        stale: 'rejected',
        partialPublish: false,
      };
    });
    const canB = await create('SouthActuator', 'can-signals-v1');
    await checked('can-original-B', async () => {
      const view = await projection();
      const fields = [fieldBy(view, '/CanIfTxPduCanId'), fieldBy(view, '/ComSignalInitValue')];
      const period = fieldBy(view, '/ComTxModeTimePeriod');
      await apply(view, [
        valueChange(fields[0], '913'),
        valueChange(fields[1], '16909060'),
        valueChange(period, '0.040'),
      ]);
      await save();
      const reopened = await refreshSurface(canB.manifest);
      assert.equal(fieldBy(reopened, '/CanIfTxPduCanId').current.value.lexeme, '913');
      assert.equal(fieldBy(reopened, '/ComSignalInitValue').current.value.lexeme, '16909060');
      assert.equal(fieldBy(reopened, '/ComTxModeTimePeriod').current.value.lexeme, '0.040');
      assert.notEqual(
        sha256(await readFile(canA.view.sources[0].path)),
        sha256(await readFile(reopened.sources[0].path)),
        'Two CAN inputs must differ in actual configuration, not only file location',
      );
      await exportSource('SouthActuator-source', false);
      return {
        canId: 913,
        periodMs: 40,
        initial: '16909060',
        sources: reopened.sources,
      };
    });
    await checked('structure-and-incoming-reference-closure', async () => {
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
        (object) =>
          object.definitionId === '/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection',
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
      await refuse(
        'prepare_configuration_change',
        { changeSet: changeSet(current, [createChange]) },
        'Incomplete required creation cannot publish',
      );
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
      const fieldId = created.outcome.createdFields.find(
        (item) => item.entryKey === 'length',
      ).fieldId;
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
      return {
        renamedObjectId: signal.objectId,
        inboundCount: incoming.length,
        deletedIdNotReused: true,
      };
    });
    await refreshSurface(canB.manifest);
    const standard = await create('GatewayReference', 'standard-ecu-v1');
    await checked('seven-source-standard-variant', async () => {
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
      assert.equal(actual.description.component.periodMs, 20);
      assert.deepEqual(
        actual.description.signals.map((signal) => signal.canId),
        [1104, 1105],
      );
      for (const source of savedBytes.filter((source) =>
        ['types.arxml', 'services.arxml', 'unrelated.arxml'].includes(path.basename(source.path)),
      ))
        assert.deepEqual(await readFile(source.path), source.bytes);
      await sourceSurface(await projection());
      const result = await exportSource('GatewayReference-source', true);
      result.expectedInputs = {
        periodMs: 20,
        receiveCanId: 1104,
        transmitCanId: 1105,
      };
      return {
        inputCount: 7,
        independentPeriod: 20,
        independentCanIds: [1104, 1105],
        unchanged: ['types.arxml', 'services.arxml', 'unrelated.arxml'],
        sourceFallback:
          'two distinct sources, closed active B restores A header, rendered bytes, and full native clipboard',
      };
    });
    await checked('member-portability-and-save-as', async () => {
      const before = await projection();
      const moved = path.join(scratch, 'projects', 'MovedGateway');
      await rename(standard.directory, moved);
      const reopened = await openMember(path.join(moved, 'workbench-project.json'));
      assert.notEqual(reopened.workspaceEpoch, before.workspaceEpoch);
      assert.equal(reopened.sources.length, 7);
      await refuse(
        'read_project_source',
        { sourceId: before.sources[0].sourceId },
        'Reopened workspace rejects previous source identity',
      );
      const original = await sourceBytes(reopened);
      const saveAsDirectory = path.join(scratch, 'projects', 'CopiedGateway');
      const preview = await invoke('preview_save_as_project', {
        directory: saveAsDirectory,
        name: 'CopiedGateway',
      });
      await assert.rejects(access(saveAsDirectory), { code: 'ENOENT' });
      await invoke('save_as_project_previewed', { preview });
      await unchanged(original);
      const copied = await projection();
      assert.equal(copied.sources.length, 7);
      assert.deepEqual(copied.acceptedExtensionDefinitions, []);
      await refreshSurface(copied.projectPath);
      await refuse(
        'preview_project_creation',
        { directory: moved, name: 'Occupied', templateId: 'can-empty-v1' },
        'Occupied destination cannot be overwritten',
      );
      const manifestPath = copied.projectPath;
      const manifestBytes = await readFile(manifestPath);
      const manifest = JSON.parse(manifestBytes);
      const manifestFailures = {
        parentEscape: (value) => {
          value.inputs[0].path = '../outside.arxml';
        },
        absolutePath: (value) => {
          value.inputs[0].path = path.join(moved, manifest.inputs[0].path);
        },
        duplicatePath: (value) => {
          value.inputs.push({ ...value.inputs[0] });
        },
        unsupportedVersion: (value) => {
          value.formatVersion = 999;
        },
      };
      for (const [kind, mutate] of Object.entries(manifestFailures)) {
        const candidate = structuredClone(manifest);
        mutate(candidate);
        const bytes = Buffer.from(json(candidate));
        await writeFile(manifestPath, bytes);
        await refuse('open_member_project', { path: manifestPath }, `Manifest ${kind} refusal`);
        assert.deepEqual(
          await readFile(manifestPath),
          bytes,
          'Refused manifest stays byte-identical',
        );
        await unchanged(original);
      }
      await writeFile(manifestPath, manifestBytes);
      return {
        moved,
        saveAsDirectory,
        manifestRefusals: Object.keys(manifestFailures),
        originalBytesRetained: true,
      };
    });
    await checked('v2-relocation-snapshot-import-and-refusals', async () => {
      const delivery = evidence.packages.find((entry) => entry.name === 'GatewayReference-source');
      assert(delivery);
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
        () => menu('重导入 ECU 交接包…'),
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
      await awaitProjectSurfaceReady();
      const imported = await projection();
      assert.notEqual(imported.workspaceEpoch, previous.workspaceEpoch);
      assert.equal(imported.sources.length, 7);
      const metadata = JSON.parse(await readFile(path.join(relocated, 'handoff.json'), 'utf8'));
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
      assert.equal(plan.description.component.periodMs, 20);
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
          const ledgerPath = path.join(variant, 'workbench-ownership.json');
          const ledger = JSON.parse(await readFile(ledgerPath, 'utf8'));
          ledger.files.find(
            (entry) => entry.path === metadata.inputSnapshots[0].packagePath,
          ).owner = 'user-application';
          await writeFile(ledgerPath, json(ledger));
          await resealPrivateVariant(variant);
        } else if (kind === 'payloadBytes') {
          const source = path.join(variant, metadata.inputSnapshots[0].packagePath);
          await writeFile(
            source,
            Buffer.concat([
              await readFile(source),
              Buffer.from('\n<!-- external snapshot edit -->\n'),
            ]),
          );
        } else {
          const altered = structuredClone(metadata);
          if (kind === 'unknownFormat') altered.format = 'unknown-handoff-v99';
          if (kind === 'ruleIdentity')
            altered.resourceIdentities.ruleSetIdentity.sha256 = '0'.repeat(64);
          if (kind === 'duplicateSnapshots')
            altered.inputSnapshots.push({ ...altered.inputSnapshots[0] });
          await writeFile(path.join(variant, 'handoff.json'), json(altered));
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
      const generatedMetadata = path.join(relocated, 'handoff.json');
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
      return {
        relocated,
        destination,
        sourceSnapshotsIndependent: true,
        refusalKinds: [
          'unknownFormat',
          'ruleIdentity',
          'duplicateSnapshots',
          'ownerElevation',
          'payloadBytes',
          'occupiedDestination',
          'modifiedGeneratedFile',
        ],
        originalLiveBytesRetained: true,
      };
    });
    await checked('live-application-initialization-and-immutable-snapshot', async () => {
      const before = await projection();
      const manifestPath = before.projectPath;
      const root = path.dirname(manifestPath);
      const manifestBytes = await readFile(manifestPath);
      const originalSources = await sourceBytes(before);
      const preview = await invoke('preview_application_initialization');
      assert.equal(preview.slots.length, 1);
      assert.equal(preview.slots[0].producerSlot, 'epic4-single-application-v1');
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
        `Boolean([...document.querySelectorAll('[role=dialog]')].find(node=>node.textContent.includes('epic4-single-application-v1')))`,
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
      const writeLine = /^    application\.write_status = [A-Za-z_]\w*\(value\);$/m;
      assert.equal([...seed.matchAll(new RegExp(writeLine.source, 'gm'))].length, 1);
      const userCode = seed.replace(writeLine, '    value &= UINT32_C(2147483647);\n$&');
      await writeFile(live, userCode);
      await refreshSurface(manifestPath);
      const delivered = await exportSource('GatewayUserApplication-source', true);
      assert.deepEqual(
        await readFile(path.join(delivered.directory, 'src/Application.c')),
        Buffer.from(userCode),
        'Compiled user snapshot must contain the actual live bytes',
      );
      const ledger = JSON.parse(
        await readFile(path.join(delivered.directory, 'workbench-ownership.json'), 'utf8'),
      );
      const owner = ledger.files.find((entry) => entry.path === 'src/Application.c');
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
      delivered.expectedInputs = {
        periodMs: 20,
        receiveCanId: 1104,
        transmitCanId: 1105,
        userApplication: 'uint32-low31-mask',
      };
      return {
        slots: preview.slots,
        live,
        createOnly: true,
        actualPublicCommand: initializationCommand,
        userSha256: sha256(Buffer.from(userCode)),
        immutableSnapshotRetained: true,
        lateChangeRefused: true,
      };
    });
    const authored = await createBuiltinInputs(scratch);
    await menu('导入 ARXML…');
    await input(label('ARXML 来源路径'), authored.sources.join('\n'));
    await click('导入并进入工程');
    await until(
      `Boolean(document.querySelector('.document-tabs')) && !document.querySelector('[role=dialog]')`,
      'authored multi-module import',
    );
    await awaitImportedSources(authored.sources);
    await checked('seven-value-types-and-cross-source-batch', async () => {
      const view = await projection();
      assert.deepEqual(
        new Set(view.fields.filter((field) => field.writable).map((field) => field.kind)),
        new Set([
          'integer',
          'float',
          'boolean',
          'enumeration',
          'string',
          'function-name',
          'reference',
        ]),
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
        applied.outcome.projection.references.find(
          (edge) => edge.fieldId === referenceField.fieldId,
        ).rawPath,
        '/Acceptance/Os/DiagnosticReady',
      );
      for (const [field, lexeme] of fields)
        assert.equal(
          applied.outcome.projection.fields.find((item) => item.fieldId === field.fieldId).current
            .value.lexeme,
          lexeme,
        );
      const preserved = view.sources.find(
        (source) => path.basename(source.path) === 'unknown.arxml',
      );
      const bytes = await readFile(preserved.path);
      const unknownObject = view.objects.find((object) => object.shortName === 'PreservedType');
      assert(unknownObject && !unknownObject.writable);
      assert(
        view.validation.some(
          (scope) => scope.status === 'unsupported' || scope.status === 'failed',
        ),
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
          target: existing(
            reopened.objects.find((object) => object.path === candidate.path).objectId,
          ),
        },
      };
      await refuse(
        'prepare_configuration_change',
        { changeSet: changeSet(reopened, [badReference]) },
        'Wrong reference DEST',
      );
      return {
        types: expected,
        modules: ['Os', 'Com', 'Can', 'CanIf', 'Dcm'],
        reference: '/Acceptance/Os/DiagnosticReady',
        unknownSource: preserved.path,
        unknownBytes: sha256(bytes),
        defaultOrigin: fieldBy(reopened, '/CanDevErrorDetect').defaultOrigin,
      };
    });
    await checked('absent-default-explicit-empty-and-removal', async () => {
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
      assert.deepEqual(
        fieldBy(reopened, '/CanVersionInfoApi').current,
        explicit('boolean', 'false'),
      );
      assert.deepEqual(fieldBy(reopened, '/CanIfInitCfgSet').current, explicit('string', ''));
      assert.deepEqual(fieldBy(reopened, '/DcmDspDataReadFnc').current, {
        state: 'absent',
      });
      return {
        defaultOrigin: defaultField.defaultOrigin,
        emptyIsExplicit: true,
        removalIsAbsent: true,
      };
    });
    await checked('source-safety-rejections', async () => {
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
      return {
        rejected: Object.keys(authored.safety),
        externalSave: 'rejected-with-bytes-retained',
      };
    });
    await checked('extension-isolation-and-default-preservation', async () => {
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
      return {
        accepted: extension.identity,
        failedImportRetainsOldCatalog: true,
        immutableCacheRechecked: true,
        missingCachePreservesBuiltinConsumer: true,
      };
    });
    // Reopen through the public surface; never synchronize by private React setters.
    await refreshSurface(null, authored.sources);
    await checked('themes-settings-layout-and-keyboard', async () => {
      const view = await projection();
      const field = fieldBy(view, '/OsEventMask');
      await selectObject(field.objectId);
      await input(`[data-field-id=${json(field.fieldId)}]`, '18446744073709551614');
      const draftSelector = `[data-field-id=${json(field.fieldId)}]`;
      for (const [width, height] of [
        [1480, 920],
        [1080, 720],
      ]) {
        const nativeWindow = await resize(width, height);
        await click('设置');
        await until(
          `Boolean(document.querySelector('[role=dialog][aria-label="设置"]'))`,
          'settings dialog',
        );
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
          await screenshot(`layout-${width}x${height}-${theme}.png`);
        }
        for (const scheme of ['dark', 'light']) {
          const reduceMotion = scheme === 'dark';
          const receipt = await media(scheme, reduceMotion);
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
          evidence.checks.at(-1).systemMedia ??= [];
          evidence.checks.at(-1).systemMedia.push(receipt);
          await screenshot(`layout-${width}x${height}-system-event-${scheme}.png`);
        }
        await click('执行工具', "document.querySelector('[role=dialog]')");
        const caps = await capabilities();
        await input(label('编译器保存路径'), path.join(scratch, 'must-not-be-saved'));
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
          await evaluate(
            `document.querySelector('[role=dialog]').contains(document.activeElement)`,
          ),
          'Dialog must trap keyboard focus',
        );
        await key('Escape');
        await until(`!document.querySelector('[role=dialog]')`, 'Escape closes settings');
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
        evidence.checks.at(-1).layouts ??= [];
        evidence.checks.at(-1).layouts.push({
          ...bounds,
          requested: [width, height],
          nativeWindow,
        });
      }
      await key('4', { alt: true });
      await until(
        `Boolean(document.querySelector('[role=tabpanel][aria-label="构建"]'))`,
        'Alt+4 opens build',
      );
      await key('9', { alt: true });
      await until(
        `Boolean(document.querySelector('[role=tabpanel][aria-label="主机验证"]'))`,
        'Alt+9 opens host',
      );
      await input(label('搜索对象表名称、路径、定义、值'), 'does-not-match');
      assert.equal(
        await evaluate(`document.querySelector(${json(draftSelector)}).value`),
        '18446744073709551614',
        'Filtering must not discard hidden object draft',
      );
      await input(label('搜索对象表名称、路径、定义、值'), '');
      const second = view.objects.find(
        (object) => object.objectId !== field.objectId && object.writable,
      );
      await selectObject(second.objectId);
      await until(`Boolean(document.querySelector('[role=dialog]'))`, 'context draft guard');
      await click('留在原处', "document.querySelector('[role=dialog]')");
      assert.equal(
        await evaluate(`document.querySelector(${json(draftSelector)}).value`),
        '18446744073709551614',
      );
      await input(draftSelector, '18446744073709551616');
      await selectObject(second.objectId);
      await click('应用并继续', "document.querySelector('[role=dialog]')");
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
        `Boolean(document.querySelector('[role=dialog]'))`,
        'retained invalid draft guard',
      );
      await click('放弃草稿并继续', "document.querySelector('[role=dialog]')");
      await until(
        `document.querySelector('[data-object-title]')?.dataset.objectTitle === ${json(second.objectId)}`,
        'explicit draft discard',
      );
      return {
        themes: ['light', 'dark', 'system'],
        sizes: [
          [1480, 920],
          [1080, 720],
        ],
        failedApplyRetainsContext: true,
        categoriesIsolated: true,
      };
    });
    await checked('language-preview-save-preserves-project-and-draft', async () => {
      const before = await projection();
      const field = fieldBy(before, '/OsEventMask');
      await selectObject(field.objectId);
      const draftSelector = `[data-field-id=${json(field.fieldId)}]`;
      const draftValue = '18446744073709551614';
      await input(draftSelector, draftValue);
      const caps = await capabilities();
      await click('设置');
      await input(label('界面语言'), 'en');
      await until(`document.documentElement.lang === 'en'`, 'English language preview');
      assert.equal((await capabilities()).language, 'zh-CN', 'Preview must not save');
      assert.equal((await capabilities()).fingerprint, caps.fingerprint);
      await screenshot('language-english-preview.png');
      await key('Escape');
      await until(`document.documentElement.lang === 'zh-CN'`, 'Canceled language preview');
      assert.equal(
        await evaluate(`document.querySelector(${json(draftSelector)}).value`),
        draftValue,
      );
      await click('设置');
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
      assert.equal(
        await evaluate(`document.querySelector(${json(draftSelector)}).value`),
        draftValue,
      );
      await screenshot('language-english-saved.png');
      await click('Settings');
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
      assert.equal(
        await evaluate(`document.querySelector(${json(draftSelector)}).value`),
        draftValue,
      );
      // Restore the clean editor needed by the following independent scenarios.
      await refreshSurface(null, authored.sources);
      return {
        previewCanceled: true,
        savedLanguages: ['en', 'zh-CN'],
        fingerprint: caps.fingerprint,
      };
    });
    await checked('problem-focus', async () => {
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
      assert.deepEqual(JSON.parse(details), issue);
      const copiedDiagnostic = await clipboard(
        details,
        `[...document.querySelectorAll('.problem-list li')].filter(${visible})[${issueIndex}]`,
        '复制详情',
      );
      const location = await evaluate(`(() => {
        const rows = [...document.querySelectorAll('.problem-list li')].filter(${visible});
        const row = rows[${issueIndex}];
        if (!row?.textContent.includes(${json(issue.code)}) || !row.textContent.includes(${json(issue.path ?? issue.file)}))
          throw new Error('Actual projected diagnostic row identity changed');
        const button = row.querySelector('button'); if (!button) throw new Error('Genuine issue locator missing');
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
      return {
        actualIssue: issue,
        copiedDiagnostic,
        boundedAndCompleteOwnedLog: 'separate-declared-tool-phase',
      };
    });
    await checked('expanded-input-responsiveness-and-cancel', async () => {
      const before = await projection();
      await memorySample('before-expanded-input');
      await menu('导入 ARXML…');
      await input(label('ARXML 来源路径'), authored.large);
      await click('导入并进入工程');
      await until(
        `(async()=>Boolean((await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation))()`,
        'real pending parse observed',
        10000,
      );
      const responseStart = performance.now();
      await key('4', { alt: true });
      await until(
        `Boolean(document.querySelector('[role=tabpanel][aria-label="构建"]'))`,
        'tool remains responsive during parse',
        5000,
      );
      await click('取消', "document.querySelector('.context-toolbar')");
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
      await input(label('ARXML 来源路径'), authored.large);
      await click('导入并进入工程');
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
          `Boolean(document.querySelector('[role=tabpanel][aria-label="构建"]'))`,
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
      evidence.snapshotReuse = {
        status: 'passed',
        before: beforeFilters,
        after: afterFilters,
        definitionArchiveDecompressions:
          'not_applicable: native extension catalogs use raw listed ARXML, no ZIP path exists',
      };
      assert.deepEqual(
        await projection(),
        expanded,
        'Filter/expand cannot alter native source snapshot or IDs',
      );
      await memorySample('expanded-filtered-repeatedly');
      evidence.memory.status = 'observed';
      return {
        inputBytes: (await readFile(authored.large)).length,
        timings,
        cancelledResponseMs,
        oldProjectRetained: true,
      };
    });
    evidence.configurationStatus = 'passed';
    await checked('declared-consumer-tools-and-real-owned-failure', async () => {
      const tools = scenario.performanceTools;
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
      const actualNotice = await evaluate(
        `document.querySelector('.notice[role=alert]').textContent`,
      );
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
      await until(
        `Boolean(${owned}?.getClientRects().length)`,
        'actual owned failure details opened',
      );
      assert.equal(await evaluate(`${owned}.querySelector('pre').textContent`), tail);
      const copied = await clipboard(actual.text, owned);
      return {
        phase: 'after-compiler-free-source-delivery',
        tools,
        stdout: actual.stdout,
        stderr: actual.stderr,
        exitCode: 23,
        actualNotice,
        completeLogSha256: sha256(Buffer.from(actual.text)),
        copied,
      };
    });
    await checked('owned-cancel-and-late-reply-fence', async () => {
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
        `Boolean(document.querySelector('[role=tabpanel][aria-label="构建"]'))`,
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
      await click(
        '操作日志',
        "document.querySelector('[aria-label=" + json('底部工具窗口') + "]')",
      );
      assert.equal(
        await evaluate(completedOwnedLogs),
        completed,
        'Cancelled/late owned replies must not publish another returned owned result',
      );
      return {
        realDelayMs: 5000,
        staleResultPublished: false,
        originalWorkspaceRetained: true,
        operation: null,
      };
    });
    evidence.status = 'configuration-passed';
    evidence.unverified = ['independentConsumer'];
    console.log(
      `BUILTIN_NATIVE_CONFIGURATION PASS platform=${platform}; independent consumer requires its separate receipt`,
    );
  } catch (error) {
    evidence.error = String(error);
    console.error(String(error), await evaluate('document.body.innerText'));
    await screenshot('builtin-native-failure.png');
    throw error;
  } finally {
    try {
      evidence.nativeTransportUnchanged = await evaluate(
        'window.__TAURI_INTERNALS__.invoke === window.__BUILTIN_NATIVE_TRANSPORT__',
      );
      await writeFile(
        path.join(scratch, 'native-builtin-results.json'),
        json(evidence, null, 2) + '\n',
      );
      assert.equal(
        evidence.nativeTransportUnchanged,
        true,
        'Never replace the installed native IPC transport',
      );
    } finally {
      await ipcLog.close();
    }
  }
}
