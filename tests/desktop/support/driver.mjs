// One isolated native application session per test file, driven through the
// official tauri-driver WebDriver intermediary. Linux isolates itself on a
// private Xvfb display and D-Bus; Windows relies on a dedicated runner or VM.
import assert from 'node:assert/strict';
import { execFile, spawn, spawnSync } from 'node:child_process';
import { access, mkdir, mkdtemp, readFile, readdir, readlink, writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { promisify } from 'node:util';

const run = promisify(execFile);
const sleep = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));
const platform = { linux: 'linux', win32: 'windows' }[process.platform];
// Let Ctrl-C and CI cancellation run the 'exit' cleanup registered by launch().
for (const [signal, code] of [
  ['SIGINT', 130],
  ['SIGTERM', 143],
])
  process.once(signal, () => process.exit(code));

// Execution tools are declared to the application only through its settings UI.
export const EXECUTION_VARIABLES = [
  'AUTOSAR_CC',
  'AUTOSAR_OBJDUMP',
  'AUTOSAR_NM',
  'AUTOSAR_GIT',
  'AUTOSAR_PYTHON',
  'AUTOSAR_XSD_ARCHIVE',
  'AUTOSAR_MOD_ARCHIVE',
];
const DEVELOPMENT_VARIABLES = [
  'VIRTUAL_ENV',
  'PYTHONPATH',
  'PYTHONHOME',
  'NODE_OPTIONS',
  'ECU_OWNER_SOCKET',
  'ECU_OWNER_TOKEN',
  'ECU_OWNER_SCOPE',
  'GTK_THEME',
  'WAYLAND_DISPLAY',
];

async function tool(name) {
  for (const directory of (process.env.PATH ?? '').split(path.delimiter).filter(Boolean))
    for (const suffix of platform === 'windows' ? ['.exe', ''] : ['']) {
      const candidate = path.join(directory, name + suffix);
      try {
        await access(candidate);
        return candidate;
      } catch {
        // Keep searching PATH.
      }
    }
  throw new Error(`Required desktop test tool is missing from PATH: ${name}`);
}

async function freePort() {
  const server = createServer();
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const { port } = server.address();
  await new Promise((resolve) => server.close(resolve));
  return port;
}

async function until(probe, description, timeout = 30000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const value = await probe();
    if (value) return value;
    await sleep(100);
  }
  throw new Error(`Timed out waiting for ${description}`);
}

// Start a child and resolve with the first stdout line it prints.
function started(command, args, options, description) {
  const child = spawn(command, args, {
    ...options,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let stderr = '';
  child.stderr.on('data', (chunk) => (stderr += chunk));
  const line = new Promise((resolve, reject) => {
    let stdout = '';
    const timer = setTimeout(
      () => reject(new Error(`${description} did not start: ${stderr}`)),
      15000,
    );
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
      if (stdout.includes('\n')) {
        clearTimeout(timer);
        resolve(stdout.split('\n')[0].trim());
      }
    });
    child.on('exit', (code) => {
      clearTimeout(timer);
      reject(new Error(`${description} exited with ${code}: ${stderr}`));
    });
  });
  return { child, line };
}

function stopped(child) {
  if (child.exitCode !== null || child.signalCode !== null) return Promise.resolve();
  return new Promise((resolve) => {
    child.once('exit', resolve);
    child.kill();
  });
}

function windowsGuard() {
  const dedicated =
    process.env.GITHUB_ACTIONS === 'true' ||
    os.userInfo().username === 'WDAGUtilityAccount' ||
    process.env.AUTOSAR_DESKTOP_DEDICATED_SESSION === '1';
  if (!dedicated)
    throw new Error(
      'Windows desktop tests run only in a dedicated session (GitHub-hosted runner, Windows Sandbox, ' +
        'or a VM with AUTOSAR_DESKTOP_DEDICATED_SESSION=1); they never use your active desktop.',
    );
}

async function powershell(script) {
  const { stdout } = await run(
    'powershell.exe',
    ['-NoProfile', '-NonInteractive', '-Command', script],
    { maxBuffer: 64 * 1024 * 1024, windowsHide: true },
  );
  return stdout;
}

async function processes() {
  const listed = [];
  if (platform === 'windows') {
    const items = JSON.parse(
      await powershell(
        '[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-CimInstance Win32_Process | ' +
          'Select-Object ProcessId,ParentProcessId,Name,CommandLine,WorkingSetSize,PrivatePageCount,' +
          '@{n="Created";e={if ($_.CreationDate) { $_.CreationDate.ToFileTimeUtc() }}} | ConvertTo-Json -Compress',
      ),
    );
    for (const item of items)
      listed.push({
        pid: item.ProcessId,
        parent: item.ParentProcessId,
        name: item.Name,
        command: item.CommandLine,
        identity: String(item.Created),
        workingSet: item.WorkingSetSize,
        privateBytes: item.PrivatePageCount,
      });
  } else {
    for (const entry of await readdir('/proc')) {
      if (!/^\d+$/.test(entry)) continue;
      try {
        const status = await readFile(`/proc/${entry}/status`, 'utf8');
        const fields = Object.fromEntries(
          status
            .split('\n')
            .map((line) => line.split(/:\s*/, 2))
            .filter((pair) => pair.length === 2),
        );
        const stat = (await readFile(`/proc/${entry}/stat`, 'utf8')).split(') ')[1].split(' ');
        listed.push({
          pid: Number(entry),
          parent: Number(fields.PPid),
          name: fields.Name,
          command: (await readFile(`/proc/${entry}/cmdline`, 'utf8')).replaceAll('\0', ' ').trim(),
          identity: stat[19],
          VmRSS: Number.parseInt(fields.VmRSS ?? '0', 10) * 1024,
          VmHWM: Number.parseInt(fields.VmHWM ?? '0', 10) * 1024,
        });
      } catch {
        // The process exited during the scan.
      }
    }
  }
  return listed;
}

// Processes below the driver: native WebDriver, application and WebView.
async function processTree(root) {
  const listed = await processes();
  const owned = new Set([root]);
  for (let grew = true; grew;) {
    grew = false;
    for (const item of listed)
      if (owned.has(item.parent) && !owned.has(item.pid)) {
        owned.add(item.pid);
        grew = true;
      }
  }
  return listed.filter((item) => owned.has(item.pid));
}

async function privateDisplay(scratch, environment) {
  const xvfb = started(
    await tool('Xvfb'),
    ['-displayfd', '1', '-screen', '0', '1440x1200x24', '-nolisten', 'tcp', '-nolisten', 'unix'],
    { env: environment },
    'Private Xvfb',
  );
  environment.DISPLAY = `:${await xvfb.line}`;
  // Only owner connections and no service activation: nothing reaches the
  // user's session bus, and nothing outlives this daemon.
  const bus = path.join(scratch, 'bus');
  await mkdir(bus, { mode: 0o700 });
  await writeFile(
    path.join(bus, 'bus.conf'),
    `<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:path=${path.join(bus, 'bus.sock')}</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow send_destination="*"/>
    <allow receive_sender="*"/>
    <allow own="*"/>
    <deny send_destination="org.freedesktop.DBus" send_interface="org.freedesktop.DBus" send_member="StartServiceByName"/>
  </policy>
</busconfig>
`,
  );
  const dbus = started(
    await tool('dbus-daemon'),
    ['--nofork', '--nosyslog', `--config-file=${path.join(bus, 'bus.conf')}`, '--print-address=1'],
    { env: environment },
    'Private D-Bus',
  );
  environment.DBUS_SESSION_BUS_ADDRESS = await dbus.line;
  const settings = path.join(scratch, 'xsettings.conf');
  const media = async (scheme, reduceMotion) => {
    await writeFile(
      settings,
      `Net/ThemeName "${scheme === 'dark' ? 'Adwaita-dark' : 'Adwaita'}"\nGtk/EnableAnimations ${reduceMotion ? 0 : 1}\n`,
    );
  };
  await media('light', false);
  const xsettings = spawn(await tool('xsettingsd'), ['--config', settings], {
    env: environment,
    stdio: 'ignore',
  });
  return {
    children: [xsettings, dbus.child, xvfb.child],
    async media(scheme, reduceMotion) {
      await media(scheme, reduceMotion);
      xsettings.kill('SIGHUP');
      return {
        scheme,
        reduceMotion,
        scope: 'private-xvfb-xsettings',
        display: environment.DISPLAY,
      };
    },
  };
}

async function linuxChoose(environment, application, kind, selected) {
  const xdotool = await tool('xdotool');
  const xclip = await tool('xclip');
  const options = { env: environment };
  const windows = async () => {
    try {
      const { stdout } = await run(
        xdotool,
        ['search', '--onlyvisible', '--name', '^(Open|Save|Select|Choose|打开|保存|选择)'],
        options,
      );
      return stdout.split(/\s+/).filter(Boolean);
    } catch (error) {
      if (error.code === 1) return [];
      throw error;
    }
  };
  const window = await until(
    async () => ((await windows()).length === 1 ? (await windows())[0] : null),
    'one native chooser on the private display',
  );
  const owner = Number((await run(xdotool, ['getwindowpid', window], options)).stdout.trim());
  assert.equal(
    owner,
    await application(),
    'Native chooser must belong to the application under test',
  );
  const focused = await run(xdotool, ['windowfocus', '--sync', window, 'getwindowfocus'], options);
  assert.equal(focused.stdout.trim(), window, 'Private chooser must receive X11 focus');
  // Paste the URI into GtkFileChooser's file list; location-entry completion
  // only navigates and does not accept a folder.
  const uri = new URL(`file://${selected}`).href;
  const clipboard = spawn(xclip, ['-quiet', '-selection', 'clipboard', '-i'], {
    env: environment,
    stdio: ['pipe', 'ignore', 'ignore'],
  });
  clipboard.stdin.end(uri);
  try {
    await until(async () => {
      try {
        return (await run(xclip, ['-selection', 'clipboard', '-out'], options)).stdout === uri;
      } catch {
        return false;
      }
    }, 'private chooser URI clipboard');
    for (const action of [
      ['key', '--clearmodifiers', 'alt+Home'],
      ['sleep', '0.2'],
      ['key', '--clearmodifiers', 'ctrl+v'],
      ['sleep', '0.2'],
      ['key', '--clearmodifiers', 'Return'],
    ])
      await run(xdotool, action, options);
    let accepted = false;
    await until(
      async () => {
        if (!(await windows()).includes(window)) return true;
        // A populated folder is first navigated into; then use its Open button.
        if (kind === 'directory' && !accepted) {
          await run(xdotool, ['sleep', '0.2', 'key', '--clearmodifiers', 'alt+o'], options);
          accepted = true;
        }
        return false;
      },
      'native chooser to accept the selection',
      10000,
    );
  } finally {
    await stopped(clipboard);
  }
}

const WINDOWS_CHOOSER = `
param([int]$ProcessId, [string]$Kind, [string]$Selected)
$ErrorActionPreference = 'Stop'
# UIA does not expose the chooser's Win32 name field, so drive it by control ID.
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class Chooser {
  delegate bool Visit(IntPtr window, IntPtr data);
  [DllImport("user32.dll")] static extern bool EnumWindows(Visit visit, IntPtr data);
  [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr parent, Visit visit, IntPtr data);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr window, StringBuilder name, int size);
  [DllImport("user32.dll")] static extern int GetDlgCtrlID(IntPtr window);
  [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr window);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr window);
  [DllImport("user32.dll")] static extern bool IsWindowEnabled(IntPtr window);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern IntPtr SendMessage(IntPtr window, uint message, IntPtr wParam, string lParam);
  [DllImport("user32.dll")] static extern bool PostMessage(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
  static string ClassOf(IntPtr window) { var name = new StringBuilder(256); GetClassName(window, name, name.Capacity); return name.ToString(); }
  public static List<IntPtr> Dialogs(uint process) {
    var dialogs = new List<IntPtr>();
    EnumWindows((window, data) => {
      uint owner; GetWindowThreadProcessId(window, out owner);
      if (owner == process && IsWindowVisible(window) && ClassOf(window) == "#32770") dialogs.Add(window);
      return true;
    }, IntPtr.Zero);
    return dialogs;
  }
  public static bool Accept(IntPtr dialog, int[] identifiers, string path) {
    var edits = new List<IntPtr>();
    IntPtr accept = IntPtr.Zero;
    EnumChildWindows(dialog, (window, data) => {
      string name = ClassOf(window);
      if (name == "Edit" && IsWindowVisible(window) && IsWindowEnabled(window)) edits.Add(window);
      if (name == "Button" && GetDlgCtrlID(window) == 1) accept = window;
      return true;
    }, IntPtr.Zero);
    if (accept == IntPtr.Zero) return false;
    foreach (int identifier in identifiers)
      foreach (IntPtr edit in edits)
        if (GetDlgCtrlID(edit) == identifier) {
          SendMessage(edit, 0x000C, IntPtr.Zero, path); // WM_SETTEXT
          PostMessage(accept, 0x00F5, IntPtr.Zero, IntPtr.Zero); // BM_CLICK
          return true;
        }
    return false;
  }
}
'@
# File pickers name their field 1148; the folder picker uses 1152.
$identifiers = if ($Kind -eq 'directory') { [int[]](1152, 1148) } else { [int[]](1148) }
function Shown {
  Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
  $texts = foreach ($candidate in [Chooser]::Dialogs($ProcessId)) {
    $names = [Windows.Automation.AutomationElement]::FromHandle($candidate).FindAll([Windows.Automation.TreeScope]::Descendants, [Windows.Automation.Condition]::TrueCondition) | ForEach-Object { $_.Current.Name } | Where-Object { $_ }
    "[$($names -join ' | ')]"
  }
  $texts -join ' '
}
$dialog = [IntPtr]::Zero
$deadline = (Get-Date).AddSeconds(30)
while ($dialog -eq [IntPtr]::Zero -and (Get-Date) -lt $deadline) {
  foreach ($candidate in [Chooser]::Dialogs($ProcessId)) {
    if ([Chooser]::Accept($candidate, $identifiers, $Selected)) { $dialog = $candidate; break }
  }
  if ($dialog -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 100 }
}
if ($dialog -eq [IntPtr]::Zero) {
  throw "No native chooser with a name field belongs to the application under test: $(Shown)"
}
$deadline = (Get-Date).AddSeconds(10)
while ((Get-Date) -lt $deadline) {
  if (-not [Chooser]::IsWindow($dialog)) {
    # Any other dialog would be an unexpected modal the scenario never answered.
    if ([Chooser]::Dialogs($ProcessId).Count) { throw "Native dialog left open after the chooser: $(Shown)" }
    exit 0
  }
  Start-Sleep -Milliseconds 100
}
throw "Native chooser stayed open after accepting the selection: $(Shown)"
`;

class WebDriver {
  constructor(base) {
    this.base = base;
  }
  async request(method, route, body) {
    const response = await fetch(`${this.base}${route}`, {
      method,
      headers: { 'Content-Type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(300000),
    });
    const result = await response.json();
    if (!response.ok || result.value?.error)
      throw new Error(`WebDriver ${method} ${route}: ${JSON.stringify(result)}`);
    return result.value;
  }
}

/**
 * Launch the application named by AUTOSAR_DESKTOP_APP in a private session.
 * Results (screenshots, IPC log) go to AUTOSAR_DESKTOP_RESULTS or the system
 * temporary directory.
 */
export async function launch(name) {
  if (!platform)
    throw new Error(`tauri-driver supports Linux and Windows desktops, not ${process.platform}`);
  if (platform === 'windows') windowsGuard();
  const application = process.env.AUTOSAR_DESKTOP_APP;
  assert(
    application && path.isAbsolute(application),
    'Set AUTOSAR_DESKTOP_APP to the absolute path of a native-webdriver build',
  );
  await access(application);
  const results = process.env.AUTOSAR_DESKTOP_RESULTS ?? os.tmpdir();
  await mkdir(results, { recursive: true });
  const scratch = await mkdtemp(path.join(path.resolve(results), `${name}-`));
  for (const directory of [
    'app-config',
    'app-local',
    'app-home',
    'app-temp',
    'app-path',
    'config',
    'data',
    'projects',
    'deliveries',
  ])
    await mkdir(path.join(scratch, directory));
  // Settings written by the retired XSD/MOD validation mode must still load.
  await writeFile(
    path.join(scratch, 'app-config', 'settings.json'),
    JSON.stringify({
      xsdArchive: 'retired-xsd.zip',
      modArchive: 'retired-mod.zip',
    }),
  );
  const environment = { ...process.env };
  for (const variable of Object.keys(environment))
    if (
      EXECUTION_VARIABLES.includes(variable) ||
      DEVELOPMENT_VARIABLES.includes(variable) ||
      /^(UV|CARGO|RUSTUP|NPM_CONFIG)_/i.test(variable)
    )
      delete environment[variable];
  Object.assign(environment, {
    AUTOSAR_CONFIG_DIR: path.join(scratch, 'app-config'),
    APPDATA: path.join(scratch, 'app-config'),
    LOCALAPPDATA: path.join(scratch, 'app-local'),
    XDG_CONFIG_HOME: path.join(scratch, 'config'),
    XDG_DATA_HOME: path.join(scratch, 'data'),
    HOME: path.join(scratch, 'app-home'),
    USERPROFILE: path.join(scratch, 'app-home'),
    TEMP: path.join(scratch, 'app-temp'),
    TMP: path.join(scratch, 'app-temp'),
    TMPDIR: path.join(scratch, 'app-temp'),
    PATH: path.join(scratch, 'app-path'),
    NO_AT_BRIDGE: '1',
  });
  // Accepted only by the native-webdriver feature: delay real backend work so
  // pending operations and cancellation are observable.
  for (const [action, delay] of [
    ['SOURCE_READ', 500],
    ['SNAPSHOT_BUILD', 250],
    ['SCHEMA_CHECK', 100],
    ['SOURCE_SCAN', 250],
    ['DEFINITION_READ', 100],
  ])
    environment[`AUTOSAR_VERIFY_DELAY_${action}_MS`] = String(delay);

  const children = [];
  let display;
  let driverProcess;
  let abandon;
  let session;
  let client;
  try {
    let nativeDriver;
    if (platform === 'linux') {
      environment.GDK_BACKEND = 'x11';
      environment.WEBKIT_DISABLE_DMABUF_RENDERER = '1';
      display = await privateDisplay(scratch, environment);
      children.push(...display.children);
      nativeDriver = await tool('WebKitWebDriver');
    } else {
      nativeDriver = await tool('msedgedriver');
    }
    const port = await freePort();
    driverProcess = spawn(
      await tool('tauri-driver'),
      [
        '--port',
        String(port),
        '--native-port',
        String(await freePort()),
        '--native-driver',
        nativeDriver,
      ],
      {
        env: environment,
        detached: platform === 'linux',
        stdio: ['ignore', 'ignore', 'pipe'],
        windowsHide: true,
      },
    );
    // An interrupted run never reaches close(); end the whole session with it.
    abandon = () => {
      if (platform === 'linux') {
        try {
          process.kill(-driverProcess.pid, 'SIGKILL');
        } catch {
          // Already gone.
        }
      } else spawnSync('taskkill', ['/PID', String(driverProcess.pid), '/T', '/F']);
      for (const child of children) child.kill('SIGKILL');
    };
    process.once('exit', abandon);
    let driverLog = '';
    driverProcess.stderr.on('data', (chunk) => (driverLog += chunk));
    client = new WebDriver(`http://127.0.0.1:${port}`);
    await until(async () => {
      try {
        return (await fetch(`${client.base}/status`)).ok;
      } catch {
        if (driverProcess.exitCode !== null) throw new Error(`tauri-driver exited: ${driverLog}`);
        return false;
      }
    }, 'tauri-driver');
    session = (
      await client.request('POST', '/session', {
        capabilities: { alwaysMatch: { 'tauri:options': { application } } },
      })
    ).sessionId;
    await client.request('POST', `/session/${session}/timeouts`, {
      script: 300000,
      pageLoad: 300000,
    });
  } catch (error) {
    if (abandon) {
      process.off('exit', abandon);
      abandon();
    }
    await Promise.all([driverProcess, ...children].filter(Boolean).map(stopped));
    throw error;
  }
  const route = (suffix) => `/session/${session}${suffix}`;
  const applicationProcess = async () => {
    const binary = path.resolve(application).toLowerCase();
    const tree = await processTree(driverProcess.pid);
    for (const item of tree) {
      if (platform === 'linux') {
        try {
          if ((await readlink(`/proc/${item.pid}/exe`)).toLowerCase() === binary) return item.pid;
        } catch {
          // Exited or not ours.
        }
      } else if (item.name.toLowerCase() === path.basename(binary)) return item.pid;
    }
    throw new Error('Application process is not running below tauri-driver');
  };
  const pressed = {
    Escape: '',
    Tab: '',
    ArrowDown: '',
    ArrowUp: '',
    ArrowLeft: '',
    ArrowRight: '',
    Enter: '',
  };
  const held = { alt: '', ctrl: '', meta: '', shift: '' };
  return {
    platform,
    scratch,
    async evaluate(expression) {
      const result = await client.request('POST', route('/execute/async'), {
        script:
          'const done = arguments[arguments.length - 1]; Promise.resolve().then(() => (0, eval)(arguments[0])).then(value => done({value}), error => done({error: String(error)}));',
        args: [expression],
      });
      if (result.error) throw new Error(result.error);
      return result.value;
    },
    async screenshot(file) {
      const image = await client.request('GET', route('/screenshot'));
      await writeFile(path.join(scratch, file), Buffer.from(image, 'base64'));
    },
    async resize(width, height) {
      return client.request('POST', route('/window/rect'), { width, height });
    },
    async key(value, modifiers = {}) {
      const codes = Object.entries(held)
        .filter(([modifier]) => modifiers[modifier])
        .map(([, code]) => code);
      const code = pressed[value] ?? value;
      await client.request('POST', route('/actions'), {
        actions: [
          {
            type: 'key',
            id: 'keyboard',
            actions: [
              ...codes.map((item) => ({ type: 'keyDown', value: item })),
              { type: 'keyDown', value: code },
              { type: 'keyUp', value: code },
              ...codes.reverse().map((item) => ({ type: 'keyUp', value: item })),
            ],
          },
        ],
      });
      await client.request('DELETE', route('/actions'));
    },
    async pointer(x, y) {
      await client.request('POST', route('/actions'), {
        actions: [
          {
            type: 'pointer',
            id: 'mouse',
            parameters: { pointerType: 'mouse' },
            actions: [
              {
                type: 'pointerMove',
                duration: 0,
                origin: 'viewport',
                x: Math.round(x),
                y: Math.round(y),
              },
              { type: 'pointerDown', button: 0 },
              { type: 'pointerUp', button: 0 },
            ],
          },
        ],
      });
      await client.request('DELETE', route('/actions'));
    },
    // Linux changes the real GTK setting on the private display; WebView2 has
    // no per-session system setting, so Windows emulates the media features.
    async media(scheme, reduceMotion = false) {
      if (platform === 'linux') return display.media(scheme, reduceMotion);
      await client.request('POST', route('/ms/cdp/execute'), {
        cmd: 'Emulation.setEmulatedMedia',
        params: {
          features: [
            { name: 'prefers-color-scheme', value: scheme },
            {
              name: 'prefers-reduced-motion',
              value: reduceMotion ? 'reduce' : 'no-preference',
            },
          ],
        },
      });
      return { scheme, reduceMotion, scope: 'webview2-cdp-emulated-media' };
    },
    async choose(kind, selected) {
      if (platform === 'linux') return linuxChoose(environment, applicationProcess, kind, selected);
      const script = path.join(scratch, 'choose.ps1');
      await writeFile(script, WINDOWS_CHOOSER);
      await run(
        'powershell.exe',
        [
          '-NoProfile',
          '-NonInteractive',
          '-ExecutionPolicy',
          'Bypass',
          '-File',
          script,
          '-ProcessId',
          String(await applicationProcess()),
          '-Kind',
          kind,
          '-Selected',
          selected,
        ],
        { windowsHide: true },
      );
    },
    async clipboard() {
      const text =
        platform === 'linux'
          ? (
              await run(await tool('xclip'), ['-selection', 'clipboard', '-out'], {
                env: environment,
                maxBuffer: 64 * 1024 * 1024,
              })
            ).stdout
          : await powershell(
              '[Console]::OutputEncoding=[Text.Encoding]::UTF8; [Console]::Out.Write((Get-Clipboard -Raw))',
            );
      return text.replaceAll('\r\n', '\n');
    },
    async memory() {
      return processTree(driverProcess.pid);
    },
    // Close the session, end the driver tree and prove nothing survived.
    async close() {
      const tree = await processTree(driverProcess.pid);
      try {
        await client.request('DELETE', route(''));
      } finally {
        if (platform === 'linux') {
          try {
            process.kill(-driverProcess.pid, 'SIGTERM');
          } catch (error) {
            if (error.code !== 'ESRCH') throw error;
          }
        } else {
          await run('taskkill', ['/PID', String(driverProcess.pid), '/T', '/F']).catch(
            () => undefined,
          );
        }
        await Promise.all([stopped(driverProcess), ...children.map(stopped)]);
      }
      const survivors = [];
      await until(
        async () => {
          const current = await processes();
          survivors.length = 0;
          const remaining = new Map(current.map((item) => [item.pid, item.identity]));
          for (const item of tree)
            if (remaining.get(item.pid) === item.identity) survivors.push(item);
          return survivors.length === 0;
        },
        'the application process tree to exit',
        15000,
      ).catch(() => {
        throw new Error(
          `Desktop session left processes running: ${JSON.stringify(survivors, null, 2)}`,
        );
      });
      process.off('exit', abandon);
      await writeFile(path.join(scratch, 'process-tree.json'), JSON.stringify(tree, null, 2));
    },
  };
}
