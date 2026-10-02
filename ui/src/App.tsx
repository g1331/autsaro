import { useWorkbench } from './workbench/useWorkbench';
import { IntegrationDelivery } from './IntegrationDelivery';
import { EditorPage, EditorInspector } from './pages/EditorPage';
import { DiagnosticsPage } from './pages/DiagnosticsPage';
import { BuildPage } from './pages/BuildPage';
import { VirtualPage } from './pages/VirtualPage';
import { IntegrationPage } from './pages/IntegrationPage';
import { SettingsPage } from './pages/SettingsPage';
import {
  ArrowRight,
  Cable,
  CircleAlert,
  FileCode2,
  FileInput,
  FolderOpen,
  FolderPlus,
  HardDrive,
  ListChecks,
  MonitorPlay,
  Plus,
  Waypoints,
} from 'lucide-react';
import { labelFromPath } from './workbench/forms';

export default function App() {
  const controller = useWorkbench();
  const {
    workspace,
    integrationProcessing,
    selection,
    source,
    setSource,
    projectName,
    setProjectName,
    projectDirectory,
    setProjectDirectory,
    importPaths,
    setImportPaths,
    importPathText,
    setImportPathText,
    generationPreview,
    setGenerationPreview,
    generationPreviewPath,
    setGenerationPreviewPath,
    generationKind,
    notice,
    busy,
    page,
    setPage,
    savePreview,
    setSavePreview,
    previewPath,
    setPreviewPath,
    issues,
    chooseDirectory,
    chooseFiles,
    choose,
    openCreator,
    startProject,
    createProject,
    importProject,
    importHandoff,
    confirmSave,
    confirmGenerate,
    previewFile,
    delta,
    generationPreviewFile,
    generationDelta,
    disabled,
    native,
  } = controller;
  return (
    <div className="app-shell">
      <div className="section-actions">
        <button
          className="quiet-button"
          type="button"
          onClick={() => controller.setSettingsOpen(true)}
        >
          工作台设置
        </button>
        {busy && (
          <button
            className="quiet-button"
            type="button"
            onClick={() => void controller.cancelOperation()}
          >
            取消当前操作
          </button>
        )}
        <span role="status">
          {controller.capabilities?.resourceError
            ? '规范档案未就绪'
            : controller.capabilities
              ? '规范档案已就绪'
              : '正在读取工作台能力'}
        </span>
      </div>
      {!workspace ? (
        <main className="source-page">
          <div className="source-layout">
            <section className="source-primary" aria-label="项目来源">
              <div className="source-head">
                <img src="/workbench.svg" alt="Classic CAN 配置工作台" width="64" height="64" />
                <p className="eyebrow">AUTOSAR CLASSIC / R24-11</p>
                <h1>配置项目</h1>
                <p>从空项目开始，或导入同一 ECU 的多份 ARXML。配置始终保存在原始文件集合中。</p>
              </div>
              <div className="source-switch" role="group" aria-label="选择项目来源">
                <button
                  type="button"
                  className={source === 'empty' ? 'selected' : ''}
                  onClick={() => setSource('empty')}
                >
                  <FolderPlus aria-hidden="true" size={17} />
                  新建空项目
                </button>
                <button
                  type="button"
                  className={source === 'import' ? 'selected' : ''}
                  onClick={() => setSource('import')}
                >
                  <FileInput aria-hidden="true" size={17} />
                  导入 ARXML
                </button>
              </div>
              <section
                className="source-body"
                aria-label={source === 'empty' ? '新建项目' : '导入项目'}
              >
                {source === 'empty' ? (
                  <>
                    <div className="source-fields">
                      <label>
                        项目名称
                        <input
                          value={projectName}
                          onChange={(event) => setProjectName(event.target.value)}
                          placeholder="例如：Powertrain_ECU"
                          autoComplete="off"
                        />
                      </label>
                      <label>
                        项目目录
                        <div className="path-picker">
                          <input
                            readOnly
                            value={projectDirectory}
                            placeholder="选择保存 ARXML 的目录"
                            aria-label="项目目录"
                          />
                          <button
                            type="button"
                            onClick={() => void chooseDirectory(setProjectDirectory)}
                            disabled={disabled}
                          >
                            <FolderOpen aria-hidden="true" size={15} />
                            选择目录
                          </button>
                        </div>
                      </label>
                      <p className="field-help">
                        生成的 C99 工程稍后选择独立输出目录，不覆盖项目源文件。
                      </p>
                      <button
                        type="button"
                        className="primary-button"
                        onClick={createProject}
                        disabled={disabled}
                      >
                        {busy === '创建项目' ? '创建中…' : '创建并进入工作区'}{' '}
                        <ArrowRight aria-hidden="true" size={16} />
                      </button>
                    </div>
                  </>
                ) : (
                  <>
                    <div className="source-fields">
                      <p className="import-copy">
                        选择同一项目的全部 ARXML。保留未支持的内容；无法安全编辑时内核会拒绝操作。
                      </p>
                      <button
                        type="button"
                        className="outline-button"
                        onClick={() => void chooseFiles()}
                        disabled={disabled}
                      >
                        <FileInput aria-hidden="true" size={16} />
                        选择多份 .arxml 文件
                      </button>
                      <button
                        type="button"
                        className="outline-button"
                        onClick={importHandoff}
                        disabled={disabled}
                      >
                        导入可重建主机交付包
                      </button>
                      <div className="import-list" aria-live="polite">
                        {importPaths.length ? (
                          importPaths.map((path) => (
                            <div key={path}>
                              <FileCode2 aria-hidden="true" size={15} />
                              <span title={path}>{path}</span>
                            </div>
                          ))
                        ) : (
                          <p>尚未选择文件</p>
                        )}
                      </div>
                      <details>
                        <summary>直接填写来源路径</summary>
                        <label className="field">
                          <span>每行一份 ARXML 的完整路径</span>
                          <textarea
                            aria-label="ARXML 来源路径"
                            rows={7}
                            value={importPathText}
                            onChange={(event) => {
                              setImportPathText(event.target.value);
                              setImportPaths(
                                event.target.value
                                  .split(/\r?\n/)
                                  .map((line) => line.trim())
                                  .filter(Boolean),
                              );
                            }}
                            disabled={disabled}
                          />
                        </label>
                      </details>
                      <button
                        type="button"
                        className="primary-button"
                        onClick={importProject}
                        disabled={disabled || !importPaths.length}
                      >
                        {busy === '导入项目' ? '导入中…' : `导入 ${importPaths.length} 份文件`}{' '}
                        <ArrowRight aria-hidden="true" size={16} />
                      </button>
                    </div>
                  </>
                )}
              </section>
            </section>
          </div>
          {!native && (
            <p className="environment-warning" role="status">
              需要桌面运行环境。浏览器预览不可选择本地文件、保存、生成、构建或运行。
            </p>
          )}
          {notice && (
            <p className={`notice ${notice.tone}`} role="alert">
              {notice.text}
            </p>
          )}
        </main>
      ) : (
        <main className="workbench">
          <aside className="project-rail">
            <div className="workspace-heading">
              <img src="/workbench.svg" alt="" aria-hidden="true" width="32" height="32" />
              <div>
                <p className="eyebrow">当前项目</p>
                <h1 title={workspace.name}>{workspace.name}</h1>
                <p className={workspace.dirty ? 'dirty-label' : ''}>
                  {workspace.dirty ? '未保存修改' : '配置已保存'}
                </p>
              </div>
              <button
                type="button"
                className="quiet-button"
                aria-label="切换项目"
                title="切换项目"
                onClick={startProject}
                disabled={Boolean(busy) || integrationProcessing}
              >
                <FolderOpen aria-hidden="true" size={16} />
              </button>
            </div>
            <nav className="project-nav" aria-label="项目工作页">
              <button
                type="button"
                className={page === 'integration' ? 'active' : ''}
                aria-current={page === 'integration' ? 'page' : undefined}
                onClick={() => setPage('integration')}
                disabled={integrationProcessing}
              >
                <ListChecks aria-hidden="true" size={16} />
                标准输入{workspace.dirty && <span className="nav-alert">未保存</span>}
              </button>
              <button
                type="button"
                className={page === 'editor' ? 'active' : ''}
                aria-current={page === 'editor' ? 'page' : undefined}
                disabled={Boolean(workspace.integrationCandidate) || integrationProcessing}
                onClick={() => setPage('editor')}
              >
                <Cable aria-hidden="true" size={16} />
                配置{workspace.dirty && <span className="nav-alert">未保存</span>}
              </button>
              <button
                type="button"
                className={page === 'diagnostics' ? 'active' : ''}
                aria-current={page === 'diagnostics' ? 'page' : undefined}
                disabled={Boolean(workspace.integrationCandidate) || integrationProcessing}
                onClick={() => setPage('diagnostics')}
              >
                <CircleAlert aria-hidden="true" size={16} />
                诊断
                {!workspace.integrationCandidate && issues.length > 0 && (
                  <span className="nav-count">{issues.length}</span>
                )}
              </button>
              <button
                type="button"
                className={page === 'build' ? 'active' : ''}
                aria-current={page === 'build' ? 'page' : undefined}
                disabled={integrationProcessing}
                onClick={() => setPage('build')}
              >
                <HardDrive aria-hidden="true" size={16} />
                生成与构建
              </button>
              <button
                type="button"
                className={page === 'virtual' ? 'active' : ''}
                aria-current={page === 'virtual' ? 'page' : undefined}
                disabled={integrationProcessing}
                onClick={() => setPage('virtual')}
              >
                <MonitorPlay aria-hidden="true" size={16} />
                虚拟运行
              </button>
            </nav>
            {page === 'editor' && (
              <nav className="tree-pane" aria-label="工程树">
                <div className="tree-section-title">
                  项目文件 <span>{workspace.files.length}</span>
                </div>
                <div className="tree-items">
                  {workspace.files.map((file) => (
                    <button
                      key={file.path}
                      type="button"
                      className={`tree-item ${selection?.kind === 'file' && selection.path === file.path ? 'active' : ''}`}
                      onClick={() => choose({ kind: 'file', path: file.path })}
                      title={file.path}
                    >
                      <span className="tree-symbol">
                        <FileCode2 aria-hidden="true" size={15} />
                      </span>
                      <span className="tree-name">{labelFromPath(file.path)}</span>
                      {file.readonly && (
                        <span className="tree-tail" title="只读">
                          只读
                        </span>
                      )}
                    </button>
                  ))}
                </div>
                <div className="tree-section-title with-action">
                  CAN 帧 <span>{workspace.frames.length}</span>
                  <button
                    type="button"
                    aria-label="添加 CAN 帧"
                    title="添加 CAN 帧"
                    onClick={() => openCreator('frame')}
                    disabled={disabled}
                  >
                    <Plus aria-hidden="true" size={16} />
                  </button>
                </div>
                <div className="tree-items">
                  {workspace.frames.map((frame) => (
                    <div key={frame.path}>
                      <button
                        type="button"
                        className={`tree-item ${selection?.kind === 'frame' && selection.path === frame.path ? 'active' : ''}`}
                        onClick={() => choose({ kind: 'frame', path: frame.path })}
                        title={frame.path}
                      >
                        <span className="tree-symbol">
                          <Cable aria-hidden="true" size={15} />
                        </span>
                        <span className="tree-name">{frame.name}</span>
                        <span className="tree-tail">{frame.direction.toUpperCase()}</span>
                      </button>
                      {workspace.signals
                        .filter((signal) => signal.framePath === frame.path)
                        .map((signal) => (
                          <button
                            key={signal.path}
                            type="button"
                            className={`tree-item nested ${selection?.kind === 'signal' && selection.path === signal.path ? 'active' : ''}`}
                            onClick={() => choose({ kind: 'signal', path: signal.path })}
                            title={signal.path}
                          >
                            <span className="tree-symbol">
                              <Waypoints aria-hidden="true" size={14} />
                            </span>
                            <span className="tree-name">{signal.name}</span>
                          </button>
                        ))}
                    </div>
                  ))}
                  {!workspace.frames.length && (
                    <p className="empty-tree">暂无帧。添加标准 CAN 帧以开始配置。</p>
                  )}
                </div>
                {workspace.files.some((file) => file.retainedCount > 0) && (
                  <div className="tree-footer">
                    <span>保留项只读</span>
                    <strong>
                      {workspace.files.reduce((count, file) => count + file.retainedCount, 0)}
                    </strong>
                  </div>
                )}
              </nav>
            )}
          </aside>
          <div className="workspace-content">
            <div className={`workspace-grid${page === 'editor' ? '' : ' single-page'}`}>
              <section
                className="main-pane"
                aria-label={page === 'editor' ? '配置工作区' : '项目工作页'}
              >
                {workspace.integrationCandidate && (
                  <IntegrationDelivery
                    controller={controller}
                    active={page === 'build' || page === 'virtual'}
                  />
                )}
                {(workspace.integrationCandidate || page === 'integration') && (
                  <IntegrationPage controller={controller} />
                )}
                {page === 'editor' && <EditorPage controller={controller} />}
                {page === 'diagnostics' && <DiagnosticsPage controller={controller} />}
                {page === 'build' && !workspace.integrationCandidate && (
                  <BuildPage controller={controller} />
                )}
                {page === 'virtual' && !workspace.integrationCandidate && (
                  <VirtualPage controller={controller} />
                )}
              </section>
              {page === 'editor' && <EditorInspector controller={controller} />}
            </div>
            {!native && (
              <p className="environment-warning" role="status">
                需要桌面运行环境。当前仅能查看界面，不能执行文件或内核操作。
              </p>
            )}
            {notice && (
              <p className={`notice ${notice.tone}`} role="alert">
                {notice.text}
              </p>
            )}
          </div>
        </main>
      )}
      {savePreview && (
        <div className="save-preview-backdrop">
          <section
            className="save-preview-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="save-preview-title"
          >
            <header>
              <div>
                <p className="eyebrow">ARXML SAVE PREVIEW</p>
                <h2 id="save-preview-title">确认文件改动</h2>
                <p>预览不会写入磁盘。逐份查看原文与拟保存内容，再确认保存。</p>
              </div>
              <button
                type="button"
                className="quiet-button"
                onClick={() => setSavePreview(null)}
                disabled={Boolean(busy)}
              >
                关闭
              </button>
            </header>
            <div className="save-preview-body">
              <nav aria-label="预览文件">
                {savePreview.files.map((file) => (
                  <button
                    type="button"
                    key={file.path}
                    className={file.path === previewPath ? 'active' : ''}
                    onClick={() => setPreviewPath(file.path)}
                    title={file.path}
                  >
                    <span>{labelFromPath(file.path)}</span>
                    <small>{file.changed ? '将修改' : '保持不变'}</small>
                  </button>
                ))}
              </nav>
              <div className="save-preview-content">
                {previewFile && (
                  <>
                    <p className="mono path-text">{previewFile.path}</p>
                    {delta ? (
                      <>
                        <p>
                          差异范围包含全部改动；相同的开头和结尾已折叠。为阅读方便，相邻 XML
                          标签已分行，原始字节见下方完整文本。
                        </p>
                        <div className="save-preview-compare">
                          <div>
                            <h3>当前文件 · 预览第 {delta.oldStart} 行起</h3>
                            <pre>{delta.oldText || '（此处无内容）'}</pre>
                          </div>
                          <div>
                            <h3>拟保存 · 预览第 {delta.newStart} 行起</h3>
                            <pre>{delta.newText || '（此处无内容）'}</pre>
                          </div>
                        </div>
                        <details className="save-preview-full">
                          <summary>查看两份完整文本</summary>
                          <div className="save-preview-compare">
                            <div>
                              <h3>当前文件</h3>
                              <pre>{previewFile.before}</pre>
                            </div>
                            <div>
                              <h3>拟保存</h3>
                              <pre>{previewFile.after}</pre>
                            </div>
                          </div>
                        </details>
                      </>
                    ) : (
                      <p className="save-preview-unchanged">这份来源文件保持原字节，不会重写。</p>
                    )}
                  </>
                )}
              </div>
            </div>
            <footer>
              <span>
                {savePreview.files.filter((file) => file.changed).length} 份将修改 ·{' '}
                {savePreview.files.filter((file) => !file.changed).length} 份保持不变
              </span>
              <div>
                <button
                  type="button"
                  className="quiet-button"
                  onClick={() => setSavePreview(null)}
                  disabled={Boolean(busy)}
                >
                  取消
                </button>
                <button
                  type="button"
                  className="primary-button compact"
                  onClick={confirmSave}
                  disabled={Boolean(busy) || !savePreview.files.some((file) => file.changed)}
                >
                  确认保存
                </button>
              </div>
            </footer>
          </section>
        </div>
      )}
      {generationPreview && !workspace?.integrationCandidate && (
        <div className="save-preview-backdrop">
          <section
            className="save-preview-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="generation-preview-title"
          >
            <header>
              <div>
                <p className="eyebrow">C99 GENERATION PREVIEW</p>
                <h2 id="generation-preview-title">
                  {generationKind === 'handoff' ? '确认导出可重建主机交付包' : '确认生成工程'}
                </h2>
                <p>预览不写入输出目录。确认后生成完整工程；原有工程将保留在备份目录。</p>
              </div>
              <button
                type="button"
                className="quiet-button"
                onClick={() => setGenerationPreview(null)}
                disabled={Boolean(busy)}
              >
                关闭
              </button>
            </header>
            <div className="save-preview-body">
              <nav aria-label="生成文件预览">
                {generationPreview.files.map((file) => (
                  <button
                    type="button"
                    key={file.path}
                    className={file.path === generationPreviewPath ? 'active' : ''}
                    onClick={() => setGenerationPreviewPath(file.path)}
                    title={file.path}
                  >
                    <span>{file.path}</span>
                    <small>
                      {file.status === 'new'
                        ? '将新增'
                        : file.status === 'changed'
                          ? '将修改'
                          : '内容不变'}
                    </small>
                  </button>
                ))}
              </nav>
              <div className="save-preview-content">
                {generationPreviewFile && (
                  <>
                    <p className="mono path-text">
                      {generationPreview.outputDirectory} / {generationPreviewFile.path}
                    </p>
                    {generationDelta ? (
                      <>
                        <div className="save-preview-compare">
                          <div>
                            <h3>当前文件 · 第 {generationDelta.oldStart} 行起</h3>
                            <pre>{generationDelta.oldText || '（此处无内容）'}</pre>
                          </div>
                          <div>
                            <h3>拟生成 · 第 {generationDelta.newStart} 行起</h3>
                            <pre>{generationDelta.newText || '（此处无内容）'}</pre>
                          </div>
                        </div>
                        <details className="save-preview-full">
                          <summary>查看两份完整文本</summary>
                          <div className="save-preview-compare">
                            <div>
                              <h3>当前文件</h3>
                              <pre>{generationPreviewFile.before}</pre>
                            </div>
                            <div>
                              <h3>拟生成</h3>
                              <pre>{generationPreviewFile.after}</pre>
                            </div>
                          </div>
                        </details>
                      </>
                    ) : generationPreviewFile.status === 'new' ? (
                      <div className="save-preview-compare">
                        <div>
                          <h3>拟生成文件</h3>
                          <pre>{generationPreviewFile.after}</pre>
                        </div>
                      </div>
                    ) : (
                      <p className="save-preview-unchanged">
                        文件内容不变；确认后仍会生成完整工程并保留旧目录。
                      </p>
                    )}
                  </>
                )}
              </div>
            </div>
            <footer>
              <span>
                {generationPreview.files.filter((file) => file.status === 'new').length} 个新增 ·{' '}
                {generationPreview.files.filter((file) => file.status === 'changed').length} 个修改
                · {generationPreview.files.filter((file) => file.status === 'unchanged').length}{' '}
                个内容不变
              </span>
              <div>
                <button
                  type="button"
                  className="quiet-button"
                  onClick={() => setGenerationPreview(null)}
                  disabled={Boolean(busy)}
                >
                  取消
                </button>
                <button
                  type="button"
                  className="primary-button compact"
                  onClick={confirmGenerate}
                  disabled={Boolean(busy)}
                >
                  {generationKind === 'handoff' ? '确认导出' : '确认生成'}
                </button>
              </div>
            </footer>
          </section>
        </div>
      )}
      <SettingsPage controller={controller} />
    </div>
  );
}
