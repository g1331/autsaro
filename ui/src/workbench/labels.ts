import { i18n, translate } from '../i18n';
import type { WorkbenchState } from './state';

/** Product wording for an internal identifier; unknown values stay visible verbatim. */
function known(prefix: string, value: string): string {
  const key = `${prefix}.${value}`;
  return i18n.exists(key) ? translate(key) : value;
}

export const profileLabel = (profile: string) => known('shell.profile', profile);
export const targetLabel = (target: string) => known('shell.target', target);
export const commandLabel = (command: string) => known('shell.ipc', command);
export const fileStatusLabel = (status: string) => known('shell.fileStatus', status);
export const ownerLabel = (owner: string) => known('shell.owner', owner);

/** A direct ARXML import is marked so it cannot be mistaken for a saved project. */
export function workspaceTitle(state: Pick<WorkbenchState, 'workspace' | 'memberProjectPath'>) {
  const name = state.workspace?.name ?? '';
  return state.memberProjectPath ? name : translate('shell.app.unsavedImport', { name });
}
