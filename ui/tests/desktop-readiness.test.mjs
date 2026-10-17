// @vitest-environment jsdom
import { afterEach, expect, test } from 'vitest';
import {
  catalogResultReady,
  standardInspectionReady,
} from '../../tests/desktop/support/readiness.mjs';

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
