import { writeFile, readFile, rename } from 'node:fs/promises';
import path from 'node:path';
import { runScenario } from './desktop_scenario.mjs';

const [base, application, scratch, platform] = process.argv.slice(2);
let session;
async function request(method, route, body) {
  const response = await fetch(`${base}${route}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(300000),
  });
  const result = await response.json();
  if (!response.ok || result.value?.error) throw new Error(JSON.stringify(result));
  return result.value;
}
try {
  const created = await request('POST', '/session', {
    capabilities: { alwaysMatch: { 'tauri:options': { application } } },
  });
  session = created.sessionId;
  await request('POST', `/session/${session}/timeouts`, { script: 300000, pageLoad: 300000 });
  async function evaluate(expression) {
    const result = await request('POST', `/session/${session}/execute/async`, {
      script:
        'const done = arguments[arguments.length - 1]; Promise.resolve().then(() => (0, eval)(arguments[0])).then(value => done({value}), error => done({error: String(error)}));',
      args: [expression],
    });
    if (result.error) throw new Error(result.error);
    return result.value;
  }
  async function screenshot(name) {
    const image = await request('GET', `/session/${session}/screenshot`);
    await writeFile(path.join(scratch, name), Buffer.from(image, 'base64'));
  }
  async function resize(width, height) {
    const rectangle = await request('POST', `/session/${session}/window/rect`, { width, height });
    return { scope: 'native-webdriver-window', rectangle };
  }
  async function key(value, modifiers = {}) {
    const codes = {
      Escape: '\uE00C',
      Tab: '\uE004',
      ArrowDown: '\uE015',
      ArrowUp: '\uE013',
      ArrowLeft: '\uE012',
      ArrowRight: '\uE014',
      Enter: '\uE007',
    };
    const held = Object.entries({ alt: '\uE00A', ctrl: '\uE009', meta: '\uE03D', shift: '\uE008' })
      .filter(([name]) => modifiers[name])
      .map(([, code]) => code);
    const pressed = codes[value] ?? value;
    await request('POST', `/session/${session}/actions`, {
      actions: [
        {
          type: 'key',
          id: 'native-keyboard',
          actions: [
            ...held.map((code) => ({ type: 'keyDown', value: code })),
            { type: 'keyDown', value: pressed },
            { type: 'keyUp', value: pressed },
            ...held.reverse().map((code) => ({ type: 'keyUp', value: code })),
          ],
        },
      ],
    });
    await request('DELETE', `/session/${session}/actions`);
  }
  async function pointer(x, y) {
    await request('POST', `/session/${session}/actions`, {
      actions: [
        {
          type: 'pointer',
          id: 'native-mouse',
          parameters: { pointerType: 'mouse' },
          actions: [
            {
              type: 'pointerMove',
              duration: 0,
              origin: 'viewport',
              x: Math.round(x),
              y: Math.round(y),
            },
            { type: 'pointerDown', button: 0 },
            { type: 'pointerUp', button: 0 },
          ],
        },
      ],
    });
    await request('DELETE', `/session/${session}/actions`);
  }
  let mediaSequence = 0;
  async function media(scheme, reduceMotion = false) {
    if (platform !== 'linux')
      throw new Error('This native WebDriver host has no isolated system media controller');
    const value = { requestId: `media-${++mediaSequence}`, scheme, reduceMotion };
    const staging = path.join(scratch, 'native-media-request.next.json');
    await writeFile(staging, JSON.stringify(value));
    await rename(staging, path.join(scratch, 'native-media-request.json'));
    const deadline = Date.now() + 30000;
    while (Date.now() < deadline) {
      let result;
      try {
        result = JSON.parse(await readFile(path.join(scratch, 'native-media-result.json'), 'utf8'));
      } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
      if (result?.requestId === value.requestId) return result;
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
    throw new Error('Private XSettings did not acknowledge actual system media preferences');
  }
  await runScenario({ evaluate, screenshot, resize, key, pointer, media, scratch, platform });
} catch (error) {
  console.error(error);
  process.exitCode = 1;
} finally {
  if (session) await request('DELETE', `/session/${session}`);
}
