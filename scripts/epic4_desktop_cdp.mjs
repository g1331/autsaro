import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const [url, scratch] = process.argv.slice(2);
const sources = JSON.parse(
  await readFile(path.join(scratch, "inputs.json"), "utf8"),
);
const ws = new WebSocket(url);
await new Promise((resolve, reject) => {
  ws.addEventListener("open", resolve, { once: true });
  ws.addEventListener("error", reject, { once: true });
});
let sequence = 0;
let rejectedDialogs = 0;
const pending = new Map();
ws.addEventListener("message", (event) => {
  const message = JSON.parse(event.data);
  if (message.method === "Page.javascriptDialogOpening") {
    rejectedDialogs += 1;
    void cdp("Page.handleJavaScriptDialog", { accept: false });
  }
  if (!message.id) return;
  const entry = pending.get(message.id);
  if (!entry) return;
  pending.delete(message.id);
  clearTimeout(entry.timer);
  if (message.error) entry.reject(new Error(JSON.stringify(message.error)));
  else entry.resolve(message.result);
});
function cdp(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`CDP timed out: ${method}`));
    }, 30000);
    pending.set(id, { resolve, reject, timer });
    ws.send(JSON.stringify({ id, method, params }));
  });
}
async function evaluate(expression) {
  const result = await cdp("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (result.exceptionDetails)
    throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function until(expression, expected = true) {
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    const result = await evaluate(expression);
    if (result === expected) return;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`UI condition did not become true: ${expression}`);
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
async function screenshot(name) {
  const metrics = await cdp("Page.getLayoutMetrics");
  const size = metrics.cssContentSize;
  const result = await cdp("Page.captureScreenshot", {
    format: "png",
    captureBeyondViewport: true,
    clip: { x: 0, y: 0, width: size.width, height: size.height, scale: 1 },
  });
  await writeFile(path.join(scratch, name), Buffer.from(result.data, "base64"));
}

try {
  await cdp("Page.enable");
  await until(
    `Boolean(window.__TAURI_INTERNALS__ && document.querySelector('button'))`,
  );
  // Use the real path-entry flow. Native Tauri transport remains untouched.
  await click("导入 ARXML");
  await evaluate(`document.querySelector('details').open = true`);
  await input("ARXML 来源路径", sources.join("\n"));
  await until(`document.body.innerText.includes('导入 7 份文件')`);
  await click("导入 7 份文件");
  await until(
    `document.body.innerText.includes('标准输入已校验，尚未生成运行工程')`,
  );
  const roles = await evaluate(
    `[...document.querySelectorAll('.integration-view tbody tr')].map(node => node.innerText)`,
  );
  assert.equal(roles.length, 7);
  assert(roles.some((row) => row.includes("ECU Extract")));
  assert(roles.some((row) => row.includes("BSW 实现")));
  await screenshot("standard-input-plan.png");
  await input("接收 CAN ID", "1100");
  await input("发送 CAN ID", "1101");
  await input("应用周期", "20");
  await click("应用并校验修改");
  await until(
    `document.body.innerText.includes('修改已通过同一计划校验，尚未保存')`,
  );
  await click("预览保存");
  await until(`document.body.innerText.includes('4 个文件将修改')`);
  await evaluate(
    `document.querySelector('[aria-label="标准保存预览"]').scrollIntoView({ block: 'end' })`,
  );
  for (const source of sources) {
    const original = await readFile(
      path.join("core/tests/fixtures/epic4/positive", path.basename(source)),
    );
    assert.deepEqual(
      await readFile(source),
      original,
      "Preview must not save original inputs",
    );
  }
  await screenshot("standard-input-preview.png");
  await click("确认保存标准输入");
  await until(
    `document.body.innerText.includes('标准输入已保存，尚未生成运行工程')`,
  );
  await click("重开来源");
  await until(`document.body.innerText.includes('已重开并校验保存的标准输入')`);
  assert.equal(
    await evaluate(
      `document.querySelector('input[aria-label="接收 CAN ID"]').value`,
    ),
    "1100",
  );
  assert.equal(
    await evaluate(
      `document.querySelector('input[aria-label="发送 CAN ID"]').value`,
    ),
    "1101",
  );
  assert.equal(
    await evaluate(
      `document.querySelector('input[aria-label="应用周期"]').value`,
    ),
    "20",
  );
  for (const name of ["types.arxml", "services.arxml", "unrelated.arxml"]) {
    assert.deepEqual(
      await readFile(path.join(scratch, "inputs", name)),
      await readFile(path.join("core/tests/fixtures/epic4/positive", name)),
    );
  }
  await input("接收 CAN ID", "1101");
  await click("应用并校验修改");
  await until(`document.body.innerText.includes('编辑被拒绝，原配置保持')`);
  assert(await evaluate(`document.body.innerText.includes('CAN_ID_CONFLICT')`));
  await evaluate(
    `document.querySelector('.integration-view [role="alert"]').scrollIntoView({ block: 'start' })`,
  );
  await screenshot("standard-input-rejection.png");
  await click("还原草稿");
  assert.equal(
    await evaluate(
      `document.querySelector('input[aria-label="接收 CAN ID"]').value`,
    ),
    "1100",
  );
  const checkedPlan = await evaluate(
    `window.__TAURI_INTERNALS__.invoke('inspect_integration')`,
  );
  assert.equal(checkedPlan.description.component.periodMs, 20);
  assert.deepEqual(
    checkedPlan.description.signals.map((signal) => signal.canId),
    [1100, 1101],
  );
  await input("应用周期", "21");
  await until(
    `Boolean([...document.querySelectorAll('button')].find(node => node.textContent.trim() === '应用并校验修改' && !node.disabled))`,
  );
  await evaluate(
    `document.querySelector('button[aria-label="切换项目"]').click()`,
  );
  await until(
    `document.querySelector('button[aria-label="切换项目"]').disabled`,
  );
  await until(
    `!document.querySelector('button[aria-label="切换项目"]').disabled`,
  );
  assert.equal(
    await evaluate(
      `document.querySelector('input[aria-label="应用周期"]').value`,
    ),
    "21",
  );
  await click("还原草稿");
  await evaluate(
    `document.querySelector('button[aria-label="切换项目"]').click()`,
  );
  await until(`document.body.innerText.includes('配置项目')`);
  await click("导入 ARXML");
  await evaluate(`document.querySelector('details').open = true`);
  await input(
    "ARXML 来源路径",
    sources.find((source) => path.basename(source) === "types.arxml"),
  );
  await until(`document.body.innerText.includes('导入 1 份文件')`);
  await click("导入 1 份文件");
  await until(`Boolean(document.querySelector('.project-nav'))`);
  await click("标准输入");
  await until(`document.body.innerText.includes('输入未通过，无法生成')`);
  assert(
    await evaluate(`document.body.innerText.includes('TARGET_NOT_UNIQUE')`),
  );
  await writeFile(
    path.join(scratch, "native-ipc.json"),
    JSON.stringify(
      {
        passed: true,
        roles,
        verifiedFlows: [
          "import-paths",
          "inspect",
          "edit",
          "preview",
          "save",
          "reopen",
          "reject-duplicate-id",
          "cancel-unapplied-draft-discard",
          "missing-extract-blocked",
        ],
        channels: [1100, 1101],
        periodMs: 20,
        changedFiles: 4,
        untouchedFiles: ["types.arxml", "services.arxml", "unrelated.arxml"],
        screenshotFiles: [
          "standard-input-plan.png",
          "standard-input-preview.png",
          "standard-input-rejection.png",
        ],
        sourceSelection: "real UI path entry; unmodified native IPC",
      },
      null,
      2,
    ),
  );
  console.log(
    "Native standard-input edit/preview/save/reopen/rejection passed",
  );
} catch (error) {
  await writeFile(
    path.join(scratch, "native-failure.json"),
    JSON.stringify(
      {
        error: String(error),
        body: await evaluate("document.body.innerText"),
      },
      null,
      2,
    ),
  );
  await screenshot("native-failure.png");
  throw error;
} finally {
  for (const entry of pending.values()) clearTimeout(entry.timer);
  ws.close();
}
