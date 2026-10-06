import { FolderOpen } from 'lucide-react';
import type { Workbench } from './useWorkbench';

export function SourceEntry({ controller: c, logo }: { controller: Workbench; logo: string }) {
  const openReason = c.actionReason('open');
  const createReason = c.actionReason('create');
  const saveAs = c.source === 'save-as';
  return (
    <div className="start-page">
      <header>
        <img src={logo} width="76" height="76" alt="" />
        <h1>{c.workspace ? '工程来源' : 'Classic CAN 配置工作台'}</h1>
        <p>
          {c.workspace
            ? '当前工程保持；只有真实预览确认或导入成功才替换。'
            : '从内置模板新建工程，或导入 ARXML、打开已有工程。'}
        </p>
      </header>
      <div className="start-layout">
        <nav aria-label="工程来源">
          <button
            type="button"
            className={c.source === 'empty' ? 'selected' : ''}
            onClick={() => c.openProjectEntry('empty')}
          >
            新建工程
          </button>
          <button
            type="button"
            className={c.source === 'import' ? 'selected' : ''}
            onClick={() => c.openProjectEntry('import')}
          >
            导入 ARXML
          </button>
          {c.workspace ? (
            <button
              type="button"
              className={saveAs ? 'selected' : ''}
              onClick={() => c.openProjectEntry('save-as')}
            >
              保存为成员工程
            </button>
          ) : null}
          <button
            type="button"
            onClick={() => void c.openMemberProject()}
            disabled={Boolean(openReason)}
          >
            打开成员工程…
          </button>
          <button type="button" onClick={c.importHandoff} disabled={Boolean(openReason)}>
            导入主机交付包…
          </button>
          <button
            type="button"
            onClick={() => void c.chooseDirectory(c.importHandoffDirectory)}
            disabled={Boolean(openReason)}
          >
            重导入 ECU 交接包…
          </button>
        </nav>
        <section>
          {c.source !== 'import' ? (
            <div className="form-fields">
              <label>
                工程名称
                <input
                  aria-label="工程名称"
                  value={c.projectName}
                  onChange={(event) => c.setProjectName(event.target.value)}
                  autoComplete="off"
                />
              </label>
              {!saveAs ? (
                <label>
                  工程模板
                  <select
                    aria-label="工程模板"
                    value={c.templateId}
                    onChange={(event) => c.setTemplateId(event.target.value as typeof c.templateId)}
                  >
                    <option value="can-empty-v1">空 CAN 工程</option>
                    <option value="can-signals-v1">CAN 信号工程</option>
                    <option value="standard-ecu-v1">标准 ECU 工程</option>
                  </select>
                </label>
              ) : (
                <p>从当前工程预览复制原始源集合与明确接纳的定义身份；原目录不被覆盖。</p>
              )}
              <label>
                新空目录
                <div className="path-picker">
                  <input value={c.projectDirectory} readOnly aria-label="新工程目录" />
                  <button
                    type="button"
                    onClick={() => void c.chooseDirectory(c.setProjectDirectory)}
                    disabled={!c.native || Boolean(c.busy)}
                  >
                    <FolderOpen size={15} />
                    选择
                  </button>
                </div>
              </label>
              <button
                className="primary-button"
                type="button"
                onClick={() => (saveAs ? void c.previewSaveAs() : c.createProject())}
                disabled={Boolean(saveAs ? c.actionReason('save') : createReason)}
              >
                {saveAs ? '预览保存为工程' : '预览工程文件'}
              </button>
              {saveAs ? (
                c.actionReason('save') ? (
                  <p className="field-help">{c.actionReason('save')}</p>
                ) : null
              ) : createReason ? (
                <p className="field-help">{createReason}</p>
              ) : null}
            </div>
          ) : (
            <div className="form-fields">
              <button
                type="button"
                onClick={() => void c.chooseFiles()}
                disabled={Boolean(openReason)}
              >
                选择多份 .arxml 文件
              </button>
              <label>
                每行一份 ARXML 路径
                <textarea
                  aria-label="ARXML 来源路径"
                  value={c.importPathText}
                  rows={7}
                  onChange={(event) => {
                    c.setImportPathText(event.target.value);
                    c.setImportPaths(
                      event.target.value
                        .split(/\r?\n/)
                        .map((path) => path.trim())
                        .filter(Boolean),
                    );
                  }}
                />
              </label>
              <p>{c.importPaths.length} 份输入 · 未知有效内容保持</p>
              <button
                type="button"
                className="primary-button"
                disabled={Boolean(openReason) || !c.importPaths.length}
                onClick={c.importProject}
              >
                导入并进入工程
              </button>
              {openReason ? <p className="field-help">{openReason}</p> : null}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
