import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { requestConfirmation } from './confirmation';
import type { IntegrationInspection, PlanDiagnostic, SavePreview, WorkspaceView } from './types';

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

function diagnostics(error: unknown): PlanDiagnostic[] {
  if (Array.isArray(error) && error.every((item) => item && typeof item.code === 'string')) {
    return error as PlanDiagnostic[];
  }
  return [
    {
      category: 'tool',
      code: 'IPC_ERROR',
      file: null,
      object: null,
      message: String(error),
      remedy: '检查桌面连接及输入来源后重试。',
    },
  ];
}

type Props = {
  workspace: WorkspaceView;
  native: boolean;
  onView: (view: WorkspaceView) => void;
  onDraftChange: (changed: boolean) => void;
  onBusyChange: (busy: boolean) => void;
  locked: boolean;
};

export function IntegrationPanel({
  workspace,
  native,
  onView,
  onDraftChange,
  onBusyChange,
  locked,
}: Props) {
  const [inspection, setInspection] = useState<IntegrationInspection | null>(null);
  const [issues, setIssues] = useState<PlanDiagnostic[]>([]);
  const [ids, setIds] = useState<Record<string, string>>({});
  const [period, setPeriod] = useState('');
  const [busy, setBusy] = useState(native);
  const [preview, setPreview] = useState<SavePreview | null>(null);
  const [notice, setNotice] = useState('尚未检查标准输入');
  const [selectedFile, setSelectedFile] = useState('');

  function processing(value: boolean) {
    setBusy(value);
    onBusyChange(value);
  }

  const accept = useCallback(
    (report: IntegrationInspection) => {
      setInspection(report);
      setIssues(report.diagnostics);
      setPreview(null);
      onDraftChange(false);
      if (report.description) {
        setIds(
          Object.fromEntries(
            report.description.signals.map((signal) => [signal.port, String(signal.canId)]),
          ),
        );
        setPeriod(String(report.description.component.periodMs));
      }
    },
    [onDraftChange],
  );

  async function inspect() {
    if (!native || busy || locked) return;
    processing(true);
    try {
      const report = await invoke<IntegrationInspection>('inspect_integration');
      accept(report);
      setNotice(report.description ? '标准输入已校验，尚未生成运行工程' : '输入未通过，无法生成');
    } catch (error) {
      setInspection(null);
      setIssues(diagnostics(error));
      setNotice('检查失败，无法生成');
    } finally {
      processing(false);
    }
  }

  useEffect(() => {
    if (!native) return;
    let active = true;
    void invoke<IntegrationInspection>('inspect_integration')
      .then((report) => {
        if (!active) return;
        accept(report);
        setNotice(report.description ? '标准输入已校验，尚未生成运行工程' : '输入未通过，无法生成');
      })
      .catch((error: unknown) => {
        if (active) {
          setInspection(null);
          setIssues(diagnostics(error));
          setNotice('检查失败，无法生成');
        }
      })
      .finally(() => {
        if (active) {
          setBusy(false);
          onBusyChange(false);
        }
      });
    return () => {
      active = false;
    };
  }, [native, accept, onBusyChange]);

  async function apply() {
    if (!inspection?.description || busy || locked) return;
    const numericIds: Record<string, number> = {};
    for (const [path, value] of Object.entries(ids)) {
      if (!/^\d+$/.test(value) || Number(value) > 2047) {
        setIssues(diagnostics('CAN ID 必须是 0–2047 的十进制整数。'));
        return;
      }
      numericIds[path] = Number(value);
    }
    if (!/^\d+$/.test(period) || !Number.isSafeInteger(Number(period)) || Number(period) < 1) {
      setIssues(diagnostics('周期必须是正整数毫秒。'));
      return;
    }
    processing(true);
    try {
      const report = await invoke<IntegrationInspection>('edit_integration', {
        changes: { canIds: numericIds, applicationPeriodMs: Number(period) },
      });
      accept(report);
      onView(await invoke<WorkspaceView>('workspace_view'));
      setNotice('修改已通过同一计划校验，尚未保存');
    } catch (error) {
      setIssues(diagnostics(error));
      setNotice('编辑被拒绝，原配置保持');
    } finally {
      processing(false);
    }
  }

  async function showPreview() {
    if (busy || locked || !native) return;
    processing(true);
    try {
      const result = await invoke<SavePreview>('preview_integration_save');
      setPreview(result);
      setSelectedFile(
        result.files.find((file) => file.changed)?.path ?? result.files[0]?.path ?? '',
      );
      setIssues([]);
    } catch (error) {
      setIssues(diagnostics(error));
      setPreview(null);
    } finally {
      processing(false);
    }
  }

  async function save() {
    if (!preview || busy || locked) return;
    processing(true);
    try {
      accept(
        await invoke<IntegrationInspection>('save_integration', { revision: preview.revision }),
      );
      onView(await invoke<WorkspaceView>('workspace_view'));
      setNotice('标准输入已保存，尚未生成运行工程');
    } catch (error) {
      setIssues(diagnostics(error));
      setNotice('保存未完成，请检查原因与恢复文件');
    } finally {
      processing(false);
    }
  }

  async function reopen() {
    if (busy || locked || !native) return;
    processing(true);
    try {
      if (
        (workspace.dirty || changed) &&
        !(await requestConfirmation('重开将放弃尚未保存或应用的修改，确定继续？'))
      )
        return;
      onView(
        await invoke<WorkspaceView>('open_project', {
          paths: workspace.files.map((file) => file.path),
        }),
      );
      const report = await invoke<IntegrationInspection>('inspect_integration');
      accept(report);
      setNotice(report.description ? '已重开并校验保存的标准输入' : '重开输入未通过，无法生成');
    } catch (error) {
      setInspection(null);
      setIssues(diagnostics(error));
    } finally {
      processing(false);
    }
  }

  const plan = inspection?.description;
  const selected = preview?.files.find((file) => file.path === selectedFile);
  const changed = Boolean(
    plan &&
    (period !== String(plan.component.periodMs) ||
      plan.signals.some((signal) => ids[signal.port] !== String(signal.canId))),
  );

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
        有效无关内容按原字节保留；XSD 与 MOD 是外部提供的校验依赖。
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
                onClick={() => accept(inspection)}
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
