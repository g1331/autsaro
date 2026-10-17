import { message, useLocale } from './i18n';
import { displayPath } from './pathDisplay';
import { useEffect, useEffectEvent, useId, useRef, useState } from 'react';
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
import { ObjectInspector } from './workbench/ObjectEditor';
import { ConfigurationDocument } from './workbench/ContainerView';
import { PreviewDialogs } from './workbench/PreviewDialogs';
import {
  CopyText,
  Dialog,
  NoticeBanner,
  rememberDialogOpener,
  TextSnapshot,
} from './workbench/Dialog';
import { CommandPalette } from './workbench/CommandPalette';
import { ToolWindows } from './workbench/ToolWindows';
import { WindowControls } from './workbench/WindowControls';
import { PanelResizeHandle } from './workbench/PanelResizeHandle';
import { SourceEntry } from './workbench/SourceEntry';
import { Tabs, tabPanelProps } from './workbench/Tabs';
import { iconSize } from './workbench/icons';
import { labelFromPath } from './workbench/forms';
import { profileLabel, targetLabel, workspaceTitle } from './workbench/labels';
import { WorkbenchMenubar } from './workbench/menu';
import type { Command } from './workbench/commands';
import { CommandButton, CommandReasons, reasonIdFor } from './workbench/CommandActions';
import { Tooltip } from './workbench/Tooltip';
import type { DocumentTab } from './workbench/projectTypes';

// Fields whose keys belong to text entry; global shortcuts leave them alone.
const editableFields =
  'textarea, select, [contenteditable="true"], input:not([type="checkbox"]):not([type="radio"]):not([type="button"]):not([type="submit"]):not([type="reset"]):not([type="range"]):not([type="color"]):not([type="file"])';
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
  const [palette, setPalette] = useState(false);
  const [help, setHelp] = useState(false);
  const [systemDark, setSystemDark] = useState(
    () => window.matchMedia('(prefers-color-scheme: dark)').matches,
  );
  const toolOpener = useRef<HTMLElement | null>(null);
  const toolbarReasonId = useId();
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
    await c.replaceProject(message('activity.exit'), () => getCurrentWindow().destroy());
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
  const commands: Command[] = [
    ...c.commands,
    {
      id: 'help',
      label: message('shell.command.help'),
      menu: 'help',
      reason: '',
      run: () => setHelp(true),
    },
  ];
  const command = (id: string) => commands.find((entry) => entry.id === id)!;
  const busyStatus = c.busy ? t('shell.busy', { operation: text(c.busy) }) : '';
  // While busy, the toolbar's own status already states the reason.
  const toolbarCommands = ['save', 'validate', 'delivery']
    .map(command)
    .filter((entry) => text(entry.reason) !== busyStatus);
  const toolbarReason = reasonIdFor(toolbarCommands, toolbarReasonId, text);
  const dispatchCommand = (id: string) => {
    const entry = commands.find((item) => item.id === id);
    if (!entry || entry.reason) return;
    rememberDialogOpener();
    setPalette(false);
    void entry.run();
  };
  // Preview dialogs show the current notice inside themselves (see PreviewDialogs).
  const previewOpen = Boolean(
    c.guard ||
    c.catalogConfirmation ||
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
    // Keys typed into a field belong to the field, Escape included (open menus and dialogs handle their own).
    if (event.target instanceof Element && event.target.closest(editableFields)) return;
    const modal = c.settingsOpen || previewOpen || palette || help;
    if (event.key === 'Escape') {
      if (modal || !c.toolWindow) return;
      event.preventDefault();
      c.setToolWindow(null);
      toolOpener.current?.focus();
      return;
    }
    if (modal) return;
    // `code` is the physical key, so shortcuts also work with non-Latin keyboard layouts.
    if ((event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey) {
      if (event.code === 'KeyK') {
        event.preventDefault();
        rememberDialogOpener();
        setPalette(true);
      } else if (event.code === 'KeyS' || event.code === 'KeyO') {
        event.preventDefault();
        dispatchCommand(event.code === 'KeyS' ? 'save' : 'open');
      }
    } else if (event.altKey && !event.ctrlKey && !event.metaKey) {
      const id = ({ Digit1: 'tree', Digit4: 'build', Digit9: 'host' } as Record<string, string>)[
        event.code
      ];
      if (id) {
        event.preventDefault();
        toolOpener.current =
          document.activeElement instanceof HTMLElement ? document.activeElement : null;
        dispatchCommand(id);
      }
    }
  });
  useEffect(() => {
    window.addEventListener('keydown', shortcut);
    return () => window.removeEventListener('keydown', shortcut);
  }, []);
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
        <img
          src={logo}
          width="27"
          height="27"
          alt={t('controller.app.title')}
          data-tauri-drag-region
        />
        <WorkbenchMenubar label={t('shell.app.menu')} commands={commands} run={dispatchCommand} />
        <div className="titlebar-drag-region" data-tauri-drag-region>
          <span className="app-title">{t('controller.app.title')}</span>
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
            {(
              [
                ['save', Save],
                ['validate', ListChecks],
                ['delivery', Code2],
              ] as const
            ).map(([id, Icon]) => (
              <CommandButton
                key={id}
                command={command(id)}
                run={() => dispatchCommand(id)}
                reasonId={
                  text(command(id).reason) === busyStatus
                    ? 'operation-status'
                    : toolbarReason(command(id))
                }
              >
                <Icon size={iconSize.control} aria-hidden="true" />
                {text(command(id).label)}
              </CommandButton>
            ))}
            <CommandReasons
              commands={toolbarCommands}
              id={toolbarReasonId}
              className="toolbar-reasons"
            />
          </>
        ) : null}
        <span className="toolbar-spacer" />
        {c.busy ? (
          <>
            <span className="busy-indicator" aria-hidden="true" />
            <span role="status" id="operation-status">
              {busyStatus}
            </span>
            <button type="button" onClick={() => dispatchCommand('cancel')}>
              {t('action.cancelOperation')}
            </button>
          </>
        ) : null}
        <button
          type="button"
          className="toolbar-search"
          onClick={() => {
            rememberDialogOpener();
            setPalette(true);
          }}
        >
          <Search size={iconSize.control} aria-hidden="true" />
          {t('shell.app.searchPlaceholder')}
          <kbd>Ctrl+K</kbd>
        </button>
        <Tooltip label={t('shell.settings.title')}>
          <button
            type="button"
            className="icon-button"
            aria-label={t('shell.settings.title')}
            onClick={() => dispatchCommand('settings')}
          >
            <Settings size={iconSize.control} aria-hidden="true" />
          </button>
        </Tooltip>
      </div>
      <main className="workspace-frame">
        <nav className="tool-rail" aria-label={t('shell.app.rail')}>
          {c.workspace ? (
            <>
              <Tooltip label={t('shell.app.treeShortcut')}>
                <button
                  type="button"
                  className="ghost-button"
                  aria-label={t('shell.app.treeShortcut')}
                  aria-pressed={c.treeVisible}
                  onClick={() => dispatchCommand('tree')}
                >
                  <FolderTree size={iconSize.rail} aria-hidden="true" />
                </button>
              </Tooltip>
              <Tooltip label={t('shell.app.problems')}>
                <button
                  type="button"
                  className="ghost-button"
                  aria-label={t('shell.app.problems')}
                  aria-pressed={c.toolWindow === 'problems'}
                  onClick={() => dispatchCommand('problems')}
                >
                  <CircleAlert size={iconSize.rail} aria-hidden="true" />
                </button>
              </Tooltip>
              <Tooltip label={t('shell.app.buildShortcut')}>
                <button
                  type="button"
                  className="ghost-button"
                  aria-label={t('shell.app.buildShortcut')}
                  aria-pressed={c.toolWindow === 'build'}
                  onClick={() => dispatchCommand('build')}
                >
                  <Hammer size={iconSize.rail} aria-hidden="true" />
                </button>
              </Tooltip>
              <Tooltip label={t('shell.app.hostShortcut')}>
                <button
                  type="button"
                  className="ghost-button"
                  aria-label={t('shell.app.hostShortcut')}
                  aria-pressed={c.toolWindow === 'host'}
                  onClick={() => dispatchCommand('host')}
                >
                  <MonitorPlay size={iconSize.rail} aria-hidden="true" />
                </button>
              </Tooltip>
              <span />
              <Tooltip label={t('shell.command.inspector')}>
                <button
                  type="button"
                  className="ghost-button"
                  aria-label={t('shell.command.inspector')}
                  aria-pressed={c.inspectorVisible}
                  onClick={() => dispatchCommand('inspector')}
                >
                  <PanelRight size={iconSize.rail} aria-hidden="true" />
                </button>
              </Tooltip>
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
                <Tooltip label={t('shell.app.openConfiguration')}>
                  <button
                    type="button"
                    className="icon-button"
                    aria-label={t('shell.app.openConfiguration')}
                    onClick={() => c.openDocument({ kind: 'configuration' })}
                  >
                    <Plus size={iconSize.control} aria-hidden="true" />
                  </button>
                </Tooltip>
              </div>
              <div
                className="document-content"
                {...tabPanelProps('documents', tabKey(activeDocument))}
              >
                {activeDocument.kind === 'project-entry' ? (
                  <SourceEntry controller={c} logo={logo} />
                ) : null}
                {activeDocument.kind === 'configuration' ? (
                  <ConfigurationDocument controller={c} />
                ) : null}
                {activeDocument.kind === 'integration' ? <IntegrationPanel controller={c} /> : null}
                {activeDocument.kind === 'delivery' ? (
                  <IntegrationDelivery controller={c} active />
                ) : null}
                {activeDocument.kind === 'source' ? (
                  <div className="source-document">
                    <header className="source-heading">
                      <h2>{labelFromPath(source?.path ?? '')}</h2>
                      <span className="mono source-path" title={displayPath(source?.path ?? '')}>
                        {displayPath(source?.path ?? '')}
                      </span>
                      {source ? (
                        <CopyText text={displayPath(source.path)} label={t('action.copyPath')} />
                      ) : null}
                      <span className="role-tag">{t('editor.common.readonly')}</span>
                    </header>
                    <p className="field-help">
                      {source?.readonly
                        ? t('shell.app.sourceReadonly')
                        : t('shell.app.sourceTransaction')}
                    </p>
                    {source && c.activeSourceId === source.sourceId && c.sourceText !== null ? (
                      <TextSnapshot
                        key={source?.sourceId}
                        text={c.sourceText}
                        label={t('shell.app.fullSource')}
                      />
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
            <NoticeBanner
              notice={c.notice}
              onDismiss={() => c.setNotice(null)}
              onDetails={
                c.notice.rejected && c.rejection
                  ? () => {
                      c.setToolWindow('problems');
                      requestAnimationFrame(() =>
                        document.getElementById('rejection-details')?.focus(),
                      );
                    }
                  : undefined
              }
            />
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
              <Tooltip label={t('shell.app.collapseInspector')}>
                <button
                  type="button"
                  className="icon-button"
                  aria-label={t('shell.app.collapseInspector')}
                  onClick={() => c.setInspectorVisible(false)}
                >
                  <ChevronRight size={iconSize.control} aria-hidden="true" />
                </button>
              </Tooltip>
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
        <CommandPalette
          commands={commands}
          onRun={dispatchCommand}
          onClose={() => setPalette(false)}
        />
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
          <p>{t('shell.help.inputs')}</p>
          <p>{t('shell.help.extensions')}</p>
          <p>{t('shell.help.application')}</p>
          <p>{t('shell.help.preflight')}</p>
        </Dialog>
      ) : null}
    </div>
  );
}
