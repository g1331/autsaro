import assert from 'node:assert/strict';
import { access, readFile } from 'node:fs/promises';
import path from 'node:path';

export async function assertUnavailableSourceCheckout(scratch, sourceCheckout, platform) {
  assert.equal(typeof sourceCheckout, 'string');
  assert(path.isAbsolute(sourceCheckout));
  const receipt = JSON.parse(
    await readFile(path.join(scratch, 'source-checkout-boundary.json'), 'utf8'),
  );
  assert.equal(receipt.sourceCheckout, sourceCheckout);
  assert.equal(receipt.sourceCheckoutAvailable, false);
  assert.equal(receipt.platform, platform);
  assert.equal(receipt.callerUid, typeof process.getuid === 'function' ? process.getuid() : null);
  let expectedCode = 'ENOENT';
  if (receipt.availabilityReason === 'caller_inaccessible') {
    assert.equal(receipt.statErrno, 13);
    expectedCode = 'EACCES';
  } else {
    assert.equal(receipt.availabilityReason, 'not_found');
    assert.equal(receipt.statErrno, null);
  }
  try {
    await access(sourceCheckout);
  } catch (error) {
    if (error.code !== expectedCode) throw error;
    if (expectedCode === 'EACCES' && process.platform === 'linux') {
      assert.equal(error.errno, -13);
    }
    return receipt;
  }
  assert.fail(`Build checkout is accessible to the scenario caller: ${sourceCheckout}`);
}

export async function runScenario(options) {
  const { runBuiltinScenario } = await import('./desktop_builtin_scenario.mjs');
  return runBuiltinScenario(options);
}
