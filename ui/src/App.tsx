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
  X,
} from 'lucide-react';
import { useWorkbench } from './workbench/useWorkbench';
import { IntegrationPanel } from './IntegrationPanel';
import { IntegrationDelivery } from './IntegrationDelivery';
import { SettingsPage } from './pages/SettingsPage';
import { ProjectTree } from './workbench/ProjectTree';
import { ObjectEditor, ObjectInspector } from './workbench/ObjectEditor';
import { PreviewDialogs } from './workbench/PreviewDialogs';
import {
  CopyText,
  Dialog,
  NoticeBanner,
  rememberDialogOpener,
  TextSnapshot,
} from './workbench/Dialog';
import { ToolWindows } from './workbench/ToolWindows';
import { WindowControls } from './workbench/WindowControls';
import { PanelResizeHandle } from './workbench/PanelResizeHandle';
import { SourceEntry } from './workbench/SourceEntry';
import { Tabs, tabPanelProps } from './workbench/Tabs';
import { iconSize } from './workbench/icons';
import { labelFromPath } from './workbench/forms';
import { profileLabel, targetLabel, workspaceTitle } from './workbench/labels';
import { menuRows } from './workbench/menu';
import type { Command } from './workbench/menu';
import type { DocumentTab } from './workbench/projectTypes';

const documentLabels = {
  configuration: 'shell.document.configuration',
  integration: 'shell.document.integration',
  delivery: 'shell.document.delivery',
  source: 'shell.document.source',
  'project-entry': 'shell.document.project-entry',
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
  const notice = c.notice;
  const setNotice = c.setNotice;
  useEffect(() => {
    if (notice?.tone !== 'success') return;
    // Success needs no action; errors and warnings stay until dismissed or superseded.
    const timer = window.setTimeout(
      () => setNotice((current) => (current === notice ? null : current)),
      6000,
    );
    return () => window.clearTimeout(timer);
  }, [notice, setNotice]);
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
  const commands: Command[] = [
    {
      id: 'new',
      group: 'open',
      label: t('shell.command.new'),
      menu: 'file',
      reason: text(c.actionReason('create')),
      run: () => c.openProjectEntry('empty'),
    },
    {
      id: 'open',
      group: 'open',
      label: t('shell.command.open'),
      menu: 'file',
      shortcut: 'Ctrl+O',
      reason: text(c.actionReason('open')),
      run: () => c.openProjectEntry('import'),
    },
    {
      id: 'member',
      group: 'open',
      label: t('shell.command.member'),
      menu: 'file',
      reason: c.busy || !c.native ? t('shell.reason.desktopBusy') : '',
      run: c.openMemberProject,
    },
    {
      id: 'ecu-package',
      group: 'open',
      label: t('shell.command.ecuPackage'),
      menu: 'file',
      reason: c.busy || !c.native ? t('shell.reason.desktopBusy') : '',
      run: c.importHandoff,
    },
    {
      id: 'save',
      group: 'save',
      label: t('shell.command.save'),
      menu: 'file',
      shortcut: 'Ctrl+S',
      reason: workspaceReason,
      run: c.requestSave,
    },
    {
      id: 'save-as',
      group: 'save',
      label: t('shell.command.saveAs'),
      menu: 'file',
      reason: workspaceReason || text(c.actionReason('save')),
      run: () => c.openProjectEntry('save-as'),
    },
    {
      id: 'close',
      group: 'close',
      label: t('shell.command.close'),
      menu: 'file',
      reason: !c.workspace ? t('shell.reason.noProject') : text(c.actionReason('open')),
      run: () => c.startProject(),
    },
    {
      id: 'settings',
      group: 'settings',
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
      reason: workspaceReason || (!c.unapplied ? t('shell.reason.noDrafts') : ''),
      run: c.restoreAllDrafts,
    },
    {
      id: 'tree',
      group: 'panels',
      label: t('shell.command.tree'),
      menu: 'view',
      shortcut: 'Alt+1',
      reason: '',
      run: () => c.setTreeVisible((previous) => !previous),
    },
    {
      id: 'inspector',
      group: 'panels',
      label: t('shell.command.inspector'),
      menu: 'view',
      reason: '',
      run: () => c.setInspectorVisible((previous) => !previous),
    },
    {
      id: 'objects',
      group: 'documents',
      label: t('shell.document.configuration'),
      menu: 'view',
      reason: workspaceReason,
      run: () => c.openDocument({ kind: 'configuration' }),
    },
    {
      id: 'standard',
      group: 'documents',
      label: t('shell.document.integration'),
      menu: 'view',
      reason: workspaceReason,
      run: () => c.openDocument({ kind: 'integration' }),
    },
    {
      id: 'delivery',
      group: 'documents',
      label: t('shell.document.delivery'),
      menu: 'view',
      reason: workspaceReason,
      run: () => c.openDocument({ kind: 'delivery' }),
    },
    {
      id: 'validate',
      group: 'run',
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
      group: 'run',
      label: t('shell.command.preview'),
      menu: 'tools',
      reason:
        sourceReason || (!c.workspace?.integrationCandidate ? t('shell.reason.standard') : ''),
      run: () => {
        c.openDocument({ kind: 'delivery' });
        if (!c.ecuOutputDirectory) void c.chooseDirectory(c.changeEcuOutput);
        else c.previewEcu();
      },
    },
    {
      id: 'preflight',
      group: 'run',
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
      group: 'run',
      label: t('shell.command.initialize'),
      menu: 'tools',
      reason:
        sourceReason ||
        text(c.actionReason('edit')) ||
        (!c.workspace?.integrationCandidate ? t('shell.reason.noSlots') : ''),
      run: c.previewApplicationInitialization,
    },
    {
      id: 'problems',
      group: 'tool-windows',
      label: t('shell.command.problems'),
      menu: 'view',
      reason: '',
      run: () => c.setToolWindow('problems'),
    },
    {
      id: 'build',
      group: 'tool-windows',
      label: t('shell.command.build'),
      menu: 'view',
      shortcut: 'Alt+4',
      reason: '',
      run: () => c.setToolWindow('build'),
    },
    {
      id: 'host',
      group: 'tool-windows',
      label: t('shell.command.host'),
      menu: 'view',
      shortcut: 'Alt+9',
      reason: '',
      run: () => c.setToolWindow('host'),
    },
    {
      id: 'cancel',
      group: 'cancel',
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
  // Preview dialogs show the current notice inside themselves (see PreviewDialogs).
  const previewOpen = Boolean(
    c.guard ||
    c.changePreview ||
    c.savePreview ||
    c.integrationPreview ||
    c.generationPreview ||
    c.projectPreview ||
    c.applicationPreview ||
    c.handoffImportOpen,
  );
  const startSurface = !c.workspace || c.activeDocument.kind === 'project-entry';
  const shortcut = useEffectEvent((event: KeyboardEvent) => {
    if (event.isComposing || event.defaultPrevented) return;
    const modal = c.settingsOpen || previewOpen || palette || help;
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
  const tabKey = (tab: DocumentTab) => `${tab.kind}-${tab.sourceId ?? ''}`;
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
                className="ghost-button"
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
                  {menuRows(
                    commands.filter((command) => command.menu === label),
                    dispatchCommand,
                  )}
                </div>
              ) : null}
            </div>
          ))}
        </nav>
        <div className="titlebar-drag-region" data-tauri-drag-region>
          <span className="app-title">{t('shell.app.title')}</span>
        </div>
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
        <strong className="workspace-name" title={c.workspace ? workspaceTitle(c) : undefined}>
          {c.workspace ? workspaceTitle(c) : t('shell.app.noProject')}
        </strong>
        <span className="toolbar-separator" />
        {c.workspace ? (
          <>
            <span>
              {c.projection
                ? profileLabel(c.projection.profile)
                : c.workspace.integrationCandidate
                  ? t('shell.app.standardEcu')
                  : null}
            </span>
            <button
              type="button"
              onClick={() => dispatchCommand('save')}
              disabled={Boolean(workspaceReason)}
            >
              <Save size={iconSize.control} aria-hidden="true" />
              {t('shell.command.save')}
            </button>
            <button
              type="button"
              onClick={() => dispatchCommand('validate')}
              disabled={Boolean(workspaceReason)}
            >
              <ListChecks size={iconSize.control} aria-hidden="true" />
              {t('shell.app.validate')}
            </button>
            <button type="button" onClick={() => dispatchCommand('delivery')}>
              <Code2 size={iconSize.control} aria-hidden="true" />
              {t('shell.document.delivery')}
            </button>
          </>
        ) : null}
        <span className="toolbar-spacer" />
        {c.busy ? (
          <>
            <span className="busy-indicator" aria-hidden="true" />
            <span role="status">{t('shell.busy', { operation: text(c.busy) })}</span>
            <button type="button" onClick={() => dispatchCommand('cancel')}>
              {t('shell.app.cancel')}
            </button>
          </>
        ) : null}
        <button
          type="button"
          className="toolbar-search"
          aria-label={t('shell.app.commandShortcut')}
          onClick={() => {
            setQuery('');
            setPalette(true);
          }}
        >
          <Search size={iconSize.control} aria-hidden="true" />
          {t('shell.app.searchPlaceholder')}
          <kbd>Ctrl+K</kbd>
        </button>
        <button
          type="button"
          className="icon-button"
          aria-label={t('shell.settings.title')}
          title={t('shell.settings.title')}
          onClick={() => dispatchCommand('settings')}
        >
          <Settings size={iconSize.control} aria-hidden="true" />
        </button>
      </div>
      <main className="workspace-frame">
        <nav className="tool-rail" aria-label={t('shell.app.rail')}>
          {c.workspace ? (
            <>
              <button
                type="button"
                className="ghost-button"
                aria-label={t('shell.app.treeShortcut')}
                title={t('shell.app.treeShortcut')}
                aria-pressed={c.treeVisible}
                onClick={() => dispatchCommand('tree')}
              >
                <FolderTree size={iconSize.rail} aria-hidden="true" />
              </button>
              <button
                type="button"
                className="ghost-button"
                aria-label={t('shell.app.problems')}
                title={t('shell.app.problems')}
                aria-pressed={c.toolWindow === 'problems'}
                onClick={() => dispatchCommand('problems')}
              >
                <CircleAlert size={iconSize.rail} aria-hidden="true" />
              </button>
              <button
                type="button"
                className="ghost-button"
                aria-label={t('shell.app.buildShortcut')}
                title={t('shell.app.buildShortcut')}
                aria-pressed={c.toolWindow === 'build'}
                onClick={() => dispatchCommand('build')}
              >
                <Hammer size={iconSize.rail} aria-hidden="true" />
              </button>
              <button
                type="button"
                className="ghost-button"
                aria-label={t('shell.app.hostShortcut')}
                title={t('shell.app.hostShortcut')}
                aria-pressed={c.toolWindow === 'host'}
                onClick={() => dispatchCommand('host')}
              >
                <MonitorPlay size={iconSize.rail} aria-hidden="true" />
              </button>
              <span />
              <button
                type="button"
                className="ghost-button"
                aria-label={t('shell.command.inspector')}
                title={t('shell.command.inspector')}
                aria-pressed={c.inspectorVisible}
                onClick={() => dispatchCommand('inspector')}
              >
                <PanelRight size={iconSize.rail} aria-hidden="true" />
              </button>
            </>
          ) : null}
        </nav>
        {c.workspace ? <ProjectTree controller={c} /> : null}
        <section className="central-workspace">
          {c.workspace ? (
            <>
              <div className="document-tabs">
                {/* Activation reads sources and runs draft guards, so arrows only move focus. */}
                <Tabs
                  id="documents"
                  manual
                  label={t('shell.app.documents')}
                  selected={tabKey(activeDocument)}
                  onSelect={(key) => {
                    const tab = c.tabs.find((item) => tabKey(item) === key);
                    if (!tab) return;
                    if (tab.kind === 'source' && tab.sourceId) void c.readSource(tab.sourceId);
                    else c.openDocument(tab);
                  }}
                  items={c.tabs.map((tab) => ({
                    key: tabKey(tab),
                    label: tabLabel(tab),
                    onClose: () => void c.closeDocument(tab),
                    closeLabel: t('shell.app.closeTab', { tab: tabLabel(tab) }),
                    closeIcon: <X size={iconSize.inline} aria-hidden="true" />,
                  }))}
                />
                <button
                  type="button"
                  className="icon-button"
                  aria-label={t('shell.app.openConfiguration')}
                  title={t('shell.app.openConfiguration')}
                  onClick={() => c.openDocument({ kind: 'configuration' })}
                >
                  <Plus size={iconSize.control} aria-hidden="true" />
                </button>
              </div>
              <div
                className="document-content"
                {...tabPanelProps('documents', tabKey(activeDocument))}
              >
                {activeDocument.kind === 'project-entry' ? (
                  <SourceEntry controller={c} logo={logo} />
                ) : null}
                {activeDocument.kind === 'configuration' ? <ObjectEditor controller={c} /> : null}
                {activeDocument.kind === 'integration' ? <IntegrationPanel controller={c} /> : null}
                {activeDocument.kind === 'delivery' ? (
                  <IntegrationDelivery controller={c} active />
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
          {c.notice && !previewOpen && !c.settingsOpen && !startSurface ? (
            <NoticeBanner notice={c.notice} onDismiss={() => c.setNotice(null)} />
          ) : null}
          {!c.native ? <p className="notice info">{t('shell.app.browser')}</p> : null}
          {c.capabilities?.ruleError ? (
            <p className="notice error" role="alert">
              {t('shell.app.ruleError', { error: text(c.capabilities.ruleError) })}
            </p>
          ) : null}
          {c.workspace ? <ToolWindows controller={c} /> : null}
        </section>
        {c.workspace ? (
          <aside className="inspector-pane" hidden={!c.inspectorVisible}>
            <PanelResizeHandle kind="inspector" />
            <div className="panel-tabs">
              <Tabs
                id="inspector"
                label={t('shell.app.inspectorViews')}
                selected={c.inspectorTab}
                onSelect={c.setInspectorTab}
                items={[
                  { key: 'properties', label: t('shell.app.properties') },
                  { key: 'references', label: t('shell.app.references') },
                ]}
              />
              <button
                type="button"
                className="icon-button"
                aria-label={t('shell.app.collapseInspector')}
                title={t('shell.app.collapseInspector')}
                onClick={() => c.setInspectorVisible(false)}
              >
                <ChevronRight size={iconSize.control} aria-hidden="true" />
              </button>
            </div>
            <div className="inspector-scroll" {...tabPanelProps('inspector', c.inspectorTab)}>
              <ObjectInspector controller={c} />
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
          {c.unapplied
            ? t('shell.app.unapplied')
            : c.workspace?.dirty || c.projection?.dirty
              ? t('shell.app.unsaved')
              : c.workspace
                ? t('shell.stage.saved')
                : t('shell.app.noProject')}
        </span>
        <span className="toolbar-spacer" />
        {c.workspace ? (
          <button
            type="button"
            className="ghost-button"
            onClick={() => c.setToolWindow(c.busy ? 'log' : 'problems')}
          >
            {c.busy
              ? t('shell.busy', { operation: text(c.busy) })
              : c.stages.validate.state === 'pending'
                ? t('shell.app.autoCheck', { count: c.errorCount })
                : text(c.stages.validate.detail)}
          </button>
        ) : null}
        <span>
          {targetLabel(c.buildTarget)} ·{' '}
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
            <Search size={iconSize.control} aria-hidden="true" />
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
                <button
                  key={command.id}
                  type="button"
                  className="ghost-button"
                  onClick={() => dispatchCommand(command.id)}
                >
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
