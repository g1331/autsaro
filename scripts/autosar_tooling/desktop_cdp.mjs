import { runScenario } from './desktop_scenario.mjs';
import { writeFile } from 'node:fs/promises';
import path from 'node:path';

const [url, scratch] = process.argv.slice(2);
const ws = new WebSocket(url);
await new Promise((resolve, reject) => {
  ws.addEventListener('open', resolve, { once: true });
  ws.addEventListener('error', reject, { once: true });
});
let sequence = 0;
let rejectedDialogs = 0;
const pending = new Map();
ws.addEventListener('message', (event) => {
  const message = JSON.parse(event.data);
  if (message.method === 'Page.javascriptDialogOpening') {
    rejectedDialogs += 1;
    void cdp('Page.handleJavaScriptDialog', { accept: false });
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
    }, 300000);
    pending.set(id, { resolve, reject, timer });
    ws.send(JSON.stringify({ id, method, params }));
  });
}
async function evaluate(expression) {
  const result = await cdp('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function screenshot(name) {
  const metrics = await cdp('Page.getLayoutMetrics');
  const size = metrics.cssContentSize;
  const result = await cdp('Page.captureScreenshot', {
    format: 'png',
    captureBeyondViewport: true,
    clip: { x: 0, y: 0, width: size.width, height: size.height, scale: 1 },
  });
  await writeFile(path.join(scratch, name), Buffer.from(result.data, 'base64'));
}
await cdp('Page.enable');
try {
  await runScenario({ evaluate, screenshot, scratch, platform: 'windows' });
} finally {
  for (const entry of pending.values()) clearTimeout(entry.timer);
  ws.close();
}
