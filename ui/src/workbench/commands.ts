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
      label: message('shell.command.new'),
      menu: 'file',
      reason: c.actionReason('create'),
      run: () => c.openProjectEntry('empty'),
    },
    {
      id: 'open',
      group: 'open',
      label: message('shell.command.open'),
      menu: 'file',
      shortcut: 'Ctrl+O',
      reason: c.actionReason('open'),
      run: () => c.openProjectEntry('import'),
    },
    {
      id: 'member',
      group: 'open',
      label: message('shell.command.member'),
      menu: 'file',
      reason: c.actionReason('open'),
      run: c.openMemberProject,
    },
    {
      id: 'ecu-package',
      group: 'open',
      label: message('shell.command.ecuPackage'),
      menu: 'file',
      reason: c.actionReason('open'),
      run: c.importHandoff,
    },
    {
      id: 'save',
      group: 'save',
      label: message('shell.command.save'),
      menu: 'file',
      shortcut: 'Ctrl+S',
      reason: workspaceReason,
      run: c.requestSave,
    },
    {
      id: 'save-as',
      group: 'save',
      label: message('shell.command.saveAs'),
      menu: 'file',
      reason: workspaceReason || c.actionReason('save'),
      run: () => c.openProjectEntry('save-as'),
    },
    {
      id: 'close',
      group: 'close',
      label: message('shell.command.close'),
      menu: 'file',
      reason: !c.workspace ? message('shell.reason.noProject') : c.actionReason('open'),
      run: () => c.startProject(),
    },
    {
      id: 'settings',
      group: 'settings',
      label: message('shell.command.settings'),
      menu: 'file',
      reason: '',
      run: () => c.setSettingsOpen(true),
    },
    {
      id: 'apply',
      label: message('shell.command.apply'),
      menu: 'edit',
      reason: workspaceReason || (!c.unapplied ? message('shell.reason.noChanges') : ''),
      run: c.applyCurrentDrafts,
    },
    {
      id: 'restore',
      label: message('shell.command.restore'),
      menu: 'edit',
      reason: workspaceReason || (!c.unapplied ? message('shell.reason.noDrafts') : ''),
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
      label: c.workspace?.integrationCandidate
        ? message('shell.command.inspect')
        : message('shell.command.validate'),
      menu: 'tools',
      reason: workspaceReason,
      run: () =>
        c.guardContext(message('shell.command.validate'), () =>
          c.workspace?.integrationCandidate ? c.inspectIntegration() : c.validateProject(),
        ),
    },
    {
      id: 'initialize-application',
      group: 'check',
      label: message('shell.command.initialize'),
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
      label: message('shell.command.preview'),
      menu: 'tools',
      reason: deliveryReason,
      run: () => {
        c.openDocument({ kind: 'delivery' });
        if (!c.ecuOutputDirectory.trim()) void c.chooseDirectory(c.changeEcuOutput);
        else c.previewEcu();
      },
    },
    {
      id: 'preflight',
      group: 'native',
      label: message('shell.command.preflight'),
      menu: 'tools',
      reason: deliveryReason || toolReason,
      run: c.preflightEcu,
    },
    {
      id: 'build-ecu',
      group: 'native',
      label: message('shell.delivery.build'),
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
      label: message('shell.delivery.verify'),
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
      label: message('shell.command.cancel'),
      menu: 'tools',
      reason: !c.busy ? message('shell.reason.idle') : '',
      run: c.cancelOperation,
    },
    {
      id: 'preview-changes',
      label: message('editor.objects.previewBatch'),
      reason: workspaceReason || (!c.changes.length ? message('shell.reason.noChanges') : ''),
      run: c.prepareChanges,
    },
    {
      id: 'discard-changes',
      label: message('editor.objects.restoreBatch'),
      reason: workspaceReason || (!c.changes.length ? message('shell.reason.noDrafts') : ''),
      run: () => c.discardChanges(),
    },
    {
      id: 'accept-definitions',
      label: message('shell.settings.acceptCatalog'),
      reason: workspaceReason,
      run: c.importDefinitionCatalog,
    },
    {
      id: 'execution-tools',
      label: message('shell.command.executionTools'),
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
