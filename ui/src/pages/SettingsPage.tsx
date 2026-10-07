import { useState } from 'react';
import type { ExecutionTools } from '../types';
import type { Appearance } from '../workbench/projectTypes';
import type { Workbench } from '../workbench/useWorkbench';
import { CopyText, Dialog } from '../workbench/Dialog';

const toolLabels: { key: keyof ExecutionTools; label: string }[] = [
  { key: 'compiler', label: '编译器' },
  { key: 'objdump', label: 'Objdump' },
  { key: 'git', label: 'Git' },
  { key: 'python', label: 'Python 解释器' },
];

export function SettingsPage({ controller: c }: { controller: Workbench }) {
  const [category, setCategory] = useState<'appearance' | 'definitions' | 'tools'>('appearance');
  if (
    !c.settingsOpen ||
    c.guard ||
    c.changePreview ||
    c.savePreview ||
    c.integrationPreview ||
    c.projectPreview ||
    c.generationPreview ||
    c.applicationPreview ||
    c.handoffImportOpen
  )
    return null;
  const locked = Boolean(c.busy) || !c.native;
  function close() {
    if (c.busy) return;
    c.setAppearanceDraft(c.capabilities?.appearance ?? 'system');
    c.setResourceDraft({
      xsdArchive: c.capabilities?.xsdArchive ?? '',
      modArchive: c.capabilities?.modArchive ?? '',
    });
    c.setToolDraft(
      c.capabilities?.configuredExecutionTools ?? {
        compiler: '',
        objdump: '',
        git: '',
        python: '',
      },
    );
    c.setSettingsNotice('');
    c.setSettingsOpen(false);
  }
  const identity = c.projection?.ruleSetIdentity ?? c.capabilities?.ruleSetIdentity;
  return (
    <Dialog
      title="设置"
      onClose={close}
      footer={
        <>
          <span role="status">{c.settingsNotice}</span>
          <button type="button" onClick={close} disabled={Boolean(c.busy)}>
            关闭
          </button>
          {category === 'appearance' ? (
            <button
              className="primary-button"
              type="button"
              disabled={locked}
              onClick={() => void c.configureAppearance()}
            >
              保存外观
            </button>
          ) : category === 'tools' ? (
            <button
              className="primary-button"
              type="button"
              disabled={locked}
              onClick={() => void c.configureTools()}
            >
              保存执行工具
            </button>
          ) : null}
        </>
      }
    >
      <div className="settings-layout">
        <nav aria-label="设置类别">
          {(
            [
              ['appearance', '外观'],
              ['definitions', '规则与模块定义'],
              ['tools', '执行工具'],
            ] as const
          ).map(([key, label]) => (
            <button
              key={key}
              type="button"
              aria-current={category === key ? 'page' : undefined}
              className={category === key ? 'selected' : ''}
              onClick={() => setCategory(key)}
            >
              {label}
            </button>
          ))}
        </nav>
        <div className="settings-content">
          <section hidden={category !== 'appearance'}>
            <h3>外观</h3>
            <p>预览仅改变呈现；关闭恢复已保存选择，工程与字段草稿保持。</p>
            <label>
              主题
              <select
                aria-label="主题"
                value={c.appearanceDraft}
                onChange={(event) => c.setAppearanceDraft(event.target.value as Appearance)}
              >
                <option value="light">浅色</option>
                <option value="dark">深色</option>
                <option value="system">跟随系统</option>
              </select>
            </label>
          </section>
          <section hidden={category !== 'definitions'}>
            <h3>产品内置规则</h3>
            {c.capabilities?.ruleError ? (
              <p className="notice error" role="alert">
                {c.capabilities.ruleError}。修复或重装匹配发布版；官方档案不能替代产品规则。
              </p>
            ) : null}
            {identity ? (
              <dl className="property-list">
                <div>
                  <dt>版次</dt>
                  <dd>{identity.release}</dd>
                </div>
                <div>
                  <dt>规则版号</dt>
                  <dd>{identity.rulesVersion}</dd>
                </div>
                <div>
                  <dt>库存 SHA-256</dt>
                  <dd className="mono path-text">
                    {identity.sha256}
                    <CopyText text={identity.sha256} />
                  </dd>
                </div>
              </dl>
            ) : (
              <p>尚未获得后台可信规则身份。</p>
            )}
            <details>
              <summary>完整声明覆盖</summary>
              <div className="table-wrap">
                <table>
                  <thead>
                    <tr>
                      <th>作用域 / Rule ID</th>
                      <th>对象 / 模块</th>
                      <th>覆盖</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(c.capabilities?.ruleCoverage ?? []).map((coverage) => (
                      <tr key={coverage.ruleId}>
                        <td>
                          {coverage.scope}
                          <br />
                          {coverage.ruleId}
                        </td>
                        <td>{coverage.subjects.join('、')}</td>
                        <td>
                          {coverage.supported ? '支持' : '不支持'}
                          <small>{coverage.reason}</small>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </details>
            <h3>工程明确接纳的扩展</h3>
            <p>
              内置定义不可替换。扩展只约束实际消费者，不自动授予生成能力；接纳变化须保存工程，直接
              ARXML 会话重开不隐式恢复。
            </p>
            <button
              type="button"
              disabled={locked || !c.workspace}
              onClick={() => void c.importDefinitionCatalog()}
            >
              选择并明确接纳 catalog.json
            </button>
            {(c.projection?.acceptedExtensionDefinitions ?? []).map((identity) => (
              <section className="catalog-row" key={identity.catalogId}>
                <strong>{identity.catalogId}</strong>
                <span>
                  {identity.release} · {identity.version}
                </span>
                <p className="mono path-text">{identity.sha256}</p>
                <CopyText text={JSON.stringify(identity, null, 2)} label="复制精确身份" />
                <button
                  type="button"
                  disabled={locked}
                  onClick={() => void c.removeDefinitionCatalog(identity.catalogId)}
                >
                  明确移除
                </button>
              </section>
            ))}
            {c.projection?.extensionDefinitions.map((extension) => (
              <details key={extension.identity.catalogId}>
                <summary>
                  {extension.identity.catalogId} · {extension.available ? '可用' : '不可用'}
                </summary>
                <p className="mono path-text">来源 {extension.source ?? '后台未提供来源路径'}</p>
                <p>{extension.reason}</p>
                <h3>实际消费者</h3>
                {extension.consumers.length ? (
                  <ul>
                    {extension.consumers.map((consumer) => (
                      <li key={consumer} className="mono path-text">
                        {consumer}
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p>后台没有返回当前消费者。</p>
                )}
                <CopyText text={JSON.stringify(extension, null, 2)} label="复制来源与消费者" />
              </details>
            ))}
            {c.projection?.dirty ? (
              <p className="warning-text">接纳集合或源配置尚未保存；返回工程预览保存。</p>
            ) : null}
            <details className="legacy-settings">
              <summary>历史兼容 / 开发：官方资源</summary>
              <p>仅用于旧 v1 交接兼容或开发 oracle；不是正常配置门禁，不能替换内置规则。</p>
              {c.capabilities?.resourceError ? (
                <p className="error-text">{c.capabilities.resourceError}</p>
              ) : null}
              <div className="form-fields">
                <label>
                  XSD 档案
                  <input
                    aria-label="历史 XSD 档案路径"
                    value={c.resourceDraft.xsdArchive}
                    disabled={locked}
                    onChange={(event) =>
                      c.setResourceDraft({ ...c.resourceDraft, xsdArchive: event.target.value })
                    }
                  />
                </label>
                <label>
                  MOD 档案
                  <input
                    aria-label="历史 MOD 档案路径"
                    value={c.resourceDraft.modArchive}
                    disabled={locked}
                    onChange={(event) =>
                      c.setResourceDraft({ ...c.resourceDraft, modArchive: event.target.value })
                    }
                  />
                </label>
              </div>
              <button
                type="button"
                disabled={locked || !c.resourceDraft.xsdArchive || !c.resourceDraft.modArchive}
                onClick={() => void c.configureResources()}
              >
                核对并保存兼容资源
              </button>
            </details>
          </section>
          <section hidden={category !== 'tools'}>
            <h3>执行工具保存值</h3>
            <p>
              仅此类别明确保存的路径写入设置。留空不指定保存路径；环境覆盖优先，不自动复制回保存值。保存不等于编译预检通过；缺编译器不阻断配置与纯源码准备。
            </p>
            {c.capabilities?.toolError ? (
              <p className="error-text">{c.capabilities.toolError}</p>
            ) : null}
            <div className="form-fields">
              {toolLabels.map(({ key, label }) => (
                <label key={key}>
                  {label}
                  <div className="path-picker">
                    <input
                      aria-label={`${label}保存路径`}
                      value={c.toolDraft[key]}
                      disabled={locked}
                      onChange={(event) =>
                        c.setToolDraft({ ...c.toolDraft, [key]: event.target.value })
                      }
                    />
                    <button
                      type="button"
                      disabled={locked}
                      onClick={() =>
                        void c.chooseBinary((path) =>
                          c.setToolDraft((previous) => ({ ...previous, [key]: path })),
                        )
                      }
                    >
                      浏览
                    </button>
                  </div>
                </label>
              ))}
            </div>
            <h3>当前有效工具（只读）</h3>
            <dl>
              {toolLabels.map(({ key, label }) => (
                <div key={key}>
                  <dt>{label}</dt>
                  <dd className="mono path-text">
                    {c.capabilities?.executionTools?.[key] ?? '后台未提供有效路径'}
                  </dd>
                </div>
              ))}
            </dl>
            <h3>环境覆盖（只读）</h3>
            {c.capabilities?.environmentOverrides.length ? (
              <ul>
                {c.capabilities.environmentOverrides.map((entry) => (
                  <li key={entry} className="mono path-text">
                    {entry}
                  </li>
                ))}
              </ul>
            ) : (
              <p>无环境覆盖。</p>
            )}
          </section>
        </div>
      </div>
    </Dialog>
  );
}
