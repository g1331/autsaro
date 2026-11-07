import { appendFile } from 'node:fs/promises';

// Serialized into the page. The optional observer is diagnostic only; the
// expression and native IPC still execute through the original transport.
export function evaluateInPage(expression, id, done, endpoint) {
  function observe(stage) {
    const sample = {
      id,
      stage,
      at: Date.now(),
      timeOrigin: performance.timeOrigin,
      url: location.href,
      readyState: document.readyState,
      expression: expression.slice(0, 512),
      nativeCallbacks: window.__TAURI_INTERNALS__?.callbacks?.size,
    };
    window.__AUTOSAR_DESKTOP_EVALUATION__ = sample;
    try {
      if (endpoint && !navigator.sendBeacon(endpoint, JSON.stringify(sample)))
        sample.observerError = 'beacon not queued';
      window.__AUTOSAR_DESKTOP_OBSERVER__?.(sample);
    } catch (error) {
      sample.observerError = String(error);
    }
  }
  observe('entered');
  Promise.resolve()
    .then(() => (0, eval)(expression))
    .then((value) => {
      observe('resolved');
      // Keep the complete result opaque to WebDriver's recursive JSON clone.
      // Undefined has WebDriver's JSON-null representation, including properties.
      return JSON.stringify({ value }, (_key, item) => (item === undefined ? null : item));
    })
    .catch((error) => {
      observe('rejected');
      return JSON.stringify({ error: String(error) });
    })
    .then((encoded) => {
      observe('callback');
      done(encoded);
    });
}

export class WebDriver {
  constructor(base, log, evidence) {
    this.base = base;
    this.log = log;
    this.sequence = 0;
    this.evidence = evidence;
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
      evaluation: route.endsWith('/execute/async') ? body?.args?.[1] : undefined,
    };
    const record = async (fields) => {
      const value = { ...request, at: Date.now(), ...fields };
      await appendFile(this.log, JSON.stringify(value) + '\n');
      await this.evidence?.record(value);
    };
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
      if (this.evidence) {
        const text = await response.text();
        await this.evidence.record({
          ...request,
          phase: 'node-body',
          status: response.status,
          utf8ByteLength: Buffer.byteLength(text),
          codeUnits: text.length,
        });
        result = JSON.parse(text);
      } else result = await response.json();
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
