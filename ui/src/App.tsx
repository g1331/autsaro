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
  configuration: '配置对象',
  communication: 'CAN 通信',
  diagnostic: '诊断配置',
  integration: '标准输入',
  delivery: '源码交付',
  source: '源原文',
  'project-entry': '工程来源',
};
type Command = {
  id: string;
  label: string;
  menu?: '文件' | '编辑' | '视图' | '工具' | '帮助';
  shortcut?: string;
  reason: string;
  run: () => void | Promise<unknown>;
};

export default function App() {
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
    document.documentElement.dataset.theme = effectiveTheme;
    document
      .querySelector<HTMLMetaElement>('meta[name="theme-color"]')
      ?.setAttribute('content', effectiveTheme === 'dark' ? '#1b1b1b' : '#f3f3f3');
    document.querySelector<HTMLLinkElement>('link[rel="icon"]')?.setAttribute('href', logo);
  }, [effectiveTheme, logo]);
  const requestExit = async () => {
    await c.replaceProject('退出工作台', () => getCurrentWindow().destroy());
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
    ? '请先打开工程'
    : c.disabled
      ? c.busy
        ? `正在${c.busy}`
        : (c.capabilities?.ruleError ?? '后台能力尚未就绪')
      : '';
  const sourceReason =
    workspaceReason ||
    (c.workspace?.dirty || c.projection?.dirty ? '请先预览并确认保存工程' : '') ||
    (c.unapplied ? '请先应用或还原草稿' : '');
  const legacyReason =
    workspaceReason || (c.workspace?.integrationCandidate ? '标准输入不使用普通 CAN 创建器' : '');
  const commands: Command[] = [
    {
      id: 'new',
      label: '新建工程…',
      menu: '文件',
      reason: c.actionReason('create'),
      run: () => c.openProjectEntry('empty'),
    },
    {
      id: 'open',
      label: '导入 ARXML…',
      menu: '文件',
      shortcut: 'Ctrl+O',
      reason: c.actionReason('open'),
      run: () => c.openProjectEntry('import'),
    },
    {
      id: 'member',
      label: '打开成员工程…',
      menu: '文件',
      reason: c.busy || !c.native ? '后台繁忙或非桌面环境' : '',
      run: c.openMemberProject,
    },
    {
      id: 'host-package',
      label: '导入可重建主机交付包…',
      menu: '文件',
      reason: c.busy || !c.native ? '后台繁忙或非桌面环境' : '',
      run: c.importHandoff,
    },
    {
      id: 'ecu-package',
      label: '重导入 ECU 交接包…',
      menu: '文件',
      reason: c.busy || !c.native ? '后台繁忙或非桌面环境' : '',
      run: () => c.chooseDirectory((directory) => c.importHandoffDirectory(directory)),
    },
    {
      id: 'save',
      label: '预览保存',
      menu: '文件',
      shortcut: 'Ctrl+S',
      reason: workspaceReason,
      run: c.requestSave,
    },
    {
      id: 'save-as',
      label: '保存为成员工程…',
      menu: '文件',
      reason: workspaceReason || c.actionReason('save'),
      run: () => c.openProjectEntry('save-as'),
    },
    {
      id: 'close',
      label: '关闭工程',
      menu: '文件',
      reason: !c.workspace ? '请先打开工程' : c.actionReason('open'),
      run: () => c.startProject(),
    },
    {
      id: 'settings',
      label: '设置…',
      menu: '文件',
      reason: '',
      run: () => c.setSettingsOpen(true),
    },
    {
      id: 'apply',
      label: '应用当前草稿',
      menu: '编辑',
      reason: workspaceReason || (!c.unapplied ? '没有未应用更改' : ''),
      run: c.applyCurrentDrafts,
    },
    {
      id: 'restore',
      label: '还原未应用草稿',
      menu: '编辑',
      reason: workspaceReason || (!c.unapplied && !c.creating ? '没有未应用草稿' : ''),
      run: c.restoreAllDrafts,
    },
    {
      id: 'frame',
      label: '添加 CAN 帧',
      menu: '编辑',
      reason: legacyReason,
      run: () => c.openCreator('frame'),
    },
    {
      id: 'signal',
      label: '添加 CAN 信号',
      menu: '编辑',
      reason: legacyReason || (!c.focusedFrame ? '请先选择所属帧' : ''),
      run: () => c.openCreator('signal'),
    },
    {
      id: 'tree',
      label: '工程树',
      menu: '视图',
      shortcut: 'Alt+1',
      reason: '',
      run: () => c.setTreeVisible((previous) => !previous),
    },
    {
      id: 'inspector',
      label: '检查器',
      menu: '视图',
      reason: '',
      run: () => c.setInspectorVisible((previous) => !previous),
    },
    {
      id: 'objects',
      label: '配置对象',
      menu: '视图',
      reason: workspaceReason,
      run: () => c.openDocument({ kind: 'configuration' }),
    },
    {
      id: 'communication',
      label: 'CAN 通信',
      menu: '视图',
      reason: legacyReason,
      run: () => c.openDocument({ kind: 'communication' }),
    },
    {
      id: 'diagnostic',
      label: '诊断配置',
      menu: '视图',
      reason: legacyReason,
      run: () => c.openDocument({ kind: 'diagnostic' }),
    },
    {
      id: 'standard',
      label: '标准输入',
      menu: '视图',
      reason: workspaceReason,
      run: () => c.openDocument({ kind: 'integration' }),
    },
    {
      id: 'delivery',
      label: '源码交付',
      menu: '视图',
      reason: workspaceReason,
      run: () => c.openDocument({ kind: 'delivery' }),
    },
    {
      id: 'validate',
      label: c.workspace?.integrationCandidate ? '检查标准输入' : '校验工程',
      menu: '工具',
      reason: workspaceReason,
      run: () =>
        c.guardContext('校验工程', () =>
          c.workspace?.integrationCandidate ? c.inspectIntegration() : c.validateProject(),
        ),
    },
    {
      id: 'preview',
      label: '预览源码…',
      menu: '工具',
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
      label: '显式编译预检',
      menu: '工具',
      reason:
        sourceReason ||
        c.executionReason ||
        (!c.workspace?.integrationCandidate ? '此入口适用于标准 ECU' : ''),
      run: c.preflightEcu,
    },
    {
      id: 'initialize-application',
      label: '预览初始化用户应用…',
      menu: '工具',
      reason:
        sourceReason ||
        c.actionReason('edit') ||
        (!c.workspace?.integrationCandidate ? '当前 CAN 剖面没有用户应用槽' : ''),
      run: c.previewApplicationInitialization,
    },
    {
      id: 'build',
      label: '构建窗口',
      menu: '工具',
      shortcut: 'Alt+4',
      reason: '',
      run: () => c.setToolWindow('build'),
    },
    {
      id: 'host',
      label: '主机验证窗口',
      menu: '工具',
      shortcut: 'Alt+9',
      reason: '',
      run: () => c.setToolWindow('host'),
    },
    {
      id: 'problems',
      label: '问题窗口',
      menu: '工具',
      reason: '',
      run: () => c.setToolWindow('problems'),
    },
    {
      id: 'cancel',
      label: '取消当前操作',
      menu: '工具',
      reason: !c.busy ? '没有运行中的操作' : '',
      run: c.cancelOperation,
    },
    { id: 'help', label: '支持范围与依赖', menu: '帮助', reason: '', run: () => setHelp(true) },
  ];
  if (c.capabilities?.verificationMode === true) {
    const reason = !c.native ? '需要桌面运行环境' : c.busy ? `正在${c.busy}` : '';
    commands.push(
      {
        id: 'verification-owned-long-failure',
        label: '验证：受管长日志失败',
        reason,
        run: () => c.verificationOwnedFailure(0),
      },
      {
        id: 'verification-owned-cancel',
        label: '验证：受管取消',
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
            documentLabels.source,
        )
      : documentLabels[tab.kind];
  return (
    <div className="app-shell">
      <header className="menubar" data-tauri-drag-region>
        <img
          src={logo}
          width="27"
          height="27"
          alt="Classic CAN 配置工作台"
          data-tauri-drag-region
        />
        <nav aria-label="应用菜单">
          {(['文件', '编辑', '视图', '工具', '帮助'] as const).map((label) => (
            <div className="menu-root" key={label}>
              <button
                type="button"
                aria-expanded={menu === label}
                aria-haspopup="menu"
                onClick={() => setMenu(menu === label ? null : label)}
              >
                {label}
              </button>
              {menu === label ? (
                <div
                  className="menu-popup"
                  role="menu"
                  aria-label={label}
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
          <span className="app-title">Classic CAN 配置工作台</span>
        </div>
        <button
          type="button"
          onClick={() => {
            setQuery('');
            setPalette(true);
          }}
          aria-label="命令搜索 Ctrl+K"
        >
          <Search size={16} aria-hidden="true" />
        </button>
        {c.native ? (
          <WindowControls
            onClose={requestExit}
            onError={(error) =>
              c.setNotice({ tone: 'error', text: `窗口操作失败：${String(error)}` })
            }
          />
        ) : null}
      </header>
      <div className="context-toolbar">
        <strong>{c.workspace?.name ?? '未打开工程'}</strong>
        <span className="toolbar-separator" />
        {c.workspace ? (
          <>
            <span>
              {c.projection?.profile ??
                (c.workspace.integrationCandidate ? '标准 ECU' : 'CAN 信号')}
            </span>
            <button
              type="button"
              onClick={() => dispatchCommand('save')}
              disabled={Boolean(workspaceReason)}
            >
              <Save size={15} aria-hidden="true" />
              预览保存
            </button>
            <button
              type="button"
              onClick={() => dispatchCommand('validate')}
              disabled={Boolean(workspaceReason)}
            >
              <ListChecks size={15} aria-hidden="true" />
              校验
            </button>
            <button type="button" onClick={() => dispatchCommand('delivery')}>
              <Code2 size={15} aria-hidden="true" />
              源码交付
            </button>
          </>
        ) : null}
        <span className="toolbar-spacer" />
        {c.busy ? (
          <>
            <span role="status">正在{c.busy}</span>
            <button type="button" onClick={() => dispatchCommand('cancel')}>
              <Square size={13} aria-hidden="true" />
              取消
            </button>
          </>
        ) : null}
        <button type="button" aria-label="设置" onClick={() => dispatchCommand('settings')}>
          <Settings size={16} aria-hidden="true" />
        </button>
      </div>
      <main className="workspace-frame">
        <nav className="tool-rail" aria-label="工作台工具轨">
          <button
            type="button"
            aria-label="工程树 Alt+1"
            title="工程树 Alt+1"
            aria-pressed={c.treeVisible}
            onClick={() => dispatchCommand('tree')}
          >
            <FolderTree size={19} />
          </button>
          <button
            type="button"
            aria-label="问题"
            title="问题"
            aria-pressed={c.toolWindow === 'problems'}
            onClick={() => dispatchCommand('problems')}
          >
            <CircleAlert size={19} />
          </button>
          <button
            type="button"
            aria-label="构建 Alt+4"
            title="构建 Alt+4"
            aria-pressed={c.toolWindow === 'build'}
            onClick={() => dispatchCommand('build')}
          >
            <Hammer size={19} />
          </button>
          <button
            type="button"
            aria-label="主机验证 Alt+9"
            title="主机验证 Alt+9"
            aria-pressed={c.toolWindow === 'host'}
            onClick={() => dispatchCommand('host')}
          >
            <MonitorPlay size={19} />
          </button>
          <span />
          <button
            type="button"
            aria-label="检查器"
            title="检查器"
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
              <div className="document-tabs" role="tablist" aria-label="工程文档">
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
                      aria-label={`关闭 ${tabLabel(tab)}`}
                      onClick={() => void c.closeDocument(tab)}
                    >
                      <X size={12} />
                    </button>
                  </div>
                ))}
                <button
                  type="button"
                  aria-label="打开配置对象文档"
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
                    <p className="mono path-text">{source?.path}</p>
                    {source ? <CopyText text={source.path} label="复制源路径" /> : null}
                    <p>只读原文 · {source?.readonly ? '来源只读' : '通过配置事务修改'}</p>
                    {source && c.activeSourceId === source.sourceId && c.sourceText !== null ? (
                      <>
                        <CopyText text={c.sourceText} label="复制完整原文" />
                        <TextSnapshot
                          key={source?.sourceId}
                          text={c.sourceText}
                          label="完整源原文"
                        />
                      </>
                    ) : (
                      <p>原文未读取。</p>
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
              <span>{c.notice.text.slice(0, 3000)}</span>
              {c.notice.text.length > 3000 ? (
                <CopyText text={c.notice.text} label="复制完整详情" />
              ) : null}
            </div>
          ) : null}
          {!c.native ? (
            <p className="notice info">需要桌面运行环境；浏览器不提供文件、内核或 ECU 执行能力。</p>
          ) : null}
          {c.capabilities?.ruleError ? (
            <p className="notice error" role="alert">
              内置规则安装错误：{c.capabilities.ruleError}
            </p>
          ) : null}
          <ToolWindows controller={c} />
        </section>
        {c.workspace ? (
          <aside className="inspector-pane" hidden={!c.inspectorVisible}>
            <PanelResizeHandle kind="inspector" />
            <div className="panel-tabs" role="tablist" aria-label="检查器视图">
              {(['properties', 'references'] as const).map((key) => (
                <button
                  key={key}
                  type="button"
                  role="tab"
                  aria-selected={c.inspectorTab === key}
                  onClick={() => c.setInspectorTab(key)}
                >
                  {key === 'properties' ? '属性' : '引用'}
                </button>
              ))}
              <button
                type="button"
                className="panel-icon-button"
                aria-label="折叠检查器"
                title="折叠检查器"
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
          {c.projection?.release ?? c.capabilities?.ruleSetIdentity?.release ?? '内置规则尚未就绪'}
        </span>
        <span>
          {c.unapplied || c.creating
            ? '未应用草稿'
            : c.workspace?.dirty || c.projection?.dirty
              ? '未保存'
              : c.workspace
                ? '已保存'
                : '未打开工程'}
        </span>
        <span className="toolbar-spacer" />
        <button type="button" onClick={() => c.setToolWindow(c.busy ? 'log' : 'problems')}>
          {c.busy ? `正在${c.busy}` : c.stages.validate.detail}
        </button>
        <span>
          {c.legacyTarget} · {c.capabilities?.nativeExecution ? '本机可执行' : '本机不适用执行'}
        </span>
      </footer>
      <SettingsPage controller={c} />
      <PreviewDialogs controller={c} />
      {palette ? (
        <Dialog title="命令搜索" onClose={() => setPalette(false)}>
          <label className="search-field">
            <Search size={16} />
            <input
              autoFocus
              data-initial-focus
              value={query}
              aria-label="搜索当前可用命令"
              onChange={(event) => setQuery(event.target.value)}
              placeholder="搜索命令"
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
        <Dialog title="支持范围与依赖" onClose={() => setHelp(false)}>
          <h3>使用说明</h3>
          <ol>
            <li>
              <strong>打开工程：</strong>
              在“文件”菜单中新建工程、导入 ARXML
              或打开已有成员工程。新建时选择内置工程模板和新的空目录。
            </li>
            <li>
              <strong>编辑配置：</strong>
              在工程树中选择对象，在“属性”中编辑可写参数；查看整批修改的影响后应用。只读对象可查看原文和引用。
            </li>
            <li>
              <strong>检查与保存：</strong>
              使用“校验”查看问题并定位对象；使用“预览保存”检查文件差异，确认后才写入磁盘。未应用草稿不会自动保存。
            </li>
            <li>
              <strong>交付源码：</strong>
              打开“源码交付”，选择目标和独立输出目录，先预览再确认生成。生成完成不等于编译或运行验证通过。
            </li>
          </ol>
          <p>
            <strong>面板布局：</strong>
            拖动工程树与检查器的内侧边缘调整宽度，拖动底部工具窗口的上边缘调整高度。
            方向键微调，Shift＋方向键加大步幅；双击或 Enter 恢复默认。尺寸在当前运行期间保留。
          </p>
          <h3>工具依赖</h3>
          <p>
            普通配置编辑、保存和源码准备不需要编译器。构建或主机验证前，在“设置”的执行工具中配置所选目标需要的工具；
            工具未就绪时仍可继续编辑配置。
          </p>
          <h3>支持范围</h3>
          <p>R24-11 原生结构、定义与目标生成分别报告覆盖；不声明完整官方 XSD、SWS 或实机认证。</p>
          <p>
            CAN 信号使用 11 位 Classical CAN、有界 DoCAN / DID 与可选单 DTC；标准 ECU 保留当前受限
            CAN ID 与应用周期编辑，类型、端口和映射只读。
          </p>
          <p>
            源码预览不要求编译器。预检、构建、主机行为验证遵守当前工具、宿主和受控目标；存在旧二进制不证明本次通过。
          </p>
        </Dialog>
      ) : null}
    </div>
  );
}
