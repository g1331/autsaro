import { message, useLocale } from './i18n';
import { displayPath } from './pathDisplay';
import { useEffect, useEffectEvent, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  ChevronRight,
  CircleAlert,
  Code2,
  FolderTree,
  Hammer,
  ListChecks,
  MonitorPlay,
  PanelRight,
  Plus,
  Save,
  Search,
  Settings,
  Square,
  X,
} from 'lucide-react';
import { useWorkbench } from './workbench/useWorkbench';
import { EditorPage, EditorInspector } from './pages/EditorPage';
import { IntegrationPanel } from './IntegrationPanel';
import { IntegrationDelivery } from './IntegrationDelivery';
import { BuildPage } from './pages/BuildPage';
import { SettingsPage } from './pages/SettingsPage';
import { ProjectTree } from './workbench/ProjectTree';
import { ObjectEditor, ObjectInspector } from './workbench/ObjectEditor';
import { PreviewDialogs } from './workbench/PreviewDialogs';
import { CopyText, Dialog, rememberDialogOpener, TextSnapshot } from './workbench/Dialog';
import { ToolWindows } from './workbench/ToolWindows';
import { WindowControls } from './workbench/WindowControls';
import { PanelResizeHandle } from './workbench/PanelResizeHandle';
import { SourceEntry } from './workbench/SourceEntry';
import { labelFromPath } from './workbench/forms';
import type { DocumentTab } from './workbench/projectTypes';

const documentLabels = {
  configuration: 'shell.document.configuration',
  communication: 'shell.document.communication',
  diagnostic: 'shell.document.diagnostic',
  integration: 'shell.document.integration',
  delivery: 'shell.document.delivery',
  source: 'shell.document.source',
  'project-entry': 'shell.document.project-entry',
};
type Command = {
  id: string;
  label: string;
  menu?: 'file' | 'edit' | 'view' | 'tools' | 'help';
  shortcut?: string;
  reason: string;
  run: () => void | Promise<unknown>;
};

export default function App() {
  const { t, text } = useLocale();
  const c = useWorkbench();
  const [menu, setMenu] = useState<string | null>(null);
  const [palette, setPalette] = useState(false);
  const [query, setQuery] = useState('');
  const [help, setHelp] = useState(false);
  const [systemDark, setSystemDark] = useState(
    () => window.matchMedia('(prefers-color-scheme: dark)').matches,
  );
  const toolOpener = useRef<HTMLElement | null>(null);
  const preference = c.settingsOpen ? c.appearanceDraft : (c.capabilities?.appearance ?? 'system');
  const effectiveTheme = preference === 'system' ? (systemDark ? 'dark' : 'light') : preference;
  const logo = effectiveTheme === 'dark' ? '/logo-workbench-dark.png' : '/logo-workbench.png';
  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const change = (event: MediaQueryListEvent) => setSystemDark(event.matches);
    media.addEventListener('change', change);
    return () => media.removeEventListener('change', change);
  }, []);
  useEffect(() => {
    const media = window.matchMedia('(prefers-reduced-motion: reduce)');
    const update = () => {
      // Native media events can precede CSS media-rule invalidation.
      document.documentElement.dataset.reducedMotion = String(media.matches);
    };
    update();
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, []);
  useEffect(() => {
    document.documentElement.dataset.theme = effectiveTheme;
    document
      .querySelector<HTMLMetaElement>('meta[name="theme-color"]')
      ?.setAttribute('content', effectiveTheme === 'dark' ? '#1b1b1b' : '#f3f3f3');
    document.querySelector<HTMLLinkElement>('link[rel="icon"]')?.setAttribute('href', logo);
  }, [effectiveTheme, logo]);
  const requestExit = async () => {
    await c.replaceProject(message('shell.command.exit'), () => getCurrentWindow().destroy());
  };
  const exitRequested = useEffectEvent(requestExit);
  useEffect(() => {
    if (!c.native) return;
    let live = true;
    let stop: (() => void) | undefined;
    void getCurrentWindow()
      .onCloseRequested((event) => {
        event.preventDefault();
        void exitRequested();
      })
      .then((unlisten) => {
        if (live) stop = unlisten;
        else unlisten();
      });
    return () => {
      live = false;
      stop?.();
    };
  }, [c.native]);
  const workspaceReason = !c.workspace
    ? t('shell.reason.noProject')
    : c.disabled
      ? c.busy
        ? t('shell.busy', { operation: text(c.busy) })
        : c.capabilities?.ruleError
          ? text(c.capabilities.ruleError)
          : t('shell.reason.backend')
      : '';
  const sourceReason =
    workspaceReason ||
    (c.workspace?.dirty || c.projection?.dirty ? t('shell.reason.save') : '') ||
    (c.unapplied ? t('shell.reason.draft') : '');
  const legacyReason =
    workspaceReason || (c.workspace?.integrationCandidate ? t('shell.reason.standardCreator') : '');
  const commands: Command[] = [
    {
      id: 'new',
      label: t('shell.command.new'),
      menu: 'file',
      reason: text(c.actionReason('create')),
      run: () => c.openProjectEntry('empty'),
    },
    {
      id: 'open',
      label: t('shell.command.open'),
      menu: 'file',
      shortcut: 'Ctrl+O',
      reason: text(c.actionReason('open')),
      run: () => c.openProjectEntry('import'),
    },
    {
      id: 'member',
      label: t('shell.command.member'),
      menu: 'file',
      reason: c.busy || !c.native ? t('shell.reason.desktopBusy') : '',
      run: c.openMemberProject,
    },
    {
      id: 'host-package',
      label: t('shell.command.hostPackage'),
      menu: 'file',
      reason: c.busy || !c.native ? t('shell.reason.desktopBusy') : '',
      run: c.importHandoff,
    },
    {
      id: 'ecu-package',
      label: t('shell.command.ecuPackage'),
      menu: 'file',
      reason: c.busy || !c.native ? t('shell.reason.desktopBusy') : '',
      run: () => c.chooseDirectory((directory) => c.importHandoffDirectory(directory)),
    },
    {
      id: 'save',
      label: t('shell.command.save'),
      menu: 'file',
      shortcut: 'Ctrl+S',
      reason: workspaceReason,
      run: c.requestSave,
    },
    {
      id: 'save-as',
      label: t('shell.command.saveAs'),
      menu: 'file',
      reason: workspaceReason || text(c.actionReason('save')),
      run: () => c.openProjectEntry('save-as'),
    },
    {
      id: 'close',
      label: t('shell.command.close'),
      menu: 'file',
      reason: !c.workspace ? t('shell.reason.noProject') : text(c.actionReason('open')),
      run: () => c.startProject(),
    },
    {
      id: 'settings',
      label: t('shell.command.settings'),
      menu: 'file',
      reason: '',
      run: () => c.setSettingsOpen(true),
    },
    {
      id: 'apply',
      label: t('shell.command.apply'),
      menu: 'edit',
      reason: workspaceReason || (!c.unapplied ? t('shell.reason.noChanges') : ''),
      run: c.applyCurrentDrafts,
    },
    {
      id: 'restore',
      label: t('shell.command.restore'),
      menu: 'edit',
      reason: workspaceReason || (!c.unapplied && !c.creating ? t('shell.reason.noDrafts') : ''),
      run: c.restoreAllDrafts,
    },
    {
      id: 'frame',
      label: t('shell.command.frame'),
      menu: 'edit',
      reason: legacyReason,
      run: () => c.openCreator('frame'),
    },
    {
      id: 'signal',
      label: t('shell.command.signal'),
      menu: 'edit',
      reason: legacyReason || (!c.focusedFrame ? t('shell.reason.frame') : ''),
      run: () => c.openCreator('signal'),
    },
    {
      id: 'tree',
      label: t('shell.command.tree'),
      menu: 'view',
      shortcut: 'Alt+1',
      reason: '',
      run: () => c.setTreeVisible((previous) => !previous),
    },
    {
      id: 'inspector',
      label: t('shell.command.inspector'),
      menu: 'view',
      reason: '',
      run: () => c.setInspectorVisible((previous) => !previous),
    },
    {
      id: 'objects',
      label: t('shell.document.configuration'),
      menu: 'view',
      reason: workspaceReason,
      run: () => c.openDocument({ kind: 'configuration' }),
    },
    {
      id: 'communication',
      label: t('shell.document.communication'),
      menu: 'view',
      reason: legacyReason,
      run: () => c.openDocument({ kind: 'communication' }),
    },
    {
      id: 'diagnostic',
      label: t('shell.document.diagnostic'),
      menu: 'view',
      reason: legacyReason,
      run: () => c.openDocument({ kind: 'diagnostic' }),
    },
    {
      id: 'standard',
      label: t('shell.document.integration'),
      menu: 'view',
      reason: workspaceReason,
      run: () => c.openDocument({ kind: 'integration' }),
    },
    {
      id: 'delivery',
      label: t('shell.document.delivery'),
      menu: 'view',
      reason: workspaceReason,
      run: () => c.openDocument({ kind: 'delivery' }),
    },
    {
      id: 'validate',
      label: c.workspace?.integrationCandidate
        ? t('shell.command.inspect')
        : t('shell.command.validate'),
      menu: 'tools',
      reason: workspaceReason,
      run: () =>
        c.guardContext(message('shell.command.validate'), () =>
          c.workspace?.integrationCandidate ? c.inspectIntegration() : c.validateProject(),
        ),
    },
    {
      id: 'preview',
      label: t('shell.command.preview'),
      menu: 'tools',
      reason: sourceReason,
      run: () => {
        c.openDocument({ kind: 'delivery' });
        if (c.workspace?.integrationCandidate) {
          if (!c.ecuOutputDirectory) void c.chooseDirectory(c.changeEcuOutput);
          else c.previewEcu();
        } else c.generateProject();
      },
    },
    {
      id: 'preflight',
      label: t('shell.command.preflight'),
      menu: 'tools',
      reason:
        sourceReason ||
        text(c.executionReason) ||
        (!c.workspace?.integrationCandidate ? t('shell.reason.standard') : ''),
      run: c.preflightEcu,
    },
    {
      id: 'initialize-application',
      label: t('shell.command.initialize'),
      menu: 'tools',
      reason:
        sourceReason ||
        text(c.actionReason('edit')) ||
        (!c.workspace?.integrationCandidate ? t('shell.reason.noSlots') : ''),
      run: c.previewApplicationInitialization,
    },
    {
      id: 'build',
      label: t('shell.command.build'),
      menu: 'tools',
      shortcut: 'Alt+4',
      reason: '',
      run: () => c.setToolWindow('build'),
    },
    {
      id: 'host',
      label: t('shell.command.host'),
      menu: 'tools',
      shortcut: 'Alt+9',
      reason: '',
      run: () => c.setToolWindow('host'),
    },
    {
      id: 'problems',
      label: t('shell.command.problems'),
      menu: 'tools',
      reason: '',
      run: () => c.setToolWindow('problems'),
    },
    {
      id: 'cancel',
      label: t('shell.command.cancel'),
      menu: 'tools',
      reason: !c.busy ? t('shell.reason.idle') : '',
      run: c.cancelOperation,
    },
    {
      id: 'help',
      label: t('shell.command.help'),
      menu: 'help',
      reason: '',
      run: () => setHelp(true),
    },
  ];
  if (c.capabilities?.verificationMode === true) {
    const reason = !c.native
      ? t('shell.reason.desktop')
      : c.busy
        ? t('shell.busy', { operation: text(c.busy) })
        : '';
    commands.push(
      {
        id: 'verification-owned-long-failure',
        label: t('shell.command.verifyFailure'),
        reason,
        run: () => c.verificationOwnedFailure(0),
      },
      {
        id: 'verification-owned-cancel',
        label: t('shell.command.verifyCancel'),
        reason,
        run: () => c.verificationOwnedFailure(5000),
      },
    );
  }
  const dispatchCommand = (id: string) => {
    const command = commands.find((entry) => entry.id === id);
    if (!command || command.reason) return;
    rememberDialogOpener();
    setMenu(null);
    setPalette(false);
    void command.run();
  };
  const shortcut = useEffectEvent((event: KeyboardEvent) => {
    if (event.isComposing || event.defaultPrevented) return;
    const modal =
      c.settingsOpen ||
      c.guard ||
      c.changePreview ||
      c.savePreview ||
      c.integrationPreview ||
      c.generationPreview ||
      c.projectPreview ||
      c.applicationPreview ||
      c.handoffImportOpen ||
      palette ||
      help;
    if (event.key === 'Escape') {
      if (modal) return;
      if (menu) {
        event.preventDefault();
        setMenu(null);
        return;
      }
      if (c.toolWindow) {
        event.preventDefault();
        c.setToolWindow(null);
        toolOpener.current?.focus();
      }
      return;
    }
    if (modal) return;
    const target = event.target instanceof HTMLElement ? event.target : null;
    const editing = target?.matches('input, textarea, select, [contenteditable="true"]');
    if ((event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey) {
      const key = event.key.toLocaleLowerCase();
      if (key === 'k') {
        event.preventDefault();
        setQuery('');
        setPalette(true);
      } else if (key === 's' || key === 'o') {
        event.preventDefault();
        dispatchCommand(key === 's' ? 'save' : 'open');
      }
    } else if (!editing && event.altKey && !event.ctrlKey && !event.metaKey) {
      const command =
        event.key === '1'
          ? 'tree'
          : event.key === '4'
            ? 'build'
            : event.key === '9'
              ? 'host'
              : null;
      if (command) {
        event.preventDefault();
        toolOpener.current =
          document.activeElement instanceof HTMLElement ? document.activeElement : null;
        dispatchCommand(command);
      }
    }
  });
  useEffect(() => {
    window.addEventListener('keydown', shortcut);
    return () => window.removeEventListener('keydown', shortcut);
  }, []);
  useEffect(() => {
    if (!menu) return;
    document.querySelector<HTMLButtonElement>('.menu-popup button:not(:disabled)')?.focus();
    const closeOutside = (event: PointerEvent) => {
      if (event.target instanceof HTMLElement && !event.target.closest('.menu-root')) setMenu(null);
    };
    window.addEventListener('pointerdown', closeOutside);
    return () => window.removeEventListener('pointerdown', closeOutside);
  }, [menu]);
  const activeDocument = c.activeDocument;
  const source = c.projection?.sources.find((item) => item.sourceId === activeDocument.sourceId);
  const tabLabel = (tab: DocumentTab) =>
    tab.kind === 'source'
      ? labelFromPath(
          c.projection?.sources.find((item) => item.sourceId === tab.sourceId)?.path ??
            t(documentLabels.source),
        )
      : t(documentLabels[tab.kind]);
  return (
    <div className="app-shell">
      <header className="menubar" data-tauri-drag-region>
        <img src={logo} width="27" height="27" alt={t('shell.app.title')} data-tauri-drag-region />
        <nav aria-label={t('shell.app.menu')}>
          {(['file', 'edit', 'view', 'tools', 'help'] as const).map((label) => (
            <div className="menu-root" key={label}>
              <button
                type="button"
                aria-expanded={menu === label}
                aria-haspopup="menu"
                onClick={() => setMenu(menu === label ? null : label)}
              >
                {t(`shell.menu.${label}`)}
              </button>
              {menu === label ? (
                <div
                  className="menu-popup"
                  role="menu"
                  aria-label={t(`shell.menu.${label}`)}
                  onKeyDown={(event) => {
                    if (event.key === 'Escape') {
                      event.preventDefault();
                      setMenu(null);
                      (
                        event.currentTarget.parentElement?.querySelector(
                          'button',
                        ) as HTMLElement | null
                      )?.focus();
                    }
                    if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
                      event.preventDefault();
                      const items = Array.from(
                        event.currentTarget.querySelectorAll<HTMLButtonElement>(
                          'button:not(:disabled)',
                        ),
                      );
                      const index = items.indexOf(event.target as HTMLButtonElement);
                      items[
                        event.key === 'Home'
                          ? 0
                          : event.key === 'End'
                            ? items.length - 1
                            : (index + (event.key === 'ArrowDown' ? 1 : -1) + items.length) %
                              items.length
                      ]?.focus();
                    }
                  }}
                >
                  {commands
                    .filter((command) => command.menu === label)
                    .map((command) => (
                      <div key={command.id}>
                        <button
                          type="button"
                          role="menuitem"
                          disabled={Boolean(command.reason)}
                          onClick={() => dispatchCommand(command.id)}
                        >
                          <span>{command.label}</span>
                          <kbd>{command.shortcut}</kbd>
                        </button>
                        {command.reason ? <small>{command.reason}</small> : null}
                      </div>
                    ))}
                </div>
              ) : null}
            </div>
          ))}
        </nav>
        <div className="titlebar-drag-region" data-tauri-drag-region>
          <span className="app-title">{t('shell.app.title')}</span>
        </div>
        <button
          type="button"
          onClick={() => {
            setQuery('');
            setPalette(true);
          }}
          aria-label={t('shell.app.commandShortcut')}
        >
          <Search size={16} aria-hidden="true" />
        </button>
        {c.native ? (
          <WindowControls
            onClose={requestExit}
            onError={(error) =>
              c.setNotice({
                tone: 'error',
                text: message('shell.windowError', { error: String(error) }),
              })
            }
          />
        ) : null}
      </header>
      <div className="context-toolbar">
        <strong>{c.workspace?.name ?? t('shell.app.noProject')}</strong>
        <span className="toolbar-separator" />
        {c.workspace ? (
          <>
            <span>
              {c.projection?.profile ??
                (c.workspace.integrationCandidate
                  ? t('shell.app.standardEcu')
                  : t('shell.app.canSignals'))}
            </span>
            <button
              type="button"
              onClick={() => dispatchCommand('save')}
              disabled={Boolean(workspaceReason)}
            >
              <Save size={15} aria-hidden="true" />
              {t('shell.command.save')}
            </button>
            <button
              type="button"
              onClick={() => dispatchCommand('validate')}
              disabled={Boolean(workspaceReason)}
            >
              <ListChecks size={15} aria-hidden="true" />
              {t('shell.app.validate')}
            </button>
            <button type="button" onClick={() => dispatchCommand('delivery')}>
              <Code2 size={15} aria-hidden="true" />
              {t('shell.document.delivery')}
            </button>
          </>
        ) : null}
        <span className="toolbar-spacer" />
        {c.busy ? (
          <>
            <span role="status">{t('shell.busy', { operation: text(c.busy) })}</span>
            <button type="button" onClick={() => dispatchCommand('cancel')}>
              <Square size={13} aria-hidden="true" />
              {t('shell.app.cancel')}
            </button>
          </>
        ) : null}
        <button
          type="button"
          aria-label={t('shell.settings.title')}
          onClick={() => dispatchCommand('settings')}
        >
          <Settings size={16} aria-hidden="true" />
        </button>
      </div>
      <main className="workspace-frame">
        <nav className="tool-rail" aria-label={t('shell.app.rail')}>
          <button
            type="button"
            aria-label={t('shell.app.treeShortcut')}
            title={t('shell.app.treeShortcut')}
            aria-pressed={c.treeVisible}
            onClick={() => dispatchCommand('tree')}
          >
            <FolderTree size={19} />
          </button>
          <button
            type="button"
            aria-label={t('shell.app.problems')}
            title={t('shell.app.problems')}
            aria-pressed={c.toolWindow === 'problems'}
            onClick={() => dispatchCommand('problems')}
          >
            <CircleAlert size={19} />
          </button>
          <button
            type="button"
            aria-label={t('shell.app.buildShortcut')}
            title={t('shell.app.buildShortcut')}
            aria-pressed={c.toolWindow === 'build'}
            onClick={() => dispatchCommand('build')}
          >
            <Hammer size={19} />
          </button>
          <button
            type="button"
            aria-label={t('shell.app.hostShortcut')}
            title={t('shell.app.hostShortcut')}
            aria-pressed={c.toolWindow === 'host'}
            onClick={() => dispatchCommand('host')}
          >
            <MonitorPlay size={19} />
          </button>
          <span />
          <button
            type="button"
            aria-label={t('shell.command.inspector')}
            title={t('shell.command.inspector')}
            aria-pressed={c.inspectorVisible}
            onClick={() => dispatchCommand('inspector')}
          >
            <PanelRight size={19} />
          </button>
        </nav>
        {c.workspace ? <ProjectTree controller={c} /> : null}
        <section className="central-workspace">
          {c.workspace ? (
            <>
              <div className="document-tabs" role="tablist" aria-label={t('shell.app.documents')}>
                {c.tabs.map((tab) => (
                  <div
                    key={`${tab.kind}-${tab.sourceId ?? ''}`}
                    className={
                      activeDocument.kind === tab.kind && activeDocument.sourceId === tab.sourceId
                        ? 'active'
                        : ''
                    }
                  >
                    <button
                      type="button"
                      role="tab"
                      aria-selected={
                        activeDocument.kind === tab.kind && activeDocument.sourceId === tab.sourceId
                      }
                      onClick={() =>
                        tab.kind === 'source' && tab.sourceId
                          ? void c.readSource(tab.sourceId)
                          : c.openDocument(tab)
                      }
                    >
                      {tabLabel(tab)}
                    </button>
                    <button
                      type="button"
                      aria-label={t('shell.app.closeTab', { tab: tabLabel(tab) })}
                      onClick={() => void c.closeDocument(tab)}
                    >
                      <X size={12} />
                    </button>
                  </div>
                ))}
                <button
                  type="button"
                  aria-label={t('shell.app.openConfiguration')}
                  onClick={() => c.openDocument({ kind: 'configuration' })}
                >
                  <Plus size={14} />
                </button>
              </div>
              <div
                className="document-content"
                role="tabpanel"
                aria-label={tabLabel(activeDocument)}
              >
                {activeDocument.kind === 'project-entry' ? (
                  <SourceEntry controller={c} logo={logo} />
                ) : null}
                {activeDocument.kind === 'configuration' ? <ObjectEditor controller={c} /> : null}
                {activeDocument.kind === 'communication' || activeDocument.kind === 'diagnostic' ? (
                  <EditorPage controller={c} section={activeDocument.kind} />
                ) : null}
                {activeDocument.kind === 'integration' ? <IntegrationPanel controller={c} /> : null}
                {activeDocument.kind === 'delivery' ? (
                  c.workspace.integrationCandidate ? (
                    <IntegrationDelivery controller={c} active />
                  ) : (
                    <BuildPage controller={c} />
                  )
                ) : null}
                {activeDocument.kind === 'source' ? (
                  <div className="source-document">
                    <h2>{labelFromPath(source?.path ?? '')}</h2>
                    <p className="mono path-text">{displayPath(source?.path ?? '')}</p>
                    {source ? (
                      <CopyText
                        text={displayPath(source.path)}
                        label={t('shell.app.copySourcePath')}
                      />
                    ) : null}
                    <p>
                      {t('shell.app.readonlySource')} ·{' '}
                      {source?.readonly
                        ? t('shell.app.sourceReadonly')
                        : t('shell.app.sourceTransaction')}
                    </p>
                    {source && c.activeSourceId === source.sourceId && c.sourceText !== null ? (
                      <>
                        <CopyText text={c.sourceText} label={t('shell.app.copySource')} />
                        <TextSnapshot
                          key={source?.sourceId}
                          text={c.sourceText}
                          label={t('shell.app.fullSource')}
                        />
                      </>
                    ) : (
                      <p>{t('shell.app.noSource')}</p>
                    )}
                  </div>
                ) : null}
              </div>
            </>
          ) : (
            <SourceEntry controller={c} logo={logo} />
          )}
          {c.notice ? (
            <div
              className={`notice ${c.notice.tone}`}
              role={c.notice.tone === 'error' ? 'alert' : 'status'}
            >
              <span>{text(c.notice.text).slice(0, 3000)}</span>
              {text(c.notice.text).length > 3000 ? (
                <CopyText text={text(c.notice.text)} label={t('shell.app.copyDetails')} />
              ) : null}
            </div>
          ) : null}
          {!c.native ? <p className="notice info">{t('shell.app.browser')}</p> : null}
          {c.capabilities?.ruleError ? (
            <p className="notice error" role="alert">
              {t('shell.app.ruleError', { error: text(c.capabilities.ruleError) })}
            </p>
          ) : null}
          <ToolWindows controller={c} />
        </section>
        {c.workspace ? (
          <aside className="inspector-pane" hidden={!c.inspectorVisible}>
            <PanelResizeHandle kind="inspector" />
            <div className="panel-tabs" role="tablist" aria-label={t('shell.app.inspectorViews')}>
              {(['properties', 'references'] as const).map((key) => (
                <button
                  key={key}
                  type="button"
                  role="tab"
                  aria-selected={c.inspectorTab === key}
                  onClick={() => c.setInspectorTab(key)}
                >
                  {key === 'properties' ? t('shell.app.properties') : t('shell.app.references')}
                </button>
              ))}
              <button
                type="button"
                className="panel-icon-button"
                aria-label={t('shell.app.collapseInspector')}
                title={t('shell.app.collapseInspector')}
                onClick={() => c.setInspectorVisible(false)}
              >
                <ChevronRight size={16} aria-hidden="true" />
              </button>
            </div>
            <div className="inspector-scroll">
              {activeDocument.kind === 'communication' && c.inspectorTab === 'properties' ? (
                <EditorInspector controller={c} />
              ) : (
                <ObjectInspector controller={c} />
              )}
            </div>
          </aside>
        ) : null}
      </main>
      <footer className="statusbar">
        <span>
          {c.projection?.release ??
            c.capabilities?.ruleSetIdentity?.release ??
            t('shell.app.rulesNotReady')}
        </span>
        <span>
          {c.unapplied || c.creating
            ? t('shell.app.unapplied')
            : c.workspace?.dirty || c.projection?.dirty
              ? t('shell.app.unsaved')
              : c.workspace
                ? t('shell.stage.saved')
                : t('shell.app.noProject')}
        </span>
        <span className="toolbar-spacer" />
        <button type="button" onClick={() => c.setToolWindow(c.busy ? 'log' : 'problems')}>
          {c.busy ? t('shell.busy', { operation: text(c.busy) }) : text(c.stages.validate.detail)}
        </button>
        <span>
          {c.legacyTarget} ·{' '}
          {c.capabilities?.nativeExecution
            ? t('shell.app.nativeExecution')
            : t('shell.app.noNativeExecution')}
        </span>
      </footer>
      <SettingsPage controller={c} />
      <PreviewDialogs controller={c} />
      {palette ? (
        <Dialog title={t('shell.app.commandSearch')} onClose={() => setPalette(false)}>
          <label className="search-field">
            <Search size={16} />
            <input
              autoFocus
              data-initial-focus
              value={query}
              aria-label={t('shell.app.searchAvailable')}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t('shell.app.searchPlaceholder')}
            />
          </label>
          <div className="command-results">
            {commands
              .filter((command) => !command.reason && command.label.includes(query))
              .map((command) => (
                <button key={command.id} type="button" onClick={() => dispatchCommand(command.id)}>
                  {command.label}
                  <kbd>{command.shortcut}</kbd>
                </button>
              ))}
          </div>
        </Dialog>
      ) : null}
      {help ? (
        <Dialog title={t('shell.command.help')} onClose={() => setHelp(false)}>
          <h3>{t('shell.help.instructions')}</h3>
          <ol>
            <li>
              <strong>{t('shell.help.openTitle')}</strong>
              {t('shell.help.open')}
            </li>
            <li>
              <strong>{t('shell.help.editTitle')}</strong>
              {t('shell.help.edit')}
            </li>
            <li>
              <strong>{t('shell.help.saveTitle')}</strong>
              {t('shell.help.save')}
            </li>
            <li>
              <strong>{t('shell.help.deliverTitle')}</strong>
              {t('shell.help.deliver')}
            </li>
          </ol>
          <p>
            <strong>{t('shell.help.layoutTitle')}</strong>
            {t('shell.help.layout')}
          </p>
          <h3>{t('shell.help.dependencies')}</h3>
          <p>{t('shell.help.dependencyText')}</p>
          <h3>{t('shell.help.support')}</h3>
          <p>{t('shell.help.coverage')}</p>
          <p>{t('shell.help.can')}</p>
          <p>{t('shell.help.preflight')}</p>
        </Dialog>
      ) : null}
    </div>
  );
}
