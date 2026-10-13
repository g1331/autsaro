import assert from 'node:assert/strict';
import { access, mkdir, readFile, writeFile, rename, cp, readdir, stat } from 'node:fs/promises';
import path from 'node:path';
import { openWorkbench, json } from './support/workbench.mjs';

const {
  check,
  key,
  scratch,
  invoke,
  projection,
  refreshSurface,
  sourceBytes,
  unchanged,
  refuse,
  save,
  create,
  openMember,
} = await openWorkbench('projects');

let standard;
check('member-portability-and-save-as', async () => {
  standard = await create('GatewayReference');
  const before = await projection();
  const moved = path.join(scratch, 'projects', 'MovedGateway');
  await rename(standard.directory, moved);
  const reopened = await openMember(path.join(moved, 'workbench-project.json'));
  assert.notEqual(reopened.workspaceEpoch, before.workspaceEpoch);
  assert.equal(reopened.sources.length, 7);
  await refuse(
    'read_project_source',
    { sourceId: before.sources[0].sourceId },
    'Reopened workspace rejects previous source identity',
  );
  const original = await sourceBytes(reopened);
  const saveAsDirectory = path.join(scratch, 'projects', 'CopiedGateway');
  const preview = await invoke('preview_save_as_project', {
    directory: saveAsDirectory,
    name: 'CopiedGateway',
  });
  await assert.rejects(access(saveAsDirectory), { code: 'ENOENT' });
  await invoke('save_as_project_previewed', { preview });
  await unchanged(original);
  const copied = await projection();
  assert.equal(copied.sources.length, 7);
  assert.deepEqual(copied.acceptedExtensionDefinitions, []);
  await refreshSurface(copied.projectPath);
  await refuse(
    'preview_project_creation',
    { directory: moved, name: 'Occupied', templateId: 'standard-ecu' },
    'Occupied destination cannot be overwritten',
  );
  const manifestPath = copied.projectPath;
  const manifestBytes = await readFile(manifestPath);
  const manifest = JSON.parse(manifestBytes);
  const manifestFailures = {
    parentEscape: (value) => {
      value.inputs[0].path = '../outside.arxml';
    },
    absolutePath: (value) => {
      value.inputs[0].path = path.join(moved, manifest.inputs[0].path);
    },
    duplicatePath: (value) => {
      value.inputs.push({ ...value.inputs[0] });
    },
    unsupportedVersion: (value) => {
      value.formatVersion = 999;
    },
  };
  for (const [kind, mutate] of Object.entries(manifestFailures)) {
    const candidate = structuredClone(manifest);
    mutate(candidate);
    const bytes = Buffer.from(json(candidate));
    await writeFile(manifestPath, bytes);
    await refuse('open_member_project', { path: manifestPath }, `Manifest ${kind} refusal`);
    assert.deepEqual(await readFile(manifestPath), bytes, 'Refused manifest stays byte-identical');
    await unchanged(original);
  }
  await writeFile(manifestPath, manifestBytes);
});
check('retired-identities-refused-unchanged', async () => {
  const before = await projection();
  async function snapshot(directory) {
    const files = new Map();
    for (const name of await readdir(directory, { recursive: true })) {
      const file = path.join(directory, name);
      if (!(await stat(file)).isDirectory()) files.set(name, await readFile(file));
    }
    return files;
  }
  async function refusalKey(command, payload) {
    let refused = null;
    try {
      await invoke(command, payload);
    } catch (error) {
      refused = JSON.parse(error.message);
    }
    assert(refused, `${command}: a retired identity must be refused`);
    return refused.key;
  }
  // A project whose manifest names the removed frame/signal host profile.
  const retiredProject = path.join(scratch, 'projects', 'RetiredHostProject');
  await cp(path.dirname(before.projectPath), retiredProject, { recursive: true });
  const retiredManifest = path.join(retiredProject, 'workbench-project.json');
  const manifest = JSON.parse(await readFile(retiredManifest, 'utf8'));
  manifest.profileHint = 'host-can-v1';
  await writeFile(retiredManifest, json(manifest));
  const projectBytes = await snapshot(retiredProject);
  assert.equal(
    await refusalKey('open_member_project', { path: retiredManifest }),
    'backend.arxml.project.retired_profile',
  );
  assert.deepEqual(await snapshot(retiredProject), projectBytes, 'Retired project unchanged');
  // Handoff directories of the retired host, single-component and earlier workbench formats,
  // which kept handoff.json at the package root.
  for (const format of [
    'autosar-host-handoff-v1',
    'autosar-ecu-handoff-v1',
    'autosar-workbench-handoff-v2',
    'autosar-workbench-handoff-v3',
  ]) {
    const directory = path.join(scratch, 'deliveries', `retired-${format}`);
    await mkdir(directory);
    await writeFile(path.join(directory, 'handoff.json'), json({ format }));
    const bytes = await snapshot(directory);
    const destination = path.join(scratch, 'projects', `Retired-${format}`);
    assert.equal(
      await refusalKey('open_handoff_project', {
        directory,
        newWorkspaceDirectory: destination,
      }),
      'backend.project.retired_handoff_format',
    );
    assert.deepEqual(await snapshot(directory), bytes, `Retired ${format} unchanged`);
    await assert.rejects(access(destination), { code: 'ENOENT' });
  }
  assert.deepEqual(await projection(), before, 'Refusals must keep the open workspace');
});
