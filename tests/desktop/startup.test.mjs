import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { dialogNamed, openWorkbench } from './support/workbench.mjs';

const { check, initialCapabilities, evaluate, capabilities, scratch } =
  await openWorkbench('startup');

check('retired-validation-archive-settings-ignored', async () => {
  // The harness seeded settings.json with the retired xsdArchive/modArchive fields.
  assert.equal(initialCapabilities.settingsError, null, 'Retired archive settings must still load');
  assert.equal(
    initialCapabilities.settingsNotice?.key,
    'backend.workbench.retired_validation_archives_ignored',
  );
  const caps = await capabilities();
  assert(!caps.settingsNotice, 'Retired archive settings are dropped by the first settings save');
  const saved = JSON.parse(
    await readFile(path.join(scratch, 'app-config', 'settings.json'), 'utf8'),
  );
  assert(
    !('xsdArchive' in saved) && !('modArchive' in saved),
    'Settings are saved to the private configuration',
  );
});

check('packaged-builtin-boundary', async () => {
  const requests = (await readFile(path.join(scratch, 'webdriver.jsonl'), 'utf8'))
    .trim()
    .split('\n')
    .map(JSON.parse);
  const startupProbes = requests.filter((request) =>
    request.script?.includes('Boolean(window.__TAURI_INTERNALS__'),
  );
  assert(startupProbes.length >= 2, 'Initial and reloaded application readiness were observed');
  assert(
    startupProbes.every((request) => request.route === '/session/execute/sync'),
    'Navigation readiness must not depend on an async callback in the departing document',
  );
  const location = await evaluate(
    `({protocol: location.protocol, hostname: location.hostname, port: location.port})`,
  );
  assert(
    (location.protocol === 'tauri:' && location.hostname === 'localhost') ||
      (['http:', 'https:'].includes(location.protocol) && location.hostname === 'tauri.localhost'),
    `The frontend is packaged into the application: ${JSON.stringify(location)}`,
  );
  assert.equal(location.port, '');
  const caps = await capabilities();
  assert.equal(caps.ruleError, null);
  assert(caps.ruleSetIdentity?.sha256 && caps.ruleCoverage.length > 0);
  assert(caps.toolError, 'Execution tools must not leak into the configuration process');
  assert(!(await evaluate(`Boolean(${dialogNamed('设置')})`)), 'No resource setup gate');
});
