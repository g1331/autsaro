import { writeFile } from 'node:fs/promises';
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
  await runScenario({ evaluate, screenshot, scratch, platform });
} finally {
  if (session) await request('DELETE', `/session/${session}`);
}
