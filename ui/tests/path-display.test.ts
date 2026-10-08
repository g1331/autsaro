import { expect, test } from 'vitest';
import { displayPath } from '../src/pathDisplay';

const longLeaf = `${'目录 with spaces\\'.repeat(24)}Application.arxml`;

test.each([
  [
    'uppercase disk',
    String.raw`\\?\C:\project\Application.arxml`,
    String.raw`C:\project\Application.arxml`,
  ],
  [
    'lowercase disk',
    String.raw`\\?\z:\project\Application.arxml`,
    String.raw`z:\project\Application.arxml`,
  ],
  ['disk root', '\\\\?\\C:\\', 'C:\\'],
  [
    'Chinese and spaces',
    String.raw`\\?\D:\工作 目录\应用 文件.arxml`,
    String.raw`D:\工作 目录\应用 文件.arxml`,
  ],
  ['UNC share root', String.raw`\\?\UNC\server\share`, String.raw`\\server\share`],
  ['UNC root separator', '\\\\?\\UNC\\server\\share\\', '\\\\server\\share\\'],
  [
    'UNC child',
    String.raw`\\?\UNC\server\share\project\Application.arxml`,
    String.raw`\\server\share\project\Application.arxml`,
  ],
  [
    'lowercase UNC',
    String.raw`\\?\unc\server\share\file.arxml`,
    String.raw`\\server\share\file.arxml`,
  ],
  [
    'mixed-case UNC',
    String.raw`\\?\UnC\服务器\共享 目录\文件.arxml`,
    String.raw`\\服务器\共享 目录\文件.arxml`,
  ],
  [
    'disk dot segments remain literal',
    String.raw`\\?\C:\project\.\..\file.arxml`,
    String.raw`C:\project\.\..\file.arxml`,
  ],
  [
    'UNC dot segments remain literal',
    String.raw`\\?\UNC\server\share\.\..\file.arxml`,
    String.raw`\\server\share\.\..\file.arxml`,
  ],
  ['path beyond MAX_PATH', `\\\\?\\C:\\${longLeaf}`, `C:\\${longLeaf}`],
  [
    'UNC path beyond MAX_PATH',
    `\\\\?\\UNC\\server\\share\\${longLeaf}`,
    `\\\\server\\share\\${longLeaf}`,
  ],
])('formats %s without rewriting the remainder', (_name, input, expected) => {
  expect(displayPath(input)).toBe(expected);
});

test.each([
  ['empty', ''],
  ['ordinary disk', String.raw`C:\工作目录\Application.arxml`],
  ['ordinary UNC', String.raw`\\server\share\Application.arxml`],
  ['Linux absolute', '/home/user/project/Application.arxml'],
  ['relative Windows', String.raw`src\Application.arxml`],
  ['relative POSIX', 'src/Application.arxml'],
  ['AUTOSAR object identity', '/Package/Component/Port'],
  ['disk-relative', String.raw`C:Application.arxml`],
  ['verbatim disk-relative', String.raw`\\?\C:Application.arxml`],
  ['bare verbatim drive', String.raw`\\?\C:`],
  ['forward-slash drive', String.raw`\\?\C:/Application.arxml`],
  ['numeric drive', String.raw`\\?\1:\Application.arxml`],
  ['non-ASCII drive', String.raw`\\?\中:\Application.arxml`],
  ['multi-character drive', String.raw`\\?\AB:\Application.arxml`],
  ['bare namespace prefix', '\\\\?\\'],
  ['UNC marker without separator', String.raw`\\?\UNC`],
  ['UNC marker only', '\\\\?\\UNC\\'],
  ['UNC server only', String.raw`\\?\UNC\server`],
  ['UNC empty share', '\\\\?\\UNC\\server\\'],
  ['UNC empty server', String.raw`\\?\UNC\\share\Application.arxml`],
  ['UNC doubled separator before share', String.raw`\\?\UNC\server\\Application.arxml`],
  ['near-UNC marker', String.raw`\\?\UNCx\server\share\Application.arxml`],
  ['volume namespace', String.raw`\\?\Volume{1234-5678}\Application.arxml`],
  ['device namespace', String.raw`\\.\C:\Application.arxml`],
  ['device pipe', String.raw`\\.\pipe\autosar`],
  ['verbatim device name', String.raw`\\?\GLOBALROOT\Device\HarddiskVolume1`],
  ['prefix in evidence', String.raw`compiler: \\?\C:\目录\file.c`],
  ['leading whitespace', String.raw` \\?\C:\目录\file.c`],
  ['trailing whitespace', String.raw`C:\目录\file.c `],
  ['unrelated question mark', String.raw`src\file?.arxml`],
])('preserves %s exactly', (_name, input) => {
  expect(displayPath(input)).toBe(input);
});

test('long-path fixtures exercise strings longer than the Windows legacy limit', () => {
  expect(`C:\\${longLeaf}`.length).toBeGreaterThan(260);
  expect(displayPath(`\\\\?\\C:\\${longLeaf}`)).toBe(`C:\\${longLeaf}`);
});

test('does not trim trailing whitespace or normalize slashes after a recognized prefix', () => {
  expect(displayPath(String.raw`\\?\C:\目录/文件.c `)).toBe(String.raw`C:\目录/文件.c `);
  expect(displayPath(String.raw`\\?\UNC\server\share\目录/文件.c `)).toBe(
    String.raw`\\server\share\目录/文件.c `,
  );
});
