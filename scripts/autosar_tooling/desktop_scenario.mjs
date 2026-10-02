import assert from 'node:assert/strict';
import { access, readFile, writeFile, rename } from 'node:fs/promises';
import path from 'node:path';

export async function runScenario({ evaluate, screenshot, scratch, platform }) {
  const sources = JSON.parse(await readFile(path.join(scratch, 'inputs.json'), 'utf8'));
  const scenario = JSON.parse(await readFile(path.join(scratch, 'scenario.json'), 'utf8'));
  assert.equal(typeof scenario.installed, 'boolean');
  const runtimeEvidence = { ...scenario, platform };
  async function until(expression, expected = true) {
    const deadline = Date.now() + 300000;
    while (Date.now() < deadline) {
      const result = await evaluate(expression);
      if (result === expected) return;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    throw new Error(`UI condition did not become true: ${expression}`);
  }
  async function stage(name, expected) {
    const selector = `[data-stage=${JSON.stringify(name)}]`;
    const deadline = Date.now() + 300000;
    while (Date.now() < deadline) {
      const value = await evaluate(
        `document.querySelector(${JSON.stringify(selector)})?.textContent`,
      );
      if (value === expected) return;
      if (value === '失败' && expected === '已通过')
        throw new Error(`Actual ${name} failed: ${await evaluate('document.body.innerText')}`);
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    throw new Error(`Actual ${name} did not reach ${expected}`);
  }
  async function click(text) {
    await evaluate(`(() => {
    const button = [...document.querySelectorAll('button')].find(node => node.textContent.trim() === ${JSON.stringify(text)});
    if (!button || button.disabled) throw new Error('Missing/disabled button: ' + ${JSON.stringify(text)});
    button.click();
  })()`);
  }
  async function input(label, value) {
    await evaluate(`(() => {
    const element = document.querySelector('[aria-label=${JSON.stringify(label)}]');
    if (!element || element.disabled) throw new Error('Missing/disabled input');
    Object.getOwnPropertyDescriptor(element.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype, 'value').set.call(element, ${JSON.stringify(value)});
    element.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
  }
  try {
    await until(`Boolean(window.__TAURI_INTERNALS__ && document.querySelector('button'))`);
    const location = await evaluate(`(() => {
      window.__NATIVE_VERIFY_LOG__ = [];
      window.__NATIVE_VERIFY_TRANSPORT__ = window.__TAURI_INTERNALS__.invoke;
      return {
        href: window.location.href,
        protocol: window.location.protocol,
        hostname: window.location.hostname,
        port: window.location.port,
        nativeInvoke: typeof window.__TAURI_INTERNALS__.invoke === 'function'
      };
    })()`);
    runtimeEvidence.location = location;
    runtimeEvidence.transport = '__TAURI_INTERNALS__.invoke';
    assert.equal(location.nativeInvoke, true, 'The real native Tauri transport is required');
    if (scenario.installed) {
      assert.equal(typeof scenario.sourceCheckout, 'string');
      assert(path.isAbsolute(scenario.sourceCheckout));
      await assert.rejects(access(scenario.sourceCheckout), { code: 'ENOENT' });
      runtimeEvidence.sourceCheckoutAvailable = false;
      assert(
        (location.protocol === 'tauri:' && location.hostname === 'localhost') ||
          (['http:', 'https:'].includes(location.protocol) &&
            location.hostname === 'tauri.localhost'),
        `Installed UI must use the embedded native application origin: ${location.href}`,
      );
      assert.equal(location.port, '', 'Installed UI must not use a Vite development port');
    }
    const resources = JSON.parse(await readFile(path.join(scratch, 'resources.json'), 'utf8'));
    if (scenario.installed) {
      for (const resource of [resources.xsdArchive, resources.modArchive, ...sources]) {
        const relative = path.relative(scratch, resource);
        assert(
          relative &&
            !relative.startsWith(`..${path.sep}`) &&
            relative !== '..' &&
            !path.isAbsolute(relative),
          `Installed inputs and archives must belong to private scratch: ${resource}`,
        );
      }
    }
    await until(`Boolean(document.querySelector('[aria-label="XSD 档案路径"]'))`);
    const missingResources = await evaluate(
      "window.__TAURI_INTERNALS__.invoke('workbench_capabilities')",
    );
    assert(missingResources.resourceError, 'Missing archives must open the settings surface');
    runtimeEvidence.initialCapabilities = missingResources;
    await evaluate(`window.__NATIVE_VERIFY_LOG__.push({
      command: 'workbench_capabilities', status: 'returned',
      evidence: ${JSON.stringify(runtimeEvidence)}
    })`);
    await screenshot('initial-missing-resources.png');
    await input('XSD 档案路径', resources.xsdArchive);
    await input('MOD 档案路径', resources.modArchive);
    await click('核对并保存规范档案');
    await until(`document.body.innerText.includes('规范档案已就绪')`);
    const configuredResources = await evaluate(
      "window.__TAURI_INTERNALS__.invoke('workbench_capabilities')",
    );
    assert.equal(configuredResources.resourceError, null);
    assert.equal(configuredResources.xsdArchive, resources.xsdArchive);
    assert.equal(configuredResources.modArchive, resources.modArchive);
    await click('关闭设置');
    await evaluate(`window.__NATIVE_VERIFY_INVOKE__ = async (command, payload = {}) => {
  const caps = await window.__TAURI_INTERNALS__.invoke('workbench_capabilities');
  const event = {command, fingerprint: caps.fingerprint, payload};
  try {
    const reply = await window.__TAURI_INTERNALS__.invoke(command, {...payload, fingerprint: caps.fingerprint});
    window.__NATIVE_VERIFY_LOG__.push({...event, status: 'returned', inputFingerprint: reply.inputFingerprint, replyFingerprint: reply.capabilities.fingerprint});
    return reply.value;
  } catch (error) {
    window.__NATIVE_VERIFY_LOG__.push({...event, status: 'rejected', error: String(error)});
    throw error;
  }
};`);
    // Use the real path-entry flow. Native Tauri transport remains untouched.
    await click('导入 ARXML');
    await evaluate(`document.querySelector('details').open = true`);
    await input('ARXML 来源路径', sources.join('\n'));
    await until(`document.body.innerText.includes('导入 7 份文件')`);
    await click('导入 7 份文件');
    await until(`document.body.innerText.includes('标准输入已校验，尚未生成运行工程')`);
    const roles = await evaluate(
      `[...document.querySelectorAll('.integration-view tbody tr')].map(node => node.innerText)`,
    );
    assert.equal(roles.length, 7);
    assert(roles.some((row) => row.includes('ECU Extract')));
    assert(roles.some((row) => row.includes('BSW 实现')));
    await screenshot('standard-input-plan.png');
    const resourceCaps = await evaluate(
      "window.__TAURI_INTERNALS__.invoke('workbench_capabilities')",
    );
    const resourceWorkspace = await evaluate("window.__NATIVE_VERIFY_INVOKE__('workspace_view')");
    await click('工作台设置');
    await input('XSD 档案路径', resourceCaps.modArchive);
    await click('关闭设置');
    await click('工作台设置');
    assert.equal(
      await evaluate(`document.querySelector('[aria-label="XSD 档案路径"]').value`),
      resourceCaps.xsdArchive,
      'Closing settings must discard the unsaved resource draft',
    );
    await input('XSD 档案路径', resourceCaps.modArchive);
    await click('核对并保存规范档案');
    await until(
      `document.querySelector('.save-preview-dialog [role="status"]')?.textContent.includes('DEPENDENCY_IDENTITY')`,
    );
    const rejectedResources = await evaluate(
      "window.__TAURI_INTERNALS__.invoke('workbench_capabilities')",
    );
    assert.equal(rejectedResources.fingerprint, resourceCaps.fingerprint);
    assert.equal(rejectedResources.xsdArchive, resourceCaps.xsdArchive);
    assert.deepEqual(
      await evaluate("window.__NATIVE_VERIFY_INVOKE__('workspace_view')"),
      resourceWorkspace,
      'An archive identity rejection must preserve the open workspace',
    );
    await screenshot('resource-identity-rejection.png');
    await click('关闭设置');
    await input('接收 CAN ID', '1100');
    await input('发送 CAN ID', '1101');
    await input('应用周期', '20');
    await click('应用并校验修改');
    await until(`document.body.innerText.includes('修改已通过同一计划校验，尚未保存')`);
    await click('预览保存');
    await until(`document.body.innerText.includes('4 个文件将修改')`);
    await evaluate(
      `document.querySelector('[aria-label="标准保存预览"]').scrollIntoView({ block: 'end' })`,
    );
    for (const source of sources) {
      const original = await readFile(path.join(scratch, 'expected', path.basename(source)));
      assert.deepEqual(await readFile(source), original, 'Preview must not save original inputs');
    }
    await screenshot('standard-input-preview.png');
    await click('确认保存标准输入');
    await until(`document.body.innerText.includes('标准输入已保存，尚未生成运行工程')`);
    await click('重开来源');
    await until(`document.body.innerText.includes('标准输入已校验，尚未生成运行工程')`);
    assert.equal(
      await evaluate(`document.querySelector('input[aria-label="接收 CAN ID"]').value`),
      '1100',
    );
    assert.equal(
      await evaluate(`document.querySelector('input[aria-label="发送 CAN ID"]').value`),
      '1101',
    );
    assert.equal(
      await evaluate(`document.querySelector('input[aria-label="应用周期"]').value`),
      '20',
    );
    for (const name of ['types.arxml', 'services.arxml', 'unrelated.arxml']) {
      assert.deepEqual(
        await readFile(path.join(scratch, 'inputs', name)),
        await readFile(path.join(scratch, 'expected', name)),
      );
    }
    await input('接收 CAN ID', '1101');
    await click('应用并校验修改');
    await until(`document.body.innerText.includes('编辑被拒绝，原配置保持')`);
    assert(await evaluate(`document.body.innerText.includes('CAN_ID_CONFLICT')`));
    await evaluate(
      `document.querySelector('.integration-view [role="alert"]').scrollIntoView({ block: 'start' })`,
    );
    await screenshot('standard-input-rejection.png');
    await click('还原草稿');
    assert.equal(
      await evaluate(`document.querySelector('input[aria-label="接收 CAN ID"]').value`),
      '1100',
    );
    const delivery = path.join(scratch, 'ECU source');
    const moved = path.join(scratch, 'moved ECU source');
    const regenerated = path.join(scratch, 'regenerated ECU source');
    await click('生成与构建');
    await input('ECU 输出目录', delivery);
    await input('ECU 构建目录', path.join(scratch, 'ECU build'));
    await click('预览 ECU 交付');
    await until(`Boolean(document.querySelector('[aria-label="ECU 文件预览"]'))`);
    if (platform === 'macos') {
      const capabilities = await evaluate(
        "window.__TAURI_INTERNALS__.invoke('workbench_capabilities')",
      );
      assert.equal(capabilities.nativeExecution, false);
      for (const button of ['显式编译预检', '构建 ECU', '验证 ECU 主机行为']) {
        assert(
          await evaluate(
            `[...document.querySelectorAll('button')].find(node => node.textContent.trim() === ${JSON.stringify(button)}).disabled`,
          ),
        );
      }
      await assert.rejects(access(delivery), { code: 'ENOENT' });
      runtimeEvidence.nativeExecution = 'unsupported';
      runtimeEvidence.macosNativeRuntimeVerification = 'unverified';
      await screenshot('macos-source-workbench-preview.png');
      console.log(
        'macos_isolated_source_workbench_ipc PASS; native ECU execution unsupported; macOS native runtime unverified',
      );
      return;
    }
    await click('显式编译预检');
    await until(`document.body.innerText.includes('编译预检：passed；源码身份：')`);
    assert.equal(
      await evaluate(`document.querySelector('[data-stage="生成"]').textContent`),
      '未执行',
    );
    await screenshot('ecu-source-preview.png');
    async function selectTargetInUi(target) {
      await evaluate(`(() => {
        const select = document.querySelector('[aria-label="ECU 构建目标"]');
        if (select.disabled) throw new Error('Target selection is disabled');
        select.value = ${JSON.stringify(target)};
        select.dispatchEvent(new Event('change', {bubbles: true}));
      })()`);
    }
    const alternateTarget = resourceCaps.targets.find((target) => target !== resourceCaps.target);
    await selectTargetInUi(alternateTarget);
    await until(
      `[...document.querySelectorAll('button')].find(node => node.textContent.trim() === '显式编译预检').disabled`,
    );
    assert(await evaluate(`!document.querySelector('[aria-label="ECU 文件预览"]')`));
    assert(await evaluate(`!document.body.innerText.includes('编译预检：passed')`));
    await screenshot('target-invalidates-native-preflight.png');
    await selectTargetInUi(resourceCaps.target);
    await until(
      `![...document.querySelectorAll('button')].find(node => node.textContent.trim() === '显式编译预检').disabled`,
    );
    await click('预览 ECU 交付');
    await until(`Boolean(document.querySelector('[aria-label="ECU 文件预览"]'))`);
    await click('显式编译预检');
    await until(`document.body.innerText.includes('编译预检：passed；源码身份：')`);
    await click('确认生成 ECU');
    await until(
      `document.body.innerText.includes('已取消生成，输出目录未改动') && ![...document.querySelectorAll('button')].find(node => node.textContent.trim() === '确认生成 ECU').disabled`,
    );
    await writeFile(path.join(scratch, 'accept-generation-path.txt'), delivery);
    await click('确认生成 ECU');
    await until(`document.querySelector('[data-stage="生成"]').textContent === '已通过'`);
    await click('构建 ECU');
    await until(`document.querySelector('[data-stage="构建"]').textContent === '已通过'`);
    const originalBinary = await readFile(
      path.join(scratch, 'ECU build', `ecu_host_batch${platform === 'windows' ? '.exe' : ''}`),
    );
    await click('构建 ECU');
    await stage('构建', '失败');
    await stage('主机行为', '已失效');
    assert(
      await evaluate(
        `![...document.querySelectorAll('button')].find(node => node.textContent.trim() === '验证 ECU 主机行为' && !node.disabled)`,
      ),
    );
    assert(await evaluate(`!document.querySelector('[aria-label="ECU 行为日志"]')`));
    assert.deepEqual(
      await readFile(
        path.join(scratch, 'ECU build', `ecu_host_batch${platform === 'windows' ? '.exe' : ''}`),
      ),
      originalBinary,
    );
    await input('ECU 构建目录', path.join(scratch, 'ECU recovery build'));
    await click('构建 ECU');
    await stage('构建', '已通过');
    await click('验证 ECU 主机行为');
    await until(`document.querySelector('[data-stage="主机行为"]').textContent === '已通过'`);
    assert.equal(
      await evaluate(`document.querySelector('[data-stage="完整 SC1 工程等级复验"]').textContent`),
      '当前工程未验证',
    );
    assert.equal(
      await evaluate(`document.querySelector('[data-stage="实机"]').textContent`),
      '未验证',
    );
    await screenshot('ecu-host-behavior.png');
    await evaluate(
      `document.querySelector('[aria-label="ECU 行为日志"]').scrollIntoView({block: 'end'})`,
    );
    await screenshot('ecu-host-log.png');
    const runtime = path.join(delivery, 'src', 'Rte.c');
    const originalRuntime = await readFile(runtime);
    await writeFile(
      runtime,
      Buffer.concat([originalRuntime, Buffer.from('\n/* external edit */\n')]),
    );
    await click('验证 ECU 主机行为');
    await stage('主机行为', '失败');
    await screenshot('ecu-source-rejection.png');
    await writeFile(runtime, originalRuntime);
    await click('验证 ECU 主机行为');
    await stage('主机行为', '已通过');
    await click('标准输入');
    await until(
      `Boolean(document.querySelector('input[aria-label="应用周期"]') && !document.querySelector('input[aria-label="应用周期"]').disabled)`,
    );
    await input('应用周期', '21');
    await click('生成与构建');
    await stage('生成', '已失效');
    assert(
      await evaluate(
        `([...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='构建 ECU')).disabled`,
      ),
    );
    await click('标准输入');
    await click('还原草稿');
    await click('生成与构建');
    // Apply/save is a separate invalidation path from a draft or reimport.
    for (const name of ['校验', '生成', '构建', '主机行为']) {
      await stage(name, '已通过');
    }
    const savedInputs = await Promise.all(sources.map((source) => readFile(source)));
    const deliveredNames = (await readFile(path.join(delivery, 'files.list'), 'utf8'))
      .trim()
      .split(/\r?\n/);
    const protectedNames = [...deliveredNames, 'files.list', 'files.sha256'];
    const protectedBytes = await Promise.all(
      protectedNames.map((name) => readFile(path.join(delivery, name))),
    );
    const recoveryBinaryPath = path.join(
      scratch,
      'ECU recovery build',
      `ecu_host_batch${platform === 'windows' ? '.exe' : ''}`,
    );
    const recoveryBinary = await readFile(recoveryBinaryPath);
    await click('标准输入');
    await input('应用周期', '21');
    await click('应用并校验修改');
    await until(`document.body.innerText.includes('修改已通过同一计划校验，尚未保存')`);
    await click('生成与构建');
    await stage('校验', '需重新校验');
    for (const name of ['生成', '构建', '主机行为']) {
      await stage(name, '已失效');
    }
    async function assertOldDeliveryCleared() {
      assert(
        await evaluate(
          `['ECU 文件预览', 'ECU 构建日志', 'ECU 行为日志'].every(label => !document.querySelector('[aria-label="' + label + '"]'))`,
        ),
        'Old delivery preview and logs must be cleared',
      );
      for (const text of ['确认生成 ECU', '构建 ECU', '验证 ECU 主机行为']) {
        assert(
          await evaluate(
            `([...document.querySelectorAll('button')].find(button => button.textContent.trim() === ${JSON.stringify(text)})).disabled`,
          ),
          `Old delivery action must stay disabled: ${text}`,
        );
      }
    }
    await assertOldDeliveryCleared();
    // The real navigation label includes its dirty-workspace badge after apply.
    await click('标准输入未保存');
    await click('预览保存');
    await until(`Boolean(document.querySelector('[aria-label="标准保存预览"]'))`);
    for (let index = 0; index < sources.length; index += 1) {
      assert.deepEqual(await readFile(sources[index]), savedInputs[index]);
    }
    await click('确认保存标准输入');
    await until(`document.body.innerText.includes('标准输入已保存，尚未生成运行工程')`);
    const updatedInputs = await Promise.all(sources.map((source) => readFile(source)));
    assert(updatedInputs.some((bytes, index) => !bytes.equals(savedInputs[index])));
    await click('生成与构建');
    for (const name of ['校验', '生成', '构建', '主机行为']) {
      await stage(name, '未执行');
    }
    await assertOldDeliveryCleared();
    await screenshot('ecu-post-save-invalidated.png');
    for (let index = 0; index < protectedNames.length; index += 1) {
      assert.deepEqual(
        await readFile(path.join(delivery, protectedNames[index])),
        protectedBytes[index],
        'Editing/saving inputs must preserve the previous source package',
      );
    }
    assert.deepEqual(await readFile(recoveryBinaryPath), recoveryBinary);
    const postSave = path.join(scratch, 'post-save ECU source');
    await input('ECU 输出目录', postSave);
    await input('ECU 构建目录', path.join(scratch, 'post-save ECU build'));
    await assertOldDeliveryCleared();
    await click('预览 ECU 交付');
    await until(`Boolean(document.querySelector('[aria-label="ECU 文件预览"]'))`);
    await writeFile(path.join(scratch, 'accept-generation-path.txt'), postSave);
    await click('确认生成 ECU');
    await stage('生成', '已通过');
    const postSaveInputs = JSON.parse(
      await readFile(path.join(postSave, 'verification', 'inputs.json'), 'utf8'),
    );
    assert.equal(postSaveInputs.periodMs, 21);
    await click('构建 ECU');
    await stage('构建', '已通过');
    await click('验证 ECU 主机行为');
    await stage('主机行为', '已通过');
    await screenshot('ecu-post-save-verified.png');
    await rename(delivery, moved);
    await input('重导入 ECU 目录', moved);
    await click('重导入 ECU 交接包');
    await until(`document.body.innerText.includes('标准输入已校验，尚未生成运行工程')`);
    assert.equal(
      await evaluate(`document.querySelector('input[aria-label="接收 CAN ID"]').value`),
      '1100',
    );
    await click('生成与构建');
    for (const stage of ['校验', '生成', '构建', '主机行为']) {
      assert.equal(
        await evaluate(
          `document.querySelector('[data-stage=${JSON.stringify(stage)}]').textContent`,
        ),
        '未执行',
      );
    }
    await input('ECU 输出目录', regenerated);
    await click('预览 ECU 交付');
    await until(`Boolean(document.querySelector('[aria-label="ECU 文件预览"]'))`);
    await writeFile(path.join(scratch, 'accept-generation-path.txt'), regenerated);
    await click('确认生成 ECU');
    await until(`document.querySelector('[data-stage="生成"]').textContent === '已通过'`);
    const files = (await readFile(path.join(moved, 'files.list'), 'utf8')).trim().split(/\r?\n/);
    for (const file of [...files, 'files.list', 'files.sha256']) {
      assert.deepEqual(
        await readFile(path.join(regenerated, file)),
        await readFile(path.join(moved, file)),
        `Reimport/regenerate changed ${file}`,
      );
    }
    await click('标准输入');
    await until(`Boolean(document.querySelector('input[aria-label="应用周期"]'))`);
    const checkedPlan = await evaluate(`window.__NATIVE_VERIFY_INVOKE__('inspect_integration')`);
    assert.equal(checkedPlan.description.component.periodMs, 20);
    assert.deepEqual(
      checkedPlan.description.signals.map((signal) => signal.canId),
      [1100, 1101],
    );
    await input('应用周期', '21');
    await until(
      `Boolean([...document.querySelectorAll('button')].find(node => node.textContent.trim() === '应用并校验修改' && !node.disabled))`,
    );
    await evaluate(`document.querySelector('button[aria-label="切换项目"]').click()`);
    await until(`document.querySelector('button[aria-label="切换项目"]').disabled`);
    await until(`!document.querySelector('button[aria-label="切换项目"]').disabled`);
    assert.equal(
      await evaluate(`document.querySelector('input[aria-label="应用周期"]').value`),
      '21',
    );
    await click('还原草稿');
    await click('生成与构建');
    await click('预览 ECU 交付');
    await until(
      `Boolean(document.querySelector('[aria-label="ECU 文件预览"]')) && !document.querySelector('button[aria-label="切换项目"]').disabled`,
    );
    await evaluate(`document.querySelector('button[aria-label="切换项目"]').click()`);
    await until(`document.body.innerText.includes('配置项目')`);
    assert(
      await evaluate(`!document.querySelector('[aria-label="生成文件预览"]')`),
      'Closing an ECU project must discard its generation preview',
    );
    await click('导入 ARXML');
    await evaluate(`document.querySelector('details').open = true`);
    await input(
      'ARXML 来源路径',
      sources.find((source) => path.basename(source) === 'types.arxml'),
    );
    await until(`document.body.innerText.includes('导入 1 份文件')`);
    await click('导入 1 份文件');
    await until(`Boolean(document.querySelector('.project-nav'))`);
    await click('标准输入');
    await until(`document.body.innerText.includes('输入未通过，无法生成')`);
    assert(await evaluate(`document.body.innerText.includes('TARGET_NOT_UNIQUE')`));
    console.log(
      'Native standard-input and ECU preview/export/build/behavior/failure/move/reimport/regenerate passed',
    );
    const legacyDirectory = path.join(scratch, 'legacy', 'Alpha');
    const legacy = await evaluate(
      `window.__NATIVE_VERIFY_INVOKE__('open_handoff_project', {directory: ${JSON.stringify(legacyDirectory)}})`,
    );
    assert.equal(legacy.name, 'Alpha');
    assert.equal(Boolean(legacy.integrationCandidate), false);
    assert(legacy.frames.length > 0 && legacy.signals.length > 0);
    const unknown = path.join(legacyDirectory, 'handoff.json');
    const original = await readFile(unknown);
    const changed = JSON.parse(original.toString());
    changed.format = 'unknown-handoff-v99';
    await writeFile(unknown, JSON.stringify(changed));
    assert(
      await evaluate(
        `window.__NATIVE_VERIFY_INVOKE__('open_handoff_project', {directory: ${JSON.stringify(legacyDirectory)}}).then(()=>false,()=>true)`,
      ),
    );
    const retained = await evaluate(`window.__NATIVE_VERIFY_INVOKE__('workspace_view')`);
    assert.equal(retained.name, 'Alpha');
    await writeFile(unknown, original);
    console.log('Real native host-v1 dispatch and rejected-format workspace retention passed');
    const nativeTarget =
      platform === 'linux' ? 'linux-x64-controlled-v1' : 'windows-x64-controlled-v1';
    const otherTarget =
      platform === 'linux' ? 'windows-x64-controlled-v1' : 'linux-x64-controlled-v1';
    async function pendingBuild(name) {
      const output = path.join(scratch, name);
      await evaluate(`(() => {
      window.__NATIVE_VERIFY_PENDING_BUILD__ = window.__NATIVE_VERIFY_INVOKE__('build_project', {
        outputDirectory: ${JSON.stringify(legacyDirectory)},
        buildDirectory: ${JSON.stringify(output)}
      }).then(value => ({passed: true, value}), error => ({passed: false, error: String(error)}));
      return true;
    })()`);
      await until(
        `(async () => (await window.__TAURI_INTERNALS__.invoke('workbench_capabilities')).operation?.stage === 'build host')()`,
      );
      return output;
    }
    async function rejectedBuild(output) {
      const result = await evaluate('window.__NATIVE_VERIFY_PENDING_BUILD__');
      assert.equal(result.passed, false, 'Obsolete native build must not publish');
      await assert.rejects(access(output), { code: 'ENOENT' });
      const caps = await evaluate("window.__TAURI_INTERNALS__.invoke('workbench_capabilities')");
      assert.equal(caps.operation, null, 'Cancellation must acknowledge owner closure');
    }
    const switchedBuild = await pendingBuild('cancelled-target-build');
    await evaluate(
      `window.__NATIVE_VERIFY_INVOKE__('select_build_target', {target: ${JSON.stringify(otherTarget)}})`,
    );
    await rejectedBuild(switchedBuild);
    await evaluate(
      `window.__NATIVE_VERIFY_INVOKE__('select_build_target', {target: ${JSON.stringify(nativeTarget)}})`,
    );
    const editedBuild = await pendingBuild('cancelled-input-build');
    await evaluate(`window.__NATIVE_VERIFY_INVOKE__('add_frame', {
    name: 'ConcurrentEdit', id: 1408, dlc: 1, direction: 'tx', periodMs: 100, timeoutMs: null
  })`);
    await rejectedBuild(editedBuild);
    const editedView = await evaluate("window.__NATIVE_VERIFY_INVOKE__('workspace_view')");
    assert(editedView.frames.some((frame) => frame.name === 'ConcurrentEdit'));
    assert.deepEqual(
      await readFile(unknown),
      original,
      'Concurrent edit must not overwrite the old package',
    );
    console.log(
      'Real native target/input changes cancelled owned builds and rejected obsolete publication',
    );
  } catch (error) {
    console.error(String(error), await evaluate('document.body.innerText'));
    await screenshot('native-failure.png');
    throw error;
  } finally {
    runtimeEvidence.nativeTransportUnchanged = await evaluate(
      'window.__TAURI_INTERNALS__?.invoke === window.__NATIVE_VERIFY_TRANSPORT__',
    );
    await writeFile(
      path.join(scratch, 'native-runtime.json'),
      JSON.stringify(runtimeEvidence, null, 2) + '\n',
    );
    const events = await evaluate('window.__NATIVE_VERIFY_LOG__ ?? []');
    await writeFile(
      path.join(scratch, 'native-ipc.jsonl'),
      events.map((event) => JSON.stringify(event)).join('\n') + '\n',
    );
    assert.equal(
      runtimeEvidence.nativeTransportUnchanged,
      true,
      'Native verification must not replace the Tauri IPC transport',
    );
  }
}
