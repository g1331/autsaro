import type { Text } from '../i18n';
import type { WorkbenchCapabilities } from '../ipc/types';
import type { Notice } from '../shared/ui/forms';

export type ToolWindow = 'problems' | 'generation' | 'build' | 'host' | 'log';

export type ShellState = {
  capabilities: WorkbenchCapabilities | null;
  operationGeneration: number;
  busy: Text | null;
  notice: Notice;
  operationLog: { command: string; outcome: 'done' | 'failed'; detail: Text }[];
  toolWindow: ToolWindow | null;
  treeVisible: boolean;
  inspectorVisible: boolean;
  inspectorTab: 'properties' | 'references';
  treeMode: 'objects' | 'files';
  treeFilter: string;
  treeRevealId: string | null;
  objectFilter: string;
};

export function initialShellState(): ShellState {
  return {
    capabilities: null,
    operationGeneration: 0,
    busy: null,
    notice: null,
    operationLog: [],
    toolWindow: null,
    treeVisible: true,
    inspectorVisible: true,
    inspectorTab: 'properties',
    treeMode: 'objects',
    treeFilter: '',
    treeRevealId: null,
    objectFilter: '',
  };
}
