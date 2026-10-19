// @vitest-environment jsdom
import { afterEach, expect, test } from 'vitest';
import {
  catalogResultReady,
  projectObjectsReady,
  sampleDomQuery,
  standardInspectionReady,
} from '../../tests/desktop/support/readiness.mjs';

afterEach(() => document.body.replaceChildren());

test.each([
  ['start page before a workspace is published', '<div class="start-page"></div>', false],
  [
    'workspace tabs published while its object projection is pending',
    '<div class="document-tabs"></div><div class="tree-content"><p>Loading project objects</p></div>',
    false,
  ],
  [
    'group placeholder before any object has been published',
    '<div class="document-tabs"></div><div class="tree-content"><div data-tree-key="group:other"></div></div>',
    false,
  ],
  [
    'accepted object rows in the published workspace',
    '<div class="document-tabs"></div><div class="tree-content"><div data-object-id="first-object"></div></div>',
    true,
  ],
])('desktop object projection boundary: %s', (_scene, markup, ready) => {
  document.body.innerHTML = markup;
  const evaluate = new Function('document', `return (${projectObjectsReady.toString()})(document)`);
  expect(evaluate(document)).toBe(ready);
});

test('DOM query evidence preserves the result and same-turn operation state', async () => {
  document.body.innerHTML =
    '<span id="operation-status" role="status">Inspecting</span><div class="integration-fields"><label>Receive<input disabled value="1104"></label></div>';
  const serialized = new Function(
    'expression',
    `return (${sampleDomQuery.toString()})(expression)`,
  );
  const before = Date.now();
  const sample = await serialized(
    'document.querySelector(".integration-fields input:not(:disabled)")',
  );
  expect(sample).toMatchObject({
    value: false,
    uiOperation: 'Inspecting',
    statuses: ['Inspecting'],
    editors: [{ label: 'Receive', disabled: true, value: '1104' }],
  });
  expect(sample.at).toBeGreaterThanOrEqual(before);
  expect(sample.at).toBeLessThanOrEqual(Date.now());
  expect((await serialized('document.querySelector("input")')).value).toBe(true);
});

test('DOM evidence awaits async predicates without treating a Promise as ready', async () => {
  expect((await sampleDomQuery('Promise.resolve(false)')).value).toBe(false);
  expect((await sampleDomQuery('Promise.resolve(true)')).value).toBe(true);
  await expect(sampleDomQuery('Promise.reject(new Error("query failed"))')).rejects.toThrow(
    'query failed',
  );
});

test('synchronous query evidence is captured before a queued UI state change', async () => {
  document.body.innerHTML = '<span id="operation-status">Inspecting</span>';
  const sample = await sampleDomQuery(`(() => {
    queueMicrotask(() => { document.querySelector('#operation-status').textContent = 'Done'; });
    return false;
  })()`);
  expect(sample.uiOperation).toBe('Inspecting');
  expect(document.querySelector('#operation-status').textContent).toBe('Done');
});

test.each([
  ['before the automatic inspection starts, with an idle backend', '', false],
  [
    'while inspection has no published result',
    '<section aria-label="标准输入工作区"></section>',
    false,
  ],
  [
    'after publishing the result, before the UI releases busy state',
    '<section aria-label="标准输入工作区"><div class="integration-fields"><label>接收 CAN ID<input disabled></label></div></section>',
    false,
  ],
  [
    'after accepting the inspection reply and releasing busy state',
    '<section aria-label="标准输入工作区"><div class="integration-fields"><label>接收 CAN ID<input></label></div></section>',
    true,
  ],
])('desktop standard inspection boundary: %s', (_scene, markup, ready) => {
  document.body.innerHTML = markup;
  // The desktop helper serializes this predicate into the real WebView.
  const evaluate = new Function(
    'document',
    `return (${standardInspectionReady.toString()})(document)`,
  );
  expect(evaluate(document)).toBe(ready);
});

test.each([
  [
    'confirmation closed before a result is accepted',
    '<div class="catalog-row"></div><button>接纳扩展定义…</button>',
    0,
    false,
  ],
  [
    'new result present while the transaction is busy',
    '<button aria-disabled="true">接纳扩展定义…</button>',
    0,
    false,
  ],
  [
    'native disabled control before transaction completion',
    '<button disabled>接纳扩展定义…</button>',
    0,
    false,
  ],
  ['accepted removal result with controls released', '<button>接纳扩展定义…</button>', 0, true],
  [
    'accepted import result with controls released',
    '<div class="catalog-row"></div><button>接纳扩展定义…</button>',
    1,
    true,
  ],
])('desktop catalog result boundary: %s', (_scene, markup, count, ready) => {
  document.body.innerHTML = `<section>${markup}</section>`;
  const evaluate = new Function(
    'settings',
    'count',
    `return (${catalogResultReady.toString()})(settings, count)`,
  );
  expect(evaluate(document.querySelector('section'), count)).toBe(ready);
  expect(evaluate(null, count)).toBe(false);
});
