import { workbenchCommands } from '../../src/app/commands';

/** A mock controller with the command state `useWorkbench` derives from it. */
export function withCommands(controller) {
  const base = { actionReason: () => '', executionReason: '', ...controller };
  return { ...base, commands: workbenchCommands(base) };
}
