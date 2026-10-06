import type { Workbench } from './workbench/useWorkbench';

const roleNames: Record<string, string> = {
  ecu_extract: 'ECU Extract',
  application: '应用组件',
  service_client: '服务使用端',
  types: '数据类型',
  bsw_description: 'BSW 描述',
  bsw_implementation: 'BSW 实现',
  ecuc_values: 'ECUC 配置值',
  retained: '保留内容',
};

export function IntegrationPanel({ controller }: { controller: Workbench }) {
  const {
    workspace,
    native,
    integrationUnapplied: changed,
    integrationIds: ids,
    integrationPeriod: period,
    integrationPreview: preview,
    integrationPreviewPath: selectedFile,
    integrationNotice: notice,
    integrationIssues: issues,
    integrationInspection: inspection,
    inspectIntegration: inspect,
    reopenIntegration: reopen,
    applyIntegration: apply,
    previewIntegrationSave: showPreview,
    saveIntegration: save,
    restoreIntegrationDraft,
    setIntegrationIds: setIds,
    setIntegrationPeriod: setPeriod,
    setIntegrationPreview: setPreview,
    setIntegrationPreviewPath: setSelectedFile,
    setIntegrationUnapplied: onDraftChange,
  } = controller;
  if (!workspace) return null;
  const busy = Boolean(controller.busy);
  const locked = busy;
  const plan = inspection?.description;
  const selected = preview?.files.find((file) => file.path === selectedFile);
  return (
    <div className="workflow-page integration-view" aria-label="标准输入工作区">
      <div className="section-header">
        <div>
          <p className="eyebrow">STANDARD ECU INPUTS</p>
          <h2>标准输入</h2>
          <p role="status">{busy ? '正在处理…' : notice}</p>
        </div>
        <div className="section-actions">
          <button
            className="outline-button small"
            onClick={inspect}
            disabled={busy || locked || !native || changed}
          >
            检查计划
          </button>
          <button
            className="outline-button small"
            onClick={reopen}
            disabled={busy || locked || !native}
          >
            重开来源
          </button>
        </div>
      </div>
      <p>
        必需输入：ECU Extract、应用与类型、服务使用端、所选 BSW 描述/实现及 ECUC 配置值。
        有效无关内容按原字节保留；内置支持范围不要求用户注册官方 XSD 或 MOD。
      </p>
      {issues.length > 0 && (
        <div className="inspector-block" role="alert">
          <h3>阻断原因与补救</h3>
          {issues.map((issue, index) => (
            <div key={`${issue.code}-${index}`}>
              <strong>
                {issue.code} · {issue.category}
              </strong>
              <p>{issue.message}</p>
              <p className="mono path-text">
                {issue.file} {issue.object}
              </p>
              <p>{issue.remedy}</p>
            </div>
          ))}
        </div>
      )}
      {plan && (
        <>
          <div className="inspector-block">
            <h3>来源角色</h3>
            <table>
              <thead>
                <tr>
                  <th>来源</th>
                  <th>实际角色</th>
                  <th>原字节 SHA-256</th>
                </tr>
              </thead>
              <tbody>
                {plan.sources.map((source) => (
                  <tr key={source.logicalPath}>
                    <td>{source.logicalPath}</td>
                    <td>{source.roles.map((role) => roleNames[role] ?? role).join('、')}</td>
                    <td className="mono path-text" title={source.rawSha256}>
                      {source.rawSha256.slice(0, 16)}…
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="inspector-block">
            <h3>受支持参数</h3>
            <p className="mono path-text">{plan.component.instance}</p>
            <div className="integration-fields">
              {plan.signals.map((signal) => (
                <label className="field" key={signal.port}>
                  <span>
                    {signal.receive ? '接收' : '发送'} CAN ID · {signal.port}
                  </span>
                  <input
                    aria-label={signal.receive ? '接收 CAN ID' : '发送 CAN ID'}
                    value={ids[signal.port] ?? ''}
                    onChange={(event) => {
                      const next = { ...ids, [signal.port]: event.target.value };
                      setIds(next);
                      onDraftChange(
                        period !== String(plan.component.periodMs) ||
                          plan.signals.some((item) => next[item.port] !== String(item.canId)),
                      );
                      setPreview(null);
                    }}
                    disabled={busy || locked}
                    inputMode="numeric"
                  />
                </label>
              ))}
              <label className="field">
                <span>应用 / Com 发送 / 对应 Alarm 周期（ms）</span>
                <input
                  aria-label="应用周期"
                  value={period}
                  onChange={(event) => {
                    setPeriod(event.target.value);
                    onDraftChange(
                      event.target.value !== String(plan.component.periodMs) ||
                        plan.signals.some((item) => ids[item.port] !== String(item.canId)),
                    );
                    setPreview(null);
                  }}
                  disabled={busy || locked}
                  inputMode="numeric"
                />
              </label>
            </div>
            <p>
              诊断 CAN ID {plan.diagnostic.requestCanId} / {plan.diagnostic.responseCanId}，DID 0x
              {plan.diagnostic.did.toString(16).toUpperCase()}。
              诊断参数、类型、端口和引用在此入口保持只读。
            </p>
            <div className="section-actions">
              <button
                className="primary-button compact"
                onClick={apply}
                disabled={busy || locked || !changed}
              >
                应用并校验修改
              </button>
              <button
                className="outline-button small"
                onClick={() => restoreIntegrationDraft()}
                disabled={busy || locked || !changed}
              >
                还原草稿
              </button>
              <button
                className="outline-button small"
                onClick={showPreview}
                disabled={busy || locked || changed || !workspace.dirty}
              >
                预览保存
              </button>
            </div>
          </div>
        </>
      )}
      {preview && (
        <div className="inspector-block" aria-label="标准保存预览">
          <h3>保存预览</h3>
          <p>
            {preview.files.filter((file) => file.changed).length} 个文件将修改；其余文件保持原字节。
          </p>
          <div className="section-actions">
            {preview.files.map((file) => (
              <button
                className="outline-button small"
                key={file.path}
                onClick={() => setSelectedFile(file.path)}
              >
                {file.path.split(/[\\/]/).pop()} · {file.changed ? '将修改' : '不变'}
              </button>
            ))}
          </div>
          {selected && (
            <>
              <p className="mono path-text">{selected.path}</p>
              {selected.changed ? (
                <div className="integration-diff">
                  <details>
                    <summary>保存前原文</summary>
                    <pre>{selected.before}</pre>
                  </details>
                  <details>
                    <summary>保存后原文</summary>
                    <pre>{selected.after}</pre>
                  </details>
                </div>
              ) : (
                <p>此文件原字节保持不变。</p>
              )}
            </>
          )}
          <button
            className="primary-button compact"
            onClick={save}
            disabled={busy || locked || changed}
          >
            确认保存标准输入
          </button>
        </div>
      )}
    </div>
  );
}
