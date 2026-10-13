import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import test from 'node:test';
import { descendants, processTree } from './support/driver.mjs';

test('reused parent PID does not adopt older session processes', () => {
  // Session processes whose short-lived smss.exe (8040) exited before the
  // driver tree reused its PID; their ParentProcessId still says 8040.
  const listed = [
    { pid: 8052, parent: 8040, name: 'csrss.exe', identity: '134030000000000000' },
    { pid: 8100, parent: 8040, name: 'winlogon.exe', identity: '134030000000000001' },
    { pid: 8148, parent: 8100, name: 'fontdrvhost.exe', identity: '134030000000000002' },
    { pid: 6096, parent: 8100, name: 'dwm.exe', identity: '134030000000000003' },
    { pid: 1000, parent: 900, name: 'tauri-driver.exe', identity: '134040000000000000' },
    { pid: 8040, parent: 1000, name: 'msedgedriver.exe', identity: '134040000000000001' },
    { pid: 2000, parent: 8040, name: 'autosar-config-desktop.exe', identity: '134040000000000001' },
    { pid: 2100, parent: 2000, name: 'msedgewebview2.exe', identity: '134040000000000002' },
    { pid: 2200, parent: 2000, name: 'unknown-start.exe', identity: 'null' },
  ];
  assert.deepEqual(
    descendants(listed, 1000).map((item) => item.pid),
    [1000, 8040, 2000, 2100],
  );
});

test('a real child process is attributed to its live parent', async () => {
  const child = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 60000)'], {
    stdio: 'ignore',
  });
  try {
    await once(child, 'spawn');
    const tree = await processTree(process.pid);
    assert(
      tree.some((item) => item.pid === child.pid),
      'a residual child below the root must remain visible to the close() check',
    );
  } finally {
    child.kill();
  }
});
