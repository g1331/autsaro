import { initialDomains, workbenchSnapshot } from '../../src/app/state';

export function initialState() {
  return workbenchSnapshot(initialDomains());
}
