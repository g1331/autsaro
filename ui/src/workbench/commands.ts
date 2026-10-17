import { composedMessage, message } from '../i18n';
import type { Text } from '../i18n';
import type { Workbench } from './useWorkbench';

export type Command = {
  id: string;
  label: Text;
  menu?: 'file' | 'edit' | 'view' | 'tools' | 'help';
  /** Menu separators fall between consecutive commands of different groups. */
  group?: string;
  shortcut?: string;
  /** Why the command is unavailable; empty when it can run. */
  reason: Text;
  /** The command that resolves `reason`, offered beside it. */
  remedy?: Command;
  run: () => void | Promise<unknown>;
};

/**
 * Every entry (menu, toolbar, page, tool window, command search) reads availability and
 * reason from here, so one action has one precondition wherever it is triggered.
 */
export function workbenchCommands(c: Omit<Workbench, 'commands'>): Command[] {
  const workspaceReason: Text = !c.workspace
    ? message('shell.reason.noProject')
    : c.disabled
      ? c.busy
        ? composedMessage('shell.busy', { operation: c.busy })
        : (c.capabilities?.ruleError ?? message('shell.reason.backend'))
      : '';
  const sourceReason: Text =
    workspaceReason ||
    (c.workspace?.dirty || c.projection?.dirty ? message('shell.reason.save') : '') ||
    (c.unapplied ? message('shell.reason.draft') : '');
  // Delivery compiles the user application; the backend rejects it with APPLICATION_UNINITIALIZED.
  const initializeReason = message('backend.prepared.application_initialization_required');
  const deliveryReason: Text =
    sourceReason ||
    (!c.workspace?.integrationCandidate ? message('shell.reason.standard') : '') ||
    (!c.workspace?.applicationInitialized ? initializeReason : '');
  const toolReason = c.executionReason;
  const commands: Command[] = [
    {
      id: 'new',
      group: 'open',
      label: message('action.newProject'),
      menu: 'file',
      reason: c.actionReason('create'),
      run: () => c.openProjectEntry('empty'),
    },
    {
      id: 'open',
      group: 'open',
      label: message('action.importArxml'),
      menu: 'file',
      shortcut: 'Ctrl+O',
      reason: c.actionReason('open'),
      run: () => c.openProjectEntry('import'),
    },
    {
      id: 'member',
      group: 'open',
      label: message('action.openProject'),
      menu: 'file',
      reason: c.actionReason('open'),
      run: c.openMemberProject,
    },
    {
      id: 'ecu-package',
      group: 'open',
      label: message('action.importHandoff'),
      menu: 'file',
      reason: c.actionReason('open'),
      run: c.importHandoff,
    },
    {
      id: 'save',
      group: 'save',
      label: message('action.previewSave'),
      menu: 'file',
      shortcut: 'Ctrl+S',
      reason: workspaceReason,
      run: c.requestSave,
    },
    {
      id: 'save-as',
      group: 'save',
      label: message('action.saveAs'),
      menu: 'file',
      reason: workspaceReason || c.actionReason('save'),
      run: () => c.openProjectEntry('save-as'),
    },
    {
      id: 'close',
      group: 'close',
      label: message('action.closeProject'),
      menu: 'file',
      reason: !c.workspace ? message('shell.reason.noProject') : c.actionReason('open'),
      run: () => c.startProject(),
    },
    {
      id: 'settings',
      group: 'settings',
      label: message('action.settings'),
      menu: 'file',
      reason: '',
      run: () => c.setSettingsOpen(true),
    },
    {
      id: 'apply',
      label: message('action.apply'),
      menu: 'edit',
      reason: workspaceReason || (!c.unapplied ? message('shell.reason.noChanges') : ''),
      run: c.applyCurrentDrafts,
    },
    {
      id: 'restore',
      label: message('action.revert'),
      menu: 'edit',
      reason: workspaceReason || (!c.unapplied ? message('shell.reason.noChanges') : ''),
      run: c.restoreAllDrafts,
    },
    {
      id: 'tree',
      group: 'panels',
      label: message('shell.command.tree'),
      menu: 'view',
      shortcut: 'Alt+1',
      reason: '',
      run: () => c.setTreeVisible((previous) => !previous),
    },
    {
      id: 'inspector',
      group: 'panels',
      label: message('shell.command.inspector'),
      menu: 'view',
      reason: '',
      run: () => c.setInspectorVisible((previous) => !previous),
    },
    {
      id: 'objects',
      group: 'documents',
      label: message('shell.document.configuration'),
      menu: 'view',
      reason: workspaceReason,
      run: () => c.openDocument({ kind: 'configuration' }),
    },
    {
      id: 'standard',
      group: 'documents',
      label: message('shell.document.integration'),
      menu: 'view',
      reason: workspaceReason,
      run: () => c.openDocument({ kind: 'integration' }),
    },
    {
      id: 'delivery',
      group: 'documents',
      label: message('shell.document.delivery'),
      menu: 'view',
      reason: workspaceReason,
      run: () => c.openDocument({ kind: 'delivery' }),
    },
    {
      id: 'problems',
      group: 'tool-windows',
      label: message('shell.command.problems'),
      menu: 'view',
      reason: '',
      run: () => c.setToolWindow('problems'),
    },
    {
      id: 'build',
      group: 'tool-windows',
      label: message('shell.command.build'),
      menu: 'view',
      shortcut: 'Alt+4',
      reason: '',
      run: () => c.setToolWindow('build'),
    },
    {
      id: 'host',
      group: 'tool-windows',
      label: message('shell.command.host'),
      menu: 'view',
      shortcut: 'Alt+9',
      reason: '',
      run: () => c.setToolWindow('host'),
    },
    {
      id: 'validate',
      group: 'check',
      label: message('action.validate'),
      menu: 'tools',
      reason: workspaceReason,
      run: () =>
        c.guardContext(message('activity.validate'), () =>
          c.workspace?.integrationCandidate ? c.inspectIntegration() : c.validateProject(),
        ),
    },
    {
      id: 'initialize-application',
      group: 'check',
      label: message('action.initializeApplication'),
      menu: 'tools',
      reason:
        sourceReason ||
        c.actionReason('edit') ||
        (!c.workspace?.integrationCandidate ? message('shell.reason.noSlots') : ''),
      run: c.previewApplicationInitialization,
    },
    {
      id: 'preview',
      group: 'delivery',
      label: message('action.previewSource'),
      menu: 'tools',
      reason: deliveryReason,
      run: () => {
        c.openDocument({ kind: 'delivery' });
        if (!c.ecuOutputDirectory.trim()) void c.chooseDirectory(c.changeEcuOutput);
        else c.previewEcu();
      },
    },
    {
      id: 'generate',
      group: 'delivery',
      label: message('action.confirmGenerate'),
      menu: 'tools',
      reason: deliveryReason || (!c.generationPreview ? message('shell.reason.previewFirst') : ''),
      run: c.generateEcu,
    },
    {
      id: 'preflight',
      group: 'native',
      label: message('action.preflight'),
      menu: 'tools',
      reason: deliveryReason || toolReason,
      run: c.preflightEcu,
    },
    {
      id: 'build-ecu',
      group: 'native',
      label: message('action.build'),
      menu: 'tools',
      reason:
        sourceReason ||
        toolReason ||
        (!c.generated || c.stages.generate.state !== 'done'
          ? message('shell.reason.generateFirst')
          : '') ||
        (!c.buildDirectory.trim() ? message('shell.reason.buildDirectory') : ''),
      run: c.buildEcu,
    },
    {
      id: 'verify-ecu',
      group: 'native',
      label: message('action.verifyHost'),
      menu: 'tools',
      reason:
        sourceReason ||
        toolReason ||
        (!c.built || c.stages.build.state !== 'done' ? message('shell.reason.buildFirst') : ''),
      run: c.verifyEcu,
    },
    {
      id: 'cancel',
      group: 'cancel',
      label: message('action.cancelOperation'),
      menu: 'tools',
      reason: !c.busy ? message('shell.reason.idle') : '',
      run: c.cancelOperation,
    },
    {
      id: 'preview-changes',
      label: message('action.previewChanges'),
      reason: workspaceReason || (!c.changes.length ? message('shell.reason.noChanges') : ''),
      run: c.prepareChanges,
    },
    {
      id: 'discard-changes',
      label: message('action.revert'),
      reason: workspaceReason || (!c.changes.length ? message('shell.reason.noChanges') : ''),
      run: () => c.discardChanges(),
    },
    {
      id: 'accept-definitions',
      label: message('action.acceptDefinitions'),
      reason: workspaceReason,
      run: c.importDefinitionCatalog,
    },
    {
      id: 'execution-tools',
      label: message('action.executionTools'),
      reason: '',
      run: () => {
        c.setSettingsCategory('tools');
        c.setSettingsOpen(true);
      },
    },
  ];
  if (c.capabilities?.verificationMode === true) {
    const reason: Text = !c.native
      ? message('shell.reason.desktop')
      : c.busy
        ? composedMessage('shell.busy', { operation: c.busy })
        : '';
    commands.push(
      {
        id: 'verification-owned-long-failure',
        label: message('shell.command.verifyFailure'),
        reason,
        run: () => c.verificationOwnedFailure(0),
      },
      {
        id: 'verification-owned-cancel',
        label: message('shell.command.verifyCancel'),
        reason,
        run: () => c.verificationOwnedFailure(5000),
      },
    );
  }
  const byId = (id: string) => commands.find((command) => command.id === id)!;
  // A tool problem is fixed in Settings › Execution tools; an uninitialized application by initializing it.
  for (const command of commands) {
    if (command.reason && command.reason === toolReason && c.capabilities?.toolError)
      command.remedy = byId('execution-tools');
    else if (command.reason === initializeReason) command.remedy = byId('initialize-application');
  }
  return commands;
}

export function commandById(c: { commands: Command[] }, id: string): Command {
  const command = c.commands.find((entry) => entry.id === id);
  if (!command) throw new Error(`Unknown command ${id}`);
  return command;
}
