import { initialDeliveryState } from '../features/delivery/state';
import type { DeliveryState } from '../features/delivery/state';
import { initialEditingState } from '../features/editing/state';
import type { EditingState } from '../features/editing/state';
import { initialProjectState, projectionBaseline } from '../features/project/state';
import type { ProjectState } from '../features/project/state';
import { initialSettingsState } from '../features/settings/state';
import type { SettingsState } from '../features/settings/state';
import { initialShellState } from './shellState';
import type { ShellState } from './shellState';

export type WorkbenchState = ProjectState &
  EditingState &
  DeliveryState &
  SettingsState &
  ShellState;
export type WorkbenchPatch = Partial<WorkbenchState>;

export function initialDomains() {
  return {
    project: initialProjectState(),
    editing: initialEditingState(),
    delivery: initialDeliveryState(),
    settings: initialSettingsState(),
    shell: initialShellState(),
  };
}

type Domains = ReturnType<typeof initialDomains>;

export function workbenchSnapshot({
  project,
  editing,
  delivery,
  settings,
  shell,
}: Domains): WorkbenchState {
  return { ...project, ...editing, ...delivery, ...settings, ...shell };
}

function acceptedPatch(patch: WorkbenchPatch): WorkbenchPatch {
  return {
    ...patch,
    ...projectionBaseline(patch.projection),
    ...(patch.projection === null ? { rejection: null } : {}),
  };
}

function patchDomain<S extends object>(state: S, patch: Partial<S>): S {
  let next = state;
  for (const key of Object.keys(patch) as (keyof S)[]) {
    if (!Object.hasOwn(state, key) || Object.is(state[key], patch[key])) continue;
    if (next === state) next = { ...state };
    next[key] = patch[key] as S[typeof key];
  }
  return next;
}

/** All domains accept the same transaction before any consumer sees the new snapshot. */
export function reduceWorkbench(state: Domains, patch: WorkbenchPatch): Domains {
  const accepted = acceptedPatch(patch);
  return {
    project: patchDomain(state.project, accepted),
    editing: patchDomain(state.editing, accepted),
    delivery: patchDomain(state.delivery, accepted),
    settings: patchDomain(state.settings, accepted),
    shell: patchDomain(state.shell, accepted),
  };
}
