import { runScenario } from './desktop_scenario.mjs';
import { writeFile, readFile, rename } from 'node:fs/promises';
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
let windowSequence = 0;
async function resize(width, height) {
  const request = { requestId: `window-${++windowSequence}`, width, height };
  const staging = path.join(scratch, 'native-window-request.next.json');
  await writeFile(staging, JSON.stringify(request));
  await rename(staging, path.join(scratch, 'native-window-request.json'));
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    let result;
    try {
      result = JSON.parse(await readFile(path.join(scratch, 'native-window-result.json'), 'utf8'));
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
    if (result?.requestId === request.requestId) return result;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error('Owned native window did not acknowledge its real physical resize');
}
async function key(value, modifiers = {}) {
  const keys = {
    Escape: 'Escape',
    Tab: 'Tab',
    ArrowDown: 'ArrowDown',
    ArrowUp: 'ArrowUp',
    ArrowLeft: 'ArrowLeft',
    ArrowRight: 'ArrowRight',
    Enter: 'Enter',
  };
  const mask =
    (modifiers.alt ? 1 : 0) |
    (modifiers.ctrl ? 2 : 0) |
    (modifiers.meta ? 4 : 0) |
    (modifiers.shift ? 8 : 0);
  const code =
    keys[value] ??
    (value.length === 1
      ? /^\d$/.test(value)
        ? `Digit${value}`
        : `Key${value.toUpperCase()}`
      : value);
  const params = { key: value, code, modifiers: mask };
  await cdp('Input.dispatchKeyEvent', { ...params, type: 'keyDown' });
  await cdp('Input.dispatchKeyEvent', { ...params, type: 'keyUp' });
}
async function pointer(x, y) {
  await cdp('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y });
  await cdp('Input.dispatchMouseEvent', {
    type: 'mousePressed',
    x,
    y,
    button: 'left',
    clickCount: 1,
  });
  await cdp('Input.dispatchMouseEvent', {
    type: 'mouseReleased',
    x,
    y,
    button: 'left',
    clickCount: 1,
  });
}
async function media(scheme, reduceMotion = false) {
  await cdp('Emulation.setEmulatedMedia', {
    features: [
      { name: 'prefers-color-scheme', value: scheme },
      { name: 'prefers-reduced-motion', value: reduceMotion ? 'reduce' : 'no-preference' },
    ],
  });
  return { scheme, reduceMotion, scope: 'native-webview-CDP-media' };
}
await cdp('Page.enable');
try {
  await runScenario({
    evaluate,
    screenshot,
    resize,
    key,
    pointer,
    media,
    scratch,
    platform: 'windows',
  });
} finally {
  try {
    await cdp('Emulation.setEmulatedMedia', { features: [] });
  } finally {
    for (const entry of pending.values()) clearTimeout(entry.timer);
    ws.close();
  }
}
