import { appendFile } from 'node:fs/promises';

export class WebDriver {
  constructor(base, log) {
    this.base = base;
    this.log = log;
    this.sequence = 0;
  }
  async refresh(sessionRoute) {
    return this.request('POST', `${sessionRoute}/refresh`, {});
  }
  async request(method, route, body) {
    if (this.failure) throw this.failure;
    const started = performance.now();
    const request = {
      id: ++this.sequence,
      method,
      route: route.replace(/session\/[^/]+/, 'session'),
      script: body?.args?.[0],
    };
    const record = (fields) =>
      appendFile(this.log, JSON.stringify({ ...request, at: Date.now(), ...fields }) + '\n');
    await record({ phase: 'started' });
    let response;
    let result;
    try {
      response = await fetch(`${this.base}${route}`, {
        method,
        headers: { 'Content-Type': 'application/json' },
        body: body === undefined ? undefined : JSON.stringify(body),
        signal: AbortSignal.timeout(300000),
      });
      result = await response.json();
    } catch (error) {
      // An aborted transport cannot cancel the command already running in WebKit.
      this.failure = error;
      await record({
        phase: 'failed',
        durationMs: performance.now() - started,
        status: response?.status,
        error: result?.value?.error ? result.value : String(error),
      });
      throw error;
    }
    if (!response.ok || result.value?.error) {
      await record({
        phase: 'failed',
        durationMs: performance.now() - started,
        status: response.status,
        error: result.value,
      });
      throw new Error(`WebDriver ${method} ${route}: ${JSON.stringify(result)}`);
    }
    await record({
      phase: 'completed',
      durationMs: performance.now() - started,
      status: response.status,
      probe: body?.args?.[0]?.includes('Boolean(window.__TAURI_INTERNALS__')
        ? result.value
        : undefined,
    });
    return result.value;
  }
}
