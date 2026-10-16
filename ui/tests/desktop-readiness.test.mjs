// @vitest-environment jsdom
import { afterEach, expect, test } from 'vitest';
import { standardInspectionReady } from '../../tests/desktop/support/readiness.mjs';

afterEach(() => document.body.replaceChildren());

test.each([
  ['before the automatic inspection starts, with an idle backend', '', false],
  [
    'while inspection has no published result',
    '<section aria-label="标准输入工作区"></section>',
    false,
  ],
  [
    'after publishing the result, before the UI releases busy state',
    '<section aria-label="标准输入工作区"><input aria-label="接收 CAN ID" disabled></section>',
    false,
  ],
  [
    'after accepting the inspection reply and releasing busy state',
    '<section aria-label="标准输入工作区"><input aria-label="接收 CAN ID"></section>',
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
