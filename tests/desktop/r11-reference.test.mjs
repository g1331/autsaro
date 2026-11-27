import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { cp, mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { openWorkbench } from './support/workbench.mjs';

const {
  check,
  scratch,
  openMember,
  invoke,
  menu,
  field,
  evaluate,
  key,
  click,
  until,
  refreshSurface,
  save,
  exportSource,
} = await openWorkbench('r11-reference');

for (const [name, initial] of [
  ['two-bus', 17],
  ['four-bus', -33],
]) {
  check(`${name}: edit, reopen, and consume the sealed project`, async () => {
    const fixtures = process.env.AUTOSAR_R11_PROJECTS;
    assert(fixtures, 'Set AUTOSAR_R11_PROJECTS to the staged core reference projects');
    const project = path.join(scratch, name);
    await cp(path.join(fixtures, name), project, { recursive: true });
    const manifest = path.join(project, 'workbench-project.json');
    await openMember(manifest);
    await menu('标准输入');
    const option = field('commissioned').expression;
    await until(`Boolean((${option})?.matches(':enabled'))`, 'configured calibration choice ready');
    const before = await readFile(manifest);

    await assert.rejects(
      invoke('preview_variant_selection', {
        selection: { calibration: ['invalid'] },
      }),
      (error) => JSON.stringify(error.ipc ?? error).includes('TYPE_CONFLICT'),
    );
    assert.deepEqual(await readFile(manifest), before);
    assert.deepEqual((await invoke('inspect_integration')).layers.groups.calibration.selected, [
      'platform',
    ]);

    await evaluate(`(${option}).focus()`);
    await key(' ');
    await click('比较变体');
    await until(
      `[...document.querySelectorAll('.integration-view p')].some(node => node.textContent.includes('ComSignalInitValue') && node.textContent.includes('0 → ${initial}'))`,
      'commissioned calibration changes the transmitted initial value',
    );
    await click('应用更改');
    await until(
      `document.querySelector('.integration-view input[type=radio]:checked')?.parentElement.textContent.trim() === 'commissioned' && [...document.querySelectorAll('.integration-view button')].some(node => node.textContent.trim() === '预览保存' && !node.disabled)`,
      'variant application completed',
    );
    await save();
    await refreshSurface(manifest);
    const reopened = await invoke('inspect_integration');
    assert.deepEqual(reopened.layers.groups.calibration.selected, ['commissioned']);
    assert.equal(
      reopened.description.signals.find((signal) => signal.port.endsWith('/TxValue')).initialValue,
      initial,
    );
    const delivery = await exportSource(name);
    const receiver = path.join(scratch, `receiver-${name}`);
    await mkdir(receiver);
    const run = promisify(execFile);
    const python = process.env.AUTOSAR_PYTHON;
    assert(python && path.isAbsolute(python), 'Declare an absolute AUTOSAR_PYTHON');
    const oracle = new URL('./support/r11_receiver.py', import.meta.url);
    try {
      const result = await run(
        python,
        ['-I', '-S', fileURLToPath(oracle), name, delivery.directory, receiver],
        { cwd: receiver, timeout: 240000 },
      );
      await writeFile(path.join(receiver, 'receiver.log'), result.stdout);
      assert.match(
        result.stdout,
        new RegExp(`^${name}: sealed build and HostBatch vectors passed$`, 'm'),
      );
    } catch (error) {
      await writeFile(
        path.join(receiver, 'receiver.log'),
        `${error.stdout ?? ''}${error.stderr ?? ''}`,
      );
      throw error;
    }
  });
}
