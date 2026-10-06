import { useState } from 'react';
import type { Workbench } from './useWorkbench';
import { CopyText, Dialog, TextSnapshot } from './Dialog';

export function PreviewDialogs({ controller: c }: { controller: Workbench }) {
  const [projectFile, setProjectFile] = useState('');
  const [applicationFile, setApplicationFile] = useState('');
  if (c.guard) {
    const project = c.guard.kind === 'project';
    return (
      <Dialog
        title={c.guard.title}
        onClose={() => void c.resolveGuard('cancel')}
        footer={
          <>
            <button type="button" onClick={() => void c.resolveGuard('cancel')}>
              {project ? '取消替换' : '留在原处'}
            </button>
            <button type="button" onClick={() => void c.resolveGuard('discard')}>
              {project ? '明确放弃并继续' : '放弃草稿并继续'}
            </button>
            <button
              type="button"
              className="primary-button"
              onClick={() => void c.resolveGuard('apply')}
            >
              {project ? '返回并保存' : '应用并继续'}
            </button>
          </>
        }
      >
        <p>
          {project
            ? '此操作将替换或关闭当前工程。返回保存会先应用全部草稿，再显示真实保存预览；只有明确确认保存成功才继续。'
            : '当前上下文有未应用输入。应用成功后继续；任何拒绝保持当前位置与输入。'}
        </p>
        <p>
          尚未保存：{c.workspace?.dirty || c.projection?.dirty ? '是' : '否'} · 未应用：
          {c.unapplied || c.creating ? '是' : '否'}
        </p>
      </Dialog>
    );
  }
  if (c.changePreview)
    return (
      <Dialog
        title="确认原子应用批次"
        onClose={() => {
          if (!c.busy) c.setChangePreview(null);
        }}
        footer={
          <>
            <span>{c.changePreview.impacts.length} 项真实影响 · 不写磁盘</span>
            <button
              type="button"
              disabled={Boolean(c.busy)}
              onClick={() => c.setChangePreview(null)}
            >
              取消确认
            </button>
            <button
              type="button"
              className="primary-button"
              disabled={Boolean(c.busy) || !c.preparedChangeSet}
              onClick={() => void c.applyChanges()}
            >
              确认应用全部变化
            </button>
          </>
        }
      >
        <p className="mono path-text">
          输入 {c.changePreview.inputFingerprint}
          <br />
          定义 {c.changePreview.definitionFingerprint}
          <br />
          确认 {c.changePreview.changeRevision}
        </p>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>对象 / Change ID</th>
                <th>旧值</th>
                <th>新值</th>
                <th>影响</th>
              </tr>
            </thead>
            <tbody>
              {c.changePreview.impacts.map((impact, index) => (
                <tr key={`${impact.changeId}-${index}`}>
                  <td className="mono path-text">
                    {impact.path}
                    <small>{impact.changeId}</small>
                  </td>
                  <td className="mono">{impact.before}</td>
                  <td className="mono">{impact.after}</td>
                  <td>{impact.incoming ? '入站引用联动' : '显式修改'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {c.changePreview.diagnostics.map((issue, index) => (
          <p
            key={`${issue.code}-${index}`}
            className={issue.severity === 'error' ? 'error-text' : 'field-help'}
          >
            {issue.scope} · {issue.code} · {issue.message} · {issue.remedy}
          </p>
        ))}
      </Dialog>
    );
  if (c.applicationPreview) {
    const preview = c.applicationPreview;
    const selected =
      preview.files.find((file) => file.path === applicationFile) ?? preview.files[0];
    const close = () => {
      if (!c.busy) c.setApplicationPreview(null);
    };
    return (
      <Dialog
        title="确认 create-only 用户应用初始化"
        onClose={close}
        footer={
          <>
            <span>{preview.files.length} 份后台 seed 文件 · 仅创建不存在的源</span>
            <button type="button" disabled={Boolean(c.busy)} onClick={close}>
              取消初始化
            </button>
            <button
              type="button"
              className="primary-button"
              disabled={Boolean(c.busy)}
              onClick={() => void c.initializeApplicationPreviewed()}
            >
              确认创建用户应用与成员记录
            </button>
          </>
        }
      >
        <p>
          预览不写入；确认重新核对真实组件契约、源字节、规则、定义、成员清单与目的路径。任何现存
          live 源都不能覆盖，封存副本不作为 live 源。
        </p>
        <dl>
          <dt>真实应用槽</dt>
          <dd className="mono">{preview.slot.producerSlot}</dd>
          <dt>组件</dt>
          <dd className="mono path-text">{preview.slot.componentPath}</dd>
          <dt>源成员</dt>
          <dd>
            {preview.slot.sourcePaths.map((path) => (
              <p key={path} className="mono path-text">
                {path}
              </p>
            ))}
          </dd>
          <dt>生成接口头</dt>
          <dd>
            {preview.slot.generatedHeaders.map((path) => (
              <p key={path} className="mono path-text">
                {path}
              </p>
            ))}
          </dd>
          <dt>入口符号</dt>
          <dd className="mono">{preview.slot.entrySymbols.join('\n')}</dd>
        </dl>
        <CopyText text={JSON.stringify(preview.slot, null, 2)} label="复制完整后台槽描述" />
        <div className="preview-layout">
          <nav aria-label="用户应用 seed 文件">
            {preview.files.map((file) => (
              <button
                key={file.path}
                type="button"
                className={selected?.path === file.path ? 'selected' : ''}
                onClick={() => setApplicationFile(file.path)}
              >
                {file.path}
              </button>
            ))}
          </nav>
          <section>
            {selected ? (
              <>
                <h3 className="mono path-text">{selected.path}</h3>
                <CopyText text={selected.contents} label="复制完整 seed 文件" />
                <TextSnapshot
                  key={selected.path}
                  text={selected.contents}
                  label="后台提供的完整应用 seed"
                />
              </>
            ) : null}
          </section>
        </div>
        <details>
          <summary>成员清单真实变化</summary>
          <div className="preview-compare">
            <section>
              <h3>初始化前</h3>
              <CopyText text={preview.manifestBefore} label="复制完整原成员清单" />
              <TextSnapshot text={preview.manifestBefore} label="初始化前成员清单" />
            </section>
            <section>
              <h3>拟写入成员清单</h3>
              <CopyText text={preview.manifestAfter} label="复制完整拟写成员清单" />
              <TextSnapshot text={preview.manifestAfter} label="拟写入成员清单" />
            </section>
          </div>
        </details>
        {c.notice?.tone === 'error' ? (
          <>
            <p role="alert" className="error-text">
              {c.notice.text.slice(0, 2000)}
            </p>
            <CopyText text={c.notice.text} label="复制完整初始化错误" />
          </>
        ) : null}
      </Dialog>
    );
  }
  if (c.projectPreview) {
    const preview = c.projectPreview;
    const selected = preview.files.find((file) => file.path === projectFile) ?? preview.files[0];
    return (
      <Dialog
        title={c.projectPreviewKind === 'create' ? '确认新建工程' : '确认保存为成员工程'}
        onClose={() => {
          if (!c.busy) c.setProjectPreview(null);
        }}
        footer={
          <>
            <span>{preview.files.length} 份文件 · 新空目录</span>
            <button
              type="button"
              disabled={Boolean(c.busy)}
              onClick={() => c.setProjectPreview(null)}
            >
              取消
            </button>
            <button
              type="button"
              disabled={Boolean(c.busy)}
              className="primary-button"
              onClick={() => void c.confirmProjectPreview()}
            >
              {c.projectPreviewKind === 'create' ? '确认创建工程' : '确认保存为工程'}
            </button>
          </>
        }
      >
        <p className="mono path-text">
          {preview.directory} · {preview.name} · {preview.templateId}
        </p>
        <p>预览未写入；原始输入不被覆盖。</p>
        <div className="preview-layout">
          <nav aria-label="工程创建文件">
            {preview.files.map((file) => (
              <button
                key={file.path}
                type="button"
                className={selected?.path === file.path ? 'selected' : ''}
                onClick={() => setProjectFile(file.path)}
              >
                {file.path}
              </button>
            ))}
          </nav>
          <section>
            {selected ? (
              <>
                <h3>{selected.path}</h3>
                <CopyText text={selected.contents} label="复制完整文件" />
                <TextSnapshot key={selected.path} text={selected.contents} label="完整工程文件" />
              </>
            ) : null}
          </section>
        </div>
        <details>
          <summary>精确扩展接纳集合（{preview.acceptedExtensionDefinitions.length}）</summary>
          <pre>{JSON.stringify(preview.acceptedExtensionDefinitions, null, 2)}</pre>
        </details>
      </Dialog>
    );
  }
  const save = c.savePreview ?? c.integrationPreview;
  if (save) {
    const standard = Boolean(c.integrationPreview);
    const selectedPath = standard ? c.integrationPreviewPath : c.previewPath;
    const selected = save.files.find((file) => file.path === selectedPath) ?? save.files[0];
    const close = () => c.cancelSavePreview();
    return (
      <Dialog
        title="确认保存源配置"
        onClose={close}
        footer={
          <>
            <span>
              {save.files.filter((file) => file.changed).length} 份将修改 · 其余原字节保持
            </span>
            <button type="button" onClick={close} disabled={Boolean(c.busy)}>
              取消
            </button>
            <button
              type="button"
              className="primary-button"
              onClick={standard ? c.saveIntegration : c.confirmSave}
              disabled={
                Boolean(c.busy) ||
                (!save.files.some((file) => file.changed) && !c.savingForReplacement)
              }
            >
              确认保存
            </button>
          </>
        }
      >
        <p>预览不会写入；确认仍核对原字节、规则、定义、路径与外部状态。</p>
        <div className="preview-layout">
          <nav aria-label="保存文件">
            {save.files.map((file) => (
              <button
                type="button"
                key={file.path}
                className={selected?.path === file.path ? 'selected' : ''}
                onClick={() =>
                  standard ? c.setIntegrationPreviewPath(file.path) : c.setPreviewPath(file.path)
                }
              >
                {file.path}
                <small>{file.changed ? '将修改' : '保持不变'}</small>
              </button>
            ))}
          </nav>
          <section>
            {selected ? (
              <>
                <p className="mono path-text">{selected.path}</p>
                <div className="preview-compare">
                  <div>
                    <h3>当前原文</h3>
                    {selected.before !== null ? (
                      <>
                        <CopyText text={selected.before} label="复制完整原文" />
                        <TextSnapshot
                          key={selected.path + ':before'}
                          text={selected.before}
                          label="完整保存前原文"
                        />
                      </>
                    ) : (
                      <p>没有可展示原文。</p>
                    )}
                  </div>
                  <div>
                    <h3>拟保存</h3>
                    {selected.after !== null ? (
                      <>
                        <CopyText text={selected.after} label="复制完整拟保存内容" />
                        <TextSnapshot
                          key={selected.path + ':after'}
                          text={selected.after}
                          label="完整拟保存原文"
                        />
                      </>
                    ) : (
                      <p>没有可展示文本。</p>
                    )}
                  </div>
                </div>
              </>
            ) : null}
          </section>
        </div>
      </Dialog>
    );
  }
  if (c.generationPreview) {
    const preview = c.generationPreview;
    const file =
      preview.files.find((item) => item.path === c.generationPreviewPath) ?? preview.files[0];
    const close = () => {
      if (!c.busy) c.setGenerationPreview(null);
    };
    return (
      <Dialog
        title={c.generationKind === 'handoff' ? '确认导出可重建交接包' : '确认生成独立源码'}
        onClose={close}
        footer={
          <>
            <span>
              {preview.files.length} 个文件 · {c.legacyTarget}
            </span>
            <button type="button" disabled={Boolean(c.busy)} onClick={close}>
              取消
            </button>
            <button
              type="button"
              className="primary-button"
              disabled={Boolean(c.busy)}
              onClick={() =>
                c.workspace?.integrationCandidate ? void c.generateEcu() : c.confirmGenerate()
              }
            >
              {c.generationKind === 'handoff' ? '确认导出' : '确认生成'}
            </button>
          </>
        }
      >
        <p className="mono path-text">{preview.outputDirectory}</p>
        <p>源码预览不运行编译器；确认安装本次预览字节，构建与行为验证另行执行。</p>
        <div className="preview-layout">
          <nav aria-label="生成文件">
            {preview.files.map((item) => (
              <button
                type="button"
                key={item.path}
                className={file?.path === item.path ? 'selected' : ''}
                onClick={() => c.setGenerationPreviewPath(item.path)}
              >
                {item.path}
                <small>
                  {item.status} · {item.owner ?? '兼容 DTO 未提供拥有权'}
                </small>
              </button>
            ))}
          </nav>
          <section>
            {file ? (
              <>
                <h3 className="mono path-text">{file.path}</h3>
                <dl>
                  <dt>拥有权</dt>
                  <dd>{file.owner ?? '后台未提供'}</dd>
                  <dt>生产者</dt>
                  <dd className="mono path-text">{file.producerId ?? '后台未提供'}</dd>
                  {file.snapshotOf ? (
                    <>
                      <dt>快照来源</dt>
                      <dd className="mono path-text">{file.snapshotOf}</dd>
                    </>
                  ) : null}
                </dl>
                <div className="preview-compare">
                  <div>
                    <h3>此前内容</h3>
                    {file.before !== null ? (
                      <>
                        <CopyText text={file.before} label="复制完整此前内容" />
                        <TextSnapshot
                          key={file.path + ':before'}
                          text={file.before}
                          label="完整此前源码"
                        />
                      </>
                    ) : (
                      <p>新文件或不可展示的二进制。</p>
                    )}
                  </div>
                  <div>
                    <h3>拟生成内容</h3>
                    {file.after !== null ? (
                      <>
                        <CopyText text={file.after} label="复制完整拟生成内容" />
                        <TextSnapshot
                          key={file.path + ':after'}
                          text={file.after}
                          label="完整拟生成源码"
                        />
                      </>
                    ) : (
                      <p>{file.status === 'removed' ? '核定移除' : '二进制或不可展示内容'}</p>
                    )}
                  </div>
                </div>
              </>
            ) : null}
          </section>
        </div>
      </Dialog>
    );
  }
  if (c.handoffImportOpen) {
    const close = () => {
      if (!c.busy) c.setHandoffImportOpen(false);
    };
    const v2 = c.handoffImportMode === 'v2';
    return (
      <Dialog
        title="导入封存交付包"
        onClose={close}
        footer={
          <>
            <button type="button" disabled={Boolean(c.busy)} onClick={close}>
              取消导入
            </button>
            <button
              type="button"
              className="primary-button"
              disabled={
                Boolean(c.busy) || !c.ecuImportDirectory || (v2 && !c.handoffImportDestination)
              }
              onClick={() => void c.confirmHandoffImport()}
            >
              核验并导入交付包
            </button>
          </>
        }
      >
        <p>
          后台按真实 handoff.format 精确分派，核对身份、源、seal
          与重新生成闭包；全部成功才替换当前工程，失败保留当前工程与封存包。
        </p>
        <div className="form-fields">
          <label>
            交付包目录
            <div className="path-picker">
              <input aria-label="交付包目录" value={c.ecuImportDirectory} readOnly />
              <button
                type="button"
                disabled={Boolean(c.busy)}
                onClick={() => void c.chooseDirectory(c.setEcuImportDirectory)}
              >
                选择交付包
              </button>
            </div>
          </label>
          <label>
            源目录方式
            <select
              aria-label="交付包源目录方式"
              value={c.handoffImportMode}
              disabled={Boolean(c.busy)}
              onChange={(event) => c.setHandoffImportMode(event.target.value as 'v2' | 'legacy')}
            >
              <option value="v2">v2：在新空目录重建 live 源</option>
              <option value="legacy">旧 v1：按原兼容规则打开</option>
            </select>
          </label>
          {v2 ? (
            <label>
              新的空 live 工作目录
              <div className="path-picker">
                <input
                  aria-label="新的空 live 工作目录"
                  value={c.handoffImportDestination}
                  readOnly
                />
                <button
                  type="button"
                  disabled={Boolean(c.busy)}
                  onClick={() => void c.chooseDirectory(c.setHandoffImportDestination)}
                >
                  选择新空目录
                </button>
              </div>
            </label>
          ) : null}
        </div>
        <p className="field-help">
          v2 必须使用新的空 live 目录，封存源不变为可写应用树；选择的目标目录只用于实际 v2 包。旧
          host/ECU v1 仍核对原固定官方档案与完整性，不自动升级；未知格式拒绝。
        </p>
        {c.notice?.tone === 'error' ? (
          <>
            <p role="alert" className="error-text">
              {c.notice.text.slice(0, 2000)}
            </p>
            <CopyText text={c.notice.text} label="复制完整导入错误" />
          </>
        ) : null}
      </Dialog>
    );
  }
  return null;
}
