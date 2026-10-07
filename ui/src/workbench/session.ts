import type { Dispatch, RefObject, SetStateAction } from 'react';
import type { WorkbenchCapabilities } from '../types';
import type { WorkbenchState } from './state';

/** Internal shared state and command boundary; actions never own a second workspace. */
export interface WorkbenchSession {
  state: Readonly<WorkbenchState>;
  stateRef: RefObject<WorkbenchState>;
  capabilitiesRef: RefObject<WorkbenchCapabilities | null>;
  epoch: RefObject<number>;
  running: RefObject<boolean>;
  pendingGuard: RefObject<(() => void | Promise<void>) | null>;
  pendingReplacement: RefObject<(() => void | Promise<void>) | null>;
  changeConfirmation: RefObject<((applied: boolean) => void) | null>;
  field<K extends keyof WorkbenchState>(key: K): Dispatch<SetStateAction<WorkbenchState[K]>>;
  patchState(patch: Partial<WorkbenchState>): void;
  call<T>(command: string, payload?: Record<string, unknown>): Promise<T>;
}
