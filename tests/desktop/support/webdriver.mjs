import { appendFile } from 'node:fs/promises';

export class WebDriver {
  constructor(base, log) {
    this.base = base;
    this.log = log;
  }
  async refresh(sessionRoute) {
    return this.request('POST', `${sessionRoute}/refresh`, {});
  }
  async request(method, route, body) {
    const started = performance.now();
    const response = await fetch(`${this.base}${route}`, {
      method,
      headers: { 'Content-Type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(300000),
    });
    const result = await response.json();
    await appendFile(
      this.log,
      JSON.stringify({
        at: Date.now(),
        durationMs: performance.now() - started,
        method,
        route: route.replace(/session\/[^/]+/, 'session'),
        script: route.endsWith('/execute/sync') ? body?.args?.[0] : body?.args?.[0]?.slice(0, 240),
        status: response.status,
        error: result.value?.error ? result.value : undefined,
        probe: body?.args?.[0]?.includes('Boolean(window.__TAURI_INTERNALS__')
          ? result.value
          : undefined,
      }) + '\n',
    );
    if (!response.ok || result.value?.error)
      throw new Error(`WebDriver ${method} ${route}: ${JSON.stringify(result)}`);
    return result.value;
  }
}
