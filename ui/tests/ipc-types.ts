// Compiled by the normal tsc build; these errors must remain rejected.
import type { WorkbenchCall } from '../src/ipc';
import { invoke } from '../src/ipc';

declare const call: WorkbenchCall;
if (false) {
  const view = call('workspace_view');
  void view.then((workspace) => workspace.name);
  void call('select_build_target', { target: 'linux-x64-controlled-v1' });
  // @ts-expect-error Unknown command names cannot cross the boundary.
  void call('workspace_veiw');
  // @ts-expect-error Required payload cannot be omitted.
  void call('select_build_target');
  // @ts-expect-error Rust enum values constrain arguments.
  void call('select_build_target', { target: 'linux' });
  // @ts-expect-error Rust parameters constrain payload keys.
  void call('read_project_source', { source_id: 'source' });
  // @ts-expect-error Callers cannot choose their own result type.
  const result: Promise<string> = call('workspace_view');
  void result;
  // @ts-expect-error Native invocation requires the fingerprint parameter.
  void invoke('project_projection', {});
  // @ts-expect-error Result fields come from the Rust DTO.
  void view.then((workspace) => workspace.workspace_name);
}

// The production and verification builds must agree on every normal command.
import type { CommandMap as Native } from '../src/ipc/bindings';
import type { CommandMap as Production } from '../src/ipc/bindings.production';
type Normal = Exclude<keyof Native, 'verification_metrics' | 'verification_owned_failure'>;
type Same =
  Pick<Native, Normal> extends Pick<Production, Normal>
    ? Pick<Production, Normal> extends Pick<Native, Normal>
      ? true
      : false
    : false;
const same: Same = true;
void same;
