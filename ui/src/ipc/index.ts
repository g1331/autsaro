import { invoke as nativeInvoke } from '@tauri-apps/api/core';
import type { CommandMap } from './bindings';

export type Command = keyof CommandMap;
export type WorkbenchCommand = {
  [C in Command]: CommandMap[C]['result'] extends { inputFingerprint: string } ? C : never;
}[Command];
export type Payload<C extends WorkbenchCommand> = Omit<CommandMap[C]['args'], 'fingerprint'>;
export type CallArgs<C extends WorkbenchCommand> =
  Payload<C> extends Record<string, never>
    ? [payload?: Record<string, never>]
    : [payload: Payload<C>];
export type Value<C extends WorkbenchCommand> = CommandMap[C]['result'] extends { value: infer T }
  ? T
  : never;
export type WorkbenchCall = <C extends WorkbenchCommand>(
  command: C,
  ...args: CallArgs<C>
) => Promise<Value<C>>;

// A type assertion only: keep the native function, payload and Promise unchanged.
export const invoke = nativeInvoke as <C extends Command>(
  command: C,
  ...args: CommandMap[C]['args'] extends Record<string, never>
    ? [payload?: CommandMap[C]['args']]
    : [payload: CommandMap[C]['args']]
) => Promise<CommandMap[C]['result']>;
