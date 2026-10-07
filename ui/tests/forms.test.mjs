import assert from 'node:assert/strict';
import { test } from 'vitest';

import { intInRange } from '../src/workbench/forms';

test('invalid form values are rejected before native commands are sent', () => {
  for (const value of ['', '1.5', 'NaN', '-1', '9']) {
    assert.throws(() => intInRange(value, 'DLC', 1, 8));
  }
  assert.equal(intInRange('8', 'DLC', 1, 8), 8);
});
